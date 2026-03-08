//! Shared validation utilities for ONVIF services.
//!
//! This module provides common validation functions used across multiple
//! ONVIF services (Device, Media, PTZ, Imaging) to ensure consistent
//! input validation and error handling.
//!
//! # Usage
//!
//! ```
//! use onvif_rust::onvif::common::validation::{
//!     validate_reference_token, validate_range_f32, validate_range_i32, validate_max_length,
//! };
//!
//! // Validate a profile token
//! validate_reference_token("ProfileToken123", "profile")?;
//!
//! // Validate a float range (e.g., brightness 0-100)
//! validate_range_f32(50.0, 0.0, 100.0, "brightness")?;
//!
//! // Validate an integer range (e.g., PTZ speed 1-10)
//! validate_range_i32(5, 1, 10, "speed")?;
//!
//! // Validate string length
//! validate_max_length("some_string", 64, "name")?;
//! ```

use crate::onvif::error::{OnvifError, OnvifResult};

/// Validate a reference token (used for profiles, tokens, etc.).
///
/// Reference tokens in ONVIF are typically alphanumeric strings that
/// reference entities like profiles, configurations, or presets.
///
/// # Arguments
///
/// * `token` - The token string to validate
/// * `kind` - The kind of token for error messages (e.g., "profile", "PTZConfiguration")
///
/// # Returns
///
/// `Ok(())` if valid, or an appropriate `OnvifError::InvalidArgVal` if invalid/empty.
///
/// # Errors
///
/// Returns an error if:
/// - The token is empty
/// - The token contains invalid characters (whitespace, special chars)
///
/// # Example
///
/// ```
/// use onvif_rust::onvif::common::validation::validate_reference_token;
///
/// // Valid tokens
/// assert!(validate_reference_token("Profile_001", "profile").is_ok());
/// assert!(validate_reference_token("abc123", "profile").is_ok());
///
/// // Invalid tokens
/// assert!(validate_reference_token("", "profile").is_err());
/// assert!(validate_reference_token("   ", "profile").is_err());
/// assert!(validate_reference_token("profile with spaces", "profile").is_err());
/// ```
pub fn validate_reference_token(token: &str, kind: &str) -> OnvifResult<()> {
    if token.is_empty() {
        return Err(OnvifError::missing_arg(kind));
    }

    // Check for whitespace which is invalid in ONVIF tokens
    if token.chars().any(|c| c.is_whitespace()) {
        return Err(OnvifError::invalid_arg(
            "InvalidToken",
            format!("{} token '{}' contains invalid whitespace", kind, token),
        ));
    }

    // Check for other invalid characters
    let invalid_chars = ['<', '>', '&', '\'', '"'];
    if token.chars().any(|c| invalid_chars.contains(&c)) {
        return Err(OnvifError::invalid_arg(
            "InvalidToken",
            format!("{} token '{}' contains invalid characters", kind, token),
        ));
    }

    Ok(())
}

/// Validate a 32-bit floating point value is within a specified range.
///
/// # Arguments
///
/// * `value` - The value to validate
/// * `min` - Minimum allowed value (inclusive)
/// * `max` - Maximum allowed value (inclusive)
/// * `param` - Parameter name for error messages
///
/// # Returns
///
/// `Ok(())` if valid, or `OnvifError::InvalidArgVal` if out of range.
///
/// # Example
///
/// ```
/// use onvif_rust::onvif::common::validation::validate_range_f32;
///
/// // Valid range
/// assert!(validate_range_f32(50.0, 0.0, 100.0, "brightness").is_ok());
/// assert!(validate_range_f32(0.0, 0.0, 100.0, "brightness").is_ok());
/// assert!(validate_range_f32(100.0, 0.0, 100.0, "brightness").is_ok());
///
/// // Invalid ranges
/// assert!(validate_range_f32(-1.0, 0.0, 100.0, "brightness").is_err());
/// assert!(validate_range_f32(101.0, 0.0, 100.0, "brightness").is_err());
/// ```
pub fn validate_range_f32(value: f32, min: f32, max: f32, param: &str) -> OnvifResult<()> {
    if !(min..=max).contains(&value) {
        return Err(OnvifError::out_of_range(param, min, max));
    }
    Ok(())
}

/// Validate a 32-bit integer value is within a specified range.
///
/// # Arguments
///
/// * `value` - The value to validate
/// * `min` - Minimum allowed value (inclusive)
/// * `max` - Maximum allowed value (inclusive)
/// * `param` - Parameter name for error messages
///
/// # Returns
///
/// `Ok(())` if valid, or `OnvifError::InvalidArgVal` if out of range.
///
/// # Example
///
/// ```
/// use onvif_rust::onvif::common::validation::validate_range_i32;
///
/// // Valid range
/// assert!(validate_range_i32(5, 1, 10, "speed").is_ok());
/// assert!(validate_range_i32(1, 1, 10, "speed").is_ok());
/// assert!(validate_range_i32(10, 1, 10, "speed").is_ok());
///
/// // Invalid ranges
/// assert!(validate_range_i32(0, 1, 10, "speed").is_err());
/// assert!(validate_range_i32(11, 1, 10, "speed").is_err());
/// ```
pub fn validate_range_i32(value: i32, min: i32, max: i32, param: &str) -> OnvifResult<()> {
    if !(min..=max).contains(&value) {
        return Err(OnvifError::out_of_range(param, min, max));
    }
    Ok(())
}

