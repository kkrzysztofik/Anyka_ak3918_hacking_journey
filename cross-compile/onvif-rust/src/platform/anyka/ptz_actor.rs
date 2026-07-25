//! PTZ command actor.
//!
//! A single owner thread ([`run_actor`]) holds exclusive access to the motor HAL and
//! serialises every PTZ command. Async callers (see [`super::ptz_control`]) submit
//! [`PtzCommand`]s over a **bounded** `tokio::sync::mpsc` channel and await the reply on
//! a `tokio::sync::oneshot`. This keeps the blocking `select()`/`read()` motor waits off
//! the tokio worker threads entirely.
//!
//! # Supersede semantics
//!
//! When the actor wakes it drains everything currently queued and collapses the batch:
//! a `Stop` anywhere in the batch wins; otherwise only the newest movement command is
//! executed. Superseded commands are acknowledged (their `oneshot` gets `Ok(())`) so no
//! caller is left awaiting a reply that never comes.
//!
//! Preempting an already in-flight turn is handled by the driver's interrupt flag
//! (`PtzHalTrait::ptz_interrupt`), set by the platform layer before each new command so
//! the actor's blocking wait unwinds quickly.

use std::sync::Arc;

use parking_lot::RwLock;
use tokio::sync::{mpsc, oneshot};

use crate::hal::anyka::sdk::PtzDirection;
use crate::hal::common::AK_SUCCESS_I32;
use crate::hal::common::ptz::PtzHalTrait;

use crate::platform::traits::{PlatformError, PlatformResult, PtzPosition, PtzVelocity};

use super::ptz_control::{
    PTZ_MAX_PAN_DEGREES, PTZ_MAX_TILT_DEGREES, PTZ_MIN_MOVE_THRESHOLD, PTZ_MIN_PAN_DEGREES,
    PTZ_MIN_TILT_DEGREES, PTZ_STOP_DIRECTIONS, direction_to_ffi, iter_ffi_directions,
};

/// Shared, lock-guarded PTZ state written by the actor and read by the platform layer.
///
/// Kept behind `Arc` so `get_position`/preset logic on the async side observe the truth
/// the actor commits after each turn.
pub(crate) struct PtzActorState {
    pub position: RwLock<PtzPosition>,
    pub velocity: RwLock<PtzVelocity>,
}

impl PtzActorState {
    pub(crate) fn new() -> Self {
        Self {
            position: RwLock::new(PtzPosition::HOME),
            velocity: RwLock::new(PtzVelocity::STOP),
        }
    }
}

/// A PTZ command submitted to the actor. Each variant carries a `oneshot` reply channel.
pub(crate) enum PtzCommand {
    /// Absolute move. Replied once the move completes (or fails).
    MoveTo {
        position: PtzPosition,
        reply: oneshot::Sender<PlatformResult<()>>,
    },
    /// Continuous move. Replied once the command is *accepted* (motor started); the
    /// actor then keeps the axis wait draining until interrupted or timed out.
    Continuous {
        velocity: PtzVelocity,
        reply: oneshot::Sender<PlatformResult<()>>,
    },
    /// Stop all motion.
    Stop {
        reply: oneshot::Sender<PlatformResult<()>>,
    },
}

impl PtzCommand {
    fn is_stop(&self) -> bool {
        matches!(self, PtzCommand::Stop { .. })
    }

    /// Consume the command and return its reply channel (used to acknowledge a
    /// superseded command).
    fn into_reply(self) -> oneshot::Sender<PlatformResult<()>> {
        match self {
            PtzCommand::MoveTo { reply, .. }
            | PtzCommand::Continuous { reply, .. }
            | PtzCommand::Stop { reply } => reply,
        }
    }
}

/// Actor entry point. Runs on a dedicated OS thread until the command channel closes.
pub(crate) fn run_actor(
    ffi: Arc<dyn PtzHalTrait>,
    state: Arc<PtzActorState>,
    mut rx: mpsc::Receiver<PtzCommand>,
) {
    while let Some(first) = rx.blocking_recv() {
        let mut batch = vec![first];
        while let Ok(next) = rx.try_recv() {
            batch.push(next);
        }
        dispatch_batch(ffi.as_ref(), &state, batch);
    }
    tracing::debug!("PTZ actor thread exiting");
}

/// Collapse a drained batch to a single winner and acknowledge the rest.
fn dispatch_batch(ffi: &dyn PtzHalTrait, state: &PtzActorState, batch: Vec<PtzCommand>) {
    // Stop always wins; otherwise the newest (last) command wins.
    let winner_idx = batch
        .iter()
        .rposition(PtzCommand::is_stop)
        .unwrap_or(batch.len() - 1);

    for (idx, cmd) in batch.into_iter().enumerate() {
        if idx == winner_idx {
            execute(ffi, state, cmd);
        } else {
            // Superseded: acknowledge acceptance so the caller's await returns.
            let _ = cmd.into_reply().send(Ok(()));
        }
    }
}

fn execute(ffi: &dyn PtzHalTrait, state: &PtzActorState, cmd: PtzCommand) {
    match cmd {
        PtzCommand::MoveTo { position, reply } => {
            let result = do_move(ffi, state, position);
            let _ = reply.send(result);
        }
        PtzCommand::Continuous { velocity, reply } => {
            do_continuous(ffi, state, velocity, reply);
        }
        PtzCommand::Stop { reply } => {
            let result = do_stop(ffi, state);
            let _ = reply.send(result);
        }
    }
}

