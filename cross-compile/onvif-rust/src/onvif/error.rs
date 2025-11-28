//! ONVIF error types and SOAP fault generation.
//!
//! This module defines the error types used throughout the ONVIF implementation
//! and provides SOAP fault generation for returning proper ONVIF-compliant errors.
//!
//! # Error Codes
//!
//! The module implements the following ONVIF error codes:
//!
//! | Code   | Name                  | Description                          |
//! |--------|-----------------------|--------------------------------------|
//! | EC-001 | ActionNotSupported    | Action not supported by service      |
//! | EC-002 | WellFormed            | Malformed XML or SOAP structure      |
//! | EC-003 | InvalidArgVal         | Invalid argument value               |
//! | EC-005 | HardwareFailure       | Hardware operation failed            |
//! | EC-006 | InvalidArgVal         | Out of range value                   |
//! | EC-011 | NotAuthorized         | Authentication required              |
//! | EC-013 | MaxUsers              | Maximum users exceeded               |
//! | EC-015 | ConfigurationConflict | Configuration conflict               |
//!
//! # Example
//!
//! ```
//! use onvif_rust::onvif::error::OnvifError;
//! use axum::http::StatusCode;
//!
//! let error = OnvifError::ActionNotSupported("GetUnknownThing".to_string());
//! assert_eq!(error.http_status(), StatusCode::BAD_REQUEST);
//!
//! let fault = error.to_soap_fault();
//! assert!(fault.contains("ActionNotSupported"));
//! ```

use axum::http::StatusCode;
use thiserror::Error;

use super::soap::build_soap_fault;

/// Result type for ONVIF operations.
pub type OnvifResult<T> = Result<T, OnvifError>;

/// ONVIF error types with SOAP fault generation.
///
/// Each variant maps to a specific ONVIF error code and generates
/// the appropriate SOAP fault response.
#[derive(Debug, Clone, Error)]
pub enum OnvifError {
    /// EC-001: The requested action is not supported by the service.
    #[error("Action not supported: {0}")]
    ActionNotSupported(String),

    /// EC-002: The request XML is malformed or doesn't follow SOAP structure.
    #[error("Well-formed error: {0}")]
    WellFormed(String),

    /// EC-003/EC-006: Invalid argument value or out of range.
    #[error("Invalid argument value ({subcode}): {reason}")]
    InvalidArgVal {
        /// Specific subcode for the error.
        subcode: String,
        /// Human-readable reason.
        reason: String,
    },

    /// EC-005: Hardware operation failed.
    #[error("Hardware failure: {0}")]
    HardwareFailure(String),

    /// EC-011: Authentication required but not provided or invalid.
    #[error("Not authorized")]
    NotAuthorized,

    /// EC-013: Maximum number of users exceeded.
    #[error("Maximum users exceeded")]
    MaxUsers,

    /// EC-015: Configuration conflict detected.
    #[error("Configuration conflict: {0}")]
    ConfigurationConflict(String),

    /// Internal server error (not a standard ONVIF code).
    #[error("Internal error: {0}")]
    Internal(String),

    /// Resource not found (not a standard ONVIF code).
    #[error("Resource not found: {0}")]
    NotFound(String),
}

impl OnvifError {
    /// Get the appropriate HTTP status code for this error.
    ///
    /// # Returns
    ///
    /// The HTTP status code that should be returned with this error.
    pub fn http_status(&self) -> StatusCode {
        match self {
            OnvifError::ActionNotSupported(_) => StatusCode::BAD_REQUEST,
            OnvifError::WellFormed(_) => StatusCode::BAD_REQUEST,
            OnvifError::InvalidArgVal { .. } => StatusCode::BAD_REQUEST,
            OnvifError::HardwareFailure(_) => StatusCode::INTERNAL_SERVER_ERROR,
            OnvifError::NotAuthorized => StatusCode::UNAUTHORIZED,
            OnvifError::MaxUsers => StatusCode::FORBIDDEN,
            OnvifError::ConfigurationConflict(_) => StatusCode::CONFLICT,
            OnvifError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            OnvifError::NotFound(_) => StatusCode::NOT_FOUND,
        }
    }

