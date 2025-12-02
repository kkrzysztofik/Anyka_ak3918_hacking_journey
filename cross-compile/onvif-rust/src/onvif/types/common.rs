//! Common ONVIF schema types from onvif.xsd (tt:* namespace).
//!
//! This module contains the core types shared across all ONVIF services.
//! These types are defined in the `http://www.onvif.org/ver10/schema` namespace.
//!
//! # Type Categories
//!
//! - **Basic Types**: `ReferenceToken`, `Name`, primitive wrappers
//! - **Range Types**: `IntRange`, `FloatRange`, `DurationRange`
//! - **Geometry Types**: `IntRectangle`, `VideoResolution`, `Vector2D`, `Vector1D`
//! - **Media Types**: `VideoSource`, `AudioSource`, `Profile`, configurations
//! - **PTZ Types**: `PTZConfiguration`, `PTZSpeed`, `PTZVector`, `PTZStatus`
//! - **Imaging Types**: `ImagingSettings20`, `Exposure20`, `FocusConfiguration20`
//! - **Date/Time Types**: `DateTime`, `TimeZone`, `SystemDateTime`

use serde::{Deserialize, Serialize};

use super::Extension;

// ============================================================================
// Basic Types
// ============================================================================

/// Reference token type - unique identifier for ONVIF entities.
///
/// Used to reference profiles, configurations, sources, etc.
/// Length up to 64 characters.
pub type ReferenceToken = String;

/// Name type - user readable name.
///
/// Length up to 64 characters.
pub type Name = String;

/// ONVIF version information.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OnvifVersion {
    /// Major version number.
    #[serde(rename = "tt:Major", alias = "Major")]
    pub major: i32,

    /// Minor version number.
    #[serde(rename = "tt:Minor", alias = "Minor")]
    pub minor: i32,
}

impl Default for OnvifVersion {
    fn default() -> Self {
        Self { major: 2, minor: 5 }
    }
}

// ============================================================================
// Range Types
// ============================================================================

/// Integer range with min/max values.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct IntRange {
    /// Minimum value.
    #[serde(rename = "Min")]
    pub min: i32,

    /// Maximum value.
    #[serde(rename = "Max")]
    pub max: i32,
}

/// Float range with min/max values.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct FloatRange {
    /// Minimum value.
    #[serde(rename = "Min")]
    pub min: f32,

    /// Maximum value.
    #[serde(rename = "Max")]
    pub max: f32,
}

/// Duration range with min/max values.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DurationRange {
    /// Minimum duration (ISO 8601 format).
    #[serde(rename = "Min")]
    pub min: String,

    /// Maximum duration (ISO 8601 format).
    #[serde(rename = "Max")]
    pub max: String,
}

/// List of integers.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct IntItems {
    /// Integer items.
    #[serde(rename = "Items", default)]
    pub items: Vec<i32>,
}

/// List of floats.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct FloatItems {
    /// Float items.
    #[serde(rename = "Items", default)]
    pub items: Vec<f32>,
}

// ============================================================================
// Geometry Types
// ============================================================================

/// Integer rectangle defined by position and size.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IntRectangle {
    /// X coordinate.
    #[serde(rename = "@x")]
    pub x: i32,

    /// Y coordinate.
    #[serde(rename = "@y")]
    pub y: i32,

    /// Width.
    #[serde(rename = "@width")]
    pub width: i32,

    /// Height.
    #[serde(rename = "@height")]
    pub height: i32,
}

impl Default for IntRectangle {
    fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        }
    }
}

/// Integer rectangle range.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IntRectangleRange {
    /// X-axis range.
    #[serde(rename = "XRange")]
    pub x_range: IntRange,

    /// Y-axis range.
    #[serde(rename = "YRange")]
    pub y_range: IntRange,

    /// Width range.
    #[serde(rename = "WidthRange")]
    pub width_range: IntRange,

    /// Height range.
    #[serde(rename = "HeightRange")]
    pub height_range: IntRange,
}

/// Video resolution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VideoResolution {
    /// Horizontal resolution in pixels.
    #[serde(rename = "Width")]
    pub width: i32,

    /// Vertical resolution in pixels.
    #[serde(rename = "Height")]
    pub height: i32,
}

impl Default for VideoResolution {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
        }
    }
}

/// 2D vector for PTZ movements.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Vector2D {
    /// X component.
    #[serde(rename = "@x")]
    pub x: f32,

    /// Y component.
    #[serde(rename = "@y")]
    pub y: f32,

    /// Coordinate space URI (optional).
    #[serde(rename = "@space", default, skip_serializing_if = "Option::is_none")]
    pub space: Option<String>,
}

impl Default for Vector2D {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            space: None,
        }
    }
}

/// 1D vector for zoom movements.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Vector1D {
    /// X component.
    #[serde(rename = "@x")]
    pub x: f32,

    /// Coordinate space URI (optional).
    #[serde(rename = "@space", default, skip_serializing_if = "Option::is_none")]
    pub space: Option<String>,
}

impl Default for Vector1D {
    fn default() -> Self {
        Self {
            x: 0.0,
            space: None,
        }
    }
}

// ============================================================================
// Date/Time Types
// ============================================================================

/// Time structure.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Time {
    /// Hour (0-23).
    #[serde(rename = "tt:Hour", alias = "Hour")]
    pub hour: i32,

    /// Minute (0-59).
    #[serde(rename = "tt:Minute", alias = "Minute")]
    pub minute: i32,

    /// Second (0-59).
    #[serde(rename = "tt:Second", alias = "Second")]
    pub second: i32,
}

/// Date structure.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Date {
    /// Year.
    #[serde(rename = "tt:Year", alias = "Year")]
    pub year: i32,

    /// Month (1-12).
    #[serde(rename = "tt:Month", alias = "Month")]
    pub month: i32,

    /// Day (1-31).
    #[serde(rename = "tt:Day", alias = "Day")]
    pub day: i32,
}

