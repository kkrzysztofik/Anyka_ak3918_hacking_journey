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
//! use onvif_rust::ffi::audio::*;
//! use onvif_rust::ffi::pcm_param;
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

#[cfg(not(use_stubs))]
use crate::ffi::generated::{aenc_attr, audio_param, pcm_param};

#[cfg(use_stubs)]
use crate::ffi::{aenc_attr, audio_param, pcm_param};

use crate::ffi::{AK_FAILED_I32, AK_SUCCESS_I32};

/// Internal trait for abstracting audio FFI calls to enable mocking in tests.
#[cfg_attr(test, mockall::automock)]
pub(crate) trait AudioFfiTrait: Send + Sync {
    fn ai_open(&self, param: *const pcm_param) -> *mut c_void;
    fn ai_close(&self, handle: *mut c_void) -> i32;
    fn ai_set_adc_volume(&self, handle: *mut c_void, vol: i32) -> i32;
    fn ai_set_aslc_volume(&self, handle: *mut c_void, vol: i32) -> i32;
    fn aenc_open(&self, param: *const audio_param) -> *mut c_void;
    fn aenc_close(&self, handle: *mut c_void) -> i32;
    fn aenc_set_attr(&self, enc_handle: *mut c_void, attr: *const aenc_attr) -> i32;
}

/// Default implementation that calls the real FFI functions.
pub(crate) struct RealAudioFfi;

impl AudioFfiTrait for RealAudioFfi {
    #[cfg(not(use_stubs))]
    fn ai_open(&self, param: *const pcm_param) -> *mut c_void {
        unsafe extern "C" {
            fn ak_ai_open(param: *const pcm_param) -> *mut c_void;
        }
        unsafe { ak_ai_open(param) }
    }

    #[cfg(use_stubs)]
    fn ai_open(&self, _param: *const pcm_param) -> *mut c_void {
        std::ptr::NonNull::<c_void>::dangling().as_ptr()
    }

    #[cfg(not(use_stubs))]
    fn ai_close(&self, handle: *mut c_void) -> i32 {
        unsafe extern "C" {
            fn ak_ai_close(handle: *mut c_void) -> i32;
        }
        unsafe { ak_ai_close(handle) }
    }

    #[cfg(use_stubs)]
    fn ai_close(&self, _handle: *mut c_void) -> i32 {
        AK_SUCCESS_I32
    }

    #[cfg(not(use_stubs))]
    fn ai_set_adc_volume(&self, handle: *mut c_void, vol: i32) -> i32 {
        unsafe extern "C" {
            fn ak_ai_set_adc_volume(handle: *mut c_void, vol: i32) -> i32;
        }
        unsafe { ak_ai_set_adc_volume(handle, vol) }
    }

    #[cfg(use_stubs)]
    fn ai_set_adc_volume(&self, _handle: *mut c_void, _vol: i32) -> i32 {
        AK_SUCCESS_I32
    }

    #[cfg(not(use_stubs))]
    fn ai_set_aslc_volume(&self, handle: *mut c_void, vol: i32) -> i32 {
        unsafe extern "C" {
            fn ak_ai_set_aslc_volume(handle: *mut c_void, vol: i32) -> i32;
        }
        unsafe { ak_ai_set_aslc_volume(handle, vol) }
    }

    #[cfg(use_stubs)]
    fn ai_set_aslc_volume(&self, _handle: *mut c_void, _vol: i32) -> i32 {
        AK_SUCCESS_I32
    }

    #[cfg(not(use_stubs))]
    fn aenc_open(&self, param: *const audio_param) -> *mut c_void {
        unsafe extern "C" {
            fn ak_aenc_open(param: *const audio_param) -> *mut c_void;
        }
        unsafe { ak_aenc_open(param) }
    }

    #[cfg(use_stubs)]
    fn aenc_open(&self, _param: *const audio_param) -> *mut c_void {
        std::ptr::NonNull::<c_void>::dangling().as_ptr()
    }

    #[cfg(not(use_stubs))]
    fn aenc_close(&self, handle: *mut c_void) -> i32 {
        unsafe extern "C" {
            fn ak_aenc_close(handle: *mut c_void) -> i32;
        }
        unsafe { ak_aenc_close(handle) }
    }

    #[cfg(use_stubs)]
    fn aenc_close(&self, _handle: *mut c_void) -> i32 {
        AK_SUCCESS_I32
    }

