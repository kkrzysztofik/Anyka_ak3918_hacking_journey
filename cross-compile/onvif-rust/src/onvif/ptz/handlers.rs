//! PTZ Service request handlers.
//!
//! This module implements the ONVIF PTZ Service operation handlers including:
//! - Node and configuration discovery
//! - Movement operations (absolute, relative, continuous, stop)
//! - Preset management
//! - Home position management
//! - Service capabilities

use std::sync::Arc;

use crate::config::ConfigRuntime;
use crate::onvif::dispatcher::ServiceHandler;
use crate::onvif::error::{OnvifError, OnvifResult};
use crate::onvif::types::common::{
    PTZConfiguration, PTZSpeed, PTZVector, PanTiltLimits, Vector1D, Vector2D, ZoomLimits,
};
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
use crate::platform::{PTZControl, Platform, PtzPosition, PtzVelocity};

use super::state::PTZStateManager;
use super::types::{
    DEFAULT_CONFIG_TOKEN, DEFAULT_NODE_TOKEN, DEFAULT_PTZ_TIMEOUT, MAX_PRESETS,
    SPACE_ABSOLUTE_PAN_TILT, SPACE_ABSOLUTE_ZOOM, build_configuration_options, build_ptz_spaces,
    build_service_capabilities,
};

// ============================================================================
// PTZService
// ============================================================================

/// ONVIF PTZ Service.
///
/// Handles PTZ Service operations including:
/// - Node discovery and configuration
/// - Movement operations
/// - Preset management
/// - Home position management
pub struct PTZService {
    /// PTZ state manager.
    state: Arc<PTZStateManager>,
    /// Configuration runtime.
    #[allow(dead_code)]
    config: Arc<ConfigRuntime>,
    /// Platform PTZ control (optional for software-only mode).
    ptz_control: Option<Arc<dyn PTZControl>>,
}

impl PTZService {
    /// Create a new PTZ Service.
    pub fn new(state: Arc<PTZStateManager>, config: Arc<ConfigRuntime>) -> Self {
        Self {
            state,
            config,
            ptz_control: None,
        }
    }

    /// Create a new PTZ Service with platform PTZ control.
    pub fn with_platform(
        state: Arc<PTZStateManager>,
        config: Arc<ConfigRuntime>,
        platform: Arc<dyn Platform>,
    ) -> Self {
        Self {
            state,
            config,
            ptz_control: platform.ptz_control(),
        }
    }

    /// Create a new PTZ Service with direct PTZ control.
    pub fn with_ptz_control(
        state: Arc<PTZStateManager>,
        config: Arc<ConfigRuntime>,
        ptz_control: Arc<dyn PTZControl>,
    ) -> Self {
        Self {
            state,
            config,
            ptz_control: Some(ptz_control),
        }
    }

    // ========================================================================
    // Helper Methods
    // ========================================================================

    /// Validate profile token.
    fn validate_profile_token(&self, token: &str) -> OnvifResult<()> {
        if token.is_empty() {
            return Err(OnvifError::InvalidArgVal {
                subcode: "ter:NoToken".to_string(),
                reason: "Profile token is required".to_string(),
            });
        }
        // For now, accept any non-empty token
        // In production, this would validate against actual profile tokens
        Ok(())
    }

    /// Validate configuration token.
    fn validate_config_token(&self, token: &str) -> OnvifResult<()> {
        if token != DEFAULT_CONFIG_TOKEN {
            return Err(OnvifError::InvalidArgVal {
                subcode: "ter:InvalidToken".to_string(),
                reason: format!("Configuration '{}' not found", token),
            });
        }
        Ok(())
    }

    /// Validate node token.
    fn validate_node_token(&self, token: &str) -> OnvifResult<()> {
        if token != DEFAULT_NODE_TOKEN {
            return Err(OnvifError::InvalidArgVal {
                subcode: "ter:InvalidToken".to_string(),
                reason: format!("Node '{}' not found", token),
            });
        }
        Ok(())
    }

