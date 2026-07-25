//! Stub PTZ HAL implementation for host-side testing.

/// Stub implementation for host tests (use_stubs). No hardware.
#[allow(dead_code)] // Used on host targets only
pub(crate) struct StubPtzHal;

#[cfg(use_stubs)]
use crate::hal::common::ptz::PtzHalTrait;
#[cfg(use_stubs)]
use crate::hal::common::{AK_SUCCESS_I32, ptz_device, ptz_feedback_pin, ptz_turn_direction};

#[cfg(use_stubs)]
impl PtzHalTrait for StubPtzHal {
    fn ptz_open(&self) -> i32 {
        AK_SUCCESS_I32
    }

    fn ptz_close(&self) -> i32 {
        AK_SUCCESS_I32
    }

    fn ptz_check_self(&self, _pin_type: ptz_feedback_pin) -> i32 {
        AK_SUCCESS_I32
    }

    fn ptz_turn(&self, _direction: ptz_turn_direction, _degree: i32) -> i32 {
        AK_SUCCESS_I32
    }

    fn ptz_start_turn(&self, _direction: ptz_turn_direction, _degree: i32) -> i32 {
        AK_SUCCESS_I32
    }

    fn ptz_wait_turn(&self, _direction: ptz_turn_direction) -> i32 {
        AK_SUCCESS_I32
    }

    fn ptz_get_step_pos(&self, _motor_no: ptz_device) -> i32 {
        0 // Return 0 steps for stub
    }

    fn ptz_stop(&self, _direction: ptz_turn_direction) -> i32 {
        AK_SUCCESS_I32
    }

    fn ptz_interrupt(&self) {}
}

#[cfg(all(test, use_stubs))]
mod tests {
    use super::*;
    use crate::hal::common::ptz::ptz_open;

    #[test]
    fn test_ptz_open_success() {
        let stub: std::sync::Arc<dyn PtzHalTrait> = std::sync::Arc::new(StubPtzHal);
        let result = ptz_open(stub);
        assert!(result.is_ok());
        let handle = result.unwrap();
        assert!(handle.is_opened());
    }
}
