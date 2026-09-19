//! Control channel: a Unix socket that the supervisor listens on, letting
//! external tooling query service state and request restarts without
//! telnet-logging into the camera.

use crate::supervise::{RestartHistory, SvcState};
use std::time::Instant;

/// Where `spawn_control_thread` binds the listener. The onvif-rust client in
/// `diagnostics/services.rs` connects to this exact path; keep them in sync.
pub const SOCKET_PATH: &str = "/tmp/anyka-supervisor.sock";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceStatus {
    pub name: String,
    pub state: &'static str,
    pub pid: Option<i32>,
    pub uptime_s: u64,
    pub restarts: u64,
    pub retry_in_s: u64,
}

impl ServiceStatus {
    pub fn from_svc_state(
        name: &str,
        state: &SvcState,
        hist: &RestartHistory,
        now: Instant,
    ) -> Self {
        match state {
            SvcState::Running { pid, since } => Self {
                name: name.to_owned(),
                state: "running",
                pid: Some(*pid),
                uptime_s: now.duration_since(*since).as_secs(),
                restarts: hist.len() as u64,
                retry_in_s: 0,
            },
            SvcState::Backoff { until, attempt } => Self {
                name: name.to_owned(),
                state: "backoff",
                pid: None,
                uptime_s: 0,
                restarts: *attempt as u64,
                retry_in_s: until.saturating_duration_since(now).as_secs(),
            },
        }
    }

    /// One TSV row: `name\tstate\tpid\tuptime_s\trestarts\tretry_in_s`.
    /// `pid` is `-1` when there is none.
    pub fn encode_tsv(&self) -> String {
        let pid = self.pid.map_or("-1".to_string(), |p| p.to_string());
        format!(
            "{}\t{}\t{}\t{}\t{}\t{}\n",
            self.name, self.state, pid, self.uptime_s, self.restarts, self.retry_in_s
        )
    }
}

pub fn encode_status(rows: &[ServiceStatus]) -> String {
    let mut out = String::new();
    for r in rows {
        out.push_str(&r.encode_tsv());
    }
    out.push('\n');
    out
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    Status,
    Restart(String),
}

/// Parse one line. `"status\n"` -> `Status`,
/// `"restart <name>\n"` -> `Restart(name)`. Everything else -> `None`.
/// A blank name is rejected, as is a name containing a tab.
pub fn parse_request(line: &str) -> Option<Request> {
    let line = line.trim_end_matches(['\r', '\n']);
    if line == "status" {
        return Some(Request::Status);
    }
    let name = line.strip_prefix("restart ")?.trim();
    if name.is_empty() || name.contains('\t') {
        return None;
    }
    Some(Request::Restart(name.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_from_svc_state_running() {
        let now = Instant::now();
        let since = now - Duration::from_secs(90);
        let st = SvcState::Running { pid: 42, since };
        let hist = RestartHistory::default();
        let got = ServiceStatus::from_svc_state("onvif-rust", &st, &hist, now);
        assert_eq!(got.name, "onvif-rust");
        assert_eq!(got.state, "running");
        assert_eq!(got.pid, Some(42));
        assert_eq!(got.uptime_s, 90);
        assert_eq!(got.restarts, 0);
        assert_eq!(got.retry_in_s, 0);
    }

    #[test]
    fn test_from_svc_state_backoff() {
        let now = Instant::now();
        let until = now + Duration::from_secs(12);
        let st = SvcState::Backoff { until, attempt: 3 };
        let hist = RestartHistory::default();
        let got = ServiceStatus::from_svc_state("vendor-daemon", &st, &hist, now);
        assert_eq!(got.state, "backoff");
        assert_eq!(got.pid, None);
        assert_eq!(got.uptime_s, 0);
        assert_eq!(got.restarts, 3);
        assert_eq!(got.retry_in_s, 12);
    }

    #[test]
    fn test_encode_status() {
        let rows = vec![
            ServiceStatus {
                name: "onvif-rust".into(),
                state: "running",
                pid: Some(42),
                uptime_s: 90,
                restarts: 3,
                retry_in_s: 0,
            },
            ServiceStatus {
                name: "vendor-daemon".into(),
                state: "backoff",
                pid: None,
                uptime_s: 0,
                restarts: 7,
                retry_in_s: 12,
            },
        ];
        let got = encode_status(&rows);
        assert_eq!(
            got,
            "onvif-rust\trunning\t42\t90\t3\t0\nvendor-daemon\tbackoff\t-1\t0\t7\t12\n\n"
        );
    }

    #[test]
    fn test_parse_request_status() {
        assert_eq!(parse_request("status\n"), Some(Request::Status));
    }

    #[test]
    fn test_parse_request_restart() {
        assert_eq!(
            parse_request("restart onvif-rust\n"),
            Some(Request::Restart("onvif-rust".into()))
        );
    }

    #[test]
    fn test_parse_request_rejects_blank_or_tab() {
        assert_eq!(parse_request("restart \n"), None);
        assert_eq!(parse_request("restart a\tb\n"), None);
    }

    #[test]
    fn test_parse_request_rejects_unknown() {
        assert_eq!(parse_request("nonsense\n"), None);
        assert_eq!(parse_request("\n"), None);
    }
}
