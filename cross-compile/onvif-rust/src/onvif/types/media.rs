//! Media Service types from media.wsdl (trt:* namespace).
//!
//! This module contains request/response types for the ONVIF Media Service.
//! These types are defined in the `http://www.onvif.org/ver10/media/wsdl` namespace.
//!
//! # Operations
//!
//! ## Profile Management
//! - `GetProfiles` - Get all media profiles
//! - `GetProfile` - Get a specific profile
//! - `CreateProfile` / `DeleteProfile` - Profile management
//!
//! ## Video Sources
//! - `GetVideoSources` - Get available video sources
//! - `GetVideoSourceConfigurations` - Get video source configurations
//! - `GetVideoSourceConfiguration` - Get a specific configuration
//!
//! ## Video Encoding
//! - `GetVideoEncoderConfigurations` - Get all video encoder configurations
//! - `GetVideoEncoderConfiguration` - Get a specific configuration
//! - `SetVideoEncoderConfiguration` - Update encoder settings
//! - `GetVideoEncoderConfigurationOptions` - Get supported options
//!
//! ## Audio
//! - `GetAudioSources` - Get available audio sources
//! - `GetAudioEncoderConfigurations` - Get audio encoder configurations
//!
//! ## Streaming
//! - `GetStreamUri` - Get RTSP stream URI
//! - `GetSnapshotUri` - Get snapshot URI

use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::Extension;
use super::common::{
    AudioEncoderConfiguration, AudioSource, AudioSourceConfiguration, IntRange, MediaUri, Name,
    Profile, ReferenceToken, StreamSetup, VideoEncoderConfiguration, VideoResolution, VideoSource,
    VideoSourceConfiguration,
};

// ============================================================================
// Profiles
// ============================================================================

/// GetProfiles request.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetProfiles")]
pub struct GetProfiles {}

/// GetProfiles response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "trt:GetProfilesResponse")]
pub struct GetProfilesResponse {
    /// List of profiles.
    #[serde(rename = "trt:Profiles", alias = "Profiles", default)]
    pub profiles: Vec<Profile>,
}

/// GetProfile request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetProfile")]
pub struct GetProfile {
    /// Profile token.
    #[serde(rename = "ProfileToken")]
    pub profile_token: ReferenceToken,
}

/// GetProfile response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "trt:GetProfileResponse")]
pub struct GetProfileResponse {
    /// The requested profile.
    #[serde(rename = "trt:Profile", alias = "Profile")]
    pub profile: Profile,
}

/// CreateProfile request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "CreateProfile")]
pub struct CreateProfile {
    /// Profile name.
    #[serde(rename = "Name")]
    pub name: Name,

    /// Optional profile token.
    #[serde(rename = "Token", default, skip_serializing_if = "Option::is_none")]
    pub token: Option<ReferenceToken>,
}

/// CreateProfile response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "trt:CreateProfileResponse")]
pub struct CreateProfileResponse {
    /// The created profile.
    #[serde(rename = "trt:Profile", alias = "Profile")]
    pub profile: Profile,
}

/// DeleteProfile request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "DeleteProfile")]
pub struct DeleteProfile {
    /// Profile token to delete.
    #[serde(rename = "ProfileToken")]
    pub profile_token: ReferenceToken,
}

/// DeleteProfile response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "trt:DeleteProfileResponse")]
pub struct DeleteProfileResponse {}

// ============================================================================
// Video Sources
// ============================================================================

/// GetVideoSources request.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetVideoSources")]
pub struct GetVideoSources {}

/// GetVideoSources response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "trt:GetVideoSourcesResponse")]
pub struct GetVideoSourcesResponse {
    /// List of video sources.
    #[serde(rename = "trt:VideoSources", alias = "VideoSources", default)]
    pub video_sources: Vec<VideoSource>,
}

/// GetVideoSourceConfigurations request.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetVideoSourceConfigurations")]
pub struct GetVideoSourceConfigurations {}

/// GetVideoSourceConfigurations response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "trt:GetVideoSourceConfigurationsResponse")]
pub struct GetVideoSourceConfigurationsResponse {
    /// List of video source configurations.
    #[serde(rename = "trt:Configurations", alias = "Configurations", default)]
    pub configurations: Vec<VideoSourceConfiguration>,
}

/// GetVideoSourceConfiguration request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetVideoSourceConfiguration")]
pub struct GetVideoSourceConfiguration {
    /// Configuration token.
    #[serde(rename = "ConfigurationToken")]
    pub configuration_token: ReferenceToken,
}

/// GetVideoSourceConfiguration response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "trt:GetVideoSourceConfigurationResponse")]
pub struct GetVideoSourceConfigurationResponse {
    /// The requested configuration.
    #[serde(rename = "trt:Configuration", alias = "Configuration")]
    pub configuration: VideoSourceConfiguration,
}

/// SetVideoSourceConfiguration request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "SetVideoSourceConfiguration")]
pub struct SetVideoSourceConfiguration {
    /// Configuration to set.
    #[serde(rename = "Configuration")]
    pub configuration: VideoSourceConfiguration,