/// Blocking absolute move: turn each axis to its target, committing tracked position
/// after each axis completes (never on submission).
fn do_move(ffi: &dyn PtzHalTrait, state: &PtzActorState, position: PtzPosition) -> PlatformResult<()> {
    let clamped_pan = position.pan.clamp(PTZ_MIN_PAN_DEGREES, PTZ_MAX_PAN_DEGREES);
    let clamped_tilt = position.tilt.clamp(PTZ_MIN_TILT_DEGREES, PTZ_MAX_TILT_DEGREES);

    let current = *state.position.read();
    let pan_delta = clamped_pan - current.pan;
    let tilt_delta = clamped_tilt - current.tilt;

    // Pan: positive delta → Right, negative → Left (continuous_move convention).
    if pan_delta.abs() > PTZ_MIN_MOVE_THRESHOLD {
        let direction = if pan_delta > 0.0 {
            PtzDirection::Right
        } else {
            PtzDirection::Left
        };
        turn_blocking(ffi, direction, pan_delta.abs())?;
        state.position.write().pan = clamped_pan;
    }

    // Tilt: positive delta → Down, negative → Up (matches C adapter).
    if tilt_delta.abs() > PTZ_MIN_MOVE_THRESHOLD {
        let direction = if tilt_delta > 0.0 {
            PtzDirection::Down
        } else {
            PtzDirection::Up
        };
        turn_blocking(ffi, direction, tilt_delta.abs())?;
        state.position.write().tilt = clamped_tilt;
    }

    // No hardware zoom on AK3918.
    state.position.write().zoom = position.zoom.clamp(1.0, 1.0);
    *state.velocity.write() = PtzVelocity::STOP;
    Ok(())
}

fn turn_blocking(ffi: &dyn PtzHalTrait, direction: PtzDirection, degrees: f32) -> PlatformResult<()> {
    let sdk_dir = direction_to_ffi(direction);
    let degree_int = degrees.round() as i32;
    let ret = ffi.ptz_turn(sdk_dir, degree_int);
    if ret == AK_SUCCESS_I32 {
        Ok(())
    } else {
        Err(PlatformError::HardwareFailure(format!(
            "ptz_turn({:?}, {}) failed: error code {}",
            direction, degree_int, ret
        )))
    }
}

/// Continuous move: issue the (non-blocking) start on each active axis, acknowledge
/// acceptance, then keep draining the axis waits so kernel notifications are consumed
/// and the actor unwinds promptly when interrupted.
fn do_continuous(
    ffi: &dyn PtzHalTrait,
    state: &PtzActorState,
    velocity: PtzVelocity,
    reply: oneshot::Sender<PlatformResult<()>>,
) {
    // (axis velocity, positive dir, negative dir, sweep magnitude)
    let axes = [
        (
            velocity.pan,
            PtzDirection::Right,
            PtzDirection::Left,
            PTZ_MAX_PAN_DEGREES,
        ),
        (
            velocity.tilt,
            PtzDirection::Down,
            PtzDirection::Up,
            PTZ_MAX_TILT_DEGREES,
        ),
    ];

    let mut started = Vec::new();
    let mut result = Ok(());
    for (axis_velocity, dir_pos, dir_neg, sweep) in axes {
        if axis_velocity.abs() <= f32::EPSILON {
            continue;
        }
        let direction = if axis_velocity > 0.0 { dir_pos } else { dir_neg };
        let sdk_dir = direction_to_ffi(direction);
        let ret = ffi.ptz_start_turn(sdk_dir, sweep.round() as i32);
        if ret != AK_SUCCESS_I32 {
            result = Err(PlatformError::HardwareFailure(format!(
                "ptz_start_turn({:?}) failed: error code {}",
                direction, ret
            )));
            break;
        }
        started.push(sdk_dir);
    }

    if result.is_ok() {
        *state.velocity.write() = velocity;
    }
    // Acknowledge acceptance (ONVIF continuous moves are asynchronous).
    let _ = reply.send(result);

    // Drain each started axis's completion/interrupt off the tokio workers.
    for sdk_dir in started {
        let ret = ffi.ptz_wait_turn(sdk_dir);
        if ret != AK_SUCCESS_I32 {
            tracing::warn!("ptz_wait_turn failed after continuous start: error code {}", ret);
        }
    }
}

/// Stop every axis. Attempts all four stop directions even if one fails, returning the
/// first error (mirrors the C adapter's belt-and-suspenders stop).
fn do_stop(ffi: &dyn PtzHalTrait, state: &PtzActorState) -> PlatformResult<()> {
    let mut first_error: Option<PlatformError> = None;
    for (dir, sdk_dir) in iter_ffi_directions(&PTZ_STOP_DIRECTIONS) {
        let ret = ffi.ptz_stop(sdk_dir);
        if ret != AK_SUCCESS_I32 && first_error.is_none() {
            first_error = Some(PlatformError::HardwareFailure(format!(
                "ptz_turn_stop({:?}) failed: error code {}",
                dir, ret
            )));
        }
    }
    *state.velocity.write() = PtzVelocity::STOP;
    match first_error {
        Some(e) => Err(e),
        None => Ok(()),
    }
}
