# IR Cut Filter and LED Illumination Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Wire the already-plumbed ONVIF IR cut filter and IR/white illuminator state through to the real GPIO nodes on the AK3918, including an AUTO day/night mode driven by the board's light sensor.

**Architecture:** One new Rust module, `platform/anyka/night_mode.rs`, splits a pure `plan(target) -> Vec<Step>` planner from an `execute(steps)` writer. Writes go straight to `/sys/user-gpio/*` with `std::fs::write`; only the ISP day/night switch crosses IPC to the vendor daemon. Every other change is an edit to an existing hollow hook.

**Tech Stack:** Rust (vendored `arm-anykav200` toolchain), `tokio`, `mockall`, `tempfile`; React 19 + Vitest + MSW for the WebUI.

**Design doc:** `docs/plans/2026-08-02-ir-led-support-design.md` — read it first. Hardware facts H1-H6 there are load-bearing and contradict the vendor reference sources.

---

## Before you start

**Branch:**

```bash
cd /home/kmk/dev/anyka-dev
git checkout -b feat/ir-led-support
```

**Toolchain — the system `cargo` will fail with version errors. Always use:**

```bash
source ./setenv.sh              # sets $CARGO
cd cross-compile/onvif-rust
$CARGO test --target x86_64-unknown-linux-gnu --lib
$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
$CARGO fmt
```

`clippy` needs the toolchain `bin` directory first on `PATH` or it dies with `E0514`. `setenv.sh` handles this; if you invoke the binary directly, prefix `PATH` yourself.

**WebUI:**

```bash
cd cross-compile/www
npm run test
npm run lint
npm run type-check
```

**Three facts that will otherwise waste your time:**

1. The GPIO nodes on this board are `ircut_a`, `ircut_b`, `IR_LED`, `WHITE_LED` — **not** the `gpio-` prefixed names in `anyka_reference/`. See design H1.
2. `/sys/user-gpio/gpio-rf_feed` does not exist here. The light sensor is `/sys/kernel/ain/ain0`. See design H2.
3. The vendor's default thresholds return `-1` at this board's actual reading. Do not copy them as working values. See design H5. Task 14 measures the real ones.

---

## Task 1: Pure transition planner

The ordering of GPIO writes relative to the ISP switch is the whole safety story: the lamp must be on before the ISP goes to night, and in two-line mode both coil lines must return to `0`. Making the plan a pure value means these are `assert_eq!` on a `Vec`, with no filesystem and no mocks.

**Files:**
- Create: `cross-compile/onvif-rust/src/platform/anyka/night_mode.rs`
- Modify: `cross-compile/onvif-rust/src/platform/anyka/mod.rs` (add `mod night_mode;`)

**Step 1: Write the failing test**

Create the file with only the test module and the type declarations it needs:

```rust
//! Night-mode control: IR cut filter, IR illuminator, white floodlight.
//!
//! The write ordering and the two-line coil-idle pulse are taken from the
//! vendor reference (`anyka_reference/libre_anyka_app/main.c:740-752` and
//! `platform/libplat/src/drv/ak_drv_ir.c:230-255`) and are load-bearing.
//! See `docs/plans/2026-08-02-ir-led-support-design.md`.

use std::time::Duration;

/// Delay after a mode switch, letting the solenoid and ISP settle.
pub(super) const SETTLE: Duration = Duration::from_millis(300);

/// Width of the two-line coil pulse before both lines return to idle.
const PULSE: Duration = Duration::from_millis(10);

/// Day or night target state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DayNight {
    Day,
    Night,
}

/// A GPIO node this module drives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Node {
    IrCutA,
    IrCutB,
    IrLed,
    WhiteLed,
}

/// How many lines the IR cut filter solenoid is wired with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LineMode {
    /// Single node, level written directly.
    One,
    /// H-bridge: opposed pulse, then both lines back to zero.
    Two,
}

/// One step of a transition plan.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Step {
    Write { node: Node, value: u8 },
    /// Cross the IPC boundary to `ak_vi_switch_mode`.
    IspMode(DayNight),
    Sleep(Duration),
}

/// Board wiring polarity.
#[derive(Debug, Clone, Copy)]
pub(super) struct Polarity {
    /// `true` when writing `1` to the ircut node selects night.
    pub ircut_high_is_night: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pol() -> Polarity {
        Polarity {
            ircut_high_is_night: true,
        }
    }

    #[test]
    fn test_night_mode_plan_lights_lamp_before_isp_switch() {
        let steps = plan(DayNight::Night, pol(), LineMode::Two);

        assert_eq!(
            steps,
            vec![
                Step::Write {
                    node: Node::IrLed,
                    value: 1
                },
                Step::IspMode(DayNight::Night),
                Step::Write {
                    node: Node::IrCutA,
                    value: 1
                },
                Step::Write {
                    node: Node::IrCutB,
                    value: 0
                },
                Step::Sleep(PULSE),
                Step::Write {
                    node: Node::IrCutA,
                    value: 0
                },
                Step::Write {
                    node: Node::IrCutB,
                    value: 0
                },
                Step::Sleep(SETTLE),
            ]
        );
    }

    #[test]
    fn test_day_mode_plan_moves_filter_before_isp_switch() {
        let steps = plan(DayNight::Day, pol(), LineMode::Two);

        assert_eq!(
            steps,
            vec![
                Step::Write {
                    node: Node::IrCutA,
                    value: 0
                },
                Step::Write {
                    node: Node::IrCutB,
                    value: 1
                },
                Step::Sleep(PULSE),
                Step::Write {
                    node: Node::IrCutA,
                    value: 0
                },
                Step::Write {
                    node: Node::IrCutB,
                    value: 0
                },
                Step::IspMode(DayNight::Day),
                Step::Write {
                    node: Node::IrLed,
                    value: 0
                },
                Step::Sleep(SETTLE),
            ]
        );
    }

    #[test]
    fn test_one_line_mode_writes_single_node_without_pulse() {
        let steps = plan(DayNight::Night, pol(), LineMode::One);

        assert_eq!(
            steps,
            vec![
                Step::Write {
                    node: Node::IrLed,
                    value: 1
                },
                Step::IspMode(DayNight::Night),
                Step::Write {
                    node: Node::IrCutA,
                    value: 1
                },
                Step::Sleep(SETTLE),
            ]
        );
    }

    #[test]
    fn test_inverted_polarity_flips_only_the_ircut_step() {
        let inverted = Polarity {
            ircut_high_is_night: false,
        };

        let steps = plan(DayNight::Night, inverted, LineMode::One);

        assert_eq!(
            steps,
            vec![
                Step::Write {
                    node: Node::IrLed,
                    value: 1
                },
                Step::IspMode(DayNight::Night),
                Step::Write {
                    node: Node::IrCutA,
                    value: 0
                },
                Step::Sleep(SETTLE),
            ]
        );
    }
}
```

Add to `src/platform/anyka/mod.rs`, next to the other `mod` declarations:

```rust
mod night_mode;
```

**Step 2: Run the tests to verify they fail**

```bash
source ./setenv.sh && cd cross-compile/onvif-rust
$CARGO test --target x86_64-unknown-linux-gnu --lib night_mode
```

