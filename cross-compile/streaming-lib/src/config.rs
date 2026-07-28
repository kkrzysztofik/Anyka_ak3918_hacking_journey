//! Streaming configuration module
//!
//! This module provides the `StreamingConfig` struct which replaces global state
//! with explicit configuration passed to components.

use crate::protocol::rtsp::session::server_session::LagRecoveryMode;
use tracing::{debug, warn};

/// Configuration for the streaming service
///
/// This struct replaces global static variables with explicit configuration,
/// making the code more testable and thread-safe.
#[derive(Debug, Clone)]
pub struct StreamingConfig {
    /// RTP packet sampling interval for debug logging (every N packets, 0 = disabled)
    ///
    /// When set to a non-zero value, every Nth RTP packet will be logged at debug level
    /// with statistics (SSRC, timestamp, sequence number). This is for troubleshooting
    /// RTP stream issues, not RTCP sender report generation.
    pub rtp_sample_interval: u32,

    /// Maximum age of frames to deliver in milliseconds
    ///
    /// Frames older than this will be dropped during playback to ensure
    /// latency doesn't grow unbounded.
    ///
    /// Default **1500 ms**, aligned with [`Self::play_ready_timeout_ms`] so playback
    /// does not drop frames while tracks are still warming up after PLAY. Adjust
    /// both together if you tighten staleness (e.g. lower `max_frame_age_ms` only
    /// after measuring that IDR/SPS/PPS delivery fits within the smaller window).
    pub max_frame_age_ms: u32,

    /// Lag recovery mode for handling playback delays
    pub lag_recovery_mode: LagRecoveryMode,

    /// Play ready timeout in milliseconds
    ///
    /// Maximum time to wait for media tracks (SPS/PPS/codec info) to become available
    /// after receiving a PLAY request, before sending the PLAY response. If tracks are
    /// not ready within this timeout, the server sends a 503 Service Unavailable response.
    pub play_ready_timeout_ms: u64,

    /// Sleep every this many UDP RTP packets while flushing one framed unit (marker bit).
    /// Values ≤1 disable intra-frame pacing; **the default is 0 (off)**.
    ///
    /// Pacing was meant to help the kernel drain TX queues on embedded targets, but on the
    /// AK3918 camera it cost far more than it saved. Two tokio workers share one core with the
    /// vendor daemon and the WiFi TX thread, so a `sleep(300us)` resolves to ~12 ms: measured
    /// across 1301 logged frames, `send_ms ~= 31.6 + 12.1 * (batches - 1)`. At the previous
    /// default of 10, a ~110 KB I-frame became 8 batches and took ~116 ms to leave the box —
    /// against a 66 ms frame budget at 15 fps — with ~85 ms of that spent asleep.
    ///
    /// The kernel already paces this path: the UDP send buffer (~304 KB after the `wmem_max`
    /// clamp) exceeds any single frame, and the device reported zero `SndbufErrors` across 287k
    /// datagrams. Set a positive value only on a link where the socket buffer measurably fills.
    pub udp_pace_batch: usize,

    /// Sleep duration between UDP pace batches (microseconds). Used with [`Self::udp_pace_batch`]
    /// so pacing is time-based, not only scheduler yields. `0` means use the library default.
    pub udp_pace_sleep_micros: u32,

    /// Maximum bytes buffered for one logical TCP interleaved RTP frame (until the RTP marker
    /// bit ends the frame).
    ///
    /// Default **1 MiB** (`1024 * 1024`). That is a large per-connection allocation on very small
    /// targets (for example ~24 MiB RAM); consider **64 KiB–256 KiB** if you control max GOP/I-frame
    /// size and want to cap worst-case buffering. The same value bounds UDP marker-terminated
    /// packet accumulation in the play path. Lower this before raising stream bitrates only after
    /// validating that your largest access units still fit.
    pub tcp_interleaved_buffer_max: usize,

    /// RTSP server listen address
    ///
    /// Format: "0.0.0.0:554" or "[::]:554" for IPv6
    pub rtsp_listen_addr: String,

