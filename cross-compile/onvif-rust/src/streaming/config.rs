//! Streaming configuration derived from the application's runtime config.

use crate::config::ConfigRuntime;

/// Configuration for the live streaming subsystem.
#[derive(Clone)]
pub struct StreamingConfig {
    /// RTSP server port (default: 554).
    pub rtsp_port: u16,
    /// HTTP-FLV server port (default: 8080).
    pub httpflv_port: u16,
    /// Audio sample rate in Hz for stream metadata (default: 8000).
    pub audio_sample_rate: u32,
    /// Video frame rate for SDP `a=framerate` attribute (default: 15).
    /// Helps clients like VLC pre-configure their jitter buffer.
    pub video_framerate: u32,
    /// Main stream path name (default: "main").
    pub main_stream_name: String,
    /// Sub stream path name (default: "sub").
    pub sub_stream_name: String,
    /// Application name for HTTP-FLV (default: "live").
    pub app_name: String,
    /// Optional stream authentication.
    pub auth: Option<streaming_lib::common::auth::Auth>,
    /// Whether streaming is enabled (default: true).
    pub enabled: bool,
    /// UDP RTP datagrams per pacing batch; `0`/`1` hands a whole frame over in one `sendmmsg`.
    /// See [`crate::config::MediaConfig::udp_pace_batch`] for the trade-off.
    pub udp_pace_batch: usize,
}

impl StreamingConfig {
    /// Build a `StreamingConfig` from the application's runtime configuration.
    pub fn from_config(config: &ConfigRuntime) -> Self {
        let c = config.read();

        let rtsp_port = if c.media.rtsp_port == 0 {
            554
        } else {
            c.media.rtsp_port
        };
        let httpflv_port = if c.media.httpflv_port == 0 {
            8080
        } else {
            c.media.httpflv_port
        };

        let audio_sample_rate = if (8000..=48000).contains(&c.stream_profile_1.audio_sample_rate) {
            c.stream_profile_1.audio_sample_rate
        } else {
            tracing::warn!(
                rate = c.stream_profile_1.audio_sample_rate,
                default = 8000,
                "Invalid audio_sample_rate, using default"
            );
            8000
        };

        let video_framerate = if (1..=120).contains(&c.stream_profile_1.framerate) {
            c.stream_profile_1.framerate
        } else {
            15
        };

        Self {
            rtsp_port,
            httpflv_port,
            audio_sample_rate,
            video_framerate,
            main_stream_name: "main".to_string(),
            sub_stream_name: "sub".to_string(),
            app_name: "live".to_string(),
            auth: None,
            enabled: c.media.streaming_enabled,
            udp_pace_batch: c.media.udp_pace_batch,
        }
    }
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            rtsp_port: 554,
            httpflv_port: 8080,
            audio_sample_rate: 8000,
            video_framerate: 15,
            main_stream_name: "main".to_string(),
            sub_stream_name: "sub".to_string(),
            app_name: "live".to_string(),
            auth: None,
            enabled: true,
            udp_pace_batch: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default_values() {
        let config = StreamingConfig::default();
        assert_eq!(config.rtsp_port, 554);
        assert_eq!(config.httpflv_port, 8080);
        assert_eq!(config.audio_sample_rate, 8000);
        assert_eq!(config.video_framerate, 15);
        assert_eq!(config.main_stream_name, "main");
        assert_eq!(config.sub_stream_name, "sub");
        assert_eq!(config.app_name, "live");
        assert!(config.auth.is_none());
        assert!(config.enabled);
        assert_eq!(config.udp_pace_batch, 0);
    }

    /// The knob is only useful if it survives the trip from `[media]` to the streaming layer;
    /// a field added to the struct but never read from the runtime config fails silently.
    #[test]
    fn test_config_from_runtime_carries_udp_pace_batch() {
        let runtime = ConfigRuntime::new(Default::default());
        runtime.write().media.udp_pace_batch = 32;

        assert_eq!(StreamingConfig::from_config(&runtime).udp_pace_batch, 32);
    }

    #[test]
    fn test_config_from_runtime_defaults() {
        let runtime = ConfigRuntime::new(Default::default());
        let config = StreamingConfig::from_config(&runtime);

        assert_eq!(config.rtsp_port, 554);
        assert_eq!(config.audio_sample_rate, 8000);
        assert!(config.enabled);
    }

    #[test]
    fn test_config_from_runtime_custom_ports() {
        let runtime = ConfigRuntime::new(Default::default());
        {
            let mut c = runtime.write();
            c.media.rtsp_port = 8554;
            c.media.httpflv_port = 9080;
            c.stream_profile_1.audio_sample_rate = 44100;
        }

        let config = StreamingConfig::from_config(&runtime);
        assert_eq!(config.rtsp_port, 8554);
        assert_eq!(config.httpflv_port, 9080);
        assert_eq!(config.audio_sample_rate, 44100);
    }

    #[test]
    fn test_config_from_runtime_disabled() {
        let runtime = ConfigRuntime::new(Default::default());
        runtime.write().media.streaming_enabled = false;

        let config = StreamingConfig::from_config(&runtime);
        assert!(!config.enabled);
    }

    #[test]
    fn test_config_from_runtime_out_of_range_values_use_defaults() {
        let mut app_config = crate::config::AppConfig::default();
        app_config.media.rtsp_port = 0; // invalid: 0 is not a valid port
        app_config.media.httpflv_port = 0;
        app_config.stream_profile_1.audio_sample_rate = 96000; // out of 8000-48000

        let runtime = ConfigRuntime::new(app_config);
        let config = StreamingConfig::from_config(&runtime);

        assert_eq!(config.rtsp_port, 554); // fallback
        assert_eq!(config.httpflv_port, 8080); // fallback
        assert_eq!(config.audio_sample_rate, 8000); // fallback
    }
}