/// Date and time structure.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DateTime {
    /// Time component.
    #[serde(rename = "tt:Time", alias = "Time")]
    pub time: Time,

    /// Date component.
    #[serde(rename = "tt:Date", alias = "Date")]
    pub date: Date,
}

/// Timezone in POSIX format.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimeZone {
    /// Timezone string in POSIX 1003.1 format.
    #[serde(rename = "tt:TZ", alias = "TZ")]
    pub tz: String,
}

impl Default for TimeZone {
    fn default() -> Self {
        Self {
            tz: "UTC0".to_string(),
        }
    }
}

/// Date/time type enumeration.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum SetDateTimeType {
    /// Manual date/time setting.
    #[default]
    Manual,
    /// NTP synchronized.
    NTP,
}

/// System date and time information.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SystemDateTime {
    /// How date/time is set (Manual or NTP).
    #[serde(rename = "tt:DateTimeType", alias = "DateTimeType")]
    pub date_time_type: SetDateTimeType,

    /// Daylight savings enabled.
    #[serde(rename = "tt:DaylightSavings", alias = "DaylightSavings")]
    pub daylight_savings: bool,

    /// Timezone information.
    #[serde(
        rename = "tt:TimeZone",
        alias = "TimeZone",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub time_zone: Option<TimeZone>,

    /// UTC date and time.
    #[serde(
        rename = "tt:UTCDateTime",
        alias = "UTCDateTime",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub utc_date_time: Option<DateTime>,

    /// Local date and time.
    #[serde(
        rename = "tt:LocalDateTime",
        alias = "LocalDateTime",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub local_date_time: Option<DateTime>,

    /// Extension.
    #[serde(
        rename = "tt:Extension",
        alias = "Extension",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub extension: Option<Extension>,
}

// ============================================================================
// Device Entity Base Types
// ============================================================================

/// Base type for physical entities with a token.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceEntity {
    /// Unique identifier token.
    #[serde(rename = "@token")]
    pub token: ReferenceToken,
}

// ============================================================================
// Video Source Types
// ============================================================================

/// Physical video input representation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VideoSource {
    /// Unique identifier token.
    #[serde(rename = "@token")]
    pub token: ReferenceToken,

    /// Frame rate in frames per second.
    #[serde(rename = "Framerate")]
    pub framerate: f32,

    /// Resolution.
    #[serde(rename = "Resolution")]
    pub resolution: VideoResolution,

    /// Optional imaging settings.
    #[serde(rename = "Imaging", default, skip_serializing_if = "Option::is_none")]
    pub imaging: Option<ImagingSettings>,

    /// Extension.
    #[serde(rename = "Extension", default, skip_serializing_if = "Option::is_none")]
    pub extension: Option<VideoSourceExtension>,
}

/// Video source extension.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct VideoSourceExtension {
    /// Imaging settings 2.0.
    #[serde(rename = "Imaging", default, skip_serializing_if = "Option::is_none")]
    pub imaging: Option<ImagingSettings20>,

    /// Further extension.
    #[serde(rename = "Extension", default, skip_serializing_if = "Option::is_none")]
    pub extension: Option<Extension>,
}

// ============================================================================
// Audio Source Types
// ============================================================================

/// Physical audio input representation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioSource {
    /// Unique identifier token.
    #[serde(rename = "@token")]
    pub token: ReferenceToken,

    /// Number of audio channels (1=mono, 2=stereo).
    #[serde(rename = "Channels")]
    pub channels: i32,
}

// ============================================================================
// Configuration Entity Types
// ============================================================================

/// Base configuration properties.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConfigurationEntity {
    /// Configuration token.
    #[serde(rename = "@token")]
    pub token: ReferenceToken,

    /// User readable name.
    #[serde(rename = "Name")]
    pub name: Name,

    /// Number of references using this configuration.
    #[serde(rename = "UseCount")]
    pub use_count: i32,
}

/// Video source configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VideoSourceConfiguration {
    /// Configuration token.
    #[serde(rename = "@token")]
    pub token: ReferenceToken,

    /// User readable name.
    #[serde(rename = "Name")]
    pub name: Name,

    /// Number of references using this configuration.
    #[serde(rename = "UseCount")]
    pub use_count: i32,

    /// View mode.
    #[serde(rename = "@ViewMode", default, skip_serializing_if = "Option::is_none")]
    pub view_mode: Option<String>,

    /// Reference to the physical video source.
    #[serde(rename = "SourceToken")]
    pub source_token: ReferenceToken,

    /// Capturing area bounds.
    #[serde(rename = "Bounds")]
    pub bounds: IntRectangle,

    /// Extension.
    #[serde(rename = "Extension", default, skip_serializing_if = "Option::is_none")]
    pub extension: Option<Extension>,
}

/// Audio source configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioSourceConfiguration {
    /// Configuration token.
    #[serde(rename = "@token")]
    pub token: ReferenceToken,

    /// User readable name.
    #[serde(rename = "Name")]
    pub name: Name,

    /// Number of references using this configuration.
    #[serde(rename = "UseCount")]
    pub use_count: i32,

    /// Reference to the physical audio source.
    #[serde(rename = "SourceToken")]
    pub source_token: ReferenceToken,
}

// ============================================================================
// Video Encoder Types
// ============================================================================

/// Video encoding enumeration.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum VideoEncoding {
    /// JPEG encoding.
    JPEG,
    /// MPEG4 encoding.
    MPEG4,
    /// H.264 encoding.
    #[default]
    H264,
}

/// H.264 profile enumeration.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum H264Profile {
    Baseline,
    #[default]
    Main,
    Extended,
    High,
}

/// MPEG4 profile enumeration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Mpeg4Profile {
    SP,
    ASP,
}

