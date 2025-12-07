//! Imaging Service request handlers.
//!
//! This module implements the ONVIF Imaging Service operation handlers including:
//! - GetImagingSettings / SetImagingSettings
//! - GetOptions
//! - GetStatus
//! - Move / Stop / GetMoveOptions (focus control)
//! - GetServiceCapabilities

use std::sync::Arc;

use crate::config::ConfigRuntime;
use crate::onvif::error::{OnvifError, OnvifResult};
use crate::onvif::types::imaging::{
    GetImagingSettings, GetImagingSettingsResponse, GetMoveOptions, GetMoveOptionsResponse,
    GetOptions, GetOptionsResponse, GetServiceCapabilities, GetServiceCapabilitiesResponse,
    GetStatus, GetStatusResponse, ImagingServiceCapabilities, Move, MoveOptions20, MoveResponse,
    SetImagingSettings, SetImagingSettingsResponse, Stop, StopResponse,
};
use crate::platform::Platform;

use super::settings_store::{ImagingSettingsError, ImagingSettingsStore};

// ============================================================================
// ImagingService
// ============================================================================

/// ONVIF Imaging Service.
///
/// Handles Imaging Service operations including:
/// - Image settings management
/// - Options retrieval
/// - Status monitoring
/// - Focus control (when supported)
pub struct ImagingService {
    /// Settings storage.
    settings_store: Arc<ImagingSettingsStore>,
    /// Configuration runtime.
    #[allow(dead_code)]
    config: Arc<ConfigRuntime>,
    /// Platform abstraction (optional).
    platform: Option<Arc<dyn Platform>>,
    /// Focus control supported.
    focus_supported: bool,
}

impl ImagingService {
    /// Create a new Imaging Service.
    pub fn new() -> Self {
        Self {
            settings_store: Arc::new(ImagingSettingsStore::new()),
            config: Arc::new(ConfigRuntime::new(Default::default())),
            platform: None,
            focus_supported: false,
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
            config: Arc::new(ConfigRuntime::new(Default::default())),
            platform: Some(platform),
            focus_supported: false, // Focus control not typically available
        }
    }

