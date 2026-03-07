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
            subcode: "ter:InvalidArgVal".to_string(),
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
