//! Stub PTZ HAL implementation for host-side testing.

/// Stub implementation for host tests (use_stubs). No hardware.
#[allow(dead_code)] // Used on host targets only
pub(crate) struct StubPtzHal;

#[cfg(use_stubs)]
use crate::hal::common::ptz::{PtzHalTrait, TurnOutcome};
#[cfg(use_stubs)]
use crate::hal::common::{ptz_device, ptz_feedback_pin, ptz_turn_direction};
#[cfg(use_stubs)]
use crate::platform::PlatformResult;

#[cfg(use_stubs)]
impl PtzHalTrait for StubPtzHal {
    fn ptz_open(&self) -> PlatformResult<()> {
        Ok(())
    }

    fn ptz_close(&self) -> PlatformResult<()> {
        Ok(())
    }

    fn ptz_check_self(&self, _pin_type: ptz_feedback_pin) -> PlatformResult<()> {
        Ok(())
    }

    fn ptz_start_turn(&self, _direction: ptz_turn_direction, _degree: i32) -> PlatformResult<bool> {
        Ok(true)
    }

    fn ptz_wait_turn(&self, _direction: ptz_turn_direction) -> PlatformResult<TurnOutcome> {
        Ok(TurnOutcome::default())
    }

    fn ptz_get_step_pos(&self, _motor_no: ptz_device) -> PlatformResult<i32> {
        Ok(0) // Return 0 steps for stub
    }

    fn ptz_stop(&self, _direction: ptz_turn_direction) -> PlatformResult<()> {
        Ok(())
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
