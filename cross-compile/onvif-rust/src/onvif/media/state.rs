//! Media State - In-memory profile and configuration state.
//!
//! This module provides thread-safe in-memory state management for media profiles,
//! video/audio sources, and configurations using RwLock primitives.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};

use parking_lot::RwLock;

use crate::onvif::types::common::{
    AudioEncoderConfiguration, AudioSource, AudioSourceConfiguration, Profile, ReferenceToken,
    VideoEncoderConfiguration, VideoSource, VideoSourceConfiguration,
};
use crate::platform::Resolution;

#[allow(unused_imports)]
use super::types::{
    AUDIO_ENCODER_CONFIG_PREFIX, AUDIO_SOURCE_CONFIG_PREFIX, DEFAULT_AUDIO_SOURCE_TOKEN,
    DEFAULT_PTZ_NODE_TOKEN, DEFAULT_VIDEO_SOURCE_TOKEN, MAX_PROFILES, PROFILE_TOKEN_PREFIX,
    PTZ_CONFIG_PREFIX, VIDEO_ENCODER_CONFIG_PREFIX, VIDEO_SOURCE_CONFIG_PREFIX,
};
use super::validation;
use crate::onvif::error::{OnvifError, OnvifResult};

/// Media state containing all profiles, sources, and configurations.
/// Thread-safe using RwLock for concurrent read access.
pub struct MediaState {
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
}

impl MediaState {
    /// Create a new MediaState with default configuration.
    pub fn new(max_resolution: Resolution) -> Self {
        Self {
            profiles: RwLock::new(HashMap::new()),
            video_sources: RwLock::new(HashMap::new()),
            audio_sources: RwLock::new(HashMap::new()),
            video_source_configs: RwLock::new(HashMap::new()),
            video_encoder_configs: RwLock::new(HashMap::new()),
            audio_source_configs: RwLock::new(HashMap::new()),
            audio_encoder_configs: RwLock::new(HashMap::new()),
            profile_counter: AtomicU32::new(0),
            max_sensor_resolution: max_resolution,
        }
    }

    /// Get all profiles.
    pub fn get_profiles(&self) -> Vec<Profile> {
        self.profiles.read().values().cloned().collect()
    }

    /// Get a profile by token.
    pub fn get_profile(&self, token: &str) -> OnvifResult<Profile> {
        validation::validate_profile_token(&token.to_string())?;
        self.profiles
            .read()
            .get(token)
            .cloned()
            .ok_or_else(|| super::faults::no_profile_error(token))
    }

    /// Create a new profile.
    pub fn create_profile(&self, name: String, token: Option<String>) -> OnvifResult<Profile> {
        validation::validate_profile_name(&name)?;

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
            validation::validate_profile_token(&t)?;
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

        profiles.insert(profile_token, profile.clone());
        Ok(profile)
    }

