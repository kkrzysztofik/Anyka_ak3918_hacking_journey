//! Media Service fault mappings.
//!
//! This module provides media-specific fault codes and validation helpers.

pub use super::validation::{
    validate_bitrate, validate_config_token, validate_frame_rate, validate_profile_name,
    validate_profile_token, validate_quality, validate_resolution, validate_source_token,
};
use crate::onvif::error::OnvifError;

/// Create a NoProfile error.
pub fn no_profile_error(token: &str) -> OnvifError {
    OnvifError::invalid_arg_val(
        "NoProfile",
        format!("Profile with token '{}' not found", token),
    )
}

/// Create a NoConfig error.
pub fn no_config_error(token: &str) -> OnvifError {
    OnvifError::invalid_arg_val(
        "NoConfig",
        format!("Configuration with token '{}' not found", token),
    )
}

/// Create a NoSource error.
pub fn no_source_error(token: &str) -> OnvifError {
    OnvifError::invalid_arg_val(
        "NoSource",
        format!("Source with token '{}' not found", token),
    )
}

/// Create a ConfigurationConflict error.
pub fn config_conflict_error(reason: &str) -> OnvifError {
    OnvifError::ConfigurationConflict(reason.to_string())
}

/// Create a ConfigModify error for fixed configurations.
pub fn config_modify_error() -> OnvifError {
    OnvifError::invalid_arg_val("ConfigModify", "Cannot modify fixed configuration")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::onvif::common::limits::MAX_REFERENCE_TOKEN_CHARS;

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
    fn test_no_profile_error() {
        let err = no_profile_error("Profile_99");
        assert!(
            matches!(err, OnvifError::InvalidArgVal { ref subcode, .. } if subcode == "NoProfile")
        );
        assert!(err.to_string().contains("Profile_99"));
    }

    #[test]
    fn test_no_config_error() {
        let err = no_config_error("Config_99");
        assert!(
            matches!(err, OnvifError::InvalidArgVal { ref subcode, .. } if subcode == "NoConfig")
        );
        assert!(err.to_string().contains("Config_99"));
    }

    #[test]
    fn test_no_source_error() {
        let err = no_source_error("Source_99");
        assert!(
            matches!(err, OnvifError::InvalidArgVal { ref subcode, .. } if subcode == "NoSource")
        );
        assert!(err.to_string().contains("Source_99"));
    }

    #[test]
    fn test_config_conflict_error() {
        let err = config_conflict_error("Configuration already in use");
        assert!(matches!(err, OnvifError::ConfigurationConflict(_)));
        assert!(err.to_string().contains("Configuration already in use"));
    }

    #[test]
    fn test_config_modify_error() {
        let err = config_modify_error();
        assert!(
            matches!(err, OnvifError::InvalidArgVal { ref subcode, .. } if subcode == "ConfigModify")
        );
        assert!(
            err.to_string()
                .contains("Cannot modify fixed configuration")
        );
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
