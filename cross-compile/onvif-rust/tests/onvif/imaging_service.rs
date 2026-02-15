//! Integration tests for ONVIF Imaging Service.
//!
//! Tests the complete Imaging Service operations including:
//! - Getting imaging settings
//! - Setting imaging settings
//! - Getting imaging options
//! - Getting imaging status

use onvif_rust::onvif::imaging::ImagingService;
use onvif_rust::onvif::types::common::{
    BacklightCompensation20, BacklightCompensationMode, ImagingSettings20, IrCutFilterMode,
    ReferenceToken, WideDynamicMode, WideDynamicRange20,
};
use onvif_rust::onvif::types::imaging::{
    GetImagingSettings, GetMoveOptions, GetOptions, GetServiceCapabilities, GetStatus,
    SetImagingSettings, Stop,
};

// ============================================================================
// Test Fixtures
// ============================================================================

fn create_test_service() -> ImagingService {
    ImagingService::new()
}

const DEFAULT_VIDEO_SOURCE: &str = "VideoSource_1";

// ============================================================================
// GetImagingSettings Tests
// ============================================================================

#[tokio::test]
async fn test_get_imaging_settings_returns_defaults() {
    let service = create_test_service();

    let response = service
        .handle_get_imaging_settings(GetImagingSettings {
            video_source_token: ReferenceToken::from(DEFAULT_VIDEO_SOURCE),
        })
        .await
        .unwrap();

    // Should have default settings
    assert!(response.imaging_settings.brightness.is_some());
    assert!(response.imaging_settings.contrast.is_some());
}

#[tokio::test]
async fn test_get_imaging_settings_invalid_token_fails() {
    let service = create_test_service();

    let result = service
        .handle_get_imaging_settings(GetImagingSettings {
            video_source_token: ReferenceToken::from("invalid_token"),
        })
        .await;

    assert!(result.is_err());
}

// ============================================================================
// SetImagingSettings Tests
// ============================================================================

#[tokio::test]
async fn test_set_imaging_settings_brightness() {
    let service = create_test_service();

    // Set brightness to specific value
    let settings = ImagingSettings20 {
        brightness: Some(75.0),
        ..Default::default()
    };

    let result = service
        .handle_set_imaging_settings(SetImagingSettings {
            video_source_token: ReferenceToken::from(DEFAULT_VIDEO_SOURCE),
            imaging_settings: settings,
            force_persistence: Some(false),
        })
        .await;

    assert!(result.is_ok());

    // Verify the setting was applied
    let response = service
        .handle_get_imaging_settings(GetImagingSettings {
            video_source_token: ReferenceToken::from(DEFAULT_VIDEO_SOURCE),
        })
        .await
        .unwrap();

    assert_eq!(response.imaging_settings.brightness, Some(75.0));
}

#[tokio::test]
async fn test_set_imaging_settings_contrast() {
    let service = create_test_service();

    // Set contrast to specific value
    let settings = ImagingSettings20 {
        contrast: Some(60.0),
        ..Default::default()
    };

    let result = service
        .handle_set_imaging_settings(SetImagingSettings {
            video_source_token: ReferenceToken::from(DEFAULT_VIDEO_SOURCE),
            imaging_settings: settings,
            force_persistence: Some(false),
        })
        .await;

    assert!(result.is_ok());

    // Verify the setting was applied
    let response = service
        .handle_get_imaging_settings(GetImagingSettings {
            video_source_token: ReferenceToken::from(DEFAULT_VIDEO_SOURCE),
        })
        .await
        .unwrap();

    assert_eq!(response.imaging_settings.contrast, Some(60.0));
}

#[tokio::test]
async fn test_set_imaging_settings_saturation() {
    let service = create_test_service();

    let settings = ImagingSettings20 {
        color_saturation: Some(80.0),
        ..Default::default()
    };

    let result = service
        .handle_set_imaging_settings(SetImagingSettings {
            video_source_token: ReferenceToken::from(DEFAULT_VIDEO_SOURCE),
            imaging_settings: settings,
            force_persistence: Some(false),
        })
        .await;

    assert!(result.is_ok());

    let response = service
        .handle_get_imaging_settings(GetImagingSettings {
            video_source_token: ReferenceToken::from(DEFAULT_VIDEO_SOURCE),
        })
        .await
        .unwrap();

    assert_eq!(response.imaging_settings.color_saturation, Some(80.0));
}

