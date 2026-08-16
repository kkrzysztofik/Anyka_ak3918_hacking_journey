# PTZ Position Tracking Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make the camera know where its lens is pointing, so PTZ presets work and the view survives a restart.

**Architecture:** There is no hardware position readback (`MOTOR_GET_STATUS` is a silent no-op on this hardware), so position is dead-reckoned: motor-on time × a measured degrees-per-second constant, accumulated per axis on the PTZ actor thread. The physical limit-switch sweep that already runs at every `open()` provides the absolute origin, and `GotoHomePosition` re-runs it as the drift reset. Position and presets persist to their own TOML files beside the config.

**Tech Stack:** Rust 2024, `tokio`, `mockall`, `parking_lot`, `serde`/`toml`. Vendored cross toolchain.

**Design doc:** `docs/plans/2026-08-16-ptz-position-tracking-design.md` — read it first.

---

## Before You Start

**Toolchain.** All cargo commands use the vendored toolchain, never the system one:

```bash
export CARGO=toolchain/arm-anykav200-crosstool-ng/bin/cargo
export PATH=toolchain/arm-anykav200-crosstool-ng/bin:$PATH
cd cross-compile/onvif-rust
```

The `PATH` prefix is not optional for clippy — the vendored clippy fails with `E0514` without it.

**Every host-side command needs `--target x86_64-unknown-linux-gnu`.** Without it cargo builds for ARM and the tests will not run.

**The check you run after every task:**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --lib
$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
$CARGO fmt
```

**Conventions to match** (from `AGENTS.md`): test names are `test_<subject>_<behavior>` — `test_move_positive_pan_turns_right`, never `test_init`. No `unwrap()`/`expect()` outside tests. Mocks are `mockall` `#[automock]`, never hand-written.

**Orientation.** The three files you will spend most of this plan in:
- `src/platform/anyka/ptz_actor.rs` — the OS thread that owns the motor HAL and is the only writer of tracked position.
- `src/platform/anyka/ptz_control.rs` — the async shim in front of that thread.
- `src/onvif/ptz/` — the SOAP layer, which keeps its own cached copy of position in `PTZStateManager`.

Tasks 1–9 are pure logic and run entirely on the host. Tasks 10–13 add persistence. Task 14 is the only one needing the camera.

---

## Task 1: Add the calibration constants to config

**Files:**
- Modify: `src/config/types.rs:602-636` (`PtzConfig` + its `Default`), `src/config/types.rs:152-158` (validation)

**Step 1: Write the failing test**

Add to the `mod tests` block at the bottom of `src/config/types.rs`:

```rust
#[test]
fn test_ptz_config_default_rates_are_positive() {
    let c = PtzConfig::default();
    assert!(c.pan_degrees_per_sec > 0.0);
    assert!(c.tilt_degrees_per_sec > 0.0);
}

#[test]
fn test_ptz_config_rejects_zero_pan_rate() {
    let mut config = AppConfig::default();
    config.ptz.pan_degrees_per_sec = 0.0;
    let errors = config.validate().expect_err("a zero rate divides motion by nothing");
    assert!(errors.iter().any(|e| e.contains("pan_degrees_per_sec")));
}
```

**Step 2: Run it and watch it fail**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --lib config::types::tests::test_ptz_config
```
Expected: FAIL — `no field pan_degrees_per_sec on type PtzConfig`.

**Step 3: Implement**

In `PtzConfig`, add:

```rust
    /// Degrees per second the pan axis travels at the driver's fixed speed.
    /// Measured on hardware — see the design doc §4 for the procedure.
    pub pan_degrees_per_sec: f64,
    /// Degrees per second the tilt axis travels at the driver's fixed speed.
    pub tilt_degrees_per_sec: f64,
