//! Imaging settings operations.
//!
//! This module provides handlers for imaging settings operations:
//! - GetImagingSettings - Retrieve current imaging settings
//! - SetImagingSettings - Update imaging settings
//! - GetOptions - Retrieve supported imaging options

use std::sync::Arc;

use crate::onvif::error::{OnvifError, OnvifResult};
use crate::onvif::types::imaging::{
    GetImagingSettings, GetImagingSettingsResponse, GetOptions, GetOptionsResponse,
    SetImagingSettings, SetImagingSettingsResponse,
};
use crate::platform::Platform;

use crate::onvif::imaging::store::ImagingSettingsStore;

use super::error::map_settings_error;

/// Handle the ONVIF GetImagingSettings request.
///
/// Returns the current imaging settings (brightness, contrast, saturation,
/// sharpness, etc.) for the video source identified by the request token.
///
/// # Arguments
///
/// * `store` - Shared imaging settings store for looking up per-source settings
/// * `request` - The `GetImagingSettings` request containing the video source token
///
/// # Returns
///
/// `GetImagingSettingsResponse` with the current `ImagingSettings20` for the source.
///
/// # Errors
///
/// Returns `ter:InvalidArgVal` / `ter:NoSource` if the video source token is
/// not recognized by the store.
pub async fn get_imaging_settings(
    store: &ImagingSettingsStore,
    request: GetImagingSettings,
) -> OnvifResult<GetImagingSettingsResponse> {
    tracing::debug!(
        "GetImagingSettings request for token: {}",
        request.video_source_token
    );

    let settings = store
        .get_settings(&request.video_source_token)
        .await
        .map_err(map_settings_error)?;

    Ok(GetImagingSettingsResponse {
        imaging_settings: settings,
    })
}

/// Handle the ONVIF SetImagingSettings request.
///
/// Validates and persists the new imaging settings for the specified video
/// source, then applies them to the platform hardware if available.
///
/// # Arguments
///
/// * `store` - Shared imaging settings store for persisting per-source settings
/// * `platform` - Optional platform abstraction for applying settings to hardware
/// * `request` - The `SetImagingSettings` request containing the video source token,
///   new imaging settings, and an optional `force_persistence` flag
///
/// # Returns
///
/// `SetImagingSettingsResponse` (empty acknowledgement) on success.
///
/// # Errors
///
/// * Returns `ter:InvalidArgVal` / `ter:NoSource` if the video source token is invalid.
/// * Returns `ter:InvalidArgVal` / `ter:SettingsInvalid` if any setting value is
///   outside the supported range (e.g., brightness outside 0..100).
pub async fn set_imaging_settings(
    store: &ImagingSettingsStore,
    platform: Option<&Arc<dyn Platform>>,
    request: SetImagingSettings,
) -> OnvifResult<SetImagingSettingsResponse> {
    tracing::debug!(
        "SetImagingSettings request for token: {}",
        request.video_source_token
    );

    let force_persistence = request.force_persistence.unwrap_or(false);

    store
        .set_settings(
            &request.video_source_token,
            &request.imaging_settings,
            force_persistence,
        )
        .await
        .map_err(map_settings_error)?;

    apply_platform_settings(platform, &request.imaging_settings).await?;

    Ok(SetImagingSettingsResponse {})
}

/// Handle the ONVIF GetOptions request.
///
/// Returns the supported imaging option ranges (min/max for brightness,
/// contrast, etc.) for the specified video source.
///
/// # Arguments
///
/// * `store` - Shared imaging settings store for looking up per-source options
/// * `request` - The `GetOptions` request containing the video source token
///
/// # Returns
///
/// `GetOptionsResponse` with the `ImagingOptions20` describing valid ranges
/// for each imaging parameter.
///
/// # Errors
///
/// Returns `ter:InvalidArgVal` / `ter:NoSource` if the video source token is
/// not recognized by the store.
pub async fn get_options(
    store: &ImagingSettingsStore,
    request: GetOptions,
) -> OnvifResult<GetOptionsResponse> {
    tracing::debug!(
        "GetOptions request for token: {}",
        request.video_source_token
    );

    let options = store
        .get_options(&request.video_source_token)
        .await
        .map_err(map_settings_error)?;

    Ok(GetOptionsResponse {
        imaging_options: options,
    })
}