    /// Create a new Imaging Service with configuration and platform.
    pub fn with_config_and_platform(
        config: Arc<ConfigRuntime>,
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
            config,
            platform: Some(platform),
            focus_supported: false,
        }
    }

    /// Get the settings store.
    pub fn settings_store(&self) -> Arc<ImagingSettingsStore> {
        self.settings_store.clone()
    }

    // ========================================================================
    // Settings Handlers
    // ========================================================================

    /// Handle GetImagingSettings request.
    ///
    /// Returns current imaging settings for the specified video source.
    pub async fn handle_get_imaging_settings(
        &self,
        request: GetImagingSettings,
    ) -> OnvifResult<GetImagingSettingsResponse> {
        tracing::debug!(
            "GetImagingSettings request for token: {}",
            request.video_source_token
        );

        let settings = self
            .settings_store
            .get_settings(&request.video_source_token)
            .await
            .map_err(Self::map_settings_error)?;

        Ok(GetImagingSettingsResponse {
            imaging_settings: settings,
        })
    }

    /// Handle SetImagingSettings request.
    ///
    /// Updates imaging settings for the specified video source.
    pub async fn handle_set_imaging_settings(
        &self,
        request: SetImagingSettings,
    ) -> OnvifResult<SetImagingSettingsResponse> {
        tracing::debug!(
            "SetImagingSettings request for token: {}",
            request.video_source_token
        );

        let force_persistence = request.force_persistence.unwrap_or(false);

        self.settings_store
            .set_settings(
                &request.video_source_token,
                &request.imaging_settings,
                force_persistence,
            )
            .await
            .map_err(Self::map_settings_error)?;

        // Apply individual settings to platform if available
        if let Some(ref platform) = self.platform {
            if let Some(imaging) = platform.imaging_control() {
                // Apply brightness
                if let Some(brightness) = request.imaging_settings.brightness {
                    if let Err(e) = imaging.set_brightness(brightness).await {
                        tracing::warn!("Failed to set brightness: {}", e);
                    }
                }

                // Apply contrast
                if let Some(contrast) = request.imaging_settings.contrast {
                    if let Err(e) = imaging.set_contrast(contrast).await {
                        tracing::warn!("Failed to set contrast: {}", e);
                    }
                }

                // Apply saturation
                if let Some(saturation) = request.imaging_settings.color_saturation {
                    if let Err(e) = imaging.set_saturation(saturation).await {
                        tracing::warn!("Failed to set saturation: {}", e);
                    }
                }

                // Apply sharpness
                if let Some(sharpness) = request.imaging_settings.sharpness {
                    if let Err(e) = imaging.set_sharpness(sharpness).await {
                        tracing::warn!("Failed to set sharpness: {}", e);
                    }
                }
            }
        }

        Ok(SetImagingSettingsResponse {})
    }

    // ========================================================================
    // Options Handler
    // ========================================================================

    /// Handle GetOptions request.
    ///
    /// Returns supported imaging options for the specified video source.
    pub async fn handle_get_options(&self, request: GetOptions) -> OnvifResult<GetOptionsResponse> {
        tracing::debug!(
            "GetOptions request for token: {}",
            request.video_source_token
        );

        let options = self
            .settings_store
            .get_options(&request.video_source_token)
            .await
            .map_err(Self::map_settings_error)?;

        Ok(GetOptionsResponse {
            imaging_options: options,
        })
    }

    // ========================================================================
    // Status Handler
    // ========================================================================

    /// Handle GetStatus request.
    ///
    /// Returns imaging status including focus status for the specified video source.
    pub async fn handle_get_status(&self, request: GetStatus) -> OnvifResult<GetStatusResponse> {
        tracing::debug!(
            "GetStatus request for token: {}",
            request.video_source_token
        );

        let status = self
            .settings_store
            .get_status(&request.video_source_token)
            .await
            .map_err(Self::map_settings_error)?;

        Ok(GetStatusResponse { status })
    }

    // ========================================================================
    // Focus Control Handlers
    // ========================================================================

    /// Handle GetMoveOptions request.
    ///
    /// Returns supported focus move options for the specified video source.
    pub async fn handle_get_move_options(
        &self,
        request: GetMoveOptions,
    ) -> OnvifResult<GetMoveOptionsResponse> {
        tracing::debug!(
            "GetMoveOptions request for token: {}",
            request.video_source_token
        );

        // Validate token
        if !self
            .settings_store
            .is_valid_token(&request.video_source_token)
        {
            return Err(OnvifError::InvalidArgVal {
                subcode: "ter:InvalidToken".to_string(),
                reason: format!("Invalid video source token: {}", request.video_source_token),
            });
        }

        // Return default move options (focus not supported in this implementation)
        Ok(GetMoveOptionsResponse {
            move_options: MoveOptions20::default(),
        })
    }

    /// Handle Move request (focus).
    ///
    /// Performs focus move operation if supported.
    pub async fn handle_move(&self, request: Move) -> OnvifResult<MoveResponse> {
        tracing::debug!(
            "Move (focus) request for token: {}",
            request.video_source_token
        );

        // Validate token
        if !self
            .settings_store
            .is_valid_token(&request.video_source_token)
        {
            return Err(OnvifError::InvalidArgVal {
                subcode: "ter:InvalidToken".to_string(),
                reason: format!("Invalid video source token: {}", request.video_source_token),
            });
        }

        // Focus control is typically not supported on this hardware
        if !self.focus_supported {
            return Err(OnvifError::ActionNotSupported(
                "Focus move operation not supported".to_string(),
            ));
        }

        // TODO: Implement focus move when hardware supports it
        Ok(MoveResponse {})
    }

    /// Handle Stop request (focus).
    ///
    /// Stops focus movement if in progress.
    pub async fn handle_stop(&self, request: Stop) -> OnvifResult<StopResponse> {
        tracing::debug!(
            "Stop (focus) request for token: {}",
            request.video_source_token
        );

        // Validate token
        if !self
            .settings_store
            .is_valid_token(&request.video_source_token)
        {
            return Err(OnvifError::InvalidArgVal {
                subcode: "ter:InvalidToken".to_string(),
                reason: format!("Invalid video source token: {}", request.video_source_token),
            });
        }

        // Focus control is typically not supported on this hardware
        if !self.focus_supported {
            return Err(OnvifError::ActionNotSupported(
                "Focus stop operation not supported".to_string(),
            ));
        }

        // TODO: Implement focus stop when hardware supports it
        Ok(StopResponse {})
    }

    // ========================================================================
    // Service Capabilities Handler
    // ========================================================================

    /// Handle GetServiceCapabilities request.
    ///
    /// Returns imaging service capabilities.
    pub async fn handle_get_service_capabilities(
        &self,
        _request: GetServiceCapabilities,
    ) -> OnvifResult<GetServiceCapabilitiesResponse> {
        tracing::debug!("GetServiceCapabilities request");

        Ok(GetServiceCapabilitiesResponse {
            capabilities: ImagingServiceCapabilities {
                image_stabilization: Some(false),
                presets: Some(false),
                adaptable_preset: Some(false),
            },
        })
    }

    // ========================================================================
    // Individual Setting Handlers
    // ========================================================================

    /// Set brightness via platform.
    pub async fn set_brightness(&self, value: f32) -> OnvifResult<()> {
        if let Some(ref platform) = self.platform {
            if let Some(imaging) = platform.imaging_control() {
                imaging.set_brightness(value).await.map_err(|e| {
                    OnvifError::HardwareFailure(format!("Failed to set brightness: {}", e))
                })?;
            }
        }
        Ok(())
    }

    /// Set contrast via platform.
    pub async fn set_contrast(&self, value: f32) -> OnvifResult<()> {
        if let Some(ref platform) = self.platform {
            if let Some(imaging) = platform.imaging_control() {
                imaging.set_contrast(value).await.map_err(|e| {
                    OnvifError::HardwareFailure(format!("Failed to set contrast: {}", e))
                })?;
            }
        }
        Ok(())
    }

    /// Set saturation via platform.
    pub async fn set_saturation(&self, value: f32) -> OnvifResult<()> {
        if let Some(ref platform) = self.platform {
            if let Some(imaging) = platform.imaging_control() {
                imaging.set_saturation(value).await.map_err(|e| {
                    OnvifError::HardwareFailure(format!("Failed to set saturation: {}", e))
                })?;
            }
        }
        Ok(())
    }

    /// Set sharpness via platform.
    pub async fn set_sharpness(&self, value: f32) -> OnvifResult<()> {
        if let Some(ref platform) = self.platform {
            if let Some(imaging) = platform.imaging_control() {
                imaging.set_sharpness(value).await.map_err(|e| {
                    OnvifError::HardwareFailure(format!("Failed to set sharpness: {}", e))
                })?;
            }
        }
        Ok(())
    }

    // ========================================================================
    // Preset Operations (WSDL: GetPresets, GetCurrentPreset, SetCurrentPreset)
    // ========================================================================

    /// Handle GetPresets request.
    ///
    /// Returns the list of available imaging presets.
    /// Returns empty list if presets are not supported.
    pub async fn handle_get_presets(
        &self,
        request: crate::onvif::types::imaging::GetPresets,
    ) -> OnvifResult<crate::onvif::types::imaging::GetPresetsResponse> {
        // Validate video source token
        if !self
            .settings_store
            .is_valid_token(&request.video_source_token)
        {
            return Err(OnvifError::InvalidArgVal {
                subcode: "ter:InvalidToken".to_string(),
                reason: format!("Invalid video source token: {}", request.video_source_token),
            });
        }

        // Presets not supported on this device
        Ok(crate::onvif::types::imaging::GetPresetsResponse { presets: vec![] })
    }

    /// Handle GetCurrentPreset request.
    ///
    /// Returns the currently active preset, or None if no preset is active.
    pub async fn handle_get_current_preset(
        &self,
        request: crate::onvif::types::imaging::GetCurrentPreset,
    ) -> OnvifResult<crate::onvif::types::imaging::GetCurrentPresetResponse> {
        // Validate video source token
        if !self
            .settings_store
            .is_valid_token(&request.video_source_token)
        {
            return Err(OnvifError::InvalidArgVal {
                subcode: "ter:InvalidToken".to_string(),
                reason: format!("Invalid video source token: {}", request.video_source_token),
            });
        }

        // No preset currently active (presets not supported)
        Ok(crate::onvif::types::imaging::GetCurrentPresetResponse { preset: None })
    }

    /// Handle SetCurrentPreset request.
    ///
    /// Applies the specified imaging preset to the video source.
    pub async fn handle_set_current_preset(
        &self,
        request: crate::onvif::types::imaging::SetCurrentPreset,
    ) -> OnvifResult<crate::onvif::types::imaging::SetCurrentPresetResponse> {
        // Validate video source token
        if !self
            .settings_store
            .is_valid_token(&request.video_source_token)
        {
            return Err(OnvifError::InvalidArgVal {
                subcode: "ter:InvalidToken".to_string(),
                reason: format!("Invalid video source token: {}", request.video_source_token),
            });
        }

        // Presets not supported - return error
        Err(OnvifError::ActionNotSupported(
            "Imaging presets are not supported on this device".to_string(),
        ))
    }

    // ========================================================================
    // Error Mapping
    // ========================================================================

    /// Map settings error to ONVIF error.
    fn map_settings_error(err: ImagingSettingsError) -> OnvifError {
        match err {
            ImagingSettingsError::InvalidToken(token) => OnvifError::InvalidArgVal {
                subcode: "ter:InvalidToken".to_string(),
                reason: format!("Invalid video source token: {}", token),
            },
            ImagingSettingsError::OutOfRange {
                parameter,
                value,
                min,
                max,
            } => OnvifError::InvalidArgVal {
                subcode: "ter:InvalidArgVal".to_string(),
                reason: format!(
                    "Parameter '{}' value {} is out of range ({} - {})",
                    parameter, value, min, max
                ),
            },
            ImagingSettingsError::PlatformError(msg) => OnvifError::HardwareFailure(msg),
            ImagingSettingsError::ValidationFailed(msg) => OnvifError::InvalidArgVal {
                subcode: "ter:InvalidArgVal".to_string(),
                reason: msg,
            },
        }
    }
}

