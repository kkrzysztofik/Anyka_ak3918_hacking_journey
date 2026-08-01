//! Host integration tests for the supervision loop.
//!
//! These call `waitpid(-1)`, which reaps *any* child of the process. A process-
//! wide mutex serializes the suite so parallel `cargo test` cannot cross-reap.

use anyka_init::config::Config;
use anyka_init::supervisor_loop::{self, Msg};
use anyka_init::sys::{RealSys, Sys};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::time::Duration;

/// Hold for the duration of any test that starts a reaper or waits on children.
fn serialize_waitpid_tests() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

fn require_tool(candidates: &[&str]) -> PathBuf {
    for c in candidates {
        let p = Path::new(c);
        if p.is_file() {
            return p.to_path_buf();
        }
    }
    panic!("required tool not found; tried {candidates:?}");
}

fn sleep_bin() -> PathBuf {
    require_tool(&["/bin/sleep", "/usr/bin/sleep"])
}

fn env_bin() -> PathBuf {
    require_tool(&["/usr/bin/env", "/bin/env"])
}

fn write_config(dir: &Path, service_toml: &str) -> PathBuf {
    let log_dir = dir.join("logs");
    std::fs::create_dir_all(&log_dir).expect("log dir");
    let cfg_path = dir.join("anyka.toml");
    let body = format!(
        r#"
[wifi]
ssid = "test"
password = "test"

[time]
enabled = false

[monitor]
enabled = false

[supervisor]
backoff_min_sec = 1
backoff_max_sec = 4
crashloop_count = 100
crashloop_window_sec = 600

{service_toml}
"#
    );
    std::fs::write(&cfg_path, body).expect("write config");
    cfg_path
}

fn write_exec_script(path: &Path, body: &str) {
    std::fs::write(path, body).expect("write script");
    let mut perms = std::fs::metadata(path).expect("meta").permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(path, perms).expect("chmod");
}

struct Harness {
    tx: std::sync::mpsc::Sender<Msg>,
    handle: std::thread::JoinHandle<()>,
    reaper: Option<std::thread::JoinHandle<()>>,
    stop: Arc<AtomicBool>,
    _guard: MutexGuard<'static, ()>,
}

impl Harness {
    fn start(cfg: Config) -> Self {
        let guard = serialize_waitpid_tests();
        let sys: Arc<dyn Sys> = Arc::new(RealSys::new());
        let (tx, rx) = supervisor_loop::make_channel();
        let stop = Arc::new(AtomicBool::new(false));
        let reaper = supervisor_loop::spawn_reaper(Arc::clone(&sys), tx.clone(), Arc::clone(&stop));
        let cfg = Arc::new(cfg);
        let handle = std::thread::spawn(move || supervisor_loop::run(sys, &cfg, rx));
        Self {
            tx,
            handle,
            reaper,
            stop,
            _guard: guard,
        }
    }

    fn stop(self) {
        let _ = self.tx.send(Msg::Shutdown);
        let _ = self.handle.join();
        self.stop.store(true, Ordering::Relaxed);
        if let Some(reaper) = self.reaper {
            let _ = reaper.join();
        }
        // _guard drops only after the reaper has exited.
    }
}

fn drive(cfg: Config, settle: Duration) {
    let h = Harness::start(cfg);
    std::thread::sleep(settle);
    h.stop();
}

#[test]
fn test_run_service_exits_immediately_is_restarted() {
    let dir = tempfile::tempdir().expect("tempdir");
    let counter = dir.path().join("starts");
    let script = dir.path().join("flaky.sh");
    let svc_log = dir.path().join("logs").join("svc.log");
    write_exec_script(
        &script,
        &format!("#!/bin/sh\necho start >> '{}'\nexit 1\n", counter.display()),
    );

    let cfg_path = write_config(
        dir.path(),
        &format!(
            r#"
[services.flaky]
enabled = true
exec = "{}"
log = "{}"
"#,
            script.display(),
            svc_log.display()
        ),
    );
    let cfg = Config::load(cfg_path.to_str().expect("utf8")).expect("load");
    drive(cfg, Duration::from_secs(4));

    let n = std::fs::read_to_string(&counter)
        .unwrap_or_default()
        .lines()
        .count();
    assert!(
        n > 1,
        "expected more than one start after ~4s with 1s backoff, got {n}"
    );
}

