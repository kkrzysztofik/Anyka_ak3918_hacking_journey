//! Hardware PTZ control implementation.
//!
//! This module provides the real PTZ implementation that calls FFI functions
//! to control the physical stepper motors on the Anyka AK3918 camera.
//! It is always compiled (not gated behind `cfg(not(use_stubs))`) so that
//! unit tests can exercise it with `MockPtzHalTrait`.
//!
//! # Architecture
//!
//! ```text
//! HardwarePTZControl
//!   ├── ffi: Arc<dyn PtzHalTrait>  (injected, mockable)
//!   ├── handle: PTZHandle          (RAII, calls ptz_close on Drop)
//!   ├── position tracking          (in degrees, matching C adapter)
//!   └── continuous move task       (tokio::spawn with Notify cancellation)
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use parking_lot::{Mutex, RwLock};
use tokio::sync::Notify;
use tokio::task::JoinHandle;

use crate::hal::ptz::{PTZHandle, PtzHalTrait, default_ptz_hal, ptz_open_internal};
use crate::hal::{AK_SUCCESS_I32, PtzDirection};

use crate::hal::ptz_driver::ptz_turn_direction;

use super::traits::{
    PTZControl, PlatformError, PlatformResult, PtzLimits, PtzPosition, PtzPreset, PtzVelocity,
};

// Hardware constants matching the C adapter (ptz_adapter.c:31-34).
#[allow(dead_code)]
const PTZ_MAX_PAN_DEGREES: f32 = 350.0;
#[allow(dead_code)]
const PTZ_MIN_PAN_DEGREES: f32 = -350.0;
#[allow(dead_code)]
const PTZ_MAX_TILT_DEGREES: f32 = 130.0;
#[allow(dead_code)]
const PTZ_MIN_TILT_DEGREES: f32 = -130.0;
#[allow(dead_code)]
const PTZ_CONTINUOUS_TIMEOUT_SECS: u64 = 10;
#[allow(dead_code)]
const PTZ_MIN_MOVE_THRESHOLD: f32 = 0.5;
#[allow(dead_code)]
const PTZ_STOP_DIRECTIONS: [PtzDirection; 4] = [
    PtzDirection::Left,
    PtzDirection::Right,
    PtzDirection::Up,
    PtzDirection::Down,
];

/// Convert a `PtzDirection` to the FFI `ptz_turn_direction`.
///
/// Uses an exhaustive match instead of transmute so the compiler catches any
/// future enum changes at compile time rather than producing silent UB.
#[allow(dead_code)]
fn direction_to_ffi(direction: PtzDirection) -> ptz_turn_direction {
    match direction {
        PtzDirection::Left => ptz_turn_direction::PTZ_TURN_LEFT,
        PtzDirection::Right => ptz_turn_direction::PTZ_TURN_RIGHT,
        PtzDirection::Up => ptz_turn_direction::PTZ_TURN_UP,
        PtzDirection::Down => ptz_turn_direction::PTZ_TURN_DOWN,
    }
}

fn iter_ffi_directions(
    directions: &[PtzDirection],
) -> impl Iterator<Item = (PtzDirection, ptz_turn_direction)> + '_ {
    directions
        .iter()
        .copied()
        .map(|dir| (dir, direction_to_ffi(dir)))
}

/// Hardware PTZ control that delegates to FFI functions for motor control.
///
/// This struct is parameterized by `Arc<dyn PtzHalTrait>` to enable mock
/// injection for unit testing. Position is tracked in degrees (matching
/// the C adapter pattern) rather than querying hardware on every call.
///
/// # Lock ordering (acquire in this order to prevent deadlocks)
///
/// 1. `handle`
/// 2. `position`
/// 3. `velocity`
/// 4. `presets` + `next_preset_id` (always acquired together)
/// 5. `continuous_move_task`
// Used by anyka.rs on ARM builds; appears unused on x86_64 where use_stubs excludes anyka.rs.
#[allow(dead_code)]
pub(crate) struct HardwarePTZControl {
    ffi: Arc<dyn PtzHalTrait>,
    handle: RwLock<Option<PTZHandle>>,
    position: RwLock<PtzPosition>,
    velocity: RwLock<PtzVelocity>,
    presets: RwLock<HashMap<String, PtzPreset>>,
    next_preset_id: RwLock<u32>,
    continuous_move_active: Arc<AtomicBool>,
    continuous_move_cancel: Arc<Notify>,
    continuous_move_task: Mutex<Option<JoinHandle<()>>>,
}

#[allow(dead_code)]
impl HardwarePTZControl {
    /// Create a new `HardwarePTZControl` with the default FFI backend.
    /// On ARM this uses the native Rust PTZ driver (/dev/ak-motor0, /dev/ak-motor1);
    /// on host (use_stubs) uses the stub for tests.
    pub(crate) fn new() -> Self {
        Self::with_ffi(default_ptz_hal())
    }

