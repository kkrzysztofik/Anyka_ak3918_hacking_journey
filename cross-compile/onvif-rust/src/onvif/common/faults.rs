//! Shared SOAP fault builder helpers for ONVIF services.
//!
//! This module provides convenience functions for creating common ONVIF
//! SOAP fault errors. These helpers wrap the core `OnvifError` types
//! with commonly needed parameters.
//!
//! # Usage
//!
//! ```
//! use onvif_rust::onvif::common::faults::{
//!     invalid_token, no_entity, action_not_supported, out_of_range, hardware_failure,
//! };
//! use onvif_rust::onvif::error::OnvifError;
//!
//! // Invalid token error
//! let err = invalid_token("ProfileToken", "InvalidToken123");
//! assert!(err.to_string().contains("InvalidToken"));
//!
//! // Entity not found
//! let err = no_entity("ter:ProfileNotFound", "profile", "ProfileToken123");
//! assert!(err.to_string().contains("profile"));
//!
//! // Action not supported
//! let err = action_not_supported("GetUnknownFeature");
//! assert!(err.to_string().contains("Action not supported"));
//!
//! // Out of range
//! let err = out_of_range("brightness", 150.0, 0.0, 100.0);
//! assert!(err.to_string().contains("brightness"));
//!
//! // Hardware failure
//! let err = hardware_failure("PTZ movement", "Motor stuck");
//! assert!(err.to_string().contains("Hardware failure"));
//! ```

use crate::onvif::error::OnvifError;

/// Create an invalid reference token error.
///
/// This is used when a token (profile token, PTZ configuration token, etc.)
/// is provided but fails validation.
///
/// # Arguments
///
/// * `kind` - The type of token (e.g., "ProfileToken", "PTZConfigurationToken")
/// * `token` - The invalid token value
///
/// # Returns
///
/// An `OnvifError::NotFound` with a descriptive message.
///
/// # Example
///
/// ```
/// use onvif_rust::onvif::common::faults::invalid_token;
///
/// let err = invalid_token("ProfileToken", "InvalidProfile123");
/// assert!(err.to_string().contains("ProfileToken"));
/// assert!(err.to_string().contains("InvalidProfile123"));
/// ```
pub fn invalid_token(kind: &str, token: &str) -> OnvifError {
    OnvifError::NotFound(format!("Invalid {}: '{}'", kind, token))
}

/// Create a "no entity" error for when a referenced entity doesn't exist.
///
/// This is used when a client references an entity (profile, configuration, etc.)
/// that doesn't exist in the system.
///
/// # Arguments
///
/// * `subcode` - The ONVIF fault subcode (e.g., "ter:ProfileNotFound")
/// * `entity` - The type of entity (e.g., "profile", "PTZConfiguration")
/// * `token` - The token that was used to reference the entity
///
/// # Returns
///
/// An `OnvifError::NotFound` with a descriptive message.
///
/// # Example
///
/// ```
/// use onvif_rust::onvif::common::faults::no_entity;
///
/// let err = no_entity("ter:ProfileNotFound", "profile", "Profile001");
/// assert!(err.to_string().contains("profile"));
/// assert!(err.to_string().contains("Profile001"));
/// assert!(err.to_string().contains("not found"));
/// ```
pub fn no_entity(subcode: &str, entity: &str, token: &str) -> OnvifError {
    OnvifError::NotFound(format!(
        "{} with token '{}' not found ({})",
        entity, token, subcode
    ))
}

/// Create an "action not supported" error.
///
/// This is used when a client requests an operation that the service
/// doesn't support.
///
/// # Arguments
///
/// * `action` - The action/operation name that is not supported
///
/// # Returns
///
/// An `OnvifError::ActionNotSupported` error.
///
/// # Example
///
/// ```
/// use onvif_rust::onvif::common::faults::action_not_supported;
///
/// let err = action_not_supported("GetCustomFeature");
/// assert!(err.to_string().contains("GetCustomFeature"));
/// ```
pub fn action_not_supported(action: &str) -> OnvifError {
    OnvifError::ActionNotSupported(action.to_string())
}

/// Create an "out of range" error for numeric parameter validation.
///
/// This is used when a numeric parameter value is outside the allowed range.
///
/// # Arguments
///
/// * `param` - The parameter name
/// * `value` - The invalid value that was provided
/// * `min` - The minimum allowed value
/// * `max` - The maximum allowed value
///
/// # Returns
///
/// An `OnvifError::InvalidArgVal` error with OutOfRange subcode.
///
/// # Example
///
/// ```
/// use onvif_rust::onvif::common::faults::out_of_range;
///
/// let err = out_of_range("brightness", 150.0, 0.0, 100.0);
/// assert!(err.to_string().contains("brightness"));
/// assert!(err.to_string().contains("0"));
/// assert!(err.to_string().contains("100"));
/// ```
pub fn out_of_range(param: &str, value: f32, min: f32, max: f32) -> OnvifError {
    OnvifError::InvalidArgVal {
        subcode: "ter:OutOfRange".to_string(),
        reason: format!(
            "'{}' value {} is out of range (must be between {} and {})",
            param, value, min, max
        ),
    }
}

/// Create an invalid video source token error.
///
/// This is a specialized form of `invalid_token` for video source tokens,
/// providing consistent error messages across all services that validate
/// video source references.
///
/// # Arguments
///
/// * `token` - The invalid video source token value
///
/// # Returns
///
/// An `OnvifError::InvalidArgVal` with `ter:InvalidToken` subcode.
pub fn invalid_video_source(token: &str) -> OnvifError {
    OnvifError::InvalidArgVal {
        subcode: "ter:InvalidToken".to_string(),
        reason: format!("Invalid video source token: {}", token),
    }
}

