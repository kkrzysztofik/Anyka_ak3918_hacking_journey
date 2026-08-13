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

use async_trait::async_trait;

use crate::platform::PlatformError;
use crate::platform::PlatformResult;

#[cfg(test)]
use super::AK_FAILED_I32;
#[cfg(test)]
use super::AK_SUCCESS_I32;
use super::check_result;

/// Default maximum value for SDK imaging parameters (typically 255 for 8-bit registers).
pub(crate) const SDK_MAX_VALUE: i32 = 255;

/// Minimum ONVIF imaging parameter value.
const ONVIF_MIN: f32 = 0.0;

/// Maximum ONVIF imaging parameter value.
const ONVIF_MAX: f32 = 100.0;

/// Internal trait for abstracting imaging FFI calls to enable mocking in tests.
///
/// Methods are `async` so that IPC-backed implementations (e.g. `AnykaIpc`) can
/// `.await` the control-socket owner thread instead of parking a tokio worker on a
/// blocking RPC. Non-blocking implementations (stubs) simply return immediately.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
#[allow(dead_code)] // Some methods only used on ARM targets
pub(crate) trait ImagingHalTrait: Send + Sync {
    async fn set_brightness(&self, value: i32) -> i32;
    async fn set_contrast(&self, value: i32) -> i32;
    async fn set_saturation(&self, value: i32) -> i32;
    async fn set_sharpness(&self, value: i32) -> i32;
    async fn set_ir_filter(&self, enabled: bool) -> i32;
    async fn set_wdr(&self, enabled: bool) -> i32;
    /// AE average luma (`current_calc_avg_lumi`), or `None` if unavailable.
    async fn get_ae_luma(&self) -> Option<u8>;
    /// The vendor's day/night luminance ratio, or `None` if unavailable.
    ///
    /// `isp_get_cur_lum_factor() * 40 / avg_lumi` — exposure effort per unit of
    /// achieved brightness, so **higher means darker**. This is the signal the
    /// stock firmware switches on; `get_ae_luma` alone is AE-regulated and
    /// cannot see dusk. See `docs/reference/vendor-day-night-implementation.md`.
    async fn get_lum_factor(&self) -> Option<i32>;
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
/// * `sdk_max` - Maximum SDK register value
///
/// # Returns
///
/// SDK register value (0 to sdk_max)
pub(crate) fn onvif_to_sdk_value(onvif_value: f32, sdk_max: i32) -> i32 {
    ((onvif_value / ONVIF_MAX) * sdk_max as f32).round() as i32
}

/// Internal helper that takes FFI trait for testability.
pub(crate) async fn imaging_set_brightness(
    value: f32,
    ffi: &dyn ImagingHalTrait,
) -> PlatformResult<()> {
    validate_onvif_range(value, "brightness")?;
    let sdk_value = onvif_to_sdk_value(value, SDK_MAX_VALUE);
    let ret = ffi.set_brightness(sdk_value).await;
    check_result(ret, "imaging_set_brightness")
}

/// Internal helper that takes FFI trait for testability.
pub(crate) async fn imaging_set_contrast(
    value: f32,
    ffi: &dyn ImagingHalTrait,
) -> PlatformResult<()> {
    validate_onvif_range(value, "contrast")?;
    let sdk_value = onvif_to_sdk_value(value, SDK_MAX_VALUE);
    let ret = ffi.set_contrast(sdk_value).await;
    check_result(ret, "imaging_set_contrast")
}

/// Internal helper that takes FFI trait for testability.
pub(crate) async fn imaging_set_saturation(
    value: f32,
    ffi: &dyn ImagingHalTrait,
) -> PlatformResult<()> {
    validate_onvif_range(value, "saturation")?;
    let sdk_value = onvif_to_sdk_value(value, SDK_MAX_VALUE);
    let ret = ffi.set_saturation(sdk_value).await;
    check_result(ret, "imaging_set_saturation")
}

/// Internal helper that takes FFI trait for testability.
pub(crate) async fn imaging_set_sharpness(
    value: f32,
    ffi: &dyn ImagingHalTrait,
) -> PlatformResult<()> {
    validate_onvif_range(value, "sharpness")?;
    let sdk_value = onvif_to_sdk_value(value, SDK_MAX_VALUE);
    let ret = ffi.set_sharpness(sdk_value).await;
    check_result(ret, "imaging_set_sharpness")
}

#[allow(dead_code)] // Called from platform layer on ARM
pub(crate) async fn imaging_set_ir_filter(
    enabled: bool,
    ffi: &dyn ImagingHalTrait,
) -> PlatformResult<()> {
    let ret = ffi.set_ir_filter(enabled).await;
    check_result(ret, "imaging_set_ir_filter")
}

#[allow(dead_code)] // Called from platform layer on ARM
pub(crate) async fn imaging_set_wdr(
    enabled: bool,
    ffi: &dyn ImagingHalTrait,
) -> PlatformResult<()> {
    let ret = ffi.set_wdr(enabled).await;
    check_result(ret, "imaging_set_wdr")
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;

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
    fn test_onvif_to_sdk_value() {
        // 0.0 should map to 0
        assert_eq!(onvif_to_sdk_value(0.0, SDK_MAX_VALUE), 0);
        // 100.0 should map to 255
        assert_eq!(onvif_to_sdk_value(100.0, SDK_MAX_VALUE), 255);
        // 50.0 should map to approximately 128
        assert_eq!(onvif_to_sdk_value(50.0, SDK_MAX_VALUE), 128);
        // Custom max value
        assert_eq!(onvif_to_sdk_value(50.0, 100), 50);
    }

    #[tokio::test]
    async fn test_imaging_set_brightness_calls_ffi() {
        let mut mock_ffi = MockImagingHalTrait::new();

        mock_ffi
            .expect_set_brightness()
            .with(eq(128)) // 50.0 maps to 128
            .times(1)
            .returning(|_| AK_SUCCESS_I32);

        let result = imaging_set_brightness(50.0, &mock_ffi).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_imaging_set_brightness_validates_range() {
        let mock_ffi = MockImagingHalTrait::new();

        // Should fail validation before calling FFI
        let result = imaging_set_brightness(150.0, &mock_ffi).await;
        assert!(result.is_err());
        match result {
            Err(PlatformError::InvalidParameter(msg)) => {
                assert!(msg.contains("brightness"));
            }
            _ => panic!("Expected InvalidParameter error"),
        }
    }

    #[tokio::test]
    async fn test_imaging_set_brightness_propagates_error() {
        let mut mock_ffi = MockImagingHalTrait::new();

        mock_ffi
            .expect_set_brightness()
            .times(1)
            .returning(|_| AK_FAILED_I32);

        let result = imaging_set_brightness(50.0, &mock_ffi).await;
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareFailure(msg)) => {
                assert!(msg.contains("imaging_set_brightness"));
            }
            _ => panic!("Expected HardwareFailure error"),
        }
    }

