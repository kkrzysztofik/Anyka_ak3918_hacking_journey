//! Anyka platform PTZ control implementation.
//!
//! This module provides the real PTZ implementation that drives the physical stepper
//! motors on the Anyka AK3918 camera via the native Rust HAL driver in
//! `hal/anyka/ptz/`.
//!
//! # Architecture
//!
//! The async trait methods are thin shims: they submit a [`PtzCommand`] to the PTZ
//! actor (a dedicated OS thread owning the motor HAL) over a bounded channel and await
//! the reply under a `tokio::time::timeout`. All blocking motor waits happen on the
//! actor thread, never on a tokio worker.
//!
//! ```text
//! AnykaPTZControl (async shim)
//!   ├── ffi: Arc<dyn PtzHalTrait>   (injected, mockable; also used for ptz_interrupt)
//!   ├── handle: PTZHandle           (RAII, calls ptz_close on Drop)
//!   ├── shared: Arc<PtzActorState>  (position/velocity truth, written by the actor)
//!   ├── cmd_tx → PTZ actor thread   (bounded mpsc + oneshot replies)
//!   └── continuous timeout task     (tokio, routes auto-stop through the actor)
//! ```

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use parking_lot::{Mutex, RwLock};
use tokio::sync::{Notify, mpsc, oneshot};
use tokio::task::JoinHandle;

use crate::hal::anyka::sdk::PtzDirection;
use crate::hal::common::ptz::{PTZHandle, PtzHalTrait, default_ptz_hal, ptz_open};

#[cfg(not(use_stubs))]
use crate::hal::anyka::ptz::ptz_turn_direction;
#[cfg(use_stubs)]
use crate::hal::common::ptz_turn_direction;

use crate::platform::traits::{
    PTZControl, PlatformError, PlatformResult, PtzDiagnostics, PtzLimits, PtzPosition, PtzPreset,
    PtzVelocity, StepPos,
};
// `PtzPreset` is still referenced in the trait signatures; preserved in the import
// list to keep the trait contract clear at the use site.

use super::ptz_actor::{self, PtzActorState, PtzCommand, PtzRates};

// Hardware constants matching the C adapter (ptz_adapter.c:31-34).
pub(crate) const PTZ_MAX_PAN_DEGREES: f32 = 350.0;
pub(crate) const PTZ_MIN_PAN_DEGREES: f32 = -350.0;
pub(crate) const PTZ_MAX_TILT_DEGREES: f32 = 130.0;
pub(crate) const PTZ_MIN_TILT_DEGREES: f32 = -130.0;
/// Auto-stop timeout for a continuous move (mirrors the C adapter timeout thread).
const PTZ_CONTINUOUS_TIMEOUT_SECS: u64 = 10;
/// Minimum absolute-move delta (degrees) worth issuing a turn for.
pub(crate) const PTZ_MIN_MOVE_THRESHOLD: f32 = 0.5;
/// Directions issued to guarantee every axis is stopped.
pub(crate) const PTZ_STOP_DIRECTIONS: [PtzDirection; 4] = [
    PtzDirection::Left,
    PtzDirection::Right,
    PtzDirection::Up,
    PtzDirection::Down,
];
/// Upper bound on how long a caller waits for the actor to reply before the request is
/// turned into a `PlatformError::Timeout` (→ SOAP fault) instead of parking a worker.
const PTZ_CMD_TIMEOUT: Duration = Duration::from_secs(15);
/// Bounded command queue depth. Small: supersede semantics collapse bursts anyway.
const PTZ_CMD_QUEUE_CAP: usize = 8;

/// Convert a `PtzDirection` to the FFI `ptz_turn_direction`.
///
/// Uses an exhaustive match instead of transmute so the compiler catches any
/// future enum changes at compile time rather than producing silent UB.
pub(crate) fn direction_to_ffi(direction: PtzDirection) -> ptz_turn_direction {
    match direction {
        PtzDirection::Left => ptz_turn_direction::PTZ_TURN_LEFT,
        PtzDirection::Right => ptz_turn_direction::PTZ_TURN_RIGHT,
        PtzDirection::Up => ptz_turn_direction::PTZ_TURN_UP,
        PtzDirection::Down => ptz_turn_direction::PTZ_TURN_DOWN,
    }
}

pub(crate) fn iter_ffi_directions(
    directions: &[PtzDirection],
) -> impl Iterator<Item = (PtzDirection, ptz_turn_direction)> + '_ {
    directions
        .iter()
        .copied()
        .map(|dir| (dir, direction_to_ffi(dir)))
}

/// Anyka PTZ control: async front-end for the PTZ actor thread.
///
/// Parameterized by `Arc<dyn PtzHalTrait>` for mock injection in unit tests.
// Used by anyka.rs on ARM builds; appears unused on x86_64 where use_stubs excludes anyka.rs.
#[allow(dead_code)]
pub(crate) struct AnykaPTZControl {
    ffi: Arc<dyn PtzHalTrait>,
    handle: RwLock<Option<PTZHandle>>,
    shared: Arc<PtzActorState>,
    cmd_tx: Mutex<Option<mpsc::Sender<PtzCommand>>>,
    actor_handle: Mutex<Option<std::thread::JoinHandle<()>>>,
    continuous_timeout: Duration,
    continuous_move_active: Arc<AtomicBool>,
    continuous_move_cancel: Arc<Notify>,
    continuous_move_task: Mutex<Option<JoinHandle<()>>>,
}

#[allow(dead_code)]
impl AnykaPTZControl {
    /// Create a new `AnykaPTZControl` with the default FFI backend.
    pub(crate) fn new(rates: PtzRates, position_path: Option<std::path::PathBuf>) -> Self {
        Self::with_ffi(default_ptz_hal(), rates, position_path)
    }

    /// Create a new `AnykaPTZControl` with a custom FFI backend.
    ///
    /// Spawns the PTZ actor thread immediately; it idles until commands arrive.
    pub(crate) fn with_ffi(
        ffi: Arc<dyn PtzHalTrait>,
        rates: PtzRates,
        position_path: Option<std::path::PathBuf>,
    ) -> Self {
        Self::build(
            ffi,
            Duration::from_secs(PTZ_CONTINUOUS_TIMEOUT_SECS),
            rates,
            position_path,
        )
    }

    /// Test-only constructor with a custom continuous-move auto-stop timeout so the
    /// timeout path can be exercised in real time without a 10s wait.
    #[cfg(test)]
    pub(crate) fn with_ffi_and_timeout(
        ffi: Arc<dyn PtzHalTrait>,
        continuous_timeout: Duration,
    ) -> Self {
        Self::build(
            ffi,
            continuous_timeout,
            PtzRates::from_config(&crate::config::types::PtzConfig::default()),
            None,
        )
    }

