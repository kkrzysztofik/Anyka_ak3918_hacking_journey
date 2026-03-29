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

use crate::config::profiles::{
    ProfilesFile, StoredAudioEncoderConfig, StoredAudioSource, StoredAudioSourceConfig,
    StoredProfile, StoredVideoEncoderConfig, StoredVideoSource, StoredVideoSourceConfig,
};
use crate::config::{ConfigRuntime, ProfileStorage};
use crate::onvif::error::{OnvifError, OnvifResult};
use crate::onvif::types::common::{
    AudioEncoderConfiguration, AudioSource, AudioSourceConfiguration, IntRange, IntRectangle,
    MulticastConfiguration, Name, PTZConfiguration, Profile, ReferenceToken,
    VideoEncoderConfiguration, VideoRateControl, VideoResolution, VideoSource,
    VideoSourceConfiguration,
};
use crate::onvif::types::media::{
    AudioEncoderConfigurationOptions, H264Options, H264Profile, JpegOptions,
    VideoEncoderConfigurationOptions, VideoSourceConfigurationOptions,
};
use crate::platform::Resolution;

use super::defaults::{self, ProfileConfig};
use super::faults::{
    no_config_error, no_profile_error, validate_profile_name, validate_profile_token,
};
use super::types::{
    AUDIO_ENCODER_CONFIG_PREFIX, AUDIO_SOURCE_CONFIG_PREFIX, DEFAULT_AUDIO_SOURCE_TOKEN,
    DEFAULT_VIDEO_SOURCE_TOKEN, MAX_PROFILES, PROFILE_TOKEN_PREFIX, VIDEO_ENCODER_CONFIG_PREFIX,
    VIDEO_SOURCE_CONFIG_PREFIX,
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
    /// Maximum sensor resolution for profile validation.
    max_sensor_resolution: Resolution,
    /// Runtime configuration (optional) for reading `stream_profile_N` at init.
    config: Option<Arc<ConfigRuntime>>,
    /// Typed profile storage for `profiles.toml` persistence.
    profile_storage: Option<Arc<ProfileStorage>>,
}

impl ProfileManager {
    /// Create a new ProfileManager with default profiles and fallback max sensor resolution.
    /// Used primarily for testing; prefer `with_default_resolution()` for test clarity.
    pub fn new() -> Self {
        Self::with_default_resolution()
    }

    /// Create a ProfileManager with default profiles and specified max sensor resolution.
    /// This is the recommended constructor for tests and initialization without platform context.
    pub fn with_default_resolution() -> Self {
        let manager = Self::new_with_dependencies(
            None,
            Resolution::new(1920, 1080), // Fallback for tests without real platform
        );
        manager.initialize_defaults();
        manager
    }

    /// Create a ProfileManager with specified max sensor resolution.
    /// Use this when you have direct knowledge of the sensor resolution.
    pub fn with_max_resolution(max_sensor_resolution: Resolution) -> Self {
        let manager = Self::new_with_dependencies(None, max_sensor_resolution);
        manager.initialize_defaults();
        manager
    }

    /// Create a ProfileManager that reads initial profiles from config.
    pub fn with_config(config: Arc<ConfigRuntime>) -> Self {
        let manager = Self::new_with_dependencies(
            Some(Arc::clone(&config)),
            Resolution::new(1920, 1080), // Fallback for tests
        );

        manager.initialize_defaults();
        manager.persist_all();

        manager
    }

    /// Create a ProfileManager with config and max sensor resolution.
    /// Called from phase 3: MediaService will pass sensor resolution from platform.
    pub fn with_config_and_sensor_resolution(
        config: Arc<ConfigRuntime>,
        max_sensor_resolution: Resolution,
    ) -> Self {
        let manager = Self::new_with_dependencies(Some(Arc::clone(&config)), max_sensor_resolution);

        manager.initialize_defaults();
        manager.persist_all();

        manager
    }

    /// Create a ProfileManager with typed profile storage and sensor resolution.
    ///
    /// Profile data is loaded from `ProfileStorage` first. If empty, profiles
    /// are initialized from `stream_profile_N` in ConfigRuntime (or hardcoded defaults)
    /// and persisted to the storage.
    pub fn with_storage(
        config: Arc<ConfigRuntime>,
        profile_storage: Arc<ProfileStorage>,
        max_sensor_resolution: Resolution,
    ) -> Self {
        let mut manager =
            Self::new_with_dependencies(Some(Arc::clone(&config)), max_sensor_resolution);
        manager.profile_storage = Some(Arc::clone(&profile_storage));

        // Try loading from profile storage first
        if !manager.load_from_storage() {
            manager.initialize_defaults();
            manager.persist_all();
        }

        manager
    }

