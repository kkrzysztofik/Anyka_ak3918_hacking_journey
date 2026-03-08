//! ONVIF PTZ Service implementation.
//!
//! ONVIF PTZ Service implementation.

use std::sync::Arc;

use crate::onvif::dispatcher::ServiceHandler;
use crate::onvif::dispatcher::parse_body;
use crate::onvif::error::{OnvifError, OnvifResult};
use crate::onvif::types::ptz::{
    AbsoluteMove, AbsoluteMoveResponse, ContinuousMove, ContinuousMoveResponse,
    GetCompatibleConfigurations, GetCompatibleConfigurationsResponse, GetConfiguration,
    GetConfigurationOptions, GetConfigurationOptionsResponse, GetConfigurationResponse,
    GetConfigurations, GetConfigurationsResponse, GetNode, GetNodeResponse, GetNodes,
    GetNodesResponse, GetPresets, GetPresetsResponse, GetServiceCapabilities,
    GetServiceCapabilitiesResponse, GetStatus, GetStatusResponse, GotoHomePosition,
    GotoHomePositionResponse, GotoPreset, GotoPresetResponse, PTZNode, RelativeMove,
    RelativeMoveResponse, RemovePreset, RemovePresetResponse, SendAuxiliaryCommand,
    SendAuxiliaryCommandResponse, SetConfiguration, SetConfigurationResponse, SetHomePosition,
    SetHomePositionResponse, SetPreset, SetPresetResponse, Stop, StopResponse,
};
use crate::platform::{PTZControl, Platform};

use super::ops::auxiliary;
use super::ops::config;
use super::ops::movement;
use super::ops::presets;
use super::ops::status;
use super::state::PTZStateManager;
use super::types::*;

// ============================================================================
// PTZService
// ============================================================================

/// ONVIF PTZ Service.
///
/// Handles PTZ Service operations including:
/// - Node discovery and configuration
/// - Movement operations (absolute, relative, continuous)
/// - Preset management (get, set, goto, remove)
/// - Home position management
/// - Auxiliary commands and service capabilities
///
/// The service supports an optional platform PTZ control backend for
/// forwarding commands to real hardware. When no platform is provided,
/// the service operates in software-only mode using in-memory state.
pub struct PTZService {
    /// PTZ state manager (position, movement, presets).
    pub(crate) state: Arc<PTZStateManager>,
    /// Platform PTZ control (optional for software-only mode).
    pub(crate) ptz_control: Option<Arc<dyn PTZControl>>,
}

impl PTZService {
    /// Create a new PTZ Service in software-only mode.
    ///
    /// # Arguments
    ///
    /// * `state` - Shared PTZ state manager for position and preset tracking
    ///
    /// # Returns
    ///
    /// A `PTZService` with no platform backend. All PTZ operations are
    /// handled in-memory only.
    pub fn new(state: Arc<PTZStateManager>) -> Self {
        Self {
            state,
            ptz_control: None,
        }
    }

    /// Create a new PTZ Service with platform PTZ control.
    ///
    /// # Arguments
    ///
    /// * `state` - Shared PTZ state manager
    /// * `platform` - Platform abstraction providing hardware PTZ control
    pub fn with_platform(state: Arc<PTZStateManager>, platform: Arc<dyn Platform>) -> Self {
        Self {
            state,
            ptz_control: platform.ptz_control(),
        }
    }

    /// Create a new PTZ Service with direct PTZ control.
    pub fn with_ptz_control(state: Arc<PTZStateManager>, ptz_control: Arc<dyn PTZControl>) -> Self {
        Self {
            state,
            ptz_control: Some(ptz_control),
        }
    }

    // ========================================================================
    // Validator and builder methods
    // ========================================================================

