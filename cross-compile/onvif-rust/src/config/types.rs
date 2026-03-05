//! Typed configuration structs for serde-based TOML persistence.
//!
//! Replaces the dynamic `HashMap<String, String>` config system with
//! compile-time typed, serde-deserializable configuration sections.
//!
//! All section structs use `#[serde(default)]` so that missing sections
//! or fields in the TOML file get sensible defaults without parse errors.
//! This ensures backward compatibility when new fields are added.

use serde::{Deserialize, Serialize};

// ============================================================================
// Root configuration
// ============================================================================

/// Root application configuration, mapping directly to `config.toml`.
///
/// Each field corresponds to a `[section]` in the TOML file.
/// `#[serde(default)]` on the struct means an entirely missing section
/// produces the section's `Default` value.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub onvif: OnvifConfig,
    pub network: NetworkConfig,
    pub device: DeviceConfig,
    pub server: ServerConfig,
    pub logging: LoggingConfig,
    pub media: MediaConfig,
    pub ptz: PtzConfig,
    pub imaging: ImagingConfig,
    pub discovery: DiscoverySettings,
    pub memory: MemoryConfig,
    pub stream_profile_1: StreamProfileConfig,
    #[serde(default = "StreamProfileConfig::default_sub")]
    pub stream_profile_2: StreamProfileConfig,
    #[serde(default = "StreamProfileConfig::default_sub")]
    pub stream_profile_3: StreamProfileConfig,
    #[serde(default = "StreamProfileConfig::default_sub")]
    pub stream_profile_4: StreamProfileConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        let p2 = StreamProfileConfig::default_sub();
        // p2 name already "SubStream" from default_sub()

        let mut p3 = StreamProfileConfig::default_sub();
        p3.name = "Stream3".to_string();
        p3.enabled = false;

        let mut p4 = StreamProfileConfig::default_sub();
        p4.name = "Stream4".to_string();
        p4.enabled = false;

        Self {
            onvif: OnvifConfig::default(),
            network: NetworkConfig::default(),
            device: DeviceConfig::default(),
            server: ServerConfig::default(),
            logging: LoggingConfig::default(),
            media: MediaConfig::default(),
            ptz: PtzConfig::default(),
            imaging: ImagingConfig::default(),
            discovery: DiscoverySettings::default(),
            memory: MemoryConfig::default(),
            stream_profile_1: StreamProfileConfig::default(),
            stream_profile_2: p2,
            stream_profile_3: p3,
            stream_profile_4: p4,
        }
    }
}

impl AppConfig {
    /// Get a reference to stream profile N (1-4).
    ///
    /// Returns `stream_profile_1` for `n=1`, etc. Panics on out-of-range.
    pub fn stream_profile(&self, n: u32) -> &StreamProfileConfig {
        match n {
            1 => &self.stream_profile_1,
            2 => &self.stream_profile_2,
            3 => &self.stream_profile_3,
            4 => &self.stream_profile_4,
            _ => panic!("stream_profile index {n} out of range 1..=4"),
        }
    }

    /// Validate configuration values against expected ranges.
    ///
    /// Called after deserialization to catch out-of-range values early.
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // Helper: check numeric range
        fn range<T: PartialOrd + std::fmt::Display>(
            errors: &mut Vec<String>,
            name: &str,
            val: T,
            min: T,
            max: T,
        ) {
            if val < min || val > max {
                errors.push(format!("{name}: {val} not in [{min}, {max}]"));
            }
        }

        // Server
        range(&mut errors, "server.port", self.server.port, 1, u16::MAX);
        range(
            &mut errors,
            "server.max_connections",
            self.server.max_connections,
            1,
            256,
        );
        range(
            &mut errors,
            "server.request_timeout",
            self.server.request_timeout,
            1,
            300,
        );
        range(
            &mut errors,
            "server.max_body_size",
            self.server.max_body_size,
            1024,
            10_485_760,
        );
        range(
            &mut errors,
            "server.config_save_delay_ms",
            self.server.config_save_delay_ms,
            50,
            10_000,
        );

        // Media
        range(
            &mut errors,
            "media.max_streams",
            self.media.max_streams,
            1,
            8,
        );

