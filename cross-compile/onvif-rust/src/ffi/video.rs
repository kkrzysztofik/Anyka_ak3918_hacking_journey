//! Safe Rust wrappers for Anyka SDK video functions.
//!
//! This module provides RAII-based wrappers around the Anyka SDK video input
//! and video encoder functions. All handles are automatically cleaned up
//! when dropped, ensuring proper resource management.
//!
//! # RAII Pattern
//!
//! All handles implement the Drop trait to automatically clean up resources:
//!
//! ```rust,no_run
//! use onvif_rust::ffi::video::*;
//! use onvif_rust::ffi::VideoDevice;
//!
//! // Handle is automatically closed when it goes out of scope
//! {
//!     let vi_handle = video_input_open(VideoDevice::DEV0)?;
//!     // Use handle...
//! } // ak_vi_close() is called here automatically
//! ```
//!
//! # Error Handling
//!
//! All functions return `Result<T, PlatformError>`, converting SDK error codes
//! (`AK_SUCCESS`/`AK_FAILED`) into appropriate `PlatformError` variants.

use crate::platform::PlatformError;
use crate::platform::PlatformResult;
use std::ffi::{CString, c_char, c_void};
use std::path::Path;

#[cfg(not(use_stubs))]
use crate::ffi::generated::{encode_param, video_channel_attr, video_dev_type, video_resolution};

#[cfg(use_stubs)]
use crate::ffi::{encode_param, video_channel_attr, video_dev_type, video_resolution};

use crate::ffi::{AK_FAILED_I32, AK_SUCCESS_I32, Resolution, VideoDevice};

/// Internal trait for abstracting video FFI calls to enable mocking in tests.
#[cfg_attr(test, mockall::automock)]
pub(crate) trait VideoFfiTrait: Send + Sync {
    fn vi_match_sensor(&self, config_file: *const c_char) -> i32;
    fn vi_open(&self, dev: video_dev_type) -> *mut c_void;
    fn vi_close(&self, handle: *mut c_void) -> i32;
    fn vi_get_sensor_resolution(&self, handle: *mut c_void, res: *mut video_resolution) -> i32;
    fn vi_set_channel_attr(&self, handle: *mut c_void, attr: *const video_channel_attr) -> i32;
    fn vi_capture_on(&self, handle: *mut c_void) -> i32;
    fn venc_open(&self, param: *const encode_param) -> *mut c_void;
    fn venc_close(&self, handle: *mut c_void) -> i32;
    fn venc_set_rc(&self, enc_handle: *mut c_void, bps: i32) -> i32;
    fn venc_set_iframe(&self, enc_handle: *mut c_void) -> i32;
}

/// Default implementation that calls the real FFI functions.
pub(crate) struct RealVideoFfi;

impl VideoFfiTrait for RealVideoFfi {
    #[cfg(not(use_stubs))]
    fn vi_match_sensor(&self, config_file: *const c_char) -> i32 {
        unsafe extern "C" {
            fn ak_vi_match_sensor(config_file: *const c_char) -> i32;
        }
        unsafe { ak_vi_match_sensor(config_file) }
    }

    #[cfg(use_stubs)]
    fn vi_match_sensor(&self, _config_file: *const c_char) -> i32 {
        AK_SUCCESS_I32
    }

    #[cfg(not(use_stubs))]
    fn vi_open(&self, dev: video_dev_type) -> *mut c_void {
        unsafe extern "C" {
            fn ak_vi_open(dev: video_dev_type) -> *mut c_void;
        }
        unsafe { ak_vi_open(dev) }
    }

    #[cfg(use_stubs)]
    fn vi_open(&self, _dev: video_dev_type) -> *mut c_void {
        std::ptr::NonNull::<c_void>::dangling().as_ptr()
    }

    #[cfg(not(use_stubs))]
    fn vi_close(&self, handle: *mut c_void) -> i32 {
        unsafe extern "C" {
            fn ak_vi_close(handle: *mut c_void) -> i32;
        }
        unsafe { ak_vi_close(handle) }
    }

    #[cfg(use_stubs)]
    fn vi_close(&self, _handle: *mut c_void) -> i32 {
        AK_SUCCESS_I32
    }

    #[cfg(not(use_stubs))]
    fn vi_get_sensor_resolution(&self, handle: *mut c_void, res: *mut video_resolution) -> i32 {
        unsafe extern "C" {
            fn ak_vi_get_sensor_resolution(handle: *mut c_void, res: *mut video_resolution) -> i32;
        }
        unsafe { ak_vi_get_sensor_resolution(handle, res) }
    }