/// Rate control settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VideoRateControl {
    /// Maximum frame rate.
    #[serde(rename = "FrameRateLimit")]
    pub frame_rate_limit: i32,

    /// Encoding interval (key frame interval).
    #[serde(rename = "EncodingInterval")]
    pub encoding_interval: i32,

    /// Bitrate limit in kbps.
    #[serde(rename = "BitrateLimit")]
    pub bitrate_limit: i32,
}

impl Default for VideoRateControl {
    fn default() -> Self {
        Self {
            frame_rate_limit: 25,
            encoding_interval: 1,
            bitrate_limit: 4096,
        }
    }
}

/// H.264 configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct H264Configuration {
    /// GOP length.
    #[serde(rename = "GovLength")]
    pub gov_length: i32,

    /// H.264 profile.
    #[serde(rename = "H264Profile")]
    pub h264_profile: H264Profile,
}

impl Default for H264Configuration {
    fn default() -> Self {
        Self {
            gov_length: 30,
            h264_profile: H264Profile::Main,
        }
    }
}

/// MPEG4 configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Mpeg4Configuration {
    /// GOP length.
    #[serde(rename = "GovLength")]
    pub gov_length: i32,

    /// MPEG4 profile.
    #[serde(rename = "Mpeg4Profile")]
    pub mpeg4_profile: Mpeg4Profile,
}

/// Multicast configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MulticastConfiguration {
    /// Multicast address.
    #[serde(rename = "Address")]
    pub address: IpAddress,

    /// Port number.
    #[serde(rename = "Port")]
    pub port: i32,

    /// TTL value.
    #[serde(rename = "TTL")]
    pub ttl: i32,

    /// Auto-start multicast.
    #[serde(rename = "AutoStart")]
    pub auto_start: bool,
}

/// IP address structure.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IpAddress {
    /// Address type (IPv4 or IPv6).
    #[serde(rename = "Type")]
    pub address_type: IpType,

    /// IPv4 address.
    #[serde(
        rename = "IPv4Address",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub ipv4_address: Option<String>,

    /// IPv6 address.
    #[serde(
        rename = "IPv6Address",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub ipv6_address: Option<String>,
}

/// IP address type.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum IpType {
    #[default]
    IPv4,
    IPv6,
}

/// Video encoder configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VideoEncoderConfiguration {
    /// Configuration token.
    #[serde(rename = "@token")]
    pub token: ReferenceToken,

    /// User readable name.
    #[serde(rename = "Name")]
    pub name: Name,

    /// Number of references using this configuration.
    #[serde(rename = "UseCount")]
    pub use_count: i32,

    /// Encoding type.
    #[serde(rename = "Encoding")]
    pub encoding: VideoEncoding,

    /// Resolution.
    #[serde(rename = "Resolution")]
    pub resolution: VideoResolution,

    /// Video quality (1-100).
    #[serde(rename = "Quality")]
    pub quality: f32,

    /// Rate control settings.
    #[serde(
        rename = "RateControl",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub rate_control: Option<VideoRateControl>,

    /// MPEG4 configuration.
    #[serde(rename = "MPEG4", default, skip_serializing_if = "Option::is_none")]
    pub mpeg4: Option<Mpeg4Configuration>,

    /// H.264 configuration.
    #[serde(rename = "H264", default, skip_serializing_if = "Option::is_none")]
    pub h264: Option<H264Configuration>,

    /// Multicast settings.
    #[serde(rename = "Multicast", default, skip_serializing_if = "Option::is_none")]
    pub multicast: Option<MulticastConfiguration>,

    /// Session timeout.
    #[serde(rename = "SessionTimeout")]
    pub session_timeout: String,
}

// ============================================================================
// Audio Encoder Types
// ============================================================================

/// Audio encoding enumeration.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum AudioEncoding {
    #[default]
    G711,
    G726,
    AAC,
}

/// Audio encoder configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioEncoderConfiguration {
    /// Configuration token.
    #[serde(rename = "@token")]
    pub token: ReferenceToken,

    /// User readable name.
    #[serde(rename = "Name")]
    pub name: Name,

    /// Number of references using this configuration.
    #[serde(rename = "UseCount")]
    pub use_count: i32,

    /// Encoding type.
    #[serde(rename = "Encoding")]
    pub encoding: AudioEncoding,

    /// Bitrate in kbps.
    #[serde(rename = "Bitrate")]
    pub bitrate: i32,

    /// Sample rate in kHz.
    #[serde(rename = "SampleRate")]
    pub sample_rate: i32,

    /// Multicast settings.
    #[serde(rename = "Multicast", default, skip_serializing_if = "Option::is_none")]
    pub multicast: Option<MulticastConfiguration>,

    /// Session timeout.
    #[serde(rename = "SessionTimeout")]
    pub session_timeout: String,
}

// ============================================================================
// PTZ Types
// ============================================================================

