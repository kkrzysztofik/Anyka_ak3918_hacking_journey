//! Media Service implementation.
//!
//! ONVIF Media Service implementation.

use std::sync::Arc;

use crate::config::{ConfigRuntime, PersistenceHandle, ProfileStorage};
use crate::onvif::dispatcher::ServiceHandler;
use crate::onvif::error::{OnvifError, OnvifResult};
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
    GetVideoSourcesResponse, RemoveAudioEncoderConfiguration,
    RemoveAudioEncoderConfigurationResponse, RemoveAudioSourceConfiguration,
    RemoveAudioSourceConfigurationResponse, RemoveVideoEncoderConfiguration,
    RemoveVideoEncoderConfigurationResponse, RemoveVideoSourceConfiguration,
    RemoveVideoSourceConfigurationResponse, SetAudioEncoderConfiguration,
    SetAudioEncoderConfigurationResponse, SetAudioSourceConfiguration,
    SetAudioSourceConfigurationResponse, SetMetadataConfiguration,
    SetMetadataConfigurationResponse, SetVideoEncoderConfiguration,
    SetVideoEncoderConfigurationResponse, SetVideoSourceConfiguration,
    SetVideoSourceConfigurationResponse, StartMulticastStreaming, StartMulticastStreamingResponse,
    StopMulticastStreaming, StopMulticastStreamingResponse,
};
use crate::platform::{Platform, Resolution};

use super::ProfileManager;
use super::ops::audio as audio_ops;
use super::ops::capabilities as capability_ops;
use super::ops::profiles as profile_ops;
use super::ops::streaming as streaming_ops;
use super::ops::video_encoders as video_encoder_ops;
use super::ops::video_sources as video_source_ops;
use super::types::DEFAULT_RTSP_PORT;

/// ONVIF Media Service.
pub struct MediaService {
    /// Profile manager for media profiles.
    profile_manager: Arc<ProfileManager>,
    /// Configuration runtime.
    config: Arc<ConfigRuntime>,
    /// Platform abstraction (optional).
    platform: Option<Arc<dyn Platform>>,
}

impl MediaService {
    /// Create a new Media Service with default configuration (for tests).
    pub fn new() -> Self {
        let config = Arc::new(ConfigRuntime::new(Default::default()));
        Self {
            profile_manager: Arc::new(ProfileManager::with_config(Arc::clone(&config))),
            config,
            platform: None,
        }
    }

    /// Create a new Media Service with configuration (no persistence).
    pub fn with_config(config: Arc<ConfigRuntime>) -> Self {
        Self {
            profile_manager: Arc::new(ProfileManager::with_config(Arc::clone(&config))),
            config,
            platform: None,
        }
    }

    /// Create a Media Service with profile storage and optional platform.
    pub fn with_storage(
        config: Arc<ConfigRuntime>,
        profile_storage: Arc<ProfileStorage>,
        platform: Option<Arc<dyn Platform>>,
    ) -> Self {
        Self::with_storage_and_persistence(config, profile_storage, platform, None)
    }

    /// Create a Media Service with profile storage, optional platform, and an
    /// optional debounced persistence handle for off-executor profile saves.
    pub fn with_storage_and_persistence(
        config: Arc<ConfigRuntime>,
        profile_storage: Arc<ProfileStorage>,
        platform: Option<Arc<dyn Platform>>,
        persistence: Option<PersistenceHandle>,
    ) -> Self {
        let max_res = platform
            .as_ref()
            .and_then(|p| p.max_sensor_resolution().ok())
            .unwrap_or(Resolution::new(1920, 1080));

        let pm = ProfileManager::with_storage_and_persistence(
            Arc::clone(&config),
            profile_storage,
            max_res,
            persistence,
        );

        Self {
            profile_manager: Arc::new(pm),
            config,
            platform,
        }
    }

    /// Create a new Media Service with a custom profile manager (for tests).
    // TODO: Uses a default ConfigRuntime — the config from the ProfileManager is not
    // reused here. This is fine for tests but production code should use `with_config`
    // or `with_platform` which wire up the real config.
    pub fn with_profile_manager(profile_manager: Arc<ProfileManager>) -> Self {
        Self {
            profile_manager,
            config: Arc::new(ConfigRuntime::new(Default::default())),
            platform: None,
        }
    }

    /// Get the profile manager reference.
    pub fn profile_manager(&self) -> Arc<ProfileManager> {
        Arc::clone(&self.profile_manager)
    }

