//! Imaging Service implementation.
//!
//! This module contains the ImagingService struct and the ServiceHandler trait implementation.
//! Operations are delegated to modules in the `ops` directory.

use std::sync::Arc;

use crate::config::ConfigRuntime;
use crate::onvif::dispatcher::ServiceHandler;
use crate::onvif::error::{OnvifError, OnvifResult};
use crate::onvif::types::imaging::*;
use crate::platform::Platform;

use async_trait::async_trait;

// Re-export ops modules for external use
pub use super::ops::capabilities;
pub use super::ops::focus;
pub use super::ops::presets;
pub use super::ops::settings;
pub use super::ops::status;

use super::state::ImagingState;
use super::store::ImagingSettingsStore;

/// ONVIF Imaging Service.
///
/// Handles Imaging Service operations including:
/// - Image settings management
/// - Options retrieval
/// - Status monitoring
/// - Focus control
pub struct ImagingService {
    /// Settings storage.
    settings_store: Arc<ImagingSettingsStore>,
    /// Platform abstraction (optional).
    platform: Option<Arc<dyn Platform>>,
    /// Runtime state.
    state: Arc<ImagingState>,
}

impl ImagingService {
    /// Create a new Imaging Service.
    pub fn new() -> Self {
        Self {
            settings_store: Arc::new(ImagingSettingsStore::new()),
            platform: None,
            state: super::state::new_state(),
        }
    }

    /// Create a new Imaging Service with platform control.
    pub fn with_platform(platform: Arc<dyn Platform>) -> Self {
        let imaging_control = platform.imaging_control();
        let settings_store = if let Some(control) = imaging_control.clone() {
            Arc::new(ImagingSettingsStore::with_platform(control))
        } else {
            Arc::new(ImagingSettingsStore::new())
        };

        Self {
            settings_store,
            platform: Some(platform),
            state: super::state::new_state(),
        }
    }

    /// Create a new Imaging Service with configuration and platform.
    pub fn with_config_and_platform(
        _config: Arc<ConfigRuntime>,
        platform: Arc<dyn Platform>,
    ) -> Self {
        let imaging_control = platform.imaging_control();
        let settings_store = if let Some(control) = imaging_control.clone() {
            Arc::new(ImagingSettingsStore::with_platform(control))
        } else {
            Arc::new(ImagingSettingsStore::new())
        };

        Self {
            settings_store,
            platform: Some(platform),
            state: super::state::new_state(),
        }
    }

    /// Get the settings store.
    pub fn settings_store(&self) -> Arc<ImagingSettingsStore> {
        self.settings_store.clone()
    }

    /// Get the runtime state.
    pub fn state(&self) -> Arc<ImagingState> {
        self.state.clone()
    }

    // ========================================================================
    // Delegate handlers (preserving test interface)
    // ========================================================================

    /// Handle GetImagingSettings request.
    pub async fn handle_get_imaging_settings(
        &self,
        request: GetImagingSettings,
    ) -> OnvifResult<GetImagingSettingsResponse> {
        settings::get_imaging_settings(&self.settings_store, request).await
    }

    /// Handle SetImagingSettings request.
    pub async fn handle_set_imaging_settings(
        &self,
        request: SetImagingSettings,
    ) -> OnvifResult<SetImagingSettingsResponse> {
        settings::set_imaging_settings(&self.settings_store, self.platform.as_ref(), request).await
    }

    /// Handle GetOptions request.
    pub async fn handle_get_options(&self, request: GetOptions) -> OnvifResult<GetOptionsResponse> {
        settings::get_options(&self.settings_store, request).await
    }

    /// Handle GetStatus request.
    pub async fn handle_get_status(&self, request: GetStatus) -> OnvifResult<GetStatusResponse> {
        status::get_status(&self.settings_store, request).await
    }

    /// Handle GetMoveOptions request.
    pub async fn handle_get_move_options(
        &self,
        request: GetMoveOptions,
    ) -> OnvifResult<GetMoveOptionsResponse> {
        focus::get_move_options(&self.settings_store, request).await
    }

    /// Handle Move request (focus).
    pub async fn handle_move(&self, request: Move) -> OnvifResult<MoveResponse> {
        focus::handle_move(&self.settings_store, request).await
    }

