//! Streaming configuration derived from the application's runtime config.

use crate::config::ConfigRuntime;

/// Configuration for the live streaming subsystem.
#[derive(Clone)]
pub struct StreamingConfig {
    /// RTSP server port (default: 554).
    pub rtsp_port: u16,
    /// HTTP-FLV server port (default: 8080).
    pub httpflv_port: u16,
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
    /// Build a `StreamingConfig` from the application's runtime configuration.
    pub fn from_config(config: &ConfigRuntime) -> Self {
        // Validate RTSP port is in valid u16 range, default to 554 if out of range
        let rtsp_port = match config.get_int("media.rtsp_port") {
            Some(port) if (1..=65535).contains(&port) => port as u16,
            Some(port) => {
                tracing::warn!(
                    port = port,
                    "Invalid RTSP port (out of range 1-65535), using default 554"
                );
                554
            }
            None => 554,
        };
        
        // Validate HTTP-FLV port is in valid u16 range, default to 8080 if out of range
        let httpflv_port = match config.get_int("media.httpflv_port") {
            Some(port) if (1..=65535).contains(&port) => port as u16,
            Some(port) => {
                tracing::warn!(
                    port = port,
                    "Invalid HTTP-FLV port (out of range 1-65535), using default 8080"
                );
                8080
            }
            None => 8080,
        };
        
        let enabled = config.get_bool("media.streaming_enabled").unwrap_or(true);

        Self {
            rtsp_port,
            httpflv_port,
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

    #[test]
    fn test_config_default_values() {
        let config = StreamingConfig::default();
        assert_eq!(config.rtsp_port, 554);
        assert_eq!(config.httpflv_port, 8080);
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
        assert!(config.enabled);
    }

    #[test]
    fn test_config_from_runtime_custom_ports() {
        let runtime = ConfigRuntime::new(Default::default());
        runtime.set_int("media.rtsp_port", 8554).unwrap();
        runtime.set_int("media.httpflv_port", 9080).unwrap();

        let config = StreamingConfig::from_config(&runtime);
        assert_eq!(config.rtsp_port, 8554);
        assert_eq!(config.httpflv_port, 9080);
    }

    #[test]
    fn test_config_from_runtime_disabled() {
        let runtime = ConfigRuntime::new(Default::default());
        runtime.set_bool("media.streaming_enabled", false).unwrap();

        let config = StreamingConfig::from_config(&runtime);
        assert!(!config.enabled);
    }
}
