//! Imaging status operations.
//!
//! This module provides handlers for imaging status operations:
//! - GetStatus - Retrieve current imaging status

use crate::onvif::error::OnvifResult;
use crate::onvif::types::imaging::{GetStatus, GetStatusResponse};

use crate::onvif::imaging::store::{ImagingSettingsError, ImagingSettingsStore};

/// Handle GetStatus request.
///
/// Returns imaging status including focus status for the specified video source.
pub async fn get_status(
    store: &ImagingSettingsStore,
    request: GetStatus,
) -> OnvifResult<GetStatusResponse> {
    tracing::debug!(
        "GetStatus request for token: {}",
        request.video_source_token
    );

    let status = store
        .get_status(&request.video_source_token)
        .await
        .map_err(map_settings_error)?;

    Ok(GetStatusResponse { status })
}

/// Map settings error to ONVIF error.
fn map_settings_error(err: ImagingSettingsError) -> crate::onvif::error::OnvifError {
    use crate::onvif::error::OnvifError;

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

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_status() {
        let store = ImagingSettingsStore::new();
        let request = GetStatus {
            video_source_token: "VideoSource_1".to_string(),
        };
        let result = get_status(&store, request).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_status_invalid_token() {
        let store = ImagingSettingsStore::new();
        let request = GetStatus {
            video_source_token: "InvalidToken".to_string(),
        };
        let result = get_status(&store, request).await;
        assert!(result.is_err());
    }
}