        // PTZ
        range(&mut errors, "ptz.pan_speed", self.ptz.pan_speed, 0.0, 1.0);
        range(&mut errors, "ptz.tilt_speed", self.ptz.tilt_speed, 0.0, 1.0);
        range(&mut errors, "ptz.zoom_speed", self.ptz.zoom_speed, 0.0, 1.0);
        range(&mut errors, "ptz.max_presets", self.ptz.max_presets, 1, 64);
        range(&mut errors, "ptz.home_pan", self.ptz.home_pan, -1.0, 1.0);
        range(&mut errors, "ptz.home_tilt", self.ptz.home_tilt, -1.0, 1.0);
        range(&mut errors, "ptz.home_zoom", self.ptz.home_zoom, 0.0, 1.0);

        // Imaging
        range(
            &mut errors,
            "imaging.brightness",
            self.imaging.brightness,
            0.0,
            100.0,
        );
        range(
            &mut errors,
            "imaging.contrast",
            self.imaging.contrast,
            0.0,
            100.0,
        );
        range(
            &mut errors,
            "imaging.saturation",
            self.imaging.saturation,
            0.0,
            100.0,
        );
        range(
            &mut errors,
            "imaging.sharpness",
            self.imaging.sharpness,
            0.0,
            100.0,
        );

        // Discovery
        range(
            &mut errors,
            "discovery.hello_interval",
            self.discovery.hello_interval,
            30,
            3600,
        );

        // Stream profiles
        for (i, profile) in [
            &self.stream_profile_1,
            &self.stream_profile_2,
            &self.stream_profile_3,
            &self.stream_profile_4,
        ]
        .iter()
        .enumerate()
        {
            let n = i + 1;
            range(
                &mut errors,
                &format!("stream_profile_{n}.width"),
                profile.width,
                160,
                3840,
            );
            range(
                &mut errors,
                &format!("stream_profile_{n}.height"),
                profile.height,
                120,
                2160,
            );
            range(
                &mut errors,
                &format!("stream_profile_{n}.framerate"),
                profile.framerate,
                1,
                60,
            );
            range(
                &mut errors,
                &format!("stream_profile_{n}.bitrate"),
                profile.bitrate,
                64,
                16384,
            );
            range(
                &mut errors,
                &format!("stream_profile_{n}.gop_length"),
                profile.gop_length,
                1,
                300,
            );
            range(
                &mut errors,
                &format!("stream_profile_{n}.quality"),
                profile.quality,
                1,
                100,
            );
            range(
                &mut errors,
                &format!("stream_profile_{n}.audio_bitrate"),
                profile.audio_bitrate,
                8,
                512,
            );
            range(
                &mut errors,
                &format!("stream_profile_{n}.audio_sample_rate"),
                profile.audio_sample_rate,
                8000,
                48000,
            );
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

// ============================================================================
// Section: [onvif]
// ============================================================================

/// ONVIF protocol settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct OnvifConfig {
    pub device_service_path: String,
    pub media_service_path: String,
    pub ptz_service_path: String,
    pub imaging_service_path: String,
    pub profile_level: String,
}

impl Default for OnvifConfig {
    fn default() -> Self {
        Self {
            device_service_path: "/onvif/device_service".to_string(),
            media_service_path: "/onvif/media_service".to_string(),
            ptz_service_path: "/onvif/ptz_service".to_string(),
            imaging_service_path: "/onvif/imaging_service".to_string(),
            profile_level: "S".to_string(),
        }
    }
}

// ============================================================================
// Section: [network]
// ============================================================================

/// Network configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct NetworkConfig {
    pub interface: String,
    pub ip_address: String,
    pub netmask: String,
    pub gateway: String,
    pub dns_primary: String,
    pub dns_secondary: String,
    pub ntp_from_dhcp: bool,
    pub ntp_primary: String,
    pub ntp_secondary: String,
    pub http_enabled: bool,
    pub rtsp_enabled: bool,
    pub dhcp_enabled: bool,
    /// Runtime-detected IP address (not from config file).
    pub detected_ip: String,
    /// Runtime-detected MAC address (not from config file).
    pub mac_address: String,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            interface: "eth0".to_string(),
            ip_address: String::new(),
            netmask: "255.255.255.0".to_string(),
            gateway: String::new(),
            dns_primary: String::new(),
            dns_secondary: String::new(),
            ntp_from_dhcp: true,
            ntp_primary: String::new(),
            ntp_secondary: String::new(),
            http_enabled: true,
            rtsp_enabled: true,
            dhcp_enabled: true,
            detected_ip: String::new(),
            mac_address: String::new(),
        }
    }
}

// ============================================================================
// Section: [device]
// ============================================================================