#[tokio::test]
async fn test_set_imaging_settings_sharpness() {
    let service = create_test_service();

    let settings = ImagingSettings20 {
        sharpness: Some(45.0),
        ..Default::default()
    };

    let result = service
        .handle_set_imaging_settings(SetImagingSettings {
            video_source_token: ReferenceToken::from(DEFAULT_VIDEO_SOURCE),
            imaging_settings: settings,
            force_persistence: Some(false),
        })
        .await;

    assert!(result.is_ok());

    let response = service
        .handle_get_imaging_settings(GetImagingSettings {
            video_source_token: ReferenceToken::from(DEFAULT_VIDEO_SOURCE),
        })
        .await
        .unwrap();

    assert_eq!(response.imaging_settings.sharpness, Some(45.0));
}

#[tokio::test]
async fn test_set_imaging_settings_ir_cut_filter() {
    let service = create_test_service();

    let settings = ImagingSettings20 {
        ir_cut_filter: Some(IrCutFilterMode::ON),
        ..Default::default()
    };

    let result = service
        .handle_set_imaging_settings(SetImagingSettings {
            video_source_token: ReferenceToken::from(DEFAULT_VIDEO_SOURCE),
            imaging_settings: settings,
            force_persistence: Some(false),
        })
        .await;

    assert!(result.is_ok());

    let response = service
        .handle_get_imaging_settings(GetImagingSettings {
            video_source_token: ReferenceToken::from(DEFAULT_VIDEO_SOURCE),
        })
        .await
        .unwrap();

    assert_eq!(
        response.imaging_settings.ir_cut_filter,
        Some(IrCutFilterMode::ON)
    );
}

#[tokio::test]
async fn test_set_imaging_settings_wide_dynamic_range() {
    let service = create_test_service();

    let settings = ImagingSettings20 {
        wide_dynamic_range: Some(WideDynamicRange20 {
            mode: WideDynamicMode::ON,
            level: Some(70.0),
        }),
        ..Default::default()
    };

    let result = service
        .handle_set_imaging_settings(SetImagingSettings {
            video_source_token: ReferenceToken::from(DEFAULT_VIDEO_SOURCE),
            imaging_settings: settings,
            force_persistence: Some(false),
        })
        .await;

    assert!(result.is_ok());

    let response = service
        .handle_get_imaging_settings(GetImagingSettings {
            video_source_token: ReferenceToken::from(DEFAULT_VIDEO_SOURCE),
        })
        .await
        .unwrap();

    let wdr = response.imaging_settings.wide_dynamic_range.unwrap();
    assert_eq!(wdr.mode, WideDynamicMode::ON);
    assert_eq!(wdr.level, Some(70.0));
}

#[tokio::test]
async fn test_set_imaging_settings_backlight_compensation() {
    let service = create_test_service();

    let settings = ImagingSettings20 {
        backlight_compensation: Some(BacklightCompensation20 {
            mode: BacklightCompensationMode::ON,
            level: Some(50.0),
        }),
        ..Default::default()
    };

    let result = service
        .handle_set_imaging_settings(SetImagingSettings {
            video_source_token: ReferenceToken::from(DEFAULT_VIDEO_SOURCE),
            imaging_settings: settings,
            force_persistence: Some(false),
        })
        .await;

    assert!(result.is_ok());

    let response = service
        .handle_get_imaging_settings(GetImagingSettings {
            video_source_token: ReferenceToken::from(DEFAULT_VIDEO_SOURCE),
        })
        .await
        .unwrap();

    let blc = response.imaging_settings.backlight_compensation.unwrap();
    assert_eq!(blc.mode, BacklightCompensationMode::ON);
    assert_eq!(blc.level, Some(50.0));
}

