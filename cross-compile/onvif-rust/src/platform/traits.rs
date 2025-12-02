//! Platform abstraction traits.
//!
//! This module defines the traits that abstract hardware access for the ONVIF
//! implementation. All traits are designed to be mockable using mockall for testing.

use std::sync::Arc;

use async_trait::async_trait;
#[cfg(test)]
use mockall::automock;
use thiserror::Error;

/// Errors that can occur in platform operations.
#[derive(Debug, Error, Clone)]
pub enum PlatformError {
    /// Hardware initialization failed.
    #[error("Initialization failed: {0}")]
    InitializationFailed(String),

    /// Hardware not available.
    #[error("Hardware not available: {0}")]
    HardwareUnavailable(String),

    /// Operation not supported by this hardware.
    #[error("Operation not supported: {0}")]
    NotSupported(String),

    /// Invalid parameter.
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),

    /// Operation timed out.
    #[error("Operation timed out")]
    Timeout,

    /// Hardware failure during operation.
    #[error("Hardware failure: {0}")]
    HardwareFailure(String),

    /// Resource busy.
    #[error("Resource busy: {0}")]
    ResourceBusy(String),

    /// Permission denied.
    #[error("Permission denied")]
    PermissionDenied,
}

/// Result type for platform operations.
pub type PlatformResult<T> = Result<T, PlatformError>;

/// Video resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Resolution {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
}

impl Resolution {
    /// Create a new resolution.
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

/// Video encoding type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VideoEncoding {
    /// H.264/AVC encoding.
    #[default]
    H264,
    /// H.265/HEVC encoding.
    H265,
    /// MJPEG encoding.
    Mjpeg,
}

/// Bitrate control mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BitrateMode {
    /// Constant bitrate.
    #[default]
    Cbr,
    /// Variable bitrate.
    Vbr,
}

/// Video source configuration.
#[derive(Debug, Clone, Default)]
pub struct VideoSourceConfig {
    /// Source token.
    pub token: String,
    /// Source name.
    pub name: String,
    /// Native resolution.
    pub resolution: Resolution,
    /// Maximum frame rate.
    pub max_framerate: f32,
}

/// Video encoder configuration.
#[derive(Debug, Clone, Default)]
pub struct VideoEncoderConfig {
    /// Encoder token.
    pub token: String,
    /// Encoder name.
    pub name: String,
    /// Output resolution.
    pub resolution: Resolution,
    /// Frame rate in frames per second.
    pub framerate: u32,
    /// Target bitrate in kbps.
    pub bitrate: u32,
    /// Encoding type.
    pub encoding: VideoEncoding,
    /// Bitrate control mode.
    pub bitrate_mode: BitrateMode,
    /// GOP length (I-frame interval).
    pub gop_length: u32,
    /// Quality level (0-100).
    pub quality: u32,
}

/// Audio encoding type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AudioEncoding {
    /// G.711 μ-law encoding.
    #[default]
    G711U,
    /// G.711 A-law encoding.
    G711A,
    /// AAC encoding.
    Aac,
    /// PCM (raw audio).
    Pcm,
}

/// Audio source configuration.
#[derive(Debug, Clone, Default)]
pub struct AudioSourceConfig {
    /// Source token.
    pub token: String,
    /// Source name.
    pub name: String,
    /// Number of channels.
    pub channels: u32,
}

/// Audio encoder configuration.
#[derive(Debug, Clone, Default)]
pub struct AudioEncoderConfig {
    /// Encoder token.
    pub token: String,
    /// Encoder name.
    pub name: String,
    /// Sample rate in Hz.
    pub sample_rate: u32,
    /// Number of channels.
    pub channels: u32,
    /// Encoding type.
    pub encoding: AudioEncoding,
    /// Bitrate in kbps.
    pub bitrate: u32,
}

/// PTZ position in degrees.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PtzPosition {
    /// Pan position (-180.0 to 180.0 degrees).
    pub pan: f32,
    /// Tilt position (-90.0 to 90.0 degrees).
    pub tilt: f32,
    /// Zoom level (1.0 to max zoom).
    pub zoom: f32,
}

