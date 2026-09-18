//! Manual-clock marker shared with anyka-init.
//!
//! Deliberately a bare filename, not a parsed file: `anyka-init/src/update.rs:143`
//! records why structured state on exFAT after a power cut is a hazard here.

use std::path::{Path, PathBuf};

/// `{update_root}/state/ntp.disabled`.
pub struct NtpMarker {
    path: PathBuf,
}

impl NtpMarker {
    pub fn new(update_root: impl AsRef<Path>) -> Self {
        Self {
            path: update_root.as_ref().join("state/ntp.disabled"),
        }
    }

    /// Absent marker means NTP runs. Absence is the safe default: if the SD
    /// card drops, the camera resumes syncing rather than drifting silently.
    pub fn ntp_enabled(&self) -> bool {
        !self.path.is_file()
    }

    pub fn disable(&self) -> std::io::Result<()> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(&self.path, b"")?;
        // SAFETY: sync(2) takes no arguments and cannot fail.
        unsafe { libc::sync() };
        Ok(())
    }

    pub fn enable(&self) -> std::io::Result<()> {
        match std::fs::remove_file(&self.path) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
        // SAFETY: sync(2) takes no arguments and cannot fail.
        unsafe { libc::sync() };
        Ok(())
    }
}