/// Device information.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DeviceConfig {
    pub manufacturer: String,
    pub model: String,
    pub firmware_version: String,
    pub serial_number: String,
    pub hardware_id: String,
    pub hostname: String,
    /// Comma-separated ONVIF scopes.
    pub scopes: String,
    /// Device UUID (empty to auto-generate).
    pub uuid: String,
    /// Path to ISP configuration file.
    pub isp_config_path: String,
}

impl Default for DeviceConfig {
    fn default() -> Self {
        Self {
            manufacturer: "Anyka".to_string(),
            model: "AK3918 Camera".to_string(),
            firmware_version: "1.0.0".to_string(),
            serial_number: String::new(),
            hardware_id: "ak3918".to_string(),
            hostname: "ipcam".to_string(),
            scopes: String::new(),
            uuid: String::new(),
            isp_config_path: String::new(),
        }
    }
}

// ============================================================================
// Section: [server]
// ============================================================================

/// HTTP server settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    pub port: u16,
    pub bind_address: String,
    pub max_connections: u32,
    pub request_timeout: u64,
    pub max_body_size: usize,
    pub config_save_delay_ms: u64,
    pub auth_enabled: bool,
    pub realm: String,
    pub tls_enabled: bool,
    pub tls_cert_path: String,
    pub tls_key_path: String,
    pub rate_limit_per_minute: u32,
    /// Root directory for static file serving.
    pub static_root: String,
    /// Runtime-bound server address (fallback for IP detection).
    pub address: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: 80,
            bind_address: "0.0.0.0".to_string(),
            max_connections: 16,
            request_timeout: 30,
            max_body_size: 1_048_576,
            config_save_delay_ms: 500,
            auth_enabled: true,
            realm: "ONVIF Camera".to_string(),
            tls_enabled: false,
            tls_cert_path: String::new(),
            tls_key_path: String::new(),
            rate_limit_per_minute: 0,
            static_root: String::new(),
            address: String::new(),
        }
    }
}

// ============================================================================
// Section: [logging]
// ============================================================================

/// Logging configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LoggingConfig {
    pub level: String,
    pub http_verbose: bool,
    pub stream_frame_debug: bool,
    pub ipc_debug: bool,
    pub console_enabled: bool,
    /// Log file path (empty to disable file logging).
    pub file_path: String,
    pub static_assets: StaticAssetsConfig,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            http_verbose: false,
            stream_frame_debug: false,
            ipc_debug: false,
            console_enabled: true,
            file_path: String::new(),
            static_assets: StaticAssetsConfig::default(),
        }
    }
}

/// Static asset access logging configuration (`[logging.static_assets]`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct StaticAssetsConfig {
    pub enabled: bool,
    pub file_path: String,
    pub file_name: String,
}

impl Default for StaticAssetsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            file_path: "logs".to_string(),
            file_name: "access".to_string(),
        }
    }
}

// ============================================================================
// Section: [media]
// ============================================================================

/// Media stream settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MediaConfig {
    pub rtsp_port: u16,
    /// Snapshot HTTP port (0 to use main server port).
    pub snapshot_port: u16,
    pub snapshot_path: String,
    pub max_streams: u32,
    /// Whether streaming is active (set at runtime by streaming subsystem).
    pub streaming_enabled: bool,
    /// HTTP-FLV port (set at runtime by streaming subsystem).
    pub httpflv_port: u16,
}

impl Default for MediaConfig {
    fn default() -> Self {
        Self {
            rtsp_port: 554,
            snapshot_port: 0,
            snapshot_path: "/snapshot".to_string(),
            max_streams: 4,
            streaming_enabled: true,
            httpflv_port: 0,
        }
    }
}

// ============================================================================
// Section: [ptz]
// ============================================================================

/// PTZ control settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PtzConfig {
    pub enabled: bool,
    pub pan_speed: f64,
    pub tilt_speed: f64,
    pub zoom_speed: f64,
    pub max_presets: u32,
    pub home_on_start: bool,
    /// Serialized PTZ presets JSON string.
    pub presets_json: String,
    pub home_pan: f64,
    pub home_tilt: f64,
    pub home_zoom: f64,
    pub next_preset_num: u32,
}

impl Default for PtzConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            pan_speed: 0.5,
            tilt_speed: 0.5,
            zoom_speed: 0.5,
            max_presets: 16,
            home_on_start: true,
            presets_json: String::new(),
            home_pan: 0.0,
            home_tilt: 0.0,
            home_zoom: 0.0,
            next_preset_num: 1,
        }
    }
}

