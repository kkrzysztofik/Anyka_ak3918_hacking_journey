//! `/proc` walk backing the raw process table on the diagnostics page.

use axum::Json;
use axum::extract::Path as AxumPath;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Process {
    pub pid: i32,
    pub ppid: i32,
    pub comm: String,
    /// Single-letter state from `/proc/[pid]/stat`: R, S, D, Z, T.
    pub state: String,
    pub rss_kb: u64,
    /// Cumulative utime+stime in seconds.
    ///
    /// Deliberately not a percentage: that needs two samples, per-pid delta
    /// bookkeeping and pid-reuse handling, and answers no question this one
    /// does not — "has this been burning CPU since boot?".
    pub cpu_time_s: u64,
}

/// Parse one `/proc/[pid]/stat` line. `hz` is `sysconf(_SC_CLK_TCK)`.
///
/// Fields are counted from the last `)`, never from the left: `comm` is
/// parenthesised and may contain spaces and parens of its own (`kworker/0:1
/// (x)`), so a naive split shifts every field after it.
pub fn parse_stat(line: &str, hz: u64) -> Option<Process> {
    let open = line.find('(')?;
    let close = line.rfind(')')?;
    if close < open {
        return None;
    }
    let pid: i32 = line.get(..open)?.trim().parse().ok()?;
    let comm = line.get(open + 1..close)?.to_owned();

    // Fields after comm, 0-indexed: 0=state, 1=ppid, 11=utime, 12=stime.
    let rest: Vec<&str> = line.get(close + 1..)?.split_whitespace().collect();
    let state = (*rest.first()?).to_owned();
    let ppid: i32 = rest.get(1)?.parse().ok()?;
    let utime: u64 = rest.get(11)?.parse().ok()?;
    let stime: u64 = rest.get(12)?.parse().ok()?;

    Some(Process {
        pid,
        ppid,
        comm,
        state,
        rss_kb: 0,
        cpu_time_s: (utime + stime).checked_div(hz).unwrap_or(0),
    })
}

/// Pull `VmRSS` out of `/proc/[pid]/status`. Kernel threads have no such line;
/// that is legitimately zero, not a failure.
pub fn parse_status_rss(text: &str) -> u64 {
    text.lines()
        .find_map(|l| l.strip_prefix("VmRSS:"))
        .and_then(|v| v.split_whitespace().next())
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
}

/// Walk `/proc`, RSS-heaviest first.
///
/// Blocking. Callers must wrap the WHOLE walk in a single `spawn_blocking` —
/// never one per file. Each yielding await costs roughly a scheduler quantum
/// (~12 ms) on this camera, which across ~50 processes turns a
/// few-millisecond walk into most of a second.
pub fn collect() -> Vec<Process> {
    // SAFETY: sysconf with a constant name has no preconditions and no
    // side effects; it only reads a kernel-provided constant.
    let hz = unsafe { libc::sysconf(libc::_SC_CLK_TCK) }.max(1) as u64;

    let Ok(entries) = std::fs::read_dir("/proc") else {
        return Vec::new();
    };
    let mut out: Vec<Process> = entries
        .flatten()
        .filter_map(|e| {
            let name = e.file_name();
            let name = name.to_str()?;
            // Only numeric entries are processes.
            name.parse::<i32>().ok()?;
            let stat = std::fs::read_to_string(e.path().join("stat")).ok()?;
            let mut p = parse_stat(&stat, hz)?;
            // A process that exits mid-walk is normal; treat it as 0 RSS
            // rather than dropping the row we already parsed.
            p.rss_kb = std::fs::read_to_string(e.path().join("status"))
                .map(|s| parse_status_rss(&s))
                .unwrap_or(0);
            Some(p)
        })
        .collect();
    out.sort_by_key(|p| std::cmp::Reverse(p.rss_kb));
    out
}

#[derive(Debug, Clone, Serialize)]
pub struct ProcessesResponse {
    /// `None` when the supervisor control socket is unreachable. The raw
    /// table still renders; the UI degrades to a note.
    pub supervised: Option<Vec<crate::diagnostics::services::ServiceStatus>>,
    pub processes: Vec<Process>,
}