impl PtzPosition {
    /// Create a new PTZ position.
    pub fn new(pan: f32, tilt: f32, zoom: f32) -> Self {
        Self { pan, tilt, zoom }
    }

    /// Home position (center, no zoom).
    pub const HOME: PtzPosition = PtzPosition {
        pan: 0.0,
        tilt: 0.0,
        zoom: 1.0,
    };
}

/// PTZ velocity for continuous movement.
#[derive(Debug, Clone, Copy, Default)]
pub struct PtzVelocity {
    /// Pan velocity (-1.0 to 1.0).
    pub pan: f32,
    /// Tilt velocity (-1.0 to 1.0).
    pub tilt: f32,
    /// Zoom velocity (-1.0 to 1.0).
    pub zoom: f32,
}

impl PtzVelocity {
    /// Create a new PTZ velocity.
    pub fn new(pan: f32, tilt: f32, zoom: f32) -> Self {
        Self { pan, tilt, zoom }
    }

    /// Stop velocity.
    pub const STOP: PtzVelocity = PtzVelocity {
        pan: 0.0,
        tilt: 0.0,
        zoom: 0.0,
    };
}

/// PTZ preset.
#[derive(Debug, Clone)]
pub struct PtzPreset {
    /// Preset token (identifier).
    pub token: String,
    /// Preset name.
    pub name: String,
    /// Preset position.
    pub position: PtzPosition,
}

/// PTZ limits.
#[derive(Debug, Clone, Copy, Default)]
pub struct PtzLimits {
    /// Minimum pan angle.
    pub min_pan: f32,
    /// Maximum pan angle.
    pub max_pan: f32,
    /// Minimum tilt angle.
    pub min_tilt: f32,
    /// Maximum tilt angle.
    pub max_tilt: f32,
    /// Minimum zoom level.
    pub min_zoom: f32,
    /// Maximum zoom level.
    pub max_zoom: f32,
}

impl PtzLimits {
    /// Default PTZ limits.
    pub const DEFAULT: PtzLimits = PtzLimits {
        min_pan: -180.0,
        max_pan: 180.0,
        min_tilt: -90.0,
        max_tilt: 90.0,
        min_zoom: 1.0,
        max_zoom: 10.0,
    };
}

/// Imaging settings.
#[derive(Debug, Clone, Default)]
pub struct ImagingSettings {
    /// Brightness (0.0 to 100.0).
    pub brightness: f32,
    /// Contrast (0.0 to 100.0).
    pub contrast: f32,
    /// Saturation (0.0 to 100.0).
    pub saturation: f32,
    /// Sharpness (0.0 to 100.0).
    pub sharpness: f32,
    /// IR cut filter enabled.
    pub ir_cut_filter: bool,
    /// IR LED enabled.
    pub ir_led: bool,
    /// Wide dynamic range enabled.
    pub wdr: bool,
    /// Backlight compensation enabled.
    pub backlight_compensation: bool,
}

/// Imaging options (valid ranges for settings).
#[derive(Debug, Clone, Default)]
pub struct ImagingOptions {
    /// Brightness range.
    pub brightness_range: (f32, f32),
    /// Contrast range.
    pub contrast_range: (f32, f32),
    /// Saturation range.
    pub saturation_range: (f32, f32),
    /// Sharpness range.
    pub sharpness_range: (f32, f32),
    /// IR cut filter supported.
    pub ir_cut_filter_supported: bool,
    /// IR LED supported.
    pub ir_led_supported: bool,
    /// WDR supported.
    pub wdr_supported: bool,
    /// Backlight compensation supported.
    pub backlight_compensation_supported: bool,
}

impl ImagingOptions {
    /// Default imaging options.
    pub fn default_options() -> Self {
        Self {
            brightness_range: (0.0, 100.0),
            contrast_range: (0.0, 100.0),
            saturation_range: (0.0, 100.0),
            sharpness_range: (0.0, 100.0),
            ir_cut_filter_supported: true,
            ir_led_supported: true,
            wdr_supported: false,
            backlight_compensation_supported: true,
        }
    }
}