```

In `impl Default for PtzConfig`, add `pan_degrees_per_sec: 60.0,` and `tilt_degrees_per_sec: 60.0,`. These are placeholders replaced by measurement in Task 14 — leave a comment saying so.

In `validate()` beside the other `ptz.*` range checks:

```rust
        range(&mut errors, "ptz.pan_degrees_per_sec", self.ptz.pan_degrees_per_sec, 0.1, 360.0);
        range(&mut errors, "ptz.tilt_degrees_per_sec", self.ptz.tilt_degrees_per_sec, 0.1, 360.0);
```

**Step 4: Verify**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --lib config::
```
Expected: PASS.

**Step 5: Commit**

```bash
git add src/config/types.rs
git commit -m "feat(config): add PTZ degrees-per-second calibration constants"
```

---

## Task 2: Delete the dead PTZ config fields

`presets_json`, `next_preset_num`, `home_pan`, `home_tilt`, `home_zoom` are defined, defaulted, partly range-validated, and read by **nothing** — confirm with `grep -rn "presets_json\|next_preset_num\|home_pan" src/` before deleting; the only hits should be their own definitions and validation lines.

`#[serde(default)]` on `PtzConfig` means deployed config files still carrying these keys load fine — serde ignores unknown fields. Verify that claim with a test rather than trusting it.

**Files:**
- Modify: `src/config/types.rs` (`PtzConfig`, its `Default`, `validate()`)

**Step 1: Write the failing test**

```rust
#[test]
fn test_ptz_config_ignores_removed_legacy_keys() {
    // A config file written by an older build still carries these keys.
    let toml = r#"
enabled = true
presets_json = "{}"
next_preset_num = 4
home_pan = 0.5
"#;
    let parsed: PtzConfig = toml::from_str(toml).expect("legacy keys must not break loading");
    assert!(parsed.enabled);
}
```

**Step 2: Run it**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --lib test_ptz_config_ignores_removed_legacy_keys
```
Expected: PASS even before deletion (the fields still exist). That is fine — this test exists to catch the *deletion* breaking loading, so run it again in step 4.

**Step 3: Delete the five fields** from the struct, from `Default`, and delete the three `home_*` lines from `validate()`.

**Step 4: Verify**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --lib
$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
```
Expected: PASS. The legacy-keys test still passes, proving backward compatibility.

**Step 5: Commit**

```bash
git add src/config/types.rs
git commit -m "refactor(config): drop unread PTZ preset and home-position fields"
```

---

## Task 3: The integration helper

The core of the whole feature. Pure function over `PtzActorState` — no HAL, no async, fully testable.

**Files:**
- Modify: `src/platform/anyka/ptz_actor.rs`

**Step 1: Write the failing tests**

Add to `mod tests` in `ptz_actor.rs`:

```rust
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
```

**Step 2: Run and watch them fail**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --lib ptz_actor::tests::test_integrate
```
Expected: FAIL — `cannot find function integrate_axis`.

**Step 3: Implement**

Add near the top of `ptz_actor.rs`:

```rust
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
```

Add `pub rates: PtzRates` to `PtzActorState` and take it as a parameter in `PtzActorState::new(rates: PtzRates)`. Fix the one existing caller in `ptz_control.rs::build` by threading a `PtzRates` argument through (Task 6 wires it to config; for now pass a `PtzRates` built from `PtzConfig::default()` values so the crate compiles).

Then:

```rust
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
```

Add `use std::time::Duration;` if not already imported.

**Step 4: Verify**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --lib ptz_actor::
```
Expected: PASS, all six.

**Step 5: Commit**

```bash
git add src/platform/anyka/ptz_actor.rs src/platform/anyka/ptz_control.rs
git commit -m "feat(ptz): add per-axis dead-reckoning integration helper"
```

---

## Task 4: Integrate continuous moves

**Files:**
- Modify: `src/platform/anyka/ptz_actor.rs` (`do_continuous`)
- Test: same file, `mod tests`

Read the design doc §1 on why there is **one** `Instant` pair for the whole command rather than one per axis. Do not "fix" this by timing each axis separately — it is deliberate and the reasoning is non-obvious.

