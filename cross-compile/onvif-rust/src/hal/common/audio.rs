//! Safe Rust wrappers for Anyka SDK audio functions.
//!
//! This module provides RAII-based wrappers around the Anyka SDK audio input
//! and audio encoder functions. All handles are automatically cleaned up
//! when dropped, ensuring proper resource management.
//!
//! # RAII Pattern
//!
//! All handles implement the Drop trait to automatically clean up resources:
//!
//! ```rust,no_run
//! use onvif_rust::hal::common::audio::*;
//! use onvif_rust::hal::common::pcm_param;
//!
//! // Handle is automatically closed when it goes out of scope
//! {
//!     let param = pcm_param { /* ... */ };
//!     let ai_handle = audio_input_open(&param)?;
//!     // Use handle...
//! } // ak_ai_close() is called here automatically
//! ```
//!
//! # Error Handling
//!
//! All functions return `Result<T, PlatformError>`, converting SDK error codes
//! (`AK_SUCCESS`/`AK_FAILED`) into appropriate `PlatformError` variants.

use crate::platform::PlatformError;
use crate::platform::PlatformResult;
use std::ffi::c_void;

use super::{aenc_attr, audio_param, pcm_param};

use super::{AK_SUCCESS_I32, check_result};
// AK_FAILED_I32 is used by tests via `use super::*` → re-exported from common
#[cfg(test)]
use super::AK_FAILED_I32;

/// Internal trait for abstracting audio FFI calls to enable mocking in tests.
#[cfg_attr(test, mockall::automock)]
#[allow(dead_code)]
pub(crate) trait AudioHalTrait: Send + Sync {
    fn ai_open(&self, param: *const pcm_param) -> *mut c_void;
    fn ai_close(&self, handle: *mut c_void) -> i32;
    fn ai_set_adc_volume(&self, handle: *mut c_void, vol: i32) -> i32;
    fn ai_set_aslc_volume(&self, handle: *mut c_void, vol: i32) -> i32;
    fn aenc_open(&self, param: *const audio_param) -> *mut c_void;
    fn aenc_close(&self, handle: *mut c_void) -> i32;
    fn aenc_set_attr(&self, enc_handle: *mut c_void, attr: *const aenc_attr) -> i32;
    /// Start push-based audio delivery from the daemon (AAC frames on the ring).
    ///
    /// Unlike the handle-returning ops above, this carries no handles: the
    /// daemon owns the whole `ak_ai_open` → `ak_aenc_request_stream` chain.
    fn start_audio_push(&self, sample_rate: u32, channels: u32) -> PlatformResult<()>;
    /// Stop push-based audio delivery.
    fn stop_audio_push(&self) -> PlatformResult<()>;
}

/// RAII handle for audio input device.
///
/// This handle automatically closes the audio input device when dropped,
/// ensuring proper resource cleanup even in error paths.
///
/// # Thread Safety
///
/// The underlying SDK uses internal mutexes for thread safety, so this handle
/// is safe to send and share between threads.
#[allow(dead_code)] // Fields used on ARM targets
pub struct AudioInputHandle {
    handle: *mut c_void,
}

// SAFETY: AudioInputHandle is thread-safe - SDK uses internal mutexes.
// Similar to VideoInputHandle, the SDK provides internal synchronization.
unsafe impl Send for AudioInputHandle {}
unsafe impl Sync for AudioInputHandle {}

impl Drop for AudioInputHandle {
    fn drop(&mut self) {
        // In IPC mode, handles are managed by vendor-daemon - no-op cleanup.
    }
}

impl AudioInputHandle {
    /// Get the raw handle pointer.
    ///
    /// # Safety
    ///
    /// The returned pointer is only valid while the handle is alive.
    /// Do not use after the handle is dropped.
    #[allow(dead_code)] // Used on ARM targets
    pub(crate) fn as_ptr(&self) -> *mut c_void {
        self.handle
    }
}

#[allow(dead_code)] // Called from platform layer on ARM
pub(crate) fn audio_input_open(
    param: &pcm_param,
    ffi: &dyn AudioHalTrait,
) -> PlatformResult<AudioInputHandle> {
    let handle = ffi.ai_open(param);

    if handle.is_null() {
        Err(PlatformError::HardwareUnavailable(
            "Audio input device".to_string(),
        ))
    } else {
        Ok(AudioInputHandle { handle })
    }
}