    /// Get the base URL for service addresses.
    #[allow(dead_code)]
    fn base_url(&self) -> String {
        let address = crate::platform::external_ip(&self.config);
        let port = self.config.read().server.port;
        format!("http://{}:{}", address, port)
    }

    /// Get the RTSP base URL.
    #[allow(dead_code)]
    fn rtsp_url(&self) -> String {
        let address = crate::platform::external_ip(&self.config);
        let port = {
            let p = self.config.read().media.rtsp_port;
            if p == 0 { DEFAULT_RTSP_PORT } else { p }
        };
        format!("rtsp://{}:{}", address, port)
    }

    // ========================================================================
    // Profile Handlers (delegate to ops::profiles)
    // ========================================================================

    /// Handle GetProfiles request.
    pub fn handle_get_profiles(&self, _request: GetProfiles) -> OnvifResult<GetProfilesResponse> {
        profile_ops::get_profiles(&self.profile_manager)
    }

    /// Handle GetProfile request.
    pub fn handle_get_profile(&self, request: GetProfile) -> OnvifResult<GetProfileResponse> {
        profile_ops::get_profile(&self.profile_manager, request)
    }

    /// Handle CreateProfile request.
    pub fn handle_create_profile(
        &self,
        request: CreateProfile,
    ) -> OnvifResult<CreateProfileResponse> {
        profile_ops::create_profile(&self.profile_manager, request)
    }

    /// Handle DeleteProfile request.
    pub fn handle_delete_profile(
        &self,
        request: DeleteProfile,
    ) -> OnvifResult<DeleteProfileResponse> {
        profile_ops::delete_profile(&self.profile_manager, request)
    }

    // ========================================================================
    // Video Source Handlers (delegate to ops::video_sources)
    // ========================================================================

    /// Handle GetVideoSources request.
    pub fn handle_get_video_sources(
        &self,
        _request: GetVideoSources,
    ) -> OnvifResult<GetVideoSourcesResponse> {
        video_source_ops::get_video_sources(&self.profile_manager)
    }

    /// Handle GetVideoSourceConfigurations request.
    pub fn handle_get_video_source_configurations(
        &self,
        _request: GetVideoSourceConfigurations,
    ) -> OnvifResult<GetVideoSourceConfigurationsResponse> {
        video_source_ops::get_video_source_configurations(&self.profile_manager)
    }

    /// Handle GetVideoSourceConfiguration request.
    pub fn handle_get_video_source_configuration(
        &self,
        request: GetVideoSourceConfiguration,
    ) -> OnvifResult<GetVideoSourceConfigurationResponse> {
        video_source_ops::get_video_source_configuration(&self.profile_manager, request)
    }

    /// Handle SetVideoSourceConfiguration request.
    pub fn handle_set_video_source_configuration(
        &self,
        request: SetVideoSourceConfiguration,
    ) -> OnvifResult<SetVideoSourceConfigurationResponse> {
        video_source_ops::set_video_source_configuration(&self.profile_manager, request)
    }

    /// Handle GetVideoSourceConfigurationOptions request.
    // TODO: Request tokens (configuration_token, profile_token) are intentionally ignored.
    // Single-profile camera — options are identical for all video source configurations.
    pub fn handle_get_video_source_configuration_options(
        &self,
        _request: GetVideoSourceConfigurationOptions,
    ) -> OnvifResult<GetVideoSourceConfigurationOptionsResponse> {
        video_source_ops::get_video_source_configuration_options(&self.profile_manager)
    }

    // ========================================================================
    // Video Encoder Handlers (delegate to ops::video_encoders)
    // ========================================================================

    /// Handle GetVideoEncoderConfigurations request.
    pub fn handle_get_video_encoder_configurations(
        &self,
        _request: GetVideoEncoderConfigurations,
    ) -> OnvifResult<GetVideoEncoderConfigurationsResponse> {
        video_encoder_ops::get_video_encoder_configurations(&self.profile_manager)
    }

    /// Handle GetVideoEncoderConfiguration request.
    pub fn handle_get_video_encoder_configuration(
        &self,
        request: GetVideoEncoderConfiguration,
    ) -> OnvifResult<GetVideoEncoderConfigurationResponse> {
        video_encoder_ops::get_video_encoder_configuration(&self.profile_manager, request)
    }

