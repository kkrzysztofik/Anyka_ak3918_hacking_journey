//! Reader for `{update_root}/state/ntp.status`, written by anyka-init's
//! timesync thread.
//!
//! Display-only: a missing or malformed file means "unknown", never a
//! behaviour change. That is what makes it safe to parse, unlike the
//! deliberately unparsed `ntp.disabled` marker next to it.

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
pub struct NtpStatus {
    /// `None` until the first successful sync since boot.
    pub last_sync_unix: Option<i64>,
    pub last_server: Option<String>,
    pub last_delta_s: Option<i64>,
    pub servers: Vec<String>,
}

pub fn path(update_root: impl AsRef<Path>) -> PathBuf {
    update_root.as_ref().join("state/ntp.status")
}

/// `last_unix \t server \t delta_s \t s1,s2,…`
pub fn parse(text: &str) -> Option<NtpStatus> {
    let line = text.lines().next()?;
    let f: Vec<&str> = line.split('\t').collect();
    if f.len() < 4 {
        return None;
    }
    let unix: i64 = f[0].parse().ok()?;
    let delta: i64 = f[2].parse().ok()?;
    let synced = unix > 0;
    Some(NtpStatus {
        last_sync_unix: synced.then_some(unix),
        last_server: synced.then(|| f[1].to_owned()).filter(|s| !s.is_empty()),
        last_delta_s: synced.then_some(delta),
        servers: f[3]
            .split(',')
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect(),
    })
}

/// `None` when the file is absent (NTP disabled, or an older anyka-init).
pub fn read(update_root: impl AsRef<Path>) -> Option<NtpStatus> {
    parse(&std::fs::read_to_string(path(update_root)).ok()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_reads_a_full_status_line() {
        let got = parse("1760000000\t192.168.2.1\t-3\t192.168.2.1,pool.example\n").unwrap();
        assert_eq!(got.last_sync_unix, Some(1_760_000_000));
        assert_eq!(got.last_server.as_deref(), Some("192.168.2.1"));
        assert_eq!(got.last_delta_s, Some(-3));
        assert_eq!(got.servers, vec!["192.168.2.1", "pool.example"]);
    }

    #[test]
    fn test_parse_reads_a_never_synced_line() {
        let got = parse("0\t\t0\tpool.example\n").unwrap();
        assert_eq!(got.last_sync_unix, None);
        assert_eq!(got.last_server, None);
        assert_eq!(got.servers, vec!["pool.example"]);
    }

    #[test]
    fn test_parse_rejects_garbage_rather_than_guessing() {
        assert!(parse("").is_none());
        assert!(parse("nonsense").is_none());
        assert!(parse("abc\tx\t0\ts").is_none());
    }

    #[test]
    fn test_read_returns_none_when_the_file_is_absent() {
        assert!(read("/nonexistent/update-root").is_none());
    }
}
