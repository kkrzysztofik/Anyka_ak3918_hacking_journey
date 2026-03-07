//! Media Service validation functions.
//!
//! This module provides validation helpers for Media Service operations.
//! These functions validate input parameters according to ONVIF specifications.

use crate::onvif::error::OnvifError;
use crate::onvif::types::common::ReferenceToken;

/// Maximum length for ONVIF reference tokens (profile, config, source).
const MAX_TOKEN_LENGTH: usize = 64;

/// Maximum supported resolution dimension (pixels).
const MAX_RESOLUTION: i32 = 4096;

/// Maximum supported frame rate (fps).
const MAX_FRAME_RATE: i32 = 120;

/// Maximum supported bitrate (bits per second, 50 Mbps).
const MAX_BITRATE: i32 = 50_000_000;

/// Maximum length for profile names.
const MAX_PROFILE_NAME_LENGTH: usize = 64;

/// Validate a profile token.
///
/// Returns `OnvifError::NoProfile` if the token is empty.
pub fn validate_profile_token(token: &ReferenceToken) -> Result<(), OnvifError> {
    if token.is_empty() {
        return Err(OnvifError::invalid_arg_val(
            "ter:NoProfile",
            "Profile token is empty",
        ));
    }
    if token.len() > MAX_TOKEN_LENGTH {
        return Err(OnvifError::invalid_arg_val(
            "ter:InvalidToken",
            "Profile token exceeds maximum length of 64 characters",
        ));
    }
    Ok(())
}

/// Validate a configuration token.
///
/// Returns `OnvifError::NoConfig` if the token is empty.
pub fn validate_config_token(token: &ReferenceToken) -> Result<(), OnvifError> {
    if token.is_empty() {
        return Err(OnvifError::invalid_arg_val(
            "ter:NoConfig",
            "Configuration token is empty",
        ));
    }
    if token.len() > MAX_TOKEN_LENGTH {
        return Err(OnvifError::invalid_arg_val(
            "ter:InvalidToken",
            "Configuration token exceeds maximum length of 64 characters",
        ));
    }
    Ok(())
}

/// Validate a source token (video or audio).
///
/// Returns `OnvifError::NoSource` if the token is empty.
pub fn validate_source_token(token: &ReferenceToken) -> Result<(), OnvifError> {
    if token.is_empty() {
        return Err(OnvifError::invalid_arg_val(
            "ter:NoSource",
            "Source token is empty",
        ));
    }
    if token.len() > MAX_TOKEN_LENGTH {
        return Err(OnvifError::invalid_arg_val(
            "ter:InvalidToken",
            "Source token exceeds maximum length of 64 characters",
        ));
    }
    Ok(())
}

/// Validate video resolution.
///
/// Returns error if resolution is invalid.
pub fn validate_resolution(width: i32, height: i32) -> Result<(), OnvifError> {
    if width <= 0 || height <= 0 {
        return Err(OnvifError::invalid_arg_val(
            "ter:InvalidResolution",
            "Resolution width and height must be positive",
        ));
    }
    if width > MAX_RESOLUTION || height > MAX_RESOLUTION {
        return Err(OnvifError::invalid_arg_val(
            "ter:InvalidResolution",
            "Resolution exceeds maximum supported (4096x4096)",
        ));
    }
    Ok(())
}

/// Validate frame rate.
///
/// Returns error if frame rate is out of range.
pub fn validate_frame_rate(frame_rate: i32) -> Result<(), OnvifError> {
    if frame_rate <= 0 {
        return Err(OnvifError::invalid_arg_val(
            "ter:InvalidFrameRate",
            "Frame rate must be positive",
        ));
    }
    if frame_rate > MAX_FRAME_RATE {
        return Err(OnvifError::invalid_arg_val(
            "ter:InvalidFrameRate",
            "Frame rate exceeds maximum supported (120 fps)",
        ));
    }
    Ok(())
}

/// Validate bitrate.
///
/// Returns error if bitrate is out of range.
pub fn validate_bitrate(bitrate: i32) -> Result<(), OnvifError> {
    if bitrate <= 0 {
        return Err(OnvifError::invalid_arg_val(
            "ter:InvalidBitrate",
            "Bitrate must be positive",
        ));
    }
    if bitrate > MAX_BITRATE {
        return Err(OnvifError::invalid_arg_val(
            "ter:InvalidBitrate",
            "Bitrate exceeds maximum supported (50 Mbps)",
        ));
    }
    Ok(())
}