    /// Force persistence.
    #[serde(rename = "ForcePersistence")]
    pub force_persistence: bool,
}

/// SetVideoSourceConfiguration response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "trt:SetVideoSourceConfigurationResponse")]
pub struct SetVideoSourceConfigurationResponse {}

/// GetVideoSourceConfigurationOptions request.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetVideoSourceConfigurationOptions")]
pub struct GetVideoSourceConfigurationOptions {
    /// Configuration token (optional).
    #[serde(
        rename = "ConfigurationToken",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub configuration_token: Option<ReferenceToken>,

    /// Profile token (optional).
    #[serde(
        rename = "ProfileToken",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub profile_token: Option<ReferenceToken>,
}

/// GetVideoSourceConfigurationOptions response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "trt:GetVideoSourceConfigurationOptionsResponse")]
pub struct GetVideoSourceConfigurationOptionsResponse {
    /// Video source configuration options.
    #[serde(rename = "trt:Options", alias = "Options")]
    pub options: VideoSourceConfigurationOptions,
}

/// Video source configuration options.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct VideoSourceConfigurationOptions {
    /// Maximum instances of this configuration.
    #[serde(
        rename = "tt:MaximumNumberOfProfiles",
        alias = "MaximumNumberOfProfiles",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub maximum_number_of_profiles: Option<i32>,

    /// Bounds range.
    #[serde(
        rename = "tt:BoundsRange",
        alias = "BoundsRange",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub bounds_range: Option<IntRectangleRange>,

    /// Available video source tokens.
    #[serde(
        rename = "tt:VideoSourceTokensAvailable",
        alias = "VideoSourceTokensAvailable",
        default
    )]
    pub video_source_tokens_available: Vec<ReferenceToken>,

    /// Extension.
    #[serde(
        rename = "tt:Extension",
        alias = "Extension",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub extension: Option<Extension>,
}

/// Integer rectangle range.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct IntRectangleRange {
    /// X range.
    #[serde(rename = "tt:XRange", alias = "XRange")]
    pub x_range: IntRange,

    /// Y range.
    #[serde(rename = "tt:YRange", alias = "YRange")]
    pub y_range: IntRange,

    /// Width range.
    #[serde(rename = "tt:WidthRange", alias = "WidthRange")]
    pub width_range: IntRange,

    /// Height range.
    #[serde(rename = "tt:HeightRange", alias = "HeightRange")]
    pub height_range: IntRange,
}

// ============================================================================
// Video Encoder Configurations
// ============================================================================

/// GetVideoEncoderConfigurations request.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetVideoEncoderConfigurations")]
pub struct GetVideoEncoderConfigurations {}

/// GetVideoEncoderConfigurations response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "trt:GetVideoEncoderConfigurationsResponse")]
pub struct GetVideoEncoderConfigurationsResponse {
    /// List of video encoder configurations.
    #[serde(rename = "trt:Configurations", alias = "Configurations", default)]
    pub configurations: Vec<VideoEncoderConfiguration>,
}

/// GetVideoEncoderConfiguration request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetVideoEncoderConfiguration")]
pub struct GetVideoEncoderConfiguration {
    /// Configuration token.
    #[serde(rename = "ConfigurationToken")]
    pub configuration_token: ReferenceToken,
}

/// GetVideoEncoderConfiguration response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "trt:GetVideoEncoderConfigurationResponse")]
pub struct GetVideoEncoderConfigurationResponse {
    /// The requested configuration.
    #[serde(rename = "trt:Configuration", alias = "Configuration")]
    pub configuration: VideoEncoderConfiguration,
}

/// SetVideoEncoderConfiguration request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "SetVideoEncoderConfiguration")]
pub struct SetVideoEncoderConfiguration {
    /// Configuration to set.
    #[serde(rename = "Configuration")]
    pub configuration: VideoEncoderConfiguration,

    /// Force persistence.
    #[serde(rename = "ForcePersistence")]
    pub force_persistence: bool,
}

/// SetVideoEncoderConfiguration response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "trt:SetVideoEncoderConfigurationResponse")]
pub struct SetVideoEncoderConfigurationResponse {}

/// GetVideoEncoderConfigurationOptions request.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetVideoEncoderConfigurationOptions")]
pub struct GetVideoEncoderConfigurationOptions {
    /// Configuration token (optional).
    #[serde(
        rename = "ConfigurationToken",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub configuration_token: Option<ReferenceToken>,

    /// Profile token (optional).
    #[serde(
        rename = "ProfileToken",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub profile_token: Option<ReferenceToken>,
}

/// GetVideoEncoderConfigurationOptions response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "trt:GetVideoEncoderConfigurationOptionsResponse")]
pub struct GetVideoEncoderConfigurationOptionsResponse {
    /// Video encoder configuration options.
    #[serde(rename = "trt:Options", alias = "Options")]
    pub options: VideoEncoderConfigurationOptions,
}

