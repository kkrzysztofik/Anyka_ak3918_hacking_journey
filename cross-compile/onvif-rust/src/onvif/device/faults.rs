//! Device Service specific fault types and mappings.
//!
//! This module defines faults specific to the Device Service operations
//! as defined in the ONVIF Device Management WSDL specification.

use crate::onvif::error::OnvifError;

// ============================================================================
// Hostname Faults
// ============================================================================

/// Create an InvalidHostname fault.
///
/// Used when SetHostname is called with an invalid hostname format.
pub fn invalid_hostname(reason: &str) -> OnvifError {
    OnvifError::InvalidArgVal {
        subcode: "ter:InvalidHostname".to_string(),
        reason: format!("Invalid hostname: {}", reason),
    }
}

/// Validate a hostname according to RFC 1123.
///
/// Returns Ok(()) if valid, or an OnvifError if invalid.
pub fn validate_hostname(name: &str) -> Result<(), OnvifError> {
    // Check length
    if name.is_empty() {
        return Err(invalid_hostname("hostname cannot be empty"));
    }
    if name.len() > 63 {
        return Err(invalid_hostname("hostname too long (max 63 characters)"));
    }

    // Check characters (alphanumeric and hyphens only, no leading/trailing hyphens)
    let chars: Vec<char> = name.chars().collect();

    if chars[0] == '-' || chars[chars.len() - 1] == '-' {
        return Err(invalid_hostname(
            "hostname cannot start or end with a hyphen",
        ));
    }

    for c in &chars {
        if !c.is_ascii_alphanumeric() && *c != '-' {
            return Err(invalid_hostname(&format!(
                "hostname contains invalid character: '{}'",
                c
            )));
        }
    }

    // Must start with alphanumeric
    if !chars[0].is_ascii_alphanumeric() {
        return Err(invalid_hostname(
            "hostname must start with a letter or digit",
        ));
    }

    Ok(())
}

// ============================================================================
// Network Configuration Faults
// ============================================================================

/// Create an UnsupportedNetworkConfiguration fault.
///
/// Used when SetNetworkInterfaces is called with unsupported settings.
pub fn unsupported_network_config(reason: &str) -> OnvifError {
    OnvifError::InvalidArgVal {
        subcode: "ter:InvalidNetworkInterface".to_string(),
        reason: format!("Unsupported network configuration: {}", reason),
    }
}

/// Create a NetworkInterfaceNotFound fault.
///
/// Used when an operation references a non-existent network interface.
pub fn network_interface_not_found(token: &str) -> OnvifError {
    OnvifError::InvalidArgVal {
        subcode: "ter:InvalidNetworkInterface".to_string(),
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
        subcode: "ter:InvalidScope".to_string(),
        reason: format!("Invalid scope: {}", reason),
    }
}

/// Create a FixedScope fault.
///
/// Used when trying to remove or modify a fixed scope.
pub fn fixed_scope(scope: &str) -> OnvifError {
    OnvifError::InvalidArgVal {
        subcode: "ter:FixedScope".to_string(),
        reason: format!("Scope '{}' is fixed and cannot be modified", scope),
    }
}

/// Create a ScopeOverwrite fault.
///
/// Used when SetScopes would remove required scopes.
pub fn scope_overwrite(reason: &str) -> OnvifError {
    OnvifError::InvalidArgVal {
        subcode: "ter:ScopeOverwrite".to_string(),
        reason: reason.to_string(),
    }
}

/// Validate a scope URI.
///
/// Scopes must be valid URIs starting with "onvif://www.onvif.org/".
pub fn validate_scope(scope: &str) -> Result<(), OnvifError> {
    if scope.is_empty() {
        return Err(invalid_scope("scope cannot be empty"));
    }

    // Scopes should be valid URIs
    if !scope.starts_with("onvif://www.onvif.org/") {
        return Err(invalid_scope(
            "scope must start with 'onvif://www.onvif.org/'",
        ));
    }

    // Check for invalid characters (basic URI validation)
    for c in scope.chars() {
        if c.is_control() || c == ' ' {
            return Err(invalid_scope("scope contains invalid characters"));
        }
    }

    Ok(())
}

// ============================================================================
// Discovery Mode Faults
// ============================================================================

/// Create an InvalidDiscoveryMode fault.
///
/// Used when SetDiscoveryMode is called with an invalid mode value.
pub fn invalid_discovery_mode(mode: &str) -> OnvifError {
    OnvifError::InvalidArgVal {
        subcode: "ter:InvalidDiscoveryMode".to_string(),
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
        subcode: "ter:InvalidDateTime".to_string(),
        reason: format!("Invalid date/time: {}", reason),
    }
}

/// Create an InvalidTimeZone fault.
///
/// Used when an invalid timezone is specified.
pub fn invalid_timezone(tz: &str) -> OnvifError {
    OnvifError::InvalidArgVal {
        subcode: "ter:InvalidTimeZone".to_string(),
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
            matches!(err, OnvifError::InvalidArgVal { subcode, .. } if subcode == "ter:InvalidHostname")
        );

        let err = invalid_scope("test scope");
        assert!(
            matches!(err, OnvifError::InvalidArgVal { subcode, .. } if subcode == "ter:InvalidScope")
        );

        let err = network_interface_not_found("eth0");
        assert!(
            matches!(err, OnvifError::InvalidArgVal { subcode, .. } if subcode == "ter:InvalidNetworkInterface")
        );
    }
}
