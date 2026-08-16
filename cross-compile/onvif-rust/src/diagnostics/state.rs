//! Snapshot assembly and the previous-sample state used for rate deltas.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use serde::Serialize;

use super::proc::{self, CpuTimes, NetBytes};
use super::storage;
use crate::lifecycle::health::compute_health;
use crate::platform::{Platform, PtzDiagnostics, VisionDiagnostics};

const NET_IFACE: &str = "wlan0";
const STORAGE_MOUNT: &str = "/mnt";
/// Ignore rate deltas shorter than this — near-simultaneous polls inflate throughput.
const MIN_DELTA: std::time::Duration = std::time::Duration::from_millis(200);

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
    /// `git describe` at build time, matching `GetDeviceInformation`'s
    /// `FirmwareVersion`.
    pub firmware_version: String,
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
    /// Live vision/night-mode diagnostics from the imaging pipeline, if available.
    pub vision: Option<VisionDiagnostics>,
    /// PTZ subsystem state, including bring-up failures. `None` when the platform does
    /// not report PTZ at all.
    pub ptz: Option<PtzDiagnostics>,
}

/// Holds platform handles and the last `/proc` sample for computing deltas.
///
/// Constructed during the Network startup phase, before the Discovery and
/// Streaming phases run — the HTTP server (and this state, attached to it)
/// must already be listening by then. That means the `degraded_services` and
/// `streaming_active` this struct starts with are provisional: Discovery and
/// Streaming can still fail after construction. [`finalize_startup`] is
/// called once, after `App::new()` completes every phase, so
/// `/api/diagnostics` ends up reflecting the same final state as
/// `App::health()` rather than a snapshot frozen mid-startup.
///
/// [`finalize_startup`]: DiagnosticsState::finalize_startup
pub struct DiagnosticsState {
    started_at: Instant,
    platform: Option<Arc<dyn Platform>>,
    degraded_services: Mutex<Vec<String>>,
    /// Mirrors `App::streaming_service.is_some()`. Gates `stream_frame_age_ms`
    /// the same way `App::health()` does: a platform can exist (e.g. for
    /// imaging control) while streaming itself is disabled in config, in
    /// which case there is no stream to be stalled and reporting one as
    /// degraded would be a false alarm.
    streaming_active: AtomicBool,
    previous: Mutex<Option<RawSample>>,
}

impl DiagnosticsState {
    /// Create a new state.  `platform` is `None` on the first boot before the
    /// hardware pipeline attaches.  `degraded_services` lists service names
    /// known to have failed to initialise so far — provisional until
    /// [`finalize_startup`](Self::finalize_startup) is called.
    pub fn new(
        started_at: Instant,
        platform: Option<Arc<dyn Platform>>,
        degraded_services: Vec<String>,
    ) -> Self {
        Self {
            started_at,
            platform,
            degraded_services: Mutex::new(degraded_services),
            streaming_active: AtomicBool::new(false),
            previous: Mutex::new(None),
        }
    }

    /// Record the outcome of the startup phases (Discovery, Streaming) that
    /// run after this state is constructed. Called exactly once, from
    /// `App::new()`, right after those phases complete.
    pub fn finalize_startup(&self, degraded_services: Vec<String>, streaming_active: bool) {
        *self
            .degraded_services
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = degraded_services;
        self.streaming_active
            .store(streaming_active, Ordering::Relaxed);
    }

    /// Assemble a [`Snapshot`] from current `/proc` readings.
    ///
    /// Rate fields (`cpu_percent`, `network`) are `None` on the first call;
    /// subsequent calls produce rates from the delta since the previous call.
    ///
    /// `vision` is populated asynchronously from the imaging pipeline when a
    /// platform with imaging control support is attached.
    pub async fn snapshot(&self) -> Snapshot {
        let readings = match tokio::task::spawn_blocking(collect_proc_readings).await {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!(error = %e, "diagnostics proc sampling task failed");
                ProcReadings::empty()
            }
        };