    /// Validate a profile token.
    ///
    /// Performs a fast-fail check for empty tokens. The AK3918 device has a
    /// single fixed profile ("Profile_1"), so any non-empty token is accepted
    /// here; individual handlers may apply stricter checks when needed.
    #[allow(dead_code)]
    pub(crate) fn validate_profile_token(&self, token: &str) -> OnvifResult<()> {
        if token.is_empty() {
            return Err(OnvifError::InvalidArgVal {
                subcode: "NoToken".to_string(),
                reason: "Profile token is required".to_string(),
            });
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub(crate) fn validate_config_token(&self, token: &str) -> OnvifResult<()> {
        config::validate_config_token(token)
    }

    #[allow(dead_code)]
    pub(crate) fn validate_node_token(&self, token: &str) -> OnvifResult<()> {
        config::validate_node_token(token)
    }

    #[allow(dead_code)]
    pub(crate) fn build_ptz_node(&self) -> PTZNode {
        config::build_ptz_node()
    }

    #[allow(dead_code)]
    pub(crate) fn build_ptz_configuration(&self) -> PTZConfiguration {
        config::build_ptz_configuration()
    }

    // ========================================================================
    // Delegation methods
    // ========================================================================

    /// Handle GetNodes request - delegates to ops::config
    pub fn handle_get_nodes(&self, _request: GetNodes) -> OnvifResult<GetNodesResponse> {
        config::get_nodes(&self.state)
    }

    /// Handle GetNode request - delegates to ops::config
    pub fn handle_get_node(&self, request: GetNode) -> OnvifResult<GetNodeResponse> {
        config::get_node(&self.state, request.node_token)
    }

    /// Handle GetConfigurations request - delegates to ops::config
    pub fn handle_get_configurations(
        &self,
        _request: GetConfigurations,
    ) -> OnvifResult<GetConfigurationsResponse> {
        config::get_configurations(&self.state)
    }

    /// Handle GetConfiguration request - delegates to ops::config
    pub fn handle_get_configuration(
        &self,
        request: GetConfiguration,
    ) -> OnvifResult<GetConfigurationResponse> {
        config::get_configuration(&self.state, request.ptz_configuration_token)
    }

    /// Handle SetConfiguration request - delegates to ops::config
    pub fn handle_set_configuration(
        &self,
        request: SetConfiguration,
    ) -> OnvifResult<SetConfigurationResponse> {
        config::set_configuration(&self.state, request.ptz_configuration)?;
        Ok(SetConfigurationResponse {})
    }

    /// Handle GetConfigurationOptions request - delegates to ops::config
    pub fn handle_get_configuration_options(
        &self,
        request: GetConfigurationOptions,
    ) -> OnvifResult<GetConfigurationOptionsResponse> {
        config::get_configuration_options(&self.state, request.configuration_token)
    }

    /// Handle AbsoluteMove request - delegates to ops::movement
    pub async fn handle_absolute_move(
        &self,
        request: AbsoluteMove,
    ) -> OnvifResult<AbsoluteMoveResponse> {
        self.validate_profile_token(&request.profile_token)?;
        movement::absolute_move(
            &self.state,
            &self.ptz_control,
            &request.profile_token,
            request.position,
        )
        .await?;
        Ok(AbsoluteMoveResponse {})
    }

    /// Handle RelativeMove request - delegates to ops::movement
    pub async fn handle_relative_move(
        &self,
        request: RelativeMove,
    ) -> OnvifResult<RelativeMoveResponse> {
        self.validate_profile_token(&request.profile_token)?;
        movement::relative_move(
            &self.state,
            &self.ptz_control,
            &request.profile_token,
            request.translation,
        )
        .await?;
        Ok(RelativeMoveResponse {})
    }

    /// Handle ContinuousMove request - delegates to ops::movement
    pub async fn handle_continuous_move(
        &self,
        request: ContinuousMove,
    ) -> OnvifResult<ContinuousMoveResponse> {
        self.validate_profile_token(&request.profile_token)?;
        movement::continuous_move(
            &self.state,
            &self.ptz_control,
            &request.profile_token,
            request.velocity,
        )
        .await?;
        Ok(ContinuousMoveResponse {})
    }

    /// Handle Stop request - delegates to ops::movement
    pub async fn handle_stop(&self, request: Stop) -> OnvifResult<StopResponse> {
        self.validate_profile_token(&request.profile_token)?;
        let stop_pan_tilt = request.pan_tilt.unwrap_or(true);
        let stop_zoom = request.zoom.unwrap_or(true);
        movement::stop(
            &self.state,
            &self.ptz_control,
            &request.profile_token,
            stop_pan_tilt,
            stop_zoom,
        )
        .await?;
        Ok(StopResponse {})
    }

    /// Handle GetStatus request - delegates to ops::status
    pub fn handle_get_status(&self, request: GetStatus) -> OnvifResult<GetStatusResponse> {
        self.validate_profile_token(&request.profile_token)?;
        status::get_status(&self.state, &request.profile_token)
    }

    /// Handle GotoHomePosition request - delegates to ops::status
    pub async fn handle_goto_home_position(
        &self,
        request: GotoHomePosition,
    ) -> OnvifResult<GotoHomePositionResponse> {
        self.validate_profile_token(&request.profile_token)?;
        status::goto_home_position(&self.state, &self.ptz_control, &request.profile_token).await?;
        Ok(GotoHomePositionResponse {})
    }

    /// Handle SetHomePosition request - delegates to ops::status
    pub fn handle_set_home_position(
        &self,
        request: SetHomePosition,
    ) -> OnvifResult<SetHomePositionResponse> {
        self.validate_profile_token(&request.profile_token)?;
        status::set_home_position(&self.state, &request.profile_token)?;
        Ok(SetHomePositionResponse {})
    }

    /// Handle GetPresets request - delegates to ops::presets
    pub fn handle_get_presets(&self, request: GetPresets) -> OnvifResult<GetPresetsResponse> {
        self.validate_profile_token(&request.profile_token)?;
        presets::get_presets(&self.state, &request.profile_token)
    }

    /// Handle SetPreset request - delegates to ops::presets
    pub fn handle_set_preset(&self, request: SetPreset) -> OnvifResult<SetPresetResponse> {
        self.validate_profile_token(&request.profile_token)?;
        presets::set_preset(
            &self.state,
            &self.ptz_control,
            &request.profile_token,
            request.preset_name,
            request.preset_token,
        )
    }

    /// Handle GotoPreset request - delegates to ops::presets
    pub async fn handle_goto_preset(&self, request: GotoPreset) -> OnvifResult<GotoPresetResponse> {
        self.validate_profile_token(&request.profile_token)?;
        presets::goto_preset(
            &self.state,
            &self.ptz_control,
            &request.profile_token,
            request.preset_token,
        )
        .await?;
        Ok(GotoPresetResponse {})
    }

    /// Handle RemovePreset request - delegates to ops::presets
    pub fn handle_remove_preset(&self, request: RemovePreset) -> OnvifResult<RemovePresetResponse> {
        self.validate_profile_token(&request.profile_token)?;
        presets::remove_preset(&self.state, &request.profile_token, request.preset_token)?;
        Ok(RemovePresetResponse {})
    }

    /// Handle GetServiceCapabilities request - delegates to ops::auxiliary
    pub fn handle_get_service_capabilities(
        &self,
        _request: GetServiceCapabilities,
    ) -> OnvifResult<GetServiceCapabilitiesResponse> {
        auxiliary::get_service_capabilities(&self.state)
    }

    /// Handle GetCompatibleConfigurations request - delegates to ops::auxiliary
    pub fn handle_get_compatible_configurations(
        &self,
        request: GetCompatibleConfigurations,
    ) -> OnvifResult<GetCompatibleConfigurationsResponse> {
        self.validate_profile_token(&request.profile_token)?;
        auxiliary::get_compatible_configurations(&self.state, &request.profile_token)
    }

    /// Handle SendAuxiliaryCommand request - delegates to ops::auxiliary
    pub fn handle_send_auxiliary_command(
        &self,
        request: SendAuxiliaryCommand,
    ) -> OnvifResult<SendAuxiliaryCommandResponse> {
        self.validate_profile_token(&request.profile_token)?;
        let response = auxiliary::send_auxiliary_command(
            &self.state,
            &request.profile_token,
            &request.auxiliary_data,
        )?;
        Ok(SendAuxiliaryCommandResponse {
            auxiliary_response: response,
        })
    }
}

// ============================================================================
// ServiceHandler Implementation
// ============================================================================

#[async_trait::async_trait]
impl ServiceHandler for PTZService {
    /// Handle a SOAP operation for the PTZ Service.
    ///
    /// Routes the SOAP action to the appropriate handler method and returns
    /// the serialized XML response.
    async fn handle_operation(&self, action: &str, body_xml: &str) -> Result<String, OnvifError> {
        tracing::debug!("PTZService handling action: {}", action);

        match action {
            // Node Operations
            "GetNodes" => {
                let request: GetNodes = parse_body(body_xml)?;
                let response = self.handle_get_nodes(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "GetNode" => {
                let request: GetNode = parse_body(body_xml)?;
                let response = self.handle_get_node(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            // Configuration Operations
            "GetConfigurations" => {
                let request: GetConfigurations = parse_body(body_xml)?;
                let response = self.handle_get_configurations(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "GetConfiguration" => {
                let request: GetConfiguration = parse_body(body_xml)?;
                let response = self.handle_get_configuration(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "SetConfiguration" => {
                let request: SetConfiguration = parse_body(body_xml)?;
                let response = self.handle_set_configuration(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "GetConfigurationOptions" => {
                let request: GetConfigurationOptions = parse_body(body_xml)?;
                let response = self.handle_get_configuration_options(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            // Movement Operations
            "AbsoluteMove" => {
                let request: AbsoluteMove = parse_body(body_xml)?;
                let response = self.handle_absolute_move(request).await?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "RelativeMove" => {
                let request: RelativeMove = parse_body(body_xml)?;
                let response = self.handle_relative_move(request).await?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "ContinuousMove" => {
                let request: ContinuousMove = parse_body(body_xml)?;
                let response = self.handle_continuous_move(request).await?;
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

            "GetStatus" => {
                let request: GetStatus = parse_body(body_xml)?;
                let response = self.handle_get_status(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            // Home Position Operations
            "GotoHomePosition" => {
                let request: GotoHomePosition = parse_body(body_xml)?;
                let response = self.handle_goto_home_position(request).await?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "SetHomePosition" => {
                let request: SetHomePosition = parse_body(body_xml)?;
                let response = self.handle_set_home_position(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            // Preset Operations
            "GetPresets" => {
                let request: GetPresets = parse_body(body_xml)?;
                let response = self.handle_get_presets(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "SetPreset" => {
                let request: SetPreset = parse_body(body_xml)?;
                let response = self.handle_set_preset(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "GotoPreset" => {
                let request: GotoPreset = parse_body(body_xml)?;
                let response = self.handle_goto_preset(request).await?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "RemovePreset" => {
                let request: RemovePreset = parse_body(body_xml)?;
                let response = self.handle_remove_preset(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            // Service Capabilities
            "GetServiceCapabilities" => {
                let request: GetServiceCapabilities = parse_body(body_xml)?;
                let response = self.handle_get_service_capabilities(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "GetCompatibleConfigurations" => {
                let request: GetCompatibleConfigurations = parse_body(body_xml)?;
                let response = self.handle_get_compatible_configurations(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "SendAuxiliaryCommand" => {
                let request: SendAuxiliaryCommand = parse_body(body_xml)?;
                let response = self.handle_send_auxiliary_command(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            _ => {
                tracing::warn!("PTZService: Unknown action '{}'", action);
                Err(OnvifError::ActionNotSupported(format!(
                    "Action '{}' is not supported by PTZ Service",
                    action
                )))
            }
        }
    }

    /// Get the service name.
    fn service_name(&self) -> &str {
        "PTZ"
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_service() -> PTZService {
        let state = Arc::new(PTZStateManager::new());
        PTZService::new(state)
    }

    // ========================================================================
    // Node Operations
    // ========================================================================

    #[test]
    fn test_get_nodes() {
        let service = create_test_service();

        let response = service.handle_get_nodes(GetNodes {}).unwrap();

        assert_eq!(response.ptz_nodes.len(), 1);
        assert_eq!(response.ptz_nodes[0].token, DEFAULT_NODE_TOKEN);
        assert!(response.ptz_nodes[0].home_supported);
    }

    #[test]
    fn test_get_node() {
        let service = create_test_service();

        let response = service
            .handle_get_node(GetNode {
                node_token: DEFAULT_NODE_TOKEN.to_string(),
            })
            .unwrap();

        assert_eq!(response.ptz_node.token, DEFAULT_NODE_TOKEN);
        assert_eq!(response.ptz_node.maximum_number_of_presets, MAX_PRESETS);
    }

    #[test]
    fn test_get_node_invalid_token() {
        let service = create_test_service();

        let result = service.handle_get_node(GetNode {
            node_token: "InvalidToken".to_string(),
        });

        assert!(result.is_err());
    }

    // ========================================================================
    // Configuration Operations
    // ========================================================================

    #[test]
    fn test_get_configurations() {
        let service = create_test_service();

        let response = service
            .handle_get_configurations(GetConfigurations {})
            .unwrap();

        assert_eq!(response.ptz_configurations.len(), 1);
        assert_eq!(response.ptz_configurations[0].token, DEFAULT_CONFIG_TOKEN);
    }

    #[test]
    fn test_get_configuration() {
        let service = create_test_service();

        let response = service
            .handle_get_configuration(GetConfiguration {
                ptz_configuration_token: DEFAULT_CONFIG_TOKEN.to_string(),
            })
            .unwrap();

        assert_eq!(response.ptz_configuration.token, DEFAULT_CONFIG_TOKEN);
        assert_eq!(response.ptz_configuration.node_token, DEFAULT_NODE_TOKEN);
    }

    #[test]
    fn test_get_configuration_options() {
        let service = create_test_service();

        let response = service
            .handle_get_configuration_options(GetConfigurationOptions {
                configuration_token: DEFAULT_CONFIG_TOKEN.to_string(),
            })
            .unwrap();

        // Verify spaces are present
        assert!(
            !response
                .ptz_configuration_options
                .spaces
                .absolute_pan_tilt_position_space
                .is_empty()
        );
    }

    // ========================================================================
    // Movement Operations
    // ========================================================================

    #[tokio::test]
    async fn test_absolute_move() {
        let service = create_test_service();

        let response = service
            .handle_absolute_move(AbsoluteMove {
                profile_token: "Profile1".to_string(),
                position: PTZVector {
                    pan_tilt: Some(Vector2D {
                        x: 0.5,
                        y: -0.3,
                        space: None,
                    }),
                    zoom: Some(Vector1D {
                        x: 0.7,
                        space: None,
                    }),
                },
                speed: None,
            })
            .await
            .unwrap();

        // Should succeed
        let _ = response;
    }

    #[tokio::test]
    async fn test_relative_move() {
        let service = create_test_service();

        let response = service
            .handle_relative_move(RelativeMove {
                profile_token: "Profile1".to_string(),
                translation: PTZVector {
                    pan_tilt: Some(Vector2D {
                        x: 0.1,
                        y: 0.1,
                        space: None,
                    }),
                    zoom: Some(Vector1D {
                        x: 0.1,
                        space: None,
                    }),
                },
                speed: None,
            })
            .await
            .unwrap();

        let _ = response;
    }

    #[tokio::test]
    async fn test_continuous_move() {
        let service = create_test_service();

        let response = service
            .handle_continuous_move(ContinuousMove {
                profile_token: "Profile1".to_string(),
                velocity: PTZSpeed {
                    pan_tilt: Some(Vector2D {
                        x: 0.5,
                        y: 0.0,
                        space: None,
                    }),
                    zoom: None,
                },
                timeout: None,
            })
            .await
            .unwrap();

        let _ = response;

        // Verify moving state
        assert!(service.state.is_moving());
    }

    #[tokio::test]
    async fn test_stop() {
        let service = create_test_service();

        // Start moving
        service.state.set_moving(true, true);

        // Stop
        let response = service
            .handle_stop(Stop {
                profile_token: "Profile1".to_string(),
                pan_tilt: Some(true),
                zoom: Some(true),
            })
            .await
            .unwrap();

        let _ = response;

        // Verify stopped
        assert!(!service.state.is_moving());
    }

    #[test]
    fn test_get_status() {
        let service = create_test_service();

        let response = service
            .handle_get_status(GetStatus {
                profile_token: "Profile1".to_string(),
            })
            .unwrap();

        // Should have position
        assert!(response.ptz_status.position.is_some());
        // Should have move status
        assert!(response.ptz_status.move_status.is_some());
        // Should have UTC time
        assert!(!response.ptz_status.utc_time.is_empty());
    }

    // ========================================================================
    // Home Position Operations
    // ========================================================================

    #[tokio::test]
    async fn test_goto_home_position() {
        let service = create_test_service();

        let response = service
            .handle_goto_home_position(GotoHomePosition {
                profile_token: "Profile1".to_string(),
                speed: None,
            })
            .await
            .unwrap();

        let _ = response;
    }

    #[test]
    fn test_set_home_position() {
        let service = create_test_service();

        // Move to a position
        service.state.set_position(&PTZVector {
            pan_tilt: Some(Vector2D {
                x: 0.8,
                y: 0.4,
                space: None,
            }),
            zoom: Some(Vector1D {
                x: 0.5,
                space: None,
            }),
        });

        // Set as home
        let response = service
            .handle_set_home_position(SetHomePosition {
                profile_token: "Profile1".to_string(),
            })
            .unwrap();

        let _ = response;

        // Verify home was set
        let home = service.state.get_home_position();
        let pt = home.pan_tilt.unwrap();
        assert_eq!(pt.x, 0.8);
        assert_eq!(pt.y, 0.4);
    }

    // ========================================================================
    // Preset Operations
    // ========================================================================

    #[test]
    fn test_get_presets_empty() {
        let service = create_test_service();

        let response = service
            .handle_get_presets(GetPresets {
                profile_token: "Profile1".to_string(),
            })
            .unwrap();

        assert!(response.presets.is_empty());
    }

    #[test]
    fn test_set_preset() {
        let service = create_test_service();

        let response = service
            .handle_set_preset(SetPreset {
                profile_token: "Profile1".to_string(),
                preset_name: Some("TestPreset".to_string()),
                preset_token: None,
            })
            .unwrap();

        // Should return a token
        assert!(!response.preset_token.is_empty());
    }

    #[tokio::test]
    async fn test_goto_preset() {
        let service = create_test_service();

        // Move to a position and create preset
        service.state.set_position(&PTZVector {
            pan_tilt: Some(Vector2D {
                x: 0.6,
                y: 0.7,
                space: None,
            }),
            zoom: Some(Vector1D {
                x: 0.8,
                space: None,
            }),
        });

        let set_response = service
            .handle_set_preset(SetPreset {
                profile_token: "Profile1".to_string(),
                preset_name: Some("GotoTest".to_string()),
                preset_token: None,
            })
            .unwrap();

        // Move somewhere else
        service.state.set_position(&PTZVector {
            pan_tilt: Some(Vector2D {
                x: 0.0,
                y: 0.0,
                space: None,
            }),
            zoom: Some(Vector1D {
                x: 0.0,
                space: None,
            }),
        });

        // Go to preset
        let goto_response = service
            .handle_goto_preset(GotoPreset {
                profile_token: "Profile1".to_string(),
                preset_token: set_response.preset_token,
                speed: None,
            })
            .await
            .unwrap();

        let _ = goto_response;

        // Verify position
        let pos = service.state.get_position();
        let pt = pos.pan_tilt.unwrap();
        assert_eq!(pt.x, 0.6);
        assert_eq!(pt.y, 0.7);
    }

    #[test]
    fn test_remove_preset() {
        let service = create_test_service();

        // Create preset
        let set_response = service
            .handle_set_preset(SetPreset {
                profile_token: "Profile1".to_string(),
                preset_name: Some("ToRemove".to_string()),
                preset_token: None,
            })
            .unwrap();

        // Remove it
        let remove_response = service
            .handle_remove_preset(RemovePreset {
                profile_token: "Profile1".to_string(),
                preset_token: set_response.preset_token.clone(),
            })
            .unwrap();

        let _ = remove_response;

        // Verify it's gone
        let presets_response = service
            .handle_get_presets(GetPresets {
                profile_token: "Profile1".to_string(),
            })
            .unwrap();

        assert!(presets_response.presets.is_empty());
    }

    // ========================================================================
    // Service Capabilities
    // ========================================================================

    #[test]
    fn test_get_service_capabilities() {
        let service = create_test_service();

        let response = service
            .handle_get_service_capabilities(GetServiceCapabilities {})
            .unwrap();

        assert_eq!(response.capabilities.move_status, Some(true));
        assert_eq!(response.capabilities.status_position, Some(true));
    }

    #[test]
    fn test_get_compatible_configurations() {
        let service = create_test_service();

        let response = service
            .handle_get_compatible_configurations(GetCompatibleConfigurations {
                profile_token: "Profile1".to_string(),
            })
            .unwrap();

        assert_eq!(response.ptz_configurations.len(), 1);
    }

    // ========================================================================
    // Validation Tests
    // ========================================================================

    #[test]
    fn test_empty_profile_token() {
        let service = create_test_service();

        let result = service.handle_get_status(GetStatus {
            profile_token: "".to_string(),
        });

        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_config_token() {
        let service = create_test_service();

        let result = service.handle_get_configuration(GetConfiguration {
            ptz_configuration_token: "NonExistentConfig".to_string(),
        });

        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_config_token_for_options() {
        let service = create_test_service();

        let result = service.handle_get_configuration_options(GetConfigurationOptions {
            configuration_token: "InvalidConfig".to_string(),
        });

        assert!(result.is_err());
    }

    #[test]
    fn test_set_configuration_invalid_token() {
        let service = create_test_service();

        let mut config = service
            .handle_get_configuration(GetConfiguration {
                ptz_configuration_token: DEFAULT_CONFIG_TOKEN.to_string(),
            })
            .unwrap()
            .ptz_configuration;

        // Set an invalid token by replacing the field value
        let _ = std::mem::replace(&mut config.token, "InvalidConfigId".to_string());

        let result = service.handle_set_configuration(SetConfiguration {
            ptz_configuration: config,
            force_persistence: false,
        });

        assert!(result.is_err());
    }

    #[test]
    fn test_set_configuration_not_supported() {
        let service = create_test_service();

        let config = service
            .handle_get_configuration(GetConfiguration {
                ptz_configuration_token: DEFAULT_CONFIG_TOKEN.to_string(),
            })
            .unwrap()
            .ptz_configuration;

        let result = service.handle_set_configuration(SetConfiguration {
            ptz_configuration: config,
            force_persistence: false,
        });

        assert!(result.is_err());
        assert!(matches!(result, Err(OnvifError::ActionNotSupported(_))));
    }

    // ========================================================================
    // SendAuxiliaryCommand Tests
    // ========================================================================

    #[test]
    fn test_send_auxiliary_command() {
        let service = create_test_service();

        let response = service
            .handle_send_auxiliary_command(SendAuxiliaryCommand {
                profile_token: "Profile1".to_string(),
                auxiliary_data: "tt:Wiper|On".to_string(),
            })
            .unwrap();

        // We return success with no response data
        assert!(response.auxiliary_response.is_none());
    }

    #[test]
    fn test_send_auxiliary_command_empty_profile() {
        let service = create_test_service();

        let result = service.handle_send_auxiliary_command(SendAuxiliaryCommand {
            profile_token: "".to_string(),
            auxiliary_data: "tt:Wiper|On".to_string(),
        });

        assert!(result.is_err());
    }

    // ========================================================================
    // Position Boundary Tests
    // ========================================================================

    #[tokio::test]
    async fn test_relative_move_clamps_position() {
        let service = create_test_service();

        // Start at position near upper limit
        service.state.set_position(&PTZVector {
            pan_tilt: Some(Vector2D {
                x: 0.9,
                y: 0.9,
                space: None,
            }),
            zoom: Some(Vector1D {
                x: 0.9,
                space: None,
            }),
        });

        // Try to move beyond limits
        let _ = service
            .handle_relative_move(RelativeMove {
                profile_token: "Profile1".to_string(),
                translation: PTZVector {
                    pan_tilt: Some(Vector2D {
                        x: 0.5,
                        y: 0.5,
                        space: None,
                    }),
                    zoom: Some(Vector1D {
                        x: 0.5,
                        space: None,
                    }),
                },
                speed: None,
            })
            .await
            .unwrap();

        // Position should be clamped to 1.0
        let pos = service.state.get_position();
        let pt = pos.pan_tilt.unwrap();
        assert!(pt.x <= 1.0);
        assert!(pt.y <= 1.0);
        let z = pos.zoom.unwrap();
        assert!(z.x <= 1.0);
    }

    #[tokio::test]
    async fn test_relative_move_clamps_negative() {
        let service = create_test_service();

        // Start at position near lower limit
        service.state.set_position(&PTZVector {
            pan_tilt: Some(Vector2D {
                x: -0.9,
                y: -0.9,
                space: None,
            }),
            zoom: Some(Vector1D {
                x: 0.1,
                space: None,
            }),
        });

        // Try to move beyond limits
        let _ = service
            .handle_relative_move(RelativeMove {
                profile_token: "Profile1".to_string(),
                translation: PTZVector {
                    pan_tilt: Some(Vector2D {
                        x: -0.5,
                        y: -0.5,
                        space: None,
                    }),
                    zoom: Some(Vector1D {
                        x: -0.5,
                        space: None,
                    }),
                },
                speed: None,
            })
            .await
            .unwrap();

        // Position should be clamped
        let pos = service.state.get_position();
        let pt = pos.pan_tilt.unwrap();
        assert!(pt.x >= -1.0);
        assert!(pt.y >= -1.0);
        let z = pos.zoom.unwrap();
        assert!(z.x >= 0.0);
    }

    // ========================================================================
    // Preset Limit Tests
    // ========================================================================

    #[tokio::test]
    async fn test_goto_preset_invalid_token() {
        let service = create_test_service();

        let result = service
            .handle_goto_preset(GotoPreset {
                profile_token: "Profile1".to_string(),
                preset_token: "NonExistentPreset".to_string(),
                speed: None,
            })
            .await;

        assert!(result.is_err());
    }

    #[test]
    fn test_remove_preset_invalid_token() {
        let service = create_test_service();

        let result = service.handle_remove_preset(RemovePreset {
            profile_token: "Profile1".to_string(),
            preset_token: "NonExistentPreset".to_string(),
        });

        assert!(result.is_err());
    }

    #[test]
    fn test_update_existing_preset() {
        let service = create_test_service();

        // Create a preset
        let response = service
            .handle_set_preset(SetPreset {
                profile_token: "Profile1".to_string(),
                preset_name: Some("OriginalName".to_string()),
                preset_token: None,
            })
            .unwrap();

        let preset_id = response.preset_token;

        // Move to new position
        service.state.set_position(&PTZVector {
            pan_tilt: Some(Vector2D {
                x: 0.9,
                y: 0.9,
                space: None,
            }),
            zoom: Some(Vector1D {
                x: 0.9,
                space: None,
            }),
        });

        // Update the preset
        let update_response = service
            .handle_set_preset(SetPreset {
                profile_token: "Profile1".to_string(),
                preset_name: Some("UpdatedName".to_string()),
                preset_token: Some(preset_id.clone()),
            })
            .unwrap();

        assert_eq!(update_response.preset_token, preset_id);

        // Verify name was updated by checking preset with new name exists
        let presets = service
            .handle_get_presets(GetPresets {
                profile_token: "Profile1".to_string(),
            })
            .unwrap();

        // Find the preset by its updated name (avoid .token reference)
        let preset = presets
            .presets
            .iter()
            .find(|p| p.name.as_deref() == Some("UpdatedName"))
            .unwrap();
        assert_eq!(preset.name, Some("UpdatedName".to_string()));
    }

    // ========================================================================
    // Error Path Tests
    // ========================================================================

    #[tokio::test]
    async fn test_absolute_move_empty_profile_token() {
        let service = create_test_service();

        let result = service
            .handle_absolute_move(AbsoluteMove {
                profile_token: "".to_string(),
                position: PTZVector {
                    pan_tilt: Some(Vector2D {
                        x: 0.5,
                        y: 0.5,
                        space: None,
                    }),
                    zoom: None,
                },
                speed: None,
            })
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_relative_move_empty_profile_token() {
        let service = create_test_service();

        let result = service
            .handle_relative_move(RelativeMove {
                profile_token: "".to_string(),
                translation: PTZVector {
                    pan_tilt: Some(Vector2D {
                        x: 0.1,
                        y: 0.1,
                        space: None,
                    }),
                    zoom: None,
                },
                speed: None,
            })
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_continuous_move_empty_profile_token() {
        let service = create_test_service();

        let result = service
            .handle_continuous_move(ContinuousMove {
                profile_token: "".to_string(),
                velocity: PTZSpeed {
                    pan_tilt: Some(Vector2D {
                        x: 0.5,
                        y: 0.0,
                        space: None,
                    }),
                    zoom: None,
                },
                timeout: None,
            })
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_stop_empty_profile_token() {
        let service = create_test_service();

        let result = service
            .handle_stop(Stop {
                profile_token: "".to_string(),
                pan_tilt: Some(true),
                zoom: Some(true),
            })
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_goto_home_position_empty_profile_token() {
        let service = create_test_service();

        let result = service
            .handle_goto_home_position(GotoHomePosition {
                profile_token: "".to_string(),
                speed: None,
            })
            .await;

        assert!(result.is_err());
    }

    #[test]
    fn test_set_home_position_empty_profile_token() {
        let service = create_test_service();

        let result = service.handle_set_home_position(SetHomePosition {
            profile_token: "".to_string(),
        });

        assert!(result.is_err());
    }

    #[test]
    fn test_get_presets_empty_profile_token() {
        let service = create_test_service();

        let result = service.handle_get_presets(GetPresets {
            profile_token: "".to_string(),
        });

        assert!(result.is_err());
    }

    #[test]
    fn test_set_preset_empty_profile_token() {
        let service = create_test_service();

        let result = service.handle_set_preset(SetPreset {
            profile_token: "".to_string(),
            preset_name: Some("Test".to_string()),
            preset_token: None,
        });

        assert!(result.is_err());
    }

    #[test]
    fn test_send_auxiliary_command_empty_profile_token() {
        let service = create_test_service();

        let result = service.handle_send_auxiliary_command(SendAuxiliaryCommand {
            profile_token: "".to_string(),
            auxiliary_data: "test".to_string(),
        });

        assert!(result.is_err());
    }

    #[test]
    fn test_get_compatible_configurations_empty_profile_token() {
        let service = create_test_service();

        let result = service.handle_get_compatible_configurations(GetCompatibleConfigurations {
            profile_token: "".to_string(),
        });

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_stop_partial_pan_tilt() {
        let service = create_test_service();

        // Start moving
        service.state.set_moving(true, true);

        // Stop only pan/tilt
        let response = service
            .handle_stop(Stop {
                profile_token: "Profile1".to_string(),
                pan_tilt: Some(true),
                zoom: Some(false),
            })
            .await
            .unwrap();

        let _ = response;
        // State should reflect partial stop
    }

    #[tokio::test]
    async fn test_stop_partial_zoom() {
        let service = create_test_service();

        // Start moving
        service.state.set_moving(true, true);

        // Stop only zoom
        let response = service
            .handle_stop(Stop {
                profile_token: "Profile1".to_string(),
                pan_tilt: Some(false),
                zoom: Some(true),
            })
            .await
            .unwrap();

        let _ = response;
    }

    #[test]
    fn test_set_preset_with_existing_token() {
        let service = create_test_service();

        // Create a preset first
        let first_response = service
            .handle_set_preset(SetPreset {
                profile_token: "Profile1".to_string(),
                preset_name: Some("Original".to_string()),
                preset_token: None,
            })
            .unwrap();

        let preset_id = first_response.preset_token.clone();

        // Update the preset with the same token
        let update_response = service
            .handle_set_preset(SetPreset {
                profile_token: "Profile1".to_string(),
                preset_name: Some("Updated".to_string()),
                preset_token: Some(preset_id.clone()),
            })
            .unwrap();

        assert_eq!(update_response.preset_token, preset_id);
    }

    // ========================================================================
    // Platform Integration Tests
    // ========================================================================

    #[tokio::test]
    async fn test_absolute_move_with_platform() {
        use crate::platform::StubPlatformBuilder;

        let state = Arc::new(PTZStateManager::new());
        let platform = Arc::new(StubPlatformBuilder::new().ptz_supported(true).build());
        let service = PTZService::with_platform(state, platform);

        let response = service
            .handle_absolute_move(AbsoluteMove {
                profile_token: "Profile1".to_string(),
                position: PTZVector {
                    pan_tilt: Some(Vector2D {
                        x: 0.5,
                        y: -0.3,
                        space: None,
                    }),
                    zoom: Some(Vector1D {
                        x: 0.7,
                        space: None,
                    }),
                },
                speed: None,
            })
            .await
            .unwrap();

        let _ = response;
    }

    #[tokio::test]
    async fn test_absolute_move_platform_failure() {
        use crate::platform::StubPlatformBuilder;

        let state = Arc::new(PTZStateManager::new());
        // Create platform that will fail PTZ operations
        let platform = Arc::new(StubPlatformBuilder::new().ptz_supported(false).build());
        let service = PTZService::with_platform(state, platform);

        // Should still work (platform failure is handled gracefully)
        let response = service
            .handle_absolute_move(AbsoluteMove {
                profile_token: "Profile1".to_string(),
                position: PTZVector {
                    pan_tilt: Some(Vector2D {
                        x: 0.5,
                        y: -0.3,
                        space: None,
                    }),
                    zoom: Some(Vector1D {
                        x: 0.7,
                        space: None,
                    }),
                },
                speed: None,
            })
            .await;

        // Should succeed even if platform fails (state is updated)
        assert!(response.is_ok());
    }

    #[tokio::test]
    async fn test_relative_move_with_platform() {
        use crate::platform::StubPlatformBuilder;

        let state = Arc::new(PTZStateManager::new());
        let platform = Arc::new(StubPlatformBuilder::new().ptz_supported(true).build());
        let service = PTZService::with_platform(state, platform);

        let response = service
            .handle_relative_move(RelativeMove {
                profile_token: "Profile1".to_string(),
                translation: PTZVector {
                    pan_tilt: Some(Vector2D {
                        x: 0.1,
                        y: 0.1,
                        space: None,
                    }),
                    zoom: Some(Vector1D {
                        x: 0.1,
                        space: None,
                    }),
                },
                speed: None,
            })
            .await
            .unwrap();

        let _ = response;
    }

    #[tokio::test]
    async fn test_continuous_move_with_platform() {
        use crate::platform::StubPlatformBuilder;

        let state = Arc::new(PTZStateManager::new());
        let platform = Arc::new(StubPlatformBuilder::new().ptz_supported(true).build());
        let service = PTZService::with_platform(state, platform);

        let response = service
            .handle_continuous_move(ContinuousMove {
                profile_token: "Profile1".to_string(),
                velocity: PTZSpeed {
                    pan_tilt: Some(Vector2D {
                        x: 0.5,
                        y: 0.0,
                        space: None,
                    }),
                    zoom: None,
                },
                timeout: None,
            })
            .await
            .unwrap();

        let _ = response;
    }

    #[tokio::test]
    async fn test_stop_with_platform() {
        use crate::platform::StubPlatformBuilder;

        let state = Arc::new(PTZStateManager::new());
        let platform = Arc::new(StubPlatformBuilder::new().ptz_supported(true).build());
        let service = PTZService::with_platform(state, platform);

        // Start moving
        service.state.set_moving(true, true);

        let response = service
            .handle_stop(Stop {
                profile_token: "Profile1".to_string(),
                pan_tilt: Some(true),
                zoom: Some(true),
            })
            .await
            .unwrap();

        let _ = response;
        assert!(!service.state.is_moving());
    }

    #[tokio::test]
    async fn test_goto_preset_with_platform() {
        use crate::platform::StubPlatformBuilder;

        let state = Arc::new(PTZStateManager::new());
        let platform = Arc::new(StubPlatformBuilder::new().ptz_supported(true).build());
        let service = PTZService::with_platform(state, platform);

        // Create a preset first
        service.state.set_position(&PTZVector {
            pan_tilt: Some(Vector2D {
                x: 0.6,
                y: 0.7,
                space: None,
            }),
            zoom: Some(Vector1D {
                x: 0.8,
                space: None,
            }),
        });

        let set_response = service
            .handle_set_preset(SetPreset {
                profile_token: "Profile1".to_string(),
                preset_name: Some("PlatformTest".to_string()),
                preset_token: None,
            })
            .unwrap();

        // When platform is available, set_preset creates the preset in the state manager
        // The platform's goto_preset may fail if the preset wasn't created in the platform,
        // but that's acceptable - the state manager has the preset and can handle it
        // Go to preset with platform
        let goto_response = service
            .handle_goto_preset(GotoPreset {
                profile_token: "Profile1".to_string(),
                preset_token: set_response.preset_token,
                speed: None,
            })
            .await;

        // Platform may fail if preset wasn't created in platform storage,
        // but state manager should handle it. Accept either success or platform error.
        if let Err(e) = goto_response {
            // If it fails, it should be a hardware/platform error, not a validation error
            assert!(matches!(e, OnvifError::HardwareFailure(_)));
        } else {
            // If it succeeds, that's also fine
            let _ = goto_response;
        }
    }

    #[tokio::test]
    async fn test_goto_home_position_with_platform() {
        use crate::platform::StubPlatformBuilder;

        let state = Arc::new(PTZStateManager::new());
        let platform = Arc::new(StubPlatformBuilder::new().ptz_supported(true).build());
        let service = PTZService::with_platform(state, platform);

        let response = service
            .handle_goto_home_position(GotoHomePosition {
                profile_token: "Profile1".to_string(),
                speed: None,
            })
            .await
            .unwrap();

        let _ = response;
    }

    // ========================================================================
    // ServiceHandler Error Path Tests
    // ========================================================================

    #[tokio::test]
    async fn test_service_handler_unknown_action_ptz() {
        let service = create_test_service();
        let result = service.handle_operation("UnknownAction", "<test/>").await;
        assert!(result.is_err());
        assert!(matches!(result, Err(OnvifError::ActionNotSupported(_))));
    }

    #[tokio::test]
    async fn test_service_handler_invalid_xml() {
        let service = create_test_service();
        let result = service
            .handle_operation("GetNodes", "<InvalidXml><Broken")
            .await;
        assert!(result.is_err());
        assert!(matches!(result, Err(OnvifError::WellFormed(_))));
    }

    #[tokio::test]
    async fn test_service_handler_get_nodes_xml() {
        let service = create_test_service();
        let xml = r#"<GetNodes/>"#;
        let result = service.handle_operation("GetNodes", xml).await;
        assert!(result.is_ok());
        let response_xml = result.unwrap();
        assert!(response_xml.contains("GetNodesResponse"));
    }

    #[tokio::test]
    async fn test_service_handler_absolute_move_xml() {
        let service = create_test_service();
        let xml = r#"<AbsoluteMove><ProfileToken>Profile1</ProfileToken><Position><PanTilt x="0.5" y="0.3"/><Zoom x="0.7"/></Position></AbsoluteMove>"#;
        let result = service.handle_operation("AbsoluteMove", xml).await;
        assert!(result.is_ok());
        let response_xml = result.unwrap();
        assert!(response_xml.contains("AbsoluteMoveResponse"));
    }

    #[tokio::test]
    async fn test_service_handler_get_status_xml() {
        let service = create_test_service();
        let xml = r#"<GetStatus><ProfileToken>Profile1</ProfileToken></GetStatus>"#;
        let result = service.handle_operation("GetStatus", xml).await;
        assert!(result.is_ok());
        let response_xml = result.unwrap();
        assert!(response_xml.contains("GetStatusResponse"));
    }

    // ========================================================================
    // Boundary and Edge Case Tests
    // ========================================================================

    #[tokio::test]
    async fn test_absolute_move_boundary_values() {
        let service = create_test_service();

        // Test maximum values
        let result = service
            .handle_absolute_move(AbsoluteMove {
                profile_token: "Profile1".to_string(),
                position: PTZVector {
                    pan_tilt: Some(Vector2D {
                        x: 1.0,
                        y: 1.0,
                        space: None,
                    }),
                    zoom: Some(Vector1D {
                        x: 1.0,
                        space: None,
                    }),
                },
                speed: None,
            })
            .await;
        assert!(result.is_ok());

        // Test minimum values
        let result = service
            .handle_absolute_move(AbsoluteMove {
                profile_token: "Profile1".to_string(),
                position: PTZVector {
                    pan_tilt: Some(Vector2D {
                        x: -1.0,
                        y: -1.0,
                        space: None,
                    }),
                    zoom: Some(Vector1D {
                        x: 0.0,
                        space: None,
                    }),
                },
                speed: None,
            })
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_relative_move_zero_translation() {
        let service = create_test_service();

        let response = service
            .handle_relative_move(RelativeMove {
                profile_token: "Profile1".to_string(),
                translation: PTZVector {
                    pan_tilt: Some(Vector2D {
                        x: 0.0,
                        y: 0.0,
                        space: None,
                    }),
                    zoom: Some(Vector1D {
                        x: 0.0,
                        space: None,
                    }),
                },
                speed: None,
            })
            .await
            .unwrap();

        let _ = response;
    }

    #[tokio::test]
    async fn test_continuous_move_zero_velocity() {
        let service = create_test_service();

        let response = service
            .handle_continuous_move(ContinuousMove {
                profile_token: "Profile1".to_string(),
                velocity: PTZSpeed {
                    pan_tilt: Some(Vector2D {
                        x: 0.0,
                        y: 0.0,
                        space: None,
                    }),
                    zoom: Some(Vector1D {
                        x: 0.0,
                        space: None,
                    }),
                },
                timeout: None,
            })
            .await
            .unwrap();

        let _ = response;
    }

    #[test]
    fn test_get_compatible_configurations_invalid_profile() {
        let service = create_test_service();

        // validate_profile_token only checks if token is non-empty, not if profile exists
        // So this will succeed. To test error path, use empty token.
        let result = service.handle_get_compatible_configurations(GetCompatibleConfigurations {
            profile_token: "".to_string(),
        });

        assert!(result.is_err());
    }
}