/// PTZ configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PTZConfiguration {
    /// Configuration token.
    #[serde(rename = "@token")]
    pub token: ReferenceToken,

    /// User readable name.
    #[serde(rename = "Name")]
    pub name: Name,

    /// Number of references using this configuration.
    #[serde(rename = "UseCount")]
    pub use_count: i32,

    /// Move ramp (acceleration).
    #[serde(rename = "@MoveRamp", default, skip_serializing_if = "Option::is_none")]
    pub move_ramp: Option<i32>,

    /// Preset ramp.
    #[serde(
        rename = "@PresetRamp",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub preset_ramp: Option<i32>,

    /// Preset tour ramp.
    #[serde(
        rename = "@PresetTourRamp",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub preset_tour_ramp: Option<i32>,

    /// Reference to PTZ node.
    #[serde(rename = "NodeToken")]
    pub node_token: ReferenceToken,

    /// Default absolute pan/tilt position space.
    #[serde(
        rename = "DefaultAbsolutePantTiltPositionSpace",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub default_absolute_pan_tilt_position_space: Option<String>,

    /// Default absolute zoom position space.
    #[serde(
        rename = "DefaultAbsoluteZoomPositionSpace",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub default_absolute_zoom_position_space: Option<String>,

    /// Default relative pan/tilt translation space.
    #[serde(
        rename = "DefaultRelativePanTiltTranslationSpace",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub default_relative_pan_tilt_translation_space: Option<String>,

    /// Default relative zoom translation space.
    #[serde(
        rename = "DefaultRelativeZoomTranslationSpace",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub default_relative_zoom_translation_space: Option<String>,

    /// Default continuous pan/tilt velocity space.
    #[serde(
        rename = "DefaultContinuousPanTiltVelocitySpace",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub default_continuous_pan_tilt_velocity_space: Option<String>,

    /// Default continuous zoom velocity space.
    #[serde(
        rename = "DefaultContinuousZoomVelocitySpace",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub default_continuous_zoom_velocity_space: Option<String>,

    /// Default PTZ speed.
    #[serde(
        rename = "DefaultPTZSpeed",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub default_ptz_speed: Option<PTZSpeed>,

    /// Default PTZ timeout.
    #[serde(
        rename = "DefaultPTZTimeout",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub default_ptz_timeout: Option<String>,

    /// Pan/tilt limits.
    #[serde(
        rename = "PanTiltLimits",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub pan_tilt_limits: Option<PanTiltLimits>,

    /// Zoom limits.
    #[serde(
        rename = "ZoomLimits",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub zoom_limits: Option<ZoomLimits>,

    /// Extension.
    #[serde(rename = "Extension", default, skip_serializing_if = "Option::is_none")]
    pub extension: Option<PTZConfigurationExtension>,
}

/// PTZ configuration extension.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PTZConfigurationExtension {
    /// PT control direction settings.
    #[serde(
        rename = "PTControlDirection",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub pt_control_direction: Option<PTControlDirection>,

    /// Extension (for future use).
    #[serde(rename = "Extension", default, skip_serializing_if = "Option::is_none")]
    pub extension: Option<Extension>,
}

/// PT control direction settings (EFlip and Reverse modes).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PTControlDirection {
    /// EFlip mode.
    #[serde(rename = "EFlip", default, skip_serializing_if = "Option::is_none")]
    pub e_flip: Option<EFlip>,

    /// Reverse mode.
    #[serde(rename = "Reverse", default, skip_serializing_if = "Option::is_none")]
    pub reverse: Option<Reverse>,

    /// Extension.
    #[serde(rename = "Extension", default, skip_serializing_if = "Option::is_none")]
    pub extension: Option<Extension>,
}

/// EFlip settings.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct EFlip {
    /// EFlip mode.
    #[serde(rename = "Mode")]
    pub mode: EFlipMode,
}

/// Reverse settings.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Reverse {
    /// Reverse mode.
    #[serde(rename = "Mode")]
    pub mode: ReverseMode,
}

/// EFlip mode enumeration.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum EFlipMode {
    #[default]
    OFF,
    ON,
    Extended,
}

/// Reverse mode enumeration.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum ReverseMode {
    #[default]
    OFF,
    ON,
    AUTO,
    Extended,
}

/// PTZ speed.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PTZSpeed {
    /// Pan/tilt speed.
    #[serde(rename = "PanTilt", default, skip_serializing_if = "Option::is_none")]
    pub pan_tilt: Option<Vector2D>,

    /// Zoom speed.
    #[serde(rename = "Zoom", default, skip_serializing_if = "Option::is_none")]
    pub zoom: Option<Vector1D>,
}

/// PTZ vector (position).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PTZVector {
    /// Pan/tilt position.
    #[serde(rename = "PanTilt", default, skip_serializing_if = "Option::is_none")]
    pub pan_tilt: Option<Vector2D>,

    /// Zoom position.
    #[serde(rename = "Zoom", default, skip_serializing_if = "Option::is_none")]
    pub zoom: Option<Vector1D>,
}

/// Pan/tilt limits.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PanTiltLimits {
    /// Range of pan/tilt.
    #[serde(rename = "Range")]
    pub range: Space2DDescription,
}

/// Zoom limits.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ZoomLimits {
    /// Range of zoom.
    #[serde(rename = "Range")]
    pub range: Space1DDescription,
}

/// 2D space description.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Space2DDescription {
    /// Space URI.
    #[serde(rename = "URI")]
    pub uri: String,

    /// X range.
    #[serde(rename = "XRange")]
    pub x_range: FloatRange,

    /// Y range.
    #[serde(rename = "YRange")]
    pub y_range: FloatRange,
}

/// 1D space description.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Space1DDescription {
    /// Space URI.
    #[serde(rename = "URI")]
    pub uri: String,

    /// X range.
    #[serde(rename = "XRange")]
    pub x_range: FloatRange,
}

/// PTZ status.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PTZStatus {
    /// Current position.
    #[serde(rename = "Position", default, skip_serializing_if = "Option::is_none")]
    pub position: Option<PTZVector>,

    /// Move status.
    #[serde(
        rename = "MoveStatus",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub move_status: Option<PTZMoveStatus>,

    /// Error message.
    #[serde(rename = "Error", default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    /// UTC time.
    #[serde(rename = "UtcTime")]
    pub utc_time: String,
}

/// PTZ move status.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PTZMoveStatus {
    /// Pan/tilt status.
    #[serde(rename = "PanTilt", default, skip_serializing_if = "Option::is_none")]
    pub pan_tilt: Option<MoveStatus>,

    /// Zoom status.
    #[serde(rename = "Zoom", default, skip_serializing_if = "Option::is_none")]
    pub zoom: Option<MoveStatus>,
}

/// Move status enumeration.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum MoveStatus {
    #[default]
    IDLE,
    MOVING,
    UNKNOWN,
}

