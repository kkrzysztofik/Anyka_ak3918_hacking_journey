//! Streaming configuration module
//!
//! This module provides the `StreamingConfig` struct which replaces global state
//! with explicit configuration passed to components.

use crate::protocol::rtsp::session::server_session::LagRecoveryMode;

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
    pub max_frame_age_ms: u32,

    /// Lag recovery mode for handling playback delays
    pub lag_recovery_mode: LagRecoveryMode,

    /// Play ready timeout in milliseconds
    ///
    /// Maximum time to wait for media tracks (SPS/PPS/codec info) to become available
    /// after receiving a PLAY request, before sending the PLAY response. If tracks are
    /// not ready within this timeout, the server sends a 503 Service Unavailable response.
    pub play_ready_timeout_ms: u64,

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
            max_frame_age_ms: 1000,
            lag_recovery_mode: LagRecoveryMode::LatestIdr,
            play_ready_timeout_ms: 1500,
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = StreamingConfig::default();

        assert_eq!(config.rtp_sample_interval, 0);
        assert_eq!(config.max_frame_age_ms, 1000);
        assert_eq!(config.lag_recovery_mode, LagRecoveryMode::LatestIdr);
        assert_eq!(config.play_ready_timeout_ms, 1500);
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
