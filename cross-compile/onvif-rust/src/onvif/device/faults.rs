//! Device Service specific fault types and mappings.
//!
//! This module defines faults specific to the Device Service operations
//! as defined in the ONVIF Device Management WSDL specification.

#![cfg_attr(not(test), allow(dead_code))]

#[cfg(test)]
use super::validation::validate_scope;
pub use super::validation::{validate_hostname, validate_ipv4};
use crate::onvif::error::OnvifError;

// ============================================================================
// Hostname Faults
// ============================================================================

/// Create an InvalidHostname fault.
///
/// Used when SetHostname is called with an invalid hostname format.
pub fn invalid_hostname(reason: &str) -> OnvifError {
    OnvifError::InvalidArgVal {
        subcode: "InvalidHostname".to_string(),
        reason: format!("Invalid hostname: {}", reason),
    }
}

// ============================================================================
// Network Configuration Faults
// ============================================================================

/// Create an InvalidIPv4Address fault.
pub fn invalid_ipv4_address(reason: &str) -> OnvifError {
    OnvifError::InvalidArgVal {
        subcode: "InvalidIPv4Address".to_string(),
        reason: format!("Invalid IPv4 address: {reason}"),
    }
}

/// Create an UnsupportedNetworkConfiguration fault.
///
/// Used when SetNetworkInterfaces is called with unsupported settings.
pub fn unsupported_network_config(reason: &str) -> OnvifError {
    OnvifError::InvalidArgVal {
        subcode: "InvalidNetworkInterface".to_string(),
        reason: format!("Unsupported network configuration: {}", reason),
    }
}

/// Create a NetworkInterfaceNotFound fault.
///
/// Used when an operation references a non-existent network interface.
pub fn network_interface_not_found(token: &str) -> OnvifError {
    OnvifError::InvalidArgVal {
        subcode: "InvalidNetworkInterface".to_string(),
        reason: format!("Network interface '{}' not found", token),
    }
}

// ============================================================================
// Scope Faults
// ============================================================================

/// Create an InvalidScope fault.
///
/// Used when SetScopes/AddScopes is called with an invalid scope URI.
pub fn invalid_scope(reason: &str) -> OnvifError {
    OnvifError::InvalidArgVal {
        subcode: "InvalidScope".to_string(),
        reason: format!("Invalid scope: {}", reason),
    }
}

/// Create a FixedScope fault.
///
/// Used when trying to remove or modify a fixed scope.
pub fn fixed_scope(scope: &str) -> OnvifError {
    OnvifError::InvalidArgVal {
        subcode: "FixedScope".to_string(),
        reason: format!("Scope '{}' is fixed and cannot be modified", scope),
    }
}

/// Create a ScopeOverwrite fault.
///
/// Used when SetScopes would remove required scopes.
pub fn scope_overwrite(reason: &str) -> OnvifError {
    OnvifError::InvalidArgVal {
        subcode: "ScopeOverwrite".to_string(),
        reason: reason.to_string(),
    }
}

// ============================================================================
// Discovery Mode Faults
// ============================================================================

/// Create an InvalidDiscoveryMode fault.
///
/// Used when SetDiscoveryMode is called with an invalid mode value.
pub fn invalid_discovery_mode(mode: &str) -> OnvifError {
    OnvifError::InvalidArgVal {
        subcode: "InvalidDiscoveryMode".to_string(),
        reason: format!(
            "Invalid discovery mode: '{}'. Must be 'Discoverable' or 'NonDiscoverable'",
            mode
        ),
    }
}

// ============================================================================
// Date/Time Faults
// ============================================================================

/// Create an InvalidDateTime fault.
///
/// Used when SetSystemDateAndTime is called with invalid date/time values.
pub fn invalid_datetime(reason: &str) -> OnvifError {
    OnvifError::InvalidArgVal {
        subcode: "InvalidDateTime".to_string(),
        reason: format!("Invalid date/time: {}", reason),
    }
}

/// Create an InvalidTimeZone fault.
///
/// Used when an invalid timezone is specified.
pub fn invalid_timezone(tz: &str) -> OnvifError {
    OnvifError::InvalidArgVal {
        subcode: "InvalidTimeZone".to_string(),
        reason: format!("Invalid timezone: {}", tz),
    }
}

// ============================================================================
// System Faults
// ============================================================================

/// Create an OperationProhibited fault.
///
/// Used when an operation is not allowed in current state.
pub fn operation_prohibited(reason: &str) -> OnvifError {
    OnvifError::ConfigurationConflict(format!("Operation prohibited: {}", reason))
}

