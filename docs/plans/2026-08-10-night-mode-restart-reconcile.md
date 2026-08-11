# Night-Mode Restart Reconcile Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make AUTO day/night reconcile the vendor daemon's ISP state after a restart, and make the AUTO loop diagnosable from the log alone.

**Architecture:** `AutoState.current` becomes `Option<DayNight>` and starts `None` in AUTO — the controller only claims to know a mode it has driven itself. Because `Some(target) != None`, the first determinate light reading transitions unconditionally, running the full `apply()` and reconciling GPIO and ISP together. The `IR_LED` GPIO seed (`read_initial_state`) is deleted. Alongside, three log points make the loop observable at production log levels.

**Tech Stack:** Rust 2024, `tokio`, `tracing` / `tracing-subscriber` (`EnvFilter` + `reload`), `mockall`, `tempfile`. Vendored ARM toolchain; all tests run host-side on `x86_64-unknown-linux-gnu`.

**Design doc:** `docs/plans/2026-08-10-night-mode-restart-reconcile-design.md`

**Related skills:** @anyka-rust-testing for test conventions, @anyka-embedded-build for cross-compilation and deploy.

---

## Before You Start

Every task below assumes this environment. Run it once per shell:

```bash
cd <repo-root>
source setenv.sh
```

This exports `$CARGO` (the vendored toolchain at
`toolchain/arm-anykav200-crosstool-ng/bin/cargo`) and puts the toolchain bin dir
first on `PATH`. **Clippy fails with `E0514` if you skip this** — the vendored
`cargo-clippy` must win over any rustup one.

All test commands run from `cross-compile/onvif-rust`.

**Initial source files you will touch:**

| Path | Role |
|---|---|
| `platform/anyka/night_mode.rs` | the controller, `decide`, `classify`, `tick`, `apply` — most of the work |
| `platform/anyka/imaging.rs` | one caller of `current_mode()` at line 342 |
| `hal/anyka/ipc/imaging.rs` | `get_ae_luma`'s silent failure arm, line 94 |
| `logging/mod.rs` | `EnvFilter` construction, line 180 |

Tests are inline `#[cfg(test)] mod tests` at the bottom of each file — this
codebase does not use a separate `tests/` tree for unit tests.

---

## Task 1: Widen the state type (pure refactor, no behaviour change)

This task changes types only. Behaviour is identical when it lands: AUTO still
seeds from the `IR_LED` GPIO. Splitting it out keeps the behaviour change in
Task 2 reviewable on its own.

**Files:**
- Modify: `src/platform/anyka/night_mode.rs:67-108` (`AutoState`, `decide`)
- Modify: `src/platform/anyka/night_mode.rs:323-325` (`current_mode`)
- Modify: `src/platform/anyka/imaging.rs:342-345` (the one external caller)

**Step 1: Write the failing test**

Add to the `tests` module in `night_mode.rs`, next to the other `decide` tests:

```rust
#[test]
fn test_decide_transitions_when_the_current_state_is_unknown() {
    // At start-up in AUTO the controller has driven nothing, so it must act on
    // the first determinate reading rather than assume the hardware agrees.
    let t0 = std::time::Instant::now();
    let state = AutoState::new(None);

    let target = decide(&state, Some(DayNight::Night), t0, Duration::from_secs(900));

    assert_eq!(target, Some(DayNight::Night));
}
```

**Step 2: Run it and watch it fail**

```bash
cd cross-compile/onvif-rust
$CARGO test --target x86_64-unknown-linux-gnu --lib \
  test_decide_transitions_when_the_current_state_is_unknown
```

Expected: **compile error**, `expected DayNight, found Option<_>` at
`AutoState::new(None)`. That is the correct failure — the type does not exist yet.

**Step 3: Widen `AutoState`**

Replace the struct and its impl (`night_mode.rs:65-85`):