Expected: FAIL, `cannot find function 'plan' in this scope`.

**Step 3: Write the minimal implementation**

Insert above the `#[cfg(test)]` module:

```rust
/// Build the ordered step list for a transition.
///
/// Ordering follows the vendor reference and is not arbitrary: the lamp turns
/// on before the ISP switches to night and off after it switches to day, so no
/// frame is captured dark. In [`LineMode::Two`] the trailing zero writes
/// de-energise the solenoid coil and are mandatory.
pub(super) fn plan(target: DayNight, pol: Polarity, line_mode: LineMode) -> Vec<Step> {
    let night_level = u8::from(pol.ircut_high_is_night);
    let ircut_level = match target {
        DayNight::Night => night_level,
        DayNight::Day => 1 - night_level,
    };

    let mut ircut = Vec::new();
    match line_mode {
        LineMode::One => ircut.push(Step::Write {
            node: Node::IrCutA,
            value: ircut_level,
        }),
        LineMode::Two => {
            ircut.push(Step::Write {
                node: Node::IrCutA,
                value: ircut_level,
            });
            ircut.push(Step::Write {
                node: Node::IrCutB,
                value: 1 - ircut_level,
            });
            ircut.push(Step::Sleep(PULSE));
            ircut.push(Step::Write {
                node: Node::IrCutA,
                value: 0,
            });
            ircut.push(Step::Write {
                node: Node::IrCutB,
                value: 0,
            });
        }
    }

    let mut steps = Vec::new();
    match target {
        DayNight::Night => {
            steps.push(Step::Write {
                node: Node::IrLed,
                value: 1,
            });
            steps.push(Step::IspMode(DayNight::Night));
            steps.extend(ircut);
        }
        DayNight::Day => {
            steps.extend(ircut);
            steps.push(Step::IspMode(DayNight::Day));
            steps.push(Step::Write {
                node: Node::IrLed,
                value: 0,
            });
        }
    }
    steps.push(Step::Sleep(SETTLE));
    steps
}
```

**Step 4: Run the tests to verify they pass**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --lib night_mode
```

Expected: PASS, 4 tests.

**Step 5: Commit**

```bash
git add cross-compile/onvif-rust/src/platform/anyka/night_mode.rs \
        cross-compile/onvif-rust/src/platform/anyka/mod.rs
git commit -m "$(cat <<'EOF'
feat(imaging): pure night-mode transition planner

Ordering and the two-line coil-idle pulse come from the vendor reference
and are asserted as a whole Vec, so a reordering breaks a test rather
than a solenoid.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 2: Light-sensor classification

`ain0` is a raw ADC reading. The gap between the two thresholds is deliberate hysteresis, and a reading inside it is *indeterminate* — not "day". The vendor's version of this function silently returns `-1` in that band and its callers never check (design H5).

**Files:**
- Modify: `cross-compile/onvif-rust/src/platform/anyka/night_mode.rs`

**Step 1: Write the failing test**

Add to `mod tests`:

```rust
    fn thr() -> Thresholds {
        Thresholds {
            day: 1100,
            night: 300,
            ldr_high_is_day: true,
        }
    }

    #[test]
    fn test_classify_bright_reading_is_day() {
        assert_eq!(classify(1500, thr()), Reading::Settled(DayNight::Day));
    }

    #[test]
    fn test_classify_dark_reading_is_night() {
        assert_eq!(classify(120, thr()), Reading::Settled(DayNight::Night));
    }

    #[test]
    fn test_classify_reading_in_hysteresis_band_is_indeterminate() {
        // 306 is this board's captured ain0 value and sits in the vendor's
        // unhandled 300..1100 dead zone. See design H5.
        assert_eq!(classify(306, thr()), Reading::Indeterminate);
    }

    #[test]
    fn test_classify_respects_inverted_sensor_polarity() {
        let inverted = Thresholds {
            ldr_high_is_day: false,
            ..thr()
        };

        assert_eq!(classify(1500, inverted), Reading::Settled(DayNight::Night));
        assert_eq!(classify(120, inverted), Reading::Settled(DayNight::Day));
    }
```

**Step 2: Run to verify it fails**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --lib night_mode
```

Expected: FAIL, `cannot find type 'Thresholds'`.

**Step 3: Write the implementation**

```rust
/// Light-sensor thresholds. Both values must be calibrated per board:
/// the vendor defaults leave this camera's resting reading in an
/// unhandled dead zone.
#[derive(Debug, Clone, Copy)]
pub(super) struct Thresholds {
    /// At or above this raw reading, the sensor is saturated one way.
    pub day: i32,
    /// At or below this raw reading, the sensor is saturated the other way.
    pub night: i32,
    /// `true` when a high raw reading means daylight.
    pub ldr_high_is_day: bool,
}

/// Result of classifying one sensor reading.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Reading {
    Settled(DayNight),
    /// Inside the hysteresis band; the caller must hold its current mode.
    Indeterminate,
}

/// Map a raw `ain0` reading to a day/night conclusion.
pub(super) fn classify(raw: i32, thr: Thresholds) -> Reading {
    let (high, low) = if thr.ldr_high_is_day {
        (DayNight::Day, DayNight::Night)
    } else {
        (DayNight::Night, DayNight::Day)
    };

    if raw >= thr.day {
        Reading::Settled(high)
    } else if raw <= thr.night {
        Reading::Settled(low)
    } else {
        Reading::Indeterminate
    }
}
```

**Step 4: Run to verify it passes**

Expected: PASS, 8 tests.

**Step 5: Commit**

```bash
git add cross-compile/onvif-rust/src/platform/anyka/night_mode.rs
git commit -m "$(cat <<'EOF'
feat(imaging): classify ain0 readings with an explicit dead zone

The band between the thresholds is hysteresis, and a reading inside it
is Indeterminate rather than day. The vendor returns -1 here and its
callers never check.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 3: Lock-time state machine

Without this, a camera at dusk oscillates between day and night for an hour. The lock is what the vendor's `lock_time = 900000` in `[autoir]` is for.

**Files:**
- Modify: `cross-compile/onvif-rust/src/platform/anyka/night_mode.rs`

**Step 1: Write the failing test**

`Instant` cannot be constructed at an arbitrary value, so the test threads a base instant through and offsets from it. That is the standard trick here and needs no clock abstraction.

