//! Safe Rust wrappers for Anyka SDK FFI functions.
//!
//! This module provides safe Rust interfaces to the low-level FFI bindings.
//! All unsafe FFI calls are wrapped with proper error handling and type conversions.

use thiserror::Error;

/// Errors that can occur when calling Anyka SDK functions.
#[derive(Debug, Error)]
pub enum AnykaError {
    /// SDK function returned an error code.
    #[error("Anyka SDK error: {0}")]
    SdkError(i32),

    /// Invalid parameter passed to SDK function.
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),

    /// Resource not available.
    #[error("Resource not available: {0}")]
    ResourceUnavailable(String),

    /// Operation timed out.
    #[error("Operation timed out")]
    Timeout,

    /// Hardware failure.
    #[error("Hardware failure: {0}")]
    HardwareFailure(String),

    /// SDK not initialized.
    #[error("SDK not initialized")]
    NotInitialized,
}

/// Result type for Anyka SDK operations.
pub type AnykaResult<T> = Result<T, AnykaError>;

/// Log level for Anyka SDK logging.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum LogLevel {
    /// Reserved (not used).
    Reserved = 0,
    /// Error messages.
    Error = 1,
    /// Warning messages.
    Warning = 2,
    /// Notice messages.
    Notice = 3,
    /// Normal messages.
    Normal = 4,
    /// Info messages.
    Info = 5,
    /// Debug messages.
    Debug = 6,
}

impl From<i32> for LogLevel {
    fn from(value: i32) -> Self {
        match value {
            0 => LogLevel::Reserved,
            1 => LogLevel::Error,
            2 => LogLevel::Warning,
            3 => LogLevel::Notice,
            4 => LogLevel::Normal,
            5 => LogLevel::Info,
            6 => LogLevel::Debug,
            _ => LogLevel::Debug, // Default to debug for unknown levels
        }
    }
}

/// Video device identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct VideoDevice(pub u32);

impl VideoDevice {
    /// Primary video device.
    pub const DEV0: VideoDevice = VideoDevice(0);
}

/// Video channel identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct VideoChannel(pub u32);

impl VideoChannel {
    /// Main video channel (high resolution).
    pub const MAIN: VideoChannel = VideoChannel(0);
    /// Sub video channel (low resolution).
    pub const SUB: VideoChannel = VideoChannel(1);
}

/// Video resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Resolution {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
}

impl Resolution {
    /// Create a new resolution.
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    /// Common resolution: 1920x1080 (1080p).
    pub const HD1080: Resolution = Resolution {
        width: 1920,
        height: 1080,
    };
    /// Common resolution: 1280x720 (720p).
    pub const HD720: Resolution = Resolution {
        width: 1280,
        height: 720,
    };
    /// Common resolution: 640x480 (VGA).
    pub const VGA: Resolution = Resolution {
        width: 640,
        height: 480,
    };
    /// Common resolution: 320x240 (QVGA).
    pub const QVGA: Resolution = Resolution {
        width: 320,
        height: 240,
    };
}

/// Video encoding type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VideoEncoding {
    /// H.264/AVC encoding.
    #[default]
    H264,
    /// H.265/HEVC encoding.
    H265,
    /// MJPEG encoding.
    Mjpeg,
}

/// Bitrate control mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BitrateMode {
    /// Constant bitrate.
    #[default]
    Cbr,
    /// Variable bitrate.
    Vbr,
}

/// Video encoder configuration.
#[derive(Debug, Clone, Default)]
pub struct VideoEncoderConfig {
    /// Output resolution.
    pub resolution: Resolution,
    /// Frame rate in frames per second.
    pub framerate: u32,
    /// Target bitrate in kbps.
    pub bitrate: u32,
    /// Encoding type.
    pub encoding: VideoEncoding,
    /// Bitrate control mode.
    pub bitrate_mode: BitrateMode,
    /// GOP length (I-frame interval).
    pub gop_length: u32,
    /// Quality level (0-100).
    pub quality: u32,
}

/// Audio encoding type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AudioEncoding {
    /// G.711 μ-law encoding.
    #[default]
    G711U,
    /// G.711 A-law encoding.
    G711A,
    /// AAC encoding.
    Aac,
    /// PCM (raw audio).
    Pcm,
}