```rust
/// Current AUTO-mode state: what the camera is set to, and when it last moved.
///
/// `current` is `None` until this controller has driven the hardware itself.
/// GPIO survives a restart and the vendor daemon's ISP state does not, so a
/// mode we merely inferred is not something we may act on. See the design doc.
#[derive(Debug, Clone, Copy)]
pub(super) struct AutoState {
    pub(super) current: Option<DayNight>,
    last_change: Option<std::time::Instant>,
}

impl AutoState {
    pub(super) fn new(current: Option<DayNight>) -> Self {
        Self {
            current,
            last_change: None,
        }
    }

    /// Record that a transition has been applied.
    pub(super) fn record_change(&mut self, to: DayNight, at: std::time::Instant) {
        self.current = Some(to);
        self.last_change = Some(at);
    }
}
```

**Step 4: Update `decide`**

One line changes in the body (`night_mode.rs:99`):

```rust
    if state.current == Some(target) {
        return None;
    }
```

Also extend its doc comment with a sentence:

```rust
/// A `None` `state.current` never matches, so the first determinate reading
/// after start-up always transitions — that is the ISP reconcile.
```

**Step 5: Update `current_mode` and its caller**

In `night_mode.rs`:

```rust
    /// The day/night state the hardware was last driven to, or `None` if this
    /// controller has not driven it yet.
    pub(crate) async fn current_mode(&self) -> Option<DayNight> {
        self.state.lock().await.current
    }
```

In `imaging.rs:342-345`, replace the match:

```rust
        let mode = match self.night.current_mode().await {
            // Unknown means we have never driven the filter, so it sits where a
            // fresh VI leaves it: day. `VI_MODE_DAY` is the zero value of
            // `enum video_daynight_mode` and `handle_vi_open` never switches it.
            Some(DayNight::Day) | None => IrCutFilterMode::ON,
            Some(DayNight::Night) => IrCutFilterMode::OFF,
        };
```

**Step 6: Keep behaviour identical in `new`**

In `NightModeController::new` (`night_mode.rs:300-304`), wrap the existing arms —
do **not** change AUTO yet, that is Task 2:

```rust
        let initial = match mode {
            IrCutFilterMode::OFF => Some(DayNight::Night),
            IrCutFilterMode::ON => Some(DayNight::Day),
            IrCutFilterMode::AUTO => Some(read_initial_state(&paths)),
        };
```

**Step 7: Fix the existing tests the type change breaks**

`test_configured_off_mode_starts_at_night_with_auto_disabled`,
`test_tick_uses_ae_luma_when_available`,
`test_tick_falls_back_to_ain0_after_three_ae_failures`,
`test_tick_holds_when_ae_fails_and_ain0_is_uncalibrated`,
`test_tick_clears_streak_on_ae_success`,
`test_apply_does_not_record_mode_when_gpio_fails` all assert on `current_mode()`.
Wrap each expected value in `Some(...)`:

```rust
        assert_eq!(ctl.current_mode().await, Some(DayNight::Day));
```

The `decide` tests build `AutoState::new(DayNight::Day)` — change to
`AutoState::new(Some(DayNight::Day))`.

**Step 8: Run the whole suite**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --lib
```

Expected: **PASS**, including the new test. If any test now fails on
*behaviour* rather than types, stop — this task is meant to be inert.

**Step 9: Commit**

```bash
rtk git add cross-compile/onvif-rust/src/platform/anyka/night_mode.rs \
            cross-compile/onvif-rust/src/platform/anyka/imaging.rs
