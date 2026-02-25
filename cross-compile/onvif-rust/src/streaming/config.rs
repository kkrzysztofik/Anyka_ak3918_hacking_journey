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
}

impl StreamingConfig {
    fn parse_port(config: &ConfigRuntime, key: &str, default: u16, label: &str) -> u16 {
        match config.get_int(key) {
            Ok(port) if (1..=65535).contains(&port) => port as u16,
            Ok(port) => {
                tracing::warn!(
                    port = port,
                    default = default,
                    "{} port out of range 1-65535, using default",
                    label
                );
                default
            }
            Err(_) => default,
        }
    }

    fn parse_audio_sample_rate(config: &ConfigRuntime) -> u32 {
        const DEFAULT_AUDIO_SAMPLE_RATE: u32 = 8000;
        match config.get_int("stream_profile_1.audio_sample_rate") {
            Ok(rate) if (8000..=48000).contains(&rate) => rate as u32,
            Ok(rate) => {
                tracing::warn!(
                    rate = rate,
                    default = DEFAULT_AUDIO_SAMPLE_RATE,
                    "Invalid stream_profile_1.audio_sample_rate, using default"
                );
                DEFAULT_AUDIO_SAMPLE_RATE
            }
            Err(_) => DEFAULT_AUDIO_SAMPLE_RATE,
        }
    }

    /// Build a `StreamingConfig` from the application's runtime configuration.
    pub fn from_config(config: &ConfigRuntime) -> Self {
        let rtsp_port = Self::parse_port(config, "media.rtsp_port", 554, "RTSP");
        let httpflv_port = Self::parse_port(config, "media.httpflv_port", 8080, "HTTP-FLV");
        let audio_sample_rate = Self::parse_audio_sample_rate(config);
        let enabled = config.get_bool("media.streaming_enabled").unwrap_or(true);
        let video_framerate = config
            .get_int("stream_profile_1.framerate")
            .ok()
            .filter(|&v| (1..=120).contains(&v))
            .map(|v| v as u32)
            .unwrap_or(15);

        Self {
            rtsp_port,
            httpflv_port,
            audio_sample_rate,
            video_framerate,
            main_stream_name: "main".to_string(),
            sub_stream_name: "sub".to_string(),
            app_name: "live".to_string(),
            auth: None,
            enabled,
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ConfigStorage;

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
    }

    #[test]
    fn test_config_from_runtime_defaults() {
        let runtime = ConfigRuntime::new(Default::default());
        let config = StreamingConfig::from_config(&runtime);

        assert_eq!(config.rtsp_port, 554);
        assert_eq!(config.httpflv_port, 8080);
        assert_eq!(config.audio_sample_rate, 8000);
        assert!(config.enabled);
    }

    #[test]
    fn test_config_from_runtime_custom_ports() {
        let runtime = ConfigRuntime::new(Default::default());
        runtime.set_int("media.rtsp_port", 8554).unwrap();
        runtime.set_int("media.httpflv_port", 9080).unwrap();
        runtime
            .set_int("stream_profile_1.audio_sample_rate", 44100)
            .unwrap();

        let config = StreamingConfig::from_config(&runtime);
        assert_eq!(config.rtsp_port, 8554);
        assert_eq!(config.httpflv_port, 9080);
        assert_eq!(config.audio_sample_rate, 44100);
    }

    #[test]
    fn test_config_from_runtime_disabled() {
        let runtime = ConfigRuntime::new(Default::default());
        runtime.set_bool("media.streaming_enabled", false).unwrap();

        let config = StreamingConfig::from_config(&runtime);
        assert!(!config.enabled);
    }

    #[test]
    fn test_config_from_runtime_out_of_range_values_use_defaults() {
        let temp_config = std::env::temp_dir().join(format!(
            "streaming-config-out-of-range-{}.toml",
            uuid::Uuid::new_v4()
        ));
        std::fs::write(
            &temp_config,
            "[media]\nrtsp_port = 0\nhttpflv_port = 70000\n\
             [stream_profile_1]\naudio_sample_rate = 96000\n",
        )
        .unwrap();
        let app_config =
            ConfigStorage::load_or_default(temp_config.to_str().unwrap_or("/tmp/config.toml"))
                .unwrap();
        let runtime = ConfigRuntime::new(app_config);
        let _ = std::fs::remove_file(&temp_config);

        let config = StreamingConfig::from_config(&runtime);
        assert_eq!(config.rtsp_port, 554);
        assert_eq!(config.httpflv_port, 8080);
        assert_eq!(config.audio_sample_rate, 8000);
    }

    #[test]
    fn test_config_from_runtime_non_numeric_audio_sample_rate_uses_default() {
        let temp_config = std::env::temp_dir().join(format!(
            "streaming-config-non-numeric-audio-sample-rate-{}.toml",
            uuid::Uuid::new_v4()
        ));
        std::fs::write(
            &temp_config,
            "[stream_profile_1]\naudio_sample_rate = \"not-a-number\"\n",
        )
        .unwrap();
        let app_config =
            ConfigStorage::load_or_default(temp_config.to_str().unwrap_or("/tmp/config.toml"))
                .unwrap();
        let runtime = ConfigRuntime::new(app_config);
        let _ = std::fs::remove_file(&temp_config);

        let config = StreamingConfig::from_config(&runtime);
        assert_eq!(config.audio_sample_rate, 8000);
    }
}