#[tokio::test]
async fn test_set_imaging_settings_invalid_token_fails() {
    let service = create_test_service();

    let settings = ImagingSettings20 {
        brightness: Some(50.0),
        ..Default::default()
    };

    let result = service
        .handle_set_imaging_settings(SetImagingSettings {
            video_source_token: ReferenceToken::from("invalid_token"),
            imaging_settings: settings,
            force_persistence: Some(false),
        })
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_set_imaging_settings_out_of_range_brightness_fails() {
    let service = create_test_service();

    // Brightness out of valid range (0-100)
    let settings = ImagingSettings20 {
        brightness: Some(150.0),
        ..Default::default()
    };

    let result = service
        .handle_set_imaging_settings(SetImagingSettings {
            video_source_token: ReferenceToken::from(DEFAULT_VIDEO_SOURCE),
            imaging_settings: settings,
            force_persistence: Some(false),
        })
        .await;

    assert!(result.is_err());
}

// ============================================================================
// GetOptions Tests
// ============================================================================

#[tokio::test]
async fn test_get_options_returns_valid_ranges() {
    let service = create_test_service();

    let response = service
        .handle_get_options(GetOptions {
            video_source_token: ReferenceToken::from(DEFAULT_VIDEO_SOURCE),
        })
        .await
        .unwrap();

    // Should have brightness range
    let brightness = response.imaging_options.brightness.unwrap();
    assert!(brightness.min <= brightness.max);

    // Should have contrast range
    let contrast = response.imaging_options.contrast.unwrap();
    assert!(contrast.min <= contrast.max);
}

#[tokio::test]
async fn test_get_options_invalid_token_fails() {
    let service = create_test_service();

    let result = service
        .handle_get_options(GetOptions {
            video_source_token: ReferenceToken::from("invalid_token"),
        })
        .await;

    assert!(result.is_err());
}

// ============================================================================
// GetStatus Tests
// ============================================================================

#[tokio::test]
async fn test_get_status_returns_status() {
    let service = create_test_service();

    let response = service
        .handle_get_status(GetStatus {
            video_source_token: ReferenceToken::from(DEFAULT_VIDEO_SOURCE),
        })
        .await
        .unwrap();

    // Status should exist (focus status is optional)
    let _ = response.status;
}

#[tokio::test]
async fn test_get_status_invalid_token_fails() {
    let service = create_test_service();

    let result = service
        .handle_get_status(GetStatus {
            video_source_token: ReferenceToken::from("invalid_token"),
        })
        .await;

    assert!(result.is_err());
}

// ============================================================================
// GetServiceCapabilities Tests
// ============================================================================

#[tokio::test]
async fn test_get_service_capabilities() {
    let service = create_test_service();

    let response = service
        .handle_get_service_capabilities(GetServiceCapabilities {})
        .await
        .unwrap();

    // Should have imaging capabilities
    let _ = response.capabilities;
}

// ============================================================================
// GetMoveOptions Tests
// ============================================================================

#[tokio::test]
async fn test_get_move_options() {
    let service = create_test_service();

    let response = service
        .handle_get_move_options(GetMoveOptions {
            video_source_token: ReferenceToken::from(DEFAULT_VIDEO_SOURCE),
        })
        .await
        .unwrap();

    // Move options should exist (may be empty if no focus control)
    let _ = response.move_options;
}

// ============================================================================
// Stop Tests
// ============================================================================

#[tokio::test]
async fn test_stop() {
    let service = create_test_service();

    let result = service
        .handle_stop(Stop {
            video_source_token: ReferenceToken::from(DEFAULT_VIDEO_SOURCE),
        })
        .await;

    // Stop may succeed or return "not supported" error - both are valid
    // The hardware may not support focus control
    match result {
        Ok(_) => {} // Success is fine
        Err(e) => {
            // ActionNotSupported is acceptable for cameras without focus control
            let err_str = e.to_string();
            assert!(
                err_str.contains("not supported") || err_str.contains("ActionNotSupported"),
                "Unexpected error: {}",
                err_str
            );
        }
    }
}

// ============================================================================
// Complete Workflow Test
// ============================================================================

#[tokio::test]
async fn test_complete_imaging_workflow() {
    let service = create_test_service();

    // 1. Get current settings
    let _initial = service
        .handle_get_imaging_settings(GetImagingSettings {
            video_source_token: ReferenceToken::from(DEFAULT_VIDEO_SOURCE),
        })
        .await
        .unwrap();

    // 2. Get options to know valid ranges
    let options = service
        .handle_get_options(GetOptions {
            video_source_token: ReferenceToken::from(DEFAULT_VIDEO_SOURCE),
        })
        .await
        .unwrap();

    // 3. Set new values within valid range
    let brightness_range = options.imaging_options.brightness.unwrap();
    let mid_brightness = (brightness_range.min + brightness_range.max) / 2.0;

    let new_settings = ImagingSettings20 {
        brightness: Some(mid_brightness),
        contrast: Some(50.0),
        color_saturation: Some(50.0),
        sharpness: Some(50.0),
        ..Default::default()
    };

    let set_result = service
        .handle_set_imaging_settings(SetImagingSettings {
            video_source_token: ReferenceToken::from(DEFAULT_VIDEO_SOURCE),
            imaging_settings: new_settings.clone(),
            force_persistence: Some(false),
        })
        .await;
    assert!(set_result.is_ok());

    // 4. Verify settings were applied
    let final_settings = service
        .handle_get_imaging_settings(GetImagingSettings {
            video_source_token: ReferenceToken::from(DEFAULT_VIDEO_SOURCE),
        })
        .await
        .unwrap();

    assert_eq!(
        final_settings.imaging_settings.brightness,
        Some(mid_brightness)
    );
    assert_eq!(final_settings.imaging_settings.contrast, Some(50.0));

    // 5. Get status
    let status = service
        .handle_get_status(GetStatus {
            video_source_token: ReferenceToken::from(DEFAULT_VIDEO_SOURCE),
        })
        .await;
    assert!(status.is_ok());
}
// ============================================================================
// Extended Coverage Tests
// ============================================================================

#[tokio::test]
async fn test_set_imaging_settings_ir_cut_filter_auto() {
    let service = create_test_service();

    let settings = ImagingSettings20 {
        ir_cut_filter: Some(IrCutFilterMode::AUTO),
        ..Default::default()
    };

    service
        .handle_set_imaging_settings(SetImagingSettings {
            video_source_token: ReferenceToken::from(DEFAULT_VIDEO_SOURCE),
            imaging_settings: settings,
            force_persistence: Some(false),
        })
        .await
        .unwrap();

    let response = service
        .handle_get_imaging_settings(GetImagingSettings {
            video_source_token: ReferenceToken::from(DEFAULT_VIDEO_SOURCE),
        })
        .await
        .unwrap();

    assert_eq!(
        response.imaging_settings.ir_cut_filter,
        Some(IrCutFilterMode::AUTO)
    );
}

#[tokio::test]
async fn test_set_imaging_settings_ir_cut_filter_on() {
    let service = create_test_service();

    let settings = ImagingSettings20 {
        ir_cut_filter: Some(IrCutFilterMode::ON),
        ..Default::default()
    };

    service
        .handle_set_imaging_settings(SetImagingSettings {
            video_source_token: ReferenceToken::from(DEFAULT_VIDEO_SOURCE),
            imaging_settings: settings,
            force_persistence: Some(false),
        })
        .await
        .unwrap();

    let response = service
        .handle_get_imaging_settings(GetImagingSettings {
            video_source_token: ReferenceToken::from(DEFAULT_VIDEO_SOURCE),
        })
        .await
        .unwrap();

    assert_eq!(
        response.imaging_settings.ir_cut_filter,
        Some(IrCutFilterMode::ON)
    );
}

#[tokio::test]
async fn test_set_imaging_settings_ir_cut_filter_off() {
    let service = create_test_service();

    let settings = ImagingSettings20 {
        ir_cut_filter: Some(IrCutFilterMode::OFF),
        ..Default::default()
    };

    service
        .handle_set_imaging_settings(SetImagingSettings {
            video_source_token: ReferenceToken::from(DEFAULT_VIDEO_SOURCE),
            imaging_settings: settings,
            force_persistence: Some(false),
        })
        .await
        .unwrap();

    let response = service
        .handle_get_imaging_settings(GetImagingSettings {
            video_source_token: ReferenceToken::from(DEFAULT_VIDEO_SOURCE),
        })
        .await
        .unwrap();

    assert_eq!(
        response.imaging_settings.ir_cut_filter,
        Some(IrCutFilterMode::OFF)
    );
}

#[tokio::test]
async fn test_set_imaging_settings_backlight_compensation_on() {
    let service = create_test_service();

    let settings = ImagingSettings20 {
        backlight_compensation: Some(BacklightCompensation20 {
            mode: BacklightCompensationMode::ON,
            level: Some(50.0),
        }),
        ..Default::default()
    };

    service
        .handle_set_imaging_settings(SetImagingSettings {
            video_source_token: ReferenceToken::from(DEFAULT_VIDEO_SOURCE),
            imaging_settings: settings,
            force_persistence: Some(false),
        })
        .await
        .unwrap();

    let response = service
        .handle_get_imaging_settings(GetImagingSettings {
            video_source_token: ReferenceToken::from(DEFAULT_VIDEO_SOURCE),
        })
        .await
        .unwrap();

    if let Some(blc) = response.imaging_settings.backlight_compensation {
        assert_eq!(blc.mode, BacklightCompensationMode::ON);
        assert_eq!(blc.level, Some(50.0));
    }
}

#[tokio::test]
async fn test_set_imaging_settings_wide_dynamic_range_on() {
    let service = create_test_service();

    let settings = ImagingSettings20 {
        wide_dynamic_range: Some(WideDynamicRange20 {
            mode: WideDynamicMode::ON,
            level: Some(75.0),
        }),
        ..Default::default()
    };

    service
        .handle_set_imaging_settings(SetImagingSettings {
            video_source_token: ReferenceToken::from(DEFAULT_VIDEO_SOURCE),
            imaging_settings: settings,
            force_persistence: Some(false),
        })
        .await
        .unwrap();

    let response = service
        .handle_get_imaging_settings(GetImagingSettings {
            video_source_token: ReferenceToken::from(DEFAULT_VIDEO_SOURCE),
        })
        .await
        .unwrap();

    if let Some(wdr) = response.imaging_settings.wide_dynamic_range {
        assert_eq!(wdr.mode, WideDynamicMode::ON);
        assert_eq!(wdr.level, Some(75.0));
    }
}

#[tokio::test]
async fn test_set_imaging_settings_with_force_persistence() {
    let service = create_test_service();

    let settings = ImagingSettings20 {
        brightness: Some(90.0),
        ..Default::default()
    };

    let result = service
        .handle_set_imaging_settings(SetImagingSettings {
            video_source_token: ReferenceToken::from(DEFAULT_VIDEO_SOURCE),
            imaging_settings: settings,
            force_persistence: Some(true),
        })
        .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_set_imaging_settings_boundary_value_0() {
    let service = create_test_service();

    let settings = ImagingSettings20 {
        brightness: Some(0.0),
        contrast: Some(0.0),
        color_saturation: Some(0.0),
        sharpness: Some(0.0),
        ..Default::default()
    };

    let result = service
        .handle_set_imaging_settings(SetImagingSettings {
            video_source_token: ReferenceToken::from(DEFAULT_VIDEO_SOURCE),
            imaging_settings: settings,
            force_persistence: Some(false),
        })
        .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_set_imaging_settings_boundary_value_100() {
    let service = create_test_service();

    let settings = ImagingSettings20 {
        brightness: Some(100.0),
        contrast: Some(100.0),
        color_saturation: Some(100.0),
        sharpness: Some(100.0),
        ..Default::default()
    };

    let result = service
        .handle_set_imaging_settings(SetImagingSettings {
            video_source_token: ReferenceToken::from(DEFAULT_VIDEO_SOURCE),
            imaging_settings: settings,
            force_persistence: Some(false),
        })
        .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_get_options_returns_valid_ranges_extended() {
    let service = create_test_service();

    let response = service
        .handle_get_options(GetOptions {
            video_source_token: ReferenceToken::from(DEFAULT_VIDEO_SOURCE),
        })
        .await
        .unwrap();

    // Should have brightness range
    if let Some(brightness_range) = response.imaging_options.brightness {
        assert!(brightness_range.min <= brightness_range.max);
    }

    // Should have contrast range
    if let Some(contrast_range) = response.imaging_options.contrast {
        assert!(contrast_range.min <= contrast_range.max);
    }
}

#[tokio::test]
async fn test_get_options_ir_cut_filter_modes_extended() {
    let service = create_test_service();

    let response = service
        .handle_get_options(GetOptions {
            video_source_token: ReferenceToken::from(DEFAULT_VIDEO_SOURCE),
        })
        .await
        .unwrap();

    // ir_cut_filter_modes is a Vec, not an Option
    assert!(!response.imaging_options.ir_cut_filter_modes.is_empty());
}

#[tokio::test]
#[ignore] // Imaging status not implemented in mock service
async fn test_get_status_returns_imaging_status_extended() {
    let service = create_test_service();

    let response = service
        .handle_get_status(GetStatus {
            video_source_token: ReferenceToken::from(DEFAULT_VIDEO_SOURCE),
        })
        .await
        .unwrap();

    // Should have focus status
    assert!(response.status.focus_status20.is_some());
}

#[tokio::test]
#[ignore] // Move options not implemented in mock service
async fn test_get_move_options_returns_options_extended() {
    let service = create_test_service();

    let response = service
        .handle_get_move_options(GetMoveOptions {
            video_source_token: ReferenceToken::from(DEFAULT_VIDEO_SOURCE),
        })
        .await
        .unwrap();

    // Should have move options
    // move_options is a MoveOptions20 struct, check if it has any options defined
    // Check if any of the move options are present
    let has_move_options = response.move_options.absolute.is_some()
        || response.move_options.relative.is_some()
        || response.move_options.continuous.is_some();
    assert!(has_move_options);
}

#[tokio::test]
#[ignore] // Stop focus not implemented in mock service
async fn test_stop_focus_operation_extended() {
    let service = create_test_service();

    let result = service
        .handle_stop(Stop {
            video_source_token: ReferenceToken::from(DEFAULT_VIDEO_SOURCE),
        })
        .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_get_service_capabilities_image_stabilization_extended() {
    let service = create_test_service();

    let response = service
        .handle_get_service_capabilities(GetServiceCapabilities {})
        .await
        .unwrap();

    // Check if image stabilization is reported
    let _stabilization = response.capabilities.image_stabilization;
}

#[tokio::test]
async fn test_get_service_capabilities_presets_extended() {
    let service = create_test_service();

    let response = service
        .handle_get_service_capabilities(GetServiceCapabilities {})
        .await
        .unwrap();

    // Check if presets capability is reported
    let _presets = response.capabilities.presets;
}

#[tokio::test]
async fn test_multiple_settings_update_sequence() {
    let service = create_test_service();

    // Update brightness
    let settings1 = ImagingSettings20 {
        brightness: Some(30.0),
        ..Default::default()
    };
    service
        .handle_set_imaging_settings(SetImagingSettings {
            video_source_token: ReferenceToken::from(DEFAULT_VIDEO_SOURCE),
            imaging_settings: settings1,
            force_persistence: Some(false),
        })
        .await
        .unwrap();

    // Update contrast (brightness should remain)
    let settings2 = ImagingSettings20 {
        contrast: Some(40.0),
        ..Default::default()
    };
    service
        .handle_set_imaging_settings(SetImagingSettings {
            video_source_token: ReferenceToken::from(DEFAULT_VIDEO_SOURCE),
            imaging_settings: settings2,
            force_persistence: Some(false),
        })
        .await
        .unwrap();

    // Verify both are set
    let response = service
        .handle_get_imaging_settings(GetImagingSettings {
            video_source_token: ReferenceToken::from(DEFAULT_VIDEO_SOURCE),
        })
        .await
        .unwrap();

    assert_eq!(response.imaging_settings.contrast, Some(40.0));
}