rtk git commit -m "refactor(night-mode): make the AUTO state's current mode optional"
```

---

## Task 2: Reconcile the ISP on the first determinate reading

The behaviour change. This is the fix.

**Files:**
- Modify: `src/platform/anyka/night_mode.rs` (`new`, delete `read_initial_state`)

**Step 1: Write the failing test**

This reproduces the `.121` failure exactly: a restart with the lamp already on,
a dark scene, and an ISP that came up in day mode.

```rust
#[tokio::test]
async fn test_auto_syncs_the_isp_after_a_restart_with_the_lamp_already_on() {
    use crate::hal::common::imaging::MockImagingHalTrait;
    use crate::onvif::types::common::IrCutFilterMode;

    // Reproduces 192.168.30.121 on 2026-08-10: onvif-rust restarted with a
    // stale IR_LED=1, the vendor daemon restarted alongside it and came up in
    // day mode, and AUTO concluded "already at night" and never called
    // set_ir_filter. The daemon must be told, every time, on the first
    // determinate reading.
    let dir = tempfile::tempdir().expect("tempdir");
    let paths = NodePaths::rooted(dir.path(), dir.path());
    seed_gpio_nodes(&paths);
    std::fs::write(paths.node(Node::IrLed), "1").unwrap();

    let mut ffi = MockImagingHalTrait::new();
    // Below ae_night_threshold (8): a dark scene, agreeing with the stale lamp.
    ffi.expect_get_ae_luma().times(1).returning(|| Some(1));
    ffi.expect_set_ir_filter()
        .withf(|enabled| *enabled)
        .times(1)
        .returning(|_| 0);

    let ctl = NightModeController::new(
        paths,
        unlocked_config(),
        std::sync::Arc::new(ffi),
        IrCutFilterMode::AUTO,
        None,
    );
    assert_eq!(ctl.current_mode().await, None, "nothing driven yet");

    ctl.tick().await;

    assert_eq!(ctl.current_mode().await, Some(DayNight::Night));
}
```

**Step 2: Run it and watch it fail**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --lib \
  test_auto_syncs_the_isp_after_a_restart_with_the_lamp_already_on
```

Expected: **FAIL** with a `mockall` miss —
`set_ir_filter: Expectation(<anything>) called 0 time(s) which is fewer than
expected 1`. That message *is* the bug: the ISP was never told.

**Step 3: Start AUTO from unknown**

In `NightModeController::new`, replace the AUTO arm and its comment
(`night_mode.rs:297-304`):

```rust
        // AUTO starts unknown: this controller has driven nothing yet. GPIO
        // survives a restart but the vendor daemon's ISP state does not, so
        // inferring a mode from the IR_LED node lets AUTO conclude "already
        // correct" and never issue the ISP switch. The first determinate
        // reading reconciles both together. Forced modes keep their fixed
        // start so ON/OFF always lands where the user asked.
        let initial = match mode {
            IrCutFilterMode::OFF => Some(DayNight::Night),
            IrCutFilterMode::ON => Some(DayNight::Day),
            IrCutFilterMode::AUTO => None,
        };
```

**Step 4: Delete `read_initial_state` and its test**

Remove the whole function (`night_mode.rs:236-249`) and
`test_initial_state_follows_the_ir_led_gpio` from the tests module. Nothing else
references it — confirm:

```bash
grep -rn "read_initial_state" cross-compile/onvif-rust/src/
```

Expected: no output.

**Step 5: Run the new test, then the whole suite**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --lib \
  test_auto_syncs_the_isp_after_a_restart_with_the_lamp_already_on
$CARGO test --target x86_64-unknown-linux-gnu --lib
```

Expected: **PASS** both times.

**Step 6: Commit**

```bash
rtk git add cross-compile/onvif-rust/src/platform/anyka/night_mode.rs
rtk git commit -m "fix(night-mode): reconcile the ISP on the first determinate reading"
```

---

## Task 3: Stop `get_ae_luma` from failing silently

**Files:**
- Modify: `src/hal/anyka/ipc/imaging.rs:90-100`

**Note a correction to the design doc:** D5 says `warn!`. Use `error!` instead —
it matches the sibling `Err(e)` arm two lines down, and cameras run at
`logging.level = "error"`, where a `warn!` would be invisible. Update D5 in the
design doc as part of this task's commit.

**Step 1: Write the failing test**

The test module already has `test_get_ae_luma_error_is_none` and friends using a
fake daemon. Add one asserting on the *status* path:

```rust
    #[tokio::test]
    async fn test_get_ae_luma_non_success_status_is_none() {
        // A daemon STATUS_ERROR used to return None with no log at all, which
        // is what made the 2026-08-10 .121 night-mode failure undiagnosable.
        let (ipc, _srv) = fake_daemon_returning(AK_FAILED_I32, &[0u8]).await;
        assert_eq!(<AnykaIpc as ImagingHalTrait>::get_ae_luma(&ipc).await, None);
    }
