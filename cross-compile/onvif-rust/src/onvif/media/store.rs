//! Media Store - Profile persistence and conversion.
//!
//! This module provides persistence layer for media profiles and configurations,
//! handling serialization/deserialization to/from storage format.

use std::sync::Arc;

#[allow(unused_imports)]
use crate::config::profiles::{
    ProfilesFile, StoredAudioEncoderConfig, StoredAudioSource, StoredAudioSourceConfig,
    StoredProfile, StoredVideoEncoderConfig, StoredVideoSource, StoredVideoSourceConfig,
};
use crate::config::{ConfigRuntime, ProfileStorage};
use crate::onvif::types::common::{
    AudioEncoderConfiguration, AudioSource, AudioSourceConfiguration, IntRectangle,
    MulticastConfiguration, PTZConfiguration, VideoEncoderConfiguration, VideoRateControl,
    VideoResolution, VideoSourceConfiguration,
};
use crate::platform::Resolution;

use super::state::MediaState;

#[allow(unused_imports)]
use super::types::{
    AUDIO_ENCODER_CONFIG_PREFIX, AUDIO_SOURCE_CONFIG_PREFIX, DEFAULT_AUDIO_SOURCE_TOKEN,
    DEFAULT_PTZ_NODE_TOKEN, DEFAULT_VIDEO_SOURCE_TOKEN, PROFILE_TOKEN_PREFIX, PTZ_CONFIG_PREFIX,
    VIDEO_ENCODER_CONFIG_PREFIX, VIDEO_SOURCE_CONFIG_PREFIX,
};

/// Media store handles persistence and data conversion.
#[allow(dead_code)]
pub struct MediaStore {
    /// Runtime configuration (optional).
    config: Option<Arc<ConfigRuntime>>,
    /// Typed profile storage for persistence.
    #[allow(dead_code)]
    profile_storage: Option<Arc<ProfileStorage>>,
    /// Maximum sensor resolution.
    max_sensor_resolution: Resolution,
}

impl MediaStore {
    /// Create a new MediaStore.
    pub fn new(config: Option<Arc<ConfigRuntime>>, max_resolution: Resolution) -> Self {
        Self {
            config,
            profile_storage: None,
            max_sensor_resolution: max_resolution,
        }
    }

    /// Create a MediaStore with profile storage.
    pub fn with_storage(
        config: Option<Arc<ConfigRuntime>>,
        profile_storage: Arc<ProfileStorage>,
        max_resolution: Resolution,
    ) -> Self {
        Self {
            config,
            profile_storage: Some(profile_storage),
            max_sensor_resolution: max_resolution,
        }
    }

    /// Initialize default sources, configurations, and profiles in state.
    pub fn initialize_defaults(&self, state: &MediaState) {
        // Create default video source with sensor-aware resolution
        let video_source = crate::onvif::types::common::VideoSource {
            token: DEFAULT_VIDEO_SOURCE_TOKEN.to_string(),
            framerate: 30.0,
            resolution: VideoResolution {
                width: self.max_sensor_resolution.width as i32,
                height: self.max_sensor_resolution.height as i32,
            },
            imaging: None,
            extension: None,
        };
        state.insert_video_source(video_source);

        // Create default audio source
        let audio_source = AudioSource {
            token: DEFAULT_AUDIO_SOURCE_TOKEN.to_string(),
            channels: 1,
        };
        state.insert_audio_source(audio_source);

        // Create default video source configuration
        let video_source_config = VideoSourceConfiguration {
            token: format!("{}0", VIDEO_SOURCE_CONFIG_PREFIX),
            source_token: DEFAULT_VIDEO_SOURCE_TOKEN.to_string(),
            name: "VideoSourceConfig_0".to_string(),
            use_count: 1,
            view_mode: None,
            bounds: IntRectangle {
                x: 0,
                y: 0,
                width: self.max_sensor_resolution.width as i32,
                height: self.max_sensor_resolution.height as i32,
            },
            extension: None,
        };
        state.insert_video_source_config(video_source_config);

        // Create default audio source configuration
        let audio_source_config = AudioSourceConfiguration {
            token: format!("{}0", AUDIO_SOURCE_CONFIG_PREFIX),
            source_token: DEFAULT_AUDIO_SOURCE_TOKEN.to_string(),
            name: "AudioSourceConfig_0".to_string(),
            use_count: 1,
        };
        state.insert_audio_source_config(audio_source_config);

        // Initialize profiles from config or hardcoded defaults
        if let Some(ref config) = self.config {
            self.initialize_profiles_from_config(state, config);
        } else {
            self.initialize_profiles_hardcoded(state);
        }
    }

