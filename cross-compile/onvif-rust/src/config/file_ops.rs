//! Shared atomic file write utility for the config module.
//!
//! Used by config storage, user storage, and profile storage to ensure
//! crash-safe file writes using the temp-file → fsync → rename pattern.

use std::io::{self, Write};
use std::path::Path;

/// Write content to a file atomically.
///
/// Uses the temp-file → fsync → rename pattern to guarantee that the
/// target file is never in a partially-written state. On POSIX systems,
/// `rename()` is atomic — readers see either the old or new content.
///
/// # Arguments
///
/// * `path` - Target file path
/// * `content` - Bytes to write
/// * `mode` - Optional Unix file permissions (e.g., `Some(0o600)` for secrets)
pub fn atomic_write(path: &Path, content: &[u8], mode: Option<u32>) -> io::Result<()> {
    // Create parent directories if needed
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }

    // Write to temp file first
    let temp_path = path.with_extension("tmp");

    {
        let mut file = std::fs::File::create(&temp_path)?;
        file.write_all(content)?;
        file.sync_all()?;
    }

    // Atomic rename
    std::fs::rename(&temp_path, path)?;

    // Set file permissions if requested (Unix only)
    #[cfg(unix)]
    if let Some(mode_bits) = mode {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(path)?.permissions();
        perms.set_mode(mode_bits);
        std::fs::set_permissions(path, perms)?;
    }

    // Suppress unused variable warning on non-Unix
    #[cfg(not(unix))]
    let _ = mode;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_atomic_write_creates_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.toml");

        atomic_write(&path, b"hello = \"world\"", None).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, "hello = \"world\"");
    }

    #[test]
    fn test_atomic_write_creates_parent_dirs() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("sub").join("dir").join("test.toml");

        atomic_write(&path, b"nested = true", None).unwrap();

        assert!(path.exists());
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, "nested = true");
    }

    #[test]
    fn test_atomic_write_overwrites_existing() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.toml");

        atomic_write(&path, b"version = 1", None).unwrap();
        atomic_write(&path, b"version = 2", None).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, "version = 2");
    }

    #[test]
    fn test_atomic_write_no_temp_file_left() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.toml");

        atomic_write(&path, b"clean = true", None).unwrap();

        let temp_path = path.with_extension("tmp");
        assert!(
            !temp_path.exists(),
            "Temp file should be removed after rename"
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_atomic_write_sets_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempdir().unwrap();
        let path = dir.path().join("secret.toml");

        atomic_write(&path, b"password = \"secret\"", Some(0o600)).unwrap();

        let perms = std::fs::metadata(&path).unwrap().permissions();
        assert_eq!(perms.mode() & 0o777, 0o600);
    }
}