    /// Handle Stop request (focus).
    pub async fn handle_stop(&self, request: Stop) -> OnvifResult<StopResponse> {
        focus::handle_stop(&self.settings_store, request).await
    }

    /// Handle GetServiceCapabilities request.
    pub async fn handle_get_service_capabilities(
        &self,
        request: GetServiceCapabilities,
    ) -> OnvifResult<GetServiceCapabilitiesResponse> {
        capabilities::get_service_capabilities(request)
    }

    /// Handle GetPresets request.
    pub async fn handle_get_presets(&self, request: GetPresets) -> OnvifResult<GetPresetsResponse> {
        presets::get_presets(&self.settings_store, request).await
    }

    /// Handle GetCurrentPreset request.
    pub async fn handle_get_current_preset(
        &self,
        request: GetCurrentPreset,
    ) -> OnvifResult<GetCurrentPresetResponse> {
        presets::get_current_preset(&self.settings_store, request).await
    }

    /// Handle SetCurrentPreset request.
    pub async fn handle_set_current_preset(
        &self,
        request: SetCurrentPreset,
    ) -> OnvifResult<SetCurrentPresetResponse> {
        presets::set_current_preset(&self.settings_store, request).await
    }
}

impl Default for ImagingService {
    fn default() -> Self {
        Self::new()
    }
}

// ========================================================================
// ServiceHandler Implementation for ImagingService
// ========================================================================

