//! Imaging Service validation utilities.
//!
//! This module provides validation functions for imaging settings
//! and options.

use crate::onvif::error::OnvifResult;
use crate::onvif::imaging::types::ImagingOptions20;
use crate::onvif::types::common::ImagingSettings20;

/// Validate imaging settings against options.
///
/// Returns Ok(()) if valid, or an error if any parameter is out of range.
#[allow(clippy::collapsible_if)]
pub fn validate_settings(
    settings: &ImagingSettings20,
    options: &ImagingOptions20,
) -> OnvifResult<()> {
    // Validate brightness
    if let Some(brightness) = settings.brightness {
        if let Some(ref range) = options.brightness {
            if brightness < range.min || brightness > range.max {
                return Err(super::faults::parameter_out_of_range(
                    "Brightness",
                    brightness,
                    range.min,
                    range.max,
                ));
            }
        }
    }

    // Validate contrast
    if let Some(contrast) = settings.contrast {
        if let Some(ref range) = options.contrast {
            if contrast < range.min || contrast > range.max {
                return Err(super::faults::parameter_out_of_range(
                    "Contrast", contrast, range.min, range.max,
                ));
            }
        }
    }

    // Validate color saturation
    if let Some(saturation) = settings.color_saturation {
        if let Some(ref range) = options.color_saturation {
            if saturation < range.min || saturation > range.max {
                return Err(super::faults::parameter_out_of_range(
                    "ColorSaturation",
                    saturation,
                    range.min,
                    range.max,
                ));
            }
        }
    }

    // Validate sharpness
    if let Some(sharpness) = settings.sharpness {
        if let Some(ref range) = options.sharpness {
            if sharpness < range.min || sharpness > range.max {
                return Err(super::faults::parameter_out_of_range(
                    "Sharpness",
                    sharpness,
                    range.min,
                    range.max,
                ));
            }
        }
    }

    Ok(())
}

/// Validate a single float parameter against a range.
pub fn validate_range(parameter: &str, value: f32, min: f32, max: f32) -> OnvifResult<()> {
    if value < min || value > max {
        Err(super::faults::parameter_out_of_range(
            parameter, value, min, max,
        ))
    } else {
        Ok(())
    }
}

/// Validate brightness value is within standard range (0-100).
pub fn validate_brightness(value: f32) -> bool {
    (0.0..=100.0).contains(&value)
}

/// Validate contrast value is within standard range (0-100).
pub fn validate_contrast(value: f32) -> bool {
    (0.0..=100.0).contains(&value)
}

/// Validate saturation value is within standard range (0-100).
pub fn validate_saturation(value: f32) -> bool {
    (0.0..=100.0).contains(&value)
}

/// Validate sharpness value is within standard range (0-100).
pub fn validate_sharpness(value: f32) -> bool {
    (0.0..=100.0).contains(&value)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::onvif::types::common::FloatRange;

    #[test]
    fn test_validate_brightness() {
        assert!(validate_brightness(0.0));
        assert!(validate_brightness(50.0));
        assert!(validate_brightness(100.0));
        assert!(!validate_brightness(-1.0));
        assert!(!validate_brightness(101.0));
    }

    #[test]
    fn test_validate_contrast() {
        assert!(validate_contrast(0.0));
        assert!(validate_contrast(50.0));
        assert!(validate_contrast(100.0));
        assert!(!validate_contrast(-1.0));
        assert!(!validate_contrast(101.0));
    }

    #[test]
    fn test_validate_saturation() {
        assert!(validate_saturation(0.0));
        assert!(validate_saturation(50.0));
        assert!(validate_saturation(100.0));
        assert!(!validate_saturation(-1.0));
        assert!(!validate_saturation(101.0));
    }

    #[test]
    fn test_validate_sharpness() {
        assert!(validate_sharpness(0.0));
        assert!(validate_sharpness(50.0));
        assert!(validate_sharpness(100.0));
        assert!(!validate_sharpness(-1.0));
        assert!(!validate_sharpness(101.0));
    }

    #[test]
    fn test_validate_range() {
        assert!(validate_range("Test", 5.0, 0.0, 10.0).is_ok());
        assert!(validate_range("Test", 0.0, 0.0, 10.0).is_ok());
        assert!(validate_range("Test", 10.0, 0.0, 10.0).is_ok());
        assert!(validate_range("Test", -1.0, 0.0, 10.0).is_err());
        assert!(validate_range("Test", 11.0, 0.0, 10.0).is_err());
    }

    #[test]
    fn test_validate_settings_valid() {
        let settings = ImagingSettings20 {
            brightness: Some(50.0),
            contrast: Some(50.0),
            color_saturation: Some(50.0),
            sharpness: Some(50.0),
            ..Default::default()
        };

        let options = ImagingOptions20 {
            brightness: Some(FloatRange {
                min: 0.0,
                max: 100.0,
            }),
            contrast: Some(FloatRange {
                min: 0.0,
                max: 100.0,
            }),
            color_saturation: Some(FloatRange {
                min: 0.0,
                max: 100.0,
            }),
            sharpness: Some(FloatRange {
                min: 0.0,
                max: 100.0,
            }),
            ..Default::default()
        };

        assert!(validate_settings(&settings, &options).is_ok());
    }

    #[test]
    fn test_validate_settings_out_of_range() {
        let settings = ImagingSettings20 {
            brightness: Some(150.0), // Out of range
            ..Default::default()
        };

        let options = ImagingOptions20 {
            brightness: Some(FloatRange {
                min: 0.0,
                max: 100.0,
            }),
            ..Default::default()
        };

        assert!(validate_settings(&settings, &options).is_err());
    }

    #[test]
    fn test_validate_settings_multiple_params() {
        // All parameters out of range
        let settings = ImagingSettings20 {
            brightness: Some(-10.0),
            contrast: Some(200.0),
            color_saturation: Some(150.0),
            sharpness: Some(-5.0),
            ..Default::default()
        };

        let options = ImagingOptions20 {
            brightness: Some(FloatRange {
                min: 0.0,
                max: 100.0,
            }),
            contrast: Some(FloatRange {
                min: 0.0,
                max: 100.0,
            }),
            color_saturation: Some(FloatRange {
                min: 0.0,
                max: 100.0,
            }),
            sharpness: Some(FloatRange {
                min: 0.0,
                max: 100.0,
            }),
            ..Default::default()
        };

        // Should fail on first out-of-range parameter (brightness)
        assert!(validate_settings(&settings, &options).is_err());
    }
}