    /// Test-only constructor with custom timeout and rates — the rates knob is the
    /// one `with_ffi_and_timeout` does not expose.
    #[cfg(test)]
    pub(crate) fn with_ffi_and_timeout_and_rates(
        ffi: Arc<dyn PtzHalTrait>,
        continuous_timeout: Duration,
        rates: PtzRates,
    ) -> Self {
        let shared = Arc::new(PtzActorState::new(rates));
        let (tx, rx) = mpsc::channel::<PtzCommand>(PTZ_CMD_QUEUE_CAP);

        let actor_ffi = Arc::clone(&ffi);
        let actor_shared = Arc::clone(&shared);
        let (cmd_tx, actor_handle) = match std::thread::Builder::new()
            .name("ptz-actor".to_string())
            .spawn(move || ptz_actor::run_actor(actor_ffi, actor_shared, rx))
        {
            Ok(handle) => (Some(tx), Some(handle)),
            Err(e) => {
                tracing::error!("Failed to spawn PTZ actor thread: {}", e);
                (None, None)
            }
        };

        Self {
            ffi,
            handle: RwLock::new(None),
            shared,
            cmd_tx: Mutex::new(cmd_tx),
            actor_handle: Mutex::new(actor_handle),
            continuous_timeout,
            continuous_move_active: Arc::new(AtomicBool::new(false)),
            continuous_move_cancel: Arc::new(Notify::new()),
            continuous_move_task: Mutex::new(None),
        }
    }

    fn build(
        ffi: Arc<dyn PtzHalTrait>,
        continuous_timeout: Duration,
        rates: PtzRates,
        position_path: Option<std::path::PathBuf>,
    ) -> Self {
        let shared = Arc::new(PtzActorState {
            position_path,
            ..PtzActorState::new(rates)
        });
        let (tx, rx) = mpsc::channel::<PtzCommand>(PTZ_CMD_QUEUE_CAP);

        let actor_ffi = Arc::clone(&ffi);
        let actor_shared = Arc::clone(&shared);
        let (cmd_tx, actor_handle) = match std::thread::Builder::new()
            .name("ptz-actor".to_string())
            .spawn(move || ptz_actor::run_actor(actor_ffi, actor_shared, rx))
        {
            Ok(handle) => (Some(tx), Some(handle)),
            Err(e) => {
                tracing::error!("Failed to spawn PTZ actor thread: {}", e);
                (None, None)
            }
        };

        Self {
            ffi,
            handle: RwLock::new(None),
            shared,
            cmd_tx: Mutex::new(cmd_tx),
            actor_handle: Mutex::new(actor_handle),
            continuous_timeout,
            continuous_move_active: Arc::new(AtomicBool::new(false)),
            continuous_move_cancel: Arc::new(Notify::new()),
            continuous_move_task: Mutex::new(None),
        }
    }

    /// Point-in-time diagnostics for a PTZ that came up.
    ///
    /// Reads only `parking_lot` guards and atomics — no ioctl, no driver lock — because
    /// this runs on the `/api/diagnostics` poll path, which must not be blockable by an
    /// in-flight ten-second sweep.
    pub(crate) fn diagnostics(&self) -> PtzDiagnostics {
        let handle = self.handle.read();
        let opened = handle.is_some();
        let self_check = handle
            .as_ref()
            .map(|h| h.self_check_error().unwrap_or("ok").to_string());
        drop(handle);

        let position = *self.shared.position.read();
        let velocity = *self.shared.velocity.read();
        let last_step_pos = self.shared.last_step.read().as_ref().map(|s| StepPos {
            pan: s.pan,
            tilt: s.tilt,
            age_ms: s.at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
        });

        PtzDiagnostics {
            enabled: true,
            opened,
            init_error: None,
            self_check,
            position: Some([position.pan, position.tilt, position.zoom]),
            moving: velocity.pan != 0.0 || velocity.tilt != 0.0,
            last_step_pos,
            commands_completed: self.shared.commands_completed.load(Ordering::SeqCst),
        }
    }

    /// Open the PTZ device. Idempotent — returns `Ok(())` if already open.
    pub(crate) fn open(&self) -> PlatformResult<()> {
        let mut handle = self.handle.write();
        if handle.is_some() {
            return Ok(());
        }
        let h = ptz_open(Arc::clone(&self.ffi))?;
        *handle = Some(h);
        Ok(())
    }

