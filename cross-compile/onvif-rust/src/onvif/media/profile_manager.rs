//! Media Profile Manager.
//!
//! This module provides thread-safe management of media profiles including:
//! - Profile storage and retrieval
//! - Profile creation and deletion
//! - Profile token validation
//! - Video/audio source and encoder configuration management

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};

use parking_lot::RwLock;

use crate::onvif::error::{OnvifError, OnvifResult};
use crate::onvif::types::common::{
    AudioEncoderConfiguration, AudioSource, AudioSourceConfiguration, FloatRange, IntRange,
    IntRectangle, MulticastConfiguration, Name, PTZConfiguration, PTZSpeed, PanTiltLimits, Profile,
    ReferenceToken, Space1DDescription, Space2DDescription, Vector1D, Vector2D,
    VideoEncoderConfiguration, VideoRateControl, VideoResolution, VideoSource,
    VideoSourceConfiguration, ZoomLimits,
};
use crate::onvif::types::media::{
    AudioEncoderConfigurationOptions, H264Options, H264Profile, JpegOptions,
    VideoEncoderConfigurationOptions, VideoSourceConfigurationOptions,
};

use super::faults::{
    no_config_error, no_profile_error, validate_profile_name, validate_profile_token,
};
use super::types::{
    AUDIO_ENCODER_CONFIG_PREFIX, AUDIO_SOURCE_CONFIG_PREFIX, DEFAULT_AUDIO_SOURCE_TOKEN,
    DEFAULT_PTZ_NODE_TOKEN, DEFAULT_VIDEO_SOURCE_TOKEN, MAX_PROFILES, PROFILE_TOKEN_PREFIX,
    PTZ_CONFIG_PREFIX, VIDEO_ENCODER_CONFIG_PREFIX, VIDEO_SOURCE_CONFIG_PREFIX,
};

/// Profile Manager for managing media profiles.
///
/// Provides thread-safe access to profiles and configurations.
pub struct ProfileManager {
    /// Profiles storage.
    profiles: RwLock<HashMap<ReferenceToken, Profile>>,
    /// Video sources storage.
    video_sources: RwLock<HashMap<ReferenceToken, VideoSource>>,
    /// Audio sources storage.
    audio_sources: RwLock<HashMap<ReferenceToken, AudioSource>>,
    /// Video source configurations.
    video_source_configs: RwLock<HashMap<ReferenceToken, VideoSourceConfiguration>>,
    /// Video encoder configurations.
    video_encoder_configs: RwLock<HashMap<ReferenceToken, VideoEncoderConfiguration>>,
    /// Audio source configurations.
    audio_source_configs: RwLock<HashMap<ReferenceToken, AudioSourceConfiguration>>,
    /// Audio encoder configurations.
    audio_encoder_configs: RwLock<HashMap<ReferenceToken, AudioEncoderConfiguration>>,
    /// Profile counter for generating unique tokens.
    profile_counter: AtomicU32,
}

impl ProfileManager {
    /// Create a new ProfileManager with default profiles.
    pub fn new() -> Self {
        let manager = Self {
            profiles: RwLock::new(HashMap::new()),
            video_sources: RwLock::new(HashMap::new()),
            audio_sources: RwLock::new(HashMap::new()),
            video_source_configs: RwLock::new(HashMap::new()),
            video_encoder_configs: RwLock::new(HashMap::new()),
            audio_source_configs: RwLock::new(HashMap::new()),
            audio_encoder_configs: RwLock::new(HashMap::new()),
            profile_counter: AtomicU32::new(0),
        };
        manager.initialize_defaults();
        manager
    }