#[test]
fn test_run_stable_service_is_not_restarted() {
    let sleep = sleep_bin();
    let dir = tempfile::tempdir().expect("tempdir");
    let pidfile = dir.path().join("pid");
    let script = dir.path().join("sleeper.sh");
    let svc_log = dir.path().join("logs").join("svc.log");
    write_exec_script(
        &script,
        &format!(
            "#!/bin/sh\necho $$ > '{}'\nexec '{}' 300\n",
            pidfile.display(),
            sleep.display()
        ),
    );

    let cfg_path = write_config(
        dir.path(),
        &format!(
            r#"
[services.sleeper]
enabled = true
exec = "{}"
log = "{}"
"#,
            script.display(),
            svc_log.display()
        ),
    );
    let cfg = Config::load(cfg_path.to_str().expect("utf8")).expect("load");
    let h = Harness::start(cfg);

    let mut pid = None;
    for _ in 0..50 {
        if let Ok(s) = std::fs::read_to_string(&pidfile)
            && let Ok(p) = s.trim().parse::<i32>()
        {
            pid = Some(p);
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    let pid = pid.expect("pidfile");

    std::thread::sleep(Duration::from_secs(3));
    let pid_later: i32 = std::fs::read_to_string(&pidfile)
        .expect("pidfile still there")
        .trim()
        .parse()
        .expect("pid");
    assert_eq!(pid, pid_later, "stable service PID must not change");
    // SAFETY: kill with signal 0 performs a permission and existence check
    // only; it sends no signal and dereferences no pointer.
    assert_eq!(
        unsafe { libc::kill(pid, 0) },
        0,
        "child must still be alive"
    );

    h.stop();
}

#[test]
fn test_run_restart_service_message_respawns_the_child() {
    let sleep = sleep_bin();
    let dir = tempfile::tempdir().expect("tempdir");
    let pidfile = dir.path().join("pid");
    let script = dir.path().join("sleeper.sh");
    let svc_log = dir.path().join("logs").join("svc.log");
    write_exec_script(
        &script,
        &format!(
            "#!/bin/sh\necho $$ > '{}'\nexec '{}' 300\n",
            pidfile.display(),
            sleep.display()
        ),
    );

    let cfg_path = write_config(
        dir.path(),
        &format!(
            r#"
[services.sleeper]
enabled = true
exec = "{}"
log = "{}"
"#,
            script.display(),
            svc_log.display()
        ),
    );
    let cfg = Config::load(cfg_path.to_str().expect("utf8")).expect("load");
    let h = Harness::start(cfg);

    let mut first_pid = None;
    for _ in 0..50 {
        if let Ok(s) = std::fs::read_to_string(&pidfile)
            && let Ok(p) = s.trim().parse::<i32>()
        {
            first_pid = Some(p);
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    let first_pid = first_pid.expect("initial pid");

    let _ = h.tx.send(Msg::RestartService("sleeper".into()));

    let mut new_pid = None;
    for _ in 0..80 {
        if let Ok(s) = std::fs::read_to_string(&pidfile)
            && let Ok(p) = s.trim().parse::<i32>()
            && p != first_pid
        {
            new_pid = Some(p);
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    let new_pid = new_pid.expect("respawned pid after RestartService");
    assert_ne!(first_pid, new_pid);

    h.stop();
}

#[test]
fn test_run_env_is_cleared_for_children() {
    let env = env_bin();
    let dir = tempfile::tempdir().expect("tempdir");
    let svc_log = dir.path().join("logs").join("svc.log");

    // SAFETY: test process only; sets a leak marker the child must not inherit.
    unsafe {
        std::env::set_var("ANYKA_TEST_LEAK", "should-not-appear");
    }

    let cfg_path = write_config(
        dir.path(),
        &format!(
            r#"
[services.envcheck]
enabled = true
exec = "{}"
log = "{}"
env = {{ ANYKA_TEST_INJECTED = "present" }}
"#,
            env.display(),
            svc_log.display()
        ),
    );
    let cfg = Config::load(cfg_path.to_str().expect("utf8")).expect("load");
    drive(cfg, Duration::from_secs(2));

    let captured = std::fs::read_to_string(&svc_log).unwrap_or_else(|e| {
        panic!(
            "svc log missing after envcheck run (is {} executable?): {e}",
            env.display()
        )
    });
    assert!(
        captured.contains("ANYKA_TEST_INJECTED=present"),
        "injected env missing; log:\n{captured}"
    );
    assert!(
        !captured.contains("ANYKA_TEST_LEAK"),
        "parent env leaked into child; log:\n{captured}"
    );
}