    /// Generate a SOAP fault XML for this error.
    ///
    /// # Returns
    ///
    /// A complete SOAP fault envelope as an XML string.
    pub fn to_soap_fault(&self) -> String {
        let (code, subcode, reason) = self.fault_details();
        build_soap_fault(&code, &subcode, &reason)
    }

    /// Get the SOAP fault details (code, subcode, reason).
    fn fault_details(&self) -> (String, String, String) {
        match self {
            OnvifError::ActionNotSupported(action) => (
                "s:Sender".to_string(),
                "ter:ActionNotSupported".to_string(),
                format!("Action '{}' is not supported", action),
            ),
            OnvifError::WellFormed(msg) => (
                "s:Sender".to_string(),
                "ter:WellFormed".to_string(),
                msg.clone(),
            ),
            OnvifError::InvalidArgVal { subcode, reason } => (
                "s:Sender".to_string(),
                format!("ter:InvalidArgVal/{}", subcode),
                reason.clone(),
            ),
            OnvifError::HardwareFailure(msg) => (
                "s:Receiver".to_string(),
                "ter:HardwareFailure".to_string(),
                msg.clone(),
            ),
            OnvifError::NotAuthorized => (
                "s:Sender".to_string(),
                "ter:NotAuthorized".to_string(),
                "The action requires authentication".to_string(),
            ),
            OnvifError::MaxUsers => (
                "s:Sender".to_string(),
                "ter:MaxUsers".to_string(),
                "Maximum number of users has been reached".to_string(),
            ),
            OnvifError::ConfigurationConflict(msg) => (
                "s:Sender".to_string(),
                "ter:ConfigurationConflict".to_string(),
                msg.clone(),
            ),
            OnvifError::Internal(msg) => (
                "s:Receiver".to_string(),
                "ter:InternalError".to_string(),
                msg.clone(),
            ),
            OnvifError::NotFound(msg) => (
                "s:Sender".to_string(),
                "ter:NotFound".to_string(),
                msg.clone(),
            ),
        }
    }

    /// Create an InvalidArgVal error with a specific subcode.
    ///
    /// # Arguments
    ///
    /// * `subcode` - The specific error subcode
    /// * `reason` - Human-readable reason
    ///
    /// # Example
    ///
    /// ```
    /// use onvif_rust::onvif::error::OnvifError;
    ///
    /// let error = OnvifError::invalid_arg("OutOfRange", "Value must be between 0 and 100");
    /// ```
    pub fn invalid_arg(subcode: impl Into<String>, reason: impl Into<String>) -> Self {
        OnvifError::InvalidArgVal {
            subcode: subcode.into(),
            reason: reason.into(),
        }
    }

    /// Create an error for missing required argument.
    pub fn missing_arg(name: &str) -> Self {
        OnvifError::InvalidArgVal {
            subcode: "MissingArg".to_string(),
            reason: format!("Required argument '{}' is missing", name),
        }
    }

    /// Create an error for out-of-range value.
    pub fn out_of_range(name: &str, min: impl std::fmt::Display, max: impl std::fmt::Display) -> Self {
        OnvifError::InvalidArgVal {
            subcode: "OutOfRange".to_string(),
            reason: format!("'{}' must be between {} and {}", name, min, max),
        }
    }
}

