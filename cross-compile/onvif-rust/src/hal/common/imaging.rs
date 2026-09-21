//! Safe Rust wrappers for Anyka SDK imaging functions.
//!
//! This module provides safe wrappers around the Anyka SDK imaging/ISP functions
//! for controlling image quality parameters such as brightness, contrast, saturation,
//! sharpness, IR filter, and WDR settings.
//!
//! # Parameter Mapping
//!
//! ONVIF uses a 0.0-100.0 range for imaging parameters. The SDK does not take a
//! register value: `ak_vpss_effect_set` accepts a signed offset in `[-50, 50]`
//! from the ISP tuning profile, where 0 means "use the value in the ISP config
//! file". This module maps the ONVIF midpoint (50.0) to that 0 offset.
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

/// Minimum ONVIF imaging parameter value.
const ONVIF_MIN: f32 = 0.0;

/// Maximum ONVIF imaging parameter value.
const ONVIF_MAX: f32 = 100.0;

/// ONVIF midpoint, which the SDK represents as offset 0 ("ISP config default").
/// Mirrors the vendor's `IMG_EFFECT_DEF_VAL` (`libapp/src/onvif/ja_media.h:10`).
const ONVIF_MIDPOINT: f32 = 50.0;

/// Live ISP auto-exposure limits, as reported by the sensor profile in force.
///
/// Mirrors `struct vpss_isp_ae_attr` (204 bytes, ARMv5TE). Only the fields we
/// have a use for are decoded; the rest of the struct is carried by the daemon
/// but not modelled here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AeAttr {
    pub exp_time_max: u32,
    pub exp_time_min: u32,
    pub d_gain_max: u32,
    pub a_gain_max: u32,
    pub target_lumiance: u32,
}

/// Wire size of `struct vpss_isp_ae_attr` on the camera.
pub(crate) const AE_ATTR_WIRE_LEN: usize = 204;

/// Wire size of the AWB colour-bin response: `i32 total_cnt[10]`.
pub(crate) const AWB_STAT_WIRE_LEN: usize = 40;

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
    async fn set_wdr(&self, level: i32) -> i32;
    async fn set_blc(&self, level: i32) -> i32;
    /// Set the white balance operation type (`WB_OPS_TYPE_MANU`/`WB_OPS_TYPE_AUTO`).
    async fn set_wb_type(&self, wb_type: u16) -> i32;
    /// Manual white balance gains (unitless multipliers).
    async fn set_mwb_attr(&self, r_gain: u16, b_gain: u16) -> i32;
    /// The ISP's current manual white balance gains `(r_gain, b_gain)`, or
    /// `None` if unavailable.
    async fn get_mwb_attr(&self) -> Option<(u16, u16)>;
    /// AE average luma (`current_calc_avg_lumi`), or `None` if unavailable.
    async fn get_ae_luma(&self) -> Option<u8>;
    /// The vendor's day/night luminance ratio, or `None` if unavailable.
    ///
    /// `isp_get_cur_lum_factor() * 40 / avg_lumi` — exposure effort per unit of
    /// achieved brightness, so **higher means darker**. This is the signal the
    /// stock firmware switches on; `get_ae_luma` alone is AE-regulated and
    /// cannot see dusk. See `docs/reference/vendor-day-night-implementation.md`.
    async fn get_lum_factor(&self) -> Option<i32>;
    /// Live ISP AE limits, or `None` if unavailable.
    async fn get_ae_attr(&self) -> Option<AeAttr>;
    /// Live ISP AWB colour-temperature bin counts (`total_cnt[10]`), or `None`
    /// if unavailable. A zero bin is a legitimate reading (e.g. AWB going
    /// quiet under IR illumination) and must not be conflated with `None`.
    async fn get_awb_stat(&self) -> Option<[i32; 10]>;
    /// Override AE ceilings, preserving every other AE attribute (the daemon
    /// read-modify-writes the struct). `None` leaves a ceiling alone.
    async fn set_ae_attr(&self, a_gain_max: Option<i32>, exp_time_max: Option<i32>) -> i32;
    /// The AE loop's current operating point, or `None` if unavailable.
    async fn get_ae_run_info(&self) -> Option<AeRunInfo>;
    /// Select the exposure mode: `true` = auto (the AE loop runs),
    /// `false` = manual (the driver applies the manual-AE parameters).
    async fn set_ae_mode(&self, auto: bool) -> i32;
    /// Colour tint (`VPSS_EFFECT_HUE`, raw ISP scale -100..100).
    async fn set_hue(&self, value: i32) -> i32;
    /// Mains frequency for flicker reduction (`VPSS_POWER_HZ`, 50 or 60 only —
    /// the vendor lib silently accepts anything else, so callers validate).
    async fn set_power_hz(&self, hz: u16) -> i32;
    /// ISP picture-style id (`VPSS_STYLE_ID`, 0..=2; must match an isp cfg entry).
    async fn set_style_id(&self, style_id: u8) -> i32;
}