```

Match the existing helpers' names and signatures — read
`src/hal/anyka/ipc/test_helpers.rs` and the neighbouring tests first and mirror
whatever they use to stand up a fake daemon response.

**Step 2: Run it**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --lib test_get_ae_luma
```

Expected: PASS already (the arm returns `None` today). This test pins the
contract; the log line is the actual change and is not asserted — see the note
in Task 5 on why we do not test log output.

**Step 3: Add the log**

Replace the `Ok(_)` arm (`imaging.rs:94`):

```rust
            // Not silent: a bad status or a short payload is a real daemon
            // fault, and the caller only sees `None`, which it treats as
            // "hold". Without this line an ISP that never answers looks
            // exactly like a camera that is correctly holding its mode.
            Ok((status, data)) => {
                error!(status, len = data.len(), "get_ae_luma bad daemon response");
                None
            }
```

**Step 4: Update the design doc**

In `docs/plans/2026-08-10-night-mode-restart-reconcile-design.md`, change D5 to:

```markdown
| D5 | `error!` on `get_ae_luma`'s silent non-success arm, matching the sibling `Err` arm and surviving `level = "error"` |
```

**Step 5: Run the suite and commit**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --lib
rtk git add cross-compile/onvif-rust/src/hal/anyka/ipc/imaging.rs docs/plans/
rtk git commit -m "fix(hal): log the silent get_ae_luma failure arm"
```

---

## Task 4: Rate-limited AUTO sample logging

Every tick is 10s, so logging every sample is 8,640 lines/day onto an SD card.
Log on a classification change, otherwise at most every 10 minutes.

**Files:**
- Modify: `src/platform/anyka/night_mode.rs` (new constant, field, helper, `tick`)

**Step 1: Write the failing test**

The rate-limit decision is extracted as a pure function precisely so it can be
tested without standing up a `tracing` subscriber:

```rust
    #[test]
    fn test_sample_due_on_first_call() {
        let t0 = std::time::Instant::now();
        assert!(sample_due(None, Some(DayNight::Day), t0, Duration::from_secs(600)));
    }

    #[test]
    fn test_sample_due_when_the_classification_changes() {
        let t0 = std::time::Instant::now();
        let last = Some((t0, Some(DayNight::Day)));

        // One second later, but the reading flipped: log it immediately.
        assert!(sample_due(
            last,
            Some(DayNight::Night),
            t0 + Duration::from_secs(1),
            Duration::from_secs(600),
        ));
    }

    #[test]
    fn test_sample_suppressed_inside_the_window_when_unchanged() {
        let t0 = std::time::Instant::now();
        let last = Some((t0, Some(DayNight::Day)));

        assert!(!sample_due(
            last,
            Some(DayNight::Day),
            t0 + Duration::from_secs(599),
            Duration::from_secs(600),
        ));
    }

    #[test]
    fn test_sample_due_again_once_the_window_expires() {
        let t0 = std::time::Instant::now();
        let last = Some((t0, Some(DayNight::Day)));

        assert!(sample_due(
            last,
            Some(DayNight::Day),
            t0 + Duration::from_secs(600),
            Duration::from_secs(600),
        ));
    }
```

**Step 2: Run and watch them fail**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --lib test_sample_
```

Expected: **compile error**, `cannot find function sample_due in this scope`.

**Step 3: Add the constant and the pure function**

Near `POLL_INTERVAL` (`night_mode.rs:256`):