```rust
    #[test]
    fn test_decide_switches_when_reading_differs_and_unlocked() {
        let t0 = std::time::Instant::now();
        let state = AutoState::new(DayNight::Day);

        let target = decide(
            &state,
            Reading::Settled(DayNight::Night),
            t0,
            Duration::from_secs(900),
        );

        assert_eq!(target, Some(DayNight::Night));
    }

    #[test]
    fn test_decide_holds_when_reading_matches_current_mode() {
        let t0 = std::time::Instant::now();
        let state = AutoState::new(DayNight::Day);

        let target = decide(
            &state,
            Reading::Settled(DayNight::Day),
            t0,
            Duration::from_secs(900),
        );

        assert_eq!(target, None);
    }

    #[test]
    fn test_decide_holds_current_mode_on_indeterminate_reading() {
        let t0 = std::time::Instant::now();
        let state = AutoState::new(DayNight::Day);

        let target = decide(&state, Reading::Indeterminate, t0, Duration::from_secs(900));

        assert_eq!(target, None);
    }

    #[test]
    fn test_decide_refuses_to_switch_inside_the_lock_window() {
        let t0 = std::time::Instant::now();
        let mut state = AutoState::new(DayNight::Day);
        state.record_change(DayNight::Night, t0);

        // One second later, the sensor says day again — a dusk flicker.
        let target = decide(
            &state,
            Reading::Settled(DayNight::Day),
            t0 + Duration::from_secs(1),
            Duration::from_secs(900),
        );

        assert_eq!(target, None);
    }

    #[test]
    fn test_decide_switches_again_once_the_lock_expires() {
        let t0 = std::time::Instant::now();
        let mut state = AutoState::new(DayNight::Day);
        state.record_change(DayNight::Night, t0);

        let target = decide(
            &state,
            Reading::Settled(DayNight::Day),
            t0 + Duration::from_secs(901),
            Duration::from_secs(900),
        );

        assert_eq!(target, Some(DayNight::Day));
    }
```

**Step 2: Run to verify it fails**

Expected: FAIL, `cannot find type 'AutoState'`.

**Step 3: Write the implementation**

```rust
/// Current AUTO-mode state: what the camera is set to, and when it last moved.
#[derive(Debug, Clone, Copy)]
pub(super) struct AutoState {
    current: DayNight,
    last_change: Option<std::time::Instant>,
}

impl AutoState {
    pub(super) fn new(current: DayNight) -> Self {
        Self {
            current,
            last_change: None,
        }
    }

    pub(super) fn current(&self) -> DayNight {
        self.current
    }

    /// Record that a transition has been applied.
    pub(super) fn record_change(&mut self, to: DayNight, at: std::time::Instant) {
        self.current = to;
        self.last_change = Some(at);
    }
}

/// Decide whether to transition, given a reading and the lock window.
///
/// Returns `None` to hold the current mode. An `Indeterminate` reading always
/// holds; so does any reading inside `lock` of the last transition, which is
/// what stops a camera oscillating at dusk.
pub(super) fn decide(
    state: &AutoState,
    reading: Reading,
    now: std::time::Instant,
    lock: Duration,
) -> Option<DayNight> {
    let Reading::Settled(target) = reading else {
        return None;
    };
    if target == state.current {
        return None;
    }
    if let Some(last) = state.last_change {
        if now.duration_since(last) < lock {
            return None;
        }
    }
    Some(target)
}
```

**Step 4: Run to verify it passes**

Expected: PASS, 13 tests.

**Step 5: Commit**

```bash
git add cross-compile/onvif-rust/src/platform/anyka/night_mode.rs
git commit -m "$(cat <<'EOF'
feat(imaging): lock-time state machine for AUTO day/night

Holds the current mode on indeterminate readings and inside the lock
window, so a dusk flicker produces one transition rather than an hour of
oscillation.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 4: Node paths and capability probe

The paths must be injectable so the executor can be tested against a `tempdir`. Two base directories cover all six nodes, following the `FsLayout` pattern in `anyka-init/src/wifi.rs:334`.

The same `stat()` that picks one-line from two-line also answers whether the hardware exists at all, which replaces the hardcoded `true` at `platform/common/traits.rs:324-325` in Task 9.

**Files:**
- Modify: `cross-compile/onvif-rust/src/platform/anyka/night_mode.rs`
- Modify: `cross-compile/onvif-rust/Cargo.toml` (add `tempfile` to `[dev-dependencies]` if absent)

**Step 1: Check whether `tempfile` is already a dev-dependency**

```bash
grep -n "tempfile" cross-compile/onvif-rust/Cargo.toml
```

If absent, add under `[dev-dependencies]`:

```toml
tempfile = "3"
```

**Step 2: Write the failing test**

```rust
    #[test]
    fn test_probe_reports_two_line_when_both_ircut_nodes_exist() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = NodePaths::rooted(dir.path(), dir.path());
        std::fs::write(paths.node(Node::IrCutA), "0").unwrap();
        std::fs::write(paths.node(Node::IrCutB), "0").unwrap();
        std::fs::write(paths.node(Node::IrLed), "0").unwrap();

        let caps = probe(&paths);

        assert_eq!(caps.line_mode, Some(LineMode::Two));
        assert!(caps.ir_led);
    }

    #[test]
    fn test_probe_reports_one_line_when_only_ircut_a_exists() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = NodePaths::rooted(dir.path(), dir.path());
        std::fs::write(paths.node(Node::IrCutA), "0").unwrap();

        let caps = probe(&paths);

        assert_eq!(caps.line_mode, Some(LineMode::One));
        assert!(!caps.ir_led);
    }

    #[test]
    fn test_probe_reports_unsupported_when_no_ircut_nodes_exist() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = NodePaths::rooted(dir.path(), dir.path());

        let caps = probe(&paths);

        assert_eq!(caps.line_mode, None);
    }
```

**Step 3: Run to verify it fails**

Expected: FAIL, `cannot find type 'NodePaths'`.

**Step 4: Write the implementation**

```rust
use std::path::{Path, PathBuf};

/// Filesystem locations of the nodes this module drives.
///
/// Held as two base directories rather than six paths so tests can point the
/// whole set at a `tempdir` with one call.
#[derive(Debug, Clone)]
pub(super) struct NodePaths {
    user_gpio: PathBuf,
    kernel_ain: PathBuf,
}

impl Default for NodePaths {
    fn default() -> Self {
        Self {
            user_gpio: PathBuf::from("/sys/user-gpio"),
            kernel_ain: PathBuf::from("/sys/kernel/ain"),
        }
    }
}

impl NodePaths {
    /// Point both trees at explicit roots. Tests pass a `tempdir`.
    pub(super) fn rooted(user_gpio: &Path, kernel_ain: &Path) -> Self {
        Self {
            user_gpio: user_gpio.to_path_buf(),
            kernel_ain: kernel_ain.to_path_buf(),
        }
    }

    /// Path of a GPIO node. Names are this board's, not the vendor
    /// reference's `gpio-` prefixed ones. See design H1.
    pub(super) fn node(&self, node: Node) -> PathBuf {
        let name = match node {
            Node::IrCutA => "ircut_a",
            Node::IrCutB => "ircut_b",
            Node::IrLed => "IR_LED",
            Node::WhiteLed => "WHITE_LED",
        };
        self.user_gpio.join(name)
    }

    /// Path of the light sensor. `gpio-rf_feed` does not exist on this
    /// board, so the ain fallback is the only source. See design H2.
    pub(super) fn light_sensor(&self) -> PathBuf {
        self.kernel_ain.join("ain0")
    }
}

/// What the hardware actually supports, discovered at startup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Capabilities {
    /// `None` when no ircut node exists — the filter is unsupported.
    pub line_mode: Option<LineMode>,
    pub ir_led: bool,
    pub white_led: bool,
}

