//! Snapshot assembly and the previous-sample state used for rate deltas.

use std::sync::{Arc, Mutex};
use std::time::Instant;

use serde::Serialize;

use super::proc::{self, CpuTimes, NetBytes};
use super::storage;
use crate::lifecycle::health::compute_health;
use crate::platform::Platform;

const NET_IFACE: &str = "wlan0";
const STORAGE_MOUNT: &str = "/mnt";

#[derive(Debug, Clone, Copy)]
struct RawSample {
    taken_at: Instant,
    cpu: Option<CpuTimes>,
    net: Option<NetBytes>,
}

/// System and process uptime in whole seconds.
#[derive(Debug, Clone, Serialize)]
pub struct Uptime {
    pub system_s: u64,
    pub process_s: u64,
}

/// Memory utilisation snapshot in kilobytes.
#[derive(Debug, Clone, Serialize)]
pub struct Memory {
    pub total_kb: u64,
    pub used_kb: u64,
}

/// SD card capacity snapshot in kilobytes.
#[derive(Debug, Clone, Serialize)]
pub struct Storage {
    pub total_kb: u64,
    pub used_kb: u64,
}

/// Network throughput rates derived from a pair of cumulative-byte samples.
#[derive(Debug, Clone, Serialize)]
pub struct Network {
    pub rx_bps: u64,
    pub tx_bps: u64,
}

/// Health state of one service component.
#[derive(Debug, Clone, Serialize)]
pub struct Component {
    pub name: String,
    pub status: String,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Snapshot {
    pub status: String,
    pub uptime: Uptime,
    /// CPU busy percentage since the previous sample; `None` on the first call.
    pub cpu_percent: Option<f32>,
    pub memory: Option<Memory>,
    pub storage: Option<Storage>,
    /// Network throughput since the previous sample; `None` on the first call.
    pub network: Option<Network>,
    /// Age of the newest video frame from the hardware encoder, if known.
    pub stream_frame_age_ms: Option<u64>,
    pub components: Vec<Component>,
    /// Service names that failed to initialise at startup.
    pub degraded_services: Vec<String>,
}

/// Holds platform handles and the last `/proc` sample for computing deltas.
pub struct DiagnosticsState {
    started_at: Instant,
    platform: Option<Arc<dyn Platform>>,
    degraded_services: Vec<String>,
    previous: Mutex<Option<RawSample>>,
}

impl DiagnosticsState {
    /// Create a new state.  `platform` is `None` on the first boot before the
    /// hardware pipeline attaches.  `degraded_services` lists service names that
    /// failed to initialise at startup.
    pub fn new(platform: Option<Arc<dyn Platform>>, degraded_services: Vec<String>) -> Self {
        Self {
            started_at: Instant::now(),
            platform,
            degraded_services,
            previous: Mutex::new(None),
        }
    }

    /// Assemble a [`Snapshot`] from current `/proc` readings.
    ///
    /// Rate fields (`cpu_percent`, `network`) are `None` on the first call;
    /// subsequent calls produce rates from the delta since the previous call.
    pub fn snapshot(&self) -> Snapshot {
        let stat_text = read_file("/proc/stat");
        let meminfo_text = read_file("/proc/meminfo");
        let net_dev_text = read_file("/proc/net/dev");
        let uptime_text = read_file("/proc/uptime");

        let cpu_now = stat_text.as_deref().and_then(proc::parse_stat);
        let net_now = net_dev_text
            .as_deref()
            .and_then(|s| proc::parse_net_dev(s, NET_IFACE));
        let mem = meminfo_text.as_deref().and_then(proc::parse_meminfo);
        let system_s = uptime_text
            .as_deref()
            .and_then(proc::parse_uptime_secs)
            .unwrap_or(0);

        let now_sample = RawSample {
            taken_at: Instant::now(),
            cpu: cpu_now,
            net: net_now,
        };

        let prev_sample = {
            let mut guard = self.previous.lock().unwrap_or_else(|e| e.into_inner());
            guard.replace(now_sample)
        };

        let cpu_percent = prev_sample
            .as_ref()
            .and_then(|prev| proc::cpu_percent(prev.cpu?, now_sample.cpu?));

        let network = prev_sample.as_ref().and_then(|prev| {
            let elapsed = now_sample.taken_at.duration_since(prev.taken_at);
            let prev_net = prev.net?;
            let now_net = now_sample.net?;
            let rx_bps = proc::bytes_per_sec(prev_net.rx, now_net.rx, elapsed)?;
            let tx_bps = proc::bytes_per_sec(prev_net.tx, now_net.tx, elapsed)?;
            Some(Network { rx_bps, tx_bps })
        });

        let memory = mem.map(|m| Memory {
            total_kb: m.total_kb,
            used_kb: m.used_kb,
        });

        let storage = storage::storage_usage(STORAGE_MOUNT).map(|s| Storage {
            total_kb: s.total_kb,
            used_kb: s.used_kb,
        });

        // `stream_frame_age_ms` → Option<u64>; wrap in outer Option for compute_health.
        let frame_age_outer: Option<Option<u64>> =
            self.platform.as_ref().map(|p| p.stream_frame_age_ms());

        let process_s = self.started_at.elapsed().as_secs();
        let health = compute_health(
            self.started_at.elapsed(),
            frame_age_outer,
            &self.degraded_services,
        );

        let mut components: Vec<Component> = health
            .components
            .values()
            .map(|c| Component {
                name: c.name.clone(),
                status: c.status.to_string(),
                message: c.message.clone(),
            })
            .collect();
        components.sort_by(|a, b| a.name.cmp(&b.name));

        Snapshot {
            status: health.status.to_string(),
            uptime: Uptime {
                system_s,
                process_s,
            },
            cpu_percent,
            memory,
            storage,
            network,
            stream_frame_age_ms: frame_age_outer.flatten(),
            degraded_services: health.degraded_services,
            components,
        }
    }
}

/// Read a `/proc` file into a `String`, returning `None` on any I/O error.
///
/// Using `Option` rather than propagating errors means a missing or unreadable
/// file never aborts the whole diagnostics response.
fn read_file(path: &str) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_first_snapshot_has_no_rates() {
        let state = DiagnosticsState::new(None, Vec::new());
        let snap = state.snapshot();
        assert!(snap.cpu_percent.is_none());
        assert!(snap.network.is_none());
    }

    #[test]
    fn test_second_snapshot_produces_rates() {
        let state = DiagnosticsState::new(None, Vec::new());
        let _ = state.snapshot();
        std::thread::sleep(std::time::Duration::from_millis(50));
        let snap = state.snapshot();
        assert!(snap.cpu_percent.is_some(), "a second sample yields a rate");
    }

    #[test]
    fn test_snapshot_reports_process_uptime() {
        let state = DiagnosticsState::new(None, Vec::new());
        let snap = state.snapshot();
        assert!(snap.uptime.system_s >= snap.uptime.process_s);
    }

    #[test]
    fn test_snapshot_includes_startup_degraded_services() {
        let state = DiagnosticsState::new(None, vec!["PTZ".to_string()]);
        let snap = state.snapshot();
        assert_eq!(snap.status, "degraded");
        assert!(
            snap.degraded_services.contains(&"PTZ".to_string()),
            "PTZ must appear in degraded_services"
        );
    }
}
