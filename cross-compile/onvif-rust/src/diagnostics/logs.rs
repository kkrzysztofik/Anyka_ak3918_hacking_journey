//! Tailing and filtering of the on-device log files.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use serde::Deserialize;

pub const DEFAULT_TAIL_BYTES: u64 = 64 * 1024;
pub const MAX_LINES: usize = 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogSource {
    OnvifRust,
    VendorDaemon,
    AnykaInit,
    WpaSupplicant,
}

impl LogSource {
    pub fn path(self) -> &'static str {
        match self {
            LogSource::OnvifRust => "/mnt/logs/onvif_rust.log",
            LogSource::VendorDaemon => "/mnt/logs/vendor_daemon.log",
            LogSource::AnykaInit => "/mnt/logs/anyka-init.log",
            LogSource::WpaSupplicant => "/mnt/logs/wpa_supplicant.log",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    fn of_line(line: &str) -> Option<Self> {
        for (token, level) in [
            ("ERROR", LogLevel::Error),
            ("WARN", LogLevel::Warn),
            ("INFO", LogLevel::Info),
            ("DEBUG", LogLevel::Debug),
            ("TRACE", LogLevel::Trace),
        ] {
            if line.contains(token) {
                return Some(level);
            }
        }
        None
    }
}

/// Read up to `budget` bytes from the tail of `path`.
///
/// When the file is larger than `budget`, the first (potentially partial)
/// line is dropped so callers always receive complete lines.
pub fn tail_bytes(path: &Path, budget: u64) -> std::io::Result<String> {
    let mut file = File::open(path)?;
    let file_len = file.metadata()?.len();

    let start = file_len.saturating_sub(budget);
    if start > 0 {
        file.seek(SeekFrom::Start(start))?;
    }

    let mut buf = String::new();
    file.read_to_string(&mut buf)?;

    if start > 0 {
        // Drop first (possibly partial) line.
        if let Some(nl) = buf.find('\n') {
            buf = buf[nl + 1..].to_owned();
        } else {
            buf.clear();
        }
    }

    Ok(buf)
}

pub fn filter_lines(text: &str, min_level: Option<LogLevel>, limit: usize) -> Vec<&str> {
    let lines: Vec<&str> = text
        .lines()
        .filter(|line| {
            let level = LogLevel::of_line(line);
            min_level.is_none_or(|min| level.is_none_or(|l| l >= min))
        })
        .collect();

    let start = lines.len().saturating_sub(limit);
    lines[start..].to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn temp_log(contents: &str) -> tempfile::NamedTempFile {
        let mut file = tempfile::NamedTempFile::new().expect("temp file");
        file.write_all(contents.as_bytes()).expect("write");
        file.flush().expect("flush");
        file
    }

    #[test]
    fn test_tail_returns_whole_file_when_under_budget() {
        let file = temp_log("one\ntwo\nthree\n");
        let text = tail_bytes(file.path(), 4096).expect("tail");
        assert_eq!(text, "one\ntwo\nthree\n");
    }

    #[test]
    fn test_tail_drops_partial_first_line() {
        let file = temp_log("aaaaaaaaaa\nbbbb\ncccc\n");
        let text = tail_bytes(file.path(), 12).expect("tail");
        assert!(
            !text.contains('a'),
            "partial line must be dropped, got {text:?}"
        );
        assert!(text.contains("bbbb"));
    }

    #[test]
    fn test_tail_missing_file_is_error() {
        assert!(tail_bytes(Path::new("/nonexistent-log-for-tests"), 4096).is_err());
    }

    #[test]
    fn test_log_source_paths_are_fixed() {
        assert_eq!(LogSource::OnvifRust.path(), "/mnt/logs/onvif_rust.log");
        assert_eq!(
            LogSource::VendorDaemon.path(),
            "/mnt/logs/vendor_daemon.log"
        );
    }

    #[test]
    fn test_filter_lines_by_level_keeps_matching_and_worse() {
        let text = "INFO started\nWARN slow\nERROR broke\nDEBUG noise\n";
        let kept = filter_lines(text, Some(LogLevel::Warn), 100);
        assert_eq!(kept, vec!["WARN slow", "ERROR broke"]);
    }

    #[test]
    fn test_filter_lines_caps_at_requested_count() {
        let text = "INFO a\nINFO b\nINFO c\n";
        let kept = filter_lines(text, None, 2);
        assert_eq!(kept, vec!["INFO b", "INFO c"]);
    }
}
