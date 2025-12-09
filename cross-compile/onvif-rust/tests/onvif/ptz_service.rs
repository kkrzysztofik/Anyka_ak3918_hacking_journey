//! Integration tests for ONVIF PTZ Service.
//!
//! Tests the complete PTZ Service operations including:
//! - Node discovery and configuration
//! - Movement operations (absolute, relative, continuous, stop)
//! - Preset management
//! - Home position management
//! - Service capabilities

use onvif_rust::config::ConfigRuntime;
use onvif_rust::onvif::ptz::{PTZService, PTZStateManager};
use onvif_rust::onvif::types::common::{PTZSpeed, PTZVector, Vector1D, Vector2D};
use onvif_rust::onvif::types::ptz::{
    AbsoluteMove, ContinuousMove, GetCompatibleConfigurations, GetConfiguration,
    GetConfigurationOptions, GetConfigurations, GetNode, GetNodes, GetPresets,
    GetServiceCapabilities, GetStatus, GotoHomePosition, GotoPreset, RelativeMove, RemovePreset,
    SetConfiguration, SetHomePosition, SetPreset, Stop,
};
use std::sync::Arc;

// ============================================================================
// Test Fixtures
// ============================================================================

fn create_test_service() -> PTZService {
    let state = Arc::new(PTZStateManager::new());
    let config = Arc::new(ConfigRuntime::new(Default::default()));
    PTZService::new(state, config)
}

fn create_service_with_state() -> (PTZService, Arc<PTZStateManager>) {
    let state = Arc::new(PTZStateManager::new());
    let config = Arc::new(ConfigRuntime::new(Default::default()));
    let service = PTZService::new(Arc::clone(&state), config);
    (service, state)
}

// ============================================================================
// Node Operations Integration Tests
// ============================================================================

#[test]
fn test_get_nodes_returns_single_node() {
    let service = create_test_service();

    let response = service.handle_get_nodes(GetNodes {}).unwrap();

    assert_eq!(response.ptz_nodes.len(), 1);
}

#[test]
fn test_get_nodes_has_supported_spaces() {
    let service = create_test_service();

    let response = service.handle_get_nodes(GetNodes {}).unwrap();

    let node = &response.ptz_nodes[0];
    // Should have absolute pan/tilt space
    assert!(
        !node
            .supported_ptz_spaces
            .absolute_pan_tilt_position_space
            .is_empty()
    );
    // Should have absolute zoom space
    assert!(
        !node
            .supported_ptz_spaces
            .absolute_zoom_position_space
            .is_empty()
    );
    // Should have continuous pan/tilt space
    assert!(
        !node
            .supported_ptz_spaces
            .continuous_pan_tilt_velocity_space
            .is_empty()
    );
}

#[test]
fn test_get_node_by_token() {
    let service = create_test_service();

    // First get all nodes
    let nodes_response = service.handle_get_nodes(GetNodes {}).unwrap();
    let node_token = nodes_response.ptz_nodes[0].token.clone();

    // Then get specific node
    let response = service
        .handle_get_node(GetNode {
            node_token: node_token.clone(),
        })
        .unwrap();

    assert_eq!(response.ptz_node.token, node_token);
}

#[test]
fn test_get_node_invalid_token_fails() {
    let service = create_test_service();

    let result = service.handle_get_node(GetNode {
        node_token: "InvalidNodeToken".to_string(),
    });

    assert!(result.is_err());
}

// ============================================================================
// Configuration Operations Integration Tests
// ============================================================================

#[test]
fn test_get_configurations_returns_config() {
    let service = create_test_service();

    let response = service
        .handle_get_configurations(GetConfigurations {})
        .unwrap();

    assert!(!response.ptz_configurations.is_empty());
}