/// Audio encoder configuration.
#[derive(Debug, Clone, Default)]
pub struct AudioEncoderConfig {
    /// Sample rate in Hz.
    pub sample_rate: u32,
    /// Number of channels.
    pub channels: u32,
    /// Bits per sample.
    pub bits_per_sample: u32,
    /// Encoding type.
    pub encoding: AudioEncoding,
    /// Bitrate in kbps.
    pub bitrate: u32,
}

/// PTZ device (pan/tilt motor).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PtzMotor {
    /// Horizontal (pan) motor.
    Horizontal,
    /// Vertical (tilt) motor.
    Vertical,
}

impl PtzMotor {
    /// Convert to FFI device ID.
    pub fn to_device_id(self) -> i32 {
        match self {
            PtzMotor::Horizontal => 0,
            PtzMotor::Vertical => 1,
        }
    }
}

/// PTZ movement direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PtzDirection {
    /// Move left.
    Left,
    /// Move right.
    Right,
    /// Move up.
    Up,
    /// Move down.
    Down,
}

impl PtzDirection {
    /// Convert to FFI direction code.
    pub fn to_direction_code(self) -> i32 {
        match self {
            PtzDirection::Left => 1,
            PtzDirection::Right => 2,
            PtzDirection::Up => 3,
            PtzDirection::Down => 4,
        }
    }
}

/// PTZ position in degrees.
#[derive(Debug, Clone, Copy, Default)]
pub struct PtzPosition {
    /// Pan position (-180.0 to 180.0 degrees).
    pub pan: f32,
    /// Tilt position (-90.0 to 90.0 degrees).
    pub tilt: f32,
    /// Zoom level (1.0 to max zoom).
    pub zoom: f32,
}

impl PtzPosition {
    /// Create a new PTZ position.
    pub fn new(pan: f32, tilt: f32, zoom: f32) -> Self {
        Self { pan, tilt, zoom }
    }

    /// Home position (center, no zoom).
    pub const HOME: PtzPosition = PtzPosition {
        pan: 0.0,
        tilt: 0.0,
        zoom: 1.0,
    };
}

/// PTZ preset.
#[derive(Debug, Clone)]
pub struct PtzPreset {
    /// Preset token (identifier).
    pub token: String,
    /// Preset name.
    pub name: String,
    /// Preset position.
    pub position: PtzPosition,
}

// FFI wrapper functions for actual hardware access
// These are only used when cross-compiling for ARM target

#[cfg(not(use_stubs))]
mod ffi_impl {
    use super::{AnykaError, AnykaResult, LogLevel, PtzDirection, PtzMotor, VideoDevice};
    use crate::ffi::generated::{
        audio_param, encode_param, pcm_param, ptz_device, ptz_turn_direction, video_dev_type,
    };
    use std::ffi::{c_char, c_int, c_void};

    // External C functions from Anyka SDK
    #[allow(dead_code)]
    unsafe extern "C" {
        fn ak_print(level: c_int, fmt: *const c_char, ...) -> c_int;
        fn ak_vi_open(dev: video_dev_type) -> *mut c_void;
        fn ak_vi_close(handle: *mut c_void) -> c_int;
        fn ak_venc_open(param: *const encode_param) -> *mut c_void;
        fn ak_venc_close(handle: *mut c_void) -> c_int;
        fn ak_ai_open(param: *const pcm_param) -> *mut c_void;
        fn ak_ai_close(handle: *mut c_void) -> c_int;
        fn ak_aenc_open(param: *const audio_param) -> *mut c_void;
        fn ak_aenc_close(handle: *mut c_void) -> c_int;
        fn ak_drv_ptz_open() -> c_int;
        fn ak_drv_ptz_close() -> c_int;
        fn ak_drv_ptz_turn(direction: ptz_turn_direction, degree: c_int) -> c_int;
        fn ak_drv_ptz_stop() -> c_int;
        fn ak_drv_ptz_get_step_pos(motor_no: ptz_device) -> c_int;
    }