    /// Initialize profiles from config sections.
    fn initialize_profiles_from_config(&self, state: &MediaState, config: &Arc<ConfigRuntime>) {
        let mut profile_count = 0u32;
        let default_ptz_config = Self::create_default_ptz_configuration();

        for profile_num in 1..=4 {
            if !Self::is_profile_enabled(config, profile_num) {
                continue;
            }

            let profile_config = Self::read_profile_config(config, profile_num);

            // Validate against max sensor resolution
            if profile_config.width > self.max_sensor_resolution.width
                || profile_config.height > self.max_sensor_resolution.height
            {
                tracing::error!(
                    "Skipping stream_profile_{}: resolution {}x{} exceeds sensor max {}x{}",
                    profile_num,
                    profile_config.width,
                    profile_config.height,
                    self.max_sensor_resolution.width,
                    self.max_sensor_resolution.height
                );
                continue;
            }

            let video_encoding = Self::parse_video_encoding(&profile_config.encoding_str);
            let h264_profile = Self::parse_h264_profile(&profile_config.profile_str);
            let audio_encoding = Self::parse_audio_encoding(&profile_config.audio_encoding_str);

            // Create video encoder config
            let video_encoder_config = Self::create_video_encoder_config(
                profile_count,
                &profile_config.name,
                profile_config.width,
                profile_config.height,
                profile_config.framerate,
                profile_config.bitrate,
                &video_encoding,
                h264_profile,
            );
            state.insert_video_encoder_config(video_encoder_config.clone());

            // Create audio encoder config if enabled
            if profile_config.audio_enabled {
                let audio_encoder_config = Self::create_audio_encoder_config(
                    profile_count,
                    profile_config.audio_bitrate,
                    profile_config.audio_sample_rate,
                    audio_encoding,
                );
                state.insert_audio_encoder_config(audio_encoder_config.clone());

                // Create profile with audio
                let profile = Self::create_profile(
                    &profile_config.name,
                    profile_count,
                    Some(video_encoder_config),
                    Some(audio_encoder_config),
                    &default_ptz_config,
                );
                state.insert_profile(profile);
            } else {
                // Create profile without audio
                let profile = Self::create_profile(
                    &profile_config.name,
                    profile_count,
                    Some(video_encoder_config),
                    None,
                    &default_ptz_config,
                );
                state.insert_profile(profile);
            }

            profile_count += 1;
        }

        state.set_profile_counter(profile_count);
    }

