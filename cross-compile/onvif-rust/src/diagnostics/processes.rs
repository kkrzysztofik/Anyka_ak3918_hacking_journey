//! `/proc` walk backing the raw process table on the diagnostics page.

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
        cpu_time_s: if hz == 0 { 0 } else { (utime + stime) / hz },
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
    out.sort_by(|a, b| b.rss_kb.cmp(&a.rss_kb));
    out
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
}