    /// HTTP-FLV server listen address
    ///
    /// Format: "0.0.0.0:8080" or "[::]:8080" for IPv6
    pub httpflv_listen_addr: String,
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            rtp_sample_interval: 0, // disabled by default
            max_frame_age_ms: 1500,
            lag_recovery_mode: LagRecoveryMode::LatestIdr,
            play_ready_timeout_ms: 1500,
            udp_pace_batch: 0, // intra-frame pacing off; see field docs for the measurement
            udp_pace_sleep_micros: 300,
            tcp_interleaved_buffer_max: 1024 * 1024,
            rtsp_listen_addr: "0.0.0.0:554".to_string(),
            httpflv_listen_addr: "0.0.0.0:8080".to_string(),
        }
    }
}

impl StreamingConfig {
    /// Create a new configuration with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the RTP sample interval
    ///
    /// # Arguments
    ///
    /// * `interval` - Sample every N packets (0 = disabled)
    pub fn with_rtp_sample_interval(mut self, interval: u32) -> Self {
        self.rtp_sample_interval = interval;
        self
    }

    /// Set the maximum frame age
    ///
    /// # Arguments
    ///
    /// * `max_age_ms` - Maximum age in milliseconds
    pub fn with_max_frame_age(mut self, max_age_ms: u32) -> Self {
        self.max_frame_age_ms = max_age_ms;
        self
    }

    /// Set the lag recovery mode
    pub fn with_lag_recovery_mode(mut self, mode: LagRecoveryMode) -> Self {
        self.lag_recovery_mode = mode;
        self
    }

    /// Set the play ready timeout
    ///
    /// # Arguments
    ///
    /// * `timeout_ms` - Timeout in milliseconds
    pub fn with_play_ready_timeout(mut self, timeout_ms: u64) -> Self {
        self.play_ready_timeout_ms = timeout_ms;
        self
    }

    /// Set the RTSP listen address
    pub fn with_rtsp_listen_addr(mut self, addr: impl Into<String>) -> Self {
        self.rtsp_listen_addr = addr.into();
        self
    }

    /// Set the HTTP-FLV listen address
    pub fn with_httpflv_listen_addr(mut self, addr: impl Into<String>) -> Self {
        self.httpflv_listen_addr = addr.into();
        self
    }

    /// Configure UDP RTP pacing during a marker-terminated flush.
    ///
    /// # Arguments
    ///
    /// * `batch` — Emit a pacing delay after every this many packets within one flush.
    ///   Values **`0` or `1` disable pacing**: the frame is handed over in a single call. This
    ///   is the default; see field [`Self::udp_pace_batch`] for why.
    ///
    /// # Note
    ///
    /// Actual delays use [`Self::udp_pace_sleep_micros`] (see default in [`Default::default`]).
    /// Larger `batch` values only reduce how often that sleep runs. Budget roughly one scheduler
    /// quantum per sleep, not the requested microseconds, whenever the target's cores are shared.
    pub fn with_udp_pace_batch(mut self, batch: usize) -> Self {
        if batch <= 1 {
            debug!(
                batch = batch,
                "udp_pace_batch disables inter-packet pacing for values <= 1"
            );
        }
        self.udp_pace_batch = batch;
        self
    }

    /// Override TCP interleaved frame buffer cap (bytes until RTP marker).
    ///
    /// # Arguments
    ///
    /// * `max_bytes` — Upper bound on buffered bytes for one logical interleaved RTP frame
    ///   (until the RTP marker bit ends the frame).
    ///
    /// Values below **1** are rejected: `0` is replaced with [`Default::default`] and a warning
    /// is logged (`tcp_interleaved_buffer_max` field name in logs).
    pub fn with_tcp_interleaved_buffer_max(mut self, max_bytes: usize) -> Self {
        if max_bytes < 1 {
            warn!(
                field = "tcp_interleaved_buffer_max",
                value = max_bytes,
                "invalid tcp_interleaved_buffer_max; using default {}",
                Self::default().tcp_interleaved_buffer_max
            );
            self.tcp_interleaved_buffer_max = Self::default().tcp_interleaved_buffer_max;
        } else {
            self.tcp_interleaved_buffer_max = max_bytes;
        }
        self
    }

