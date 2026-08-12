//! Parsers for the `/proc` files backing the diagnostics endpoint.
//!
//! Every function takes `&str` rather than a path so the kernel's formats stay
//! testable on the host. The camera runs a kernel old enough to lack
//! `MemAvailable`, so these formats cannot be assumed to match a modern box.

use std::time::Duration;

/// Jiffy counters from the aggregate `cpu` line of `/proc/stat`.
///
/// These are cumulative since boot, so a single reading yields the since-boot
/// average, not current load. Diff two readings to compute a live figure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpuTimes {
    pub busy: u64,
    pub total: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemInfo {
    pub total_kb: u64,
    pub used_kb: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NetBytes {
    pub rx: u64,
    pub tx: u64,
}

/// Parse the aggregate `cpu` line from `/proc/stat` contents.
///
/// Returns `None` if the first line does not start with `cpu` or has fewer
/// than 5 fields (user, nice, system, idle, iowait are the minimum).
pub fn parse_stat(contents: &str) -> Option<CpuTimes> {
    let line = contents.lines().next()?;
    let mut fields = line.split_whitespace();
    if fields.next()? != "cpu" {
        return None;
    }
    let values: Vec<u64> = fields.filter_map(|f| f.parse().ok()).collect();
    if values.len() < 5 {
        return None;
    }
    let total: u64 = values.iter().sum();
    let idle = values[3] + values[4];
    Some(CpuTimes {
        busy: total.saturating_sub(idle),
        total,
    })
}

/// Parse memory statistics from `/proc/meminfo` contents.
///
/// `used_kb` is `MemTotal - MemFree - Buffers - Cached`, which excludes
/// reclaimable page-cache. This kernel predates `MemAvailable` (Linux 3.14).
/// Returns `None` if any of the four required fields is absent.
pub fn parse_meminfo(contents: &str) -> Option<MemInfo> {
    let (mut total, mut free, mut buffers, mut cached) = (None, None, None, None);
    for line in contents.lines() {
        let Some((key, rest)) = line.split_once(':') else {
            continue;
        };
        let Some(value) = rest
            .split_whitespace()
            .next()
            .and_then(|v| v.parse::<u64>().ok())
        else {
            continue;
        };
        match key {
            "MemTotal" => total = Some(value),
            "MemFree" => free = Some(value),
            "Buffers" => buffers = Some(value),
            "Cached" => cached = Some(value),
            _ => {}
        }
    }
    let total_kb = total?;
    let used_kb = total_kb
        .saturating_sub(free?)
        .saturating_sub(buffers?)
        .saturating_sub(cached?);
    Some(MemInfo { total_kb, used_kb })
}

/// Parse RX/TX byte counters for `iface` from `/proc/net/dev` contents.
///
/// Column layout (after the colon): rx_bytes is field 0, tx_bytes is field 8.
/// Returns `None` if the interface is not present or the line is too short.
pub fn parse_net_dev(contents: &str, iface: &str) -> Option<NetBytes> {
    for line in contents.lines() {
        let Some((name, rest)) = line.split_once(':') else {
            continue;
        };
        if name.trim() != iface {
            continue;
        }
        let values: Vec<u64> = rest
            .split_whitespace()
            .filter_map(|f| f.parse().ok())
            .collect();
        if values.len() < 9 {
            return None;
        }
        return Some(NetBytes {
            rx: values[0],
            tx: values[8],
        });
    }
    None
}

/// Parse the uptime in whole seconds from `/proc/uptime` contents.
///
/// The file contains two floats: uptime and idle time. Only the first is used.
pub fn parse_uptime_secs(contents: &str) -> Option<u64> {
    contents
        .split_whitespace()
        .next()?
        .parse::<f64>()
        .ok()
        .map(|secs| secs as u64)
}

/// Busy percentage between two `/proc/stat` readings.
///
/// Returns `None` when the counters did not move forward — that means a reset,
/// and a negative delta would render as a nonsense percentage.
pub fn cpu_percent(prev: CpuTimes, now: CpuTimes) -> Option<f32> {
    let total = now.total.checked_sub(prev.total)?;
    let busy = now.busy.checked_sub(prev.busy)?;
    if total == 0 {
        return None;
    }
    Some((busy as f32 / total as f32) * 100.0)
}

/// Throughput between two cumulative byte counters.
pub fn bytes_per_sec(prev: u64, now: u64, elapsed: Duration) -> Option<u64> {
    let delta = now.checked_sub(prev)?;
    let secs = elapsed.as_secs_f64();
    if secs <= 0.0 {
        return None;
    }
    Some((delta as f64 / secs) as u64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// Captured from the AK3918 on .198, 2026-08-11.
    const STAT: &str = "cpu  4169306 0 2746808 33139786 9423 0 91014 0 0 0\ncpu0 1 2 3\n";
    const MEMINFO: &str = "MemTotal:          36540 kB\nMemFree:            2756 kB\n\
                           Buffers:            4104 kB\nCached:            18356 kB\n\
                           SwapCached:            0 kB\n";
    const NET_DEV: &str = "Inter-|   Receive                    |  Transmit\n\
         face |bytes    packets errs drop fifo frame compressed multicast|bytes\n\
         wlan0: 21863020  136885    0 5684    0     0          0         0 1261807137 986538 0 0 0 0 0 0\n\
          p2p0:       0       0    0    0    0     0          0         0        0       0 0 0 0 0 0 0\n";

    #[test]
    fn test_parse_stat_sums_busy_and_total() {
        let times = parse_stat(STAT).expect("aggregate cpu line parses");
        assert_eq!(times.total, 40_156_337);
        assert_eq!(times.busy, 7_007_128);
    }

    #[test]
    fn test_parse_stat_rejects_non_cpu_first_line() {
        assert!(parse_stat("intr 12345\n").is_none());
    }

    #[test]
    fn test_parse_meminfo_excludes_buffers_and_cache() {
        let mem = parse_meminfo(MEMINFO).expect("meminfo parses");
        assert_eq!(mem.total_kb, 36_540);
        assert_eq!(mem.used_kb, 11_324);
    }

    #[test]
    fn test_parse_meminfo_ignores_swap_cached() {
        let mem = parse_meminfo(MEMINFO).expect("meminfo parses");
        assert_eq!(mem.used_kb, 11_324);
    }

    #[test]
    fn test_parse_meminfo_missing_field_is_none() {
        assert!(parse_meminfo("MemTotal:  36540 kB\n").is_none());
    }

    #[test]
    fn test_parse_net_dev_reads_rx_and_tx() {
        let bytes = parse_net_dev(NET_DEV, "wlan0").expect("wlan0 present");
        assert_eq!(bytes.rx, 21_863_020);
        assert_eq!(bytes.tx, 1_261_807_137);
    }

    #[test]
    fn test_parse_net_dev_unknown_interface_is_none() {
        assert!(parse_net_dev(NET_DEV, "eth0").is_none());
    }

    #[test]
    fn test_parse_uptime_secs_truncates() {
        assert_eq!(parse_uptime_secs("421240.95 351066.01\n"), Some(421_240));
    }

    #[test]
    fn test_cpu_percent_from_delta() {
        let prev = CpuTimes {
            busy: 100,
            total: 1000,
        };
        let now = CpuTimes {
            busy: 150,
            total: 1200,
        };
        let pct = cpu_percent(prev, now).expect("forward delta");
        assert!((pct - 25.0).abs() < 0.01, "got {pct}");
    }

    #[test]
    fn test_cpu_percent_rejects_backwards_counter() {
        let prev = CpuTimes {
            busy: 150,
            total: 1200,
        };
        let now = CpuTimes {
            busy: 100,
            total: 1000,
        };
        assert!(cpu_percent(prev, now).is_none());
    }

    #[test]
    fn test_cpu_percent_zero_elapsed_is_none() {
        let same = CpuTimes {
            busy: 100,
            total: 1000,
        };
        assert!(cpu_percent(same, same).is_none());
    }

    #[test]
    fn test_bytes_per_sec_divides_by_elapsed() {
        let rate = bytes_per_sec(1000, 6000, Duration::from_secs(5)).expect("forward delta");
        assert_eq!(rate, 1000);
    }

    #[test]
    fn test_bytes_per_sec_rejects_counter_reset() {
        assert!(bytes_per_sec(6000, 1000, Duration::from_secs(5)).is_none());
    }
}
