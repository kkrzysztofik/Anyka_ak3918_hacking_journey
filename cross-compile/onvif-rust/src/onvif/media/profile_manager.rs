//! Media Profile Manager.
//!
//! This module provides thread-safe management of media profiles including:
//! - Profile storage and retrieval
//! - Profile creation and deletion
//! - Profile token validation
//! - Video/audio source and encoder configuration management

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use parking_lot::RwLock;

use crate::config::{ApplicationConfig, ConfigError, ConfigPersistenceHandle, ConfigRuntime};
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
    /// Runtime configuration (optional) used for persistence.
    config: Option<Arc<ConfigRuntime>>,
    /// Config persistence handle for debounced saves.
    persistence: Option<ConfigPersistenceHandle>,
}

impl ProfileManager {
    /// Create a new ProfileManager with default profiles.
    pub fn new() -> Self {
        let manager = Self::new_with_dependencies(None, None);
        manager.initialize_defaults();
        manager
    }

    /// Create a ProfileManager that persists to the given configuration runtime.
    pub fn with_config(config: Arc<ConfigRuntime>) -> Self {
        Self::with_config_and_persistence(config, None)
    }

    /// Create a ProfileManager that persists to the given configuration runtime and save handle.
    pub fn with_config_and_persistence(
        config: Arc<ConfigRuntime>,
        persistence: Option<ConfigPersistenceHandle>,
    ) -> Self {
        let manager = Self::new_with_dependencies(Some(Arc::clone(&config)), persistence);

        if !manager.load_from_config() {
            manager.initialize_defaults();
            manager.persist_all();
        }

        manager
    }

    /// Internal constructor used by public builders.
    fn new_with_dependencies(
        config: Option<Arc<ConfigRuntime>>,
        persistence: Option<ConfigPersistenceHandle>,
    ) -> Self {
        Self {
            profiles: RwLock::new(HashMap::new()),
            video_sources: RwLock::new(HashMap::new()),
            audio_sources: RwLock::new(HashMap::new()),
            video_source_configs: RwLock::new(HashMap::new()),
            video_encoder_configs: RwLock::new(HashMap::new()),
            audio_source_configs: RwLock::new(HashMap::new()),
            audio_encoder_configs: RwLock::new(HashMap::new()),
            profile_counter: AtomicU32::new(0),
            config,
            persistence,
        }
    }

    /// Initialize default sources, configurations, and profiles.
    ///
    /// If a ConfigRuntime is available, reads profile settings from stream_profile_1..4 sections.
    /// Otherwise, falls back to hardcoded defaults.
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