/// Device information.
#[derive(Debug, Clone, Default)]
pub struct DeviceInfo {
    /// Manufacturer name.
    pub manufacturer: String,
    /// Device model.
    pub model: String,
    /// Firmware version.
    pub firmware_version: String,
    /// Serial number.
    pub serial_number: String,
    /// Hardware ID.
    pub hardware_id: String,
}

/// Video input trait for camera sensor access.
#[cfg_attr(test, automock)]
#[async_trait]
pub trait VideoInput: Send + Sync {
    /// Open the video input device.
    async fn open(&self) -> PlatformResult<()>;

    /// Close the video input device.
    async fn close(&self) -> PlatformResult<()>;

    /// Get the native resolution of the video input.
    async fn get_resolution(&self) -> PlatformResult<Resolution>;

    /// Get all video source configurations.
    async fn get_sources(&self) -> PlatformResult<Vec<VideoSourceConfig>>;
}

/// Video encoder trait for video encoding operations.
#[cfg_attr(test, automock)]
#[async_trait]
pub trait VideoEncoder: Send + Sync {
    /// Initialize the video encoder.
    async fn init(&self, config: &VideoEncoderConfig) -> PlatformResult<()>;

    /// Get the current encoder configuration.
    async fn get_configuration(&self) -> PlatformResult<VideoEncoderConfig>;

    /// Set the encoder configuration.
    async fn set_configuration(&self, config: &VideoEncoderConfig) -> PlatformResult<()>;

    /// Get all video encoder configurations.
    async fn get_configurations(&self) -> PlatformResult<Vec<VideoEncoderConfig>>;

    /// Get valid configuration options.
    async fn get_options(&self) -> PlatformResult<VideoEncoderOptions>;
}

/// Video encoder configuration options.
#[derive(Debug, Clone, Default)]
pub struct VideoEncoderOptions {
    /// Supported resolutions.
    pub resolutions: Vec<Resolution>,
    /// Supported encodings.
    pub encodings: Vec<VideoEncoding>,
    /// Framerate range.
    pub framerate_range: (u32, u32),
    /// Bitrate range (kbps).
    pub bitrate_range: (u32, u32),
    /// GOP length range.
    pub gop_range: (u32, u32),
    /// Quality range.
    pub quality_range: (u32, u32),
}

/// Audio input trait for microphone access.
#[cfg_attr(test, automock)]
#[async_trait]
pub trait AudioInput: Send + Sync {
    /// Open the audio input device.
    async fn open(&self) -> PlatformResult<()>;

    /// Close the audio input device.
    async fn close(&self) -> PlatformResult<()>;

    /// Get the audio input configuration.
    async fn get_configuration(&self) -> PlatformResult<AudioSourceConfig>;

    /// Get all audio source configurations.
    async fn get_sources(&self) -> PlatformResult<Vec<AudioSourceConfig>>;
}

/// Audio encoder trait for audio encoding operations.
#[cfg_attr(test, automock)]
#[async_trait]
pub trait AudioEncoder: Send + Sync {
    /// Initialize the audio encoder.
    async fn init(&self, config: &AudioEncoderConfig) -> PlatformResult<()>;

    /// Get the current encoder configuration.
    async fn get_configuration(&self) -> PlatformResult<AudioEncoderConfig>;

    /// Set the encoder configuration.
    async fn set_configuration(&self, config: &AudioEncoderConfig) -> PlatformResult<()>;

    /// Get all audio encoder configurations.
    async fn get_configurations(&self) -> PlatformResult<Vec<AudioEncoderConfig>>;
}

/// PTZ control trait for pan/tilt/zoom operations.
#[cfg_attr(test, automock)]
#[async_trait]
pub trait PTZControl: Send + Sync {
    /// Move to an absolute position.
    async fn move_to_position(&self, position: PtzPosition) -> PlatformResult<()>;

    /// Get the current position.
    async fn get_position(&self) -> PlatformResult<PtzPosition>;

    /// Start continuous movement.
    async fn continuous_move(&self, velocity: PtzVelocity) -> PlatformResult<()>;

    /// Stop all PTZ movement.
    async fn stop(&self) -> PlatformResult<()>;

