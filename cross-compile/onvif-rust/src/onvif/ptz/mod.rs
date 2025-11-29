//! ONVIF PTZ Service implementation.
//!
//! This module implements the ONVIF PTZ Service (tptz namespace) providing:
//! - Node discovery (GetNodes, GetNode)
//! - Configuration management (GetConfigurations, GetConfiguration, SetConfiguration, GetConfigurationOptions)
//! - Movement operations (AbsoluteMove, RelativeMove, ContinuousMove, Stop)
//! - Preset management (GetPresets, SetPreset, GotoPreset, RemovePreset)
//! - Home position (GetStatus, GotoHomePosition, SetHomePosition)
//! - Service capabilities (GetServiceCapabilities, GetCompatibleConfigurations)
//!
//! # PTZ Coordinate Spaces
//!
//! PTZ operations use normalized coordinate spaces:
//! - Pan: -1.0 to 1.0 (left to right)
//! - Tilt: -1.0 to 1.0 (down to up)
//! - Zoom: 0.0 to 1.0 (wide to tele)
//!
//! # Error Handling
//!
//! PTZ Service operations return ONVIF-compliant SOAP faults:
//! - `ter:NoToken` - Missing profile or configuration token
//! - `ter:InvalidToken` - Unknown profile or configuration token
//! - `ter:NoPreset` - Unknown preset token
//! - `ter:TooManyPresets` - Maximum presets reached
//! - `ter:MovingPanTilt` - Cannot operate while moving
//! - `ter:MovingZoom` - Cannot operate while zooming
//! - `ter:NotAuthorized` - Insufficient privileges

mod handlers;
mod state;
pub mod types;

pub use handlers::PTZService;
pub use state::PTZStateManager;
pub use types::*;

// Re-export WSDL types for PTZ service
pub use crate::onvif::types::ptz::{
    AbsoluteMove, AbsoluteMoveResponse, ContinuousMove, ContinuousMoveResponse,
    GetCompatibleConfigurations, GetCompatibleConfigurationsResponse, GetConfiguration,
    GetConfigurationOptions, GetConfigurationOptionsResponse, GetConfigurationResponse,
    GetConfigurations, GetConfigurationsResponse, GetNode, GetNodeResponse, GetNodes,
    GetNodesResponse, GetPresets, GetPresetsResponse, GetServiceCapabilities,
    GetServiceCapabilitiesResponse, GetStatus, GetStatusResponse, GotoHomePosition,
    GotoHomePositionResponse, GotoPreset, GotoPresetResponse, PTZNode, PTZServiceCapabilities,
    RelativeMove, RelativeMoveResponse, RemovePreset, RemovePresetResponse, SendAuxiliaryCommand,
    SendAuxiliaryCommandResponse, SetConfiguration, SetConfigurationResponse, SetHomePosition,
    SetHomePositionResponse, SetPreset, SetPresetResponse, Stop, StopResponse,
};