/// PTZ preset.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PTZPreset {
    /// Preset token.
    #[serde(rename = "@token", default, skip_serializing_if = "Option::is_none")]
    pub token: Option<ReferenceToken>,

    /// Preset name.
    #[serde(rename = "Name", default, skip_serializing_if = "Option::is_none")]
    pub name: Option<Name>,

    /// Preset position.
    #[serde(
        rename = "PTZPosition",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub ptz_position: Option<PTZVector>,
}

// ============================================================================
// Imaging Types
// ============================================================================

/// Imaging settings (version 1.0).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImagingSettings {
    /// Backlight compensation.
    #[serde(
        rename = "BacklightCompensation",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub backlight_compensation: Option<BacklightCompensation>,

    /// Brightness (0-100).
    #[serde(
        rename = "Brightness",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub brightness: Option<f32>,

    /// Color saturation (0-100).
    #[serde(
        rename = "ColorSaturation",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub color_saturation: Option<f32>,

    /// Contrast (0-100).
    #[serde(rename = "Contrast", default, skip_serializing_if = "Option::is_none")]
    pub contrast: Option<f32>,

    /// Exposure settings.
    #[serde(rename = "Exposure", default, skip_serializing_if = "Option::is_none")]
    pub exposure: Option<Exposure>,

    /// Focus settings.
    #[serde(rename = "Focus", default, skip_serializing_if = "Option::is_none")]
    pub focus: Option<FocusConfiguration>,

    /// IR cut filter mode.
    #[serde(
        rename = "IrCutFilter",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub ir_cut_filter: Option<IrCutFilterMode>,

    /// Sharpness (0-100).
    #[serde(rename = "Sharpness", default, skip_serializing_if = "Option::is_none")]
    pub sharpness: Option<f32>,

    /// Wide dynamic range.
    #[serde(
        rename = "WideDynamicRange",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub wide_dynamic_range: Option<WideDynamicRange>,

    /// White balance.
    #[serde(
        rename = "WhiteBalance",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub white_balance: Option<WhiteBalance>,

    /// Extension.
    #[serde(rename = "Extension", default, skip_serializing_if = "Option::is_none")]
    pub extension: Option<Extension>,
}

/// Imaging settings version 2.0.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ImagingSettings20 {
    /// Backlight compensation.
    #[serde(
        rename = "BacklightCompensation",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub backlight_compensation: Option<BacklightCompensation20>,

    /// Brightness (0-100).
    #[serde(
        rename = "Brightness",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub brightness: Option<f32>,

    /// Color saturation (0-100).
    #[serde(
        rename = "ColorSaturation",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub color_saturation: Option<f32>,

    /// Contrast (0-100).
    #[serde(rename = "Contrast", default, skip_serializing_if = "Option::is_none")]
    pub contrast: Option<f32>,

    /// Exposure settings.
    #[serde(rename = "Exposure", default, skip_serializing_if = "Option::is_none")]
    pub exposure: Option<Exposure20>,

    /// Focus settings.
    #[serde(rename = "Focus", default, skip_serializing_if = "Option::is_none")]
    pub focus: Option<FocusConfiguration20>,

    /// IR cut filter mode.
    #[serde(
        rename = "IrCutFilter",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub ir_cut_filter: Option<IrCutFilterMode>,

    /// Sharpness (0-100).
    #[serde(rename = "Sharpness", default, skip_serializing_if = "Option::is_none")]
    pub sharpness: Option<f32>,

    /// Wide dynamic range.
    #[serde(
        rename = "WideDynamicRange",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub wide_dynamic_range: Option<WideDynamicRange20>,

    /// White balance.
    #[serde(
        rename = "WhiteBalance",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub white_balance: Option<WhiteBalance20>,

    /// Extension.
    #[serde(rename = "Extension", default, skip_serializing_if = "Option::is_none")]
    pub extension: Option<ImagingSettingsExtension20>,
}

/// Imaging settings extension 2.0.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ImagingSettingsExtension20 {
    /// Image stabilization.
    #[serde(
        rename = "ImageStabilization",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub image_stabilization: Option<ImageStabilization>,

    /// Extension.
    #[serde(rename = "Extension", default, skip_serializing_if = "Option::is_none")]
    pub extension: Option<Extension>,
}

/// Imaging status version 2.0.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ImagingStatus20 {
    /// Focus status.
    #[serde(
        rename = "FocusStatus20",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub focus_status20: Option<FocusStatus20>,

    /// Extension.
    #[serde(rename = "Extension", default, skip_serializing_if = "Option::is_none")]
    pub extension: Option<Extension>,
}

/// Focus status version 2.0.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct FocusStatus20 {
    /// Current focus position (normalized 0-1).
    #[serde(rename = "Position")]
    pub position: f32,

    /// Move status.
    #[serde(rename = "MoveStatus")]
    pub move_status: MoveStatus,

    /// Error description if any.
    #[serde(rename = "Error", default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    /// Extension.
    #[serde(rename = "Extension", default, skip_serializing_if = "Option::is_none")]
    pub extension: Option<Extension>,
}

/// Image stabilization settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageStabilization {
    /// Mode (ON/OFF/AUTO).
    #[serde(rename = "Mode")]
    pub mode: ImageStabilizationMode,

    /// Level.
    #[serde(rename = "Level", default, skip_serializing_if = "Option::is_none")]
    pub level: Option<f32>,

    /// Extension.
    #[serde(rename = "Extension", default, skip_serializing_if = "Option::is_none")]
    pub extension: Option<Extension>,
}

/// Image stabilization mode.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ImageStabilizationMode {
    OFF,
    ON,
    AUTO,
    Extended,
}

