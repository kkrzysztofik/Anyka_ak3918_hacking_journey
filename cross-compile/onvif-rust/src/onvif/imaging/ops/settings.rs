//! Imaging settings operations.
//!
//! This module provides handlers for imaging settings operations:
//! - GetImagingSettings - Retrieve current imaging settings
//! - SetImagingSettings - Update imaging settings
//! - GetOptions - Retrieve supported imaging options

use std::sync::Arc;

use crate::onvif::error::OnvifResult;
use crate::onvif::types::imaging::{
    GetImagingSettings, GetImagingSettingsResponse, GetOptions, GetOptionsResponse,
    SetImagingSettings, SetImagingSettingsResponse,
};
use crate::platform::Platform;

use crate::onvif::imaging::store::{ImagingSettingsError, ImagingSettingsStore};

/// Handle GetImagingSettings request.
///
/// Returns current imaging settings for the specified video source.
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

/// Handle SetImagingSettings request.
///
/// Updates imaging settings for the specified video source.
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

    apply_platform_settings(platform, &request.imaging_settings).await;

    Ok(SetImagingSettingsResponse {})
}

/// Handle GetOptions request.
///
/// Returns supported imaging options for the specified video source.
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
#[allow(clippy::collapsible_if)]
async fn apply_platform_settings(
    platform: Option<&Arc<dyn Platform>>,
    settings: &crate::onvif::types::common::ImagingSettings20,
) {
    let Some(platform) = platform else {
        return;
    };
    let Some(imaging) = platform.imaging_control() else {
        return;
    };

    if let Some(brightness) = settings.brightness {
        if let Err(e) = imaging.set_brightness(brightness).await {
            tracing::warn!("Failed to set brightness: {}", e);
        }
    }

    if let Some(contrast) = settings.contrast {
        if let Err(e) = imaging.set_contrast(contrast).await {
            tracing::warn!("Failed to set contrast: {}", e);
        }
    }

    if let Some(saturation) = settings.color_saturation {
        if let Err(e) = imaging.set_saturation(saturation).await {
            tracing::warn!("Failed to set saturation: {}", e);
        }
    }

    if let Some(sharpness) = settings.sharpness {
        if let Err(e) = imaging.set_sharpness(sharpness).await {
            tracing::warn!("Failed to set sharpness: {}", e);
        }
    }
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
}