    /// Initialize hardcoded profiles (fallback).
    fn initialize_profiles_hardcoded(&self, state: &MediaState) {
        let default_ptz_config = Self::create_default_ptz_configuration();

        // Main stream video encoder config
        let video_encoder_main = VideoEncoderConfiguration {
            token: format!("{}0", VIDEO_ENCODER_CONFIG_PREFIX),
            name: "MainStream".to_string(),
            use_count: 1,
            encoding: crate::onvif::types::common::VideoEncoding::H264,
            resolution: VideoResolution {
                width: 1920,
                height: 1080,
            },
            quality: 0.8,
            rate_control: Some(VideoRateControl {
                frame_rate_limit: 30,
                encoding_interval: 1,
                bitrate_limit: 4000,
            }),
            mpeg4: None,
            h264: Some(crate::onvif::types::common::H264Configuration {
                gov_length: 30,
                h264_profile: crate::onvif::types::common::H264Profile::Main,
            }),
            multicast: Some(MulticastConfiguration {
                address: crate::onvif::types::common::IpAddress {
                    address_type: crate::onvif::types::common::IpType::IPv4,
                    ipv4_address: Some("0.0.0.0".to_string()),
                    ipv6_address: None,
                },
                port: 0,
                ttl: 0,
                auto_start: false,
            }),
            session_timeout: "PT60S".to_string(),
        };
        state.insert_video_encoder_config(video_encoder_main.clone());

        // Sub stream video encoder config
        let video_encoder_sub = VideoEncoderConfiguration {
            token: format!("{}1", VIDEO_ENCODER_CONFIG_PREFIX),
            name: "SubStream".to_string(),
            use_count: 1,
            encoding: crate::onvif::types::common::VideoEncoding::H264,
            resolution: VideoResolution {
                width: 640,
                height: 480,
            },
            quality: 0.5,
            rate_control: Some(VideoRateControl {
                frame_rate_limit: 15,
                encoding_interval: 1,
                bitrate_limit: 512,
            }),
            mpeg4: None,
            h264: Some(crate::onvif::types::common::H264Configuration {
                gov_length: 30,
                h264_profile: crate::onvif::types::common::H264Profile::Baseline,
            }),
            multicast: Some(MulticastConfiguration {
                address: crate::onvif::types::common::IpAddress {
                    address_type: crate::onvif::types::common::IpType::IPv4,
                    ipv4_address: Some("0.0.0.0".to_string()),
                    ipv6_address: None,
                },
                port: 0,
                ttl: 0,
                auto_start: false,
            }),
            session_timeout: "PT60S".to_string(),
        };
        state.insert_video_encoder_config(video_encoder_sub.clone());

        // Audio encoder config
        let audio_encoder = AudioEncoderConfiguration {
            token: format!("{}0", AUDIO_ENCODER_CONFIG_PREFIX),
            name: "AudioEncoderConfig_0".to_string(),
            use_count: 1,
            encoding: crate::onvif::types::common::AudioEncoding::G711,
            bitrate: 64,
            sample_rate: 8000,
            multicast: Some(MulticastConfiguration {
                address: crate::onvif::types::common::IpAddress {
                    address_type: crate::onvif::types::common::IpType::IPv4,
                    ipv4_address: Some("0.0.0.0".to_string()),
                    ipv6_address: None,
                },
                port: 0,
                ttl: 0,
                auto_start: false,
            }),
            session_timeout: "PT60S".to_string(),
        };
        state.insert_audio_encoder_config(audio_encoder.clone());

        // Main profile
        let main_profile = Self::create_profile(
            "MainStream",
            0,
            Some(video_encoder_main.clone()),
            Some(audio_encoder.clone()),
            &default_ptz_config,
        );
        state.insert_profile(main_profile);

        // Sub profile
        let sub_profile = Self::create_profile(
            "SubStream",
            1,
            Some(video_encoder_sub),
            Some(audio_encoder),
            &default_ptz_config,
        );
        state.insert_profile(sub_profile);

        state.set_profile_counter(2);
    }

    /// Create a profile with given parameters.
    fn create_profile(
        name: &str,
        profile_count: u32,
        video_encoder: Option<VideoEncoderConfiguration>,
        audio_encoder: Option<AudioEncoderConfiguration>,
        ptz_config: &PTZConfiguration,
    ) -> crate::onvif::types::common::Profile {
        let profile_token = format!("{}{}", PROFILE_TOKEN_PREFIX, name);
        crate::onvif::types::common::Profile {
            token: profile_token.clone(),
            fixed: Some(true),
            name: name.to_string(),
            video_source_configuration: Some(VideoSourceConfiguration {
                token: format!("{}0", VIDEO_SOURCE_CONFIG_PREFIX),
                source_token: DEFAULT_VIDEO_SOURCE_TOKEN.to_string(),
                name: "VideoSourceConfig_0".to_string(),
                use_count: (profile_count + 1) as i32,
                view_mode: None,
                bounds: IntRectangle {
                    x: 0,
                    y: 0,
                    width: video_encoder
                        .as_ref()
                        .map(|c| c.resolution.width)
                        .unwrap_or(1920),
                    height: video_encoder
                        .as_ref()
                        .map(|c| c.resolution.height)
                        .unwrap_or(1080),
                },
                extension: None,
            }),
            audio_source_configuration: if audio_encoder.is_some() {
                Some(AudioSourceConfiguration {
                    token: format!("{}0", AUDIO_SOURCE_CONFIG_PREFIX),
                    source_token: DEFAULT_AUDIO_SOURCE_TOKEN.to_string(),
                    name: "AudioSourceConfig_0".to_string(),
                    use_count: (profile_count + 1) as i32,
                })
            } else {
                None
            },
            video_encoder_configuration: video_encoder,
            audio_encoder_configuration: audio_encoder,
            ptz_configuration: Some(ptz_config.clone()),
            metadata_configuration: None,
            extension: None,
        }
    }

