//! PTZ Service types and type utilities.
//!
//! This module re-exports WSDL-generated types from `crate::onvif::types::ptz`
//! and adds any implementation-specific type extensions.

// Re-export all WSDL-generated PTZ types
pub use crate::onvif::types::ptz::*;

// Re-export common PTZ-related types
pub use crate::onvif::types::common::{
    FloatRange, MoveStatus, PTZConfiguration, PTZMoveStatus, PTZPreset, PTZSpeed, PTZStatus,
    PTZVector, PanTiltLimits, ReferenceToken, Space1DDescription, Space2DDescription, Vector1D,
    Vector2D, ZoomLimits,
};

// ============================================================================
// Constants
// ============================================================================

/// PTZ Service namespace URI.
pub const PTZ_SERVICE_NAMESPACE: &str = "http://www.onvif.org/ver20/ptz/wsdl";

/// Default PTZ node token.
pub const DEFAULT_NODE_TOKEN: &str = "PTZNodeToken1";

/// Default PTZ configuration token.
pub const DEFAULT_CONFIG_TOKEN: &str = "PTZConfigToken1";

/// Maximum number of presets supported.
pub const MAX_PRESETS: i32 = 255;

/// Default PTZ timeout in seconds.
pub const DEFAULT_PTZ_TIMEOUT: &str = "PT5S";

/// Minimum PTZ timeout.
pub const MIN_PTZ_TIMEOUT: &str = "PT1S";

/// Maximum PTZ timeout.
pub const MAX_PTZ_TIMEOUT: &str = "PT60S";

// ============================================================================
// Space URIs (ONVIF standard)
// ============================================================================

/// Absolute pan/tilt position space (normalized -1 to 1).
pub const SPACE_ABSOLUTE_PAN_TILT: &str =
    "http://www.onvif.org/ver10/tptz/PanTiltSpaces/PositionGenericSpace";

/// Absolute zoom position space (normalized 0 to 1).
pub const SPACE_ABSOLUTE_ZOOM: &str =
    "http://www.onvif.org/ver10/tptz/ZoomSpaces/PositionGenericSpace";

/// Relative pan/tilt translation space (normalized -1 to 1).
pub const SPACE_RELATIVE_PAN_TILT: &str =
    "http://www.onvif.org/ver10/tptz/PanTiltSpaces/TranslationGenericSpace";

/// Relative zoom translation space (normalized -1 to 1).
pub const SPACE_RELATIVE_ZOOM: &str =
    "http://www.onvif.org/ver10/tptz/ZoomSpaces/TranslationGenericSpace";

/// Continuous pan/tilt velocity space (normalized -1 to 1).
pub const SPACE_CONTINUOUS_PAN_TILT: &str =
    "http://www.onvif.org/ver10/tptz/PanTiltSpaces/VelocityGenericSpace";

/// Continuous zoom velocity space (normalized -1 to 1).
pub const SPACE_CONTINUOUS_ZOOM: &str =
    "http://www.onvif.org/ver10/tptz/ZoomSpaces/VelocityGenericSpace";

/// Pan/tilt speed space (normalized 0 to 1).
pub const SPACE_PAN_TILT_SPEED: &str =
    "http://www.onvif.org/ver10/tptz/PanTiltSpaces/GenericSpeedSpace";

/// Zoom speed space (normalized 0 to 1).
pub const SPACE_ZOOM_SPEED: &str = "http://www.onvif.org/ver10/tptz/ZoomSpaces/GenericSpeedSpace";

// ============================================================================
// Helper Functions
// ============================================================================

