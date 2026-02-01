//! Safe Rust wrappers for Anyka SDK imaging functions.
//!
//! This module provides safe wrappers around the Anyka SDK imaging/ISP functions
//! for controlling image quality parameters such as brightness, contrast, saturation,
//! sharpness, IR filter, and WDR settings.
//!
//! # Parameter Mapping
//!
//! ONVIF uses a 0.0-100.0 range for imaging parameters, while the SDK typically uses
//! register values (e.g., 0-255). This module provides conversion functions to map
//! between these ranges.
//!
//! # Error Handling
//!
//! All functions return `Result<T, PlatformError>`, converting SDK error codes
//! (`AK_SUCCESS`/`AK_FAILED`) into appropriate `PlatformError` variants.

use crate::platform::PlatformError;
use crate::platform::PlatformResult;

use crate::ffi::{AK_FAILED_I32, AK_SUCCESS_I32};

/// Default maximum value for SDK imaging parameters (typically 255 for 8-bit registers).
const SDK_MAX_VALUE: i32 = 255;

/// Minimum ONVIF imaging parameter value.
const ONVIF_MIN: f32 = 0.0;

/// Maximum ONVIF imaging parameter value.
const ONVIF_MAX: f32 = 100.0;

/// Internal trait for abstracting imaging FFI calls to enable mocking in tests.
#[cfg_attr(test, mockall::automock)]
pub(crate) trait ImagingFfiTrait: Send + Sync {
    fn set_brightness(&self, value: i32) -> i32;
    fn set_contrast(&self, value: i32) -> i32;
    fn set_saturation(&self, value: i32) -> i32;
    fn set_sharpness(&self, value: i32) -> i32;
    fn set_ir_filter(&self, enabled: bool) -> i32;
    fn set_wdr(&self, enabled: bool) -> i32;
}

/// Default implementation that calls the real FFI functions.
pub(crate) struct RealImagingFfi;

impl ImagingFfiTrait for RealImagingFfi {
    #[cfg(not(use_stubs))]
    fn set_brightness(&self, value: i32) -> i32 {
        unsafe extern "C" {
            fn ak_isp_set_brightness(value: i32) -> i32;
        }
        unsafe { ak_isp_set_brightness(value) }
    }

    #[cfg(use_stubs)]
    fn set_brightness(&self, _value: i32) -> i32 {
        AK_SUCCESS_I32
    }

    #[cfg(not(use_stubs))]
    fn set_contrast(&self, value: i32) -> i32 {
        unsafe extern "C" {
            fn ak_isp_set_contrast(value: i32) -> i32;
        }
        unsafe { ak_isp_set_contrast(value) }
    }

    #[cfg(use_stubs)]
    fn set_contrast(&self, _value: i32) -> i32 {
        AK_SUCCESS_I32
    }

    #[cfg(not(use_stubs))]
    fn set_saturation(&self, value: i32) -> i32 {
        unsafe extern "C" {
            fn ak_isp_set_saturation(value: i32) -> i32;
        }
        unsafe { ak_isp_set_saturation(value) }
    }

    #[cfg(use_stubs)]
    fn set_saturation(&self, _value: i32) -> i32 {
        AK_SUCCESS_I32
    }

    #[cfg(not(use_stubs))]
    fn set_sharpness(&self, value: i32) -> i32 {
        unsafe extern "C" {
            fn ak_isp_set_sharpness(value: i32) -> i32;
        }
        unsafe { ak_isp_set_sharpness(value) }
    }

    #[cfg(use_stubs)]
    fn set_sharpness(&self, _value: i32) -> i32 {
        AK_SUCCESS_I32
    }

    #[cfg(not(use_stubs))]
    fn set_ir_filter(&self, enabled: bool) -> i32 {
        unsafe extern "C" {
            fn ak_isp_set_ir_filter(enabled: i32) -> i32;
        }
        unsafe { ak_isp_set_ir_filter(if enabled { 1 } else { 0 }) }
    }

    #[cfg(use_stubs)]
    fn set_ir_filter(&self, _enabled: bool) -> i32 {
        AK_SUCCESS_I32
    }

    #[cfg(not(use_stubs))]
    fn set_wdr(&self, enabled: bool) -> i32 {
        unsafe extern "C" {
            fn ak_isp_set_wdr(enabled: i32) -> i32;
        }
        unsafe { ak_isp_set_wdr(if enabled { 1 } else { 0 }) }
    }

    #[cfg(use_stubs)]
    fn set_wdr(&self, _enabled: bool) -> i32 {
        AK_SUCCESS_I32
    }
}