    /// Check if a profile is enabled in config.
    fn is_profile_enabled(config: &ConfigRuntime, profile_num: u32) -> bool {
        config.read().stream_profile(profile_num).enabled
    }

    /// Read profile configuration from config.
    fn read_profile_config(config: &ConfigRuntime, profile_num: u32) -> ProfileConfig {
        let c = config.read();
        let sp = c.stream_profile(profile_num);
        ProfileConfig {
            name: if sp.name.is_empty() {
                format!("Stream{}", profile_num)
            } else {
                sp.name.clone()
            },
            width: sp.width,
            height: sp.height,
            framerate: sp.framerate,
            bitrate: sp.bitrate,
            encoding_str: sp.encoding.clone(),
            profile_str: sp.profile.clone(),
            audio_enabled: sp.audio_enabled,
            audio_encoding_str: sp.audio_encoding.clone(),
            audio_bitrate: sp.audio_bitrate,
            audio_sample_rate: sp.audio_sample_rate,
        }
    }

    /// Parse video encoding string.
    fn parse_video_encoding(encoding_str: &str) -> crate::onvif::types::common::VideoEncoding {
        match encoding_str.to_lowercase().as_str() {
            "h264" => crate::onvif::types::common::VideoEncoding::H264,
            "mjpeg" | "jpeg" => crate::onvif::types::common::VideoEncoding::JPEG,
            "mpeg4" => crate::onvif::types::common::VideoEncoding::MPEG4,
            _ => crate::onvif::types::common::VideoEncoding::H264,
        }
    }

    /// Parse H.264 profile string.
    fn parse_h264_profile(profile_str: &str) -> crate::onvif::types::common::H264Profile {
        match profile_str.to_lowercase().as_str() {
            "baseline" => crate::onvif::types::common::H264Profile::Baseline,
            "main" => crate::onvif::types::common::H264Profile::Main,
            "high" => crate::onvif::types::common::H264Profile::High,
            _ => crate::onvif::types::common::H264Profile::Main,
        }
    }

    /// Parse audio encoding string.
    fn parse_audio_encoding(encoding_str: &str) -> crate::onvif::types::common::AudioEncoding {
        match encoding_str.to_lowercase().as_str() {
            "g711" => crate::onvif::types::common::AudioEncoding::G711,
            "aac" => crate::onvif::types::common::AudioEncoding::AAC,
            "g726" => crate::onvif::types::common::AudioEncoding::G726,
            _ => crate::onvif::types::common::AudioEncoding::G711,
        }
    }

    /// Create video encoder configuration.
    #[allow(clippy::too_many_arguments)]
    fn create_video_encoder_config(
        profile_count: u32,
        name: &str,
        width: u32,
        height: u32,
        framerate: u32,
        bitrate: u32,
        video_encoding: &crate::onvif::types::common::VideoEncoding,
        h264_profile: crate::onvif::types::common::H264Profile,
    ) -> VideoEncoderConfiguration {
        let token = format!("{}{}", VIDEO_ENCODER_CONFIG_PREFIX, profile_count);
        VideoEncoderConfiguration {
            token: token.clone(),
            name: name.to_string(),
            use_count: 1,
            encoding: video_encoding.clone(),
            resolution: VideoResolution {
                width: width as i32,
                height: height as i32,
            },
            quality: 0.8,
            rate_control: Some(VideoRateControl {
                frame_rate_limit: framerate as i32,
                encoding_interval: 1,
                bitrate_limit: bitrate as i32,
            }),
            mpeg4: None,
            h264: if matches!(
                video_encoding,
                crate::onvif::types::common::VideoEncoding::H264
            ) {
                Some(crate::onvif::types::common::H264Configuration {
                    gov_length: framerate as i32,
                    h264_profile,
                })
            } else {
                None
            },
            multicast: Some(MulticastConfiguration {
                address: crate::onvif::types::common::IpAddress {
                    address_type: crate::onvif::types::common::IpType::IPv4,
                    ipv4_address: Some("0.0.0.0".to_string()),
                    ipv6_address: None,
                },
                port: 0,
                ttl: 0,
                auto_start: false,
            }),
            session_timeout: "PT60S".to_string(),
        }
    }

