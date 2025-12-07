//! Media Service fault mappings.
//!
//! This module provides media-specific fault codes and validation helpers.

use crate::onvif::error::OnvifError;
use crate::onvif::types::common::ReferenceToken;

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
    if token.len() > 64 {
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
    if token.len() > 64 {
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
    if token.len() > 64 {
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
    if width > 4096 || height > 4096 {
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
    if frame_rate > 120 {
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
    if bitrate > 50_000_000 {
        // 50 Mbps max
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
    if name.len() > 64 {
        return Err(OnvifError::invalid_arg_val(
            "ter:InvalidName",
            "Profile name exceeds maximum length of 64 characters",
        ));
    }
    Ok(())
}

/// Create a NoProfile error.
pub fn no_profile_error(token: &str) -> OnvifError {
    OnvifError::invalid_arg_val(
        "ter:NoProfile",
        format!("Profile with token '{}' not found", token),
    )
}

/// Create a NoConfig error.
pub fn no_config_error(token: &str) -> OnvifError {
    OnvifError::invalid_arg_val(
        "ter:NoConfig",
        format!("Configuration with token '{}' not found", token),
    )
}

/// Create a NoSource error.
pub fn no_source_error(token: &str) -> OnvifError {
    OnvifError::invalid_arg_val(
        "ter:NoSource",
        format!("Source with token '{}' not found", token),
    )
}

/// Create a ConfigurationConflict error.
pub fn config_conflict_error(reason: &str) -> OnvifError {
    OnvifError::ConfigurationConflict(reason.to_string())
}

/// Create a ConfigModify error for fixed configurations.
pub fn config_modify_error() -> OnvifError {
    OnvifError::invalid_arg_val("ter:ConfigModify", "Cannot modify fixed configuration")
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
}
