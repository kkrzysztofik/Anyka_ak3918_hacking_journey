//! Filesystem capacity for the SD card mount.

use std::ffi::CString;

/// Used and total capacity of a mount, in kilobytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StorageUsage {
    pub total_kb: u64,
    pub used_kb: u64,
}

/// Stat a mount point for capacity.
///
/// Uses `statvfs` rather than shelling out to `df`: spawning a process to read
/// two numbers is not something to do every five seconds on a 199 BogoMIPS core.
///
/// Returns `None` if the path cannot be stat-ed, e.g. the SD card is absent.
pub fn storage_usage(mount: &str) -> Option<StorageUsage> {
    let path = CString::new(mount).ok()?;
    // SAFETY: `statvfs` is POD; all-zeroes is a valid inhabitant.
    let mut stats: libc::statvfs = unsafe { std::mem::zeroed() };

    // SAFETY: `path` is a valid NUL-terminated C string that outlives the call,
    // and `stats` is a live, correctly-sized, zero-initialised statvfs.
    let rc = unsafe { libc::statvfs(path.as_ptr(), &mut stats) };
    if rc != 0 {
        return None;
    }

    // f_frsize is the fragment size the block counts are in; some filesystems
    // report 0 and expect f_bsize to be used instead.
    let frsize = match stats.f_frsize as u64 {
        0 => stats.f_bsize as u64,
        n => n,
    };
    let to_kb = |blocks: u64| blocks.checked_mul(frsize).map(|bytes| bytes / 1024);
    let total_kb = to_kb(stats.f_blocks as u64)?;
    let free_kb = to_kb(stats.f_bfree as u64)?;
    Some(StorageUsage {
        total_kb,
        used_kb: total_kb.saturating_sub(free_kb),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_usage_of_root_is_plausible() {
        let usage = storage_usage("/").expect("root filesystem is stat-able");
        assert!(usage.total_kb > 0, "a mounted filesystem has capacity");
        assert!(usage.used_kb <= usage.total_kb, "used cannot exceed total");
    }

    #[test]
    fn test_storage_usage_missing_path_is_none() {
        assert!(storage_usage("/nonexistent-mount-point-for-tests").is_none());
    }
}