**Step 1: Write the failing test**

`do_continuous` needs a HAL mock. Put this in `ptz_control.rs`'s test module instead, where `create_opened` already exists, since it exercises the whole shim:

```rust
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

        let ptz = create_opened_with_rates(mock, PtzRates {
            pan_deg_per_sec: 100.0,
            tilt_deg_per_sec: 100.0,
        });
        let before = ptz.shared.commands_completed.load(Ordering::SeqCst);
        ptz.continuous_move(PtzVelocity::new(1.0, 0.0, 0.0)).await.unwrap();
        await_actor_completed(&ptz, before).await;

        // 200ms at 100 deg/s ≈ 20 degrees. Wide tolerance: this measures real
        // elapsed time on a shared CI machine, so assert the magnitude, not the value.
        let pan = ptz.get_position().await.unwrap().pan;
        assert!(pan > 10.0 && pan < 40.0, "expected ~20 degrees of pan, got {}", pan);
    }

    #[tokio::test]
    async fn test_continuous_move_left_decreases_tracked_position() {
        // Same shape, velocity.pan = -1.0, assert pan < -10.0 && pan > -40.0.
    }
```

Add the `create_opened_with_rates` helper beside `create_opened`, and give `AnykaPTZControl` a test-only constructor taking `PtzRates` (mirroring the existing `with_ffi_and_timeout`).

**Step 2: Run and watch it fail**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --lib test_continuous_move_accumulates
```
Expected: FAIL — pan is `0.0`. That is the bug this whole plan exists to fix; see it fail before fixing it.

**Step 3: Implement**

In `do_continuous`, change `started` to carry the `PtzDirection` (not just `is_pan`), take `let started_at = Instant::now();` immediately before the first `ptz_start_turn`, and after the drain loop completes:

```rust
    let elapsed = started_at.elapsed();
    for (_, direction) in &started {
        integrate_axis(state, *direction, elapsed);
    }
```

Keep `state.record_step(...)` in the drain loop exactly as it is — the diagnostics pane reads it.

**Step 4: Verify**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --lib
```
Expected: PASS. Existing continuous-move tests must still pass — if `test_stop_clears_velocity` or the timeout test breaks, you changed more than the tuple shape.

**Step 5: Commit**

```bash
git add src/platform/anyka/ptz_actor.rs src/platform/anyka/ptz_control.rs
git commit -m "fix(ptz): track position across continuous moves"
```

---

## Task 5: Integrate interrupted absolute moves

Today `drain_started_turns` hits `interrupted` and `break`s, leaving the position untouched — so an interrupted absolute move loses every degree it actually travelled.

**Files:**
- Modify: `src/platform/anyka/ptz_actor.rs` (`do_move`, `drain_started_turns`)

**Step 1: Write the failing test**

```rust
    #[tokio::test]
    async fn test_interrupted_absolute_move_commits_partial_motion() {
        let mut mock = mock_with_open();
        mock.expect_ptz_start_turn().returning(|_, _| Ok(true));
        mock.expect_ptz_wait_turn().returning(|_| {
            std::thread::sleep(Duration::from_millis(200));
            Ok(TurnOutcome { interrupted: true, ..TurnOutcome::default() })
        });
        mock.expect_ptz_stop().returning(|_| Ok(()));

        let ptz = create_opened_with_rates(mock, PtzRates {
            pan_deg_per_sec: 100.0,
            tilt_deg_per_sec: 100.0,
        });
        move_and_settle(&ptz, PtzPosition::new(350.0, 0.0, 1.0)).await.unwrap();

        // Interrupted before reaching 350, but ~20 degrees of travel happened.
        let pan = ptz.get_position().await.unwrap().pan;
        assert!(pan > 10.0 && pan < 40.0, "partial motion lost: pan = {}", pan);
    }
```

**Step 2: Run and watch it fail**

Expected: FAIL — `pan = 0`.

**Step 3: Implement**

