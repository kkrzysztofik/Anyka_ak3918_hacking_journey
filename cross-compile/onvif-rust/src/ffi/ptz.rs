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
//! use onvif_rust::ffi::ptz::*;
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

#[cfg(not(use_stubs))]
use crate::ffi::generated::{ptz_device, ptz_turn_direction};

#[cfg(use_stubs)]
use crate::ffi::{ptz_device, ptz_turn_direction};

use crate::ffi::{AK_FAILED, AK_SUCCESS, PtzDirection, PtzMotor};

/// Internal trait for abstracting PTZ FFI calls to enable mocking in tests.
#[cfg_attr(test, mockall::automock)]
pub(crate) trait PtzFfiTrait: Send + Sync {
    fn ptz_open(&self) -> i32;
    fn ptz_close(&self) -> i32;
    fn ptz_turn(&self, direction: ptz_turn_direction, degree: i32) -> i32;
    fn ptz_get_step_pos(&self, motor_no: ptz_device) -> i32;
    fn ptz_stop(&self) -> i32;
}

/// Default implementation that calls the real FFI functions.
pub(crate) struct RealPtzFfi;

impl PtzFfiTrait for RealPtzFfi {
    #[cfg(not(use_stubs))]
    fn ptz_open(&self) -> i32 {
        unsafe extern "C" {
            fn ak_drv_ptz_open() -> i32;
        }
        unsafe { ak_drv_ptz_open() }
    }

    #[cfg(use_stubs)]
    fn ptz_open(&self) -> i32 {
        AK_SUCCESS
    }

    #[cfg(not(use_stubs))]
    fn ptz_close(&self) -> i32 {
        unsafe extern "C" {
            fn ak_drv_ptz_close() -> i32;
        }
        unsafe { ak_drv_ptz_close() }
    }

    #[cfg(use_stubs)]
    fn ptz_close(&self) -> i32 {
        AK_SUCCESS
    }

    #[cfg(not(use_stubs))]
    fn ptz_turn(&self, direction: ptz_turn_direction, degree: i32) -> i32 {
        unsafe extern "C" {
            fn ak_drv_ptz_turn(direction: ptz_turn_direction, degree: i32) -> i32;
        }
        unsafe { ak_drv_ptz_turn(direction, degree) }
    }

    #[cfg(use_stubs)]
    fn ptz_turn(&self, _direction: ptz_turn_direction, _degree: i32) -> i32 {
        AK_SUCCESS
    }

    #[cfg(not(use_stubs))]
    fn ptz_get_step_pos(&self, motor_no: ptz_device) -> i32 {
        unsafe extern "C" {
            fn ak_drv_ptz_get_step_pos(motor_no: ptz_device) -> i32;
        }
        unsafe { ak_drv_ptz_get_step_pos(motor_no) }
    }

    #[cfg(use_stubs)]
    fn ptz_get_step_pos(&self, _motor_no: ptz_device) -> i32 {
        0 // Return 0 steps for stub
    }

    #[cfg(not(use_stubs))]
    fn ptz_stop(&self) -> i32 {
        unsafe extern "C" {
            fn ak_drv_ptz_stop() -> i32;
        }
        unsafe { ak_drv_ptz_stop() }
    }

    #[cfg(use_stubs)]
    fn ptz_stop(&self) -> i32 {
        AK_SUCCESS
    }
}

// Global instance for default FFI implementation
static REAL_PTZ_FFI: RealPtzFfi = RealPtzFfi;

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
        AK_SUCCESS => Ok(()),
        AK_FAILED => Err(PlatformError::HardwareFailure(format!(
            "{} failed",
            context
        ))),
        _ => Err(PlatformError::HardwareFailure(format!(
            "{}: error code {}",
            context, ret
        ))),
    }
}

/// RAII handle for PTZ device.
///
/// This handle automatically closes the PTZ device when dropped,
/// ensuring proper resource cleanup even in error paths.
///
/// # Thread Safety
///
/// The underlying SDK uses internal mutexes for thread safety, so this handle
/// is safe to send and share between threads.
pub struct PTZHandle {
    opened: bool,
}