    /// Delete a profile.
    pub fn delete_profile(&self, token: &str) -> OnvifResult<()> {
        validation::validate_profile_token(&token.to_string())?;

        let mut profiles = self.profiles.write();

        // Check if profile exists
        let profile = profiles
            .get(token)
            .ok_or_else(|| super::faults::no_profile_error(token))?;

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
        token: &str,
    ) -> OnvifResult<VideoSourceConfiguration> {
        self.video_source_configs
            .read()
            .get(token)
            .cloned()
            .ok_or_else(|| super::faults::no_config_error(token))
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
        token: &str,
    ) -> OnvifResult<VideoEncoderConfiguration> {
        self.video_encoder_configs
            .read()
            .get(token)
            .cloned()
            .ok_or_else(|| super::faults::no_config_error(token))
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
        token: &str,
    ) -> OnvifResult<AudioSourceConfiguration> {
        self.audio_source_configs
            .read()
            .get(token)
            .cloned()
            .ok_or_else(|| super::faults::no_config_error(token))
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
        token: &str,
    ) -> OnvifResult<AudioEncoderConfiguration> {
        self.audio_encoder_configs
            .read()
            .get(token)
            .cloned()
            .ok_or_else(|| super::faults::no_config_error(token))
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

    /// Add video source configuration to a profile.
    pub fn add_video_source_configuration(
        &self,
        profile_token: &str,
        config_token: &str,
    ) -> OnvifResult<()> {
        let config = self.get_video_source_configuration(config_token)?;
        let mut profiles = self.profiles.write();
        let profile = profiles
            .get_mut(profile_token)
            .ok_or_else(|| super::faults::no_profile_error(profile_token))?;
        profile.video_source_configuration = Some(config);
        Ok(())
    }

    /// Remove video source configuration from a profile.
    pub fn remove_video_source_configuration(&self, profile_token: &str) -> OnvifResult<()> {
        let mut profiles = self.profiles.write();
        let profile = profiles
            .get_mut(profile_token)
            .ok_or_else(|| super::faults::no_profile_error(profile_token))?;
        profile.video_source_configuration = None;
        Ok(())
    }

    /// Add video encoder configuration to a profile.
    pub fn add_video_encoder_configuration(
        &self,
        profile_token: &str,
        config_token: &str,
    ) -> OnvifResult<()> {
        let config = self.get_video_encoder_configuration(config_token)?;
        let mut profiles = self.profiles.write();
        let profile = profiles
            .get_mut(profile_token)
            .ok_or_else(|| super::faults::no_profile_error(profile_token))?;
        profile.video_encoder_configuration = Some(config);
        Ok(())
    }

    /// Remove video encoder configuration from a profile.
    pub fn remove_video_encoder_configuration(&self, profile_token: &str) -> OnvifResult<()> {
        let mut profiles = self.profiles.write();
        let profile = profiles
            .get_mut(profile_token)
            .ok_or_else(|| super::faults::no_profile_error(profile_token))?;
        profile.video_encoder_configuration = None;
        Ok(())
    }

    /// Add audio source configuration to a profile.
    pub fn add_audio_source_configuration(
        &self,
        profile_token: &str,
        config_token: &str,
    ) -> OnvifResult<()> {
        let config = self.get_audio_source_configuration(config_token)?;
        let mut profiles = self.profiles.write();
        let profile = profiles
            .get_mut(profile_token)
            .ok_or_else(|| super::faults::no_profile_error(profile_token))?;
        profile.audio_source_configuration = Some(config);
        Ok(())
    }

    /// Remove audio source configuration from a profile.
    pub fn remove_audio_source_configuration(&self, profile_token: &str) -> OnvifResult<()> {
        let mut profiles = self.profiles.write();
        let profile = profiles
            .get_mut(profile_token)
            .ok_or_else(|| super::faults::no_profile_error(profile_token))?;
        profile.audio_source_configuration = None;
        Ok(())
    }

    /// Add audio encoder configuration to a profile.
    pub fn add_audio_encoder_configuration(
        &self,
        profile_token: &str,
        config_token: &str,
    ) -> OnvifResult<()> {
        let config = self.get_audio_encoder_configuration(config_token)?;
        let mut profiles = self.profiles.write();
        let profile = profiles
            .get_mut(profile_token)
            .ok_or_else(|| super::faults::no_profile_error(profile_token))?;
        profile.audio_encoder_configuration = Some(config);
        Ok(())
    }

    /// Remove audio encoder configuration from a profile.
    pub fn remove_audio_encoder_configuration(&self, profile_token: &str) -> OnvifResult<()> {
        let mut profiles = self.profiles.write();
        let profile = profiles
            .get_mut(profile_token)
            .ok_or_else(|| super::faults::no_profile_error(profile_token))?;
        profile.audio_encoder_configuration = None;
        Ok(())
    }

    /// Insert a profile directly (used by store.rs for loading).
    pub fn insert_profile(&self, profile: Profile) {
        self.profiles.write().insert(profile.token.clone(), profile);
    }

    /// Insert a video source directly.
    pub fn insert_video_source(&self, source: VideoSource) {
        self.video_sources
            .write()
            .insert(source.token.clone(), source);
    }

    /// Insert an audio source directly.
    pub fn insert_audio_source(&self, source: AudioSource) {
        self.audio_sources
            .write()
            .insert(source.token.clone(), source);
    }

    /// Insert a video source configuration directly.
    pub fn insert_video_source_config(&self, config: VideoSourceConfiguration) {
        self.video_source_configs
            .write()
            .insert(config.token.clone(), config);
    }

    /// Insert a video encoder configuration directly.
    pub fn insert_video_encoder_config(&self, config: VideoEncoderConfiguration) {
        self.video_encoder_configs
            .write()
            .insert(config.token.clone(), config);
    }

    /// Insert an audio source configuration directly.
    pub fn insert_audio_source_config(&self, config: AudioSourceConfiguration) {
        self.audio_source_configs
            .write()
            .insert(config.token.clone(), config);
    }

    /// Insert an audio encoder configuration directly.
    pub fn insert_audio_encoder_config(&self, config: AudioEncoderConfiguration) {
        self.audio_encoder_configs
            .write()
            .insert(config.token.clone(), config);
    }

    /// Clear all state.
    pub fn clear(&self) {
        self.profiles.write().clear();
        self.video_sources.write().clear();
        self.audio_sources.write().clear();
        self.video_source_configs.write().clear();
        self.video_encoder_configs.write().clear();
        self.audio_source_configs.write().clear();
        self.audio_encoder_configs.write().clear();
    }

    /// Get the profile counter value.
    pub fn profile_count(&self) -> usize {
        self.profiles.read().len()
    }

    /// Set profile counter.
    pub fn set_profile_counter(&self, count: u32) {
        self.profile_counter.store(count, Ordering::SeqCst);
    }

    /// Get max sensor resolution.
    pub fn max_sensor_resolution(&self) -> Resolution {
        self.max_sensor_resolution
    }
}

impl Default for MediaState {
    fn default() -> Self {
        Self::new(Resolution::new(1920, 1080))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_media_state_new() {
        let state = MediaState::new(Resolution::new(1920, 1080));
        assert!(state.get_profiles().is_empty());
    }

    #[test]
    fn test_create_and_get_profile() {
        let state = MediaState::new(Resolution::new(1920, 1080));
        let profile = state
            .create_profile("TestProfile".to_string(), None)
            .unwrap();
        assert_eq!(profile.name, "TestProfile");

        let retrieved = state.get_profile(&profile.token).unwrap();
        assert_eq!(retrieved.name, "TestProfile");
    }

    #[test]
    fn test_delete_profile() {
        let state = MediaState::new(Resolution::new(1920, 1080));
        let profile = state.create_profile("ToDelete".to_string(), None).unwrap();
        state.delete_profile(&profile.token).unwrap();
        assert!(state.get_profile(&profile.token).is_err());
    }

    #[test]
    fn test_max_profiles_limit() {
        let state = MediaState::new(Resolution::new(1920, 1080));
        // Create up to MAX_PROFILES profiles
        for i in 0..MAX_PROFILES {
            let result = state.create_profile(format!("Profile{}", i), None);
            assert!(result.is_ok(), "Failed at iteration {}", i);
        }
        // Next one should fail
        let result = state.create_profile("OneMore".to_string(), None);
        assert!(result.is_err());
    }
}