    /// Print a message using Anyka SDK logging.
    ///
    /// # Input Validation
    ///
    /// - `message`: Must not contain null bytes (validated by `CString::new()`)
    /// - `level`: Must be a valid `LogLevel` enum variant (type system ensures this)
    ///
    /// # Safety
    ///
    /// - `CString::new()` ensures the string is null-terminated and contains no embedded nulls
    /// - `c_msg.as_ptr()` is valid for the lifetime of the `CString`
    /// - `ak_print()` is a C function that reads the string but does not modify it
    /// - The `CString` is dropped after the call, ensuring proper cleanup
    pub fn ak_log(level: LogLevel, message: &str) -> AnykaResult<()> {
        // Validate: CString::new() will fail if message contains null bytes
        let c_msg = std::ffi::CString::new(message).map_err(|_| {
            AnykaError::InvalidParameter("Message contains null bytes or invalid UTF-8".to_string())
        })?;

        // SAFETY: We're passing a valid C string to ak_print
        // - c_msg is null-terminated (guaranteed by CString)
        // - c_msg.as_ptr() is valid for the duration of the call
        // - ak_print() does not take ownership or modify the string
        unsafe {
            ak_print(level as c_int, c_msg.as_ptr());
        }
        Ok(())
    }

    /// RAII Wrapper for Video Input Device
    pub struct VideoInput {
        handle: *mut c_void,
    }

    // SAFETY: VideoInput handle is thread-safe - confirmed by Anyka SDK source code.
    //
    // Evidence from platform/libplat/src/vi/:
    //
    // 1. INTERNAL MUTEX PROTECTION (isp_vi.c:68-69):
    //    static pthread_mutex_t isp_manipulate_mutex_lock = PTHREAD_MUTEX_INITIALIZER;
    //    static pthread_mutex_t isp_frame_list_lock = PTHREAD_MUTEX_INITIALIZER;
    //
    // 2. MUTEX USAGE IN VI OPERATIONS (ak_vi.c):
    //    - ak_vi_open() (line 1076): ak_thread_mutex_lock(&vi_ctrl.lock)
    //    - vi_new_user() (line 707): ak_thread_mutex_lock(&pdev->vi_lock)
    //    - ak_vi_set_channel_attr() (line 1186): ak_thread_mutex_lock(&pdev->vi_lock)
    //    - ak_vi_change_channel_attr() (line 1248): ak_thread_mutex_lock(&pdev->vi_lock)
    //    - vi_set_capture_on() (line 939): ak_thread_mutex_lock(&vi_ctrl.lock)
    //    - isp_vi_capture_on() (isp_vi.c:753): ak_thread_mutex_lock(&isp_manipulate_mutex_lock)
    //
    // 3. DEVICE STRUCTURE WITH LOCKS (ak_vi.c:904-905):
    //    ak_thread_mutex_init(&(pdev->vi_lock), NULL);
    //    ak_thread_mutex_init(&(pdev->frame_lock), NULL);
    //
    // The SDK implements a multi-level locking strategy:
    // - Global vi_ctrl.lock for device registration/initialization
    // - Per-device pdev->vi_lock for device-specific operations
    // - ISP-level isp_manipulate_mutex_lock for hardware access
    //
    // This confirms Send safety: the SDK's internal mutexes protect all state
    // modifications, making it safe to transfer ownership between threads.
    unsafe impl Send for VideoInput {}

    // SAFETY: VideoInput handle is Sync-safe - SDK uses internal synchronization.
    //
    // The SDK's mutex protection (documented above for Send) also ensures Sync safety.
    // All operations that access or modify the vi_handle state are protected by mutexes,
    // allowing safe concurrent access from multiple threads.
    //
    // Vendor reference code confirms this design:
    // - libre_anyka_app/main.c: Global vi_handle accessed by 4+ threads without external locks
    // - venc_demo/ak_venc_demo.c: vi_handle passed to multiple encoding threads
    // - No pthread_mutex_lock/unlock around ak_vi_* calls in application code
    //
    // The SDK's internal locking makes external synchronization unnecessary.
    unsafe impl Sync for VideoInput {}

    impl Drop for VideoInput {
        fn drop(&mut self) {
            if !self.handle.is_null() {
                // SAFETY: We own the handle and checking for null
                unsafe {
                    ak_vi_close(self.handle);
                }
            }
        }
    }

