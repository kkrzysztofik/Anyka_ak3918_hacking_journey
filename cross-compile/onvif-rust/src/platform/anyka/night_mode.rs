//! Night-mode control: IR cut filter, IR illuminator, white floodlight.
//!
//! The write ordering and the two-line coil-idle pulse are taken from the
//! vendor reference (`anyka_reference/libre_anyka_app/main.c:740-752` and
//! `platform/libplat/src/drv/ak_drv_ir.c:230-255`) and are load-bearing.
//! See `docs/plans/2026-08-02-ir-led-support-design.md`.

use std::path::{Path, PathBuf};
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
    Write {
        node: Node,
        value: u8,
    },
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
    /// In one-line mode, the node that exists. `None` in two-line mode.
    pub single_node: Option<Node>,
    pub ir_led: bool,
    pub white_led: bool,
}

/// Probe for the nodes. One `stat()` per node answers both the wiring
/// question and the ONVIF capability question.
pub(super) fn probe(paths: &NodePaths) -> Capabilities {
    let a = paths.node(Node::IrCutA).exists();
    let b = paths.node(Node::IrCutB).exists();

    let (line_mode, single_node) = match (a, b) {
        (true, true) => (Some(LineMode::Two), None),
        (true, false) => (Some(LineMode::One), Some(Node::IrCutA)),
        (false, true) => (Some(LineMode::One), Some(Node::IrCutB)),
        (false, false) => (None, None),
    };

    Capabilities {
        line_mode,
        single_node,
        ir_led: paths.node(Node::IrLed).exists(),
        white_led: paths.node(Node::WhiteLed).exists(),
    }
}

/// Build the ordered step list for a transition.
///
/// Ordering follows the vendor reference and is not arbitrary: the lamp turns
/// on before the ISP switches to night and off after it switches to day, so no
/// frame is captured dark. In [`LineMode::Two`] the trailing zero writes
/// de-energise the solenoid coil and are mandatory.
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

pub(super) fn plan(
    target: DayNight,
    pol: Polarity,
    line_mode: LineMode,
    single_node: Option<Node>,
) -> Vec<Step> {
    let night_level = u8::from(pol.ircut_high_is_night);
    let ircut_level = match target {
        DayNight::Night => night_level,
        DayNight::Day => 1 - night_level,
    };

    let mut ircut = Vec::new();
    match line_mode {
        LineMode::One => ircut.push(Step::Write {
            node: single_node.unwrap_or(Node::IrCutA),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn pol() -> Polarity {
        Polarity {
            ircut_high_is_night: true,
        }
    }

    fn thr() -> Thresholds {
        Thresholds {
            day: 1100,
            night: 300,
            ldr_high_is_day: true,
        }
    }

    #[test]
    fn test_probe_reports_two_line_when_both_ircut_nodes_exist() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = NodePaths::rooted(dir.path(), dir.path());
        std::fs::write(paths.node(Node::IrCutA), "0").unwrap();
        std::fs::write(paths.node(Node::IrCutB), "0").unwrap();
        std::fs::write(paths.node(Node::IrLed), "0").unwrap();

        let caps = probe(&paths);

        assert_eq!(caps.line_mode, Some(LineMode::Two));
        assert_eq!(caps.single_node, None);
        assert!(caps.ir_led);
    }

    #[test]
    fn test_probe_reports_one_line_when_only_ircut_a_exists() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = NodePaths::rooted(dir.path(), dir.path());
        std::fs::write(paths.node(Node::IrCutA), "0").unwrap();

        let caps = probe(&paths);

        assert_eq!(caps.line_mode, Some(LineMode::One));
        assert_eq!(caps.single_node, Some(Node::IrCutA));
        assert!(!caps.ir_led);
    }

    #[test]
    fn test_probe_reports_unsupported_when_no_ircut_nodes_exist() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = NodePaths::rooted(dir.path(), dir.path());

        let caps = probe(&paths);

        assert_eq!(caps.line_mode, None);
        assert_eq!(caps.single_node, None);
    }

    #[test]
    #[test]
    fn test_execute_writes_final_values_to_nodes() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = NodePaths::rooted(dir.path(), dir.path());
        std::fs::write(paths.node(Node::IrCutA), "9").unwrap();
        std::fs::write(paths.node(Node::IrCutB), "9").unwrap();
        std::fs::write(paths.node(Node::IrLed), "9").unwrap();

        let steps = plan(DayNight::Night, pol(), LineMode::Two, None);
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
        // IrLed is a directory so its write fails (sysfs missing nodes also fail;
        // a plain absent path would be created by fs::write on a tempdir).
        std::fs::write(paths.node(Node::IrCutA), "9").unwrap();
        std::fs::write(paths.node(Node::IrCutB), "9").unwrap();
        std::fs::create_dir(paths.node(Node::IrLed)).unwrap();

        let steps = plan(DayNight::Night, pol(), LineMode::Two, None);
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

    #[test]
    fn test_one_line_ircut_b_only_board_plans_write_to_ircut_b() {
        let steps = plan(DayNight::Night, pol(), LineMode::One, Some(Node::IrCutB));

        assert_eq!(
            steps,
            vec![
                Step::Write {
                    node: Node::IrLed,
                    value: 1
                },
                Step::IspMode(DayNight::Night),
                Step::Write {
                    node: Node::IrCutB,
                    value: 1
                },
                Step::Sleep(SETTLE),
            ]
        );
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

    #[test]
    fn test_night_mode_plan_lights_lamp_before_isp_switch() {
        let steps = plan(DayNight::Night, pol(), LineMode::Two, None);

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
        let steps = plan(DayNight::Day, pol(), LineMode::Two, None);

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
        let steps = plan(DayNight::Night, pol(), LineMode::One, None);

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

        let steps = plan(DayNight::Night, inverted, LineMode::One, None);

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
