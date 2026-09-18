//! Logging.
//!
//! Rotation is size-based and never time-based. `tracing-appender`'s
//! `Rotation::DAILY` names files from the wall clock, and P2.5 steps that clock
//! from the epoch to the real date mid-boot: the boot record would land in
//! `anyka-init.log.1970-01-01` with a discontinuity at every boundary.

use std::io::Write;

/// Rotate `path` if it exceeds `max_bytes`, keeping `keep` generations.
///
/// ponytail: service logs rotate only at start time. The supervisor holds the
/// child's fd, so renaming underneath a live child leaves it writing to the
/// renamed inode; correcting that needs the fd reopened and dup2'd, which is
/// only possible when the child is (re)started. Self-corrects for a
/// crash-looping service; a stable chatty one grows until its next restart.
/// Upgrade path: move service logs to syslog, or SIGSTOP/reopen/SIGCONT from
/// the monitor thread.
pub fn rotate_if_needed(path: &str, max_bytes: u64, keep: u8) -> std::io::Result<()> {
    let Ok(meta) = std::fs::metadata(path) else {
        return Ok(()); // nothing to rotate yet
    };
    if meta.len() <= max_bytes {
        return Ok(());
    }
    let oldest = format!("{path}.{keep}");
    ignore_not_found(std::fs::remove_file(&oldest))?;
    for n in (1..keep).rev() {
        let from = format!("{path}.{n}");
        let to = format!("{path}.{}", n + 1);
        ignore_not_found(std::fs::rename(&from, &to))?;
    }
    std::fs::rename(path, format!("{path}.1"))
}

