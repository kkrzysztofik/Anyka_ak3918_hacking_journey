//! Safe Rust wrappers for Anyka SDK PTZ functions.
//!
//! This module provides RAII-based wrappers around the Anyka SDK PTZ (Pan-Tilt-Zoom)
//! functions. All handles are automatically cleaned up when dropped, ensuring proper
//! resource management.
//!
//! # RAII Pattern
//!
//! All handles implement the Drop trait to automatically clean up resources:
//!
//! ```rust,no_run
//! use onvif_rust::hal::common::ptz::*;
//!
//! // Handle is automatically closed when it goes out of scope
//! {
//!     let ptz_handle = ptz_open()?;
//!     // Use handle...
//! } // ak_drv_ptz_close() is called here automatically
//! ```
//!
//! # Error Handling
//!
//! All functions return `Result<T, PlatformError>`, converting SDK error codes
//! (`AK_SUCCESS`/`AK_FAILED`) into appropriate `PlatformError` variants.

use crate::platform::PlatformError;
use crate::platform::PlatformResult;

// On ARM we use the Rust native driver; types from ptz_driver (ak_drv_ptz.h omitted from bindgen).
#[cfg(not(use_stubs))]
use crate::hal::anyka::ptz::{ptz_device, ptz_feedback_pin, ptz_turn_direction};

#[cfg(use_stubs)]
use super::{ptz_device, ptz_feedback_pin, ptz_turn_direction};

#[cfg(test)]
use super::AK_FAILED_I32;
use super::{AK_SUCCESS_I32, check_result};
use crate::hal::anyka::sdk::{PtzDirection, PtzMotor};

/// Internal trait for abstracting PTZ FFI calls to enable mocking in tests.
#[allow(dead_code)] // Some methods only used on ARM targets
#[cfg_attr(test, mockall::automock)]
pub(crate) trait PtzHalTrait: Send + Sync {
    fn ptz_open(&self) -> i32;
    fn ptz_close(&self) -> i32;
    fn ptz_check_self(&self, pin_type: ptz_feedback_pin) -> i32;
    fn ptz_turn(&self, direction: ptz_turn_direction, degree: i32) -> i32;
    fn ptz_get_step_pos(&self, motor_no: ptz_device) -> i32;
    fn ptz_stop(&self, direction: ptz_turn_direction) -> i32;
}

/// Default PTZ FFI: native Rust driver on ARM (/dev/ak-motor*), stub on host.
/// Used by HardwarePTZControl::new() and ptz_open() so the platform uses the same backend.
pub(crate) fn default_ptz_hal() -> std::sync::Arc<dyn PtzHalTrait> {
    #[cfg(not(use_stubs))]
    return crate::hal::anyka::ptz::native::NativePtzHal::new();
    #[cfg(use_stubs)]
    std::sync::Arc::new(crate::hal::stub::ptz::StubPtzHal)
}

/// RAII handle for PTZ device.
///
/// This handle automatically closes the PTZ device when dropped,
/// ensuring proper resource cleanup even in error paths.
/// The handle stores a reference to the FFI implementation used to open
/// the device, so Drop calls close on the same backend.
///
/// # Thread Safety
///
/// The underlying SDK uses internal mutexes for thread safety, so this handle
/// is safe to send and share between threads.
pub struct PTZHandle {
    opened: bool,
    ffi: std::sync::Arc<dyn PtzHalTrait>,
}

// SAFETY: PTZHandle is thread-safe - SDK uses internal mutexes.
// Similar to VideoInputHandle and AudioInputHandle, the SDK provides internal synchronization.
unsafe impl Send for PTZHandle {}
unsafe impl Sync for PTZHandle {}

impl Drop for PTZHandle {
    fn drop(&mut self) {
        if self.opened {
            let ret = self.ffi.ptz_close();
            if ret != AK_SUCCESS_I32 {
                tracing::error!(
                    "PTZ device close failed in Drop (resource may leak): error code {}",
                    ret
                );
            }
        }
    }
}

