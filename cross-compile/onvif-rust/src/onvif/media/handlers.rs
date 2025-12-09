//! Media Service request handlers.
//!
//! This module implements the ONVIF Media Service operation handlers including:
//! - Profile management (GetProfiles, GetProfile, CreateProfile, DeleteProfile)
//! - Video/audio source and configuration management
//! - Stream and snapshot URI generation
//! - Service capabilities

use std::sync::Arc;

use crate::config::{ConfigPersistenceHandle, ConfigRuntime};
use crate::net::ip_utils::external_ip;
use crate::onvif::dispatcher::ServiceHandler;
use crate::onvif::error::{OnvifError, OnvifResult};
use crate::onvif::types::common::{MediaUri, StreamSetup, StreamType, TransportProtocol};
use crate::onvif::types::media::{
    AddAudioEncoderConfiguration, AddAudioEncoderConfigurationResponse,
    AddAudioSourceConfiguration, AddAudioSourceConfigurationResponse, AddVideoEncoderConfiguration,
    AddVideoEncoderConfigurationResponse, AddVideoSourceConfiguration,
    AddVideoSourceConfigurationResponse, CreateProfile, CreateProfileResponse, DeleteProfile,
    DeleteProfileResponse, GetAudioEncoderConfiguration, GetAudioEncoderConfigurationOptions,
    GetAudioEncoderConfigurationOptionsResponse, GetAudioEncoderConfigurationResponse,
    GetAudioEncoderConfigurations, GetAudioEncoderConfigurationsResponse,
    GetAudioSourceConfiguration, GetAudioSourceConfigurationResponse, GetAudioSourceConfigurations,
    GetAudioSourceConfigurationsResponse, GetAudioSources, GetAudioSourcesResponse,
    GetCompatibleAudioEncoderConfigurations, GetCompatibleAudioEncoderConfigurationsResponse,
    GetCompatibleAudioSourceConfigurations, GetCompatibleAudioSourceConfigurationsResponse,
    GetCompatibleVideoEncoderConfigurations, GetCompatibleVideoEncoderConfigurationsResponse,
    GetCompatibleVideoSourceConfigurations, GetCompatibleVideoSourceConfigurationsResponse,
    GetMetadataConfigurations, GetMetadataConfigurationsResponse, GetProfile, GetProfileResponse,
    GetProfiles, GetProfilesResponse, GetServiceCapabilities, GetServiceCapabilitiesResponse,
    GetSnapshotUri, GetSnapshotUriResponse, GetStreamUri, GetStreamUriResponse,
    GetVideoEncoderConfiguration, GetVideoEncoderConfigurationOptions,
    GetVideoEncoderConfigurationOptionsResponse, GetVideoEncoderConfigurationResponse,
    GetVideoEncoderConfigurations, GetVideoEncoderConfigurationsResponse,
    GetVideoSourceConfiguration, GetVideoSourceConfigurationOptions,
    GetVideoSourceConfigurationOptionsResponse, GetVideoSourceConfigurationResponse,
    GetVideoSourceConfigurations, GetVideoSourceConfigurationsResponse, GetVideoSources,
    GetVideoSourcesResponse, MediaServiceCapabilities, ProfileCapabilities,
    RemoveAudioEncoderConfiguration, RemoveAudioEncoderConfigurationResponse,
    RemoveAudioSourceConfiguration, RemoveAudioSourceConfigurationResponse,
    RemoveVideoEncoderConfiguration, RemoveVideoEncoderConfigurationResponse,
    RemoveVideoSourceConfiguration, RemoveVideoSourceConfigurationResponse,
    SetAudioEncoderConfiguration, SetAudioEncoderConfigurationResponse,
    SetAudioSourceConfiguration, SetAudioSourceConfigurationResponse, SetMetadataConfiguration,
    SetMetadataConfigurationResponse, SetVideoEncoderConfiguration,
    SetVideoEncoderConfigurationResponse, SetVideoSourceConfiguration,
    SetVideoSourceConfigurationResponse, StartMulticastStreaming, StartMulticastStreamingResponse,
    StopMulticastStreaming, StopMulticastStreamingResponse, StreamingCapabilities,
};
use crate::platform::Platform;

use super::profile_manager::ProfileManager;
use super::types::{DEFAULT_RTSP_PORT, DEFAULT_SNAPSHOT_PATH, MAX_PROFILES};

// ============================================================================
// MediaService
// ============================================================================

/// ONVIF Media Service.
///
/// Handles Media Service operations including:
/// - Profile management
/// - Video/audio source and configuration management
/// - Stream and snapshot URI generation
pub struct MediaService {
    /// Profile manager for media profiles.
    profile_manager: Arc<ProfileManager>,
    /// Configuration runtime.
    config: Arc<ConfigRuntime>,
    /// Platform abstraction (optional).
    #[allow(dead_code)]
    platform: Option<Arc<dyn Platform>>,
    /// Optional config persistence handle for save requests.
    #[allow(dead_code)]
    persistence: Option<ConfigPersistenceHandle>,
}

impl MediaService {
    /// Create a new Media Service.
    pub fn new() -> Self {
        let config = Arc::new(ConfigRuntime::new(Default::default()));
        Self {
            profile_manager: Arc::new(ProfileManager::with_config(Arc::clone(&config))),
            config,
            platform: None,
            persistence: None,
        }
    }

    /// Create a new Media Service with configuration.
    pub fn with_config(config: Arc<ConfigRuntime>) -> Self {
        Self {
            profile_manager: Arc::new(ProfileManager::with_config(Arc::clone(&config))),
            config,
            platform: None,
            persistence: None,
        }
    }

    /// Create a new Media Service with configuration and platform.
    pub fn with_config_and_platform(
        config: Arc<ConfigRuntime>,
        platform: Arc<dyn Platform>,
    ) -> Self {
        Self {
            profile_manager: Arc::new(ProfileManager::with_config(Arc::clone(&config))),
            config,
            platform: Some(platform),
            persistence: None,
        }
    }

    /// Create a Media Service wired with config persistence.
    pub fn with_config_and_persistence(
        config: Arc<ConfigRuntime>,
        persistence: ConfigPersistenceHandle,
    ) -> Self {
        Self {
            profile_manager: Arc::new(ProfileManager::with_config_and_persistence(
                Arc::clone(&config),
                Some(persistence.clone()),
            )),
            config,
            platform: None,
            persistence: Some(persistence),
        }
    }

    /// Create a new Media Service with a custom profile manager.
    pub fn with_profile_manager(profile_manager: Arc<ProfileManager>) -> Self {
        Self {
            profile_manager,
            config: Arc::new(ConfigRuntime::new(Default::default())),
            platform: None,
            persistence: None,
        }
    }

    /// Get the profile manager reference.
    pub fn profile_manager(&self) -> Arc<ProfileManager> {
        Arc::clone(&self.profile_manager)
    }

    /// Get the base URL for service addresses.
    fn base_url(&self) -> String {
        let address = external_ip(&self.config);
        let port = self.config.get_int("server.port").unwrap_or(80) as u16;
        format!("http://{}:{}", address, port)
    }

    /// Get the RTSP base URL.
    fn rtsp_url(&self) -> String {
        let address = external_ip(&self.config);
        let port = self
            .config
            .get_int("media.rtsp_port")
            .unwrap_or(DEFAULT_RTSP_PORT as i64) as u16;
        format!("rtsp://{}:{}", address, port)
    }

    // ========================================================================
    // Profile Handlers
    // ========================================================================

    /// Handle GetProfiles request.
    ///
    /// Returns all configured media profiles.
    pub fn handle_get_profiles(&self, _request: GetProfiles) -> OnvifResult<GetProfilesResponse> {
        tracing::debug!("GetProfiles request");

        let profiles = self.profile_manager.get_profiles();
        Ok(GetProfilesResponse { profiles })
    }