    #[cfg(not(use_stubs))]
    fn aenc_set_attr(&self, enc_handle: *mut c_void, attr: *const aenc_attr) -> i32 {
        unsafe extern "C" {
            fn ak_aenc_set_attr(enc_handle: *mut c_void, attr: *const aenc_attr) -> i32;
        }
        unsafe { ak_aenc_set_attr(enc_handle, attr) }
    }

    #[cfg(use_stubs)]
    fn aenc_set_attr(&self, _enc_handle: *mut c_void, _attr: *const aenc_attr) -> i32 {
        AK_SUCCESS_I32
    }
}

// Global instance for default FFI implementation
static REAL_AUDIO_FFI: RealAudioFfi = RealAudioFfi;

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

/// RAII handle for audio input device.
///
/// This handle automatically closes the audio input device when dropped,
/// ensuring proper resource cleanup even in error paths.
///
/// # Thread Safety
///
/// The underlying SDK uses internal mutexes for thread safety, so this handle
/// is safe to send and share between threads.
pub struct AudioInputHandle {
    handle: *mut c_void,
}

// SAFETY: AudioInputHandle is thread-safe - SDK uses internal mutexes.
// Similar to VideoInputHandle, the SDK provides internal synchronization.
unsafe impl Send for AudioInputHandle {}
unsafe impl Sync for AudioInputHandle {}

impl Drop for AudioInputHandle {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            let _ = REAL_AUDIO_FFI.ai_close(self.handle);
        }
    }
}

impl AudioInputHandle {
    /// Get the raw handle pointer.
    ///
    /// # Safety
    ///
    /// The returned pointer is only valid while the handle is alive.
    /// Do not use after the handle is dropped.
    pub(crate) fn as_ptr(&self) -> *mut c_void {
        self.handle
    }
}