/// GET /api/processes
pub async fn handle_processes() -> impl IntoResponse {
    // One spawn_blocking for BOTH the socket round-trip and the whole /proc
    // walk — see the note on `collect`.
    let result = tokio::task::spawn_blocking(|| {
        let supervised = crate::diagnostics::services::query_status(std::path::Path::new(
            crate::diagnostics::services::SOCKET_PATH,
        ));
        let processes = collect();
        // telnetd is the one row that is not a supervised service; it is
        // synthesized from the raw scan so the UI can show and toggle it.
        let supervised = supervised.map(|mut rows| {
            insert_telnet_row(&mut rows, &processes);
            rows
        });
        ProcessesResponse {
            supervised,
            processes,
        }
    })
    .await;

    match result {
        Ok(body) => Json(body).into_response(),
        Err(e) => {
            tracing::warn!(error = %e, "process listing task failed");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// `telnetd` is not a supervised service — the P0 wrapper starts it before
/// anyka-init exists and a kill must stay final (anyka-init's
/// `handle_toggle_telnet` owns the switch) — but the Diagnostics table wants
/// it as a row with a toggle. Synthesize it from the raw scan: truth, not
/// intent. `running` with a pid when a telnetd process exists, `disabled`
/// when none does. Uptime/restarts are zero by construction: this is not a
/// supervised service, it has no backoff history.
fn insert_telnet_row(
    rows: &mut Vec<crate::diagnostics::services::ServiceStatus>,
    processes: &[Process],
) {
    rows.retain(|r| r.name != "telnetd");
    let telnet = processes.iter().find(|p| p.comm == "telnetd");
    let row = crate::diagnostics::services::ServiceStatus {
        name: "telnetd".to_string(),
        state: if telnet.is_some() {
            "running".to_string()
        } else {
            "disabled".to_string()
        },
        pid: telnet.map(|p| p.pid),
        uptime_s: 0,
        restarts: 0,
        retry_in_s: 0,
    };
    // Sorted position: the socket rows arrive in BTreeMap (name) order.
    let pos = rows
        .iter()
        .position(|r| r.name.as_str() > "telnetd")
        .unwrap_or(rows.len());
    rows.insert(pos, row);
}

/// POST /api/services/{name}/restart
///
/// 202, not 200: the supervisor SIGTERMs and the normal exit path restarts
/// under backoff. "Accepted" is the honest status — nothing here waits for the
/// service to come back.
pub async fn handle_restart_service(AxumPath(name): AxumPath<String>) -> impl IntoResponse {
    let sock = std::path::Path::new(crate::diagnostics::services::SOCKET_PATH);

    let known = tokio::task::spawn_blocking(move || {
        // Validate the name against the live snapshot so an unknown service is
        // a 404 rather than a silently-ignored "ok". Costs one extra
        // round-trip on a path a human clicks, which is free.
        crate::diagnostics::services::query_status(sock)
            .map(|rows| {
                rows.iter()
                    .find(|r| r.name == name)
                    .map(|r| r.state.clone())
            })
            .map(|state| (state, name))
    })
    .await;

    let Ok(Some((state, name))) = known else {
        return (StatusCode::SERVICE_UNAVAILABLE, "supervisor unreachable").into_response();
    };
    let Some(state) = state else {
        return (StatusCode::NOT_FOUND, "unknown service").into_response();
    };
    if state == "disabled" {
        // The supervisor would accept this and do nothing: there is no process
        // to signal. Enable it instead.
        return (StatusCode::CONFLICT, "service is disabled").into_response();
    }

    let accepted = tokio::task::spawn_blocking(move || {
        crate::diagnostics::services::request_restart(sock, &name)
    })
    .await;

    match accepted {
        Ok(true) => StatusCode::ACCEPTED.into_response(),
        _ => (StatusCode::SERVICE_UNAVAILABLE, "restart not accepted").into_response(),
    }
}

/// Shared core for the enable/disable routes.
///
/// 202, not 200: accepted, not confirmed — the supervisor SIGTERMs (disable)
/// or starts under backoff (enable) on its own schedule.
async fn toggle_service(name: String, enabled: bool) -> impl IntoResponse {
    use crate::diagnostics::services::{SOCKET_PATH, ToggleReply, request_toggle};
    let sock = std::path::Path::new(SOCKET_PATH);

    // One round-trip. The restart route pre-flights `query_status` because the
    // supervisor answers every `restart` with "ok", even for a name it has
    // never heard of — the snapshot is its only route to a 404. The toggle
    // protocol carries its own `unknown`, so a second round-trip here would
    // only re-ask a question the first one already answers.
    let reply = tokio::task::spawn_blocking(move || request_toggle(sock, &name, enabled)).await;

    match reply {
        Ok(ToggleReply::Accepted) => StatusCode::ACCEPTED.into_response(),
        // 202 as well: accepted, outcome unconfirmed. The supervisor persists
        // and applies before it replies, so a slow answer is not a failure —
        // calling it one would report "nothing changed" for a change that did.
        Ok(ToggleReply::Pending) => (
            StatusCode::ACCEPTED,
            "accepted; the supervisor did not confirm in time — re-check the service list",
        )
            .into_response(),
        Ok(ToggleReply::Unknown) => {
            (StatusCode::NOT_FOUND, "unknown or non-toggleable service").into_response()
        }
        // Two distinct 503 bodies: on a camera with no shell, the body is the
        // only thing separating "no supervisor" from "supervisor said no".
        Ok(ToggleReply::Unreachable) | Err(_) => {
            (StatusCode::SERVICE_UNAVAILABLE, "supervisor unreachable").into_response()
        }
        Ok(ToggleReply::Error) => (
            StatusCode::SERVICE_UNAVAILABLE,
            "supervisor rejected the toggle; nothing changed",
        )
            .into_response(),
    }
}

/// POST /api/services/{name}/enable and .../disable
///
/// One route for both: they differ by a bool. A path segment cannot be
/// constrained to two words, so anything else is a 404 here — the same answer
/// the route would have given when it did not exist.
pub async fn handle_toggle_service(
    AxumPath((name, action)): AxumPath<(String, String)>,
) -> impl IntoResponse {
    match action.as_str() {
        "enable" => toggle_service(name, true).await.into_response(),
        "disable" => toggle_service(name, false).await.into_response(),
        _ => (StatusCode::NOT_FOUND, "unknown action").into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A real line. Note `comm` is parenthesised and may itself contain
    /// spaces and parens, so fields must be counted from the LAST ')'.
    const STAT: &str = "1234 (onvif-rust.bin) S 1 1234 1234 0 -1 4194560 812 0 0 0 150 30 0 0 20 0 9 0 5678 24576000 1500 4294967295 1 2 3 4 5 6 7";

    #[test]
    fn test_parse_stat_extracts_pid_comm_state_and_ppid() {
        let got = parse_stat(STAT, 100).expect("a well-formed stat line must parse");
        assert_eq!(got.pid, 1234);
        assert_eq!(got.comm, "onvif-rust.bin");
        assert_eq!(got.state, "S");
        assert_eq!(got.ppid, 1);
    }

    fn row(name: &str) -> crate::diagnostics::services::ServiceStatus {
        crate::diagnostics::services::ServiceStatus {
            name: name.to_string(),
            state: "running".to_string(),
            pid: Some(1),
            uptime_s: 10,
            restarts: 0,
            retry_in_s: 0,
        }
    }

    #[test]
    fn test_insert_telnet_row_running_is_inserted_in_sorted_position() {
        let mut rows = vec![row("onvif"), row("snmp"), row("udhcpc")];
        let procs = vec![Process {
            pid: 42,
            ppid: 1,
            comm: "telnetd".to_string(),
            state: "S".to_string(),
            rss_kb: 0,
            cpu_time_s: 0,
        }];
        insert_telnet_row(&mut rows, &procs);
        let names: Vec<&str> = rows.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, vec!["onvif", "snmp", "telnetd", "udhcpc"]);
        let t = rows
            .iter()
            .find(|r| r.name == "telnetd")
            .expect("telnetd row");
        assert_eq!(t.state, "running");
        assert_eq!(t.pid, Some(42));
    }

    #[test]
    fn test_insert_telnet_row_absent_process_is_disabled_with_null_pid() {
        let mut rows = vec![row("snmp")];
        let procs = vec![Process {
            pid: 7,
            ppid: 1,
            comm: "ntpd".to_string(),
            state: "S".to_string(),
            rss_kb: 0,
            cpu_time_s: 0,
        }];
        insert_telnet_row(&mut rows, &procs);
        let t = rows
            .iter()
            .find(|r| r.name == "telnetd")
            .expect("telnetd row");
        assert_eq!(t.state, "disabled");
        assert_eq!(t.pid, None);
    }

    #[test]
    fn test_insert_telnet_row_replaces_a_stale_synthetic_row() {
        let mut rows = vec![row("snmp")];
        insert_telnet_row(&mut rows, &[]);
        insert_telnet_row(&mut rows, &[]);
        assert_eq!(rows.iter().filter(|r| r.name == "telnetd").count(), 1);
    }

    /// utime=150 + stime=30 jiffies at 100 Hz = 1 second.
    #[test]
    fn test_parse_stat_converts_jiffies_to_seconds() {
        let got = parse_stat(STAT, 100).unwrap();
        assert_eq!(got.cpu_time_s, 1);
    }

    /// A kernel thread's comm contains spaces and parens. Counting fields from
    /// the left would shift every subsequent field and report a bogus state.
    #[test]
    fn test_parse_stat_handles_a_comm_containing_spaces_and_parens() {
        let line = "7 (kworker/0:1 (x)) D 2 0 0 0 -1 0 0 0 0 0 5 5 0 0 20 0 1 0 9 0 0";
        let got = parse_stat(line, 100).expect("comm with parens must still parse");
        assert_eq!(got.comm, "kworker/0:1 (x)");
        assert_eq!(got.state, "D");
        assert_eq!(got.ppid, 2);
    }

    #[test]
    fn test_parse_stat_rejects_a_truncated_line() {
        assert!(parse_stat("1234 (x) S", 100).is_none());
        assert!(parse_stat("garbage", 100).is_none());
    }

    #[test]
    fn test_parse_status_rss_reads_vmrss_in_kb() {
        let status = "Name:\tonvif\nState:\tS (sleeping)\nVmRSS:\t  4096 kB\nThreads:\t9\n";
        assert_eq!(parse_status_rss(status), 4096);
    }

    /// Kernel threads have no VmRSS line at all; that is zero, not an error.
    #[test]
    fn test_parse_status_rss_without_a_vmrss_line_is_zero() {
        assert_eq!(parse_status_rss("Name:\tkworker\nState:\tS\n"), 0);
    }

    /// The socket path is a constant, so the testable axis on a dev host is
    /// the no-supervisor path. Pin that it degrades to 503 and never 500.
    #[tokio::test]
    async fn test_toggle_service_without_a_supervisor_is_503() {
        let resp = toggle_service("snmp".to_string(), true)
            .await
            .into_response();
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    }
}