```rust
/// Heartbeat cadence for the AUTO sample line when nothing is changing.
///
/// The poll is every 10s; logging each one is ~8,600 lines a day onto an SD
/// card. A change always logs immediately, so this only bounds the quiet case.
const SAMPLE_LOG_INTERVAL: Duration = Duration::from_secs(600);

/// Whether a sample line is due: always on a classification change, otherwise
/// at most once per `interval`.
fn sample_due(
    last: Option<(std::time::Instant, Option<DayNight>)>,
    class: Option<DayNight>,
    now: std::time::Instant,
    interval: Duration,
) -> bool {
    match last {
        None => true,
        Some((at, prev)) => prev != class || now.duration_since(at) >= interval,
    }
}
```

**Step 4: Add the field**

On `NightModeController`:

```rust
    /// When the last sample line was logged, and what it classified to.
    /// `None` = nothing logged yet. Guards [`SAMPLE_LOG_INTERVAL`].
    sample_log: tokio::sync::Mutex<Option<(std::time::Instant, Option<DayNight>)>>,
```

Initialise it in `new`:

```rust
            sample_log: tokio::sync::Mutex::new(None),
```

**Step 5: Add the logging helper**

```rust
    /// Log one AUTO sample, rate-limited by [`sample_due`].
    ///
    /// `info!`, not `debug!`: cameras run at `level = "error"` and a day/night
    /// failure is only diagnosable after the fact. See the filter directive in
    /// `logging::init_logging_impl`.
    async fn log_sample(&self, raw: i32, src: &'static str, class: Option<DayNight>) {
        let now = std::time::Instant::now();
        let mut last = self.sample_log.lock().await;
        if !sample_due(*last, class, now, SAMPLE_LOG_INTERVAL) {
            return;
        }
        *last = Some((now, class));
        match class {
            Some(mode) => tracing::info!(raw, src, ?mode, "night sample"),
            None => tracing::info!(raw, src, "night sample: indeterminate, holding"),
        }
    }
```

**Step 6: Restructure `tick` to produce `(raw, src, reading)`**

Replace the body of `tick` (`night_mode.rs:416-476`) up to the `decide` call.
The classification logic is unchanged — it is only reshaped so the raw value and
its source survive to the log call:

```rust
        let (raw, src, reading) = match self.ffi.get_ae_luma().await {
            Some(luma) => {
                self.ae_fail_streak.store(0, Ordering::SeqCst);
                let raw = i32::from(luma);
                let class = classify(
                    raw,
                    Thresholds {
                        day: self.cfg.ae_day_threshold,
                        night: self.cfg.ae_night_threshold,
                        // AE high = bright = day.
                        ldr_high_is_day: true,
                    },
                );
                (raw, "ae", class)
            }
            None => {
                let n = self.ae_fail_streak.fetch_add(1, Ordering::SeqCst) + 1;
                if n < AE_FAIL_STREAK_MAX {
                    return;
                }
                if n == AE_FAIL_STREAK_MAX {
                    tracing::warn!(streak = n, "AE luma unavailable; falling back to ain0");
                }
                let (Some(day), Some(night)) = (self.cfg.day_threshold, self.cfg.night_threshold)
                else {
                    return;
                };
                let Some(raw) = read_light_sensor(&self.paths) else {
                    return;
                };
                let class = classify(
                    raw,
                    Thresholds {
                        day,
                        night,
                        ldr_high_is_day: self.cfg.ldr_high_is_day,
                    },
                );
                (raw, "ain0", class)
            }
        };

        self.log_sample(raw, src, reading).await;
```

The rest of `tick` (the `decide` block and the `apply` call) is unchanged.

**Step 7: Run and commit**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --lib
rtk git add cross-compile/onvif-rust/src/platform/anyka/night_mode.rs
rtk git commit -m "feat(night-mode): log AUTO samples on change and every 10 minutes"
```

---

## Task 5: Log every apply, not just failures

Today a working ISP switch is indistinguishable in the log from one that never
ran — the only `set_ir_filter` line is a `warn!` on failure.

**Files:**
- Modify: `src/platform/anyka/night_mode.rs:406-412` (end of `apply`)

**Step 1: Add the line**

Immediately before `state.record_change(...)`:

```rust
        // Logged unconditionally: a silent success and a transition that never
        // happened look identical otherwise, which is what hid the 2026-08-10
        // failure. `isp` is the daemon's ak_vi_switch_mode return.
        tracing::info!(from = ?state.current, to = ?target, isp, "night mode applied");
        state.record_change(target, std::time::Instant::now());