/// Video encoder configuration options.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct VideoEncoderConfigurationOptions {
    /// Quality range.
    #[serde(rename = "tt:QualityRange", alias = "QualityRange")]
    pub quality_range: IntRange,

    /// JPEG options.
    #[serde(
        rename = "tt:JPEG",
        alias = "JPEG",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub jpeg: Option<JpegOptions>,

    /// MPEG4 options.
    #[serde(
        rename = "tt:MPEG4",
        alias = "MPEG4",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub mpeg4: Option<Mpeg4Options>,

    /// H.264 options.
    #[serde(
        rename = "tt:H264",
        alias = "H264",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub h264: Option<H264Options>,

    /// Extension.
    #[serde(
        rename = "tt:Extension",
        alias = "Extension",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub extension: Option<VideoEncoderConfigurationOptionsExtension>,
}

/// Video encoder configuration options extension.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct VideoEncoderConfigurationOptionsExtension {
    /// JPEG options (alternative).
    #[serde(
        rename = "tt:JPEG",
        alias = "JPEG",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub jpeg: Option<JpegOptions2>,

    /// MPEG4 options (alternative).
    #[serde(
        rename = "tt:MPEG4",
        alias = "MPEG4",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub mpeg4: Option<Mpeg4Options2>,

    /// H.264 options (alternative).
    #[serde(
        rename = "tt:H264",
        alias = "H264",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub h264: Option<H264Options2>,

    /// Extension.
    #[serde(
        rename = "tt:Extension",
        alias = "Extension",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub extension: Option<Extension>,
}

/// JPEG encoder options.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct JpegOptions {
    /// Supported resolutions.
    #[serde(
        rename = "tt:ResolutionsAvailable",
        alias = "ResolutionsAvailable",
        default
    )]
    pub resolutions_available: Vec<VideoResolution>,

    /// Frame rate range.
    #[serde(rename = "tt:FrameRateRange", alias = "FrameRateRange")]
    pub frame_rate_range: IntRange,

    /// Encoding interval range.
    #[serde(rename = "tt:EncodingIntervalRange", alias = "EncodingIntervalRange")]
    pub encoding_interval_range: IntRange,
}

/// JPEG encoder options (extended).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct JpegOptions2 {
    /// Supported resolutions.
    #[serde(
        rename = "tt:ResolutionsAvailable",
        alias = "ResolutionsAvailable",
        default
    )]
    pub resolutions_available: Vec<VideoResolution>,

    /// Frame rate range.
    #[serde(rename = "tt:FrameRateRange", alias = "FrameRateRange")]
    pub frame_rate_range: IntRange,

    /// Encoding interval range.
    #[serde(rename = "tt:EncodingIntervalRange", alias = "EncodingIntervalRange")]
    pub encoding_interval_range: IntRange,

    /// Bitrate range.
    #[serde(
        rename = "tt:BitrateRange",
        alias = "BitrateRange",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub bitrate_range: Option<IntRange>,
}

/// MPEG4 encoder options.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Mpeg4Options {
    /// Supported resolutions.
    #[serde(
        rename = "tt:ResolutionsAvailable",
        alias = "ResolutionsAvailable",
        default
    )]
    pub resolutions_available: Vec<VideoResolution>,

    /// GOP length range.
    #[serde(rename = "tt:GovLengthRange", alias = "GovLengthRange")]
    pub gov_length_range: IntRange,

    /// Frame rate range.
    #[serde(rename = "tt:FrameRateRange", alias = "FrameRateRange")]
    pub frame_rate_range: IntRange,

    /// Encoding interval range.
    #[serde(rename = "tt:EncodingIntervalRange", alias = "EncodingIntervalRange")]
    pub encoding_interval_range: IntRange,

    /// Supported MPEG4 profiles.
    #[serde(
        rename = "tt:Mpeg4ProfilesSupported",
        alias = "Mpeg4ProfilesSupported",
        default
    )]
    pub mpeg4_profiles_supported: Vec<Mpeg4Profile>,
}

/// MPEG4 encoder options (extended).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Mpeg4Options2 {
    /// Supported resolutions.
    #[serde(
        rename = "tt:ResolutionsAvailable",
        alias = "ResolutionsAvailable",
        default
    )]
    pub resolutions_available: Vec<VideoResolution>,

    /// GOP length range.
    #[serde(rename = "tt:GovLengthRange", alias = "GovLengthRange")]
    pub gov_length_range: IntRange,

    /// Frame rate range.
    #[serde(rename = "tt:FrameRateRange", alias = "FrameRateRange")]
    pub frame_rate_range: IntRange,

    /// Encoding interval range.
    #[serde(rename = "tt:EncodingIntervalRange", alias = "EncodingIntervalRange")]
    pub encoding_interval_range: IntRange,

    /// Supported MPEG4 profiles.
    #[serde(
        rename = "tt:Mpeg4ProfilesSupported",
        alias = "Mpeg4ProfilesSupported",
        default
    )]
    pub mpeg4_profiles_supported: Vec<Mpeg4Profile>,

    /// Bitrate range.
    #[serde(
        rename = "tt:BitrateRange",
        alias = "BitrateRange",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub bitrate_range: Option<IntRange>,
}

/// MPEG4 profile.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum Mpeg4Profile {
    #[default]
    SP,
    ASP,
}