    /// Handle SetVideoEncoderConfiguration request.
    pub fn handle_set_video_encoder_configuration(
        &self,
        request: SetVideoEncoderConfiguration,
    ) -> OnvifResult<SetVideoEncoderConfigurationResponse> {
        // Validate sensor resolution if platform is available
        if let Some(ref platform) = self.platform {
            let requested_width = request.configuration.resolution.width as u32;
            let requested_height = request.configuration.resolution.height as u32;

            let max_resolution = platform.max_sensor_resolution().map_err(|e| {
                OnvifError::HardwareFailure(format!("Failed to get sensor resolution: {}", e))
            })?;

            if requested_width > max_resolution.width || requested_height > max_resolution.height {
                return Err(OnvifError::invalid_arg_val(
                    "ter:InvalidResolution",
                    format!(
                        "Requested resolution {}x{} exceeds sensor maximum of {}x{}",
                        requested_width,
                        requested_height,
                        max_resolution.width,
                        max_resolution.height
                    ),
                ));
            }
        }

        video_encoder_ops::set_video_encoder_configuration(&self.profile_manager, request)
    }

    /// Handle GetVideoEncoderConfigurationOptions request.
    // TODO: Request tokens (configuration_token, profile_token) are intentionally ignored.
    // Single-profile camera — options are identical for all video encoder configurations.
    pub fn handle_get_video_encoder_configuration_options(
        &self,
        _request: GetVideoEncoderConfigurationOptions,
    ) -> OnvifResult<GetVideoEncoderConfigurationOptionsResponse> {
        video_encoder_ops::get_video_encoder_configuration_options(&self.profile_manager)
    }

    // ========================================================================
    // Audio Handlers (delegate to ops::audio)
    // ========================================================================

    /// Handle GetAudioSources request.
    pub fn handle_get_audio_sources(
        &self,
        _request: GetAudioSources,
    ) -> OnvifResult<GetAudioSourcesResponse> {
        audio_ops::get_audio_sources(&self.profile_manager)
    }

    /// Handle GetAudioSourceConfigurations request.
    pub fn handle_get_audio_source_configurations(
        &self,
        _request: GetAudioSourceConfigurations,
    ) -> OnvifResult<GetAudioSourceConfigurationsResponse> {
        audio_ops::get_audio_source_configurations(&self.profile_manager)
    }

    /// Handle GetAudioSourceConfiguration request.
    pub fn handle_get_audio_source_configuration(
        &self,
        request: GetAudioSourceConfiguration,
    ) -> OnvifResult<GetAudioSourceConfigurationResponse> {
        audio_ops::get_audio_source_configuration(&self.profile_manager, request)
    }

    /// Handle GetAudioEncoderConfigurations request.
    pub fn handle_get_audio_encoder_configurations(
        &self,
        _request: GetAudioEncoderConfigurations,
    ) -> OnvifResult<GetAudioEncoderConfigurationsResponse> {
        audio_ops::get_audio_encoder_configurations(&self.profile_manager)
    }

    /// Handle GetAudioEncoderConfiguration request.
    pub fn handle_get_audio_encoder_configuration(
        &self,
        request: GetAudioEncoderConfiguration,
    ) -> OnvifResult<GetAudioEncoderConfigurationResponse> {
        audio_ops::get_audio_encoder_configuration(&self.profile_manager, request)
    }

    /// Handle SetAudioEncoderConfiguration request.
    pub fn handle_set_audio_encoder_configuration(
        &self,
        request: SetAudioEncoderConfiguration,
    ) -> OnvifResult<SetAudioEncoderConfigurationResponse> {
        audio_ops::set_audio_encoder_configuration(&self.profile_manager, request)
    }

    /// Handle GetAudioEncoderConfigurationOptions request.
    // TODO: Request tokens (configuration_token, profile_token) are intentionally ignored.
    // Single-profile camera — options are identical for all audio encoder configurations.
    pub fn handle_get_audio_encoder_configuration_options(
        &self,
        _request: GetAudioEncoderConfigurationOptions,
    ) -> OnvifResult<GetAudioEncoderConfigurationOptionsResponse> {
        audio_ops::get_audio_encoder_configuration_options(&self.profile_manager)
    }

    // ========================================================================
    // Stream URI Handlers (delegate to ops::streaming)
    // ========================================================================

    /// Handle GetStreamUri request.
    pub fn handle_get_stream_uri(
        &self,
        request: GetStreamUri,
    ) -> OnvifResult<GetStreamUriResponse> {
        streaming_ops::get_stream_uri(&self.profile_manager, &self.config, request)
    }

