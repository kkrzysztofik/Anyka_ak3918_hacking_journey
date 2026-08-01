//! Restart policy. Everything here is pure so it can be exercised exhaustively
//! on the host; the caller applies the returned `Action` through `Sys`.
//!
//! All time is monotonic (`Instant`). P2.5 steps the wall clock by decades
//! seconds into supervision, so any policy built on `SystemTime` would either
//! fire instantly or never.

use crate::sys::Pid;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Exponential backoff: `min << (attempt - 1)`, clamped to `max`.
pub fn backoff_delay(attempt: u32, min: Duration, max: Duration) -> Duration {
    if attempt == 0 {
        return min;
    }
    // Shifts past 63 would overflow; anything that large is already >= max.
    let shift = attempt - 1;
    if shift >= 63 {
        return max;
    }
    match min.checked_mul(1u32 << shift.min(31)) {
        Some(d) if d < max => d,
        _ => max,
    }
}

/// Sliding window of restart timestamps, used for the crash-loop cap.
#[derive(Debug, Default)]
pub struct RestartHistory {
    stamps: VecDeque<Instant>,
}

impl RestartHistory {
    pub fn record(&mut self, at: Instant) {
        self.stamps.push_back(at);
    }

    /// Drops entries strictly older than `window`. An entry exactly `window`
    /// old is still inside the window.
    pub fn prune(&mut self, now: Instant, window: Duration) {
        while let Some(&front) = self.stamps.front() {
            if now.duration_since(front) > window {
                self.stamps.pop_front();
            } else {
                break;
            }
        }
    }

    pub fn len(&self) -> usize {
        self.stamps.len()
    }

    pub fn is_empty(&self) -> bool {
        self.stamps.is_empty()
    }