    /// Build default PTZ node.
    fn build_ptz_node(&self) -> PTZNode {
        PTZNode {
            token: DEFAULT_NODE_TOKEN.to_string(),
            fixed_home_position: Some(false),
            geo_move: Some(false),
            name: Some("PTZ Node".to_string()),
            supported_ptz_spaces: build_ptz_spaces(),
            maximum_number_of_presets: MAX_PRESETS,
            home_supported: true,
            auxiliary_commands: vec![],
            extension: None,
        }
    }

    /// Build default PTZ configuration.
    fn build_ptz_configuration(&self) -> PTZConfiguration {
        PTZConfiguration {
            token: DEFAULT_CONFIG_TOKEN.to_string(),
            name: "PTZ Configuration".to_string(),
            use_count: 1,
            node_token: DEFAULT_NODE_TOKEN.to_string(),
            default_absolute_pan_tilt_position_space: Some(SPACE_ABSOLUTE_PAN_TILT.to_string()),
            default_absolute_zoom_position_space: Some(SPACE_ABSOLUTE_ZOOM.to_string()),
            default_relative_pan_tilt_translation_space: None,
            default_relative_zoom_translation_space: None,
            default_continuous_pan_tilt_velocity_space: None,
            default_continuous_zoom_velocity_space: None,
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
            default_ptz_timeout: Some(DEFAULT_PTZ_TIMEOUT.to_string()),
            pan_tilt_limits: Some(PanTiltLimits {
                range: crate::onvif::types::common::Space2DDescription {
                    uri: SPACE_ABSOLUTE_PAN_TILT.to_string(),
                    x_range: crate::onvif::types::common::FloatRange {
                        min: -1.0,
                        max: 1.0,
                    },
                    y_range: crate::onvif::types::common::FloatRange {
                        min: -1.0,
                        max: 1.0,
                    },
                },
            }),
            zoom_limits: Some(ZoomLimits {
                range: crate::onvif::types::common::Space1DDescription {
                    uri: SPACE_ABSOLUTE_ZOOM.to_string(),
                    x_range: crate::onvif::types::common::FloatRange { min: 0.0, max: 1.0 },
                },
            }),
            extension: None,
            move_ramp: None,
            preset_ramp: None,
            preset_tour_ramp: None,
        }
    }

    /// Convert PTZVector to platform PtzPosition.
    #[allow(dead_code)]
    fn vector_to_position(vector: &PTZVector) -> PtzPosition {
        let mut pos = PtzPosition::default();
        if let Some(pt) = &vector.pan_tilt {
            // ONVIF uses -1 to 1, platform uses degrees
            pos.pan = pt.x * 180.0;
            pos.tilt = pt.y * 90.0;
        }
        if let Some(z) = &vector.zoom {
            // ONVIF uses 0 to 1, platform uses 1 to 10
            pos.zoom = z.x * 9.0 + 1.0;
        }
        pos
    }

    /// Convert platform PtzPosition to PTZVector.
    #[allow(dead_code)]
    fn position_to_vector(pos: &PtzPosition) -> PTZVector {
        PTZVector {
            pan_tilt: Some(Vector2D {
                x: pos.pan / 180.0,
                y: pos.tilt / 90.0,
                space: Some(SPACE_ABSOLUTE_PAN_TILT.to_string()),
            }),
            zoom: Some(Vector1D {
                x: (pos.zoom - 1.0) / 9.0,
                space: Some(SPACE_ABSOLUTE_ZOOM.to_string()),
            }),
        }
    }

    /// Convert PTZSpeed to platform PtzVelocity.
    fn speed_to_velocity(speed: &PTZSpeed) -> PtzVelocity {
        let mut vel = PtzVelocity::STOP;
        if let Some(pt) = &speed.pan_tilt {
            vel.pan = pt.x;
            vel.tilt = pt.y;
        }
        if let Some(z) = &speed.zoom {
            vel.zoom = z.x;
        }
        vel
    }

    // ========================================================================
    // Node Operations
    // ========================================================================

    /// Handle GetNodes request.
    pub fn handle_get_nodes(&self, _request: GetNodes) -> OnvifResult<GetNodesResponse> {
        tracing::debug!("GetNodes request");

        Ok(GetNodesResponse {
            ptz_nodes: vec![self.build_ptz_node()],
        })
    }

