//! POSIX TZ string parsing.
//!
//! A POSIX TZ string carries its own DST rules, so no zoneinfo database is
//! needed — which matters on a rootfs that has no `/usr/share/zoneinfo` and no
//! `/etc/localtime`. This is also the format ONVIF specifies for `TimeZone/TZ`.
//!
//! chrono has an equivalent parser but it is crate-private (`tz_info`).

use std::sync::{OnceLock, RwLock};

use chrono::{DateTime, Datelike, FixedOffset, NaiveDate, NaiveDateTime, Utc};

/// The zone every consumer reads: the OSD renderer, the log timer and the
/// ONVIF handlers.
///
/// A global rather than state threaded through `AppState` because the log
/// timer is constructed before `AppState` exists, and because the alternative —
/// reading the config lock — deadlocks: `onvif/media/ops/osd.rs:114` holds a
/// config write guard across a call that logs.
///
/// INVARIANT: never log while holding this lock.
fn cell() -> &'static RwLock<PosixTz> {
    static CELL: OnceLock<RwLock<PosixTz>> = OnceLock::new();
    CELL.get_or_init(|| RwLock::new(PosixTz::utc()))
}

/// Serializes the tests that mutate the process-wide cell. stdlib mutex: the
/// plan forbids adding `serial_test`, and a poisoned lock must not cascade
/// into unrelated failures.
#[cfg(test)]
pub fn test_lock() -> parking_lot::ReentrantMutexGuard<'static, ()> {
    static TEST_LOCK: parking_lot::ReentrantMutex<()> = parking_lot::ReentrantMutex::new(());
    TEST_LOCK.lock()
}

/// The zone currently in force.
///
/// Falls back to UTC if the lock was poisoned, because a log line with a
/// slightly wrong timestamp beats a panic inside the logger.
pub fn current() -> PosixTz {
    match cell().read() {
        Ok(tz) => tz.clone(),
        Err(_) => PosixTz::utc(),
    }
}

/// Replace the zone in force. Call after loading config and on every accepted
/// `SetSystemDateAndTime`.
pub fn set_current(tz: PosixTz) {
    if let Ok(mut slot) = cell().write() {
        *slot = tz;
    }
}

/// A transition rule in the `Mm.w.d[/time]` form.
///
/// `week` 5 means "last such weekday in the month", not "the fifth".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Rule {
    month: u32,
    week: u32,
    dow: u32,
    secs: i32,
}

/// A parsed POSIX timezone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PosixTz {
    std: FixedOffset,
    dst: Option<(FixedOffset, Rule, Rule)>,
}

impl PosixTz {
    /// UTC, used as the fallback when no zone is configured.
    pub fn utc() -> Self {
        Self {
            std: FixedOffset::east_opt(0).expect("0 is a valid offset"),
            dst: None,
        }
    }

    /// The offset in effect at `when`.
    pub fn offset_at(&self, when: DateTime<Utc>) -> FixedOffset {
        let Some((dst_off, start, end)) = self.dst else {
            return self.std;
        };
        let year = when.naive_utc().year();
        // Transition times are given in local time: the start rule in standard
        // time, the end rule in DST time. Convert both to UTC before comparing.
        let Some(start_utc) = rule_instant(year, start, self.std) else {
            return self.std;
        };
        let Some(end_utc) = rule_instant(year, end, dst_off) else {
            return self.std;
        };
        let now = when.naive_utc();
        let in_dst = if start_utc <= end_utc {
            now >= start_utc && now < end_utc
        } else {
            // Southern hemisphere: the DST window wraps the new year.
            now >= start_utc || now < end_utc
        };
        if in_dst { dst_off } else { self.std }
    }

    /// Render `when` in this zone.
    pub fn convert(&self, when: DateTime<Utc>) -> DateTime<FixedOffset> {
        when.with_timezone(&self.offset_at(when))
    }

    /// Is DST in force at `when`?
    pub fn is_dst(&self, when: DateTime<Utc>) -> bool {
        self.offset_at(when) != self.std
    }
}

/// The UTC instant a rule fires in `year`, given the offset in force just before it.
fn rule_instant(year: i32, rule: Rule, before: FixedOffset) -> Option<NaiveDateTime> {
    let day = nth_weekday(year, rule.month, rule.week, rule.dow)?;
    let local = day.and_hms_opt(0, 0, 0)? + chrono::Duration::seconds(rule.secs as i64);
    Some(local - chrono::Duration::seconds(before.local_minus_utc() as i64))
}

/// The `week`-th `dow` of `month`; `week == 5` means the last one.
fn nth_weekday(year: i32, month: u32, week: u32, dow: u32) -> Option<NaiveDate> {
    let first = NaiveDate::from_ymd_opt(year, month, 1)?;
    let first_dow = first.weekday().num_days_from_sunday();
    let offset = (dow + 7 - first_dow) % 7;
    let mut day = first + chrono::Duration::days(offset as i64);
    for _ in 1..week {
        let next = day + chrono::Duration::days(7);
        if next.month() != month {
            break;
        }
        day = next;
    }
    Some(day)
}

