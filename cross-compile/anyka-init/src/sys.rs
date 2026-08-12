//! The syscall boundary.
//!
//! Everything the supervisor does to the outside world goes through `Sys`, so
//! that `supervise::decide` and friends can be exercised on the host with
//! `MockSys`. This is the only module in the crate that contains `unsafe`.

use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::process::{Command, Stdio};
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
}

impl RealSys {
    pub fn new() -> Self {
        Self {
            started: Instant::now(),
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
        let pid = child.id() as Pid;
        // The reaper thread owns reaping via waitpid(-1). Dropping Child would
        // race that waitpid.
        std::mem::forget(child);
        Ok(pid)
    }

    fn wait_any(&self) -> Result<Option<(Pid, ExitStatus)>, SysError> {
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
        // Uses std's wait, which races the reaper's waitpid(-1). Only call
        // during P2, before the reaper starts.
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
        let pid = child.id() as Pid;
        // Same forget pattern as spawn(): the reaper owns waitpid.
        std::mem::forget(child);
        Ok(pid)
    }
}