    #[cfg(use_stubs)]
    fn vi_get_sensor_resolution(&self, _handle: *mut c_void, res: *mut video_resolution) -> i32 {
        unsafe {
            (*res).width = 1920;
            (*res).height = 1080;
            (*res).max_width = 1920;
            (*res).max_height = 1080;
        }
        AK_SUCCESS_I32
    }

    #[cfg(not(use_stubs))]
    fn vi_set_channel_attr(&self, handle: *mut c_void, attr: *const video_channel_attr) -> i32 {
        unsafe extern "C" {
            fn ak_vi_set_channel_attr(handle: *mut c_void, attr: *const video_channel_attr) -> i32;
        }
        unsafe { ak_vi_set_channel_attr(handle, attr) }
    }

    #[cfg(use_stubs)]
    fn vi_set_channel_attr(&self, _handle: *mut c_void, _attr: *const video_channel_attr) -> i32 {
        AK_SUCCESS_I32
    }

    #[cfg(not(use_stubs))]
    fn vi_capture_on(&self, handle: *mut c_void) -> i32 {
        unsafe extern "C" {
            fn ak_vi_capture_on(handle: *mut c_void) -> i32;
        }
        unsafe { ak_vi_capture_on(handle) }
    }

    #[cfg(use_stubs)]
    fn vi_capture_on(&self, _handle: *mut c_void) -> i32 {
        AK_SUCCESS_I32
    }

    #[cfg(not(use_stubs))]
    fn venc_open(&self, param: *const encode_param) -> *mut c_void {
        unsafe extern "C" {
            fn ak_venc_open(param: *const encode_param) -> *mut c_void;
        }
        unsafe { ak_venc_open(param) }
    }

    #[cfg(use_stubs)]
    fn venc_open(&self, _param: *const encode_param) -> *mut c_void {
        std::ptr::NonNull::<c_void>::dangling().as_ptr()
    }

    #[cfg(not(use_stubs))]
    fn venc_close(&self, handle: *mut c_void) -> i32 {
        unsafe extern "C" {
            fn ak_venc_close(handle: *mut c_void) -> i32;
        }
        unsafe { ak_venc_close(handle) }
    }

    #[cfg(use_stubs)]
    fn venc_close(&self, _handle: *mut c_void) -> i32 {
        AK_SUCCESS_I32
    }

    #[cfg(not(use_stubs))]
    fn venc_set_rc(&self, enc_handle: *mut c_void, bps: i32) -> i32 {
        unsafe extern "C" {
            fn ak_venc_set_rc(enc_handle: *mut c_void, bps: i32) -> i32;
        }
        unsafe { ak_venc_set_rc(enc_handle, bps) }
    }

    #[cfg(use_stubs)]
    fn venc_set_rc(&self, _enc_handle: *mut c_void, _bps: i32) -> i32 {
        AK_SUCCESS_I32
    }

    #[cfg(not(use_stubs))]
    fn venc_set_iframe(&self, enc_handle: *mut c_void) -> i32 {
        unsafe extern "C" {
            fn ak_venc_set_iframe(enc_handle: *mut c_void) -> i32;
        }
        unsafe { ak_venc_set_iframe(enc_handle) }
    }

    #[cfg(use_stubs)]
    fn venc_set_iframe(&self, _enc_handle: *mut c_void) -> i32 {
        AK_SUCCESS_I32
    }
}

// Global instance for default FFI implementation
static REAL_VIDEO_FFI: RealVideoFfi = RealVideoFfi;

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

/// RAII handle for video input device.
///
/// This handle automatically closes the video input device when dropped,
/// ensuring proper resource cleanup even in error paths.
///
/// # Thread Safety
///
/// The underlying SDK uses internal mutexes for thread safety, so this handle
/// is safe to send and share between threads.
pub struct VideoInputHandle {
    handle: Option<*mut c_void>,
}

// SAFETY: VideoInputHandle is thread-safe - SDK uses internal mutexes.
// See anyka_sdk.rs for detailed documentation of SDK thread safety.
unsafe impl Send for VideoInputHandle {}
unsafe impl Sync for VideoInputHandle {}

impl Drop for VideoInputHandle {
    fn drop(&mut self) {
        if let Some(handle) = self.handle
            && !handle.is_null()
        {
            let _ = REAL_VIDEO_FFI.vi_close(handle);
        }
    }
}