        let cpu_now = readings.stat_text.as_deref().and_then(proc::parse_stat);
        let net_now = readings
            .net_dev_text
            .as_deref()
            .and_then(|s| proc::parse_net_dev(s, NET_IFACE));
        let mem = readings
            .meminfo_text
            .as_deref()
            .and_then(proc::parse_meminfo);
        let system_s = readings
            .uptime_text
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
            let prev = guard.as_ref().copied();
            let too_soon =
                prev.is_some_and(|p| now_sample.taken_at.duration_since(p.taken_at) < MIN_DELTA);
            if !too_soon {
                guard.replace(now_sample);
            }
            prev
        };

        let cpu_percent = prev_sample
            .as_ref()
            .and_then(|prev| proc::cpu_percent(prev.cpu?, now_sample.cpu?));

        let network = prev_sample.as_ref().and_then(|prev| {
            let elapsed = now_sample.taken_at.duration_since(prev.taken_at);
            if elapsed < MIN_DELTA {
                return None;
            }
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

        let storage = readings.storage.map(|s| Storage {
            total_kb: s.total_kb,
            used_kb: s.used_kb,
        });

        // Outer Option is Some iff streaming is actually active, matching
        // App::health()'s `self.streaming_service.as_ref().map(...)` gate.
        // A platform can exist for imaging control with streaming disabled in
        // config; in that case there is no stream to report on at all.
        let frame_age_outer: Option<Option<u64>> = if self.streaming_active.load(Ordering::Relaxed)
        {
            Some(self.platform.as_ref().and_then(|p| p.stream_frame_age_ms()))
        } else {
            None
        };

        let process_s = self.started_at.elapsed().as_secs();
        let degraded_services = self
            .degraded_services
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let health = compute_health(
            self.started_at.elapsed(),
            frame_age_outer,
            &degraded_services,
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

        let vision = match self.platform.as_ref().and_then(|p| p.imaging_control()) {
            Some(imaging) => match imaging.vision_diagnostics().await {
                Ok(v) => v,
                Err(e) => {
                    tracing::warn!(error = %e, "vision diagnostics unavailable");
                    None
                }
            },
            None => None,
        };

        let ptz = self.platform.as_ref().and_then(|p| p.ptz_diagnostics());

        Snapshot {
            status: health.status.to_string(),
            firmware_version: crate::build_version().to_string(),
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
            vision,
            ptz,
        }
    }
}

/// Blocking `/proc` and mount reads collected off the async executor.
struct ProcReadings {
    stat_text: Option<String>,
    meminfo_text: Option<String>,
    net_dev_text: Option<String>,
    uptime_text: Option<String>,
    storage: Option<storage::StorageUsage>,
}

impl ProcReadings {
    fn empty() -> Self {
        Self {
            stat_text: None,
            meminfo_text: None,
            net_dev_text: None,
            uptime_text: None,
            storage: None,
        }
    }
}

fn collect_proc_readings() -> ProcReadings {
    ProcReadings {
        stat_text: read_file("/proc/stat"),
        meminfo_text: read_file("/proc/meminfo"),
        net_dev_text: read_file("/proc/net/dev"),
        uptime_text: read_file("/proc/uptime"),
        storage: storage::storage_usage(STORAGE_MOUNT),
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

    #[tokio::test]
    async fn test_snapshot_first_call_has_no_rates() {
        let state = DiagnosticsState::new(Instant::now(), None, Vec::new());
        let snap = state.snapshot().await;
        assert!(snap.cpu_percent.is_none());
        assert!(snap.network.is_none());
    }

    #[tokio::test]
    async fn test_snapshot_reports_firmware_version() {
        let state = DiagnosticsState::new(Instant::now(), None, Vec::new());
        let snap = state.snapshot().await;
        assert_eq!(
            snap.firmware_version,
            crate::build_version(),
            "the diagnostics snapshot must carry the same build identity as \
             GetDeviceInformation's FirmwareVersion"
        );
    }

    #[tokio::test]
    async fn test_snapshot_second_call_produces_rates() {
        if !std::path::Path::new("/proc/stat").exists() {
            return;
        }
        let state = DiagnosticsState::new(Instant::now(), None, Vec::new());
        let _ = state.snapshot().await;
        std::thread::sleep(std::time::Duration::from_millis(50));
        let snap = state.snapshot().await;
        assert!(snap.cpu_percent.is_some(), "a second sample yields a rate");
    }

    #[tokio::test]
    async fn test_snapshot_reports_process_uptime() {
        let state = DiagnosticsState::new(Instant::now(), None, Vec::new());
        let snap = state.snapshot().await;
        assert!(snap.uptime.system_s >= snap.uptime.process_s);
    }

    #[tokio::test]
    async fn test_snapshot_includes_startup_degraded_services() {
        let state = DiagnosticsState::new(Instant::now(), None, vec!["PTZ".to_string()]);
        let snap = state.snapshot().await;
        assert_eq!(snap.status, "degraded");
        assert!(
            snap.degraded_services.contains(&"PTZ".to_string()),
            "PTZ must appear in degraded_services"
        );
    }

    #[tokio::test]
    async fn test_snapshot_vision_none_without_platform() {
        let state = DiagnosticsState::new(Instant::now(), None, Vec::new());
        let snap = state.snapshot().await;
        assert!(snap.vision.is_none());
    }

    #[tokio::test]
    async fn test_snapshot_without_platform_ptz_is_none() {
        let state = DiagnosticsState::new(Instant::now(), None, Vec::new());
        assert!(state.snapshot().await.ptz.is_none());
    }

    #[tokio::test]
    async fn test_snapshot_ptz_init_error_is_carried() {
        use crate::platform::MockPlatform;

        let mut platform = MockPlatform::new();
        platform.expect_ptz_diagnostics().returning(|| {
            Some(crate::platform::PtzDiagnostics::failed(
                "open /dev/ak-motor0: errno 19".to_string(),
            ))
        });
        platform.expect_imaging_control().returning(|| None);

        let state = DiagnosticsState::new(
            Instant::now(),
            Some(std::sync::Arc::new(platform)),
            Vec::new(),
        );
        let snap = state.snapshot().await;
        let ptz = snap
            .ptz
            .expect("a platform reporting PTZ must reach the snapshot");
        assert_eq!(
            ptz.init_error.as_deref(),
            Some("open /dev/ak-motor0: errno 19")
        );
    }

    // ── finalize_startup ─────────────────────────────────────────────────
    //
    // These cover the fix for two bugs found validating the diagnostics
    // rollout: (1) degraded_services was captured before the Discovery and
    // Streaming startup phases ran, so a stream that failed to start was
    // reported healthy; (2) the stream-health gate used platform-presence
    // instead of streaming_service-presence, so streaming_enabled=false
    // configs could report a falsely degraded stream.

    #[tokio::test]
    async fn test_finalize_startup_replaces_initial_degraded_services() {
        let state = DiagnosticsState::new(Instant::now(), None, vec!["A".to_string()]);
        state.finalize_startup(vec!["B".to_string()], false);
        let snap = state.snapshot().await;
        assert_eq!(snap.degraded_services, vec!["B".to_string()]);
    }

    #[tokio::test]
    async fn test_before_finalize_streaming_inactive_frame_age_is_none() {
        // Mirrors the constructor-only state a request could observe in the
        // brief window between the HTTP server binding (Network phase) and
        // Discovery/Streaming completing.
        let state = DiagnosticsState::new(Instant::now(), None, Vec::new());
        let snap = state.snapshot().await;
        assert!(snap.stream_frame_age_ms.is_none());
        assert_eq!(snap.status, "healthy");
    }

    #[tokio::test]
    async fn test_finalize_startup_streaming_active_without_platform_reports_degraded() {
        // App::health() gates on `streaming_service.is_some()`, not on
        // platform presence. A streaming service marked active with no
        // frame-age data available must show up as degraded ("enabled but
        // no frames observed yet"), matching compute_health's Some(None)
        // branch, not silently stay healthy because platform is None.
        let state = DiagnosticsState::new(Instant::now(), None, Vec::new());
        state.finalize_startup(Vec::new(), true);
        let snap = state.snapshot().await;
        assert_eq!(snap.status, "degraded");
        assert!(
            snap.degraded_services
                .contains(&"stream_health".to_string()),
            "got {:?}",
            snap.degraded_services
        );
    }

    #[tokio::test]
    async fn test_finalize_startup_streaming_inactive_stays_healthy() {
        // A platform can exist for imaging control with streaming disabled
        // in config (media.streaming_enabled = false). That must not be
        // reported as a stalled stream, since there is no stream at all.
        let state = DiagnosticsState::new(Instant::now(), None, Vec::new());
        state.finalize_startup(Vec::new(), false);
        let snap = state.snapshot().await;
        assert_eq!(snap.status, "healthy");
        assert!(snap.stream_frame_age_ms.is_none());
    }

    #[tokio::test]
    async fn test_snapshot_within_min_delta_reuses_baseline() {
        if !std::path::Path::new("/proc/stat").exists() {
            return;
        }
        let state = DiagnosticsState::new(Instant::now(), None, Vec::new());
        let _ = state.snapshot().await;
        std::thread::sleep(std::time::Duration::from_millis(50));
        let baseline = state.snapshot().await;
        let too_soon = state.snapshot().await;
        assert!(
            too_soon.network.is_none(),
            "polls closer than MIN_DELTA must not emit inflated network rates"
        );
        let _ = baseline;
    }

    fn sample_vision() -> crate::platform::VisionDiagnostics {
        crate::platform::VisionDiagnostics {
            mode: Some("night".to_string()),
            ae_luma: Some(42),
            ae_a_gain_max: Some(10),
            ae_exp_time_max: Some(2250),
            ae_target_luminance: Some(40),
            awb_cnt: Some([0, 1, 2, 3, 4, 5, 6, 7, 8, 9]),
            ain0: Some(100),
            ir_led: Some(true),
            ircut_a: Some(false),
            ircut_b: Some(true),
            white_led: None,
            supported: crate::platform::VisionSupported {
                ir_led: true,
                ircut: true,
                white_led: false,
            },
        }
    }

    fn platform_with_imaging(
        imaging: std::sync::Arc<dyn crate::platform::ImagingControl>,
    ) -> std::sync::Arc<dyn crate::platform::Platform> {
        let mut platform = crate::platform::MockPlatform::new();
        platform
            .expect_imaging_control()
            .returning(move || Some(std::sync::Arc::clone(&imaging)));
        platform.expect_ptz_diagnostics().returning(|| None);
        std::sync::Arc::new(platform)
    }

    #[tokio::test]
    async fn test_snapshot_vision_ok_some_reaches_snapshot() {
        use crate::platform::MockImagingControl;

        let expected = sample_vision();
        let mut imaging = MockImagingControl::new();
        imaging
            .expect_vision_diagnostics()
            .returning(move || Ok(Some(expected.clone())));

        let state = DiagnosticsState::new(
            Instant::now(),
            Some(platform_with_imaging(std::sync::Arc::new(imaging))),
            Vec::new(),
        );
        let snap = state.snapshot().await;
        assert_eq!(snap.vision.as_ref(), Some(&sample_vision()));
    }

    #[tokio::test]
    async fn test_snapshot_vision_ok_none_returns_none() {
        use crate::platform::MockImagingControl;

        let mut imaging = MockImagingControl::new();
        imaging.expect_vision_diagnostics().returning(|| Ok(None));

        let state = DiagnosticsState::new(
            Instant::now(),
            Some(platform_with_imaging(std::sync::Arc::new(imaging))),
            Vec::new(),
        );
        let snap = state.snapshot().await;
        assert!(snap.vision.is_none());
    }

    #[tokio::test]
    async fn test_snapshot_vision_err_returns_none_without_failing() {
        use crate::platform::{MockImagingControl, PlatformError};

        let mut imaging = MockImagingControl::new();
        imaging
            .expect_vision_diagnostics()
            .returning(|| Err(PlatformError::HardwareFailure("vision read failed".into())));

        let state = DiagnosticsState::new(
            Instant::now(),
            Some(platform_with_imaging(std::sync::Arc::new(imaging))),
            Vec::new(),
        );
        let snap = state.snapshot().await;
        assert!(snap.vision.is_none());
        assert_eq!(snap.status, "healthy");
    }
}
