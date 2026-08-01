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
    let _ = std::fs::remove_file(&oldest);
    for n in (1..keep).rev() {
        let from = format!("{path}.{n}");
        let to = format!("{path}.{}", n + 1);
        let _ = std::fs::rename(&from, &to);
    }
    std::fs::rename(path, format!("{path}.1"))
}

/// Install the tracing subscriber writing to `<dir>/anyka-init.log`, with
/// ERROR-level events additionally reaching stderr (which `service.sh` leaves
/// attached to the boot console).
pub fn init(dir: &str, level: &str) -> anyhow::Result<()> {
    std::fs::create_dir_all(dir)?;
    let path = format!("{dir}/anyka-init.log");
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
                .with_writer(file),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .with_writer(std::io::stderr)
                .with_filter(tracing_subscriber::filter::LevelFilter::ERROR),
        )
        .init();
    Ok(())
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
}
