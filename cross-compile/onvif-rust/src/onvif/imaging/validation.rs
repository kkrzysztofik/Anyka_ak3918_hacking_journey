//! Imaging Service validation utilities.
//!
//! This module provides validation functions for imaging settings
//! and options.

use crate::onvif::error::{OnvifError, OnvifResult};
use crate::onvif::imaging::types::ImagingOptions20;
use crate::onvif::types::common::ImagingSettings20;

/// Validate imaging settings against device-reported option ranges (dynamic validation).
///
/// Each setting value is checked against the corresponding range from
/// `ImagingOptions20`. Settings whose option range is `None` are
/// unconstrained and always accepted.
pub fn validate_settings(
    settings: &ImagingSettings20,
    options: &ImagingOptions20,
) -> OnvifResult<()> {
    let checks = [
        ("Brightness", settings.brightness, &options.brightness),
        ("Contrast", settings.contrast, &options.contrast),
        (
            "ColorSaturation",
            settings.color_saturation,
            &options.color_saturation,
        ),
        ("Sharpness", settings.sharpness, &options.sharpness),
    ];

    for (name, value, range) in checks {
        if let (Some(value), Some(range)) = (value, range.as_ref()) {
            validate_range(name, value, range.min, range.max)?;
        }
    }

    Ok(())
}

/// Validate a single float parameter against a range.
///
/// Rejects non-finite values (NaN, +/-infinity) before the range check.
pub fn validate_range(parameter: &str, value: f32, min: f32, max: f32) -> OnvifResult<()> {
    if !value.is_finite() {
        return Err(OnvifError::invalid_arg(
            "InvalidValue",
            format!("{} must be a finite number", parameter),
        ));
    }
    if value < min || value > max {
        Err(super::faults::parameter_out_of_range(
            parameter, value, min, max,
        ))
    } else {
        Ok(())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::onvif::types::common::FloatRange;

    #[test]
    fn test_validate_range() {
        assert!(validate_range("Test", 5.0, 0.0, 10.0).is_ok());
        assert!(validate_range("Test", 0.0, 0.0, 10.0).is_ok());
        assert!(validate_range("Test", 10.0, 0.0, 10.0).is_ok());
        assert!(validate_range("Test", -1.0, 0.0, 10.0).is_err());
        assert!(validate_range("Test", 11.0, 0.0, 10.0).is_err());
    }

    #[test]
    fn test_validate_range_rejects_nan() {
        assert!(validate_range("Brightness", f32::NAN, 0.0, 100.0).is_err());
    }

    #[test]
    fn test_validate_range_rejects_infinity() {
        assert!(validate_range("Brightness", f32::INFINITY, 0.0, 100.0).is_err());
    }

    #[test]
    fn test_validate_range_rejects_neg_infinity() {
        assert!(validate_range("Brightness", f32::NEG_INFINITY, 0.0, 100.0).is_err());
    }

    #[test]
    fn test_validate_range_accepts_finite_in_range() {
        assert!(validate_range("Brightness", 50.0, 0.0, 100.0).is_ok());
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
