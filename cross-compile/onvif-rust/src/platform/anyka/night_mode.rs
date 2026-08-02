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