    /// Handle GetNode request.
    pub fn handle_get_node(&self, request: GetNode) -> OnvifResult<GetNodeResponse> {
        tracing::debug!("GetNode request for {}", request.node_token);

        self.validate_node_token(&request.node_token)?;

        Ok(GetNodeResponse {
            ptz_node: self.build_ptz_node(),
        })
    }

    // ========================================================================
    // Configuration Operations
    // ========================================================================

    /// Handle GetConfigurations request.
    pub fn handle_get_configurations(
        &self,
        _request: GetConfigurations,
    ) -> OnvifResult<GetConfigurationsResponse> {
        tracing::debug!("GetConfigurations request");

        Ok(GetConfigurationsResponse {
            ptz_configurations: vec![self.build_ptz_configuration()],
        })
    }

    /// Handle GetConfiguration request.
    pub fn handle_get_configuration(
        &self,
        request: GetConfiguration,
    ) -> OnvifResult<GetConfigurationResponse> {
        tracing::debug!(
            "GetConfiguration request for {}",
            request.ptz_configuration_token
        );

        self.validate_config_token(&request.ptz_configuration_token)?;

        Ok(GetConfigurationResponse {
            ptz_configuration: self.build_ptz_configuration(),
        })
    }

    /// Handle SetConfiguration request.
    pub fn handle_set_configuration(
        &self,
        request: SetConfiguration,
    ) -> OnvifResult<SetConfigurationResponse> {
        tracing::debug!(
            "SetConfiguration request for {}",
            request.ptz_configuration.token
        );

        self.validate_config_token(&request.ptz_configuration.token)?;

        // TODO: Actually update configuration if persistent
        // For now, we accept the request but don't persist changes

        Ok(SetConfigurationResponse {})
    }

    /// Handle GetConfigurationOptions request.
    pub fn handle_get_configuration_options(
        &self,
        request: GetConfigurationOptions,
    ) -> OnvifResult<GetConfigurationOptionsResponse> {
        tracing::debug!(
            "GetConfigurationOptions request for {}",
            request.configuration_token
        );

        self.validate_config_token(&request.configuration_token)?;

        Ok(GetConfigurationOptionsResponse {
            ptz_configuration_options: build_configuration_options(),
        })
    }

    // ========================================================================
    // Movement Operations
    // ========================================================================

    /// Handle AbsoluteMove request.
    pub async fn handle_absolute_move(
        &self,
        request: AbsoluteMove,
    ) -> OnvifResult<AbsoluteMoveResponse> {
        tracing::debug!("AbsoluteMove request for profile {}", request.profile_token);

        self.validate_profile_token(&request.profile_token)?;

        // Update state
        self.state.set_position(&request.position);
        self.state.set_moving(true, true);

        // Call platform if available
        if let Some(ref ptz) = self.ptz_control {
            let pos = Self::vector_to_position(&request.position);
            ptz.move_to_position(pos).await.map_err(|e| {
                self.state.stop();
                OnvifError::HardwareFailure(format!("PTZ move failed: {}", e))
            })?;
        }

        // Mark as stopped (instant move for stub)
        self.state.stop();

        Ok(AbsoluteMoveResponse {})
    }