#[allow(dead_code)] // Called from platform layer on ARM
pub(crate) fn audio_input_set_volume(
    handle: &AudioInputHandle,
    volume: u8,
    ffi: &dyn AudioHalTrait,
) -> PlatformResult<()> {
    // Validate volume range (0-15) before computing ADC/ASLC splits
    if volume > 15 {
        return Err(PlatformError::InvalidParameter(format!(
            "Volume out of range: {}. Supported range is 0-15",
            volume
        )));
    }

    // Implement the macro logic: ak_ai_set_volume splits volume into ADC and ASLC
    let adc_vol = if volume >= 8 { 8 } else { (volume % 8) as i32 };
    let aslc_vol = if volume >= 8 { (volume - 8) as i32 } else { 0 };

    let adc_ret = ffi.ai_set_adc_volume(handle.as_ptr(), adc_vol);
    let aslc_ret = ffi.ai_set_aslc_volume(handle.as_ptr(), aslc_vol);

    // Both must succeed (matching SDK macro behavior)
    if adc_ret == AK_SUCCESS_I32 && aslc_ret == AK_SUCCESS_I32 {
        Ok(())
    } else {
        Err(PlatformError::HardwareFailure(
            "ak_ai_set_volume failed".to_string(),
        ))
    }
}

/// RAII handle for audio encoder.
///
/// This handle automatically closes the audio encoder when dropped,
/// ensuring proper resource cleanup even in error paths.
///
/// # Thread Safety
///
/// The underlying SDK uses internal mutexes for thread safety, so this handle
/// is safe to send and share between threads.
#[allow(dead_code)] // Fields used on ARM targets
pub struct AudioEncoderHandle {
    handle: *mut c_void,
}

// SAFETY: AudioEncoderHandle is thread-safe - SDK uses internal mutexes.
// Similar to other handles, the SDK provides internal synchronization.
unsafe impl Send for AudioEncoderHandle {}
unsafe impl Sync for AudioEncoderHandle {}

impl Drop for AudioEncoderHandle {
    fn drop(&mut self) {
        // In IPC mode, handles are managed by vendor-daemon - no-op cleanup.
    }
}

impl AudioEncoderHandle {
    /// Get the raw handle pointer.
    ///
    /// # Safety
    ///
    /// The returned pointer is only valid while the handle is alive.
    /// Do not use after the handle is dropped.
    #[allow(dead_code)] // Used on ARM targets
    pub(crate) fn as_ptr(&self) -> *mut c_void {
        self.handle
    }
}

#[allow(dead_code)] // Called from platform layer on ARM
pub(crate) fn audio_encoder_open(
    param: &audio_param,
    ffi: &dyn AudioHalTrait,
) -> PlatformResult<AudioEncoderHandle> {
    let handle = ffi.aenc_open(param);

    if handle.is_null() {
        Err(PlatformError::HardwareUnavailable(
            "Audio encoder".to_string(),
        ))
    } else {
        Ok(AudioEncoderHandle { handle })
    }
}

