//! Host integration tests for the supervision loop.
//!
//! These call `waitpid(-1)`, which reaps *any* child of the process, so they
//! must run single-threaded: `cargo test -- --test-threads=1`.

use anyka_init::config::Config;
use anyka_init::supervisor_loop::{self, Msg};
use anyka_init::sys::{RealSys, Sys};
use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;
use std::time::Duration;

fn write_config(dir: &std::path::Path, service_toml: &str) -> std::path::PathBuf {
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

fn write_exec_script(path: &std::path::Path, body: &str) {
    std::fs::write(path, body).expect("write script");
    let mut perms = std::fs::metadata(path).expect("meta").permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(path, perms).expect("chmod");
}

fn drive(cfg: Config, settle: Duration) {
    use std::sync::atomic::{AtomicBool, Ordering};

    let sys: Arc<dyn Sys> = Arc::new(RealSys::new());
    let (tx, rx) = supervisor_loop::make_channel();
    let stop = Arc::new(AtomicBool::new(false));
    supervisor_loop::spawn_reaper(Arc::clone(&sys), tx.clone(), Arc::clone(&stop));

    let cfg = Arc::new(cfg);
    let cfg_thread = Arc::clone(&cfg);
    let handle = std::thread::spawn(move || {
        supervisor_loop::run(sys, &cfg_thread, rx);
    });

    std::thread::sleep(settle);
    let _ = tx.send(Msg::Shutdown);
    let _ = handle.join();
    stop.store(true, Ordering::Relaxed);
    std::thread::sleep(Duration::from_millis(100));
}

#[test]
fn test_service_that_exits_immediately_is_restarted() {
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
fn test_stable_service_is_not_restarted() {
    let dir = tempfile::tempdir().expect("tempdir");
    let pidfile = dir.path().join("pid");
    let script = dir.path().join("sleeper.sh");
    let svc_log = dir.path().join("logs").join("svc.log");
    write_exec_script(
        &script,
        &format!(
            "#!/bin/sh\necho $$ > '{}'\nexec /bin/sleep 300\n",
            pidfile.display()
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

    use std::sync::atomic::{AtomicBool, Ordering};

    let sys: Arc<dyn Sys> = Arc::new(RealSys::new());
    let (tx, rx) = supervisor_loop::make_channel();
    let stop = Arc::new(AtomicBool::new(false));
    supervisor_loop::spawn_reaper(Arc::clone(&sys), tx.clone(), Arc::clone(&stop));
    let cfg = Arc::new(cfg);
    let cfg_thread = Arc::clone(&cfg);
    let handle = std::thread::spawn(move || {
        supervisor_loop::run(sys, &cfg_thread, rx);
    });

    // Wait until the pidfile appears.
    let mut pid = None;
    for _ in 0..50 {
        if let Ok(s) = std::fs::read_to_string(&pidfile) {
            if let Ok(p) = s.trim().parse::<i32>() {
                pid = Some(p);
                break;
            }
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
    assert_eq!(
        unsafe { libc::kill(pid, 0) },
        0,
        "child must still be alive"
    );

    let _ = tx.send(Msg::Shutdown);
    let _ = handle.join();
    stop.store(true, Ordering::Relaxed);
    std::thread::sleep(Duration::from_millis(100));
}

#[test]
fn test_env_is_cleared_for_children() {
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
exec = "/usr/bin/env"
log = "{}"
env = {{ ANYKA_TEST_INJECTED = "present" }}
"#,
            svc_log.display()
        ),
    );
    let cfg = Config::load(cfg_path.to_str().expect("utf8")).expect("load");
    drive(cfg, Duration::from_secs(2));

    let captured = std::fs::read_to_string(&svc_log).expect("svc log");
    assert!(
        captured.contains("ANYKA_TEST_INJECTED=present"),
        "injected env missing; log:\n{captured}"
    );
    assert!(
        !captured.contains("ANYKA_TEST_LEAK"),
        "parent env leaked into child; log:\n{captured}"
    );
}
