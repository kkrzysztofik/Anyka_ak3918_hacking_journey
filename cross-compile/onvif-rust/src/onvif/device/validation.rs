//! Device Service validation functions.
//!
//! This module contains input validation functions for Device Service operations.
//! These functions are extracted from faults.rs to separate validation logic
//! from fault construction.

use crate::onvif::common::limits::MAX_SCOPE_URI_CHARS;
use crate::onvif::error::OnvifResult;

/// Validate a hostname according to RFC 1123.
///
/// Returns Ok(()) if valid, or an OnvifError if invalid.
pub fn validate_hostname(name: &str) -> OnvifResult<()> {
    use crate::onvif::device::faults::invalid_hostname;

    // Check length
    if name.is_empty() {
        return Err(invalid_hostname("hostname cannot be empty"));
    }
    if name.len() > 63 {
        return Err(invalid_hostname("hostname too long (max 63 characters)"));
    }

    // Check characters (alphanumeric and hyphens only, no leading/trailing hyphens)
    if name.starts_with('-') || name.ends_with('-') {
        return Err(invalid_hostname(
            "hostname cannot start or end with a hyphen",
        ));
    }

    for c in name.chars() {
        if !c.is_ascii_alphanumeric() && c != '-' {
            return Err(invalid_hostname(&format!(
                "hostname contains invalid character: '{}'",
                c
            )));
        }
    }

    // Must start with alphanumeric
    if !name.starts_with(|c: char| c.is_ascii_alphanumeric()) {
        return Err(invalid_hostname(
            "hostname must start with a letter or digit",
        ));
    }

    Ok(())
}

/// Validate a scope URI.
///
/// Scopes must be valid URIs starting with "onvif://www.onvif.org/".
pub fn validate_scope(scope: &str) -> OnvifResult<()> {
    use crate::onvif::device::faults::invalid_scope;

    if scope.is_empty() {
        return Err(invalid_scope("scope cannot be empty"));
    }

    if scope.len() > MAX_SCOPE_URI_CHARS {
        return Err(invalid_scope(&format!(
            "scope exceeds maximum length of {} characters",
            MAX_SCOPE_URI_CHARS
        )));
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

    #[test]
    fn test_validate_scope_too_long() {
        let prefix = "onvif://www.onvif.org/name/";
        let filler = "x".repeat(MAX_SCOPE_URI_CHARS - prefix.len() + 1);
        let long_scope = format!("{prefix}{filler}");
        assert!(long_scope.len() > MAX_SCOPE_URI_CHARS);
        assert!(validate_scope(&long_scope).is_err());
    }
}