/// H.264 encoder options.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct H264Options {
    /// Supported resolutions.
    #[serde(
        rename = "tt:ResolutionsAvailable",
        alias = "ResolutionsAvailable",
        default
    )]
    pub resolutions_available: Vec<VideoResolution>,

    /// GOP length range.
    #[serde(rename = "tt:GovLengthRange", alias = "GovLengthRange")]
    pub gov_length_range: IntRange,

    /// Frame rate range.
    #[serde(rename = "tt:FrameRateRange", alias = "FrameRateRange")]
    pub frame_rate_range: IntRange,

    /// Encoding interval range.
    #[serde(rename = "tt:EncodingIntervalRange", alias = "EncodingIntervalRange")]
    pub encoding_interval_range: IntRange,

    /// Supported H.264 profiles.
    #[serde(
        rename = "tt:H264ProfilesSupported",
        alias = "H264ProfilesSupported",
        default,
        with = "h264_profiles_vec"
    )]
    pub h264_profiles_supported: Vec<H264Profile>,
}

/// H.264 encoder options (extended).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct H264Options2 {
    /// Supported resolutions.
    #[serde(
        rename = "tt:ResolutionsAvailable",
        alias = "ResolutionsAvailable",
        default
    )]
    pub resolutions_available: Vec<VideoResolution>,

    /// GOP length range.
    #[serde(rename = "tt:GovLengthRange", alias = "GovLengthRange")]
    pub gov_length_range: IntRange,

    /// Frame rate range.
    #[serde(rename = "tt:FrameRateRange", alias = "FrameRateRange")]
    pub frame_rate_range: IntRange,

    /// Encoding interval range.
    #[serde(rename = "tt:EncodingIntervalRange", alias = "EncodingIntervalRange")]
    pub encoding_interval_range: IntRange,

    /// Supported H.264 profiles.
    #[serde(
        rename = "tt:H264ProfilesSupported",
        alias = "H264ProfilesSupported",
        default,
        with = "h264_profiles_vec"
    )]
    pub h264_profiles_supported: Vec<H264Profile>,

    /// Bitrate range.
    #[serde(
        rename = "tt:BitrateRange",
        alias = "BitrateRange",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub bitrate_range: Option<IntRange>,
}

/// H.264 profile.
#[derive(Debug, Clone, Default, PartialEq)]
pub enum H264Profile {
    #[default]
    Baseline,
    Main,
    Extended,
    High,
}

/// Error type for H264Profile parsing.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseH264ProfileError(String);

impl std::fmt::Display for ParseH264ProfileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown H264Profile: {}", self.0)
    }
}

impl std::error::Error for ParseH264ProfileError {}

impl FromStr for H264Profile {
    type Err = ParseH264ProfileError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Baseline" => Ok(Self::Baseline),
            "Main" => Ok(Self::Main),
            "Extended" => Ok(Self::Extended),
            "High" => Ok(Self::High),
            _ => Err(ParseH264ProfileError(s.to_string())),
        }
    }
}

impl H264Profile {
    /// Convert to string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Baseline => "Baseline",
            Self::Main => "Main",
            Self::Extended => "Extended",
            Self::High => "High",
        }
    }
}

impl Serialize for H264Profile {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for H264Profile {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse()
            .map_err(|e: ParseH264ProfileError| serde::de::Error::custom(e.to_string()))
    }
}

/// Wrapper for deserializing H264Profile from repeated XML elements.
/// The text content of `<H264ProfilesSupported>Baseline</H264ProfilesSupported>` elements.
#[derive(Debug, Clone, Deserialize, Serialize)]
struct H264ProfileWrapper {
    #[serde(rename = "$text")]
    value: String,
}

/// Custom serialization/deserialization for Vec<H264Profile>.
/// Handles repeated `<H264ProfilesSupported>Value</H264ProfilesSupported>` elements.
mod h264_profiles_vec {
    use super::{H264Profile, H264ProfileWrapper};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::str::FromStr;

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<H264Profile>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wrappers: Vec<H264ProfileWrapper> = Vec::deserialize(deserializer)?;
        wrappers
            .into_iter()
            .map(|w| {
                H264Profile::from_str(&w.value).map_err(|e| serde::de::Error::custom(e.to_string()))
            })
            .collect()
    }

    pub fn serialize<S>(profiles: &[H264Profile], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let wrappers: Vec<H264ProfileWrapper> = profiles
            .iter()
            .map(|p| H264ProfileWrapper {
                value: p.as_str().to_string(),
            })
            .collect();
        wrappers.serialize(serializer)
    }
}

// ============================================================================
// Audio Sources
// ============================================================================

/// GetAudioSources request.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetAudioSources")]
pub struct GetAudioSources {}

/// GetAudioSources response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "trt:GetAudioSourcesResponse")]
pub struct GetAudioSourcesResponse {
    /// List of audio sources.
    #[serde(rename = "trt:AudioSources", alias = "AudioSources", default)]
    pub audio_sources: Vec<AudioSource>,
}

/// GetAudioSourceConfigurations request.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetAudioSourceConfigurations")]
pub struct GetAudioSourceConfigurations {}

