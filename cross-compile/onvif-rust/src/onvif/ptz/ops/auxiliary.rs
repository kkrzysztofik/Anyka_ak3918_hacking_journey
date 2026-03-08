//! PTZ auxiliary operations.
//!
//! This module handles auxiliary and capability-related PTZ operations:
//! - GetServiceCapabilities: Get PTZ service capabilities
//! - GetCompatibleConfigurations: Get compatible PTZ configurations
//! - SendAuxiliaryCommand: Send auxiliary command

use crate::onvif::error::OnvifResult;
use crate::onvif::types::ptz::{
    GetCompatibleConfigurationsResponse, GetServiceCapabilitiesResponse,
};

use crate::onvif::ptz::ops::config::build_ptz_configuration;
use crate::onvif::ptz::state::PTZStateManager;
use crate::onvif::ptz::types::build_service_capabilities;

/// Handle GetServiceCapabilities request.
///
/// # Arguments
///
/// * `_state` - PTZ state manager (unused; capabilities are static)
///
/// # Returns
///
/// Service capabilities describing supported PTZ features.
pub fn get_service_capabilities(
    _state: &PTZStateManager,
) -> OnvifResult<GetServiceCapabilitiesResponse> {
    tracing::debug!("GetServiceCapabilities request");

    Ok(GetServiceCapabilitiesResponse {
        capabilities: build_service_capabilities(),
    })
}

/// Handle GetCompatibleConfigurations request.
///
/// # Arguments
///
/// * `_state` - PTZ state manager (unused; configurations are static)
/// * `profile_token` - The media profile to query compatible configurations for
///
/// # Returns
///
/// A list of PTZ configurations compatible with the given profile.
/// Currently returns the single fixed configuration.
pub fn get_compatible_configurations(
    _state: &PTZStateManager,
    profile_token: &str,
) -> OnvifResult<GetCompatibleConfigurationsResponse> {
    tracing::debug!(
        "GetCompatibleConfigurations request for profile {}",
        profile_token
    );

    // Return all configurations (we only have one)
    Ok(GetCompatibleConfigurationsResponse {
        ptz_configurations: vec![build_ptz_configuration()],
    })
}

/// Handle SendAuxiliaryCommand request.
///
/// # Arguments
///
/// * `_state` - PTZ state manager (unused; no auxiliary hardware)
/// * `profile_token` - The media profile token
/// * `auxiliary_data` - The auxiliary command string (e.g. `"tt:Wiper|On"`)
///
/// # Returns
///
/// `Ok(None)` — auxiliary commands are accepted but not acted upon because
/// the AK3918 has no auxiliary hardware (wiper, heater, etc.).
///
/// # Errors
///
/// Currently infallible. Will return errors if auxiliary hardware support
/// is added in the future.
pub fn send_auxiliary_command(
    _state: &PTZStateManager,
    profile_token: &str,
    auxiliary_data: &str,
) -> OnvifResult<Option<String>> {
    tracing::debug!(
        "SendAuxiliaryCommand request for profile {}, command: {}",
        profile_token,
        auxiliary_data
    );

    // We don't support auxiliary commands, return success with empty response
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn create_test_state() -> Arc<PTZStateManager> {
        Arc::new(PTZStateManager::new())
    }

    #[test]
    fn test_get_service_capabilities_returns_move_status_and_position() {
        let state = create_test_state();

        let response = get_service_capabilities(&state).unwrap();

        assert_eq!(response.capabilities.move_status, Some(true));
        assert_eq!(response.capabilities.status_position, Some(true));
    }

    #[test]
    fn test_get_compatible_configurations_returns_single_config() {
        let state = create_test_state();

        let response = get_compatible_configurations(&state, "Profile1").unwrap();

        assert_eq!(response.ptz_configurations.len(), 1);
    }

    #[test]
    fn test_send_auxiliary_command_returns_none_when_unsupported() {
        let state = create_test_state();

        let response = send_auxiliary_command(&state, "Profile1", "tt:Wiper|On").unwrap();

        // We return success with no response data
        assert!(response.is_none());
    }
}