Pass `started_at: Instant` into `drain_started_turns` (captured in `do_move` before the first `ptz_start_turn`), extend the `started` tuples to carry `PtzDirection`, and replace the `interrupted` branch's bare `break` with:

```rust
        if outcome.interrupted {
            integrate_axis(state, direction, started_at.elapsed());
            tracing::debug!(
                "absolute move preempted on {:?} (step_pos={}); committed dead-reckoned partial motion",
                axis,
                outcome.step_pos
            );
            break;
        }
```

Leave the clean-completion branch alone — committing the exact commanded target is strictly better than integrating an estimate when the motor reports it arrived.

**Step 4: Verify**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --lib
```
Expected: PASS. `test_position_unchanged_after_pan_ffi_failure` must still pass — a *failed start* is not an interrupt and must still leave position alone.

**Step 5: Commit**

```bash
git add src/platform/anyka/ptz_actor.rs
git commit -m "fix(ptz): commit partial motion when an absolute move is interrupted"
```

---

## Task 6: Thread the config rates through

**Files:**
- Modify: `src/platform/anyka/ptz_control.rs` (`new`, `with_ffi`, `build`), `src/platform/anyka/mod.rs` (`init_ptz_control`, `with_isp_config`)

**Step 1: Write the failing test**

```rust
    #[tokio::test]
    async fn test_configured_rate_scales_tracked_motion() {
        // Two controls, same 200ms mock wait, rates differing 4x.
        // Assert the faster one accumulated roughly 4x the degrees.
        // (Ratio, not absolute values — immune to timing jitter.)
    }
```

Write it out fully following the Task 4 shape; the point is that the constant is genuinely read from config rather than hardcoded.

**Step 2: Run it** — Expected: FAIL, no way to inject rates through the public constructor.

**Step 3: Implement**

`AnykaPTZControl::new(rates: PtzRates)`; `init_ptz_control(enabled: bool, rates: PtzRates)`; `with_isp_config` gains a `PtzConfig` parameter (it already takes `ImagingConfig`, so this matches). Convert `f64` config values to `f32` at the boundary.

**Step 4: Verify** — `$CARGO test --target x86_64-unknown-linux-gnu` (full suite, including `tests/`, since `with_isp_config` callers live there).

**Step 5: Commit**

```bash
git commit -am "feat(ptz): drive dead reckoning from configured axis rates"
```

---

## Task 7: `PtzCommand::Home`

**Files:**
- Modify: `src/platform/anyka/ptz_actor.rs`, `src/platform/anyka/ptz_control.rs`, `src/platform/common/traits.rs`, `src/platform/stub/mod.rs`

**Step 1: Write the failing test**

```rust
    #[tokio::test]
    async fn test_home_runs_self_check_and_resets_position() {
        let mut mock = mock_with_open();
        mock.expect_ptz_start_turn().returning(|_, _| Ok(true));
        mock.expect_ptz_wait_turn().returning(|_| Ok(TurnOutcome::default()));
        mock.expect_ptz_stop().returning(|_| Ok(()));
        // check_self runs once at open() and once more for the re-home.
        mock.expect_ptz_check_self().times(2).returning(|_| Ok(()));

        let ptz = create_opened(mock);
        move_and_settle(&ptz, PtzPosition::new(90.0, 45.0, 1.0)).await.unwrap();

        let before = ptz.shared.commands_completed.load(Ordering::SeqCst);
        ptz.home().await.unwrap();
        await_actor_completed(&ptz, before).await;

        let pos = ptz.get_position().await.unwrap();
        assert!(pos.pan.abs() < f32::EPSILON);
        assert!(pos.tilt.abs() < f32::EPSILON);
    }