#[async_trait]
impl ServiceHandler for ImagingService {
    /// Handle a SOAP operation for the Imaging Service.
    ///
    /// Routes the SOAP action to the appropriate handler method and returns
    /// the serialized XML response.
    async fn handle_operation(&self, action: &str, body_xml: &str) -> OnvifResult<String> {
        use crate::onvif::dispatcher::parse_body;

        tracing::debug!("ImagingService handling action: {}", action);

        match action {
            // Settings Operations
            "GetImagingSettings" => {
                let request: GetImagingSettings = parse_body(body_xml)?;
                let response = self.handle_get_imaging_settings(request).await?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "SetImagingSettings" => {
                let request: SetImagingSettings = parse_body(body_xml)?;
                let response = self.handle_set_imaging_settings(request).await?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            // Options Operation
            "GetOptions" => {
                let request: GetOptions = parse_body(body_xml)?;
                let response = self.handle_get_options(request).await?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            // Status Operation
            "GetStatus" => {
                let request: GetStatus = parse_body(body_xml)?;
                let response = self.handle_get_status(request).await?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            // Focus Operations
            "GetMoveOptions" => {
                let request: GetMoveOptions = parse_body(body_xml)?;
                let response = self.handle_get_move_options(request).await?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "Move" => {
                let request: Move = parse_body(body_xml)?;
                let response = self.handle_move(request).await?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "Stop" => {
                let request: Stop = parse_body(body_xml)?;
                let response = self.handle_stop(request).await?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            // Service Capabilities
            "GetServiceCapabilities" => {
                let request: GetServiceCapabilities = parse_body(body_xml)?;
                let response = self.handle_get_service_capabilities(request).await?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            // Preset Operations
            "GetPresets" => {
                let request: GetPresets = parse_body(body_xml)?;
                let response = self.handle_get_presets(request).await?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "GetCurrentPreset" => {
                let request: GetCurrentPreset = parse_body(body_xml)?;
                let response = self.handle_get_current_preset(request).await?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "SetCurrentPreset" => {
                let request: SetCurrentPreset = parse_body(body_xml)?;
                let response = self.handle_set_current_preset(request).await?;
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
        "Imaging"
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::onvif::types::common::ImagingSettings20;

    #[test]
    fn test_new_imaging_service() {
        let service = ImagingService::new();
        assert!(service.platform.is_none());
    }

    #[tokio::test]
    async fn test_service_handler_service_name() {
        let service = ImagingService::new();
        assert_eq!(service.service_name(), "Imaging");
    }

    #[tokio::test]
    async fn test_service_handler_unknown_action() {
        let service = ImagingService::new();
        let result = service.handle_operation("UnknownAction", "<test/>").await;
        assert!(result.is_err());
        assert!(
            matches!(&result, Err(OnvifError::ActionNotSupported(action)) if action == "UnknownAction")
        );
    }

    #[tokio::test]
    async fn test_service_handler_get_imaging_settings_xml() {
        let service = ImagingService::new();
        let xml = r#"<GetImagingSettings><VideoSourceToken>VideoSource_1</VideoSourceToken></GetImagingSettings>"#;
        let result = service.handle_operation("GetImagingSettings", xml).await;
        assert!(result.is_ok());
        let response_xml = result.unwrap();
        assert!(response_xml.contains("GetImagingSettingsResponse"));
        assert!(response_xml.contains("ImagingSettings"));
    }

    #[tokio::test]
    async fn test_service_handler_set_imaging_settings_xml() {
        let service = ImagingService::new();
        let xml = r#"<SetImagingSettings><VideoSourceToken>VideoSource_1</VideoSourceToken><ImagingSettings><Brightness>75.0</Brightness></ImagingSettings></SetImagingSettings>"#;
        let result = service.handle_operation("SetImagingSettings", xml).await;
        assert!(result.is_ok());
        let response_xml = result.unwrap();
        assert!(response_xml.contains("SetImagingSettingsResponse"));
    }

    #[tokio::test]
    async fn test_service_handler_get_options_xml() {
        let service = ImagingService::new();
        let xml = r#"<GetOptions><VideoSourceToken>VideoSource_1</VideoSourceToken></GetOptions>"#;
        let result = service.handle_operation("GetOptions", xml).await;
        assert!(result.is_ok());
        let response_xml = result.unwrap();
        assert!(response_xml.contains("GetOptionsResponse"));
    }

    #[tokio::test]
    async fn test_service_handler_get_status_xml() {
        let service = ImagingService::new();
        let xml = r#"<GetStatus><VideoSourceToken>VideoSource_1</VideoSourceToken></GetStatus>"#;
        let result = service.handle_operation("GetStatus", xml).await;
        assert!(result.is_ok());
        let response_xml = result.unwrap();
        assert!(response_xml.contains("GetStatusResponse"));
    }

    #[tokio::test]
    async fn test_service_handler_invalid_xml() {
        let service = ImagingService::new();
        let xml = "<InvalidXml><Broken";
        let result = service.handle_operation("GetImagingSettings", xml).await;
        assert!(result.is_err());
        assert!(matches!(result, Err(OnvifError::WellFormed(_))));
    }

    #[tokio::test]
    async fn test_service_handler_unknown_action_imaging() {
        let service = ImagingService::new();
        let result = service.handle_operation("UnknownAction", "<test/>").await;
        assert!(result.is_err());
        assert!(matches!(result, Err(OnvifError::ActionNotSupported(_))));
    }

    // ========================================================================
    // Delegate Handler Tests
    // ========================================================================

    #[tokio::test]
    async fn test_get_imaging_settings() {
        let service = ImagingService::new();
        let request = GetImagingSettings {
            video_source_token: "VideoSource_1".to_string(),
        };
        let result = service.handle_get_imaging_settings(request).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.imaging_settings.brightness, Some(50.0));
    }

    #[tokio::test]
    async fn test_set_imaging_settings() {
        let service = ImagingService::new();
        let settings = ImagingSettings20 {
            brightness: Some(75.0),
            contrast: Some(60.0),
            color_saturation: Some(50.0),
            sharpness: Some(50.0),
            ..Default::default()
        };
        let request = SetImagingSettings {
            video_source_token: "VideoSource_1".to_string(),
            imaging_settings: settings,
            force_persistence: Some(false),
        };
        let result = service.handle_set_imaging_settings(request).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_options() {
        let service = ImagingService::new();
        let request = GetOptions {
            video_source_token: "VideoSource_1".to_string(),
        };
        let result = service.handle_get_options(request).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.imaging_options.brightness.is_some());
    }

    #[tokio::test]
    async fn test_get_status() {
        let service = ImagingService::new();
        let request = GetStatus {
            video_source_token: "VideoSource_1".to_string(),
        };
        let result = service.handle_get_status(request).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_move_options() {
        let service = ImagingService::new();
        let request = GetMoveOptions {
            video_source_token: "VideoSource_1".to_string(),
        };
        let result = service.handle_get_move_options(request).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_move_not_supported() {
        let service = ImagingService::new();
        let request = Move {
            video_source_token: "VideoSource_1".to_string(),
            focus: FocusMove::default(),
        };
        let result = service.handle_move(request).await;
        assert!(result.is_err());
        assert!(matches!(result, Err(OnvifError::ActionNotSupported(_))));
    }

    #[tokio::test]
    async fn test_stop_not_supported() {
        let service = ImagingService::new();
        let request = Stop {
            video_source_token: "VideoSource_1".to_string(),
        };
        let result = service.handle_stop(request).await;
        assert!(result.is_err());
        assert!(matches!(result, Err(OnvifError::ActionNotSupported(_))));
    }

    #[tokio::test]
    async fn test_get_service_capabilities() {
        let service = ImagingService::new();
        let request = GetServiceCapabilities {};
        let result = service.handle_get_service_capabilities(request).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.capabilities.image_stabilization, Some(false));
        assert_eq!(response.capabilities.presets, Some(false));
    }

    #[tokio::test]
    async fn test_get_presets() {
        let service = ImagingService::new();
        let request = GetPresets {
            video_source_token: "VideoSource_1".to_string(),
        };
        let result = service.handle_get_presets(request).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.presets.is_empty());
    }

    #[tokio::test]
    async fn test_get_current_preset() {
        let service = ImagingService::new();
        let request = GetCurrentPreset {
            video_source_token: "VideoSource_1".to_string(),
        };
        let result = service.handle_get_current_preset(request).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.preset.is_none());
    }

    #[tokio::test]
    async fn test_set_current_preset_not_supported() {
        let service = ImagingService::new();
        let request = SetCurrentPreset {
            video_source_token: "VideoSource_1".to_string(),
            preset_token: "Preset_1".to_string(),
        };
        let result = service.handle_set_current_preset(request).await;
        assert!(result.is_err());
        assert!(matches!(result, Err(OnvifError::ActionNotSupported(_))));
    }

    // ========================================================================
    // Platform Integration Tests
    // ========================================================================

    #[tokio::test]
    async fn test_set_imaging_settings_with_platform() {
        use crate::platform::StubPlatformBuilder;

        let platform = Arc::new(StubPlatformBuilder::new().imaging_supported(true).build());
        let service = ImagingService::with_platform(platform);

        let response = service
            .handle_set_imaging_settings(SetImagingSettings {
                video_source_token: "VideoSource_1".to_string(),
                imaging_settings: ImagingSettings20 {
                    brightness: Some(75.0),
                    contrast: Some(80.0),
                    color_saturation: Some(70.0),
                    sharpness: Some(65.0),
                    ..Default::default()
                },
                force_persistence: None,
            })
            .await
            .unwrap();

        let _ = response;
    }

    #[tokio::test]
    async fn test_set_imaging_settings_platform_failure() {
        use crate::platform::StubPlatformBuilder;

        let platform = Arc::new(StubPlatformBuilder::new().imaging_supported(false).build());
        let service = ImagingService::with_platform(platform);

        // Should still work (platform failure handled gracefully)
        let response = service
            .handle_set_imaging_settings(SetImagingSettings {
                video_source_token: "VideoSource_1".to_string(),
                imaging_settings: ImagingSettings20 {
                    brightness: Some(75.0),
                    contrast: Some(80.0),
                    color_saturation: Some(70.0),
                    sharpness: Some(65.0),
                    ..Default::default()
                },
                force_persistence: None,
            })
            .await;

        assert!(response.is_ok());
    }

    #[tokio::test]
    async fn test_move_focus_not_supported() {
        let service = ImagingService::new();
        let result = service
            .handle_move(Move {
                video_source_token: "VideoSource_1".to_string(),
                focus: FocusMove::default(),
            })
            .await;

        assert!(result.is_err());
        assert!(matches!(result, Err(OnvifError::ActionNotSupported(_))));
    }

    #[tokio::test]
    async fn test_move_invalid_token() {
        let service = ImagingService::new();
        let result = service
            .handle_move(Move {
                video_source_token: "InvalidToken".to_string(),
                focus: FocusMove::default(),
            })
            .await;

        assert!(result.is_err());
        assert!(matches!(result, Err(OnvifError::InvalidArgVal { .. })));
    }
}