```

`isp` is already in scope from the `set_ir_filter` call above. Keep the existing
`warn!` on `isp != 0` — an anomaly deserves its own level.

**Step 2: Run the suite**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --lib
```

Expected: **PASS**, unchanged. This is a log line only.

**Step 3: Commit**

```bash
rtk git add cross-compile/onvif-rust/src/platform/anyka/night_mode.rs
rtk git commit -m "feat(night-mode): log every apply with its ISP return code"
```

---

## Task 6: Warn once when AUTO cannot sync

Closes the one hole in the design: if the reading is indeterminate from boot,
nothing is applied and both actuators stay stale. That is the signature of
miscalibrated thresholds, so say so.

**Files:**
- Modify: `src/platform/anyka/night_mode.rs` (field, helper, `tick`)

**Step 1: Add the field**

```rust
    /// Whether the "never synced" warning has already fired this process.
    unsynced_warned: std::sync::atomic::AtomicBool,
```

Initialise in `new`:

```rust
            unsynced_warned: std::sync::atomic::AtomicBool::new(false),
```

**Step 2: Add the helper**

```rust
    /// Warn once per process when AUTO has driven nothing and the sensor will
    /// not say which way to go.
    ///
    /// This is what miscalibrated thresholds look like from the outside: the
    /// ISP is still at its power-on day mode, the lamp is wherever it was, and
    /// no reading will ever break the tie.
    fn warn_unsynced_once(&self, raw: i32, src: &'static str) {
        use std::sync::atomic::Ordering;
        if self.unsynced_warned.swap(true, Ordering::SeqCst) {
            return;
        }
        tracing::warn!(
            raw,
            src,
            "night mode has never driven the hardware and the reading is indeterminate; check thresholds"
        );
    }
```

**Step 3: Call it from `tick`**

Replace the `decide` block so it also reports whether the state is still unknown:

```rust
        let (target, unsynced) = {
            let state = self.state.lock().await;
            let target = decide(
                &state,
                reading,
                std::time::Instant::now(),
                Duration::from_millis(self.cfg.lock_time_ms),
            );
            (target, state.current.is_none())
        };

        if unsynced && reading.is_none() {
            self.warn_unsynced_once(raw, src);
        }
```

**Step 4: Run, lint, commit**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --lib
rtk git add cross-compile/onvif-rust/src/platform/anyka/night_mode.rs
rtk git commit -m "feat(night-mode): warn once when AUTO cannot sync the hardware"
```

---

## Task 7: Keep night-mode logs visible at `level = "error"`

Without this the previous three tasks produce nothing on a real camera.

**Files:**
- Modify: `src/logging/mod.rs:179-182`

**Step 1: Add the directive**

```rust
    // Create env filter with default level
    let mut env_filter = EnvFilter::builder()
        .with_default_directive(level.into())
        .from_env_lossy();

    // Night-mode AUTO diagnostics stay visible whatever `logging.level` says.
    // Cameras run at "error" in production and a day/night failure is only
    // diagnosable after the fact, from these lines. `if let` rather than
    // `expect`: a bad directive must not take the process down over logging.
    if let Ok(directive) = "onvif_rust::platform::anyka::night_mode=info".parse() {
        env_filter = env_filter.add_directive(directive);
    }
```

`tracing` defaults a macro's target to its module path, so this covers every
`info!`/`warn!` added in Tasks 4–6 and nothing else.

**Step 2: Verify it compiles and the suite passes**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --lib
```

**Step 3: Commit**

```bash
rtk git add cross-compile/onvif-rust/src/logging/mod.rs
rtk git commit -m "feat(logging): pin night-mode diagnostics to info regardless of level"
```

---

## Task 8: Quality gate

**Step 1: Format, lint, full test run**