/// Probe for the nodes. One `stat()` per node answers both the wiring
/// question and the ONVIF capability question.
pub(super) fn probe(paths: &NodePaths) -> Capabilities {
    let a = paths.node(Node::IrCutA).exists();
    let b = paths.node(Node::IrCutB).exists();

    let line_mode = match (a, b) {
        (true, true) => Some(LineMode::Two),
        (true, false) | (false, true) => Some(LineMode::One),
        (false, false) => None,
    };

    Capabilities {
        line_mode,
        ir_led: paths.node(Node::IrLed).exists(),
        white_led: paths.node(Node::WhiteLed).exists(),
    }
}
```

Note: `probe` picks `LineMode::One` when only `ircut_b` exists, but `plan` always writes `Node::IrCutA` in one-line mode. Resolve this by having `probe` return the surviving node. Add to `Capabilities`:

```rust
    /// In one-line mode, the node that exists.
    pub single_node: Option<Node>,
```

set it to `Some(Node::IrCutA)` or `Some(Node::IrCutB)` accordingly and `None` in two-line mode, then thread it into `plan`'s one-line branch. Add a test asserting a `ircut_b`-only board plans a write to `IrCutB`.

**Step 5: Run to verify it passes**

Expected: PASS.

**Step 6: Commit**

```bash
git add cross-compile/onvif-rust/src/platform/anyka/night_mode.rs cross-compile/onvif-rust/Cargo.toml
git commit -m "$(cat <<'EOF'
feat(imaging): injectable node paths and hardware capability probe

Two base dirs cover all six nodes so tests retarget them at a tempdir.
One stat() per node answers both the one-line/two-line wiring question
and the ONVIF supported/unsupported question.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 5: The executor, and the coil guard

**This is the task where a mistake damages hardware.** In two-line mode the solenoid coil is energised between the opposed pulse and the trailing zero writes. If an error propagates out of that window with `?`, the coil stays energised indefinitely.

The rule: **no `?` between the pulse and the idle writes.** Errors are collected and returned after the zeros have been written.

**Files:**
- Modify: `cross-compile/onvif-rust/src/platform/anyka/night_mode.rs`

**Step 1: Write the failing test**

```rust
    #[test]
    fn test_execute_writes_final_values_to_nodes() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = NodePaths::rooted(dir.path(), dir.path());
        std::fs::write(paths.node(Node::IrCutA), "9").unwrap();
        std::fs::write(paths.node(Node::IrCutB), "9").unwrap();
        std::fs::write(paths.node(Node::IrLed), "9").unwrap();

        let steps = plan(DayNight::Night, pol(), LineMode::Two);
        let outcome = execute_gpio(&steps, &paths);

        assert!(outcome.is_ok());
        assert_eq!(
            std::fs::read_to_string(paths.node(Node::IrLed)).unwrap(),
            "1"
        );
        // Both coil lines back to idle.
        assert_eq!(
            std::fs::read_to_string(paths.node(Node::IrCutA)).unwrap(),
            "0"
        );
        assert_eq!(
            std::fs::read_to_string(paths.node(Node::IrCutB)).unwrap(),
            "0"
        );
    }

    #[test]
    fn test_execute_still_idles_the_coil_when_an_earlier_write_fails() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = NodePaths::rooted(dir.path(), dir.path());
        // IrLed is absent, so its write fails. IrCutA/B exist.
        std::fs::write(paths.node(Node::IrCutA), "9").unwrap();
        std::fs::write(paths.node(Node::IrCutB), "9").unwrap();

        let steps = plan(DayNight::Night, pol(), LineMode::Two);
        let outcome = execute_gpio(&steps, &paths);

        assert!(outcome.is_err(), "the failed IrLed write must be reported");
        assert_eq!(
            std::fs::read_to_string(paths.node(Node::IrCutA)).unwrap(),
            "0",
            "coil must be de-energised even though an earlier step failed"
        );
        assert_eq!(
            std::fs::read_to_string(paths.node(Node::IrCutB)).unwrap(),
            "0"
        );
    }
```

The second test is the one that matters. Confirm it fails for the right reason before implementing — an implementation using `?` will leave `ircut_a` at `1`.

**Step 2: Run to verify it fails**

Expected: FAIL, `cannot find function 'execute_gpio'`.

**Step 3: Write the implementation**

```rust
/// Execute the GPIO and sleep steps of a plan.
///
/// [`Step::IspMode`] is skipped here; the caller performs it over IPC, because
/// it needs the vendor daemon's VI handle.
///
/// Every step runs even if an earlier one failed, and the first error is
/// returned at the end. This is deliberate and must not be "cleaned up" into
/// `?`: in [`LineMode::Two`] the solenoid coil is energised between the pulse
/// and the trailing zero writes, and an early return leaves it that way.
pub(super) fn execute_gpio(steps: &[Step], paths: &NodePaths) -> std::io::Result<()> {
    let mut first_err: Option<std::io::Error> = None;

    for step in steps {
        match step {
            Step::Write { node, value } => {
                let path = paths.node(*node);
                if let Err(e) = std::fs::write(&path, value.to_string()) {
                    tracing::warn!(path = %path.display(), value, error = %e, "GPIO write failed");
                    first_err.get_or_insert(e);
                }
            }
            Step::Sleep(d) => std::thread::sleep(*d),
            Step::IspMode(_) => {}
        }
    }

    match first_err {
        Some(e) => Err(e),
        None => Ok(()),
    }
}
```

`std::thread::sleep` is correct here rather than `tokio::time::sleep`: the whole call is wrapped in `spawn_blocking` by its caller in Task 8, and the 10 ms pulse must not be at the mercy of the scheduler. On this SoC any yielding await costs roughly a scheduler quantum, which is a large fraction of a 10 ms pulse.

**Step 4: Run to verify it passes**

Expected: PASS. If `test_execute_still_idles_the_coil_when_an_earlier_write_fails` fails, you used `?`.

**Step 5: Run clippy and format**

```bash
$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
$CARGO fmt
```

**Step 6: Commit**

```bash
git add cross-compile/onvif-rust/src/platform/anyka/night_mode.rs
git commit -m "$(cat <<'EOF'
feat(imaging): GPIO executor that always de-energises the coil

No ? between the two-line pulse and the trailing zero writes: an early
return there leaves a solenoid coil energised. Errors are collected and
returned after the idle writes, and a test pins the behaviour.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 6: Read the light sensor

**Files:**
- Modify: `cross-compile/onvif-rust/src/platform/anyka/night_mode.rs`

**Step 1: Write the failing test**

```rust
    #[test]
    fn test_read_light_sensor_parses_raw_value() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = NodePaths::rooted(dir.path(), dir.path());
        std::fs::write(paths.light_sensor(), "306\n").unwrap();

        assert_eq!(read_light_sensor(&paths).unwrap(), 306);
    }

    #[test]
    fn test_read_light_sensor_reports_error_when_node_is_absent() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = NodePaths::rooted(dir.path(), dir.path());

        assert!(read_light_sensor(&paths).is_none());
    }

    #[test]
    fn test_read_light_sensor_reports_error_on_unparseable_content() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = NodePaths::rooted(dir.path(), dir.path());
        std::fs::write(paths.light_sensor(), "not a number").unwrap();

        assert!(read_light_sensor(&paths).is_none());
    }
