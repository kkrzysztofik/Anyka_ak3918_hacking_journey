//! Imaging preset operations.
//!
//! This module provides handlers for imaging preset operations:
//! - GetPresets - Retrieve available imaging presets
//! - GetCurrentPreset - Get currently active preset
//! - SetCurrentPreset - Apply a specific preset

use crate::onvif::error::{OnvifError, OnvifResult};
use crate::onvif::types::imaging::{
    GetCurrentPreset, GetCurrentPresetResponse, GetPresets, GetPresetsResponse, SetCurrentPreset,
    SetCurrentPresetResponse,
};

use crate::onvif::imaging::faults;
use crate::onvif::imaging::store::ImagingSettingsStore;

/// Handle GetPresets request.
///
/// Returns the list of available imaging presets.
/// Returns empty list if presets are not supported.
pub async fn get_presets(
    store: &ImagingSettingsStore,
    request: GetPresets,
) -> OnvifResult<GetPresetsResponse> {
    // Validate video source token
    if !store.is_valid_token(&request.video_source_token) {
        return Err(faults::invalid_video_source_token(
            &request.video_source_token,
        ));
    }

    // Presets not supported on this device
    Ok(GetPresetsResponse { presets: vec![] })
}

/// Handle GetCurrentPreset request.
///
/// Returns the currently active preset, or None if no preset is active.
pub async fn get_current_preset(
    store: &ImagingSettingsStore,
    request: GetCurrentPreset,
) -> OnvifResult<GetCurrentPresetResponse> {
    // Validate video source token
    if !store.is_valid_token(&request.video_source_token) {
        return Err(faults::invalid_video_source_token(
            &request.video_source_token,
        ));
    }

    // No preset currently active (presets not supported)
    Ok(GetCurrentPresetResponse { preset: None })
}

/// Handle SetCurrentPreset request.
///
/// Applies the specified imaging preset to the video source.
pub async fn set_current_preset(
    store: &ImagingSettingsStore,
    request: SetCurrentPreset,
) -> OnvifResult<SetCurrentPresetResponse> {
    // Validate video source token
    if !store.is_valid_token(&request.video_source_token) {
        return Err(faults::invalid_video_source_token(
            &request.video_source_token,
        ));
    }

    // Presets not supported - return error
    Err(OnvifError::ActionNotSupported(
        "Imaging presets are not supported on this device".to_string(),
    ))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_presets() {
        let store = ImagingSettingsStore::new();
        let request = GetPresets {
            video_source_token: "VideoSource_1".to_string(),
        };
        let result = get_presets(&store, request).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.presets.is_empty()); // No presets supported
    }

    #[tokio::test]
    async fn test_get_presets_invalid_token() {
        let store = ImagingSettingsStore::new();
        let request = GetPresets {
            video_source_token: "InvalidToken".to_string(),
        };
        let result = get_presets(&store, request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_current_preset() {
        let store = ImagingSettingsStore::new();
        let request = GetCurrentPreset {
            video_source_token: "VideoSource_1".to_string(),
        };
        let result = get_current_preset(&store, request).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.preset.is_none()); // No preset active
    }

    #[tokio::test]
    async fn test_get_current_preset_invalid_token() {
        let store = ImagingSettingsStore::new();
        let request = GetCurrentPreset {
            video_source_token: "InvalidToken".to_string(),
        };
        let result = get_current_preset(&store, request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_set_current_preset_not_supported() {
        let store = ImagingSettingsStore::new();
        let request = SetCurrentPreset {
            video_source_token: "VideoSource_1".to_string(),
            preset_token: "Preset_1".to_string(),
        };
        let result = set_current_preset(&store, request).await;
        assert!(result.is_err());
        assert!(matches!(result, Err(OnvifError::ActionNotSupported(_))));
    }
}
