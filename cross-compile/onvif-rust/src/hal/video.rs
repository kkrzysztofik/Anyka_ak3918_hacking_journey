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
//! use onvif_rust::hal::video::*;
//! use onvif_rust::hal::VideoDevice;
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
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering};
use std::time::Duration;

use crate::hal::{
    encode_param, video_channel_attr, video_dev_type, video_resolution, video_stream,
};

use crate::hal::{AK_FAILED_I32, AK_SUCCESS_I32, Resolution, VideoDevice};

/// Internal trait for abstracting video FFI calls to enable mocking in tests.
#[cfg_attr(test, mockall::automock)]
#[allow(dead_code)]
pub(crate) trait VideoHalTrait: Send + Sync {
    fn vi_match_sensor(&self, config_file: *const c_char) -> i32;
    fn vi_open(&self, dev: video_dev_type) -> *mut c_void;
    fn vi_close(&self, handle: *mut c_void) -> i32;
    fn vi_get_sensor_resolution(&self, handle: *mut c_void, res: *mut video_resolution) -> i32;
    fn vi_set_channel_attr(&self, handle: *mut c_void, attr: *const video_channel_attr) -> i32;
    fn vi_capture_on(&self, handle: *mut c_void) -> i32;
    fn vi_capture_off(&self, handle: *mut c_void) -> i32;
    fn vpss_init(&self, vi_handle: *mut c_void, dev: i32);
    fn vpss_destroy(&self, dev: i32);
    fn venc_set_cfg_path(&self, path: *const c_char) -> i32;
    fn venc_open(&self, param: *const encode_param) -> *mut c_void;
    fn venc_close(&self, handle: *mut c_void) -> i32;
    fn venc_set_rc(&self, enc_handle: *mut c_void, bps: i32) -> i32;
    fn venc_set_iframe(&self, enc_handle: *mut c_void) -> i32;
    fn venc_request_stream(&self, vi_handle: *mut c_void, venc_handle: *mut c_void) -> *mut c_void;
    fn venc_get_stream(&self, stream_handle: *mut c_void, stream: *mut video_stream) -> i32;
    fn venc_release_stream(&self, stream_handle: *mut c_void, stream: *mut video_stream) -> i32;
    fn venc_cancel_stream(&self, stream_handle: *mut c_void) -> i32;
    /// Get the last SDK error number (thread-local).
    fn get_error_no(&self) -> i32;
    /// Get the last SDK error string (thread-local). Returns empty string on stub.
    fn get_error_str(&self) -> String;
}

/// Default implementation that calls the real FFI functions.
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

// Global instance for default FFI implementation
static DEFAULT_VIDEO_HAL: StubVideoHal = StubVideoHal;

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
            "{} failed (AK_FAILED={})",
            context, AK_FAILED_I32
        ))),
        _ => Err(PlatformError::HardwareFailure(format!(
            "{}: error code {}",
            context, ret
        ))),
    }
}

