//! Imaging Service specific fault types and mappings.
//!
//! This module defines faults specific to the Imaging Service operations
//! as defined in the ONVIF Imaging Service WSDL specification.

use crate::onvif::error::OnvifError;

// ============================================================================
// Video Source Token Faults
// ============================================================================

/// Create an InvalidVideoSourceToken fault.
///
/// Used when the specified video source token is not recognized.
pub fn invalid_video_source_token(token: &str) -> OnvifError {
    OnvifError::InvalidArgVal {
        subcode: "ter:InvalidToken".to_string(),
        reason: format!("Invalid video source token: {}", token),
    }
}

// ============================================================================
// Parameter Faults
// ============================================================================

/// Create a ParameterOutOfRange fault.
///
/// Used when a parameter value is outside the valid range.
pub fn parameter_out_of_range(parameter: &str, value: f32, min: f32, max: f32) -> OnvifError {
    OnvifError::InvalidArgVal {
        subcode: "ter:InvalidArgVal".to_string(),
        reason: format!(
            "Parameter '{}' value {} is out of range ({} - {})",
            parameter, value, min, max
        ),
    }
}

// ============================================================================
// Focus Faults
// ============================================================================

/// Create a FocusNotSupported fault.
///
/// Used when focus operations are requested but not supported by the device.
pub fn focus_not_supported() -> OnvifError {
    OnvifError::ActionNotSupported("Focus operation not supported".to_string())
}

// ============================================================================
// Preset Faults
// ============================================================================

/// Create a PresetsNotSupported fault.
///
/// Used when preset operations are requested but not supported by the device.
pub fn presets_not_supported() -> OnvifError {
    OnvifError::ActionNotSupported("Imaging presets are not supported on this device".to_string())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_video_source_token() {
        let err = invalid_video_source_token("InvalidToken");
        assert!(
            matches!(err, OnvifError::InvalidArgVal { ref subcode, .. } if subcode == "ter:InvalidToken")
        );
        assert!(err.to_string().contains("InvalidToken"));
    }

    #[test]
    fn test_parameter_out_of_range() {
        let err = parameter_out_of_range("Brightness", 150.0, 0.0, 100.0);
        assert!(
            matches!(err, OnvifError::InvalidArgVal { ref subcode, .. } if subcode == "ter:InvalidArgVal")
        );
        assert!(err.to_string().contains("Brightness"));
        assert!(err.to_string().contains("150"));
    }

    #[test]
    fn test_focus_not_supported() {
        let err = focus_not_supported();
        assert!(matches!(err, OnvifError::ActionNotSupported(_)));
        assert!(err.to_string().contains("Focus"));
    }

    #[test]
    fn test_presets_not_supported() {
        let err = presets_not_supported();
        assert!(matches!(err, OnvifError::ActionNotSupported(_)));
        assert!(err.to_string().contains("presets"));
    }
}