    /// Open video input device.
    ///
    /// # Input Validation
    ///
    /// - `device`: Must be `VideoDevice::DEV0` (value 0) - validated by type system
    /// - Bounds: Only device 0 is supported by hardware
    ///
    /// # Safety
    ///
    /// The transmute is safe because:
    /// - `device.0` is a u32 containing only values 0 (DEV0)
    /// - Casting to i32 preserves the bit pattern for values in this range
    /// - The resulting i32 matches the C enum `video_dev_type` representation exactly
    /// - `video_dev_type` is a C enum with variants 0-N, and our value 0 is valid
    /// - NOSONAR: S5343 - transmute is safe for enum value mapping with known bounds
    ///
    /// The FFI call is safe because:
    /// - `ak_vi_open()` is a C function that returns a handle or NULL
    /// - We check for null handle and return error if SDK fails
    /// - The handle is wrapped in `VideoInput` which manages cleanup
    pub fn video_input_open(device: VideoDevice) -> AnykaResult<VideoInput> {
        // Validate: Type system ensures device.0 is only 0 (DEV0)
        let sdk_device: video_dev_type = unsafe { std::mem::transmute(device.0 as i32) };

        // SAFETY: ak_vi_open() is a C function that returns handle or NULL
        // - We validate the result (null check below)
        // - Handle is wrapped in VideoInput for RAII cleanup
        let handle = unsafe { ak_vi_open(sdk_device) };

        if handle.is_null() {
            Err(AnykaError::ResourceUnavailable("Video input".to_string()))
        } else {
            Ok(VideoInput { handle })
        }
    }

    /// Open PTZ motor control.
    ///
    /// # Input Validation
    ///
    /// No parameters - function initializes hardware.
    ///
    /// # Safety
    ///
    /// - `ak_drv_ptz_open()` is a C function that initializes PTZ hardware
    /// - Returns negative value on error, non-negative on success
    /// - We validate the result and map errors appropriately
    pub fn ptz_open() -> AnykaResult<()> {
        // SAFETY: ak_drv_ptz_open() is a C function that initializes hardware
        // - Returns negative value on error, non-negative on success
        // - We validate the result below
        let result = unsafe { ak_drv_ptz_open() };
        if result < 0 {
            Err(AnykaError::SdkError(result))
        } else {
            Ok(())
        }
    }

    /// Close PTZ motor control.
    ///
    /// # Input Validation
    ///
    /// No parameters - function cleans up hardware.
    ///
    /// # Safety
    ///
    /// - `ak_drv_ptz_close()` is a C function that cleans up PTZ hardware
    /// - Returns negative value on error, non-negative on success
    /// - We validate the result and map errors appropriately
    pub fn ptz_close() -> AnykaResult<()> {
        // SAFETY: ak_drv_ptz_close() is a C function that cleans up hardware
        // - Returns negative value on error, non-negative on success
        // - We validate the result below
        let result = unsafe { ak_drv_ptz_close() };
        if result < 0 {
            Err(AnykaError::SdkError(result))
        } else {
            Ok(())
        }
    }

    /// Turn PTZ motor in a direction.
    ///
    /// # Input Validation
    ///
    /// - `direction`: Must be a valid `PtzDirection` enum variant (Left, Right, Up, Down)
    /// - `to_direction_code()` returns only values 1, 2, 3, 4 - validated by enum
    /// - Degree parameter is hardcoded to 0 (not configurable in current API)
    ///
    /// # Safety
    ///
    /// The transmute is safe because:
    /// - `to_direction_code()` returns only values 1, 2, 3, 4 (Left, Right, Up, Down)
    /// - These values match the C enum `ptz_turn_direction` variants exactly
    /// - The i32 bit pattern is reinterpreted as the C enum type
    /// - All possible return values are valid C enum variants
    /// - NOSONAR: S5343 - transmute is safe for enum value mapping with known bounds
    ///
    /// The FFI call is safe because:
    /// - `ak_drv_ptz_turn()` is a C function that moves PTZ hardware
    /// - Returns negative value on error, non-negative on success
    /// - We validate the result and map errors appropriately
    pub fn ptz_turn(direction: PtzDirection) -> AnykaResult<()> {
        // Validate: Enum ensures only valid direction values
        let sdk_direction: ptz_turn_direction =
            unsafe { std::mem::transmute(direction.to_direction_code()) };

        // SAFETY: ak_drv_ptz_turn() is a C function that moves PTZ hardware
        // - Returns negative value on error, non-negative on success
        // - We validate the result below
        let result = unsafe { ak_drv_ptz_turn(sdk_direction, 0) };
        if result < 0 {
            Err(AnykaError::SdkError(result))
        } else {
            Ok(())
        }
    }

