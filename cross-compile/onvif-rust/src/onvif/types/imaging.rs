//! Imaging Service types from imaging.wsdl (timg:* namespace).
//!
//! This module contains request/response types for the ONVIF Imaging Service.
//! These types are defined in the `http://www.onvif.org/ver20/imaging/wsdl` namespace.
//!
//! # Operations
//!
//! ## Settings
//! - `GetImagingSettings` - Get current imaging settings
//! - `SetImagingSettings` - Update imaging settings
//! - `GetOptions` - Get supported imaging options
//!
//! ## Focus
//! - `Move` - Perform focus move operation
//! - `Stop` - Stop focus movement
//! - `GetMoveOptions` - Get supported focus move options
//!
//! ## Status
//! - `GetStatus` - Get current imaging status

use serde::{Deserialize, Serialize};

use super::Extension;
use super::common::{FloatRange, ImagingSettings20, ImagingStatus20, ReferenceToken};

// ============================================================================
// Imaging Settings
// ============================================================================

/// GetImagingSettings request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetImagingSettings")]
pub struct GetImagingSettings {
    /// Video source token.
    #[serde(rename = "VideoSourceToken")]
    pub video_source_token: ReferenceToken,
}

/// GetImagingSettings response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "timg:GetImagingSettingsResponse")]
pub struct GetImagingSettingsResponse {
    /// Current imaging settings.
    #[serde(rename = "timg:ImagingSettings")]
    pub imaging_settings: ImagingSettings20,
}

/// SetImagingSettings request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "SetImagingSettings")]
pub struct SetImagingSettings {
    /// Video source token.
    #[serde(rename = "VideoSourceToken")]
    pub video_source_token: ReferenceToken,

    /// New imaging settings.
    #[serde(rename = "ImagingSettings")]
    pub imaging_settings: ImagingSettings20,

    /// Force persistence.
    #[serde(
        rename = "ForcePersistence",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub force_persistence: Option<bool>,
}

/// SetImagingSettings response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "timg:SetImagingSettingsResponse")]
pub struct SetImagingSettingsResponse {}

/// GetOptions request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetOptions")]
pub struct GetOptions {
    /// Video source token.
    #[serde(rename = "VideoSourceToken")]
    pub video_source_token: ReferenceToken,
}

/// GetOptions response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "timg:GetOptionsResponse")]
pub struct GetOptionsResponse {
    /// Imaging options.
    #[serde(rename = "timg:ImagingOptions")]
    pub imaging_options: ImagingOptions20,
}

/// Imaging options (version 2.0).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ImagingOptions20 {
    /// Backlight compensation options.
    #[serde(
        rename = "BacklightCompensation",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub backlight_compensation: Option<BacklightCompensationOptions20>,

    /// Brightness options.
    #[serde(
        rename = "Brightness",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub brightness: Option<FloatRange>,

    /// Color saturation options.
    #[serde(
        rename = "ColorSaturation",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub color_saturation: Option<FloatRange>,

    /// Contrast options.
    #[serde(rename = "Contrast", default, skip_serializing_if = "Option::is_none")]
    pub contrast: Option<FloatRange>,

    /// Exposure options.
    #[serde(rename = "Exposure", default, skip_serializing_if = "Option::is_none")]
    pub exposure: Option<ExposureOptions20>,

    /// Focus options.
    #[serde(rename = "Focus", default, skip_serializing_if = "Option::is_none")]
    pub focus: Option<FocusOptions20>,

    /// IR cut filter options.
    #[serde(rename = "IrCutFilterModes", default)]
    pub ir_cut_filter_modes: Vec<IrCutFilterMode>,

    /// Sharpness options.
    #[serde(rename = "Sharpness", default, skip_serializing_if = "Option::is_none")]
    pub sharpness: Option<FloatRange>,

    /// Wide dynamic range options.
    #[serde(
        rename = "WideDynamicRange",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub wide_dynamic_range: Option<WideDynamicRangeOptions20>,

    /// White balance options.
    #[serde(
        rename = "WhiteBalance",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub white_balance: Option<WhiteBalanceOptions20>,

    /// Extension.
    #[serde(rename = "Extension", default, skip_serializing_if = "Option::is_none")]
    pub extension: Option<ImagingOptions20Extension>,
}