```

Note `mock_with_open` sets `expect_ptz_check_self` permissively — override it in this test so the count is meaningful.

**Step 2: Run it** — Expected: FAIL, `no method named home`.

**Step 3: Implement**

Add `Home { reply: oneshot::Sender<PlatformResult<()>> }` to `PtzCommand`, include it in `into_reply`, and in `execute`:

```rust
        PtzCommand::Home { reply } => {
            // Reply on acceptance: the limit-switch sweep can outrun PTZ_CMD_TIMEOUT,
            // and ONVIF treats these moves as asynchronous (same as MoveTo).
            let _ = reply.send(Ok(()));
            match ffi.ptz_check_self(ptz_feedback_pin::PTZ_FEEDBACK_PIN_EXIST) {
                Ok(()) => {
                    *state.position.write() = PtzPosition::HOME;
                    *state.velocity.write() = PtzVelocity::STOP;
                }
                Err(e) => tracing::error!("PTZ re-home sweep failed, position unchanged: {}", e),
            }
        }
```

Check the exact `ptz_check_self` argument type against `PtzHalTrait` in `src/hal/common/ptz.rs` and match what `ptz_open` passes.

Add `async fn home(&self) -> PlatformResult<()>;` to the `PTZControl` trait, implement on `AnykaPTZControl` via `self.submit(|reply| PtzCommand::Home { reply })` after `cancel_continuous_timeout()`, and on `StubPTZControl` as `*self.position.write() = PtzPosition::HOME; Ok(())`.

**Step 4: Verify** — full test suite. Adding a trait method breaks every `MockPTZControl` user that sets strict expectations; fix those.

**Step 5: Commit**

```bash
git commit -am "feat(ptz): add Home command driving the limit-switch sweep"
```

---

## Task 8: `GotoHomePosition` re-homes

**Files:**
- Modify: `src/onvif/ptz/ops/status.rs:70-95`

**Step 1: Write the failing test**

In the same file's `mod tests`, using `MockPTZControl`:

```rust
    #[tokio::test]
    async fn test_goto_home_position_rehomes_before_moving() {
        let state = create_test_state();
        let mut mock = MockPTZControl::new();
        let mut seq = mockall::Sequence::new();
        mock.expect_home().times(1).in_sequence(&mut seq).returning(|| Ok(()));
        mock.expect_move_to_position().times(1).in_sequence(&mut seq).returning(|_| Ok(()));
        mock.expect_get_position().returning(|| Ok(PtzPosition::HOME));

        let ptz: Option<Arc<dyn PTZControl>> = Some(Arc::new(mock));
        goto_home_position(&state, &ptz, "Profile1").await.unwrap();
    }
```

The `Sequence` is the point: re-home must happen *before* the move, or the dead-reckoned leg starts from an uncalibrated origin.

**Step 2: Run it** — Expected: FAIL, `expect_home` never called.

**Step 3: Implement** — in `goto_home_position`, immediately inside `if let Some(ptz) = ptz_control`, before the existing `move_to_position`:

```rust
        // Re-establish the physical origin first: this is the drift reset, and it makes
        // the dead-reckoned leg below the most accurate move the system can make.
        ptz.home().await.map_err(|e| {
            state.stop();
            crate::onvif::error::OnvifError::HardwareFailure(format!("PTZ re-home failed: {}", e))
        })?;
```

**Step 4: Verify** — `$CARGO test --target x86_64-unknown-linux-gnu --lib onvif::ptz::`

**Step 5: Commit**

```bash
git commit -am "feat(ptz): re-home the motors on GotoHomePosition"
```

---

## Task 9: Sync cached position on stop

Without this the whole feature *looks* broken: jog → save preset would still snapshot a stale `0,0`, because `PTZStateManager` caches its own copy and only `absolute_move`, `relative_move` and `GetStatus` refresh it.

**Files:**
- Modify: `src/onvif/ptz/ops/movement.rs:206` (`stop`)

**Step 1: Write the failing test**

```rust
    #[tokio::test]
    async fn test_stop_refreshes_cached_position_from_platform() {
        let state = create_test_state();
        let mut mock = MockPTZControl::new();
        mock.expect_stop().returning(|| Ok(()));
        mock.expect_get_position().returning(|| Ok(PtzPosition::new(90.0, 45.0, 1.0)));

        let ptz: Option<Arc<dyn PTZControl>> = Some(Arc::new(mock));
        stop(&state, &ptz, "Profile1", true, true).await.unwrap();

        let pan = state.get_position().pan_tilt.expect("pan_tilt present").x;
        assert!(pan.abs() > f32::EPSILON, "cached position still at origin after stop");
    }