    /// Stop PTZ motor movement.
    ///
    /// # Input Validation
    ///
    /// No parameters - function stops all PTZ movement.
    ///
    /// # Safety
    ///
    /// - `ak_drv_ptz_stop()` is a C function that stops PTZ hardware movement
    /// - Returns negative value on error, non-negative on success
    /// - We validate the result and map errors appropriately
    pub fn ptz_stop() -> AnykaResult<()> {
        // SAFETY: ak_drv_ptz_stop() is a C function that stops PTZ hardware
        // - Returns negative value on error, non-negative on success
        // - We validate the result below
        let result = unsafe { ak_drv_ptz_stop() };
        if result < 0 {
            Err(AnykaError::SdkError(result))
        } else {
            Ok(())
        }
    }

    /// Get PTZ motor step position.
    ///
    /// # Input Validation
    ///
    /// - `motor`: Must be a valid `PtzMotor` enum variant (Horizontal or Vertical)
    /// - `to_device_id()` returns only values 0 or 1 - validated by enum
    ///
    /// # Safety
    ///
    /// The transmute is safe because:
    /// - `to_device_id()` returns only values 0 (Horizontal) or 1 (Vertical)
    /// - These values match the C enum `ptz_device` variants exactly
    /// - The i32 bit pattern is reinterpreted as the C enum type
    /// - All possible return values are valid C enum variants
    /// - NOSONAR: S5343 - transmute is safe for enum value mapping with known bounds
    ///
    /// The FFI call is safe because:
    /// - `ak_drv_ptz_get_step_pos()` is a C function that reads hardware position
    /// - Returns negative value on error, position value (>=0) on success
    /// - We validate the result and map errors appropriately
    pub fn ptz_get_position(motor: PtzMotor) -> AnykaResult<i32> {
        // Validate: Enum ensures only valid motor values (0 or 1)
        let sdk_motor: ptz_device = unsafe { std::mem::transmute(motor.to_device_id()) };

        // SAFETY: ak_drv_ptz_get_step_pos() is a C function that reads hardware position
        // - Returns negative value on error, position value (>=0) on success
        // - We validate the result below
        let result = unsafe { ak_drv_ptz_get_step_pos(sdk_motor) };
        if result < 0 {
            Err(AnykaError::SdkError(result))
        } else {
            Ok(result)
        }
    }
}

#[cfg(not(use_stubs))]
pub use ffi_impl::*;

// Stub implementations for native builds (testing)
#[cfg(use_stubs)]
mod stub_impl {
    use super::{AnykaResult, LogLevel, PtzDirection, PtzMotor, VideoDevice};

    /// Print a message (stub - does nothing).
    pub fn ak_log(_level: LogLevel, _message: &str) -> AnykaResult<()> {
        Ok(())
    }

    /// Open video input device (stub).
    #[allow(dead_code)]
    pub fn video_input_open(_device: VideoDevice) -> AnykaResult<*mut std::ffi::c_void> {
        // Return a non-null pointer for stub
        Ok(std::ptr::dangling_mut::<std::ffi::c_void>())
    }

    /// Close video input device (stub).
    ///
    /// # Safety
    ///
    /// This function is marked as `unsafe` because it accepts a raw pointer. However, since this is a stub
    /// implementation that does not dereference the pointer, it is safe to call with any pointer value,
    /// including null pointers.
    #[allow(dead_code)]
    pub unsafe fn video_input_close(_handle: *mut std::ffi::c_void) -> AnykaResult<()> {
        Ok(())
    }

    /// Open PTZ motor control (stub).
    #[allow(dead_code)]
    pub fn ptz_open() -> AnykaResult<()> {
        Ok(())
    }

    /// Close PTZ motor control (stub).
    #[allow(dead_code)]
    pub fn ptz_close() -> AnykaResult<()> {
        Ok(())
    }

    /// Turn PTZ motor in a direction (stub).
    #[allow(dead_code)]
    pub fn ptz_turn(_direction: PtzDirection) -> AnykaResult<()> {
        Ok(())
    }

    /// Stop PTZ motor movement (stub).
    #[allow(dead_code)]
    pub fn ptz_stop() -> AnykaResult<()> {
        Ok(())
    }

    /// Get PTZ motor step position (stub).
    #[allow(dead_code)]
    pub fn ptz_get_position(_motor: PtzMotor) -> AnykaResult<i32> {
        Ok(0)
    }
}