impl VideoInputHandle {
    /// Get the raw handle pointer.
    ///
    /// Get the underlying FFI handle as a raw pointer.
    ///
    /// When `self.handle` is `None` (which occurs when created via `test_handle()` for testing),
    /// this returns `std::ptr::null_mut()`. Callers must NOT pass this null pointer to FFI
    /// functions expecting a valid handle, as this would result in undefined behavior.
    ///
    /// # Examples
    ///
    /// The test helper `test_handle()` can produce null handles for testing scenarios where
    /// mock FFI backends are used:
    ///
    /// ```no_run
    /// # #[cfg(test)]
    /// # {
    /// let test_handle = VideoInputHandle::test_handle();
    /// let ptr = test_handle.as_ptr();
    /// // ptr is now std::ptr::null_mut() - never pass this to actual FFI functions!
    /// # }
    /// ```
    ///
    /// # Safety
    ///
    /// The returned pointer is only valid while the handle is alive.
    /// Do not use after the handle is dropped.
    pub(crate) fn as_ptr(&self) -> *mut c_void {
        self.handle.unwrap_or(std::ptr::null_mut())
    }

    /// Create a test handle that will not be closed on drop.
    ///
    /// Used for testing with mock FFI backends.
    #[cfg(test)]
    pub(crate) fn test_handle() -> Self {
        Self { handle: None }
    }
}

/// Internal helper that takes FFI trait for testability.
pub(crate) fn video_input_open_internal(
    device: VideoDevice,
    ffi: &dyn VideoFfiTrait,
) -> PlatformResult<VideoInputHandle> {
    // Validate device ID before transmuting to prevent invalid values from reaching SDK
    let sdk_device: video_dev_type = if device == VideoDevice::DEV0 {
        unsafe { std::mem::transmute::<i32, video_dev_type>(0i32) }
    } else {
        return Err(PlatformError::InvalidParameter(format!(
            "Invalid video device ID: {}. Only VideoDevice::DEV0 is supported",
            device.0
        )));
    };

    let handle = ffi.vi_open(sdk_device);

    if handle.is_null() {
        Err(PlatformError::HardwareUnavailable(
            "Video input device".to_string(),
        ))
    } else {
        Ok(VideoInputHandle {
            handle: Some(handle),
        })
    }
}

/// Open a video input device.
///
/// # Arguments
///
/// * `device` - Video device identifier (typically `VideoDevice::DEV0`)
///
/// # Returns
///
/// * `Ok(VideoInputHandle)` on success
/// * `Err(PlatformError::HardwareUnavailable)` if device cannot be opened
///
/// # Safety
///
/// The FFI call is safe because:
/// - `ak_vi_open()` returns a handle or NULL
/// - We validate the result (null check)
/// - Handle is wrapped in `VideoInputHandle` for RAII cleanup
pub fn video_input_open(device: VideoDevice) -> PlatformResult<VideoInputHandle> {
    video_input_open_internal(device, &REAL_VIDEO_FFI)
}

/// Internal helper that takes FFI trait for testability.
pub(crate) fn video_input_get_sensor_resolution_internal(
    handle: &VideoInputHandle,
    ffi: &dyn VideoFfiTrait,
) -> PlatformResult<Resolution> {
    let mut res = video_resolution {
        width: 0,
        height: 0,
        max_width: 0,
        max_height: 0,
    };

    let ret = ffi.vi_get_sensor_resolution(handle.as_ptr(), &mut res);

    check_result(ret, "ak_vi_get_sensor_resolution")?;

    // Validate resolution values are non-negative before casting to u32
    let width = u32::try_from(res.width)
        .map_err(|_| PlatformError::InvalidParameter("width must be non-negative".to_string()))?;
    let height = u32::try_from(res.height)
        .map_err(|_| PlatformError::InvalidParameter("height must be non-negative".to_string()))?;

    Ok(Resolution { width, height })
}

/// Get sensor resolution.
///
/// # Arguments
///
/// * `handle` - Video input handle
///
/// # Returns
///
/// * `Ok(Resolution)` with sensor resolution on success
/// * `Err(PlatformError::HardwareFailure)` on SDK error
pub fn video_input_get_sensor_resolution(handle: &VideoInputHandle) -> PlatformResult<Resolution> {
    video_input_get_sensor_resolution_internal(handle, &REAL_VIDEO_FFI)
}