// SAFETY: PTZHandle is thread-safe - SDK uses internal mutexes.
// Similar to VideoInputHandle and AudioInputHandle, the SDK provides internal synchronization.
unsafe impl Send for PTZHandle {}
unsafe impl Sync for PTZHandle {}

impl Drop for PTZHandle {
    fn drop(&mut self) {
        if self.opened {
            let _ = REAL_PTZ_FFI.ptz_close();
        }
    }
}

impl PTZHandle {
    /// Check if the handle is opened.
    #[allow(dead_code)]
    pub(crate) fn is_opened(&self) -> bool {
        self.opened
    }
}

/// Internal helper that takes FFI trait for testability.
pub(crate) fn ptz_open_internal(ffi: &dyn PtzFfiTrait) -> PlatformResult<PTZHandle> {
    let ret = ffi.ptz_open();
    check_result(ret, "ak_drv_ptz_open")?;
    Ok(PTZHandle { opened: true })
}

/// Open a PTZ device.
///
/// # Returns
///
/// * `Ok(PTZHandle)` on success
/// * `Err(PlatformError::HardwareFailure)` if device cannot be opened
///
/// # Safety
///
/// The FFI call is safe because:
/// - `ak_drv_ptz_open()` returns an error code (0 = success, negative = error)
/// - We validate the result and map errors appropriately
/// - Handle is wrapped in `PTZHandle` for RAII cleanup
pub fn ptz_open() -> PlatformResult<PTZHandle> {
    ptz_open_internal(&REAL_PTZ_FFI)
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
pub fn degrees_to_steps(degrees: f32, cycle_steps: i32) -> i32 {
    ((degrees / 360.0) * cycle_steps as f32).round() as i32
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
pub fn steps_to_degrees(steps: i32, cycle_steps: i32) -> f32 {
    (steps as f32 / cycle_steps as f32) * 360.0
}

/// Internal helper that takes FFI trait for testability.
pub(crate) fn ptz_turn_internal(
    _handle: &PTZHandle,
    direction: PtzDirection,
    degrees: f32,
    ffi: &dyn PtzFfiTrait,
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

    // Validate: Enum ensures only valid direction values
    let sdk_direction: ptz_turn_direction =
        unsafe { std::mem::transmute(direction.to_direction_code()) };

    // Convert degrees to steps (using default cycle_steps if needed)
    // Note: The SDK's ak_drv_ptz_turn() takes degrees directly, but we validate ranges
    let degree_int = degrees.round() as i32;

    let ret = ffi.ptz_turn(sdk_direction, degree_int);
    check_result(ret, "ak_drv_ptz_turn")
}

/// Turn PTZ motor in a direction by a specified angle.
///
/// # Arguments
///
/// * `handle` - PTZ handle
/// * `direction` - Direction to turn (Left, Right, Up, Down)
/// * `degrees` - Angle in degrees (validated: ±180° for pan, ±90° for tilt)
///
/// # Returns
///
/// * `Ok(())` on success
/// * `Err(PlatformError::InvalidParameter)` if angle is out of range
/// * `Err(PlatformError::HardwareFailure)` on SDK error
pub fn ptz_turn(handle: &PTZHandle, direction: PtzDirection, degrees: f32) -> PlatformResult<()> {
    ptz_turn_internal(handle, direction, degrees, &REAL_PTZ_FFI)
}

/// Internal helper that takes FFI trait for testability.
pub(crate) fn ptz_get_step_pos_internal(
    _handle: &PTZHandle,
    motor: PtzMotor,
    ffi: &dyn PtzFfiTrait,
) -> PlatformResult<i32> {
    // Validate: Enum ensures only valid motor values (0 or 1)
    let sdk_motor: ptz_device = unsafe { std::mem::transmute(motor.to_device_id()) };

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

/// Get PTZ motor step position.
///
/// # Arguments
///
/// * `handle` - PTZ handle
/// * `motor` - Motor to query (Horizontal or Vertical)
///
/// # Returns
///
/// * `Ok(i32)` with step position on success
/// * `Err(PlatformError::HardwareFailure)` on SDK error
pub fn ptz_get_step_pos(handle: &PTZHandle, motor: PtzMotor) -> PlatformResult<i32> {
    ptz_get_step_pos_internal(handle, motor, &REAL_PTZ_FFI)
}

/// Internal helper that takes FFI trait for testability.
pub(crate) fn ptz_stop_internal(
    _handle: &PTZHandle,
    _direction: PtzDirection,
    ffi: &dyn PtzFfiTrait,
) -> PlatformResult<()> {
    let ret = ffi.ptz_stop();
    check_result(ret, "ak_drv_ptz_stop")
}

/// Stop PTZ motor movement.
///
/// # Arguments
///
/// * `handle` - PTZ handle
/// * `direction` - Direction parameter (kept for API compatibility, but SDK stop function stops all movement)
///
/// # Returns
///
/// * `Ok(())` on success
/// * `Err(PlatformError::HardwareFailure)` on SDK error
pub fn ptz_stop(handle: &PTZHandle, direction: PtzDirection) -> PlatformResult<()> {
    ptz_stop_internal(handle, direction, &REAL_PTZ_FFI)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_result_success() {
        let result = check_result(AK_SUCCESS, "test_function");
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_result_failed() {
        let result = check_result(AK_FAILED, "test_function");
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
        assert_eq!(degrees_to_steps(360.0, 2000), 2000);
        // 180 degrees should equal half cycle_steps
        assert_eq!(degrees_to_steps(180.0, 2000), 1000);
        // 90 degrees should equal quarter cycle_steps
        assert_eq!(degrees_to_steps(90.0, 2000), 500);
        // 0 degrees should equal 0 steps
        assert_eq!(degrees_to_steps(0.0, 2000), 0);
    }

    #[test]
    fn test_steps_to_degrees() {
        // cycle_steps should equal 360 degrees
        assert!((steps_to_degrees(2000, 2000) - 360.0).abs() < 0.01);
        // Half cycle_steps should equal 180 degrees
        assert!((steps_to_degrees(1000, 2000) - 180.0).abs() < 0.01);
        // Quarter cycle_steps should equal 90 degrees
        assert!((steps_to_degrees(500, 2000) - 90.0).abs() < 0.01);
        // 0 steps should equal 0 degrees
        assert!((steps_to_degrees(0, 2000) - 0.0).abs() < 0.01);
    }

    #[test]
    #[cfg(use_stubs)]
    fn test_ptz_open_success() {
        let result = ptz_open();
        assert!(result.is_ok());
        let handle = result.unwrap();
        assert!(handle.is_opened());
    }

    #[test]
    fn test_ptz_open_internal_calls_ffi_and_returns_handle() {
        let mut mock_ffi = MockPtzFfiTrait::new();

        mock_ffi.expect_ptz_open().times(1).returning(|| AK_SUCCESS);

        let result = ptz_open_internal(&mock_ffi);
        assert!(result.is_ok());
        let handle = result.unwrap();
        assert!(handle.is_opened());
    }

    #[test]
    fn test_ptz_open_internal_returns_error_on_failure() {
        let mut mock_ffi = MockPtzFfiTrait::new();

        mock_ffi.expect_ptz_open().times(1).returning(|| AK_FAILED);

        let result = ptz_open_internal(&mock_ffi);
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareFailure(msg)) => {
                assert!(msg.contains("ak_drv_ptz_open"));
            }
            _ => panic!("Expected HardwareFailure error"),
        }
    }

    #[test]
    fn test_ptz_turn_internal_calls_ffi() {
        let mut mock_ffi = MockPtzFfiTrait::new();
        let handle = PTZHandle { opened: true };

        mock_ffi
            .expect_ptz_turn()
            .withf(|dir, deg| {
                // Check direction and degree values
                let dir_val: i32 = unsafe { std::mem::transmute(*dir) };
                dir_val == 1 && *deg == 45 // Left direction, 45 degrees
            })
            .times(1)
            .returning(|_, _| AK_SUCCESS);

        let result = ptz_turn_internal(&handle, PtzDirection::Left, 45.0, &mock_ffi);
        assert!(result.is_ok());
    }

    #[test]
    fn test_ptz_turn_internal_validates_pan_range() {
        let mock_ffi = MockPtzFfiTrait::new();
        let handle = PTZHandle { opened: true };

        // Should fail validation before calling FFI
        let result = ptz_turn_internal(&handle, PtzDirection::Left, 200.0, &mock_ffi);
        assert!(result.is_err());
        match result {
            Err(PlatformError::InvalidParameter(msg)) => {
                assert!(msg.contains("Pan angle"));
            }
            _ => panic!("Expected InvalidParameter error"),
        }
    }

    #[test]
    fn test_ptz_turn_internal_validates_tilt_range() {
        let mock_ffi = MockPtzFfiTrait::new();
        let handle = PTZHandle { opened: true };

        // Should fail validation before calling FFI
        let result = ptz_turn_internal(&handle, PtzDirection::Up, 100.0, &mock_ffi);
        assert!(result.is_err());
        match result {
            Err(PlatformError::InvalidParameter(msg)) => {
                assert!(msg.contains("Tilt angle"));
            }
            _ => panic!("Expected InvalidParameter error"),
        }
    }

    #[test]
    fn test_ptz_turn_internal_propagates_error() {
        let mut mock_ffi = MockPtzFfiTrait::new();
        let handle = PTZHandle { opened: true };

        mock_ffi
            .expect_ptz_turn()
            .times(1)
            .returning(|_, _| AK_FAILED);

        let result = ptz_turn_internal(&handle, PtzDirection::Right, 30.0, &mock_ffi);
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareFailure(msg)) => {
                assert!(msg.contains("ak_drv_ptz_turn"));
            }
            _ => panic!("Expected HardwareFailure error"),
        }
    }

    #[test]
    fn test_ptz_get_step_pos_internal_calls_ffi_and_returns_position() {
        let mut mock_ffi = MockPtzFfiTrait::new();
        let handle = PTZHandle { opened: true };

        mock_ffi
            .expect_ptz_get_step_pos()
            .withf(|motor| {
                let motor_val: i32 = unsafe { std::mem::transmute(*motor) };
                motor_val == 0 // Horizontal
            })
            .times(1)
            .returning(|_| 1500);

        let result = ptz_get_step_pos_internal(&handle, PtzMotor::Horizontal, &mock_ffi);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1500);
    }

    #[test]
    fn test_ptz_get_step_pos_internal_propagates_error() {
        let mut mock_ffi = MockPtzFfiTrait::new();
        let handle = PTZHandle { opened: true };

        mock_ffi
            .expect_ptz_get_step_pos()
            .times(1)
            .returning(|_| -1);

        let result = ptz_get_step_pos_internal(&handle, PtzMotor::Vertical, &mock_ffi);
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareFailure(msg)) => {
                assert!(msg.contains("ak_drv_ptz_get_step_pos"));
            }
            _ => panic!("Expected HardwareFailure error"),
        }
    }

    #[test]
    fn test_ptz_stop_internal_calls_ffi() {
        let mut mock_ffi = MockPtzFfiTrait::new();
        let handle = PTZHandle { opened: true };

        mock_ffi.expect_ptz_stop().times(1).returning(|| AK_SUCCESS);

        let result = ptz_stop_internal(&handle, PtzDirection::Right, &mock_ffi);
        assert!(result.is_ok());
    }

    #[test]
    fn test_ptz_stop_internal_propagates_error() {
        let mut mock_ffi = MockPtzFfiTrait::new();
        let handle = PTZHandle { opened: true };

        mock_ffi.expect_ptz_stop().times(1).returning(|| AK_FAILED);

        let result = ptz_stop_internal(&handle, PtzDirection::Down, &mock_ffi);
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareFailure(msg)) => {
                assert!(msg.contains("ak_drv_ptz_stop"));
            }
            _ => panic!("Expected HardwareFailure error"),
        }
    }
}
