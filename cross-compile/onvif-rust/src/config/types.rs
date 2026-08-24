//! Typed configuration structs for serde-based TOML persistence.
//!
//! Replaces the dynamic `HashMap<String, String>` config system with
//! compile-time typed, serde-deserializable configuration sections.
//!
//! All section structs use `#[serde(default)]` so that missing sections
//! or fields in the TOML file get sensible defaults without parse errors.
//! This ensures backward compatibility when new fields are added.

use serde::{Deserialize, Serialize};

use crate::onvif::types::common::DiscoveryMode;
use crate::osd::format::{DateFormat, TimeFormat};
use crate::osd::layout::Corner;

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
    /// Shared with anyka-init `[update]`; spool is `{update.root}/spool`.
    pub update: UpdateConfig,
    pub stream_profile_1: StreamProfileConfig,
    #[serde(default = "StreamProfileConfig::default_sub")]
    pub stream_profile_2: StreamProfileConfig,
    #[serde(default = "StreamProfileConfig::default_sub")]
    pub stream_profile_3: StreamProfileConfig,
    #[serde(default = "StreamProfileConfig::default_sub")]
    pub stream_profile_4: StreamProfileConfig,
    /// On-screen display (camera name + timestamp burned into video).
    #[serde(default)]
    pub osd: OsdConfig,
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
            update: UpdateConfig::default(),
            stream_profile_1: StreamProfileConfig::default(),
            stream_profile_2: p2,
            stream_profile_3: p3,
            stream_profile_4: p4,
            osd: OsdConfig::default(),
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
        // NaN fails every PartialOrd compare, so `range` alone would accept it.
        if !self.ptz.pan_degrees_per_sec.is_finite() {
            errors.push(format!(
                "ptz.pan_degrees_per_sec: {} is not a finite number",
                self.ptz.pan_degrees_per_sec
            ));
        } else {
            range(
                &mut errors,
                "ptz.pan_degrees_per_sec",
                self.ptz.pan_degrees_per_sec,
                0.1,
                360.0,
            );
        }
        if !self.ptz.tilt_degrees_per_sec.is_finite() {
            errors.push(format!(
                "ptz.tilt_degrees_per_sec: {} is not a finite number",
                self.ptz.tilt_degrees_per_sec
            ));
        } else {
            range(
                &mut errors,
                "ptz.tilt_degrees_per_sec",
                self.ptz.tilt_degrees_per_sec,
                0.1,
                360.0,
            );
        }

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

        // Night thresholds. Ordering, not range, is what matters: each pair
        // must leave a hysteresis band between them, and an inverted pair
        // classifies every reading as both day and night, which `classify`
        // resolves to whichever branch it tests first. That is a camera
        // oscillating at every poll, so refuse the config instead.
        let night = &self.imaging.night;
        if night.ae_night_threshold >= night.ae_day_threshold {
            errors.push(format!(
                "imaging.night.ae_night_threshold ({}) must be below ae_day_threshold ({})",
                night.ae_night_threshold, night.ae_day_threshold
            ));
        }
        // The luminance factor runs the other way: high means dark.
        if night.lum_day_threshold >= night.lum_night_threshold {
            errors.push(format!(
                "imaging.night.lum_day_threshold ({}) must be below lum_night_threshold ({})",
                night.lum_day_threshold, night.lum_night_threshold
            ));
        }
        if let (Some(day), Some(n)) = (night.day_threshold, night.night_threshold)
            && n >= day
        {
            errors.push(format!(
                "imaging.night.night_threshold ({n}) must be below day_threshold ({day})"
            ));
        }

        // OSD — colour is a 16-entry palette index; alpha is the vendor 1..=100 range.
        range(&mut errors, "osd.color", self.osd.color, 0, 15);
        range(&mut errors, "osd.alpha", self.osd.alpha, 1, 100);
        // Empty name text falls back to the device name at render time; non-empty
        // values must already be encodable so a bad anyka.toml cannot reach the
        // renderer.
        if !self.osd.name.text.is_empty()
            && let Err(e) = crate::osd::encode::encode_glyphs(&self.osd.name.text)
        {
            errors.push(format!("osd.name.text: {e}"));
        }

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
            // The SDK documents [20,25] for encode_param.minqp. Out-of-range values are clamped
            // at the encoder rather than rejected, but flagging them here is what stops a typo
            // from silently doing nothing.
            range(
                &mut errors,
                &format!("stream_profile_{n}.min_qp"),
                profile.min_qp,
                20,
                25,
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
// Section: [update]
// ============================================================================

/// Firmware-upgrade paths shared with anyka-init's `[update]` section.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UpdateConfig {
    /// Root holding `active`, `slots/`, `state/`, and `spool/`.
    pub root: String,
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self {
            // Keep in sync with anyka-init `[update] root` and
            // `diagnostics::update::DEFAULT_UPDATE_ROOT`.
            root: "/mnt/anyka_hack".to_string(),
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
    /// ONVIF configurable scopes. Fixed scopes are derived at boot, never stored.
    pub scopes: Vec<String>,
    /// Device UUID (empty to auto-generate).
    pub uuid: String,
    /// Path to ISP configuration file.
    pub isp_config_path: String,
}

impl Default for DeviceConfig {
    fn default() -> Self {
        Self {
            manufacturer: String::new(),
            model: String::new(),
            firmware_version: "1.0.0".to_string(),
            serial_number: String::new(),
            hardware_id: String::new(),
            hostname: "ipcam".to_string(),
            scopes: vec![
                "onvif://www.onvif.org/location/country/unknown".to_string(),
                "onvif://www.onvif.org/name/OnvifCamera".to_string(),
            ],
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
            // Was 0, which the limiter read as "allow nothing" and closed the
            // ONVIF API on every deployment that omitted the key. 0 now means
            // unlimited; this default keeps the DoS bound switched on.
            //
            // 300/min, not RateLimiter's library default of 60: the WebUI fires
            // a burst of SOAP calls on login and on every page, and 60 (1/s) is
            // tight enough to 429 a legitimate operator. Matches the shipped
            // config.toml. Calibration knob: lower it with evidence.
            rate_limit_per_minute: 300,
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
            // error, not warn: measured on .121 that warn + an active streaming
            // client emits ~144 MB/day of slow_tcp_write / slow_rtp_pack lines.
            // error logs ~260 KB/day. See docs/plans/2026-08-10-crash-hardening.md.
            level: "error".to_string(),
            http_verbose: false,
            stream_frame_debug: false,
            ipc_debug: false,
            console_enabled: true,
            file_path: "/mnt/logs/onvif-debug.log".to_string(),
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
    /// Split each UDP RTP frame into batches of this many datagrams (`0`/`1` = one `sendmmsg`).
    ///
    /// Exposed because the right value is a property of the *receiver*, not of this device, so it
    /// cannot be settled at build time. An I-frame here is ~100 KB / ~75 datagrams, and with
    /// pacing off the whole run leaves in a single syscall; a receiver whose socket buffer is
    /// smaller than that clump drops part of every I-frame, and a partial I-frame costs the whole
    /// GOP. Batching trades latency for burst size: each batch boundary sleeps, and on this SoC a
    /// sleep costs ~12 ms whatever you ask for, so `32` on a 75-packet frame is ~24 ms added.
    pub udp_pace_batch: usize,
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
            udp_pace_batch: 0,
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
    /// Degrees per second the pan axis travels at the driver's fixed speed.
    /// Measured on hardware — see the design doc §4 for the procedure.
    pub pan_degrees_per_sec: f64,
    /// Degrees per second the tilt axis travels at the driver's fixed speed.
    pub tilt_degrees_per_sec: f64,
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
            // Measured on AK3918 hardware (AbsoluteMove timing of start_turn→wait_turn;
            // median of three 180° pan / 90° tilt trials). See design doc §4.
            pan_degrees_per_sec: 175.6,
            tilt_degrees_per_sec: 175.4,
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
    /// Widened from `bool`: a bool cannot express AUTO, which is why AUTO
    /// was previously unimplementable at this layer.
    pub ir_cut_filter: crate::onvif::types::common::IrCutFilterMode,
    pub ir_led: bool,
    pub night: NightConfig,
}

impl Default for ImagingConfig {
    fn default() -> Self {
        Self {
            brightness: 50.0,
            contrast: 50.0,
            saturation: 50.0,
            sharpness: 50.0,
            ir_cut_filter: crate::onvif::types::common::IrCutFilterMode::AUTO,
            ir_led: false,
            night: NightConfig::default(),
        }
    }
}

/// Day/night calibration. Polarity and thresholds vary per board; the
/// settle delay and poll interval do not and are constants in
/// `platform::anyka::night_mode`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct NightConfig {
    /// `true` when a high `ain0` reading means daylight.
    pub ldr_high_is_day: bool,
    /// `true` when writing `1` to the ircut node selects night.
    pub ircut_high_is_night: bool,
    /// At or above this raw `ain0` reading, the sensor is saturated bright.
    /// No default: this is a raw ADC value and is board-specific (.198 reads
    /// 648-670, .121 reads 548-639). An uncalibrated board holds instead of
    /// guessing. See wiki/IR-Night-Mode-Calibration.md.
    pub day_threshold: Option<i32>,
    /// At or below this raw `ain0` reading, the sensor is saturated dark.
    /// MUST be calibrated on hardware.
    pub night_threshold: Option<i32>,
    /// Minimum time between transitions, preventing dusk oscillation.
    pub lock_time_ms: u64,
    /// At or above this AE `current_calc_avg_lumi`, treat as day.
    pub ae_day_threshold: i32,
    /// At or below this AE luma, treat as night.
    pub ae_night_threshold: i32,
    /// At or above this ISP luminance factor, treat as night.
    ///
    /// The factor is exposure effort per unit brightness, so it runs the
    /// opposite way to AE luma: high means dark. Defaults are the vendor's own
    /// `[autoir]` values, present in `/etc/jffs2/anyka_cfg.ini` on every camera.
    pub lum_night_threshold: i32,
    /// At or below this ISP luminance factor, treat as day.
    pub lum_day_threshold: i32,
}

impl Default for NightConfig {
    fn default() -> Self {
        Self {
            ldr_high_is_day: true,
            ircut_high_is_night: true,
            // No day_threshold / night_threshold here: they are raw ADC values,
            // board-specific (.198 reads 648-670, .121 reads 548-639). Leaving
            // them absent means an uncalibrated board holds instead of guessing.
            day_threshold: None,
            night_threshold: None,
            lock_time_ms: 900_000,
            // Calibrated on .198 (2026-08-04): room≈34, dark-box≈0..1.
            ae_day_threshold: 28,
            ae_night_threshold: 8,
            // The vendor's own thresholds, read out of every camera's
            // `[autoir]` block: day_to_night_lum / night_to_day_lum.
            lum_night_threshold: 6400,
            lum_day_threshold: 2048,
        }
    }
}

// ============================================================================
// Section: [osd]
// ============================================================================

/// One text overlay: the camera name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct OsdNameConfig {
    pub enabled: bool,
    pub position: Corner,
    /// Empty means "fall back to the ONVIF device name".
    pub text: String,
}

impl Default for OsdNameConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            position: Corner::UpperLeft,
            text: String::new(),
        }
    }
}

