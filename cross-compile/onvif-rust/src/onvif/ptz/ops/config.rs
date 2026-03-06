//! PTZ configuration and node operations.
//!
//! This module handles configuration and node-related PTZ operations:
//! - GetNodes: Get all PTZ nodes
//! - GetNode: Get a specific PTZ node
//! - GetConfigurations: Get all PTZ configurations
//! - GetConfiguration: Get a specific PTZ configuration
//! - SetConfiguration: Set a PTZ configuration
//! - GetConfigurationOptions: Get configuration options

use crate::onvif::error::{OnvifError, OnvifResult};
use crate::onvif::types::common::{
    PTZConfiguration, PTZSpeed, PanTiltLimits, Vector1D, Vector2D, ZoomLimits,
};
use crate::onvif::types::ptz::{
    GetConfigurationOptionsResponse, GetConfigurationResponse, GetConfigurationsResponse,
    GetNodeResponse, GetNodesResponse, PTZNode,
};

use crate::onvif::ptz::state::PTZStateManager;
use crate::onvif::ptz::types::{
    DEFAULT_CONFIG_TOKEN, DEFAULT_NODE_TOKEN, DEFAULT_PTZ_TIMEOUT, MAX_PRESETS,
    SPACE_ABSOLUTE_PAN_TILT, SPACE_ABSOLUTE_ZOOM, build_configuration_options, build_ptz_spaces,
};

/// Validate configuration token.
pub fn validate_config_token(token: &str) -> OnvifResult<()> {
    if token != DEFAULT_CONFIG_TOKEN {
        return Err(OnvifError::InvalidArgVal {
            subcode: "ter:InvalidToken".to_string(),
            reason: format!("Configuration '{}' not found", token),
        });
    }
    Ok(())
}

/// Validate node token.
pub fn validate_node_token(token: &str) -> OnvifResult<()> {
    if token != DEFAULT_NODE_TOKEN {
        return Err(OnvifError::InvalidArgVal {
            subcode: "ter:InvalidToken".to_string(),
            reason: format!("Node '{}' not found", token),
        });
    }
    Ok(())
}

/// Build default PTZ node.
pub fn build_ptz_node() -> PTZNode {
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
pub fn build_ptz_configuration() -> PTZConfiguration {
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

/// Handle GetNodes request.
pub fn get_nodes(_state: &PTZStateManager) -> OnvifResult<GetNodesResponse> {
    tracing::debug!("GetNodes request");

    Ok(GetNodesResponse {
        ptz_nodes: vec![build_ptz_node()],
    })
}

/// Handle GetNode request.
pub fn get_node(_state: &PTZStateManager, node_token: String) -> OnvifResult<GetNodeResponse> {
    tracing::debug!("GetNode request for {}", node_token);

    validate_node_token(&node_token)?;

    Ok(GetNodeResponse {
        ptz_node: build_ptz_node(),
    })
}

/// Handle GetConfigurations request.
pub fn get_configurations(_state: &PTZStateManager) -> OnvifResult<GetConfigurationsResponse> {
    tracing::debug!("GetConfigurations request");

    Ok(GetConfigurationsResponse {
        ptz_configurations: vec![build_ptz_configuration()],
    })
}

/// Handle GetConfiguration request.
pub fn get_configuration(
    _state: &PTZStateManager,
    ptz_configuration_token: String,
) -> OnvifResult<GetConfigurationResponse> {
    tracing::debug!("GetConfiguration request for {}", ptz_configuration_token);

    validate_config_token(&ptz_configuration_token)?;

    Ok(GetConfigurationResponse {
        ptz_configuration: build_ptz_configuration(),
    })
}

/// Handle SetConfiguration request.
///
/// Not supported - returns ActionNotSupported error.
pub fn set_configuration(
    _state: &PTZStateManager,
    ptz_configuration: PTZConfiguration,
) -> OnvifResult<()> {
    tracing::debug!(
        "SetConfiguration request for {} (not supported)",
        ptz_configuration.token
    );

    validate_config_token(&ptz_configuration.token)?;

    Err(OnvifError::ActionNotSupported(
        "SetConfiguration".to_string(),
    ))
}

/// Handle GetConfigurationOptions request.
pub fn get_configuration_options(
    _state: &PTZStateManager,
    configuration_token: String,
) -> OnvifResult<GetConfigurationOptionsResponse> {
    tracing::debug!(
        "GetConfigurationOptions request for {}",
        configuration_token
    );

    validate_config_token(&configuration_token)?;

    Ok(GetConfigurationOptionsResponse {
        ptz_configuration_options: build_configuration_options(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn create_test_state() -> Arc<PTZStateManager> {
        Arc::new(PTZStateManager::new())
    }

    #[test]
    fn test_get_nodes() {
        let state = create_test_state();

        let response = get_nodes(&state).unwrap();

        assert_eq!(response.ptz_nodes.len(), 1);
        assert_eq!(response.ptz_nodes[0].token, DEFAULT_NODE_TOKEN);
        assert!(response.ptz_nodes[0].home_supported);
    }

    #[test]
    fn test_get_node() {
        let state = create_test_state();

        let response = get_node(&state, DEFAULT_NODE_TOKEN.to_string()).unwrap();

        assert_eq!(response.ptz_node.token, DEFAULT_NODE_TOKEN);
        assert_eq!(response.ptz_node.maximum_number_of_presets, MAX_PRESETS);
    }

    #[test]
    fn test_get_node_invalid_token() {
        let state = create_test_state();

        let result = get_node(&state, "InvalidToken".to_string());

        assert!(result.is_err());
    }

    #[test]
    fn test_get_configurations() {
        let state = create_test_state();

        let response = get_configurations(&state).unwrap();

        assert_eq!(response.ptz_configurations.len(), 1);
        assert_eq!(response.ptz_configurations[0].token, DEFAULT_CONFIG_TOKEN);
    }

    #[test]
    fn test_get_configuration() {
        let state = create_test_state();

        let response = get_configuration(&state, DEFAULT_CONFIG_TOKEN.to_string()).unwrap();

        assert_eq!(response.ptz_configuration.token, DEFAULT_CONFIG_TOKEN);
        assert_eq!(response.ptz_configuration.node_token, DEFAULT_NODE_TOKEN);
    }

    #[test]
    fn test_get_configuration_options() {
        let state = create_test_state();

        let response = get_configuration_options(&state, DEFAULT_CONFIG_TOKEN.to_string()).unwrap();

        // Verify spaces are present
        assert!(
            !response
                .ptz_configuration_options
                .spaces
                .absolute_pan_tilt_position_space
                .is_empty()
        );
    }

    #[test]
    fn test_set_configuration_not_supported() {
        let state = create_test_state();

        let config = build_ptz_configuration();

        let result = set_configuration(&state, config);

        assert!(result.is_err());
        assert!(matches!(result, Err(OnvifError::ActionNotSupported(_))));
    }

    #[test]
    fn test_invalid_config_token() {
        let state = create_test_state();

        let result = get_configuration(&state, "NonExistentConfig".to_string());

        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_config_token_for_options() {
        let state = create_test_state();

        let result = get_configuration_options(&state, "InvalidConfig".to_string());

        assert!(result.is_err());
    }
}