    /// Close the PTZ device. Cancels the continuous-move timeout and drops the handle.
    /// The actor thread keeps running so the device can be reopened.
    pub(crate) async fn close(&self) {
        self.cancel_continuous_timeout().await;
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

    /// Cancel any active continuous-move timeout task and await its completion.
    async fn cancel_continuous_timeout(&self) {
        if self.continuous_move_active.swap(false, Ordering::SeqCst) {
            self.continuous_move_cancel.notify_one();
        }
        let task = self.continuous_move_task.lock().take();
        if let Some(handle) = task
            && let Err(e) = handle.await
            && !e.is_cancelled()
        {
            tracing::error!("PTZ continuous timeout task failed: {}", e);
        }
    }

    /// Submit a command to the actor and await its reply under a timeout.
    ///
    /// Before sending, the driver interrupt flag is set so any in-flight turn unwinds
    /// promptly and can be superseded by this command.
    async fn submit<F>(&self, build: F) -> PlatformResult<()>
    where
        F: FnOnce(oneshot::Sender<PlatformResult<()>>) -> PtzCommand,
    {
        self.ensure_open()?;
        let tx = self.cmd_tx.lock().clone().ok_or_else(|| {
            PlatformError::HardwareUnavailable("PTZ actor unavailable".to_string())
        })?;

        // Preempt any in-flight turn so this command supersedes it quickly.
        self.ffi.ptz_interrupt();

        let (reply_tx, reply_rx) = oneshot::channel();
        tx.send(build(reply_tx))
            .await
            .map_err(|_| PlatformError::HardwareUnavailable("PTZ actor stopped".to_string()))?;

        match tokio::time::timeout(PTZ_CMD_TIMEOUT, reply_rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(PlatformError::HardwareFailure(
                "PTZ actor dropped reply".to_string(),
            )),
            Err(_) => Err(PlatformError::Timeout),
        }
    }

    /// Spawn the continuous-move auto-stop timeout. On expiry it routes a `Stop`
    /// through the actor (no `spawn_blocking`); cancelled by any superseding command.
    fn spawn_continuous_timeout(&self) {
        self.continuous_move_active.store(true, Ordering::SeqCst);
        let active = self.continuous_move_active.clone();
        let cancel = self.continuous_move_cancel.clone();
        let ffi = Arc::clone(&self.ffi);
        let timeout = self.continuous_timeout;
        // WeakSender so this task never keeps the actor channel alive past Drop.
        let weak_tx = self.cmd_tx.lock().as_ref().map(mpsc::Sender::downgrade);

        let task = tokio::spawn(async move {
            tokio::select! {
                () = tokio::time::sleep(timeout) => {
                    if active.swap(false, Ordering::SeqCst) {
                        tracing::info!(
                            "PTZ continuous move timeout after {:?}, stopping via actor",
                            timeout
                        );
                        // Interrupt the in-flight sweep, then enqueue a Stop.
                        ffi.ptz_interrupt();
                        if let Some(tx) = weak_tx.and_then(|w| w.upgrade()) {
                            let (reply_tx, _reply_rx) = oneshot::channel();
                            if let Err(e) = tx.send(PtzCommand::Stop { reply: reply_tx }).await {
                                tracing::error!("PTZ timeout stop enqueue failed: {}", e);
                            }
                        }
                    }
                }
                () = cancel.notified() => {}
            }
        });

        *self.continuous_move_task.lock() = Some(task);
    }
}

impl Drop for AnykaPTZControl {
    fn drop(&mut self) {
        // Cancel the timeout task (it only holds a WeakSender, so it cannot keep the
        // channel open), then close the channel and join the actor thread.
        if let Some(task) = self.continuous_move_task.lock().take() {
            task.abort();
        }
        *self.cmd_tx.lock() = None;
        if let Some(handle) = self.actor_handle.lock().take()
            && handle.join().is_err()
        {
            tracing::error!("PTZ actor thread panicked on shutdown");
        }
    }
}

impl AnykaPTZControl {
    /// Read the saved position from disk and, if non-trivial, dead-reckon the
    /// motors from their just-calibrated origin to it.
    ///
    /// Called from the async init path *after* `check_self` has just driven the
    /// motors to true center, so the resulting move starts from the most accurate
    /// origin available. Errors are logged and swallowed — a state file must
    /// never fail boot.
    pub(crate) async fn restore_position_from_disk(
        &self,
        path: &std::path::Path,
    ) -> PlatformResult<()> {
        let Some(saved) = ptz_actor::load_position(path) else {
            return Ok(());
        };
        // Below the move threshold, the integrator would no-op anyway — skip the
        // round-trip and keep PTZ_CMD_TIMEOUT free for real callers.
        if saved.pan.abs() < PTZ_MIN_MOVE_THRESHOLD && saved.tilt.abs() < PTZ_MIN_MOVE_THRESHOLD {
            // Still seed the in-memory position so platform reads report truth
            // even if the user never moves the lens after boot.
            *self.shared.position.write() = saved;
            return Ok(());
        }
        if let Err(e) = self.move_to_position(saved).await {
            tracing::warn!("PTZ position restore failed, staying centered: {}", e);
        }
        Ok(())
    }
}

#[async_trait]
impl PTZControl for AnykaPTZControl {
    async fn move_to_position(&self, position: PtzPosition) -> PlatformResult<()> {
        self.cancel_continuous_timeout().await;
        self.submit(|reply| PtzCommand::MoveTo { position, reply })
            .await
    }

    async fn get_position(&self) -> PlatformResult<PtzPosition> {
        self.ensure_open()?;
        Ok(*self.shared.position.read())
    }

    async fn continuous_move(&self, velocity: PtzVelocity) -> PlatformResult<()> {
        self.cancel_continuous_timeout().await;
        self.submit(|reply| PtzCommand::Continuous { velocity, reply })
            .await?;
        self.spawn_continuous_timeout();
        Ok(())
    }

    async fn stop(&self) -> PlatformResult<()> {
        self.cancel_continuous_timeout().await;
        self.submit(|reply| PtzCommand::Stop { reply }).await
    }

    async fn home(&self) -> PlatformResult<()> {
        self.cancel_continuous_timeout().await;
        self.submit(|reply| PtzCommand::Home { reply }).await
    }

    async fn get_presets(&self) -> PlatformResult<Vec<PtzPreset>> {
        // Presets are stored in the ONVIF layer (`PresetStore`), not in the platform.
        // The trait method stays so the ONVIF path can ask for them, but the platform
        // declares it unsupported.
        Err(PlatformError::NotSupported(
            "presets are stored in the ONVIF layer".into(),
        ))
    }

    async fn set_preset(&self, _name: &str) -> PlatformResult<String> {
        Err(PlatformError::NotSupported(
            "presets are stored in the ONVIF layer".into(),
        ))
    }

    async fn goto_preset(&self, _token: &str) -> PlatformResult<()> {
        Err(PlatformError::NotSupported(
            "presets are stored in the ONVIF layer".into(),
        ))
    }

