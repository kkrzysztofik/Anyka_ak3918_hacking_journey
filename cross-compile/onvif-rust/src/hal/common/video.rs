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
//! use onvif_rust::hal::common::video::*;
//! use onvif_rust::hal::anyka::sdk::VideoDevice;
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

use super::{encode_param, video_channel_attr, video_dev_type, video_resolution, video_stream};

#[cfg(test)]
use super::AK_FAILED_I32;
use super::{AK_SUCCESS_I32, check_result};
use crate::hal::anyka::sdk::{Resolution, VideoDevice};

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
    fn vi_set_flip_mirror(&self, handle: *mut c_void, flip: bool, mirror: bool) -> i32;
    fn vpss_init(&self, vi_handle: *mut c_void, dev: i32);
    fn vpss_destroy(&self, dev: i32);
    fn venc_set_cfg_path(&self, path: *const c_char) -> i32;
    fn venc_open(&self, param: *const encode_param) -> *mut c_void;
    fn venc_close(&self, handle: *mut c_void) -> i32;
    fn venc_set_rc(&self, enc_handle: *mut c_void, bps: i32) -> i32;
    fn venc_set_iframe(&self, enc_handle: *mut c_void) -> i32;
    /// Request an IDR frame using a raw address.
    ///
    /// This method accepts a `usize` address and handles the pointer cast internally,
    /// keeping raw pointer operations in the HAL layer.
    fn venc_set_iframe_by_addr(&self, addr: usize) -> i32 {
        self.venc_set_iframe(addr as *mut c_void)
    }
    fn venc_request_stream(&self, vi_handle: *mut c_void, venc_handle: *mut c_void) -> *mut c_void;
    fn venc_get_stream(&self, stream_handle: *mut c_void, stream: *mut video_stream) -> i32;
    fn venc_release_stream(&self, stream_handle: *mut c_void, stream: *mut video_stream) -> i32;
    fn venc_cancel_stream(&self, stream_handle: *mut c_void) -> i32;
    /// Get the last SDK error number (thread-local).
    fn get_error_no(&self) -> i32;
    /// Get the last SDK error string (thread-local). Returns empty string on stub.
    fn get_error_str(&self) -> String;
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
            // NOSONAR rust:S9168 -- intentional leak: destructor would race/hang (stuck FFI join)
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
pub(crate) fn video_input_open(
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

/// Internal helper that takes FFI trait for testability.
pub(crate) fn video_input_get_sensor_resolution(
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

/// Internal helper that takes FFI trait for testability.
pub(crate) fn video_input_set_channel_attr(
    handle: &VideoInputHandle,
    attr: &video_channel_attr,
    ffi: &dyn VideoHalTrait,
) -> PlatformResult<()> {
    let ret = ffi.vi_set_channel_attr(handle.as_ptr(), attr);
    check_result(ret, "ak_vi_set_channel_attr")
}

/// Internal helper that takes FFI trait for testability.
///
/// Loads the ISP sensor configuration from the specified path. This must be
/// called before `ak_vi_open()` so the ISP subsystem has a valid config buffer.
pub(crate) fn video_input_match_sensor(
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

/// Internal helper that takes FFI trait for testability.
pub(crate) fn video_input_capture_on(
    handle: &VideoInputHandle,
    ffi: &dyn VideoHalTrait,
) -> PlatformResult<()> {
    let ret = ffi.vi_capture_on(handle.as_ptr());
    check_result(ret, "ak_vi_capture_on")
}

/// Internal helper that takes FFI trait for testability.
pub(crate) fn video_input_capture_off(
    handle: &VideoInputHandle,
    ffi: &dyn VideoHalTrait,
) -> PlatformResult<()> {
    let ret = ffi.vi_capture_off(handle.as_ptr());
    check_result(ret, "ak_vi_capture_off")
}

/// Internal helper that takes FFI trait for testability.
pub(crate) fn video_input_set_flip_mirror(
    handle: &VideoInputHandle,
    flip: bool,
    mirror: bool,
    ffi: &dyn VideoHalTrait,
) -> PlatformResult<()> {
    let ret = ffi.vi_set_flip_mirror(handle.as_ptr(), flip, mirror);
    check_result(ret, "ak_vi_set_flip_mirror")
}

/// Internal helper that takes FFI trait for testability.
///
/// Initializes the Video Post-Processing SubSystem (VPSS).
/// This MUST be called immediately after `video_input_open()` and
/// BEFORE any other video operations.
pub(crate) fn vpss_init(
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

/// Internal helper that takes FFI trait for testability.
///
/// Destroys the Video Post-Processing SubSystem (VPSS).
/// This MUST be called BEFORE `video_input_close()` during cleanup.
pub(crate) fn vpss_destroy(device: VideoDevice, ffi: &dyn VideoHalTrait) -> PlatformResult<()> {
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
pub(crate) fn video_encoder_open(
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

/// Internal helper that takes FFI trait for testability.
pub(crate) fn video_encoder_set_rc(
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

/// Internal helper that takes FFI trait for testability.
pub(crate) fn video_encoder_request_idr(
    handle: &VideoEncoderHandle,
    ffi: &dyn VideoHalTrait,
) -> PlatformResult<()> {
    let ret = ffi.venc_set_iframe(handle.as_ptr());
    check_result(ret, "ak_venc_set_iframe")
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;

    fn video_dev0() -> video_dev_type {
        video_dev_type::Dev0
    }

    // Mockall-based tests for wrapper functions
    #[test]
    fn test_video_input_open_calls_ffi_and_returns_handle() {
        let mut mock_ffi = MockVideoHalTrait::new();
        let test_handle = std::ptr::NonNull::<c_void>::dangling().as_ptr();

        let test_handle_usize = test_handle as usize;
        mock_ffi
            .expect_vi_open()
            .with(eq(video_dev0()))
            .times(1)
            .returning(move |_| test_handle_usize as *mut c_void);

        let result = video_input_open(VideoDevice::DEV0, &mock_ffi);
        assert!(result.is_ok());
        let handle = result.unwrap();
        assert_eq!(handle.as_ptr(), test_handle);
    }

    #[test]
    fn test_video_input_open_returns_error_on_null_handle() {
        let mut mock_ffi = MockVideoHalTrait::new();

        mock_ffi
            .expect_vi_open()
            .with(eq(video_dev0()))
            .times(1)
            .returning(|_| std::ptr::null_mut());

        let result = video_input_open(VideoDevice::DEV0, &mock_ffi);
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareUnavailable(_)) => {}
            _ => panic!("Expected HardwareUnavailable error"),
        }
    }

    #[test]
    fn test_video_input_open_rejects_invalid_device() {
        let mock_ffi = MockVideoHalTrait::new();

        // Try with invalid device ID (not DEV0)
        let invalid_device = VideoDevice(1);
        let result = video_input_open(invalid_device, &mock_ffi);
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
    fn test_video_input_get_sensor_resolution_calls_ffi_and_returns_resolution() {
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

        let result = video_input_get_sensor_resolution(&vi_handle, &mock_ffi);
        assert!(result.is_ok());
        let res = result.unwrap();
        assert_eq!(res.width, 1920);
        assert_eq!(res.height, 1080);
    }

    #[test]
    fn test_video_input_get_sensor_resolution_propagates_error() {
        let mut mock_ffi = MockVideoHalTrait::new();
        let vi_handle = VideoInputHandle::test_handle();

        mock_ffi
            .expect_vi_get_sensor_resolution()
            .withf(|handle, _| handle.is_null())
            .times(1)
            .returning(|_, _| AK_FAILED_I32);

        let result = video_input_get_sensor_resolution(&vi_handle, &mock_ffi);
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareFailure(msg)) => {
                assert!(msg.contains("ak_vi_get_sensor_resolution"));
            }
            _ => panic!("Expected HardwareFailure error"),
        }
    }

    #[test]
    fn test_video_input_set_channel_attr_calls_ffi() {
        let mut mock_ffi = MockVideoHalTrait::new();
        let vi_handle = VideoInputHandle::test_handle();
        let attr = video_channel_attr::default();

        mock_ffi
            .expect_vi_set_channel_attr()
            .withf(|handle, _| handle.is_null())
            .times(1)
            .returning(|_, _| AK_SUCCESS_I32);

        let result = video_input_set_channel_attr(&vi_handle, &attr, &mock_ffi);
        assert!(result.is_ok());
    }

    #[test]
    fn test_video_input_set_channel_attr_propagates_error() {
        let mut mock_ffi = MockVideoHalTrait::new();
        let vi_handle = VideoInputHandle::test_handle();
        let attr = video_channel_attr::default();

        mock_ffi
            .expect_vi_set_channel_attr()
            .withf(|handle, _| handle.is_null())
            .times(1)
            .returning(|_, _| AK_FAILED_I32);

        let result = video_input_set_channel_attr(&vi_handle, &attr, &mock_ffi);
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareFailure(msg)) => {
                assert!(msg.contains("ak_vi_set_channel_attr"));
            }
            _ => panic!("Expected HardwareFailure error"),
        }
    }

    #[test]
    fn test_video_encoder_open_calls_ffi_and_returns_handle() {
        let mut mock_ffi = MockVideoHalTrait::new();
        let test_handle = std::ptr::NonNull::<c_void>::dangling().as_ptr();
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

        let test_handle_usize = test_handle as usize;
        mock_ffi
            .expect_venc_open()
            .times(1)
            .returning(move |_| test_handle_usize as *mut c_void);

        let result = video_encoder_open(&param, &mock_ffi);
        assert!(result.is_ok());
        let handle = result.unwrap();
        assert_eq!(handle.as_ptr(), test_handle);
    }

    #[test]
    fn test_video_encoder_open_returns_error_on_null_handle() {
        let mut mock_ffi = MockVideoHalTrait::new();
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

        mock_ffi
            .expect_venc_open()
            .times(1)
            .returning(|_| std::ptr::null_mut());

        let result = video_encoder_open(&param, &mock_ffi);
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareUnavailable(_)) => {}
            _ => panic!("Expected HardwareUnavailable error"),
        }
    }

    #[test]
    fn test_video_encoder_set_rc_calls_ffi() {
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

        let result = video_encoder_set_rc(&enc_handle, 3000000, &mock_ffi);
        assert!(result.is_ok());
    }

    #[test]
    fn test_video_encoder_set_rc_propagates_error() {
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

        let result = video_encoder_set_rc(&enc_handle, 3000000, &mock_ffi);
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareFailure(msg)) => {
                assert!(msg.contains("ak_venc_set_rc"));
            }
            _ => panic!("Expected HardwareFailure error"),
        }
    }

    #[test]
    fn test_video_encoder_request_idr_calls_ffi() {
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

        let result = video_encoder_request_idr(&enc_handle, &mock_ffi);
        assert!(result.is_ok());
    }

    #[test]
    fn test_video_encoder_request_idr_propagates_error() {
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

        let result = video_encoder_request_idr(&enc_handle, &mock_ffi);
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareFailure(msg)) => {
                assert!(msg.contains("ak_venc_set_iframe"));
            }
            _ => panic!("Expected HardwareFailure error"),
        }
    }

    // ---- Tests for video_input_match_sensor ----

    #[test]
    fn test_video_input_match_sensor_success() {
        let mut mock_ffi = MockVideoHalTrait::new();

        mock_ffi
            .expect_vi_match_sensor()
            .times(1)
            .returning(|_| AK_SUCCESS_I32);

        let path = Path::new("/tmp/sensor/isp_gc1084.conf");
        let result = video_input_match_sensor(path, &mock_ffi);
        assert!(result.is_ok());
    }

    #[test]
    fn test_video_input_match_sensor_ffi_failure() {
        let mut mock_ffi = MockVideoHalTrait::new();

        mock_ffi
            .expect_vi_match_sensor()
            .times(1)
            .returning(|_| AK_FAILED_I32);

        let path = Path::new("/tmp/sensor/isp_gc1084.conf");
        let result = video_input_match_sensor(path, &mock_ffi);
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareFailure(msg)) => {
                assert!(msg.contains("ak_vi_match_sensor"));
            }
            _ => panic!("Expected HardwareFailure error"),
        }
    }

    #[test]
    fn test_video_input_match_sensor_passes_correct_path() {
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
        let result = video_input_match_sensor(path, &mock_ffi);
        assert!(result.is_ok());
    }

    // ---- Tests for video_input_capture_on ----

    #[test]
    fn test_video_input_capture_on_success() {
        let mut mock_ffi = MockVideoHalTrait::new();
        let vi_handle = VideoInputHandle::test_handle();

        mock_ffi
            .expect_vi_capture_on()
            .times(1)
            .returning(|_| AK_SUCCESS_I32);

        let result = video_input_capture_on(&vi_handle, &mock_ffi);
        assert!(result.is_ok());
    }

    #[test]
    fn test_video_input_capture_on_ffi_failure() {
        let mut mock_ffi = MockVideoHalTrait::new();
        let vi_handle = VideoInputHandle::test_handle();

        mock_ffi
            .expect_vi_capture_on()
            .times(1)
            .returning(|_| AK_FAILED_I32);

        let result = video_input_capture_on(&vi_handle, &mock_ffi);
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareFailure(msg)) => {
                assert!(msg.contains("ak_vi_capture_on"));
            }
            _ => panic!("Expected HardwareFailure error"),
        }
    }

    #[test]
    fn test_video_input_capture_on_unknown_error_code() {
        let mut mock_ffi = MockVideoHalTrait::new();
        let vi_handle = VideoInputHandle::test_handle();

        mock_ffi.expect_vi_capture_on().times(1).returning(|_| -99);

        let result = video_input_capture_on(&vi_handle, &mock_ffi);
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareFailure(msg)) => {
                assert!(msg.contains("error code -99"));
            }
            _ => panic!("Expected HardwareFailure error"),
        }
    }

    #[test]
    fn test_video_input_capture_off_success() {
        let mut mock_ffi = MockVideoHalTrait::new();
        let vi_handle = VideoInputHandle::test_handle();

        mock_ffi
            .expect_vi_capture_off()
            .times(1)
            .returning(|_| AK_SUCCESS_I32);

        let result = video_input_capture_off(&vi_handle, &mock_ffi);
        assert!(result.is_ok());
    }

    #[test]
    fn test_video_input_capture_off_ffi_failure() {
        let mut mock_ffi = MockVideoHalTrait::new();
        let vi_handle = VideoInputHandle::test_handle();

        mock_ffi
            .expect_vi_capture_off()
            .times(1)
            .returning(|_| AK_FAILED_I32);

        let result = video_input_capture_off(&vi_handle, &mock_ffi);
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareFailure(msg)) => {
                assert!(msg.contains("ak_vi_capture_off"));
            }
            _ => panic!("Expected HardwareFailure error"),
        }
    }

    #[test]
    fn test_video_input_set_flip_mirror_calls_ffi_with_correct_flags() {
        let mut mock_ffi = MockVideoHalTrait::new();
        let vi_handle = VideoInputHandle::test_handle();

        mock_ffi
            .expect_vi_set_flip_mirror()
            .withf(|handle, flip, mirror| handle.is_null() && *flip && *mirror)
            .times(1)
            .returning(|_, _, _| AK_SUCCESS_I32);

        let result = video_input_set_flip_mirror(&vi_handle, true, true, &mock_ffi);
        assert!(result.is_ok());
    }

    #[test]
    fn test_video_input_set_flip_mirror_does_not_swap_flip_and_mirror() {
        let mut mock_ffi = MockVideoHalTrait::new();
        let vi_handle = VideoInputHandle::test_handle();

        mock_ffi
            .expect_vi_set_flip_mirror()
            .withf(|_, flip, mirror| *flip && !*mirror)
            .times(1)
            .returning(|_, _, _| AK_SUCCESS_I32);

        let result = video_input_set_flip_mirror(&vi_handle, true, false, &mock_ffi);
        assert!(result.is_ok());
    }

    #[test]
    fn test_video_input_set_flip_mirror_propagates_error() {
        let mut mock_ffi = MockVideoHalTrait::new();
        let vi_handle = VideoInputHandle::test_handle();

        mock_ffi
            .expect_vi_set_flip_mirror()
            .times(1)
            .returning(|_, _, _| AK_FAILED_I32);

        let result = video_input_set_flip_mirror(&vi_handle, true, true, &mock_ffi);
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareFailure(msg)) => {
                assert!(msg.contains("ak_vi_set_flip_mirror"));
            }
            _ => panic!("Expected HardwareFailure error"),
        }
    }

    // ---- Tests for vpss_init and vpss_destroy ----

    #[test]
    fn test_vpss_init_calls_ffi() {
        let mut mock_ffi = MockVideoHalTrait::new();
        let vi_handle = VideoInputHandle::test_handle();

        mock_ffi
            .expect_vpss_init()
            .withf(|handle, _dev| handle.is_null())
            .times(1)
            .returning(|_, _| ());

        let result = vpss_init(&vi_handle, VideoDevice::DEV0, &mock_ffi);
        assert!(result.is_ok());
    }

    #[test]
    fn test_vpss_init_rejects_invalid_device() {
        let mock_ffi = MockVideoHalTrait::new();
        let vi_handle = VideoInputHandle::test_handle();

        // Try with invalid device ID (not DEV0)
        let invalid_device = VideoDevice(1);
        let result = vpss_init(&vi_handle, invalid_device, &mock_ffi);
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
    fn test_vpss_destroy_calls_ffi() {
        let mut mock_ffi = MockVideoHalTrait::new();

        mock_ffi
            .expect_vpss_destroy()
            .withf(|_dev| true)
            .times(1)
            .returning(|_| ());

        let result = vpss_destroy(VideoDevice::DEV0, &mock_ffi);
        assert!(result.is_ok());
    }

    #[test]
    fn test_vpss_destroy_rejects_invalid_device() {
        let mock_ffi = MockVideoHalTrait::new();

        // Try with invalid device ID (not DEV0)
        let invalid_device = VideoDevice(1);
        let result = vpss_destroy(invalid_device, &mock_ffi);
        assert!(result.is_err());
        match result {
            Err(PlatformError::InvalidParameter(msg)) => {
                assert!(msg.contains("Invalid video device ID"));
                assert!(msg.contains("1"));
            }
            _ => panic!("Expected InvalidParameter error"),
        }
    }
}
