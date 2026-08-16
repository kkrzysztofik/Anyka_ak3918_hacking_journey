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
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

use parking_lot::RwLock;
use tokio::sync::{mpsc, oneshot};

use crate::hal::anyka::sdk::PtzDirection;

#[cfg(not(use_stubs))]
use crate::hal::anyka::ptz::ptz_turn_direction;
#[cfg(use_stubs)]
use crate::hal::common::ptz_turn_direction;

use crate::hal::common::ptz::{PtzHalTrait, TurnOutcome};

use crate::platform::traits::{PlatformError, PlatformResult, PtzPosition, PtzVelocity};

use super::ptz_control::{
    PTZ_MAX_PAN_DEGREES, PTZ_MAX_TILT_DEGREES, PTZ_MIN_MOVE_THRESHOLD, PTZ_MIN_PAN_DEGREES,
    PTZ_MIN_TILT_DEGREES, PTZ_STOP_DIRECTIONS, direction_to_ffi, iter_ffi_directions,
};

/// Degrees per second each axis travels at the driver's fixed speed setting.
///
/// ponytail: plain constants, because the driver sets speed exactly once
/// (`DEFAULT_SPEED`) and nothing varies it — a constant is as expressive as the
/// hardware currently is. If variable speed ever lands these become a function of
/// `motor_parm.speed_step`.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PtzRates {
    pub pan_deg_per_sec: f32,
    pub tilt_deg_per_sec: f32,
}

impl PtzRates {
    /// Convert from the configuration's `f64` degrees-per-second fields.
    pub(crate) fn from_config(cfg: &crate::config::types::PtzConfig) -> Self {
        Self {
            pan_deg_per_sec: cfg.pan_degrees_per_sec as f32,
            tilt_deg_per_sec: cfg.tilt_degrees_per_sec as f32,
        }
    }
}

/// The most recent motor step readback, per axis, with when it was taken.
#[derive(Debug, Clone, Copy)]
pub(crate) struct StepSample {
    pub pan: Option<i32>,
    pub tilt: Option<i32>,
    pub at: Instant,
}

/// Shared, lock-guarded PTZ state written by the actor and read by the platform layer.
///
/// Kept behind `Arc` so `get_position`/preset logic on the async side observe the truth
/// the actor commits after each turn.
pub(crate) struct PtzActorState {
    pub position: RwLock<PtzPosition>,
    pub velocity: RwLock<PtzVelocity>,
    /// Monotonic count of commands the actor has *fully* finished processing (including
    /// the asynchronous completion/reconciliation tail that runs after an acceptance
    /// reply). Lets observers know when an accepted-but-async move has actually landed.
    /// `u32` (not `u64`) so it is lock-free on the 32-bit ARM target.
    pub commands_completed: AtomicU32,
    /// Last `TurnOutcome.step_pos` observed per axis — never from a fresh ioctl.
    pub last_step: RwLock<Option<StepSample>>,
    /// Per-axis degrees-per-second used by dead-reckoning integration.
    pub rates: PtzRates,
}

impl PtzActorState {
    pub(crate) fn new(rates: PtzRates) -> Self {
        Self {
            position: RwLock::new(PtzPosition::HOME),
            velocity: RwLock::new(PtzVelocity::STOP),
            commands_completed: AtomicU32::new(0),
            last_step: RwLock::new(None),
            rates,
        }
    }

    /// Record a step readback observed at the end of a turn.
    ///
    /// Called only from the actor thread with a `TurnOutcome` already in hand — this
    /// never issues an ioctl of its own.
    pub(crate) fn record_step(&self, is_pan: bool, step: i32) {
        let mut guard = self.last_step.write();
        let mut sample = (*guard).unwrap_or(StepSample {
            pan: None,
            tilt: None,
            at: Instant::now(),
        });
        if is_pan {
            sample.pan = Some(step);
        } else {
            sample.tilt = Some(step);
        }
        sample.at = Instant::now();
        *guard = Some(sample);
    }
}

/// A PTZ command submitted to the actor. Each variant carries a `oneshot` reply channel.
pub(crate) enum PtzCommand {
    /// Absolute move. Replied once the command is *accepted* (all axis turns issued to
    /// hardware); ONVIF absolute moves are asynchronous, so the actor then keeps
    /// draining each axis to completion/interrupt and reconciles tracked position.
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