    /// Create a new `HardwarePTZControl` with a custom FFI backend.
    ///
    /// Used by `anyka.rs` in production and by tests with `MockPtzHalTrait`.
    pub(crate) fn with_ffi(ffi: Arc<dyn PtzHalTrait>) -> Self {
        Self {
            ffi,
            handle: RwLock::new(None),
            position: RwLock::new(PtzPosition::HOME),
            velocity: RwLock::new(PtzVelocity::STOP),
            presets: RwLock::new(HashMap::new()),
            next_preset_id: RwLock::new(1),
            continuous_move_active: Arc::new(AtomicBool::new(false)),
            continuous_move_cancel: Arc::new(Notify::new()),
            continuous_move_task: Mutex::new(None),
        }
    }

    /// Open the PTZ device. Idempotent — returns `Ok(())` if already open.
    pub(crate) fn open(&self) -> PlatformResult<()> {
        let mut handle = self.handle.write();
        if handle.is_some() {
            return Ok(());
        }
        let h = ptz_open_internal(Arc::clone(&self.ffi))?;
        *handle = Some(h);
        Ok(())
    }

    /// Close the PTZ device. Cancels any continuous move and drops the handle.
    pub(crate) async fn close(&self) {
        self.cancel_continuous_move().await;
        // Drop the handle — PTZHandle::Drop calls ptz_close via RAII
        let _ = self.handle.write().take();
    }

    /// Verify the device is open before performing operations.
    fn ensure_open(&self) -> PlatformResult<()> {
        if self.handle.read().is_none() {
            return Err(PlatformError::HardwareUnavailable(
                "PTZ device not opened".to_string(),
            ));
        }
        Ok(())
    }

    /// Cancel any active continuous move and await its completion.
    async fn cancel_continuous_move(&self) {
        if self.continuous_move_active.swap(false, Ordering::SeqCst) {
            self.continuous_move_cancel.notify_one();
        }
        let task = self.continuous_move_task.lock().take();
        if let Some(handle) = task
            && let Err(e) = handle.await
        {
            tracing::error!(
                "PTZ continuous move task failed: {}. This indicates a bug in the timeout task.",
                e
            );
        }
    }

    /// Clamp a pan value to hardware limits.
    fn clamp_pan(pan: f32) -> f32 {
        pan.clamp(PTZ_MIN_PAN_DEGREES, PTZ_MAX_PAN_DEGREES)
    }

    /// Clamp a tilt value to hardware limits.
    fn clamp_tilt(tilt: f32) -> f32 {
        tilt.clamp(PTZ_MIN_TILT_DEGREES, PTZ_MAX_TILT_DEGREES)
    }

    /// Issue a turn command via FFI.
    fn turn(&self, direction: PtzDirection, degrees: f32) -> PlatformResult<()> {
        let sdk_dir = direction_to_ffi(direction);
        let degree_int = degrees.round() as i32;
        let ret = self.ffi.ptz_turn(sdk_dir, degree_int);
        if ret == AK_SUCCESS_I32 {
            Ok(())
        } else {
            Err(PlatformError::HardwareFailure(format!(
                "ptz_turn({:?}, {}) failed: error code {}",
                direction, degree_int, ret
            )))
        }
    }

    /// Issue stop commands for all four directions via FFI.
    ///
    /// Matches the C adapter pattern (ptz_adapter.c:376-382) which calls
    /// `platform_ptz_turn_stop()` for Left, Right, Up, and Down to ensure
    /// all motors are stopped regardless of which axis was moving.
    ///
    /// Unlike a simple `?`-based loop, this attempts to stop **all** axes
    /// even if one fails, then returns the first error (if any).
    fn stop_hardware(&self) -> PlatformResult<()> {
        let mut first_error: Option<PlatformError> = None;
        for (dir, sdk_dir) in iter_ffi_directions(&PTZ_STOP_DIRECTIONS) {
            let ret = self.ffi.ptz_stop(sdk_dir);
            if ret != AK_SUCCESS_I32 && first_error.is_none() {
                first_error = Some(PlatformError::HardwareFailure(format!(
                    "ptz_turn_stop({:?}) failed: error code {}",
                    dir, ret
                )));
            }
        }
        match first_error {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }
}

#[async_trait]
impl PTZControl for HardwarePTZControl {
    async fn move_to_position(&self, position: PtzPosition) -> PlatformResult<()> {
        self.ensure_open()?;
        self.cancel_continuous_move().await;

        let clamped_pan = Self::clamp_pan(position.pan);
        let clamped_tilt = Self::clamp_tilt(position.tilt);

        let current = *self.position.read();
        let pan_delta = clamped_pan - current.pan;
        let tilt_delta = clamped_tilt - current.tilt;

        // Pan movement — direction convention:
        // positive delta → Right, negative → Left.
        // NOTE: The C adapter relative_move (ptz_adapter.c:258) uses the opposite
        // convention (positive → Left), but its continuous_move (ptz_adapter.c:331)
        // maps positive → Right. We follow the continuous_move convention and the
        // plan specification consistently for both move types.
        if pan_delta.abs() > PTZ_MIN_MOVE_THRESHOLD {
            let direction = if pan_delta > 0.0 {
                PtzDirection::Right
            } else {
                PtzDirection::Left
            };
            self.turn(direction, pan_delta.abs())?;
            // Update tracked position immediately after successful pan turn,
            // so partial failure (pan OK, tilt fail) still reflects the pan movement.
            self.position.write().pan = clamped_pan;
        }

        // Tilt movement — direction mapping matches C adapter (ptz_adapter.c:274):
        // positive delta → Down, negative → Up
        if tilt_delta.abs() > PTZ_MIN_MOVE_THRESHOLD {
            let direction = if tilt_delta > 0.0 {
                PtzDirection::Down
            } else {
                PtzDirection::Up
            };
            self.turn(direction, tilt_delta.abs())?;
            self.position.write().tilt = clamped_tilt;
        }

        // Always update zoom (no FFI call involved, no hardware zoom on AK3918)
        self.position.write().zoom = position.zoom.clamp(1.0, 1.0);

        *self.velocity.write() = PtzVelocity::STOP;

        Ok(())
    }

