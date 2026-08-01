//! System resource sampling, replacing `sys_monitor.sh`, plus wifi link health.

use crate::config::MonitorCfg;
use crate::netstat::{self, Action, Health, Policy};
use crate::storm::StormState;
use crate::supervisor_loop::Msg;
use crate::sys::Sys;
use crate::wifi::udhcpc_oneshot_args;
use std::sync::mpsc::Sender;
use std::time::Duration;

const BUSYBOX: &str = "/bin/busybox";

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

fn sample_link(iface: &str, probe: bool) -> Health {
    let oper = std::fs::read_to_string(format!("/sys/class/net/{iface}/operstate"))
        .map(|s| netstat::parse_operstate(&s))
        .unwrap_or(false);
    let carrier = std::fs::read_to_string(format!("/sys/class/net/{iface}/carrier"))
        .map(|s| s.trim() == "1")
        .unwrap_or(false);
    let carrier = oper && carrier;
    let route = std::fs::read_to_string("/proc/net/route")
        .ok()
        .and_then(|s| netstat::parse_default_route(&s, iface))
        .is_some();
    let reachable = if carrier && route && probe {
        if let Some(gw) = std::fs::read_to_string("/proc/net/route")
            .ok()
            .and_then(|s| netstat::parse_default_route(&s, iface))
        {
            netstat::gateway_reachable(&gw)
        } else {
            false
        }
    } else {
        // Without a probe, treat route presence as reachability so we do not
        // escalate on L3 alone when probing is disabled.
        route
    };
    Health {
        carrier,
        route,
        reachable,
    }
}

/// Sampling loop. Also owns the storm-guard reset: once this process has been
/// up longer than the configured threshold, the boot is considered good.
pub fn run(
    sys: &dyn Sys,
    cfg: &MonitorCfg,
    iface: &str,
    state_path: &str,
    reset_after: Duration,
    tx: Sender<Msg>,
) {
    let interval = Duration::from_secs(cfg.interval_sec);
    let base_policy = Policy {
        dhcp_after_ticks: cfg.wifi_dhcp_after_ticks,
        supplicant_after_ticks: cfg.wifi_supplicant_after_ticks,
        reboot_after_ticks: cfg.wifi_reboot_after_ticks,
        reboot_cap: cfg.wifi_reboot_cap,
        wifi_reboots_used: 0,
    };
    let mut reset_done = false;
    let mut ticks: u32 = 0;
    loop {
        sample();
        if !reset_done && sys.uptime() > reset_after {
            // Reset only the crash-loop counter. wifi_reboots is cleared solely
            // by a successful wifi::bring_up (B4) — uptime alone would wipe it
            // before the link ever recovers.
            let mut storm = StormState::load(state_path);
            storm.fast_reboots = 0;
            match storm.save(state_path) {
                Ok(()) => tracing::info!("boot considered good; storm-guard counter reset"),
                Err(e) => tracing::warn!(error = %e, "failed to reset storm-guard state"),
            }
            reset_done = true;
        }

        if cfg.wifi {
            let storm = StormState::load(state_path);
            let h = sample_link(iface, cfg.wifi_probe);
            if h.ok() {
                ticks = 0;
            } else {
                ticks = ticks.saturating_add(1);
                tracing::warn!(
                    carrier = h.carrier,
                    route = h.route,
                    reachable = h.reachable,
                    ticks,
                    "wifi link unhealthy"
                );
            }
            let policy = Policy {
                wifi_reboots_used: storm.wifi_reboots,
                ..base_policy
            };
            match netstat::decide(h, ticks, &policy) {
                Action::Nothing => {}
                Action::RunDhcp => {
                    tracing::warn!("no default route; re-running udhcpc");
                    let _ = sys.run_to_completion(BUSYBOX, &udhcpc_oneshot_args(iface));
                    ticks = 0;
                }
                Action::RestartSupplicant => {
                    let _ = tx.send(Msg::RestartService("wpa_supplicant".into()));
                    ticks = 0;
                }
                Action::Reboot => {
                    tracing::error!(
                        wifi_reboots = storm.wifi_reboots,
                        "wifi down past the reboot threshold; rebooting"
                    );
                    let mut storm = storm;
                    storm.wifi_reboots = storm.wifi_reboots.saturating_add(1);
                    let _ = storm.save(state_path);
                    let _ = sys.reboot();
                }
                Action::LogOnly => {
                    tracing::error!("wifi down and the reboot budget is exhausted; not rebooting");
                }
            }
        }

        std::thread::sleep(interval);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_mem_kb_prefers_available() {
        let src = "MemTotal: 100\nMemFree: 10\nMemAvailable: 40\n";
        assert_eq!(parse_mem_kb(src), Some(40));
    }

    #[test]
    fn test_parse_mem_kb_falls_back_to_free() {
        let src = "MemTotal: 100\nMemFree: 10\n";
        assert_eq!(parse_mem_kb(src), Some(10));
    }

    #[test]
    fn test_parse_loadavg_first_field() {
        assert_eq!(parse_loadavg("0.42 0.35 0.30 1/100 1234\n"), Some(0.42));
    }
}