    /// Handle GetSnapshotUri request.
    pub fn handle_get_snapshot_uri(
        &self,
        request: GetSnapshotUri,
    ) -> OnvifResult<GetSnapshotUriResponse> {
        streaming_ops::get_snapshot_uri(&self.profile_manager, &self.config, request)
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
    pub fn handle_get_service_capabilities(
        &self,
        _request: GetServiceCapabilities,
    ) -> OnvifResult<GetServiceCapabilitiesResponse> {
        capability_ops::get_service_capabilities()
    }

    // ========================================================================
    // Compatible Configuration Handlers
    // ========================================================================

    /// Handle GetCompatibleVideoSourceConfigurations request.
    pub fn handle_get_compatible_video_source_configurations(
        &self,
        request: GetCompatibleVideoSourceConfigurations,
    ) -> OnvifResult<GetCompatibleVideoSourceConfigurationsResponse> {
        video_source_ops::get_compatible_video_source_configurations(&self.profile_manager, request)
    }

    /// Handle GetCompatibleVideoEncoderConfigurations request.
    pub fn handle_get_compatible_video_encoder_configurations(
        &self,
        request: GetCompatibleVideoEncoderConfigurations,
    ) -> OnvifResult<GetCompatibleVideoEncoderConfigurationsResponse> {
        video_encoder_ops::get_compatible_video_encoder_configurations(
            &self.profile_manager,
            request,
        )
    }

    /// Handle GetCompatibleAudioSourceConfigurations request.
    pub fn handle_get_compatible_audio_source_configurations(
        &self,
        request: GetCompatibleAudioSourceConfigurations,
    ) -> OnvifResult<GetCompatibleAudioSourceConfigurationsResponse> {
        audio_ops::get_compatible_audio_source_configurations(&self.profile_manager, request)
    }

    /// Handle GetCompatibleAudioEncoderConfigurations request.
    pub fn handle_get_compatible_audio_encoder_configurations(
        &self,
        request: GetCompatibleAudioEncoderConfigurations,
    ) -> OnvifResult<GetCompatibleAudioEncoderConfigurationsResponse> {
        audio_ops::get_compatible_audio_encoder_configurations(&self.profile_manager, request)
    }

    // ========================================================================
    // Metadata Configuration Handlers
    // ========================================================================

    /// Handle GetMetadataConfigurations request.
    pub fn handle_get_metadata_configurations(
        &self,
        _request: GetMetadataConfigurations,
    ) -> OnvifResult<GetMetadataConfigurationsResponse> {
        tracing::debug!("GetMetadataConfigurations request");
        Ok(GetMetadataConfigurationsResponse {
            configurations: vec![],
        })
    }

    /// Handle SetMetadataConfiguration request.
    pub fn handle_set_metadata_configuration(
        &self,
        request: SetMetadataConfiguration,
    ) -> OnvifResult<SetMetadataConfigurationResponse> {
        tracing::debug!(
            "SetMetadataConfiguration for token: {}",
            request.configuration.token
        );
        Err(OnvifError::invalid_arg_val(
            "ter:NoConfig",
            format!(
                "Metadata configuration '{}' not found",
                request.configuration.token
            ),
        ))
    }

    // ========================================================================
    // Multicast Streaming Handlers
    // ========================================================================

    /// Handle StartMulticastStreaming request.
    pub fn handle_start_multicast_streaming(
        &self,
        request: StartMulticastStreaming,
    ) -> OnvifResult<StartMulticastStreamingResponse> {
        streaming_ops::start_multicast_streaming(&self.profile_manager, request)
    }

    /// Handle StopMulticastStreaming request.
    pub fn handle_stop_multicast_streaming(
        &self,
        request: StopMulticastStreaming,
    ) -> OnvifResult<StopMulticastStreamingResponse> {
        streaming_ops::stop_multicast_streaming(&self.profile_manager, request)
    }

    // ========================================================================
    // Audio Source Configuration Handler
    // ========================================================================

    /// Handle SetAudioSourceConfiguration request.
    pub fn handle_set_audio_source_configuration(
        &self,
        request: SetAudioSourceConfiguration,
    ) -> OnvifResult<SetAudioSourceConfigurationResponse> {
        audio_ops::set_audio_source_configuration(&self.profile_manager, request)
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

            // Compatible Configurations
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

            // Metadata Configuration
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

            // Multicast Streaming
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

            // Audio Source Configuration
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
}

// Include the tests from handlers.rs that we want to preserve
#[cfg(test)]
mod tests {
    use super::super::types::MAX_PROFILES;
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
    fn test_delete_fixed_profile_fails() {
        let service = MediaService::new();
        // MainStream is a fixed profile
        let result = service.handle_delete_profile(DeleteProfile {
            profile_token: "Profile_MainStream".to_string(),
        });
        assert!(result.is_err());
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
    fn test_get_stream_uri() {
        use crate::onvif::types::common::{StreamSetup, StreamType, TransportProtocol};

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

    #[test]
    fn test_get_profile_invalid_token_returns_no_profile_fault() {
        let service = MediaService::new();
        let result = service.handle_get_profile(GetProfile {
            profile_token: "InvalidToken123".to_string(),
        });
        assert!(result.is_err());
        let err = result.unwrap_err();
        if let OnvifError::InvalidArgVal { subcode, .. } = &err {
            assert!(
                subcode.contains("NoProfile"),
                "subcode should contain NoProfile"
            );
        } else {
            panic!("Expected InvalidArgVal error, got: {:?}", err);
        }
    }

    #[test]
    fn test_set_video_encoder_configuration_invalid_resolution() {
        use crate::onvif::types::common::{
            H264Configuration, H264Profile, VideoEncoderConfiguration, VideoEncoding,
            VideoResolution,
        };

        let service = MediaService::new();
        let result = service.handle_set_video_encoder_configuration(SetVideoEncoderConfiguration {
            configuration: VideoEncoderConfiguration {
                token: "VideoEncoderConfig_0".to_string(),
                name: "TestEncoder".to_string(),
                use_count: 1,
                encoding: VideoEncoding::H264,
                resolution: VideoResolution {
                    width: 0, // Invalid
                    height: 1080,
                },
                quality: 0.5,
                rate_control: None,
                mpeg4: None,
                h264: Some(H264Configuration {
                    gov_length: 30,
                    h264_profile: H264Profile::Baseline,
                }),
                multicast: None,
                session_timeout: "PT60S".to_string(),
            },
            force_persistence: false,
        });
        assert!(result.is_err());
    }

    // ========================================================================
    // ServiceHandler Error Path Tests
    // ========================================================================

    #[tokio::test]
    async fn test_service_handler_unknown_action_media() {
        let service = MediaService::new();
        let result = service.handle_operation("UnknownAction", "<test/>").await;
        assert!(result.is_err());
        assert!(matches!(result, Err(OnvifError::ActionNotSupported(_))));
    }

    #[tokio::test]
    async fn test_service_handler_invalid_xml() {
        let service = MediaService::new();
        let result = service
            .handle_operation("GetProfiles", "<InvalidXml><Broken")
            .await;
        assert!(result.is_err());
        assert!(matches!(result, Err(OnvifError::WellFormed(_))));
    }

    #[tokio::test]
    async fn test_service_handler_get_profiles_xml() {
        let service = MediaService::new();
        let xml = r#"<GetProfiles/>"#;
        let result = service.handle_operation("GetProfiles", xml).await;
        assert!(result.is_ok());
        let response_xml = result.unwrap();
        assert!(response_xml.contains("GetProfilesResponse"));
    }

    #[tokio::test]
    async fn test_service_handler_get_video_sources_xml() {
        let service = MediaService::new();
        let xml = r#"<GetVideoSources/>"#;
        let result = service.handle_operation("GetVideoSources", xml).await;
        assert!(result.is_ok());
        let response_xml = result.unwrap();
        assert!(response_xml.contains("GetVideoSourcesResponse"));
    }

    #[test]
    fn test_set_video_encoder_configuration_invalid_quality() {
        use crate::onvif::types::common::{
            H264Configuration, H264Profile, VideoEncoderConfiguration, VideoEncoding,
            VideoResolution,
        };

        let service = MediaService::new();
        let result = service.handle_set_video_encoder_configuration(SetVideoEncoderConfiguration {
            configuration: VideoEncoderConfiguration {
                token: "VideoEncoderConfig_0".to_string(),
                name: "TestEncoder".to_string(),
                use_count: 1,
                encoding: VideoEncoding::H264,
                resolution: VideoResolution {
                    width: 1920,
                    height: 1080,
                },
                quality: 1.5, // Invalid (> 1.0)
                rate_control: None,
                mpeg4: None,
                h264: Some(H264Configuration {
                    gov_length: 30,
                    h264_profile: H264Profile::Baseline,
                }),
                multicast: None,
                session_timeout: "PT60S".to_string(),
            },
            force_persistence: false,
        });
        assert!(result.is_err());
    }
}