```bash
cd cross-compile/onvif-rust
$CARGO fmt
$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
$CARGO test --target x86_64-unknown-linux-gnu
```

All three must be clean. If clippy dies with `E0514`, you did not
`source setenv.sh` — see **Before You Start**.

**Step 2: Cross-compile for the camera**

```bash
$CARGO build --release
```

Expected: a binary at
`target/armv5te-unknown-linux-uclibceabi/release/onvif-rust`.

**Step 3: Commit any formatting churn**

```bash
rtk git add -A cross-compile/onvif-rust
rtk git commit -m "style(onvif-rust): cargo fmt"
```

---

## Task 9: Verify on hardware

Do this on `.198` first — it is the calibrated board with local access. Only
then `.121`. See @anyka-embedded-build for deploy and @anyka-remote-debugging
for the shell.

**Step 1: Reproduce the bug against the old binary on `.198`**

```bash
scripts/debugging/cam_exec.py --host 192.168.2.198 \
  'cat /sys/user-gpio/IR_LED' \
  'killall onvif-rust.bin'
```

With the *pre-fix* binary, a restart while `IR_LED=1` in a dark room produces no
`set_ir_filter` at all. Confirm that first, so you know the fix changed something.

**Step 2: Deploy the new binary and repeat**

Expected in `/mnt/logs/onvif-debug.log.*` within one poll interval:

```
INFO ... night sample raw=1 src="ae" mode=Night
INFO ... night mode applied from=None to=Night isp=0
```

**`isp=0` is the assertion that matters.** The wiki
(`wiki/IR-Night-Mode-Calibration.md:166`) records a stale `isp=-1` caveat that
predates the `isp_first_vi` fix in the daemon. If you see `isp=-1`, stop and
raise it — the reconcile is landing but the daemon is not acting on it, and that
is a separate defect.

**Step 3: Confirm visibility at production log level**

Set `logging.level = "error"` in the device `config.toml`, restart, and confirm
the `night sample` and `night mode applied` lines still appear. If they do not,
Task 7 did not work.

**Step 4: Confirm the heartbeat is quiet**

Leave it an hour in steady light. Expect ~6 `night sample` lines, not ~360.

**Step 5: Deploy to `.121` and watch one dusk**

```bash
ssh -f -N -L 2324:192.168.30.121:24 root@192.168.3.137
uv run scripts/debugging/cam_exec.py --host 127.0.0.1 --port 2324 \
  'grep "night " /mnt/logs/onvif-debug.log.$(date +%Y-%m-%d) | tail -40'
```

The `raw=` values across a natural day/night cycle are the input data for the
deferred P2/P3 calibration work — keep them.

**Step 6: Update the wiki**

`wiki/IR-Night-Mode-Calibration.md:57` tells the operator to patch source with a
`tracing::info!` to read luma. That is now false. Replace that paragraph with a
pointer to the `night sample` line, and remove the stale `isp=-1` caveat at line
166 if Step 2 showed `isp=0`.

```bash
rtk git add wiki/IR-Night-Mode-Calibration.md
rtk git commit -m "docs(wiki): calibrate from the night sample log, not a source patch"
```

---

## Done When

- [ ] `test_auto_syncs_the_isp_after_a_restart_with_the_lamp_already_on` passes, and failed before Task 2
- [ ] `read_initial_state` is gone; `grep` finds no references
- [ ] `fmt`, `clippy -D warnings`, and the full suite are clean
- [ ] ARM release binary builds
- [ ] `.198` logs `night mode applied from=None to=Night isp=0` after a restart with a stale lamp
- [ ] Those lines appear with `logging.level = "error"`
- [ ] Steady-state heartbeat is ~6 lines/hour, not ~360
- [ ] Wiki no longer tells operators to patch source to calibrate

## Not In This Plan

Tracked in the design doc's Out of Scope, from the same investigation:
**P2** `.121` `ain0` calibration · **P3** `.121` LDR polarity ·
**P5** deployed configs carrying another board's thresholds ·
**P6** the venc unsafe-teardown restart loop.