/// Imaging options extension.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ImagingOptions20Extension {
    /// Image stabilization options.
    #[serde(
        rename = "ImageStabilization",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub image_stabilization: Option<ImageStabilizationOptions>,

    /// Extension.
    #[serde(rename = "Extension", default, skip_serializing_if = "Option::is_none")]
    pub extension: Option<ImagingOptions20Extension2>,
}

/// Imaging options extension 2.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ImagingOptions20Extension2 {
    /// IR cut filter auto adjustment options.
    #[serde(
        rename = "IrCutFilterAutoAdjustment",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub ir_cut_filter_auto_adjustment: Option<IrCutFilterAutoAdjustmentOptions>,

    /// Extension.
    #[serde(rename = "Extension", default, skip_serializing_if = "Option::is_none")]
    pub extension: Option<ImagingOptions20Extension3>,
}

/// Imaging options extension 3.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ImagingOptions20Extension3 {
    /// Tone compensation options.
    #[serde(
        rename = "ToneCompensationOptions",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub tone_compensation_options: Option<ToneCompensationOptions>,

    /// Defogging options.
    #[serde(
        rename = "DefoggingOptions",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub defogging_options: Option<DefoggingOptions>,

    /// Noise reduction options.
    #[serde(
        rename = "NoiseReductionOptions",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub noise_reduction_options: Option<NoiseReductionOptions>,

    /// Extension.
    #[serde(rename = "Extension", default, skip_serializing_if = "Option::is_none")]
    pub extension: Option<Extension>,
}

/// Backlight compensation options.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BacklightCompensationOptions20 {
    /// Supported modes.
    #[serde(rename = "Mode", default)]
    pub mode: Vec<BacklightCompensationMode>,

    /// Level range.
    #[serde(rename = "Level", default, skip_serializing_if = "Option::is_none")]
    pub level: Option<FloatRange>,
}

/// Backlight compensation mode.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum BacklightCompensationMode {
    #[default]
    OFF,
    ON,
}

/// Exposure options (version 2.0).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ExposureOptions20 {
    /// Supported modes.
    #[serde(rename = "Mode", default)]
    pub mode: Vec<ExposureMode>,

    /// Priority options.
    #[serde(rename = "Priority", default)]
    pub priority: Vec<ExposurePriority>,

    /// Min exposure time range.
    #[serde(
        rename = "MinExposureTime",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub min_exposure_time: Option<FloatRange>,

    /// Max exposure time range.
    #[serde(
        rename = "MaxExposureTime",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub max_exposure_time: Option<FloatRange>,

    /// Min gain range.
    #[serde(rename = "MinGain", default, skip_serializing_if = "Option::is_none")]
    pub min_gain: Option<FloatRange>,

    /// Max gain range.
    #[serde(rename = "MaxGain", default, skip_serializing_if = "Option::is_none")]
    pub max_gain: Option<FloatRange>,

    /// Min iris range.
    #[serde(rename = "MinIris", default, skip_serializing_if = "Option::is_none")]
    pub min_iris: Option<FloatRange>,

    /// Max iris range.
    #[serde(rename = "MaxIris", default, skip_serializing_if = "Option::is_none")]
    pub max_iris: Option<FloatRange>,

    /// Exposure time range.
    #[serde(
        rename = "ExposureTime",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub exposure_time: Option<FloatRange>,

    /// Gain range.
    #[serde(rename = "Gain", default, skip_serializing_if = "Option::is_none")]
    pub gain: Option<FloatRange>,

    /// Iris range.
    #[serde(rename = "Iris", default, skip_serializing_if = "Option::is_none")]
    pub iris: Option<FloatRange>,
}

/// Exposure mode.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum ExposureMode {
    #[default]
    AUTO,
    MANUAL,
}

/// Exposure priority.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum ExposurePriority {
    LowNoise,
    #[default]
    FrameRate,
}

