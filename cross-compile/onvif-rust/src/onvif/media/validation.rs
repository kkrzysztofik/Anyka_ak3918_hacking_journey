//! Media Service validation functions.
//!
//! This module provides validation helpers for Media Service operations.
//! These functions validate input parameters according to ONVIF specifications.

use crate::onvif::common::MAX_REFERENCE_TOKEN_CHARS;
use crate::onvif::error::OnvifError;
use crate::onvif::types::common::ReferenceToken;

/// Maximum supported resolution dimension (pixels).
const MAX_RESOLUTION: i32 = 4096;

/// Maximum supported frame rate (fps).
const MAX_FRAME_RATE: i32 = 120;

/// Maximum supported bitrate (bits per second, 50 Mbps).
const MAX_BITRATE: i32 = 50_000_000;

/// Maximum length for profile names.
const MAX_PROFILE_NAME_LENGTH: usize = 64;

/// Static error text for oversized profile tokens (avoid heap `format!` on hot path).
const PROFILE_TOKEN_EXCEEDS_MAX_LENGTH: &str = "Profile token exceeds maximum length";

/// Static error text for oversized configuration tokens (avoid heap `format!` on hot path).
const CONFIG_TOKEN_EXCEEDS_MAX_LENGTH: &str = "Configuration token exceeds maximum length";

/// Static error text for oversized source tokens (avoid heap `format!` on hot path).
const SOURCE_TOKEN_EXCEEDS_MAX_LENGTH: &str = "Source token exceeds maximum length";

/// Validate a profile reference token.
///
/// Checks that the token is non-empty and does not exceed
/// [`MAX_REFERENCE_TOKEN_CHARS`].
///
/// # Arguments
///
/// * `token` - The profile reference token to validate.
///
/// # Returns
///
/// `Ok(())` when the token is valid.
///
/// # Errors
///
/// * `OnvifError::InvalidArgVal("NoProfile", ...)` -- token is empty.
/// * `OnvifError::InvalidArgVal("InvalidToken", ...)` -- token exceeds max length.
///
/// # Examples
///
/// ```rust,ignore
/// assert!(validate_profile_token(&"Profile_0".to_string()).is_ok());
/// assert!(validate_profile_token(&String::new()).is_err());
/// ```
pub fn validate_profile_token(token: &ReferenceToken) -> Result<(), OnvifError> {
    if token.is_empty() {
        return Err(OnvifError::invalid_arg_val(
            "NoProfile",
            "Profile token is empty",
        ));
    }
    if token.len() > MAX_REFERENCE_TOKEN_CHARS {
        return Err(OnvifError::invalid_arg_val(
            "InvalidToken",
            PROFILE_TOKEN_EXCEEDS_MAX_LENGTH,
        ));
    }
    Ok(())
}

/// Validate a configuration reference token.
///
/// Checks that the token is non-empty and within the 64-character ONVIF limit.
///
/// # Arguments
///
/// * `token` - The configuration reference token to validate.
///
/// # Returns
///
/// `Ok(())` when the token is valid.
///
/// # Errors
///
/// * `OnvifError::InvalidArgVal("NoConfig", ...)` -- token is empty.
/// * `OnvifError::InvalidArgVal("InvalidToken", ...)` -- token exceeds max length.
///
/// # Examples
///
/// ```rust,ignore
/// assert!(validate_config_token(&"VideoEncoderConfig_0".to_string()).is_ok());
/// assert!(validate_config_token(&String::new()).is_err());
/// ```
pub fn validate_config_token(token: &ReferenceToken) -> Result<(), OnvifError> {
    if token.is_empty() {
        return Err(OnvifError::invalid_arg_val(
            "NoConfig",
            "Configuration token is empty",
        ));
    }
    if token.len() > MAX_REFERENCE_TOKEN_CHARS {
        return Err(OnvifError::invalid_arg_val(
            "InvalidToken",
            CONFIG_TOKEN_EXCEEDS_MAX_LENGTH,
        ));
    }
    Ok(())
}