        // Initialize profiles from configuration if explicit stream profiles exist; otherwise use hardcoded defaults.
        if let Some(ref config) = self.config {
            let snapshot = config.snapshot();
            let has_stream_profiles = snapshot
                .keys()
                .any(|key| key.starts_with("stream_profile_"));

            if has_stream_profiles {
                self.initialize_profiles_from_config(config, default_ptz_config);
            } else {
                self.initialize_profiles_hardcoded(default_ptz_config);
            }
        } else {
            self.initialize_profiles_hardcoded(default_ptz_config);
        }
    }

    /// Initialize profiles from config sections (stream_profile_1..4).
    fn initialize_profiles_from_config(
        &self,
        config: &Arc<ConfigRuntime>,
        default_ptz_config: PTZConfiguration,
    ) {
        let mut profile_count = 0;

        // Iterate through stream_profile_1 to stream_profile_4
        for profile_num in 1..=4 {
            let prefix = format!("stream_profile_{}", profile_num);

            // Read video configuration
            // Default to enabled=true for better UX (profiles load unless explicitly disabled)
            let enabled = config
                .get_bool(&format!("{}.enabled", prefix))
                .unwrap_or(true);
            if !enabled {
                continue;
            }

            let name = config
                .get_string(&format!("{}.name", prefix))
                .unwrap_or_else(|_| format!("Stream{}", profile_num));
            let width = config.get_int(&format!("{}.width", prefix)).unwrap_or(1920) as u32;
            let height = config
                .get_int(&format!("{}.height", prefix))
                .unwrap_or(1080) as u32;
            let framerate = config
                .get_int(&format!("{}.framerate", prefix))
                .unwrap_or(30) as u32;
            let bitrate = config
                .get_int(&format!("{}.bitrate", prefix))
                .unwrap_or(4000) as u32;
            let encoding_str = config
                .get_string(&format!("{}.encoding", prefix))
                .unwrap_or_else(|_| "h264".to_string());
            let profile_str = config
                .get_string(&format!("{}.profile", prefix))
                .unwrap_or_else(|_| "main".to_string());

            // Read audio configuration
            let audio_enabled = config
                .get_bool(&format!("{}.audio_enabled", prefix))
                .unwrap_or(false);
            let audio_encoding_str = config
                .get_string(&format!("{}.audio_encoding", prefix))
                .unwrap_or_else(|_| "g711".to_string());
            let audio_bitrate = config
                .get_int(&format!("{}.audio_bitrate", prefix))
                .unwrap_or(64) as u32;
            let audio_sample_rate = config
                .get_int(&format!("{}.audio_sample_rate", prefix))
                .unwrap_or(8000) as u32;

            // Parse video encoding
            let video_encoding = match encoding_str.to_lowercase().as_str() {
                "h264" => crate::onvif::types::common::VideoEncoding::H264,
                "mjpeg" | "jpeg" => crate::onvif::types::common::VideoEncoding::JPEG,
                "mpeg4" => crate::onvif::types::common::VideoEncoding::MPEG4,
                _ => {
                    eprintln!(
                        "[WARN] Unknown video encoding '{}', defaulting to H264",
                        encoding_str
                    );
                    crate::onvif::types::common::VideoEncoding::H264
                }
            };

            // Parse H.264 profile
            let h264_profile = match profile_str.to_lowercase().as_str() {
                "baseline" => crate::onvif::types::common::H264Profile::Baseline,
                "main" => crate::onvif::types::common::H264Profile::Main,
                "high" => crate::onvif::types::common::H264Profile::High,
                _ => {
                    eprintln!(
                        "[WARN] Unknown H.264 profile '{}', defaulting to Main",
                        profile_str
                    );
                    crate::onvif::types::common::H264Profile::Main
                }
            };

            // Parse audio encoding
            let audio_encoding = match audio_encoding_str.to_lowercase().as_str() {
                "g711" => crate::onvif::types::common::AudioEncoding::G711,
                "aac" => crate::onvif::types::common::AudioEncoding::AAC,
                "g726" => crate::onvif::types::common::AudioEncoding::G726,
                _ => {
                    eprintln!(
                        "[WARN] Unknown audio encoding '{}', defaulting to G711",
                        audio_encoding_str
                    );
                    crate::onvif::types::common::AudioEncoding::G711
                }
            };

            // Create video encoder configuration
            let video_encoder_token = format!("{}{}", VIDEO_ENCODER_CONFIG_PREFIX, profile_count);
            let video_encoder_config = VideoEncoderConfiguration {
                token: video_encoder_token.clone(),
                name: name.clone(),
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
            };
            self.video_encoder_configs
                .write()
                .insert(video_encoder_token.clone(), video_encoder_config.clone());

            // Create audio encoder configuration if audio is enabled
            let audio_encoder_config = if audio_enabled {
                let audio_encoder_token =
                    format!("{}{}", AUDIO_ENCODER_CONFIG_PREFIX, profile_count);
                let config = AudioEncoderConfiguration {
                    token: audio_encoder_token.clone(),
                    name: format!("AudioEncoderConfig_{}", profile_count),
                    use_count: 1,
                    encoding: audio_encoding,
                    bitrate: audio_bitrate as i32,
                    sample_rate: audio_sample_rate as i32,
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
                self.audio_encoder_configs
                    .write()
                    .insert(audio_encoder_token.clone(), config.clone());
                Some(config)
            } else {
                None
            };

            // Create profile
            let profile_token = format!("{}{}", PROFILE_TOKEN_PREFIX, name);
            let profile = Profile {
                token: profile_token.clone(),
                fixed: Some(true),
                name: name.clone(),
                video_source_configuration: Some(VideoSourceConfiguration {
                    token: format!("{}0", VIDEO_SOURCE_CONFIG_PREFIX),
                    source_token: DEFAULT_VIDEO_SOURCE_TOKEN.to_string(),
                    name: "VideoSourceConfig_0".to_string(),
                    use_count: profile_count + 1,
                    view_mode: None,
                    bounds: IntRectangle {
                        x: 0,
                        y: 0,
                        width: width as i32,
                        height: height as i32,
                    },
                    extension: None,
                }),
                audio_source_configuration: if audio_enabled {
                    Some(AudioSourceConfiguration {
                        token: format!("{}0", AUDIO_SOURCE_CONFIG_PREFIX),
                        source_token: DEFAULT_AUDIO_SOURCE_TOKEN.to_string(),
                        name: "AudioSourceConfig_0".to_string(),
                        use_count: profile_count + 1,
                    })
                } else {
                    None
                },
                video_encoder_configuration: Some(video_encoder_config),
                audio_encoder_configuration: audio_encoder_config,
                ptz_configuration: Some(default_ptz_config.clone()),
                metadata_configuration: None,
                extension: None,
            };

            self.profiles.write().insert(profile_token, profile);
            profile_count += 1;
        }

        self.profile_counter
            .store(profile_count as u32, Ordering::SeqCst);
    }

    /// Initialize hardcoded profiles (fallback when no config is available).
    fn initialize_profiles_hardcoded(&self, default_ptz_config: PTZConfiguration) {
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
            video_encoder_config_main.clone(),
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
            video_encoder_config_sub.clone(),
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
            audio_encoder_config.clone(),
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
            video_encoder_configuration: Some(video_encoder_config_main),
            audio_encoder_configuration: Some(audio_encoder_config.clone()),
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
            video_encoder_configuration: Some(video_encoder_config_sub),
            audio_encoder_configuration: Some(audio_encoder_config),
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
                format!("Maximum number of profiles ({}) reached", MAX_PROFILES),
            ));
        }

        // Generate or validate token
        let profile_token = if let Some(t) = token {
            validate_profile_token(&t)?;
            if profiles.contains_key(&t) {
                return Err(OnvifError::invalid_arg_val(
                    "ter:TokenConflict",
                    format!("Profile with token '{}' already exists", t),
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

        {
            profiles.insert(profile_token, profile.clone());
        }

        drop(profiles);
        self.persist_all();
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

        drop(profiles);
        self.persist_all();
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
        {
            let mut configs = self.video_source_configs.write();
            configs.insert(config.token.clone(), config);
        }

        self.persist_all();
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
        {
            let mut configs = self.video_encoder_configs.write();
            configs.insert(config.token.clone(), config);
        }

        self.persist_all();
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
        {
            let mut configs = self.audio_source_configs.write();
            configs.insert(config.token.clone(), config);
        }

        self.persist_all();
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
        {
            let mut configs = self.audio_encoder_configs.write();
            configs.insert(config.token.clone(), config);
        }

        self.persist_all();
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
        {
            let mut profiles = self.profiles.write();

            let profile = profiles
                .get_mut(profile_token)
                .ok_or_else(|| no_profile_error(profile_token))?;

            profile.video_source_configuration = Some(config);
        }

        self.persist_all();
        Ok(())
    }

    /// Remove video source configuration from a profile.
    pub fn remove_video_source_configuration(
        &self,
        profile_token: &ReferenceToken,
    ) -> OnvifResult<()> {
        {
            let mut profiles = self.profiles.write();

            let profile = profiles
                .get_mut(profile_token)
                .ok_or_else(|| no_profile_error(profile_token))?;

            profile.video_source_configuration = None;
        }

        self.persist_all();
        Ok(())
    }

    /// Add video encoder configuration to a profile.
    pub fn add_video_encoder_configuration(
        &self,
        profile_token: &ReferenceToken,
        config_token: &ReferenceToken,
    ) -> OnvifResult<()> {
        let config = self.get_video_encoder_configuration(config_token)?;
        {
            let mut profiles = self.profiles.write();

            let profile = profiles
                .get_mut(profile_token)
                .ok_or_else(|| no_profile_error(profile_token))?;

            profile.video_encoder_configuration = Some(config);
        }

        self.persist_all();
        Ok(())
    }

    /// Remove video encoder configuration from a profile.
    pub fn remove_video_encoder_configuration(
        &self,
        profile_token: &ReferenceToken,
    ) -> OnvifResult<()> {
        {
            let mut profiles = self.profiles.write();

            let profile = profiles
                .get_mut(profile_token)
                .ok_or_else(|| no_profile_error(profile_token))?;

            profile.video_encoder_configuration = None;
        }

        self.persist_all();
        Ok(())
    }

    /// Add audio source configuration to a profile.
    pub fn add_audio_source_configuration(
        &self,
        profile_token: &ReferenceToken,
        config_token: &ReferenceToken,
    ) -> OnvifResult<()> {
        let config = self.get_audio_source_configuration(config_token)?;
        {
            let mut profiles = self.profiles.write();

            let profile = profiles
                .get_mut(profile_token)
                .ok_or_else(|| no_profile_error(profile_token))?;

            profile.audio_source_configuration = Some(config);
        }

        self.persist_all();
        Ok(())
    }

    /// Remove audio source configuration from a profile.
    pub fn remove_audio_source_configuration(
        &self,
        profile_token: &ReferenceToken,
    ) -> OnvifResult<()> {
        {
            let mut profiles = self.profiles.write();

            let profile = profiles
                .get_mut(profile_token)
                .ok_or_else(|| no_profile_error(profile_token))?;

            profile.audio_source_configuration = None;
        }

        self.persist_all();
        Ok(())
    }

    /// Add audio encoder configuration to a profile.
    pub fn add_audio_encoder_configuration(
        &self,
        profile_token: &ReferenceToken,
        config_token: &ReferenceToken,
    ) -> OnvifResult<()> {
        let config = self.get_audio_encoder_configuration(config_token)?;
        {
            let mut profiles = self.profiles.write();

            let profile = profiles
                .get_mut(profile_token)
                .ok_or_else(|| no_profile_error(profile_token))?;

            profile.audio_encoder_configuration = Some(config);
        }

        self.persist_all();
        Ok(())
    }

    /// Remove audio encoder configuration from a profile.
    pub fn remove_audio_encoder_configuration(
        &self,
        profile_token: &ReferenceToken,
    ) -> OnvifResult<()> {
        {
            let mut profiles = self.profiles.write();

            let profile = profiles
                .get_mut(profile_token)
                .ok_or_else(|| no_profile_error(profile_token))?;

            profile.audio_encoder_configuration = None;
        }

        self.persist_all();
        Ok(())
    }

    /// Persist the current profile state to configuration storage if available.
    fn persist_all(&self) {
        let Some(config) = &self.config else {
            return;
        };

        if let Err(err) = self.persist_to_config(config) {
            tracing::warn!("Failed to persist media profiles: {err}");
        } else if let Some(handle) = &self.persistence {
            handle.request_save();
        }
    }

    fn persist_to_config(&self, config: &ConfigRuntime) -> Result<(), ConfigError> {
        // Snapshot all media state while holding read locks, then release before writing config
        let (
            profiles_data,
            video_sources_data,
            audio_sources_data,
            video_source_configs_data,
            video_encoder_configs_data,
            audio_source_configs_data,
            audio_encoder_configs_data,
        ) = {
            let profiles = self.profiles.read();
            let video_sources = self.video_sources.read();
            let audio_sources = self.audio_sources.read();
            let video_source_configs = self.video_source_configs.read();
            let video_encoder_configs = self.video_encoder_configs.read();
            let audio_source_configs = self.audio_source_configs.read();
            let audio_encoder_configs = self.audio_encoder_configs.read();

            (
                profiles
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect::<Vec<_>>(),
                video_sources
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect::<Vec<_>>(),
                audio_sources
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect::<Vec<_>>(),
                video_source_configs
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect::<Vec<_>>(),
                video_encoder_configs
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect::<Vec<_>>(),
                audio_source_configs
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect::<Vec<_>>(),
                audio_encoder_configs
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect::<Vec<_>>(),
            )
        };

        // Persist profile list
        let mut tokens: Vec<String> = profiles_data.iter().map(|(k, _)| k.clone()).collect();
        tokens.sort();
        config.set_string("media.profiles", &tokens.join(","))?;

        // Persist sources
        for (token, source) in video_sources_data.iter() {
            let prefix = format!("media.video_source.{token}");
            config.set_float(
                &format!("{}.framerate", prefix),
                f64::from(source.framerate),
            )?;
            config.set_int(&format!("{}.width", prefix), source.resolution.width as i64)?;
            config.set_int(
                &format!("{}.height", prefix),
                source.resolution.height as i64,
            )?;
        }

        for (token, source) in audio_sources_data.iter() {
            let prefix = format!("media.audio_source.{token}");
            config.set_int(&format!("{}.channels", prefix), source.channels as i64)?;
        }

        // Persist configurations
        for (token, cfg) in video_source_configs_data.iter() {
            let prefix = format!("media.video_source_config.{token}");
            config.set_string(&format!("{}.source_token", prefix), &cfg.source_token)?;
            config.set_string(&format!("{}.name", prefix), &cfg.name)?;
            config.set_int(&format!("{}.use_count", prefix), cfg.use_count as i64)?;
            config.set_int(&format!("{}.x", prefix), cfg.bounds.x as i64)?;
            config.set_int(&format!("{}.y", prefix), cfg.bounds.y as i64)?;
            config.set_int(&format!("{}.width", prefix), cfg.bounds.width as i64)?;
            config.set_int(&format!("{}.height", prefix), cfg.bounds.height as i64)?;
        }

        for (token, cfg) in video_encoder_configs_data.iter() {
            let prefix = format!("media.video_encoder_config.{token}");
            config.set_string(&format!("{}.name", prefix), &cfg.name)?;
            config.set_string(
                &format!("{}.encoding", prefix),
                match cfg.encoding {
                    crate::onvif::types::common::VideoEncoding::H264 => "H264",
                    crate::onvif::types::common::VideoEncoding::JPEG => "MJPEG",
                    crate::onvif::types::common::VideoEncoding::MPEG4 => "MPEG4",
                },
            )?;
            config.set_int(&format!("{}.width", prefix), cfg.resolution.width as i64)?;
            config.set_int(&format!("{}.height", prefix), cfg.resolution.height as i64)?;
            config.set_float(&format!("{}.quality", prefix), f64::from(cfg.quality))?;

            if let Some(rc) = &cfg.rate_control {
                config.set_int(
                    &format!("{}.frame_rate_limit", prefix),
                    rc.frame_rate_limit as i64,
                )?;
                config.set_int(
                    &format!("{}.encoding_interval", prefix),
                    rc.encoding_interval as i64,
                )?;
                config.set_int(
                    &format!("{}.bitrate_limit", prefix),
                    rc.bitrate_limit as i64,
                )?;
            }

            if let Some(h264) = &cfg.h264 {
                config.set_int(&format!("{}.gov_length", prefix), h264.gov_length as i64)?;
                config.set_string(
                    &format!("{}.h264_profile", prefix),
                    match h264.h264_profile {
                        crate::onvif::types::common::H264Profile::Baseline => "Baseline",
                        crate::onvif::types::common::H264Profile::Main => "Main",
                        crate::onvif::types::common::H264Profile::Extended => "Extended",
                        crate::onvif::types::common::H264Profile::High => "High",
                    },
                )?;
            }

            config.set_string(&format!("{}.session_timeout", prefix), &cfg.session_timeout)?;
        }

        for (token, cfg) in audio_source_configs_data.iter() {
            let prefix = format!("media.audio_source_config.{token}");
            config.set_string(&format!("{}.source_token", prefix), &cfg.source_token)?;
            config.set_string(&format!("{}.name", prefix), &cfg.name)?;
            config.set_int(&format!("{}.use_count", prefix), cfg.use_count as i64)?;
        }

        for (token, cfg) in audio_encoder_configs_data.iter() {
            let prefix = format!("media.audio_encoder_config.{token}");
            config.set_string(&format!("{}.name", prefix), &cfg.name)?;
            config.set_string(
                &format!("{}.encoding", prefix),
                match cfg.encoding {
                    crate::onvif::types::common::AudioEncoding::G711 => "G711",
                    crate::onvif::types::common::AudioEncoding::G726 => "G726",
                    crate::onvif::types::common::AudioEncoding::AAC => "AAC",
                },
            )?;
            config.set_int(&format!("{}.bitrate", prefix), cfg.bitrate as i64)?;
            config.set_int(&format!("{}.sample_rate", prefix), cfg.sample_rate as i64)?;
            config.set_string(&format!("{}.session_timeout", prefix), &cfg.session_timeout)?;
        }

        // Persist profiles
        for (token, profile) in profiles_data.iter() {
            let prefix = format!("media.profile.{token}");
            config.set_string(&format!("{}.name", prefix), &profile.name)?;
            config.set_bool(&format!("{}.fixed", prefix), profile.fixed.unwrap_or(false))?;

            if let Some(vs) = &profile.video_source_configuration {
                config.set_string(&format!("{}.video_source_config", prefix), &vs.token)?;
            }
            if let Some(ve) = &profile.video_encoder_configuration {
                config.set_string(&format!("{}.video_encoder_config", prefix), &ve.token)?;
            }
            if let Some(asrc) = &profile.audio_source_configuration {
                config.set_string(&format!("{}.audio_source_config", prefix), &asrc.token)?;
            }
            if let Some(aenc) = &profile.audio_encoder_configuration {
                config.set_string(&format!("{}.audio_encoder_config", prefix), &aenc.token)?;
            }
            if let Some(ptz) = &profile.ptz_configuration {
                config.set_string(&format!("{}.ptz_config", prefix), &ptz.token)?;
            }
        }

        Ok(())
    }

    /// Attempt to load profiles from configuration. Returns `true` on success.
    fn load_from_config(&self) -> bool {
        let Some(config) = &self.config else {
            return false;
        };

        let snapshot = config.snapshot();
        let Some(tokens_raw) = snapshot.get("media.profiles") else {
            return false;
        };

        let tokens: Vec<String> = tokens_raw
            .split(',')
            .filter(|t| !t.trim().is_empty())
            .map(|t| t.trim().to_string())
            .collect();

        if tokens.is_empty() {
            return false;
        }

        // Clear existing state before loading
        let mut profiles_guard = self.profiles.write();
        let mut video_sources = self.video_sources.write();
        let mut audio_sources = self.audio_sources.write();
        let mut video_source_configs = self.video_source_configs.write();
        let mut video_encoder_configs = self.video_encoder_configs.write();
        let mut audio_source_configs = self.audio_source_configs.write();
        let mut audio_encoder_configs = self.audio_encoder_configs.write();

        profiles_guard.clear();
        video_sources.clear();
        audio_sources.clear();
        video_source_configs.clear();
        video_encoder_configs.clear();
        audio_source_configs.clear();
        audio_encoder_configs.clear();

        for token in tokens.iter() {
            if let Some(profile) = self.load_profile(&snapshot, token) {
                if let Some(ref cfg) = profile.video_source_configuration {
                    video_source_configs.insert(cfg.token.clone(), cfg.clone());
                    if let Some(source) = self.load_video_source(&snapshot, &cfg.source_token) {
                        video_sources.insert(cfg.source_token.clone(), source);
                    }
                }

                if let Some(ref cfg) = profile.audio_source_configuration {
                    audio_source_configs.insert(cfg.token.clone(), cfg.clone());
                    if let Some(source) = self.load_audio_source(&snapshot, &cfg.source_token) {
                        audio_sources.insert(cfg.source_token.clone(), source);
                    }
                }

                if let Some(ref cfg) = profile.video_encoder_configuration {
                    video_encoder_configs.insert(cfg.token.clone(), cfg.clone());
                }

                if let Some(ref cfg) = profile.audio_encoder_configuration {
                    audio_encoder_configs.insert(cfg.token.clone(), cfg.clone());
                }

                profiles_guard.insert(token.clone(), profile);
            } else {
                tracing::warn!("Skipping profile '{token}' due to missing fields");
            }
        }

        // Set counter for new tokens
        self.profile_counter
            .store(profiles_guard.len() as u32, Ordering::SeqCst);

        !profiles_guard.is_empty()
    }

    fn load_profile(&self, snapshot: &ApplicationConfig, token: &str) -> Option<Profile> {
        let prefix = format!("media.profile.{token}");
        let name = snapshot.get(&format!("{prefix}.name"))?.clone();
        let fixed = snapshot
            .get(&format!("{prefix}.fixed"))
            .and_then(|s| s.parse::<bool>().ok())
            .unwrap_or(false);

        let video_source_config_token = snapshot.get(&format!("{prefix}.video_source_config"));
        let video_encoder_config_token = snapshot.get(&format!("{prefix}.video_encoder_config"));
        let audio_source_config_token = snapshot.get(&format!("{prefix}.audio_source_config"));
        let audio_encoder_config_token = snapshot.get(&format!("{prefix}.audio_encoder_config"));
        let ptz_config_token = snapshot.get(&format!("{prefix}.ptz_config"));

        let video_source_configuration =
            video_source_config_token.and_then(|t| self.load_video_source_config(snapshot, t));
        let video_encoder_configuration =
            video_encoder_config_token.and_then(|t| self.load_video_encoder_config(snapshot, t));
        let audio_source_configuration =
            audio_source_config_token.and_then(|t| self.load_audio_source_config(snapshot, t));
        let audio_encoder_configuration =
            audio_encoder_config_token.and_then(|t| self.load_audio_encoder_config(snapshot, t));

        let ptz_configuration = ptz_config_token.map(|_| Self::create_default_ptz_configuration());

        Some(Profile {
            token: token.to_string(),
            fixed: Some(fixed),
            name,
            video_source_configuration,
            audio_source_configuration,
            video_encoder_configuration,
            audio_encoder_configuration,
            ptz_configuration,
            metadata_configuration: None,
            extension: None,
        })
    }

    fn load_video_source_config(
        &self,
        snapshot: &ApplicationConfig,
        token: &str,
    ) -> Option<VideoSourceConfiguration> {
        let prefix = format!("media.video_source_config.{token}");
        Some(VideoSourceConfiguration {
            token: token.to_string(),
            source_token: snapshot.get(&format!("{prefix}.source_token"))?.clone(),
            name: snapshot.get(&format!("{prefix}.name"))?.clone(),
            use_count: snapshot.get(&format!("{prefix}.use_count"))?.parse().ok()?,
            view_mode: None,
            bounds: IntRectangle {
                x: snapshot.get(&format!("{prefix}.x"))?.parse().ok()?,
                y: snapshot.get(&format!("{prefix}.y"))?.parse().ok()?,
                width: snapshot.get(&format!("{prefix}.width"))?.parse().ok()?,
                height: snapshot.get(&format!("{prefix}.height"))?.parse().ok()?,
            },
            extension: None,
        })
    }

    fn load_video_encoder_config(
        &self,
        snapshot: &ApplicationConfig,
        token: &str,
    ) -> Option<VideoEncoderConfiguration> {
        let prefix = format!("media.video_encoder_config.{token}");

        let encoding_str = snapshot.get(&format!("{prefix}.encoding"))?;
        let encoding = match encoding_str.as_str() {
            "H264" | "H265" => crate::onvif::types::common::VideoEncoding::H264,
            "MJPEG" => crate::onvif::types::common::VideoEncoding::JPEG,
            "MPEG4" => crate::onvif::types::common::VideoEncoding::MPEG4,
            _ => return None,
        };

        let h264_profile = snapshot
            .get(&format!("{prefix}.h264_profile"))
            .and_then(|p| match p.as_str() {
                "Baseline" => Some(crate::onvif::types::common::H264Profile::Baseline),
                "Main" => Some(crate::onvif::types::common::H264Profile::Main),
                "Extended" => Some(crate::onvif::types::common::H264Profile::Extended),
                "High" => Some(crate::onvif::types::common::H264Profile::High),
                _ => None,
            });

        let rate_control = match (
            snapshot.get(&format!("{prefix}.frame_rate_limit")),
            snapshot.get(&format!("{prefix}.encoding_interval")),
            snapshot.get(&format!("{prefix}.bitrate_limit")),
        ) {
            (Some(fr), Some(interval), Some(br)) => Some(VideoRateControl {
                frame_rate_limit: fr.parse().ok()?,
                encoding_interval: interval.parse().ok()?,
                bitrate_limit: br.parse().ok()?,
            }),
            _ => None,
        };

        Some(VideoEncoderConfiguration {
            token: token.to_string(),
            name: snapshot.get(&format!("{prefix}.name"))?.clone(),
            use_count: 1,
            encoding,
            resolution: VideoResolution {
                width: snapshot.get(&format!("{prefix}.width"))?.parse().ok()?,
                height: snapshot.get(&format!("{prefix}.height"))?.parse().ok()?,
            },
            quality: snapshot
                .get(&format!("{prefix}.quality"))?
                .parse()
                .unwrap_or(0.5),
            rate_control,
            mpeg4: None,
            h264: h264_profile.map(|profile| crate::onvif::types::common::H264Configuration {
                gov_length: snapshot
                    .get(&format!("{prefix}.gov_length"))
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(30),
                h264_profile: profile,
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
            session_timeout: snapshot
                .get(&format!("{prefix}.session_timeout"))
                .cloned()
                .unwrap_or_else(|| "PT60S".to_string()),
        })
    }

    fn load_audio_source_config(
        &self,
        snapshot: &ApplicationConfig,
        token: &str,
    ) -> Option<AudioSourceConfiguration> {
        let prefix = format!("media.audio_source_config.{token}");
        Some(AudioSourceConfiguration {
            token: token.to_string(),
            source_token: snapshot.get(&format!("{prefix}.source_token"))?.clone(),
            name: snapshot.get(&format!("{prefix}.name"))?.clone(),
            use_count: snapshot.get(&format!("{prefix}.use_count"))?.parse().ok()?,
        })
    }

    fn load_audio_encoder_config(
        &self,
        snapshot: &ApplicationConfig,
        token: &str,
    ) -> Option<AudioEncoderConfiguration> {
        let prefix = format!("media.audio_encoder_config.{token}");
        let encoding_str = snapshot.get(&format!("{prefix}.encoding"))?;
        let encoding = match encoding_str.as_str() {
            "G711" | "PCMU" | "PCMA" => crate::onvif::types::common::AudioEncoding::G711,
            "G726" => crate::onvif::types::common::AudioEncoding::G726,
            "AAC" => crate::onvif::types::common::AudioEncoding::AAC,
            _ => return None,
        };

        Some(AudioEncoderConfiguration {
            token: token.to_string(),
            name: snapshot.get(&format!("{prefix}.name"))?.clone(),
            use_count: 1,
            encoding,
            bitrate: snapshot.get(&format!("{prefix}.bitrate"))?.parse().ok()?,
            sample_rate: snapshot
                .get(&format!("{prefix}.sample_rate"))?
                .parse()
                .ok()?,
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
            session_timeout: snapshot
                .get(&format!("{prefix}.session_timeout"))
                .cloned()
                .unwrap_or_else(|| "PT60S".to_string()),
        })
    }

    fn load_video_source(&self, snapshot: &ApplicationConfig, token: &str) -> Option<VideoSource> {
        let prefix = format!("media.video_source.{token}");
        Some(VideoSource {
            token: token.to_string(),
            framerate: snapshot
                .get(&format!("{prefix}.framerate"))
                .and_then(|v| v.parse().ok())
                .unwrap_or(30.0),
            resolution: VideoResolution {
                width: snapshot
                    .get(&format!("{prefix}.width"))
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(1920),
                height: snapshot
                    .get(&format!("{prefix}.height"))
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(1080),
            },
            imaging: None,
            extension: None,
        })
    }

    fn load_audio_source(&self, snapshot: &ApplicationConfig, token: &str) -> Option<AudioSource> {
        let prefix = format!("media.audio_source.{token}");
        Some(AudioSource {
            token: token.to_string(),
            channels: snapshot
                .get(&format!("{prefix}.channels"))
                .and_then(|v| v.parse().ok())
                .unwrap_or(1),
        })
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
    use std::sync::Arc;

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

    #[test]
    fn test_persist_defaults_into_config_runtime() {
        let runtime = Arc::new(ConfigRuntime::new(Default::default()));
        let _manager = ProfileManager::with_config(Arc::clone(&runtime));

        let snapshot = runtime.snapshot();
        assert!(snapshot.get("media.profiles").is_some());
    }

    #[test]
    fn test_load_profiles_from_config_runtime() {
        let runtime = Arc::new(ConfigRuntime::new(Default::default()));
        let manager = ProfileManager::with_config(Arc::clone(&runtime));
        // create new profile and ensure it persists
        let profile = manager
            .create_profile("PersistedProfile".to_string(), None)
            .unwrap();
        let snapshot = runtime.snapshot();
        assert!(
            snapshot
                .get("media.profiles")
                .map(|p| p.contains(&profile.token))
                .unwrap_or(false)
        );

        // New manager should load persisted profile
        let manager_reloaded = ProfileManager::with_config(Arc::clone(&runtime));
        let loaded = manager_reloaded.get_profile(&profile.token).unwrap();
        assert_eq!(loaded.name, "PersistedProfile");
    }
}