/// Generate a complete SOAP fault envelope XML.
///
/// This is a convenience function for generating fault responses
/// without creating an `OnvifError` first.
///
/// # Arguments
///
/// * `code` - The SOAP fault code (e.g., "s:Sender", "s:Receiver")
/// * `subcode` - The fault subcode (e.g., "ter:ActionNotSupported")
/// * `reason` - Human-readable reason for the fault
///
/// # Returns
///
/// A complete SOAP fault envelope as an XML string.
///
/// # Example
///
/// ```
/// use onvif_rust::onvif::error::soap_fault;
///
/// let fault = soap_fault("s:Sender", "ter:InvalidToken", "Invalid security token");
/// assert!(fault.contains("InvalidToken"));
/// ```
pub fn soap_fault(code: &str, subcode: &str, reason: &str) -> String {
    build_soap_fault(code, subcode, reason)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_not_supported() {
        let error = OnvifError::ActionNotSupported("GetUnknownThing".to_string());

        assert_eq!(error.http_status(), StatusCode::BAD_REQUEST);

        let fault = error.to_soap_fault();
        assert!(fault.contains("ActionNotSupported"));
        assert!(fault.contains("GetUnknownThing"));
    }

    #[test]
    fn test_well_formed_error() {
        let error = OnvifError::WellFormed("Missing Body element".to_string());

        assert_eq!(error.http_status(), StatusCode::BAD_REQUEST);

        let fault = error.to_soap_fault();
        assert!(fault.contains("WellFormed"));
        assert!(fault.contains("Missing Body element"));
    }

    #[test]
    fn test_invalid_arg_val() {
        let error = OnvifError::InvalidArgVal {
            subcode: "OutOfRange".to_string(),
            reason: "Value must be between 0 and 100".to_string(),
        };

        assert_eq!(error.http_status(), StatusCode::BAD_REQUEST);

        let fault = error.to_soap_fault();
        assert!(fault.contains("InvalidArgVal"));
        assert!(fault.contains("OutOfRange"));
    }

    #[test]
    fn test_hardware_failure() {
        let error = OnvifError::HardwareFailure("Camera sensor failed".to_string());

        assert_eq!(error.http_status(), StatusCode::INTERNAL_SERVER_ERROR);

        let fault = error.to_soap_fault();
        assert!(fault.contains("HardwareFailure"));
        assert!(fault.contains("Camera sensor failed"));
    }

    #[test]
    fn test_not_authorized() {
        let error = OnvifError::NotAuthorized;

        assert_eq!(error.http_status(), StatusCode::UNAUTHORIZED);

        let fault = error.to_soap_fault();
        assert!(fault.contains("NotAuthorized"));
    }

    #[test]
    fn test_max_users() {
        let error = OnvifError::MaxUsers;

        assert_eq!(error.http_status(), StatusCode::FORBIDDEN);

        let fault = error.to_soap_fault();
        assert!(fault.contains("MaxUsers"));
    }

    #[test]
    fn test_configuration_conflict() {
        let error = OnvifError::ConfigurationConflict("Profile already exists".to_string());

        assert_eq!(error.http_status(), StatusCode::CONFLICT);

        let fault = error.to_soap_fault();
        assert!(fault.contains("ConfigurationConflict"));
        assert!(fault.contains("Profile already exists"));
    }

    #[test]
    fn test_invalid_arg_helper() {
        let error = OnvifError::invalid_arg("InvalidFormat", "Expected YYYY-MM-DD");

        match error {
            OnvifError::InvalidArgVal { subcode, reason } => {
                assert_eq!(subcode, "InvalidFormat");
                assert_eq!(reason, "Expected YYYY-MM-DD");
            }
            _ => panic!("Expected InvalidArgVal variant"),
        }
    }

    #[test]
    fn test_missing_arg_helper() {
        let error = OnvifError::missing_arg("ProfileToken");

        match error {
            OnvifError::InvalidArgVal { subcode, reason } => {
                assert_eq!(subcode, "MissingArg");
                assert!(reason.contains("ProfileToken"));
            }
            _ => panic!("Expected InvalidArgVal variant"),
        }
    }

    #[test]
    fn test_out_of_range_helper() {
        let error = OnvifError::out_of_range("brightness", 0, 100);

        match error {
            OnvifError::InvalidArgVal { subcode, reason } => {
                assert_eq!(subcode, "OutOfRange");
                assert!(reason.contains("brightness"));
                assert!(reason.contains("0"));
                assert!(reason.contains("100"));
            }
            _ => panic!("Expected InvalidArgVal variant"),
        }
    }

    #[test]
    fn test_soap_fault_helper() {
        let fault = soap_fault("s:Sender", "ter:CustomError", "Custom error message");

        assert!(fault.contains("Envelope"));
        assert!(fault.contains("Fault"));
        assert!(fault.contains("s:Sender"));
        assert!(fault.contains("ter:CustomError"));
        assert!(fault.contains("Custom error message"));
    }

    #[test]
    fn test_error_display() {
        let error = OnvifError::ActionNotSupported("GetTest".to_string());
        let display = format!("{}", error);
        assert!(display.contains("GetTest"));

        let error = OnvifError::NotAuthorized;
        let display = format!("{}", error);
        assert!(display.contains("authorized"));
    }

    #[test]
    fn test_internal_error_variant() {
        let error = OnvifError::Internal("Database connection failed".to_string());

        // Test HTTP status code
        assert_eq!(error.http_status(), StatusCode::INTERNAL_SERVER_ERROR);

        // Test display
        let display = format!("{}", error);
        assert!(display.contains("Internal error"));
        assert!(display.contains("Database connection failed"));

        // Test SOAP fault generation
        let fault = error.to_soap_fault();
        assert!(fault.contains("s:Receiver"));
        assert!(fault.contains("ter:InternalError"));
        assert!(fault.contains("Database connection failed"));
    }

    #[test]
    fn test_not_found_error_variant() {
        let error = OnvifError::NotFound("Profile 'InvalidToken' not found".to_string());

        // Test HTTP status code
        assert_eq!(error.http_status(), StatusCode::NOT_FOUND);

        // Test display
        let display = format!("{}", error);
        assert!(display.contains("not found"));
        assert!(display.contains("InvalidToken"));

        // Test SOAP fault generation
        let fault = error.to_soap_fault();
        assert!(fault.contains("s:Sender"));
        assert!(fault.contains("ter:NotFound"));
        assert!(fault.contains("InvalidToken"));
    }

    #[test]
    fn test_hardware_failure_error_variant() {
        let error = OnvifError::HardwareFailure("PTZ motor stuck".to_string());

        // Test HTTP status code
        assert_eq!(error.http_status(), StatusCode::INTERNAL_SERVER_ERROR);

        // Test display
        let display = format!("{}", error);
        assert!(display.contains("Hardware failure"));
        assert!(display.contains("PTZ motor stuck"));

        // Test SOAP fault generation
        let fault = error.to_soap_fault();
        assert!(fault.contains("s:Receiver"));
        assert!(fault.contains("ter:HardwareFailure"));
        assert!(fault.contains("PTZ motor stuck"));
    }

    #[test]
    fn test_configuration_conflict_error_variant() {
        let error = OnvifError::ConfigurationConflict("Profile in use".to_string());

        // Test HTTP status code
        assert_eq!(error.http_status(), StatusCode::CONFLICT);

        // Test display
        let display = format!("{}", error);
        assert!(display.contains("Configuration conflict"));
        assert!(display.contains("Profile in use"));

        // Test SOAP fault generation
        let fault = error.to_soap_fault();
        assert!(fault.contains("s:Sender"));
        assert!(fault.contains("ter:ConfigurationConflict"));
        assert!(fault.contains("Profile in use"));
    }

    #[test]
    fn test_max_users_error_variant() {
        let error = OnvifError::MaxUsers;

        // Test HTTP status code
        assert_eq!(error.http_status(), StatusCode::FORBIDDEN);

        // Test display
        let display = format!("{}", error);
        assert!(display.contains("Maximum"));
        assert!(display.contains("users"));

        // Test SOAP fault generation
        let fault = error.to_soap_fault();
        assert!(fault.contains("s:Sender"));
        assert!(fault.contains("ter:MaxUsers"));
        assert!(fault.contains("Maximum number of users"));
    }
}
