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

/// Handle the ONVIF GetMoveOptions request for imaging focus.
///
/// Returns the supported focus move options for the specified video source.
/// Since this device does not support motorized focus, the returned options
/// are empty defaults.
///
/// # Arguments
///
/// * `store` - Shared imaging settings store used to validate the video source token
/// * `request` - The `GetMoveOptions` request containing the video source token
///
/// # Returns
///
/// `GetMoveOptionsResponse` with default (empty) `MoveOptions20`.
///
/// # Errors
///
/// Returns `ter:InvalidArgVal` / `ter:NoSource` if the video source token is invalid.
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

/// Handle the ONVIF Move request for imaging focus.
///
/// This device does not support motorized focus, so a valid token produces
/// an `ActionNotSupported` error after token validation.
///
/// # Arguments
///
/// * `store` - Shared imaging settings store used to validate the video source token
/// * `request` - The `Move` request containing the video source token and focus move data
///
/// # Returns
///
/// This function never returns `Ok`; it always produces an error.
///
/// # Errors
///
/// * Returns `ter:InvalidArgVal` / `ter:NoSource` if the video source token is invalid.
/// * Returns `ter:ActionNotSupported` for any valid token because focus move is
///   not supported on this device.
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

/// Handle the ONVIF Stop request for imaging focus.
///
/// This device does not support motorized focus, so a valid token produces
/// an `ActionNotSupported` error after token validation.
///
/// # Arguments
///
/// * `store` - Shared imaging settings store used to validate the video source token
/// * `request` - The `Stop` request containing the video source token
///
/// # Returns
///
/// This function never returns `Ok`; it always produces an error.
///
/// # Errors
///
/// * Returns `ter:InvalidArgVal` / `ter:NoSource` if the video source token is invalid.
/// * Returns `ter:ActionNotSupported` for any valid token because focus stop is
///   not supported on this device.
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
    async fn test_focus_get_move_options_returns_defaults() {
        let store = ImagingSettingsStore::new();
        let request = GetMoveOptions {
            video_source_token: "VideoSource_1".to_string(),
        };
        let result = get_move_options(&store, request).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_focus_get_move_options_invalid_token_returns_error() {
        let store = ImagingSettingsStore::new();
        let request = GetMoveOptions {
            video_source_token: "InvalidToken".to_string(),
        };
        let result = get_move_options(&store, request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_focus_move_valid_token_returns_not_supported() {
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
    async fn test_focus_move_invalid_token_returns_error() {
        let store = ImagingSettingsStore::new();
        let request = Move {
            video_source_token: "InvalidToken".to_string(),
            focus: FocusMove::default(),
        };
        let result = handle_move(&store, request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_focus_stop_valid_token_returns_not_supported() {
        let store = ImagingSettingsStore::new();
        let request = Stop {
            video_source_token: "VideoSource_1".to_string(),
        };
        let result = handle_stop(&store, request).await;
        assert!(result.is_err());
        assert!(matches!(result, Err(OnvifError::ActionNotSupported(_))));
    }

    #[tokio::test]
    async fn test_focus_stop_invalid_token_returns_error() {
        let store = ImagingSettingsStore::new();
        let request = Stop {
            video_source_token: "InvalidToken".to_string(),
        };
        let result = handle_stop(&store, request).await;
        assert!(result.is_err());
    }
}
