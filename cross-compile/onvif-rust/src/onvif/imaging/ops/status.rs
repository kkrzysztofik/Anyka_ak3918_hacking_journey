//! Imaging status operations.
//!
//! This module provides handlers for imaging status operations:
//! - GetStatus - Retrieve current imaging status

use crate::onvif::error::OnvifResult;
use crate::onvif::types::imaging::{GetStatus, GetStatusResponse};

use crate::onvif::imaging::store::ImagingSettingsStore;

use super::error::map_settings_error;

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
