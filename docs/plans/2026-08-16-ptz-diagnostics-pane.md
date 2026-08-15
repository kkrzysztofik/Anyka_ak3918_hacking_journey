# PTZ Diagnostics Pane — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give the WebUI diagnostics view a PTZ card that explains *why* PTZ is dead — config flag, motor open errno, self-check outcome — alongside live tracked position and the cached motor step readback.

**Architecture:** `init_ptz_control` stops discarding its failure reason and returns an `OptionalInitResult`. `AnykaPlatform` stores it plus a concrete `Arc<AnykaPTZControl>`, and answers a new sync `Platform::ptz_diagnostics()` (default `None`, mirroring `stream_frame_age_ms`). The diagnostics snapshot gains a `ptz` block beside `vision`; the page renders a `PtzCard` mirroring `VisionCard`. No hardware I/O and no driver lock on the poll path — the step position comes from the `TurnOutcome` the actor already reads after every turn.

**Tech Stack:** Rust (onvif-rust, serde, mockall, host-side `x86_64-unknown-linux-gnu` tests) + TypeScript/React 19/Vitest (www).

**Spec:** `docs/plans/2026-08-16-ptz-diagnostics-pane-design.md`

## Global Constraints

- Rust: no `unwrap()`/`expect()`/`panic!()` in production paths; `tracing` for logs; tests in `#[cfg(test)] mod tests` next to the code; test names `test_<component>_<scenario>_<outcome>`.
- Rust toolchain: `source ./setenv.sh` (repo root), then `$CARGO test --target x86_64-unknown-linux-gnu` from `cross-compile/onvif-rust`.
- **Never call `ptz_get_step_pos` from the diagnostics path.** It takes the driver's main lock, which a continuous sweep holds for up to 10 s. Everything this plan reads is a `parking_lot` guard or an atomic.
- www: `data-testid` selectors only (no role/text/class); shadcn primitives from `src/components/ui/`; strict TS, no `any`.
- www quality gates from `cross-compile/www`: `npm run lint`, `npm run type-check`, `npm run test`.
- Keep existing tests green; add or update tests for every behavior change.

---

### Task 1: `PtzDiagnostics` type and the `Platform` hook

**Files:**
- Modify: `cross-compile/onvif-rust/src/platform/common/traits.rs` (new types near `VisionDiagnostics` at line ~483; new trait method in `trait Platform` next to `stream_frame_age_ms` at line ~770)
- Modify: `cross-compile/onvif-rust/src/platform/mod.rs` (re-export, if `VisionDiagnostics` is re-exported there — check and match)

**Interfaces:**
- Produces: `PtzDiagnostics`, `StepPos`, `Platform::ptz_diagnostics(&self) -> Option<PtzDiagnostics>` defaulting to `None`.

- [ ] **Step 1: Write the failing test**

In `traits.rs`, in `mod tests` (add one if absent), add:

```rust
#[test]
fn test_ptz_diagnostics_disabled_reports_not_enabled() {
    let d = PtzDiagnostics::disabled();
    assert!(!d.enabled);
    assert!(!d.opened);
    assert!(d.init_error.is_none());
    assert!(d.position.is_none());
}

#[test]
fn test_ptz_diagnostics_failed_retains_the_error() {
    let d = PtzDiagnostics::failed("open /dev/ak-motor0: errno 19".to_string());
    assert!(d.enabled, "the flag was on; the hardware is what failed");
    assert!(!d.opened);
    assert_eq!(
        d.init_error.as_deref(),
        Some("open /dev/ak-motor0: errno 19"),
        "the device path and errno are the whole point of this field"
    );
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run (from `cross-compile/onvif-rust`): `source ../../setenv.sh && $CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust test_ptz_diagnostics`
Expected: FAIL — `PtzDiagnostics` does not exist.

- [ ] **Step 3: Add the types**

In `traits.rs`, immediately after the `VisionDiagnostics` struct:

```rust
/// Last motor step position observed by the PTZ actor, with its age.
///
/// Sampled from the `TurnOutcome` the actor already reads at the end of every turn —
/// never by a fresh `ptz_get_step_pos`, which takes the driver's main lock a continuous
/// sweep can hold for ten seconds.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub struct StepPos {
    pub pan: Option<i32>,
    pub tilt: Option<i32>,
    /// Milliseconds since this sample was taken.
    pub age_ms: u64,
}