/// Backlight compensation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BacklightCompensation {
    /// Mode (ON/OFF).
    #[serde(rename = "Mode")]
    pub mode: BacklightCompensationMode,

    /// Level.
    #[serde(rename = "Level")]
    pub level: f32,
}

/// Backlight compensation 2.0.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BacklightCompensation20 {
    /// Mode.
    #[serde(rename = "Mode")]
    pub mode: BacklightCompensationMode,

    /// Level.
    #[serde(rename = "Level", default, skip_serializing_if = "Option::is_none")]
    pub level: Option<f32>,
}

/// Backlight compensation mode.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum BacklightCompensationMode {
    #[default]
    OFF,
    ON,
}

/// Exposure settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Exposure {
    /// Mode (AUTO/MANUAL).
    #[serde(rename = "Mode")]
    pub mode: ExposureMode,

    /// Priority.
    #[serde(rename = "Priority")]
    pub priority: ExposurePriority,

    /// Exposure window.
    #[serde(rename = "Window")]
    pub window: Rectangle,

    /// Minimum exposure time.
    #[serde(rename = "MinExposureTime")]
    pub min_exposure_time: f32,

    /// Maximum exposure time.
    #[serde(rename = "MaxExposureTime")]
    pub max_exposure_time: f32,

    /// Minimum gain.
    #[serde(rename = "MinGain")]
    pub min_gain: f32,

    /// Maximum gain.
    #[serde(rename = "MaxGain")]
    pub max_gain: f32,

    /// Minimum iris.
    #[serde(rename = "MinIris")]
    pub min_iris: f32,

    /// Maximum iris.
    #[serde(rename = "MaxIris")]
    pub max_iris: f32,

    /// Exposure time.
    #[serde(rename = "ExposureTime")]
    pub exposure_time: f32,

    /// Gain.
    #[serde(rename = "Gain")]
    pub gain: f32,

    /// Iris.
    #[serde(rename = "Iris")]
    pub iris: f32,
}

/// Exposure settings 2.0.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Exposure20 {
    /// Mode (AUTO/MANUAL).
    #[serde(rename = "Mode")]
    pub mode: ExposureMode,

    /// Priority.
    #[serde(rename = "Priority", default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<ExposurePriority>,

    /// Exposure window.
    #[serde(rename = "Window", default, skip_serializing_if = "Option::is_none")]
    pub window: Option<Rectangle>,

    /// Minimum exposure time.
    #[serde(
        rename = "MinExposureTime",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub min_exposure_time: Option<f32>,

    /// Maximum exposure time.
    #[serde(
        rename = "MaxExposureTime",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub max_exposure_time: Option<f32>,

    /// Minimum gain.
    #[serde(rename = "MinGain", default, skip_serializing_if = "Option::is_none")]
    pub min_gain: Option<f32>,

    /// Maximum gain.
    #[serde(rename = "MaxGain", default, skip_serializing_if = "Option::is_none")]
    pub max_gain: Option<f32>,

    /// Minimum iris.
    #[serde(rename = "MinIris", default, skip_serializing_if = "Option::is_none")]
    pub min_iris: Option<f32>,

    /// Maximum iris.
    #[serde(rename = "MaxIris", default, skip_serializing_if = "Option::is_none")]
    pub max_iris: Option<f32>,

    /// Exposure time.
    #[serde(
        rename = "ExposureTime",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub exposure_time: Option<f32>,

    /// Gain.
    #[serde(rename = "Gain", default, skip_serializing_if = "Option::is_none")]
    pub gain: Option<f32>,

    /// Iris.
    #[serde(rename = "Iris", default, skip_serializing_if = "Option::is_none")]
    pub iris: Option<f32>,
}

/// Exposure mode.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum ExposureMode {
    #[default]
    AUTO,
    MANUAL,
}

/// Exposure priority.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum ExposurePriority {
    #[default]
    LowNoise,
    FrameRate,
}

/// Rectangle for exposure window.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rectangle {
    /// Bottom coordinate.
    #[serde(rename = "@bottom")]
    pub bottom: f32,

    /// Top coordinate.
    #[serde(rename = "@top")]
    pub top: f32,

    /// Right coordinate.
    #[serde(rename = "@right")]
    pub right: f32,

    /// Left coordinate.
    #[serde(rename = "@left")]
    pub left: f32,
}

impl Default for Rectangle {
    fn default() -> Self {
        Self {
            bottom: 1.0,
            top: 0.0,
            right: 1.0,
            left: 0.0,
        }
    }
}

/// Focus configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FocusConfiguration {
    /// Auto-focus mode.
    #[serde(rename = "AutoFocusMode")]
    pub auto_focus_mode: AutoFocusMode,

    /// Default speed.
    #[serde(rename = "DefaultSpeed")]
    pub default_speed: f32,

    /// Near limit.
    #[serde(rename = "NearLimit")]
    pub near_limit: f32,

    /// Far limit.
    #[serde(rename = "FarLimit")]
    pub far_limit: f32,
}

/// Focus configuration 2.0.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FocusConfiguration20 {
    /// Auto-focus mode.
    #[serde(rename = "AutoFocusMode")]
    pub auto_focus_mode: AutoFocusMode,

    /// Default speed.
    #[serde(
        rename = "DefaultSpeed",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub default_speed: Option<f32>,

    /// Near limit.
    #[serde(rename = "NearLimit", default, skip_serializing_if = "Option::is_none")]
    pub near_limit: Option<f32>,

    /// Far limit.
    #[serde(rename = "FarLimit", default, skip_serializing_if = "Option::is_none")]
    pub far_limit: Option<f32>,

    /// Extension.
    #[serde(rename = "Extension", default, skip_serializing_if = "Option::is_none")]
    pub extension: Option<Extension>,
}

/// Auto-focus mode.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum AutoFocusMode {
    #[default]
    AUTO,
    MANUAL,
}

/// IR cut filter mode.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum IrCutFilterMode {
    ON,
    OFF,
    #[default]
    AUTO,
}