/// Wire size of the daemon's `AK_ISP_MWB_ATTR` response
/// (`u16 r_gain, u16 g_gain, u16 b_gain, s16 r_offset, s16 g_offset, s16 b_offset`).
pub(crate) const MWB_ATTR_WIRE_LEN: usize = 12;

/// Live AE operating point, decoded from `struct vpss_isp_ae_run_info`
/// (36 bytes on the wire; the `*_step` fields are not modelled).
///
/// All fields are the driver's raw units — `exp_time` is a sensor line count
/// and the gains are fixed-point. Interpretation (µs, dB) belongs to
/// `docs/reference/anyka-ae-units.md`; nothing here pre-converts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct AeRunInfo {
    pub avg_lumi: u8,
    pub compensation_lumi: u8,
    /// 1 when the driver considers the scene darked (night path).
    pub darked_flag: u8,
    pub a_gain: i32,
    pub d_gain: i32,
    pub isp_d_gain: i32,
    pub exp_time: i32,
}

/// Wire size of `struct vpss_isp_ae_run_info` on the camera.
pub(crate) const AE_RUN_INFO_WIRE_LEN: usize = 36;

/// The driver's `WB_OPS_TYPE_MANU`: manual white balance.
pub const WB_TYPE_MANUAL: u16 = 0;

/// The driver's `WB_OPS_TYPE_AUTO`: auto white balance.
pub const WB_TYPE_AUTO: u16 = 1;

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

/// Convert an ONVIF parameter value (0.0-100.0) to an Anyka ISP effect offset.
///
/// The SDK does not take a register value. `ak_vpss_effect_set` accepts a
/// signed offset in `[-50, 50]` where **0 means "use the value in the ISP
/// config file"**, and `isp_set_effect` rejects anything outside that range
/// with `AK_FAILED` rather than clamping it. The vendor's own ONVIF layer
/// maps the two scales by subtracting the midpoint, and so do we.
///
/// See `docs/plans/2026-09-21-imaging-tab-completion-design.md`.
pub(crate) fn onvif_to_effect_value(onvif_value: f32) -> i32 {
    (onvif_value - ONVIF_MIDPOINT).round() as i32
}

/// Internal helper that takes FFI trait for testability.
pub(crate) async fn imaging_set_brightness(
    value: f32,
    ffi: &dyn ImagingHalTrait,
) -> PlatformResult<()> {
    validate_onvif_range(value, "brightness")?;
    let sdk_value = onvif_to_effect_value(value);
    let ret = ffi.set_brightness(sdk_value).await;
    check_result(ret, "imaging_set_brightness")
}

/// Internal helper that takes FFI trait for testability.
pub(crate) async fn imaging_set_contrast(
    value: f32,
    ffi: &dyn ImagingHalTrait,
) -> PlatformResult<()> {
    validate_onvif_range(value, "contrast")?;
    let sdk_value = onvif_to_effect_value(value);
    let ret = ffi.set_contrast(sdk_value).await;
    check_result(ret, "imaging_set_contrast")
}

/// Internal helper that takes FFI trait for testability.
pub(crate) async fn imaging_set_saturation(
    value: f32,
    ffi: &dyn ImagingHalTrait,
) -> PlatformResult<()> {
    validate_onvif_range(value, "saturation")?;
    let sdk_value = onvif_to_effect_value(value);
    let ret = ffi.set_saturation(sdk_value).await;
    check_result(ret, "imaging_set_saturation")
}