/// GetAudioSourceConfigurations response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "trt:GetAudioSourceConfigurationsResponse")]
pub struct GetAudioSourceConfigurationsResponse {
    /// List of audio source configurations.
    #[serde(rename = "trt:Configurations", alias = "Configurations", default)]
    pub configurations: Vec<AudioSourceConfiguration>,
}

/// GetAudioSourceConfiguration request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetAudioSourceConfiguration")]
pub struct GetAudioSourceConfiguration {
    /// Configuration token.
    #[serde(rename = "ConfigurationToken")]
    pub configuration_token: ReferenceToken,
}

/// GetAudioSourceConfiguration response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "trt:GetAudioSourceConfigurationResponse")]
pub struct GetAudioSourceConfigurationResponse {
    /// The requested configuration.
    #[serde(rename = "trt:Configuration", alias = "Configuration")]
    pub configuration: AudioSourceConfiguration,
}

// ============================================================================
// Audio Encoder Configurations
// ============================================================================

/// GetAudioEncoderConfigurations request.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetAudioEncoderConfigurations")]
pub struct GetAudioEncoderConfigurations {}

/// GetAudioEncoderConfigurations response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "trt:GetAudioEncoderConfigurationsResponse")]
pub struct GetAudioEncoderConfigurationsResponse {
    /// List of audio encoder configurations.
    #[serde(rename = "trt:Configurations", alias = "Configurations", default)]
    pub configurations: Vec<AudioEncoderConfiguration>,
}

/// GetAudioEncoderConfiguration request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetAudioEncoderConfiguration")]
pub struct GetAudioEncoderConfiguration {
    /// Configuration token.
    #[serde(rename = "ConfigurationToken")]
    pub configuration_token: ReferenceToken,
}

/// GetAudioEncoderConfiguration response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "trt:GetAudioEncoderConfigurationResponse")]
pub struct GetAudioEncoderConfigurationResponse {
    /// The requested configuration.
    #[serde(rename = "trt:Configuration", alias = "Configuration")]
    pub configuration: AudioEncoderConfiguration,
}

/// SetAudioEncoderConfiguration request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "SetAudioEncoderConfiguration")]
pub struct SetAudioEncoderConfiguration {
    /// Configuration to set.
    #[serde(rename = "Configuration")]
    pub configuration: AudioEncoderConfiguration,

    /// Force persistence.
    #[serde(rename = "ForcePersistence")]
    pub force_persistence: bool,
}

/// SetAudioEncoderConfiguration response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "trt:SetAudioEncoderConfigurationResponse")]
pub struct SetAudioEncoderConfigurationResponse {}

/// GetAudioEncoderConfigurationOptions request.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetAudioEncoderConfigurationOptions")]
pub struct GetAudioEncoderConfigurationOptions {
    /// Configuration token (optional).
    #[serde(
        rename = "ConfigurationToken",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub configuration_token: Option<ReferenceToken>,

    /// Profile token (optional).
    #[serde(
        rename = "ProfileToken",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub profile_token: Option<ReferenceToken>,
}

/// GetAudioEncoderConfigurationOptions response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "trt:GetAudioEncoderConfigurationOptionsResponse")]
pub struct GetAudioEncoderConfigurationOptionsResponse {
    /// Audio encoder configuration options.
    #[serde(rename = "trt:Options", alias = "Options")]
    pub options: AudioEncoderConfigurationOptions,
}

/// Audio encoder configuration options.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AudioEncoderConfigurationOptions {
    /// Options for each encoding.
    #[serde(rename = "tt:Options", alias = "Options", default)]
    pub options: Vec<AudioEncoderConfigurationOption>,
}

/// Audio encoder configuration option.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioEncoderConfigurationOption {
    /// Audio encoding type.
    #[serde(rename = "tt:Encoding", alias = "Encoding")]
    pub encoding: AudioEncoding,

    /// Bitrate range.
    #[serde(rename = "tt:BitrateList", alias = "BitrateList")]
    pub bitrate_list: IntList,

    /// Sample rate range.
    #[serde(rename = "tt:SampleRateList", alias = "SampleRateList")]
    pub sample_rate_list: IntList,
}

/// Audio encoding type.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum AudioEncoding {
    #[default]
    G711,
    G726,
    AAC,
}

/// Integer list.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct IntList {
    /// List of integer items.
    #[serde(rename = "tt:Items", alias = "Items", default)]
    pub items: Vec<i32>,
}

// ============================================================================
// Stream URI
// ============================================================================

/// GetStreamUri request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetStreamUri")]
pub struct GetStreamUri {
    /// Stream setup parameters.
    #[serde(rename = "StreamSetup")]
    pub stream_setup: StreamSetup,

    /// Profile token.
    #[serde(rename = "ProfileToken")]
    pub profile_token: ReferenceToken,
}

/// GetStreamUri response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "trt:GetStreamUriResponse")]
pub struct GetStreamUriResponse {
    /// Media URI.
    #[serde(rename = "trt:MediaUri", alias = "MediaUri")]
    pub media_uri: MediaUri,
}