/// Validate a string does not exceed maximum length.
///
/// # Arguments
///
/// * `value` - The string to validate
/// * `max` - Maximum allowed length
/// * `param` - Parameter name for error messages
///
/// # Returns
///
/// `Ok(())` if valid, or `OnvifError::InvalidArgVal` if too long.
///
/// # Example
///
/// ```
/// use onvif_rust::onvif::common::validation::validate_max_length;
///
/// // Valid lengths
/// assert!(validate_max_length("short", 64, "name").is_ok());
/// assert!(validate_max_length("", 64, "name").is_ok());
/// assert!(validate_max_length("a".repeat(64).as_str(), 64, "name").is_ok());
///
/// // Invalid lengths
/// assert!(validate_max_length("a".repeat(65).as_str(), 64, "name").is_err());
/// ```
pub fn validate_max_length(value: &str, max: usize, param: &str) -> OnvifResult<()> {
    // NOTE: Uses byte length (.len()), not character count (.chars().count()).
    // ONVIF tokens are ASCII-only (xs:token), so byte length == character count.
    if value.len() > max {
        return Err(OnvifError::invalid_arg(
            "TooLong",
            format!(
                "'{}' exceeds maximum length of {} characters (got {})",
                param,
                max,
                value.len()
            ),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test validate_reference_token

    #[test]
    fn test_validate_reference_token_valid() {
        assert!(validate_reference_token("Profile001", "profile").is_ok());
        assert!(validate_reference_token("token_123", "token").is_ok());
        assert!(validate_reference_token("a", "profile").is_ok());
    }

    #[test]
    fn test_validate_reference_token_empty() {
        let result = validate_reference_token("", "profile");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("MissingArg"));
        assert!(err.to_string().contains("profile"));
    }

    #[test]
    fn test_validate_reference_token_whitespace() {
        let result = validate_reference_token("token with space", "profile");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("InvalidToken"));
        assert!(err.to_string().contains("whitespace"));
    }

    #[test]
    fn test_validate_reference_token_invalid_chars() {
        let result = validate_reference_token("token<tag>", "profile");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("InvalidToken"));
        assert!(err.to_string().contains("invalid characters"));
    }

    // Test validate_range_f32

    #[test]
    fn test_validate_range_f32_valid() {
        assert!(validate_range_f32(50.0, 0.0, 100.0, "brightness").is_ok());
        assert!(validate_range_f32(0.0, 0.0, 100.0, "brightness").is_ok());
        assert!(validate_range_f32(100.0, 0.0, 100.0, "brightness").is_ok());
    }

    #[test]
    fn test_validate_range_f32_below_min() {
        let result = validate_range_f32(-1.0, 0.0, 100.0, "brightness");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("brightness"));
        assert!(err.to_string().contains("0"));
        assert!(err.to_string().contains("100"));
    }

    #[test]
    fn test_validate_range_f32_above_max() {
        let result = validate_range_f32(101.0, 0.0, 100.0, "brightness");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("brightness"));
    }

    // Test validate_range_i32

    #[test]
    fn test_validate_range_i32_valid() {
        assert!(validate_range_i32(5, 1, 10, "speed").is_ok());
        assert!(validate_range_i32(1, 1, 10, "speed").is_ok());
        assert!(validate_range_i32(10, 1, 10, "speed").is_ok());
    }

    #[test]
    fn test_validate_range_i32_below_min() {
        let result = validate_range_i32(0, 1, 10, "speed");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_range_i32_above_max() {
        let result = validate_range_i32(11, 1, 10, "speed");
        assert!(result.is_err());
    }

    // Test validate_max_length

    #[test]
    fn test_validate_max_length_valid() {
        assert!(validate_max_length("short", 64, "name").is_ok());
        assert!(validate_max_length("", 64, "name").is_ok());
        assert!(validate_max_length("a".repeat(64).as_str(), 64, "name").is_ok());
    }

    #[test]
    fn test_validate_max_length_too_long() {
        let result = validate_max_length("a".repeat(65).as_str(), 64, "name");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("TooLong"));
        assert!(err.to_string().contains("64"));
        assert!(err.to_string().contains("65"));
    }

    #[test]
    fn test_validate_max_length_exact() {
        // Should pass - exact match
        assert!(validate_max_length("exact", 5, "name").is_ok());
    }

    // Test error messages contain correct parameter names

    #[test]
    fn test_validate_range_f32_param_in_error() {
        let result = validate_range_f32(200.0, 0.0, 100.0, "pan_speed");
        let err = result.unwrap_err();
        assert!(err.to_string().contains("pan_speed"));
    }

    #[test]
    fn test_validate_range_i32_param_in_error() {
        let result = validate_range_i32(50, 0, 10, "tilt_speed");
        let err = result.unwrap_err();
        assert!(err.to_string().contains("tilt_speed"));
    }
}