    async fn remove_preset(&self, _token: &str) -> PlatformResult<()> {
        Err(PlatformError::NotSupported(
            "presets are stored in the ONVIF layer".into(),
        ))
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
    use crate::hal::common::ptz::{MockPtzHalTrait, TurnOutcome};

    /// A driver-shaped failure: carries the device path and errno the way the real
    /// `NativePtzDriver` does, so tests can assert that detail survives the HAL boundary
    /// instead of being flattened to an SDK-style `-1`.
    fn motor_error() -> PlatformError {
        PlatformError::HardwareFailure("motor turn failed on /dev/ak-motor0: errno 5".to_string())
    }

    /// Wait (bounded) until the actor has fully finished more commands than `before`.
    ///
    /// Absolute/continuous moves reply on *acceptance*; the position/velocity commit
    /// happens on the actor thread afterwards. This lets a test observe that async tail
    /// deterministically (the counter increments once the actor finishes a command).
    async fn await_actor_completed(ptz: &AnykaPTZControl, before: u32) {
        for _ in 0..2000 {
            if ptz.shared.commands_completed.load(Ordering::SeqCst) > before {
                return;
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        panic!("PTZ actor did not complete the command in time");
    }

    /// Submit an absolute move and wait for its async completion tail to land, so a
    /// subsequent `get_position` observes the committed position deterministically.
    async fn move_and_settle(ptz: &AnykaPTZControl, position: PtzPosition) -> PlatformResult<()> {
        let before = ptz.shared.commands_completed.load(Ordering::SeqCst);
        let result = ptz.move_to_position(position).await;
        await_actor_completed(ptz, before).await;
        result
    }

    /// Helper: create a mock FFI that succeeds on open (including self-check).
    ///
    /// `ptz_interrupt` is set up permissively because every submitted command sets the
    /// interrupt flag before sending.
    fn mock_with_open() -> MockPtzHalTrait {
        let mut mock = MockPtzHalTrait::new();
        mock.expect_ptz_open().returning(|| Ok(()));
        mock.expect_ptz_check_self().returning(|_| Ok(()));
        // Allow close to be called during Drop
        mock.expect_ptz_close().returning(|| Ok(()));
        // Every submitted command interrupts any in-flight turn first.
        mock.expect_ptz_interrupt().returning(|| ());
        mock
    }

    /// Helper: create a AnykaPTZControl with mock and open it.
    fn create_opened(mock: MockPtzHalTrait) -> AnykaPTZControl {
        let ptz = AnykaPTZControl::with_ffi(
            Arc::new(mock),
            PtzRates::from_config(&crate::config::types::PtzConfig::default()),
            None,
        );
        ptz.open().expect("open should succeed");
        ptz
    }

    /// Helper: create a AnykaPTZControl with mock + custom rates and open it.
    ///
    /// Test-only — uses the same custom-timeout constructor path so the rates
    /// are the only knob that differs from `create_opened`.
    fn create_opened_with_rates(mock: MockPtzHalTrait, rates: PtzRates) -> AnykaPTZControl {
        let ptz = AnykaPTZControl::with_ffi_and_timeout_and_rates(
            Arc::new(mock),
            Duration::from_secs(PTZ_CONTINUOUS_TIMEOUT_SECS),
            rates,
        );
        ptz.open().expect("open should succeed");
        ptz
    }

    // =========================================================================
    // Initialization tests
    // =========================================================================

    #[tokio::test]
    async fn test_diagnostics_reports_open_state_and_tracked_position() {
        let ptz = AnykaPTZControl::with_ffi(
            Arc::new(mock_with_open()),
            PtzRates::from_config(&crate::config::types::PtzConfig::default()),
            None,
        );
        ptz.open().unwrap();

        let d = ptz.diagnostics();
        assert!(d.enabled && d.opened);
        assert!(d.init_error.is_none());
        assert_eq!(d.self_check.as_deref(), Some("ok"));
        assert_eq!(d.position, Some([0.0, 0.0, 1.0]), "HOME until a move lands");
        assert!(!d.moving);
        assert!(d.last_step_pos.is_none(), "no turn has completed yet");
    }

    #[tokio::test]
    async fn test_diagnostics_before_open_reports_not_opened() {
        let ptz = AnykaPTZControl::with_ffi(
            Arc::new(mock_with_open()),
            PtzRates::from_config(&crate::config::types::PtzConfig::default()),
            None,
        );
        let d = ptz.diagnostics();
        assert!(!d.opened);
        assert!(d.self_check.is_none(), "the sweep only runs on open");
    }

    #[test]
    fn test_open_success() {
        let mock = mock_with_open();
        let ptz = AnykaPTZControl::with_ffi(
            Arc::new(mock),
            PtzRates::from_config(&crate::config::types::PtzConfig::default()),
            None,
        );
        assert!(ptz.open().is_ok());
    }

    #[test]
    fn test_open_failure() {
        let mut mock = MockPtzHalTrait::new();
        mock.expect_ptz_open().returning(|| {
            Err(PlatformError::HardwareFailure(
                "open /dev/ak-motor0: errno 19".to_string(),
            ))
        });
        let ptz = AnykaPTZControl::with_ffi(
            Arc::new(mock),
            PtzRates::from_config(&crate::config::types::PtzConfig::default()),
            None,
        );
        let result = ptz.open();
        match result {
            Err(PlatformError::HardwareFailure(msg)) => {
                // The driver's detail must reach the caller, not a generic "failed".
                assert!(msg.contains("/dev/ak-motor0"), "lost device path: {}", msg);
                assert!(msg.contains("errno 19"), "lost errno: {}", msg);
            }
            _ => panic!("Expected HardwareFailure"),
        }
    }

    #[test]
    fn test_open_idempotent() {
        let mut mock = MockPtzHalTrait::new();
        // open called only once despite two open() calls
        mock.expect_ptz_open().times(1).returning(|| Ok(()));
        mock.expect_ptz_check_self().times(1).returning(|_| Ok(()));
        mock.expect_ptz_close().returning(|| Ok(()));
        mock.expect_ptz_interrupt().returning(|| ());
        let ptz = AnykaPTZControl::with_ffi(
            Arc::new(mock),
            PtzRates::from_config(&crate::config::types::PtzConfig::default()),
            None,
        );
        assert!(ptz.open().is_ok());
        assert!(ptz.open().is_ok());
    }

    #[tokio::test]
    async fn test_operation_without_open_fails() {
        let mock = MockPtzHalTrait::new();
        let ptz = AnykaPTZControl::with_ffi(
            Arc::new(mock),
            PtzRates::from_config(&crate::config::types::PtzConfig::default()),
            None,
        );
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
        mock.expect_ptz_start_turn()
            .withf(|dir, deg| *dir == ptz_turn_direction::PTZ_TURN_RIGHT && *deg == 90)
            .times(1)
            .returning(|_, _| Ok(true));
        mock.expect_ptz_wait_turn()
            .returning(|_| Ok(TurnOutcome::default()));
        mock.expect_ptz_stop().returning(|_| Ok(()));

        let ptz = create_opened(mock);
        let result = move_and_settle(&ptz, PtzPosition::new(90.0, 0.0, 1.0)).await;
        assert!(result.is_ok());

        let pos = ptz.get_position().await.unwrap();
        assert!((pos.pan - 90.0).abs() < f32::EPSILON);
    }

    #[tokio::test]
    async fn test_move_negative_pan_turns_left() {
        let mut mock = mock_with_open();
        mock.expect_ptz_start_turn()
            .withf(|dir, deg| *dir == ptz_turn_direction::PTZ_TURN_LEFT && *deg == 45)
            .times(1)
            .returning(|_, _| Ok(true));
        mock.expect_ptz_wait_turn()
            .returning(|_| Ok(TurnOutcome::default()));
        mock.expect_ptz_stop().returning(|_| Ok(()));

        let ptz = create_opened(mock);
        let result = move_and_settle(&ptz, PtzPosition::new(-45.0, 0.0, 1.0)).await;
        assert!(result.is_ok());

        let pos = ptz.get_position().await.unwrap();
        assert!((pos.pan - (-45.0)).abs() < f32::EPSILON);
    }

    #[tokio::test]
    async fn test_move_positive_tilt_turns_down() {
        let mut mock = mock_with_open();
        mock.expect_ptz_start_turn()
            .withf(|dir, deg| *dir == ptz_turn_direction::PTZ_TURN_DOWN && *deg == 60)
            .times(1)
            .returning(|_, _| Ok(true));
        mock.expect_ptz_wait_turn()
            .returning(|_| Ok(TurnOutcome::default()));
        mock.expect_ptz_stop().returning(|_| Ok(()));

        let ptz = create_opened(mock);
        let result = move_and_settle(&ptz, PtzPosition::new(0.0, 60.0, 1.0)).await;
        assert!(result.is_ok());

        let pos = ptz.get_position().await.unwrap();
        assert!((pos.tilt - 60.0).abs() < f32::EPSILON);
    }

    #[tokio::test]
    async fn test_move_negative_tilt_turns_up() {
        let mut mock = mock_with_open();
        mock.expect_ptz_start_turn()
            .withf(|dir, deg| *dir == ptz_turn_direction::PTZ_TURN_UP && *deg == 30)
            .times(1)
            .returning(|_, _| Ok(true));
        mock.expect_ptz_wait_turn()
            .returning(|_| Ok(TurnOutcome::default()));
        mock.expect_ptz_stop().returning(|_| Ok(()));

        let ptz = create_opened(mock);
        let result = move_and_settle(&ptz, PtzPosition::new(0.0, -30.0, 1.0)).await;
        assert!(result.is_ok());

        let pos = ptz.get_position().await.unwrap();
        assert!((pos.tilt - (-30.0)).abs() < f32::EPSILON);
    }

    #[tokio::test]
    async fn test_move_both_pan_and_tilt() {
        let mut mock = mock_with_open();
        let call_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let count_clone = call_count.clone();
        mock.expect_ptz_start_turn()
            .times(2)
            .returning(move |_, _| {
                count_clone.fetch_add(1, Ordering::SeqCst);
                Ok(true)
            });
        mock.expect_ptz_wait_turn()
            .returning(|_| Ok(TurnOutcome::default()));
        mock.expect_ptz_stop().returning(|_| Ok(()));

        let ptz = create_opened(mock);
        let result = move_and_settle(&ptz, PtzPosition::new(100.0, 50.0, 1.0)).await;
        assert!(result.is_ok());
        assert_eq!(call_count.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_move_clamps_to_limits() {
        let mut mock = mock_with_open();
        // Pan should be clamped to 350, not 500
        mock.expect_ptz_start_turn()
            .withf(|dir, deg| *dir == ptz_turn_direction::PTZ_TURN_RIGHT && *deg == 350)
            .times(1)
            .returning(|_, _| Ok(true));
        mock.expect_ptz_wait_turn()
            .returning(|_| Ok(TurnOutcome::default()));
        mock.expect_ptz_stop().returning(|_| Ok(()));

        let ptz = create_opened(mock);
        let result = move_and_settle(&ptz, PtzPosition::new(500.0, 0.0, 1.0)).await;
        assert!(result.is_ok());

        let pos = ptz.get_position().await.unwrap();
        assert!((pos.pan - PTZ_MAX_PAN_DEGREES).abs() < f32::EPSILON);
    }

    #[tokio::test]
    async fn test_move_small_delta_ignored() {
        let mut mock = mock_with_open();
        // No turn expected for a 0.3 degree move (below threshold)
        mock.expect_ptz_start_turn().never();
        mock.expect_ptz_wait_turn().never();
        mock.expect_ptz_stop().returning(|_| Ok(()));

        let ptz = create_opened(mock);
        let result = move_and_settle(&ptz, PtzPosition::new(0.3, 0.2, 1.0)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_move_ffi_error_propagation() {
        let mut mock = mock_with_open();
        // A failed start means the command cannot be accepted → error returned to caller.
        mock.expect_ptz_start_turn()
            .returning(|_, _| Err(motor_error()));
        mock.expect_ptz_wait_turn()
            .returning(|_| Ok(TurnOutcome::default()));
        mock.expect_ptz_stop().returning(|_| Ok(()));

        let ptz = create_opened(mock);
        let result = move_and_settle(&ptz, PtzPosition::new(90.0, 0.0, 1.0)).await;
        match result {
            Err(PlatformError::HardwareFailure(msg)) => {
                // The driver's message reaches the ONVIF caller verbatim.
                assert!(msg.contains("/dev/ak-motor0"), "lost device path: {}", msg);
                assert!(msg.contains("errno 5"), "lost errno: {}", msg);
            }
            _ => panic!("Expected HardwareFailure"),
        }
    }

    #[tokio::test]
    async fn test_move_position_tracking_updates() {
        let mut mock = mock_with_open();
        mock.expect_ptz_start_turn().returning(|_, _| Ok(true));
        mock.expect_ptz_wait_turn()
            .returning(|_| Ok(TurnOutcome::default()));
        mock.expect_ptz_stop().returning(|_| Ok(()));

        let ptz = create_opened(mock);

        // Move to (50, 30)
        move_and_settle(&ptz, PtzPosition::new(50.0, 30.0, 1.0))
            .await
            .unwrap();
        let pos = ptz.get_position().await.unwrap();
        assert!((pos.pan - 50.0).abs() < f32::EPSILON);
        assert!((pos.tilt - 30.0).abs() < f32::EPSILON);

        // Move to (80, 10) — delta is (30, -20)
        move_and_settle(&ptz, PtzPosition::new(80.0, 10.0, 1.0))
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
        mock.expect_ptz_start_turn()
            .withf(|dir, deg| *dir == ptz_turn_direction::PTZ_TURN_RIGHT && *deg == 350)
            .times(1)
            .returning(|_, _| Ok(true));
        mock.expect_ptz_wait_turn()
            .returning(|_| Ok(TurnOutcome::default()));
        mock.expect_ptz_stop().returning(|_| Ok(()));

        let ptz = create_opened(mock);
        let result = ptz.continuous_move(PtzVelocity::new(0.5, 0.0, 0.0)).await;
        assert!(result.is_ok());

        // Clean up — cancel the timeout task
        ptz.close().await;
    }

    #[tokio::test]
    async fn test_continuous_move_negative_tilt() {
        let mut mock = mock_with_open();
        mock.expect_ptz_start_turn()
            .withf(|dir, deg| *dir == ptz_turn_direction::PTZ_TURN_UP && *deg == 130)
            .times(1)
            .returning(|_, _| Ok(true));
        mock.expect_ptz_wait_turn()
            .returning(|_| Ok(TurnOutcome::default()));
        mock.expect_ptz_stop().returning(|_| Ok(()));

        let ptz = create_opened(mock);
        let result = ptz.continuous_move(PtzVelocity::new(0.0, -0.5, 0.0)).await;
        assert!(result.is_ok());

        ptz.close().await;
    }

    #[tokio::test]
    async fn test_continuous_move_cancelled_by_stop() {
        let mut mock = mock_with_open();
        mock.expect_ptz_start_turn().returning(|_, _| Ok(true));
        mock.expect_ptz_wait_turn()
            .returning(|_| Ok(TurnOutcome::default()));
        mock.expect_ptz_stop().returning(|_| Ok(()));

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
        mock.expect_ptz_start_turn().returning(|_, _| Ok(true));
        mock.expect_ptz_wait_turn()
            .returning(|_| Ok(TurnOutcome::default()));
        mock.expect_ptz_stop().returning(|_| Ok(()));

        let ptz = create_opened(mock);

        // Start continuous move
        ptz.continuous_move(PtzVelocity::new(1.0, 0.0, 0.0))
            .await
            .unwrap();

        // New move_to_position should cancel the continuous move
        let result = move_and_settle(&ptz, PtzPosition::new(50.0, 0.0, 1.0)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_continuous_move_accumulates_tracked_position() {
        let mut mock = mock_with_open();
        mock.expect_ptz_start_turn().returning(|_, _| Ok(true));
        // The motor "runs" for 200ms before the wait returns.
        mock.expect_ptz_wait_turn().returning(|_| {
            std::thread::sleep(Duration::from_millis(200));
            Ok(TurnOutcome::default())
        });
        mock.expect_ptz_stop().returning(|_| Ok(()));

        let ptz = create_opened_with_rates(
            mock,
            PtzRates {
                pan_deg_per_sec: 100.0,
                tilt_deg_per_sec: 100.0,
            },
        );
        let before = ptz.shared.commands_completed.load(Ordering::SeqCst);
        ptz.continuous_move(PtzVelocity::new(1.0, 0.0, 0.0))
            .await
            .unwrap();
        await_actor_completed(&ptz, before).await;

        // 200ms at 100 deg/s ≈ 20 degrees. Wide tolerance: this measures real
        // elapsed time on a shared CI machine, so assert the magnitude, not the value.
        let pan = ptz.get_position().await.unwrap().pan;
        assert!(
            pan > 10.0 && pan < 40.0,
            "expected ~20 degrees of pan, got {}",
            pan
        );
    }

    #[tokio::test]
    async fn test_continuous_move_left_decreases_tracked_position() {
        let mut mock = mock_with_open();
        mock.expect_ptz_start_turn().returning(|_, _| Ok(true));
        mock.expect_ptz_wait_turn().returning(|_| {
            std::thread::sleep(Duration::from_millis(200));
            Ok(TurnOutcome::default())
        });
        mock.expect_ptz_stop().returning(|_| Ok(()));

        let ptz = create_opened_with_rates(
            mock,
            PtzRates {
                pan_deg_per_sec: 100.0,
                tilt_deg_per_sec: 100.0,
            },
        );
        let before = ptz.shared.commands_completed.load(Ordering::SeqCst);
        ptz.continuous_move(PtzVelocity::new(-1.0, 0.0, 0.0))
            .await
            .unwrap();
        await_actor_completed(&ptz, before).await;

        let pan = ptz.get_position().await.unwrap().pan;
        assert!(
            pan < -10.0 && pan > -40.0,
            "expected ~-20 degrees of pan, got {}",
            pan
        );
    }

    #[tokio::test]
    async fn test_interrupted_absolute_move_commits_partial_motion() {
        let mut mock = mock_with_open();
        mock.expect_ptz_start_turn().returning(|_, _| Ok(true));
        mock.expect_ptz_wait_turn().returning(|_| {
            std::thread::sleep(Duration::from_millis(200));
            Ok(TurnOutcome {
                interrupted: true,
                ..TurnOutcome::default()
            })
        });
        mock.expect_ptz_stop().returning(|_| Ok(()));

        let ptz = create_opened_with_rates(
            mock,
            PtzRates {
                pan_deg_per_sec: 100.0,
                tilt_deg_per_sec: 100.0,
            },
        );
        move_and_settle(&ptz, PtzPosition::new(350.0, 0.0, 1.0))
            .await
            .unwrap();

        // Interrupted before reaching 350, but ~20 degrees of travel happened.
        let pan = ptz.get_position().await.unwrap().pan;
        assert!(
            pan > 10.0 && pan < 40.0,
            "partial motion lost: pan = {}",
            pan
        );
    }

    #[tokio::test]
    async fn test_home_runs_self_check_and_resets_position() {
        // Build the mock from scratch so the strict `times(2)` for `ptz_check_self`
        // is the only expectation on the method — mockall's multiple-expectation
        // dispatch never gets crossed with the permissive default from
        // `mock_with_open()`.
        let mut mock = MockPtzHalTrait::new();
        mock.expect_ptz_open().returning(|| Ok(()));
        mock.expect_ptz_close().returning(|| Ok(()));
        mock.expect_ptz_interrupt().returning(|| ());
        mock.expect_ptz_start_turn().returning(|_, _| Ok(true));
        mock.expect_ptz_wait_turn()
            .returning(|_| Ok(TurnOutcome::default()));
        mock.expect_ptz_stop().returning(|_| Ok(()));
        // check_self runs once at open() and once more for the re-home.
        mock.expect_ptz_check_self().times(2).returning(|_| Ok(()));

        let ptz = create_opened(mock);
        move_and_settle(&ptz, PtzPosition::new(90.0, 45.0, 1.0))
            .await
            .unwrap();

        let before = ptz.shared.commands_completed.load(Ordering::SeqCst);
        ptz.home().await.unwrap();
        await_actor_completed(&ptz, before).await;

        let pos = ptz.get_position().await.unwrap();
        assert!(pos.pan.abs() < f32::EPSILON);
        assert!(pos.tilt.abs() < f32::EPSILON);
    }

    #[tokio::test]
    async fn test_startup_restores_saved_position() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ptz_position.toml");
        ptz_actor::save_position(&path, PtzPosition::new(90.0, -45.0, 1.0));

        let mut mock = mock_with_open();
        mock.expect_ptz_start_turn().returning(|_, _| Ok(true));
        mock.expect_ptz_wait_turn()
            .returning(|_| Ok(TurnOutcome::default()));
        mock.expect_ptz_stop().returning(|_| Ok(()));

        let ptz = create_opened(mock);
        ptz.restore_position_from_disk(&path).await.unwrap();

        let pos = ptz.get_position().await.unwrap();
        // The move lands via the dead-reckoning integrator, so the initial pan
        // delta comes from `(90 - 0) deg` traveling at the default rate. Verify
        // the direction is correct (positive) rather than the exact value.
        assert!(
            pos.pan > 0.0,
            "expected pan to grow positive, got {}",
            pos.pan
        );
    }

    #[tokio::test]
    async fn test_startup_does_not_move_when_saved_position_is_home() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ptz_position.toml");
        ptz_actor::save_position(&path, PtzPosition::HOME);

        let mut mock = mock_with_open();
        // 0,0 falls below PTZ_MIN_MOVE_THRESHOLD — no motor turn should be issued.
        mock.expect_ptz_start_turn().never();
        mock.expect_ptz_stop().returning(|_| Ok(()));

        let ptz = create_opened(mock);
        ptz.restore_position_from_disk(&path).await.unwrap();
    }

    #[tokio::test]
    async fn test_configured_rate_scales_tracked_motion() {
        // Two controls, same 200ms mock wait, rates differing 4x.
        // The faster one must accumulate roughly 4x the degrees.
        // Asserting the ratio, not absolutes, is what keeps this from flaking
        // on a shared CI machine — elapsed wall-clock is the same in both runs.

        let mut slow_mock = mock_with_open();
        slow_mock.expect_ptz_start_turn().returning(|_, _| Ok(true));
        slow_mock.expect_ptz_wait_turn().returning(|_| {
            std::thread::sleep(Duration::from_millis(200));
            Ok(TurnOutcome::default())
        });
        slow_mock.expect_ptz_stop().returning(|_| Ok(()));
        let slow_ptz = create_opened_with_rates(
            slow_mock,
            PtzRates {
                pan_deg_per_sec: 50.0,
                tilt_deg_per_sec: 50.0,
            },
        );

        let mut fast_mock = mock_with_open();
        fast_mock.expect_ptz_start_turn().returning(|_, _| Ok(true));
        fast_mock.expect_ptz_wait_turn().returning(|_| {
            std::thread::sleep(Duration::from_millis(200));
            Ok(TurnOutcome::default())
        });
        fast_mock.expect_ptz_stop().returning(|_| Ok(()));
        let fast_ptz = create_opened_with_rates(
            fast_mock,
            PtzRates {
                pan_deg_per_sec: 200.0,
                tilt_deg_per_sec: 200.0,
            },
        );

        let before_slow = slow_ptz.shared.commands_completed.load(Ordering::SeqCst);
        slow_ptz
            .continuous_move(PtzVelocity::new(1.0, 0.0, 0.0))
            .await
            .unwrap();
        await_actor_completed(&slow_ptz, before_slow).await;

        let before_fast = fast_ptz.shared.commands_completed.load(Ordering::SeqCst);
        fast_ptz
            .continuous_move(PtzVelocity::new(1.0, 0.0, 0.0))
            .await
            .unwrap();
        await_actor_completed(&fast_ptz, before_fast).await;

        let slow = slow_ptz.get_position().await.unwrap().pan;
        let fast = fast_ptz.get_position().await.unwrap().pan;
        let ratio = fast / slow;
        assert!(
            ratio > 3.0 && ratio < 5.0,
            "expected ~4x scaling, got slow={} fast={} ratio={}",
            slow,
            fast,
            ratio
        );
    }

    // =========================================================================
    // Stop tests
    // =========================================================================

    #[tokio::test]
    async fn test_stop_calls_ffi() {
        let mut mock = mock_with_open();
        // do_stop calls ptz_stop for all 4 directions (Left, Right, Up, Down)
        mock.expect_ptz_stop().times(4).returning(|_| Ok(()));

        let ptz = create_opened(mock);
        let result = ptz.stop().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_stop_clears_velocity() {
        let mut mock = mock_with_open();
        mock.expect_ptz_start_turn().returning(|_, _| Ok(true));
        mock.expect_ptz_wait_turn()
            .returning(|_| Ok(TurnOutcome::default()));
        mock.expect_ptz_stop().returning(|_| Ok(()));

        let ptz = create_opened(mock);
        ptz.continuous_move(PtzVelocity::new(1.0, 0.5, 0.0))
            .await
            .unwrap();
        ptz.stop().await.unwrap();

        let vel = *ptz.shared.velocity.read();
        assert!((vel.pan).abs() < f32::EPSILON);
        assert!((vel.tilt).abs() < f32::EPSILON);
    }

    #[tokio::test]
    async fn test_stop_ffi_error() {
        let mut mock = mock_with_open();
        mock.expect_ptz_stop().returning(|_| Err(motor_error()));

        let ptz = create_opened(mock);
        let result = ptz.stop().await;
        assert!(result.is_err());
    }

    // =========================================================================
    // Preset tests
    // =========================================================================

    #[tokio::test]
    async fn test_platform_presets_are_not_supported() {
        let ptz = create_opened(mock_with_open());
        assert!(matches!(
            ptz.set_preset("x").await,
            Err(PlatformError::NotSupported(_))
        ));
    }

    // =========================================================================
    // Limits tests
    // =========================================================================

    /// The driver seeds the kernel's travel range via `MOTOR_PARM` from its own
    /// constants; if they drift from the limits reported over ONVIF, the motor is
    /// configured for a different range than the one clients are told about.
    #[test]
    fn test_driver_travel_matches_reported_limits() {
        use crate::hal::anyka::ptz::driver::{PAN_TRAVEL_DEGREES, TILT_TRAVEL_DEGREES};

        assert_eq!(
            PAN_TRAVEL_DEGREES,
            (PTZ_MAX_PAN_DEGREES - PTZ_MIN_PAN_DEGREES) as i32
        );
        assert_eq!(
            TILT_TRAVEL_DEGREES,
            (PTZ_MAX_TILT_DEGREES - PTZ_MIN_TILT_DEGREES) as i32
        );
    }

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
        mock.expect_ptz_open().times(2).returning(|| Ok(()));
        mock.expect_ptz_check_self().returning(|_| Ok(()));
        mock.expect_ptz_close().returning(|| Ok(()));
        mock.expect_ptz_stop().returning(|_| Ok(()));
        mock.expect_ptz_interrupt().returning(|| ());

        let ptz = AnykaPTZControl::with_ffi(
            Arc::new(mock),
            PtzRates::from_config(&crate::config::types::PtzConfig::default()),
            None,
        );
        ptz.open().unwrap();
        ptz.close().await;

        // Should be able to open again
        assert!(ptz.open().is_ok());
    }

    // =========================================================================
    // Timeout, position tracking, FFI error propagation
    // =========================================================================

    #[tokio::test]
    async fn test_continuous_move_timeout_fires_stop() {
        // Uses a real (short) timeout instead of paused time: the actor replies from a
        // separate OS thread, which paused-time auto-advance would race against.
        let mut mock = mock_with_open();
        mock.expect_ptz_start_turn().returning(|_, _| Ok(true));
        mock.expect_ptz_wait_turn()
            .returning(|_| Ok(TurnOutcome::default()));
        // ptz_stop should be called 4 times by the timeout-driven actor Stop.
        mock.expect_ptz_stop().times(4).returning(|_| Ok(()));

        let ptz = AnykaPTZControl::with_ffi_and_timeout(Arc::new(mock), Duration::from_millis(50));
        ptz.open().expect("open should succeed");
        ptz.continuous_move(PtzVelocity::new(1.0, 0.0, 0.0))
            .await
            .unwrap();

        // Wait past the 50ms auto-stop timeout so the timeout task enqueues a Stop and
        // the actor thread processes it.
        tokio::time::sleep(Duration::from_millis(300)).await;

        // Dropping joins the actor thread, ensuring the Stop was processed before the
        // mock verifies the expected ptz_stop call count.
        drop(ptz);
    }

    #[tokio::test]
    async fn test_position_unchanged_after_pan_ffi_failure() {
        let mut mock = mock_with_open();
        mock.expect_ptz_start_turn()
            .returning(|_, _| Err(motor_error()));
        mock.expect_ptz_wait_turn()
            .returning(|_| Ok(TurnOutcome::default()));
        mock.expect_ptz_stop().returning(|_| Ok(()));

        let ptz = create_opened(mock);
        let result = move_and_settle(&ptz, PtzPosition::new(90.0, 45.0, 1.0)).await;
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
        mock.expect_ptz_start_turn().returning(move |_, _| {
            let count = count_clone.fetch_add(1, Ordering::SeqCst);
            if count == 0 {
                Ok(true)
            } else {
                Err(motor_error())
            }
        });
        mock.expect_ptz_wait_turn()
            .returning(|_| Ok(TurnOutcome::default()));
        mock.expect_ptz_stop().returning(|_| Ok(()));

        let ptz = create_opened(mock);
        let result = move_and_settle(&ptz, PtzPosition::new(90.0, 45.0, 1.0)).await;
        assert!(result.is_err());

        // Pan should be updated (turn succeeded), tilt should remain at 0
        let pos = ptz.get_position().await.unwrap();
        assert!((pos.pan - 90.0).abs() < f32::EPSILON);
        assert!((pos.tilt - 0.0).abs() < f32::EPSILON);
    }

    #[tokio::test]
    async fn test_continuous_move_ffi_error_propagation() {
        let mut mock = mock_with_open();
        mock.expect_ptz_start_turn()
            .returning(|_, _| Err(motor_error()));
        mock.expect_ptz_wait_turn()
            .returning(|_| Ok(TurnOutcome::default()));
        mock.expect_ptz_stop().returning(|_| Ok(()));

        let ptz = create_opened(mock);
        let result = ptz.continuous_move(PtzVelocity::new(1.0, 0.0, 0.0)).await;
        match result {
            Err(PlatformError::HardwareFailure(msg)) => {
                assert!(msg.contains("/dev/ak-motor0"), "lost device path: {}", msg);
                assert!(msg.contains("errno 5"), "lost errno: {}", msg);
            }
            _ => panic!("Expected HardwareFailure"),
        }
    }

    #[tokio::test]
    async fn test_move_clamps_negative_pan_to_limits() {
        let mut mock = mock_with_open();
        // Pan -500 should be clamped to -350, turning Left 350 degrees
        mock.expect_ptz_start_turn()
            .withf(|dir, deg| *dir == ptz_turn_direction::PTZ_TURN_LEFT && *deg == 350)
            .times(1)
            .returning(|_, _| Ok(true));
        mock.expect_ptz_wait_turn()
            .returning(|_| Ok(TurnOutcome::default()));
        mock.expect_ptz_stop().returning(|_| Ok(()));

        let ptz = create_opened(mock);
        let result = move_and_settle(&ptz, PtzPosition::new(-500.0, 0.0, 1.0)).await;
        assert!(result.is_ok());

        let pos = ptz.get_position().await.unwrap();
        assert!((pos.pan - PTZ_MIN_PAN_DEGREES).abs() < f32::EPSILON);
    }

    #[tokio::test]
    async fn test_move_clamps_negative_tilt_to_limits() {
        let mut mock = mock_with_open();
        // Tilt -200 should be clamped to -130, turning Up 130 degrees
        mock.expect_ptz_start_turn()
            .withf(|dir, deg| *dir == ptz_turn_direction::PTZ_TURN_UP && *deg == 130)
            .times(1)
            .returning(|_, _| Ok(true));
        mock.expect_ptz_wait_turn()
            .returning(|_| Ok(TurnOutcome::default()));
        mock.expect_ptz_stop().returning(|_| Ok(()));

        let ptz = create_opened(mock);
        let result = move_and_settle(&ptz, PtzPosition::new(0.0, -200.0, 1.0)).await;
        assert!(result.is_ok());

        let pos = ptz.get_position().await.unwrap();
        assert!((pos.tilt - PTZ_MIN_TILT_DEGREES).abs() < f32::EPSILON);
    }

    #[tokio::test]
    async fn test_move_both_axes_verifies_directions() {
        let mut mock = mock_with_open();
        let mut seq = mockall::Sequence::new();

        // First call: pan Right 100 degrees (positive pan delta)
        mock.expect_ptz_start_turn()
            .withf(|dir, deg| *dir == ptz_turn_direction::PTZ_TURN_RIGHT && *deg == 100)
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_, _| Ok(true));

        // Second call: tilt Up 50 degrees (negative tilt delta → Up)
        mock.expect_ptz_start_turn()
            .withf(|dir, deg| *dir == ptz_turn_direction::PTZ_TURN_UP && *deg == 50)
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_, _| Ok(true));

        mock.expect_ptz_wait_turn()
            .returning(|_| Ok(TurnOutcome::default()));
        mock.expect_ptz_stop().returning(|_| Ok(()));

        let ptz = create_opened(mock);
        let result = move_and_settle(&ptz, PtzPosition::new(100.0, -50.0, 1.0)).await;
        assert!(result.is_ok());
    }
}