/// Wide dynamic range.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WideDynamicRange {
    /// Mode (ON/OFF).
    #[serde(rename = "Mode")]
    pub mode: WideDynamicMode,

    /// Level.
    #[serde(rename = "Level")]
    pub level: f32,
}

/// Wide dynamic range 2.0.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WideDynamicRange20 {
    /// Mode.
    #[serde(rename = "Mode")]
    pub mode: WideDynamicMode,

    /// Level.
    #[serde(rename = "Level", default, skip_serializing_if = "Option::is_none")]
    pub level: Option<f32>,
}

/// Wide dynamic mode.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum WideDynamicMode {
    #[default]
    OFF,
    ON,
}

/// White balance settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WhiteBalance {
    /// Mode (AUTO/MANUAL).
    #[serde(rename = "Mode")]
    pub mode: WhiteBalanceMode,

    /// Cr gain.
    #[serde(rename = "CrGain")]
    pub cr_gain: f32,

    /// Cb gain.
    #[serde(rename = "CbGain")]
    pub cb_gain: f32,
}

/// White balance settings 2.0.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WhiteBalance20 {
    /// Mode.
    #[serde(rename = "Mode")]
    pub mode: WhiteBalanceMode,

    /// Cr gain.
    #[serde(rename = "CrGain", default, skip_serializing_if = "Option::is_none")]
    pub cr_gain: Option<f32>,

    /// Cb gain.
    #[serde(rename = "CbGain", default, skip_serializing_if = "Option::is_none")]
    pub cb_gain: Option<f32>,

    /// Extension.
    #[serde(rename = "Extension", default, skip_serializing_if = "Option::is_none")]
    pub extension: Option<Extension>,
}

/// White balance mode.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum WhiteBalanceMode {
    #[default]
    AUTO,
    MANUAL,
}

// ============================================================================
// Profile Type
// ============================================================================

/// Media profile.
///
/// A profile consists of a set of interconnected configuration entities
/// used to configure media streaming.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Profile {
    /// Profile token.
    #[serde(rename = "@token")]
    pub token: ReferenceToken,

    /// Fixed profile flag.
    #[serde(rename = "@fixed", default, skip_serializing_if = "Option::is_none")]
    pub fixed: Option<bool>,

    /// Profile name.
    #[serde(rename = "Name")]
    pub name: Name,

    /// Video source configuration.
    #[serde(
        rename = "VideoSourceConfiguration",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub video_source_configuration: Option<VideoSourceConfiguration>,

    /// Audio source configuration.
    #[serde(
        rename = "AudioSourceConfiguration",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub audio_source_configuration: Option<AudioSourceConfiguration>,

    /// Video encoder configuration.
    #[serde(
        rename = "VideoEncoderConfiguration",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub video_encoder_configuration: Option<VideoEncoderConfiguration>,

    /// Audio encoder configuration.
    #[serde(
        rename = "AudioEncoderConfiguration",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub audio_encoder_configuration: Option<AudioEncoderConfiguration>,

    /// PTZ configuration.
    #[serde(
        rename = "PTZConfiguration",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub ptz_configuration: Option<PTZConfiguration>,

    /// Metadata configuration.
    #[serde(
        rename = "MetadataConfiguration",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub metadata_configuration: Option<MetadataConfiguration>,

    /// Extension.
    #[serde(rename = "Extension", default, skip_serializing_if = "Option::is_none")]
    pub extension: Option<ProfileExtension>,
}

/// Metadata configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetadataConfiguration {
    /// Configuration token.
    #[serde(rename = "@token")]
    pub token: ReferenceToken,

    /// User readable name.
    #[serde(rename = "Name")]
    pub name: Name,

    /// Number of references using this configuration.
    #[serde(rename = "UseCount")]
    pub use_count: i32,

    /// PTZ status filter.
    #[serde(rename = "PTZStatus", default, skip_serializing_if = "Option::is_none")]
    pub ptz_status: Option<PTZFilter>,

    /// Analytics flag.
    #[serde(rename = "Analytics", default, skip_serializing_if = "Option::is_none")]
    pub analytics: Option<bool>,

    /// Multicast configuration.
    #[serde(rename = "Multicast", default, skip_serializing_if = "Option::is_none")]
    pub multicast: Option<MulticastConfiguration>,

    /// Session timeout.
    #[serde(rename = "SessionTimeout")]
    pub session_timeout: String,

    /// Extension.
    #[serde(rename = "Extension", default, skip_serializing_if = "Option::is_none")]
    pub extension: Option<Extension>,
}

/// PTZ filter for metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PTZFilter {
    /// Include PTZ status.
    #[serde(rename = "Status")]
    pub status: bool,

    /// Include PTZ position.
    #[serde(rename = "Position")]
    pub position: bool,
}

/// Profile extension.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ProfileExtension {
    /// Audio output configuration.
    #[serde(
        rename = "AudioOutputConfiguration",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub audio_output_configuration: Option<AudioOutputConfiguration>,

    /// Audio decoder configuration.
    #[serde(
        rename = "AudioDecoderConfiguration",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub audio_decoder_configuration: Option<AudioDecoderConfiguration>,

    /// Extension.
    #[serde(rename = "Extension", default, skip_serializing_if = "Option::is_none")]
    pub extension: Option<Extension>,
}

/// Audio output configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioOutputConfiguration {
    /// Configuration token.
    #[serde(rename = "@token")]
    pub token: ReferenceToken,

    /// User readable name.
    #[serde(rename = "Name")]
    pub name: Name,

    /// Number of references using this configuration.
    #[serde(rename = "UseCount")]
    pub use_count: i32,

    /// Output token.
    #[serde(rename = "OutputToken")]
    pub output_token: ReferenceToken,

    /// Output level.
    #[serde(
        rename = "OutputLevel",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub output_level: Option<i32>,
}