// ============================================================================
// Section: [imaging]
// ============================================================================

/// Imaging settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ImagingConfig {
    pub brightness: f64,
    pub contrast: f64,
    pub saturation: f64,
    pub sharpness: f64,
    pub ir_cut_filter: bool,
    pub ir_led: bool,
}

impl Default for ImagingConfig {
    fn default() -> Self {
        Self {
            brightness: 50.0,
            contrast: 50.0,
            saturation: 50.0,
            sharpness: 50.0,
            ir_cut_filter: true,
            ir_led: false,
        }
    }
}

// ============================================================================
// Section: [discovery]
// ============================================================================

/// WS-Discovery settings.
///
/// Named `DiscoverySettings` to avoid collision with the runtime
/// `DiscoveryConfig` in `onvif::discovery`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DiscoverySettings {
    /// Device endpoint UUID (empty to auto-generate and persist).
    pub endpoint_uuid: String,
    /// Local IP for XAddrs (`"auto"` for automatic detection).
    pub local_ip: String,
    pub enabled: bool,
    /// Hello announcement interval in seconds.
    pub hello_interval: u32,
}

impl Default for DiscoverySettings {
    fn default() -> Self {
        Self {
            endpoint_uuid: String::new(),
            local_ip: "auto".to_string(),
            enabled: true,
            hello_interval: 300,
        }
    }
}

// ============================================================================
// Section: [memory]
// ============================================================================

/// Memory management settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MemoryConfig {
    /// Soft memory limit in MB (0 = disabled).
    pub soft_limit_mb: u32,
    /// Hard memory limit in MB.
    pub hard_limit_mb: u32,
    /// Interval for memory usage logging in seconds.
    pub logging_interval_secs: u32,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            soft_limit_mb: 0,
            hard_limit_mb: 24,
            logging_interval_secs: 60,
        }
    }
}

// ============================================================================
// Section: [stream_profile_N]
// ============================================================================

/// Stream profile configuration.
///
/// `Default` produces HD settings (profile 1: 1920x1080@25fps).
/// Use `default_sub()` for sub-stream SD settings (640x480@15fps).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct StreamProfileConfig {
    pub name: String,
    pub enabled: bool,
    pub width: u32,
    pub height: u32,
    pub framerate: u32,
    pub bitrate: u32,
    pub encoding: String,
    pub gop_length: u32,
    pub quality: u32,
    /// H264 profile (`"Baseline"`, `"Main"`, `"High"`).
    pub profile: String,
    pub audio_enabled: bool,
    pub audio_encoding: String,
    pub audio_bitrate: u32,
    pub audio_sample_rate: u32,
}

impl Default for StreamProfileConfig {
    fn default() -> Self {
        Self {
            name: "MainStream".to_string(),
            enabled: true,
            width: 1920,
            height: 1080,
            framerate: 25,
            bitrate: 4000,
            encoding: "H264".to_string(),
            gop_length: 50,
            quality: 80,
            profile: String::new(),
            audio_enabled: true,
            audio_encoding: "G711".to_string(),
            audio_bitrate: 64,
            audio_sample_rate: 8000,
        }
    }
}