    /// Handle GetProfile request.
    ///
    /// Returns a specific profile by token.
    pub fn handle_get_profile(&self, request: GetProfile) -> OnvifResult<GetProfileResponse> {
        tracing::debug!("GetProfile request for token: {}", request.profile_token);

        let profile = self.profile_manager.get_profile(&request.profile_token)?;
        Ok(GetProfileResponse { profile })
    }

    /// Handle CreateProfile request.
    ///
    /// Creates a new media profile.
    pub fn handle_create_profile(
        &self,
        request: CreateProfile,
    ) -> OnvifResult<CreateProfileResponse> {
        tracing::debug!("CreateProfile request: name={}", request.name);

        let profile = self
            .profile_manager
            .create_profile(request.name, request.token)?;
        Ok(CreateProfileResponse { profile })
    }

    /// Handle DeleteProfile request.
    ///
    /// Deletes a media profile.
    pub fn handle_delete_profile(
        &self,
        request: DeleteProfile,
    ) -> OnvifResult<DeleteProfileResponse> {
        tracing::debug!("DeleteProfile request for token: {}", request.profile_token);

        self.profile_manager
            .delete_profile(&request.profile_token)?;
        Ok(DeleteProfileResponse {})
    }

    // ========================================================================
    // Video Source Handlers
    // ========================================================================

    /// Handle GetVideoSources request.
    ///
    /// Returns all available video sources.
    pub fn handle_get_video_sources(
        &self,
        _request: GetVideoSources,
    ) -> OnvifResult<GetVideoSourcesResponse> {
        tracing::debug!("GetVideoSources request");

        let video_sources = self.profile_manager.get_video_sources();
        Ok(GetVideoSourcesResponse { video_sources })
    }

    /// Handle GetVideoSourceConfigurations request.
    ///
    /// Returns all video source configurations.
    pub fn handle_get_video_source_configurations(
        &self,
        _request: GetVideoSourceConfigurations,
    ) -> OnvifResult<GetVideoSourceConfigurationsResponse> {
        tracing::debug!("GetVideoSourceConfigurations request");

        let configurations = self.profile_manager.get_video_source_configurations();
        Ok(GetVideoSourceConfigurationsResponse { configurations })
    }

    /// Handle GetVideoSourceConfiguration request.
    ///
    /// Returns a specific video source configuration.
    pub fn handle_get_video_source_configuration(
        &self,
        request: GetVideoSourceConfiguration,
    ) -> OnvifResult<GetVideoSourceConfigurationResponse> {
        tracing::debug!(
            "GetVideoSourceConfiguration request for token: {}",
            request.configuration_token
        );

        let configuration = self
            .profile_manager
            .get_video_source_configuration(&request.configuration_token)?;
        Ok(GetVideoSourceConfigurationResponse { configuration })
    }

    /// Handle SetVideoSourceConfiguration request.
    ///
    /// Updates a video source configuration.
    pub fn handle_set_video_source_configuration(
        &self,
        request: SetVideoSourceConfiguration,
    ) -> OnvifResult<SetVideoSourceConfigurationResponse> {
        tracing::debug!(
            "SetVideoSourceConfiguration request for token: {}",
            request.configuration.token
        );

        self.profile_manager
            .set_video_source_configuration(request.configuration)?;
        Ok(SetVideoSourceConfigurationResponse {})
    }

    /// Handle GetVideoSourceConfigurationOptions request.
    ///
    /// Returns valid options for video source configuration.
    pub fn handle_get_video_source_configuration_options(
        &self,
        _request: GetVideoSourceConfigurationOptions,
    ) -> OnvifResult<GetVideoSourceConfigurationOptionsResponse> {
        tracing::debug!("GetVideoSourceConfigurationOptions request");

        let options = self
            .profile_manager
            .get_video_source_configuration_options();
        Ok(GetVideoSourceConfigurationOptionsResponse { options })
    }

    // ========================================================================
    // Video Encoder Handlers
    // ========================================================================

    /// Handle GetVideoEncoderConfigurations request.
    ///
    /// Returns all video encoder configurations.
    pub fn handle_get_video_encoder_configurations(
        &self,
        _request: GetVideoEncoderConfigurations,
    ) -> OnvifResult<GetVideoEncoderConfigurationsResponse> {
        tracing::debug!("GetVideoEncoderConfigurations request");

        let configurations = self.profile_manager.get_video_encoder_configurations();
        Ok(GetVideoEncoderConfigurationsResponse { configurations })
    }

    /// Handle GetVideoEncoderConfiguration request.
    ///
    /// Returns a specific video encoder configuration.
    pub fn handle_get_video_encoder_configuration(
        &self,
        request: GetVideoEncoderConfiguration,
    ) -> OnvifResult<GetVideoEncoderConfigurationResponse> {
        tracing::debug!(
            "GetVideoEncoderConfiguration request for token: {}",
            request.configuration_token
        );

        let configuration = self
            .profile_manager
            .get_video_encoder_configuration(&request.configuration_token)?;
        Ok(GetVideoEncoderConfigurationResponse { configuration })
    }

    /// Handle SetVideoEncoderConfiguration request.
    ///
    /// Updates a video encoder configuration.
    pub fn handle_set_video_encoder_configuration(
        &self,
        request: SetVideoEncoderConfiguration,
    ) -> OnvifResult<SetVideoEncoderConfigurationResponse> {
        tracing::debug!(
            "SetVideoEncoderConfiguration request for token: {}",
            request.configuration.token
        );

        // Validate configuration values
        super::faults::validate_resolution(
            request.configuration.resolution.width,
            request.configuration.resolution.height,
        )?;
        super::faults::validate_quality(request.configuration.quality)?;

        if let Some(ref rate_control) = request.configuration.rate_control {
            super::faults::validate_frame_rate(rate_control.frame_rate_limit)?;
            super::faults::validate_bitrate(rate_control.bitrate_limit)?;
        }

        self.profile_manager
            .set_video_encoder_configuration(request.configuration)?;
        Ok(SetVideoEncoderConfigurationResponse {})
    }

    /// Handle GetVideoEncoderConfigurationOptions request.
    ///
    /// Returns valid options for video encoder configuration.
    pub fn handle_get_video_encoder_configuration_options(
        &self,
        _request: GetVideoEncoderConfigurationOptions,
    ) -> OnvifResult<GetVideoEncoderConfigurationOptionsResponse> {
        tracing::debug!("GetVideoEncoderConfigurationOptions request");

        let options = self
            .profile_manager
            .get_video_encoder_configuration_options();
        Ok(GetVideoEncoderConfigurationOptionsResponse { options })
    }

    // ========================================================================
    // Audio Source Handlers
    // ========================================================================

    /// Handle GetAudioSources request.
    ///
    /// Returns all available audio sources.
    pub fn handle_get_audio_sources(
        &self,
        _request: GetAudioSources,
    ) -> OnvifResult<GetAudioSourcesResponse> {
        tracing::debug!("GetAudioSources request");

        let audio_sources = self.profile_manager.get_audio_sources();
        Ok(GetAudioSourcesResponse { audio_sources })
    }

    /// Handle GetAudioSourceConfigurations request.
    ///
    /// Returns all audio source configurations.
    pub fn handle_get_audio_source_configurations(
        &self,
        _request: GetAudioSourceConfigurations,
    ) -> OnvifResult<GetAudioSourceConfigurationsResponse> {
        tracing::debug!("GetAudioSourceConfigurations request");

        let configurations = self.profile_manager.get_audio_source_configurations();
        Ok(GetAudioSourceConfigurationsResponse { configurations })
    }