/// A point-in-time snapshot of the PTZ subsystem, including why it failed to come up.
///
/// Lives on [`Platform`] rather than on [`PTZControl`] deliberately: `ptz_control()` is
/// `None` exactly when the interesting failure has happened, so a `PTZControl` method
/// would go blank precisely when PTZ breaks.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PtzDiagnostics {
    /// `[ptz] enabled` from config.
    pub enabled: bool,
    /// Motor devices opened successfully.
    pub opened: bool,
    /// Why bring-up failed, carrying the device path and errno. `None` when it did not.
    pub init_error: Option<String>,
    /// Calibration sweep outcome: `None` = never run, `Some("ok")`, or the failure text.
    pub self_check: Option<String>,
    /// Tracked pan/tilt/zoom in degrees — dead-reckoned from commanded moves, not
    /// measured. See `last_step_pos` for what the motor actually reported.
    pub position: Option<[f32; 3]>,
    pub moving: bool,
    pub last_step_pos: Option<StepPos>,
    /// Commands the actor has fully finished. Answers "did my command reach the motor?"
    pub commands_completed: u32,
}

impl PtzDiagnostics {
    /// PTZ turned off in configuration: no bring-up was attempted.
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            opened: false,
            init_error: None,
            self_check: None,
            position: None,
            moving: false,
            last_step_pos: None,
            commands_completed: 0,
        }
    }

    /// PTZ was enabled but the motor devices could not be opened.
    pub fn failed(error: String) -> Self {
        Self {
            enabled: true,
            init_error: Some(error),
            ..Self::disabled()
        }
    }
}
```

In `trait Platform`, directly below `stream_frame_age_ms`:

```rust
    /// Point-in-time PTZ diagnostics, including bring-up failures.
    ///
    /// Sync and I/O-free by contract: it is called from the `/api/diagnostics` poll and
    /// must never take the motor driver's lock. Default: `None` (no PTZ reporting).
    fn ptz_diagnostics(&self) -> Option<PtzDiagnostics> {
        None
    }
```

Check how `VisionDiagnostics` is re-exported (`rtk grep -n "VisionDiagnostics" src/platform/mod.rs`) and add `PtzDiagnostics`/`StepPos` alongside it.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `$CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust test_ptz_diagnostics`
Expected: PASS. Then `$CARGO build --target x86_64-unknown-linux-gnu` — expect a clean build (the trait default means no implementor changes).

- [ ] **Step 5: Commit**

```bash
rtk git add cross-compile/onvif-rust/src/platform/common/traits.rs cross-compile/onvif-rust/src/platform/mod.rs
rtk git commit -m "feat(platform): add PtzDiagnostics and the Platform::ptz_diagnostics hook"
```

---

### Task 2: `PTZHandle` keeps the self-check outcome

**Files:**
- Modify: `cross-compile/onvif-rust/src/hal/common/ptz.rs` (`PTZHandle` at line ~77, `ptz_open` at line ~105, `mod tests`)

**Interfaces:**
- Produces: `PTZHandle::self_check_error() -> Option<&str>` — `None` when the sweep succeeded.

- [ ] **Step 1: Write the failing test**

In `hal/common/ptz.rs` `mod tests`, extend the two existing open tests:

```rust
#[test]
fn test_ptz_open_records_check_self_failure_on_the_handle() {
    let mut mock_ffi = MockPtzHalTrait::new();
    mock_ffi.expect_ptz_open().times(1).returning(|| Ok(()));
    mock_ffi.expect_ptz_check_self().times(1).returning(|_| {
        Err(PlatformError::HardwareFailure("motor wait timed out".to_string()))
    });
    mock_ffi.expect_ptz_close().returning(|| Ok(()));

    let handle = ptz_open(std::sync::Arc::new(mock_ffi)).unwrap();
    assert!(
        handle.self_check_error().is_some_and(|e| e.contains("motor wait timed out")),
        "a warn! log is not reachable from the WebUI; the handle must carry it"
    );
}