impl PTZHandle {
    /// Check if the handle is opened.
    #[cfg(test)]
    pub(crate) fn is_opened(&self) -> bool {
        self.opened
    }
}

/// Internal helper that takes FFI trait for testability.
///
/// Calls `ak_drv_ptz_open()` followed by `ak_drv_ptz_check_self(0)` to complete
/// SDK initialization, matching the C adapter's `platform_ptz_init()` sequence.
/// The self-check calibrates motor positions and transitions the SDK from
/// `PTZ_WAIT_INIT` to `PTZ_INIT_OK` state.
pub(crate) fn ptz_open(ffi: std::sync::Arc<dyn PtzHalTrait>) -> PlatformResult<PTZHandle> {
    let ret = ffi.ptz_open();
    check_result(ret, "ak_drv_ptz_open")?;

    // Self-check is required for the SDK to consider PTZ initialized.
    // PTZ_FEEDBACK_PIN_NONE = 0 (no feedback pin on this hardware).
    let ret = ffi.ptz_check_self(ptz_feedback_pin::PTZ_FEEDBACK_PIN_NONE);
    if ret != AK_SUCCESS_I32 {
        tracing::warn!(
            "PTZ self-check failed (error code {}), continuing anyway",
            ret
        );
        // Don't return error — PTZ may still work without self-check,
        // matching the C adapter's behavior.
    }

    Ok(PTZHandle { opened: true, ffi })
}

/// Validate pan range (±180 degrees).
///
/// # Arguments
///
/// * `pan` - Pan angle in degrees
///
/// # Returns
///
/// * `Ok(())` if pan is within valid range
/// * `Err(PlatformError::InvalidParameter)` if out of range
pub fn validate_pan_range(pan: f32) -> PlatformResult<()> {
    if !(-180.0..=180.0).contains(&pan) {
        Err(PlatformError::InvalidParameter(format!(
            "Pan angle {} is out of range (-180.0 to 180.0 degrees)",
            pan
        )))
    } else {
        Ok(())
    }
}

/// Validate tilt range (±90 degrees).
///
/// # Arguments
///
/// * `tilt` - Tilt angle in degrees
///
/// # Returns
///
/// * `Ok(())` if tilt is within valid range
/// * `Err(PlatformError::InvalidParameter)` if out of range
pub fn validate_tilt_range(tilt: f32) -> PlatformResult<()> {
    if !(-90.0..=90.0).contains(&tilt) {
        Err(PlatformError::InvalidParameter(format!(
            "Tilt angle {} is out of range (-90.0 to 90.0 degrees)",
            tilt
        )))
    } else {
        Ok(())
    }
}

/// Convert degrees to motor steps.
///
/// # Arguments
///
/// * `degrees` - Angle in degrees
/// * `cycle_steps` - Number of motor steps for a full 360-degree rotation
///
/// # Returns
///
/// Motor steps corresponding to the given angle
///
/// # Errors
///
/// Returns `PlatformError::InvalidParameter` if `cycle_steps` is zero
pub fn degrees_to_steps(degrees: f32, cycle_steps: i32) -> PlatformResult<i32> {
    if cycle_steps == 0 {
        return Err(PlatformError::InvalidParameter(
            "cycle_steps must be non-zero".into(),
        ));
    }
    Ok(((degrees / 360.0) * cycle_steps as f32).round() as i32)
}

/// Convert motor steps to degrees.
///
/// # Arguments
///
/// * `steps` - Motor step position
/// * `cycle_steps` - Number of motor steps for a full 360-degree rotation
///
/// # Returns
///
/// Angle in degrees corresponding to the given step position
///
/// # Errors
///
/// Returns `PlatformError::InvalidParameter` if `cycle_steps` is zero
pub fn steps_to_degrees(steps: i32, cycle_steps: i32) -> PlatformResult<f32> {
    if cycle_steps == 0 {
        return Err(PlatformError::InvalidParameter(
            "cycle_steps must be non-zero".into(),
        ));
    }
    Ok((steps as f32 / cycle_steps as f32) * 360.0)
}