```

**Step 2: Run to verify it fails**

**Step 3: Write the implementation**

```rust
/// Read the raw light-sensor value.
///
/// Returns `None` on any failure. The caller must treat that as "hold the
/// current mode" — never as day, which would switch off night vision the
/// moment the sensor node hiccups.
pub(super) fn read_light_sensor(paths: &NodePaths) -> Option<i32> {
    let path = paths.light_sensor();
    let raw = std::fs::read_to_string(&path)
        .map_err(|e| tracing::warn!(path = %path.display(), error = %e, "light sensor read failed"))
        .ok()?;
    raw.trim().parse::<i32>().ok()
}
```

**Step 4: Run to verify it passes**

**Step 5: Commit**

```bash
git add cross-compile/onvif-rust/src/platform/anyka/night_mode.rs
git commit -m "$(cat <<'EOF'
feat(imaging): read ain0 light sensor, failing to None

None means hold the current mode. Defaulting to day here would drop
night vision whenever the sensor node hiccups.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 7: Config surface

`ImagingConfig.ir_cut_filter` is a `bool` today, which is precisely why `AUTO` was never implementable (design D3). Widen it rather than adding a parallel key, so there is one source of truth.

**Files:**
- Modify: `cross-compile/onvif-rust/src/config/types.rs:580-604`
- Modify: `SD_card_contents/anyka_hack/anyka.toml` (tracked template)
- Check: `.deploy/anyka.toml` is untracked and holds the real WiFi PSK — update it too, but do not commit it.

**Step 1: Write the failing test**

In the `types.rs` test module:

```rust
    #[test]
    fn test_imaging_config_defaults_to_auto_ir_cut_filter() {
        let cfg = ImagingConfig::default();
        assert_eq!(cfg.ir_cut_filter, IrCutFilterMode::AUTO);
    }

    #[test]
    fn test_imaging_config_parses_ir_cut_filter_mode_from_toml() {
        let cfg: ImagingConfig = toml::from_str(r#"ir_cut_filter = "OFF""#).unwrap();
        assert_eq!(cfg.ir_cut_filter, IrCutFilterMode::OFF);
    }

    #[test]
    fn test_night_config_defaults_match_vendor_lock_time() {
        let cfg = NightConfig::default();
        assert_eq!(cfg.lock_time_ms, 900_000);
        assert!(cfg.ldr_high_is_day);
        assert!(cfg.ircut_high_is_night);
    }
```

**Step 2: Run to verify it fails**

**Step 3: Write the implementation**

Replace the `ir_cut_filter: bool` field and add the night block:

```rust
use crate::onvif::types::common::IrCutFilterMode;

/// Imaging settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ImagingConfig {
    pub brightness: f64,
    pub contrast: f64,
    pub saturation: f64,
    pub sharpness: f64,
    /// Widened from `bool`: a bool cannot express AUTO, which is why AUTO
    /// was previously unimplementable at this layer.
    pub ir_cut_filter: IrCutFilterMode,
    pub ir_led: bool,
    pub night: NightConfig,
}

impl Default for ImagingConfig {
    fn default() -> Self {
        Self {
            brightness: 50.0,
            contrast: 50.0,
            saturation: 50.0,
            sharpness: 50.0,
            ir_cut_filter: IrCutFilterMode::AUTO,
            ir_led: false,
            night: NightConfig::default(),
        }
    }
}

/// Day/night calibration. Polarity and thresholds vary per board; the
/// settle delay and poll interval do not and are constants in
/// `platform::anyka::night_mode`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct NightConfig {
    /// `true` when a high `ain0` reading means daylight.
    pub ldr_high_is_day: bool,
    /// `true` when writing `1` to the ircut node selects night.
    pub ircut_high_is_night: bool,
    /// At or above this raw `ain0` reading, the sensor is saturated bright.
    /// MUST be calibrated on hardware — see design H5, and Task 14.
    pub day_threshold: i32,
    /// At or below this raw `ain0` reading, the sensor is saturated dark.
    /// MUST be calibrated on hardware.
    pub night_threshold: i32,
    /// Minimum time between transitions, preventing dusk oscillation.
    pub lock_time_ms: u64,
}

impl Default for NightConfig {
    fn default() -> Self {
        Self {
            ldr_high_is_day: true,
            ircut_high_is_night: true,
            day_threshold: 1100,
            night_threshold: 300,
            lock_time_ms: 900_000,
        }
    }
}
```

**Step 4: Fix the compile errors this causes**

`ir_cut_filter` was a `bool`. Build and fix each call site:

```bash
$CARGO build --target x86_64-unknown-linux-gnu 2>&1 | head -40
```

**Step 5: Add the config template block**

In `SD_card_contents/anyka_hack/anyka.toml`, under `[imaging]`:

```toml
[imaging.night]
# Board wiring. Verify on hardware before trusting these.
ldr_high_is_day = true
ircut_high_is_night = true
# Raw /sys/kernel/ain/ain0 thresholds. These are the VENDOR defaults and
# they are WRONG for this board: its resting reading of 306 falls in the
# unhandled band between them. Calibrate per Task 14 before enabling AUTO.
day_threshold = 1100
night_threshold = 300
# Minimum ms between day/night transitions. Prevents dusk oscillation.
lock_time_ms = 900000
```

**Step 6: Run tests, clippy, format, commit**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --lib
$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
$CARGO fmt
git add cross-compile/onvif-rust/src/config/types.rs SD_card_contents/anyka_hack/anyka.toml
git commit -m "$(cat <<'EOF'
feat(config): widen ir_cut_filter to IrCutFilterMode, add [imaging.night]

The bool could not express AUTO, which is why AUTO was never
implementable at the config layer. Widening keeps one source of truth
rather than adding a parallel mode key.

Thresholds ship as vendor defaults and are documented as wrong for this
board pending hardware calibration.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 8: Widen the platform boundary

`onvif_to_platform_settings` (`onvif/imaging/store.rs:617-626`) collapses `IrCutFilterMode` to a bool with `matches!(m, ON | AUTO)`, so ON and AUTO arrive at the platform indistinguishable. This is the same defect as D3 at a second location, and it must be fixed before AUTO can work.

**Files:**
- Modify: `cross-compile/onvif-rust/src/platform/common/traits.rs:286`
- Modify: `cross-compile/onvif-rust/src/onvif/imaging/store.rs:617-630`
- Modify: `cross-compile/onvif-rust/src/platform/anyka/imaging.rs:91`
- Modify: `cross-compile/onvif-rust/src/platform/stub/mod.rs:365,1411,1663`

**Step 1: Write the failing test**

In `store.rs` tests:

```rust
    #[test]
    fn test_auto_ir_cut_filter_survives_conversion_to_platform() {
        use crate::onvif::types::common::{ImagingSettings20, IrCutFilterMode};

        let onvif = ImagingSettings20 {
            ir_cut_filter: Some(IrCutFilterMode::AUTO),
            ..Default::default()
        };

        let platform = ImagingStore::onvif_to_platform_settings(&onvif);

        assert_eq!(platform.ir_cut_filter, IrCutFilterMode::AUTO);
    }

    #[test]
    fn test_on_and_auto_are_distinguishable_at_the_platform_boundary() {
        use crate::onvif::types::common::{ImagingSettings20, IrCutFilterMode};

        let on = ImagingStore::onvif_to_platform_settings(&ImagingSettings20 {
            ir_cut_filter: Some(IrCutFilterMode::ON),
            ..Default::default()
        });
        let auto = ImagingStore::onvif_to_platform_settings(&ImagingSettings20 {
            ir_cut_filter: Some(IrCutFilterMode::AUTO),
            ..Default::default()
        });

        assert_ne!(on.ir_cut_filter, auto.ir_cut_filter);
    }
```

