//! Media State - In-memory profile and configuration state.
//!
//! This module provides thread-safe in-memory state management for media profiles,
//! video/audio sources, and configurations using RwLock primitives.

#![cfg_attr(not(test), allow(dead_code))]

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};

use parking_lot::RwLock;

use crate::onvif::types::common::{
    AudioEncoderConfiguration, AudioSource, AudioSourceConfiguration, Profile, ReferenceToken,
    VideoEncoderConfiguration, VideoSource, VideoSourceConfiguration,
};
use crate::platform::Resolution;

use super::types::{MAX_PROFILES, PROFILE_TOKEN_PREFIX};
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
    /// Create a new empty [`MediaState`] with the given sensor resolution limit.
    ///
    /// All internal collections (profiles, sources, configurations) start empty.
    /// Call [`MediaStore::initialize_defaults`] to populate default profiles.
    ///
    /// # Arguments
    ///
    /// * `max_resolution` - Maximum sensor resolution used to validate profile
    ///   configurations. Profiles exceeding this resolution are rejected during
    ///   initialization.
    ///
    /// # Returns
    ///
    /// A new `MediaState` with empty collections and a zeroed profile counter.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use crate::platform::Resolution;
    /// let state = MediaState::new(Resolution::new(1920, 1080));
    /// assert!(state.get_profiles().is_empty());
    /// ```
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
    ///
    /// Returns cloned values because the `RwLock` guard cannot outlive this
    /// method. Returning `&[Profile]` is not possible since the guard would be
    /// dropped before the caller could use the reference.
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
                "MaxProfiles",
                format!("Maximum number of profiles ({}) reached", MAX_PROFILES),
            ));
        }

        // Generate or validate token
        let profile_token = if let Some(t) = token {
            validation::validate_profile_token(&t)?;
            if profiles.contains_key(&t) {
                return Err(OnvifError::invalid_arg_val(
                    "TokenConflict",
                    format!("Profile with token '{}' already exists", t),
                ));
            }
            t
        } else {
            // Generate a unique token, retrying on collision (e.g. if tokens
            // were manually inserted via insert_profile with overlapping names).
            loop {
                let counter = self.profile_counter.fetch_add(1, Ordering::SeqCst);
                let candidate = format!("{}{}", PROFILE_TOKEN_PREFIX, counter);
                if !profiles.contains_key(&candidate) {
                    break candidate;
                }
            }
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
                "DeletionOfFixedProfile",
                "Cannot delete a fixed profile",
            ));
        }

        profiles.remove(token);
        Ok(())
    }

    /// Get all video sources.
    ///
    /// Returns cloned values because the `RwLock` guard cannot outlive this
    /// method (see [`get_profiles`](Self::get_profiles) for rationale).
    pub fn get_video_sources(&self) -> Vec<VideoSource> {
        self.video_sources.read().values().cloned().collect()
    }

    /// Get all video source configurations.
    ///
    /// Returns cloned values because the `RwLock` guard cannot outlive this
    /// method (see [`get_profiles`](Self::get_profiles) for rationale).
    pub fn get_video_source_configurations(&self) -> Vec<VideoSourceConfiguration> {
        self.video_source_configs.read().values().cloned().collect()
    }

    /// Get a video source configuration by token.
    pub fn get_video_source_configuration(
        &self,
        token: &str,
    ) -> OnvifResult<VideoSourceConfiguration> {
        validation::validate_config_token(&token.to_string())?;
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
    ///
    /// Returns cloned values because the `RwLock` guard cannot outlive this
    /// method (see [`get_profiles`](Self::get_profiles) for rationale).
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
        validation::validate_config_token(&token.to_string())?;
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
    ///
    /// Returns cloned values because the `RwLock` guard cannot outlive this
    /// method (see [`get_profiles`](Self::get_profiles) for rationale).
    pub fn get_audio_sources(&self) -> Vec<AudioSource> {
        self.audio_sources.read().values().cloned().collect()
    }

    /// Get all audio source configurations.
    ///
    /// Returns cloned values because the `RwLock` guard cannot outlive this
    /// method (see [`get_profiles`](Self::get_profiles) for rationale).
    pub fn get_audio_source_configurations(&self) -> Vec<AudioSourceConfiguration> {
        self.audio_source_configs.read().values().cloned().collect()
    }

    /// Get an audio source configuration by token.
    pub fn get_audio_source_configuration(
        &self,
        token: &str,
    ) -> OnvifResult<AudioSourceConfiguration> {
        validation::validate_config_token(&token.to_string())?;
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
    ///
    /// Returns cloned values because the `RwLock` guard cannot outlive this
    /// method (see [`get_profiles`](Self::get_profiles) for rationale).
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
        validation::validate_config_token(&token.to_string())?;
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
        validation::validate_profile_token(&profile_token.to_string())?;
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
        validation::validate_profile_token(&profile_token.to_string())?;
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
        validation::validate_profile_token(&profile_token.to_string())?;
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
        validation::validate_profile_token(&profile_token.to_string())?;
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
    pub(crate) fn insert_profile(&self, profile: Profile) {
        self.profiles.write().insert(profile.token.clone(), profile);
    }

    /// Insert a video source directly.
    pub(crate) fn insert_video_source(&self, source: VideoSource) {
        self.video_sources
            .write()
            .insert(source.token.clone(), source);
    }

    /// Insert an audio source directly.
    pub(crate) fn insert_audio_source(&self, source: AudioSource) {
        self.audio_sources
            .write()
            .insert(source.token.clone(), source);
    }

    /// Insert a video source configuration directly.
    pub(crate) fn insert_video_source_config(&self, config: VideoSourceConfiguration) {
        self.video_source_configs
            .write()
            .insert(config.token.clone(), config);
    }

    /// Insert a video encoder configuration directly.
    pub(crate) fn insert_video_encoder_config(&self, config: VideoEncoderConfiguration) {
        self.video_encoder_configs
            .write()
            .insert(config.token.clone(), config);
    }

    /// Insert an audio source configuration directly.
    pub(crate) fn insert_audio_source_config(&self, config: AudioSourceConfiguration) {
        self.audio_source_configs
            .write()
            .insert(config.token.clone(), config);
    }

    /// Insert an audio encoder configuration directly.
    pub(crate) fn insert_audio_encoder_config(&self, config: AudioEncoderConfiguration) {
        self.audio_encoder_configs
            .write()
            .insert(config.token.clone(), config);
    }

    /// Clear all state.
    // NOTE: Clear acquires write locks sequentially. Brief inconsistency is acceptable
    // during service shutdown — no client requests are expected during this window.
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
    pub(crate) fn set_profile_counter(&self, count: u32) {
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
    use crate::onvif::media::store::MediaStore;
    use crate::onvif::media::types::PROFILE_TOKEN_PREFIX;
    use crate::platform::Resolution;

    fn create_initialized_state() -> MediaState {
        let state = MediaState::new(Resolution::new(1920, 1080));
        let store = MediaStore::new(None, Resolution::new(1920, 1080));
        store.initialize_defaults(&state);
        state
    }

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

    #[test]
    fn test_create_profile_duplicate_token_fails() {
        let state = MediaState::new(Resolution::new(1920, 1080));
        let token = "Profile_Custom".to_string();

        let created = state
            .create_profile("Primary".to_string(), Some(token.clone()))
            .unwrap();
        assert_eq!(created.token, token);

        let result = state.create_profile("Duplicate".to_string(), Some(token));
        assert!(result.is_err());
    }

    #[test]
    fn test_delete_fixed_profile_fails() {
        let state = create_initialized_state();
        let profile = state.get_profiles().into_iter().next().unwrap();

        let result = state.delete_profile(&profile.token);
        assert!(result.is_err());
    }

    #[test]
    fn test_set_and_get_video_source_configuration() {
        let state = create_initialized_state();
        let mut configuration = state.get_video_source_configurations().pop().unwrap();
        configuration.name = "UpdatedVideoSource".to_string();

        state
            .set_video_source_configuration(configuration.clone())
            .unwrap();

        let stored = state
            .get_video_source_configuration(&configuration.token)
            .unwrap();
        assert_eq!(stored.name, "UpdatedVideoSource");
    }

    #[test]
    fn test_set_and_get_video_encoder_configuration() {
        let state = create_initialized_state();
        let mut configuration = state.get_video_encoder_configurations().pop().unwrap();
        configuration.name = "UpdatedVideoEncoder".to_string();

        state
            .set_video_encoder_configuration(configuration.clone())
            .unwrap();

        let stored = state
            .get_video_encoder_configuration(&configuration.token)
            .unwrap();
        assert_eq!(stored.name, "UpdatedVideoEncoder");
    }

    #[test]
    fn test_set_and_get_audio_source_configuration() {
        let state = create_initialized_state();
        let mut configuration = state.get_audio_source_configurations().pop().unwrap();
        configuration.name = "UpdatedAudioSource".to_string();

        state
            .set_audio_source_configuration(configuration.clone())
            .unwrap();

        let stored = state
            .get_audio_source_configuration(&configuration.token)
            .unwrap();
        assert_eq!(stored.name, "UpdatedAudioSource");
    }

    #[test]
    fn test_set_and_get_audio_encoder_configuration() {
        let state = create_initialized_state();
        let mut configuration = state.get_audio_encoder_configurations().pop().unwrap();
        configuration.name = "UpdatedAudioEncoder".to_string();

        state
            .set_audio_encoder_configuration(configuration.clone())
            .unwrap();

        let stored = state
            .get_audio_encoder_configuration(&configuration.token)
            .unwrap();
        assert_eq!(stored.name, "UpdatedAudioEncoder");
    }

    #[test]
    fn test_add_and_remove_video_source_configuration() {
        let state = create_initialized_state();
        let profile = state
            .create_profile("AttachVideoSource".to_string(), None)
            .unwrap();
        let config = state.get_video_source_configurations().pop().unwrap();

        state
            .add_video_source_configuration(&profile.token, &config.token)
            .unwrap();
        let updated = state.get_profile(&profile.token).unwrap();
        assert_eq!(
            updated.video_source_configuration.unwrap().token,
            config.token
        );

        state
            .remove_video_source_configuration(&profile.token)
            .unwrap();
        let cleared = state.get_profile(&profile.token).unwrap();
        assert!(cleared.video_source_configuration.is_none());
    }

    #[test]
    fn test_add_and_remove_video_encoder_configuration() {
        let state = create_initialized_state();
        let profile = state
            .create_profile("AttachVideoEncoder".to_string(), None)
            .unwrap();
        let config = state.get_video_encoder_configurations().pop().unwrap();

        state
            .add_video_encoder_configuration(&profile.token, &config.token)
            .unwrap();
        let updated = state.get_profile(&profile.token).unwrap();
        assert_eq!(
            updated.video_encoder_configuration.unwrap().token,
            config.token
        );

        state
            .remove_video_encoder_configuration(&profile.token)
            .unwrap();
        let cleared = state.get_profile(&profile.token).unwrap();
        assert!(cleared.video_encoder_configuration.is_none());
    }

    #[test]
    fn test_add_and_remove_audio_configurations() {
        let state = create_initialized_state();
        let profile = state
            .create_profile("AttachAudio".to_string(), None)
            .unwrap();
        let source_config = state.get_audio_source_configurations().pop().unwrap();
        let encoder_config = state.get_audio_encoder_configurations().pop().unwrap();

        state
            .add_audio_source_configuration(&profile.token, &source_config.token)
            .unwrap();
        state
            .add_audio_encoder_configuration(&profile.token, &encoder_config.token)
            .unwrap();

        let updated = state.get_profile(&profile.token).unwrap();
        assert_eq!(
            updated.audio_source_configuration.unwrap().token,
            source_config.token
        );
        assert_eq!(
            updated.audio_encoder_configuration.unwrap().token,
            encoder_config.token
        );

        state
            .remove_audio_source_configuration(&profile.token)
            .unwrap();
        state
            .remove_audio_encoder_configuration(&profile.token)
            .unwrap();

        let cleared = state.get_profile(&profile.token).unwrap();
        assert!(cleared.audio_source_configuration.is_none());
        assert!(cleared.audio_encoder_configuration.is_none());
    }

    #[test]
    fn test_configuration_getters_return_error_for_missing_tokens() {
        let state = create_initialized_state();

        assert!(
            state
                .get_video_source_configuration("missing-video-source")
                .is_err()
        );
        assert!(
            state
                .get_video_encoder_configuration("missing-video-encoder")
                .is_err()
        );
        assert!(
            state
                .get_audio_source_configuration("missing-audio-source")
                .is_err()
        );
        assert!(
            state
                .get_audio_encoder_configuration("missing-audio-encoder")
                .is_err()
        );
    }

    #[test]
    fn test_add_configuration_missing_profile_fails() {
        let state = create_initialized_state();
        let video_source = state.get_video_source_configurations().pop().unwrap();

        let result = state.add_video_source_configuration("missing-profile", &video_source.token);
        assert!(result.is_err());
    }

    #[test]
    fn test_clear_resets_all_state_and_profile_counter_is_used_for_tokens() {
        let state = create_initialized_state();
        state.set_profile_counter(7);

        let created = state.create_profile("Generated".to_string(), None).unwrap();
        assert_eq!(created.token, format!("{}7", PROFILE_TOKEN_PREFIX));
        assert_eq!(state.max_sensor_resolution(), Resolution::new(1920, 1080));

        state.clear();

        assert_eq!(state.profile_count(), 0);
        assert!(state.get_profiles().is_empty());
        assert!(state.get_video_sources().is_empty());
        assert!(state.get_audio_sources().is_empty());
        assert!(state.get_video_source_configurations().is_empty());
        assert!(state.get_video_encoder_configurations().is_empty());
        assert!(state.get_audio_source_configurations().is_empty());
        assert!(state.get_audio_encoder_configurations().is_empty());
    }
}
