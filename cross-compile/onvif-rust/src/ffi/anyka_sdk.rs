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
    use super::*;
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
    pub fn ak_log(level: LogLevel, message: &str) -> AnykaResult<()> {
        let c_msg = std::ffi::CString::new(message)
            .map_err(|_| AnykaError::InvalidParameter("Invalid message string".to_string()))?;

        // SAFETY: We're passing a valid C string to ak_print
        unsafe {
            ak_print(level as c_int, c_msg.as_ptr());
        }
        Ok(())
    }

    /// Open video input device.
    pub fn video_input_open(device: VideoDevice) -> AnykaResult<*mut c_void> {
        // SAFETY: Calling FFI function with valid device ID
        let sdk_device: video_dev_type = unsafe { std::mem::transmute(device.0 as i32) };
        let handle = unsafe { ak_vi_open(sdk_device) };
        if handle.is_null() {
            Err(AnykaError::ResourceUnavailable("Video input".to_string()))
        } else {
            Ok(handle)
        }
    }

    /// Close video input device.
    /// Close video input device.
    ///
    /// # Safety
    /// Caller must ensure `handle` was obtained from `video_input_open` and is valid for the
    /// underlying SDK. Passing an arbitrary or freed pointer is undefined behavior.
    pub unsafe fn video_input_close(handle: *mut c_void) -> AnykaResult<()> {
        if handle.is_null() {
            return Err(AnykaError::InvalidParameter("Null handle".to_string()));
        }
        // SAFETY: Calling FFI function with valid handle
        let result = unsafe { ak_vi_close(handle) };
        if result < 0 {
            Err(AnykaError::SdkError(result))
        } else {
            Ok(())
        }
    }

    /// Open PTZ motor control.
    pub fn ptz_open() -> AnykaResult<()> {
        // SAFETY: Calling FFI function
        let result = unsafe { ak_drv_ptz_open() };
        if result < 0 {
            Err(AnykaError::SdkError(result))
        } else {
            Ok(())
        }
    }

    /// Close PTZ motor control.
    pub fn ptz_close() -> AnykaResult<()> {
        // SAFETY: Calling FFI function
        let result = unsafe { ak_drv_ptz_close() };
        if result < 0 {
            Err(AnykaError::SdkError(result))
        } else {
            Ok(())
        }
    }

    /// Turn PTZ motor in a direction.
    pub fn ptz_turn(direction: PtzDirection) -> AnykaResult<()> {
        // SAFETY: Calling FFI function with valid direction code
        let sdk_direction: ptz_turn_direction =
            unsafe { std::mem::transmute(direction.to_direction_code()) };
        let result = unsafe { ak_drv_ptz_turn(sdk_direction, 0) };
        if result < 0 {
            Err(AnykaError::SdkError(result))
        } else {
            Ok(())
        }
    }

    /// Stop PTZ motor movement.
    pub fn ptz_stop() -> AnykaResult<()> {
        // SAFETY: Calling FFI function
        let result = unsafe { ak_drv_ptz_stop() };
        if result < 0 {
            Err(AnykaError::SdkError(result))
        } else {
            Ok(())
        }
    }

    /// Get PTZ motor step position.
    pub fn ptz_get_position(motor: PtzMotor) -> AnykaResult<i32> {
        // SAFETY: Calling FFI function with valid motor ID
        let sdk_motor: ptz_device = unsafe { std::mem::transmute(motor.to_device_id()) };
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
    use super::*;

    /// Print a message (stub - does nothing).
    pub fn ak_log(_level: LogLevel, _message: &str) -> AnykaResult<()> {
        Ok(())
    }

    /// Open video input device (stub).
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
    pub unsafe fn video_input_close(_handle: *mut std::ffi::c_void) -> AnykaResult<()> {
        Ok(())
    }

    /// Open PTZ motor control (stub).
    pub fn ptz_open() -> AnykaResult<()> {
        Ok(())
    }

    /// Close PTZ motor control (stub).
    pub fn ptz_close() -> AnykaResult<()> {
        Ok(())
    }

    /// Turn PTZ motor in a direction (stub).
    pub fn ptz_turn(_direction: PtzDirection) -> AnykaResult<()> {
        Ok(())
    }

    /// Stop PTZ motor movement (stub).
    pub fn ptz_stop() -> AnykaResult<()> {
        Ok(())
    }

    /// Get PTZ motor step position (stub).
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
}