    /// Create audio encoder configuration.
    fn create_audio_encoder_config(
        profile_count: u32,
        bitrate: u32,
        sample_rate: u32,
        encoding: crate::onvif::types::common::AudioEncoding,
    ) -> AudioEncoderConfiguration {
        let token = format!("{}{}", AUDIO_ENCODER_CONFIG_PREFIX, profile_count);
        AudioEncoderConfiguration {
            token: token.clone(),
            name: format!("AudioEncoderConfig_{}", profile_count),
            use_count: 1,
            encoding,
            bitrate: bitrate as i32,
            sample_rate: sample_rate as i32,
            multicast: Some(MulticastConfiguration {
                address: crate::onvif::types::common::IpAddress {
                    address_type: crate::onvif::types::common::IpType::IPv4,
                    ipv4_address: Some("0.0.0.0".to_string()),
                    ipv6_address: None,
                },
                port: 0,
                ttl: 0,
                auto_start: false,
            }),
            session_timeout: "PT60S".to_string(),
        }
    }

    /// Create default PTZ configuration.
    fn create_default_ptz_configuration() -> PTZConfiguration {
        use crate::onvif::types::common::{
            FloatRange, PTZSpeed, PanTiltLimits, Space1DDescription, Space2DDescription, Vector1D,
            Vector2D, ZoomLimits,
        };

        PTZConfiguration {
            token: format!("{}0", PTZ_CONFIG_PREFIX),
            name: "DefaultPTZConfig".to_string(),
            use_count: 2,
            move_ramp: None,
            preset_ramp: None,
            preset_tour_ramp: None,
            node_token: DEFAULT_PTZ_NODE_TOKEN.to_string(),
            default_absolute_pan_tilt_position_space: Some(
                "http://www.onvif.org/ver10/tptz/PanTiltSpaces/PositionGenericSpace".to_string(),
            ),
            default_absolute_zoom_position_space: Some(
                "http://www.onvif.org/ver10/tptz/ZoomSpaces/PositionGenericSpace".to_string(),
            ),
            default_relative_pan_tilt_translation_space: Some(
                "http://www.onvif.org/ver10/tptz/PanTiltSpaces/TranslationGenericSpace".to_string(),
            ),
            default_relative_zoom_translation_space: Some(
                "http://www.onvif.org/ver10/tptz/ZoomSpaces/TranslationGenericSpace".to_string(),
            ),
            default_continuous_pan_tilt_velocity_space: Some(
                "http://www.onvif.org/ver10/tptz/PanTiltSpaces/VelocityGenericSpace".to_string(),
            ),
            default_continuous_zoom_velocity_space: Some(
                "http://www.onvif.org/ver10/tptz/ZoomSpaces/VelocityGenericSpace".to_string(),
            ),
            default_ptz_speed: Some(PTZSpeed {
                pan_tilt: Some(Vector2D {
                    x: 0.5,
                    y: 0.5,
                    space: None,
                }),
                zoom: Some(Vector1D {
                    x: 0.5,
                    space: None,
                }),
            }),
            default_ptz_timeout: Some("PT5S".to_string()),
            pan_tilt_limits: Some(PanTiltLimits {
                range: Space2DDescription {
                    uri: "http://www.onvif.org/ver10/tptz/PanTiltSpaces/PositionGenericSpace"
                        .to_string(),
                    x_range: FloatRange {
                        min: -1.0,
                        max: 1.0,
                    },
                    y_range: FloatRange {
                        min: -1.0,
                        max: 1.0,
                    },
                },
            }),
            zoom_limits: Some(ZoomLimits {
                range: Space1DDescription {
                    uri: "http://www.onvif.org/ver10/tptz/ZoomSpaces/PositionGenericSpace"
                        .to_string(),
                    x_range: FloatRange { min: 0.0, max: 1.0 },
                },
            }),
            extension: None,
        }
    }

    /// Save state to storage.
    pub fn save(&self, _state: &MediaState) {
        // Note: Full persistence implementation would serialize state to storage
        // For now, this is a placeholder - the ProfileManager handles persistence
    }
}

/// Profile configuration read from config file.
struct ProfileConfig {
    name: String,
    width: u32,
    height: u32,
    framerate: u32,
    bitrate: u32,
    encoding_str: String,
    profile_str: String,
    audio_enabled: bool,
    audio_encoding_str: String,
    audio_bitrate: u32,
    audio_sample_rate: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_media_store_new() {
        let store = MediaStore::new(None, Resolution::new(1920, 1080));
        // Basic creation test
        assert_eq!(store.max_sensor_resolution.width, 1920);
    }
}