    /// Get all presets.
    async fn get_presets(&self) -> PlatformResult<Vec<PtzPreset>>;

    /// Set a preset at the current position.
    async fn set_preset(&self, name: &str) -> PlatformResult<String>;

    /// Go to a preset position.
    async fn goto_preset(&self, token: &str) -> PlatformResult<()>;

    /// Remove a preset.
    async fn remove_preset(&self, token: &str) -> PlatformResult<()>;

    /// Get PTZ limits.
    async fn get_limits(&self) -> PlatformResult<PtzLimits>;
}

/// Imaging control trait for image settings.
#[cfg_attr(test, automock)]
#[async_trait]
pub trait ImagingControl: Send + Sync {
    /// Get current imaging settings.
    async fn get_settings(&self) -> PlatformResult<ImagingSettings>;

    /// Set imaging settings.
    async fn set_settings(&self, settings: &ImagingSettings) -> PlatformResult<()>;

    /// Get valid imaging options.
    async fn get_options(&self) -> PlatformResult<ImagingOptions>;

    /// Set brightness.
    async fn set_brightness(&self, value: f32) -> PlatformResult<()>;

    /// Set contrast.
    async fn set_contrast(&self, value: f32) -> PlatformResult<()>;

    /// Set saturation.
    async fn set_saturation(&self, value: f32) -> PlatformResult<()>;

    /// Set sharpness.
    async fn set_sharpness(&self, value: f32) -> PlatformResult<()>;
}

// ============================================================================
// Network Information
// ============================================================================

/// Network interface information.
#[derive(Debug, Clone, Default)]
pub struct NetworkInterfaceInfo {
    /// Interface token (e.g., "eth0").
    pub token: String,
    /// Interface name.
    pub name: String,
    /// Whether the interface is enabled.
    pub enabled: bool,
    /// IPv4 address (if configured).
    pub ipv4_address: Option<String>,
    /// IPv4 prefix length (subnet mask).
    pub ipv4_prefix_length: Option<u8>,
    /// Whether DHCP is enabled for IPv4.
    pub ipv4_dhcp: bool,
    /// MAC address.
    pub mac_address: Option<String>,
    /// Link speed in Mbps (if available).
    pub link_speed: Option<u32>,
}

/// DNS configuration information.
#[derive(Debug, Clone, Default)]
pub struct DnsInfo {
    /// Whether DNS is obtained from DHCP.
    pub from_dhcp: bool,
    /// DNS search domains.
    pub search_domains: Vec<String>,
    /// DNS server addresses (obtained via DHCP).
    pub dns_from_dhcp: Vec<String>,
    /// Manually configured DNS server addresses.
    pub dns_manual: Vec<String>,
}

/// NTP configuration information.
#[derive(Debug, Clone, Default)]
pub struct NtpInfo {
    /// Whether NTP is obtained from DHCP.
    pub from_dhcp: bool,
    /// NTP servers obtained via DHCP.
    pub ntp_from_dhcp: Vec<String>,
    /// Manually configured NTP servers.
    pub ntp_manual: Vec<String>,
}

/// Network protocol information.
#[derive(Debug, Clone)]
pub struct NetworkProtocolInfo {
    /// Protocol name (HTTP, HTTPS, RTSP).
    pub name: String,
    /// Whether the protocol is enabled.
    pub enabled: bool,
    /// Port numbers.
    pub ports: Vec<u16>,
}

/// Network information trait for querying system network configuration.
#[cfg_attr(test, automock)]
#[async_trait]
pub trait NetworkInfo: Send + Sync {
    /// Get all network interfaces.
    async fn get_network_interfaces(&self) -> PlatformResult<Vec<NetworkInterfaceInfo>>;

    /// Get DNS configuration.
    async fn get_dns_info(&self) -> PlatformResult<DnsInfo>;

    /// Get NTP configuration.
    async fn get_ntp_info(&self) -> PlatformResult<NtpInfo>;

    /// Get enabled network protocols.
    async fn get_network_protocols(&self) -> PlatformResult<Vec<NetworkProtocolInfo>>;