/// Internal helper that takes FFI trait for testability.
pub(crate) fn video_input_set_channel_attr_internal(
    handle: &VideoInputHandle,
    attr: &video_channel_attr,
    ffi: &dyn VideoFfiTrait,
) -> PlatformResult<()> {
    let ret = ffi.vi_set_channel_attr(handle.as_ptr(), attr);
    check_result(ret, "ak_vi_set_channel_attr")
}

/// Set video channel attributes.
///
/// # Arguments
///
/// * `handle` - Video input handle
/// * `attr` - Channel attributes to set
///
/// # Returns
///
/// * `Ok(())` on success
/// * `Err(PlatformError::HardwareFailure)` on SDK error
pub fn video_input_set_channel_attr(
    handle: &VideoInputHandle,
    attr: &video_channel_attr,
) -> PlatformResult<()> {
    video_input_set_channel_attr_internal(handle, attr, &REAL_VIDEO_FFI)
}

/// Internal helper that takes FFI trait for testability.
///
/// Loads the ISP sensor configuration from the specified path. This must be
/// called before `ak_vi_open()` so the ISP subsystem has a valid config buffer.
pub(crate) fn video_input_match_sensor_internal(
    config_path: &Path,
    ffi: &dyn VideoFfiTrait,
) -> PlatformResult<()> {
    let path_str = config_path.to_str().ok_or_else(|| {
        PlatformError::InvalidParameter("ISP config path is not valid UTF-8".to_string())
    })?;

    let c_path = CString::new(path_str).map_err(|_| {
        PlatformError::InvalidParameter("ISP config path contains null byte".to_string())
    })?;

    let ret = ffi.vi_match_sensor(c_path.as_ptr());
    check_result(ret, "ak_vi_match_sensor")
}

/// Load ISP sensor configuration for the video input subsystem.
///
/// This must be called **before** `video_input_open()` to load the binary ISP
/// configuration that the sensor requires. Without this call, `ak_vi_open()`
/// will fail because `isp_init()` expects the config buffer to already be loaded.
///
/// # Arguments
///
/// * `config_path` - Path to the binary ISP config file (e.g., `isp_gc1084.conf`)
///
/// # Errors
///
/// * `PlatformError::InvalidParameter` if the path is not valid UTF-8 or contains null bytes
/// * `PlatformError::HardwareFailure` if the SDK rejects the config file
pub fn video_input_match_sensor(config_path: &Path) -> PlatformResult<()> {
    video_input_match_sensor_internal(config_path, &REAL_VIDEO_FFI)
}

/// Internal helper that takes FFI trait for testability.
pub(crate) fn video_input_capture_on_internal(
    handle: &VideoInputHandle,
    ffi: &dyn VideoFfiTrait,
) -> PlatformResult<()> {
    let ret = ffi.vi_capture_on(handle.as_ptr());
    check_result(ret, "ak_vi_capture_on")
}

/// Start the ISP capture pipeline on the video input device.
///
/// This must be called **after** `video_input_set_channel_attr()` to begin
/// the image capture pipeline. The SDK vendor code calls this as the final
/// step of video input initialization.
///
/// # Arguments
///
/// * `handle` - Video input handle from a successful `video_input_open()` call
///
/// # Errors
///
/// * `PlatformError::HardwareFailure` if the SDK call fails
pub fn video_input_capture_on(handle: &VideoInputHandle) -> PlatformResult<()> {
    video_input_capture_on_internal(handle, &REAL_VIDEO_FFI)
}

/// RAII handle for video encoder.
///
/// This handle automatically closes the video encoder when dropped,
/// ensuring proper resource cleanup even in error paths.
///
/// # Thread Safety
///
/// The underlying SDK uses internal mutexes for thread safety, so this handle
/// is safe to send and share between threads.
pub struct VideoEncoderHandle {
    handle: Option<*mut c_void>,
}

// SAFETY: VideoEncoderHandle is thread-safe - SDK uses internal mutexes.
// Similar to VideoInputHandle, the SDK provides internal synchronization.
unsafe impl Send for VideoEncoderHandle {}
unsafe impl Sync for VideoEncoderHandle {}

impl Drop for VideoEncoderHandle {
    fn drop(&mut self) {
        if let Some(handle) = self.handle
            && !handle.is_null()
        {
            let _ = REAL_VIDEO_FFI.venc_close(handle);
        }
    }
}

