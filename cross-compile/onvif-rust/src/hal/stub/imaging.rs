//! Stub imaging HAL implementation for host-side testing.

use crate::hal::common::AK_SUCCESS_I32;
use crate::hal::common::imaging::ImagingHalTrait;

/// Stub implementation that returns success for all imaging operations.
#[allow(dead_code)] // Used on host targets only
pub(crate) struct StubImagingHal;

impl ImagingHalTrait for StubImagingHal {
    fn set_brightness(&self, _value: i32) -> i32 {
        AK_SUCCESS_I32
    }

    fn set_contrast(&self, _value: i32) -> i32 {
        AK_SUCCESS_I32
    }

    fn set_saturation(&self, _value: i32) -> i32 {
        AK_SUCCESS_I32
    }

    fn set_sharpness(&self, _value: i32) -> i32 {
        AK_SUCCESS_I32
    }

    fn set_ir_filter(&self, _enabled: bool) -> i32 {
        AK_SUCCESS_I32
    }

    fn set_wdr(&self, _enabled: bool) -> i32 {
        AK_SUCCESS_I32
    }
}

#[cfg(all(test, use_stubs))]
mod tests {
    use super::*;
    use crate::hal::common::imaging::{
        imaging_set_brightness, imaging_set_contrast, imaging_set_ir_filter,
        imaging_set_saturation, imaging_set_sharpness, imaging_set_wdr,
    };

    #[test]
    fn test_imaging_set_brightness_success() {
        let stub = StubImagingHal;
        let result = imaging_set_brightness(50.0, &stub);
        assert!(result.is_ok());
    }

    #[test]
    fn test_imaging_set_contrast_success() {
        let stub = StubImagingHal;
        let result = imaging_set_contrast(75.0, &stub);
        assert!(result.is_ok());
    }

    #[test]
    fn test_imaging_set_saturation_success() {
        let stub = StubImagingHal;
        let result = imaging_set_saturation(60.0, &stub);
        assert!(result.is_ok());
    }

    #[test]
    fn test_imaging_set_sharpness_success() {
        let stub = StubImagingHal;
        let result = imaging_set_sharpness(80.0, &stub);
        assert!(result.is_ok());
    }

    #[test]
    fn test_imaging_set_ir_filter_success() {
        let stub = StubImagingHal;
        let result = imaging_set_ir_filter(true, &stub);
        assert!(result.is_ok());
    }

    #[test]
    fn test_imaging_set_wdr_success() {
        let stub = StubImagingHal;
        let result = imaging_set_wdr(false, &stub);
        assert!(result.is_ok());
    }
}
