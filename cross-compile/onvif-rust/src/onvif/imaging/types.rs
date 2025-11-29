//! Imaging Service types re-exported from WSDL-generated types.
//!
//! This module re-exports and extends the WSDL-generated imaging types
//! from `crate::onvif::types::imaging`.

// Re-export all imaging types from the generated module
pub use crate::onvif::types::imaging::*;

// Re-export common types used by imaging service
// These are intentionally re-exported for consumers of this module
#[allow(unused_imports)]
pub use crate::onvif::types::common::{
    FloatRange, ImagingSettings20, ImagingStatus20, ReferenceToken,
};

// ============================================================================
// Service Constants
// ============================================================================

/// Imaging service namespace URI.
pub const IMAGING_SERVICE_NAMESPACE: &str = "http://www.onvif.org/ver20/imaging/wsdl";

/// Default video source token.
pub const DEFAULT_VIDEO_SOURCE_TOKEN: &str = "VideoSource_1";

// ============================================================================
// Validation Helpers
// ============================================================================

/// Validate brightness value is within range.
pub fn validate_brightness(value: f32) -> bool {
    (0.0..=100.0).contains(&value)
}

/// Validate contrast value is within range.
pub fn validate_contrast(value: f32) -> bool {
    (0.0..=100.0).contains(&value)
}

/// Validate saturation value is within range.
pub fn validate_saturation(value: f32) -> bool {
    (0.0..=100.0).contains(&value)
}

/// Validate sharpness value is within range.
pub fn validate_sharpness(value: f32) -> bool {
    (0.0..=100.0).contains(&value)
}

/// Validate a float value is within a given range.
pub fn validate_range(value: f32, min: f32, max: f32) -> bool {
    value >= min && value <= max
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(validate_range(5.0, 0.0, 10.0));
        assert!(validate_range(0.0, 0.0, 10.0));
        assert!(validate_range(10.0, 0.0, 10.0));
        assert!(!validate_range(-1.0, 0.0, 10.0));
        assert!(!validate_range(11.0, 0.0, 10.0));
    }
}