    /// Handle GetAudioSourceConfiguration request.
    ///
    /// Returns a specific audio source configuration.
    pub fn handle_get_audio_source_configuration(
        &self,
        request: GetAudioSourceConfiguration,
    ) -> OnvifResult<GetAudioSourceConfigurationResponse> {
        tracing::debug!(
            "GetAudioSourceConfiguration request for token: {}",
            request.configuration_token
        );

        let configuration = self
            .profile_manager
            .get_audio_source_configuration(&request.configuration_token)?;
        Ok(GetAudioSourceConfigurationResponse { configuration })
    }

    // ========================================================================
    // Audio Encoder Handlers
    // ========================================================================

    /// Handle GetAudioEncoderConfigurations request.
    ///
    /// Returns all audio encoder configurations.
    pub fn handle_get_audio_encoder_configurations(
        &self,
        _request: GetAudioEncoderConfigurations,
    ) -> OnvifResult<GetAudioEncoderConfigurationsResponse> {
        tracing::debug!("GetAudioEncoderConfigurations request");

        let configurations = self.profile_manager.get_audio_encoder_configurations();
        Ok(GetAudioEncoderConfigurationsResponse { configurations })
    }

    /// Handle GetAudioEncoderConfiguration request.
    ///
    /// Returns a specific audio encoder configuration.
    pub fn handle_get_audio_encoder_configuration(
        &self,
        request: GetAudioEncoderConfiguration,
    ) -> OnvifResult<GetAudioEncoderConfigurationResponse> {
        tracing::debug!(
            "GetAudioEncoderConfiguration request for token: {}",
            request.configuration_token
        );

        let configuration = self
            .profile_manager
            .get_audio_encoder_configuration(&request.configuration_token)?;
        Ok(GetAudioEncoderConfigurationResponse { configuration })
    }

    /// Handle SetAudioEncoderConfiguration request.
    ///
    /// Updates an audio encoder configuration.
    pub fn handle_set_audio_encoder_configuration(
        &self,
        request: SetAudioEncoderConfiguration,
    ) -> OnvifResult<SetAudioEncoderConfigurationResponse> {
        tracing::debug!(
            "SetAudioEncoderConfiguration request for token: {}",
            request.configuration.token
        );

        self.profile_manager
            .set_audio_encoder_configuration(request.configuration)?;
        Ok(SetAudioEncoderConfigurationResponse {})
    }

    /// Handle GetAudioEncoderConfigurationOptions request.
    ///
    /// Returns valid options for audio encoder configuration.
    pub fn handle_get_audio_encoder_configuration_options(
        &self,
        _request: GetAudioEncoderConfigurationOptions,
    ) -> OnvifResult<GetAudioEncoderConfigurationOptionsResponse> {
        tracing::debug!("GetAudioEncoderConfigurationOptions request");

        let options = self
            .profile_manager
            .get_audio_encoder_configuration_options();
        Ok(GetAudioEncoderConfigurationOptionsResponse { options })
    }

    // ========================================================================
    // Stream URI Handlers
    // ========================================================================

    /// Handle GetStreamUri request.
    ///
    /// Returns the RTSP URI for a profile.
    pub fn handle_get_stream_uri(
        &self,
        request: GetStreamUri,
    ) -> OnvifResult<GetStreamUriResponse> {
        tracing::debug!(
            "GetStreamUri request for profile: {}",
            request.profile_token
        );

        // Validate profile exists
        let _profile = self.profile_manager.get_profile(&request.profile_token)?;

        // Build RTSP URI based on profile
        let stream_path = self.get_stream_path(&request.profile_token, &request.stream_setup);
        let uri = format!("{}{}", self.rtsp_url(), stream_path);

        Ok(GetStreamUriResponse {
            media_uri: MediaUri {
                uri,
                invalid_after_connect: false,
                invalid_after_reboot: false,
                timeout: "PT60S".to_string(),
            },
        })
    }

    /// Get stream path for a profile.
    fn get_stream_path(&self, profile_token: &str, stream_setup: &StreamSetup) -> String {
        // Determine stream name based on profile
        let stream_name = if profile_token.contains("MainStream") {
            "main"
        } else if profile_token.contains("SubStream") {
            "sub"
        } else {
            "stream"
        };

        // Build path based on transport protocol
        let _protocol = match stream_setup.transport.protocol {
            TransportProtocol::UDP => "udp",
            TransportProtocol::TCP => "tcp",
            TransportProtocol::RTSP => "rtsp",
            TransportProtocol::HTTP => "http",
        };

        // Determine stream type suffix
        let type_suffix = match stream_setup.stream {
            StreamType::RtpUnicast => "",
            StreamType::RtpMulticast => "_multicast",
        };

        format!("/{}{}", stream_name, type_suffix)
    }

    /// Handle GetSnapshotUri request.
    ///
    /// Returns the HTTP URI for a JPEG snapshot.
    pub fn handle_get_snapshot_uri(
        &self,
        request: GetSnapshotUri,
    ) -> OnvifResult<GetSnapshotUriResponse> {
        tracing::debug!(
            "GetSnapshotUri request for profile: {}",
            request.profile_token
        );

        // Validate profile exists
        let _profile = self.profile_manager.get_profile(&request.profile_token)?;

        // Build snapshot URI
        let snapshot_path = self
            .config
            .get_string("media.snapshot_path")
            .unwrap_or_else(|_| DEFAULT_SNAPSHOT_PATH.to_string());

        let uri = format!(
            "{}{}?profile={}",
            self.base_url(),
            snapshot_path,
            request.profile_token
        );

        Ok(GetSnapshotUriResponse {
            media_uri: MediaUri {
                uri,
                invalid_after_connect: false,
                invalid_after_reboot: false,
                timeout: "PT60S".to_string(),
            },
        })
    }

    // ========================================================================
    // Profile Configuration Handlers
    // ========================================================================

    /// Handle AddVideoSourceConfiguration request.
    pub fn handle_add_video_source_configuration(
        &self,
        request: AddVideoSourceConfiguration,
    ) -> OnvifResult<AddVideoSourceConfigurationResponse> {
        tracing::debug!(
            "AddVideoSourceConfiguration: profile={}, config={}",
            request.profile_token,
            request.configuration_token
        );

        self.profile_manager
            .add_video_source_configuration(&request.profile_token, &request.configuration_token)?;
        Ok(AddVideoSourceConfigurationResponse {})
    }

    /// Handle RemoveVideoSourceConfiguration request.
    pub fn handle_remove_video_source_configuration(
        &self,
        request: RemoveVideoSourceConfiguration,
    ) -> OnvifResult<RemoveVideoSourceConfigurationResponse> {
        tracing::debug!(
            "RemoveVideoSourceConfiguration: profile={}",
            request.profile_token
        );

        self.profile_manager
            .remove_video_source_configuration(&request.profile_token)?;
        Ok(RemoveVideoSourceConfigurationResponse {})
    }

    /// Handle AddVideoEncoderConfiguration request.
    pub fn handle_add_video_encoder_configuration(
        &self,
        request: AddVideoEncoderConfiguration,
    ) -> OnvifResult<AddVideoEncoderConfigurationResponse> {
        tracing::debug!(
            "AddVideoEncoderConfiguration: profile={}, config={}",
            request.profile_token,
            request.configuration_token
        );

        self.profile_manager.add_video_encoder_configuration(
            &request.profile_token,
            &request.configuration_token,
        )?;
        Ok(AddVideoEncoderConfigurationResponse {})
    }

    /// Handle RemoveVideoEncoderConfiguration request.
    pub fn handle_remove_video_encoder_configuration(
        &self,
        request: RemoveVideoEncoderConfiguration,
    ) -> OnvifResult<RemoveVideoEncoderConfigurationResponse> {
        tracing::debug!(
            "RemoveVideoEncoderConfiguration: profile={}",
            request.profile_token
        );

        self.profile_manager
            .remove_video_encoder_configuration(&request.profile_token)?;
        Ok(RemoveVideoEncoderConfigurationResponse {})
    }