/// Parse a POSIX TZ string.
///
/// Accepts `STDoffset[DST[offset][,start[/time],end[/time]]]` with `Mm.w.d`
/// transition dates. The `Jn` and `n` day forms are rejected rather than
/// silently misparsed — a wrong DST date is worse than a refused config.
pub fn parse(s: &str) -> Result<PosixTz, &'static str> {
    let s = s.trim();
    if s.is_empty() {
        return Err("empty timezone");
    }
    let (std_name_len, rest) = split_name(s)?;
    if std_name_len == 0 {
        return Err("missing standard zone name");
    }
    let (std_off, rest) = parse_offset(rest)?;

    if rest.is_empty() {
        return Ok(PosixTz {
            std: std_off,
            dst: None,
        });
    }

    let (dst_name_len, rest) = split_name(rest)?;
    if dst_name_len == 0 {
        return Err("trailing junk after offset");
    }
    // A DST name with no offset means one hour ahead.
    let (dst_off, rest) = if rest.starts_with(',') || rest.is_empty() {
        (
            FixedOffset::east_opt(std_off.local_minus_utc() + 3600)
                .ok_or("dst offset out of range")?,
            rest,
        )
    } else {
        parse_offset(rest)?
    };

    let rest = rest.strip_prefix(',').ok_or("missing DST rules")?;
    let (start_s, end_s) = rest.split_once(',').ok_or("missing DST end rule")?;
    let start = parse_rule(start_s)?;
    let end = parse_rule(end_s)?;
    Ok(PosixTz {
        std: std_off,
        dst: Some((dst_off, start, end)),
    })
}

/// Consume a zone name, either `<+05>` or three or more letters.
fn split_name(s: &str) -> Result<(usize, &str), &'static str> {
    if let Some(rest) = s.strip_prefix('<') {
        let end = rest.find('>').ok_or("unterminated <> zone name")?;
        return Ok((end, &rest[end + 1..]));
    }
    let len = s.chars().take_while(|c| c.is_ascii_alphabetic()).count();
    if len > 0 && len < 3 {
        return Err("zone name shorter than three characters");
    }
    Ok((len, &s[len..]))
}

/// Consume `[+|-]hh[:mm[:ss]]`, returning the offset east of UTC.
///
/// POSIX states the offset as time to ADD to local to get UTC, so the sign is
/// inverted relative to the usual convention: `CET-1` is UTC+1.
fn parse_offset(s: &str) -> Result<(FixedOffset, &str), &'static str> {
    let (sign, s) = match s.as_bytes().first() {
        Some(b'-') => (-1, &s[1..]),
        Some(b'+') => (1, &s[1..]),
        _ => (1, s),
    };
    let mut parts = [0i32; 3];
    let mut rest = s;
    for (i, part) in parts.iter_mut().enumerate() {
        let len = rest.chars().take_while(|c| c.is_ascii_digit()).count();
        if len == 0 {
            if i == 0 {
                return Err("missing offset digits");
            }
            break;
        }
        *part = rest[..len].parse().map_err(|_| "bad offset number")?;
        rest = &rest[len..];
        match rest.strip_prefix(':') {
            Some(next) if i < 2 => rest = next,
            _ => break,
        }
    }
    let secs = parts[0] * 3600 + parts[1] * 60 + parts[2];
    // POSIX sign is inverted, hence the negation.
    let off = FixedOffset::east_opt(-sign * secs).ok_or("offset out of range")?;
    Ok((off, rest))
}

