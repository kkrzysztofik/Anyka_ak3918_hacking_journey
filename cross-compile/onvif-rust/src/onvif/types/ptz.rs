//! PTZ Service types from ptz.wsdl (tptz:* namespace).
//!
//! This module contains request/response types for the ONVIF PTZ Service.
//! These types are defined in the `http://www.onvif.org/ver20/ptz/wsdl` namespace.
//!
//! # Operations
//!
//! ## Configuration
//! - `GetConfigurations` - Get all PTZ configurations
//! - `GetConfiguration` - Get a specific PTZ configuration
//! - `GetConfigurationOptions` - Get supported configuration options
//! - `SetConfiguration` - Update PTZ configuration
//!
//! ## Movement
//! - `AbsoluteMove` - Move to absolute position
//! - `RelativeMove` - Move by relative offset
//! - `ContinuousMove` - Start continuous movement
//! - `Stop` - Stop PTZ movement
//! - `GotoHomePosition` - Move to home position
//! - `SetHomePosition` - Set current position as home
//!
//! ## Presets
//! - `GetPresets` - Get all presets
//! - `SetPreset` - Create or update a preset
//! - `GotoPreset` - Move to a preset position
//! - `RemovePreset` - Delete a preset
//!
//! ## Status
//! - `GetStatus` - Get current PTZ status

use serde::{Deserialize, Serialize};

use super::Extension;
use super::common::{
    Name, PTZConfiguration, PTZPreset, PTZSpeed, PTZStatus, PTZVector, ReferenceToken,
    Space1DDescription, Space2DDescription,
};

// ============================================================================
// Configurations
// ============================================================================

/// GetConfigurations request.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetConfigurations")]
pub struct GetConfigurations {}

/// GetConfigurations response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "tptz:GetConfigurationsResponse")]
pub struct GetConfigurationsResponse {
    /// List of PTZ configurations.
    #[serde(rename = "tptz:PTZConfiguration", alias = "PTZConfiguration", default)]
    pub ptz_configurations: Vec<PTZConfiguration>,
}

/// GetConfiguration request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetConfiguration")]
pub struct GetConfiguration {
    /// Configuration token.
    #[serde(rename = "PTZConfigurationToken")]
    pub ptz_configuration_token: ReferenceToken,
}

/// GetConfiguration response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "tptz:GetConfigurationResponse")]
pub struct GetConfigurationResponse {
    /// The requested PTZ configuration.
    #[serde(rename = "tptz:PTZConfiguration", alias = "PTZConfiguration")]
    pub ptz_configuration: PTZConfiguration,
}

/// SetConfiguration request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "SetConfiguration")]
pub struct SetConfiguration {
    /// PTZ configuration to set.
    #[serde(rename = "PTZConfiguration")]
    pub ptz_configuration: PTZConfiguration,

    /// Force persistence.
    #[serde(rename = "ForcePersistence")]
    pub force_persistence: bool,
}

/// SetConfiguration response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "tptz:SetConfigurationResponse")]
pub struct SetConfigurationResponse {}

/// GetConfigurationOptions request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetConfigurationOptions")]
pub struct GetConfigurationOptions {
    /// Configuration token.
    #[serde(rename = "ConfigurationToken")]
    pub configuration_token: ReferenceToken,
}

/// GetConfigurationOptions response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "tptz:GetConfigurationOptionsResponse")]
pub struct GetConfigurationOptionsResponse {
    /// PTZ configuration options.
    #[serde(
        rename = "tptz:PTZConfigurationOptions",
        alias = "PTZConfigurationOptions"
    )]
    pub ptz_configuration_options: PTZConfigurationOptions,
}

/// PTZ configuration options.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PTZConfigurationOptions {
    /// Supported PTZ spaces.
    #[serde(rename = "tt:Spaces", alias = "Spaces")]
    pub spaces: PTZSpaces,

    /// PTZ timeout range.
    #[serde(rename = "tt:PTZTimeout", alias = "PTZTimeout")]
    pub ptz_timeout: DurationRange,

    /// PT control direction options.
    #[serde(
        rename = "tt:PTControlDirection",
        alias = "PTControlDirection",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub pt_control_direction: Option<PTControlDirectionOptions>,

    /// Extension.
    #[serde(
        rename = "tt:Extension",
        alias = "Extension",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub extension: Option<PTZConfigurationOptionsExtension>,

    /// PTZ ranges mode attribute.
    #[serde(rename = "@PTZRamps", default, skip_serializing_if = "Option::is_none")]
    pub ptz_ramps: Option<i32>,
}