    #[tokio::test]
    async fn test_imaging_set_contrast_calls_ffi() {
        let mut mock_ffi = MockImagingHalTrait::new();

        mock_ffi
            .expect_set_contrast()
            .with(eq(255)) // 100.0 maps to 255
            .times(1)
            .returning(|_| AK_SUCCESS_I32);

        let result = imaging_set_contrast(100.0, &mock_ffi).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_imaging_set_saturation_calls_ffi() {
        let mut mock_ffi = MockImagingHalTrait::new();

        mock_ffi
            .expect_set_saturation()
            .with(eq(0)) // 0.0 maps to 0
            .times(1)
            .returning(|_| AK_SUCCESS_I32);

        let result = imaging_set_saturation(0.0, &mock_ffi).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_imaging_set_sharpness_calls_ffi() {
        let mut mock_ffi = MockImagingHalTrait::new();

        mock_ffi
            .expect_set_sharpness()
            .with(eq(64)) // 25.0 maps to approximately 64
            .times(1)
            .returning(|_| AK_SUCCESS_I32);

        let result = imaging_set_sharpness(25.0, &mock_ffi).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_imaging_set_ir_filter_calls_ffi_enabled() {
        let mut mock_ffi = MockImagingHalTrait::new();

        mock_ffi
            .expect_set_ir_filter()
            .with(eq(true))
            .times(1)
            .returning(|_| AK_SUCCESS_I32);

        let result = imaging_set_ir_filter(true, &mock_ffi).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_imaging_set_ir_filter_calls_ffi_disabled() {
        let mut mock_ffi = MockImagingHalTrait::new();

        mock_ffi
            .expect_set_ir_filter()
            .with(eq(false))
            .times(1)
            .returning(|_| AK_SUCCESS_I32);

        let result = imaging_set_ir_filter(false, &mock_ffi).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_imaging_set_ir_filter_propagates_error() {
        let mut mock_ffi = MockImagingHalTrait::new();

        mock_ffi
            .expect_set_ir_filter()
            .times(1)
            .returning(|_| AK_FAILED_I32);

        let result = imaging_set_ir_filter(true, &mock_ffi).await;
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareFailure(msg)) => {
                assert!(msg.contains("imaging_set_ir_filter"));
            }
            _ => panic!("Expected HardwareFailure error"),
        }
    }

    #[tokio::test]
    async fn test_imaging_set_wdr_calls_ffi_enabled() {
        let mut mock_ffi = MockImagingHalTrait::new();

        mock_ffi
            .expect_set_wdr()
            .with(eq(true))
            .times(1)
            .returning(|_| AK_SUCCESS_I32);

        let result = imaging_set_wdr(true, &mock_ffi).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_imaging_set_wdr_calls_ffi_disabled() {
        let mut mock_ffi = MockImagingHalTrait::new();

        mock_ffi
            .expect_set_wdr()
            .with(eq(false))
            .times(1)
            .returning(|_| AK_SUCCESS_I32);

        let result = imaging_set_wdr(false, &mock_ffi).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_imaging_set_wdr_propagates_error() {
        let mut mock_ffi = MockImagingHalTrait::new();

        mock_ffi
            .expect_set_wdr()
            .times(1)
            .returning(|_| AK_FAILED_I32);

        let result = imaging_set_wdr(true, &mock_ffi).await;
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareFailure(msg)) => {
                assert!(msg.contains("imaging_set_wdr"));
            }
            _ => panic!("Expected HardwareFailure error"),
        }
    }
}
