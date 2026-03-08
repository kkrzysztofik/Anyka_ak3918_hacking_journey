//! PTZ-specific SOAP fault constructors.
//!
//! This module provides PTZ-specific fault error constructors that follow
//! ONVIF fault codes for PTZ-related errors. These are used throughout
//! the PTZ service for consistent error handling.
//!
//! # Usage
//!
//! ```
//! use onvif_rust::onvif::ptz::faults::{no_preset, too_many_presets};
//!
//! // Preset not found
//! let err = no_preset("Preset999");
//! assert!(err.to_string().contains("Preset999"));
//!
//! // Too many presets
//! let err = too_many_presets(255);
//! assert!(err.to_string().contains("255"));
//! ```

use crate::onvif::error::OnvifError;

/// Create a "no preset" fault (ter:NoPreset).
///
/// This error is returned when a client references a preset that does not exist.
///
/// # Arguments
///
/// * `token` - The preset token that was not found
///
/// # Returns
///
/// An `OnvifError::NotFound` with ter:NoPreset subcode.
///
/// # Example
///
/// ```
/// use onvif_rust::onvif::ptz::faults::no_preset;
///
/// let err = no_preset("Preset001");
/// assert!(err.to_string().contains("Preset001"));
/// assert!(err.to_string().contains("not found"));
/// ```
pub fn no_preset(token: &str) -> OnvifError {
    OnvifError::NotFound(format!("Preset '{}' not found", token))
}

/// Create a "too many presets" fault (ter:TooManyPresets).
///
/// This error is returned when the client attempts to create more presets
/// than the device supports.
///
/// # Arguments
///
/// * `max` - The maximum number of presets supported
///
/// # Returns
///
/// An `OnvifError::InvalidArgVal` with ter:TooManyPresets subcode.
///
/// # Example
///
/// ```
/// use onvif_rust::onvif::ptz::faults::too_many_presets;
///
/// let err = too_many_presets(255);
/// assert!(err.to_string().contains("255"));
/// ```
pub fn too_many_presets(max: i32) -> OnvifError {
    OnvifError::InvalidArgVal {
        subcode: "TooManyPresets".to_string(),
        reason: format!("Maximum number of {} presets reached", max),
    }
}

/// Create a "moving pan/tilt" fault (ter:MovingPanTilt).
///
/// This error is returned when an operation cannot be performed because
/// the device is currently moving the pan/tilt axes.
///
/// # Returns
///
/// An `OnvifError::InvalidArgVal` with ter:MovingPanTilt subcode.
///
/// # Example
///
/// ```
/// use onvif_rust::onvif::ptz::faults::moving_pan_tilt;
///
/// let err = moving_pan_tilt();
/// assert!(err.to_string().contains("Pan/Tilt"));
/// ```
pub fn moving_pan_tilt() -> OnvifError {
    OnvifError::InvalidArgVal {
        subcode: "MovingPanTilt".to_string(),
        reason: "Pan/Tilt movement in progress".to_string(),
    }
}

/// Create a "moving zoom" fault (ter:MovingZoom).
///
/// This error is returned when an operation cannot be performed because
/// the device is currently moving the zoom axis.
///
/// # Returns
///
/// An `OnvifError::InvalidArgVal` with ter:MovingZoom subcode.
///
/// # Example
///
/// ```
/// use onvif_rust::onvif::ptz::faults::moving_zoom;
///
/// let err = moving_zoom();
/// assert!(err.to_string().contains("Zoom"));
/// ```
pub fn moving_zoom() -> OnvifError {
    OnvifError::InvalidArgVal {
        subcode: "MovingZoom".to_string(),
        reason: "Zoom movement in progress".to_string(),
    }
}

/// Create an "invalid node token" fault (ter:InvalidNodeToken).
///
/// This error is returned when a PTZ node token is invalid or unknown.
///
/// # Arguments
///
/// * `token` - The invalid node token
///
/// # Returns
///
/// An `OnvifError::NotFound` with appropriate subcode.
///
/// # Example
///
/// ```
/// use onvif_rust::onvif::ptz::faults::invalid_node_token;
///
/// let err = invalid_node_token("InvalidNode001");
/// assert!(err.to_string().contains("InvalidNode001"));
/// assert!(err.to_string().contains("not found"));
/// ```
pub fn invalid_node_token(token: &str) -> OnvifError {
    OnvifError::NotFound(format!("PTZ node '{}' not found", token))
}