/// Create a "hardware failure" error.
///
/// This is used when a hardware-related operation fails.
///
/// # Arguments
///
/// * `operation` - The operation that failed (e.g., "PTZ movement", "Sensor read")
/// * `detail` - Additional detail about the failure
///
/// # Returns
///
/// An `OnvifError::HardwareFailure` error.
///
/// # Example
///
/// ```
/// use onvif_rust::onvif::common::faults::hardware_failure;
///
/// let err = hardware_failure("PTZ movement", "Motor stalled");
/// assert!(err.to_string().contains("PTZ movement"));
/// assert!(err.to_string().contains("Motor stalled"));
/// ```
pub fn hardware_failure(operation: &str, detail: &str) -> OnvifError {
    OnvifError::HardwareFailure(format!("{}: {}", operation, detail))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test invalid_token

    #[test]
    fn test_invalid_token_creation() {
        let err = invalid_token("ProfileToken", "InvalidToken123");
        match err {
            OnvifError::NotFound(msg) => {
                assert!(msg.contains("ProfileToken"));
                assert!(msg.contains("InvalidToken123"));
            }
            _ => panic!("Expected NotFound variant"),
        }
    }

    #[test]
    fn test_invalid_token_soap_fault() {
        let err = invalid_token("ProfileToken", "BadToken");
        let fault = err.to_soap_fault();
        assert!(fault.contains("NotFound"));
    }

    // Test no_entity

    #[test]
    fn test_no_entity_creation() {
        let err = no_entity("ter:ProfileNotFound", "profile", "Profile001");
        match err {
            OnvifError::NotFound(msg) => {
                assert!(msg.contains("profile"));
                assert!(msg.contains("Profile001"));
                assert!(msg.contains("ter:ProfileNotFound"));
                // Verify the message clearly states what entity was not found
                assert!(msg.contains("with token"));
                assert!(msg.contains("not found"));
            }
            _ => panic!("Expected NotFound variant"),
        }
    }

    #[test]
    fn test_no_entity_soap_fault() {
        let err = no_entity("ter:PTZConfigurationNotFound", "PTZConfiguration", "PTZ001");
        let fault = err.to_soap_fault();
        assert!(fault.contains("NotFound"));
    }

    // Test action_not_supported

    #[test]
    fn test_action_not_supported_creation() {
        let err = action_not_supported("GetCustomFeature");
        match err {
            OnvifError::ActionNotSupported(action) => {
                assert_eq!(action, "GetCustomFeature");
            }
            _ => panic!("Expected ActionNotSupported variant"),
        }
    }

    #[test]
    fn test_action_not_supported_http_status() {
        let err = action_not_supported("SomeAction");
        assert_eq!(err.http_status(), axum::http::StatusCode::BAD_REQUEST);
    }

    // Test out_of_range

    #[test]
    fn test_out_of_range_creation() {
        let err = out_of_range("brightness", 150.0, 0.0, 100.0);
        match err {
            OnvifError::InvalidArgVal { subcode, reason } => {
                assert_eq!(subcode, "ter:OutOfRange");
                assert!(reason.contains("brightness"));
                assert!(reason.contains("150"));
                assert!(reason.contains("0"));
                assert!(reason.contains("100"));
            }
            _ => panic!("Expected InvalidArgVal variant"),
        }
    }

    #[test]
    fn test_out_of_range_soap_fault() {
        let err = out_of_range("pan_speed", 200.0, -1.0, 1.0);
        let fault = err.to_soap_fault();
        assert!(fault.contains("OutOfRange"));
    }

    // Test hardware_failure

    #[test]
    fn test_hardware_failure_creation() {
        let err = hardware_failure("PTZ movement", "Motor stalled");
        match err {
            OnvifError::HardwareFailure(msg) => {
                assert!(msg.contains("PTZ movement"));
                assert!(msg.contains("Motor stalled"));
            }
            _ => panic!("Expected HardwareFailure variant"),
        }
    }

    #[test]
    fn test_hardware_failure_http_status() {
        let err = hardware_failure("Sensor", "Read failed");
        assert_eq!(
            err.http_status(),
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[test]
    fn test_hardware_failure_soap_fault() {
        let err = hardware_failure("Lens control", "Communication error");
        let fault = err.to_soap_fault();
        assert!(fault.contains("HardwareFailure"));
    }

    // Test all error types have correct HTTP status

    #[test]
    fn test_invalid_token_http_status() {
        let err = invalid_token("ProfileToken", "Bad");
        assert_eq!(err.http_status(), axum::http::StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_no_entity_http_status() {
        let err = no_entity("ter:NotFound", "entity", "token");
        assert_eq!(err.http_status(), axum::http::StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_out_of_range_http_status() {
        let err = out_of_range("param", 100.0, 0.0, 10.0);
        assert_eq!(err.http_status(), axum::http::StatusCode::BAD_REQUEST);
    }

    // Edge cases

    #[test]
    fn test_invalid_token_empty_values() {
        // Should still work with empty values (though not recommended)
        let err = invalid_token("", "");
        assert!(err.to_string().contains("''"));
    }

    #[test]
    fn test_no_entity_special_chars_in_token() {
        let err = no_entity("ter:NotFound", "profile", "token<special>");
        assert!(err.to_string().contains("token<special>"));
    }

    #[test]
    fn test_hardware_failure_empty_operation() {
        let err = hardware_failure("", "detail only");
        assert!(err.to_string().contains("detail only"));
    }

    #[test]
    fn test_out_of_range_negative_values() {
        let err = out_of_range("position", -100.0, -50.0, 50.0);
        assert!(err.to_string().contains("-100"));
    }
}