/// Run a blocking FFI call on a dedicated thread with a timeout.
/// Returns `Ok(ret)` if the call completed within the deadline, `Err(())` if it
/// timed out. On timeout the spawned thread is detached — we cannot force-kill a
/// thread stuck in kernel I/O.
fn ffi_call_with_timeout<F>(name: &str, timeout: Duration, f: F) -> Result<i32, ()>
where
    F: FnOnce() -> i32 + Send + 'static,
{
    let (tx, rx) = std::sync::mpsc::channel();
    static FFI_TIMEOUT_THREAD_SEQ: AtomicUsize = AtomicUsize::new(0);
    let seq = FFI_TIMEOUT_THREAD_SEQ.fetch_add(1, Ordering::Relaxed) % 100;
    let base = match name {
        "ak_venc_cancel_stream" => "ffi-venc-can",
        "ak_venc_close" => "ffi-venc-clo",
        "ak_vi_close" => "ffi-vi-close",
        _ => "ffi-call",
    };
    let thread_name = format!("{}-{:02}", base, seq);
    let join_handle = match std::thread::Builder::new()
        .name(thread_name.clone())
        .spawn(move || {
            let ret = f();
            let _ = tx.send(ret);
        }) {
        Ok(handle) => handle,
        Err(e) => {
            tracing::warn!(
                thread_name = thread_name.as_str(),
                call = name,
                error = %e,
                "Failed to create named FFI timeout thread"
            );
            return Err(());
        }
    };
    match rx.recv_timeout(timeout) {
        Ok(ret) => {
            // Thread finished — join it to clean up
            let _ = join_handle.join();
            Ok(ret)
        }
        Err(_) => {
            tracing::error!("FFI call '{}' timed out after {:?}", name, timeout);
            // Detach the stuck thread
            std::mem::forget(join_handle);
            Err(())
        }
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
        // In IPC mode, handles are managed by vendor-daemon - no-op cleanup.
        // The vendor-daemon handles resource cleanup on its side.
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
    ffi: &dyn VideoHalTrait,
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
    video_input_open_internal(device, &DEFAULT_VIDEO_HAL)
}

/// Internal helper that takes FFI trait for testability.
pub(crate) fn video_input_get_sensor_resolution_internal(
    handle: &VideoInputHandle,
    ffi: &dyn VideoHalTrait,
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
    video_input_get_sensor_resolution_internal(handle, &DEFAULT_VIDEO_HAL)
}

/// Internal helper that takes FFI trait for testability.
pub(crate) fn video_input_set_channel_attr_internal(
    handle: &VideoInputHandle,
    attr: &video_channel_attr,
    ffi: &dyn VideoHalTrait,
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
    video_input_set_channel_attr_internal(handle, attr, &DEFAULT_VIDEO_HAL)
}

/// Internal helper that takes FFI trait for testability.
///
/// Loads the ISP sensor configuration from the specified path. This must be
/// called before `ak_vi_open()` so the ISP subsystem has a valid config buffer.
pub(crate) fn video_input_match_sensor_internal(
    config_path: &Path,
    ffi: &dyn VideoHalTrait,
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
    video_input_match_sensor_internal(config_path, &DEFAULT_VIDEO_HAL)
}

/// Internal helper that takes FFI trait for testability.
pub(crate) fn video_input_capture_on_internal(
    handle: &VideoInputHandle,
    ffi: &dyn VideoHalTrait,
) -> PlatformResult<()> {
    let ret = ffi.vi_capture_on(handle.as_ptr());
    check_result(ret, "ak_vi_capture_on")
}

/// Internal helper that takes FFI trait for testability.
pub(crate) fn video_input_capture_off_internal(
    handle: &VideoInputHandle,
    ffi: &dyn VideoHalTrait,
) -> PlatformResult<()> {
    let ret = ffi.vi_capture_off(handle.as_ptr());
    check_result(ret, "ak_vi_capture_off")
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
    video_input_capture_on_internal(handle, &DEFAULT_VIDEO_HAL)
}

/// Stop the ISP capture pipeline on the video input device.
///
/// This mirrors `ak_vi_capture_off()` and is used for cleanup/recovery between
/// capture start attempts.
///
/// # Arguments
///
/// * `handle` - Video input handle from a successful `video_input_open()` call
///
/// # Errors
///
/// * `PlatformError::HardwareFailure` if the SDK call fails
pub fn video_input_capture_off(handle: &VideoInputHandle) -> PlatformResult<()> {
    video_input_capture_off_internal(handle, &DEFAULT_VIDEO_HAL)
}

/// Internal helper that takes FFI trait for testability.
///
/// Initializes the Video Post-Processing SubSystem (VPSS).
/// This MUST be called immediately after `video_input_open()` and
/// BEFORE any other video operations.
pub(crate) fn vpss_init_internal(
    handle: &VideoInputHandle,
    device: VideoDevice,
    ffi: &dyn VideoHalTrait,
) -> PlatformResult<()> {
    let dev_id = if device == VideoDevice::DEV0 {
        0i32
    } else {
        return Err(PlatformError::InvalidParameter(format!(
            "Invalid video device ID: {}. Only VideoDevice::DEV0 is supported",
            device.0
        )));
    };

    ffi.vpss_init(handle.as_ptr(), dev_id);
    Ok(())
}

/// Initialize the Video Post-Processing SubSystem (VPSS).
///
/// This function MUST be called immediately after `video_input_open()` and
/// BEFORE any other video input operations. The reference implementation shows
/// this is a critical initialization step that sets up the ISP processing pipeline.
///
/// # Arguments
///
/// * `handle` - Video input handle from a successful `video_input_open()` call
/// * `device` - Video device identifier (must match the device used in `video_input_open()`)
///
/// # Errors
///
/// * `PlatformError::InvalidParameter` if device ID is invalid
///
/// # Safety
///
/// The FFI call is safe because:
/// - `ak_vpss_init()` is called with a valid vi_handle from `ak_vi_open()`
/// - The function returns void (no error codes to validate)
/// - VPSS state is managed internally by the SDK
///
/// # Example Initialization Sequence
///
/// ```no_run
/// use onvif_rust::hal::{VideoDevice, video_input_open, vpss_init};
///
/// // Step 1: Open video input
/// let vi_handle = video_input_open(VideoDevice::DEV0)?;
///
/// // Step 2: Initialize VPSS (CRITICAL - must be called before other operations)
/// vpss_init(&vi_handle, VideoDevice::DEV0)?;
///
/// // Step 3: Continue with sensor resolution query, channel config, etc.
/// # Ok::<(), onvif_rust::platform::PlatformError>(())
/// ```
pub fn vpss_init(handle: &VideoInputHandle, device: VideoDevice) -> PlatformResult<()> {
    vpss_init_internal(handle, device, &DEFAULT_VIDEO_HAL)
}

/// Internal helper that takes FFI trait for testability.
///
/// Destroys the Video Post-Processing SubSystem (VPSS).
/// This MUST be called BEFORE `video_input_close()` during cleanup.
pub(crate) fn vpss_destroy_internal(
    device: VideoDevice,
    ffi: &dyn VideoHalTrait,
) -> PlatformResult<()> {
    let dev_id = if device == VideoDevice::DEV0 {
        0i32
    } else {
        return Err(PlatformError::InvalidParameter(format!(
            "Invalid video device ID: {}. Only VideoDevice::DEV0 is supported",
            device.0
        )));
    };

    ffi.vpss_destroy(dev_id);
    Ok(())
}

/// Destroy the Video Post-Processing SubSystem (VPSS).
///
/// This function MUST be called BEFORE closing the video input device during
/// cleanup/shutdown. The reference implementation shows this must be done in
/// the correct order to avoid resource leaks.
///
/// # Arguments
///
/// * `device` - Video device identifier (must match the device used in initialization)
///
/// # Errors
///
/// * `PlatformError::InvalidParameter` if device ID is invalid
///
/// # Safety
///
/// The FFI call is safe because:
/// - `ak_vpss_destroy()` takes only a device enum (no pointer validation needed)
/// - The function returns void (no error codes to validate)
/// - VPSS state cleanup is managed internally by the SDK
///
/// # Example Cleanup Sequence
///
/// ```no_run
/// use onvif_rust::hal::{VideoDevice, vpss_destroy};
///
/// // Step 1: Destroy VPSS BEFORE closing video input
/// vpss_destroy(VideoDevice::DEV0)?;
///
/// // Step 2: Close video input (handle Drop will call ak_vi_close)
/// drop(vi_handle);
/// # Ok::<(), onvif_rust::platform::PlatformError>(())
/// ```
pub fn vpss_destroy(device: VideoDevice) -> PlatformResult<()> {
    vpss_destroy_internal(device, &DEFAULT_VIDEO_HAL)
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
    closed: AtomicBool,
}

// SAFETY: VideoEncoderHandle is thread-safe - SDK uses internal mutexes.
// Similar to VideoInputHandle, the SDK provides internal synchronization.
unsafe impl Send for VideoEncoderHandle {}
unsafe impl Sync for VideoEncoderHandle {}

impl Drop for VideoEncoderHandle {
    fn drop(&mut self) {
        // In IPC mode, handles are managed by vendor-daemon - no-op cleanup.
        // The vendor-daemon handles resource cleanup on its side.
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

    /// Explicitly close encoder in the current thread.
    ///
    /// This path is used by platform shutdown, where we want strict call ordering
    /// and no detached timeout threads for `ak_venc_close()`.
    pub(crate) fn close_blocking_with_ffi(&self, ffi: &dyn VideoHalTrait) -> PlatformResult<()> {
        if self.closed.swap(true, Ordering::SeqCst) {
            return Ok(());
        }

        let Some(handle) = self.handle else {
            return Ok(());
        };
        if handle.is_null() {
            return Ok(());
        }

        let ret = ffi.venc_close(handle);
        check_result(ret, "ak_venc_close")
    }
}

/// RAII handle for a video stream (bound VI + encoder pair).
///
/// Created by `ak_venc_request_stream()` and automatically cancelled on drop
/// via `ak_venc_cancel_stream()`. The stream handle is used with
/// `ak_venc_get_stream()` / `ak_venc_release_stream()` to poll encoded frames.
///
/// # Thread Safety
///
/// The underlying SDK uses internal mutexes for thread safety.
pub struct VideoStreamHandle {
    handle: *mut c_void,
    ffi: Arc<dyn VideoHalTrait>,
    cancel_state: AtomicU8,
}

const CANCEL_STATE_ACTIVE: u8 = 0;
const CANCEL_STATE_PENDING: u8 = 1;
const CANCEL_STATE_DONE: u8 = 2;
const CANCEL_STATE_UNKNOWN: u8 = 3;

// SAFETY: VideoStreamHandle is thread-safe - SDK uses internal mutexes.
unsafe impl Send for VideoStreamHandle {}
unsafe impl Sync for VideoStreamHandle {}

impl Drop for VideoStreamHandle {
    fn drop(&mut self) {
        let _ = self.cancel_with_timeout(Duration::from_secs(2));
    }
}

impl VideoStreamHandle {
    /// Create a new stream handle by binding a video input and encoder.
    ///
    /// Calls `ak_venc_request_stream()` which starts the hardware encode pipeline.
    pub(crate) fn new(
        vi_handle: *mut c_void,
        venc_handle: *mut c_void,
        ffi: Arc<dyn VideoHalTrait>,
    ) -> PlatformResult<Self> {
        tracing::debug!(
            vi_handle = ?vi_handle,
            venc_handle = ?venc_handle,
            "Requesting video stream from SDK"
        );
        let handle = ffi.venc_request_stream(vi_handle, venc_handle);
        if handle.is_null() {
            tracing::error!(
                vi_handle = ?vi_handle,
                venc_handle = ?venc_handle,
                "venc_request_stream returned null"
            );
            return Err(PlatformError::HardwareFailure(
                "ak_venc_request_stream returned null".to_string(),
            ));
        }
        tracing::debug!(
            stream_handle = ?handle,
            "Video stream requested successfully"
        );
        Ok(Self {
            handle,
            ffi,
            cancel_state: AtomicU8::new(CANCEL_STATE_ACTIVE),
        })
    }

    /// Explicitly cancel the stream to unblock pending `ak_venc_get_stream()` calls.
    ///
    /// Idempotent — safe to call multiple times; only the first call invokes the SDK.
    /// Returns `true` when the cancel call completed (success or SDK error), `false` on
    /// timeout.
    pub fn cancel(&self) -> bool {
        self.cancel_with_timeout(Duration::from_secs(2))
    }

    /// Explicitly cancel the stream with a caller-specified timeout.
    ///
    /// This is used by shutdown paths that must not block indefinitely when vendor
    /// SDK calls are stuck in kernel I/O.
    pub fn cancel_with_timeout(&self, timeout: Duration) -> bool {
        let state = self.cancel_state.load(Ordering::SeqCst);
        if state == CANCEL_STATE_DONE {
            return true;
        }
        if state == CANCEL_STATE_UNKNOWN || state == CANCEL_STATE_PENDING {
            return false;
        }
        if self
            .cancel_state
            .compare_exchange(
                CANCEL_STATE_ACTIVE,
                CANCEL_STATE_PENDING,
                Ordering::SeqCst,
                Ordering::SeqCst,
            )
            .is_err()
        {
            return false;
        }

        if self.handle.is_null() {
            self.cancel_state.store(CANCEL_STATE_DONE, Ordering::SeqCst);
            return true;
        }

        let handle_ptr = self.handle as usize; // Copy for Send
        let ffi = Arc::clone(&self.ffi);
        tracing::info!(stream_handle = ?self.handle, "Cancelling video stream");
        match ffi_call_with_timeout("ak_venc_cancel_stream", timeout, move || {
            ffi.venc_cancel_stream(handle_ptr as *mut c_void)
        }) {
            Ok(ret) => {
                self.cancel_state.store(CANCEL_STATE_DONE, Ordering::SeqCst);
                if ret != AK_SUCCESS_I32 {
                    tracing::warn!(
                        stream_handle = ?self.handle,
                        error_code = ret,
                        "ak_venc_cancel_stream failed during explicit cancel"
                    );
                }
                true
            }
            Err(()) => {
                // Timeout means we do not know whether the SDK consumed the handle.
                // Keep this as unknown rather than "done" so callers can make an
                // explicit hard-exit decision instead of assuming success.
                self.cancel_state
                    .store(CANCEL_STATE_UNKNOWN, Ordering::SeqCst);
                false
            }
        }
    }

    /// Cancel the stream with a timeout and return a checked result.
    ///
    /// This is the preferred shutdown path: it is timeout-bounded and surfaces
    /// non-success SDK return codes as errors so callers can decide whether to
    /// require a hard process termination.
    pub fn cancel_checked_with_timeout(&self, timeout: Duration) -> PlatformResult<()> {
        let state = self.cancel_state.load(Ordering::SeqCst);
        if state == CANCEL_STATE_DONE {
            return Ok(());
        }
        if state == CANCEL_STATE_PENDING || state == CANCEL_STATE_UNKNOWN {
            return Err(PlatformError::HardwareFailure(
                "stream cancel state is indeterminate".to_string(),
            ));
        }
        if self
            .cancel_state
            .compare_exchange(
                CANCEL_STATE_ACTIVE,
                CANCEL_STATE_PENDING,
                Ordering::SeqCst,
                Ordering::SeqCst,
            )
            .is_err()
        {
            return Err(PlatformError::HardwareFailure(
                "failed to enter stream cancel state".to_string(),
            ));
        }

        if self.handle.is_null() {
            self.cancel_state.store(CANCEL_STATE_DONE, Ordering::SeqCst);
            return Ok(());
        }

        let handle_ptr = self.handle as usize; // Copy for Send
        let ffi = Arc::clone(&self.ffi);
        tracing::info!(stream_handle = ?self.handle, "Cancelling video stream (timeout)");
        match ffi_call_with_timeout("ak_venc_cancel_stream", timeout, move || {
            ffi.venc_cancel_stream(handle_ptr as *mut c_void)
        }) {
            Ok(ret) => {
                self.cancel_state.store(CANCEL_STATE_DONE, Ordering::SeqCst);
                check_result(ret, "ak_venc_cancel_stream")
            }
            Err(()) => {
                self.cancel_state
                    .store(CANCEL_STATE_UNKNOWN, Ordering::SeqCst);
                Err(PlatformError::HardwareFailure(
                    "ak_venc_cancel_stream timed out".to_string(),
                ))
            }
        }
    }

    /// Explicitly cancel stream in the current thread (no timeout wrapper).
    ///
    /// This path is used by platform shutdown to preserve strict ordering:
    /// join reader thread -> cancel stream -> close encoder.
    pub fn cancel_blocking(&self) -> PlatformResult<()> {
        let state = self.cancel_state.load(Ordering::SeqCst);
        if state == CANCEL_STATE_DONE {
            return Ok(());
        }
        if state == CANCEL_STATE_PENDING || state == CANCEL_STATE_UNKNOWN {
            return Err(PlatformError::HardwareFailure(
                "stream cancel state is indeterminate".to_string(),
            ));
        }
        if self
            .cancel_state
            .compare_exchange(
                CANCEL_STATE_ACTIVE,
                CANCEL_STATE_PENDING,
                Ordering::SeqCst,
                Ordering::SeqCst,
            )
            .is_err()
        {
            return Err(PlatformError::HardwareFailure(
                "failed to enter stream cancel state".to_string(),
            ));
        }

        if self.handle.is_null() {
            self.cancel_state.store(CANCEL_STATE_DONE, Ordering::SeqCst);
            return Ok(());
        }

        tracing::info!(stream_handle = ?self.handle, "Cancelling video stream (blocking)");
        let ret = self.ffi.venc_cancel_stream(self.handle);
        self.cancel_state.store(CANCEL_STATE_DONE, Ordering::SeqCst);
        check_result(ret, "ak_venc_cancel_stream")
    }

    /// Get the raw stream handle pointer for FFI calls.
    pub(crate) fn as_ptr(&self) -> *mut c_void {
        self.handle
    }
}

/// Internal helper that takes FFI trait for testability.
pub(crate) fn video_encoder_open_internal(
    param: &encode_param,
    ffi: &dyn VideoHalTrait,
) -> PlatformResult<VideoEncoderHandle> {
    let handle = ffi.venc_open(param);

    if handle.is_null() {
        Err(PlatformError::HardwareUnavailable(
            "Video encoder".to_string(),
        ))
    } else {
        Ok(VideoEncoderHandle {
            handle: Some(handle),
            closed: AtomicBool::new(false),
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
    video_encoder_open_internal(param, &DEFAULT_VIDEO_HAL)
}

/// Internal helper that takes FFI trait for testability.
pub(crate) fn video_encoder_set_rc_internal(
    handle: &VideoEncoderHandle,
    bps: i32,
    ffi: &dyn VideoHalTrait,
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
    video_encoder_set_rc_internal(handle, bps, &DEFAULT_VIDEO_HAL)
}

/// Internal helper that takes FFI trait for testability.
pub(crate) fn video_encoder_request_idr_internal(
    handle: &VideoEncoderHandle,
    ffi: &dyn VideoHalTrait,
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
    tracing::debug!(encoder_handle = ?handle.as_ptr(), "Forcing IDR frame");
    video_encoder_request_idr_internal(handle, &DEFAULT_VIDEO_HAL)
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
        let mut mock_ffi = MockVideoHalTrait::new();
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
        let mut mock_ffi = MockVideoHalTrait::new();

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
        let mock_ffi = MockVideoHalTrait::new();

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
        let mut mock_ffi = MockVideoHalTrait::new();
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
        let mut mock_ffi = MockVideoHalTrait::new();
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
        let mut mock_ffi = MockVideoHalTrait::new();
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
        let mut mock_ffi = MockVideoHalTrait::new();
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
        let mut mock_ffi = MockVideoHalTrait::new();
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
        let mut mock_ffi = MockVideoHalTrait::new();
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
        let mut mock_ffi = MockVideoHalTrait::new();
        let enc_handle = VideoEncoderHandle {
            handle: None, // Use None to prevent Drop from calling venc_close on dangling pointer
            closed: AtomicBool::new(false),
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
        let mut mock_ffi = MockVideoHalTrait::new();
        let enc_handle = VideoEncoderHandle {
            handle: None, // Use None to prevent Drop from calling venc_close on dangling pointer
            closed: AtomicBool::new(false),
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
        let mut mock_ffi = MockVideoHalTrait::new();
        let enc_handle = VideoEncoderHandle {
            handle: None, // Use None to prevent Drop from calling venc_close on dangling pointer
            closed: AtomicBool::new(false),
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
        let mut mock_ffi = MockVideoHalTrait::new();
        let enc_handle = VideoEncoderHandle {
            handle: None, // Use None to prevent Drop from calling venc_close on dangling pointer
            closed: AtomicBool::new(false),
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
        let mut mock_ffi = MockVideoHalTrait::new();

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
        let mut mock_ffi = MockVideoHalTrait::new();

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
        let mut mock_ffi = MockVideoHalTrait::new();

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
        let mut mock_ffi = MockVideoHalTrait::new();
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
        let mut mock_ffi = MockVideoHalTrait::new();
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
        let mut mock_ffi = MockVideoHalTrait::new();
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

    #[test]
    fn test_video_input_capture_off_internal_success() {
        let mut mock_ffi = MockVideoHalTrait::new();
        let vi_handle = VideoInputHandle::test_handle();

        mock_ffi
            .expect_vi_capture_off()
            .times(1)
            .returning(|_| AK_SUCCESS_I32);

        let result = video_input_capture_off_internal(&vi_handle, &mock_ffi);
        assert!(result.is_ok());
    }

    #[test]
    fn test_video_input_capture_off_internal_ffi_failure() {
        let mut mock_ffi = MockVideoHalTrait::new();
        let vi_handle = VideoInputHandle::test_handle();

        mock_ffi
            .expect_vi_capture_off()
            .times(1)
            .returning(|_| AK_FAILED_I32);

        let result = video_input_capture_off_internal(&vi_handle, &mock_ffi);
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareFailure(msg)) => {
                assert!(msg.contains("ak_vi_capture_off"));
            }
            _ => panic!("Expected HardwareFailure error"),
        }
    }

    // ---- Tests for vpss_init_internal and vpss_destroy_internal ----

    #[test]
    fn test_vpss_init_internal_calls_ffi() {
        let mut mock_ffi = MockVideoHalTrait::new();
        let vi_handle = VideoInputHandle::test_handle();

        mock_ffi
            .expect_vpss_init()
            .withf(|handle, _dev| handle.is_null())
            .times(1)
            .returning(|_, _| ());

        let result = vpss_init_internal(&vi_handle, VideoDevice::DEV0, &mock_ffi);
        assert!(result.is_ok());
    }

    #[test]
    fn test_vpss_init_internal_rejects_invalid_device() {
        let mock_ffi = MockVideoHalTrait::new();
        let vi_handle = VideoInputHandle::test_handle();

        // Try with invalid device ID (not DEV0)
        let invalid_device = VideoDevice(1);
        let result = vpss_init_internal(&vi_handle, invalid_device, &mock_ffi);
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
    fn test_vpss_destroy_internal_calls_ffi() {
        let mut mock_ffi = MockVideoHalTrait::new();

        mock_ffi
            .expect_vpss_destroy()
            .withf(|_dev| true)
            .times(1)
            .returning(|_| ());

        let result = vpss_destroy_internal(VideoDevice::DEV0, &mock_ffi);
        assert!(result.is_ok());
    }

    #[test]
    fn test_vpss_destroy_internal_rejects_invalid_device() {
        let mock_ffi = MockVideoHalTrait::new();

        // Try with invalid device ID (not DEV0)
        let invalid_device = VideoDevice(1);
        let result = vpss_destroy_internal(invalid_device, &mock_ffi);
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
    #[cfg(use_stubs)]
    fn test_vpss_init_success() {
        let handle = video_input_open(VideoDevice::DEV0).unwrap();
        let result = vpss_init(&handle, VideoDevice::DEV0);
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(use_stubs)]
    fn test_vpss_destroy_success() {
        let result = vpss_destroy(VideoDevice::DEV0);
        assert!(result.is_ok());
    }
}