```

Check `stop`'s real signature and the normalized-vs-degrees conversion in `sync_position_from_platform` before asserting an exact value — assert "not zero", not "equals 90".

**Step 2: Run it** — Expected: FAIL, cached position still `0`.

**Step 3: Implement** — after the hardware stop succeeds:

```rust
        // Motion has ended and the actor has already committed the integrated position
        // (the Stop batch is only dequeued after do_continuous returns), so this cannot
        // read a half-updated value.
        sync_position_from_platform(state, ptz).await;
```

**Step 4: Verify** — `$CARGO test --target x86_64-unknown-linux-gnu --lib onvif::ptz::`

**Step 5: Commit**

```bash
git commit -am "fix(ptz): refresh cached position when motion stops"
```

---

## Task 10: Delete the unreachable platform preset map

`AnykaPTZControl` holds `presets: RwLock<HashMap<String, PtzPreset>>` and `next_preset_id` that no ONVIF path reaches — `ops/presets.rs:35` says `ptz_control` is "accepted but not yet used". Two preset stores, one dead. Delete it so a later fix cannot land on the wrong one.

The `PTZControl` trait's four preset methods stay (the trait is the platform contract and `StubPTZControl` implements them); only `AnykaPTZControl`'s *storage* goes. Implement its preset methods as `Err(PlatformError::NotSupported("presets are stored in the ONVIF layer".into()))` and delete the fields.

**Step 1:** Confirm nothing calls them: `grep -rn "ptz_control.*set_preset\|ptz.set_preset" src/`. Expect no hits outside `ptz_control.rs` tests.

**Step 2:** Delete the fields, the constructor initializers, and rewrite the four methods.

**Step 3:** Delete the now-meaningless tests in `ptz_control.rs` (`test_set_and_get_preset`, `test_goto_preset_triggers_move`, `test_goto_nonexistent_preset`, `test_remove_preset`, `test_remove_nonexistent_preset`), replacing them with one:

```rust
    #[tokio::test]
    async fn test_platform_presets_are_not_supported() {
        let ptz = create_opened(mock_with_open());
        assert!(matches!(
            ptz.set_preset("x").await,
            Err(PlatformError::NotSupported(_))
        ));
    }
```

**Step 4:** `$CARGO test --target x86_64-unknown-linux-gnu` + clippy.

**Step 5:**

```bash
git commit -am "refactor(ptz): remove the unreachable platform-layer preset store"
```

---

## Task 11: Persist tracked position

**Files:**
- Modify: `src/platform/anyka/ptz_actor.rs`, `src/platform/anyka/ptz_control.rs`, `src/platform/anyka/mod.rs`

Uses `crate::config::file_ops::atomic_write` (already `pub(crate)`, already does temp → `fsync` → rename). Write no file-writing code.

**Step 1: Write the failing tests**

```rust
    #[test]
    fn test_position_file_roundtrips_through_toml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ptz_position.toml");
        save_position(&path, PtzPosition::new(90.0, -45.0, 1.0));
        let loaded = load_position(&path).expect("just-written file must load");
        assert!((loaded.pan - 90.0).abs() < 0.01);
        assert!((loaded.tilt + 45.0).abs() < 0.01);
    }

    #[test]
    fn test_load_position_returns_none_for_missing_file() {
        assert!(load_position(std::path::Path::new("/nonexistent/ptz_position.toml")).is_none());
    }

    #[test]
    fn test_load_position_returns_none_for_corrupt_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ptz_position.toml");
        std::fs::write(&path, b"this is not toml {{{").unwrap();
        assert!(load_position(&path).is_none(), "a corrupt file must never fail boot");
    }

    #[test]
    fn test_load_position_clamps_out_of_range_values() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ptz_position.toml");
        std::fs::write(&path, b"pan = 9999.0\ntilt = -9999.0\n").unwrap();
        let loaded = load_position(&path).expect("out-of-range is clamped, not rejected");
        assert!((loaded.pan - PTZ_MAX_PAN_DEGREES).abs() < f32::EPSILON);
    }
