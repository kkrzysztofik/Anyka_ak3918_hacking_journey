//! System resource sampling, replacing `sys_monitor.sh`.

use crate::storm::StormState;
use crate::sys::Sys;
use std::time::Duration;

pub fn parse_mem_kb(meminfo: &str) -> Option<u64> {
    let field = |name: &str| -> Option<u64> {
        meminfo
            .lines()
            .find(|l| l.starts_with(name))
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|v| v.parse().ok())
    };
    field("MemAvailable:").or_else(|| field("MemFree:"))
}

pub fn parse_loadavg(src: &str) -> Option<f32> {
    src.split_whitespace().next().and_then(|v| v.parse().ok())
}

fn sample() {
    let mem = std::fs::read_to_string("/proc/meminfo")
        .ok()
        .and_then(|s| parse_mem_kb(&s));
    let load = std::fs::read_to_string("/proc/loadavg")
        .ok()
        .and_then(|s| parse_loadavg(&s));
    tracing::info!(mem_avail_kb = mem, load1 = load, "sys");
}

/// Sampling loop. Also owns the storm-guard reset: once this process has been
/// up longer than the configured threshold, the boot is considered good.
pub fn run(sys: &dyn Sys, interval: Duration, state_path: &str, reset_after: Duration) {
    let mut reset_done = false;
    loop {
        sample();
        if !reset_done && sys.uptime() > reset_after {
            match (StormState { fast_reboots: 0 }).save(state_path) {
                Ok(()) => tracing::info!("boot considered good; storm-guard counter reset"),
                Err(e) => tracing::warn!(error = %e, "failed to reset storm-guard state"),
            }
            reset_done = true;
        }
        std::thread::sleep(interval);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MEMINFO: &str = "\
MemTotal:          36864 kB
MemFree:            4096 kB
MemAvailable:       8192 kB
Buffers:             512 kB
";

    #[test]
    fn test_parse_mem_available_preferred_over_free() {
        assert_eq!(parse_mem_kb(MEMINFO), Some(8192));
    }

    #[test]
    fn test_parse_mem_falls_back_to_free_when_available_absent() {
        let src = "MemTotal: 36864 kB\nMemFree: 4096 kB\n";
        assert_eq!(parse_mem_kb(src), Some(4096));
    }

    #[test]
    fn test_parse_mem_returns_none_on_garbage() {
        assert_eq!(parse_mem_kb(""), None);
        assert_eq!(parse_mem_kb("MemFree: not-a-number kB"), None);
    }

    #[test]
    fn test_parse_loadavg_takes_first_field() {
        assert_eq!(parse_loadavg("0.52 0.31 0.19 1/84 1234"), Some(0.52));
        assert_eq!(parse_loadavg(""), None);
    }
}