// ============================================================================
// Snapshot URI
// ============================================================================

/// GetSnapshotUri request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetSnapshotUri")]
pub struct GetSnapshotUri {
    /// Profile token.
    #[serde(rename = "ProfileToken")]
    pub profile_token: ReferenceToken,
}

/// GetSnapshotUri response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "trt:GetSnapshotUriResponse")]
pub struct GetSnapshotUriResponse {
    /// Media URI.
    #[serde(rename = "trt:MediaUri", alias = "MediaUri")]
    pub media_uri: MediaUri,
}

// ============================================================================
// Profile Configuration Management
// ============================================================================

/// AddVideoEncoderConfiguration request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "AddVideoEncoderConfiguration")]
pub struct AddVideoEncoderConfiguration {
    /// Profile token.
    #[serde(rename = "ProfileToken")]
    pub profile_token: ReferenceToken,

    /// Configuration token.
    #[serde(rename = "ConfigurationToken")]
    pub configuration_token: ReferenceToken,
}

/// AddVideoEncoderConfiguration response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "trt:AddVideoEncoderConfigurationResponse")]
pub struct AddVideoEncoderConfigurationResponse {}

/// RemoveVideoEncoderConfiguration request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "RemoveVideoEncoderConfiguration")]
pub struct RemoveVideoEncoderConfiguration {
    /// Profile token.
    #[serde(rename = "ProfileToken")]
    pub profile_token: ReferenceToken,
}

/// RemoveVideoEncoderConfiguration response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "trt:RemoveVideoEncoderConfigurationResponse")]
pub struct RemoveVideoEncoderConfigurationResponse {}

/// AddVideoSourceConfiguration request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "AddVideoSourceConfiguration")]
pub struct AddVideoSourceConfiguration {
    /// Profile token.
    #[serde(rename = "ProfileToken")]
    pub profile_token: ReferenceToken,

    /// Configuration token.
    #[serde(rename = "ConfigurationToken")]
    pub configuration_token: ReferenceToken,
}

/// AddVideoSourceConfiguration response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "trt:AddVideoSourceConfigurationResponse")]
pub struct AddVideoSourceConfigurationResponse {}

/// RemoveVideoSourceConfiguration request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "RemoveVideoSourceConfiguration")]
pub struct RemoveVideoSourceConfiguration {
    /// Profile token.
    #[serde(rename = "ProfileToken")]
    pub profile_token: ReferenceToken,
}

/// RemoveVideoSourceConfiguration response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "trt:RemoveVideoSourceConfigurationResponse")]
pub struct RemoveVideoSourceConfigurationResponse {}

/// AddAudioEncoderConfiguration request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "AddAudioEncoderConfiguration")]
pub struct AddAudioEncoderConfiguration {
    /// Profile token.
    #[serde(rename = "ProfileToken")]
    pub profile_token: ReferenceToken,

    /// Configuration token.
    #[serde(rename = "ConfigurationToken")]
    pub configuration_token: ReferenceToken,
}

/// AddAudioEncoderConfiguration response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "trt:AddAudioEncoderConfigurationResponse")]
pub struct AddAudioEncoderConfigurationResponse {}

/// RemoveAudioEncoderConfiguration request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "RemoveAudioEncoderConfiguration")]
pub struct RemoveAudioEncoderConfiguration {
    /// Profile token.
    #[serde(rename = "ProfileToken")]
    pub profile_token: ReferenceToken,
}

/// RemoveAudioEncoderConfiguration response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "trt:RemoveAudioEncoderConfigurationResponse")]
pub struct RemoveAudioEncoderConfigurationResponse {}

/// AddAudioSourceConfiguration request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "AddAudioSourceConfiguration")]
pub struct AddAudioSourceConfiguration {
    /// Profile token.
    #[serde(rename = "ProfileToken")]
    pub profile_token: ReferenceToken,

    /// Configuration token.
    #[serde(rename = "ConfigurationToken")]
    pub configuration_token: ReferenceToken,
}

/// AddAudioSourceConfiguration response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "trt:AddAudioSourceConfigurationResponse")]
pub struct AddAudioSourceConfigurationResponse {}

/// RemoveAudioSourceConfiguration request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "RemoveAudioSourceConfiguration")]
pub struct RemoveAudioSourceConfiguration {
    /// Profile token.
    #[serde(rename = "ProfileToken")]
    pub profile_token: ReferenceToken,
}

/// RemoveAudioSourceConfiguration response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "trt:RemoveAudioSourceConfigurationResponse")]
pub struct RemoveAudioSourceConfigurationResponse {}

/// AddPTZConfiguration request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "AddPTZConfiguration")]
pub struct AddPTZConfiguration {
    /// Profile token.
    #[serde(rename = "ProfileToken")]
    pub profile_token: ReferenceToken,

    /// Configuration token.
    #[serde(rename = "ConfigurationToken")]
    pub configuration_token: ReferenceToken,
}

/// AddPTZConfiguration response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "trt:AddPTZConfigurationResponse")]
pub struct AddPTZConfigurationResponse {}