    /// Internal constructor used by public builders.
    fn new_with_dependencies(
        config: Option<Arc<ConfigRuntime>>,
        max_sensor_resolution: Resolution,
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
            max_sensor_resolution,
            config,
            profile_storage: None,
        }
    }

    /// Initialize default sources, configurations, and profiles.
    ///
    /// If a ConfigRuntime is available, reads profile settings from stream_profile_1..4 sections.
    /// Otherwise, falls back to hardcoded defaults.
    fn initialize_defaults(&self) {
        let default_ptz_config = Self::create_default_ptz_configuration();
        let default_sources = defaults::create_default_sources(self.max_sensor_resolution);
        self.video_sources.write().insert(
            DEFAULT_VIDEO_SOURCE_TOKEN.to_string(),
            default_sources.video_source,
        );
        self.audio_sources.write().insert(
            DEFAULT_AUDIO_SOURCE_TOKEN.to_string(),
            default_sources.audio_source,
        );
        self.video_source_configs.write().insert(
            format!("{}0", VIDEO_SOURCE_CONFIG_PREFIX),
            default_sources.video_source_config,
        );
        self.audio_source_configs.write().insert(
            format!("{}0", AUDIO_SOURCE_CONFIG_PREFIX),
            default_sources.audio_source_config,
        );

        // Initialize profiles from configuration (stream_profile_1..4 always exist with defaults).
        if let Some(ref config) = self.config {
            self.initialize_profiles_from_config(config, default_ptz_config);
        } else {
            self.initialize_profiles_hardcoded(default_ptz_config);
        }
    }

    /// Initialize profiles from config sections (stream_profile_1..4).
    /// Validates each profile against max sensor resolution, skipping profiles that exceed capabilities.
    fn initialize_profiles_from_config(
        &self,
        config: &Arc<ConfigRuntime>,
        default_ptz_config: PTZConfiguration,
    ) {
        let mut profile_count = 0;

        for profile_num in 1..=4 {
            if !Self::is_profile_enabled(config, profile_num) {
                continue;
            }

            let profile_config = Self::read_profile_config(config, profile_num);

            // Validate profile resolution against sensor maximum capabilities
            if profile_config.width > self.max_sensor_resolution.width
                || profile_config.height > self.max_sensor_resolution.height
            {
                tracing::error!(
                    "Skipping stream_profile_{} from config.toml: resolution {}x{} exceeds \
                     sensor maximum {}x{}. Check config.toml stream_profile_{} settings.",
                    profile_num,
                    profile_config.width,
                    profile_config.height,
                    self.max_sensor_resolution.width,
                    self.max_sensor_resolution.height,
                    profile_num
                );
                continue; // Skip this profile entirely
            }

            let video_encoding = Self::parse_video_encoding(&profile_config.encoding_str);
            let h264_profile = Self::parse_h264_profile(&profile_config.profile_str);
            let audio_encoding = Self::parse_audio_encoding(&profile_config.audio_encoding_str);

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
            self.video_encoder_configs.write().insert(
                video_encoder_config.token.clone(),
                video_encoder_config.clone(),
            );

            let audio_encoder_config = if profile_config.audio_enabled {
                let config = Self::create_audio_encoder_config(
                    profile_count,
                    profile_config.audio_bitrate,
                    profile_config.audio_sample_rate,
                    audio_encoding,
                );
                self.audio_encoder_configs
                    .write()
                    .insert(config.token.clone(), config.clone());
                Some(config)
            } else {
                None
            };

            let profile = Self::create_profile_from_config(
                &profile_config.name,
                profile_config.width,
                profile_config.height,
                profile_count,
                profile_config.audio_enabled,
                video_encoder_config,
                audio_encoder_config,
                &default_ptz_config,
            );
            self.profiles.write().insert(profile.token.clone(), profile);
            profile_count += 1;
        }

        self.profile_counter.store(profile_count, Ordering::SeqCst);
    }

    /// Check if a profile is enabled.
    fn is_profile_enabled(config: &ConfigRuntime, profile_num: u32) -> bool {
        defaults::is_profile_enabled(config, profile_num)
    }

    /// Read profile configuration from config.
    fn read_profile_config(config: &ConfigRuntime, profile_num: u32) -> ProfileConfig {
        defaults::read_profile_config(config, profile_num)
    }

    /// Parse video encoding string.
    fn parse_video_encoding(encoding_str: &str) -> crate::onvif::types::common::VideoEncoding {
        defaults::parse_video_encoding(encoding_str, true)
    }

    /// Parse H.264 profile string.
    fn parse_h264_profile(profile_str: &str) -> crate::onvif::types::common::H264Profile {
        defaults::parse_h264_profile(profile_str, true)
    }

    /// Parse audio encoding string.
    fn parse_audio_encoding(encoding_str: &str) -> crate::onvif::types::common::AudioEncoding {
        defaults::parse_audio_encoding(encoding_str, true)
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
        defaults::create_video_encoder_config(
            profile_count,
            name,
            width,
            height,
            framerate,
            bitrate,
            video_encoding,
            h264_profile,
        )
    }

    /// Create audio encoder configuration.
    fn create_audio_encoder_config(
        profile_count: u32,
        bitrate: u32,
        sample_rate: u32,
        encoding: crate::onvif::types::common::AudioEncoding,
    ) -> AudioEncoderConfiguration {
        defaults::create_audio_encoder_config(profile_count, bitrate, sample_rate, encoding)
    }

    /// Create profile from configuration.
    #[allow(clippy::too_many_arguments)]
    fn create_profile_from_config(
        name: &str,
        width: u32,
        height: u32,
        profile_count: u32,
        audio_enabled: bool,
        video_encoder_config: VideoEncoderConfiguration,
        audio_encoder_config: Option<AudioEncoderConfiguration>,
        default_ptz_config: &PTZConfiguration,
    ) -> Profile {
        let _ = (width, height);
        defaults::create_profile(
            name,
            profile_count,
            Some(video_encoder_config),
            if audio_enabled {
                audio_encoder_config
            } else {
                None
            },
            default_ptz_config,
        )
    }

    /// Initialize hardcoded profiles (fallback when no config is available).
    fn initialize_profiles_hardcoded(&self, default_ptz_config: PTZConfiguration) {
        let video_encoder_config_main = Self::create_video_encoder_config(
            0,
            "MainStream",
            1920,
            1080,
            30,
            4000,
            &crate::onvif::types::common::VideoEncoding::H264,
            crate::onvif::types::common::H264Profile::Main,
        );
        self.video_encoder_configs.write().insert(
            format!("{}0", VIDEO_ENCODER_CONFIG_PREFIX),
            video_encoder_config_main.clone(),
        );

        let video_encoder_config_sub = Self::create_video_encoder_config(
            1,
            "SubStream",
            640,
            480,
            15,
            512,
            &crate::onvif::types::common::VideoEncoding::H264,
            crate::onvif::types::common::H264Profile::Baseline,
        );
        self.video_encoder_configs.write().insert(
            format!("{}1", VIDEO_ENCODER_CONFIG_PREFIX),
            video_encoder_config_sub.clone(),
        );

        let audio_encoder_config = Self::create_audio_encoder_config(
            0,
            64,
            8000,
            crate::onvif::types::common::AudioEncoding::G711,
        );
        self.audio_encoder_configs.write().insert(
            format!("{}0", AUDIO_ENCODER_CONFIG_PREFIX),
            audio_encoder_config.clone(),
        );

        let main_profile = defaults::create_profile(
            "MainStream",
            0,
            Some(video_encoder_config_main),
            Some(audio_encoder_config.clone()),
            &default_ptz_config,
        );
        self.profiles
            .write()
            .insert(format!("{}MainStream", PROFILE_TOKEN_PREFIX), main_profile);

        let sub_profile = defaults::create_profile(
            "SubStream",
            1,
            Some(video_encoder_config_sub),
            Some(audio_encoder_config),
            &default_ptz_config,
        );
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

    /// Get video encoder configuration options with dynamic resolution lists based on sensor maximum.
    pub fn get_video_encoder_configuration_options(&self) -> VideoEncoderConfigurationOptions {
        let resolutions = Self::generate_standard_resolutions(self.max_sensor_resolution);

        VideoEncoderConfigurationOptions {
            quality_range: IntRange { min: 0, max: 100 },
            jpeg: Some(JpegOptions {
                resolutions_available: resolutions.clone(),
                frame_rate_range: IntRange { min: 1, max: 30 },
                encoding_interval_range: IntRange { min: 1, max: 30 },
            }),
            mpeg4: None,
            h264: Some(H264Options {
                resolutions_available: resolutions,
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

    /// Generate standard resolution list filtered by sensor maximum capability.
    /// Filters common resolutions (1920x1080, 1280x720, 640x480, 320x240) to only those
    /// that fit within the sensor maximum resolution.
    fn generate_standard_resolutions(max: Resolution) -> Vec<VideoResolution> {
        let standard_resolutions = vec![
            (1920u32, 1080u32),
            (1280u32, 720u32),
            (640u32, 480u32),
            (320u32, 240u32),
        ];

        standard_resolutions
            .into_iter()
            .filter(|(w, h)| *w <= max.width && *h <= max.height)
            .map(|(w, h)| VideoResolution {
                width: w as i32,
                height: h as i32,
            })
            .collect()
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

    /// Persist the current profile state to storage if available.
    fn persist_all(&self) {
        let Some(storage) = &self.profile_storage else {
            return;
        };
        let snapshot = self.to_stored_snapshot();
        storage.replace(snapshot);
        if let Err(e) = storage.save() {
            tracing::warn!(
                "Failed to save profiles to {}: {e}",
                storage.path().display()
            );
        }
    }

    /// Load profiles from typed ProfileStorage. Returns `true` on success.
    fn load_from_storage(&self) -> bool {
        let Some(storage) = &self.profile_storage else {
            return false;
        };

        let data = storage.snapshot();
        if data.profiles.is_empty() {
            return false;
        }

        self.apply_stored_snapshot(&data);
        true
    }

    // ========================================================================
    // Conversion: ONVIF domain types ↔ stored DTOs
    // ========================================================================

    /// Convert current in-memory state to a `ProfilesFile` for serialization.
    fn to_stored_snapshot(&self) -> ProfilesFile {
        let profiles = self.profiles.read();
        let video_sources = self.video_sources.read();
        let audio_sources = self.audio_sources.read();
        let video_source_configs = self.video_source_configs.read();
        let video_encoder_configs = self.video_encoder_configs.read();
        let audio_source_configs = self.audio_source_configs.read();
        let audio_encoder_configs = self.audio_encoder_configs.read();

        ProfilesFile {
            profiles: profiles.values().map(Self::profile_to_stored).collect(),
            video_sources: video_sources
                .values()
                .map(Self::video_source_to_stored)
                .collect(),
            audio_sources: audio_sources
                .values()
                .map(Self::audio_source_to_stored)
                .collect(),
            video_source_configs: video_source_configs
                .values()
                .map(Self::video_source_config_to_stored)
                .collect(),
            video_encoder_configs: video_encoder_configs
                .values()
                .map(Self::video_encoder_config_to_stored)
                .collect(),
            audio_source_configs: audio_source_configs
                .values()
                .map(Self::audio_source_config_to_stored)
                .collect(),
            audio_encoder_configs: audio_encoder_configs
                .values()
                .map(Self::audio_encoder_config_to_stored)
                .collect(),
        }
    }

    /// Populate in-memory state from a `ProfilesFile`.
    fn apply_stored_snapshot(&self, data: &ProfilesFile) {
        self.clear_state();

        // Load sources first (profiles reference them)
        {
            let mut vs = self.video_sources.write();
            for stored in &data.video_sources {
                vs.insert(stored.token.clone(), Self::stored_to_video_source(stored));
            }
        }
        {
            let mut aus = self.audio_sources.write();
            for stored in &data.audio_sources {
                aus.insert(stored.token.clone(), Self::stored_to_audio_source(stored));
            }
        }
        {
            let mut vsc = self.video_source_configs.write();
            for stored in &data.video_source_configs {
                vsc.insert(
                    stored.token.clone(),
                    Self::stored_to_video_source_config(stored),
                );
            }
        }
        {
            let mut vec = self.video_encoder_configs.write();
            for stored in &data.video_encoder_configs {
                if let Some(cfg) = Self::stored_to_video_encoder_config(stored) {
                    vec.insert(stored.token.clone(), cfg);
                }
            }
        }
        {
            let mut asc = self.audio_source_configs.write();
            for stored in &data.audio_source_configs {
                asc.insert(
                    stored.token.clone(),
                    Self::stored_to_audio_source_config(stored),
                );
            }
        }
        {
            let mut aec = self.audio_encoder_configs.write();
            for stored in &data.audio_encoder_configs {
                if let Some(cfg) = Self::stored_to_audio_encoder_config(stored) {
                    aec.insert(stored.token.clone(), cfg);
                }
            }
        }

        // Load profiles last (they reference configs)
        {
            let mut prof = self.profiles.write();
            for stored in &data.profiles {
                if let Some(profile) = self.stored_to_profile(stored, data) {
                    prof.insert(stored.token.clone(), profile);
                }
            }
        }

        let count = data.profiles.len() as u32;
        self.profile_counter.store(count, Ordering::SeqCst);
    }

    // --- Individual converters: ONVIF → Stored ---

    fn profile_to_stored(profile: &Profile) -> StoredProfile {
        StoredProfile {
            token: profile.token.clone(),
            name: profile.name.clone(),
            fixed: profile.fixed.unwrap_or(false),
            video_source_config: profile
                .video_source_configuration
                .as_ref()
                .map(|c| c.token.clone()),
            video_encoder_config: profile
                .video_encoder_configuration
                .as_ref()
                .map(|c| c.token.clone()),
            audio_source_config: profile
                .audio_source_configuration
                .as_ref()
                .map(|c| c.token.clone()),
            audio_encoder_config: profile
                .audio_encoder_configuration
                .as_ref()
                .map(|c| c.token.clone()),
            ptz_config: profile.ptz_configuration.as_ref().map(|c| c.token.clone()),
        }
    }

    fn video_source_to_stored(s: &VideoSource) -> StoredVideoSource {
        StoredVideoSource {
            token: s.token.clone(),
            framerate: f64::from(s.framerate),
            width: s.resolution.width as u32,
            height: s.resolution.height as u32,
        }
    }

    fn audio_source_to_stored(s: &AudioSource) -> StoredAudioSource {
        StoredAudioSource {
            token: s.token.clone(),
            channels: s.channels as u32,
        }
    }

    fn video_source_config_to_stored(c: &VideoSourceConfiguration) -> StoredVideoSourceConfig {
        StoredVideoSourceConfig {
            token: c.token.clone(),
            source_token: c.source_token.clone(),
            name: c.name.clone(),
            use_count: c.use_count as u32,
            x: c.bounds.x,
            y: c.bounds.y,
            width: c.bounds.width as u32,
            height: c.bounds.height as u32,
        }
    }

    fn video_encoder_config_to_stored(c: &VideoEncoderConfiguration) -> StoredVideoEncoderConfig {
        StoredVideoEncoderConfig {
            token: c.token.clone(),
            name: c.name.clone(),
            encoding: match c.encoding {
                crate::onvif::types::common::VideoEncoding::H264 => "H264",
                crate::onvif::types::common::VideoEncoding::JPEG => "MJPEG",
                crate::onvif::types::common::VideoEncoding::MPEG4 => "MPEG4",
            }
            .to_string(),
            width: c.resolution.width as u32,
            height: c.resolution.height as u32,
            quality: f64::from(c.quality),
            frame_rate_limit: c.rate_control.as_ref().map(|r| r.frame_rate_limit as u32),
            encoding_interval: c.rate_control.as_ref().map(|r| r.encoding_interval as u32),
            bitrate_limit: c.rate_control.as_ref().map(|r| r.bitrate_limit as u32),
            gov_length: c.h264.as_ref().map(|h| h.gov_length as u32),
            h264_profile: c.h264.as_ref().map(|h| {
                match h.h264_profile {
                    crate::onvif::types::common::H264Profile::Baseline => "Baseline",
                    crate::onvif::types::common::H264Profile::Main => "Main",
                    crate::onvif::types::common::H264Profile::Extended => "Extended",
                    crate::onvif::types::common::H264Profile::High => "High",
                }
                .to_string()
            }),
            session_timeout: Some(c.session_timeout.clone()),
        }
    }

    fn audio_source_config_to_stored(c: &AudioSourceConfiguration) -> StoredAudioSourceConfig {
        StoredAudioSourceConfig {
            token: c.token.clone(),
            source_token: c.source_token.clone(),
            name: c.name.clone(),
            use_count: c.use_count as u32,
        }
    }

    fn audio_encoder_config_to_stored(c: &AudioEncoderConfiguration) -> StoredAudioEncoderConfig {
        StoredAudioEncoderConfig {
            token: c.token.clone(),
            name: c.name.clone(),
            encoding: match c.encoding {
                crate::onvif::types::common::AudioEncoding::G711 => "G711",
                crate::onvif::types::common::AudioEncoding::G726 => "G726",
                crate::onvif::types::common::AudioEncoding::AAC => "AAC",
            }
            .to_string(),
            bitrate: Some(c.bitrate as u32),
            sample_rate: Some(c.sample_rate as u32),
            session_timeout: Some(c.session_timeout.clone()),
        }
    }

    // --- Individual converters: Stored → ONVIF ---

    fn stored_to_video_source(s: &StoredVideoSource) -> VideoSource {
        VideoSource {
            token: s.token.clone(),
            framerate: s.framerate as f32,
            resolution: VideoResolution {
                width: s.width as i32,
                height: s.height as i32,
            },
            imaging: None,
            extension: None,
        }
    }

    fn stored_to_audio_source(s: &StoredAudioSource) -> AudioSource {
        AudioSource {
            token: s.token.clone(),
            channels: s.channels as i32,
        }
    }

    fn stored_to_video_source_config(s: &StoredVideoSourceConfig) -> VideoSourceConfiguration {
        VideoSourceConfiguration {
            token: s.token.clone(),
            source_token: s.source_token.clone(),
            name: s.name.clone(),
            use_count: s.use_count as i32,
            view_mode: None,
            bounds: IntRectangle {
                x: s.x,
                y: s.y,
                width: s.width as i32,
                height: s.height as i32,
            },
            extension: None,
        }
    }

    fn stored_to_video_encoder_config(
        s: &StoredVideoEncoderConfig,
    ) -> Option<VideoEncoderConfiguration> {
        let encoding = match s.encoding.as_str() {
            "H264" => crate::onvif::types::common::VideoEncoding::H264,
            "MJPEG" => crate::onvif::types::common::VideoEncoding::JPEG,
            "MPEG4" => crate::onvif::types::common::VideoEncoding::MPEG4,
            _ => return None,
        };

        let h264_profile = s.h264_profile.as_deref().and_then(|p| match p {
            "Baseline" => Some(crate::onvif::types::common::H264Profile::Baseline),
            "Main" => Some(crate::onvif::types::common::H264Profile::Main),
            "Extended" => Some(crate::onvif::types::common::H264Profile::Extended),
            "High" => Some(crate::onvif::types::common::H264Profile::High),
            _ => None,
        });

        let rate_control = match (s.frame_rate_limit, s.encoding_interval, s.bitrate_limit) {
            (Some(fr), Some(interval), Some(br)) => Some(VideoRateControl {
                frame_rate_limit: fr as i32,
                encoding_interval: interval as i32,
                bitrate_limit: br as i32,
            }),
            _ => None,
        };

        Some(VideoEncoderConfiguration {
            token: s.token.clone(),
            name: s.name.clone(),
            use_count: 1,
            encoding,
            resolution: VideoResolution {
                width: s.width as i32,
                height: s.height as i32,
            },
            quality: s.quality as f32,
            rate_control,
            mpeg4: None,
            h264: h264_profile.map(|profile| crate::onvif::types::common::H264Configuration {
                gov_length: s.gov_length.unwrap_or(30) as i32,
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
            session_timeout: s
                .session_timeout
                .clone()
                .unwrap_or_else(|| "PT60S".to_string()),
        })
    }

    fn stored_to_audio_source_config(s: &StoredAudioSourceConfig) -> AudioSourceConfiguration {
        AudioSourceConfiguration {
            token: s.token.clone(),
            source_token: s.source_token.clone(),
            name: s.name.clone(),
            use_count: s.use_count as i32,
        }
    }

    fn stored_to_audio_encoder_config(
        s: &StoredAudioEncoderConfig,
    ) -> Option<AudioEncoderConfiguration> {
        let encoding = match s.encoding.as_str() {
            "G711" | "PCMU" | "PCMA" => crate::onvif::types::common::AudioEncoding::G711,
            "G726" => crate::onvif::types::common::AudioEncoding::G726,
            "AAC" => crate::onvif::types::common::AudioEncoding::AAC,
            _ => return None,
        };

        Some(AudioEncoderConfiguration {
            token: s.token.clone(),
            name: s.name.clone(),
            use_count: 1,
            encoding,
            bitrate: s.bitrate.unwrap_or(64) as i32,
            sample_rate: s.sample_rate.unwrap_or(8000) as i32,
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
            session_timeout: s
                .session_timeout
                .clone()
                .unwrap_or_else(|| "PT60S".to_string()),
        })
    }

    fn stored_to_profile(&self, stored: &StoredProfile, file: &ProfilesFile) -> Option<Profile> {
        let video_source_configuration = stored.video_source_config.as_ref().and_then(|token| {
            file.video_source_configs
                .iter()
                .find(|c| c.token == *token)
                .map(Self::stored_to_video_source_config)
        });

        let video_encoder_configuration = stored.video_encoder_config.as_ref().and_then(|token| {
            file.video_encoder_configs
                .iter()
                .find(|c| c.token == *token)
                .and_then(Self::stored_to_video_encoder_config)
        });

        let audio_source_configuration = stored.audio_source_config.as_ref().and_then(|token| {
            file.audio_source_configs
                .iter()
                .find(|c| c.token == *token)
                .map(Self::stored_to_audio_source_config)
        });

        let audio_encoder_configuration = stored.audio_encoder_config.as_ref().and_then(|token| {
            file.audio_encoder_configs
                .iter()
                .find(|c| c.token == *token)
                .and_then(Self::stored_to_audio_encoder_config)
        });

        let ptz_configuration = stored
            .ptz_config
            .as_ref()
            .map(|_| Self::create_default_ptz_configuration());

        Some(Profile {
            token: stored.token.clone(),
            fixed: Some(stored.fixed),
            name: stored.name.clone(),
            video_source_configuration,
            audio_source_configuration,
            video_encoder_configuration,
            audio_encoder_configuration,
            ptz_configuration,
            metadata_configuration: None,
            extension: None,
        })
    }

    /// Clear all state before loading.
    fn clear_state(&self) {
        self.profiles.write().clear();
        self.video_sources.write().clear();
        self.audio_sources.write().clear();
        self.video_source_configs.write().clear();
        self.video_encoder_configs.write().clear();
        self.audio_source_configs.write().clear();
        self.audio_encoder_configs.write().clear();
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
        defaults::create_default_ptz_configuration()
    }
}

impl Default for ProfileManager {
    fn default() -> Self {
        Self::new()
    }
}

// ProfileManager fields use thread-safe primitives (RwLock, AtomicU32, Arc)
// so Send/Sync are automatically derived by the compiler.

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
    fn test_persist_defaults_with_profile_storage() {
        let dir = tempfile::tempdir().unwrap();
        let runtime = Arc::new(ConfigRuntime::new(Default::default()));
        let storage = Arc::new(ProfileStorage::new(dir.path().join("profiles.toml")));
        let manager = ProfileManager::with_storage(
            Arc::clone(&runtime),
            Arc::clone(&storage),
            Resolution::new(1920, 1080),
        );

        // Storage should have profiles after initialization
        let snapshot = storage.snapshot();
        assert!(!snapshot.profiles.is_empty());
        // Should have at least the enabled default profiles
        assert!(manager.get_profiles().len() >= 2);
    }

    #[test]
    fn test_load_profiles_from_storage() {
        let dir = tempfile::tempdir().unwrap();
        let runtime = Arc::new(ConfigRuntime::new(Default::default()));
        let storage = Arc::new(ProfileStorage::new(dir.path().join("profiles.toml")));
        let manager = ProfileManager::with_storage(
            Arc::clone(&runtime),
            Arc::clone(&storage),
            Resolution::new(1920, 1080),
        );

        // Create new profile and ensure it persists to storage
        let profile = manager
            .create_profile("PersistedProfile".to_string(), None)
            .unwrap();
        let snapshot = storage.snapshot();
        assert!(snapshot.profiles.iter().any(|p| p.token == profile.token));

        // New manager with same storage should load persisted profile
        let manager_reloaded = ProfileManager::with_storage(
            Arc::clone(&runtime),
            Arc::clone(&storage),
            Resolution::new(1920, 1080),
        );
        let loaded = manager_reloaded.get_profile(&profile.token).unwrap();
        assert_eq!(loaded.name, "PersistedProfile");
    }

    // ========================================================================
    // Parsing Helper Tests
    // ========================================================================

    #[test]
    fn test_parse_video_encoding_h264() {
        let encoding = ProfileManager::parse_video_encoding("h264");
        assert!(matches!(
            encoding,
            crate::onvif::types::common::VideoEncoding::H264
        ));
    }

    #[test]
    fn test_parse_video_encoding_h264_uppercase() {
        let encoding = ProfileManager::parse_video_encoding("H264");
        assert!(matches!(
            encoding,
            crate::onvif::types::common::VideoEncoding::H264
        ));
    }

    #[test]
    fn test_parse_video_encoding_mjpeg() {
        let encoding = ProfileManager::parse_video_encoding("mjpeg");
        assert!(matches!(
            encoding,
            crate::onvif::types::common::VideoEncoding::JPEG
        ));
    }

    #[test]
    fn test_parse_video_encoding_jpeg() {
        let encoding = ProfileManager::parse_video_encoding("jpeg");
        assert!(matches!(
            encoding,
            crate::onvif::types::common::VideoEncoding::JPEG
        ));
    }

    #[test]
    fn test_parse_video_encoding_mpeg4() {
        let encoding = ProfileManager::parse_video_encoding("mpeg4");
        assert!(matches!(
            encoding,
            crate::onvif::types::common::VideoEncoding::MPEG4
        ));
    }

    #[test]
    fn test_parse_video_encoding_unknown_defaults_to_h264() {
        let encoding = ProfileManager::parse_video_encoding("unknown");
        assert!(matches!(
            encoding,
            crate::onvif::types::common::VideoEncoding::H264
        ));
    }

    #[test]
    fn test_parse_h264_profile_baseline() {
        let profile = ProfileManager::parse_h264_profile("baseline");
        assert!(matches!(
            profile,
            crate::onvif::types::common::H264Profile::Baseline
        ));
    }

    #[test]
    fn test_parse_h264_profile_main() {
        let profile = ProfileManager::parse_h264_profile("main");
        assert!(matches!(
            profile,
            crate::onvif::types::common::H264Profile::Main
        ));
    }

    #[test]
    fn test_parse_h264_profile_high() {
        let profile = ProfileManager::parse_h264_profile("high");
        assert!(matches!(
            profile,
            crate::onvif::types::common::H264Profile::High
        ));
    }

    #[test]
    fn test_parse_h264_profile_unknown_defaults_to_main() {
        let profile = ProfileManager::parse_h264_profile("unknown");
        assert!(matches!(
            profile,
            crate::onvif::types::common::H264Profile::Main
        ));
    }

    #[test]
    fn test_parse_audio_encoding_g711() {
        let encoding = ProfileManager::parse_audio_encoding("g711");
        assert!(matches!(
            encoding,
            crate::onvif::types::common::AudioEncoding::G711
        ));
    }

    #[test]
    fn test_parse_audio_encoding_aac() {
        let encoding = ProfileManager::parse_audio_encoding("aac");
        assert!(matches!(
            encoding,
            crate::onvif::types::common::AudioEncoding::AAC
        ));
    }

    #[test]
    fn test_parse_audio_encoding_g726() {
        let encoding = ProfileManager::parse_audio_encoding("g726");
        assert!(matches!(
            encoding,
            crate::onvif::types::common::AudioEncoding::G726
        ));
    }

    #[test]
    fn test_parse_audio_encoding_unknown_defaults_to_g711() {
        let encoding = ProfileManager::parse_audio_encoding("unknown");
        assert!(matches!(
            encoding,
            crate::onvif::types::common::AudioEncoding::G711
        ));
    }

    // ========================================================================
    // Configuration Methods Tests
    // ========================================================================

    #[test]
    fn test_get_video_source_configurations() {
        let manager = ProfileManager::new();
        let configs = manager.get_video_source_configurations();
        assert!(!configs.is_empty());
    }

    #[test]
    fn test_get_audio_source_configurations() {
        let manager = ProfileManager::new();
        let configs = manager.get_audio_source_configurations();
        assert!(!configs.is_empty());
    }

    #[test]
    fn test_get_audio_encoder_configurations() {
        let manager = ProfileManager::new();
        let configs = manager.get_audio_encoder_configurations();
        assert!(!configs.is_empty());
    }

    #[test]
    fn test_get_video_source_configuration_by_token() {
        let manager = ProfileManager::new();
        let configs = manager.get_video_source_configurations();
        if !configs.is_empty() {
            let token = &configs[0].token;
            let result = manager.get_video_source_configuration(token);
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_get_video_encoder_configuration_by_token() {
        let manager = ProfileManager::new();
        let configs = manager.get_video_encoder_configurations();
        if !configs.is_empty() {
            let token = &configs[0].token;
            let result = manager.get_video_encoder_configuration(token);
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_get_audio_source_configuration_by_token() {
        let manager = ProfileManager::new();
        let configs = manager.get_audio_source_configurations();
        if !configs.is_empty() {
            let token = &configs[0].token;
            let result = manager.get_audio_source_configuration(token);
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_get_audio_encoder_configuration_by_token() {
        let manager = ProfileManager::new();
        let configs = manager.get_audio_encoder_configurations();
        if !configs.is_empty() {
            let token = &configs[0].token;
            let result = manager.get_audio_encoder_configuration(token);
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_get_nonexistent_configuration_returns_error() {
        let manager = ProfileManager::new();
        let result = manager.get_video_encoder_configuration(&"NonExistent".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_add_video_source_configuration() {
        let manager = ProfileManager::new();
        let profile = manager
            .create_profile("TestProfile".to_string(), None)
            .unwrap();
        let config_token = format!("{}0", VIDEO_SOURCE_CONFIG_PREFIX);
        let result = manager.add_video_source_configuration(&profile.token, &config_token);
        assert!(result.is_ok());

        let updated_profile = manager.get_profile(&profile.token).unwrap();
        assert!(updated_profile.video_source_configuration.is_some());
    }

    #[test]
    fn test_add_audio_source_configuration() {
        let manager = ProfileManager::new();
        let profile = manager
            .create_profile("TestProfile".to_string(), None)
            .unwrap();
        let config_token = format!("{}0", AUDIO_SOURCE_CONFIG_PREFIX);
        let result = manager.add_audio_source_configuration(&profile.token, &config_token);
        assert!(result.is_ok());

        let updated_profile = manager.get_profile(&profile.token).unwrap();
        assert!(updated_profile.audio_source_configuration.is_some());
    }

    #[test]
    fn test_add_audio_encoder_configuration() {
        let manager = ProfileManager::new();
        let profile = manager
            .create_profile("TestProfile".to_string(), None)
            .unwrap();
        let config_token = format!("{}0", AUDIO_ENCODER_CONFIG_PREFIX);
        let result = manager.add_audio_encoder_configuration(&profile.token, &config_token);
        assert!(result.is_ok());

        let updated_profile = manager.get_profile(&profile.token).unwrap();
        assert!(updated_profile.audio_encoder_configuration.is_some());
    }

    #[test]
    fn test_remove_video_encoder_configuration() {
        let manager = ProfileManager::new();
        let profile = manager
            .create_profile("TestProfile".to_string(), None)
            .unwrap();
        let config_token = format!("{}0", VIDEO_ENCODER_CONFIG_PREFIX);
        manager
            .add_video_encoder_configuration(&profile.token, &config_token)
            .unwrap();

        let result = manager.remove_video_encoder_configuration(&profile.token);
        assert!(result.is_ok());

        let updated_profile = manager.get_profile(&profile.token).unwrap();
        assert!(updated_profile.video_encoder_configuration.is_none());
    }

    #[test]
    fn test_remove_audio_encoder_configuration() {
        let manager = ProfileManager::new();
        let profile = manager
            .create_profile("TestProfile".to_string(), None)
            .unwrap();
        let config_token = format!("{}0", AUDIO_ENCODER_CONFIG_PREFIX);
        manager
            .add_audio_encoder_configuration(&profile.token, &config_token)
            .unwrap();

        let result = manager.remove_audio_encoder_configuration(&profile.token);
        assert!(result.is_ok());

        let updated_profile = manager.get_profile(&profile.token).unwrap();
        assert!(updated_profile.audio_encoder_configuration.is_none());
    }

    #[test]
    fn test_get_compatible_video_source_configurations() {
        let manager = ProfileManager::new();
        let compatible =
            manager.get_compatible_video_source_configurations(&"SomeToken".to_string());
        // Should return all video source configurations
        assert!(!compatible.is_empty());
    }

    #[test]
    fn test_get_compatible_video_encoder_configurations() {
        let manager = ProfileManager::new();
        let compatible =
            manager.get_compatible_video_encoder_configurations(&"SomeToken".to_string());
        assert!(!compatible.is_empty());
    }

    #[test]
    fn test_get_compatible_audio_source_configurations() {
        let manager = ProfileManager::new();
        let compatible =
            manager.get_compatible_audio_source_configurations(&"SomeToken".to_string());
        assert!(!compatible.is_empty());
    }

    #[test]
    fn test_get_compatible_audio_encoder_configurations() {
        let manager = ProfileManager::new();
        let compatible =
            manager.get_compatible_audio_encoder_configurations(&"SomeToken".to_string());
        assert!(!compatible.is_empty());
    }

    #[test]
    fn test_get_video_encoder_configuration_options() {
        let manager = ProfileManager::new();
        let options = manager.get_video_encoder_configuration_options();
        // Quality range should have valid min/max values
        assert!(options.quality_range.min <= options.quality_range.max);
    }

    #[test]
    fn test_profile_manager_default() {
        let manager = ProfileManager::default();
        let profiles = manager.get_profiles();
        assert_eq!(profiles.len(), 2);
    }

    #[test]
    fn test_profile_manager_create_profile_at_limit_succeeds() {
        let manager = ProfileManager::new();
        assert_eq!(manager.get_profiles().len(), 2);
        for i in 0..(MAX_PROFILES - 2) {
            assert!(
                manager
                    .create_profile(format!("Profile{}", i), None)
                    .is_ok(),
                "Failed to create profile {} when under limit (MAX_PROFILES={})",
                i,
                MAX_PROFILES
            );
        }
        assert_eq!(manager.get_profiles().len(), MAX_PROFILES);
    }

    #[test]
    fn test_profile_manager_create_profile_exceeds_limit_fails() {
        let manager = ProfileManager::new();
        // Verify manager starts with 2 default profiles
        assert_eq!(
            manager.get_profiles().len(),
            2,
            "expected 2 default profiles at startup"
        );
        // Manager starts with 2 default profiles, so we can create MAX_PROFILES - 2 more
        for i in 0..(MAX_PROFILES - 2) {
            let result = manager.create_profile(format!("Profile{}", i), None);
            assert!(
                result.is_ok(),
                "Failed to create profile {} when under limit",
                i
            );
        }
        // Now at MAX_PROFILES, next create should fail
        let result = manager.create_profile("OneMoreProfile".to_string(), None);
        assert!(
            result.is_err(),
            "Expected error when creating profile beyond MAX_PROFILES limit"
        );
    }

    #[test]
    fn test_create_profile_empty_name_fails() {
        let manager = ProfileManager::new();
        // Empty name should fail validation
        let result = manager.create_profile("".to_string(), None);
        assert!(result.is_err());
    }

    #[test]
    fn test_add_configuration_to_nonexistent_profile() {
        let manager = ProfileManager::new();
        let config_token = format!("{}0", VIDEO_ENCODER_CONFIG_PREFIX);
        let result =
            manager.add_video_encoder_configuration(&"NonExistent".to_string(), &config_token);
        assert!(result.is_err());
    }

    #[test]
    fn test_add_nonexistent_configuration_to_profile() {
        let manager = ProfileManager::new();
        let profile = manager
            .create_profile("TestProfile".to_string(), None)
            .unwrap();
        let result =
            manager.add_video_encoder_configuration(&profile.token, &"NonExistent".to_string());
        assert!(result.is_err());
    }
}