// Global instance for default FFI implementation
static REAL_IMAGING_FFI: RealImagingFfi = RealImagingFfi;

/// Helper function to convert SDK return codes to PlatformResult.
///
/// # Arguments
///
/// * `ret` - SDK return code (0 = AK_SUCCESS, -1 = AK_FAILED, or other error codes)
/// * `context` - Context string for error messages
///
/// # Returns
///
/// * `Ok(())` if `ret == AK_SUCCESS`
/// * `Err(PlatformError::HardwareFailure(...))` otherwise
fn check_result(ret: i32, context: &str) -> PlatformResult<()> {
    match ret {
        AK_SUCCESS_I32 => Ok(()),
        AK_FAILED_I32 => Err(PlatformError::HardwareFailure(format!(
            "{} failed",
            context
        ))),
        _ => Err(PlatformError::HardwareFailure(format!(
            "{}: error code {}",
            context, ret
        ))),
    }
}

/// Validate ONVIF imaging parameter range (0.0-100.0).
///
/// # Arguments
///
/// * `value` - Parameter value to validate
/// * `param_name` - Name of the parameter for error messages
///
/// # Returns
///
/// * `Ok(())` if value is within valid range
/// * `Err(PlatformError::InvalidParameter)` if out of range
pub fn validate_onvif_range(value: f32, param_name: &str) -> PlatformResult<()> {
    if !(ONVIF_MIN..=ONVIF_MAX).contains(&value) {
        Err(PlatformError::InvalidParameter(format!(
            "{} value {} is out of range ({:.1} to {:.1})",
            param_name, value, ONVIF_MIN, ONVIF_MAX
        )))
    } else {
        Ok(())
    }
}

/// Convert ONVIF parameter value (0.0-100.0) to SDK register value.
///
/// # Arguments
///
/// * `onvif_value` - ONVIF parameter value (0.0-100.0)
/// * `sdk_max` - Maximum SDK register value (default: 255)
///
/// # Returns
///
/// SDK register value (0 to sdk_max)
pub fn onvif_to_sdk_brightness(onvif_value: f32) -> i32 {
    onvif_to_sdk_value(onvif_value, SDK_MAX_VALUE)
}

/// Convert ONVIF parameter value (0.0-100.0) to SDK register value.
///
/// # Arguments
///
/// * `onvif_value` - ONVIF parameter value (0.0-100.0)
/// * `sdk_max` - Maximum SDK register value (default: 255)
///
/// # Returns
///
/// SDK register value (0 to sdk_max)
pub fn onvif_to_sdk_contrast(onvif_value: f32) -> i32 {
    onvif_to_sdk_value(onvif_value, SDK_MAX_VALUE)
}

/// Convert ONVIF parameter value (0.0-100.0) to SDK register value.
///
/// # Arguments
///
/// * `onvif_value` - ONVIF parameter value (0.0-100.0)
/// * `sdk_max` - Maximum SDK register value (default: 255)
///
/// # Returns
///
/// SDK register value (0 to sdk_max)
pub fn onvif_to_sdk_saturation(onvif_value: f32) -> i32 {
    onvif_to_sdk_value(onvif_value, SDK_MAX_VALUE)
}

/// Convert ONVIF parameter value (0.0-100.0) to SDK register value.
///
/// # Arguments
///
/// * `onvif_value` - ONVIF parameter value (0.0-100.0)
/// * `sdk_max` - Maximum SDK register value (default: 255)
///
/// # Returns
///
/// SDK register value (0 to sdk_max)
pub fn onvif_to_sdk_sharpness(onvif_value: f32) -> i32 {
    onvif_to_sdk_value(onvif_value, SDK_MAX_VALUE)
}

/// Generic function to convert ONVIF value to SDK register value.
///
/// # Arguments
///
/// * `onvif_value` - ONVIF parameter value (0.0-100.0)
/// * `sdk_max` - Maximum SDK register value
///
/// # Returns
///
/// SDK register value (0 to sdk_max)
fn onvif_to_sdk_value(onvif_value: f32, sdk_max: i32) -> i32 {
    ((onvif_value / ONVIF_MAX) * sdk_max as f32).round() as i32
}

/// Internal helper that takes FFI trait for testability.
pub(crate) fn imaging_set_brightness_internal(
    value: f32,
    ffi: &dyn ImagingFfiTrait,
) -> PlatformResult<()> {
    validate_onvif_range(value, "brightness")?;
    let sdk_value = onvif_to_sdk_brightness(value);
    let ret = ffi.set_brightness(sdk_value);
    check_result(ret, "imaging_set_brightness")
}