/// Internal helper that takes FFI trait for testability.
pub(crate) fn audio_input_open_internal(
    param: &pcm_param,
    ffi: &dyn AudioFfiTrait,
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

/// Open an audio input device.
///
/// # Arguments
///
/// * `param` - PCM parameters for audio input
///
/// # Returns
///
/// * `Ok(AudioInputHandle)` on success
/// * `Err(PlatformError::HardwareUnavailable)` if device cannot be opened
///
/// # Safety
///
/// The FFI call is safe because:
/// - `ak_ai_open()` returns a handle or NULL
/// - We validate the result (null check)
/// - Handle is wrapped in `AudioInputHandle` for RAII cleanup
pub fn audio_input_open(param: &pcm_param) -> PlatformResult<AudioInputHandle> {
    audio_input_open_internal(param, &REAL_AUDIO_FFI)
}

/// Internal helper that takes FFI trait for testability.
pub(crate) fn audio_input_set_volume_internal(
    handle: &AudioInputHandle,
    volume: u8,
    ffi: &dyn AudioFfiTrait,
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

/// Set audio input volume.
///
/// This function wraps the `ak_ai_set_volume()` macro, which internally
/// calls `ak_ai_set_adc_volume()` and `ak_ai_set_aslc_volume()`.
///
/// # Arguments
///
/// * `handle` - Audio input handle
/// * `volume` - Volume level (0-15, where >=8 uses both ADC and ASLC)
///
/// # Returns
///
/// * `Ok(())` on success
/// * `Err(PlatformError::HardwareFailure)` on SDK error
///
/// # Notes
///
/// The SDK macro `ak_ai_set_volume` splits the volume:
/// - ADC volume: `volume >= 8 ? 8 : (volume % 8)`
/// - ASLC volume: `volume >= 8 ? (volume - 8) : 0`
pub fn audio_input_set_volume(handle: &AudioInputHandle, volume: u8) -> PlatformResult<()> {
    audio_input_set_volume_internal(handle, volume, &REAL_AUDIO_FFI)
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
pub struct AudioEncoderHandle {
    handle: *mut c_void,
}

// SAFETY: AudioEncoderHandle is thread-safe - SDK uses internal mutexes.
// Similar to other handles, the SDK provides internal synchronization.
unsafe impl Send for AudioEncoderHandle {}
unsafe impl Sync for AudioEncoderHandle {}

impl Drop for AudioEncoderHandle {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            let _ = REAL_AUDIO_FFI.aenc_close(self.handle);
        }
    }
}

impl AudioEncoderHandle {
    /// Get the raw handle pointer.
    ///
    /// # Safety
    ///
    /// The returned pointer is only valid while the handle is alive.
    /// Do not use after the handle is dropped.
    pub(crate) fn as_ptr(&self) -> *mut c_void {
        self.handle
    }
}

/// Internal helper that takes FFI trait for testability.
pub(crate) fn audio_encoder_open_internal(
    param: &audio_param,
    ffi: &dyn AudioFfiTrait,
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

/// Open an audio encoder.
///
/// # Arguments
///
/// * `param` - Audio encoding parameters
///
/// # Returns
///
/// * `Ok(AudioEncoderHandle)` on success
/// * `Err(PlatformError::HardwareUnavailable)` if encoder cannot be opened
///
/// # Safety
///
/// The FFI call is safe because:
/// - `ak_aenc_open()` returns a handle or NULL
/// - We validate the result (null check)
/// - Handle is wrapped in `AudioEncoderHandle` for RAII cleanup
pub fn audio_encoder_open(param: &audio_param) -> PlatformResult<AudioEncoderHandle> {
    audio_encoder_open_internal(param, &REAL_AUDIO_FFI)
}

/// Internal helper that takes FFI trait for testability.
pub(crate) fn audio_encoder_set_config_internal(
    handle: &AudioEncoderHandle,
    attr: &aenc_attr,
    ffi: &dyn AudioFfiTrait,
) -> PlatformResult<()> {
    let ret = ffi.aenc_set_attr(handle.as_ptr(), attr);
    check_result(ret, "ak_aenc_set_attr")
}

/// Set audio encoder configuration.
///
/// This wraps `ak_aenc_set_attr()` which sets audio encoder attributes
/// after the encoder has been opened.
///
/// # Arguments
///
/// * `handle` - Audio encoder handle
/// * `attr` - Audio encoder attributes to set
///
/// # Returns
///
/// * `Ok(())` on success
/// * `Err(PlatformError::HardwareFailure)` on SDK error
pub fn audio_encoder_set_config(
    handle: &AudioEncoderHandle,
    attr: &aenc_attr,
) -> PlatformResult<()> {
    audio_encoder_set_config_internal(handle, attr, &REAL_AUDIO_FFI)
}

#[cfg(test)]
mod tests {
    use super::*;

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
    #[cfg(use_stubs)]
    fn test_audio_input_open_success() {
        let param = pcm_param {
            sample_rate: 8000,
            channel_num: 1,
            sample_bits: 16,
        };
        let result = audio_input_open(&param);
        assert!(result.is_ok());
        let handle = result.unwrap();
        assert!(!handle.as_ptr().is_null());
    }

    #[test]
    #[cfg(use_stubs)]
    fn test_audio_input_set_volume_success() {
        let param = pcm_param {
            sample_rate: 8000,
            channel_num: 1,
            sample_bits: 16,
        };
        let handle = audio_input_open(&param).unwrap();
        let result = audio_input_set_volume(&handle, 10);
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(use_stubs)]
    fn test_audio_input_set_volume_low() {
        let param = pcm_param {
            sample_rate: 8000,
            channel_num: 1,
            sample_bits: 16,
        };
        let handle = audio_input_open(&param).unwrap();
        // Volume < 8: ADC = volume % 8, ASLC = 0
        let result = audio_input_set_volume(&handle, 5);
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(use_stubs)]
    fn test_audio_encoder_open_success() {
        let mut param = audio_param::default();
        param.sample_rate = 8000;
        param.channel_num = 1;
        param.sample_bits = 16;
        let result = audio_encoder_open(&param);
        assert!(result.is_ok());
        let handle = result.unwrap();
        assert!(!handle.as_ptr().is_null());
    }

    #[test]
    #[cfg(use_stubs)]
    fn test_audio_encoder_set_config_success() {
        let mut param = audio_param::default();
        param.sample_rate = 8000;
        param.channel_num = 1;
        param.sample_bits = 16;
        let handle = audio_encoder_open(&param).unwrap();
        let attr = aenc_attr::default();
        let result = audio_encoder_set_config(&handle, &attr);
        assert!(result.is_ok());
    }

    // Mockall-based tests for wrapper functions
    #[test]
    fn test_audio_input_open_internal_calls_ffi_and_returns_handle() {
        let mut mock_ffi = MockAudioFfiTrait::new();
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

        let result = audio_input_open_internal(&param, &mock_ffi);
        assert!(result.is_ok());
        let handle = result.unwrap();
        assert_eq!(handle.as_ptr(), test_handle);
    }

    #[test]
    fn test_audio_input_open_internal_returns_error_on_null_handle() {
        let mut mock_ffi = MockAudioFfiTrait::new();
        let param = pcm_param {
            sample_rate: 8000,
            channel_num: 1,
            sample_bits: 16,
        };

        mock_ffi
            .expect_ai_open()
            .times(1)
            .returning(|_| std::ptr::null_mut());

        let result = audio_input_open_internal(&param, &mock_ffi);
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareUnavailable(_)) => {}
            _ => panic!("Expected HardwareUnavailable error"),
        }
    }

    #[test]
    fn test_audio_input_set_volume_internal_calls_both_ffi_functions() {
        let mut mock_ffi = MockAudioFfiTrait::new();
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

        let result = audio_input_set_volume_internal(&ai_handle, 10, &mock_ffi);
        assert!(result.is_ok());
    }

    #[test]
    fn test_audio_input_set_volume_internal_low_volume_calls_only_adc() {
        let mut mock_ffi = MockAudioFfiTrait::new();
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

        let result = audio_input_set_volume_internal(&ai_handle, 5, &mock_ffi);
        assert!(result.is_ok());
    }

    #[test]
    fn test_audio_input_set_volume_internal_propagates_error_on_adc_failure() {
        let mut mock_ffi = MockAudioFfiTrait::new();
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

        let result = audio_input_set_volume_internal(&ai_handle, 10, &mock_ffi);
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareFailure(msg)) => {
                assert!(msg.contains("ak_ai_set_volume failed"));
            }
            _ => panic!("Expected HardwareFailure error"),
        }
    }

    #[test]
    fn test_audio_input_set_volume_internal_propagates_error_on_aslc_failure() {
        let mut mock_ffi = MockAudioFfiTrait::new();
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

        let result = audio_input_set_volume_internal(&ai_handle, 10, &mock_ffi);
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareFailure(msg)) => {
                assert!(msg.contains("ak_ai_set_volume failed"));
            }
            _ => panic!("Expected HardwareFailure error"),
        }
    }

    #[test]
    fn test_audio_input_set_volume_internal_rejects_out_of_range_volume() {
        let mock_ffi = MockAudioFfiTrait::new();
        let test_handle = std::ptr::NonNull::<c_void>::dangling().as_ptr();
        let ai_handle = AudioInputHandle {
            handle: test_handle,
        };

        // Volume > 15 should be rejected
        let result = audio_input_set_volume_internal(&ai_handle, 16, &mock_ffi);
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
        let mut mock_ffi_valid = MockAudioFfiTrait::new();
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

        let result = audio_input_set_volume_internal(&ai_handle, 15, &mock_ffi_valid);
        assert!(result.is_ok());
    }

    #[test]
    fn test_audio_encoder_open_internal_calls_ffi_and_returns_handle() {
        let mut mock_ffi = MockAudioFfiTrait::new();
        let test_handle = std::ptr::NonNull::<c_void>::dangling().as_ptr();
        let mut param = audio_param::default();
        param.sample_rate = 8000;
        param.channel_num = 1;
        param.sample_bits = 16;

        let test_handle_usize = test_handle as usize;
        mock_ffi
            .expect_aenc_open()
            .times(1)
            .returning(move |_| test_handle_usize as *mut c_void);

        let result = audio_encoder_open_internal(&param, &mock_ffi);
        assert!(result.is_ok());
        let handle = result.unwrap();
        assert_eq!(handle.as_ptr(), test_handle);
    }

    #[test]
    fn test_audio_encoder_open_internal_returns_error_on_null_handle() {
        let mut mock_ffi = MockAudioFfiTrait::new();
        let mut param = audio_param::default();
        param.sample_rate = 8000;
        param.channel_num = 1;
        param.sample_bits = 16;

        mock_ffi
            .expect_aenc_open()
            .times(1)
            .returning(|_| std::ptr::null_mut());

        let result = audio_encoder_open_internal(&param, &mock_ffi);
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareUnavailable(_)) => {}
            _ => panic!("Expected HardwareUnavailable error"),
        }
    }

    #[test]
    fn test_audio_encoder_set_config_internal_calls_ffi() {
        let mut mock_ffi = MockAudioFfiTrait::new();
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

        let result = audio_encoder_set_config_internal(&enc_handle, &attr, &mock_ffi);
        assert!(result.is_ok());
    }

    #[test]
    fn test_audio_encoder_set_config_internal_propagates_error() {
        let mut mock_ffi = MockAudioFfiTrait::new();
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

        let result = audio_encoder_set_config_internal(&enc_handle, &attr, &mock_ffi);
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareFailure(msg)) => {
                assert!(msg.contains("ak_aenc_set_attr"));
            }
            _ => panic!("Expected HardwareFailure error"),
        }
    }
}