/// RemovePTZConfiguration request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "RemovePTZConfiguration")]
pub struct RemovePTZConfiguration {
    /// Profile token.
    #[serde(rename = "ProfileToken")]
    pub profile_token: ReferenceToken,
}

/// RemovePTZConfiguration response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "trt:RemovePTZConfigurationResponse")]
pub struct RemovePTZConfigurationResponse {}

// ============================================================================
// Service Capabilities
// ============================================================================

/// GetServiceCapabilities request.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetServiceCapabilities")]
pub struct GetServiceCapabilities {}

/// GetServiceCapabilities response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "trt:GetServiceCapabilitiesResponse")]
pub struct GetServiceCapabilitiesResponse {
    /// Media service capabilities.
    #[serde(rename = "trt:Capabilities", alias = "Capabilities")]
    pub capabilities: MediaServiceCapabilities,
}

/// Media service capabilities.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MediaServiceCapabilities {
    /// Snapshot URI support.
    #[serde(
        rename = "@SnapshotUri",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub snapshot_uri: Option<bool>,

    /// Rotation support.
    #[serde(rename = "@Rotation", default, skip_serializing_if = "Option::is_none")]
    pub rotation: Option<bool>,

    /// Video source mode support.
    #[serde(
        rename = "@VideoSourceMode",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub video_source_mode: Option<bool>,

    /// OSD support.
    #[serde(rename = "@OSD", default, skip_serializing_if = "Option::is_none")]
    pub osd: Option<bool>,

    /// Temporary OSD text support.
    #[serde(
        rename = "@TemporaryOSDText",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub temporary_osd_text: Option<bool>,

    /// EXI compression support.
    #[serde(
        rename = "@EXICompression",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub exi_compression: Option<bool>,

    /// Streaming capabilities.
    #[serde(
        rename = "tt:ProfileCapabilities",
        alias = "ProfileCapabilities",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub profile_capabilities: Option<ProfileCapabilities>,

    /// Streaming capabilities.
    #[serde(
        rename = "tt:StreamingCapabilities",
        alias = "StreamingCapabilities",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub streaming_capabilities: Option<StreamingCapabilities>,
}

/// Profile capabilities.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ProfileCapabilities {
    /// Maximum number of profiles.
    #[serde(
        rename = "@MaximumNumberOfProfiles",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub maximum_number_of_profiles: Option<i32>,
}

/// Streaming capabilities.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct StreamingCapabilities {
    /// RTP multicast support.
    #[serde(
        rename = "@RTPMulticast",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub rtp_multicast: Option<bool>,

    /// RTP over TCP support.
    #[serde(rename = "@RTP_TCP", default, skip_serializing_if = "Option::is_none")]
    pub rtp_tcp: Option<bool>,

    /// RTP/RTSP/TCP support.
    #[serde(
        rename = "@RTP_RTSP_TCP",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub rtp_rtsp_tcp: Option<bool>,

    /// Non-aggregated control support.
    #[serde(
        rename = "@NonAggregateControl",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub non_aggregate_control: Option<bool>,

    /// No RTSP streaming support.
    #[serde(
        rename = "@NoRTSPStreaming",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub no_rtsp_streaming: Option<bool>,
}

// ============================================================================
// Compatible Configurations (FR-002)
// ============================================================================

/// GetCompatibleVideoSourceConfigurations request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetCompatibleVideoSourceConfigurations")]
pub struct GetCompatibleVideoSourceConfigurations {
    /// Profile token.
    #[serde(rename = "ProfileToken")]
    pub profile_token: ReferenceToken,
}

/// GetCompatibleVideoSourceConfigurations response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "trt:GetCompatibleVideoSourceConfigurationsResponse")]
pub struct GetCompatibleVideoSourceConfigurationsResponse {
    /// Compatible configurations.
    #[serde(rename = "trt:Configurations", alias = "Configurations", default)]
    pub configurations: Vec<VideoSourceConfiguration>,
}

/// GetCompatibleVideoEncoderConfigurations request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetCompatibleVideoEncoderConfigurations")]
pub struct GetCompatibleVideoEncoderConfigurations {
    /// Profile token.
    #[serde(rename = "ProfileToken")]
    pub profile_token: ReferenceToken,
}

/// GetCompatibleVideoEncoderConfigurations response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "trt:GetCompatibleVideoEncoderConfigurationsResponse")]
pub struct GetCompatibleVideoEncoderConfigurationsResponse {
    /// Compatible configurations.
    #[serde(rename = "trt:Configurations", alias = "Configurations", default)]
    pub configurations: Vec<VideoEncoderConfiguration>,
}

/// GetCompatibleAudioSourceConfigurations request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetCompatibleAudioSourceConfigurations")]
pub struct GetCompatibleAudioSourceConfigurations {
    /// Profile token.
    #[serde(rename = "ProfileToken")]
    pub profile_token: ReferenceToken,
}

/// GetCompatibleAudioSourceConfigurations response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "trt:GetCompatibleAudioSourceConfigurationsResponse")]
pub struct GetCompatibleAudioSourceConfigurationsResponse {
    /// Compatible configurations.
    #[serde(rename = "trt:Configurations", alias = "Configurations", default)]
    pub configurations: Vec<AudioSourceConfiguration>,
}

