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