/// Parse one `Mm.w.d[/time]` rule.
fn parse_rule(s: &str) -> Result<Rule, &'static str> {
    let (date, time) = match s.split_once('/') {
        Some((d, t)) => (d, Some(t)),
        None => (s, None),
    };
    let date = date
        .strip_prefix('M')
        .ok_or("only Mm.w.d rules are supported")?;
    let mut it = date.split('.');
    let month: u32 = it
        .next()
        .ok_or("bad rule")?
        .parse()
        .map_err(|_| "bad month")?;
    let week: u32 = it
        .next()
        .ok_or("bad rule")?
        .parse()
        .map_err(|_| "bad week")?;
    let dow: u32 = it
        .next()
        .ok_or("bad rule")?
        .parse()
        .map_err(|_| "bad weekday")?;
    if it.next().is_some() {
        return Err("too many fields in rule");
    }
    if !(1..=12).contains(&month) || !(1..=5).contains(&week) || dow > 6 {
        return Err("rule field out of range");
    }
    // Transitions default to 02:00 local.
    let secs = match time {
        None => 2 * 3600,
        Some(t) => {
            let (off, rest) = parse_offset(t)?;
            if !rest.is_empty() {
                return Err("trailing junk in rule time");
            }
            // parse_offset inverts the sign; undo that for a wall-clock time.
            -off.local_minus_utc()
        }
    };
    Ok(Rule {
        month,
        week,
        dow,
        secs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn utc(y: i32, m: u32, d: u32, h: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, m, d, h, 0, 0).unwrap()
    }

    #[test]
    fn test_parse_fixed_offset_zone_has_no_dst() {
        let tz = parse("UTC0").unwrap();
        assert_eq!(tz.offset_at(utc(2026, 7, 1, 12)).local_minus_utc(), 0);
    }

    #[test]
    fn test_parse_posix_sign_is_inverted() {
        // "CET-1" means UTC+1, not UTC-1.
        let tz = parse("CET-1").unwrap();
        assert_eq!(tz.offset_at(utc(2026, 1, 15, 12)).local_minus_utc(), 3600);
    }

    #[test]
    fn test_warsaw_winter_is_utc_plus_one() {
        let tz = parse("CET-1CEST,M3.5.0,M10.5.0/3").unwrap();
        assert_eq!(tz.offset_at(utc(2026, 1, 15, 12)).local_minus_utc(), 3600);
    }

    #[test]
    fn test_warsaw_summer_is_utc_plus_two() {
        let tz = parse("CET-1CEST,M3.5.0,M10.5.0/3").unwrap();
        assert_eq!(tz.offset_at(utc(2026, 7, 15, 12)).local_minus_utc(), 7200);
    }

    #[test]
    fn test_spring_forward_boundary_is_exact() {
        // 2026-03-29 01:00 UTC is the instant Europe/Warsaw jumps 02:00 -> 03:00.
        let tz = parse("CET-1CEST,M3.5.0,M10.5.0/3").unwrap();
        let before = utc(2026, 3, 29, 0);
        let after = utc(2026, 3, 29, 1);
        assert_eq!(tz.offset_at(before).local_minus_utc(), 3600);
        assert_eq!(tz.offset_at(after).local_minus_utc(), 7200);
    }

    #[test]
    fn test_fall_back_boundary_is_exact() {
        // 2026-10-25 01:00 UTC is the instant Europe/Warsaw returns to CET.
        let tz = parse("CET-1CEST,M3.5.0,M10.5.0/3").unwrap();
        assert_eq!(tz.offset_at(utc(2026, 10, 25, 0)).local_minus_utc(), 7200);
        assert_eq!(tz.offset_at(utc(2026, 10, 25, 1)).local_minus_utc(), 3600);
    }

    #[test]
    fn test_southern_hemisphere_window_wraps_the_year() {
        // NZ: DST runs Sep -> Apr, so the start rule is later than the end rule.
        let tz = parse("NZST-12NZDT,M9.5.0,M4.1.0/3").unwrap();
        assert_eq!(
            tz.offset_at(utc(2026, 1, 15, 0)).local_minus_utc(),
            13 * 3600
        );
        assert_eq!(
            tz.offset_at(utc(2026, 6, 15, 0)).local_minus_utc(),
            12 * 3600
        );
    }

    #[test]
    fn test_offset_with_minutes_is_parsed() {
        let tz = parse("IST-5:30").unwrap();
        assert_eq!(
            tz.offset_at(utc(2026, 1, 1, 0)).local_minus_utc(),
            5 * 3600 + 1800
        );
    }

    #[test]
    fn test_julian_day_rules_are_rejected_not_guessed() {
        assert!(parse("EST5EDT,J60,J300").is_err());
    }

    #[test]
    fn test_malformed_input_is_rejected() {
        for bad in ["", "X", "CET-", "CET-1CEST,M13.5.0,M10.5.0"] {
            assert!(parse(bad).is_err(), "expected {bad:?} to be rejected");
        }
    }

    #[test]
    fn test_current_defaults_to_utc_before_any_set() {
        // No set_current call in this test binary path; UTC is the safe default.
        let _lock = test_lock();
        assert_eq!(
            current().offset_at(utc(2026, 7, 1, 12)).local_minus_utc(),
            0
        );
    }

    #[test]
    fn test_set_current_is_visible_to_readers() {
        let _lock = test_lock();
        set_current(parse("CET-1CEST,M3.5.0,M10.5.0/3").unwrap());
        assert_eq!(
            current().offset_at(utc(2026, 7, 15, 12)).local_minus_utc(),
            7200
        );
        set_current(PosixTz::utc()); // restore for other tests
    }

    /// Mirrors `www/src/utils/timezones.ts`. If you add a zone to the WebUI
    /// picker, add it here too — an unparseable value would leave the operator
    /// unable to select a zone the camera accepts.
    #[test]
    fn test_every_webui_timezone_parses() {
        for tz in [
            "UTC0",
            "GMT0BST,M3.5.0/1,M10.5.0",
            "CET-1CEST,M3.5.0,M10.5.0/3",
            "EET-2EEST,M3.5.0/3,M10.5.0/4",
            "EST5EDT,M3.2.0,M11.1.0",
            "PST8PDT,M3.2.0,M11.1.0",
            "CST-8",
            "JST-9",
        ] {
            assert!(
                parse(tz).is_ok(),
                "WebUI offers {tz:?}, which the camera rejects"
            );
        }
    }
}