/// Focus options (version 2.0).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct FocusOptions20 {
    /// Supported auto focus modes.
    #[serde(rename = "AutoFocusModes", default)]
    pub auto_focus_modes: Vec<AutoFocusMode>,

    /// Default speed range.
    #[serde(
        rename = "DefaultSpeed",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub default_speed: Option<FloatRange>,

    /// Near limit range.
    #[serde(rename = "NearLimit", default, skip_serializing_if = "Option::is_none")]
    pub near_limit: Option<FloatRange>,

    /// Far limit range.
    #[serde(rename = "FarLimit", default, skip_serializing_if = "Option::is_none")]
    pub far_limit: Option<FloatRange>,

    /// Extension.
    #[serde(rename = "Extension", default, skip_serializing_if = "Option::is_none")]
    pub extension: Option<FocusOptions20Extension>,
}

/// Focus options extension.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct FocusOptions20Extension {
    /// AF modes.
    #[serde(rename = "AFModes", default)]
    pub af_modes: Vec<String>,
}

/// Auto focus mode.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum AutoFocusMode {
    #[default]
    AUTO,
    MANUAL,
}

/// IR cut filter mode.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum IrCutFilterMode {
    ON,
    OFF,
    #[default]
    AUTO,
}

/// Wide dynamic range options (version 2.0).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct WideDynamicRangeOptions20 {
    /// Supported modes.
    #[serde(rename = "Mode", default)]
    pub mode: Vec<WideDynamicMode>,

    /// Level range.
    #[serde(rename = "Level", default, skip_serializing_if = "Option::is_none")]
    pub level: Option<FloatRange>,
}

/// Wide dynamic range mode.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum WideDynamicMode {
    #[default]
    OFF,
    ON,
}

/// White balance options (version 2.0).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct WhiteBalanceOptions20 {
    /// Supported modes.
    #[serde(rename = "Mode", default)]
    pub mode: Vec<WhiteBalanceMode>,

    /// Cr gain range.
    #[serde(rename = "YrGain", default, skip_serializing_if = "Option::is_none")]
    pub yr_gain: Option<FloatRange>,

    /// Cb gain range.
    #[serde(rename = "YbGain", default, skip_serializing_if = "Option::is_none")]
    pub yb_gain: Option<FloatRange>,

    /// Extension.
    #[serde(rename = "Extension", default, skip_serializing_if = "Option::is_none")]
    pub extension: Option<Extension>,
}

/// White balance mode.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum WhiteBalanceMode {
    #[default]
    AUTO,
    MANUAL,
}

/// Image stabilization options.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ImageStabilizationOptions {
    /// Supported modes.
    #[serde(rename = "Mode", default)]
    pub mode: Vec<ImageStabilizationMode>,

    /// Level range.
    #[serde(rename = "Level", default, skip_serializing_if = "Option::is_none")]
    pub level: Option<FloatRange>,

    /// Extension.
    #[serde(rename = "Extension", default, skip_serializing_if = "Option::is_none")]
    pub extension: Option<Extension>,
}

/// Image stabilization mode.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum ImageStabilizationMode {
    #[default]
    OFF,
    ON,
    AUTO,
    Extended,
}

/// IR cut filter auto adjustment options.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct IrCutFilterAutoAdjustmentOptions {
    /// Boundary type options.
    #[serde(rename = "BoundaryType", default)]
    pub boundary_type: Vec<String>,

    /// Response time support.
    #[serde(
        rename = "ResponseTimeSupported",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub response_time_supported: Option<bool>,

    /// Boundary offset range.
    #[serde(
        rename = "BoundaryOffset",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub boundary_offset: Option<FloatRange>,

    /// Extension.
    #[serde(rename = "Extension", default, skip_serializing_if = "Option::is_none")]
    pub extension: Option<Extension>,
}

/// Tone compensation options.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ToneCompensationOptions {
    /// Supported modes.
    #[serde(rename = "Mode", default)]
    pub mode: Vec<String>,

    /// Level support.
    #[serde(rename = "Level", default, skip_serializing_if = "Option::is_none")]
    pub level: Option<bool>,
}

/// Defogging options.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DefoggingOptions {
    /// Supported modes.
    #[serde(rename = "Mode", default)]
    pub mode: Vec<String>,

    /// Level support.
    #[serde(rename = "Level", default, skip_serializing_if = "Option::is_none")]
    pub level: Option<bool>,
}

/// Noise reduction options.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct NoiseReductionOptions {
    /// Level support.
    #[serde(rename = "Level", default, skip_serializing_if = "Option::is_none")]
    pub level: Option<bool>,
}

