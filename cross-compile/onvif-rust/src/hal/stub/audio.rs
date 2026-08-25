//! Stub audio HAL implementation for host-side testing.

use std::ffi::c_void;

use crate::hal::common::audio::AudioHalTrait;
use crate::hal::common::{AK_SUCCESS_I32, aenc_attr, audio_param, pcm_param};
use crate::platform::PlatformResult;

/// Stub implementation that returns success for all audio operations.
#[allow(dead_code)] // Used on host targets only
pub(crate) struct StubAudioHal;

impl AudioHalTrait for StubAudioHal {
    fn ai_open(&self, _param: *const pcm_param) -> *mut c_void {
        std::ptr::NonNull::<c_void>::dangling().as_ptr()
    }

    fn ai_close(&self, _handle: *mut c_void) -> i32 {
        AK_SUCCESS_I32
    }

    fn ai_set_adc_volume(&self, _handle: *mut c_void, _vol: i32) -> i32 {
        AK_SUCCESS_I32
    }

    fn ai_set_aslc_volume(&self, _handle: *mut c_void, _vol: i32) -> i32 {
        AK_SUCCESS_I32
    }

    fn aenc_open(&self, _param: *const audio_param) -> *mut c_void {
        std::ptr::NonNull::<c_void>::dangling().as_ptr()
    }

    fn aenc_close(&self, _handle: *mut c_void) -> i32 {
        AK_SUCCESS_I32
    }

    fn aenc_set_attr(&self, _enc_handle: *mut c_void, _attr: *const aenc_attr) -> i32 {
        AK_SUCCESS_I32
    }

    fn start_audio_push(&self, _sample_rate: u32, _channels: u32) -> PlatformResult<()> {
        Ok(())
    }

    fn stop_audio_push(&self) -> PlatformResult<()> {
        Ok(())
    }
}

#[cfg(all(test, use_stubs))]
mod tests {
    use super::*;
    use crate::hal::common::audio::{
        audio_encoder_open, audio_encoder_set_config, audio_input_open, audio_input_set_volume,
    };

    #[test]
    fn test_audio_input_open_success() {
        let stub = StubAudioHal;
        let param = pcm_param {
            sample_rate: 8000,
            channel_num: 1,
            sample_bits: 16,
        };
        let result = audio_input_open(&param, &stub);
        assert!(result.is_ok());
        let handle = result.unwrap();
        assert!(!handle.as_ptr().is_null());
    }

    #[test]
    fn test_audio_input_set_volume_success() {
        let stub = StubAudioHal;
        let param = pcm_param {
            sample_rate: 8000,
            channel_num: 1,
            sample_bits: 16,
        };
        let handle = audio_input_open(&param, &stub).unwrap();
        let result = audio_input_set_volume(&handle, 10, &stub);
        assert!(result.is_ok());
    }

    #[test]
    fn test_audio_input_set_volume_low() {
        let stub = StubAudioHal;
        let param = pcm_param {
            sample_rate: 8000,
            channel_num: 1,
            sample_bits: 16,
        };
        let handle = audio_input_open(&param, &stub).unwrap();
        // Volume < 8: ADC = volume % 8, ASLC = 0
        let result = audio_input_set_volume(&handle, 5, &stub);
        assert!(result.is_ok());
    }

    #[test]
    fn test_audio_encoder_open_success() {
        let stub = StubAudioHal;
        let param = audio_param {
            sample_rate: 8000,
            channel_num: 1,
            sample_bits: 16,
            ..Default::default()
        };
        let result = audio_encoder_open(&param, &stub);
        assert!(result.is_ok());
        let handle = result.unwrap();
        assert!(!handle.as_ptr().is_null());
    }

    #[test]
    fn test_audio_encoder_set_config_success() {
        let stub = StubAudioHal;
        let param = audio_param {
            sample_rate: 8000,
            channel_num: 1,
            sample_bits: 16,
            ..Default::default()
        };
        let handle = audio_encoder_open(&param, &stub).unwrap();
        let attr = aenc_attr::default();
        let result = audio_encoder_set_config(&handle, &attr, &stub);
        assert!(result.is_ok());
    }
}