    async fn get_position(&self) -> PlatformResult<PtzPosition> {
        self.ensure_open()?;
        Ok(*self.position.read())
    }

    async fn continuous_move(&self, velocity: PtzVelocity) -> PlatformResult<()> {
        self.ensure_open()?;
        self.cancel_continuous_move().await;

        // Start movement — direction mapping matches C adapter (ptz_adapter.c:331-339):
        // positive velocity → Right/Down, negative → Left/Up
        if velocity.pan.abs() > f32::EPSILON {
            let direction = if velocity.pan > 0.0 {
                PtzDirection::Right
            } else {
                PtzDirection::Left
            };
            self.turn(direction, PTZ_MAX_PAN_DEGREES)?;
        }

        if velocity.tilt.abs() > f32::EPSILON {
            let direction = if velocity.tilt > 0.0 {
                PtzDirection::Down
            } else {
                PtzDirection::Up
            };
            self.turn(direction, PTZ_MAX_TILT_DEGREES)?;
        }

        *self.velocity.write() = velocity;
        self.continuous_move_active.store(true, Ordering::SeqCst);

        // Spawn timeout task — mirrors C adapter's pthread timeout thread
        let active = self.continuous_move_active.clone();
        let cancel = self.continuous_move_cancel.clone();
        let ffi = self.ffi.clone();

        let task = tokio::spawn(async move {
            tokio::select! {
                () = tokio::time::sleep(Duration::from_secs(PTZ_CONTINUOUS_TIMEOUT_SECS)) => {
                    if active.swap(false, Ordering::SeqCst) {
                        tracing::info!(
                            "PTZ continuous move timeout after {}s, stopping",
                            PTZ_CONTINUOUS_TIMEOUT_SECS
                        );
                        // Stop all four directions by running the stop loop in spawn_blocking
                        // to avoid blocking the Tokio executor with synchronous FFI calls.
                        // Matching C adapter pattern (ptz_adapter.c:376-382).
                        let ffi_clone = ffi.clone();
                        let stop_result = tokio::task::spawn_blocking(move || {
                            for (dir, sdk_dir) in iter_ffi_directions(&PTZ_STOP_DIRECTIONS) {
                                let ret = ffi_clone.ptz_stop(sdk_dir);
                                if ret != AK_SUCCESS_I32 {
                                    tracing::error!(
                                        "CRITICAL: PTZ stop({:?}) failed after timeout \
                                         (error code {}), motor may still be running!",
                                        dir, ret
                                    );
                                }
                            }
                        }).await;

                        if let Err(e) = stop_result {
                            tracing::error!(
                                "PTZ stop task failed after timeout (possible task panic): {}",
                                e
                            );
                        }
                    }
                }
                () = cancel.notified() => {
                    // Cancelled by another operation (stop, move_to_position, or new continuous_move)
                }
            }
        });

        *self.continuous_move_task.lock() = Some(task);

        Ok(())
    }

    async fn stop(&self) -> PlatformResult<()> {
        self.ensure_open()?;
        self.cancel_continuous_move().await;
        self.stop_hardware()?;
        *self.velocity.write() = PtzVelocity::STOP;
        Ok(())
    }

    async fn get_presets(&self) -> PlatformResult<Vec<PtzPreset>> {
        Ok(self.presets.read().values().cloned().collect())
    }

    async fn set_preset(&self, name: &str) -> PlatformResult<String> {
        let position = *self.position.read();
        let mut presets = self.presets.write();
        let mut next_id = self.next_preset_id.write();
        let token = format!("preset_{}", *next_id);
        *next_id += 1;

        presets.insert(
            token.clone(),
            PtzPreset {
                token: token.clone(),
                name: name.to_string(),
                position,
            },
        );

        Ok(token)
    }

