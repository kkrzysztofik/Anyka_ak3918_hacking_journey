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

/// Requested lamp state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LampState {
    On,
    Off,
    Auto,
}

/// A recognised ONVIF auxiliary command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuxCommand {
    IrLamp(LampState),
    WhiteLight(LampState),
}

/// Parse an ONVIF auxiliary command string such as `"tt:IRLamp|On"`.
///
/// Returns `None` for anything unrecognised; the caller must raise an ONVIF
/// fault rather than reporting success for hardware that does not exist.
pub fn parse_auxiliary(data: &str) -> Option<AuxCommand> {
    let (name, state) = data.split_once('|')?;
    let state = match state {
        "On" => LampState::On,
        "Off" => LampState::Off,
        "Auto" => LampState::Auto,
        _ => return None,
    };
    match name {
        "tt:IRLamp" => Some(AuxCommand::IrLamp(state)),
        "tt:WhiteLight" => Some(AuxCommand::WhiteLight(state)),
        _ => None,
    }
}

/// Handle SendAuxiliaryCommand request.
///
/// Unknown commands return `InvalidArgVal`. Recognised lamp commands are
/// returned as [`AuxCommand`] for the service layer to dispatch to hardware.
pub fn send_auxiliary_command(
    _state: &PTZStateManager,
    profile_token: &str,
    auxiliary_data: &str,
) -> OnvifResult<AuxCommand> {
    tracing::debug!(
        profile = %profile_token,
        command = %auxiliary_data,
        "SendAuxiliaryCommand"
    );

    parse_auxiliary(auxiliary_data).ok_or_else(|| crate::onvif::error::OnvifError::InvalidArgVal {
        subcode: "InvalidArgVal".to_string(),
        reason: format!("Unsupported auxiliary command: {auxiliary_data}"),
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
    fn test_parse_ir_lamp_on() {
        assert_eq!(
            parse_auxiliary("tt:IRLamp|On"),
            Some(AuxCommand::IrLamp(LampState::On))
        );
    }

    #[test]
    fn test_parse_white_light_off() {
        assert_eq!(
            parse_auxiliary("tt:WhiteLight|Off"),
            Some(AuxCommand::WhiteLight(LampState::Off))
        );
    }

    #[test]
    fn test_parse_ir_lamp_auto() {
        assert_eq!(
            parse_auxiliary("tt:IRLamp|Auto"),
            Some(AuxCommand::IrLamp(LampState::Auto))
        );
    }

    #[test]
    fn test_parse_rejects_unknown_command() {
        assert_eq!(parse_auxiliary("tt:Wiper|On"), None);
    }

    #[test]
    fn test_parse_rejects_malformed_command() {
        assert_eq!(parse_auxiliary("tt:IRLamp"), None);
        assert_eq!(parse_auxiliary(""), None);
    }

    #[test]
    fn test_send_auxiliary_command_rejects_unknown() {
        let state = create_test_state();

        let result = send_auxiliary_command(&state, "Profile1", "tt:Wiper|On");

        assert!(result.is_err());
    }
}