/// GetCompatibleAudioEncoderConfigurations request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetCompatibleAudioEncoderConfigurations")]
pub struct GetCompatibleAudioEncoderConfigurations {
    /// Profile token.
    #[serde(rename = "ProfileToken")]
    pub profile_token: ReferenceToken,
}

/// GetCompatibleAudioEncoderConfigurations response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "trt:GetCompatibleAudioEncoderConfigurationsResponse")]
pub struct GetCompatibleAudioEncoderConfigurationsResponse {
    /// Compatible configurations.
    #[serde(rename = "trt:Configurations", alias = "Configurations", default)]
    pub configurations: Vec<AudioEncoderConfiguration>,
}

// ============================================================================
// Metadata Configurations (FR-002)
// ============================================================================

/// GetMetadataConfigurations request.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetMetadataConfigurations")]
pub struct GetMetadataConfigurations {}

/// GetMetadataConfigurations response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "trt:GetMetadataConfigurationsResponse")]
pub struct GetMetadataConfigurationsResponse {
    /// Metadata configurations.
    #[serde(rename = "trt:Configurations", alias = "Configurations", default)]
    pub configurations: Vec<MetadataConfiguration>,
}

/// Metadata configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetadataConfiguration {
    /// Configuration token.
    #[serde(rename = "@token")]
    pub token: ReferenceToken,

    /// User readable name.
    #[serde(rename = "tt:Name", alias = "Name")]
    pub name: Name,

    /// Number of references using this configuration.
    #[serde(rename = "tt:UseCount", alias = "UseCount")]
    pub use_count: i32,

    /// PTZ status filter.
    #[serde(
        rename = "tt:PTZStatus",
        alias = "PTZStatus",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub ptz_status: Option<PtzFilter>,

    /// Analytics.
    #[serde(
        rename = "tt:Analytics",
        alias = "Analytics",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub analytics: Option<bool>,

    /// Multicast configuration.
    #[serde(
        rename = "tt:Multicast",
        alias = "Multicast",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub multicast: Option<super::common::MulticastConfiguration>,

    /// Session timeout.
    #[serde(rename = "tt:SessionTimeout", alias = "SessionTimeout")]
    pub session_timeout: String,
}

/// PTZ filter for metadata.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PtzFilter {
    /// Enable PTZ status.
    #[serde(rename = "tt:Status", alias = "Status")]
    pub status: bool,

    /// Enable PTZ position.
    #[serde(rename = "tt:Position", alias = "Position")]
    pub position: bool,
}

/// SetMetadataConfiguration request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "SetMetadataConfiguration")]
pub struct SetMetadataConfiguration {
    /// Configuration to set.
    #[serde(rename = "Configuration")]
    pub configuration: MetadataConfiguration,

    /// Force persistence.
    #[serde(
        rename = "ForcePersistence",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub force_persistence: Option<bool>,
}

/// SetMetadataConfiguration response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "trt:SetMetadataConfigurationResponse")]
pub struct SetMetadataConfigurationResponse {}

// ============================================================================
// Multicast Streaming (FR-002)
// ============================================================================

/// StartMulticastStreaming request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "StartMulticastStreaming")]
pub struct StartMulticastStreaming {
    /// Profile token.
    #[serde(rename = "ProfileToken")]
    pub profile_token: ReferenceToken,
}

/// StartMulticastStreaming response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "trt:StartMulticastStreamingResponse")]
pub struct StartMulticastStreamingResponse {}

/// StopMulticastStreaming request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "StopMulticastStreaming")]
pub struct StopMulticastStreaming {
    /// Profile token.
    #[serde(rename = "ProfileToken")]
    pub profile_token: ReferenceToken,
}

/// StopMulticastStreaming response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "trt:StopMulticastStreamingResponse")]
pub struct StopMulticastStreamingResponse {}

/// SetAudioSourceConfiguration request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "SetAudioSourceConfiguration")]
pub struct SetAudioSourceConfiguration {
    /// Configuration to set.
    #[serde(rename = "Configuration")]
    pub configuration: AudioSourceConfiguration,

    /// Force persistence.
    #[serde(
        rename = "ForcePersistence",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub force_persistence: Option<bool>,
}

/// SetAudioSourceConfiguration response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "trt:SetAudioSourceConfigurationResponse")]
pub struct SetAudioSourceConfigurationResponse {}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_h264_profile_default() {
        let profile = H264Profile::default();
        assert_eq!(profile, H264Profile::Baseline);
    }

    #[test]
    fn test_audio_encoding_default() {
        let encoding = AudioEncoding::default();
        assert_eq!(encoding, AudioEncoding::G711);
    }

    #[test]
    fn test_get_profiles_default() {
        let request = GetProfiles::default();
        let _ = request; // Just check it compiles with Default
    }

    #[test]
    fn test_get_profiles_response_default() {
        let response = GetProfilesResponse::default();
        assert!(response.profiles.is_empty());
    }
}