/// Validate a source reference token (video or audio).
///
/// Checks that the token is non-empty and within [`MAX_REFERENCE_TOKEN_CHARS`].
///
/// # Arguments
///
/// * `token` - The source reference token to validate.
///
/// # Returns
///
/// `Ok(())` when the token is valid.
///
/// # Errors
///
/// * `OnvifError::InvalidArgVal("NoSource", ...)` -- token is empty.
/// * `OnvifError::InvalidArgVal("InvalidToken", ...)` -- token exceeds max length.
///
/// # Examples
///
/// ```rust,ignore
/// assert!(validate_source_token(&"VideoSource_0".to_string()).is_ok());
/// assert!(validate_source_token(&String::new()).is_err());
/// ```
pub fn validate_source_token(token: &ReferenceToken) -> Result<(), OnvifError> {
    if token.is_empty() {
        return Err(OnvifError::invalid_arg_val(
            "NoSource",
            "Source token is empty",
        ));
    }
    if token.len() > MAX_REFERENCE_TOKEN_CHARS {
        return Err(OnvifError::invalid_arg_val(
            "InvalidToken",
            SOURCE_TOKEN_EXCEEDS_MAX_LENGTH,
        ));
    }
    Ok(())
}

/// Validate video resolution dimensions.
///
/// Both dimensions must be positive and at most 4096 pixels.
///
/// # Arguments
///
/// * `width`  - Horizontal resolution in pixels.
/// * `height` - Vertical resolution in pixels.
///
/// # Returns
///
/// `Ok(())` when the resolution is within the supported range.
///
/// # Errors
///
/// * `OnvifError::InvalidArgVal("InvalidResolution", ...)` -- a dimension is
///   zero, negative, or exceeds 4096.
///
/// # Examples
///
/// ```rust,ignore
/// assert!(validate_resolution(1920, 1080).is_ok());
/// assert!(validate_resolution(0, 1080).is_err());
/// ```
pub fn validate_resolution(width: i32, height: i32) -> Result<(), OnvifError> {
    if width <= 0 || height <= 0 {
        return Err(OnvifError::invalid_arg_val(
            "InvalidResolution",
            "Resolution width and height must be positive",
        ));
    }
    if width > MAX_RESOLUTION || height > MAX_RESOLUTION {
        return Err(OnvifError::invalid_arg_val(
            "InvalidResolution",
            "Resolution exceeds maximum supported (4096x4096)",
        ));
    }
    Ok(())
}

/// Validate a frame rate value.
///
/// The frame rate must be positive and at most 120 fps.
///
/// # Arguments
///
/// * `frame_rate` - Frame rate in frames per second.
///
/// # Returns
///
/// `Ok(())` when the frame rate is within the supported range.
///
/// # Errors
///
/// * `OnvifError::InvalidArgVal("InvalidFrameRate", ...)` -- value is zero,
///   negative, or exceeds 120.
///
/// # Examples
///
/// ```rust,ignore
/// assert!(validate_frame_rate(30).is_ok());
/// assert!(validate_frame_rate(0).is_err());
/// ```
pub fn validate_frame_rate(frame_rate: i32) -> Result<(), OnvifError> {
    if frame_rate <= 0 {
        return Err(OnvifError::invalid_arg_val(
            "InvalidFrameRate",
            "Frame rate must be positive",
        ));
    }
    if frame_rate > MAX_FRAME_RATE {
        return Err(OnvifError::invalid_arg_val(
            "InvalidFrameRate",
            "Frame rate exceeds maximum supported (120 fps)",
        ));
    }
    Ok(())
}