    // The winner has fully finished (including any post-acceptance wait/reconcile tail).
    state.commands_completed.fetch_add(1, Ordering::SeqCst);
}

fn execute(ffi: &dyn PtzHalTrait, state: &PtzActorState, cmd: PtzCommand) {
    match cmd {
        PtzCommand::MoveTo { position, reply } => {
            do_move(ffi, state, position, reply);
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

/// Absolute move.
///
/// ONVIF treats absolute moves as asynchronous: this issues the (non-blocking) turn on
/// each active axis, replies **once the command is accepted** (all axis turns issued to
/// hardware), and only then drains each axis to completion/interrupt and reconciles
/// tracked position. Acceptance fails (reply `Err`) only if an axis turn cannot be
/// issued at all.
///
/// Note: while draining `ptz_wait_turn` below the actor is *not* dequeuing new commands;
/// a superseding command (e.g. `Stop`) preempts the wait via the driver interrupt flag
/// (`ptz_interrupt`, set by the platform layer before each submit), which bounds
/// preemption latency to one ~100ms poll tick in the driver's wait loop.
fn do_move(
    ffi: &dyn PtzHalTrait,
    state: &PtzActorState,
    position: PtzPosition,
    reply: oneshot::Sender<PlatformResult<()>>,
) {
    let clamped_pan = position.pan.clamp(PTZ_MIN_PAN_DEGREES, PTZ_MAX_PAN_DEGREES);
    let clamped_tilt = position
        .tilt
        .clamp(PTZ_MIN_TILT_DEGREES, PTZ_MAX_TILT_DEGREES);

    let current = *state.position.read();
    let pan_delta = clamped_pan - current.pan;
    let tilt_delta = clamped_tilt - current.tilt;

    let plan = build_move_plan(pan_delta, tilt_delta, clamped_pan, clamped_tilt);

    // Issue each axis turn (non-blocking). Acceptance = all planned turns issued.
    let mut started = Vec::new();
    let mut result = Ok(());
    for (direction, degrees, axis) in plan {
        let sdk_dir = direction_to_ffi(direction);
        if let Err(e) = ffi.ptz_start_turn(sdk_dir, degrees.round() as i32) {
            result = Err(e);
            break;
        }
        started.push((sdk_dir, axis));
    }

    // Reply on acceptance (before waiting for the motor to finish).
    let _ = reply.send(result);

    drain_started_turns(ffi, state, started);

    // No hardware zoom on AK3918.
    state.position.write().zoom = position.zoom.clamp(1.0, 1.0);
    *state.velocity.write() = PtzVelocity::STOP;
}

/// Build the per-axis plan (direction + committed target once that axis completes).
///
/// Pan: positive delta → Right, negative → Left (continuous_move convention).
/// Tilt: positive delta → Down, negative → Up (matches C adapter).
fn build_move_plan(
    pan_delta: f32,
    tilt_delta: f32,
    clamped_pan: f32,
    clamped_tilt: f32,
) -> Vec<(PtzDirection, f32, Axis)> {
    let mut plan: Vec<(PtzDirection, f32, Axis)> = Vec::new();
    if pan_delta.abs() > PTZ_MIN_MOVE_THRESHOLD {
        let direction = if pan_delta > 0.0 {
            PtzDirection::Right
        } else {
            PtzDirection::Left
        };
        plan.push((direction, pan_delta.abs(), Axis::Pan(clamped_pan)));
    }
    if tilt_delta.abs() > PTZ_MIN_MOVE_THRESHOLD {
        let direction = if tilt_delta > 0.0 {
            PtzDirection::Down
        } else {
            PtzDirection::Up
        };
        plan.push((direction, tilt_delta.abs(), Axis::Tilt(clamped_tilt)));
    }
    plan
}

/// Accumulate dead-reckoned motion for one axis into the tracked position.
///
/// Sign convention matches `build_move_plan`: Right and Down are positive.
/// There is no hardware readback to correct against — see the design doc.
fn integrate_axis(state: &PtzActorState, direction: PtzDirection, elapsed: Duration) {
    let seconds = elapsed.as_secs_f32();
    let mut position = state.position.write();
    match direction {
        PtzDirection::Right | PtzDirection::Left => {
            let sign = if matches!(direction, PtzDirection::Right) { 1.0 } else { -1.0 };
            position.pan = (position.pan + sign * state.rates.pan_deg_per_sec * seconds)
                .clamp(PTZ_MIN_PAN_DEGREES, PTZ_MAX_PAN_DEGREES);
        }
        PtzDirection::Down | PtzDirection::Up => {
            let sign = if matches!(direction, PtzDirection::Down) { 1.0 } else { -1.0 };
            position.tilt = (position.tilt + sign * state.rates.tilt_deg_per_sec * seconds)
                .clamp(PTZ_MIN_TILT_DEGREES, PTZ_MAX_TILT_DEGREES);
        }
    }
}

/// Drain each started axis off the tokio workers; commit its target once it lands.
///
/// A `TurnOutcome`/`PtzWaitOutcome` is used (never discarded): on interruption we
/// stop reconciling further axes and leave the last known position untouched rather
/// than committing a target the motor never reached.
fn drain_started_turns(
    ffi: &dyn PtzHalTrait,
    state: &PtzActorState,
    started: Vec<(ptz_turn_direction, Axis)>,
) {
    for (sdk_dir, axis) in started {
        let outcome: TurnOutcome = match ffi.ptz_wait_turn(sdk_dir) {
            Ok(outcome) => outcome,
            Err(e) => {
                // The wait itself failed, so we do not know where the motor stopped.
                // Committing the commanded target here would record a position the
                // hardware may never have reached.
                tracing::warn!(
                    "PTZ wait failed on {:?}, leaving tracked position: {}",
                    axis,
                    e
                );
                break;
            }
        };
        let is_pan = matches!(axis, Axis::Pan(_));
        state.record_step(is_pan, outcome.step_pos);

        if outcome.interrupted {
            tracing::debug!(
                "absolute move preempted on {:?} (step_pos={}); leaving tracked position",
                axis,
                outcome.step_pos
            );
            break;
        }
        // Completed cleanly: commit the clamped target for this axis. (Hardware-step →
        // degree reconciliation via `outcome.step_pos` awaits calibration wiring.)
        match axis {
            Axis::Pan(target) => state.position.write().pan = target,
            Axis::Tilt(target) => state.position.write().tilt = target,
        }
    }
}

/// Which axis a planned/started turn belongs to, carrying the target degrees to commit
/// once the turn completes cleanly.
#[derive(Debug, Clone, Copy)]
enum Axis {
    Pan(f32),
    Tilt(f32),
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
    // (axis velocity, positive dir, negative dir, sweep magnitude, is_pan)
    let axes = [
        (
            velocity.pan,
            PtzDirection::Right,
            PtzDirection::Left,
            PTZ_MAX_PAN_DEGREES,
            true,
        ),
        (
            velocity.tilt,
            PtzDirection::Down,
            PtzDirection::Up,
            PTZ_MAX_TILT_DEGREES,
            false,
        ),
    ];

    let mut started = Vec::new();
    let mut result = Ok(());
    for (axis_velocity, dir_pos, dir_neg, sweep, is_pan) in axes {
        if axis_velocity.abs() <= f32::EPSILON {
            continue;
        }
        let direction = if axis_velocity > 0.0 {
            dir_pos
        } else {
            dir_neg
        };
        let sdk_dir = direction_to_ffi(direction);
        if let Err(e) = ffi.ptz_start_turn(sdk_dir, sweep.round() as i32) {
            result = Err(e);
            break;
        }
        started.push((sdk_dir, is_pan));
    }

    if result.is_ok() {
        *state.velocity.write() = velocity;
    }
    // Acknowledge acceptance (ONVIF continuous moves are asynchronous).
    let _ = reply.send(result);

    // Drain each started axis's completion/interrupt off the tokio workers.
    //
    // While blocked here the actor is *not* dequeuing new commands; a superseding
    // command (e.g. `Stop`) preempts via the driver interrupt flag (`ptz_interrupt`,
    // set by the platform layer before each submit), bounding preemption latency to one
    // ~100ms poll tick in the driver's wait loop. The `PtzWaitOutcome` is consumed (not
    // discarded) for the reconciled step position / interrupt signal.
    for (sdk_dir, is_pan) in started {
        match ffi.ptz_wait_turn(sdk_dir) {
            Ok(outcome) => {
                state.record_step(is_pan, outcome.step_pos);
                tracing::debug!(
                    "continuous axis wait finished: interrupted={}, step_pos={}",
                    outcome.interrupted,
                    outcome.step_pos
                );
            }
            Err(e) => tracing::warn!("continuous axis wait failed: {}", e),
        }
    }
}

/// Stop every axis. Attempts all four stop directions even if one fails, returning the
/// first error (mirrors the C adapter's belt-and-suspenders stop).
fn do_stop(ffi: &dyn PtzHalTrait, state: &PtzActorState) -> PlatformResult<()> {
    let mut first_error: Option<PlatformError> = None;
    for (dir, sdk_dir) in iter_ffi_directions(&PTZ_STOP_DIRECTIONS) {
        if let Err(e) = ffi.ptz_stop(sdk_dir) {
            tracing::warn!("PTZ stop failed on {:?}: {}", dir, e);
            first_error.get_or_insert(e);
        }
    }
    *state.velocity.write() = PtzVelocity::STOP;
    match first_error {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// Rates chosen so one second of motion equals exactly ten degrees, making
    /// every expectation below readable by inspection.
    fn test_state() -> PtzActorState {
        PtzActorState::new(PtzRates {
            pan_deg_per_sec: 10.0,
            tilt_deg_per_sec: 10.0,
        })
    }

    #[test]
    fn test_integrate_pan_right_accumulates_positive_degrees() {
        let state = test_state();
        integrate_axis(&state, PtzDirection::Right, Duration::from_secs(2));
        assert!((state.position.read().pan - 20.0).abs() < 0.01);
    }

    #[test]
    fn test_integrate_pan_left_accumulates_negative_degrees() {
        let state = test_state();
        integrate_axis(&state, PtzDirection::Left, Duration::from_secs(2));
        assert!((state.position.read().pan + 20.0).abs() < 0.01);
    }

    #[test]
    fn test_integrate_tilt_down_is_positive_up_is_negative() {
        let state = test_state();
        integrate_axis(&state, PtzDirection::Down, Duration::from_secs(1));
        assert!((state.position.read().tilt - 10.0).abs() < 0.01);
        integrate_axis(&state, PtzDirection::Up, Duration::from_secs(3));
        assert!((state.position.read().tilt + 20.0).abs() < 0.01);
    }

    #[test]
    fn test_integrate_accumulates_across_calls() {
        let state = test_state();
        integrate_axis(&state, PtzDirection::Right, Duration::from_secs(1));
        integrate_axis(&state, PtzDirection::Right, Duration::from_secs(1));
        assert!((state.position.read().pan - 20.0).abs() < 0.01);
    }

    #[test]
    fn test_integrate_clamps_to_pan_limit() {
        let state = test_state();
        // 100 seconds at 10 deg/s = 1000 degrees, far past the 350 limit.
        integrate_axis(&state, PtzDirection::Right, Duration::from_secs(100));
        assert!((state.position.read().pan - PTZ_MAX_PAN_DEGREES).abs() < f32::EPSILON);
    }

    #[test]
    fn test_integrate_clamps_to_negative_tilt_limit() {
        let state = test_state();
        integrate_axis(&state, PtzDirection::Up, Duration::from_secs(100));
        assert!((state.position.read().tilt - PTZ_MIN_TILT_DEGREES).abs() < f32::EPSILON);
    }

    #[test]
    fn test_actor_state_record_step_keeps_both_axes() {
        let state = PtzActorState::new(PtzRates {
            pan_deg_per_sec: 60.0,
            tilt_deg_per_sec: 60.0,
        });
        assert!(state.last_step.read().is_none(), "nothing observed yet");

        state.record_step(true, 0);
        state.record_step(false, 137);

        let sample = state.last_step.read().expect("a sample was recorded");
        assert_eq!(
            sample.pan,
            Some(0),
            "a zero step is a real reading, not absence"
        );
        assert_eq!(sample.tilt, Some(137));
    }
}