/// PTZ configuration options extension.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PTZConfigurationOptionsExtension {
    /// Supported preset tour.
    #[serde(
        rename = "tt:PTControlDirection",
        alias = "PTControlDirection",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub pt_control_direction: Option<PTControlDirectionOptions>,

    /// Extension.
    #[serde(
        rename = "tt:Extension",
        alias = "Extension",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub extension: Option<Extension>,
}

/// PTZ spaces.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PTZSpaces {
    /// Absolute pan/tilt position space.
    #[serde(
        rename = "tt:AbsolutePanTiltPositionSpace",
        alias = "AbsolutePanTiltPositionSpace",
        default
    )]
    pub absolute_pan_tilt_position_space: Vec<Space2DDescription>,

    /// Absolute zoom position space.
    #[serde(
        rename = "tt:AbsoluteZoomPositionSpace",
        alias = "AbsoluteZoomPositionSpace",
        default
    )]
    pub absolute_zoom_position_space: Vec<Space1DDescription>,

    /// Relative pan/tilt translation space.
    #[serde(
        rename = "tt:RelativePanTiltTranslationSpace",
        alias = "RelativePanTiltTranslationSpace",
        default
    )]
    pub relative_pan_tilt_translation_space: Vec<Space2DDescription>,

    /// Relative zoom translation space.
    #[serde(
        rename = "tt:RelativeZoomTranslationSpace",
        alias = "RelativeZoomTranslationSpace",
        default
    )]
    pub relative_zoom_translation_space: Vec<Space1DDescription>,

    /// Continuous pan/tilt velocity space.
    #[serde(
        rename = "tt:ContinuousPanTiltVelocitySpace",
        alias = "ContinuousPanTiltVelocitySpace",
        default
    )]
    pub continuous_pan_tilt_velocity_space: Vec<Space2DDescription>,

    /// Continuous zoom velocity space.
    #[serde(
        rename = "tt:ContinuousZoomVelocitySpace",
        alias = "ContinuousZoomVelocitySpace",
        default
    )]
    pub continuous_zoom_velocity_space: Vec<Space1DDescription>,

    /// Pan/tilt speed space.
    #[serde(rename = "tt:PanTiltSpeedSpace", alias = "PanTiltSpeedSpace", default)]
    pub pan_tilt_speed_space: Vec<Space1DDescription>,

    /// Zoom speed space.
    #[serde(rename = "tt:ZoomSpeedSpace", alias = "ZoomSpeedSpace", default)]
    pub zoom_speed_space: Vec<Space1DDescription>,

    /// Extension.
    #[serde(
        rename = "tt:Extension",
        alias = "Extension",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub extension: Option<Extension>,
}

/// Duration range.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DurationRange {
    /// Minimum duration.
    #[serde(rename = "tt:Min", alias = "Min")]
    pub min: String,

    /// Maximum duration.
    #[serde(rename = "tt:Max", alias = "Max")]
    pub max: String,
}

/// PT control direction options.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PTControlDirectionOptions {
    /// EFlip options.
    #[serde(
        rename = "tt:EFlip",
        alias = "EFlip",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub e_flip: Option<EFlipOptions>,

    /// Reverse options.
    #[serde(
        rename = "tt:Reverse",
        alias = "Reverse",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub reverse: Option<ReverseOptions>,

    /// Extension.
    #[serde(
        rename = "tt:Extension",
        alias = "Extension",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub extension: Option<Extension>,
}

/// EFlip options.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct EFlipOptions {
    /// Supported EFlip modes.
    #[serde(rename = "tt:Mode", alias = "Mode", default)]
    pub mode: Vec<EFlipMode>,

    /// Extension.
    #[serde(
        rename = "tt:Extension",
        alias = "Extension",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub extension: Option<Extension>,
}

/// EFlip mode.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum EFlipMode {
    #[default]
    OFF,
    ON,
    Extended,
}

/// Reverse options.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ReverseOptions {
    /// Supported reverse modes.
    #[serde(rename = "tt:Mode", alias = "Mode", default)]
    pub mode: Vec<ReverseMode>,

    /// Extension.
    #[serde(
        rename = "tt:Extension",
        alias = "Extension",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub extension: Option<Extension>,
}

/// Reverse mode.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum ReverseMode {
    #[default]
    OFF,
    ON,
    AUTO,
    Extended,
}