impl VideoEncoderHandle {
    /// Get the raw handle pointer.
    ///
    /// # Safety
    ///
    /// The returned pointer is only valid while the handle is alive.
    /// Do not use after the handle is dropped.
    pub(crate) fn as_ptr(&self) -> *mut c_void {
        self.handle.unwrap_or(std::ptr::null_mut())
    }
}

/// Internal helper that takes FFI trait for testability.
pub(crate) fn video_encoder_open_internal(
    param: &encode_param,
    ffi: &dyn VideoFfiTrait,
) -> PlatformResult<VideoEncoderHandle> {
    let handle = ffi.venc_open(param);

    if handle.is_null() {
        Err(PlatformError::HardwareUnavailable(
            "Video encoder".to_string(),
        ))
    } else {
        Ok(VideoEncoderHandle {
            handle: Some(handle),
        })
    }
}

/// Open a video encoder.
///
/// # Arguments
///
/// * `param` - Encoding parameters
///
/// # Returns
///
/// * `Ok(VideoEncoderHandle)` on success
/// * `Err(PlatformError::HardwareUnavailable)` if encoder cannot be opened
///
/// # Safety
///
/// The FFI call is safe because:
/// - `ak_venc_open()` returns a handle or NULL
/// - We validate the result (null check)
/// - Handle is wrapped in `VideoEncoderHandle` for RAII cleanup
pub fn video_encoder_open(param: &encode_param) -> PlatformResult<VideoEncoderHandle> {
    video_encoder_open_internal(param, &REAL_VIDEO_FFI)
}

/// Internal helper that takes FFI trait for testability.
pub(crate) fn video_encoder_set_rc_internal(
    handle: &VideoEncoderHandle,
    bps: i32,
    ffi: &dyn VideoFfiTrait,
) -> PlatformResult<()> {
    if bps <= 0 {
        return Err(PlatformError::InvalidParameter(
            "bitrate must be positive".to_string(),
        ));
    }
    let ret = ffi.venc_set_rc(handle.as_ptr(), bps);
    check_result(ret, "ak_venc_set_rc")
}

/// Set video encoder rate control (bitrate).
///
/// # Arguments
///
/// * `handle` - Video encoder handle
/// * `bps` - Target bitrate in bits per second
///
/// # Returns
///
/// * `Ok(())` on success
/// * `Err(PlatformError::HardwareFailure)` on SDK error
pub fn video_encoder_set_rc(handle: &VideoEncoderHandle, bps: i32) -> PlatformResult<()> {
    video_encoder_set_rc_internal(handle, bps, &REAL_VIDEO_FFI)
}

/// Internal helper that takes FFI trait for testability.
pub(crate) fn video_encoder_request_idr_internal(
    handle: &VideoEncoderHandle,
    ffi: &dyn VideoFfiTrait,
) -> PlatformResult<()> {
    let ret = ffi.venc_set_iframe(handle.as_ptr());
    check_result(ret, "ak_venc_set_iframe")
}