fn ignore_not_found(result: std::io::Result<()>) -> std::io::Result<()> {
    match result {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

/// Install the tracing subscriber writing to `<dir>/anyka-init.log`, with
/// ERROR-level events additionally reaching stderr (which `service.sh` leaves
/// attached to the boot console).
pub fn init(dir: &str, level: &str, max_bytes: u64, keep: u8) -> anyhow::Result<()> {
    std::fs::create_dir_all(dir)?;
    let path = format!("{dir}/anyka-init.log");
    rotate_if_needed(&path, max_bytes, keep)?;
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;

    use tracing_subscriber::prelude::*;
    let filter = tracing_subscriber::EnvFilter::try_new(level)
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .with_writer(file)
                .with_timer(LocalTimer),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .with_writer(std::io::stderr)
                .with_timer(LocalTimer)
                .with_filter(tracing_subscriber::filter::LevelFilter::ERROR),
        )
        .init();
    Ok(())
}

/// Timestamps log lines in the process's configured zone (the `TZ` env var).
///
/// libc `localtime_r` + `strftime` rather than chrono: this process is the
/// supervisor and its zone comes straight from `TZ`. The microsecond and
/// offset parts are assembled by hand so it works on C libraries without the
/// `%f`/`%:z` extensions (the camera's libc generation is unknown).
pub struct LocalTimer;

impl LocalTimer {
    /// Write an RFC3339-style timestamp in the current zone; returns length.
    pub fn format_now(buf: &mut [u8]) -> usize {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        // time_t is 64-bit on host glibc but 32-bit on the camera's uclibc;
        // the mask keeps the cast lossless on the 32-bit target.
        let secs: libc::time_t = (now.as_secs() & 0x7FFF_FFFF) as libc::time_t;
        let mut zeroed: libc::tm = unsafe { std::mem::zeroed() };
        // glibc caches the parsed zone and does not re-read a changed TZ env
        // var on its own (verified on glibc 2.43); tzset() forces the reparse.
        // Declared locally: the libc crate only exposes tzset for Windows.
        // SAFETY: tzset reads the process env and updates libc's own state;
        // it is async-signal-safe and cannot fail.
        unsafe extern "C" {
            fn tzset();
        }
        unsafe { tzset() };
        // SAFETY: localtime_r writes into the caller-provided struct and
        // never reads it uninitialized; it returns null on a broken clock,
        // in which case the zeroed struct (1970) is used instead of a deref.
        let ptr = unsafe { libc::localtime_r(&secs, &mut zeroed) };
        let tm = if ptr.is_null() {
            &zeroed
        } else {
            unsafe { &*ptr }
        };
        let mut out = [0u8; 20];
        // SAFETY: `out` fits "YYYY-MM-DDTHH:MM:SS" (19 chars + NUL) and the
        // format string is a NUL-terminated static byte array.
        let n = unsafe {
            libc::strftime(
                out.as_mut_ptr().cast::<libc::c_char>(),
                out.len(),
                c"%Y-%m-%dT%H:%M:%S".as_ptr(),
                tm,
            )
        };
        let micros = now.subsec_micros();
        let offset = tm.tm_gmtoff;
        let mut s = String::from_utf8_lossy(&out[..n]).into_owned();
        s.push_str(&format!(".{micros:06}"));
        s.push(if offset < 0 { '-' } else { '+' });
        let abs = offset.unsigned_abs();
        s.push_str(&format!("{:02}:{:02}", abs / 3600, (abs % 3600) / 60));
        let len = s.len().min(buf.len());
        buf[..len].copy_from_slice(s.as_bytes()[..len].try_into().unwrap_or(&[0u8; 0]));
        len
    }
}

impl tracing_subscriber::fmt::time::FormatTime for LocalTimer {
    fn format_time(&self, w: &mut tracing_subscriber::fmt::format::Writer<'_>) -> std::fmt::Result {
        let mut buf = [0u8; 64];
        let n = Self::format_now(&mut buf);
        write!(w, "{}", String::from_utf8_lossy(&buf[..n]))
    }
}

/// Last-resort output when the subscriber is not up yet or the log is
/// unwritable. Goes to the boot console.
pub fn console(msg: &str) {
    let _ = writeln!(std::io::stderr(), "anyka-init: {msg}");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_rotate_is_noop_below_threshold() {
        let dir = tempfile::tempdir().expect("tempdir");
        let p = dir.path().join("a.log");
        std::fs::write(&p, b"small").expect("write");
        rotate_if_needed(p.to_str().expect("utf8"), 1000, 2).expect("rotate");
        assert!(p.exists());
        assert!(!dir.path().join("a.log.1").exists());
    }

    #[test]
    fn test_rotate_moves_oversized_file_to_dot_one() {
        let dir = tempfile::tempdir().expect("tempdir");
        let p = dir.path().join("a.log");
        let mut f = std::fs::File::create(&p).expect("create");
        f.write_all(&vec![b'x'; 2048]).expect("fill");
        drop(f);
        rotate_if_needed(p.to_str().expect("utf8"), 1024, 2).expect("rotate");
        assert!(dir.path().join("a.log.1").exists());
        assert!(!p.exists(), "current log is moved aside, not copied");
    }

    #[test]
    fn test_rotate_discards_beyond_keep_count() {
        let dir = tempfile::tempdir().expect("tempdir");
        let base = dir.path().join("a.log");
        for name in ["a.log.1", "a.log.2"] {
            std::fs::write(dir.path().join(name), b"old").expect("write");
        }
        let mut f = std::fs::File::create(&base).expect("create");
        f.write_all(&vec![b'x'; 2048]).expect("fill");
        drop(f);
        rotate_if_needed(base.to_str().expect("utf8"), 1024, 2).expect("rotate");
        assert!(dir.path().join("a.log.1").exists());
        assert!(dir.path().join("a.log.2").exists());
        assert!(
            !dir.path().join("a.log.3").exists(),
            "keep=2 must not create .3"
        );
    }

    #[test]
    fn test_rotate_missing_file_is_ok() {
        let dir = tempfile::tempdir().expect("tempdir");
        let p = dir.path().join("nope.log");
        assert!(rotate_if_needed(p.to_str().expect("utf8"), 1024, 2).is_ok());
    }

    #[test]
    fn test_console_writes_without_panicking() {
        // stderr is the only sink; there is nothing to assert beyond "did not
        // panic", which is the whole point of a last-resort logger.
        console("boot: last-resort message");
    }

    #[test]
    fn test_local_timer_uses_the_process_tz() {
        // set_var here touches process-global libc TZ state; restore UTC when
        // done so no later test in this binary sees a shifted zone.
        unsafe { std::env::set_var("TZ", "UTC") };
        let mut buf = [0u8; 64];
        let n = LocalTimer::format_now(&mut buf);
        let s = String::from_utf8_lossy(&buf[..n]);
        assert!(
            s.ends_with("+00:00"),
            "UTC zone must carry an explicit offset, got {s}"
        );
        unsafe { std::env::set_var("TZ", "CET-1CEST,M3.5.0,M10.5.0/3") };
        let n2 = LocalTimer::format_now(&mut buf);
        let s2 = String::from_utf8_lossy(&buf[..n2]);
        assert!(
            s2.contains("+01") || s2.contains("+02"),
            "expected a CET/CEST offset, got {s2}"
        );
        unsafe { std::env::set_var("TZ", "UTC") };
    }
}