// ============================================================================
// Movement Operations
// ============================================================================

/// AbsoluteMove request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "AbsoluteMove")]
pub struct AbsoluteMove {
    /// Profile token.
    #[serde(rename = "ProfileToken")]
    pub profile_token: ReferenceToken,

    /// Target position.
    #[serde(rename = "Position")]
    pub position: PTZVector,

    /// Movement speed (optional).
    #[serde(rename = "Speed", default, skip_serializing_if = "Option::is_none")]
    pub speed: Option<PTZSpeed>,
}

/// AbsoluteMove response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "tptz:AbsoluteMoveResponse")]
pub struct AbsoluteMoveResponse {}

/// RelativeMove request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "RelativeMove")]
pub struct RelativeMove {
    /// Profile token.
    #[serde(rename = "ProfileToken")]
    pub profile_token: ReferenceToken,

    /// Translation vector.
    #[serde(rename = "Translation")]
    pub translation: PTZVector,

    /// Movement speed (optional).
    #[serde(rename = "Speed", default, skip_serializing_if = "Option::is_none")]
    pub speed: Option<PTZSpeed>,
}

/// RelativeMove response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "tptz:RelativeMoveResponse")]
pub struct RelativeMoveResponse {}

/// ContinuousMove request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "ContinuousMove")]
pub struct ContinuousMove {
    /// Profile token.
    #[serde(rename = "ProfileToken")]
    pub profile_token: ReferenceToken,

    /// Velocity vector.
    #[serde(rename = "Velocity")]
    pub velocity: PTZSpeed,

    /// Timeout duration (optional).
    #[serde(rename = "Timeout", default, skip_serializing_if = "Option::is_none")]
    pub timeout: Option<String>,
}

/// ContinuousMove response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "tptz:ContinuousMoveResponse")]
pub struct ContinuousMoveResponse {}

/// Stop request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "Stop")]
pub struct Stop {
    /// Profile token.
    #[serde(rename = "ProfileToken")]
    pub profile_token: ReferenceToken,

    /// Stop pan/tilt movement (optional).
    #[serde(rename = "PanTilt", default, skip_serializing_if = "Option::is_none")]
    pub pan_tilt: Option<bool>,

    /// Stop zoom movement (optional).
    #[serde(rename = "Zoom", default, skip_serializing_if = "Option::is_none")]
    pub zoom: Option<bool>,
}

/// Stop response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "tptz:StopResponse")]
pub struct StopResponse {}

/// GotoHomePosition request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GotoHomePosition")]
pub struct GotoHomePosition {
    /// Profile token.
    #[serde(rename = "ProfileToken")]
    pub profile_token: ReferenceToken,

    /// Movement speed (optional).
    #[serde(rename = "Speed", default, skip_serializing_if = "Option::is_none")]
    pub speed: Option<PTZSpeed>,
}

/// GotoHomePosition response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "tptz:GotoHomePositionResponse")]
pub struct GotoHomePositionResponse {}

/// SetHomePosition request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "SetHomePosition")]
pub struct SetHomePosition {
    /// Profile token.
    #[serde(rename = "ProfileToken")]
    pub profile_token: ReferenceToken,
}

/// SetHomePosition response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "tptz:SetHomePositionResponse")]
pub struct SetHomePositionResponse {}

// ============================================================================
// Presets
// ============================================================================

/// GetPresets request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetPresets")]
pub struct GetPresets {
    /// Profile token.
    #[serde(rename = "ProfileToken")]
    pub profile_token: ReferenceToken,
}

/// GetPresets response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "tptz:GetPresetsResponse")]
pub struct GetPresetsResponse {
    /// List of presets.
    #[serde(rename = "tptz:Preset", alias = "Preset", default)]
    pub presets: Vec<PTZPreset>,
}

/// SetPreset request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "SetPreset")]
pub struct SetPreset {
    /// Profile token.
    #[serde(rename = "ProfileToken")]
    pub profile_token: ReferenceToken,

    /// Preset name (optional for updates).
    #[serde(
        rename = "PresetName",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub preset_name: Option<String>,

    /// Preset token (optional for new presets).
    #[serde(
        rename = "PresetToken",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub preset_token: Option<ReferenceToken>,
}

/// SetPreset response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "tptz:SetPresetResponse")]
pub struct SetPresetResponse {
    /// Token of the created/updated preset.
    #[serde(rename = "tptz:PresetToken", alias = "PresetToken")]
    pub preset_token: ReferenceToken,
}

