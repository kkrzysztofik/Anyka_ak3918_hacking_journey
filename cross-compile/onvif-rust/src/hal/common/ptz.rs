//! Safe Rust wrappers for the native PTZ motor driver.
//!
//! The PTZ path does not go through the Anyka C SDK: `hal/anyka/ptz/driver.rs` talks to
//! `/dev/ak-motor{0,1}` directly via ioctl. [`PtzHalTrait`] is the mockable seam between
//! that driver and the platform layer.
//!
//! # Error Handling
//!
//! Every fallible method returns [`PlatformResult`], **not** an SDK-style `i32`. The
//! driver builds errors that carry the failing device path and `errno`; flattening those
//! to `-1` at this boundary is what made PTZ bring-up failures undiagnosable.

use crate::platform::PlatformError;
use crate::platform::PlatformResult;
use serde::Serialize;

// On ARM we use the Rust native driver; types from ptz/driver.rs (ak_drv_ptz.h omitted
// from bindgen). On host builds the mirrored types in `common::sdk_types` stand in.
#[cfg(not(use_stubs))]
use crate::hal::anyka::ptz::{ptz_device, ptz_feedback_pin, ptz_turn_direction};

#[cfg(use_stubs)]
use super::{ptz_device, ptz_feedback_pin, ptz_turn_direction};

/// Whether the kernel's own position accounting is live on this motor driver.
///
/// V500 boards accept `MOTOR_GET_STATUS`, return success, and write nothing into the
/// caller's buffer. A step position read back from such a board is a zero, not a
/// measurement, and rendering it as data misleads whoever is diagnosing the camera.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum StepReadback {
    /// The kernel wrote plausible geometry back — step positions mean something.
    Working,
    /// The ioctl succeeded but wrote nothing. Step positions are always zero.
    Unsupported,
    /// Never probed, or the probe itself failed.
    #[default]
    Unknown,
}

impl StepReadback {
    /// Combine two per-motor verdicts into one for the device.
    ///
    /// Pessimistic on purpose: if either motor cannot report its position, the pair
    /// cannot, and a consumer has nothing useful to do with "pan works, tilt does not".
    pub fn worst_of(a: Self, b: Self) -> Self {
        use StepReadback::{Unknown, Unsupported, Working};
        match (a, b) {
            (Unsupported, _) | (_, Unsupported) => Unsupported,
            (Unknown, _) | (_, Unknown) => Unknown,
            (Working, Working) => Working,
        }
    }
}

/// Outcome of waiting for an in-flight motor turn to finish or be interrupted.
///
/// Defined here rather than in the ARM-only driver so the trait signature — and its
/// mock — are available on host builds too.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TurnOutcome {
    /// `true` if the wait ended because the stop/interrupt flag was set (preempted),
    /// not because the motor signalled completion.
    pub interrupted: bool,
    /// Motor step position read back from `MOTOR_GET_STATUS` after the turn.
    pub step_pos: i32,
    /// Raw event bitmask from the kernel `notify_data` (0 when interrupted).
    pub event: i32,
}

/// Internal trait abstracting the PTZ motor driver, to enable mocking in tests.
#[allow(dead_code)] // Some methods only used on ARM targets
#[cfg_attr(test, mockall::automock)]
pub(crate) trait PtzHalTrait: Send + Sync {
    fn ptz_open(&self) -> PlatformResult<()>;
    fn ptz_close(&self) -> PlatformResult<()>;
    /// Run the calibration sweep (turn to physical limit, then to middle).
    ///
    /// Returns whether the kernel's position accounting proved live during the sweep —
    /// the driver probes `MOTOR_GET_STATUS` once the motors have demonstrably moved,
    /// which is the only moment the answer is knowable.
    fn ptz_check_self(&self, pin_type: ptz_feedback_pin) -> PlatformResult<StepReadback>;
    /// Issue a turn without waiting for completion (non-blocking).
    ///
    /// Used by the PTZ actor so it can acknowledge the command immediately and wait for
    /// completion separately via [`Self::ptz_wait_turn`]. `Ok(false)` means the requested
    /// move rounded to zero steps and no turn was issued.
    fn ptz_start_turn(&self, direction: ptz_turn_direction, degree: i32) -> PlatformResult<bool>;
    /// Block until the in-flight turn on `direction`'s motor completes or is interrupted,
    /// reconciling position from `MOTOR_GET_STATUS`.
    fn ptz_wait_turn(&self, direction: ptz_turn_direction) -> PlatformResult<TurnOutcome>;
    fn ptz_get_step_pos(&self, motor_no: ptz_device) -> PlatformResult<i32>;
    fn ptz_stop(&self, direction: ptz_turn_direction) -> PlatformResult<()>;
    /// Set the interrupt flag so any in-flight turn/wait returns promptly. Safe to call
    /// concurrently with a pending turn (does not take the driver's main lock).
    fn ptz_interrupt(&self);
}

