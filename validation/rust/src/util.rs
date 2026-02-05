//! Shared utilities (log tail, artifact paths).

use anyhow::{Context, Result};
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub const MAX_TOOL_LOG_BYTES: usize = 2_000_000; // 2MB per log file (tail kept if exceeded)

pub fn tail_lossy(s: &str, max_chars: usize) -> String {
    let mut truncated = false;
    let mut seen = 0usize;
    for _ in s.chars() {
        seen += 1;
        if seen > max_chars {
            truncated = true;
            break;
        }
    }
    if !truncated {
        return s.to_string();
    }
    let mut it = s.chars().rev().take(max_chars).collect::<Vec<char>>();
    it.reverse();
    let tail: String = it.into_iter().collect();
    format!("…{}", tail)
}

pub fn write_bytes_tail(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut f = File::create(path).with_context(|| format!("create {}", path.display()))?;
    if bytes.len() <= MAX_TOOL_LOG_BYTES {
        f.write_all(bytes)
            .with_context(|| format!("write {}", path.display()))?;
        return Ok(());
    }
    let tail = &bytes[bytes.len().saturating_sub(MAX_TOOL_LOG_BYTES)..];
    writeln!(
        f,
        "[truncated: wrote last {} bytes of {}]\n",
        tail.len(),
        bytes.len()
    )
    .with_context(|| format!("write {}", path.display()))?;
    f.write_all(tail)
        .with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

pub fn run_artifacts_dir_name(timestamp_utc: &str, pid: u32) -> String {
    format!("{}_pid{}", timestamp_utc, pid)
}

#[cfg(test)]
mod tests {
    use super::{run_artifacts_dir_name, tail_lossy, write_bytes_tail, MAX_TOOL_LOG_BYTES};

    #[test]
    fn test_run_artifacts_dir_name_format() {
        assert_eq!(
            run_artifacts_dir_name("20260205T120102Z", 12345),
            "20260205T120102Z_pid12345"
        );
    }

    #[test]
    fn test_tail_lossy_no_truncation() {
        assert_eq!(tail_lossy("hello", 10), "hello");
    }

    #[test]
    fn test_tail_lossy_truncation_adds_ellipsis() {
        assert_eq!(tail_lossy("abcdef", 3), "…def");
    }

    #[test]
    fn test_tail_lossy_unicode_safe() {
        let s = "aé🙂b";
        assert_eq!(tail_lossy(s, 2), "…🙂b");
    }

    #[test]
    fn test_write_bytes_tail_truncates() {
        let path = std::env::temp_dir().join(format!(
            "rtsp_validation_tool_test_{}.log",
            std::process::id()
        ));
        let bytes = vec![b'a'; MAX_TOOL_LOG_BYTES + 16];
        write_bytes_tail(&path, &bytes).unwrap();
        let out = std::fs::read(&path).unwrap();
        assert!(out.starts_with(b"[truncated:"));
        let _ = std::fs::remove_file(&path);
    }
}
