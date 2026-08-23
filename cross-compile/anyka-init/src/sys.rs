//! The syscall boundary.
//!
//! Everything the supervisor does to the outside world goes through `Sys`, so
//! that `supervise::decide` and friends can be exercised on the host with
//! `MockSys`. This is the only module in the crate that contains `unsafe`.

use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::mem::ManuallyDrop;
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub type Pid = i32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnSpec {
    pub exec: String,
    pub args: Vec<String>,
    /// Applied after the child's environment is cleared. See
    /// `config::ServiceCfg::env`.
    pub env: BTreeMap<String, String>,
    pub log: String,
    pub core_dump: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitStatus {
    Code(i32),
    Signal(i32),
}

impl ExitStatus {
    /// Zero exit code and no signal. The `sha256sum -c` contract: `success()`
    /// means every file in the manifest matched.
    pub fn success(&self) -> bool {
        matches!(self, ExitStatus::Code(0))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SysError {
    #[error("start {exec} failed: {source}")]
    Spawn {
        exec: String,
        #[source]
        source: std::io::Error,
    },
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Other(String),
}

#[cfg_attr(test, mockall::automock)]
pub trait Sys: Send + Sync {
    fn spawn(&self, spec: &SpawnSpec) -> Result<Pid, SysError>;
    /// Non-blocking reap of any exited child (`waitpid(-1, WNOHANG)`).
    ///
    /// Returns `Ok(None)` when no child has exited yet. The reaper thread polls
    /// this so it can observe a shutdown flag — a blocking `waitpid` would make
    /// the reaper unstoppable and steal exits from later host tests.
    fn wait_any(&self) -> Result<Option<(Pid, ExitStatus)>, SysError>;
    fn kill(&self, pid: Pid, sig: i32) -> Result<(), SysError>;
    fn reboot(&self) -> Result<(), SysError>;
    /// Monotonic. Never `SystemTime` — P2.5 steps the wall clock by decades.
    fn now(&self) -> Instant;
    fn realtime(&self) -> SystemTime;
    fn set_realtime(&self, t: SystemTime) -> Result<(), SysError>;
    /// Elapsed since this process started. Used by the storm-guard reset.
    fn uptime(&self) -> Duration;
    fn insmod(&self, path: &str) -> Result<(), SysError>;
    fn rmmod(&self, name: &str) -> Result<(), SysError>;
    /// Mockable sleep. Bring-up has several vendor-transcribed settle delays;
    /// a real `thread::sleep` would make the bring-up tests take 40 seconds.
    fn sleep(&self, d: Duration);
    fn run_to_completion(&self, prog: &str, args: &[String]) -> Result<ExitStatus, SysError>;
    /// Spawn without log-file plumbing. Used by wifi bring-up before the
    /// supervised service table is registered.
    fn spawn_detached(&self, prog: &str, args: &[String]) -> Result<Pid, SysError>;
}

/// Production syscall backend.
pub struct RealSys {
    started: Instant,
    /// Serializes `wait_any` (the reaper thread) against `run_to_completion`
    /// (the update path). Both call `waitpid`, and only one thread may hold a
    /// child's exit status. Without this lock the reaper's `waitpid(-1,
    /// WNOHANG)` can reap a child that `run_to_completion` spawned, and the
    /// latter then sees `ECHILD` and discards a valid update.
    reap_lock: Mutex<()>,
}

impl RealSys {
    pub fn new() -> Self {
        Self {
            started: Instant::now(),
            reap_lock: Mutex::new(()),
        }
    }
}

impl Default for RealSys {
    fn default() -> Self {
        Self::new()
    }
}

impl Sys for RealSys {
    fn spawn(&self, spec: &SpawnSpec) -> Result<Pid, SysError> {
        let log = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&spec.log)
            .map_err(|source| SysError::Spawn {
                exec: spec.exec.clone(),
                source,
            })?;
        let log_err = log.try_clone().map_err(|source| SysError::Spawn {
            exec: spec.exec.clone(),
            source,
        })?;

        let mut cmd = Command::new(&spec.exec);
        cmd.args(&spec.args)
            .env_clear()
            .envs(&spec.env)
            .stdin(Stdio::null())
            .stdout(Stdio::from(log))
            .stderr(Stdio::from(log_err));

        // Run each service from its own directory. This makes relative paths in
        // a service's own config resolve inside its slot: onvif-rust's
        // `static_root = "www"` (config_debug.toml:129) resolved against CWD=/
        // before, which is how the WebUI came up serving nothing.
        //
        // A bare executable name (`exec = "busybox"`) has an empty parent,
        // which `current_dir` rejects with ENOENT; only touch the directory
        // when the exec path actually names one.
        if let Some(parent) = Path::new(&spec.exec).parent()
            && !parent.as_os_str().is_empty()
        {
            cmd.current_dir(parent);
        }

        if spec.core_dump {
            // SAFETY: setrlimit is async-signal-safe and only touches this
            // child's own soft/hard RLIMIT_CORE before exec.
            unsafe {
                cmd.pre_exec(|| {
                    let lim = libc::rlimit {
                        rlim_cur: libc::RLIM_INFINITY,
                        rlim_max: libc::RLIM_INFINITY,
                    };
                    if libc::setrlimit(libc::RLIMIT_CORE, &lim) != 0 {
                        return Err(std::io::Error::last_os_error());
                    }
                    Ok(())
                });
            }
        }

        let child = cmd.spawn().map_err(|source| SysError::Spawn {
            exec: spec.exec.clone(),
            source,
        })?;
        // The reaper thread owns reaping via waitpid(-1). Dropping Child would
        // race that waitpid. Wrap in ManuallyDrop so Drop/wait never runs;
        // do not into_inner — reaper owns waitpid(-1).
        let child = ManuallyDrop::new(child);
        let pid = child.id() as Pid;
        Ok(pid)
    }

    fn wait_any(&self) -> Result<Option<(Pid, ExitStatus)>, SysError> {
        // Hold the reap lock so a `run_to_completion` from the update path
        // cannot have its child's exit status stolen. The reaper and the
        // update thread are the only two `waitpid` callers in the process.
        let _guard = self
            .reap_lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut status: libc::c_int = 0;
        // SAFETY: waitpid(-1, WNOHANG) reaps any exited child; status is stack.
        let pid = unsafe { libc::waitpid(-1, &mut status, libc::WNOHANG) };
        if pid < 0 {
            let err = std::io::Error::last_os_error();
            if err.raw_os_error() == Some(libc::ECHILD) {
                return Ok(None);
            }
            return Err(SysError::Io(err));
        }
        if pid == 0 {
            return Ok(None);
        }
        // WIF* helpers are safe wrappers around the wait status integer.
        let exit = if libc::WIFSIGNALED(status) {
            ExitStatus::Signal(libc::WTERMSIG(status))
        } else {
            ExitStatus::Code(libc::WEXITSTATUS(status))
        };
        Ok(Some((pid, exit)))
    }

    fn kill(&self, pid: Pid, sig: i32) -> Result<(), SysError> {
        // SAFETY: kill(2) with a pid we previously spawned.
        let rc = unsafe { libc::kill(pid, sig) };
        if rc != 0 {
            return Err(SysError::Io(std::io::Error::last_os_error()));
        }
        Ok(())
    }

    fn reboot(&self) -> Result<(), SysError> {
        // SAFETY: sync(2) takes no args; reboot(RB_AUTOBOOT) does not return
        // on success.
        unsafe {
            libc::sync();
            let rc = libc::reboot(libc::RB_AUTOBOOT);
            if rc != 0 {
                return Err(SysError::Io(std::io::Error::last_os_error()));
            }
        }
        Err(SysError::Other("reboot returned unexpectedly".into()))
    }

    fn now(&self) -> Instant {
        Instant::now()
    }

    fn realtime(&self) -> SystemTime {
        SystemTime::now()
    }

    fn set_realtime(&self, t: SystemTime) -> Result<(), SysError> {
        let dur = t
            .duration_since(UNIX_EPOCH)
            .map_err(|e| SysError::Other(format!("time before unix epoch: {e}")))?;
        let tv_sec = libc::time_t::try_from(dur.as_secs()).map_err(|_| {
            SysError::Other(format!(
                "unix time {} overflows this platform's time_t",
                dur.as_secs()
            ))
        })?;
        let ts = libc::timespec {
            tv_sec,
            tv_nsec: dur.subsec_nanos() as libc::c_long,
        };
        // SAFETY: timespec is stack-local and valid for CLOCK_REALTIME.
        let rc = unsafe { libc::clock_settime(libc::CLOCK_REALTIME, &ts) };
        if rc != 0 {
            return Err(SysError::Io(std::io::Error::last_os_error()));
        }
        Ok(())
    }

    fn uptime(&self) -> Duration {
        self.started.elapsed()
    }

    fn insmod(&self, path: &str) -> Result<(), SysError> {
        match self.run_to_completion("insmod", &[path.to_string()])? {
            ExitStatus::Code(0) => Ok(()),
            other => Err(SysError::Other(format!("insmod {path} failed: {other:?}"))),
        }
    }

    fn rmmod(&self, name: &str) -> Result<(), SysError> {
        // Unlike insmod, a failure here is routine and not an error: the module
        // is usually not loaded yet on a cold boot. The caller logs at debug.
        self.run_to_completion("rmmod", &[name.to_string()])?;
        Ok(())
    }

    fn sleep(&self, d: Duration) {
        std::thread::sleep(d);
    }

    fn run_to_completion(&self, prog: &str, args: &[String]) -> Result<ExitStatus, SysError> {
        // Hold the reap lock across spawn + wait so the reaper's
        // `waitpid(-1, WNOHANG)` cannot steal this child between the two
        // syscalls. Safe to call from any thread, unlike the older constraint
        // that this only run during P2 before the reaper starts — the lock
        // makes the update thread (which outlives the reaper) safe too.
        let _guard = self
            .reap_lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let status = Command::new(prog).args(args).status()?;
        if let Some(code) = status.code() {
            Ok(ExitStatus::Code(code))
        } else if let Some(sig) = status.signal() {
            Ok(ExitStatus::Signal(sig))
        } else {
            Err(SysError::Other(format!(
                "{prog} exited with neither code nor signal"
            )))
        }
    }

    fn spawn_detached(&self, prog: &str, args: &[String]) -> Result<Pid, SysError> {
        let mut cmd = Command::new(prog);
        cmd.args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let child = cmd.spawn().map_err(|source| SysError::Spawn {
            exec: prog.to_string(),
            source,
        })?;
        // Same ManuallyDrop pattern as spawn(): the reaper owns waitpid.
        // Do not into_inner / drop — reaper owns waitpid(-1).
        let child = ManuallyDrop::new(child);
        let pid = child.id() as Pid;
        Ok(pid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    /// Serializes every test in this binary that forks.
    ///
    /// `fork` clones the whole fd table, so a child inherits any fd another
    /// test thread happens to have open at that instant — including the write
    /// handle `fs::write` holds while creating the probe script below. Linux
    /// then refuses to `execve` that script while any process holds it open
    /// for writing, and the spawn fails with ETXTBSY ("Text file busy").
    ///
    /// These two tests are the only fork sites in the lib test binary
    /// (`supervision.rs` is a separate process), so serializing them closes
    /// the window entirely rather than retrying around it. Measured at 5
    /// failures per 120 runs of the full binary before this lock, 0 after.
    /// A new test that spawns must take this lock too.
    static FORK_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn a_spawned_child_runs_in_its_executable_directory() {
        let _fork_guard = FORK_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let d = tempfile::tempdir().unwrap();
        let script = d.path().join("probe.sh");
        std::fs::write(&script, "#!/bin/sh\npwd\n").unwrap();
        // set the exec bit the same way tests/supervision.rs does
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();

        let log = d.path().join("out.log");
        let spec = SpawnSpec {
            exec: script.to_string_lossy().into_owned(),
            args: Vec::new(),
            env: BTreeMap::new(),
            log: log.to_string_lossy().into_owned(),
            core_dump: false,
        };
        let pid = RealSys::new().spawn(&spec).unwrap();

        // Poll the log briefly: the child prints its working directory.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let contents = loop {
            if let Ok(s) = std::fs::read_to_string(&log)
                && !s.is_empty()
            {
                break s;
            }
            if std::time::Instant::now() >= deadline {
                panic!("child never wrote to {}", log.display());
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        };
        assert!(
            contents.trim().starts_with(&*d.path().to_string_lossy()),
            "child CWD was {:?}, expected the executable's directory {}",
            contents.trim(),
            d.path().display()
        );
        // RealSys::spawn wraps Child in ManuallyDrop (the reaper owns waitpid in
        // production), so this test must reap the pid it spawned or a zombie
        // lingers for the whole test binary.
        // SAFETY: pid is the still-running child this test spawned.
        let mut status: libc::c_int = 0;
        assert_eq!(unsafe { libc::waitpid(pid, &mut status, 0) }, pid);
    }

    #[test]
    fn a_bare_executable_name_spawns_through_path() {
        let _fork_guard = FORK_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        // `exec = "busybox"` style: no directory component, so no current_dir
        // may be set (an empty one makes spawn fail with ENOENT). `true` is
        // resolved through PATH and must start cleanly.
        let d = tempfile::tempdir().unwrap();
        let log = d.path().join("out.log");
        let spec = SpawnSpec {
            exec: "true".to_string(),
            args: Vec::new(),
            env: BTreeMap::new(),
            log: log.to_string_lossy().into_owned(),
            core_dump: false,
        };
        let pid = RealSys::new()
            .spawn(&spec)
            .expect("bare exec must spawn via PATH");
        // SAFETY: pid is the child this test spawned.
        let mut status: libc::c_int = 0;
        assert_eq!(unsafe { libc::waitpid(pid, &mut status, 0) }, pid);
        assert!(libc::WIFEXITED(status) && libc::WEXITSTATUS(status) == 0);
    }
}