    pub fn clear(&mut self) {
        self.stamps.clear();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SvcState {
    Running { pid: Pid, since: Instant },
    Backoff { until: Instant, attempt: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Exited,
    Tick,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Start,
    Sleep(Duration),
    Reboot(String),
    None,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decision {
    pub action: Action,
    pub next: SvcState,
}

#[derive(Debug, Clone, Copy)]
pub struct Policy {
    pub backoff_min: Duration,
    pub backoff_max: Duration,
    pub crashloop_count: u32,
    pub crashloop_window: Duration,
}

/// The supervisor's entire restart policy, as a total function.
///
/// `hist` is `&mut` only so that pruning and recording happen in one place;
/// the function has no other effects and performs no I/O.
pub fn decide(
    state: &SvcState,
    hist: &mut RestartHistory,
    ev: Event,
    now: Instant,
    p: &Policy,
) -> Decision {
    match (state, ev) {
        (SvcState::Running { since, .. }, Event::Exited) => {
            hist.prune(now, p.crashloop_window);
            hist.record(now);

            if hist.len() as u32 >= p.crashloop_count {
                return Decision {
                    action: Action::Reboot(format!(
                        "{} restarts within {}s",
                        hist.len(),
                        p.crashloop_window.as_secs()
                    )),
                    next: SvcState::Backoff {
                        until: now,
                        attempt: 0,
                    },
                };
            }

            // A run longer than the backoff ceiling counts as stable, so the
            // escalation resets. The threshold sits *above* the ceiling on
            // purpose: at or below it, a service dying every `backoff_max - 1`
            // seconds would reset forever and never reach the cap.
            let ran = now.duration_since(*since);
            let attempt = if ran > p.backoff_max {
                hist.clear();
                1
            } else {
                hist.len() as u32
            };

            let delay = backoff_delay(attempt, p.backoff_min, p.backoff_max);
            Decision {
                action: Action::Sleep(delay),
                next: SvcState::Backoff {
                    until: now + delay,
                    attempt,
                },
            }
        }

        (SvcState::Backoff { until, attempt }, _) => {
            if now >= *until {
                Decision {
                    action: Action::Start,
                    next: SvcState::Backoff {
                        until: *until,
                        attempt: *attempt,
                    },
                }
            } else {
                Decision {
                    action: Action::Sleep(until.duration_since(now)),
                    next: *state,
                }
            }
        }

        (SvcState::Running { .. }, Event::Tick) => Decision {
            action: Action::None,
            next: *state,
        },
    }
}

#[cfg(test)]
mod backoff_tests {
    use super::*;
    use std::time::Duration;

    const MIN: Duration = Duration::from_secs(1);
    const MAX: Duration = Duration::from_secs(60);

    #[test]
    fn test_backoff_doubles_from_min() {
        assert_eq!(backoff_delay(1, MIN, MAX), Duration::from_secs(1));
        assert_eq!(backoff_delay(2, MIN, MAX), Duration::from_secs(2));
        assert_eq!(backoff_delay(3, MIN, MAX), Duration::from_secs(4));
        assert_eq!(backoff_delay(4, MIN, MAX), Duration::from_secs(8));
        assert_eq!(backoff_delay(5, MIN, MAX), Duration::from_secs(16));
        assert_eq!(backoff_delay(6, MIN, MAX), Duration::from_secs(32));
    }

    #[test]
    fn test_backoff_saturates_at_max() {
        assert_eq!(backoff_delay(7, MIN, MAX), MAX);
        assert_eq!(backoff_delay(50, MIN, MAX), MAX);
        // Must not panic or wrap on a shift far past u64 width.
        assert_eq!(backoff_delay(u32::MAX, MIN, MAX), MAX);
    }

    #[test]
    fn test_backoff_attempt_zero_returns_min() {
        assert_eq!(backoff_delay(0, MIN, MAX), MIN);
    }
}

#[cfg(test)]
mod history_tests {
    use super::*;
    use std::time::{Duration, Instant};

    const WINDOW: Duration = Duration::from_secs(600);

    #[test]
    fn test_history_counts_restarts_inside_window() {
        let t0 = Instant::now();
        let mut h = RestartHistory::default();
        for i in 0..5 {
            h.record(t0 + Duration::from_secs(i * 10));
        }
        h.prune(t0 + Duration::from_secs(50), WINDOW);
        assert_eq!(h.len(), 5);
    }

    #[test]
    fn test_history_prunes_entries_older_than_window() {
        let t0 = Instant::now();
        let mut h = RestartHistory::default();
        h.record(t0);
        h.record(t0 + Duration::from_secs(100));
        // 700s after t0: the first entry is 700s old, outside a 600s window.
        h.prune(t0 + Duration::from_secs(700), WINDOW);
        assert_eq!(h.len(), 1);
    }

    #[test]
    fn test_history_entry_exactly_at_window_edge_is_kept() {
        let t0 = Instant::now();
        let mut h = RestartHistory::default();
        h.record(t0);
        h.prune(t0 + WINDOW, WINDOW);
        assert_eq!(h.len(), 1, "an entry exactly `window` old is still inside");
    }

    #[test]
    fn test_history_empty_prune_is_noop() {
        let mut h = RestartHistory::default();
        h.prune(Instant::now(), WINDOW);
        assert_eq!(h.len(), 0);
    }
}

#[cfg(test)]
mod decide_tests {
    use super::*;
    use std::time::{Duration, Instant};

    fn policy() -> Policy {
        Policy {
            backoff_min: Duration::from_secs(1),
            backoff_max: Duration::from_secs(60),
            crashloop_count: 10,
            crashloop_window: Duration::from_secs(600),
        }
    }

    #[test]
    fn test_decide_exit_after_short_run_enters_backoff() {
        let t0 = Instant::now();
        let st = SvcState::Running { pid: 42, since: t0 };
        let mut h = RestartHistory::default();
        let d = decide(
            &st,
            &mut h,
            Event::Exited,
            t0 + Duration::from_secs(1),
            &policy(),
        );
        assert!(matches!(d.action, Action::Sleep(x) if x == Duration::from_secs(1)));
        assert!(matches!(d.next, SvcState::Backoff { attempt: 1, .. }));
    }

    #[test]
    fn test_decide_stable_run_resets_attempt_counter() {
        let t0 = Instant::now();
        let mut h = RestartHistory::default();
        h.record(t0);
        h.record(t0);
        h.record(t0);
        let running = SvcState::Running { pid: 7, since: t0 };
        // Ran 61s, above backoff_max of 60s => considered stable.
        let d = decide(
            &running,
            &mut h,
            Event::Exited,
            t0 + Duration::from_secs(61),
            &policy(),
        );
        assert!(
            matches!(d.next, SvcState::Backoff { attempt: 1, .. }),
            "a run longer than backoff_max resets the escalation"
        );
    }

    #[test]
    fn test_decide_run_just_under_max_does_not_reset() {
        let t0 = Instant::now();
        let mut h = RestartHistory::default();
        // 59s < backoff_max: must keep escalating, or a service dying every
        // 59s would reset forever and never hit the cap.
        let st = SvcState::Running { pid: 7, since: t0 };
        h.record(t0);
        h.record(t0);
        let d = decide(
            &st,
            &mut h,
            Event::Exited,
            t0 + Duration::from_secs(59),
            &policy(),
        );
        assert!(matches!(d.next, SvcState::Backoff { attempt: 3, .. }));
    }

    #[test]
    fn test_decide_crashloop_cap_triggers_reboot() {
        let t0 = Instant::now();
        let mut h = RestartHistory::default();
        for i in 0..9 {
            h.record(t0 + Duration::from_secs(i));
        }
        let st = SvcState::Running { pid: 9, since: t0 };
        let d = decide(
            &st,
            &mut h,
            Event::Exited,
            t0 + Duration::from_secs(10),
            &policy(),
        );
        assert!(
            matches!(d.action, Action::Reboot(_)),
            "10th restart in window must reboot"
        );
    }

    #[test]
    fn test_decide_crashloop_not_triggered_one_restart_early() {
        let t0 = Instant::now();
        let mut h = RestartHistory::default();
        for i in 0..8 {
            h.record(t0 + Duration::from_secs(i));
        }
        let st = SvcState::Running { pid: 9, since: t0 };
        let d = decide(
            &st,
            &mut h,
            Event::Exited,
            t0 + Duration::from_secs(10),
            &policy(),
        );
        assert!(
            matches!(d.action, Action::Sleep(_)),
            "9th restart must not reboot"
        );
    }

    #[test]
    fn test_decide_crashloop_ignores_restarts_outside_window() {
        let t0 = Instant::now();
        let mut h = RestartHistory::default();
        for i in 0..20 {
            h.record(t0 + Duration::from_secs(i));
        }
        let st = SvcState::Running { pid: 9, since: t0 };
        // 1h later: every recorded restart is outside the 600s window.
        let d = decide(
            &st,
            &mut h,
            Event::Exited,
            t0 + Duration::from_secs(3600),
            &policy(),
        );
        assert!(matches!(d.action, Action::Sleep(_)));
    }

    #[test]
    fn test_decide_backoff_expired_yields_start() {
        let t0 = Instant::now();
        let mut h = RestartHistory::default();
        let st = SvcState::Backoff {
            until: t0,
            attempt: 2,
        };
        let d = decide(
            &st,
            &mut h,
            Event::Tick,
            t0 + Duration::from_secs(1),
            &policy(),
        );
        assert!(matches!(d.action, Action::Start));
    }

    #[test]
    fn test_decide_backoff_not_yet_expired_sleeps_remaining() {
        let t0 = Instant::now();
        let mut h = RestartHistory::default();
        let st = SvcState::Backoff {
            until: t0 + Duration::from_secs(10),
            attempt: 3,
        };
        let d = decide(
            &st,
            &mut h,
            Event::Tick,
            t0 + Duration::from_secs(4),
            &policy(),
        );
        assert!(matches!(d.action, Action::Sleep(x) if x == Duration::from_secs(6)));
    }

    #[test]
    fn test_decide_tick_while_running_is_noop() {
        let t0 = Instant::now();
        let mut h = RestartHistory::default();
        let st = SvcState::Running { pid: 3, since: t0 };
        let d = decide(
            &st,
            &mut h,
            Event::Tick,
            t0 + Duration::from_secs(5),
            &policy(),
        );
        assert!(matches!(d.action, Action::None));
    }
}