/// Validate quality setting.
///
/// Returns error if quality is out of range (0.0 to 1.0).
pub fn validate_quality(quality: f32) -> Result<(), OnvifError> {
    if !(0.0..=1.0).contains(&quality) {
        return Err(OnvifError::invalid_arg_val(
            "ter:InvalidQuality",
            "Quality must be between 0.0 and 1.0",
        ));
    }
    Ok(())
}

/// Validate profile name.
///
/// Returns error if name is empty or too long.
pub fn validate_profile_name(name: &str) -> Result<(), OnvifError> {
    if name.is_empty() {
        return Err(OnvifError::invalid_arg_val(
            "ter:InvalidName",
            "Profile name is empty",
        ));
    }
    if name.len() > MAX_PROFILE_NAME_LENGTH {
        return Err(OnvifError::invalid_arg_val(
            "ter:InvalidName",
            "Profile name exceeds maximum length of 64 characters",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_profile_token_empty() {
        let result = validate_profile_token(&String::new());
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_profile_token_valid() {
        let result = validate_profile_token(&"Profile_1".to_string());
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_profile_token_too_long() {
        let long_token = "x".repeat(65);
        let result = validate_profile_token(&long_token);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_resolution_valid() {
        assert!(validate_resolution(1920, 1080).is_ok());
        assert!(validate_resolution(640, 480).is_ok());
    }

    #[test]
    fn test_validate_resolution_invalid() {
        assert!(validate_resolution(0, 1080).is_err());
        assert!(validate_resolution(1920, 0).is_err());
        assert!(validate_resolution(-1, 1080).is_err());
        assert!(validate_resolution(5000, 5000).is_err());
    }

    #[test]
    fn test_validate_frame_rate() {
        assert!(validate_frame_rate(30).is_ok());
        assert!(validate_frame_rate(0).is_err());
        assert!(validate_frame_rate(150).is_err());
    }

    #[test]
    fn test_validate_quality() {
        assert!(validate_quality(0.5).is_ok());
        assert!(validate_quality(0.0).is_ok());
        assert!(validate_quality(1.0).is_ok());
        assert!(validate_quality(-0.1).is_err());
        assert!(validate_quality(1.1).is_err());
    }

    #[test]
    fn test_validate_config_token() {
        assert!(validate_config_token(&"Config_1".to_string()).is_ok());
        assert!(validate_config_token(&String::new()).is_err());
        let long_token = "x".repeat(65);
        assert!(validate_config_token(&long_token).is_err());
        // Exactly 64 characters should be valid
        let max_token = "x".repeat(64);
        assert!(validate_config_token(&max_token).is_ok());
    }

    #[test]
    fn test_validate_source_token() {
        assert!(validate_source_token(&"Source_1".to_string()).is_ok());
        assert!(validate_source_token(&String::new()).is_err());
        let long_token = "x".repeat(65);
        assert!(validate_source_token(&long_token).is_err());
        // Exactly 64 characters should be valid
        let max_token = "x".repeat(64);
        assert!(validate_source_token(&max_token).is_ok());
    }

    #[test]
    fn test_validate_bitrate() {
        assert!(validate_bitrate(1000000).is_ok());
        assert!(validate_bitrate(50000000).is_ok()); // Max
        assert!(validate_bitrate(0).is_err());
        assert!(validate_bitrate(-1).is_err());
        assert!(validate_bitrate(50000001).is_err()); // Over max
    }

    #[test]
    fn test_validate_profile_name() {
        assert!(validate_profile_name("MyProfile").is_ok());
        assert!(validate_profile_name(&"x".repeat(64)).is_ok()); // Max length
        assert!(validate_profile_name("").is_err());
        assert!(validate_profile_name(&"x".repeat(65)).is_err()); // Too long
    }

    #[test]
    fn test_validate_resolution_boundaries() {
        // Boundary values
        assert!(validate_resolution(1, 1).is_ok());
        assert!(validate_resolution(4096, 4096).is_ok()); // Max
        assert!(validate_resolution(4097, 1080).is_err());
        assert!(validate_resolution(1920, 4097).is_err());
    }

    #[test]
    fn test_validate_frame_rate_boundaries() {
        assert!(validate_frame_rate(1).is_ok());
        assert!(validate_frame_rate(120).is_ok()); // Max
        assert!(validate_frame_rate(121).is_err());
    }
}