    /// Handle RelativeMove request.
    pub async fn handle_relative_move(
        &self,
        request: RelativeMove,
    ) -> OnvifResult<RelativeMoveResponse> {
        tracing::debug!("RelativeMove request for profile {}", request.profile_token);

        self.validate_profile_token(&request.profile_token)?;

        // Get current position and apply translation
        let current = self.state.get_position();
        let mut new_pos = current.clone();

        if let (Some(cur_pt), Some(trans_pt)) = (&current.pan_tilt, &request.translation.pan_tilt) {
            new_pos.pan_tilt = Some(Vector2D {
                x: (cur_pt.x + trans_pt.x).clamp(-1.0, 1.0),
                y: (cur_pt.y + trans_pt.y).clamp(-1.0, 1.0),
                space: cur_pt.space.clone(),
            });
        }

        if let (Some(cur_z), Some(trans_z)) = (&current.zoom, &request.translation.zoom) {
            new_pos.zoom = Some(Vector1D {
                x: (cur_z.x + trans_z.x).clamp(0.0, 1.0),
                space: cur_z.space.clone(),
            });
        }

        // Update state
        self.state.set_position(&new_pos);
        self.state.set_moving(true, true);

        // Call platform if available
        if let Some(ref ptz) = self.ptz_control {
            let pos = Self::vector_to_position(&new_pos);
            ptz.move_to_position(pos).await.map_err(|e| {
                self.state.stop();
                OnvifError::HardwareFailure(format!("PTZ relative move failed: {}", e))
            })?;
        }

        // Mark as stopped
        self.state.stop();

        Ok(RelativeMoveResponse {})
    }

    /// Handle ContinuousMove request.
    pub async fn handle_continuous_move(
        &self,
        request: ContinuousMove,
    ) -> OnvifResult<ContinuousMoveResponse> {
        tracing::debug!(
            "ContinuousMove request for profile {}",
            request.profile_token
        );

        self.validate_profile_token(&request.profile_token)?;

        // Set moving state
        let pan_tilt_moving = request
            .velocity
            .pan_tilt
            .as_ref()
            .is_some_and(|pt| pt.x != 0.0 || pt.y != 0.0);
        let zoom_moving = request.velocity.zoom.as_ref().is_some_and(|z| z.x != 0.0);
        self.state.set_moving(pan_tilt_moving, zoom_moving);

        // Call platform if available
        if let Some(ref ptz) = self.ptz_control {
            let vel = Self::speed_to_velocity(&request.velocity);
            ptz.continuous_move(vel).await.map_err(|e| {
                self.state.stop();
                OnvifError::HardwareFailure(format!("PTZ continuous move failed: {}", e))
            })?;
        }

        Ok(ContinuousMoveResponse {})
    }

    /// Handle Stop request.
    pub async fn handle_stop(&self, request: Stop) -> OnvifResult<StopResponse> {
        tracing::debug!("Stop request for profile {}", request.profile_token);

        self.validate_profile_token(&request.profile_token)?;

        let stop_pan_tilt = request.pan_tilt.unwrap_or(true);
        let stop_zoom = request.zoom.unwrap_or(true);

        // Update state
        if stop_pan_tilt && stop_zoom {
            self.state.stop();
        } else {
            let mut pan_tilt_moving = self.state.is_moving();
            let mut zoom_moving = self.state.is_moving();

            if stop_pan_tilt {
                pan_tilt_moving = false;
            }
            if stop_zoom {
                zoom_moving = false;
            }
            self.state.set_moving(pan_tilt_moving, zoom_moving);
        }

        // Call platform if available
        if let Some(ref ptz) = self.ptz_control {
            ptz.stop()
                .await
                .map_err(|e| OnvifError::HardwareFailure(format!("PTZ stop failed: {}", e)))?;
        }

        Ok(StopResponse {})
    }

    /// Handle GetStatus request.
    pub fn handle_get_status(&self, request: GetStatus) -> OnvifResult<GetStatusResponse> {
        tracing::debug!("GetStatus request for profile {}", request.profile_token);

        self.validate_profile_token(&request.profile_token)?;

        Ok(GetStatusResponse {
            ptz_status: self.state.get_status(),
        })
    }

    // ========================================================================
    // Home Position Operations
    // ========================================================================

    /// Handle GotoHomePosition request.
    pub async fn handle_goto_home_position(
        &self,
        request: GotoHomePosition,
    ) -> OnvifResult<GotoHomePositionResponse> {
        tracing::debug!(
            "GotoHomePosition request for profile {}",
            request.profile_token
        );

        self.validate_profile_token(&request.profile_token)?;

        // Get home position and move there
        self.state.set_moving(true, true);
        self.state.goto_home();

        // Call platform if available
        if let Some(ref ptz) = self.ptz_control {
            let home_pos = self.state.get_position();
            let pos = Self::vector_to_position(&home_pos);
            ptz.move_to_position(pos).await.map_err(|e| {
                self.state.stop();
                OnvifError::HardwareFailure(format!("PTZ goto home failed: {}", e))
            })?;
        }

        self.state.stop();

        Ok(GotoHomePositionResponse {})
    }