    /// Detect the local IP address.
    ///
    /// Uses UDP socket trick to determine the outbound IP address without
    /// actually sending any packets. Falls back to first interface IP if
    /// socket method fails.
    fn detect_local_ip(&self) -> Option<String> {
        use std::net::UdpSocket;
        match UdpSocket::bind("0.0.0.0:0") {
            Ok(socket) => {
                if socket.connect("8.8.8.8:80").is_ok() {
                    if let Ok(addr) = socket.local_addr() {
                        let ip = addr.ip().to_string();
                        if ip != "0.0.0.0" {
                            return Some(ip);
                        }
                    }
                }
                None
            }
            Err(_) => None,
        }
    }

    /// Set network interface configuration (stub - platform may not support).
    async fn set_network_interface(
        &self,
        _token: &str,
        _ipv4_address: Option<String>,
        _ipv4_prefix_length: Option<u8>,
        _ipv4_dhcp: bool,
    ) -> PlatformResult<()> {
        Err(PlatformError::NotSupported(
            "set_network_interface".to_string(),
        ))
    }

    /// Set DNS configuration (stub - platform may not support).
    async fn set_dns(&self, _dns_servers: &[String], _search_domains: &[String]) -> PlatformResult<()> {
        Err(PlatformError::NotSupported("set_dns".to_string()))
    }

    /// Set default gateway (stub - platform may not support).
    async fn set_gateway(&self, _gateway: &str) -> PlatformResult<()> {
        Err(PlatformError::NotSupported("set_gateway".to_string()))
    }
}

/// Main platform trait combining all hardware abstractions.
#[async_trait]
pub trait Platform: Send + Sync {
    /// Get device information.
    async fn get_device_info(&self) -> PlatformResult<DeviceInfo>;

    /// Get video input interface.
    fn video_input(&self) -> Arc<dyn VideoInput>;

    /// Get video encoder interface.
    fn video_encoder(&self) -> Arc<dyn VideoEncoder>;

    /// Get audio input interface.
    fn audio_input(&self) -> Arc<dyn AudioInput>;

    /// Get audio encoder interface.
    fn audio_encoder(&self) -> Arc<dyn AudioEncoder>;

    /// Get PTZ control interface (optional).
    fn ptz_control(&self) -> Option<Arc<dyn PTZControl>>;

    /// Get imaging control interface (optional).
    fn imaging_control(&self) -> Option<Arc<dyn ImagingControl>>;

    /// Get network information interface (optional).
    fn network_info(&self) -> Option<Arc<dyn NetworkInfo>>;

    /// Check if the platform is initialized.
    fn is_initialized(&self) -> bool;

    /// Initialize the platform.
    async fn initialize(&self) -> PlatformResult<()>;

    /// Shutdown the platform.
    async fn shutdown(&self) -> PlatformResult<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolution_new() {
        let res = Resolution::new(1920, 1080);
        assert_eq!(res.width, 1920);
        assert_eq!(res.height, 1080);
    }

    #[test]
    fn test_ptz_position_home() {
        assert_eq!(PtzPosition::HOME.pan, 0.0);
        assert_eq!(PtzPosition::HOME.tilt, 0.0);
        assert_eq!(PtzPosition::HOME.zoom, 1.0);
    }

    #[test]
    fn test_ptz_velocity_stop() {
        assert_eq!(PtzVelocity::STOP.pan, 0.0);
        assert_eq!(PtzVelocity::STOP.tilt, 0.0);
        assert_eq!(PtzVelocity::STOP.zoom, 0.0);
    }

    #[test]
    fn test_ptz_limits_default() {
        let limits = PtzLimits::DEFAULT;
        assert_eq!(limits.min_pan, -180.0);
        assert_eq!(limits.max_pan, 180.0);
        assert_eq!(limits.min_tilt, -90.0);
        assert_eq!(limits.max_tilt, 90.0);
    }

    #[test]
    fn test_imaging_options_default() {
        let opts = ImagingOptions::default_options();
        assert_eq!(opts.brightness_range, (0.0, 100.0));
        assert!(opts.ir_cut_filter_supported);
    }
}
