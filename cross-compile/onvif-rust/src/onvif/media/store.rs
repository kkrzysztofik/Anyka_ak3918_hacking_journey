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
    AudioEncoderConfiguration, PTZConfiguration, VideoEncoderConfiguration,
};
use crate::platform::Resolution;

use super::defaults::{self, ProfileConfig};
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
        let default_sources = defaults::create_default_sources(self.max_sensor_resolution);
        state.insert_video_source(default_sources.video_source);
        state.insert_audio_source(default_sources.audio_source);
        state.insert_video_source_config(default_sources.video_source_config);
        state.insert_audio_source_config(default_sources.audio_source_config);

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

        let video_encoder_main = Self::create_video_encoder_config(
            0,
            "MainStream",
            1920,
            1080,
            30,
            4000,
            &crate::onvif::types::common::VideoEncoding::H264,
            crate::onvif::types::common::H264Profile::Main,
        );
        state.insert_video_encoder_config(video_encoder_main.clone());

        let video_encoder_sub = Self::create_video_encoder_config(
            1,
            "SubStream",
            640,
            480,
            15,
            512,
            &crate::onvif::types::common::VideoEncoding::H264,
            crate::onvif::types::common::H264Profile::Baseline,
        );
        state.insert_video_encoder_config(video_encoder_sub.clone());

        let audio_encoder = Self::create_audio_encoder_config(
            0,
            64,
            8000,
            crate::onvif::types::common::AudioEncoding::G711,
        );
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
        defaults::create_profile(
            name,
            profile_count,
            video_encoder,
            audio_encoder,
            ptz_config,
        )
    }

    /// Check if a profile is enabled in config.
    fn is_profile_enabled(config: &ConfigRuntime, profile_num: u32) -> bool {
        defaults::is_profile_enabled(config, profile_num)
    }

    /// Read profile configuration from config.
    fn read_profile_config(config: &ConfigRuntime, profile_num: u32) -> ProfileConfig {
        defaults::read_profile_config(config, profile_num)
    }

    /// Parse video encoding string.
    fn parse_video_encoding(encoding_str: &str) -> crate::onvif::types::common::VideoEncoding {
        defaults::parse_video_encoding(encoding_str, false)
    }

    /// Parse H.264 profile string.
    fn parse_h264_profile(profile_str: &str) -> crate::onvif::types::common::H264Profile {
        defaults::parse_h264_profile(profile_str, false)
    }

    /// Parse audio encoding string.
    fn parse_audio_encoding(encoding_str: &str) -> crate::onvif::types::common::AudioEncoding {
        defaults::parse_audio_encoding(encoding_str, false)
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

    /// Create default PTZ configuration.
    fn create_default_ptz_configuration() -> PTZConfiguration {
        defaults::create_default_ptz_configuration()
    }

    /// Save state to storage.
    pub fn save(&self, _state: &MediaState) {
        // Note: Full persistence implementation would serialize state to storage
        // For now, this is a placeholder - the ProfileManager handles persistence
    }
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