// ============================================================================
// Focus Movement
// ============================================================================

/// Move request (focus).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "Move")]
pub struct Move {
    /// Video source token.
    #[serde(rename = "VideoSourceToken")]
    pub video_source_token: ReferenceToken,

    /// Focus move parameters.
    #[serde(rename = "Focus")]
    pub focus: FocusMove,
}

/// Move response (focus).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "timg:MoveResponse")]
pub struct MoveResponse {}

/// Focus move parameters.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct FocusMove {
    /// Absolute move.
    #[serde(rename = "Absolute", default, skip_serializing_if = "Option::is_none")]
    pub absolute: Option<AbsoluteFocus>,

    /// Relative move.
    #[serde(rename = "Relative", default, skip_serializing_if = "Option::is_none")]
    pub relative: Option<RelativeFocus>,

    /// Continuous move.
    #[serde(
        rename = "Continuous",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub continuous: Option<ContinuousFocus>,
}

/// Absolute focus move.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AbsoluteFocus {
    /// Target position.
    #[serde(rename = "Position")]
    pub position: f32,

    /// Move speed (optional).
    #[serde(rename = "Speed", default, skip_serializing_if = "Option::is_none")]
    pub speed: Option<f32>,
}

/// Relative focus move.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelativeFocus {
    /// Distance to move.
    #[serde(rename = "Distance")]
    pub distance: f32,

    /// Move speed (optional).
    #[serde(rename = "Speed", default, skip_serializing_if = "Option::is_none")]
    pub speed: Option<f32>,
}

/// Continuous focus move.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContinuousFocus {
    /// Move speed.
    #[serde(rename = "Speed")]
    pub speed: f32,
}

/// Stop request (focus).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "Stop")]
pub struct Stop {
    /// Video source token.
    #[serde(rename = "VideoSourceToken")]
    pub video_source_token: ReferenceToken,
}

/// Stop response (focus).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "timg:StopResponse")]
pub struct StopResponse {}

/// GetMoveOptions request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetMoveOptions")]
pub struct GetMoveOptions {
    /// Video source token.
    #[serde(rename = "VideoSourceToken")]
    pub video_source_token: ReferenceToken,
}

/// GetMoveOptions response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "timg:GetMoveOptionsResponse")]
pub struct GetMoveOptionsResponse {
    /// Focus move options.
    #[serde(rename = "timg:MoveOptions")]
    pub move_options: MoveOptions20,
}

/// Focus move options (version 2.0).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MoveOptions20 {
    /// Absolute move options.
    #[serde(rename = "Absolute", default, skip_serializing_if = "Option::is_none")]
    pub absolute: Option<AbsoluteFocusOptions>,

    /// Relative move options.
    #[serde(rename = "Relative", default, skip_serializing_if = "Option::is_none")]
    pub relative: Option<RelativeFocusOptions20>,

    /// Continuous move options.
    #[serde(
        rename = "Continuous",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub continuous: Option<ContinuousFocusOptions>,
}

/// Absolute focus options.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AbsoluteFocusOptions {
    /// Position range.
    #[serde(rename = "Position")]
    pub position: FloatRange,

    /// Speed range.
    #[serde(rename = "Speed", default, skip_serializing_if = "Option::is_none")]
    pub speed: Option<FloatRange>,
}

/// Relative focus options (version 2.0).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RelativeFocusOptions20 {
    /// Distance range.
    #[serde(rename = "Distance")]
    pub distance: FloatRange,

    /// Speed range.
    #[serde(rename = "Speed", default, skip_serializing_if = "Option::is_none")]
    pub speed: Option<FloatRange>,
}

/// Continuous focus options.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ContinuousFocusOptions {
    /// Speed range.
    #[serde(rename = "Speed")]
    pub speed: FloatRange,
}

// ============================================================================
// Status
// ============================================================================

/// GetStatus request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetStatus")]
pub struct GetStatus {
    /// Video source token.
    #[serde(rename = "VideoSourceToken")]
    pub video_source_token: ReferenceToken,
}

/// GetStatus response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "timg:GetStatusResponse")]
pub struct GetStatusResponse {
    /// Imaging status.
    #[serde(rename = "timg:Status")]
    pub status: ImagingStatus20,
}

