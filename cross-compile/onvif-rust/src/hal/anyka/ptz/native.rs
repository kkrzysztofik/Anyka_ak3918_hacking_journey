//! Native Rust PTZ driver implementation for ARM targets.
//!
//! This module is only compiled on ARM (`#[cfg(not(use_stubs))]`).
//! It wraps `NativePtzDriver` to implement `PtzHalTrait`.

use std::sync::Arc;

use super::driver::{NativePtzDriver, ptz_device, ptz_feedback_pin, ptz_turn_direction};
use crate::hal::common::ptz::{PtzHalTrait, PtzWaitOutcome};
use crate::hal::common::{AK_FAILED_I32, AK_SUCCESS_I32};

/// Native Rust PTZ driver implementation for ARM. Talks to /dev/ak-motor0, /dev/ak-motor1.
pub(crate) struct NativePtzHal {
    driver: Arc<NativePtzDriver>,
}

impl NativePtzHal {
    pub(crate) fn new() -> Arc<Self> {
        Arc::new(Self {
            driver: Arc::new(NativePtzDriver::new()),
        })
    }
}

impl PtzHalTrait for NativePtzHal {
    fn ptz_open(&self) -> i32 {
        self.driver
            .open()
            .map(|()| AK_SUCCESS_I32)
            .unwrap_or(AK_FAILED_I32)
    }

    fn ptz_close(&self) -> i32 {
        self.driver
            .close()
            .map(|()| AK_SUCCESS_I32)
            .unwrap_or(AK_FAILED_I32)
    }

    fn ptz_check_self(&self, pin_type: ptz_feedback_pin) -> i32 {
        self.driver
            .check_self(pin_type)
            .map(|()| AK_SUCCESS_I32)
            .unwrap_or(AK_FAILED_I32)
    }

    fn ptz_turn(&self, direction: ptz_turn_direction, degree: i32) -> i32 {
        self.driver
            .turn(direction, degree)
            .map(|_| AK_SUCCESS_I32)
            .unwrap_or(AK_FAILED_I32)
    }

    fn ptz_start_turn(&self, direction: ptz_turn_direction, degree: i32) -> i32 {
        self.driver
            .start_turn(direction, degree)
            .map(|_| AK_SUCCESS_I32)
            .unwrap_or(AK_FAILED_I32)
    }

    fn ptz_wait_turn(&self, direction: ptz_turn_direction) -> PtzWaitOutcome {
        match self.driver.wait_turn(direction) {
            Ok(outcome) => PtzWaitOutcome {
                interrupted: outcome.interrupted,
                step_pos: outcome.step_pos,
            },
            Err(e) => {
                // Log here (the trait no longer carries an i32 error channel) and hand
                // back a best-effort outcome so the caller can still make progress.
                tracing::warn!("ptz_wait_turn failed: {}", e);
                PtzWaitOutcome {
                    interrupted: false,
                    step_pos: -1,
                }
            }
        }
    }

    fn ptz_get_step_pos(&self, motor_no: ptz_device) -> i32 {
        self.driver.get_step_pos(motor_no).unwrap_or(-1)
    }

    fn ptz_stop(&self, direction: ptz_turn_direction) -> i32 {
        self.driver
            .stop(direction)
            .map(|()| AK_SUCCESS_I32)
            .unwrap_or(AK_FAILED_I32)
    }

    fn ptz_interrupt(&self) {
        self.driver.interrupt();
    }
}
