//! Imaging focus operations.
//!
//! This module provides handlers for imaging focus operations:
//! - GetMoveOptions - Retrieve supported focus move options
//! - Move - Perform focus movement
//! - Stop - Stop ongoing focus movement

use crate::onvif::error::{OnvifError, OnvifResult};
use crate::onvif::types::imaging::{
    GetMoveOptions, GetMoveOptionsResponse, Move, MoveResponse, Stop, StopResponse,
};

use crate::onvif::imaging::faults;
use crate::onvif::imaging::store::ImagingSettingsStore;

/// Handle GetMoveOptions request.
///
/// Returns supported focus move options for the specified video source.
pub async fn get_move_options(
    store: &ImagingSettingsStore,
    request: GetMoveOptions,
) -> OnvifResult<GetMoveOptionsResponse> {
    tracing::debug!(
        "GetMoveOptions request for token: {}",
        request.video_source_token
    );

    // Validate token
    if !store.is_valid_token(&request.video_source_token) {
        return Err(faults::invalid_video_source_token(
            &request.video_source_token,
        ));
    }

    // Return default move options (focus not supported in this implementation)
    Ok(GetMoveOptionsResponse {
        move_options: crate::onvif::types::imaging::MoveOptions20::default(),
    })
}

/// Handle Move request (focus).
///
/// Not supported - returns ActionNotSupported error.
pub async fn handle_move(store: &ImagingSettingsStore, request: Move) -> OnvifResult<MoveResponse> {
    tracing::debug!(
        "Move (focus) request for token: {} (not supported)",
        request.video_source_token
    );

    // Validate token
    if !store.is_valid_token(&request.video_source_token) {
        return Err(faults::invalid_video_source_token(
            &request.video_source_token,
        ));
    }

    Err(OnvifError::ActionNotSupported(
        "Focus move operation not supported".to_string(),
    ))
}

/// Handle Stop request (focus).
///
/// Not supported - returns ActionNotSupported error.
pub async fn handle_stop(store: &ImagingSettingsStore, request: Stop) -> OnvifResult<StopResponse> {
    tracing::debug!(
        "Stop (focus) request for token: {} (not supported)",
        request.video_source_token
    );

    // Validate token
    if !store.is_valid_token(&request.video_source_token) {
        return Err(faults::invalid_video_source_token(
            &request.video_source_token,
        ));
    }

    Err(OnvifError::ActionNotSupported(
        "Focus stop operation not supported".to_string(),
    ))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::onvif::types::imaging::FocusMove;

    #[tokio::test]
    async fn test_get_move_options() {
        let store = ImagingSettingsStore::new();
        let request = GetMoveOptions {
            video_source_token: "VideoSource_1".to_string(),
        };
        let result = get_move_options(&store, request).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_move_options_invalid_token() {
        let store = ImagingSettingsStore::new();
        let request = GetMoveOptions {
            video_source_token: "InvalidToken".to_string(),
        };
        let result = get_move_options(&store, request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_move_not_supported() {
        let store = ImagingSettingsStore::new();
        let request = Move {
            video_source_token: "VideoSource_1".to_string(),
            focus: FocusMove::default(),
        };
        let result = handle_move(&store, request).await;
        assert!(result.is_err());
        assert!(matches!(result, Err(OnvifError::ActionNotSupported(_))));
    }

    #[tokio::test]
    async fn test_move_invalid_token() {
        let store = ImagingSettingsStore::new();
        let request = Move {
            video_source_token: "InvalidToken".to_string(),
            focus: FocusMove::default(),
        };
        let result = handle_move(&store, request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_stop_not_supported() {
        let store = ImagingSettingsStore::new();
        let request = Stop {
            video_source_token: "VideoSource_1".to_string(),
        };
        let result = handle_stop(&store, request).await;
        assert!(result.is_err());
        assert!(matches!(result, Err(OnvifError::ActionNotSupported(_))));
    }

    #[tokio::test]
    async fn test_stop_invalid_token() {
        let store = ImagingSettingsStore::new();
        let request = Stop {
            video_source_token: "InvalidToken".to_string(),
        };
        let result = handle_stop(&store, request).await;
        assert!(result.is_err());
    }
}