    async fn goto_preset(&self, token: &str) -> PlatformResult<()> {
        let position = {
            let presets = self.presets.read();
            let preset = presets.get(token).ok_or_else(|| {
                PlatformError::InvalidParameter(format!("Unknown preset: {}", token))
            })?;
            preset.position
        };

        self.move_to_position(position).await
    }

    async fn remove_preset(&self, token: &str) -> PlatformResult<()> {
        let mut presets = self.presets.write();
        if presets.remove(token).is_some() {
            Ok(())
        } else {
            Err(PlatformError::InvalidParameter(format!(
                "Unknown preset: {}",
                token
            )))
        }
    }

    async fn get_limits(&self) -> PlatformResult<PtzLimits> {
        Ok(PtzLimits {
            min_pan: PTZ_MIN_PAN_DEGREES,
            max_pan: PTZ_MAX_PAN_DEGREES,
            min_tilt: PTZ_MIN_TILT_DEGREES,
            max_tilt: PTZ_MAX_TILT_DEGREES,
            min_zoom: 1.0,
            max_zoom: 1.0, // No hardware zoom on AK3918
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hal::ptz::MockPtzHalTrait;
    use crate::hal::{AK_FAILED_I32, AK_SUCCESS_I32};

    /// Helper: create a mock FFI that succeeds on open (including self-check).
    fn mock_with_open() -> MockPtzHalTrait {
        let mut mock = MockPtzHalTrait::new();
        mock.expect_ptz_open().returning(|| AK_SUCCESS_I32);
        mock.expect_ptz_check_self().returning(|_| AK_SUCCESS_I32);
        // Allow close to be called during Drop
        mock.expect_ptz_close().returning(|| AK_SUCCESS_I32);
        mock
    }

    /// Helper: create a HardwarePTZControl with mock and open it.
    fn create_opened(mock: MockPtzHalTrait) -> HardwarePTZControl {
        let ptz = HardwarePTZControl::with_ffi(Arc::new(mock));
        ptz.open().expect("open should succeed");
        ptz
    }

    // =========================================================================
    // Initialization tests
    // =========================================================================

    #[test]
    fn test_open_success() {
        let mock = mock_with_open();
        let ptz = HardwarePTZControl::with_ffi(Arc::new(mock));
        assert!(ptz.open().is_ok());
    }

    #[test]
    fn test_open_failure() {
        let mut mock = MockPtzHalTrait::new();
        mock.expect_ptz_open().returning(|| AK_FAILED_I32);
        let ptz = HardwarePTZControl::with_ffi(Arc::new(mock));
        let result = ptz.open();
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareFailure(msg)) => {
                assert!(msg.contains("ak_drv_ptz_open"));
            }
            _ => panic!("Expected HardwareFailure"),
        }
    }

    #[test]
    fn test_open_idempotent() {
        let mut mock = MockPtzHalTrait::new();
        // open called only once despite two open() calls
        mock.expect_ptz_open().times(1).returning(|| AK_SUCCESS_I32);
        mock.expect_ptz_check_self()
            .times(1)
            .returning(|_| AK_SUCCESS_I32);
        mock.expect_ptz_close().returning(|| AK_SUCCESS_I32);
        let ptz = HardwarePTZControl::with_ffi(Arc::new(mock));
        assert!(ptz.open().is_ok());
        assert!(ptz.open().is_ok());
    }

    #[tokio::test]
    async fn test_operation_without_open_fails() {
        let mock = MockPtzHalTrait::new();
        let ptz = HardwarePTZControl::with_ffi(Arc::new(mock));
        let result = ptz.move_to_position(PtzPosition::HOME).await;
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareUnavailable(msg)) => {
                assert!(msg.contains("not opened"));
            }
            _ => panic!("Expected HardwareUnavailable"),
        }
    }

    // =========================================================================
    // Absolute move tests
    // =========================================================================

    #[tokio::test]
    async fn test_move_positive_pan_turns_right() {
        let mut mock = mock_with_open();
        mock.expect_ptz_turn()
            .withf(|dir, deg| *dir == ptz_turn_direction::PTZ_TURN_RIGHT && *deg == 90)
            .times(1)
            .returning(|_, _| AK_SUCCESS_I32);
        mock.expect_ptz_stop().returning(|_| AK_SUCCESS_I32);

        let ptz = create_opened(mock);
        let result = ptz.move_to_position(PtzPosition::new(90.0, 0.0, 1.0)).await;
        assert!(result.is_ok());

        let pos = ptz.get_position().await.unwrap();
        assert!((pos.pan - 90.0).abs() < f32::EPSILON);
    }

    #[tokio::test]
    async fn test_move_negative_pan_turns_left() {
        let mut mock = mock_with_open();
        mock.expect_ptz_turn()
            .withf(|dir, deg| *dir == ptz_turn_direction::PTZ_TURN_LEFT && *deg == 45)
            .times(1)
            .returning(|_, _| AK_SUCCESS_I32);
        mock.expect_ptz_stop().returning(|_| AK_SUCCESS_I32);

        let ptz = create_opened(mock);
        let result = ptz
            .move_to_position(PtzPosition::new(-45.0, 0.0, 1.0))
            .await;
        assert!(result.is_ok());

        let pos = ptz.get_position().await.unwrap();
        assert!((pos.pan - (-45.0)).abs() < f32::EPSILON);
    }