#[allow(dead_code)] // Called from platform layer on ARM
pub(crate) fn ptz_turn(
    _handle: &PTZHandle,
    direction: PtzDirection,
    degrees: f32,
    ffi: &dyn PtzHalTrait,
) -> PlatformResult<()> {
    // Validate range based on direction
    match direction {
        PtzDirection::Left | PtzDirection::Right => {
            validate_pan_range(degrees)?;
        }
        PtzDirection::Up | PtzDirection::Down => {
            validate_tilt_range(degrees)?;
        }
    }

    // Convert direction to FFI enum using exhaustive match instead of transmute.
    // This ensures the compiler catches any future enum changes at compile time
    // rather than producing silent undefined behavior.
    let sdk_direction = match direction {
        PtzDirection::Left => ptz_turn_direction::PTZ_TURN_LEFT,
        PtzDirection::Right => ptz_turn_direction::PTZ_TURN_RIGHT,
        PtzDirection::Up => ptz_turn_direction::PTZ_TURN_UP,
        PtzDirection::Down => ptz_turn_direction::PTZ_TURN_DOWN,
    };

    // Convert degrees to steps (using default cycle_steps if needed)
    // Note: The SDK's ak_drv_ptz_turn() takes degrees directly, but we validate ranges
    let degree_int = degrees.round() as i32;

    let ret = ffi.ptz_turn(sdk_direction, degree_int);
    check_result(ret, "ak_drv_ptz_turn")
}

#[allow(dead_code)] // Called from platform layer on ARM
pub(crate) fn ptz_get_step_pos(
    _handle: &PTZHandle,
    motor: PtzMotor,
    ffi: &dyn PtzHalTrait,
) -> PlatformResult<i32> {
    // Convert motor to FFI enum using exhaustive match instead of transmute.
    let sdk_motor = match motor {
        PtzMotor::Horizontal => ptz_device::PTZ_DEV_H,
        PtzMotor::Vertical => ptz_device::PTZ_DEV_V,
    };

    let result = ffi.ptz_get_step_pos(sdk_motor);
    if result < 0 {
        Err(PlatformError::HardwareFailure(format!(
            "ak_drv_ptz_get_step_pos failed: error code {}",
            result
        )))
    } else {
        Ok(result)
    }
}

