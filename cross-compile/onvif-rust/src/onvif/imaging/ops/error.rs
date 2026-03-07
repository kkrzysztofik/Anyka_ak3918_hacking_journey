use crate::onvif::error::OnvifError;

use super::super::store::ImagingSettingsError;

pub(super) fn map_settings_error(err: ImagingSettingsError) -> OnvifError {
    match err {
        ImagingSettingsError::InvalidToken(token) => OnvifError::InvalidArgVal {
            subcode: "ter:InvalidToken".to_string(),
            reason: format!("Invalid video source token: {}", token),
        },
        ImagingSettingsError::OutOfRange {
            parameter,
            value,
            min,
            max,
        } => OnvifError::InvalidArgVal {
            subcode: "ter:InvalidArgVal".to_string(),
            reason: format!(
                "Parameter '{}' value {} is out of range ({} - {})",
                parameter, value, min, max
            ),
        },
        ImagingSettingsError::PlatformError(msg) => OnvifError::HardwareFailure(msg),
        ImagingSettingsError::ValidationFailed(msg) => OnvifError::InvalidArgVal {
            subcode: "ter:InvalidArgVal".to_string(),
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
                assert_eq!(subcode, "ter:InvalidToken");
                assert!(reason.contains("bad-token"));
            }
            other => assert!(
                matches!(other, OnvifError::InvalidArgVal { .. }),
                "unexpected error: {:?}",
                other
            ),
        }
    }
}