```

**Step 2: Run** — FAIL, functions do not exist.

**Step 3: Implement** in `ptz_actor.rs`:

```rust
/// On-disk form of the tracked position. Degrees, matching `PtzPosition`.
#[derive(Debug, Serialize, Deserialize)]
struct PersistedPosition {
    pan: f32,
    tilt: f32,
}

/// Load a previously saved position, clamped to the travel limits.
///
/// Returns `None` for every failure — missing, unreadable, malformed. A state file
/// must never be able to fail boot.
pub(crate) fn load_position(path: &Path) -> Option<PtzPosition> { /* read_to_string, toml::from_str, clamp */ }

/// Write the tracked position. Failures are logged, never propagated: losing the
/// saved view is not worth failing a move the motor already made.
pub(crate) fn save_position(path: &Path, position: PtzPosition) { /* toml::to_string + atomic_write */ }
```

Give `PtzActorState` a `position_path: Option<PathBuf>`. In `dispatch_batch`, after `commands_completed.fetch_add`:

```rust
    // ponytail: one write per completed command, no debounce — dispatch_batch's
    // supersede semantics already collapse a jog burst into a single command, and this
    // runs on the actor's own OS thread where blocking I/O is free. Add debouncing via
    // config::persistence if write volume ever becomes a problem.
    if let Some(path) = &state.position_path {
        save_position(path, *state.position.read());
    }
```

Thread the path from `AnykaPlatform::with_isp_config` down through `init_ptz_control`, derived from the config path's parent exactly as `app.rs:799-802` derives `profiles.toml`.

**Step 4: Verify** — `$CARGO test --target x86_64-unknown-linux-gnu --lib ptz_actor::`

**Step 5: Commit**

```bash
git commit -am "feat(ptz): persist tracked position across restarts"
```

---

## Task 12: Restore position after calibration

**Files:**
- Modify: `src/platform/anyka/mod.rs` (the async platform init, beside `init_video_input`)

Ordering is load-bearing: the restore read must happen before any motion, and the first save can only follow a completed command. Pin it with a test, because a future "save initial state at startup" change would silently break exactly this.

**Step 1: Write the failing test**

```rust
    #[tokio::test]
    async fn test_startup_restores_saved_position() {
        // Write a ptz_position.toml, construct the control pointing at it,
        // run the restore, assert a move_to_position was issued with those degrees.
    }

    #[tokio::test]
    async fn test_startup_does_not_move_when_saved_position_is_home() {
        // A 0,0 file falls below PTZ_MIN_MOVE_THRESHOLD — expect_ptz_start_turn().never().
    }
```

**Step 2: Run** — FAIL.

**Step 3: Implement** — after `init_ptz_control` reports success, in the async init path:

```rust
        // check_self has just driven the motors to true center, so this dead-reckoned
        // move starts from the most accurate origin available.
        if let Some(saved) = restored_position {
            if let Err(e) = ptz.move_to_position(saved).await {
                tracing::warn!("PTZ position restore failed, staying centered: {}", e);
            }
        }