/// Set brightness.
///
/// # Arguments
///
/// * `value` - Brightness value (0.0 to 100.0)
///
/// # Returns
///
/// * `Ok(())` on success
/// * `Err(PlatformError::InvalidParameter)` if value is out of range
/// * `Err(PlatformError::HardwareFailure)` on SDK error
pub fn imaging_set_brightness(value: f32) -> PlatformResult<()> {
    imaging_set_brightness_internal(value, &REAL_IMAGING_FFI)
}

/// Internal helper that takes FFI trait for testability.
pub(crate) fn imaging_set_contrast_internal(
    value: f32,
    ffi: &dyn ImagingFfiTrait,
) -> PlatformResult<()> {
    validate_onvif_range(value, "contrast")?;
    let sdk_value = onvif_to_sdk_contrast(value);
    let ret = ffi.set_contrast(sdk_value);
    check_result(ret, "imaging_set_contrast")
}

/// Set contrast.
///
/// # Arguments
///
/// * `value` - Contrast value (0.0 to 100.0)
///
/// # Returns
///
/// * `Ok(())` on success
/// * `Err(PlatformError::InvalidParameter)` if value is out of range
/// * `Err(PlatformError::HardwareFailure)` on SDK error
pub fn imaging_set_contrast(value: f32) -> PlatformResult<()> {
    imaging_set_contrast_internal(value, &REAL_IMAGING_FFI)
}

/// Internal helper that takes FFI trait for testability.
pub(crate) fn imaging_set_saturation_internal(
    value: f32,
    ffi: &dyn ImagingFfiTrait,
) -> PlatformResult<()> {
    validate_onvif_range(value, "saturation")?;
    let sdk_value = onvif_to_sdk_saturation(value);
    let ret = ffi.set_saturation(sdk_value);
    check_result(ret, "imaging_set_saturation")
}

/// Set saturation (color saturation).
///
/// # Arguments
///
/// * `value` - Saturation value (0.0 to 100.0)
///
/// # Returns
///
/// * `Ok(())` on success
/// * `Err(PlatformError::InvalidParameter)` if value is out of range
/// * `Err(PlatformError::HardwareFailure)` on SDK error
pub fn imaging_set_saturation(value: f32) -> PlatformResult<()> {
    imaging_set_saturation_internal(value, &REAL_IMAGING_FFI)
}

/// Internal helper that takes FFI trait for testability.
pub(crate) fn imaging_set_sharpness_internal(
    value: f32,
    ffi: &dyn ImagingFfiTrait,
) -> PlatformResult<()> {
    validate_onvif_range(value, "sharpness")?;
    let sdk_value = onvif_to_sdk_sharpness(value);
    let ret = ffi.set_sharpness(sdk_value);
    check_result(ret, "imaging_set_sharpness")
}

/// Set sharpness.
///
/// # Arguments
///
/// * `value` - Sharpness value (0.0 to 100.0)
///
/// # Returns
///
/// * `Ok(())` on success
/// * `Err(PlatformError::InvalidParameter)` if value is out of range
/// * `Err(PlatformError::HardwareFailure)` on SDK error
pub fn imaging_set_sharpness(value: f32) -> PlatformResult<()> {
    imaging_set_sharpness_internal(value, &REAL_IMAGING_FFI)
}

/// Internal helper that takes FFI trait for testability.
pub(crate) fn imaging_set_ir_filter_internal(
    enabled: bool,
    ffi: &dyn ImagingFfiTrait,
) -> PlatformResult<()> {
    let ret = ffi.set_ir_filter(enabled);
    check_result(ret, "imaging_set_ir_filter")
}

/// Set IR cut filter state.
///
/// # Arguments
///
/// * `enabled` - Whether to enable the IR cut filter
///
/// # Returns
///
/// * `Ok(())` on success
/// * `Err(PlatformError::HardwareFailure)` on SDK error
pub fn imaging_set_ir_filter(enabled: bool) -> PlatformResult<()> {
    imaging_set_ir_filter_internal(enabled, &REAL_IMAGING_FFI)
}

/// Internal helper that takes FFI trait for testability.
pub(crate) fn imaging_set_wdr_internal(
    enabled: bool,
    ffi: &dyn ImagingFfiTrait,
) -> PlatformResult<()> {
    let ret = ffi.set_wdr(enabled);
    check_result(ret, "imaging_set_wdr")
}

