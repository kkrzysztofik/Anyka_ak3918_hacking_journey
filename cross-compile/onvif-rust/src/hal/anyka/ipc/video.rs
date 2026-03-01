//! VideoHalTrait implementation for AnykaIpc.

use std::ffi::{c_char, c_void};
use tracing::error;

use crate::hal::common::video::VideoHalTrait;
use crate::hal::common::{
    AK_FAILED_I32, AK_SUCCESS_I32, encode_param, video_channel_attr, video_dev_type,
    video_resolution, video_stream,
};

use super::{
    AnykaIpc, CMD_GET_ERROR_NO, CMD_GET_ERROR_STR, CMD_VENC_CANCEL_STREAM, CMD_VENC_CLOSE,
    CMD_VENC_OPEN, CMD_VENC_REQUEST_STREAM, CMD_VENC_SET_IFRAME, CMD_VENC_SET_RC,
    CMD_VI_CAPTURE_OFF, CMD_VI_CAPTURE_ON, CMD_VI_CLOSE, CMD_VI_GET_SENSOR_RESOLUTION,
    CMD_VI_MATCH_SENSOR, CMD_VI_OPEN, CMD_VI_SET_CHANNEL_ATTR,
};

impl VideoHalTrait for AnykaIpc {
    fn vi_match_sensor(&self, config_file: *const c_char) -> i32 {
        if config_file.is_null() {
            return AK_FAILED_I32;
        }
        // SAFETY: caller guarantees `config_file` is a valid, null-terminated C string
        // for the duration of this call (same contract as the underlying FFI).
        let c_str = unsafe { std::ffi::CStr::from_ptr(config_file) };
        let path_str = match c_str.to_str() {
            Ok(s) => s,
            Err(_) => return AK_FAILED_I32,
        };

        match self.send_i32_request(CMD_VI_MATCH_SENSOR, path_str.as_bytes()) {
            Ok(status) => status,
            Err(e) => {
                error!(error = %e, "vi_match_sensor IPC failed");
                AK_FAILED_I32
            }
        }
    }

    fn vi_open(&self, dev: video_dev_type) -> *mut c_void {
        let (req_buf, req_len) = Self::encode_video_dev_type_buf(dev);
        match self.send_handle_request(CMD_VI_OPEN, &req_buf[..req_len]) {
            Ok(handle) => handle,
            Err(e) => {
                error!(error = %e, "vi_open IPC failed");
                std::ptr::null_mut()
            }
        }
    }

    fn vi_close(&self, handle: *mut c_void) -> i32 {
        let handle_val = handle as u64;
        let req_data = handle_val.to_le_bytes().to_vec();
        match self.send_request(CMD_VI_CLOSE, &req_data) {
            Ok((status, _)) => status,
            Err(e) => {
                error!(error = %e, "vi_close IPC failed");
                AK_FAILED_I32
            }
        }
    }

    fn vi_get_sensor_resolution(&self, handle: *mut c_void, res: *mut video_resolution) -> i32 {
        let handle_val = handle as u64;
        let req_data = handle_val.to_le_bytes().to_vec();
        match self.send_request(CMD_VI_GET_SENSOR_RESOLUTION, &req_data) {
            Ok((status, data)) => {
                if status == AK_SUCCESS_I32 {
                    match Self::decode_video_resolution(&data) {
                        Ok(r) => {
                            // SAFETY: caller guarantees `res` is a valid, properly aligned
                            // pointer to a `video_resolution` struct that we may write.
                            unsafe {
                                *res = r;
                            }
                            return AK_SUCCESS_I32;
                        }
                        Err(e) => {
                            error!(error = %e, "Failed to decode vi_get_sensor_resolution response");
                            return AK_FAILED_I32;
                        }
                    }
                }
                status
            }
            Err(e) => {
                error!(error = %e, "vi_get_sensor_resolution IPC failed");
                AK_FAILED_I32
            }
        }
    }

    fn vi_set_channel_attr(&self, handle: *mut c_void, attr: *const video_channel_attr) -> i32 {
        let handle_val = handle as u64;
        let mut req_buf = [0u8; 56]; // 8 bytes handle + 48 bytes attr
        req_buf[0..8].copy_from_slice(&handle_val.to_le_bytes());
        // SAFETY: caller guarantees `attr` is a valid, non-null pointer to a
        // `video_channel_attr` that remains valid for the duration of this call.
        let (attr_buf, attr_len) = unsafe { Self::encode_video_channel_attr_buf(&*attr) };
        req_buf[8..8 + attr_len].copy_from_slice(&attr_buf[..attr_len]);
        match self.send_request(CMD_VI_SET_CHANNEL_ATTR, &req_buf[..8 + attr_len]) {
            Ok((status, _)) => status,
            Err(e) => {
                error!(error = %e, "vi_set_channel_attr IPC failed");
                AK_FAILED_I32
            }
        }
    }

    fn vi_capture_on(&self, handle: *mut c_void) -> i32 {
        let handle_val = handle as u64;
        let req_data = handle_val.to_le_bytes().to_vec();
        match self.send_request(CMD_VI_CAPTURE_ON, &req_data) {
            Ok((status, _)) => status,
            Err(e) => {
                error!(error = %e, "vi_capture_on IPC failed");
                AK_FAILED_I32
            }
        }
    }

    fn vi_capture_off(&self, handle: *mut c_void) -> i32 {
        let handle_val = handle as u64;
        let req_data = handle_val.to_le_bytes().to_vec();
        match self.send_request(CMD_VI_CAPTURE_OFF, &req_data) {
            Ok((status, _)) => status,
            Err(e) => {
                error!(error = %e, "vi_capture_off IPC failed");
                AK_FAILED_I32
            }
        }
    }