/// GotoPreset request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GotoPreset")]
pub struct GotoPreset {
    /// Profile token.
    #[serde(rename = "ProfileToken")]
    pub profile_token: ReferenceToken,

    /// Preset token.
    #[serde(rename = "PresetToken")]
    pub preset_token: ReferenceToken,

    /// Movement speed (optional).
    #[serde(rename = "Speed", default, skip_serializing_if = "Option::is_none")]
    pub speed: Option<PTZSpeed>,
}

/// GotoPreset response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "tptz:GotoPresetResponse")]
pub struct GotoPresetResponse {}

/// RemovePreset request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "RemovePreset")]
pub struct RemovePreset {
    /// Profile token.
    #[serde(rename = "ProfileToken")]
    pub profile_token: ReferenceToken,

    /// Preset token.
    #[serde(rename = "PresetToken")]
    pub preset_token: ReferenceToken,
}

/// RemovePreset response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "tptz:RemovePresetResponse")]
pub struct RemovePresetResponse {}

// ============================================================================
// Status
// ============================================================================

/// GetStatus request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetStatus")]
pub struct GetStatus {
    /// Profile token.
    #[serde(rename = "ProfileToken")]
    pub profile_token: ReferenceToken,
}

/// GetStatus response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "tptz:GetStatusResponse")]
pub struct GetStatusResponse {
    /// PTZ status.
    #[serde(rename = "tptz:PTZStatus", alias = "PTZStatus")]
    pub ptz_status: PTZStatus,
}

// ============================================================================
// Nodes
// ============================================================================

/// GetNodes request.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetNodes")]
pub struct GetNodes {}

/// GetNodes response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "tptz:GetNodesResponse")]
pub struct GetNodesResponse {
    /// List of PTZ nodes.
    #[serde(rename = "tptz:PTZNode", alias = "PTZNode", default)]
    pub ptz_nodes: Vec<PTZNode>,
}

/// GetNode request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetNode")]
pub struct GetNode {
    /// Node token.
    #[serde(rename = "NodeToken")]
    pub node_token: ReferenceToken,
}

/// GetNode response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "tptz:GetNodeResponse")]
pub struct GetNodeResponse {
    /// The requested PTZ node.
    #[serde(rename = "tptz:PTZNode", alias = "PTZNode")]
    pub ptz_node: PTZNode,
}

/// PTZ node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PTZNode {
    /// Node token.
    #[serde(rename = "@token")]
    pub token: ReferenceToken,

    /// Fixed home position support.
    #[serde(
        rename = "@FixedHomePosition",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub fixed_home_position: Option<bool>,

    /// Geo move support.
    #[serde(rename = "@GeoMove", default, skip_serializing_if = "Option::is_none")]
    pub geo_move: Option<bool>,

    /// Node name.
    #[serde(
        rename = "tt:Name",
        alias = "Name",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub name: Option<Name>,

    /// Supported PTZ spaces.
    #[serde(rename = "tt:SupportedPTZSpaces", alias = "SupportedPTZSpaces")]
    pub supported_ptz_spaces: PTZSpaces,

    /// Maximum number of presets.
    #[serde(rename = "tt:MaximumNumberOfPresets", alias = "MaximumNumberOfPresets")]
    pub maximum_number_of_presets: i32,

    /// Home position support.
    #[serde(rename = "tt:HomeSupported", alias = "HomeSupported")]
    pub home_supported: bool,

    /// Auxiliary commands.
    #[serde(rename = "tt:AuxiliaryCommands", alias = "AuxiliaryCommands", default)]
    pub auxiliary_commands: Vec<String>,

    /// Extension.
    #[serde(
        rename = "tt:Extension",
        alias = "Extension",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub extension: Option<PTZNodeExtension>,
}

/// PTZ node extension.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PTZNodeExtension {
    /// Supported preset tour.
    #[serde(
        rename = "tt:SupportedPresetTour",
        alias = "SupportedPresetTour",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub supported_preset_tour: Option<PTZPresetTourSupported>,

    /// Extension.
    #[serde(
        rename = "tt:Extension",
        alias = "Extension",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub extension: Option<Extension>,
}

