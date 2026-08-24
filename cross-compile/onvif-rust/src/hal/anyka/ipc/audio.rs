//! AudioHalTrait implementation for AnykaIpc.

use std::ffi::c_void;
use tracing::error;

use crate::hal::common::audio::AudioHalTrait;
use crate::hal::common::{AK_FAILED_I32, aenc_attr, audio_param, pcm_param};
use crate::platform::PlatformResult;

use super::{
    AnykaIpc, CMD_AENC_CLOSE, CMD_AENC_OPEN, CMD_AENC_SET_ATTR, CMD_AI_CLOSE, CMD_AI_OPEN,
    CMD_AI_SET_ADC_VOLUME, CMD_AI_SET_ASLC_VOLUME,
};

impl AudioHalTrait for AnykaIpc {
    fn ai_open(&self, param: *const pcm_param) -> *mut c_void {
        // SAFETY: caller guarantees `param` is a valid, non-null pointer to a
        // `pcm_param` that remains valid for the duration of this call.
        let (req_buf, req_len) = unsafe { Self::encode_pcm_param_buf(&*param) };
        match self.send_handle_request(CMD_AI_OPEN, &req_buf[..req_len]) {
            Ok(handle) => handle,
            Err(e) => {
                error!(error = %e, "ai_open IPC failed");
                std::ptr::null_mut()
            }
        }
    }

    fn ai_close(&self, handle: *mut c_void) -> i32 {
        let handle_val = handle as u64;
        let req_data = handle_val.to_le_bytes().to_vec();
        match self.send_request(CMD_AI_CLOSE, &req_data) {
            Ok((status, _)) => status,
            Err(e) => {
                error!(error = %e, "ai_close IPC failed");
                AK_FAILED_I32
            }
        }
    }

    fn ai_set_adc_volume(&self, handle: *mut c_void, vol: i32) -> i32 {
        let handle_val = handle as u64;
        let mut req_data = handle_val.to_le_bytes().to_vec();
        req_data.extend_from_slice(&vol.to_le_bytes());
        match self.send_request(CMD_AI_SET_ADC_VOLUME, &req_data) {
            Ok((status, _)) => status,
            Err(e) => {
                error!(error = %e, "ai_set_adc_volume IPC failed");
                AK_FAILED_I32
            }
        }
    }

    fn ai_set_aslc_volume(&self, handle: *mut c_void, vol: i32) -> i32 {
        let handle_val = handle as u64;
        let mut req_data = handle_val.to_le_bytes().to_vec();
        req_data.extend_from_slice(&vol.to_le_bytes());
        match self.send_request(CMD_AI_SET_ASLC_VOLUME, &req_data) {
            Ok((status, _)) => status,
            Err(e) => {
                error!(error = %e, "ai_set_aslc_volume IPC failed");
                AK_FAILED_I32
            }
        }
    }

    fn aenc_open(&self, param: *const audio_param) -> *mut c_void {
        // SAFETY: caller guarantees `param` is a valid, non-null pointer to an
        // `audio_param` that remains valid for the duration of this call.
        let (req_buf, req_len) = unsafe { Self::encode_audio_param_buf(&*param) };
        match self.send_handle_request(CMD_AENC_OPEN, &req_buf[..req_len]) {
            Ok(handle) => handle,
            Err(e) => {
                error!(error = %e, "aenc_open IPC failed");
                std::ptr::null_mut()
            }
        }
    }

    fn aenc_close(&self, handle: *mut c_void) -> i32 {
        let handle_val = handle as u64;
        let req_data = handle_val.to_le_bytes().to_vec();
        match self.send_request(CMD_AENC_CLOSE, &req_data) {
            Ok((status, _)) => status,
            Err(e) => {
                error!(error = %e, "aenc_close IPC failed");
                AK_FAILED_I32
            }
        }
    }

    fn aenc_set_attr(&self, handle: *mut c_void, attr: *const aenc_attr) -> i32 {
        let handle_val = handle as u64;
        let mut req_buf = [0u8; 16]; // 8 bytes handle + 8 bytes padding for alignment
        req_buf[0..8].copy_from_slice(&handle_val.to_le_bytes());
        // SAFETY: caller guarantees `attr` is a valid, non-null pointer to an
        // `aenc_attr` that remains valid for the duration of this call.
        let (attr_buf, attr_len) = unsafe { Self::encode_aenc_attr_buf(&*attr) };
        req_buf[8..8 + attr_len].copy_from_slice(&attr_buf[..attr_len]);
        match self.send_request(CMD_AENC_SET_ATTR, &req_buf[..8 + attr_len]) {
            Ok((status, _)) => status,
            Err(e) => {
                error!(error = %e, "aenc_set_attr IPC failed");
                AK_FAILED_I32
            }
        }
    }

    fn start_audio_push(&self, sample_rate: u32, channels: u32) -> PlatformResult<()> {
        // The inherent method on AnykaIpc (mod.rs) has the same name; Rust
        // method resolution prefers it over this trait method, so no recursion.
        self.start_audio_push(sample_rate, channels)
    }

    fn stop_audio_push(&self) -> PlatformResult<()> {
        self.stop_audio_push()
    }
}