/// Default PTZ backend: native Rust driver on ARM (/dev/ak-motor*), stub on host.
/// Used by `AnykaPTZControl` in `platform/anyka/ptz_control.rs` and `ptz_open()`
/// so the platform uses the same backend.
pub(crate) fn default_ptz_hal() -> std::sync::Arc<dyn PtzHalTrait> {
    #[cfg(not(use_stubs))]
    return std::sync::Arc::new(crate::hal::anyka::ptz::NativePtzDriver::new());
    #[cfg(use_stubs)]
    std::sync::Arc::new(crate::hal::stub::ptz::StubPtzHal)
}

/// RAII handle for the PTZ device.
///
/// Closes the device when dropped, so error paths cannot leak it. The handle keeps a
/// reference to the backend it was opened with, so `Drop` closes the same one.
pub struct PTZHandle {
    opened: bool,
    ffi: std::sync::Arc<dyn PtzHalTrait>,
    /// Calibration sweep failure, kept for diagnostics. `None` = the sweep succeeded.
    self_check_error: Option<String>,
    /// Whether the motor driver's position accounting proved live during the sweep.
    step_readback: StepReadback,
}

impl Drop for PTZHandle {
    fn drop(&mut self) {
        if self.opened
            && let Err(e) = self.ffi.ptz_close()
        {
            tracing::error!("PTZ device close failed in Drop (resource may leak): {}", e);
        }
    }
}

impl PTZHandle {
    /// Check if the handle is opened.
    #[cfg(test)]
    pub(crate) fn is_opened(&self) -> bool {
        self.opened
    }

    /// Why the calibration sweep failed, or `None` if it succeeded.
    pub(crate) fn self_check_error(&self) -> Option<&str> {
        self.self_check_error.as_deref()
    }

    /// Whether motor step positions from this device are measurements or always zero.
    pub(crate) fn step_readback(&self) -> StepReadback {
        self.step_readback
    }
}

/// Open the motor devices and run the calibration self-check.
///
/// Takes the backend as an argument for testability. A self-check failure is logged but
/// not fatal: the devices are open and relative moves still work, they are just not
/// referenced to a known origin.
pub(crate) fn ptz_open(ffi: std::sync::Arc<dyn PtzHalTrait>) -> PlatformResult<PTZHandle> {
    ffi.ptz_open()?;

    // PTZ_FEEDBACK_PIN_NONE = 0 (no feedback pin on this hardware).
    let (self_check_error, step_readback) =
        match ffi.ptz_check_self(ptz_feedback_pin::PTZ_FEEDBACK_PIN_NONE) {
            Ok(readback) => (None, readback),
            Err(e) => {
                tracing::warn!("PTZ self-check failed, continuing anyway: {}", e);
                // The sweep never reached its probe, so the readback question is open.
                (Some(e.to_string()), StepReadback::Unknown)
            }
        };

    Ok(PTZHandle {
        opened: true,
        ffi,
        self_check_error,
        step_readback,
    })
}