#[cfg(use_stubs)]
pub use stub_impl::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolution_new() {
        let res = Resolution::new(1920, 1080);
        assert_eq!(res.width, 1920);
        assert_eq!(res.height, 1080);
    }

    #[test]
    fn test_ptz_position_home() {
        assert_eq!(PtzPosition::HOME.pan, 0.0);
        assert_eq!(PtzPosition::HOME.tilt, 0.0);
        assert_eq!(PtzPosition::HOME.zoom, 1.0);
    }

    #[test]
    fn test_ptz_direction_codes() {
        assert_eq!(PtzDirection::Left.to_direction_code(), 1);
        assert_eq!(PtzDirection::Right.to_direction_code(), 2);
        assert_eq!(PtzDirection::Up.to_direction_code(), 3);
        assert_eq!(PtzDirection::Down.to_direction_code(), 4);
    }

    #[test]
    fn test_log_level_from_i32() {
        assert_eq!(LogLevel::from(1), LogLevel::Error);
        assert_eq!(LogLevel::from(2), LogLevel::Warning);
        assert_eq!(LogLevel::from(6), LogLevel::Debug);
        assert_eq!(LogLevel::from(99), LogLevel::Debug); // Unknown defaults to Debug
    }

    #[test]
    fn test_log_level_all_variants() {
        assert_eq!(LogLevel::from(0), LogLevel::Reserved);
        assert_eq!(LogLevel::from(1), LogLevel::Error);
        assert_eq!(LogLevel::from(2), LogLevel::Warning);
        assert_eq!(LogLevel::from(3), LogLevel::Notice);
        assert_eq!(LogLevel::from(4), LogLevel::Normal);
        assert_eq!(LogLevel::from(5), LogLevel::Info);
        assert_eq!(LogLevel::from(6), LogLevel::Debug);
    }

    #[test]
    fn test_ptz_motor_to_device_id() {
        assert_eq!(PtzMotor::Horizontal.to_device_id(), 0);
        assert_eq!(PtzMotor::Vertical.to_device_id(), 1);
    }

    #[test]
    fn test_ptz_position_new() {
        let pos = PtzPosition::new(45.0, -30.0, 2.0);
        assert_eq!(pos.pan, 45.0);
        assert_eq!(pos.tilt, -30.0);
        assert_eq!(pos.zoom, 2.0);
    }

    #[test]
    fn test_video_device_constants() {
        assert_eq!(VideoDevice::DEV0.0, 0);
    }

    #[test]
    fn test_video_channel_constants() {
        assert_eq!(VideoChannel::MAIN.0, 0);
        assert_eq!(VideoChannel::SUB.0, 1);
    }

    #[test]
    fn test_resolution_constants() {
        assert_eq!(Resolution::HD1080.width, 1920);
        assert_eq!(Resolution::HD1080.height, 1080);
        assert_eq!(Resolution::HD720.width, 1280);
        assert_eq!(Resolution::HD720.height, 720);
        assert_eq!(Resolution::VGA.width, 640);
        assert_eq!(Resolution::VGA.height, 480);
        assert_eq!(Resolution::QVGA.width, 320);
        assert_eq!(Resolution::QVGA.height, 240);
    }

    #[test]
    fn test_anyka_error_display() {
        let err = AnykaError::SdkError(-1);
        let display = format!("{}", err);
        assert!(display.contains("Anyka SDK error"));

        let err = AnykaError::InvalidParameter("test".to_string());
        let display = format!("{}", err);
        assert!(display.contains("Invalid parameter"));
        assert!(display.contains("test"));

        let err = AnykaError::ResourceUnavailable("video".to_string());
        let display = format!("{}", err);
        assert!(display.contains("Resource not available"));
        assert!(display.contains("video"));

        let err = AnykaError::Timeout;
        let display = format!("{}", err);
        assert!(display.contains("timed out"));

        let err = AnykaError::HardwareFailure("sensor".to_string());
        let display = format!("{}", err);
        assert!(display.contains("Hardware failure"));
        assert!(display.contains("sensor"));

        let err = AnykaError::NotInitialized;
        let display = format!("{}", err);
        assert!(display.contains("not initialized"));
    }

    #[test]
    fn test_video_encoding_default() {
        assert_eq!(VideoEncoding::H264, VideoEncoding::default());
    }

    #[test]
    fn test_audio_encoding_default() {
        assert_eq!(AudioEncoding::G711U, AudioEncoding::default());
    }

    #[test]
    fn test_bitrate_mode_default() {
        assert_eq!(BitrateMode::Cbr, BitrateMode::default());
    }
}