/// Timestamp overlay settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct OsdDateTimeConfig {
    pub enabled: bool,
    pub position: Corner,
    pub date_format: DateFormat,
    pub time_format: TimeFormat,
}

impl Default for OsdDateTimeConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            position: Corner::LowerRight,
            date_format: DateFormat::Iso,
            time_format: TimeFormat::H24,
        }
    }
}

/// On-screen display settings.
///
/// `color` and `alpha` are device-global, not per-item: the vendor API
/// (`ak_osd_set_color`, `ak_osd_set_alpha`) takes no channel or rect argument.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct OsdConfig {
    pub enabled: bool,
    /// Index into the vendor's 16-entry colour table, 0..=15.
    pub color: u8,
    /// Overlay opacity, 1..=100.
    pub alpha: u8,
    pub name: OsdNameConfig,
    pub datetime: OsdDateTimeConfig,
}

impl Default for OsdConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            color: 1,
            alpha: 80,
            name: OsdNameConfig::default(),
            datetime: OsdDateTimeConfig::default(),
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
    /// Discovery mode. `NonDiscoverable` keeps the service running but silent.
    pub mode: DiscoveryMode,
}

impl Default for DiscoverySettings {
    fn default() -> Self {
        Self {
            endpoint_uuid: String::new(),
            local_ip: "auto".to_string(),
            enabled: true,
            hello_interval: 300,
            mode: DiscoveryMode::Discoverable,
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
    /// Quantiser floor passed to `ak_venc_open`, SDK range `[20,25]`.
    ///
    /// **The AK3918 encoder ignores this.** Measured on device: with `min_qp = 25` the I-frame
    /// still encoded at QP 23, read from the bitstream via
    /// `SliceQP = 26 + pic_init_qp_minus26 + slice_qp_delta`. A floor that is honoured cannot be
    /// undercut, and I-frame size was unchanged (83.8 KB at 20 vs 84.7 KB at 25).
    ///
    /// Kept because the value does travel correctly to `ak_venc_open` and the SDK documents the
    /// field; a different firmware may honour it. Do not expect changing it to alter the encoded
    /// output on this hardware. `ak_venc_set_rc_weight` is the knob that plausibly does, and is
    /// not wired up.
    pub min_qp: u32,
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
            min_qp: 20,
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
            min_qp: 20,
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
    fn test_device_scopes_default_includes_factory_name_and_location() {
        let config = AppConfig::default();
        assert!(
            config
                .device
                .scopes
                .iter()
                .any(|s| s.ends_with("/name/OnvifCamera"))
        );
        assert!(
            config
                .device
                .scopes
                .iter()
                .any(|s| s.ends_with("/location/country/unknown"))
        );
    }

    #[test]
    fn test_device_identity_defaults_are_empty_for_override_detection() {
        // Empty means "not overridden" so the platform descriptor wins.
        let config = AppConfig::default();
        assert_eq!(config.device.manufacturer, "");
        assert_eq!(config.device.model, "");
        assert_eq!(config.device.serial_number, "");
        assert_eq!(config.device.hardware_id, "");
    }

    #[test]
    fn test_discovery_mode_defaults_to_discoverable() {
        let config = AppConfig::default();
        assert_eq!(config.discovery.mode, DiscoveryMode::Discoverable);
    }

    #[test]
    fn test_device_scopes_round_trip_through_toml() {
        let toml_str = r#"
[device]
scopes = ["onvif://www.onvif.org/name/Front%20Door"]
"#;
        let config: AppConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(
            config.device.scopes,
            vec!["onvif://www.onvif.org/name/Front%20Door"]
        );
    }

    #[test]
    fn test_deployed_config_without_device_section_still_parses() {
        // No .deploy/*.toml carries [device] or [discovery]; serde(default) must cover it.
        let config: AppConfig = toml::from_str("[server]\nport = 80\n").unwrap();
        assert!(
            config
                .device
                .scopes
                .iter()
                .any(|s| s.ends_with("/name/OnvifCamera"))
        );
        assert_eq!(config.discovery.mode, DiscoveryMode::Discoverable);
    }

    /// `validate` must refuse a night config whose threshold pairs leave no
    /// hysteresis band: an equal or inverted pair classifies every reading as
    /// both day and night, which makes the camera oscillate at every poll.
    #[test]
    fn test_validate_ae_thresholds_equal_reports_ordering_error() {
        let mut config = AppConfig::default();
        config.imaging.night.ae_night_threshold = config.imaging.night.ae_day_threshold;

        let errors = config
            .validate()
            .expect_err("equal AE thresholds must be refused");
        assert_eq!(
            errors,
            vec!["imaging.night.ae_night_threshold (28) must be below ae_day_threshold (28)"]
        );
    }

    #[test]
    fn test_validate_ae_thresholds_inverted_reports_ordering_error() {
        let mut config = AppConfig::default();
        config.imaging.night.ae_night_threshold = 40; // default ae_day_threshold is 28

        let errors = config
            .validate()
            .expect_err("inverted AE thresholds must be refused");
        assert_eq!(
            errors,
            vec!["imaging.night.ae_night_threshold (40) must be below ae_day_threshold (28)"]
        );
    }

    /// The luminance factor runs the other way (high means dark), so the
    /// ordering check is `lum_day < lum_night`.
    #[test]
    fn test_validate_lum_thresholds_equal_reports_ordering_error() {
        let mut config = AppConfig::default();
        config.imaging.night.lum_day_threshold = config.imaging.night.lum_night_threshold;

        let errors = config
            .validate()
            .expect_err("equal luminance thresholds must be refused");
        assert_eq!(
            errors,
            vec!["imaging.night.lum_day_threshold (6400) must be below lum_night_threshold (6400)"]
        );
    }

    #[test]
    fn test_validate_lum_thresholds_inverted_reports_ordering_error() {
        let mut config = AppConfig::default();
        config.imaging.night.lum_day_threshold = 8000; // default lum_night_threshold is 6400

        let errors = config
            .validate()
            .expect_err("inverted luminance thresholds must be refused");
        assert_eq!(
            errors,
            vec!["imaging.night.lum_day_threshold (8000) must be below lum_night_threshold (6400)"]
        );
    }

    /// The raw ADC pair is optional (board-specific calibration), but when
    /// both halves are present the same ordering rule applies.
    #[test]
    fn test_validate_raw_thresholds_equal_reports_ordering_error() {
        let mut config = AppConfig::default();
        config.imaging.night.day_threshold = Some(640);
        config.imaging.night.night_threshold = Some(640);

        let errors = config
            .validate()
            .expect_err("equal raw thresholds must be refused");
        assert_eq!(
            errors,
            vec!["imaging.night.night_threshold (640) must be below day_threshold (640)"]
        );
    }

    #[test]
    fn test_validate_raw_thresholds_inverted_reports_ordering_error() {
        let mut config = AppConfig::default();
        config.imaging.night.day_threshold = Some(640);
        config.imaging.night.night_threshold = Some(800);

        let errors = config
            .validate()
            .expect_err("inverted raw thresholds must be refused");
        assert_eq!(
            errors,
            vec!["imaging.night.night_threshold (800) must be below day_threshold (640)"]
        );
    }

    /// A single present raw threshold (or none at all) is an uncalibrated
    /// board and must not trip the ordering check.
    #[test]
    fn test_validate_raw_thresholds_single_set_is_valid() {
        let mut config = AppConfig::default();
        config.imaging.night.day_threshold = Some(640);

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
        assert_eq!(config.device.model, "");
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
        assert!(sample.contains("manufacturer = \"\""));
    }

    /// `min_qp` is documented as an inclusive `[20, 25]` range, so the boundaries themselves must
    /// validate and the values just outside must not. Off-by-one here is silent: `validate` only
    /// reports, so a wrongly-rejected 25 would look like a config typo to whoever hit it.
    #[test]
    fn test_validate_min_qp_inclusive_boundaries() {
        let validate_with = |min_qp: u32| {
            let mut config = AppConfig::default();
            config.stream_profile_1.min_qp = min_qp;
            config.validate()
        };

        assert!(validate_with(20).is_ok(), "20 is the inclusive lower bound");
        assert!(validate_with(25).is_ok(), "25 is the inclusive upper bound");

        for out_of_range in [19, 26] {
            let errors = validate_with(out_of_range)
                .expect_err("a min_qp outside [20, 25] must be reported");
            assert!(
                errors.iter().any(|e| e.contains("stream_profile_1.min_qp")),
                "the error must name the field that is wrong, got {errors:?}"
            );
        }
    }

    #[test]
    fn test_imaging_config_defaults_to_auto_ir_cut_filter() {
        let cfg = ImagingConfig::default();
        assert_eq!(
            cfg.ir_cut_filter,
            crate::onvif::types::common::IrCutFilterMode::AUTO
        );
    }

    #[test]
    fn test_imaging_config_parses_ir_cut_filter_mode_from_toml() {
        let cfg: ImagingConfig = toml::from_str(r#"ir_cut_filter = "OFF""#).unwrap();
        assert_eq!(
            cfg.ir_cut_filter,
            crate::onvif::types::common::IrCutFilterMode::OFF
        );
    }

    #[test]
    fn test_night_config_defaults_match_vendor_lock_time() {
        let cfg = NightConfig::default();
        assert_eq!(cfg.lock_time_ms, 900_000);
        assert!(cfg.ldr_high_is_day);
        assert!(cfg.ircut_high_is_night);
    }

    #[test]
    fn test_night_config_defaults_leave_thresholds_uncalibrated() {
        // A raw ain0 threshold is board-specific (.198 reads 648-670, .121
        // reads 548-639). Shipping no default means an uncalibrated board holds
        // instead of guessing with another board's numbers.
        let cfg = NightConfig::default();
        assert_eq!(cfg.day_threshold, None);
        assert_eq!(cfg.night_threshold, None);
    }

    #[test]
    fn test_imaging_config_without_night_section_leaves_ain0_uncalibrated() {
        // Regression: a partial config that omits [imaging.night] must not
        // guess the raw ain0 thresholds. AE thresholds still have defaults.
        let cfg: ImagingConfig = toml::from_str(
            r#"
            brightness = 70.0
            contrast = 50.0
            saturation = 50.0
            sharpness = 50.0
            ir_cut_filter = "AUTO"
            "#,
        )
        .unwrap();
        assert_eq!(cfg.night.day_threshold, None);
        assert_eq!(cfg.night.night_threshold, None);
    }

    #[test]
    fn test_night_config_ae_thresholds_default() {
        let cfg = NightConfig::default();
        assert_eq!(cfg.ae_day_threshold, 28);
        assert_eq!(cfg.ae_night_threshold, 8);
    }

    #[test]
    fn test_ptz_config_default_rates_are_positive() {
        let c = PtzConfig::default();
        assert!(c.pan_degrees_per_sec > 0.0);
        assert!(c.tilt_degrees_per_sec > 0.0);
    }

    #[test]
    fn test_ptz_config_rejects_zero_pan_rate() {
        let mut config = AppConfig::default();
        config.ptz.pan_degrees_per_sec = 0.0;
        let errors = config
            .validate()
            .expect_err("a zero rate divides motion by nothing");
        assert!(errors.iter().any(|e| e.contains("pan_degrees_per_sec")));
    }

    #[test]
    fn test_ptz_config_rejects_nan_rates() {
        let mut config = AppConfig::default();
        config.ptz.pan_degrees_per_sec = f64::NAN;
        config.ptz.tilt_degrees_per_sec = f64::INFINITY;
        let errors = config
            .validate()
            .expect_err("non-finite rates must not pass validation");
        assert!(errors.iter().any(|e| e.contains("pan_degrees_per_sec")));
        assert!(errors.iter().any(|e| e.contains("tilt_degrees_per_sec")));
    }

    #[test]
    fn test_ptz_config_ignores_removed_legacy_keys() {
        // A config file written by an older build still carries these keys.
        let toml = r#"
enabled = true
presets_json = "{}"
next_preset_num = 4
home_pan = 0.5
"#;
        let parsed: PtzConfig = toml::from_str(toml).expect("legacy keys must not break loading");
        assert!(parsed.enabled);
    }

    #[test]
    fn test_validate_osd_name_text_rejects_non_ascii() {
        let mut config = AppConfig::default();
        config.osd.name.text = "Ogród".into();
        let errors = config.validate().expect_err("non-ASCII name text");
        assert!(errors.iter().any(|e| e.contains("osd.name.text")));
    }

    #[test]
    fn test_validate_osd_name_text_rejects_over_max_glyphs() {
        let mut config = AppConfig::default();
        config.osd.name.text = "A".repeat(crate::osd::encode::MAX_GLYPHS + 1);
        let errors = config.validate().expect_err("over-long name text");
        assert!(errors.iter().any(|e| e.contains("osd.name.text")));
    }

    #[test]
    fn test_validate_osd_name_text_accepts_max_glyphs() {
        let mut config = AppConfig::default();
        config.osd.name.text = "A".repeat(crate::osd::encode::MAX_GLYPHS);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_osd_config_defaults_are_sane() {
        let cfg = OsdConfig::default();
        assert!(cfg.enabled);
        assert_eq!(cfg.alpha, 80);
        assert_eq!(cfg.name.position, Corner::UpperLeft);
        assert_eq!(cfg.datetime.position, Corner::LowerRight);
    }

    #[test]
    fn test_osd_config_round_trips_through_toml() {
        let cfg = OsdConfig::default();
        let text = toml::to_string(&cfg).unwrap();
        let back: OsdConfig = toml::from_str(&text).unwrap();
        assert_eq!(cfg, back);
    }

    #[test]
    fn test_app_config_parses_without_an_osd_section() {
        // Existing deployed anyka.toml files have no [osd] section and must keep
        // loading — a missing section means "defaults", not "reject the config".
        let cfg: AppConfig = toml::from_str("").unwrap();
        assert!(cfg.osd.enabled);
    }
}