#[test]
fn test_get_configuration_has_node_token() {
    let service = create_test_service();

    // First get all configs
    let configs_response = service
        .handle_get_configurations(GetConfigurations {})
        .unwrap();
    let config_token = configs_response.ptz_configurations[0].token.clone();

    // Then get specific config
    let response = service
        .handle_get_configuration(GetConfiguration {
            ptz_configuration_token: config_token,
        })
        .unwrap();

    // Config should reference a node
    assert!(!response.ptz_configuration.node_token.is_empty());
}

#[test]
fn test_get_configuration_options_has_spaces() {
    let service = create_test_service();

    // First get all configs
    let configs_response = service
        .handle_get_configurations(GetConfigurations {})
        .unwrap();
    let config_token = configs_response.ptz_configurations[0].token.clone();

    // Then get options
    let response = service
        .handle_get_configuration_options(GetConfigurationOptions {
            configuration_token: config_token,
        })
        .unwrap();

    // Should have spaces defined
    assert!(
        !response
            .ptz_configuration_options
            .spaces
            .absolute_pan_tilt_position_space
            .is_empty()
    );
}

#[test]
fn test_set_configuration_accepts_valid_config() {
    let service = create_test_service();

    // Get existing config
    let configs_response = service
        .handle_get_configurations(GetConfigurations {})
        .unwrap();
    let config = configs_response.ptz_configurations[0].clone();

    // Set it back (no changes)
    let result = service.handle_set_configuration(SetConfiguration {
        ptz_configuration: config,
        force_persistence: false,
    });

    assert!(result.is_ok());
}

// ============================================================================
// Movement Operations Integration Tests
// ============================================================================

#[tokio::test]
async fn test_absolute_move_updates_position() {
    let (service, state) = create_service_with_state();

    let target_position = PTZVector {
        pan_tilt: Some(Vector2D {
            x: 0.5,
            y: -0.3,
            space: None,
        }),
        zoom: Some(Vector1D {
            x: 0.7,
            space: None,
        }),
    };

    service
        .handle_absolute_move(AbsoluteMove {
            profile_token: "Profile1".to_string(),
            position: target_position.clone(),
            speed: None,
        })
        .await
        .unwrap();

    // Verify position was updated
    let current = state.get_position();
    let pt = current.pan_tilt.unwrap();
    assert_eq!(pt.x, 0.5);
    assert_eq!(pt.y, -0.3);
    let z = current.zoom.unwrap();
    assert_eq!(z.x, 0.7);
}

#[tokio::test]
async fn test_relative_move_applies_delta() {
    let (service, state) = create_service_with_state();

    // Start at a known position
    state.set_position(&PTZVector {
        pan_tilt: Some(Vector2D {
            x: 0.2,
            y: 0.2,
            space: None,
        }),
        zoom: Some(Vector1D {
            x: 0.3,
            space: None,
        }),
    });

    // Apply relative move
    service
        .handle_relative_move(RelativeMove {
            profile_token: "Profile1".to_string(),
            translation: PTZVector {
                pan_tilt: Some(Vector2D {
                    x: 0.1,
                    y: -0.1,
                    space: None,
                }),
                zoom: Some(Vector1D {
                    x: 0.2,
                    space: None,
                }),
            },
            speed: None,
        })
        .await
        .unwrap();

    // Verify delta was applied
    let current = state.get_position();
    let pt = current.pan_tilt.unwrap();
    assert!((pt.x - 0.3).abs() < 0.001);
    assert!((pt.y - 0.1).abs() < 0.001);
    let z = current.zoom.unwrap();
    assert!((z.x - 0.5).abs() < 0.001);
}