**Step 2: Run to verify it fails**

Expected: FAIL, type mismatch — `bool` vs `IrCutFilterMode`.

**Step 3: Change the field type**

In `traits.rs`:

```rust
    /// IR cut filter mode. ON = filter in (day), OFF = filter out (night),
    /// AUTO = follow the light sensor.
    pub ir_cut_filter: crate::onvif::types::common::IrCutFilterMode,
```

`ImagingSettings` derives `Default`; `IrCutFilterMode` already derives `Default = AUTO`, so that still works.

In `store.rs`, replace the collapsing map:

```rust
            ir_cut_filter: settings.ir_cut_filter.clone().unwrap_or_default(),
```

Then fix `anyka/imaging.rs:91` and the three `stub/mod.rs` sites, replacing `true`/`false` with `IrCutFilterMode::AUTO` / `IrCutFilterMode::OFF` as the surrounding intent requires.

**Step 4: Run all tests**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --lib
```

**Step 5: Commit**

```bash
git add cross-compile/onvif-rust/src
git commit -m "$(cat <<'EOF'
fix(imaging): stop collapsing IrCutFilterMode to bool at the platform boundary

onvif_to_platform_settings mapped ON|AUTO to true, so the two arrived
indistinguishable and AUTO could never be acted on.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 9: Honest capability reporting

`ImagingOptions::default_options` hardcodes `ir_cut_filter_supported: true, ir_led_supported: true` (design D4). Feed it the probe from Task 4.

**Files:**
- Modify: `cross-compile/onvif-rust/src/platform/common/traits.rs:318-330`
- Modify: `cross-compile/onvif-rust/src/platform/anyka/imaging.rs:170-172`

**Step 1: Write the failing test**

In `anyka/imaging.rs` tests:

```rust
    #[tokio::test]
    async fn test_get_options_reports_ir_unsupported_when_nodes_are_absent() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = crate::platform::anyka::night_mode::NodePaths::rooted(dir.path(), dir.path());
        let control = AnykaImagingControl::with_ffi_and_paths(
            Arc::new(MockImagingHalTrait::new()),
            paths,
        );

        let options = control.get_options().await.unwrap();

        assert!(!options.ir_cut_filter_supported);
        assert!(!options.ir_led_supported);
    }
```

**Step 2: Run to verify it fails**

**Step 3: Implement**

Add a `paths: NodePaths` and a cached `Capabilities` to `AnykaImagingControl`, populated by `probe()` in the constructor, and have `get_options` build from it:

```rust
    async fn get_options(&self) -> PlatformResult<ImagingOptions> {
        Ok(ImagingOptions {
            ir_cut_filter_supported: self.caps.line_mode.is_some(),
            ir_led_supported: self.caps.ir_led,
            ..ImagingOptions::default_options()
        })
    }
```

Leave `default_options` alone — the stub platform still wants it.

**Step 4: Run tests, commit**

```bash
git add cross-compile/onvif-rust/src
git commit -m "$(cat <<'EOF'
feat(imaging): report IR capabilities from a node probe, not a hardcode

ImagingOptions advertised ir_cut_filter_supported and ir_led_supported
as unconditionally true. One stat() per node replaces both lies.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 10: Apply transitions, and the AUTO tick

This wires the pure parts to the hardware. Mutual exclusion is one `tokio::sync::Mutex` around `apply`, shared by the AUTO tick, `SetImagingSettings`, and auxiliary commands.

**Files:**
- Modify: `cross-compile/onvif-rust/src/platform/anyka/night_mode.rs`
- Modify: `cross-compile/onvif-rust/src/platform/anyka/imaging.rs:151-168`

**Step 1: Write the failing test**

```rust
    #[tokio::test]
    async fn test_apply_night_calls_isp_switch_and_writes_gpios() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = NodePaths::rooted(dir.path(), dir.path());
        for n in [Node::IrCutA, Node::IrCutB, Node::IrLed] {
            std::fs::write(paths.node(n), "9").unwrap();
        }

        let mut ffi = MockImagingHalTrait::new();
        ffi.expect_set_ir_filter()
            .withf(|enabled| *enabled)
            .times(1)
            .returning(|_| 0);

        let ctl = NightModeController::new(paths.clone(), test_config(), Arc::new(ffi));
        ctl.apply(DayNight::Night).await.unwrap();

        assert_eq!(
            std::fs::read_to_string(paths.node(Node::IrLed)).unwrap(),
            "1"
        );
    }

    #[tokio::test]
    async fn test_apply_still_writes_gpios_when_the_isp_call_fails() {
        // A daemon restart makes the IPC call fail. GPIO state is durable and
        // must still be correct, so the next tick only has to redo the ISP.
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = NodePaths::rooted(dir.path(), dir.path());
        for n in [Node::IrCutA, Node::IrCutB, Node::IrLed] {
            std::fs::write(paths.node(n), "9").unwrap();
        }

        let mut ffi = MockImagingHalTrait::new();
        ffi.expect_set_ir_filter().returning(|_| -1);

        let ctl = NightModeController::new(paths.clone(), test_config(), Arc::new(ffi));
        let _ = ctl.apply(DayNight::Night).await;

        assert_eq!(
            std::fs::read_to_string(paths.node(Node::IrLed)).unwrap(),
            "1"
        );
    }
```

**Step 2: Run to verify it fails**

**Step 3: Implement**

```rust
/// Owns the night-mode state and serialises every transition.
pub(super) struct NightModeController {
    paths: NodePaths,
    caps: Capabilities,
    cfg: crate::config::types::NightConfig,
    ffi: Arc<dyn crate::hal::common::imaging::ImagingHalTrait>,
    state: tokio::sync::Mutex<AutoState>,
}

impl NightModeController {
    /// Apply a transition. The mutex makes concurrent transitions
    /// impossible, which matters because the two-line coil must never be
    /// driven by two callers at once.
    pub(super) async fn apply(&self, target: DayNight) -> PlatformResult<()> {
        let mut state = self.state.lock().await;

        let Some(line_mode) = self.caps.line_mode else {
            return Ok(()); // no filter hardware; nothing to do
        };
        let steps = plan(
            target,
            Polarity {
                ircut_high_is_night: self.cfg.ircut_high_is_night,
            },
            line_mode,
        );

        // GPIO first: it is durable and survives a daemon restart, whereas
        // the ISP mode does not. A bounce then costs one tick of
        // wrong-looking image rather than a stuck filter.
        let paths = self.paths.clone();
        let gpio_steps = steps.clone();
        let gpio_result =
            tokio::task::spawn_blocking(move || execute_gpio(&gpio_steps, &paths)).await;

        // The ISP step is the only part that needs the daemon.
        let isp = self.ffi.set_ir_filter(matches!(target, DayNight::Night)).await;

        state.record_change(target, std::time::Instant::now());

        if isp != 0 {
            tracing::warn!(isp, "ISP day/night switch failed; will retry next tick");
        }
        match gpio_result {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(PlatformError::OperationFailed(format!("GPIO write: {e}"))),
            Err(e) => Err(PlatformError::OperationFailed(format!("join: {e}"))),
        }
    }