    /// Handle SetHomePosition request.
    pub fn handle_set_home_position(
        &self,
        request: SetHomePosition,
    ) -> OnvifResult<SetHomePositionResponse> {
        tracing::debug!(
            "SetHomePosition request for profile {}",
            request.profile_token
        );

        self.validate_profile_token(&request.profile_token)?;

        self.state.set_home_position();

        Ok(SetHomePositionResponse {})
    }

    // ========================================================================
    // Preset Operations
    // ========================================================================

    /// Handle GetPresets request.
    pub fn handle_get_presets(&self, request: GetPresets) -> OnvifResult<GetPresetsResponse> {
        tracing::debug!("GetPresets request for profile {}", request.profile_token);

        self.validate_profile_token(&request.profile_token)?;

        Ok(GetPresetsResponse {
            presets: self.state.get_presets(),
        })
    }

    /// Handle SetPreset request.
    pub fn handle_set_preset(&self, request: SetPreset) -> OnvifResult<SetPresetResponse> {
        tracing::debug!("SetPreset request for profile {}", request.profile_token);

        self.validate_profile_token(&request.profile_token)?;

        let name = request.preset_name.unwrap_or_else(|| "Unnamed".to_string());
        let token = self.state.set_preset(name, request.preset_token)?;

        Ok(SetPresetResponse {
            preset_token: token,
        })
    }

    /// Handle GotoPreset request.
    pub async fn handle_goto_preset(&self, request: GotoPreset) -> OnvifResult<GotoPresetResponse> {
        tracing::debug!(
            "GotoPreset request for profile {}, preset {}",
            request.profile_token,
            request.preset_token
        );

        self.validate_profile_token(&request.profile_token)?;

        // Set moving state
        self.state.set_moving(true, true);

        // Go to preset position
        self.state.goto_preset(&request.preset_token)?;

        // Call platform if available
        if let Some(ref ptz) = self.ptz_control {
            ptz.goto_preset(&request.preset_token).await.map_err(|e| {
                self.state.stop();
                OnvifError::HardwareFailure(format!("PTZ goto preset failed: {}", e))
            })?;
        }

        self.state.stop();

        Ok(GotoPresetResponse {})
    }

    /// Handle RemovePreset request.
    pub fn handle_remove_preset(&self, request: RemovePreset) -> OnvifResult<RemovePresetResponse> {
        tracing::debug!(
            "RemovePreset request for profile {}, preset {}",
            request.profile_token,
            request.preset_token
        );

        self.validate_profile_token(&request.profile_token)?;

        self.state.remove_preset(&request.preset_token)?;

        Ok(RemovePresetResponse {})
    }

    // ========================================================================
    // Service Capabilities
    // ========================================================================

    /// Handle GetServiceCapabilities request.
    pub fn handle_get_service_capabilities(
        &self,
        _request: GetServiceCapabilities,
    ) -> OnvifResult<GetServiceCapabilitiesResponse> {
        tracing::debug!("GetServiceCapabilities request");

        Ok(GetServiceCapabilitiesResponse {
            capabilities: build_service_capabilities(),
        })
    }

    /// Handle GetCompatibleConfigurations request.
    pub fn handle_get_compatible_configurations(
        &self,
        request: GetCompatibleConfigurations,
    ) -> OnvifResult<GetCompatibleConfigurationsResponse> {
        tracing::debug!(
            "GetCompatibleConfigurations request for profile {}",
            request.profile_token
        );

        self.validate_profile_token(&request.profile_token)?;

        // Return all configurations (we only have one)
        Ok(GetCompatibleConfigurationsResponse {
            ptz_configurations: vec![self.build_ptz_configuration()],
        })
    }