/// Convert degrees to motor steps.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_step_readback_worst_of_prefers_unsupported() {
        use StepReadback::{Unknown, Unsupported, Working};
        assert_eq!(StepReadback::worst_of(Working, Working), Working);
        assert_eq!(StepReadback::worst_of(Working, Unknown), Unknown);
        assert_eq!(StepReadback::worst_of(Unknown, Unsupported), Unsupported);
        assert_eq!(StepReadback::worst_of(Unsupported, Working), Unsupported);
    }

    #[test]
    fn test_ptz_open_records_working_step_readback() {
        let mut ffi = MockPtzHalTrait::new();
        ffi.expect_ptz_open().returning(|| Ok(()));
        ffi.expect_ptz_close().returning(|| Ok(()));
        ffi.expect_ptz_check_self()
            .returning(|_| Ok(StepReadback::Working));

        let handle = ptz_open(std::sync::Arc::new(ffi)).expect("open should succeed");
        assert_eq!(handle.step_readback(), StepReadback::Working);
        assert!(handle.self_check_error().is_none());
    }

    #[test]
    fn test_ptz_open_self_check_failure_leaves_readback_unknown() {
        let mut ffi = MockPtzHalTrait::new();
        ffi.expect_ptz_open().returning(|| Ok(()));
        ffi.expect_ptz_close().returning(|| Ok(()));
        ffi.expect_ptz_check_self()
            .returning(|_| Err(PlatformError::HardwareFailure("sweep timed out".into())));

        let handle = ptz_open(std::sync::Arc::new(ffi)).expect("open still succeeds");
        // A failed sweep says nothing about whether the status ioctl works.
        assert_eq!(handle.step_readback(), StepReadback::Unknown);
        assert!(handle.self_check_error().is_some());
        assert!(
            handle.is_opened(),
            "a failed sweep must not close the device"
        );
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
        assert!(degrees_to_steps(90.0, 0).is_err());
    }

    #[test]
    fn test_steps_to_degrees_zero_cycle_steps() {
        assert!(steps_to_degrees(500, 0).is_err());
    }

    #[test]
    fn test_ptz_open_calls_ffi_and_returns_handle() {
        let mut mock_ffi = MockPtzHalTrait::new();

        mock_ffi.expect_ptz_open().times(1).returning(|| Ok(()));
        mock_ffi
            .expect_ptz_check_self()
            .times(1)
            .returning(|_| Ok(StepReadback::Unknown));
        mock_ffi.expect_ptz_close().returning(|| Ok(()));

        let result = ptz_open(std::sync::Arc::new(mock_ffi));
        assert!(result.is_ok());
        let handle = result.unwrap();
        assert!(handle.is_opened());
    }

    /// The whole point of the `PlatformResult` boundary: the driver's message (device
    /// path + errno) must survive to the caller instead of collapsing to `-1`.
    #[test]
    fn test_ptz_open_propagates_driver_error_detail() {
        let mut mock_ffi = MockPtzHalTrait::new();

        mock_ffi.expect_ptz_open().times(1).returning(|| {
            Err(PlatformError::HardwareFailure(
                "open /dev/ak-motor0: errno 19".to_string(),
            ))
        });

        let result = ptz_open(std::sync::Arc::new(mock_ffi));
        match result {
            Err(PlatformError::HardwareFailure(msg)) => {
                assert!(msg.contains("/dev/ak-motor0"), "lost device path: {}", msg);
                assert!(msg.contains("errno 19"), "lost errno: {}", msg);
            }
            _ => panic!("Expected HardwareFailure error"),
        }
    }

    /// A self-check failure must not fail the open — the devices are usable without it.
    #[test]
    fn test_ptz_open_tolerates_check_self_failure() {
        let mut mock_ffi = MockPtzHalTrait::new();

        mock_ffi.expect_ptz_open().times(1).returning(|| Ok(()));
        mock_ffi.expect_ptz_check_self().times(1).returning(|_| {
            Err(PlatformError::HardwareFailure(
                "motor wait timed out".to_string(),
            ))
        });
        mock_ffi.expect_ptz_close().returning(|| Ok(()));

        assert!(ptz_open(std::sync::Arc::new(mock_ffi)).is_ok());
    }

    #[test]
    fn test_ptz_open_records_check_self_failure_on_the_handle() {
        let mut mock_ffi = MockPtzHalTrait::new();
        mock_ffi.expect_ptz_open().times(1).returning(|| Ok(()));
        mock_ffi.expect_ptz_check_self().times(1).returning(|_| {
            Err(PlatformError::HardwareFailure(
                "motor wait timed out".to_string(),
            ))
        });
        mock_ffi.expect_ptz_close().returning(|| Ok(()));

        let handle = ptz_open(std::sync::Arc::new(mock_ffi)).unwrap();
        assert!(
            handle
                .self_check_error()
                .is_some_and(|e| e.contains("motor wait timed out")),
            "a warn! log is not reachable from the WebUI; the handle must carry it"
        );
    }

    #[test]
    fn test_ptz_open_clean_self_check_records_no_error() {
        let mut mock_ffi = MockPtzHalTrait::new();
        mock_ffi.expect_ptz_open().times(1).returning(|| Ok(()));
        mock_ffi
            .expect_ptz_check_self()
            .times(1)
            .returning(|_| Ok(StepReadback::Unknown));
        mock_ffi.expect_ptz_close().returning(|| Ok(()));

        let handle = ptz_open(std::sync::Arc::new(mock_ffi)).unwrap();
        assert!(handle.self_check_error().is_none());
    }
}