    /// One AUTO poll: read, classify, decide, maybe apply.
    pub(super) async fn tick(&self) {
        let Some(raw) = read_light_sensor(&self.paths) else {
            return; // hold current mode
        };
        let reading = classify(
            raw,
            Thresholds {
                day: self.cfg.day_threshold,
                night: self.cfg.night_threshold,
                ldr_high_is_day: self.cfg.ldr_high_is_day,
            },
        );
        let target = {
            let state = self.state.lock().await;
            decide(
                &state,
                reading,
                std::time::Instant::now(),
                Duration::from_millis(self.cfg.lock_time_ms),
            )
        };
        if let Some(target) = target {
            if let Err(e) = self.apply(target).await {
                tracing::warn!(error = %e, "night-mode transition failed");
            }
        }
    }
}
```

Spawn the AUTO loop where the platform is constructed, using `POLL_INTERVAL`. Only tick while the configured mode is `AUTO`.

**Step 4: Wire `set_settings`**

In `anyka/imaging.rs:151`, after the four existing parameter calls:

```rust
        match settings.ir_cut_filter {
            IrCutFilterMode::ON => self.night.apply(DayNight::Day).await?,
            IrCutFilterMode::OFF => self.night.apply(DayNight::Night).await?,
            IrCutFilterMode::AUTO => {} // the AUTO loop owns transitions
        }
```

**Step 5: Run tests, clippy, format, commit**

```bash
git add cross-compile/onvif-rust/src
git commit -m "$(cat <<'EOF'
feat(imaging): apply night-mode transitions and run the AUTO poll loop

One tokio Mutex serialises the AUTO tick, SetImagingSettings, and
auxiliary commands, so the two-line coil can never be driven by two
callers at once. GPIO precedes the IPC call because GPIO state survives
a daemon restart and ISP state does not.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 11: Auxiliary commands for the lamps

`send_auxiliary_command` currently logs and returns success for anything, including hardware that does not exist (design D5).

**Files:**
- Modify: `cross-compile/onvif-rust/src/onvif/ptz/ops/auxiliary.rs:79-92`
- Modify: `cross-compile/onvif-rust/src/onvif/ptz/ops/config.rs:58`

**Step 1: Write the failing test**

Replace `test_send_auxiliary_command_returns_none_when_unsupported`:

```rust
    #[test]
    fn test_parse_ir_lamp_on() {
        assert_eq!(
            parse_auxiliary("tt:IRLamp|On"),
            Some(AuxCommand::IrLamp(LampState::On))
        );
    }

    #[test]
    fn test_parse_white_light_off() {
        assert_eq!(
            parse_auxiliary("tt:WhiteLight|Off"),
            Some(AuxCommand::WhiteLight(LampState::Off))
        );
    }

    #[test]
    fn test_parse_ir_lamp_auto() {
        assert_eq!(
            parse_auxiliary("tt:IRLamp|Auto"),
            Some(AuxCommand::IrLamp(LampState::Auto))
        );
    }

    #[test]
    fn test_parse_rejects_unknown_command() {
        assert_eq!(parse_auxiliary("tt:Wiper|On"), None);
    }

    #[test]
    fn test_parse_rejects_malformed_command() {
        assert_eq!(parse_auxiliary("tt:IRLamp"), None);
        assert_eq!(parse_auxiliary(""), None);
    }
```

**Step 2: Run to verify it fails**

**Step 3: Implement**

```rust
/// Requested lamp state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LampState {
    On,
    Off,
    Auto,
}

/// A recognised ONVIF auxiliary command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuxCommand {
    IrLamp(LampState),
    WhiteLight(LampState),
}

/// Parse an ONVIF auxiliary command string such as `"tt:IRLamp|On"`.
///
/// Returns `None` for anything unrecognised; the caller must raise an ONVIF
/// fault rather than reporting success for hardware that does not exist.
pub fn parse_auxiliary(data: &str) -> Option<AuxCommand> {
    let (name, state) = data.split_once('|')?;
    let state = match state {
        "On" => LampState::On,
        "Off" => LampState::Off,
        "Auto" => LampState::Auto,
        _ => return None,
    };
    match name {
        "tt:IRLamp" => Some(AuxCommand::IrLamp(state)),
        "tt:WhiteLight" => Some(AuxCommand::WhiteLight(state)),
        _ => None,
    }
}
```

Then make `send_auxiliary_command` dispatch: unknown or unsupported returns `OnvifError::InvalidArgVal`, recognised commands drive the controller. `WhiteLight` writes `Node::WhiteLed` directly; `IRLamp` with `Auto` re-enables the AUTO loop.

Advertise the commands in `config.rs`, in `build_ptz_node`:

```rust
        auxiliary_commands: vec![
            "tt:IRLamp|On".to_string(),
            "tt:IRLamp|Off".to_string(),
            "tt:IRLamp|Auto".to_string(),
            "tt:WhiteLight|On".to_string(),
            "tt:WhiteLight|Off".to_string(),
        ],
```

Populate this from the capability probe so a board without `WHITE_LED` does not advertise it.

**Step 4: Run tests, commit**

```bash
git add cross-compile/onvif-rust/src/onvif/ptz/ops
git commit -m "$(cat <<'EOF'
feat(ptz): act on tt:IRLamp and tt:WhiteLight auxiliary commands

SendAuxiliaryCommand previously logged every command and returned
success, including for hardware that does not exist. Unknown commands
now raise a fault and recognised ones drive the lamps.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 12: WebUI SOAP body and service call

**Files:**
- Modify: `cross-compile/www/src/services/soap/client.ts:214+` (`soapBodies`)
- Modify: `cross-compile/www/src/services/ptzService.ts`
- Modify: `cross-compile/www/src/services/ptzService.test.ts`

**Step 1: Write the failing test**

In `ptzService.test.ts`, following the existing MSW patterns in that file:

```ts
  it('sends an auxiliary command with the given data', async () => {
    const spy = vi.spyOn(soapClient, 'soapRequest').mockResolvedValue({} as never);

    await sendAuxiliaryCommand('Profile1', 'tt:IRLamp|On');

    expect(spy).toHaveBeenCalledWith(
      ENDPOINTS.ptz,
      expect.stringContaining('<tptz:AuxiliaryData>tt:IRLamp|On</tptz:AuxiliaryData>'),
    );
  });

  it('escapes XML metacharacters in the profile token', async () => {
    const spy = vi.spyOn(soapClient, 'soapRequest').mockResolvedValue({} as never);

    await sendAuxiliaryCommand('a&b', 'tt:IRLamp|On');

    expect(spy).toHaveBeenCalledWith(ENDPOINTS.ptz, expect.stringContaining('a&amp;b'));
  });