/// Create an "invalid configuration token" fault (ter:InvalidPTZConfiguration).
///
/// This error is returned when a PTZ configuration token is invalid or unknown.
///
/// # Arguments
///
/// * `token` - The invalid configuration token
///
/// # Returns
///
/// An `OnvifError::NotFound` with appropriate subcode.
///
/// # Example
///
/// ```
/// use onvif_rust::onvif::ptz::faults::invalid_config_token;
///
/// let err = invalid_config_token("InvalidConfig001");
/// assert!(err.to_string().contains("InvalidConfig001"));
/// assert!(err.to_string().contains("not found"));
/// ```
pub fn invalid_config_token(token: &str) -> OnvifError {
    OnvifError::NotFound(format!("PTZ configuration '{}' not found", token))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test no_preset

    #[test]
    fn test_no_preset_creation() {
        let err = no_preset("Preset001");
        match err {
            OnvifError::NotFound(msg) => {
                assert!(msg.contains("Preset001"));
                assert!(msg.contains("not found"));
            }
            _ => panic!("Expected NotFound variant"),
        }
    }

    #[test]
    fn test_no_preset_empty_token() {
        let err = no_preset("");
        let msg = err.to_string();
        assert!(msg.contains("Preset"));
        assert!(msg.contains("not found"));
    }

    // Test too_many_presets

    #[test]
    fn test_too_many_presets_creation() {
        let err = too_many_presets(255);
        match err {
            OnvifError::InvalidArgVal { subcode, reason } => {
                assert_eq!(subcode, "TooManyPresets");
                assert!(reason.contains("255"));
            }
            _ => panic!("Expected InvalidArgVal variant"),
        }
    }

    #[test]
    fn test_too_many_presets_soap_fault() {
        let err = too_many_presets(100);
        let fault = err.to_soap_fault();
        assert!(fault.contains("TooManyPresets"));
    }

    // Test moving_pan_tilt

    #[test]
    fn test_moving_pan_tilt_creation() {
        let err = moving_pan_tilt();
        match err {
            OnvifError::InvalidArgVal { subcode, reason } => {
                assert_eq!(subcode, "MovingPanTilt");
                assert!(reason.contains("Pan/Tilt"));
            }
            _ => panic!("Expected InvalidArgVal variant"),
        }
    }

    #[test]
    fn test_moving_pan_tilt_soap_fault() {
        let err = moving_pan_tilt();
        let fault = err.to_soap_fault();
        assert!(fault.contains("MovingPanTilt"));
    }

    // Test moving_zoom

    #[test]
    fn test_moving_zoom_creation() {
        let err = moving_zoom();
        match err {
            OnvifError::InvalidArgVal { subcode, reason } => {
                assert_eq!(subcode, "MovingZoom");
                assert!(reason.contains("Zoom"));
            }
            _ => panic!("Expected InvalidArgVal variant"),
        }
    }

    #[test]
    fn test_moving_zoom_soap_fault() {
        let err = moving_zoom();
        let fault = err.to_soap_fault();
        assert!(fault.contains("MovingZoom"));
    }

    // Test invalid_node_token

    #[test]
    fn test_invalid_node_token_creation() {
        let err = invalid_node_token("Node001");
        match err {
            OnvifError::NotFound(msg) => {
                assert!(msg.contains("Node001"));
                assert!(msg.contains("not found"));
            }
            _ => panic!("Expected NotFound variant"),
        }
    }

    #[test]
    fn test_invalid_node_token_soap_fault() {
        let err = invalid_node_token("BadNode");
        let fault = err.to_soap_fault();
        assert!(fault.contains("NotFound"));
    }

    // Test invalid_config_token

    #[test]
    fn test_invalid_config_token_creation() {
        let err = invalid_config_token("Config001");
        match err {
            OnvifError::NotFound(msg) => {
                assert!(msg.contains("Config001"));
                assert!(msg.contains("not found"));
            }
            _ => panic!("Expected NotFound variant"),
        }
    }

    #[test]
    fn test_invalid_config_token_soap_fault() {
        let err = invalid_config_token("BadConfig");
        let fault = err.to_soap_fault();
        assert!(fault.contains("NotFound"));
    }

    // Test HTTP status codes

    #[test]
    fn test_no_preset_http_status() {
        let err = no_preset("test");
        assert_eq!(err.http_status(), axum::http::StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_too_many_presets_http_status() {
        let err = too_many_presets(255);
        assert_eq!(err.http_status(), axum::http::StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_moving_pan_tilt_http_status() {
        let err = moving_pan_tilt();
        assert_eq!(err.http_status(), axum::http::StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_moving_zoom_http_status() {
        let err = moving_zoom();
        assert_eq!(err.http_status(), axum::http::StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_invalid_node_token_http_status() {
        let err = invalid_node_token("test");
        assert_eq!(err.http_status(), axum::http::StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_invalid_config_token_http_status() {
        let err = invalid_config_token("test");
        assert_eq!(err.http_status(), axum::http::StatusCode::NOT_FOUND);
    }
}