/// Set Wide Dynamic Range (WDR) state.
///
/// # Arguments
///
/// * `enabled` - Whether to enable WDR
///
/// # Returns
///
/// * `Ok(())` on success
/// * `Err(PlatformError::HardwareFailure)` on SDK error
pub fn imaging_set_wdr(enabled: bool) -> PlatformResult<()> {
    imaging_set_wdr_internal(enabled, &REAL_IMAGING_FFI)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;

    #[test]
    fn test_check_result_success() {
        let result = check_result(AK_SUCCESS_I32, "test_function");
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_result_failed() {
        let result = check_result(AK_FAILED_I32, "test_function");
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareFailure(msg)) => {
                assert!(msg.contains("test_function"));
                assert!(msg.contains("failed"));
            }
            _ => panic!("Expected HardwareFailure error"),
        }
    }

    #[test]
    fn test_validate_onvif_range_valid() {
        assert!(validate_onvif_range(0.0, "test").is_ok());
        assert!(validate_onvif_range(50.0, "test").is_ok());
        assert!(validate_onvif_range(100.0, "test").is_ok());
    }

    #[test]
    fn test_validate_onvif_range_invalid() {
        assert!(validate_onvif_range(-1.0, "brightness").is_err());
        assert!(validate_onvif_range(101.0, "contrast").is_err());
        match validate_onvif_range(150.0, "saturation") {
            Err(PlatformError::InvalidParameter(msg)) => {
                assert!(msg.contains("saturation"));
                assert!(msg.contains("out of range"));
            }
            _ => panic!("Expected InvalidParameter error"),
        }
    }

    #[test]
    fn test_onvif_to_sdk_brightness() {
        // 0.0 should map to 0
        assert_eq!(onvif_to_sdk_brightness(0.0), 0);
        // 100.0 should map to 255
        assert_eq!(onvif_to_sdk_brightness(100.0), 255);
        // 50.0 should map to approximately 128
        assert_eq!(onvif_to_sdk_brightness(50.0), 128);
    }

    #[test]
    fn test_onvif_to_sdk_contrast() {
        assert_eq!(onvif_to_sdk_contrast(0.0), 0);
        assert_eq!(onvif_to_sdk_contrast(100.0), 255);
        assert_eq!(onvif_to_sdk_contrast(50.0), 128);
    }

    #[test]
    fn test_onvif_to_sdk_saturation() {
        assert_eq!(onvif_to_sdk_saturation(0.0), 0);
        assert_eq!(onvif_to_sdk_saturation(100.0), 255);
        assert_eq!(onvif_to_sdk_saturation(50.0), 128);
    }

    #[test]
    fn test_onvif_to_sdk_sharpness() {
        assert_eq!(onvif_to_sdk_sharpness(0.0), 0);
        assert_eq!(onvif_to_sdk_sharpness(100.0), 255);
        assert_eq!(onvif_to_sdk_sharpness(50.0), 128);
    }

    #[test]
    fn test_imaging_set_brightness_internal_calls_ffi() {
        let mut mock_ffi = MockImagingFfiTrait::new();

        mock_ffi
            .expect_set_brightness()
            .with(eq(128)) // 50.0 maps to 128
            .times(1)
            .returning(|_| AK_SUCCESS_I32);

        let result = imaging_set_brightness_internal(50.0, &mock_ffi);
        assert!(result.is_ok());
    }

    #[test]
    fn test_imaging_set_brightness_internal_validates_range() {
        let mock_ffi = MockImagingFfiTrait::new();

        // Should fail validation before calling FFI
        let result = imaging_set_brightness_internal(150.0, &mock_ffi);
        assert!(result.is_err());
        match result {
            Err(PlatformError::InvalidParameter(msg)) => {
                assert!(msg.contains("brightness"));
            }
            _ => panic!("Expected InvalidParameter error"),
        }
    }

    #[test]
    fn test_imaging_set_brightness_internal_propagates_error() {
        let mut mock_ffi = MockImagingFfiTrait::new();

        mock_ffi
            .expect_set_brightness()
            .times(1)
            .returning(|_| AK_FAILED_I32);

        let result = imaging_set_brightness_internal(50.0, &mock_ffi);
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareFailure(msg)) => {
                assert!(msg.contains("imaging_set_brightness"));
            }
            _ => panic!("Expected HardwareFailure error"),
        }
    }

    #[test]
    fn test_imaging_set_contrast_internal_calls_ffi() {
        let mut mock_ffi = MockImagingFfiTrait::new();

        mock_ffi
            .expect_set_contrast()
            .with(eq(255)) // 100.0 maps to 255
            .times(1)
            .returning(|_| AK_SUCCESS_I32);

        let result = imaging_set_contrast_internal(100.0, &mock_ffi);
        assert!(result.is_ok());
    }

    #[test]
    fn test_imaging_set_saturation_internal_calls_ffi() {
        let mut mock_ffi = MockImagingFfiTrait::new();

        mock_ffi
            .expect_set_saturation()
            .with(eq(0)) // 0.0 maps to 0
            .times(1)
            .returning(|_| AK_SUCCESS_I32);

        let result = imaging_set_saturation_internal(0.0, &mock_ffi);
        assert!(result.is_ok());
    }

    #[test]
    fn test_imaging_set_sharpness_internal_calls_ffi() {
        let mut mock_ffi = MockImagingFfiTrait::new();

        mock_ffi
            .expect_set_sharpness()
            .with(eq(64)) // 25.0 maps to approximately 64
            .times(1)
            .returning(|_| AK_SUCCESS_I32);

        let result = imaging_set_sharpness_internal(25.0, &mock_ffi);
        assert!(result.is_ok());
    }

    #[test]
    fn test_imaging_set_ir_filter_internal_calls_ffi_enabled() {
        let mut mock_ffi = MockImagingFfiTrait::new();

        mock_ffi
            .expect_set_ir_filter()
            .with(eq(true))
            .times(1)
            .returning(|_| AK_SUCCESS_I32);

        let result = imaging_set_ir_filter_internal(true, &mock_ffi);
        assert!(result.is_ok());
    }

    #[test]
    fn test_imaging_set_ir_filter_internal_calls_ffi_disabled() {
        let mut mock_ffi = MockImagingFfiTrait::new();

        mock_ffi
            .expect_set_ir_filter()
            .with(eq(false))
            .times(1)
            .returning(|_| AK_SUCCESS_I32);

        let result = imaging_set_ir_filter_internal(false, &mock_ffi);
        assert!(result.is_ok());
    }

    #[test]
    fn test_imaging_set_ir_filter_internal_propagates_error() {
        let mut mock_ffi = MockImagingFfiTrait::new();

        mock_ffi
            .expect_set_ir_filter()
            .times(1)
            .returning(|_| AK_FAILED_I32);

        let result = imaging_set_ir_filter_internal(true, &mock_ffi);
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareFailure(msg)) => {
                assert!(msg.contains("imaging_set_ir_filter"));
            }
            _ => panic!("Expected HardwareFailure error"),
        }
    }

    #[test]
    fn test_imaging_set_wdr_internal_calls_ffi_enabled() {
        let mut mock_ffi = MockImagingFfiTrait::new();

        mock_ffi
            .expect_set_wdr()
            .with(eq(true))
            .times(1)
            .returning(|_| AK_SUCCESS_I32);

        let result = imaging_set_wdr_internal(true, &mock_ffi);
        assert!(result.is_ok());
    }

    #[test]
    fn test_imaging_set_wdr_internal_calls_ffi_disabled() {
        let mut mock_ffi = MockImagingFfiTrait::new();

        mock_ffi
            .expect_set_wdr()
            .with(eq(false))
            .times(1)
            .returning(|_| AK_SUCCESS_I32);

        let result = imaging_set_wdr_internal(false, &mock_ffi);
        assert!(result.is_ok());
    }

    #[test]
    fn test_imaging_set_wdr_internal_propagates_error() {
        let mut mock_ffi = MockImagingFfiTrait::new();

        mock_ffi
            .expect_set_wdr()
            .times(1)
            .returning(|_| AK_FAILED_I32);

        let result = imaging_set_wdr_internal(true, &mock_ffi);
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareFailure(msg)) => {
                assert!(msg.contains("imaging_set_wdr"));
            }
            _ => panic!("Expected HardwareFailure error"),
        }
    }

    #[test]
    #[cfg(use_stubs)]
    fn test_imaging_set_brightness_success() {
        let result = imaging_set_brightness(50.0);
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(use_stubs)]
    fn test_imaging_set_contrast_success() {
        let result = imaging_set_contrast(75.0);
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(use_stubs)]
    fn test_imaging_set_saturation_success() {
        let result = imaging_set_saturation(60.0);
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(use_stubs)]
    fn test_imaging_set_sharpness_success() {
        let result = imaging_set_sharpness(80.0);
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(use_stubs)]
    fn test_imaging_set_ir_filter_success() {
        let result = imaging_set_ir_filter(true);
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(use_stubs)]
    fn test_imaging_set_wdr_success() {
        let result = imaging_set_wdr(false);
        assert!(result.is_ok());
    }
}