    /// Override UDP inter-packet pacing sleep (microseconds).
    ///
    /// # Arguments
    ///
    /// * `micros` — Sleep between pace batches during a marker-terminated UDP flush.
    ///
    /// # Note
    ///
    /// `0` is not stored: it is normalized to [`Default::default`] so pacing always uses a
    /// positive sleep (see server session fallback). A warning is logged (`udp_pace_sleep_micros`).
    pub fn with_udp_pace_sleep_micros(mut self, micros: u32) -> Self {
        if micros < 1 {
            warn!(
                field = "udp_pace_sleep_micros",
                value = micros,
                "invalid udp_pace_sleep_micros; using default {}",
                Self::default().udp_pace_sleep_micros
            );
            self.udp_pace_sleep_micros = Self::default().udp_pace_sleep_micros;
        } else {
            self.udp_pace_sleep_micros = micros;
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = StreamingConfig::default();

        assert_eq!(config.rtp_sample_interval, 0);
        assert_eq!(config.max_frame_age_ms, 1500);
        assert_eq!(config.lag_recovery_mode, LagRecoveryMode::LatestIdr);
        assert_eq!(config.play_ready_timeout_ms, 1500);
        assert_eq!(
            config.udp_pace_batch, 0,
            "intra-frame pacing off by default"
        );
        assert_eq!(config.udp_pace_sleep_micros, 300);
        assert_eq!(config.tcp_interleaved_buffer_max, 1024 * 1024);
        assert_eq!(config.rtsp_listen_addr, "0.0.0.0:554");
        assert_eq!(config.httpflv_listen_addr, "0.0.0.0:8080");
    }

    #[test]
    fn test_config_builder() {
        let config = StreamingConfig::new()
            .with_rtp_sample_interval(1000)
            .with_max_frame_age(2000)
            .with_rtsp_listen_addr("0.0.0.0:8554")
            .with_httpflv_listen_addr("0.0.0.0:8888");

        assert_eq!(config.rtp_sample_interval, 1000);
        assert_eq!(config.max_frame_age_ms, 2000);
        assert_eq!(config.rtsp_listen_addr, "0.0.0.0:8554");
        assert_eq!(config.httpflv_listen_addr, "0.0.0.0:8888");
    }

    #[test]
    fn test_play_ready_timeout_builder() {
        let config = StreamingConfig::new().with_play_ready_timeout(3000);

        assert_eq!(config.play_ready_timeout_ms, 3000);
    }

    #[test]
    fn test_play_ready_timeout_default() {
        let config = StreamingConfig::new();

        assert_eq!(config.play_ready_timeout_ms, 1500);
    }

    #[test]
    fn test_udp_pace_batch_builder_sets_field() {
        let config = StreamingConfig::new().with_udp_pace_batch(25);
        assert_eq!(config.udp_pace_batch, 25);
        assert_eq!(
            config.play_ready_timeout_ms, 1500,
            "other defaults unchanged"
        );
    }

    #[test]
    fn test_udp_pace_sleep_micros_builder_sets_field() {
        let config = StreamingConfig::new().with_udp_pace_sleep_micros(500);
        assert_eq!(config.udp_pace_sleep_micros, 500);
        assert_eq!(config.udp_pace_batch, 0, "other defaults unchanged");
    }

    #[test]
    fn test_tcp_interleaved_buffer_max_builder_sets_field() {
        let config = StreamingConfig::new().with_tcp_interleaved_buffer_max(512 * 1024);
        assert_eq!(config.tcp_interleaved_buffer_max, 512 * 1024);
        assert_eq!(
            config.rtsp_listen_addr, "0.0.0.0:554",
            "other defaults unchanged"
        );
    }

    #[test]
    fn test_config_clone() {
        let config1 = StreamingConfig::default();
        let config2 = config1.clone();

        assert_eq!(config1.rtp_sample_interval, config2.rtp_sample_interval);
        assert_eq!(config1.rtsp_listen_addr, config2.rtsp_listen_addr);
    }

    #[test]
    fn test_config_debug() {
        let config = StreamingConfig::default();
        let debug = format!("{:?}", config);

        assert!(debug.contains("StreamingConfig"));
        assert!(debug.contains("rtp_sample_interval"));
    }
}