/// Create a RebootFailed fault.
///
/// Used when SystemReboot fails.
pub fn reboot_failed(reason: &str) -> OnvifError {
    OnvifError::HardwareFailure(format!("Reboot failed: {}", reason))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_hostname_valid() {
        assert!(validate_hostname("camera1").is_ok());
        assert!(validate_hostname("my-camera").is_ok());
        assert!(validate_hostname("camera-01").is_ok());
        assert!(validate_hostname("a").is_ok());
        assert!(validate_hostname("abc123").is_ok());
    }

    #[test]
    fn test_validate_hostname_invalid() {
        // Empty
        assert!(validate_hostname("").is_err());

        // Too long
        let long_name = "a".repeat(64);
        assert!(validate_hostname(&long_name).is_err());

        // Invalid characters
        assert!(validate_hostname("camera.local").is_err());
        assert!(validate_hostname("camera_1").is_err());
        assert!(validate_hostname("camera 1").is_err());

        // Leading/trailing hyphen
        assert!(validate_hostname("-camera").is_err());
        assert!(validate_hostname("camera-").is_err());
    }

    #[test]
    fn test_validate_scope_valid() {
        assert!(validate_scope("onvif://www.onvif.org/type/video_encoder").is_ok());
        assert!(validate_scope("onvif://www.onvif.org/name/MyCamera").is_ok());
        assert!(validate_scope("onvif://www.onvif.org/location/room1").is_ok());
    }

    #[test]
    fn test_validate_scope_invalid() {
        // Empty
        assert!(validate_scope("").is_err());

        // Wrong prefix
        assert!(validate_scope("http://example.com/scope").is_err());
        assert!(validate_scope("onvif://other.org/scope").is_err());
    }

    #[test]
    fn test_fault_messages() {
        let err = invalid_hostname("test reason");
        assert!(
            matches!(err, OnvifError::InvalidArgVal { subcode, .. } if subcode == "InvalidHostname")
        );

        let err = invalid_scope("test scope");
        assert!(
            matches!(err, OnvifError::InvalidArgVal { subcode, .. } if subcode == "InvalidScope")
        );

        let err = network_interface_not_found("eth0");
        assert!(
            matches!(err, OnvifError::InvalidArgVal { subcode, .. } if subcode == "InvalidNetworkInterface")
        );
    }

    #[test]
    fn test_unsupported_network_config() {
        let err = unsupported_network_config("IPv6 not supported");
        assert!(
            matches!(err, OnvifError::InvalidArgVal { ref subcode, .. } if subcode == "InvalidNetworkInterface")
        );
        assert!(err.to_string().contains("IPv6 not supported"));
    }

    #[test]
    fn test_network_interface_not_found() {
        let err = network_interface_not_found("eth99");
        assert!(
            matches!(err, OnvifError::InvalidArgVal { ref subcode, .. } if subcode == "InvalidNetworkInterface")
        );
        assert!(err.to_string().contains("eth99"));
    }

    #[test]
    fn test_fixed_scope() {
        let err = fixed_scope("onvif://www.onvif.org/type/video_encoder");
        assert!(
            matches!(err, OnvifError::InvalidArgVal { ref subcode, .. } if subcode == "FixedScope")
        );
        assert!(err.to_string().contains("fixed and cannot be modified"));
    }

    #[test]
    fn test_scope_overwrite() {
        let err = scope_overwrite("Cannot remove required scope");
        assert!(
            matches!(err, OnvifError::InvalidArgVal { ref subcode, .. } if subcode == "ScopeOverwrite")
        );
        assert!(err.to_string().contains("Cannot remove required scope"));
    }

    #[test]
    fn test_invalid_discovery_mode() {
        let err = invalid_discovery_mode("invalid");
        assert!(
            matches!(err, OnvifError::InvalidArgVal { ref subcode, .. } if subcode == "InvalidDiscoveryMode")
        );
        assert!(err.to_string().contains("invalid"));
        assert!(err.to_string().contains("Discoverable"));
    }

    #[test]
    fn test_invalid_datetime() {
        let err = invalid_datetime("year out of range");
        assert!(
            matches!(err, OnvifError::InvalidArgVal { ref subcode, .. } if subcode == "InvalidDateTime")
        );
        assert!(err.to_string().contains("year out of range"));
    }

    #[test]
    fn test_invalid_timezone() {
        let err = invalid_timezone("GMT+99");
        assert!(
            matches!(err, OnvifError::InvalidArgVal { ref subcode, .. } if subcode == "InvalidTimeZone")
        );
        assert!(err.to_string().contains("GMT+99"));
    }

    #[test]
    fn test_operation_prohibited() {
        let err = operation_prohibited("Device is in maintenance mode");
        assert!(matches!(err, OnvifError::ConfigurationConflict(_)));
        assert!(err.to_string().contains("maintenance mode"));
    }

    #[test]
    fn test_reboot_failed() {
        let err = reboot_failed("Hardware timeout");
        assert!(matches!(err, OnvifError::HardwareFailure(_)));
        assert!(err.to_string().contains("Hardware timeout"));
    }

    #[test]
    fn test_validate_hostname_edge_cases() {
        // Exactly 63 characters (max valid)
        let max_valid = "a".repeat(63);
        assert!(validate_hostname(&max_valid).is_ok());

        // 64 characters (too long)
        let too_long = "a".repeat(64);
        assert!(validate_hostname(&too_long).is_err());

        // Single character
        assert!(validate_hostname("a").is_ok());

        // All hyphens in middle
        assert!(validate_hostname("a-b-c").is_ok());

        // Numbers only
        assert!(validate_hostname("123").is_ok());

        // Mixed alphanumeric
        assert!(validate_hostname("camera123").is_ok());
    }

    #[test]
    fn test_validate_scope_edge_cases() {
        // Valid with path
        assert!(validate_scope("onvif://www.onvif.org/type/video_encoder").is_ok());
        assert!(validate_scope("onvif://www.onvif.org/name/MyCamera").is_ok());

        // Invalid: control characters
        let scope_with_control = format!("onvif://www.onvif.org/type/video{}encoder", '\0');
        assert!(validate_scope(&scope_with_control).is_err());

        // Invalid: spaces
        assert!(validate_scope("onvif://www.onvif.org/type/video encoder").is_err());

        // Valid: special URI characters
        assert!(validate_scope("onvif://www.onvif.org/name/Camera%20Name").is_ok());
    }
}