/// PTZ preset tour supported information.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PTZPresetTourSupported {
    /// Maximum number of preset tours.
    #[serde(
        rename = "tt:MaximumNumberOfPresetTours",
        alias = "MaximumNumberOfPresetTours"
    )]
    pub maximum_number_of_preset_tours: i32,

    /// Supported preset tour operations.
    /// Note: Uses String since ONVIF XML has repeated `<PTZPresetTourOperation>Value</PTZPresetTourOperation>`
    /// elements which quick-xml parses as text content.
    #[serde(
        rename = "tt:PTZPresetTourOperation",
        alias = "PTZPresetTourOperation",
        default
    )]
    pub ptz_preset_tour_operation: Vec<String>,

    /// Extension.
    #[serde(
        rename = "tt:Extension",
        alias = "Extension",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub extension: Option<Extension>,
}

/// PTZ preset tour operation.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum PTZPresetTourOperation {
    #[default]
    Start,
    Stop,
    Pause,
    Extended,
}

// ============================================================================
// Service Capabilities
// ============================================================================

/// GetServiceCapabilities request.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetServiceCapabilities")]
pub struct GetServiceCapabilities {}

/// GetServiceCapabilities response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "tptz:GetServiceCapabilitiesResponse")]
pub struct GetServiceCapabilitiesResponse {
    /// PTZ service capabilities.
    #[serde(rename = "tptz:Capabilities", alias = "Capabilities")]
    pub capabilities: PTZServiceCapabilities,
}

/// PTZ service capabilities.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PTZServiceCapabilities {
    /// EFlip support.
    #[serde(rename = "@EFlip", default, skip_serializing_if = "Option::is_none")]
    pub e_flip: Option<bool>,

    /// Reverse support.
    #[serde(rename = "@Reverse", default, skip_serializing_if = "Option::is_none")]
    pub reverse: Option<bool>,

    /// GetCompatibleConfigurations support.
    #[serde(
        rename = "@GetCompatibleConfigurations",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub get_compatible_configurations: Option<bool>,

    /// Move status support.
    #[serde(
        rename = "@MoveStatus",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub move_status: Option<bool>,

    /// Status position support.
    #[serde(
        rename = "@StatusPosition",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub status_position: Option<bool>,
}

// ============================================================================
// Auxiliary Operations
// ============================================================================

/// SendAuxiliaryCommand request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "SendAuxiliaryCommand")]
pub struct SendAuxiliaryCommand {
    /// Profile token.
    #[serde(rename = "ProfileToken")]
    pub profile_token: ReferenceToken,

    /// Auxiliary data command.
    #[serde(rename = "AuxiliaryData")]
    pub auxiliary_data: String,
}

/// SendAuxiliaryCommand response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "tptz:SendAuxiliaryCommandResponse")]
pub struct SendAuxiliaryCommandResponse {
    /// Auxiliary response.
    #[serde(
        rename = "tptz:AuxiliaryResponse",
        alias = "AuxiliaryResponse",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub auxiliary_response: Option<String>,
}

// ============================================================================
// Compatible Configurations
// ============================================================================

/// GetCompatibleConfigurations request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetCompatibleConfigurations")]
pub struct GetCompatibleConfigurations {
    /// Profile token.
    #[serde(rename = "ProfileToken")]
    pub profile_token: ReferenceToken,
}

/// GetCompatibleConfigurations response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "tptz:GetCompatibleConfigurationsResponse")]
pub struct GetCompatibleConfigurationsResponse {
    /// List of compatible PTZ configurations.
    #[serde(rename = "tptz:PTZConfiguration", alias = "PTZConfiguration", default)]
    pub ptz_configurations: Vec<PTZConfiguration>,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eflip_mode_default() {
        let mode = EFlipMode::default();
        assert_eq!(mode, EFlipMode::OFF);
    }

    #[test]
    fn test_reverse_mode_default() {
        let mode = ReverseMode::default();
        assert_eq!(mode, ReverseMode::OFF);
    }

    #[test]
    fn test_ptz_preset_tour_operation_default() {
        let op = PTZPresetTourOperation::default();
        assert_eq!(op, PTZPresetTourOperation::Start);
    }

    #[test]
    fn test_get_configurations_default() {
        let request = GetConfigurations::default();
        let _ = request;
    }

    #[test]
    fn test_get_configurations_response_default() {
        let response = GetConfigurationsResponse::default();
        assert!(response.ptz_configurations.is_empty());
    }

    #[test]
    fn test_get_presets_response_default() {
        let response = GetPresetsResponse::default();
        assert!(response.presets.is_empty());
    }
}
