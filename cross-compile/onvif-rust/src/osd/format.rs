//! Timestamp formatting for the OSD.
//!
//! Lives in Rust rather than the daemon so it can be tested on the host and so
//! the C side needs no strftime or TZ handling.

use chrono::{DateTime, TimeZone};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DateFormat {
    /// `2026-08-24`
    Iso,
    /// `24/08/2026`
    European,
    /// `08/24/2026`
    Us,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TimeFormat {
    /// `15:04:05`
    H24,
    /// `03:04:05 PM`
    H12,
}

/// Render `when` as an ASCII date-and-time string.
pub fn format_datetime<Tz: TimeZone>(
    when: DateTime<Tz>,
    date: DateFormat,
    time: TimeFormat,
) -> String
where
    Tz::Offset: std::fmt::Display,
{
    let date_pattern = match date {
        DateFormat::Iso => "%Y-%m-%d",
        DateFormat::European => "%d/%m/%Y",
        DateFormat::Us => "%m/%d/%Y",
    };
    let time_pattern = match time {
        TimeFormat::H24 => "%H:%M:%S",
        TimeFormat::H12 => "%I:%M:%S %p",
    };
    when.format(&format!("{date_pattern} {time_pattern}"))
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn sample() -> chrono::DateTime<chrono::Utc> {
        chrono::Utc.with_ymd_and_hms(2026, 8, 24, 15, 4, 5).unwrap()
    }

    #[test]
    fn test_format_iso_date_with_24h_clock() {
        let s = format_datetime(sample(), DateFormat::Iso, TimeFormat::H24);
        assert_eq!(s, "2026-08-24 15:04:05");
    }

    #[test]
    fn test_format_european_date_with_12h_clock() {
        let s = format_datetime(sample(), DateFormat::European, TimeFormat::H12);
        assert_eq!(s, "24/08/2026 03:04:05 PM");
    }

    #[test]
    fn test_format_us_date() {
        let s = format_datetime(sample(), DateFormat::Us, TimeFormat::H24);
        assert_eq!(s, "08/24/2026 15:04:05");
    }

    #[test]
    fn test_formatted_output_is_always_ascii() {
        // Feeds straight into encode_glyphs, which rejects non-ASCII.
        for date in [DateFormat::Iso, DateFormat::European, DateFormat::Us] {
            for time in [TimeFormat::H12, TimeFormat::H24] {
                assert!(format_datetime(sample(), date, time).is_ascii());
            }
        }
    }
}