    /// Initialize default sources, configurations, and profiles.
    fn initialize_defaults(&self) {
        // Create default PTZ configuration used by all profiles
        let default_ptz_config = Self::create_default_ptz_configuration();

        // Create default video source
        let video_source = VideoSource {
            token: DEFAULT_VIDEO_SOURCE_TOKEN.to_string(),
            framerate: 30.0,
            resolution: VideoResolution {
                width: 1920,
                height: 1080,
            },
            imaging: None,
            extension: None,
        };
        self.video_sources
            .write()
            .insert(DEFAULT_VIDEO_SOURCE_TOKEN.to_string(), video_source);

        // Create default audio source
        let audio_source = AudioSource {
            token: DEFAULT_AUDIO_SOURCE_TOKEN.to_string(),
            channels: 1,
        };
        self.audio_sources
            .write()
            .insert(DEFAULT_AUDIO_SOURCE_TOKEN.to_string(), audio_source);

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
                width: 1920,
                height: 1080,
            },
            extension: None,
        };
        self.video_source_configs.write().insert(
            format!("{}0", VIDEO_SOURCE_CONFIG_PREFIX),
            video_source_config,
        );

        // Create default video encoder configuration (Main stream - H.264)
        let video_encoder_config_main = VideoEncoderConfiguration {
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
        self.video_encoder_configs.write().insert(
            format!("{}0", VIDEO_ENCODER_CONFIG_PREFIX),
            video_encoder_config_main,
        );

        // Create sub-stream encoder config (lower resolution)
        let video_encoder_config_sub = VideoEncoderConfiguration {
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
        self.video_encoder_configs.write().insert(
            format!("{}1", VIDEO_ENCODER_CONFIG_PREFIX),
            video_encoder_config_sub,
        );

        // Create default audio source configuration
        let audio_source_config = AudioSourceConfiguration {
            token: format!("{}0", AUDIO_SOURCE_CONFIG_PREFIX),
            source_token: DEFAULT_AUDIO_SOURCE_TOKEN.to_string(),
            name: "AudioSourceConfig_0".to_string(),
            use_count: 1,
        };
        self.audio_source_configs.write().insert(
            format!("{}0", AUDIO_SOURCE_CONFIG_PREFIX),
            audio_source_config,
        );

        // Create default audio encoder configuration
        let audio_encoder_config = AudioEncoderConfiguration {
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
        self.audio_encoder_configs.write().insert(
            format!("{}0", AUDIO_ENCODER_CONFIG_PREFIX),
            audio_encoder_config,
        );

        // Create default profile (Main Stream)
        let main_profile = Profile {
            token: format!("{}MainStream", PROFILE_TOKEN_PREFIX),
            fixed: Some(true),
            name: "MainStream".to_string(),
            video_source_configuration: Some(VideoSourceConfiguration {
                token: format!("{}0", VIDEO_SOURCE_CONFIG_PREFIX),
                source_token: DEFAULT_VIDEO_SOURCE_TOKEN.to_string(),
                name: "VideoSourceConfig_0".to_string(),
                use_count: 1,
                view_mode: None,
                bounds: IntRectangle {
                    x: 0,
                    y: 0,
                    width: 1920,
                    height: 1080,
                },
                extension: None,
            }),
            audio_source_configuration: Some(AudioSourceConfiguration {
                token: format!("{}0", AUDIO_SOURCE_CONFIG_PREFIX),
                source_token: DEFAULT_AUDIO_SOURCE_TOKEN.to_string(),
                name: "AudioSourceConfig_0".to_string(),
                use_count: 1,
            }),
            video_encoder_configuration: Some(VideoEncoderConfiguration {
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
            }),
            audio_encoder_configuration: Some(AudioEncoderConfiguration {
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
            }),
            ptz_configuration: Some(default_ptz_config.clone()),
            metadata_configuration: None,
            extension: None,
        };
        self.profiles
            .write()
            .insert(format!("{}MainStream", PROFILE_TOKEN_PREFIX), main_profile);

        // Create sub-stream profile
        let sub_profile = Profile {
            token: format!("{}SubStream", PROFILE_TOKEN_PREFIX),
            fixed: Some(true),
            name: "SubStream".to_string(),
            video_source_configuration: Some(VideoSourceConfiguration {
                token: format!("{}0", VIDEO_SOURCE_CONFIG_PREFIX),
                source_token: DEFAULT_VIDEO_SOURCE_TOKEN.to_string(),
                name: "VideoSourceConfig_0".to_string(),
                use_count: 2,
                view_mode: None,
                bounds: IntRectangle {
                    x: 0,
                    y: 0,
                    width: 640,
                    height: 480,
                },
                extension: None,
            }),
            audio_source_configuration: Some(AudioSourceConfiguration {
                token: format!("{}0", AUDIO_SOURCE_CONFIG_PREFIX),
                source_token: DEFAULT_AUDIO_SOURCE_TOKEN.to_string(),
                name: "AudioSourceConfig_0".to_string(),
                use_count: 2,
            }),
            video_encoder_configuration: Some(VideoEncoderConfiguration {
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
            }),
            audio_encoder_configuration: Some(AudioEncoderConfiguration {
                token: format!("{}0", AUDIO_ENCODER_CONFIG_PREFIX),
                name: "AudioEncoderConfig_0".to_string(),
                use_count: 2,
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
            }),
            ptz_configuration: Some(default_ptz_config),
            metadata_configuration: None,
            extension: None,
        };
        self.profiles
            .write()
            .insert(format!("{}SubStream", PROFILE_TOKEN_PREFIX), sub_profile);

        self.profile_counter.store(2, Ordering::SeqCst);
    }

    // ========================================================================
    // Profile Operations
    // ========================================================================

    /// Get all profiles.
    pub fn get_profiles(&self) -> Vec<Profile> {
        self.profiles.read().values().cloned().collect()
    }

    /// Get a profile by token.
    pub fn get_profile(&self, token: &ReferenceToken) -> OnvifResult<Profile> {
        validate_profile_token(token)?;
        self.profiles
            .read()
            .get(token)
            .cloned()
            .ok_or_else(|| no_profile_error(token))
    }

    /// Create a new profile.
    ///
    /// Returns the created profile with a generated token if not provided.
    pub fn create_profile(
        &self,
        name: Name,
        token: Option<ReferenceToken>,
    ) -> OnvifResult<Profile> {
        validate_profile_name(&name)?;

        let mut profiles = self.profiles.write();

        // Check max profiles limit
        if profiles.len() >= MAX_PROFILES {
            return Err(OnvifError::invalid_arg_val(
                "ter:MaxProfiles",
                &format!("Maximum number of profiles ({}) reached", MAX_PROFILES),
            ));
        }

        // Generate or validate token
        let profile_token = if let Some(t) = token {
            validate_profile_token(&t)?;
            if profiles.contains_key(&t) {
                return Err(OnvifError::invalid_arg_val(
                    "ter:TokenConflict",
                    &format!("Profile with token '{}' already exists", t),
                ));
            }
            t
        } else {
            let counter = self.profile_counter.fetch_add(1, Ordering::SeqCst);
            format!("{}{}", PROFILE_TOKEN_PREFIX, counter)
        };

        let profile = Profile {
            token: profile_token.clone(),
            fixed: Some(false),
            name,
            video_source_configuration: None,
            audio_source_configuration: None,
            video_encoder_configuration: None,
            audio_encoder_configuration: None,
            ptz_configuration: None,
            metadata_configuration: None,
            extension: None,
        };

        profiles.insert(profile_token, profile.clone());
        Ok(profile)
    }

    /// Delete a profile.
    pub fn delete_profile(&self, token: &ReferenceToken) -> OnvifResult<()> {
        validate_profile_token(token)?;

        let mut profiles = self.profiles.write();

        // Check if profile exists
        let profile = profiles.get(token).ok_or_else(|| no_profile_error(token))?;

        // Cannot delete fixed profiles
        if profile.fixed.unwrap_or(false) {
            return Err(OnvifError::invalid_arg_val(
                "ter:DeletionOfFixedProfile",
                "Cannot delete a fixed profile",
            ));
        }

        profiles.remove(token);
        Ok(())
    }

    // ========================================================================
    // Video Source Operations
    // ========================================================================

    /// Get all video sources.
    pub fn get_video_sources(&self) -> Vec<VideoSource> {
        self.video_sources.read().values().cloned().collect()
    }

    /// Get all video source configurations.
    pub fn get_video_source_configurations(&self) -> Vec<VideoSourceConfiguration> {
        self.video_source_configs.read().values().cloned().collect()
    }

    /// Get a video source configuration by token.
    pub fn get_video_source_configuration(
        &self,
        token: &ReferenceToken,
    ) -> OnvifResult<VideoSourceConfiguration> {
        self.video_source_configs
            .read()
            .get(token)
            .cloned()
            .ok_or_else(|| no_config_error(token))
    }

    /// Set video source configuration.
    pub fn set_video_source_configuration(
        &self,
        config: VideoSourceConfiguration,
    ) -> OnvifResult<()> {
        let mut configs = self.video_source_configs.write();
        configs.insert(config.token.clone(), config);
        Ok(())
    }

    /// Get video source configuration options.
    pub fn get_video_source_configuration_options(&self) -> VideoSourceConfigurationOptions {
        let video_source_tokens: Vec<ReferenceToken> =
            self.video_sources.read().keys().cloned().collect();

        VideoSourceConfigurationOptions {
            maximum_number_of_profiles: Some(MAX_PROFILES as i32),
            bounds_range: Some(crate::onvif::types::media::IntRectangleRange {
                x_range: IntRange { min: 0, max: 0 },
                y_range: IntRange { min: 0, max: 0 },
                width_range: IntRange {
                    min: 160,
                    max: 1920,
                },
                height_range: IntRange {
                    min: 120,
                    max: 1080,
                },
            }),
            video_source_tokens_available: video_source_tokens,
            extension: None,
        }
    }

    // ========================================================================
    // Video Encoder Operations
    // ========================================================================

    /// Get all video encoder configurations.
    pub fn get_video_encoder_configurations(&self) -> Vec<VideoEncoderConfiguration> {
        self.video_encoder_configs
            .read()
            .values()
            .cloned()
            .collect()
    }

    /// Get a video encoder configuration by token.
    pub fn get_video_encoder_configuration(
        &self,
        token: &ReferenceToken,
    ) -> OnvifResult<VideoEncoderConfiguration> {
        self.video_encoder_configs
            .read()
            .get(token)
            .cloned()
            .ok_or_else(|| no_config_error(token))
    }

    /// Set video encoder configuration.
    pub fn set_video_encoder_configuration(
        &self,
        config: VideoEncoderConfiguration,
    ) -> OnvifResult<()> {
        let mut configs = self.video_encoder_configs.write();
        configs.insert(config.token.clone(), config);
        Ok(())
    }

    /// Get video encoder configuration options.
    pub fn get_video_encoder_configuration_options(&self) -> VideoEncoderConfigurationOptions {
        VideoEncoderConfigurationOptions {
            quality_range: IntRange { min: 0, max: 100 },
            jpeg: Some(JpegOptions {
                resolutions_available: vec![
                    VideoResolution {
                        width: 1920,
                        height: 1080,
                    },
                    VideoResolution {
                        width: 1280,
                        height: 720,
                    },
                    VideoResolution {
                        width: 640,
                        height: 480,
                    },
                    VideoResolution {
                        width: 320,
                        height: 240,
                    },
                ],
                frame_rate_range: IntRange { min: 1, max: 30 },
                encoding_interval_range: IntRange { min: 1, max: 30 },
            }),
            mpeg4: None,
            h264: Some(H264Options {
                resolutions_available: vec![
                    VideoResolution {
                        width: 1920,
                        height: 1080,
                    },
                    VideoResolution {
                        width: 1280,
                        height: 720,
                    },
                    VideoResolution {
                        width: 640,
                        height: 480,
                    },
                    VideoResolution {
                        width: 320,
                        height: 240,
                    },
                ],
                gov_length_range: IntRange { min: 1, max: 300 },
                frame_rate_range: IntRange { min: 1, max: 30 },
                encoding_interval_range: IntRange { min: 1, max: 30 },
                h264_profiles_supported: vec![
                    H264Profile::Baseline,
                    H264Profile::Main,
                    H264Profile::High,
                ],
            }),
            extension: None,
        }
    }

    // ========================================================================
    // Audio Source Operations
    // ========================================================================

    /// Get all audio sources.
    pub fn get_audio_sources(&self) -> Vec<AudioSource> {
        self.audio_sources.read().values().cloned().collect()
    }

    /// Get all audio source configurations.
    pub fn get_audio_source_configurations(&self) -> Vec<AudioSourceConfiguration> {
        self.audio_source_configs.read().values().cloned().collect()
    }

    /// Get an audio source configuration by token.
    pub fn get_audio_source_configuration(
        &self,
        token: &ReferenceToken,
    ) -> OnvifResult<AudioSourceConfiguration> {
        self.audio_source_configs
            .read()
            .get(token)
            .cloned()
            .ok_or_else(|| no_config_error(token))
    }

    /// Set audio source configuration.
    pub fn set_audio_source_configuration(
        &self,
        config: AudioSourceConfiguration,
    ) -> OnvifResult<()> {
        let mut configs = self.audio_source_configs.write();
        configs.insert(config.token.clone(), config);
        Ok(())
    }

    // ========================================================================
    // Audio Encoder Operations
    // ========================================================================

    /// Get all audio encoder configurations.
    pub fn get_audio_encoder_configurations(&self) -> Vec<AudioEncoderConfiguration> {
        self.audio_encoder_configs
            .read()
            .values()
            .cloned()
            .collect()
    }

    /// Get an audio encoder configuration by token.
    pub fn get_audio_encoder_configuration(
        &self,
        token: &ReferenceToken,
    ) -> OnvifResult<AudioEncoderConfiguration> {
        self.audio_encoder_configs
            .read()
            .get(token)
            .cloned()
            .ok_or_else(|| no_config_error(token))
    }

    /// Set audio encoder configuration.
    pub fn set_audio_encoder_configuration(
        &self,
        config: AudioEncoderConfiguration,
    ) -> OnvifResult<()> {
        let mut configs = self.audio_encoder_configs.write();
        configs.insert(config.token.clone(), config);
        Ok(())
    }

    /// Get audio encoder configuration options.
    pub fn get_audio_encoder_configuration_options(&self) -> AudioEncoderConfigurationOptions {
        AudioEncoderConfigurationOptions {
            options: vec![
                crate::onvif::types::media::AudioEncoderConfigurationOption {
                    encoding: crate::onvif::types::media::AudioEncoding::G711,
                    bitrate_list: crate::onvif::types::media::IntList {
                        items: vec![64, 128],
                    },
                    sample_rate_list: crate::onvif::types::media::IntList {
                        items: vec![8000, 16000],
                    },
                },
                crate::onvif::types::media::AudioEncoderConfigurationOption {
                    encoding: crate::onvif::types::media::AudioEncoding::AAC,
                    bitrate_list: crate::onvif::types::media::IntList {
                        items: vec![32, 64, 128],
                    },
                    sample_rate_list: crate::onvif::types::media::IntList {
                        items: vec![8000, 16000, 32000],
                    },
                },
            ],
        }
    }

    // ========================================================================
    // Profile Configuration Operations
    // ========================================================================

    /// Add video source configuration to a profile.
    pub fn add_video_source_configuration(
        &self,
        profile_token: &ReferenceToken,
        config_token: &ReferenceToken,
    ) -> OnvifResult<()> {
        let config = self.get_video_source_configuration(config_token)?;
        let mut profiles = self.profiles.write();

        let profile = profiles
            .get_mut(profile_token)
            .ok_or_else(|| no_profile_error(profile_token))?;

        profile.video_source_configuration = Some(config);
        Ok(())
    }

    /// Remove video source configuration from a profile.
    pub fn remove_video_source_configuration(
        &self,
        profile_token: &ReferenceToken,
    ) -> OnvifResult<()> {
        let mut profiles = self.profiles.write();

        let profile = profiles
            .get_mut(profile_token)
            .ok_or_else(|| no_profile_error(profile_token))?;

        profile.video_source_configuration = None;
        Ok(())
    }

    /// Add video encoder configuration to a profile.
    pub fn add_video_encoder_configuration(
        &self,
        profile_token: &ReferenceToken,
        config_token: &ReferenceToken,
    ) -> OnvifResult<()> {
        let config = self.get_video_encoder_configuration(config_token)?;
        let mut profiles = self.profiles.write();

        let profile = profiles
            .get_mut(profile_token)
            .ok_or_else(|| no_profile_error(profile_token))?;

        profile.video_encoder_configuration = Some(config);
        Ok(())
    }

    /// Remove video encoder configuration from a profile.
    pub fn remove_video_encoder_configuration(
        &self,
        profile_token: &ReferenceToken,
    ) -> OnvifResult<()> {
        let mut profiles = self.profiles.write();

        let profile = profiles
            .get_mut(profile_token)
            .ok_or_else(|| no_profile_error(profile_token))?;

        profile.video_encoder_configuration = None;
        Ok(())
    }

    /// Add audio source configuration to a profile.
    pub fn add_audio_source_configuration(
        &self,
        profile_token: &ReferenceToken,
        config_token: &ReferenceToken,
    ) -> OnvifResult<()> {
        let config = self.get_audio_source_configuration(config_token)?;
        let mut profiles = self.profiles.write();

        let profile = profiles
            .get_mut(profile_token)
            .ok_or_else(|| no_profile_error(profile_token))?;

        profile.audio_source_configuration = Some(config);
        Ok(())
    }

    /// Remove audio source configuration from a profile.
    pub fn remove_audio_source_configuration(
        &self,
        profile_token: &ReferenceToken,
    ) -> OnvifResult<()> {
        let mut profiles = self.profiles.write();

        let profile = profiles
            .get_mut(profile_token)
            .ok_or_else(|| no_profile_error(profile_token))?;

        profile.audio_source_configuration = None;
        Ok(())
    }

    /// Add audio encoder configuration to a profile.
    pub fn add_audio_encoder_configuration(
        &self,
        profile_token: &ReferenceToken,
        config_token: &ReferenceToken,
    ) -> OnvifResult<()> {
        let config = self.get_audio_encoder_configuration(config_token)?;
        let mut profiles = self.profiles.write();

        let profile = profiles
            .get_mut(profile_token)
            .ok_or_else(|| no_profile_error(profile_token))?;

        profile.audio_encoder_configuration = Some(config);
        Ok(())
    }

    /// Remove audio encoder configuration from a profile.
    pub fn remove_audio_encoder_configuration(
        &self,
        profile_token: &ReferenceToken,
    ) -> OnvifResult<()> {
        let mut profiles = self.profiles.write();

        let profile = profiles
            .get_mut(profile_token)
            .ok_or_else(|| no_profile_error(profile_token))?;

        profile.audio_encoder_configuration = None;
        Ok(())
    }

    // ========================================================================
    // Compatible Configurations
    // ========================================================================

    /// Get compatible video source configurations for a profile.
    pub fn get_compatible_video_source_configurations(
        &self,
        _profile_token: &ReferenceToken,
    ) -> Vec<VideoSourceConfiguration> {
        // All video source configurations are compatible
        self.get_video_source_configurations()
    }

    /// Get compatible video encoder configurations for a profile.
    pub fn get_compatible_video_encoder_configurations(
        &self,
        _profile_token: &ReferenceToken,
    ) -> Vec<VideoEncoderConfiguration> {
        // All video encoder configurations are compatible
        self.get_video_encoder_configurations()
    }

    /// Get compatible audio source configurations for a profile.
    pub fn get_compatible_audio_source_configurations(
        &self,
        _profile_token: &ReferenceToken,
    ) -> Vec<AudioSourceConfiguration> {
        // All audio source configurations are compatible
        self.get_audio_source_configurations()
    }

    /// Get compatible audio encoder configurations for a profile.
    pub fn get_compatible_audio_encoder_configurations(
        &self,
        _profile_token: &ReferenceToken,
    ) -> Vec<AudioEncoderConfiguration> {
        // All audio encoder configurations are compatible
        self.get_audio_encoder_configurations()
    }

    /// Create default PTZ configuration for profiles.
    ///
    /// This provides a standard PTZ configuration that ODM and other clients expect
    /// to see in profiles for PTZ-capable devices.
    fn create_default_ptz_configuration() -> PTZConfiguration {
        PTZConfiguration {
            token: format!("{}0", PTZ_CONFIG_PREFIX),
            name: "DefaultPTZConfig".to_string(),
            use_count: 2, // Used by both MainStream and SubStream
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
}

impl Default for ProfileManager {
    fn default() -> Self {
        Self::new()
    }
}

// Thread-safe marker
unsafe impl Send for ProfileManager {}
unsafe impl Sync for ProfileManager {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_manager_new() {
        let manager = ProfileManager::new();
        let profiles = manager.get_profiles();
        assert_eq!(profiles.len(), 2);
    }

    #[test]
    fn test_get_profile_existing() {
        let manager = ProfileManager::new();
        let result = manager.get_profile(&format!("{}MainStream", PROFILE_TOKEN_PREFIX));
        assert!(result.is_ok());
        let profile = result.unwrap();
        assert_eq!(profile.name, "MainStream");
    }

    #[test]
    fn test_get_profile_not_found() {
        let manager = ProfileManager::new();
        let result = manager.get_profile(&"NonExistent".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_create_profile() {
        let manager = ProfileManager::new();
        let result = manager.create_profile("TestProfile".to_string(), None);
        assert!(result.is_ok());
        let profile = result.unwrap();
        assert_eq!(profile.name, "TestProfile");
        assert!(profile.token.starts_with(PROFILE_TOKEN_PREFIX));
    }

    #[test]
    fn test_create_profile_with_token() {
        let manager = ProfileManager::new();
        let result =
            manager.create_profile("TestProfile".to_string(), Some("CustomToken".to_string()));
        assert!(result.is_ok());
        let profile = result.unwrap();
        assert_eq!(profile.token, "CustomToken");
    }

    #[test]
    fn test_create_profile_duplicate_token() {
        let manager = ProfileManager::new();
        let _ = manager.create_profile("Test1".to_string(), Some("Token1".to_string()));
        let result = manager.create_profile("Test2".to_string(), Some("Token1".to_string()));
        assert!(result.is_err());
    }

    #[test]
    fn test_delete_profile() {
        let manager = ProfileManager::new();
        let profile = manager
            .create_profile("TestProfile".to_string(), None)
            .unwrap();
        let result = manager.delete_profile(&profile.token);
        assert!(result.is_ok());
    }

    #[test]
    fn test_delete_fixed_profile() {
        let manager = ProfileManager::new();
        let result = manager.delete_profile(&format!("{}MainStream", PROFILE_TOKEN_PREFIX));
        assert!(result.is_err());
    }

    #[test]
    fn test_get_video_sources() {
        let manager = ProfileManager::new();
        let sources = manager.get_video_sources();
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].token, DEFAULT_VIDEO_SOURCE_TOKEN);
    }

    #[test]
    fn test_get_audio_sources() {
        let manager = ProfileManager::new();
        let sources = manager.get_audio_sources();
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].token, DEFAULT_AUDIO_SOURCE_TOKEN);
    }

    #[test]
    fn test_get_video_encoder_configurations() {
        let manager = ProfileManager::new();
        let configs = manager.get_video_encoder_configurations();
        assert_eq!(configs.len(), 2);
    }

    #[test]
    fn test_add_video_encoder_configuration() {
        let manager = ProfileManager::new();
        let profile = manager
            .create_profile("TestProfile".to_string(), None)
            .unwrap();
        let config_token = format!("{}0", VIDEO_ENCODER_CONFIG_PREFIX);
        let result = manager.add_video_encoder_configuration(&profile.token, &config_token);
        assert!(result.is_ok());

        let updated_profile = manager.get_profile(&profile.token).unwrap();
        assert!(updated_profile.video_encoder_configuration.is_some());
    }
}
