//! Stub imaging HAL implementation for host-side testing.

use async_trait::async_trait;

use crate::hal::common::AK_SUCCESS_I32;
use crate::hal::common::imaging::{AeAttr, AeRunInfo, ImagingHalTrait};

/// Stub implementation that returns success for all imaging operations.
#[allow(dead_code)] // Used on host targets only
pub(crate) struct StubImagingHal;

#[async_trait]
impl ImagingHalTrait for StubImagingHal {
    async fn set_brightness(&self, _value: i32) -> i32 {
        AK_SUCCESS_I32
    }

    async fn set_contrast(&self, _value: i32) -> i32 {
        AK_SUCCESS_I32
    }

    async fn set_saturation(&self, _value: i32) -> i32 {
        AK_SUCCESS_I32
    }

    async fn set_sharpness(&self, _value: i32) -> i32 {
        AK_SUCCESS_I32
    }

    async fn set_ir_filter(&self, _enabled: bool) -> i32 {
        AK_SUCCESS_I32
    }

    async fn set_wdr(&self, _level: i32) -> i32 {
        AK_SUCCESS_I32
    }

    async fn set_blc(&self, _level: i32) -> i32 {
        AK_SUCCESS_I32
    }

    async fn set_wb_type(&self, _wb_type: u16) -> i32 {
        AK_SUCCESS_I32
    }

    async fn set_mwb_attr(&self, _r_gain: u16, _b_gain: u16) -> i32 {
        AK_SUCCESS_I32
    }

    async fn get_mwb_attr(&self) -> Option<(u16, u16)> {
        None
    }

    async fn get_ae_luma(&self) -> Option<u8> {
        None
    }

    async fn get_lum_factor(&self) -> Option<i32> {
        None
    }

    async fn get_ae_attr(&self) -> Option<AeAttr> {
        None
    }

    async fn get_awb_stat(&self) -> Option<[i32; 10]> {
        None
    }

    async fn get_ae_run_info(&self) -> Option<AeRunInfo> {
        None
    }

    async fn set_hue(&self, _value: i32) -> i32 {
        AK_SUCCESS_I32
    }

    async fn set_power_hz(&self, _hz: u16) -> i32 {
        AK_SUCCESS_I32
    }

    async fn set_style_id(&self, _style_id: u8) -> i32 {
        AK_SUCCESS_I32
    }
}

#[cfg(test)]
mod stub_getter_tests {
    use super::*;

    #[tokio::test]
    async fn test_stub_get_ae_luma_returns_none() {
        let stub = StubImagingHal;
        assert!(stub.get_ae_luma().await.is_none());
    }

    #[tokio::test]
    async fn test_stub_get_lum_factor_returns_none() {
        let stub = StubImagingHal;
        assert!(stub.get_lum_factor().await.is_none());
    }

    #[tokio::test]
    async fn test_stub_get_ae_attr_returns_none() {
        let stub = StubImagingHal;
        assert!(stub.get_ae_attr().await.is_none());
    }

    #[tokio::test]
    async fn test_stub_get_awb_stat_returns_none() {
        let stub = StubImagingHal;
        assert!(stub.get_awb_stat().await.is_none());
    }

    #[tokio::test]
    async fn test_stub_get_mwb_attr_returns_none() {
        let stub = StubImagingHal;
        assert!(stub.get_mwb_attr().await.is_none());
    }
}

#[cfg(all(test, use_stubs))]
mod tests {
    use super::*;
    use crate::hal::common::imaging::{
        imaging_set_brightness, imaging_set_contrast, imaging_set_ir_filter,
        imaging_set_saturation, imaging_set_sharpness, imaging_set_wdr, imaging_set_wdr_disabled,
    };

    #[tokio::test]
    async fn test_imaging_set_brightness_success() {
        let stub = StubImagingHal;
        let result = imaging_set_brightness(50.0, &stub).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_imaging_set_contrast_success() {
        let stub = StubImagingHal;
        let result = imaging_set_contrast(75.0, &stub).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_imaging_set_saturation_success() {
        let stub = StubImagingHal;
        let result = imaging_set_saturation(60.0, &stub).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_imaging_set_sharpness_success() {
        let stub = StubImagingHal;
        let result = imaging_set_sharpness(80.0, &stub).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_imaging_set_ir_filter_success() {
        let stub = StubImagingHal;
        let result = imaging_set_ir_filter(true, &stub).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_imaging_set_wdr_success() {
        let stub = StubImagingHal;
        assert!(imaging_set_wdr(70.0, &stub).await.is_ok());
        assert!(imaging_set_wdr_disabled(&stub).await.is_ok());
    }
}