    #[tokio::test]
    async fn test_move_positive_tilt_turns_down() {
        let mut mock = mock_with_open();
        mock.expect_ptz_turn()
            .withf(|dir, deg| *dir == ptz_turn_direction::PTZ_TURN_DOWN && *deg == 60)
            .times(1)
            .returning(|_, _| AK_SUCCESS_I32);
        mock.expect_ptz_stop().returning(|_| AK_SUCCESS_I32);

        let ptz = create_opened(mock);
        let result = ptz.move_to_position(PtzPosition::new(0.0, 60.0, 1.0)).await;
        assert!(result.is_ok());

        let pos = ptz.get_position().await.unwrap();
        assert!((pos.tilt - 60.0).abs() < f32::EPSILON);
    }

    #[tokio::test]
    async fn test_move_negative_tilt_turns_up() {
        let mut mock = mock_with_open();
        mock.expect_ptz_turn()
            .withf(|dir, deg| *dir == ptz_turn_direction::PTZ_TURN_UP && *deg == 30)
            .times(1)
            .returning(|_, _| AK_SUCCESS_I32);
        mock.expect_ptz_stop().returning(|_| AK_SUCCESS_I32);

        let ptz = create_opened(mock);
        let result = ptz
            .move_to_position(PtzPosition::new(0.0, -30.0, 1.0))
            .await;
        assert!(result.is_ok());

        let pos = ptz.get_position().await.unwrap();
        assert!((pos.tilt - (-30.0)).abs() < f32::EPSILON);
    }