/// Request next frame to be an I-frame (IDR frame).
///
/// This is equivalent to `ak_venc_set_iframe()` in the SDK, which sets
/// the next encoded frame to be an I-frame.
///
/// # Arguments
///
/// * `handle` - Video encoder handle
///
/// # Returns
///
/// * `Ok(())` on success
/// * `Err(PlatformError::HardwareFailure)` on SDK error
pub fn video_encoder_request_idr(handle: &VideoEncoderHandle) -> PlatformResult<()> {
    video_encoder_request_idr_internal(handle, &REAL_VIDEO_FFI)
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
    fn test_check_result_unknown_error() {
        let result = check_result(-42, "test_function");
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareFailure(msg)) => {
                assert!(msg.contains("test_function"));
                assert!(msg.contains("error code -42"));
            }
            _ => panic!("Expected HardwareFailure error"),
        }
    }

    #[test]
    #[cfg(use_stubs)]
    fn test_video_input_open_success() {
        let result = video_input_open(VideoDevice::DEV0);
        assert!(result.is_ok());
        let handle = result.unwrap();
        assert!(!handle.as_ptr().is_null());
    }

    #[test]
    #[cfg(use_stubs)]
    fn test_video_input_get_sensor_resolution_success() {
        let handle = video_input_open(VideoDevice::DEV0).unwrap();
        let result = video_input_get_sensor_resolution(&handle);
        assert!(result.is_ok());
        let res = result.unwrap();
        assert_eq!(res.width, 1920);
        assert_eq!(res.height, 1080);
    }

    #[test]
    #[cfg(use_stubs)]
    fn test_video_encoder_open_success() {
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
        let result = video_encoder_open(&param);
        assert!(result.is_ok());
        let handle = result.unwrap();
        assert!(!handle.as_ptr().is_null());
    }

    #[test]
    #[cfg(use_stubs)]
    fn test_video_encoder_set_rc_success() {
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
        let handle = video_encoder_open(&param).unwrap();
        let result = video_encoder_set_rc(&handle, 3000000);
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(use_stubs)]
    fn test_video_encoder_request_idr_success() {
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
        let handle = video_encoder_open(&param).unwrap();
        let result = video_encoder_request_idr(&handle);
        assert!(result.is_ok());
    }

    fn video_dev0() -> video_dev_type {
        #[cfg(use_stubs)]
        {
            video_dev_type::Dev0
        }
        #[cfg(not(use_stubs))]
        {
            video_dev_type::VIDEO_DEV0
        }
    }

    // Mockall-based tests for wrapper functions
    #[test]
    fn test_video_input_open_internal_calls_ffi_and_returns_handle() {
        let mut mock_ffi = MockVideoFfiTrait::new();
        let test_handle = std::ptr::NonNull::<c_void>::dangling().as_ptr();

        let test_handle_usize = test_handle as usize;
        mock_ffi
            .expect_vi_open()
            .with(eq(video_dev0()))
            .times(1)
            .returning(move |_| test_handle_usize as *mut c_void);

        let result = video_input_open_internal(VideoDevice::DEV0, &mock_ffi);
        assert!(result.is_ok());
        let handle = result.unwrap();
        assert_eq!(handle.as_ptr(), test_handle);
    }

    #[test]
    fn test_video_input_open_internal_returns_error_on_null_handle() {
        let mut mock_ffi = MockVideoFfiTrait::new();

        mock_ffi
            .expect_vi_open()
            .with(eq(video_dev0()))
            .times(1)
            .returning(|_| std::ptr::null_mut());

        let result = video_input_open_internal(VideoDevice::DEV0, &mock_ffi);
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareUnavailable(_)) => {}
            _ => panic!("Expected HardwareUnavailable error"),
        }
    }

    #[test]
    fn test_video_input_open_internal_rejects_invalid_device() {
        let mock_ffi = MockVideoFfiTrait::new();

        // Try with invalid device ID (not DEV0)
        let invalid_device = VideoDevice(1);
        let result = video_input_open_internal(invalid_device, &mock_ffi);
        assert!(result.is_err());
        match result {
            Err(PlatformError::InvalidParameter(msg)) => {
                assert!(msg.contains("Invalid video device ID"));
                assert!(msg.contains("1"));
            }
            _ => panic!("Expected InvalidParameter error"),
        }
    }

    #[test]
    fn test_video_input_get_sensor_resolution_internal_calls_ffi_and_returns_resolution() {
        let mut mock_ffi = MockVideoFfiTrait::new();
        let vi_handle = VideoInputHandle::test_handle();

        mock_ffi
            .expect_vi_get_sensor_resolution()
            .withf(|handle, _| handle.is_null())
            .times(1)
            .returning(|_, res| {
                unsafe {
                    (*res).width = 1920;
                    (*res).height = 1080;
                    (*res).max_width = 1920;
                    (*res).max_height = 1080;
                }
                AK_SUCCESS_I32
            });

        let result = video_input_get_sensor_resolution_internal(&vi_handle, &mock_ffi);
        assert!(result.is_ok());
        let res = result.unwrap();
        assert_eq!(res.width, 1920);
        assert_eq!(res.height, 1080);
    }

    #[test]
    fn test_video_input_get_sensor_resolution_internal_propagates_error() {
        let mut mock_ffi = MockVideoFfiTrait::new();
        let vi_handle = VideoInputHandle::test_handle();

        mock_ffi
            .expect_vi_get_sensor_resolution()
            .withf(|handle, _| handle.is_null())
            .times(1)
            .returning(|_, _| AK_FAILED_I32);

        let result = video_input_get_sensor_resolution_internal(&vi_handle, &mock_ffi);
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareFailure(msg)) => {
                assert!(msg.contains("ak_vi_get_sensor_resolution"));
            }
            _ => panic!("Expected HardwareFailure error"),
        }
    }

    #[test]
    fn test_video_input_set_channel_attr_internal_calls_ffi() {
        let mut mock_ffi = MockVideoFfiTrait::new();
        let vi_handle = VideoInputHandle::test_handle();
        let attr = video_channel_attr::default();

        mock_ffi
            .expect_vi_set_channel_attr()
            .withf(|handle, _| handle.is_null())
            .times(1)
            .returning(|_, _| AK_SUCCESS_I32);

        let result = video_input_set_channel_attr_internal(&vi_handle, &attr, &mock_ffi);
        assert!(result.is_ok());
    }

    #[test]
    fn test_video_input_set_channel_attr_internal_propagates_error() {
        let mut mock_ffi = MockVideoFfiTrait::new();
        let vi_handle = VideoInputHandle::test_handle();
        let attr = video_channel_attr::default();

        mock_ffi
            .expect_vi_set_channel_attr()
            .withf(|handle, _| handle.is_null())
            .times(1)
            .returning(|_, _| AK_FAILED_I32);

        let result = video_input_set_channel_attr_internal(&vi_handle, &attr, &mock_ffi);
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareFailure(msg)) => {
                assert!(msg.contains("ak_vi_set_channel_attr"));
            }
            _ => panic!("Expected HardwareFailure error"),
        }
    }

    #[test]
    fn test_video_encoder_open_internal_calls_ffi_and_returns_handle() {
        let mut mock_ffi = MockVideoFfiTrait::new();
        let test_handle = std::ptr::NonNull::<c_void>::dangling().as_ptr();
        let mut param = encode_param::default();
        param.width = 1920;
        param.height = 1080;
        param.minqp = 10;
        param.maxqp = 51;
        param.fps = 30;
        param.goplen = 30;
        param.bps = 2000000;

        let test_handle_usize = test_handle as usize;
        mock_ffi
            .expect_venc_open()
            .times(1)
            .returning(move |_| test_handle_usize as *mut c_void);

        let result = video_encoder_open_internal(&param, &mock_ffi);
        assert!(result.is_ok());
        let handle = result.unwrap();
        assert_eq!(handle.as_ptr(), test_handle);
    }

    #[test]
    fn test_video_encoder_open_internal_returns_error_on_null_handle() {
        let mut mock_ffi = MockVideoFfiTrait::new();
        let mut param = encode_param::default();
        param.width = 1920;
        param.height = 1080;
        param.minqp = 10;
        param.maxqp = 51;
        param.fps = 30;
        param.goplen = 30;
        param.bps = 2000000;

        mock_ffi
            .expect_venc_open()
            .times(1)
            .returning(|_| std::ptr::null_mut());

        let result = video_encoder_open_internal(&param, &mock_ffi);
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareUnavailable(_)) => {}
            _ => panic!("Expected HardwareUnavailable error"),
        }
    }

    #[test]
    fn test_video_encoder_set_rc_internal_calls_ffi() {
        let mut mock_ffi = MockVideoFfiTrait::new();
        let enc_handle = VideoEncoderHandle {
            handle: None, // Use None to prevent Drop from calling venc_close on dangling pointer
        };

        mock_ffi
            .expect_venc_set_rc()
            .withf(|handle, bps| handle.is_null() && *bps == 3000000)
            .times(1)
            .returning(|_, _| AK_SUCCESS_I32);

        let result = video_encoder_set_rc_internal(&enc_handle, 3000000, &mock_ffi);
        assert!(result.is_ok());
    }

    #[test]
    fn test_video_encoder_set_rc_internal_propagates_error() {
        let mut mock_ffi = MockVideoFfiTrait::new();
        let enc_handle = VideoEncoderHandle {
            handle: None, // Use None to prevent Drop from calling venc_close on dangling pointer
        };

        mock_ffi
            .expect_venc_set_rc()
            .withf(|handle, _| handle.is_null())
            .times(1)
            .returning(|_, _| AK_FAILED_I32);

        let result = video_encoder_set_rc_internal(&enc_handle, 3000000, &mock_ffi);
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareFailure(msg)) => {
                assert!(msg.contains("ak_venc_set_rc"));
            }
            _ => panic!("Expected HardwareFailure error"),
        }
    }

    #[test]
    fn test_video_encoder_request_idr_internal_calls_ffi() {
        let mut mock_ffi = MockVideoFfiTrait::new();
        let enc_handle = VideoEncoderHandle {
            handle: None, // Use None to prevent Drop from calling venc_close on dangling pointer
        };

        mock_ffi
            .expect_venc_set_iframe()
            .withf(|handle| handle.is_null())
            .times(1)
            .returning(|_| AK_SUCCESS_I32);

        let result = video_encoder_request_idr_internal(&enc_handle, &mock_ffi);
        assert!(result.is_ok());
    }

    #[test]
    fn test_video_encoder_request_idr_internal_propagates_error() {
        let mut mock_ffi = MockVideoFfiTrait::new();
        let enc_handle = VideoEncoderHandle {
            handle: None, // Use None to prevent Drop from calling venc_close on dangling pointer
        };

        mock_ffi
            .expect_venc_set_iframe()
            .withf(|handle| handle.is_null())
            .times(1)
            .returning(|_| AK_FAILED_I32);

        let result = video_encoder_request_idr_internal(&enc_handle, &mock_ffi);
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareFailure(msg)) => {
                assert!(msg.contains("ak_venc_set_iframe"));
            }
            _ => panic!("Expected HardwareFailure error"),
        }
    }

    // ---- Tests for video_input_match_sensor_internal ----

    #[test]
    fn test_video_input_match_sensor_internal_success() {
        let mut mock_ffi = MockVideoFfiTrait::new();

        mock_ffi
            .expect_vi_match_sensor()
            .times(1)
            .returning(|_| AK_SUCCESS_I32);

        let path = Path::new("/tmp/sensor/isp_gc1084.conf");
        let result = video_input_match_sensor_internal(path, &mock_ffi);
        assert!(result.is_ok());
    }

    #[test]
    fn test_video_input_match_sensor_internal_ffi_failure() {
        let mut mock_ffi = MockVideoFfiTrait::new();

        mock_ffi
            .expect_vi_match_sensor()
            .times(1)
            .returning(|_| AK_FAILED_I32);

        let path = Path::new("/tmp/sensor/isp_gc1084.conf");
        let result = video_input_match_sensor_internal(path, &mock_ffi);
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareFailure(msg)) => {
                assert!(msg.contains("ak_vi_match_sensor"));
            }
            _ => panic!("Expected HardwareFailure error"),
        }
    }

    #[test]
    fn test_video_input_match_sensor_internal_passes_correct_path() {
        let mut mock_ffi = MockVideoFfiTrait::new();

        mock_ffi
            .expect_vi_match_sensor()
            .withf(|config_file| {
                let c_str = unsafe { std::ffi::CStr::from_ptr(*config_file) };
                c_str.to_str().unwrap() == "/etc/jffs2/isp_gc1084.conf"
            })
            .times(1)
            .returning(|_| AK_SUCCESS_I32);

        let path = Path::new("/etc/jffs2/isp_gc1084.conf");
        let result = video_input_match_sensor_internal(path, &mock_ffi);
        assert!(result.is_ok());
    }

    // ---- Tests for video_input_capture_on_internal ----

    #[test]
    fn test_video_input_capture_on_internal_success() {
        let mut mock_ffi = MockVideoFfiTrait::new();
        let vi_handle = VideoInputHandle::test_handle();

        mock_ffi
            .expect_vi_capture_on()
            .times(1)
            .returning(|_| AK_SUCCESS_I32);

        let result = video_input_capture_on_internal(&vi_handle, &mock_ffi);
        assert!(result.is_ok());
    }

    #[test]
    fn test_video_input_capture_on_internal_ffi_failure() {
        let mut mock_ffi = MockVideoFfiTrait::new();
        let vi_handle = VideoInputHandle::test_handle();

        mock_ffi
            .expect_vi_capture_on()
            .times(1)
            .returning(|_| AK_FAILED_I32);

        let result = video_input_capture_on_internal(&vi_handle, &mock_ffi);
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareFailure(msg)) => {
                assert!(msg.contains("ak_vi_capture_on"));
            }
            _ => panic!("Expected HardwareFailure error"),
        }
    }

    #[test]
    fn test_video_input_capture_on_internal_unknown_error_code() {
        let mut mock_ffi = MockVideoFfiTrait::new();
        let vi_handle = VideoInputHandle::test_handle();

        mock_ffi.expect_vi_capture_on().times(1).returning(|_| -99);

        let result = video_input_capture_on_internal(&vi_handle, &mock_ffi);
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareFailure(msg)) => {
                assert!(msg.contains("error code -99"));
            }
            _ => panic!("Expected HardwareFailure error"),
        }
    }
}