/// Validate a bitrate value.
///
/// The bitrate must be positive and at most 50 Mbps (50,000,000 bps).
///
/// # Arguments
///
/// * `bitrate` - Bitrate in bits per second.
///
/// # Returns
///
/// `Ok(())` when the bitrate is within the supported range.
///
/// # Errors
///
/// * `OnvifError::InvalidArgVal("InvalidBitrate", ...)` -- value is zero,
///   negative, or exceeds 50 Mbps.
///
/// # Examples
///
/// ```rust,ignore
/// assert!(validate_bitrate(4_000_000).is_ok());
/// assert!(validate_bitrate(0).is_err());
/// ```
pub fn validate_bitrate(bitrate: i32) -> Result<(), OnvifError> {
    if bitrate <= 0 {
        return Err(OnvifError::invalid_arg_val(
            "InvalidBitrate",
            "Bitrate must be positive",
        ));
    }
    if bitrate > MAX_BITRATE {
        return Err(OnvifError::invalid_arg_val(
            "InvalidBitrate",
            "Bitrate exceeds maximum supported (50 Mbps)",
        ));
    }
    Ok(())
}

/// Validate a quality setting.
///
/// The quality value must be in the inclusive range `[0.0, 1.0]`.
///
/// # Arguments
///
/// * `quality` - Quality factor where `0.0` is lowest and `1.0` is highest.
///
/// # Returns
///
/// `Ok(())` when the quality is within range.
///
/// # Errors
///
/// * `OnvifError::InvalidArgVal("InvalidQuality", ...)` -- value is outside
///   the `[0.0, 1.0]` range.
///
/// # Examples
///
/// ```rust,ignore
/// assert!(validate_quality(0.8).is_ok());
/// assert!(validate_quality(1.5).is_err());
/// ```
pub fn validate_quality(quality: f32) -> Result<(), OnvifError> {
    if !(0.0..=1.0).contains(&quality) {
        return Err(OnvifError::invalid_arg_val(
            "InvalidQuality",
            "Quality must be between 0.0 and 1.0",
        ));
    }
    Ok(())
}

/// Validate a profile name.
///
/// The name must be non-empty and at most 64 characters.
///
/// # Arguments
///
/// * `name` - The human-readable profile name.
///
/// # Returns
///
/// `Ok(())` when the name is valid.
///
/// # Errors
///
/// * `OnvifError::InvalidArgVal("InvalidName", ...)` -- name is empty or
///   exceeds 64 characters.
///
/// # Examples
///
/// ```rust,ignore
/// assert!(validate_profile_name("MainStream").is_ok());
/// assert!(validate_profile_name("").is_err());
/// ```
pub fn validate_profile_name(name: &str) -> Result<(), OnvifError> {
    if name.is_empty() {
        return Err(OnvifError::invalid_arg_val(
            "InvalidName",
            "Profile name is empty",
        ));
    }
    if name.len() > MAX_PROFILE_NAME_LENGTH {
        return Err(OnvifError::invalid_arg_val(
            "InvalidName",
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
        let long_token = "x".repeat(MAX_REFERENCE_TOKEN_CHARS + 1);
        let result = validate_profile_token(&long_token);
        assert!(result.is_err());
        let valid_token = "x".repeat(MAX_REFERENCE_TOKEN_CHARS);
        assert!(validate_profile_token(&valid_token).is_ok());
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
        let long_token = "x".repeat(MAX_REFERENCE_TOKEN_CHARS + 1);
        assert!(validate_config_token(&long_token).is_err());
        let max_token = "x".repeat(MAX_REFERENCE_TOKEN_CHARS);
        assert!(validate_config_token(&max_token).is_ok());
    }

    #[test]
    fn test_validate_source_token() {
        assert!(validate_source_token(&"Source_1".to_string()).is_ok());
        assert!(validate_source_token(&String::new()).is_err());
        let long_token = "x".repeat(MAX_REFERENCE_TOKEN_CHARS + 1);
        assert!(validate_source_token(&long_token).is_err());
        let max_token = "x".repeat(MAX_REFERENCE_TOKEN_CHARS);
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