/// Audio decoder configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioDecoderConfiguration {
    /// Configuration token.
    #[serde(rename = "@token")]
    pub token: ReferenceToken,

    /// User readable name.
    #[serde(rename = "Name")]
    pub name: Name,

    /// Number of references using this configuration.
    #[serde(rename = "UseCount")]
    pub use_count: i32,
}

// ============================================================================
// Streaming Types
// ============================================================================

/// Transport protocol enumeration.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum TransportProtocol {
    UDP,
    TCP,
    #[default]
    RTSP,
    HTTP,
}

/// Transport configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Transport {
    /// Protocol.
    #[serde(rename = "Protocol")]
    pub protocol: TransportProtocol,

    /// Tunnel (for tunneling protocols).
    #[serde(rename = "Tunnel", default, skip_serializing_if = "Option::is_none")]
    pub tunnel: Option<Box<Transport>>,
}

/// Stream setup configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StreamSetup {
    /// Stream type.
    #[serde(rename = "Stream")]
    pub stream: StreamType,

    /// Transport configuration.
    #[serde(rename = "Transport")]
    pub transport: Transport,
}

/// Stream type enumeration.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum StreamType {
    #[serde(rename = "RTP-Unicast")]
    #[default]
    RtpUnicast,
    #[serde(rename = "RTP-Multicast")]
    RtpMulticast,
}

/// Media URI information.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MediaUri {
    /// Stream URI.
    #[serde(rename = "Uri")]
    pub uri: String,

    /// Invalid after connection flag.
    #[serde(rename = "InvalidAfterConnect")]
    pub invalid_after_connect: bool,

    /// Invalid after reboot flag.
    #[serde(rename = "InvalidAfterReboot")]
    pub invalid_after_reboot: bool,

    /// Timeout duration.
    #[serde(rename = "Timeout")]
    pub timeout: String,
}

// ============================================================================
// User Types
// ============================================================================

/// User level enumeration.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum UserLevel {
    Administrator,
    Operator,
    #[default]
    User,
    Anonymous,
    Extended,
}

/// User account information.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct User {
    /// Username.
    #[serde(rename = "Username")]
    pub username: String,

    /// Password (for create/update operations).
    #[serde(rename = "Password", default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,

    /// User level.
    #[serde(rename = "UserLevel")]
    pub user_level: UserLevel,

    /// Extension.
    #[serde(rename = "Extension", default, skip_serializing_if = "Option::is_none")]
    pub extension: Option<Extension>,
}

// ============================================================================
// Discovery Types
// ============================================================================

/// Discovery mode enumeration.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum DiscoveryMode {
    #[default]
    Discoverable,
    NonDiscoverable,
}

/// Scope definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Scope {
    /// Scope definition type.
    #[serde(rename = "tt:ScopeDef", alias = "ScopeDef")]
    pub scope_def: ScopeDefinition,

    /// Scope item URI.
    #[serde(rename = "tt:ScopeItem", alias = "ScopeItem")]
    pub scope_item: String,
}

/// Scope definition type.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum ScopeDefinition {
    Fixed,
    #[default]
    Configurable,
}

// ============================================================================
// Factory Default Type
// ============================================================================

/// Factory default type.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum FactoryDefaultType {
    Hard,
    #[default]
    Soft,
}

// ============================================================================
// SOAP Fault Types (for error response parsing)
// ============================================================================

/// SOAP 1.2 Fault structure.
///
/// Used to parse SOAP fault responses from ONVIF devices.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "Fault")]
pub struct SoapFault {
    /// Fault code.
    #[serde(rename = "Code")]
    pub code: SoapFaultCode,

    /// Fault reason.
    #[serde(rename = "Reason")]
    pub reason: SoapFaultReason,

    /// Optional fault detail.
    #[serde(rename = "Detail", skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// SOAP Fault code structure.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SoapFaultCode {
    /// Code value (e.g., "s:Receiver", "s:Sender").
    #[serde(rename = "Value")]
    pub value: String,

    /// Optional subcode.
    #[serde(rename = "Subcode", skip_serializing_if = "Option::is_none")]
    pub subcode: Option<SoapFaultSubcode>,
}

/// SOAP Fault subcode structure.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SoapFaultSubcode {
    /// Subcode value (e.g., "ter:ActionNotSupported").
    #[serde(rename = "Value")]
    pub value: String,
}

/// SOAP Fault reason structure.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SoapFaultReason {
    /// Reason text elements.
    #[serde(rename = "Text")]
    pub text: Vec<SoapFaultText>,
}

/// SOAP Fault text element.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SoapFaultText {
    /// Language attribute.
    #[serde(rename = "@lang", skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,

    /// Text value.
    #[serde(rename = "$text")]
    pub value: String,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_video_resolution_default() {
        let res = VideoResolution::default();
        assert_eq!(res.width, 1920);
        assert_eq!(res.height, 1080);
    }

    #[test]
    fn test_onvif_version_default() {
        let version = OnvifVersion::default();
        assert_eq!(version.major, 2);
        assert_eq!(version.minor, 5);
    }

    #[test]
    fn test_int_rectangle_default() {
        let rect = IntRectangle::default();
        assert_eq!(rect.x, 0);
        assert_eq!(rect.y, 0);
        assert_eq!(rect.width, 1920);
        assert_eq!(rect.height, 1080);
    }

    #[test]
    fn test_timezone_default() {
        let tz = TimeZone::default();
        assert_eq!(tz.tz, "UTC0");
    }

    #[test]
    fn test_user_level_default() {
        let level = UserLevel::default();
        assert_eq!(level, UserLevel::User);
    }

    #[test]
    fn test_discovery_mode_default() {
        let mode = DiscoveryMode::default();
        assert_eq!(mode, DiscoveryMode::Discoverable);
    }
}
