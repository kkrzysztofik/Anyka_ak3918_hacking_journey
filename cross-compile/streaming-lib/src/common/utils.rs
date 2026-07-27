use std::sync::Mutex;
use std::time::{Duration, Instant};

#[macro_export]
macro_rules! scanf {
    ( $string:expr, $sep:expr, $( $x:ty ),+ ) => {{
        let mut iter = $string.split($sep);
        ($(iter.next().and_then(|word| word.parse::<$x>().ok()),)*)
    }}
}

/// What a throttled call site should report when it is allowed to log again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThrottledBurst {
    /// Occurrences since the previous emitted line, including this one.
    pub occurrences: u64,
    /// Largest value observed over those occurrences.
    pub peak: u64,
}

/// Collapses a repeating condition into at most one log line per period.
///
/// A slow transmit path or an unset clock is a steady state, not an event: warning on every
/// occurrence costs a line per frame and says nothing the first line did not. Record every
/// occurrence but emit rarely, carrying the count and the peak so the collapsed lines still
/// convey rate and severity.
#[derive(Debug)]
pub struct LogThrottle {
    period: Duration,
    state: Mutex<ThrottleState>,
}

#[derive(Debug)]
struct ThrottleState {
    last_emit: Option<Instant>,
    occurrences: u64,
    peak: u64,
}

impl LogThrottle {
    pub fn new(period: Duration) -> Self {
        Self {
            period,
            state: Mutex::new(ThrottleState {
                last_emit: None,
                occurrences: 0,
                peak: 0,
            }),
        }
    }

    /// Record one occurrence carrying `value`.
    ///
    /// Returns `Some` only when the caller should emit a line — the first occurrence, then at
    /// most once per period. The returned counts cover everything suppressed since the last
    /// emitted line, so no occurrence goes unaccounted for.
    pub fn record(&self, value: u64) -> Option<ThrottledBurst> {
        self.record_at(value, Instant::now())
    }

    fn record_at(&self, value: u64, now: Instant) -> Option<ThrottledBurst> {
        // A poisoned lock only means some other thread panicked mid-update; the counters are
        // diagnostics, so recovering and carrying on beats taking the stream down with us.
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.occurrences += 1;
        state.peak = state.peak.max(value);

        let due = match state.last_emit {
            None => true,
            Some(last) => now.duration_since(last) >= self.period,
        };
        if !due {
            return None;
        }

        let burst = ThrottledBurst {
            occurrences: state.occurrences,
            peak: state.peak,
        };
        state.last_emit = Some(now);
        state.occurrences = 0;
        state.peak = 0;
        Some(burst)
    }
}

#[cfg(test)]
mod throttle_tests {
    use super::*;

    #[test]
    fn test_first_occurrence_always_emits() {
        let throttle = LogThrottle::new(Duration::from_secs(10));
        let burst = throttle.record(7).expect("first occurrence must emit");
        assert_eq!(burst.occurrences, 1);
        assert_eq!(burst.peak, 7);
    }

    #[test]
    fn test_suppresses_within_period_then_reports_count_and_peak() {
        let throttle = LogThrottle::new(Duration::from_secs(10));
        let start = Instant::now();

        assert!(throttle.record_at(5, start).is_some());
        for i in 1..=9 {
            assert!(
                throttle.record_at(i * 2, start + Duration::from_millis(i)).is_none(),
                "occurrence {i} within the period must be suppressed"
            );
        }

        let burst = throttle
            .record_at(3, start + Duration::from_secs(10))
            .expect("period elapsed, must emit");
        // 9 suppressed + the one that triggered this line.
        assert_eq!(burst.occurrences, 10);
        assert_eq!(burst.peak, 18, "peak must survive the collapse");
    }

    #[test]
    fn test_counters_reset_between_emitted_lines() {
        let throttle = LogThrottle::new(Duration::from_secs(1));
        let start = Instant::now();
        throttle.record_at(100, start);
        throttle.record_at(50, start + Duration::from_secs(1));

        let burst = throttle
            .record_at(4, start + Duration::from_secs(2))
            .expect("must emit");
        assert_eq!(burst.occurrences, 1, "previous burst must not be counted again");
        assert_eq!(burst.peak, 4, "peak must not carry over from a reported burst");
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_scanf_single_string() {
        let (val,) = scanf!("hello", " ", String);
        assert_eq!(val, Some("hello".to_string()));
    }

    #[test]
    fn test_scanf_two_integers() {
        let (a, b) = scanf!("10 20", " ", u32, u32);
        assert_eq!(a, Some(10));
        assert_eq!(b, Some(20));
    }

    #[test]
    fn test_scanf_mixed_types() {
        let (name, age) = scanf!("alice:30", ":", String, u32);
        assert_eq!(name, Some("alice".to_string()));
        assert_eq!(age, Some(30));
    }

    #[test]
    fn test_scanf_parse_failure_returns_none() {
        let (a, b) = scanf!("hello world", " ", u32, u32);
        assert_eq!(a, None);
        assert_eq!(b, None);
    }

    #[test]
    fn test_scanf_missing_fields_returns_none() {
        let (a, b, c) = scanf!("1 2", " ", u32, u32, u32);
        assert_eq!(a, Some(1));
        assert_eq!(b, Some(2));
        assert_eq!(c, None);
    }

    #[test]
    fn test_scanf_ip_address_parts() {
        let (a, b, c, d) = scanf!("192.168.1.1", ".", u8, u8, u8, u8);
        assert_eq!(a, Some(192));
        assert_eq!(b, Some(168));
        assert_eq!(c, Some(1));
        assert_eq!(d, Some(1));
    }

    #[test]
    fn test_scanf_empty_string() {
        let (a,) = scanf!("", " ", u32);
        assert_eq!(a, None);
    }
}