/// Apply imaging settings to the platform if available.
///
/// Returns an error if any platform call fails, propagating to the caller
/// so the client receives proper fault information.
#[allow(clippy::collapsible_if)]
async fn apply_platform_settings(
    platform: Option<&Arc<dyn Platform>>,
    settings: &crate::onvif::types::common::ImagingSettings20,
) -> OnvifResult<()> {
    let Some(platform) = platform else {
        return Ok(());
    };
    let Some(imaging) = platform.imaging_control() else {
        return Ok(());
    };

    if let Some(brightness) = settings.brightness {
        imaging
            .set_brightness(brightness)
            .await
            .map_err(|e| OnvifError::HardwareFailure(format!("Failed to set brightness: {}", e)))?;
    }

    if let Some(contrast) = settings.contrast {
        imaging
            .set_contrast(contrast)
            .await
            .map_err(|e| OnvifError::HardwareFailure(format!("Failed to set contrast: {}", e)))?;
    }

    if let Some(saturation) = settings.color_saturation {
        imaging
            .set_saturation(saturation)
            .await
            .map_err(|e| OnvifError::HardwareFailure(format!("Failed to set saturation: {}", e)))?;
    }

    if let Some(sharpness) = settings.sharpness {
        imaging
            .set_sharpness(sharpness)
            .await
            .map_err(|e| OnvifError::HardwareFailure(format!("Failed to set sharpness: {}", e)))?;
    }

    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::onvif::types::common::ImagingSettings20;

    #[tokio::test]
    async fn test_get_imaging_settings() {
        let store = ImagingSettingsStore::new();
        let request = GetImagingSettings {
            video_source_token: "VideoSource_1".to_string(),
        };
        let result = get_imaging_settings(&store, request).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.imaging_settings.brightness, Some(50.0));
    }

    #[tokio::test]
    async fn test_get_imaging_settings_invalid_token() {
        let store = ImagingSettingsStore::new();
        let request = GetImagingSettings {
            video_source_token: "InvalidToken".to_string(),
        };
        let result = get_imaging_settings(&store, request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_set_imaging_settings() {
        let store = ImagingSettingsStore::new();
        let settings = ImagingSettings20 {
            brightness: Some(75.0),
            contrast: Some(60.0),
            color_saturation: Some(50.0),
            sharpness: Some(50.0),
            ir_cut_filter: None,
            wide_dynamic_range: None,
            backlight_compensation: None,
            exposure: None,
            focus: None,
            white_balance: None,
            extension: None,
        };
        let request = SetImagingSettings {
            video_source_token: "VideoSource_1".to_string(),
            imaging_settings: settings,
            force_persistence: Some(false),
        };
        let result = set_imaging_settings(&store, None, request).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_set_imaging_settings_invalid_value() {
        let store = ImagingSettingsStore::new();
        let settings = ImagingSettings20 {
            brightness: Some(150.0), // Out of range
            contrast: None,
            color_saturation: None,
            sharpness: None,
            ir_cut_filter: None,
            wide_dynamic_range: None,
            backlight_compensation: None,
            exposure: None,
            focus: None,
            white_balance: None,
            extension: None,
        };
        let request = SetImagingSettings {
            video_source_token: "VideoSource_1".to_string(),
            imaging_settings: settings,
            force_persistence: None,
        };
        let result = set_imaging_settings(&store, None, request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_options() {
        let store = ImagingSettingsStore::new();
        let request = GetOptions {
            video_source_token: "VideoSource_1".to_string(),
        };
        let result = get_options(&store, request).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.imaging_options.brightness.is_some());
    }

    #[tokio::test]
    async fn test_get_options_invalid_token() {
        let store = ImagingSettingsStore::new();
        let request = GetOptions {
            video_source_token: "InvalidToken".to_string(),
        };
        let result = get_options(&store, request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_set_imaging_settings_boundary_min() {
        let store = ImagingSettingsStore::new();
        let settings = ImagingSettings20 {
            brightness: Some(0.0), // Minimum boundary
            contrast: Some(0.0),
            color_saturation: Some(0.0),
            sharpness: Some(0.0),
            ..Default::default()
        };
        let request = SetImagingSettings {
            video_source_token: "VideoSource_1".to_string(),
            imaging_settings: settings,
            force_persistence: None,
        };
        let result = set_imaging_settings(&store, None, request).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_set_imaging_settings_boundary_max() {
        let store = ImagingSettingsStore::new();
        let settings = ImagingSettings20 {
            brightness: Some(100.0), // Maximum boundary
            contrast: Some(100.0),
            color_saturation: Some(100.0),
            sharpness: Some(100.0),
            ..Default::default()
        };
        let request = SetImagingSettings {
            video_source_token: "VideoSource_1".to_string(),
            imaging_settings: settings,
            force_persistence: None,
        };
        let result = set_imaging_settings(&store, None, request).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_set_imaging_settings_negative_value() {
        let store = ImagingSettingsStore::new();
        let settings = ImagingSettings20 {
            brightness: Some(-10.0), // Negative - out of range
            ..Default::default()
        };
        let request = SetImagingSettings {
            video_source_token: "VideoSource_1".to_string(),
            imaging_settings: settings,
            force_persistence: None,
        };
        let result = set_imaging_settings(&store, None, request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_set_imaging_settings_invalid_token() {
        let store = ImagingSettingsStore::new();
        let settings = ImagingSettings20 {
            brightness: Some(50.0),
            ..Default::default()
        };
        let request = SetImagingSettings {
            video_source_token: "InvalidToken".to_string(),
            imaging_settings: settings,
            force_persistence: None,
        };
        let result = set_imaging_settings(&store, None, request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_set_imaging_settings_with_platform() {
        use crate::platform::stub::StubPlatformBuilder;

        let store = ImagingSettingsStore::new();
        let platform: Arc<dyn Platform> =
            Arc::new(StubPlatformBuilder::new().imaging_supported(true).build());
        let settings = ImagingSettings20 {
            brightness: Some(75.0),
            contrast: Some(60.0),
            color_saturation: Some(50.0),
            sharpness: Some(40.0),
            ..Default::default()
        };
        let request = SetImagingSettings {
            video_source_token: "VideoSource_1".to_string(),
            imaging_settings: settings,
            force_persistence: Some(false),
        };
        let result = set_imaging_settings(&store, Some(&platform), request).await;
        assert!(result.is_ok());

        // Verify settings were persisted in the store
        let stored = store
            .get_settings("VideoSource_1")
            .await
            .expect("settings should exist");
        assert_eq!(stored.brightness, Some(75.0));
        assert_eq!(stored.contrast, Some(60.0));
        assert_eq!(stored.color_saturation, Some(50.0));
        assert_eq!(stored.sharpness, Some(40.0));
    }
}
