//! Stub video HAL implementation for host-side testing.

use std::ffi::{c_char, c_void};

use crate::hal::common::video::VideoHalTrait;
use crate::hal::common::{
    AK_FAILED_I32, AK_SUCCESS_I32, encode_param, video_channel_attr, video_dev_type,
    video_resolution, video_stream,
};

/// Stub implementation that returns success for all video operations.
#[allow(dead_code)] // Used on host targets only
pub(crate) struct StubVideoHal;

impl VideoHalTrait for StubVideoHal {
    fn vi_match_sensor(&self, _config_file: *const c_char) -> i32 {
        AK_SUCCESS_I32
    }

    fn vi_open(&self, _dev: video_dev_type) -> *mut c_void {
        std::ptr::NonNull::<c_void>::dangling().as_ptr()
    }

    fn vi_close(&self, _handle: *mut c_void) -> i32 {
        AK_SUCCESS_I32
    }

    fn vi_get_sensor_resolution(&self, _handle: *mut c_void, res: *mut video_resolution) -> i32 {
        unsafe {
            (*res).width = 1920;
            (*res).height = 1080;
            (*res).max_width = 1920;
            (*res).max_height = 1080;
        }
        AK_SUCCESS_I32
    }

    fn vi_set_channel_attr(&self, _handle: *mut c_void, _attr: *const video_channel_attr) -> i32 {
        AK_SUCCESS_I32
    }

    fn vi_capture_on(&self, _handle: *mut c_void) -> i32 {
        AK_SUCCESS_I32
    }

    fn vi_capture_off(&self, _handle: *mut c_void) -> i32 {
        AK_SUCCESS_I32
    }

    fn vpss_init(&self, _vi_handle: *mut c_void, _dev: i32) {
        // Stub: no-op for testing
    }

    fn vpss_destroy(&self, _dev: i32) {
        // Stub: no-op for testing
    }

    fn venc_set_cfg_path(&self, _path: *const c_char) -> i32 {
        AK_SUCCESS_I32
    }

    fn venc_open(&self, _param: *const encode_param) -> *mut c_void {
        std::ptr::NonNull::<c_void>::dangling().as_ptr()
    }

    fn venc_close(&self, _handle: *mut c_void) -> i32 {
        AK_SUCCESS_I32
    }

    fn venc_set_rc(&self, _enc_handle: *mut c_void, _bps: i32) -> i32 {
        AK_SUCCESS_I32
    }

    fn venc_set_iframe(&self, _enc_handle: *mut c_void) -> i32 {
        AK_SUCCESS_I32
    }

    fn venc_request_stream(
        &self,
        _vi_handle: *mut c_void,
        _venc_handle: *mut c_void,
    ) -> *mut c_void {
        std::ptr::NonNull::<c_void>::dangling().as_ptr()
    }

    fn venc_get_stream(&self, _stream_handle: *mut c_void, _stream: *mut video_stream) -> i32 {
        AK_FAILED_I32 // No frames in stub mode
    }

    fn venc_release_stream(&self, _stream_handle: *mut c_void, _stream: *mut video_stream) -> i32 {
        AK_SUCCESS_I32
    }

    fn venc_cancel_stream(&self, _stream_handle: *mut c_void) -> i32 {
        AK_SUCCESS_I32
    }

    fn get_error_no(&self) -> i32 {
        0
    }

    fn get_error_str(&self) -> String {
        String::new()
    }
}

#[cfg(all(test, use_stubs))]
mod tests {
    use super::*;
    use crate::hal::anyka::sdk::VideoDevice;
    use crate::hal::common::video::{
        video_encoder_open, video_encoder_request_idr, video_encoder_set_rc,
        video_input_get_sensor_resolution, video_input_open, vpss_destroy, vpss_init,
    };

    #[test]
    fn test_video_input_open_success() {
        let stub = StubVideoHal;
        let result = video_input_open(VideoDevice::DEV0, &stub);
        assert!(result.is_ok());
        let handle = result.unwrap();
        assert!(!handle.as_ptr().is_null());
    }

    #[test]
    fn test_video_input_get_sensor_resolution_success() {
        let stub = StubVideoHal;
        let handle = video_input_open(VideoDevice::DEV0, &stub).unwrap();
        let result = video_input_get_sensor_resolution(&handle, &stub);
        assert!(result.is_ok());
        let res = result.unwrap();
        assert_eq!(res.width, 1920);
        assert_eq!(res.height, 1080);
    }

    #[test]
    fn test_video_encoder_open_success() {
        let stub = StubVideoHal;
        let param = encode_param {
            width: 1920,
            height: 1080,
            minqp: 10,
            maxqp: 51,
            fps: 30,
            goplen: 30,
            bps: 2000000,
            ..Default::default()
        };
        let result = video_encoder_open(&param, &stub);
        assert!(result.is_ok());
        let handle = result.unwrap();
        assert!(!handle.as_ptr().is_null());
    }

    #[test]
    fn test_video_encoder_set_rc_success() {
        let stub = StubVideoHal;
        let param = encode_param {
            width: 1920,
            height: 1080,
            minqp: 10,
            maxqp: 51,
            fps: 30,
            goplen: 30,
            bps: 2000000,
            ..Default::default()
        };
        let handle = video_encoder_open(&param, &stub).unwrap();
        let result = video_encoder_set_rc(&handle, 3000000, &stub);
        assert!(result.is_ok());
    }

    #[test]
    fn test_video_encoder_request_idr_success() {
        let stub = StubVideoHal;
        let param = encode_param {
            width: 1920,
            height: 1080,
            minqp: 10,
            maxqp: 51,
            fps: 30,
            goplen: 30,
            bps: 2000000,
            ..Default::default()
        };
        let handle = video_encoder_open(&param, &stub).unwrap();
        let result = video_encoder_request_idr(&handle, &stub);
        assert!(result.is_ok());
    }

    #[test]
    fn test_vpss_init_success() {
        let stub = StubVideoHal;
        let handle = video_input_open(VideoDevice::DEV0, &stub).unwrap();
        let result = vpss_init(&handle, VideoDevice::DEV0, &stub);
        assert!(result.is_ok());
    }

    #[test]
    fn test_vpss_destroy_success() {
        let stub = StubVideoHal;
        let result = vpss_destroy(VideoDevice::DEV0, &stub);
        assert!(result.is_ok());
    }
}