```

**Step 2: Run to verify it fails**

```bash
cd cross-compile/www && npm run test -- ptzService
```

**Step 3: Implement**

In `client.ts`, alongside the other PTZ bodies:

```ts
  sendAuxiliaryCommand: (profileToken: string, auxiliaryData: string) =>
    `<tptz:SendAuxiliaryCommand><tptz:ProfileToken>${escapeXml(profileToken)}</tptz:ProfileToken><tptz:AuxiliaryData>${escapeXml(auxiliaryData)}</tptz:AuxiliaryData></tptz:SendAuxiliaryCommand>`,
```

In `ptzService.ts`:

```ts
/** Lamp control commands supported by this camera. */
export type AuxiliaryCommand =
  | 'tt:IRLamp|On'
  | 'tt:IRLamp|Off'
  | 'tt:IRLamp|Auto'
  | 'tt:WhiteLight|On'
  | 'tt:WhiteLight|Off';

/**
 * Send an ONVIF auxiliary command, used here for the IR lamp and white light.
 *
 * @param profileToken - ONVIF media profile token
 * @param command - One of the AuxiliaryCommand strings
 */
export async function sendAuxiliaryCommand(
  profileToken: string,
  command: AuxiliaryCommand,
): Promise<void> {
  await soapRequest(ENDPOINTS.ptz, soapBodies.sendAuxiliaryCommand(profileToken, command));
}
```

**Step 4: Run tests, lint, type-check, commit**

```bash
npm run test -- ptzService && npm run lint && npm run type-check
git add cross-compile/www/src/services
git commit -m "$(cat <<'EOF'
feat(webui): sendAuxiliaryCommand for IR lamp and white light

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 13: WebUI Illumination card

The IR cut filter UI already exists and needs no change — it has been driving a backend that discarded the value. Only the lamps are new.

**Files:**
- Modify: `cross-compile/www/src/pages/settings/ImagingPage.tsx` (new card after the Infrared card at `:347-399`)
- Modify: `cross-compile/www/src/pages/settings/ImagingPage.test.tsx`

**Step 1: Write the failing test**

```tsx
  it('renders the illumination card with both lamp switches', async () => {
    renderImagingPage();

    expect(await screen.findByTestId('imaging-ir-lamp-switch')).toBeInTheDocument();
    expect(screen.getByTestId('imaging-white-light-switch')).toBeInTheDocument();
  });

  it('sends the IR lamp on command when the switch is enabled', async () => {
    const spy = vi.spyOn(ptzService, 'sendAuxiliaryCommand').mockResolvedValue();
    renderImagingPage();

    await userEvent.click(await screen.findByTestId('imaging-ir-lamp-switch'));

    expect(spy).toHaveBeenCalledWith(expect.any(String), 'tt:IRLamp|On');
  });
```

**Step 2: Run to verify it fails**

**Step 3: Implement**

Add a `SettingsCard` matching the six already in the file, using `Switch` from `@radix-ui/react-switch` (already a dependency) and a `Lightbulb` icon from `lucide-react`. Give every interactive element a `data-testid`, as the rest of the file does.

**Step 4: Handle the unsupported case**

`ImagingPage.tsx:389` falls back to showing all three IR cut modes when `irCutFilterModes` is absent. Now that the backend can report unsupported, distinguish the two:

```tsx
  // undefined  -> options not loaded yet, fall back to all three
  // []         -> backend probed and found no filter hardware, hide the card
  const irCutSupported = options?.irCutFilterModes === undefined
    || options.irCutFilterModes.length > 0;
```

Add a test for the empty-array case.

**Step 5: Run tests, lint, type-check, build, commit**

```bash
npm run test && npm run lint && npm run type-check && npm run build
git add cross-compile/www/src/pages/settings
git commit -m "$(cat <<'EOF'
feat(webui): illumination card for IR lamp and white light

Distinguishes "options not loaded" from "no filter hardware" so the
infrared card hides rather than offering modes the board cannot do.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 14: Hardware calibration and verification

**This task gates the feature.** Everything above is tested against a `tempdir`; none of it proves the board is wired as assumed, and per design H5 the shipped thresholds are known-wrong for this camera.

`orig/` is a flattened capture whose symlinks were lost, so verify paths against the live camera rather than the repo.

**Access:** telnet `192.168.2.198:24`.

**Step 1: Confirm the nodes exist with the expected names**

```sh
ls /sys/user-gpio/
ls /sys/kernel/ain/
```

Expected: `ircut_a`, `ircut_b`, `IR_LED`, `WHITE_LED`, `SPK_PA`, `wifi_en`; and `ain0`, `ain1`, `bat`.

If `ircut_b` is absent, the probe will select one-line mode — confirm it does.

**Step 2: Measure the real thresholds**

```sh
# Uncovered, in room light:
cat /sys/kernel/ain/ain0
# Cover the lens completely, wait 5 s:
cat /sys/kernel/ain/ain0
```

Record both. Set `day_threshold` slightly below the bright reading and `night_threshold` slightly above the dark one, leaving a gap for hysteresis. **Do not skip this**: the vendor defaults of 1100/300 put this board's resting reading of 306 in the dead band, where AUTO will never transition.

Write the measured values into `.deploy/anyka.toml` and the tracked template.

**Step 3: Force night mode**

Set `ir_cut_filter = "OFF"`, restart the service.

Expected: an audible click from the filter solenoid; the IR LEDs glow faint purple through a phone camera; the video stream goes monochrome.

**Step 4: Force day mode**

Set `ir_cut_filter = "ON"`, restart.

Expected: the filter clicks back and colour returns. Then confirm no coil line was left energised:

```sh
cat /sys/user-gpio/ircut_a /sys/user-gpio/ircut_b
```

Expected: `0` and `0`. **Anything else means the coil guard in Task 5 is broken — stop and fix it before continuing.**

**Step 5: Verify AUTO**

Set `ir_cut_filter = "AUTO"` with the measured thresholds, restart, cover the lens.

Expected: exactly one transition to night within a few seconds. Uncover: no transition until `lock_time_ms` has elapsed. This is the check that cannot be faked in tests.

**Step 6: Verify the lamps over ONVIF**

From an ONVIF client, send `tt:WhiteLight|On` and `tt:IRLamp|Off`, confirming each takes effect and that `tt:Wiper|On` returns a fault rather than silent success.

**Step 7: Record the results**

Append the measured `ain0` values and the outcome of each step to the design doc, then commit.

```bash
git add docs/plans/2026-08-02-ir-led-support-design.md SD_card_contents/anyka_hack/anyka.toml
git commit -m "$(cat <<'EOF'
docs(design): record measured ain0 thresholds from hardware

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
EOF
)"
```

Note: `.deploy/anyka.toml` is untracked because it holds the real WiFi PSK. Update it, but keep it out of the commit.

---

## Final verification

```bash
source ./setenv.sh
cd cross-compile/onvif-rust
$CARGO test --target x86_64-unknown-linux-gnu
$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
$CARGO fmt --check
$CARGO build --release            # ARM cross-build must succeed

cd ../www
npm run test && npm run lint && npm run type-check && npm run build
```

Then request review with `superpowers:requesting-code-review` before merging.

**Do not claim this feature works until Task 14 step 5 has been observed on hardware.** Every test above passes against a temporary directory.