// ============================================================================
// Presets
// ============================================================================

/// GetPresets request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetPresets")]
pub struct GetPresets {
    /// Video source token.
    #[serde(rename = "VideoSourceToken")]
    pub video_source_token: ReferenceToken,
}

/// GetPresets response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "timg:GetPresetsResponse")]
pub struct GetPresetsResponse {
    /// List of imaging presets.
    #[serde(rename = "timg:Preset", default)]
    pub presets: Vec<ImagingPreset>,
}

/// Imaging preset.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImagingPreset {
    /// Preset token.
    #[serde(rename = "@token")]
    pub token: ReferenceToken,

    /// Preset type.
    #[serde(rename = "@type", default, skip_serializing_if = "Option::is_none")]
    pub preset_type: Option<String>,

    /// Preset name.
    #[serde(rename = "Name")]
    pub name: String,
}

/// GetCurrentPreset request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetCurrentPreset")]
pub struct GetCurrentPreset {
    /// Video source token.
    #[serde(rename = "VideoSourceToken")]
    pub video_source_token: ReferenceToken,
}

/// GetCurrentPreset response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "timg:GetCurrentPresetResponse")]
pub struct GetCurrentPresetResponse {
    /// Current imaging preset.
    #[serde(
        rename = "timg:Preset",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub preset: Option<ImagingPreset>,
}

/// SetCurrentPreset request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "SetCurrentPreset")]
pub struct SetCurrentPreset {
    /// Video source token.
    #[serde(rename = "VideoSourceToken")]
    pub video_source_token: ReferenceToken,

    /// Preset token.
    #[serde(rename = "PresetToken")]
    pub preset_token: ReferenceToken,
}

/// SetCurrentPreset response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "timg:SetCurrentPresetResponse")]
pub struct SetCurrentPresetResponse {}

// ============================================================================
// Service Capabilities
// ============================================================================

/// GetServiceCapabilities request.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetServiceCapabilities")]
pub struct GetServiceCapabilities {}

/// GetServiceCapabilities response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "timg:GetServiceCapabilitiesResponse")]
pub struct GetServiceCapabilitiesResponse {
    /// Imaging service capabilities.
    #[serde(rename = "timg:Capabilities")]
    pub capabilities: ImagingServiceCapabilities,
}

/// Imaging service capabilities.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ImagingServiceCapabilities {
    /// Image stabilization support.
    #[serde(
        rename = "@ImageStabilization",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub image_stabilization: Option<bool>,

    /// Presets support.
    #[serde(rename = "@Presets", default, skip_serializing_if = "Option::is_none")]
    pub presets: Option<bool>,

    /// Adaptive stream support.
    #[serde(
        rename = "@AdaptablePreset",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub adaptable_preset: Option<bool>,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backlight_compensation_mode_default() {
        let mode = BacklightCompensationMode::default();
        assert_eq!(mode, BacklightCompensationMode::OFF);
    }

    #[test]
    fn test_exposure_mode_default() {
        let mode = ExposureMode::default();
        assert_eq!(mode, ExposureMode::AUTO);
    }

    #[test]
    fn test_auto_focus_mode_default() {
        let mode = AutoFocusMode::default();
        assert_eq!(mode, AutoFocusMode::AUTO);
    }

    #[test]
    fn test_white_balance_mode_default() {
        let mode = WhiteBalanceMode::default();
        assert_eq!(mode, WhiteBalanceMode::AUTO);
    }

    #[test]
    fn test_ir_cut_filter_mode_default() {
        let mode = IrCutFilterMode::default();
        assert_eq!(mode, IrCutFilterMode::AUTO);
    }

    #[test]
    fn test_image_stabilization_mode_default() {
        let mode = ImageStabilizationMode::default();
        assert_eq!(mode, ImageStabilizationMode::OFF);
    }

    #[test]
    fn test_wide_dynamic_mode_default() {
        let mode = WideDynamicMode::default();
        assert_eq!(mode, WideDynamicMode::OFF);
    }

    #[test]
    fn test_get_presets_response_default() {
        let response = GetPresetsResponse::default();
        assert!(response.presets.is_empty());
    }
}