#[test]
fn test_ptz_open_clean_self_check_records_no_error() {
    let mut mock_ffi = MockPtzHalTrait::new();
    mock_ffi.expect_ptz_open().times(1).returning(|| Ok(()));
    mock_ffi.expect_ptz_check_self().times(1).returning(|_| Ok(()));
    mock_ffi.expect_ptz_close().returning(|| Ok(()));

    let handle = ptz_open(std::sync::Arc::new(mock_ffi)).unwrap();
    assert!(handle.self_check_error().is_none());
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `$CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust test_ptz_open`
Expected: FAIL — no method `self_check_error`.

- [ ] **Step 3: Implement**

Add the field to `PTZHandle` and populate it in `ptz_open` (keep the existing `warn!`, it is still the right log):

```rust
pub struct PTZHandle {
    opened: bool,
    ffi: std::sync::Arc<dyn PtzHalTrait>,
    /// Calibration sweep failure, kept for diagnostics. `None` = the sweep succeeded.
    self_check_error: Option<String>,
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
}
```

and in `ptz_open`:

```rust
pub(crate) fn ptz_open(ffi: std::sync::Arc<dyn PtzHalTrait>) -> PlatformResult<PTZHandle> {
    ffi.ptz_open()?;

    // PTZ_FEEDBACK_PIN_NONE = 0 (no feedback pin on this hardware).
    let self_check_error = match ffi.ptz_check_self(ptz_feedback_pin::PTZ_FEEDBACK_PIN_NONE) {
        Ok(()) => None,
        Err(e) => {
            tracing::warn!("PTZ self-check failed, continuing anyway: {}", e);
            Some(e.to_string())
        }
    };

    Ok(PTZHandle {
        opened: true,
        ffi,
        self_check_error,
    })
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `$CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust ptz`
Expected: PASS (all PTZ tests).

- [ ] **Step 5: Commit**

```bash
rtk git add cross-compile/onvif-rust/src/hal/common/ptz.rs
rtk git commit -m "feat(ptz): keep the self-check outcome on PTZHandle"
```

---

### Task 3: The actor caches the step position it already reads

**Files:**
- Modify: `cross-compile/onvif-rust/src/platform/anyka/ptz_actor.rs` (`PtzActorState` at line ~46, `drain_started_turns` at line ~241, `do_continuous` at line ~289, `mod tests` — add one if absent)

**Interfaces:**
- Produces: `PtzActorState.last_step: RwLock<Option<StepSample>>` and `PtzActorState::record_step(is_pan: bool, step: i32)`.

- [ ] **Step 1: Write the failing test**

In `ptz_actor.rs`, in `mod tests`:

```rust
#[test]
fn test_actor_state_record_step_keeps_both_axes() {
    let state = PtzActorState::new();
    assert!(state.last_step.read().is_none(), "nothing observed yet");

    state.record_step(true, 0);
    state.record_step(false, 137);

    let sample = state.last_step.read().expect("a sample was recorded");
    assert_eq!(sample.pan, Some(0), "a zero step is a real reading, not absence");
    assert_eq!(sample.tilt, Some(137));
}
```

Note the assertion on `Some(0)`: on V500 hardware `MOTOR_GET_STATUS` writes nothing and the step stays 0 forever. `Some(0)` and `None` must stay distinguishable — that difference is the whole diagnostic.

- [ ] **Step 2: Run the test to verify it fails**

Run: `$CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust test_actor_state_record_step`
Expected: FAIL — no field `last_step`.

- [ ] **Step 3: Implement**

Add to `ptz_actor.rs` (needs `use std::time::Instant;`):

```rust
/// The most recent motor step readback, per axis, with when it was taken.
#[derive(Debug, Clone, Copy)]
pub(crate) struct StepSample {
    pub pan: Option<i32>,
    pub tilt: Option<i32>,
    pub at: Instant,
}
```

Extend `PtzActorState` with `pub last_step: RwLock<Option<StepSample>>` (initialised `RwLock::new(None)` in `new()`), and add:

```rust
    /// Record a step readback observed at the end of a turn.
    ///
    /// Called only from the actor thread with a `TurnOutcome` already in hand — this
    /// never issues an ioctl of its own.
    pub(crate) fn record_step(&self, is_pan: bool, step: i32) {
        let mut guard = self.last_step.write();
        let mut sample = guard.unwrap_or(StepSample {
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
```

In `drain_started_turns`, record before the `interrupted` check so a preempted turn still reports where the motor said it was:

```rust
        let is_pan = matches!(axis, Axis::Pan(_));
        state.record_step(is_pan, outcome.step_pos);

        if outcome.interrupted {
```

In `do_continuous`, carry the axis alongside the direction. Change `started.push(sdk_dir);` to `started.push((sdk_dir, is_pan));`, where the axes array gains an `is_pan` element (`true` for the pan tuple, `false` for tilt), and the drain loop becomes:

```rust
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
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `$CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust ptz_actor`
Expected: PASS, including the existing actor tests.

- [ ] **Step 5: Commit**

```bash
rtk git add cross-compile/onvif-rust/src/platform/anyka/ptz_actor.rs
rtk git commit -m "feat(ptz): cache the step position the actor already reads per turn"
```

---

### Task 4: `AnykaPTZControl::diagnostics()`

**Files:**
- Modify: `cross-compile/onvif-rust/src/platform/anyka/ptz_control.rs` (impl block at line ~111, `mod tests`)

**Interfaces:**
- Produces: `AnykaPTZControl::diagnostics(&self) -> PtzDiagnostics` — the "PTZ came up" case, so `enabled: true, opened: true`.

- [ ] **Step 1: Write the failing test**

In `ptz_control.rs` `mod tests` (it already builds mocks via `with_ffi`; follow the existing helper style):

```rust
#[tokio::test]
async fn test_diagnostics_reports_open_state_and_tracked_position() {
    let ptz = AnykaPTZControl::with_ffi(std::sync::Arc::new(mock_ptz_hal_ok()));
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
    let ptz = AnykaPTZControl::with_ffi(std::sync::Arc::new(mock_ptz_hal_ok()));
    let d = ptz.diagnostics();
    assert!(!d.opened);
    assert!(d.self_check.is_none(), "the sweep only runs on open");
}
```

Reuse or add a `mock_ptz_hal_ok()` helper returning a `MockPtzHalTrait` with `ptz_open`/`ptz_check_self`/`ptz_close` returning `Ok`. Check the existing test module first — an equivalent helper probably exists; do not duplicate it.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `$CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust test_diagnostics_reports_open_state`
Expected: FAIL — no method `diagnostics`.

- [ ] **Step 3: Implement**

In the `impl AnykaPTZControl` block:

```rust
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
        let last_step_pos = self.shared.last_step.read().map(|s| StepPos {
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
```

Import `PtzDiagnostics` and `StepPos` from `crate::platform::traits`.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `$CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust ptz_control`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
rtk git add cross-compile/onvif-rust/src/platform/anyka/ptz_control.rs
rtk git commit -m "feat(ptz): report live PTZ diagnostics without touching the driver lock"
```

---

### Task 5: `init_ptz_control` keeps its failure reason

**Files:**
- Modify: `cross-compile/onvif-rust/src/platform/anyka/mod.rs` (`init_ptz_control` at line ~117, `AnykaPlatform` struct at line ~81, `with_isp_config` at line ~287, `impl Platform` at line ~509)
- Modify: `cross-compile/onvif-rust/src/platform/anyka/tests/platform_tests.rs` (line ~208)

**Interfaces:**
- Produces: `init_ptz_control(enabled) -> OptionalInitResult<Arc<AnykaPTZControl>>`; `AnykaPlatform::ptz_diagnostics()`.

- [ ] **Step 1: Write the failing tests**

Replace the existing `test_init_ptz_control_skips_bring_up_when_disabled` in `platform_tests.rs` and add a sibling:

```rust
#[test]
fn test_init_ptz_control_disabled_reports_disabled_not_failed() {
    let result = super::super::init_ptz_control(false);
    assert!(matches!(result, OptionalInitResult::Disabled));
    assert!(
        result.error_message().is_none(),
        "disabled is a choice, not a failure"
    );
}

#[test]
fn test_init_ptz_control_enabled_succeeds_with_the_stub_hal() {
    assert!(super::super::init_ptz_control(true).is_success());
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `$CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust init_ptz_control`
Expected: FAIL — `init_ptz_control` returns `Option`, so the `matches!` does not compile.

- [ ] **Step 3: Implement**

Rewrite `init_ptz_control` (keep the doc comment, it is still accurate):

```rust
fn init_ptz_control(enabled: bool) -> OptionalInitResult<Arc<AnykaPTZControl>> {
    if !enabled {
        tracing::info!("PTZ disabled by config (ptz.enabled = false); skipping motor bring-up");
        return OptionalInitResult::Disabled;
    }

    tracing::info!("Initializing PTZ (native Rust driver, /dev/ak-motor0, /dev/ak-motor1)");
    let ptz = AnykaPTZControl::new();
    match ptz.open() {
        Ok(()) => {
            tracing::info!("PTZ device opened successfully");
            OptionalInitResult::Success(Arc::new(ptz))
        }
        Err(e) => {
            tracing::error!("PTZ device failed to open, PTZ features will be unavailable: {}", e);
            OptionalInitResult::Failed {
                component: "PTZ".to_string(),
                // The device path and errno live in this string. Losing it is what made
                // PTZ bring-up undiagnosable from anywhere but a telnet session.
                error: e.to_string(),
            }
        }
    }
}
```

Add `ptz_init: OptionalInitResult<Arc<AnykaPTZControl>>` to the `AnykaPlatform` struct, keeping `ptz_control` (the accessor macro still needs it), and in `with_isp_config` replace `let ptz_control = init_ptz_control(ptz_enabled);` with:

```rust
        let ptz_init = init_ptz_control(ptz_enabled);
        let ptz_control = match &ptz_init {
            OptionalInitResult::Success(ptz) => Some(Arc::clone(ptz) as Arc<dyn PTZControl>),
            _ => None,
        };
```

adding `ptz_init` to the struct literal. Do the same in `AnykaPlatform::new()` (line ~250) — set `ptz_init: OptionalInitResult::Disabled` if that constructor does not bring PTZ up, matching whatever it does for `ptz_control` today.

In `impl Platform for AnykaPlatform`, below `stream_frame_age_ms`:

```rust
    fn ptz_diagnostics(&self) -> Option<PtzDiagnostics> {
        Some(match &self.ptz_init {
            OptionalInitResult::Disabled => PtzDiagnostics::disabled(),
            OptionalInitResult::Failed { error, .. } => PtzDiagnostics::failed(error.clone()),
            OptionalInitResult::Success(ptz) => ptz.diagnostics(),
        })
    }
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `$CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust platform::anyka`
Expected: PASS. Fix any call sites the compiler flags.

- [ ] **Step 5: Commit**

```bash
rtk git add cross-compile/onvif-rust/src/platform/anyka/mod.rs cross-compile/onvif-rust/src/platform/anyka/tests/platform_tests.rs
rtk git commit -m "fix(ptz): stop discarding the motor open failure reason"
```

---

### Task 6: The snapshot carries the `ptz` block

**Files:**
- Modify: `cross-compile/onvif-rust/src/diagnostics/state.rs` (`Snapshot` at line ~62, `snapshot()` at line ~259, `mod tests`)

**Interfaces:**
- Produces: `Snapshot.ptz: Option<PtzDiagnostics>`, serialised as `ptz` in `/api/diagnostics`.

- [ ] **Step 1: Write the failing tests**

In `state.rs` `mod tests`:

```rust
#[tokio::test]
async fn test_snapshot_ptz_none_without_platform() {
    let state = DiagnosticsState::new(Instant::now(), None, Vec::new());
    assert!(state.snapshot().await.ptz.is_none());
}

#[tokio::test]
async fn test_snapshot_carries_ptz_init_error() {
    use crate::platform::MockPlatform;

    let mut platform = MockPlatform::new();
    platform.expect_ptz_diagnostics().returning(|| {
        Some(crate::platform::PtzDiagnostics::failed(
            "open /dev/ak-motor0: errno 19".to_string(),
        ))
    });
    platform.expect_imaging_control().returning(|| None);

    let state = DiagnosticsState::new(Instant::now(), Some(std::sync::Arc::new(platform)), Vec::new());
    let snap = state.snapshot().await;
    let ptz = snap.ptz.expect("a platform reporting PTZ must reach the snapshot");
    assert_eq!(ptz.init_error.as_deref(), Some("open /dev/ak-motor0: errno 19"));
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `$CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust test_snapshot_ptz`
Expected: FAIL — no field `ptz` on `Snapshot`.

- [ ] **Step 3: Implement**

Add to `Snapshot`, after `vision`:

```rust
    /// PTZ subsystem state, including bring-up failures. `None` when the platform does
    /// not report PTZ at all.
    pub ptz: Option<PtzDiagnostics>,
```

Import it (`use crate::platform::{Platform, PtzDiagnostics, VisionDiagnostics};`), and in `snapshot()`, next to the `vision` block — note this one is sync, so no `.await`:

```rust
        let ptz = self.platform.as_ref().and_then(|p| p.ptz_diagnostics());
```

Add `ptz` to the returned `Snapshot { .. }`.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `$CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust diagnostics`
Expected: PASS. Other `MockPlatform` tests in this module may need `expect_ptz_diagnostics().returning(|| None)` — add it where the compiler or a panicking mock says so.

- [ ] **Step 5: Commit**

```bash
rtk git add cross-compile/onvif-rust/src/diagnostics/state.rs
rtk git commit -m "feat(diagnostics): expose the PTZ block in /api/diagnostics"
```

---

### Task 7: Motor ops stop faking success when there is no PTZ

**Files:**
- Modify: `cross-compile/onvif-rust/src/onvif/ptz/ops/movement.rs` (the `else` branches at lines ~97, ~149, and the `continuous_move`/`stop` equivalents)
- Modify: `cross-compile/onvif-rust/src/onvif/ptz/ops/status.rs` (`goto_home_position`, line ~72)
- Modify: `cross-compile/onvif-rust/src/onvif/ptz/service.rs` (`create_test_service`, line ~606)
- Modify: `cross-compile/onvif-rust/src/onvif/ptz/ops/presets.rs` (test call sites at lines ~138, ~168, ~189, ~207)

**Interfaces:**
- Produces: `OnvifError::ActionNotSupported` from every motor op when `ptz_control` is `None`.

**Why:** with `ptz.enabled = false` these ops currently write the requested position into ONVIF state and return `Ok`, so a client that skips `GetCapabilities` gets `200 OK` and a `GetStatus` position that moved while the camera stood still. The simulation branch exists for unit tests; this moves it into `StubPTZControl` where it belongs.

- [ ] **Step 1: Write the failing test**

In `movement.rs` `mod tests`:

```rust
#[tokio::test]
async fn test_continuous_move_without_hardware_faults_instead_of_faking_success() {
    let state = PTZStateManager::new();
    let result = continuous_move(&state, &None, "Profile1", PTZSpeed::default()).await;
    assert!(
        matches!(result, Err(crate::onvif::error::OnvifError::ActionNotSupported(_))),
        "a disabled PTZ must refuse the move, not report a move that never happened"
    );
    assert!(!state.is_moving(), "a refused command must not leave state moving");
}
```

Use whatever `PTZStateManager` accessor exists for the moving flag; check `state.rs` and adapt the second assertion (drop it if no accessor exists rather than adding one).

- [ ] **Step 2: Run the test to verify it fails**

Run: `$CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust test_continuous_move_without_hardware`
Expected: FAIL — the op returns `Ok(())`.

- [ ] **Step 3: Implement**

In each motor op in `movement.rs` (`absolute_move`, `relative_move`, `continuous_move`) and `status.rs` (`goto_home_position`), replace the simulation `else` branch with:

```rust
    } else {
        state.stop();
        return Err(crate::onvif::error::OnvifError::ActionNotSupported(
            "PTZ is not available on this device".to_string(),
        ));
    }
```

Then give the ONVIF tests real hardware to talk to: in `service.rs`, change `create_test_service()` to attach a `StubPTZControl` (public, `platform/stub/mod.rs:789`) via the existing `with_platform` constructor used at lines 1651+, or by constructing the service with a stub platform. In `presets.rs`, the four `&None` test call sites become a `&Some(stub)`.

Leave `stop()` alone if it already tolerates a missing PTZ — stopping a motor that does not exist is not a lie.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `$CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust onvif::ptz`
Expected: PASS across all 43 service tests plus the ops tests.

**If the churn here exceeds roughly a dozen test edits, stop and report.** The pane does not depend on this task; the design explicitly allows dropping it rather than letting it balloon.

- [ ] **Step 5: Commit**

```bash
rtk git add cross-compile/onvif-rust/src/onvif/ptz/
rtk git commit -m "fix(onvif): fault instead of faking success when PTZ is unavailable"
```

---

### Task 8: WebUI service type and guard

**Files:**
- Modify: `cross-compile/www/src/services/diagnosticsService.ts` (`Diagnostics` at line ~14, `isDiagnostics` at line ~84)
- Test: `cross-compile/www/src/services/diagnosticsService.test.ts`

**Interfaces:**
- Produces: `Diagnostics['ptz']`, validated leniently — a payload without the key still parses.

- [ ] **Step 1: Write the failing test**

In `diagnosticsService.test.ts`, following the existing vision-parsing tests:

```ts
it('should accept a diagnostics payload carrying a ptz block', async () => {
  const payload = { ...validPayload, ptz: { enabled: true, opened: false, init_error: 'open /dev/ak-motor0: errno 19', self_check: null, position: null, moving: false, last_step_pos: null, commands_completed: 0 } };
  mockFetchJson(payload);
  const result = await getDiagnostics();
  expect(result.ptz?.init_error).toBe('open /dev/ak-motor0: errno 19');
});

it('should accept a diagnostics payload with no ptz key', async () => {
  mockFetchJson(validPayload);
  await expect(getDiagnostics()).resolves.toBeDefined();
});
```

Match the file's existing fixture and fetch-mock helpers instead of inventing `validPayload`/`mockFetchJson` — read the file first.

- [ ] **Step 2: Run the tests to verify they fail**

Run (from `cross-compile/www`): `npm run test -- diagnosticsService`
Expected: FAIL — `result.ptz` is not on the type.

- [ ] **Step 3: Implement**

Add to the `Diagnostics` interface:

```ts
  ptz?: {
    enabled: boolean;
    opened: boolean;
    init_error: string | null;
    self_check: string | null;
    /** Tracked pan/tilt/zoom in degrees — dead-reckoned, not measured. */
    position: [number, number, number] | null;
    moving: boolean;
    last_step_pos: { pan: number | null; tilt: number | null; age_ms: number } | null;
    commands_completed: number;
  } | null;
```

and a guard mirroring `isVision`:

```ts
function isPtz(value: unknown): value is NonNullable<Diagnostics['ptz']> {
  return (
    isRecord(value) &&
    typeof value.enabled === 'boolean' &&
    typeof value.opened === 'boolean' &&
    typeof value.moving === 'boolean' &&
    typeof value.commands_completed === 'number'
  );
}
```

In `isDiagnostics`, before `return true`:

```ts
  // Absent is fine — a snapshot from a build without PTZ reporting still validates.
  if (value.ptz !== null && value.ptz !== undefined && !isPtz(value.ptz)) return false;
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `npm run test -- diagnosticsService`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
rtk git add cross-compile/www/src/services/diagnosticsService.ts cross-compile/www/src/services/diagnosticsService.test.ts
rtk git commit -m "feat(www): parse the PTZ diagnostics block"
```

---

### Task 9: The `PtzCard`

**Files:**
- Modify: `cross-compile/www/src/pages/DiagnosticsPage.tsx` (add `PtzCard` next to `VisionCard` at line ~128; render it in the grid at line ~804)
- Test: `cross-compile/www/src/pages/DiagnosticsPage.test.tsx`

**Interfaces:**
- Produces: `data-testid="diagnostics-ptz-*"` rows, mirroring the `diagnostics-vision-*` naming.

- [ ] **Step 1: Write the failing tests**

In `DiagnosticsPage.test.tsx` (add `ptz: null` to `BASE_DIAG` first so the existing tests keep type-checking):

```tsx
const PTZ_OK: NonNullable<Diagnostics['ptz']> = {
  enabled: true,
  opened: true,
  init_error: null,
  self_check: 'ok',
  position: [-12, 4.5, 1],
  moving: false,
  last_step_pos: { pan: 0, tilt: 0, age_ms: 14_000 },
  commands_completed: 47,
};

it('should render the PTZ bring-up and motion rows', () => {
  vi.mocked(useDiagnostics).mockReturnValue(makeResult({ ptz: PTZ_OK }));
  renderWithProviders(<DiagnosticsPage />);
  expect(screen.getByTestId('diagnostics-ptz-motors')).toHaveTextContent('open');
  expect(screen.getByTestId('diagnostics-ptz-position')).toHaveTextContent('-12.0° / 4.5°');
  expect(screen.getByTestId('diagnostics-ptz-commands')).toHaveTextContent('47');
});

it('should surface the motor open errno when bring-up failed', () => {
  vi.mocked(useDiagnostics).mockReturnValue(
    makeResult({ ptz: { ...PTZ_OK, opened: false, self_check: null, position: null, init_error: 'open /dev/ak-motor0: errno 19' } }),
  );
  renderWithProviders(<DiagnosticsPage />);
  expect(screen.getByTestId('diagnostics-ptz-init-error')).toHaveTextContent('errno 19');
});

it('should warn when the motor reports no step movement after completed commands', () => {
  vi.mocked(useDiagnostics).mockReturnValue(makeResult({ ptz: PTZ_OK }));
  renderWithProviders(<DiagnosticsPage />);
  expect(screen.getByTestId('diagnostics-ptz-no-readback')).toBeInTheDocument();
});

it('should collapse the card when PTZ is disabled in configuration', () => {
  vi.mocked(useDiagnostics).mockReturnValue(
    makeResult({ ptz: { ...PTZ_OK, enabled: false, opened: false, self_check: null, position: null, last_step_pos: null, commands_completed: 0 } }),
  );
  renderWithProviders(<DiagnosticsPage />);
  expect(screen.getByTestId('diagnostics-ptz-disabled-note')).toBeInTheDocument();
  expect(screen.queryByTestId('diagnostics-ptz-position')).not.toBeInTheDocument();
});

it('should not render the PTZ card when the backend reports no PTZ', () => {
  vi.mocked(useDiagnostics).mockReturnValue(makeResult({ ptz: null }));
  renderWithProviders(<DiagnosticsPage />);
  expect(screen.queryByTestId('diagnostics-ptz-title')).not.toBeInTheDocument();
});
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `npm run test -- DiagnosticsPage`
Expected: FAIL — no `diagnostics-ptz-*` test ids.

- [ ] **Step 3: Implement**

Add `PtzCard` immediately after `VisionCard`, copying its `Card`/`CardHeader`/`dl` structure exactly (icon: `Move` or `Navigation` from lucide-react; colour: use an unused accent such as cyan). Structure:

- Bring-up rows (always): Config (`enabled ? 'enabled' : 'disabled'`), Motors (`opened ? 'open' : 'not opened'`), Self-check (`self_check ?? '—'`).
- `init_error` renders as a red line, `data-testid="diagnostics-ptz-init-error"`, only when present.
- When `!enabled`: render `data-testid="diagnostics-ptz-disabled-note"` with "PTZ disabled in configuration" and stop — no motion rows.
- Motion rows otherwise: Position (`-12.0° / 4.5°` from `position[0]`/`position[1]`, one decimal, suffixed "(tracked)"), Step pos (`pan / tilt` with `formatDuration(age_ms / 1000)` ago, `—` when null), Moving, Commands.
- The no-readback warning, `data-testid="diagnostics-ptz-no-readback"`, when `enabled && opened && commands_completed > 0 && last_step_pos` is present with both axes `0`:

```tsx
// A step position pinned at 0 after completed commands is the signature of a motor
// driver whose MOTOR_GET_STATUS returns success and writes nothing.
```

Render it in the existing grid next to `VisionCard`:

```tsx
        {data?.ptz && <PtzCard ptz={data.ptz} />}
```

Reuse `formatDuration` — do not write a second age formatter.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `npm run test -- DiagnosticsPage`
Expected: PASS, existing page tests included.

- [ ] **Step 5: Commit**

```bash
rtk git add cross-compile/www/src/pages/DiagnosticsPage.tsx cross-compile/www/src/pages/DiagnosticsPage.test.tsx
rtk git commit -m "feat(www): add the PTZ diagnostics card"
```

---

### Task 10: The loading-flash test the previous plan skipped

**Files:**
- Test: `cross-compile/www/src/pages/LiveViewPage.test.tsx`

**Interfaces:** none — this covers existing behavior (`ptzDisabled = isSuccess && !hasPtz`, `LiveViewPage.tsx:115`).

- [ ] **Step 1: Write the test**

```tsx
it('should not show the PTZ disabled note while profiles are still loading', async () => {
  const { getProfiles } = await import('@/services/profileService');
  vi.mocked(getProfiles).mockReturnValueOnce(new Promise(() => {})); // never resolves
  renderWithProviders(<LiveViewPage />);
  await screen.findByTestId('liveview-ptz-title');
  expect(screen.queryByTestId('liveview-ptz-disabled-note')).not.toBeInTheDocument();
});
```

- [ ] **Step 2: Run it**

Run: `npm run test -- LiveViewPage`
Expected: PASS immediately — the behavior already exists; this pins it. If it fails, the `isSuccess` gate regressed and that is the bug to fix.

- [ ] **Step 3: Commit**

```bash
rtk git add cross-compile/www/src/pages/LiveViewPage.test.tsx
rtk git commit -m "test(www): pin that the PTZ disabled note does not flash while loading"
```

---

## Final validation (after all tasks)

- [ ] Rust, from `cross-compile/onvif-rust`: `source ../../setenv.sh && $CARGO fmt --check && $CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings && $CARGO test --target x86_64-unknown-linux-gnu`
- [ ] ARM build, from `cross-compile/onvif-rust` (never the workspace root — it silently links with the host toolchain): `$CARGO build --release --target armv5te-unknown-linux-gnueabi`
- [ ] www, from `cross-compile/www`: `npm run lint && npm run type-check && npm run test`
- [ ] Update `docs/README.md` — flip the plan column for the 2026-08-16 row from `—` to ✅.
- [ ] Hardware check on a PTZ camera: `curl -u <user>:<pass> http://<camera>/api/diagnostics | jq .ptz` — confirm `init_error` is populated on a camera whose motors fail, and that `last_step_pos` stays `{pan: 0, tilt: 0}` on V500 hardware while `position` moves. That divergence is the expected result, not a bug in this feature.