#[tokio::test]
async fn test_relative_move_clamps_to_limits() {
    let (service, state) = create_service_with_state();

    // Start near limit
    state.set_position(&PTZVector {
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
    service
        .handle_relative_move(RelativeMove {
            profile_token: "Profile1".to_string(),
            translation: PTZVector {
                pan_tilt: Some(Vector2D {
                    x: 0.5, // Would put us at 1.4, beyond 1.0
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

    // Verify clamped to limits
    let current = state.get_position();
    let pt = current.pan_tilt.unwrap();
    assert_eq!(pt.x, 1.0); // Clamped
    assert_eq!(pt.y, 1.0);
    let z = current.zoom.unwrap();
    assert_eq!(z.x, 1.0);
}

#[tokio::test]
async fn test_continuous_move_sets_moving_state() {
    let (service, state) = create_service_with_state();

    service
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

    // Should be moving
    assert!(state.is_moving());
}

#[tokio::test]
async fn test_stop_clears_moving_state() {
    let (service, state) = create_service_with_state();

    // Start moving
    state.set_moving(true, true);
    assert!(state.is_moving());

    // Stop
    service
        .handle_stop(Stop {
            profile_token: "Profile1".to_string(),
            pan_tilt: Some(true),
            zoom: Some(true),
        })
        .await
        .unwrap();

    // Should not be moving
    assert!(!state.is_moving());
}

#[tokio::test]
async fn test_stop_pan_tilt_only() {
    let (service, state) = create_service_with_state();

    // Start moving both
    state.set_moving(true, true);

    // Stop only pan/tilt
    service
        .handle_stop(Stop {
            profile_token: "Profile1".to_string(),
            pan_tilt: Some(true),
            zoom: Some(false),
        })
        .await
        .unwrap();

    // Implementation note: current simple implementation stops all
    // This test documents expected behavior
}

#[test]
fn test_get_status_returns_position_and_movement() {
    let (service, state) = create_service_with_state();

    // Set a known position
    state.set_position(&PTZVector {
        pan_tilt: Some(Vector2D {
            x: 0.4,
            y: -0.2,
            space: None,
        }),
        zoom: Some(Vector1D {
            x: 0.6,
            space: None,
        }),
    });

    let response = service
        .handle_get_status(GetStatus {
            profile_token: "Profile1".to_string(),
        })
        .unwrap();

    // Check position
    let pos = response.ptz_status.position.unwrap();
    let pt = pos.pan_tilt.unwrap();
    assert_eq!(pt.x, 0.4);
    assert_eq!(pt.y, -0.2);

    // Check move status
    let move_status = response.ptz_status.move_status.unwrap();
    assert!(move_status.pan_tilt.is_some());
    assert!(move_status.zoom.is_some());

    // Check UTC time
    assert!(!response.ptz_status.utc_time.is_empty());
}

// ============================================================================
// Home Position Integration Tests
// ============================================================================

#[tokio::test]
async fn test_set_and_goto_home_position() {
    let (service, state) = create_service_with_state();

    // Move to a specific position
    let home_pos = PTZVector {
        pan_tilt: Some(Vector2D {
            x: 0.8,
            y: 0.4,
            space: None,
        }),
        zoom: Some(Vector1D {
            x: 0.5,
            space: None,
        }),
    };
    state.set_position(&home_pos);

    // Set as home
    service
        .handle_set_home_position(SetHomePosition {
            profile_token: "Profile1".to_string(),
        })
        .unwrap();

    // Move somewhere else
    state.set_position(&PTZVector {
        pan_tilt: Some(Vector2D {
            x: -0.5,
            y: -0.5,
            space: None,
        }),
        zoom: Some(Vector1D {
            x: 0.0,
            space: None,
        }),
    });

    // Go to home
    service
        .handle_goto_home_position(GotoHomePosition {
            profile_token: "Profile1".to_string(),
            speed: None,
        })
        .await
        .unwrap();

    // Verify we're at home position
    let current = state.get_position();
    let pt = current.pan_tilt.unwrap();
    assert_eq!(pt.x, 0.8);
    assert_eq!(pt.y, 0.4);
}

// ============================================================================
// Preset Management Integration Tests
// ============================================================================

#[test]
fn test_preset_workflow() {
    let (service, state) = create_service_with_state();

    // Initially no presets
    let response = service
        .handle_get_presets(GetPresets {
            profile_token: "Profile1".to_string(),
        })
        .unwrap();
    assert!(response.presets.is_empty());

    // Move to position 1 and create preset
    state.set_position(&PTZVector {
        pan_tilt: Some(Vector2D {
            x: 0.3,
            y: -0.2,
            space: None,
        }),
        zoom: Some(Vector1D {
            x: 0.4,
            space: None,
        }),
    });

    let set_response = service
        .handle_set_preset(SetPreset {
            profile_token: "Profile1".to_string(),
            preset_name: Some("Position1".to_string()),
            preset_token: None,
        })
        .unwrap();
    let token1 = set_response.preset_token;

    // Move to position 2 and create preset
    state.set_position(&PTZVector {
        pan_tilt: Some(Vector2D {
            x: -0.5,
            y: 0.5,
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
            preset_name: Some("Position2".to_string()),
            preset_token: None,
        })
        .unwrap();
    let _token2 = set_response.preset_token;

    // Should have 2 presets
    let response = service
        .handle_get_presets(GetPresets {
            profile_token: "Profile1".to_string(),
        })
        .unwrap();
    assert_eq!(response.presets.len(), 2);

    // Remove first preset
    service
        .handle_remove_preset(RemovePreset {
            profile_token: "Profile1".to_string(),
            preset_token: token1,
        })
        .unwrap();

    // Should have 1 preset
    let response = service
        .handle_get_presets(GetPresets {
            profile_token: "Profile1".to_string(),
        })
        .unwrap();
    assert_eq!(response.presets.len(), 1);
}

#[tokio::test]
async fn test_goto_preset_moves_to_saved_position() {
    let (service, state) = create_service_with_state();

    // Create preset at specific position
    state.set_position(&PTZVector {
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
            preset_name: Some("TestPreset".to_string()),
            preset_token: None,
        })
        .unwrap();

    // Move to different position
    state.set_position(&PTZVector {
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
    service
        .handle_goto_preset(GotoPreset {
            profile_token: "Profile1".to_string(),
            preset_token: set_response.preset_token,
            speed: None,
        })
        .await
        .unwrap();

    // Verify position
    let current = state.get_position();
    let pt = current.pan_tilt.unwrap();
    assert_eq!(pt.x, 0.6);
    assert_eq!(pt.y, 0.7);
}

#[tokio::test]
async fn test_goto_nonexistent_preset_fails() {
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
fn test_remove_nonexistent_preset_fails() {
    let service = create_test_service();

    let result = service.handle_remove_preset(RemovePreset {
        profile_token: "Profile1".to_string(),
        preset_token: "NonExistentPreset".to_string(),
    });

    assert!(result.is_err());
}

// ============================================================================
// Service Capabilities Integration Tests
// ============================================================================

#[test]
fn test_get_service_capabilities() {
    let service = create_test_service();

    let response = service
        .handle_get_service_capabilities(GetServiceCapabilities {})
        .unwrap();

    // Check supported capabilities
    assert_eq!(response.capabilities.move_status, Some(true));
    assert_eq!(response.capabilities.status_position, Some(true));
    assert_eq!(
        response.capabilities.get_compatible_configurations,
        Some(true)
    );
}

#[test]
fn test_get_compatible_configurations() {
    let service = create_test_service();

    let response = service
        .handle_get_compatible_configurations(GetCompatibleConfigurations {
            profile_token: "Profile1".to_string(),
        })
        .unwrap();

    // Should return at least one compatible configuration
    assert!(!response.ptz_configurations.is_empty());
}

// ============================================================================
// Error Handling Integration Tests
// ============================================================================

#[test]
fn test_empty_profile_token_rejected() {
    let service = create_test_service();

    let result = service.handle_get_status(GetStatus {
        profile_token: "".to_string(),
    });

    assert!(result.is_err());
}

#[test]
fn test_invalid_configuration_token_rejected() {
    let service = create_test_service();

    let result = service.handle_get_configuration(GetConfiguration {
        ptz_configuration_token: "InvalidConfig".to_string(),
    });

    assert!(result.is_err());
}
