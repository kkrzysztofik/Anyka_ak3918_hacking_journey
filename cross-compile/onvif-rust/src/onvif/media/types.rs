//! Media Service type re-exports and extensions.
//!
//! This module re-exports WSDL-generated types and provides any
//! implementation-specific extensions needed for Media Service operations.

// Re-export all WSDL types from the generated types module
pub use crate::onvif::types::media::*;

// Re-export common types used by Media Service
pub use crate::onvif::types::common::{
    AudioEncoderConfiguration, AudioSource, AudioSourceConfiguration, IntRange, MediaUri,
    MulticastConfiguration, Name, Profile, ReferenceToken, StreamSetup, StreamType, Transport,
    TransportProtocol, VideoEncoderConfiguration, VideoRateControl, VideoResolution, VideoSource,
    VideoSourceConfiguration,
};

/// Media Service namespace URI.
pub const MEDIA_SERVICE_NAMESPACE: &str = "http://www.onvif.org/ver10/media/wsdl";

/// Default maximum number of profiles supported.
pub const MAX_PROFILES: usize = 16;

/// Default video source token.
pub const DEFAULT_VIDEO_SOURCE_TOKEN: &str = "VideoSource_0";

/// Default audio source token.
pub const DEFAULT_AUDIO_SOURCE_TOKEN: &str = "AudioSource_0";

/// Default RTSP port.
pub const DEFAULT_RTSP_PORT: u16 = 554;

/// Default snapshot HTTP path.
pub const DEFAULT_SNAPSHOT_PATH: &str = "/snapshot.jpg";

/// Profile token prefix for generated profiles.
pub const PROFILE_TOKEN_PREFIX: &str = "Profile_";

/// Configuration token prefix for video source configurations.
pub const VIDEO_SOURCE_CONFIG_PREFIX: &str = "VideoSourceConfig_";

/// Configuration token prefix for video encoder configurations.
pub const VIDEO_ENCODER_CONFIG_PREFIX: &str = "VideoEncoderConfig_";

/// Configuration token prefix for audio source configurations.
pub const AUDIO_SOURCE_CONFIG_PREFIX: &str = "AudioSourceConfig_";

/// Configuration token prefix for audio encoder configurations.
pub const AUDIO_ENCODER_CONFIG_PREFIX: &str = "AudioEncoderConfig_";