    /// Handle AddAudioSourceConfiguration request.
    pub fn handle_add_audio_source_configuration(
        &self,
        request: AddAudioSourceConfiguration,
    ) -> OnvifResult<AddAudioSourceConfigurationResponse> {
        tracing::debug!(
            "AddAudioSourceConfiguration: profile={}, config={}",
            request.profile_token,
            request.configuration_token
        );

        self.profile_manager
            .add_audio_source_configuration(&request.profile_token, &request.configuration_token)?;
        Ok(AddAudioSourceConfigurationResponse {})
    }

    /// Handle RemoveAudioSourceConfiguration request.
    pub fn handle_remove_audio_source_configuration(
        &self,
        request: RemoveAudioSourceConfiguration,
    ) -> OnvifResult<RemoveAudioSourceConfigurationResponse> {
        tracing::debug!(
            "RemoveAudioSourceConfiguration: profile={}",
            request.profile_token
        );

        self.profile_manager
            .remove_audio_source_configuration(&request.profile_token)?;
        Ok(RemoveAudioSourceConfigurationResponse {})
    }

    /// Handle AddAudioEncoderConfiguration request.
    pub fn handle_add_audio_encoder_configuration(
        &self,
        request: AddAudioEncoderConfiguration,
    ) -> OnvifResult<AddAudioEncoderConfigurationResponse> {
        tracing::debug!(
            "AddAudioEncoderConfiguration: profile={}, config={}",
            request.profile_token,
            request.configuration_token
        );

        self.profile_manager.add_audio_encoder_configuration(
            &request.profile_token,
            &request.configuration_token,
        )?;
        Ok(AddAudioEncoderConfigurationResponse {})
    }

    /// Handle RemoveAudioEncoderConfiguration request.
    pub fn handle_remove_audio_encoder_configuration(
        &self,
        request: RemoveAudioEncoderConfiguration,
    ) -> OnvifResult<RemoveAudioEncoderConfigurationResponse> {
        tracing::debug!(
            "RemoveAudioEncoderConfiguration: profile={}",
            request.profile_token
        );

        self.profile_manager
            .remove_audio_encoder_configuration(&request.profile_token)?;
        Ok(RemoveAudioEncoderConfigurationResponse {})
    }

    // ========================================================================
    // Service Capabilities Handler
    // ========================================================================

    /// Handle GetServiceCapabilities request.
    ///
    /// Returns Media Service capabilities.
    pub fn handle_get_service_capabilities(
        &self,
        _request: GetServiceCapabilities,
    ) -> OnvifResult<GetServiceCapabilitiesResponse> {
        tracing::debug!("GetServiceCapabilities request");

        Ok(GetServiceCapabilitiesResponse {
            capabilities: MediaServiceCapabilities {
                snapshot_uri: Some(true),
                rotation: Some(false),
                video_source_mode: Some(false),
                osd: Some(false),
                temporary_osd_text: Some(false),
                exi_compression: Some(false),
                profile_capabilities: Some(ProfileCapabilities {
                    maximum_number_of_profiles: Some(MAX_PROFILES as i32),
                }),
                streaming_capabilities: Some(StreamingCapabilities {
                    rtp_multicast: Some(false),
                    rtp_tcp: Some(true),
                    rtp_rtsp_tcp: Some(true),
                    non_aggregate_control: Some(false),
                    no_rtsp_streaming: Some(false),
                }),
            },
        })
    }

    // ========================================================================
    // Compatible Configuration Handlers (FR-002)
    // ========================================================================

    /// Handle GetCompatibleVideoSourceConfigurations request.
    ///
    /// Returns video source configurations compatible with the given profile.
    pub fn handle_get_compatible_video_source_configurations(
        &self,
        request: GetCompatibleVideoSourceConfigurations,
    ) -> OnvifResult<GetCompatibleVideoSourceConfigurationsResponse> {
        tracing::debug!(
            "GetCompatibleVideoSourceConfigurations for profile: {}",
            request.profile_token
        );

        // Verify profile exists
        let _ = self.profile_manager.get_profile(&request.profile_token)?;

        // All video source configurations are compatible with all profiles
        let configurations = self.profile_manager.get_video_source_configurations();

        Ok(GetCompatibleVideoSourceConfigurationsResponse { configurations })
    }

    /// Handle GetCompatibleVideoEncoderConfigurations request.
    ///
    /// Returns video encoder configurations compatible with the given profile.
    pub fn handle_get_compatible_video_encoder_configurations(
        &self,
        request: GetCompatibleVideoEncoderConfigurations,
    ) -> OnvifResult<GetCompatibleVideoEncoderConfigurationsResponse> {
        tracing::debug!(
            "GetCompatibleVideoEncoderConfigurations for profile: {}",
            request.profile_token
        );

        // Verify profile exists
        let _ = self.profile_manager.get_profile(&request.profile_token)?;

        // All video encoder configurations are compatible with all profiles
        let configurations = self.profile_manager.get_video_encoder_configurations();

        Ok(GetCompatibleVideoEncoderConfigurationsResponse { configurations })
    }

    /// Handle GetCompatibleAudioSourceConfigurations request.
    ///
    /// Returns audio source configurations compatible with the given profile.
    pub fn handle_get_compatible_audio_source_configurations(
        &self,
        request: GetCompatibleAudioSourceConfigurations,
    ) -> OnvifResult<GetCompatibleAudioSourceConfigurationsResponse> {
        tracing::debug!(
            "GetCompatibleAudioSourceConfigurations for profile: {}",
            request.profile_token
        );

        // Verify profile exists
        let _ = self.profile_manager.get_profile(&request.profile_token)?;

        // All audio source configurations are compatible with all profiles
        let configurations = self.profile_manager.get_audio_source_configurations();

        Ok(GetCompatibleAudioSourceConfigurationsResponse { configurations })
    }

    /// Handle GetCompatibleAudioEncoderConfigurations request.
    ///
    /// Returns audio encoder configurations compatible with the given profile.
    pub fn handle_get_compatible_audio_encoder_configurations(
        &self,
        request: GetCompatibleAudioEncoderConfigurations,
    ) -> OnvifResult<GetCompatibleAudioEncoderConfigurationsResponse> {
        tracing::debug!(
            "GetCompatibleAudioEncoderConfigurations for profile: {}",
            request.profile_token
        );

        // Verify profile exists
        let _ = self.profile_manager.get_profile(&request.profile_token)?;

        // All audio encoder configurations are compatible with all profiles
        let configurations = self.profile_manager.get_audio_encoder_configurations();

        Ok(GetCompatibleAudioEncoderConfigurationsResponse { configurations })
    }

    // ========================================================================
    // Metadata Configuration Handlers (FR-002)
    // ========================================================================

    /// Handle GetMetadataConfigurations request.
    ///
    /// Returns all metadata configurations.
    pub fn handle_get_metadata_configurations(
        &self,
        _request: GetMetadataConfigurations,
    ) -> OnvifResult<GetMetadataConfigurationsResponse> {
        tracing::debug!("GetMetadataConfigurations request");

        // Return empty list - metadata configurations not supported on this device
        Ok(GetMetadataConfigurationsResponse {
            configurations: vec![],
        })
    }

    /// Handle SetMetadataConfiguration request.
    ///
    /// Sets a metadata configuration.
    pub fn handle_set_metadata_configuration(
        &self,
        request: SetMetadataConfiguration,
    ) -> OnvifResult<SetMetadataConfigurationResponse> {
        tracing::debug!(
            "SetMetadataConfiguration for token: {}",
            request.configuration.token
        );

        // Metadata configurations not supported - return fault
        Err(OnvifError::invalid_arg_val(
            "ter:NoConfig",
            format!(
                "Metadata configuration '{}' not found",
                request.configuration.token
            ),
        ))
    }

    // ========================================================================
    // Multicast Streaming Handlers (FR-002)
    // ========================================================================