    /// Handle SendAuxiliaryCommand request.
    pub fn handle_send_auxiliary_command(
        &self,
        request: SendAuxiliaryCommand,
    ) -> OnvifResult<SendAuxiliaryCommandResponse> {
        tracing::debug!(
            "SendAuxiliaryCommand request for profile {}, command: {}",
            request.profile_token,
            request.auxiliary_data
        );

        self.validate_profile_token(&request.profile_token)?;

        // We don't support auxiliary commands, return success with empty response
        Ok(SendAuxiliaryCommandResponse {
            auxiliary_response: None,
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
                let request: GetNodes = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_get_nodes(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "GetNode" => {
                let request: GetNode = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_get_node(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            // Configuration Operations
            "GetConfigurations" => {
                let request: GetConfigurations = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_get_configurations(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "GetConfiguration" => {
                let request: GetConfiguration = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_get_configuration(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "SetConfiguration" => {
                let request: SetConfiguration = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_set_configuration(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "GetConfigurationOptions" => {
                let request: GetConfigurationOptions = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_get_configuration_options(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            // Movement Operations
            "AbsoluteMove" => {
                let request: AbsoluteMove = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_absolute_move(request).await?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "RelativeMove" => {
                let request: RelativeMove = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_relative_move(request).await?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "ContinuousMove" => {
                let request: ContinuousMove = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_continuous_move(request).await?;
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

            "GetStatus" => {
                let request: GetStatus = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_get_status(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            // Home Position Operations
            "GotoHomePosition" => {
                let request: GotoHomePosition = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_goto_home_position(request).await?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "SetHomePosition" => {
                let request: SetHomePosition = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_set_home_position(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            // Preset Operations
            "GetPresets" => {
                let request: GetPresets = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_get_presets(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "SetPreset" => {
                let request: SetPreset = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_set_preset(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "GotoPreset" => {
                let request: GotoPreset = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_goto_preset(request).await?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "RemovePreset" => {
                let request: RemovePreset = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_remove_preset(request)?;
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

            "GetCompatibleConfigurations" => {
                let request: GetCompatibleConfigurations = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
                let response = self.handle_get_compatible_configurations(request)?;
                quick_xml::se::to_string(&response).map_err(|e| {
                    OnvifError::Internal(format!("Failed to serialize response: {}", e))
                })
            }

            "SendAuxiliaryCommand" => {
                let request: SendAuxiliaryCommand = quick_xml::de::from_str(body_xml)
                    .map_err(|e| OnvifError::WellFormed(format!("Invalid request XML: {}", e)))?;
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

    /// Get the list of supported actions.
    fn supported_actions(&self) -> Vec<&str> {
        vec![
            // Node Operations
            "GetNodes",
            "GetNode",
            // Configuration Operations
            "GetConfigurations",
            "GetConfiguration",
            "SetConfiguration",
            "GetConfigurationOptions",
            // Movement Operations
            "AbsoluteMove",
            "RelativeMove",
            "ContinuousMove",
            "Stop",
            "GetStatus",
            // Home Position Operations
            "GotoHomePosition",
            "SetHomePosition",
            // Preset Operations
            "GetPresets",
            "SetPreset",
            "GotoPreset",
            "RemovePreset",
            // Service Capabilities
            "GetServiceCapabilities",
            "GetCompatibleConfigurations",
            "SendAuxiliaryCommand",
        ]
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
        let config = Arc::new(ConfigRuntime::new(Default::default()));
        PTZService::new(state, config)
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

        config.token = "WrongToken".to_string();

        let result = service.handle_set_configuration(SetConfiguration {
            ptz_configuration: config,
            force_persistence: false,
        });

        assert!(result.is_err());
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

        let token = response.preset_token;

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
                preset_token: Some(token.clone()),
            })
            .unwrap();

        assert_eq!(update_response.preset_token, token);

        // Verify name was updated
        let presets = service
            .handle_get_presets(GetPresets {
                profile_token: "Profile1".to_string(),
            })
            .unwrap();

        let preset = presets
            .presets
            .iter()
            .find(|p| p.token == Some(token.clone()))
            .unwrap();
        assert_eq!(preset.name, Some("UpdatedName".to_string()));
    }
}