#[allow(dead_code)] // Called from platform layer on ARM
pub(crate) fn ptz_stop(
    _handle: &PTZHandle,
    direction: PtzDirection,
    ffi: &dyn PtzHalTrait,
) -> PlatformResult<()> {
    // Convert direction to FFI enum using exhaustive match instead of transmute.
    let sdk_direction = match direction {
        PtzDirection::Left => ptz_turn_direction::PTZ_TURN_LEFT,
        PtzDirection::Right => ptz_turn_direction::PTZ_TURN_RIGHT,
        PtzDirection::Up => ptz_turn_direction::PTZ_TURN_UP,
        PtzDirection::Down => ptz_turn_direction::PTZ_TURN_DOWN,
    };
    let ret = ffi.ptz_stop(sdk_direction);
    check_result(ret, "ak_drv_ptz_turn_stop")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Create a test handle that uses a no-op mock for close on drop.
    fn test_handle() -> PTZHandle {
        let mut mock = MockPtzHalTrait::new();
        mock.expect_ptz_close().returning(|| AK_SUCCESS_I32);
        PTZHandle {
            opened: true,
            ffi: std::sync::Arc::new(mock),
        }
    }

    #[test]
    fn test_validate_pan_range_valid() {
        assert!(validate_pan_range(0.0).is_ok());
        assert!(validate_pan_range(180.0).is_ok());
        assert!(validate_pan_range(-180.0).is_ok());
        assert!(validate_pan_range(90.0).is_ok());
    }

    #[test]
    fn test_validate_pan_range_invalid() {
        assert!(validate_pan_range(181.0).is_err());
        assert!(validate_pan_range(-181.0).is_err());
        match validate_pan_range(200.0) {
            Err(PlatformError::InvalidParameter(msg)) => {
                assert!(msg.contains("Pan angle"));
                assert!(msg.contains("out of range"));
            }
            _ => panic!("Expected InvalidParameter error"),
        }
    }

    #[test]
    fn test_validate_tilt_range_valid() {
        assert!(validate_tilt_range(0.0).is_ok());
        assert!(validate_tilt_range(90.0).is_ok());
        assert!(validate_tilt_range(-90.0).is_ok());
        assert!(validate_tilt_range(45.0).is_ok());
    }

    #[test]
    fn test_validate_tilt_range_invalid() {
        assert!(validate_tilt_range(91.0).is_err());
        assert!(validate_tilt_range(-91.0).is_err());
        match validate_tilt_range(100.0) {
            Err(PlatformError::InvalidParameter(msg)) => {
                assert!(msg.contains("Tilt angle"));
                assert!(msg.contains("out of range"));
            }
            _ => panic!("Expected InvalidParameter error"),
        }
    }

    #[test]
    fn test_degrees_to_steps() {
        // 360 degrees should equal cycle_steps
        assert_eq!(degrees_to_steps(360.0, 2000).unwrap(), 2000);
        // 180 degrees should equal half cycle_steps
        assert_eq!(degrees_to_steps(180.0, 2000).unwrap(), 1000);
        // 90 degrees should equal quarter cycle_steps
        assert_eq!(degrees_to_steps(90.0, 2000).unwrap(), 500);
        // 0 degrees should equal 0 steps
        assert_eq!(degrees_to_steps(0.0, 2000).unwrap(), 0);
    }

    #[test]
    fn test_steps_to_degrees() {
        // cycle_steps should equal 360 degrees
        assert!((steps_to_degrees(2000, 2000).unwrap() - 360.0).abs() < 0.01);
        // Half cycle_steps should equal 180 degrees
        assert!((steps_to_degrees(1000, 2000).unwrap() - 180.0).abs() < 0.01);
        // Quarter cycle_steps should equal 90 degrees
        assert!((steps_to_degrees(500, 2000).unwrap() - 90.0).abs() < 0.01);
        // 0 steps should equal 0 degrees
        assert!((steps_to_degrees(0, 2000).unwrap() - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_degrees_to_steps_zero_cycle_steps() {
        // Should return error for zero cycle_steps
        assert!(degrees_to_steps(90.0, 0).is_err());
    }

    #[test]
    fn test_steps_to_degrees_zero_cycle_steps() {
        // Should return error for zero cycle_steps
        assert!(steps_to_degrees(500, 0).is_err());
    }

    #[test]
    fn test_ptz_open_calls_ffi_and_returns_handle() {
        let mut mock_ffi = MockPtzHalTrait::new();

        mock_ffi
            .expect_ptz_open()
            .times(1)
            .returning(|| AK_SUCCESS_I32);
        mock_ffi
            .expect_ptz_check_self()
            .times(1)
            .returning(|_| AK_SUCCESS_I32);
        mock_ffi.expect_ptz_close().returning(|| AK_SUCCESS_I32);

        let result = ptz_open(std::sync::Arc::new(mock_ffi));
        assert!(result.is_ok());
        let handle = result.unwrap();
        assert!(handle.is_opened());
    }

    #[test]
    fn test_ptz_open_returns_error_on_failure() {
        let mut mock_ffi = MockPtzHalTrait::new();

        mock_ffi
            .expect_ptz_open()
            .times(1)
            .returning(|| AK_FAILED_I32);

        let result = ptz_open(std::sync::Arc::new(mock_ffi));
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareFailure(msg)) => {
                assert!(msg.contains("ak_drv_ptz_open"));
            }
            _ => panic!("Expected HardwareFailure error"),
        }
    }

    #[test]
    fn test_ptz_turn_calls_ffi() {
        let mut mock_ffi = MockPtzHalTrait::new();
        let handle = test_handle();

        mock_ffi
            .expect_ptz_turn()
            .withf(|dir, deg| {
                // Check direction and degree values
                let dir_val: i32 = unsafe { std::mem::transmute(*dir) };
                dir_val == 1 && *deg == 45 // Left direction, 45 degrees
            })
            .times(1)
            .returning(|_, _| AK_SUCCESS_I32);

        let result = ptz_turn(&handle, PtzDirection::Left, 45.0, &mock_ffi);
        assert!(result.is_ok());
    }

    #[test]
    fn test_ptz_turn_validates_pan_range() {
        let mock_ffi = MockPtzHalTrait::new();
        let handle = test_handle();

        // Should fail validation before calling FFI
        let result = ptz_turn(&handle, PtzDirection::Left, 200.0, &mock_ffi);
        assert!(result.is_err());
        match result {
            Err(PlatformError::InvalidParameter(msg)) => {
                assert!(msg.contains("Pan angle"));
            }
            _ => panic!("Expected InvalidParameter error"),
        }
    }

    #[test]
    fn test_ptz_turn_validates_tilt_range() {
        let mock_ffi = MockPtzHalTrait::new();
        let handle = test_handle();

        // Should fail validation before calling FFI
        let result = ptz_turn(&handle, PtzDirection::Up, 100.0, &mock_ffi);
        assert!(result.is_err());
        match result {
            Err(PlatformError::InvalidParameter(msg)) => {
                assert!(msg.contains("Tilt angle"));
            }
            _ => panic!("Expected InvalidParameter error"),
        }
    }

    #[test]
    fn test_ptz_turn_propagates_error() {
        let mut mock_ffi = MockPtzHalTrait::new();
        let handle = test_handle();

        mock_ffi
            .expect_ptz_turn()
            .times(1)
            .returning(|_, _| AK_FAILED_I32);

        let result = ptz_turn(&handle, PtzDirection::Right, 30.0, &mock_ffi);
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareFailure(msg)) => {
                assert!(msg.contains("ak_drv_ptz_turn"));
            }
            _ => panic!("Expected HardwareFailure error"),
        }
    }

    #[test]
    fn test_ptz_get_step_pos_calls_ffi_and_returns_position() {
        let mut mock_ffi = MockPtzHalTrait::new();
        let handle = test_handle();

        mock_ffi
            .expect_ptz_get_step_pos()
            .withf(|motor| {
                let motor_val: i32 = unsafe { std::mem::transmute(*motor) };
                motor_val == 0 // Horizontal
            })
            .times(1)
            .returning(|_| 1500);

        let result = ptz_get_step_pos(&handle, PtzMotor::Horizontal, &mock_ffi);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1500);
    }

    #[test]
    fn test_ptz_get_step_pos_propagates_error() {
        let mut mock_ffi = MockPtzHalTrait::new();
        let handle = test_handle();

        mock_ffi
            .expect_ptz_get_step_pos()
            .times(1)
            .returning(|_| -1);

        let result = ptz_get_step_pos(&handle, PtzMotor::Vertical, &mock_ffi);
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareFailure(msg)) => {
                assert!(msg.contains("ak_drv_ptz_get_step_pos"));
            }
            _ => panic!("Expected HardwareFailure error"),
        }
    }

    #[test]
    fn test_ptz_stop_calls_ffi() {
        let mut mock_ffi = MockPtzHalTrait::new();
        let handle = test_handle();

        mock_ffi
            .expect_ptz_stop()
            .withf(|dir| *dir == ptz_turn_direction::PTZ_TURN_RIGHT)
            .times(1)
            .returning(|_| AK_SUCCESS_I32);

        let result = ptz_stop(&handle, PtzDirection::Right, &mock_ffi);
        assert!(result.is_ok());
    }

    #[test]
    fn test_ptz_stop_propagates_error() {
        let mut mock_ffi = MockPtzHalTrait::new();
        let handle = test_handle();

        mock_ffi
            .expect_ptz_stop()
            .withf(|dir| *dir == ptz_turn_direction::PTZ_TURN_DOWN)
            .times(1)
            .returning(|_| AK_FAILED_I32);

        let result = ptz_stop(&handle, PtzDirection::Down, &mock_ffi);
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareFailure(msg)) => {
                assert!(msg.contains("ak_drv_ptz_turn_stop"));
            }
            _ => panic!("Expected HardwareFailure error"),
        }
    }
}