    fn vpss_init(&self, _vi_handle: *mut c_void, _dev: i32) {}

    fn vpss_destroy(&self, _dev: i32) {}

    fn venc_set_cfg_path(&self, _path: *const c_char) -> i32 {
        AK_SUCCESS_I32
    }

    fn venc_open(&self, param: *const encode_param) -> *mut c_void {
        // SAFETY: caller guarantees `param` is a valid, non-null pointer to an
        // `encode_param` that remains valid for the duration of this call.
        let (req_buf, req_len) = unsafe { Self::encode_encode_param_buf(&*param) };
        match self.send_handle_request(CMD_VENC_OPEN, &req_buf[..req_len]) {
            Ok(handle) => handle,
            Err(e) => {
                error!(error = %e, "venc_open IPC failed");
                std::ptr::null_mut()
            }
        }
    }

    fn venc_close(&self, handle: *mut c_void) -> i32 {
        let handle_val = handle as u64;
        let req_data = handle_val.to_le_bytes().to_vec();
        match self.send_request(CMD_VENC_CLOSE, &req_data) {
            Ok((status, _)) => status,
            Err(e) => {
                error!(error = %e, "venc_close IPC failed");
                AK_FAILED_I32
            }
        }
    }

    fn venc_set_rc(&self, enc_handle: *mut c_void, bps: i32) -> i32 {
        let handle_val = enc_handle as u64;
        let mut req_data = handle_val.to_le_bytes().to_vec();
        req_data.extend_from_slice(&bps.to_le_bytes());
        match self.send_request(CMD_VENC_SET_RC, &req_data) {
            Ok((status, _)) => status,
            Err(e) => {
                error!(error = %e, "venc_set_rc IPC failed");
                AK_FAILED_I32
            }
        }
    }

    fn venc_set_iframe(&self, enc_handle: *mut c_void) -> i32 {
        let handle_val = enc_handle as u64;
        let req_data = handle_val.to_le_bytes().to_vec();
        match self.send_request(CMD_VENC_SET_IFRAME, &req_data) {
            Ok((status, _)) => status,
            Err(e) => {
                error!(error = %e, "venc_set_iframe IPC failed");
                AK_FAILED_I32
            }
        }
    }

    fn venc_request_stream(&self, vi_handle: *mut c_void, venc_handle: *mut c_void) -> *mut c_void {
        let vi_val = vi_handle as u64;
        let venc_val = venc_handle as u64;
        let mut req_data = vi_val.to_le_bytes().to_vec();
        req_data.extend_from_slice(&venc_val.to_le_bytes());
        match self.send_handle_request(CMD_VENC_REQUEST_STREAM, &req_data) {
            Ok(handle) => handle,
            Err(e) => {
                error!(error = %e, "venc_request_stream IPC failed");
                std::ptr::null_mut()
            }
        }
    }

    fn venc_get_stream(&self, stream_handle: *mut c_void, stream: *mut video_stream) -> i32 {
        let _ = stream_handle;
        let _ = stream;
        error!("venc_get_stream is removed in push-only mode");
        AK_FAILED_I32
    }

    fn venc_release_stream(&self, stream_handle: *mut c_void, stream: *mut video_stream) -> i32 {
        let _ = stream_handle;
        let _ = stream;
        error!("venc_release_stream is removed in push-only mode");
        AK_FAILED_I32
    }

    fn venc_cancel_stream(&self, stream_handle: *mut c_void) -> i32 {
        let handle_val = stream_handle as u64;
        let req_data = handle_val.to_le_bytes().to_vec();
        match self.send_request(CMD_VENC_CANCEL_STREAM, &req_data) {
            Ok((status, _)) => status,
            Err(e) => {
                error!(error = %e, "venc_cancel_stream IPC failed");
                AK_FAILED_I32
            }
        }
    }

    fn get_error_no(&self) -> i32 {
        self.send_i32_request(CMD_GET_ERROR_NO, &[]).unwrap_or(-1)
    }

    fn get_error_str(&self) -> String {
        match self.send_request(CMD_GET_ERROR_STR, &[]) {
            Ok((_, data)) => String::from_utf8_lossy(&data).to_string(),
            Err(_) => String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_helpers::*;
    use super::*;

    /// A daemon that sends an 8-byte handle lets vi_open return a non-null pointer.
    #[test]
    fn test_vi_open_basic_roundtrip() {
        let handle_value: i64 = 0x1234_5678;
        let daemon = FakeDaemon::start(move |_cmd_id, _req| {
            (AK_SUCCESS_I32, handle_value.to_le_bytes().to_vec())
        });
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();

        let handle = {
            use crate::hal::common::sdk_types::VideoDevType;
            <AnykaIpc as VideoHalTrait>::vi_open(&ipc, VideoDevType::Dev0)
        };

        assert!(!handle.is_null(), "expected non-null handle from daemon");
        assert_eq!(
            handle as i64, handle_value,
            "handle value should match daemon response"
        );
    }

    /// venc_get_stream returns AK_FAILED in push-only mode.
    #[test]
    #[cfg(use_stubs)]
    fn test_venc_get_stream_returns_null_on_failure() {
        use std::mem::MaybeUninit;
        let daemon = FakeDaemon::start(|_cmd_id, _req| (AK_SUCCESS_I32, vec![]));

        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
        let stream_handle = 1usize as *mut std::ffi::c_void;
        let mut vs = MaybeUninit::<crate::hal::common::sdk_types::VideoStream>::zeroed();
        let vs_ptr = vs.as_mut_ptr() as *mut video_stream;

        let result = <AnykaIpc as VideoHalTrait>::venc_get_stream(&ipc, stream_handle, vs_ptr);
        assert_eq!(result, AK_FAILED_I32);
    }
}