/// Build PTZ spaces with standard ONVIF ranges.
pub fn build_ptz_spaces() -> PTZSpaces {
    PTZSpaces {
        absolute_pan_tilt_position_space: vec![Space2DDescription {
            uri: SPACE_ABSOLUTE_PAN_TILT.to_string(),
            x_range: FloatRange {
                min: -1.0,
                max: 1.0,
            },
            y_range: FloatRange {
                min: -1.0,
                max: 1.0,
            },
        }],
        absolute_zoom_position_space: vec![Space1DDescription {
            uri: SPACE_ABSOLUTE_ZOOM.to_string(),
            x_range: FloatRange { min: 0.0, max: 1.0 },
        }],
        relative_pan_tilt_translation_space: vec![Space2DDescription {
            uri: SPACE_RELATIVE_PAN_TILT.to_string(),
            x_range: FloatRange {
                min: -1.0,
                max: 1.0,
            },
            y_range: FloatRange {
                min: -1.0,
                max: 1.0,
            },
        }],
        relative_zoom_translation_space: vec![Space1DDescription {
            uri: SPACE_RELATIVE_ZOOM.to_string(),
            x_range: FloatRange {
                min: -1.0,
                max: 1.0,
            },
        }],
        continuous_pan_tilt_velocity_space: vec![Space2DDescription {
            uri: SPACE_CONTINUOUS_PAN_TILT.to_string(),
            x_range: FloatRange {
                min: -1.0,
                max: 1.0,
            },
            y_range: FloatRange {
                min: -1.0,
                max: 1.0,
            },
        }],
        continuous_zoom_velocity_space: vec![Space1DDescription {
            uri: SPACE_CONTINUOUS_ZOOM.to_string(),
            x_range: FloatRange {
                min: -1.0,
                max: 1.0,
            },
        }],
        pan_tilt_speed_space: vec![Space1DDescription {
            uri: SPACE_PAN_TILT_SPEED.to_string(),
            x_range: FloatRange { min: 0.0, max: 1.0 },
        }],
        zoom_speed_space: vec![Space1DDescription {
            uri: SPACE_ZOOM_SPEED.to_string(),
            x_range: FloatRange { min: 0.0, max: 1.0 },
        }],
        extension: None,
    }
}

/// Build default PTZ configuration options.
pub fn build_configuration_options() -> PTZConfigurationOptions {
    PTZConfigurationOptions {
        spaces: build_ptz_spaces(),
        ptz_timeout: DurationRange {
            min: MIN_PTZ_TIMEOUT.to_string(),
            max: MAX_PTZ_TIMEOUT.to_string(),
        },
        pt_control_direction: None,
        extension: None,
        ptz_ramps: None,
    }
}

/// Build default service capabilities.
pub fn build_service_capabilities() -> PTZServiceCapabilities {
    PTZServiceCapabilities {
        e_flip: Some(false),
        reverse: Some(false),
        get_compatible_configurations: Some(true),
        move_status: Some(true),
        status_position: Some(true),
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_namespace_constant() {
        assert_eq!(PTZ_SERVICE_NAMESPACE, "http://www.onvif.org/ver20/ptz/wsdl");
    }

    #[test]
    fn test_default_tokens() {
        assert!(!DEFAULT_NODE_TOKEN.is_empty());
        assert!(!DEFAULT_CONFIG_TOKEN.is_empty());
    }

    #[test]
    fn test_max_presets() {
        assert_eq!(MAX_PRESETS, 255);
    }

    #[test]
    fn test_build_ptz_spaces() {
        let spaces = build_ptz_spaces();

        // Verify pan/tilt absolute space
        assert_eq!(spaces.absolute_pan_tilt_position_space.len(), 1);
        let pt_space = &spaces.absolute_pan_tilt_position_space[0];
        assert_eq!(pt_space.uri, SPACE_ABSOLUTE_PAN_TILT);
        assert_eq!(pt_space.x_range.min, -1.0);
        assert_eq!(pt_space.x_range.max, 1.0);

        // Verify zoom absolute space
        assert_eq!(spaces.absolute_zoom_position_space.len(), 1);
        let z_space = &spaces.absolute_zoom_position_space[0];
        assert_eq!(z_space.uri, SPACE_ABSOLUTE_ZOOM);
        assert_eq!(z_space.x_range.min, 0.0);
        assert_eq!(z_space.x_range.max, 1.0);
    }

    #[test]
    fn test_build_configuration_options() {
        let options = build_configuration_options();

        // Verify timeout range
        assert_eq!(options.ptz_timeout.min, MIN_PTZ_TIMEOUT);
        assert_eq!(options.ptz_timeout.max, MAX_PTZ_TIMEOUT);
    }

    #[test]
    fn test_build_service_capabilities() {
        let caps = build_service_capabilities();

        assert_eq!(caps.move_status, Some(true));
        assert_eq!(caps.status_position, Some(true));
        assert_eq!(caps.get_compatible_configurations, Some(true));
    }
}