    #[tokio::test]
    async fn test_move_both_pan_and_tilt() {
        let mut mock = mock_with_open();
        // Expect pan Right, then tilt Down
        let call_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let count_clone = call_count.clone();
        mock.expect_ptz_turn().times(2).returning(move |_, _| {
            count_clone.fetch_add(1, Ordering::SeqCst);
            AK_SUCCESS_I32
        });
        mock.expect_ptz_stop().returning(|_| AK_SUCCESS_I32);

        let ptz = create_opened(mock);
        let result = ptz
            .move_to_position(PtzPosition::new(100.0, 50.0, 1.0))
            .await;
        assert!(result.is_ok());
        assert_eq!(call_count.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_move_clamps_to_limits() {
        let mut mock = mock_with_open();
        // Pan should be clamped to 350, not 500
        mock.expect_ptz_turn()
            .withf(|dir, deg| *dir == ptz_turn_direction::PTZ_TURN_RIGHT && *deg == 350)
            .times(1)
            .returning(|_, _| AK_SUCCESS_I32);
        mock.expect_ptz_stop().returning(|_| AK_SUCCESS_I32);

        let ptz = create_opened(mock);
        let result = ptz
            .move_to_position(PtzPosition::new(500.0, 0.0, 1.0))
            .await;
        assert!(result.is_ok());

        let pos = ptz.get_position().await.unwrap();
        assert!((pos.pan - PTZ_MAX_PAN_DEGREES).abs() < f32::EPSILON);
    }

    #[tokio::test]
    async fn test_move_small_delta_ignored() {
        let mut mock = mock_with_open();
        // No ptz_turn expected for a 0.3 degree move (below threshold)
        mock.expect_ptz_turn().never();
        mock.expect_ptz_stop().returning(|_| AK_SUCCESS_I32);

        let ptz = create_opened(mock);
        let result = ptz.move_to_position(PtzPosition::new(0.3, 0.2, 1.0)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_move_ffi_error_propagation() {
        let mut mock = mock_with_open();
        mock.expect_ptz_turn().returning(|_, _| AK_FAILED_I32);
        mock.expect_ptz_stop().returning(|_| AK_SUCCESS_I32);

        let ptz = create_opened(mock);
        let result = ptz.move_to_position(PtzPosition::new(90.0, 0.0, 1.0)).await;
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareFailure(msg)) => {
                assert!(msg.contains("ptz_turn"));
            }
            _ => panic!("Expected HardwareFailure"),
        }
    }

    #[tokio::test]
    async fn test_move_position_tracking_updates() {
        let mut mock = mock_with_open();
        mock.expect_ptz_turn().returning(|_, _| AK_SUCCESS_I32);
        mock.expect_ptz_stop().returning(|_| AK_SUCCESS_I32);

        let ptz = create_opened(mock);

        // Move to (50, 30)
        ptz.move_to_position(PtzPosition::new(50.0, 30.0, 1.0))
            .await
            .unwrap();
        let pos = ptz.get_position().await.unwrap();
        assert!((pos.pan - 50.0).abs() < f32::EPSILON);
        assert!((pos.tilt - 30.0).abs() < f32::EPSILON);

        // Move to (80, 10) — delta is (30, -20)
        ptz.move_to_position(PtzPosition::new(80.0, 10.0, 1.0))
            .await
            .unwrap();
        let pos = ptz.get_position().await.unwrap();
        assert!((pos.pan - 80.0).abs() < f32::EPSILON);
        assert!((pos.tilt - 10.0).abs() < f32::EPSILON);
    }

    // =========================================================================
    // Continuous move tests
    // =========================================================================

    #[tokio::test]
    async fn test_continuous_move_positive_pan() {
        let mut mock = mock_with_open();
        mock.expect_ptz_turn()
            .withf(|dir, deg| *dir == ptz_turn_direction::PTZ_TURN_RIGHT && *deg == 350)
            .times(1)
            .returning(|_, _| AK_SUCCESS_I32);
        mock.expect_ptz_stop().returning(|_| AK_SUCCESS_I32);

        let ptz = create_opened(mock);
        let result = ptz.continuous_move(PtzVelocity::new(0.5, 0.0, 0.0)).await;
        assert!(result.is_ok());

        // Clean up — cancel the timeout task
        ptz.close().await;
    }

    #[tokio::test]
    async fn test_continuous_move_negative_tilt() {
        let mut mock = mock_with_open();
        mock.expect_ptz_turn()
            .withf(|dir, deg| *dir == ptz_turn_direction::PTZ_TURN_UP && *deg == 130)
            .times(1)
            .returning(|_, _| AK_SUCCESS_I32);
        mock.expect_ptz_stop().returning(|_| AK_SUCCESS_I32);

        let ptz = create_opened(mock);
        let result = ptz.continuous_move(PtzVelocity::new(0.0, -0.5, 0.0)).await;
        assert!(result.is_ok());

        ptz.close().await;
    }

    #[tokio::test]
    async fn test_continuous_move_cancelled_by_stop() {
        let mut mock = mock_with_open();
        mock.expect_ptz_turn().returning(|_, _| AK_SUCCESS_I32);
        mock.expect_ptz_stop().returning(|_| AK_SUCCESS_I32);

        let ptz = create_opened(mock);
        ptz.continuous_move(PtzVelocity::new(1.0, 0.0, 0.0))
            .await
            .unwrap();
        // Stop should cancel the continuous move and call ptz_stop
        let result = ptz.stop().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_continuous_move_cancelled_by_new_move() {
        let mut mock = mock_with_open();
        mock.expect_ptz_turn().returning(|_, _| AK_SUCCESS_I32);
        mock.expect_ptz_stop().returning(|_| AK_SUCCESS_I32);

        let ptz = create_opened(mock);

        // Start continuous move
        ptz.continuous_move(PtzVelocity::new(1.0, 0.0, 0.0))
            .await
            .unwrap();

        // New move_to_position should cancel the continuous move
        let result = ptz.move_to_position(PtzPosition::new(50.0, 0.0, 1.0)).await;
        assert!(result.is_ok());
    }

    // =========================================================================
    // Stop tests
    // =========================================================================

    #[tokio::test]
    async fn test_stop_calls_ffi() {
        let mut mock = mock_with_open();
        // stop_hardware calls ptz_stop for all 4 directions (Left, Right, Up, Down)
        mock.expect_ptz_stop()
            .times(4)
            .returning(|_| AK_SUCCESS_I32);

        let ptz = create_opened(mock);
        let result = ptz.stop().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_stop_clears_velocity() {
        let mut mock = mock_with_open();
        mock.expect_ptz_turn().returning(|_, _| AK_SUCCESS_I32);
        mock.expect_ptz_stop().returning(|_| AK_SUCCESS_I32);

        let ptz = create_opened(mock);
        ptz.continuous_move(PtzVelocity::new(1.0, 0.5, 0.0))
            .await
            .unwrap();
        ptz.stop().await.unwrap();

        let vel = *ptz.velocity.read();
        assert!((vel.pan).abs() < f32::EPSILON);
        assert!((vel.tilt).abs() < f32::EPSILON);
    }

    #[tokio::test]
    async fn test_stop_ffi_error() {
        let mut mock = mock_with_open();
        mock.expect_ptz_stop().returning(|_| AK_FAILED_I32);

        let ptz = create_opened(mock);
        let result = ptz.stop().await;
        assert!(result.is_err());
    }

    // =========================================================================
    // Preset tests
    // =========================================================================

    #[tokio::test]
    async fn test_set_and_get_preset() {
        let mock = mock_with_open();
        let ptz = create_opened(mock);

        let token = ptz.set_preset("Front Door").await.unwrap();
        assert!(token.starts_with("preset_"));

        let presets = ptz.get_presets().await.unwrap();
        assert_eq!(presets.len(), 1);
        assert_eq!(presets[0].name, "Front Door");
        assert_eq!(presets[0].token, token);
    }

    #[tokio::test]
    async fn test_goto_preset_triggers_move() {
        let mut mock = mock_with_open();
        // First move to set up position
        mock.expect_ptz_turn().returning(|_, _| AK_SUCCESS_I32);
        mock.expect_ptz_stop().returning(|_| AK_SUCCESS_I32);

        let ptz = create_opened(mock);

        // Move to (90, 45), then set a preset there
        ptz.move_to_position(PtzPosition::new(90.0, 45.0, 1.0))
            .await
            .unwrap();
        let token = ptz.set_preset("Corner").await.unwrap();

        // Move back to home
        ptz.move_to_position(PtzPosition::new(0.0, 0.0, 1.0))
            .await
            .unwrap();

        // Goto preset should move back to (90, 45)
        ptz.goto_preset(&token).await.unwrap();
        let pos = ptz.get_position().await.unwrap();
        assert!((pos.pan - 90.0).abs() < f32::EPSILON);
        assert!((pos.tilt - 45.0).abs() < f32::EPSILON);
    }

    #[tokio::test]
    async fn test_goto_nonexistent_preset() {
        let mock = mock_with_open();
        let ptz = create_opened(mock);

        let result = ptz.goto_preset("nonexistent").await;
        assert!(result.is_err());
        match result {
            Err(PlatformError::InvalidParameter(msg)) => {
                assert!(msg.contains("nonexistent"));
            }
            _ => panic!("Expected InvalidParameter"),
        }
    }

    #[tokio::test]
    async fn test_remove_preset() {
        let mock = mock_with_open();
        let ptz = create_opened(mock);

        let token = ptz.set_preset("Temp").await.unwrap();
        assert!(ptz.remove_preset(&token).await.is_ok());
        assert!(ptz.get_presets().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_remove_nonexistent_preset() {
        let mock = mock_with_open();
        let ptz = create_opened(mock);

        let result = ptz.remove_preset("nonexistent").await;
        assert!(result.is_err());
    }

    // =========================================================================
    // Limits tests
    // =========================================================================

    #[tokio::test]
    async fn test_get_limits_returns_hardware_constants() {
        let mock = mock_with_open();
        let ptz = create_opened(mock);

        let limits = ptz.get_limits().await.unwrap();
        assert!((limits.min_pan - PTZ_MIN_PAN_DEGREES).abs() < f32::EPSILON);
        assert!((limits.max_pan - PTZ_MAX_PAN_DEGREES).abs() < f32::EPSILON);
        assert!((limits.min_tilt - PTZ_MIN_TILT_DEGREES).abs() < f32::EPSILON);
        assert!((limits.max_tilt - PTZ_MAX_TILT_DEGREES).abs() < f32::EPSILON);
        assert!((limits.max_zoom - 1.0).abs() < f32::EPSILON);
    }

    // =========================================================================
    // Close / cleanup tests
    // =========================================================================

    #[tokio::test]
    async fn test_close_allows_reopen() {
        let mut mock = MockPtzHalTrait::new();
        mock.expect_ptz_open().times(2).returning(|| AK_SUCCESS_I32);
        mock.expect_ptz_check_self().returning(|_| AK_SUCCESS_I32);
        mock.expect_ptz_close().returning(|| AK_SUCCESS_I32);
        mock.expect_ptz_stop().returning(|_| AK_SUCCESS_I32);

        let ptz = HardwarePTZControl::with_ffi(Arc::new(mock));
        ptz.open().unwrap();
        ptz.close().await;

        // Should be able to open again
        assert!(ptz.open().is_ok());
    }

    // =========================================================================
    // Timeout, clamping, FFI error propagation, direction verification
    // =========================================================================

    #[tokio::test(start_paused = true)]
    async fn test_continuous_move_timeout_fires_stop() {
        let mut mock = mock_with_open();
        mock.expect_ptz_turn().returning(|_, _| AK_SUCCESS_I32);
        // ptz_stop should be called 4 times — by the timeout task (once per direction)
        mock.expect_ptz_stop()
            .times(4)
            .returning(|_| AK_SUCCESS_I32);

        let ptz = create_opened(mock);
        ptz.continuous_move(PtzVelocity::new(1.0, 0.0, 0.0))
            .await
            .unwrap();

        // Yield first so the spawned task gets polled and registers its sleep timer
        tokio::task::yield_now().await;
        // Now advance past the 10-second timeout — this triggers the registered timer
        tokio::time::advance(Duration::from_secs(PTZ_CONTINUOUS_TIMEOUT_SECS + 1)).await;
        // Yield again to let the spawned task process the timer and call ptz_stop
        tokio::task::yield_now().await;

        // Clean up — properly join the timeout task so mock assertions fire
        ptz.close().await;
        // Mock drop verifies ptz_stop was called exactly once
    }

    #[tokio::test]
    async fn test_position_unchanged_after_pan_ffi_failure() {
        let mut mock = mock_with_open();
        mock.expect_ptz_turn().returning(|_, _| AK_FAILED_I32);
        mock.expect_ptz_stop().returning(|_| AK_SUCCESS_I32);

        let ptz = create_opened(mock);
        let result = ptz
            .move_to_position(PtzPosition::new(90.0, 45.0, 1.0))
            .await;
        assert!(result.is_err());

        // Position should remain at HOME — pan turn failed before update
        let pos = ptz.get_position().await.unwrap();
        assert!((pos.pan - 0.0).abs() < f32::EPSILON);
        assert!((pos.tilt - 0.0).abs() < f32::EPSILON);
    }

    #[tokio::test]
    async fn test_position_partially_updated_on_tilt_failure() {
        let mut mock = mock_with_open();
        let call_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let count_clone = call_count.clone();
        // First call (pan) succeeds, second call (tilt) fails
        mock.expect_ptz_turn().returning(move |_, _| {
            let count = count_clone.fetch_add(1, Ordering::SeqCst);
            if count == 0 {
                AK_SUCCESS_I32
            } else {
                AK_FAILED_I32
            }
        });
        mock.expect_ptz_stop().returning(|_| AK_SUCCESS_I32);

        let ptz = create_opened(mock);
        let result = ptz
            .move_to_position(PtzPosition::new(90.0, 45.0, 1.0))
            .await;
        assert!(result.is_err());

        // Pan should be updated (turn succeeded), tilt should remain at 0
        let pos = ptz.get_position().await.unwrap();
        assert!((pos.pan - 90.0).abs() < f32::EPSILON);
        assert!((pos.tilt - 0.0).abs() < f32::EPSILON);
    }

    #[tokio::test]
    async fn test_continuous_move_ffi_error_propagation() {
        let mut mock = mock_with_open();
        mock.expect_ptz_turn().returning(|_, _| AK_FAILED_I32);
        mock.expect_ptz_stop().returning(|_| AK_SUCCESS_I32);

        let ptz = create_opened(mock);
        let result = ptz.continuous_move(PtzVelocity::new(1.0, 0.0, 0.0)).await;
        assert!(result.is_err());
        match result {
            Err(PlatformError::HardwareFailure(msg)) => {
                assert!(msg.contains("ptz_turn"));
            }
            _ => panic!("Expected HardwareFailure"),
        }
    }

    #[tokio::test]
    async fn test_move_clamps_negative_pan_to_limits() {
        let mut mock = mock_with_open();
        // Pan -500 should be clamped to -350, turning Left 350 degrees
        mock.expect_ptz_turn()
            .withf(|dir, deg| *dir == ptz_turn_direction::PTZ_TURN_LEFT && *deg == 350)
            .times(1)
            .returning(|_, _| AK_SUCCESS_I32);
        mock.expect_ptz_stop().returning(|_| AK_SUCCESS_I32);

        let ptz = create_opened(mock);
        let result = ptz
            .move_to_position(PtzPosition::new(-500.0, 0.0, 1.0))
            .await;
        assert!(result.is_ok());

        let pos = ptz.get_position().await.unwrap();
        assert!((pos.pan - PTZ_MIN_PAN_DEGREES).abs() < f32::EPSILON);
    }

    #[tokio::test]
    async fn test_move_clamps_negative_tilt_to_limits() {
        let mut mock = mock_with_open();
        // Tilt -200 should be clamped to -130, turning Up 130 degrees
        mock.expect_ptz_turn()
            .withf(|dir, deg| *dir == ptz_turn_direction::PTZ_TURN_UP && *deg == 130)
            .times(1)
            .returning(|_, _| AK_SUCCESS_I32);
        mock.expect_ptz_stop().returning(|_| AK_SUCCESS_I32);

        let ptz = create_opened(mock);
        let result = ptz
            .move_to_position(PtzPosition::new(0.0, -200.0, 1.0))
            .await;
        assert!(result.is_ok());

        let pos = ptz.get_position().await.unwrap();
        assert!((pos.tilt - PTZ_MIN_TILT_DEGREES).abs() < f32::EPSILON);
    }

    #[tokio::test]
    async fn test_move_both_axes_verifies_directions() {
        let mut mock = mock_with_open();
        let mut seq = mockall::Sequence::new();

        // First call: pan Right 100 degrees (positive pan delta)
        mock.expect_ptz_turn()
            .withf(|dir, deg| *dir == ptz_turn_direction::PTZ_TURN_RIGHT && *deg == 100)
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_, _| AK_SUCCESS_I32);

        // Second call: tilt Up 50 degrees (negative tilt delta → Up)
        mock.expect_ptz_turn()
            .withf(|dir, deg| *dir == ptz_turn_direction::PTZ_TURN_UP && *deg == 50)
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_, _| AK_SUCCESS_I32);

        mock.expect_ptz_stop().returning(|_| AK_SUCCESS_I32);

        let ptz = create_opened(mock);
        let result = ptz
            .move_to_position(PtzPosition::new(100.0, -50.0, 1.0))
            .await;
        assert!(result.is_ok());
    }
}