```

**Step 4: Verify** — full suite.

**Step 5: Commit**

```bash
git commit -am "feat(ptz): restore the saved view after startup calibration"
```

---

## Task 13: Persist presets

`PresetStore` claims persistence in its module doc and delivers a `HashMap`. Copy the `ImagingSettingsStore` shape (`src/onvif/imaging/store.rs:78-201`) exactly — it is the pattern this codebase converged on.

**Files:**
- Modify: `src/onvif/ptz/store.rs`, `src/app.rs`

**Step 1: Write the failing tests**

```rust
    #[test]
    fn test_preset_store_roundtrips_through_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("ptz_presets.toml");
        let store = PresetStore::with_persistence(&path);
        let token = store.set_preset("Front Door".into(), sample_position(), None).unwrap();
        std::fs::write(&path, store.snapshot_toml().unwrap()).unwrap();

        let reloaded = PresetStore::with_persistence(&path);
        reloaded.load_from_file().unwrap();
        assert_eq!(reloaded.get(&token).unwrap().name, Some("Front Door".into()));
    }

    #[test]
    fn test_preset_store_survives_corrupt_file() {
        // Garbage in the file must leave an empty, usable store — never a boot failure.
    }

    #[test]
    fn test_preset_store_preserves_next_token_number_across_reload() {
        // Two presets, reload, add a third: its token must not collide with an existing one.
    }
```

That third test matters — `next_preset_num` resetting to 1 on reload would silently overwrite preset 1.

**Step 2: Run** — FAIL.

**Step 3: Implement** — add `persistence_path: Option<PathBuf>`, `persistence: OnceLock<PersistenceHandle>`, `with_persistence()`, `set_persistence()`, `request_save()`, `persistence_service()`, `load_from_file()`, `snapshot_toml()`. Call `self.request_save()` at the end of `set_preset` and `remove`. Serialize `next_preset_num` alongside the presets.

**Step 4: Wire in `app.rs`** — copy `wire_profile_persistence` (`app.rs:794-830`) into `wire_ptz_preset_persistence`, deriving `ptz_presets.toml` from the config path's parent, loading at startup, spawning the service against `shutdown_coordinator.subscribe()`, and passing the store into `PTZStateManager::with_preset_store`.

**Step 5: Verify** — full suite + clippy + fmt.

**Step 6: Commit**

```bash
git commit -am "feat(ptz): persist presets across restarts"
```

---

## Task 14: Measure the constants and validate on hardware

Everything above runs on placeholder rates. This is where the numbers become real. Requires the camera — see the `anyka-remote-debugging` and `anyka-embedded-build` skills.

**Step 1: Cross-compile and deploy**

```bash
cd cross-compile/onvif-rust   # NOT the workspace root, or cargo
$CARGO build --release                                # silently links the host toolchain
```

Deploy per the `anyka-embedded-build` skill.

**Step 2: Measure `deg/s` per axis**

Issue an absolute move of a known angle and time `start_turn` → `wait_turn` return. The motor runs a commanded step count and the kernel signals completion, so `deg/s = commanded_degrees / elapsed`. Use a 180° pan and a 90° tilt; repeat three times per axis and take the median.

Add temporary `tracing::info!` timing around the calls if the existing debug logs are not precise enough — and remember the shipped log level filters most of them.

**Step 3: Write the measured values** into `PtzConfig::default()` (Task 1's placeholders) and into the deployed `anyka.toml`.

**Step 4: Validate end to end on the camera**

- Jog right ~45°, check `/api/diagnostics` reports a plausible non-zero pan.
- Save preset 1. Jog away. Goto preset 1 — the view must return to roughly where it was.
- `GotoHomePosition` — the motors must audibly sweep to the limit and back, and position must read `0,0`.
- Jog somewhere distinctive, reboot, confirm the camera returns to that view rather than centre.
- Confirm presets survive the reboot.

**Step 5: Commit**

```bash
git commit -am "feat(ptz): use measured axis rates from hardware"
```

---

## Done When

- `$CARGO test --target x86_64-unknown-linux-gnu` passes.
- `$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings` is clean.
- `$CARGO fmt --check` is clean.
- Task 14's five hardware checks pass on the camera.
- The design doc's non-goals are still non-goals — no `remain_steps` probing, no variable speed.
