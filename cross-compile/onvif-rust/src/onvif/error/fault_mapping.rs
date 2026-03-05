//! Fault mapping from ONVIF errors to SOAP fault details.
//!
//! This module provides the mapping from [`OnvifError`] variants to
//! SOAP fault codes, subcodes, and reasons.

use super::OnvifError;

/// Container for SOAP fault detail components.
#[derive(Debug, Clone)]
pub(crate) struct FaultDetails {
    /// The SOAP fault code (e.g., "s:Sender", "s:Receiver").
    pub code: String,
    /// The SOAP fault subcode (e.g., "ter:ActionNotSupported").
    pub subcode: String,
    /// Human-readable reason for the fault.
    pub reason: String,
}

impl FaultDetails {
    /// Create a new FaultDetails instance.
    pub(crate) fn new(
        code: impl Into<String>,
        subcode: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            subcode: subcode.into(),
            reason: reason.into(),
        }
    }

    /// Convert to tuple form for compatibility with build_soap_fault.
    pub(crate) fn as_tuple(&self) -> (&str, &str, &str) {
        (
            self.code.as_str(),
            self.subcode.as_str(),
            self.reason.as_str(),
        )
    }
}

/// Get the SOAP fault details for an ONVIF error.
///
/// This function maps an [`OnvifError`] to its corresponding SOAP fault
/// code, subcode, and reason string.
///
/// # Arguments
///
/// * `error` - The ONVIF error to map
///
/// # Returns
///
/// A [`FaultDetails`] containing the SOAP fault code, subcode, and reason.
pub(crate) fn fault_details(error: &OnvifError) -> FaultDetails {
    match error {
        OnvifError::ActionNotSupported(action) => FaultDetails::new(
            "s:Sender",
            "ter:ActionNotSupported",
            format!("Action '{}' is not supported", action),
        ),
        OnvifError::WellFormed(msg) => FaultDetails::new("s:Sender", "ter:WellFormed", msg.clone()),
        OnvifError::InvalidArgVal { subcode, reason } => FaultDetails::new(
            "s:Sender",
            format!("ter:InvalidArgVal/{}", subcode),
            reason.clone(),
        ),
        OnvifError::HardwareFailure(msg) => {
            FaultDetails::new("s:Receiver", "ter:HardwareFailure", msg.clone())
        }
        OnvifError::NotAuthorized(reason) => FaultDetails::new(
            "s:Sender",
            "ter:NotAuthorized",
            if reason.is_empty() {
                "The action requires authentication".to_string()
            } else {
                reason.clone()
            },
        ),
        OnvifError::MaxUsers => FaultDetails::new(
            "s:Sender",
            "ter:MaxUsers",
            "Maximum number of users has been reached",
        ),
        OnvifError::ConfigurationConflict(msg) => {
            FaultDetails::new("s:Sender", "ter:ConfigurationConflict", msg.clone())
        }
        OnvifError::Internal(msg) => {
            FaultDetails::new("s:Receiver", "ter:InternalError", msg.clone())
        }
        OnvifError::NotFound(msg) => FaultDetails::new("s:Sender", "ter:NotFound", msg.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fault_details_action_not_supported() {
        let error = OnvifError::ActionNotSupported("GetUnknownThing".to_string());
        let details = fault_details(&error);

        assert_eq!(details.code, "s:Sender");
        assert_eq!(details.subcode, "ter:ActionNotSupported");
        assert!(details.reason.contains("GetUnknownThing"));
    }

    #[test]
    fn test_fault_details_well_formed() {
        let error = OnvifError::WellFormed("Missing Body element".to_string());
        let details = fault_details(&error);

        assert_eq!(details.code, "s:Sender");
        assert_eq!(details.subcode, "ter:WellFormed");
        assert_eq!(details.reason, "Missing Body element");
    }

    #[test]
    fn test_fault_details_invalid_arg_val() {
        let error = OnvifError::InvalidArgVal {
            subcode: "OutOfRange".to_string(),
            reason: "Value must be between 0 and 100".to_string(),
        };
        let details = fault_details(&error);

        assert_eq!(details.code, "s:Sender");
        assert!(details.subcode.contains("InvalidArgVal"));
        assert!(details.subcode.contains("OutOfRange"));
        assert_eq!(details.reason, "Value must be between 0 and 100");
    }

    #[test]
    fn test_fault_details_hardware_failure() {
        let error = OnvifError::HardwareFailure("Camera sensor failed".to_string());
        let details = fault_details(&error);

        assert_eq!(details.code, "s:Receiver");
        assert_eq!(details.subcode, "ter:HardwareFailure");
        assert_eq!(details.reason, "Camera sensor failed");
    }

    #[test]
    fn test_fault_details_not_authorized() {
        let error = OnvifError::NotAuthorized("Missing credentials".to_string());
        let details = fault_details(&error);

        assert_eq!(details.code, "s:Sender");
        assert_eq!(details.subcode, "ter:NotAuthorized");
        assert_eq!(details.reason, "Missing credentials");
    }

    #[test]
    fn test_fault_details_not_authorized_empty_reason() {
        let error = OnvifError::NotAuthorized(String::new());
        let details = fault_details(&error);

        assert_eq!(details.code, "s:Sender");
        assert_eq!(details.subcode, "ter:NotAuthorized");
        assert_eq!(details.reason, "The action requires authentication");
    }

    #[test]
    fn test_fault_details_max_users() {
        let error = OnvifError::MaxUsers;
        let details = fault_details(&error);

        assert_eq!(details.code, "s:Sender");
        assert_eq!(details.subcode, "ter:MaxUsers");
        assert_eq!(details.reason, "Maximum number of users has been reached");
    }

    #[test]
    fn test_fault_details_configuration_conflict() {
        let error = OnvifError::ConfigurationConflict("Profile already exists".to_string());
        let details = fault_details(&error);

        assert_eq!(details.code, "s:Sender");
        assert_eq!(details.subcode, "ter:ConfigurationConflict");
        assert_eq!(details.reason, "Profile already exists");
    }

    #[test]
    fn test_fault_details_internal() {
        let error = OnvifError::Internal("Database connection failed".to_string());
        let details = fault_details(&error);

        assert_eq!(details.code, "s:Receiver");
        assert_eq!(details.subcode, "ter:InternalError");
        assert_eq!(details.reason, "Database connection failed");
    }

    #[test]
    fn test_fault_details_not_found() {
        let error = OnvifError::NotFound("Profile 'InvalidToken' not found".to_string());
        let details = fault_details(&error);

        assert_eq!(details.code, "s:Sender");
        assert_eq!(details.subcode, "ter:NotFound");
        assert!(details.reason.contains("InvalidToken"));
    }

    #[test]
    fn test_fault_details_as_tuple() {
        let error = OnvifError::HardwareFailure("Test".to_string());
        let details = fault_details(&error);
        let (code, subcode, reason) = details.as_tuple();

        assert_eq!(code, "s:Receiver");
        assert_eq!(subcode, "ter:HardwareFailure");
        assert_eq!(reason, "Test");
    }
}