impl Default for ImagingService {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// ServiceHandler Implementation for ImagingService
// ============================================================================

use crate::onvif::dispatcher::ServiceHandler;
use async_trait::async_trait;

#[async_trait]
impl ServiceHandler for ImagingService {
    /// Handle a SOAP operation for the Imaging Service.
    ///
    /// Routes the SOAP action to the appropriate handler method and returns
    /// the serialized XML response.
    async fn handle_operation(&self, action: &str, body_xml: &str) -> OnvifResult<String> {
        tracing::debug!("ImagingService handling action: {}", action);

        match action {
            // Settings Operations
            "GetImagingSettings" => {
                let request: GetImagingSettings = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_get_imaging_settings(request).await?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "SetImagingSettings" => {
                let request: SetImagingSettings = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_set_imaging_settings(request).await?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            // Options Operation
            "GetOptions" => {
                let request: GetOptions = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_get_options(request).await?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            // Status Operation
            "GetStatus" => {
                let request: GetStatus = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_get_status(request).await?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            // Focus Operations
            "GetMoveOptions" => {
                let request: GetMoveOptions = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_get_move_options(request).await?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "Move" => {
                let request: Move = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_move(request).await?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "Stop" => {
                let request: Stop = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_stop(request).await?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            // Service Capabilities
            "GetServiceCapabilities" => {
                let request: GetServiceCapabilities = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_get_service_capabilities(request).await?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            // Preset Operations
            "GetPresets" => {
                let request: crate::onvif::types::imaging::GetPresets =
                    quick_xml::de::from_str(body_xml).map_err(|e| {
                        OnvifError::WellFormed(format!("Invalid request XML: {}", e))
                    })?;
                let response = self.handle_get_presets(request).await?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "GetCurrentPreset" => {
                let request: crate::onvif::types::imaging::GetCurrentPreset =
                    quick_xml::de::from_str(body_xml).map_err(|e| {
                        OnvifError::WellFormed(format!("Invalid request XML: {}", e))
                    })?;
                let response = self.handle_get_current_preset(request).await?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "SetCurrentPreset" => {
                let request: crate::onvif::types::imaging::SetCurrentPreset =
                    quick_xml::de::from_str(body_xml).map_err(|e| {
                        OnvifError::WellFormed(format!("Invalid request XML: {}", e))
                    })?;
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

    /// Get the list of supported actions.
    fn supported_actions(&self) -> Vec<&str> {
        vec![
            "GetImagingSettings",
            "SetImagingSettings",
            "GetOptions",
            "GetStatus",
            "GetMoveOptions",
            "Move",
            "Stop",
            "GetServiceCapabilities",
            "GetPresets",
            "GetCurrentPreset",
            "SetCurrentPreset",
        ]
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
        assert!(!service.focus_supported);
    }

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
    async fn test_get_imaging_settings_invalid_token() {
        let service = ImagingService::new();
        let request = GetImagingSettings {
            video_source_token: "InvalidToken".to_string(),
        };
        let result = service.handle_get_imaging_settings(request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_set_imaging_settings() {
        let service = ImagingService::new();
        let settings = ImagingSettings20 {
            brightness: Some(75.0),
            contrast: Some(60.0),
            color_saturation: Some(50.0),
            sharpness: Some(50.0),
            ir_cut_filter: None,
            wide_dynamic_range: None,
            backlight_compensation: None,
            exposure: None,
            focus: None,
            white_balance: None,
            extension: None,
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
    async fn test_set_imaging_settings_invalid_value() {
        let service = ImagingService::new();
        let settings = ImagingSettings20 {
            brightness: Some(150.0), // Out of range
            contrast: None,
            color_saturation: None,
            sharpness: None,
            ir_cut_filter: None,
            wide_dynamic_range: None,
            backlight_compensation: None,
            exposure: None,
            focus: None,
            white_balance: None,
            extension: None,
        };
        let request = SetImagingSettings {
            video_source_token: "VideoSource_1".to_string(),
            imaging_settings: settings,
            force_persistence: None,
        };
        let result = service.handle_set_imaging_settings(request).await;
        assert!(result.is_err());
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
            focus: crate::onvif::types::imaging::FocusMove::default(),
        };
        let result = service.handle_move(request).await;
        assert!(result.is_err());
        match result {
            Err(OnvifError::ActionNotSupported(_)) => {}
            _ => panic!("Expected ActionNotSupported error"),
        }
    }

    #[tokio::test]
    async fn test_stop_not_supported() {
        let service = ImagingService::new();
        let request = Stop {
            video_source_token: "VideoSource_1".to_string(),
        };
        let result = service.handle_stop(request).await;
        assert!(result.is_err());
        match result {
            Err(OnvifError::ActionNotSupported(_)) => {}
            _ => panic!("Expected ActionNotSupported error"),
        }
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

    // ========================================================================
    // Additional WSDL-compliant tests
    // ========================================================================

    #[tokio::test]
    async fn test_set_imaging_settings_invalid_token() {
        let service = ImagingService::new();
        let settings = ImagingSettings20 {
            brightness: Some(50.0),
            ..Default::default()
        };
        let request = SetImagingSettings {
            video_source_token: "InvalidToken".to_string(),
            imaging_settings: settings,
            force_persistence: None,
        };
        let result = service.handle_set_imaging_settings(request).await;
        assert!(result.is_err());
        match result {
            Err(OnvifError::InvalidArgVal { subcode, .. }) => {
                assert!(subcode.contains("InvalidToken"));
            }
            _ => panic!("Expected InvalidArgVal error with InvalidToken"),
        }
    }

    #[tokio::test]
    async fn test_set_imaging_settings_boundary_min() {
        let service = ImagingService::new();
        let settings = ImagingSettings20 {
            brightness: Some(0.0), // Minimum boundary
            contrast: Some(0.0),
            color_saturation: Some(0.0),
            sharpness: Some(0.0),
            ..Default::default()
        };
        let request = SetImagingSettings {
            video_source_token: "VideoSource_1".to_string(),
            imaging_settings: settings,
            force_persistence: None,
        };
        let result = service.handle_set_imaging_settings(request).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_set_imaging_settings_boundary_max() {
        let service = ImagingService::new();
        let settings = ImagingSettings20 {
            brightness: Some(100.0), // Maximum boundary
            contrast: Some(100.0),
            color_saturation: Some(100.0),
            sharpness: Some(100.0),
            ..Default::default()
        };
        let request = SetImagingSettings {
            video_source_token: "VideoSource_1".to_string(),
            imaging_settings: settings,
            force_persistence: None,
        };
        let result = service.handle_set_imaging_settings(request).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_set_imaging_settings_negative_value() {
        let service = ImagingService::new();
        let settings = ImagingSettings20 {
            brightness: Some(-10.0), // Negative - out of range
            ..Default::default()
        };
        let request = SetImagingSettings {
            video_source_token: "VideoSource_1".to_string(),
            imaging_settings: settings,
            force_persistence: None,
        };
        let result = service.handle_set_imaging_settings(request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_set_imaging_settings_contrast_invalid() {
        let service = ImagingService::new();
        let settings = ImagingSettings20 {
            contrast: Some(200.0), // Out of range
            ..Default::default()
        };
        let request = SetImagingSettings {
            video_source_token: "VideoSource_1".to_string(),
            imaging_settings: settings,
            force_persistence: None,
        };
        let result = service.handle_set_imaging_settings(request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_set_imaging_settings_saturation_invalid() {
        let service = ImagingService::new();
        let settings = ImagingSettings20 {
            color_saturation: Some(150.0), // Out of range
            ..Default::default()
        };
        let request = SetImagingSettings {
            video_source_token: "VideoSource_1".to_string(),
            imaging_settings: settings,
            force_persistence: None,
        };
        let result = service.handle_set_imaging_settings(request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_set_imaging_settings_sharpness_invalid() {
        let service = ImagingService::new();
        let settings = ImagingSettings20 {
            sharpness: Some(101.0), // Just above max
            ..Default::default()
        };
        let request = SetImagingSettings {
            video_source_token: "VideoSource_1".to_string(),
            imaging_settings: settings,
            force_persistence: None,
        };
        let result = service.handle_set_imaging_settings(request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_options_invalid_token() {
        let service = ImagingService::new();
        let request = GetOptions {
            video_source_token: "InvalidToken".to_string(),
        };
        let result = service.handle_get_options(request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_status_invalid_token() {
        let service = ImagingService::new();
        let request = GetStatus {
            video_source_token: "InvalidToken".to_string(),
        };
        let result = service.handle_get_status(request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_move_options_invalid_token() {
        let service = ImagingService::new();
        let request = GetMoveOptions {
            video_source_token: "InvalidToken".to_string(),
        };
        let result = service.handle_get_move_options(request).await;
        assert!(result.is_err());
    }

    // ========================================================================
    // Preset operation tests (WSDL compliance)
    // ========================================================================

    #[tokio::test]
    async fn test_get_presets() {
        let service = ImagingService::new();
        let request = crate::onvif::types::imaging::GetPresets {
            video_source_token: "VideoSource_1".to_string(),
        };
        let result = service.handle_get_presets(request).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.presets.is_empty()); // No presets supported
    }

    #[tokio::test]
    async fn test_get_presets_invalid_token() {
        let service = ImagingService::new();
        let request = crate::onvif::types::imaging::GetPresets {
            video_source_token: "InvalidToken".to_string(),
        };
        let result = service.handle_get_presets(request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_current_preset() {
        let service = ImagingService::new();
        let request = crate::onvif::types::imaging::GetCurrentPreset {
            video_source_token: "VideoSource_1".to_string(),
        };
        let result = service.handle_get_current_preset(request).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.preset.is_none()); // No preset active
    }

    #[tokio::test]
    async fn test_get_current_preset_invalid_token() {
        let service = ImagingService::new();
        let request = crate::onvif::types::imaging::GetCurrentPreset {
            video_source_token: "InvalidToken".to_string(),
        };
        let result = service.handle_get_current_preset(request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_set_current_preset_not_supported() {
        let service = ImagingService::new();
        let request = crate::onvif::types::imaging::SetCurrentPreset {
            video_source_token: "VideoSource_1".to_string(),
            preset_token: "Preset_1".to_string(),
        };
        let result = service.handle_set_current_preset(request).await;
        assert!(result.is_err());
        match result {
            Err(OnvifError::ActionNotSupported(_)) => {}
            _ => panic!("Expected ActionNotSupported error"),
        }
    }

    // ========================================================================
    // ServiceHandler tests
    // ========================================================================

    #[tokio::test]
    async fn test_service_handler_supported_actions() {
        let service = ImagingService::new();
        let actions = service.supported_actions();
        assert!(actions.contains(&"GetImagingSettings"));
        assert!(actions.contains(&"SetImagingSettings"));
        assert!(actions.contains(&"GetOptions"));
        assert!(actions.contains(&"GetStatus"));
        assert!(actions.contains(&"GetMoveOptions"));
        assert!(actions.contains(&"Move"));
        assert!(actions.contains(&"Stop"));
        assert!(actions.contains(&"GetServiceCapabilities"));
        assert!(actions.contains(&"GetPresets"));
        assert!(actions.contains(&"GetCurrentPreset"));
        assert!(actions.contains(&"SetCurrentPreset"));
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
        match result {
            Err(OnvifError::ActionNotSupported(action)) => {
                assert_eq!(action, "UnknownAction");
            }
            _ => panic!("Expected ActionNotSupported error"),
        }
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
    async fn test_service_handler_invalid_xml() {
        let service = ImagingService::new();
        let xml = "<InvalidXml><Broken";
        let result = service.handle_operation("GetImagingSettings", xml).await;
        assert!(result.is_err());
        match result {
            Err(OnvifError::WellFormed(_)) => {}
            _ => panic!("Expected WellFormed error"),
        }
    }
}