    /// Handle StartMulticastStreaming request.
    ///
    /// Starts multicast streaming for the given profile.
    pub fn handle_start_multicast_streaming(
        &self,
        request: StartMulticastStreaming,
    ) -> OnvifResult<StartMulticastStreamingResponse> {
        tracing::debug!(
            "StartMulticastStreaming for profile: {}",
            request.profile_token
        );

        // Verify profile exists
        let _ = self.profile_manager.get_profile(&request.profile_token)?;

        // Multicast streaming not supported - return success but do nothing
        // Per ONVIF spec, this is acceptable if multicast is disabled in capabilities
        tracing::warn!(
            "Multicast streaming requested but not supported for profile: {}",
            request.profile_token
        );

        Ok(StartMulticastStreamingResponse {})
    }

    /// Handle StopMulticastStreaming request.
    ///
    /// Stops multicast streaming for the given profile.
    pub fn handle_stop_multicast_streaming(
        &self,
        request: StopMulticastStreaming,
    ) -> OnvifResult<StopMulticastStreamingResponse> {
        tracing::debug!(
            "StopMulticastStreaming for profile: {}",
            request.profile_token
        );

        // Verify profile exists
        let _ = self.profile_manager.get_profile(&request.profile_token)?;

        // Multicast streaming not supported - return success
        Ok(StopMulticastStreamingResponse {})
    }

    // ========================================================================
    // Audio Source Configuration Handler (FR-002)
    // ========================================================================

    /// Handle SetAudioSourceConfiguration request.
    ///
    /// Sets an audio source configuration.
    pub fn handle_set_audio_source_configuration(
        &self,
        request: SetAudioSourceConfiguration,
    ) -> OnvifResult<SetAudioSourceConfigurationResponse> {
        tracing::debug!(
            "SetAudioSourceConfiguration for token: {}",
            request.configuration.token
        );

        // Audio source configuration is read-only on this device
        // Store it anyway for compliance
        self.profile_manager
            .set_audio_source_configuration(request.configuration)?;

        Ok(SetAudioSourceConfigurationResponse {})
    }
}

