use crate::onvif::error::OnvifError;

use super::super::store::ImagingSettingsError;

pub(super) fn map_settings_error(err: ImagingSettingsError) -> OnvifError {
    match err {
        ImagingSettingsError::InvalidToken(token) => {
            crate::onvif::common::invalid_video_source(&token)
        }
        ImagingSettingsError::OutOfRange {
            parameter,
            value,
            min,
            max,
        } => OnvifError::InvalidArgVal {
            subcode: "InvalidArgVal".to_string(),
            reason: format!(
                "Parameter '{}' value {} is out of range ({} - {})",
                parameter, value, min, max
            ),
        },
        ImagingSettingsError::PlatformError(msg) => {
            // Log the raw platform error for diagnostics but don't expose internals to client
            tracing::warn!("Platform imaging error: {}", msg);
            OnvifError::HardwareFailure("Hardware query failed".to_string())
        }
        ImagingSettingsError::ValidationFailed(msg) => OnvifError::InvalidArgVal {
            subcode: "InvalidArgVal".to_string(),
            reason: msg,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_settings_error_invalid_token() {
        let error = map_settings_error(ImagingSettingsError::InvalidToken("bad-token".to_string()));

        match error {
            OnvifError::InvalidArgVal { subcode, reason } => {
                assert_eq!(subcode, "InvalidToken");
                assert!(reason.contains("bad-token"));
            }
            other => panic!("expected InvalidArgVal, got: {:?}", other),
        }
    }

    #[test]
    fn test_map_settings_error_out_of_range() {
        let error = map_settings_error(ImagingSettingsError::OutOfRange {
            parameter: "Brightness".to_string(),
            value: 150.0,
            min: 0.0,
            max: 100.0,
        });

        match error {
            OnvifError::InvalidArgVal { subcode, reason } => {
                assert_eq!(subcode, "InvalidArgVal");
                assert!(reason.contains("Brightness"));
                assert!(reason.contains("150"));
            }
            other => panic!("expected InvalidArgVal, got: {:?}", other),
        }
    }

    #[test]
    fn test_map_settings_error_platform_error() {
        let error = map_settings_error(ImagingSettingsError::PlatformError(
            "i2c timeout".to_string(),
        ));

        match error {
            OnvifError::HardwareFailure(msg) => {
                // Internal details should not leak to the client
                assert!(!msg.contains("i2c"));
                assert!(msg.contains("Hardware"));
            }
            other => panic!("expected HardwareFailure, got: {:?}", other),
        }
    }

    #[test]
    fn test_map_settings_error_validation_failed() {
        let error = map_settings_error(ImagingSettingsError::ValidationFailed(
            "brightness exceeds device maximum".to_string(),
        ));

        match error {
            OnvifError::InvalidArgVal { subcode, reason } => {
                assert_eq!(subcode, "InvalidArgVal");
                assert!(reason.contains("brightness"));
            }
            other => panic!("expected InvalidArgVal, got: {:?}", other),
        }
    }
}