#[allow(dead_code)] // Called from platform layer on ARM
pub(crate) fn audio_encoder_set_config(
    handle: &AudioEncoderHandle,
    attr: &aenc_attr,
    ffi: &dyn AudioHalTrait,
) -> PlatformResult<()> {
    let ret = ffi.aenc_set_attr(handle.as_ptr(), attr);
    check_result(ret, "ak_aenc_set_attr")
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mockall-based tests for wrapper functions
    #[test]
    fn test_audio_input_open_calls_ffi_and_returns_handle() {
        let mut mock_ffi = MockAudioHalTrait::new();
        let test_handle = std::ptr::NonNull::<c_void>::dangling().as_ptr();
        let param = pcm_param {
            sample_rate: 8000,
            channel_num: 1,
            sample_bits: 16,
        };

        let test_handle_usize = test_handle as usize;
        mock_ffi
            .expect_ai_open()
            .times(1)
            .returning(move |_| test_handle_usize as *mut c_void);

        let result = audio_input_open(&param, &mock_ffi);
        assert!(result.is_ok());
        let handle = result.unwrap();
        assert_eq!(handle.as_ptr(), test_handle);
    }

    #[test]
    fn test_audio_input_open_returns_error_on_null_handle() {
        let mut mock_ffi = MockAudioHalTrait::new();
        let param = pcm_param {
            sample_rate: 8000,
            channel_num: 1,
            sample_bits: 16,
        };

        mock_ffi
            .expect_ai_open()
            .times(1)
            .returning(|_| std::ptr::null_mut());

        let result = audio_input_open(&param, &mock_ffi);
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareUnavailable(_)) => {}
            _ => panic!("Expected HardwareUnavailable error"),
        }
    }

    #[test]
    fn test_audio_input_set_volume_calls_both_ffi_functions() {
        let mut mock_ffi = MockAudioHalTrait::new();
        let test_handle = std::ptr::NonNull::<c_void>::dangling().as_ptr();
        let ai_handle = AudioInputHandle {
            handle: test_handle,
        };

        // Volume >= 8: ADC = 8, ASLC = volume - 8
        let test_handle_usize = test_handle as usize;
        mock_ffi
            .expect_ai_set_adc_volume()
            .withf(move |handle, vol| *handle as usize == test_handle_usize && *vol == 8)
            .times(1)
            .returning(|_, _| AK_SUCCESS_I32);

        mock_ffi
            .expect_ai_set_aslc_volume()
            .withf(move |handle, vol| *handle as usize == test_handle_usize && *vol == 2)
            .times(1)
            .returning(|_, _| AK_SUCCESS_I32);

        let result = audio_input_set_volume(&ai_handle, 10, &mock_ffi);
        assert!(result.is_ok());
    }

    #[test]
    fn test_audio_input_set_volume_low_volume_calls_only_adc() {
        let mut mock_ffi = MockAudioHalTrait::new();
        let test_handle = std::ptr::NonNull::<c_void>::dangling().as_ptr();
        let ai_handle = AudioInputHandle {
            handle: test_handle,
        };

        // Volume < 8: ADC = volume % 8, ASLC = 0
        let test_handle_usize = test_handle as usize;
        mock_ffi
            .expect_ai_set_adc_volume()
            .withf(move |handle, vol| *handle as usize == test_handle_usize && *vol == 5)
            .times(1)
            .returning(|_, _| AK_SUCCESS_I32);

        mock_ffi
            .expect_ai_set_aslc_volume()
            .withf(move |handle, vol| *handle as usize == test_handle_usize && *vol == 0)
            .times(1)
            .returning(|_, _| AK_SUCCESS_I32);

        let result = audio_input_set_volume(&ai_handle, 5, &mock_ffi);
        assert!(result.is_ok());
    }

    #[test]
    fn test_audio_input_set_volume_propagates_error_on_adc_failure() {
        let mut mock_ffi = MockAudioHalTrait::new();
        let test_handle = std::ptr::NonNull::<c_void>::dangling().as_ptr();
        let ai_handle = AudioInputHandle {
            handle: test_handle,
        };

        let test_handle_usize = test_handle as usize;
        mock_ffi
            .expect_ai_set_adc_volume()
            .withf(move |handle, _| *handle as usize == test_handle_usize)
            .times(1)
            .returning(|_, _| AK_FAILED_I32);

        // ASLC may or may not be called, but we expect failure from ADC
        mock_ffi
            .expect_ai_set_aslc_volume()
            .times(0..=1)
            .returning(|_, _| AK_SUCCESS_I32);

        let result = audio_input_set_volume(&ai_handle, 10, &mock_ffi);
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareFailure(msg)) => {
                assert!(msg.contains("ak_ai_set_volume failed"));
            }
            _ => panic!("Expected HardwareFailure error"),
        }
    }

    #[test]
    fn test_audio_input_set_volume_propagates_error_on_aslc_failure() {
        let mut mock_ffi = MockAudioHalTrait::new();
        let test_handle = std::ptr::NonNull::<c_void>::dangling().as_ptr();
        let ai_handle = AudioInputHandle {
            handle: test_handle,
        };

        let test_handle_usize = test_handle as usize;
        mock_ffi
            .expect_ai_set_adc_volume()
            .withf(move |handle, _| *handle as usize == test_handle_usize)
            .times(1)
            .returning(|_, _| AK_SUCCESS_I32);

        mock_ffi
            .expect_ai_set_aslc_volume()
            .withf(move |handle, _| *handle as usize == test_handle_usize)
            .times(1)
            .returning(|_, _| AK_FAILED_I32);

        let result = audio_input_set_volume(&ai_handle, 10, &mock_ffi);
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareFailure(msg)) => {
                assert!(msg.contains("ak_ai_set_volume failed"));
            }
            _ => panic!("Expected HardwareFailure error"),
        }
    }

    #[test]
    fn test_audio_input_set_volume_rejects_out_of_range_volume() {
        let mock_ffi = MockAudioHalTrait::new();
        let test_handle = std::ptr::NonNull::<c_void>::dangling().as_ptr();
        let ai_handle = AudioInputHandle {
            handle: test_handle,
        };

        // Volume > 15 should be rejected
        let result = audio_input_set_volume(&ai_handle, 16, &mock_ffi);
        assert!(result.is_err());
        match result {
            Err(PlatformError::InvalidParameter(msg)) => {
                assert!(msg.contains("Volume out of range"));
                assert!(msg.contains("16"));
                assert!(msg.contains("0-15"));
            }
            _ => panic!("Expected InvalidParameter error"),
        }

        // Volume at maximum valid value (15) should be accepted
        let mut mock_ffi_valid = MockAudioHalTrait::new();
        let test_handle_usize = test_handle as usize;
        mock_ffi_valid
            .expect_ai_set_adc_volume()
            .withf(move |handle, vol| *handle as usize == test_handle_usize && *vol == 8)
            .times(1)
            .returning(|_, _| AK_SUCCESS_I32);
        mock_ffi_valid
            .expect_ai_set_aslc_volume()
            .withf(move |handle, vol| *handle as usize == test_handle_usize && *vol == 7)
            .times(1)
            .returning(|_, _| AK_SUCCESS_I32);

        let result = audio_input_set_volume(&ai_handle, 15, &mock_ffi_valid);
        assert!(result.is_ok());
    }

    #[test]
    fn test_audio_encoder_open_calls_ffi_and_returns_handle() {
        let mut mock_ffi = MockAudioHalTrait::new();
        let test_handle = std::ptr::NonNull::<c_void>::dangling().as_ptr();
        let param = audio_param {
            sample_rate: 8000,
            channel_num: 1,
            sample_bits: 16,
            ..Default::default()
        };

        let test_handle_usize = test_handle as usize;
        mock_ffi
            .expect_aenc_open()
            .times(1)
            .returning(move |_| test_handle_usize as *mut c_void);

        let result = audio_encoder_open(&param, &mock_ffi);
        assert!(result.is_ok());
        let handle = result.unwrap();
        assert_eq!(handle.as_ptr(), test_handle);
    }

    #[test]
    fn test_audio_encoder_open_returns_error_on_null_handle() {
        let mut mock_ffi = MockAudioHalTrait::new();
        let param = audio_param {
            sample_rate: 8000,
            channel_num: 1,
            sample_bits: 16,
            ..Default::default()
        };

        mock_ffi
            .expect_aenc_open()
            .times(1)
            .returning(|_| std::ptr::null_mut());

        let result = audio_encoder_open(&param, &mock_ffi);
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareUnavailable(_)) => {}
            _ => panic!("Expected HardwareUnavailable error"),
        }
    }

    #[test]
    fn test_audio_encoder_set_config_calls_ffi() {
        let mut mock_ffi = MockAudioHalTrait::new();
        let test_handle = std::ptr::NonNull::<c_void>::dangling().as_ptr();
        let enc_handle = AudioEncoderHandle {
            handle: test_handle,
        };
        let attr = aenc_attr::default();

        let test_handle_usize = test_handle as usize;
        mock_ffi
            .expect_aenc_set_attr()
            .withf(move |handle, _| *handle as usize == test_handle_usize)
            .times(1)
            .returning(|_, _| AK_SUCCESS_I32);

        let result = audio_encoder_set_config(&enc_handle, &attr, &mock_ffi);
        assert!(result.is_ok());
    }

    #[test]
    fn test_audio_encoder_set_config_propagates_error() {
        let mut mock_ffi = MockAudioHalTrait::new();
        let test_handle = std::ptr::NonNull::<c_void>::dangling().as_ptr();
        let enc_handle = AudioEncoderHandle {
            handle: test_handle,
        };
        let attr = aenc_attr::default();

        let test_handle_usize = test_handle as usize;
        mock_ffi
            .expect_aenc_set_attr()
            .withf(move |handle, _| *handle as usize == test_handle_usize)
            .times(1)
            .returning(|_, _| AK_FAILED_I32);

        let result = audio_encoder_set_config(&enc_handle, &attr, &mock_ffi);
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareFailure(msg)) => {
                assert!(msg.contains("ak_aenc_set_attr"));
            }
            _ => panic!("Expected HardwareFailure error"),
        }
    }
}