impl StreamProfileConfig {
    /// Sub-stream defaults: SD resolution, lower bitrate, audio disabled.
    pub fn default_sub() -> Self {
        Self {
            name: "SubStream".to_string(),
            enabled: true,
            width: 640,
            height: 480,
            framerate: 15,
            bitrate: 512,
            encoding: "H264".to_string(),
            gop_length: 30,
            quality: 80,
            profile: String::new(),
            audio_enabled: false,
            audio_encoding: "G711".to_string(),
            audio_bitrate: 64,
            audio_sample_rate: 8000,
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_validates() {
        let config = AppConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_default_server_values() {
        let config = AppConfig::default();
        assert_eq!(config.server.port, 80);
        assert_eq!(config.server.bind_address, "0.0.0.0");
        assert_eq!(config.server.max_connections, 16);
        assert!(config.server.auth_enabled);
    }

    #[test]
    fn test_default_profile_values() {
        let config = AppConfig::default();

        // Profile 1: HD
        assert_eq!(config.stream_profile_1.width, 1920);
        assert_eq!(config.stream_profile_1.height, 1080);
        assert_eq!(config.stream_profile_1.framerate, 25);
        assert!(config.stream_profile_1.enabled);
        assert!(config.stream_profile_1.audio_enabled);

        // Profile 2: SD, enabled
        assert_eq!(config.stream_profile_2.width, 640);
        assert_eq!(config.stream_profile_2.height, 480);
        assert_eq!(config.stream_profile_2.framerate, 15);
        assert!(config.stream_profile_2.enabled);
        assert!(!config.stream_profile_2.audio_enabled);

        // Profile 3-4: SD, disabled
        assert!(!config.stream_profile_3.enabled);
        assert!(!config.stream_profile_4.enabled);
    }

    #[test]
    fn test_default_profile_names() {
        let config = AppConfig::default();
        assert_eq!(config.stream_profile_1.name, "MainStream");
        assert_eq!(config.stream_profile_2.name, "SubStream");
        assert_eq!(config.stream_profile_3.name, "Stream3");
        assert_eq!(config.stream_profile_4.name, "Stream4");
    }

    #[test]
    fn test_toml_roundtrip() {
        let config = AppConfig::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: AppConfig = toml::from_str(&toml_str).unwrap();

        assert_eq!(config.server.port, parsed.server.port);
        assert_eq!(config.device.manufacturer, parsed.device.manufacturer);
        assert_eq!(config.ptz.pan_speed, parsed.ptz.pan_speed);
        assert_eq!(config.stream_profile_1.width, parsed.stream_profile_1.width);
        assert_eq!(config.stream_profile_2.width, parsed.stream_profile_2.width);
    }

    #[test]
    fn test_partial_toml_loads_defaults() {
        let toml_str = r#"
[server]
port = 8080

[device]
manufacturer = "Custom"
"#;

        let config: AppConfig = toml::from_str(toml_str).unwrap();

        // Specified values
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.device.manufacturer, "Custom");

        // Defaults for unspecified
        assert!(config.server.auth_enabled);
        assert_eq!(config.device.model, "AK3918 Camera");
        assert_eq!(config.ptz.pan_speed, 0.5);
        assert_eq!(config.stream_profile_1.width, 1920);
    }

    #[test]
    fn test_empty_toml_uses_all_defaults() {
        let config: AppConfig = toml::from_str("").unwrap();
        let default = AppConfig::default();

        assert_eq!(config.server.port, default.server.port);
        assert_eq!(config.device.manufacturer, default.device.manufacturer);
        assert_eq!(
            config.stream_profile_1.width,
            default.stream_profile_1.width
        );
    }

    #[test]
    fn test_validation_rejects_invalid_port() {
        let mut config = AppConfig::default();
        config.server.port = 0;
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err()[0].contains("server.port"));
    }

    #[test]
    fn test_validation_rejects_invalid_ptz_speed() {
        let mut config = AppConfig::default();
        config.ptz.pan_speed = 1.5;
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err()[0].contains("ptz.pan_speed"));
    }

    #[test]
    fn test_validation_rejects_invalid_profile_resolution() {
        let mut config = AppConfig::default();
        config.stream_profile_1.width = 100; // below minimum 160
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err()[0].contains("stream_profile_1.width"));
    }

    #[test]
    fn test_logging_static_assets_nested() {
        let toml_str = r#"
[logging]
level = "debug"

[logging.static_assets]
enabled = false
file_path = "/var/log"
file_name = "static"
"#;

        let config: AppConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.logging.level, "debug");
        assert!(!config.logging.static_assets.enabled);
        assert_eq!(config.logging.static_assets.file_path, "/var/log");
        assert_eq!(config.logging.static_assets.file_name, "static");
    }

    #[test]
    fn test_discovery_defaults() {
        let config = AppConfig::default();
        assert!(config.discovery.enabled);
        assert_eq!(config.discovery.local_ip, "auto");
        assert_eq!(config.discovery.hello_interval, 300);
    }

    #[test]
    fn test_memory_defaults() {
        let config = AppConfig::default();
        assert_eq!(config.memory.soft_limit_mb, 0);
        assert_eq!(config.memory.hard_limit_mb, 24);
        assert_eq!(config.memory.logging_interval_secs, 60);
    }

    #[test]
    fn test_generate_sample_config() {
        let sample = toml::to_string_pretty(&AppConfig::default()).unwrap();
        assert!(sample.contains("[server]"));
        assert!(sample.contains("[device]"));
        assert!(sample.contains("port = 80"));
        assert!(sample.contains("manufacturer = \"Anyka\""));
    }
}
