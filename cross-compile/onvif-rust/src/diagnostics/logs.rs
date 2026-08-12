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
            // anyka-init's supervisor config (.deploy/anyka.toml) writes the
            // running onvif-rust process to onvif.log. onvif_rust.log is a
            // pre-cutover artifact that stopped updating 2026-08-01 and would
            // otherwise silently become the panel's default dead source.
            LogSource::OnvifRust => "/mnt/logs/onvif.log",
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

    let mut raw = Vec::new();
    file.read_to_end(&mut raw)?;

    let mut text = String::from_utf8_lossy(&raw).into_owned();

    if start > 0 {
        // Drop first (possibly partial) line.
        if let Some(nl) = text.find('\n') {
            text = text[nl + 1..].to_owned();
        } else {
            text.clear();
        }
    }

    Ok(text)
}

/// Strip ANSI/VT100 escape sequences (`\x1b[...m` SGR codes and similar).
///
/// `tracing`'s fmt subscriber and the vendor daemon's C logging both colourise
/// their output for a terminal. Serving that raw turns every line in the log
/// panel into `[2m2026-...[0m` noise around the actual message.
fn strip_ansi(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            // Consume through the final byte of the escape sequence (the
            // first char outside 0x30..=0x3f, 0x20..=0x2f). CSI sequences
            // ("\x1b[...m") end in an alphabetic byte; skip until we see one.
            for next in chars.by_ref() {
                if next.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

pub fn filter_lines(text: &str, min_level: Option<LogLevel>, limit: usize) -> Vec<String> {
    let lines: Vec<String> = text
        .lines()
        .filter(|line| {
            let level = LogLevel::of_line(line);
            min_level.is_none_or(|min| level.is_none_or(|l| l >= min))
        })
        .map(strip_ansi)
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
    fn test_tail_returns_ok_with_invalid_utf8() {
        let mut file = tempfile::NamedTempFile::new().expect("temp file");
        // Valid ASCII prefix followed by isolated invalid UTF-8 bytes.
        file.write_all(b"valid\n\xFF\xFEinvalid utf8 here\n")
            .expect("write");
        file.flush().expect("flush");
        let result = tail_bytes(file.path(), 4096);
        assert!(result.is_ok(), "invalid UTF-8 must not cause an error");
        let text = result.unwrap();
        // from_utf8_lossy replaces bad bytes with U+FFFD; valid content is preserved.
        assert!(
            text.contains("valid"),
            "valid content must be preserved; got: {text:?}"
        );
        assert!(
            text.contains(char::REPLACEMENT_CHARACTER) || text.contains("invalid utf8 here"),
            "invalid bytes must be replaced; got: {text:?}"
        );
    }

    #[test]
    fn test_tail_missing_file_is_error() {
        assert!(tail_bytes(Path::new("/nonexistent-log-for-tests"), 4096).is_err());
    }

    #[test]
    fn test_log_source_paths_are_fixed() {
        // onvif.log, not onvif_rust.log: the latter is a pre-cutover artifact
        // that stopped being written on 2026-08-01 (see .deploy/anyka.toml,
        // which points the running onvif-rust process at onvif.log).
        assert_eq!(LogSource::OnvifRust.path(), "/mnt/logs/onvif.log");
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

    #[test]
    fn test_filter_lines_strips_ansi_escapes() {
        // Real captured shape from onvif.log: tracing's colourised fmt output.
        let text = "\u{1b}[2m2026-08-01T22:33:01Z\u{1b}[0m \u{1b}[33m WARN\u{1b}[0m \u{1b}[2msrc\u{1b}[0m\u{1b}[2m:\u{1b}[0m Bye failed\n";
        let kept = filter_lines(text, None, 100);
        assert_eq!(kept, vec!["2026-08-01T22:33:01Z  WARN src: Bye failed"]);
    }

    #[test]
    fn test_filter_lines_strips_ansi_from_vendor_daemon_style_output() {
        // Real captured shape from vendor_daemon.log: raw VT100 SGR codes,
        // no tracing structure, no level token.
        let text = "\u{1b}[m\u{1b}[0;32;34m[ak_venc]: rate=160(kbps)\u{1b}[m\n";
        let kept = filter_lines(text, None, 100);
        assert_eq!(kept, vec!["[ak_venc]: rate=160(kbps)"]);
    }

    #[test]
    fn test_filter_lines_level_detection_unaffected_by_surrounding_ansi() {
        // The level token must still match while wrapped in escape codes,
        // since filtering happens on the raw line before stripping.
        let text = "\u{1b}[33m WARN\u{1b}[0m slow\n\u{1b}[32m INFO\u{1b}[0m ok\n";
        let kept = filter_lines(text, Some(LogLevel::Warn), 100);
        assert_eq!(kept, vec![" WARN slow"]);
    }
}