/// Internal helper that takes FFI trait for testability.
pub(crate) async fn imaging_set_sharpness(
    value: f32,
    ffi: &dyn ImagingHalTrait,
) -> PlatformResult<()> {
    validate_onvif_range(value, "sharpness")?;
    let sdk_value = onvif_to_effect_value(value);
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

/// Apply a wide-dynamic-range level, as an ONVIF 0-100 value.
///
/// WDR is a regular `VPSS_EFFECT_WDR` effect, so it takes the same signed
/// `[-50, 50]` offset as the other effects.
pub(crate) async fn imaging_set_wdr(value: f32, ffi: &dyn ImagingHalTrait) -> PlatformResult<()> {
    validate_onvif_range(value, "wdr level")?;
    let ret = ffi.set_wdr(onvif_to_effect_value(value)).await;
    check_result(ret, "imaging_set_wdr")
}

/// Return WDR to the ISP profile's own setting.
pub(crate) async fn imaging_set_wdr_disabled(ffi: &dyn ImagingHalTrait) -> PlatformResult<()> {
    let ret = ffi.set_wdr(0).await;
    check_result(ret, "imaging_set_wdr_disabled")
}

/// Apply a colour-tint level, as an ONVIF 0-100 value (50 = neutral).
pub(crate) async fn imaging_set_hue(value: f32, ffi: &dyn ImagingHalTrait) -> PlatformResult<()> {
    validate_onvif_range(value, "hue")?;
    let ret = ffi.set_hue(onvif_to_effect_value(value)).await;
    check_result(ret, "imaging_set_hue")
}

/// Apply the mains frequency used for flicker reduction. The vendor lib
/// silently accepts anything that is not 50/60, so the range check lives here.
pub(crate) async fn imaging_set_power_hz(hz: u16, ffi: &dyn ImagingHalTrait) -> PlatformResult<()> {
    if hz != 50 && hz != 60 {
        return Err(PlatformError::InvalidParameter(format!(
            "power_hz must be 50 or 60 (got {hz})"
        )));
    }
    let ret = ffi.set_power_hz(hz).await;
    check_result(ret, "imaging_set_power_hz")
}

/// Apply an ISP picture-style id (0-2; must match an entry in the isp cfg).
pub(crate) async fn imaging_set_style_id(
    style_id: u8,
    ffi: &dyn ImagingHalTrait,
) -> PlatformResult<()> {
    if style_id > 2 {
        return Err(PlatformError::InvalidParameter(format!(
            "style_id must be 0-2 (got {style_id})"
        )));
    }
    let ret = ffi.set_style_id(style_id).await;
    check_result(ret, "imaging_set_style_id")
}

/// Apply a backlight-compensation level, as an ONVIF 0-100 value.
///
/// BLC has no `ak_vpss` effect, so the daemon takes the same `[-50, 50]`
/// offset (0 = profile default) and applies it through the low-level ISP SDK.
pub(crate) async fn imaging_set_blc(value: f32, ffi: &dyn ImagingHalTrait) -> PlatformResult<()> {
    validate_onvif_range(value, "backlight level")?;
    let ret = ffi.set_blc(onvif_to_effect_value(value)).await;
    check_result(ret, "imaging_set_blc")
}

/// Return backlight compensation to the ISP profile's own setting.
pub(crate) async fn imaging_set_blc_disabled(ffi: &dyn ImagingHalTrait) -> PlatformResult<()> {
    let ret = ffi.set_blc(0).await;
    check_result(ret, "imaging_set_blc_disabled")
}

/// Set the white balance operation type.
pub(crate) async fn imaging_set_wb_type(
    wb_type: u16,
    ffi: &dyn ImagingHalTrait,
) -> PlatformResult<()> {
    let ret = ffi.set_wb_type(wb_type).await;
    check_result(ret, "imaging_set_wb_type")
}

/// Apply manual white balance gains.
///
/// ONVIF `CrGain`/`CbGain` are unitless multipliers, and the driver's
/// `r_gain`/`b_gain` are on the same scale, so the values pass through with
/// no conversion constant — only the f32→u16 truncation the wire format
/// forces. (The hardware gate confirms the scale on the camera.)
pub(crate) async fn imaging_set_mwb_attr(
    cr_gain: f32,
    cb_gain: f32,
    ffi: &dyn ImagingHalTrait,
) -> PlatformResult<()> {
    let ret = ffi
        .set_mwb_attr(cr_gain.round() as u16, cb_gain.round() as u16)
        .await;
    check_result(ret, "imaging_set_mwb_attr")
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
    fn test_onvif_to_effect_value_maps_neutral_to_profile_default() {
        // The SDK treats 0 as "use the value from the ISP config file", so the
        // ONVIF midpoint must map to 0 and not to a register value.
        assert_eq!(onvif_to_effect_value(50.0), 0);
    }

    #[test]
    fn test_onvif_to_effect_value_maps_full_range_within_sdk_limits() {
        assert_eq!(onvif_to_effect_value(0.0), -50);
        assert_eq!(onvif_to_effect_value(100.0), 50);
        assert_eq!(onvif_to_effect_value(25.0), -25);
        assert_eq!(onvif_to_effect_value(75.0), 25);
    }

    /// Regression: every ONVIF value must land inside the SDK's accepted
    /// `[-50, 50]`. Sending 20.0 previously produced 51, which
    /// `isp_set_effect` rejects outright with AK_FAILED.
    #[test]
    fn test_onvif_to_effect_value_never_exceeds_sdk_range() {
        for percent in 0..=100 {
            let value = onvif_to_effect_value(percent as f32);
            assert!(
                (-50..=50).contains(&value),
                "ONVIF {percent} produced out-of-range SDK value {value}"
            );
        }
    }

    #[tokio::test]
    async fn test_imaging_set_brightness_calls_ffi() {
        let mut mock_ffi = MockImagingHalTrait::new();

        mock_ffi
            .expect_set_brightness()
            .with(eq(0)) // 50.0 is the profile default
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
            .with(eq(50)) // 100.0 is +50 offset
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
            .with(eq(-50)) // 0.0 is the floor, not the default
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
            .with(eq(-25)) // 25.0 is -25 offset
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
    async fn test_imaging_set_blc_sends_effect_offset() {
        let mut mock_ffi = MockImagingHalTrait::new();
        mock_ffi
            .expect_set_blc()
            .with(eq(-10))
            .times(1)
            .returning(|_| AK_SUCCESS_I32);
        assert!(imaging_set_blc(40.0, &mock_ffi).await.is_ok());
    }

    #[tokio::test]
    async fn test_imaging_set_blc_disabled_sends_profile_default() {
        let mut mock_ffi = MockImagingHalTrait::new();
        mock_ffi
            .expect_set_blc()
            .with(eq(0))
            .times(1)
            .returning(|_| AK_SUCCESS_I32);
        assert!(imaging_set_blc_disabled(&mock_ffi).await.is_ok());
    }

    #[tokio::test]
    async fn test_imaging_set_blc_rejects_out_of_range() {
        let mock_ffi = MockImagingHalTrait::new();
        // Must fail validation before any IPC call; the mock expects none.
        assert!(imaging_set_blc(150.0, &mock_ffi).await.is_err());
    }

    #[tokio::test]
    async fn test_imaging_set_wdr_sends_effect_offset() {
        let mut mock_ffi = MockImagingHalTrait::new();
        mock_ffi
            .expect_set_wdr()
            .with(eq(20)) // 70.0 -> +20 offset from the profile default
            .times(1)
            .returning(|_| AK_SUCCESS_I32);

        let result = imaging_set_wdr(70.0, &mock_ffi).await;
        assert!(result.is_ok());
    }

    /// WDR off must mean "profile default", not "minimum", so it cannot be
    /// mapped through the same offset as an explicit level.
    #[tokio::test]
    async fn test_imaging_set_wdr_disabled_sends_profile_default() {
        let mut mock_ffi = MockImagingHalTrait::new();
        mock_ffi
            .expect_set_wdr()
            .with(eq(0))
            .times(1)
            .returning(|_| AK_SUCCESS_I32);

        let result = imaging_set_wdr_disabled(&mock_ffi).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_imaging_set_wdr_propagates_error() {
        let mut mock_ffi = MockImagingHalTrait::new();

        mock_ffi
            .expect_set_wdr()
            .times(1)
            .returning(|_| AK_FAILED_I32);

        let result = imaging_set_wdr(70.0, &mock_ffi).await;
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareFailure(msg)) => {
                assert!(msg.contains("imaging_set_wdr"));
            }
            _ => panic!("Expected HardwareFailure error"),
        }
    }

    #[tokio::test]
    async fn test_imaging_set_wb_type_calls_ffi() {
        let mut mock_ffi = MockImagingHalTrait::new();
        mock_ffi
            .expect_set_wb_type()
            .with(eq(WB_TYPE_AUTO))
            .times(1)
            .returning(|_| AK_SUCCESS_I32);
        assert!(imaging_set_wb_type(WB_TYPE_AUTO, &mock_ffi).await.is_ok());
        mock_ffi
            .expect_set_wb_type()
            .with(eq(WB_TYPE_MANUAL))
            .times(1)
            .returning(|_| AK_SUCCESS_I32);
        assert!(imaging_set_wb_type(WB_TYPE_MANUAL, &mock_ffi).await.is_ok());
    }

    /// The gains are unitless multipliers on both sides of the wire, so they
    /// must pass through without a scaling constant — assert it, don't assume.
    #[tokio::test]
    async fn test_imaging_set_mwb_attr_passes_gains_through_unscaled() {
        let mut mock_ffi = MockImagingHalTrait::new();
        mock_ffi
            .expect_set_mwb_attr()
            .with(eq(2u16), eq(3u16))
            .times(1)
            .returning(|_, _| AK_SUCCESS_I32);
        assert!(imaging_set_mwb_attr(2.0, 3.0, &mock_ffi).await.is_ok());
    }

    #[tokio::test]
    async fn test_imaging_set_mwb_attr_propagates_error() {
        let mut mock_ffi = MockImagingHalTrait::new();
        mock_ffi
            .expect_set_mwb_attr()
            .times(1)
            .returning(|_, _| AK_FAILED_I32);
        let result = imaging_set_mwb_attr(1.0, 1.0, &mock_ffi).await;
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareFailure(msg)) => {
                assert!(msg.contains("imaging_set_mwb_attr"));
            }
            _ => panic!("Expected HardwareFailure error"),
        }
    }
}