impl Default for MediaService {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// ServiceHandler Implementation
// ============================================================================

#[async_trait::async_trait]
impl ServiceHandler for MediaService {
    /// Handle a SOAP operation for the Media Service.
    ///
    /// Routes the SOAP action to the appropriate handler method and returns
    /// the serialized XML response.
    async fn handle_operation(&self, action: &str, body_xml: &str) -> Result<String, OnvifError> {
        tracing::debug!("MediaService handling action: {}", action);

        match action {
            // Profile Operations
            "GetProfiles" => {
                let request: GetProfiles = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_get_profiles(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "GetProfile" => {
                let request: GetProfile = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_get_profile(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "CreateProfile" => {
                let request: CreateProfile = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_create_profile(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "DeleteProfile" => {
                let request: DeleteProfile = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_delete_profile(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            // Video Source Operations
            "GetVideoSources" => {
                let request: GetVideoSources = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_get_video_sources(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "GetVideoSourceConfigurations" => {
                let request: GetVideoSourceConfigurations = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_get_video_source_configurations(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "GetVideoSourceConfiguration" => {
                let request: GetVideoSourceConfiguration = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_get_video_source_configuration(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "SetVideoSourceConfiguration" => {
                let request: SetVideoSourceConfiguration = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_set_video_source_configuration(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "GetVideoSourceConfigurationOptions" => {
                let request: GetVideoSourceConfigurationOptions = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_get_video_source_configuration_options(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "AddVideoSourceConfiguration" => {
                let request: AddVideoSourceConfiguration = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_add_video_source_configuration(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "RemoveVideoSourceConfiguration" => {
                let request: RemoveVideoSourceConfiguration = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_remove_video_source_configuration(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            // Video Encoder Operations
            "GetVideoEncoderConfigurations" => {
                let request: GetVideoEncoderConfigurations = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_get_video_encoder_configurations(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "GetVideoEncoderConfiguration" => {
                let request: GetVideoEncoderConfiguration = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_get_video_encoder_configuration(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "SetVideoEncoderConfiguration" => {
                let request: SetVideoEncoderConfiguration = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_set_video_encoder_configuration(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "GetVideoEncoderConfigurationOptions" => {
                let request: GetVideoEncoderConfigurationOptions =
                    quick_xml::de::from_str(body_xml).map_err(|e| {
                        OnvifError::WellFormed(format!("Invalid request XML: {}", e))
                    })?;
                let response = self.handle_get_video_encoder_configuration_options(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "AddVideoEncoderConfiguration" => {
                let request: AddVideoEncoderConfiguration = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_add_video_encoder_configuration(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "RemoveVideoEncoderConfiguration" => {
                let request: RemoveVideoEncoderConfiguration = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_remove_video_encoder_configuration(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            // Audio Source Operations
            "GetAudioSources" => {
                let request: GetAudioSources = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_get_audio_sources(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "GetAudioSourceConfigurations" => {
                let request: GetAudioSourceConfigurations = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_get_audio_source_configurations(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "GetAudioSourceConfiguration" => {
                let request: GetAudioSourceConfiguration = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_get_audio_source_configuration(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "AddAudioSourceConfiguration" => {
                let request: AddAudioSourceConfiguration = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_add_audio_source_configuration(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "RemoveAudioSourceConfiguration" => {
                let request: RemoveAudioSourceConfiguration = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_remove_audio_source_configuration(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            // Audio Encoder Operations
            "GetAudioEncoderConfigurations" => {
                let request: GetAudioEncoderConfigurations = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_get_audio_encoder_configurations(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "GetAudioEncoderConfiguration" => {
                let request: GetAudioEncoderConfiguration = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_get_audio_encoder_configuration(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "GetAudioEncoderConfigurationOptions" => {
                let request: GetAudioEncoderConfigurationOptions =
                    quick_xml::de::from_str(body_xml).map_err(|e| {
                        OnvifError::WellFormed(format!("Invalid request XML: {}", e))
                    })?;
                let response = self.handle_get_audio_encoder_configuration_options(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "SetAudioEncoderConfiguration" => {
                let request: SetAudioEncoderConfiguration = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_set_audio_encoder_configuration(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "AddAudioEncoderConfiguration" => {
                let request: AddAudioEncoderConfiguration = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_add_audio_encoder_configuration(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "RemoveAudioEncoderConfiguration" => {
                let request: RemoveAudioEncoderConfiguration = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_remove_audio_encoder_configuration(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            // Stream URI Operations
            "GetStreamUri" => {
                let request: GetStreamUri = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_get_stream_uri(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "GetSnapshotUri" => {
                let request: GetSnapshotUri = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_get_snapshot_uri(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            // Service Capabilities
            "GetServiceCapabilities" => {
                let request: GetServiceCapabilities = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_get_service_capabilities(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            // Compatible Configurations (FR-002)
            "GetCompatibleVideoSourceConfigurations" => {
                let request: GetCompatibleVideoSourceConfigurations =
                    quick_xml::de::from_str(body_xml).map_err(|e| {
                        OnvifError::WellFormed(format!("Invalid request XML: {}", e))
                    })?;
                let response = self.handle_get_compatible_video_source_configurations(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "GetCompatibleVideoEncoderConfigurations" => {
                let request: GetCompatibleVideoEncoderConfigurations =
                    quick_xml::de::from_str(body_xml).map_err(|e| {
                        OnvifError::WellFormed(format!("Invalid request XML: {}", e))
                    })?;
                let response = self.handle_get_compatible_video_encoder_configurations(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "GetCompatibleAudioSourceConfigurations" => {
                let request: GetCompatibleAudioSourceConfigurations =
                    quick_xml::de::from_str(body_xml).map_err(|e| {
                        OnvifError::WellFormed(format!("Invalid request XML: {}", e))
                    })?;
                let response = self.handle_get_compatible_audio_source_configurations(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "GetCompatibleAudioEncoderConfigurations" => {
                let request: GetCompatibleAudioEncoderConfigurations =
                    quick_xml::de::from_str(body_xml).map_err(|e| {
                        OnvifError::WellFormed(format!("Invalid request XML: {}", e))
                    })?;
                let response = self.handle_get_compatible_audio_encoder_configurations(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            // Metadata Configuration (FR-002)
            "GetMetadataConfigurations" => {
                let request: GetMetadataConfigurations = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_get_metadata_configurations(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "SetMetadataConfiguration" => {
                let request: SetMetadataConfiguration = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_set_metadata_configuration(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            // Multicast Streaming (FR-002)
            "StartMulticastStreaming" => {
                let request: StartMulticastStreaming = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_start_multicast_streaming(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "StopMulticastStreaming" => {
                let request: StopMulticastStreaming = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_stop_multicast_streaming(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            // Audio Source Configuration (FR-002)
            "SetAudioSourceConfiguration" => {
                let request: SetAudioSourceConfiguration = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_set_audio_source_configuration(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            // Unknown action
            _ => Err(OnvifError::ActionNotSupported(action.to_string())),
        }
    }

    /// Get the service name.
    fn service_name(&self) -> &str {
        "Media"
    }

    /// Get the list of supported actions.
    fn supported_actions(&self) -> Vec<&str> {
        vec![
            // Profile Operations
            "GetProfiles",
            "GetProfile",
            "CreateProfile",
            "DeleteProfile",
            // Video Source Operations
            "GetVideoSources",
            "GetVideoSourceConfigurations",
            "GetVideoSourceConfiguration",
            "SetVideoSourceConfiguration",
            "GetVideoSourceConfigurationOptions",
            "AddVideoSourceConfiguration",
            "RemoveVideoSourceConfiguration",
            // Video Encoder Operations
            "GetVideoEncoderConfigurations",
            "GetVideoEncoderConfiguration",
            "SetVideoEncoderConfiguration",
            "GetVideoEncoderConfigurationOptions",
            "AddVideoEncoderConfiguration",
            "RemoveVideoEncoderConfiguration",
            // Audio Source Operations
            "GetAudioSources",
            "GetAudioSourceConfigurations",
            "GetAudioSourceConfiguration",
            "SetAudioSourceConfiguration",
            "AddAudioSourceConfiguration",
            "RemoveAudioSourceConfiguration",
            // Audio Encoder Operations
            "GetAudioEncoderConfigurations",
            "GetAudioEncoderConfiguration",
            "GetAudioEncoderConfigurationOptions",
            "SetAudioEncoderConfiguration",
            "AddAudioEncoderConfiguration",
            "RemoveAudioEncoderConfiguration",
            // Stream URI Operations
            "GetStreamUri",
            "GetSnapshotUri",
            // Service Capabilities
            "GetServiceCapabilities",
            // Compatible Configurations (FR-002)
            "GetCompatibleVideoSourceConfigurations",
            "GetCompatibleVideoEncoderConfigurations",
            "GetCompatibleAudioSourceConfigurations",
            "GetCompatibleAudioEncoderConfigurations",
            // Metadata Configuration (FR-002)
            "GetMetadataConfigurations",
            "SetMetadataConfiguration",
            // Multicast Streaming (FR-002)
            "StartMulticastStreaming",
            "StopMulticastStreaming",
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_media_service_new() {
        let service = MediaService::new();
        let profiles = service.handle_get_profiles(GetProfiles {}).unwrap();
        assert_eq!(profiles.profiles.len(), 2);
    }

    #[test]
    fn test_get_profile() {
        let service = MediaService::new();
        let result = service.handle_get_profile(GetProfile {
            profile_token: "Profile_MainStream".to_string(),
        });
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.profile.name, "MainStream");
    }

    #[test]
    fn test_get_profile_not_found() {
        let service = MediaService::new();
        let result = service.handle_get_profile(GetProfile {
            profile_token: "NonExistent".to_string(),
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_create_profile() {
        let service = MediaService::new();
        let result = service.handle_create_profile(CreateProfile {
            name: "TestProfile".to_string(),
            token: None,
        });
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.profile.name, "TestProfile");
    }

    #[test]
    fn test_delete_profile() {
        let service = MediaService::new();

        // Create a profile first
        let create_result = service
            .handle_create_profile(CreateProfile {
                name: "ToDelete".to_string(),
                token: Some("ToDeleteToken".to_string()),
            })
            .unwrap();

        // Delete it
        let result = service.handle_delete_profile(DeleteProfile {
            profile_token: create_result.profile.token,
        });
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_video_sources() {
        let service = MediaService::new();
        let result = service.handle_get_video_sources(GetVideoSources {});
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(!response.video_sources.is_empty());
    }

    #[test]
    fn test_get_audio_sources() {
        let service = MediaService::new();
        let result = service.handle_get_audio_sources(GetAudioSources {});
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(!response.audio_sources.is_empty());
    }

    #[test]
    fn test_get_video_encoder_configurations() {
        let service = MediaService::new();
        let result =
            service.handle_get_video_encoder_configurations(GetVideoEncoderConfigurations {});
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(!response.configurations.is_empty());
    }

    #[test]
    fn test_get_stream_uri() {
        let service = MediaService::new();
        let result = service.handle_get_stream_uri(GetStreamUri {
            stream_setup: StreamSetup {
                stream: StreamType::RtpUnicast,
                transport: crate::onvif::types::common::Transport {
                    protocol: TransportProtocol::RTSP,
                    tunnel: None,
                },
            },
            profile_token: "Profile_MainStream".to_string(),
        });
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.media_uri.uri.contains("rtsp://"));
        assert!(response.media_uri.uri.contains("/main"));
    }

    #[test]
    fn test_get_snapshot_uri() {
        let service = MediaService::new();
        let result = service.handle_get_snapshot_uri(GetSnapshotUri {
            profile_token: "Profile_MainStream".to_string(),
        });
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.media_uri.uri.contains("http://"));
        assert!(response.media_uri.uri.contains("snapshot"));
    }

    #[test]
    fn test_get_service_capabilities() {
        let service = MediaService::new();
        let result = service.handle_get_service_capabilities(GetServiceCapabilities {});
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(
            response
                .capabilities
                .profile_capabilities
                .as_ref()
                .unwrap()
                .maximum_number_of_profiles,
            Some(MAX_PROFILES as i32)
        );
    }

    // ========================================================================
    // Error Path Tests
    // ========================================================================

    #[test]
    fn test_get_profile_invalid_token_returns_no_profile_fault() {
        let service = MediaService::new();
        let result = service.handle_get_profile(GetProfile {
            profile_token: "InvalidToken123".to_string(),
        });
        assert!(result.is_err());
        let err = result.unwrap_err();
        // Verify it's a NoProfile fault
        match err {
            OnvifError::InvalidArgVal { subcode, .. } => {
                assert!(subcode.contains("NoProfile"));
            }
            _ => panic!("Expected InvalidArgVal with NoProfile subcode"),
        }
    }

    #[test]
    fn test_delete_profile_not_found() {
        let service = MediaService::new();
        let result = service.handle_delete_profile(DeleteProfile {
            profile_token: "NonExistent".to_string(),
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_delete_fixed_profile_fails() {
        let service = MediaService::new();
        // MainStream is a fixed profile
        let result = service.handle_delete_profile(DeleteProfile {
            profile_token: "Profile_MainStream".to_string(),
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_get_video_source_configuration_not_found() {
        let service = MediaService::new();
        let result = service.handle_get_video_source_configuration(GetVideoSourceConfiguration {
            configuration_token: "NonExistent".to_string(),
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_get_video_encoder_configuration_not_found() {
        let service = MediaService::new();
        let result = service.handle_get_video_encoder_configuration(GetVideoEncoderConfiguration {
            configuration_token: "NonExistent".to_string(),
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_get_audio_source_configuration_not_found() {
        let service = MediaService::new();
        let result = service.handle_get_audio_source_configuration(GetAudioSourceConfiguration {
            configuration_token: "NonExistent".to_string(),
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_get_audio_encoder_configuration_not_found() {
        let service = MediaService::new();
        let result = service.handle_get_audio_encoder_configuration(GetAudioEncoderConfiguration {
            configuration_token: "NonExistent".to_string(),
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_get_stream_uri_invalid_profile() {
        let service = MediaService::new();
        let result = service.handle_get_stream_uri(GetStreamUri {
            stream_setup: StreamSetup {
                stream: StreamType::RtpUnicast,
                transport: crate::onvif::types::common::Transport {
                    protocol: TransportProtocol::RTSP,
                    tunnel: None,
                },
            },
            profile_token: "NonExistent".to_string(),
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_get_snapshot_uri_invalid_profile() {
        let service = MediaService::new();
        let result = service.handle_get_snapshot_uri(GetSnapshotUri {
            profile_token: "NonExistent".to_string(),
        });
        assert!(result.is_err());
    }

    // ========================================================================
    // Edge Case Tests
    // ========================================================================

    #[test]
    fn test_create_profile_with_custom_token() {
        let service = MediaService::new();
        let result = service.handle_create_profile(CreateProfile {
            name: "CustomTokenProfile".to_string(),
            token: Some("MyCustomToken".to_string()),
        });
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.profile.token, "MyCustomToken");
        assert_eq!(response.profile.name, "CustomTokenProfile");
    }

    #[test]
    fn test_create_profile_duplicate_token_fails() {
        let service = MediaService::new();
        // Create first profile
        let _ = service.handle_create_profile(CreateProfile {
            name: "First".to_string(),
            token: Some("DuplicateToken".to_string()),
        });
        // Try to create second with same token
        let result = service.handle_create_profile(CreateProfile {
            name: "Second".to_string(),
            token: Some("DuplicateToken".to_string()),
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_create_profile_empty_name() {
        let service = MediaService::new();
        let result = service.handle_create_profile(CreateProfile {
            name: "".to_string(),
            token: None,
        });
        // Empty name should fail validation
        assert!(result.is_err());
    }

    #[test]
    fn test_get_profiles_returns_default_profiles() {
        let service = MediaService::new();
        let result = service.handle_get_profiles(GetProfiles {});
        assert!(result.is_ok());
        let response = result.unwrap();
        // Should have MainStream and SubStream
        let names: Vec<&str> = response.profiles.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"MainStream"));
        assert!(names.contains(&"SubStream"));
    }

    #[test]
    fn test_get_stream_uri_multicast() {
        let service = MediaService::new();
        let result = service.handle_get_stream_uri(GetStreamUri {
            stream_setup: StreamSetup {
                stream: StreamType::RtpMulticast,
                transport: crate::onvif::types::common::Transport {
                    protocol: TransportProtocol::UDP,
                    tunnel: None,
                },
            },
            profile_token: "Profile_MainStream".to_string(),
        });
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.media_uri.uri.contains("_multicast"));
    }

    #[test]
    fn test_get_stream_uri_sub_stream() {
        let service = MediaService::new();
        let result = service.handle_get_stream_uri(GetStreamUri {
            stream_setup: StreamSetup {
                stream: StreamType::RtpUnicast,
                transport: crate::onvif::types::common::Transport {
                    protocol: TransportProtocol::RTSP,
                    tunnel: None,
                },
            },
            profile_token: "Profile_SubStream".to_string(),
        });
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.media_uri.uri.contains("/sub"));
    }

    // ========================================================================
    // Configuration Tests
    // ========================================================================

    #[test]
    fn test_get_video_source_configurations() {
        let service = MediaService::new();
        let result =
            service.handle_get_video_source_configurations(GetVideoSourceConfigurations {});
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(!response.configurations.is_empty());
    }

    #[test]
    fn test_get_video_source_configuration() {
        let service = MediaService::new();
        let result = service.handle_get_video_source_configuration(GetVideoSourceConfiguration {
            configuration_token: "VideoSourceConfig_0".to_string(),
        });
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_audio_source_configurations() {
        let service = MediaService::new();
        let result =
            service.handle_get_audio_source_configurations(GetAudioSourceConfigurations {});
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(!response.configurations.is_empty());
    }

    #[test]
    fn test_get_audio_encoder_configurations() {
        let service = MediaService::new();
        let result =
            service.handle_get_audio_encoder_configurations(GetAudioEncoderConfigurations {});
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(!response.configurations.is_empty());
    }

    #[test]
    fn test_get_video_source_configuration_options() {
        let service = MediaService::new();
        let result = service.handle_get_video_source_configuration_options(
            GetVideoSourceConfigurationOptions {
                configuration_token: None,
                profile_token: None,
            },
        );
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.options.maximum_number_of_profiles.is_some());
    }

    #[test]
    fn test_get_video_encoder_configuration_options() {
        let service = MediaService::new();
        let result = service.handle_get_video_encoder_configuration_options(
            GetVideoEncoderConfigurationOptions {
                configuration_token: None,
                profile_token: None,
            },
        );
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.options.h264.is_some());
        assert!(response.options.jpeg.is_some());
    }

    #[test]
    fn test_get_audio_encoder_configuration_options() {
        let service = MediaService::new();
        let result = service.handle_get_audio_encoder_configuration_options(
            GetAudioEncoderConfigurationOptions {
                configuration_token: None,
                profile_token: None,
            },
        );
        assert!(result.is_ok());
    }

    // ========================================================================
    // Add/Remove Configuration Tests
    // ========================================================================

    #[test]
    fn test_add_video_source_configuration() {
        let service = MediaService::new();
        // Create a new profile without configurations
        let profile = service
            .handle_create_profile(CreateProfile {
                name: "EmptyProfile".to_string(),
                token: Some("EmptyProfileToken".to_string()),
            })
            .unwrap();

        let result = service.handle_add_video_source_configuration(AddVideoSourceConfiguration {
            profile_token: profile.profile.token,
            configuration_token: "VideoSourceConfig_0".to_string(),
        });
        assert!(result.is_ok());
    }

    #[test]
    fn test_add_video_encoder_configuration() {
        let service = MediaService::new();
        let profile = service
            .handle_create_profile(CreateProfile {
                name: "TestEncoderProfile".to_string(),
                token: Some("TestEncoderToken".to_string()),
            })
            .unwrap();

        let result = service.handle_add_video_encoder_configuration(AddVideoEncoderConfiguration {
            profile_token: profile.profile.token,
            configuration_token: "VideoEncoderConfig_0".to_string(),
        });
        assert!(result.is_ok());
    }

    #[test]
    fn test_remove_video_source_configuration() {
        let service = MediaService::new();
        // Create profile and add config first
        let profile = service
            .handle_create_profile(CreateProfile {
                name: "ToRemoveVSC".to_string(),
                token: Some("ToRemoveVSCToken".to_string()),
            })
            .unwrap();

        let _ = service.handle_add_video_source_configuration(AddVideoSourceConfiguration {
            profile_token: profile.profile.token.clone(),
            configuration_token: "VideoSourceConfig_0".to_string(),
        });

        let result =
            service.handle_remove_video_source_configuration(RemoveVideoSourceConfiguration {
                profile_token: profile.profile.token,
            });
        assert!(result.is_ok());
    }

    #[test]
    fn test_add_audio_source_configuration() {
        let service = MediaService::new();
        let profile = service
            .handle_create_profile(CreateProfile {
                name: "AudioTestProfile".to_string(),
                token: Some("AudioTestToken".to_string()),
            })
            .unwrap();

        let result = service.handle_add_audio_source_configuration(AddAudioSourceConfiguration {
            profile_token: profile.profile.token,
            configuration_token: "AudioSourceConfig_0".to_string(),
        });
        assert!(result.is_ok());
    }

    #[test]
    fn test_add_audio_encoder_configuration() {
        let service = MediaService::new();
        let profile = service
            .handle_create_profile(CreateProfile {
                name: "AudioEncProfile".to_string(),
                token: Some("AudioEncToken".to_string()),
            })
            .unwrap();

        let result = service.handle_add_audio_encoder_configuration(AddAudioEncoderConfiguration {
            profile_token: profile.profile.token,
            configuration_token: "AudioEncoderConfig_0".to_string(),
        });
        assert!(result.is_ok());
    }

    // ========================================================================
    // Profile Manager State Tests
    // ========================================================================

    #[test]
    fn test_profile_count_after_create_delete() {
        let service = MediaService::new();
        let initial_count = service
            .handle_get_profiles(GetProfiles {})
            .unwrap()
            .profiles
            .len();

        // Create a profile
        let profile = service
            .handle_create_profile(CreateProfile {
                name: "Temporary".to_string(),
                token: Some("TempToken".to_string()),
            })
            .unwrap();

        let after_create = service
            .handle_get_profiles(GetProfiles {})
            .unwrap()
            .profiles
            .len();
        assert_eq!(after_create, initial_count + 1);

        // Delete the profile
        let _ = service.handle_delete_profile(DeleteProfile {
            profile_token: profile.profile.token,
        });

        let after_delete = service
            .handle_get_profiles(GetProfiles {})
            .unwrap()
            .profiles
            .len();
        assert_eq!(after_delete, initial_count);
    }

    #[test]
    fn test_default_profile_has_video_config() {
        let service = MediaService::new();
        let result = service.handle_get_profile(GetProfile {
            profile_token: "Profile_MainStream".to_string(),
        });
        assert!(result.is_ok());
        let profile = result.unwrap().profile;
        assert!(profile.video_source_configuration.is_some());
        assert!(profile.video_encoder_configuration.is_some());
    }

    #[test]
    fn test_default_profile_has_audio_config() {
        let service = MediaService::new();
        let result = service.handle_get_profile(GetProfile {
            profile_token: "Profile_MainStream".to_string(),
        });
        assert!(result.is_ok());
        let profile = result.unwrap().profile;
        assert!(profile.audio_source_configuration.is_some());
        assert!(profile.audio_encoder_configuration.is_some());
    }

    #[test]
    fn test_main_stream_has_higher_resolution_than_sub() {
        let service = MediaService::new();
        let main = service
            .handle_get_profile(GetProfile {
                profile_token: "Profile_MainStream".to_string(),
            })
            .unwrap()
            .profile;
        let sub = service
            .handle_get_profile(GetProfile {
                profile_token: "Profile_SubStream".to_string(),
            })
            .unwrap()
            .profile;

        let main_res = main
            .video_encoder_configuration
            .as_ref()
            .unwrap()
            .resolution
            .clone();
        let sub_res = sub
            .video_encoder_configuration
            .as_ref()
            .unwrap()
            .resolution
            .clone();

        assert!(main_res.width > sub_res.width);
        assert!(main_res.height > sub_res.height);
    }

    // ========================================================================
    // FR-002 Compatible Configuration Tests
    // ========================================================================

    #[test]
    fn test_get_compatible_video_source_configurations() {
        let service = MediaService::new();
        let result = service.handle_get_compatible_video_source_configurations(
            GetCompatibleVideoSourceConfigurations {
                profile_token: "Profile_MainStream".to_string(),
            },
        );
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(!response.configurations.is_empty());
    }

    #[test]
    fn test_get_compatible_video_source_configurations_invalid_profile() {
        let service = MediaService::new();
        let result = service.handle_get_compatible_video_source_configurations(
            GetCompatibleVideoSourceConfigurations {
                profile_token: "NonExistent".to_string(),
            },
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_get_compatible_video_encoder_configurations() {
        let service = MediaService::new();
        let result = service.handle_get_compatible_video_encoder_configurations(
            GetCompatibleVideoEncoderConfigurations {
                profile_token: "Profile_MainStream".to_string(),
            },
        );
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(!response.configurations.is_empty());
    }

    #[test]
    fn test_get_compatible_audio_source_configurations() {
        let service = MediaService::new();
        let result = service.handle_get_compatible_audio_source_configurations(
            GetCompatibleAudioSourceConfigurations {
                profile_token: "Profile_MainStream".to_string(),
            },
        );
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(!response.configurations.is_empty());
    }

    #[test]
    fn test_get_compatible_audio_encoder_configurations() {
        let service = MediaService::new();
        let result = service.handle_get_compatible_audio_encoder_configurations(
            GetCompatibleAudioEncoderConfigurations {
                profile_token: "Profile_MainStream".to_string(),
            },
        );
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(!response.configurations.is_empty());
    }

    // ========================================================================
    // FR-002 Metadata Configuration Tests
    // ========================================================================

    #[test]
    fn test_get_metadata_configurations_returns_empty() {
        let service = MediaService::new();
        let result = service.handle_get_metadata_configurations(GetMetadataConfigurations {});
        assert!(result.is_ok());
        let response = result.unwrap();
        // Device doesn't support metadata configurations
        assert!(response.configurations.is_empty());
    }

    #[test]
    fn test_set_metadata_configuration_returns_no_config() {
        use crate::onvif::types::media::{MetadataConfiguration, PtzFilter};

        let service = MediaService::new();
        let result = service.handle_set_metadata_configuration(SetMetadataConfiguration {
            configuration: MetadataConfiguration {
                token: "NonExistent".to_string(),
                name: "Test".to_string(),
                use_count: 0,
                ptz_status: Some(PtzFilter {
                    status: false,
                    position: false,
                }),
                analytics: None,
                multicast: None,
                session_timeout: "PT60S".to_string(),
            },
            force_persistence: None,
        });
        assert!(result.is_err());
    }

    // ========================================================================
    // FR-002 Multicast Streaming Tests
    // ========================================================================

    #[test]
    fn test_start_multicast_streaming_valid_profile() {
        let service = MediaService::new();
        let result = service.handle_start_multicast_streaming(StartMulticastStreaming {
            profile_token: "Profile_MainStream".to_string(),
        });
        // Should succeed even though multicast is not supported (per ONVIF spec)
        assert!(result.is_ok());
    }

    #[test]
    fn test_start_multicast_streaming_invalid_profile() {
        let service = MediaService::new();
        let result = service.handle_start_multicast_streaming(StartMulticastStreaming {
            profile_token: "NonExistent".to_string(),
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_stop_multicast_streaming_valid_profile() {
        let service = MediaService::new();
        let result = service.handle_stop_multicast_streaming(StopMulticastStreaming {
            profile_token: "Profile_MainStream".to_string(),
        });
        assert!(result.is_ok());
    }

    #[test]
    fn test_stop_multicast_streaming_invalid_profile() {
        let service = MediaService::new();
        let result = service.handle_stop_multicast_streaming(StopMulticastStreaming {
            profile_token: "NonExistent".to_string(),
        });
        assert!(result.is_err());
    }

    // ========================================================================
    // FR-002 SetAudioSourceConfiguration Test
    // ========================================================================

    #[test]
    fn test_set_audio_source_configuration() {
        use crate::onvif::types::common::AudioSourceConfiguration;

        let service = MediaService::new();
        let result = service.handle_set_audio_source_configuration(SetAudioSourceConfiguration {
            configuration: AudioSourceConfiguration {
                token: "AudioSourceConfig_0".to_string(),
                name: "AudioSourceConfig".to_string(),
                use_count: 1,
                source_token: "AudioSource_0".to_string(),
            },
            force_persistence: None,
        });
        assert!(result.is_ok());
    }
}
