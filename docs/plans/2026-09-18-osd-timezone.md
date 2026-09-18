# Timezone-Aware OSD and Log Timestamps Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make the video overlay and the camera's log files show the configured local time instead of UTC, and make the timezone settable at runtime through ONVIF and the WebUI.

**Architecture:** A self-contained POSIX TZ string (`CET-1CEST,M3.5.0,M10.5.0/3`) is parsed in-process by a new `onvif-rust/src/time/tz.rs`, so no zoneinfo database is needed on the 36 MB rootfs. The parsed zone lives in one process-global cell that both the OSD renderer and the log timer read, which guarantees they can never disagree. `onvif-rust`'s `config.toml` is the persisted source of truth; the supervisor reads it at boot to set `TZ` for the other services.

**Tech Stack:** Rust 2024 (vendored toolchain), chrono, tracing-subscriber, libc, React 19 + Vitest for the WebUI.

**Design doc:** `docs/plans/2026-09-18-osd-timezone-design.md`

---

## Required Reading Before Starting

1. `docs/plans/2026-09-18-osd-timezone-design.md` — the approved design and the rejected alternatives.
2. `AGENTS.md` — workflow and quality gates.

## Toolchain

All cargo commands use the vendored toolchain. Set this once per shell:

```bash
export CARGO=/home/kmk/dev/anyka-dev/toolchain/arm-anykav200-crosstool-ng/bin/cargo
export PATH=/home/kmk/dev/anyka-dev/toolchain/arm-anykav200-crosstool-ng/bin:$PATH
```

The `PATH` prefix is **required** — clippy fails with `E0514` without it.

Host-side tests and lints run from `cross-compile/`:

```bash
cd cross-compile
$CARGO test   --target x86_64-unknown-linux-gnu -p onvif-rust
$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
$CARGO fmt
```

An ARM release build must run from `cross-compile/onvif-rust/`, not the workspace root, or cargo silently links with the host toolchain.

---

## Critical Constraint: Never Take the Config Lock in the Log Timer

`onvif/media/ops/osd.rs:114` holds a `config.write()` guard and then calls
`push_to_renderer` at line 125, which logs. `config/runtime.rs:98-133` logs
while holding a read guard. A `FormatTime` implementation that called
`config.read()` would therefore deadlock on `parking_lot::RwLock` the first
time any of those paths emitted a line.

This is why Task 2 puts the parsed zone in a dedicated global cell with its own
lock, and why the rule below is absolute:

> **Never log while holding the `tz` cell's write lock.**

`arc-swap` is not in the lockfile and is not being added for this.

---

## Task 1: POSIX TZ Parser

**Files:**
- Create: `cross-compile/onvif-rust/src/time/mod.rs`
- Create: `cross-compile/onvif-rust/src/time/tz.rs`
- Modify: `cross-compile/onvif-rust/src/lib.rs` (add `pub mod time;`)

Note the POSIX sign convention is inverted: `CET-1` means **UTC+1**.

**Step 1: Write the failing tests**

In `src/time/tz.rs`:

```rust
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
        assert_eq!(tz.offset_at(utc(2026, 1, 15, 0)).local_minus_utc(), 13 * 3600);
        assert_eq!(tz.offset_at(utc(2026, 6, 15, 0)).local_minus_utc(), 12 * 3600);
    }

    #[test]
    fn test_offset_with_minutes_is_parsed() {
        let tz = parse("IST-5:30").unwrap();
        assert_eq!(tz.offset_at(utc(2026, 1, 1, 0)).local_minus_utc(), 5 * 3600 + 1800);
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
}
```

**Step 2: Run the tests to verify they fail**

```bash
cd cross-compile && $CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust tz::
```
Expected: FAIL — `cannot find function 'parse'`.

**Step 3: Write the implementation**

`src/time/mod.rs`:

```rust
//! Time handling: POSIX timezone parsing and the process-wide zone.

pub mod tz;
```

`src/time/tz.rs`:

```rust
//! POSIX TZ string parsing.
//!
//! A POSIX TZ string carries its own DST rules, so no zoneinfo database is
//! needed — which matters on a rootfs that has no `/usr/share/zoneinfo` and no
//! `/etc/localtime`. This is also the format ONVIF specifies for `TimeZone/TZ`.
//!
//! chrono has an equivalent parser but it is crate-private (`tz_info`).

use chrono::{DateTime, Datelike, FixedOffset, NaiveDate, NaiveDateTime, Utc};

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
        return Ok(PosixTz { std: std_off, dst: None });
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
    Ok(PosixTz { std: std_off, dst: Some((dst_off, start, end)) })
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
    let date = date.strip_prefix('M').ok_or("only Mm.w.d rules are supported")?;
    let mut it = date.split('.');
    let month: u32 = it.next().ok_or("bad rule")?.parse().map_err(|_| "bad month")?;
    let week: u32 = it.next().ok_or("bad rule")?.parse().map_err(|_| "bad week")?;
    let dow: u32 = it.next().ok_or("bad rule")?.parse().map_err(|_| "bad weekday")?;
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
    Ok(Rule { month, week, dow, secs })
}
```

Add to `src/lib.rs` beside the other `pub mod` lines:

```rust
pub mod time;
```

**Step 4: Run the tests to verify they pass**

```bash
cd cross-compile && $CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust tz::
```
Expected: PASS, 10 tests.

**Step 5: Lint and commit**

```bash
cd cross-compile && $CARGO fmt && $CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
git add cross-compile/onvif-rust/src/time/ cross-compile/onvif-rust/src/lib.rs
git commit -m "feat(time): parse POSIX TZ strings with DST rules"
```

---

## Task 2: Process-Wide Timezone Cell

The OSD renderer and the log timer must read the same zone, and the log timer
must never touch the config lock. One global cell with its own lock satisfies
both. See "Critical Constraint" above.

**Files:**
- Modify: `cross-compile/onvif-rust/src/time/tz.rs`

**Step 1: Write the failing tests**

Append to the `tests` module:

```rust
#[test]
fn test_current_defaults_to_utc_before_any_set() {
    // No set_current call in this test binary path; UTC is the safe default.
    assert_eq!(current().offset_at(utc(2026, 7, 1, 12)).local_minus_utc(), 0);
}

#[test]
fn test_set_current_is_visible_to_readers() {
    set_current(parse("CET-1CEST,M3.5.0,M10.5.0/3").unwrap());
    assert_eq!(current().offset_at(utc(2026, 7, 15, 12)).local_minus_utc(), 7200);
    set_current(PosixTz::utc()); // restore for other tests
}
```

Because these two tests share global state, they must not run concurrently
with each other. Keep them in this order and run the suite with the default
thread count; if flakiness appears, mark the pair `#[serial]` only if the crate
already uses `serial_test` — do not add a dependency for it.

**Step 2: Run to verify it fails**

```bash
cd cross-compile && $CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust tz::
```
Expected: FAIL — `cannot find function 'current'`.

**Step 3: Implement**

Add to `src/time/tz.rs`:

```rust
use std::sync::{OnceLock, RwLock};

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
```

**Step 4: Run to verify it passes**

```bash
cd cross-compile && $CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust tz::
```
Expected: PASS, 12 tests.

**Step 5: Commit**

```bash
git add cross-compile/onvif-rust/src/time/tz.rs
git commit -m "feat(time): add the process-wide timezone cell"
```

---

## Task 3: Config `[time]` Section

**Files:**
- Modify: `cross-compile/onvif-rust/src/config/types.rs`
- Modify: `SD_card_contents/anyka_hack/onvif/config.toml`

**Step 1: Write the failing test**

In `src/config/types.rs` tests:

```rust
#[test]
fn test_time_config_defaults_to_utc() {
    let c: AppConfig = toml::from_str("").unwrap_or_default();
    assert_eq!(c.time.timezone, "UTC");
}

#[test]
fn test_time_config_round_trips_a_posix_string() {
    let src = "[time]\ntimezone = \"CET-1CEST,M3.5.0,M10.5.0/3\"\n";
    let c: AppConfig = toml::from_str(src).unwrap();
    assert_eq!(c.time.timezone, "CET-1CEST,M3.5.0,M10.5.0/3");
}
```

**Step 2: Run to verify it fails**

```bash
cd cross-compile && $CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust time_config
```
Expected: FAIL — no field `time`.

**Step 3: Implement**

Add the struct next to `OsdConfig` in `types.rs`, mirroring its style:

```rust
/// System timezone (`[time]`).
///
/// A POSIX TZ string, e.g. `CET-1CEST,M3.5.0,M10.5.0/3`. Self-contained: the
/// DST rules live in the string, so no zoneinfo files are needed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct TimeConfig {
    pub timezone: String,
}

impl Default for TimeConfig {
    fn default() -> Self {
        Self { timezone: "UTC".to_string() }
    }
}
```

Add the field to `AppConfig` after `sound` (line 53):

```rust
    /// System timezone (`[time]`).
    #[serde(default)]
    pub time: TimeConfig,
```

Add `time: TimeConfig::default(),` to the `Default for AppConfig` body.

Then add to `SD_card_contents/anyka_hack/onvif/config.toml`:

```toml
[time]
# POSIX TZ. Europe/Warsaw with EU DST rules. Settable at runtime via ONVIF
# SetSystemDateAndTime; this value is the fleet default.
timezone = "CET-1CEST,M3.5.0,M10.5.0/3"
```

**Step 4: Run to verify it passes**

```bash
cd cross-compile && $CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust time_config
```
Expected: PASS.

**Step 5: Commit**

```bash
git add cross-compile/onvif-rust/src/config/types.rs SD_card_contents/anyka_hack/onvif/config.toml
git commit -m "feat(config): add the [time] timezone section"
```

---

## Task 4: Load the Configured Zone Into the Cell at Startup

**Files:**
- Modify: `cross-compile/onvif-rust/src/app.rs`

**Step 1: Write the failing test**

In `src/app.rs` tests:

```rust
#[test]
fn test_apply_configured_timezone_sets_the_cell() {
    let mut cfg = AppConfig::default();
    cfg.time.timezone = "CET-1CEST,M3.5.0,M10.5.0/3".to_string();
    apply_configured_timezone(&cfg);
    let july = chrono::Utc.with_ymd_and_hms(2026, 7, 15, 12, 0, 0).unwrap();
    assert_eq!(crate::time::tz::current().offset_at(july).local_minus_utc(), 7200);
    crate::time::tz::set_current(crate::time::tz::PosixTz::utc());
}

#[test]
fn test_apply_configured_timezone_falls_back_to_utc_on_garbage() {
    let mut cfg = AppConfig::default();
    cfg.time.timezone = "not a timezone".to_string();
    apply_configured_timezone(&cfg);
    let july = chrono::Utc.with_ymd_and_hms(2026, 7, 15, 12, 0, 0).unwrap();
    assert_eq!(crate::time::tz::current().offset_at(july).local_minus_utc(), 0);
}
```

**Step 2: Run to verify it fails**

Expected: FAIL — `cannot find function 'apply_configured_timezone'`.

**Step 3: Implement**

In `src/app.rs`:

```rust
/// Push the configured zone into the process-wide cell.
///
/// A bad string degrades to UTC with a warning rather than failing startup: a
/// camera that boots with the wrong clock display is recoverable over the
/// network, one that does not boot is not.
pub(crate) fn apply_configured_timezone(cfg: &AppConfig) {
    match crate::time::tz::parse(&cfg.time.timezone) {
        Ok(tz) => crate::time::tz::set_current(tz),
        Err(e) => {
            crate::time::tz::set_current(crate::time::tz::PosixTz::utc());
            tracing::warn!(
                timezone = %cfg.time.timezone,
                error = e,
                "invalid timezone in [time]; falling back to UTC"
            );
        }
    }
}
```

Call it in `Application::start`, immediately after the config is loaded and
**before** logging is initialised, so the first log line already carries the
right offset.

**Step 4: Run to verify it passes, then commit**

```bash
cd cross-compile && $CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust apply_configured_timezone
git add cross-compile/onvif-rust/src/app.rs
git commit -m "feat(app): apply the configured timezone at startup"
```

---

## Task 5: OSD Renderer Uses the Configured Zone

This is the bug the user reported.

**Files:**
- Modify: `cross-compile/onvif-rust/src/osd/renderer.rs:358-362`

**Step 1: Write the failing test**

In `src/osd/renderer.rs` tests:

```rust
#[test]
fn test_datetime_slot_text_uses_the_configured_zone_not_utc() {
    crate::time::tz::set_current(crate::time::tz::parse("CET-1CEST,M3.5.0,M10.5.0/3").unwrap());
    let utc_hour = chrono::Utc::now().hour();
    let text = current_datetime_text(DateFormat::Iso, TimeFormat::H24);
    let shown: u32 = text[11..13].parse().unwrap();
    assert_ne!(shown, utc_hour, "OSD still rendering UTC");
    crate::time::tz::set_current(crate::time::tz::PosixTz::utc());
}
```

**Step 2: Run to verify it fails**

Expected: FAIL — `cannot find function 'current_datetime_text'`.

**Step 3: Implement**

Extract the timestamp so it is testable without the IPC plumbing, then use it
at line 358:

```rust
/// The OSD timestamp, in the configured zone.
fn current_datetime_text(date: DateFormat, time: TimeFormat) -> String {
    let now = chrono::Utc::now();
    format_datetime(crate::time::tz::current().convert(now), date, time)
}
```

Replace the body at `renderer.rs:358-362`:

```rust
        let text = current_datetime_text(cfg.datetime.date_format, cfg.datetime.time_format);
```

Remove the now-unused `Local` import.

**Step 4: Run to verify it passes, then commit**

```bash
cd cross-compile && $CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust osd::
git add cross-compile/onvif-rust/src/osd/renderer.rs
git commit -m "fix(osd): render the timestamp in the configured timezone"
```

---

## Task 6: onvif-rust Log Timestamps

**Files:**
- Modify: `cross-compile/onvif-rust/src/logging/mod.rs`
- Modify: `cross-compile/onvif-rust/src/logging/static_assets.rs:122`

**Step 1: Write the failing test**

In `src/logging/mod.rs` tests:

```rust
#[test]
fn test_log_timer_writes_the_configured_offset() {
    use tracing_subscriber::fmt::time::FormatTime;
    crate::time::tz::set_current(crate::time::tz::parse("CET-1CEST,M3.5.0,M10.5.0/3").unwrap());
    let mut out = String::new();
    LocalTimer
        .format_time(&mut tracing_subscriber::fmt::format::Writer::new(&mut out))
        .unwrap();
    assert!(
        out.ends_with("+01:00") || out.ends_with("+02:00"),
        "expected a CET/CEST offset, got {out:?}"
    );
    crate::time::tz::set_current(crate::time::tz::PosixTz::utc());
}
```

**Step 2: Run to verify it fails**

Expected: FAIL — `cannot find value 'LocalTimer'`.

**Step 3: Implement**

In `src/logging/mod.rs`:

```rust
/// Stamps log lines in the configured zone.
///
/// Reads `time::tz`, never the config lock — `onvif/media/ops/osd.rs:114`
/// logs while holding a config write guard, so a config read here would
/// deadlock.
#[derive(Debug, Clone, Copy)]
pub struct LocalTimer;

impl tracing_subscriber::fmt::time::FormatTime for LocalTimer {
    fn format_time(&self, w: &mut tracing_subscriber::fmt::format::Writer<'_>) -> std::fmt::Result {
        let now = crate::time::tz::current().convert(chrono::Utc::now());
        write!(w, "{}", now.format("%Y-%m-%dT%H:%M:%S%.6f%:z"))
    }
}
```

Add `.with_timer(LocalTimer)` to **every** `fmt::layer()` in
`init_logging_impl` — the console layer at line 206 and each file layer
(lines 225 and 282). Missing one leaves that output on UTC.

At `static_assets.rs:122`, replace `Local::now()`:

```rust
    let now = crate::time::tz::current().convert(chrono::Utc::now());
```

**Step 4: Run to verify it passes, then commit**

```bash
cd cross-compile && $CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust logging::
git add cross-compile/onvif-rust/src/logging/
git commit -m "feat(logging): stamp onvif-rust logs in the configured timezone"
```

---

## Task 7: GetSystemDateAndTime Reports the Real Zone

**Files:**
- Modify: `cross-compile/onvif-rust/src/onvif/device/ops/system.rs:192-228`

**Step 1: Write the failing tests**

```rust
#[test]
fn test_get_system_date_and_time_reports_the_configured_zone() {
    crate::time::tz::set_current(crate::time::tz::parse("CET-1CEST,M3.5.0,M10.5.0/3").unwrap());
    let r = handle_get_system_date_and_time(GetSystemDateAndTime {}).unwrap();
    let sdt = r.system_date_and_time;
    assert_ne!(sdt.time_zone.as_ref().unwrap().tz, "UTC");
    let utc = sdt.utc_date_time.unwrap();
    let local = sdt.local_date_time.unwrap();
    assert_ne!(utc.time.hour, local.time.hour, "local still equals UTC");
    crate::time::tz::set_current(crate::time::tz::PosixTz::utc());
}
```

**Step 2: Run to verify it fails.** Expected: FAIL — the hardcoded `"UTC"`.

**Step 3: Implement**

Replace lines 213-221. Build `local_date_time` from
`tz::current().convert(now)` instead of cloning `utc_date_time`, report
`tz: <the configured string>`, and set `daylight_savings` by comparing
`offset_at(now)` with the zone's standard offset. Read the string from the
config (`config.read().time.timezone`) — this handler is not in the logging
path, so the config lock is safe here. That means the handler needs the
`&ConfigRuntime` argument; add it and update the single dispatch site at
`onvif/device/service.rs`.

Expose a helper on `PosixTz` for the DST question rather than re-deriving it:

```rust
    /// Is DST in force at `when`?
    pub fn is_dst(&self, when: DateTime<Utc>) -> bool {
        self.offset_at(when) != self.std
    }
```

**Step 4: Run to verify it passes, then commit**

```bash
cd cross-compile && $CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust system::
git add cross-compile/onvif-rust/src/onvif/device/
git commit -m "fix(onvif): report the real timezone in GetSystemDateAndTime"
```

---

## Task 8: SetSystemDateAndTime

**Files:**
- Modify: `cross-compile/onvif-rust/src/onvif/device/ops/system.rs:230-245`
- Create: `cross-compile/onvif-rust/src/time/ntp_marker.rs`

The marker is the **existence** of `{update.root}/state/ntp.disabled`,
following the `Trial` convention at `anyka-init/src/update.rs:143`: a filename
cannot be half-parsed, which matters on exFAT after a power cut.

**Step 1: Write the failing tests**

```rust
#[test]
fn test_set_system_date_and_time_rejects_a_bad_timezone() {
    let cfg = ConfigRuntime::new(AppConfig::default());
    let r = handle_set_system_date_and_time(&cfg, req_with_tz("not a zone"));
    assert!(r.is_err());
}

#[test]
fn test_set_system_date_and_time_persists_and_applies_the_zone() {
    let cfg = ConfigRuntime::new(AppConfig::default());
    handle_set_system_date_and_time(&cfg, req_with_tz("CET-1CEST,M3.5.0,M10.5.0/3")).unwrap();
    assert_eq!(cfg.read().time.timezone, "CET-1CEST,M3.5.0,M10.5.0/3");
    let july = chrono::Utc.with_ymd_and_hms(2026, 7, 15, 12, 0, 0).unwrap();
    assert_eq!(crate::time::tz::current().offset_at(july).local_minus_utc(), 7200);
    crate::time::tz::set_current(crate::time::tz::PosixTz::utc());
}

#[test]
fn test_ntp_marker_round_trips() {
    let dir = tempfile::tempdir().unwrap();
    let m = NtpMarker::new(dir.path());
    assert!(m.ntp_enabled());
    m.disable().unwrap();
    assert!(!m.ntp_enabled());
    m.enable().unwrap();
    assert!(m.ntp_enabled());
}
```

**Step 2: Run to verify it fails.**

**Step 3: Implement**

`src/time/ntp_marker.rs`:

```rust
//! Manual-clock marker shared with anyka-init.
//!
//! Deliberately a bare filename, not a parsed file: `anyka-init/src/update.rs:143`
//! records why structured state on exFAT after a power cut is a hazard here.

use std::path::{Path, PathBuf};

/// `{update_root}/state/ntp.disabled`.
pub struct NtpMarker {
    path: PathBuf,
}

impl NtpMarker {
    pub fn new(update_root: impl AsRef<Path>) -> Self {
        Self { path: update_root.as_ref().join("state/ntp.disabled") }
    }

    /// Absent marker means NTP runs. Absence is the safe default: if the SD
    /// card drops, the camera resumes syncing rather than drifting silently.
    pub fn ntp_enabled(&self) -> bool {
        !self.path.is_file()
    }

    pub fn disable(&self) -> std::io::Result<()> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(&self.path, b"")?;
        // SAFETY: sync(2) takes no arguments and cannot fail.
        unsafe { libc::sync() };
        Ok(())
    }

    pub fn enable(&self) -> std::io::Result<()> {
        match std::fs::remove_file(&self.path) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
        unsafe { libc::sync() };
        Ok(())
    }
}
```

In `handle_set_system_date_and_time`: parse the TZ first and fault with
`ter:InvalidArgVal` if it is bad — do not partially apply. Then write
`cfg.write().time.timezone`, call `tz::set_current`, and for
`SetDateTimeType::Manual` step the clock via `libc::clock_settime` and call
`marker.disable()`; for `Ntp` call `marker.enable()`.

Add `tempfile` to `[dev-dependencies]` if it is not already there.

**Step 4: Run to verify it passes, then commit**

```bash
cd cross-compile && $CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust system::
git add cross-compile/onvif-rust/src/
git commit -m "feat(onvif): implement SetSystemDateAndTime with an NTP marker"
```

---

## Task 9: Supervisor Honours the NTP Marker

**Files:**
- Modify: `cross-compile/anyka-init/src/timesync.rs:192`

Guard `sync_once`, not its callers: it is the single chokepoint for both
`first_sync` (P2.5 boot) and `resync_loop`. Guarding a caller instead lets a
reboot silently re-clobber a manually set clock.

**Step 1: Write the failing tests**

```rust
#[test]
fn test_sync_once_skips_when_the_ntp_marker_exists() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("state")).unwrap();
    std::fs::write(dir.path().join("state/ntp.disabled"), b"").unwrap();
    let mut cfg = test_cfg();
    cfg.update_root = dir.path().display().to_string();
    let sys = MockSys::new(); // no set_realtime expectation: must not be called
    assert_eq!(sync_once(&sys, &cfg, None), None);
}

#[test]
fn test_sync_once_runs_when_the_marker_is_absent() {
    let dir = tempfile::tempdir().unwrap();
    let mut cfg = test_cfg();
    cfg.update_root = dir.path().display().to_string();
    // existing happy-path expectations
}
```

**Step 2: Run to verify it fails.**

**Step 3: Implement**

At the top of `sync_once`, before the server loop:

```rust
    if std::path::Path::new(&cfg.update_root)
        .join("state/ntp.disabled")
        .is_file()
    {
        tracing::info!("NTP disabled by operator (state/ntp.disabled); not stepping the clock");
        return None;
    }
```

`TimeCfg` needs the `update_root` it does not currently carry; pass it in from
`cfg.update.root` at both construction sites.

**Step 4: Run to verify it passes, then commit**

```bash
cd cross-compile && $CARGO test --target x86_64-unknown-linux-gnu -p anyka-init timesync
git add cross-compile/anyka-init/src/
git commit -m "feat(timesync): skip NTP when the operator set the clock manually"
```

---

## Task 10: Supervisor Exports TZ to Services

This alone fixes `vendor_daemon.log`, which already calls `localtime_r`
(`vendor-daemon/src/log.c:88`) and only ever lacked the env var.

**Files:**
- Modify: `cross-compile/anyka-init/src/sys.rs:127-132`
- Modify: `cross-compile/anyka-init/src/boot.rs:235-246`

**Step 1: Write the failing test**

```rust
#[test]
fn test_resolve_timezone_prefers_the_onvif_config() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("onvif")).unwrap();
    std::fs::write(
        dir.path().join("onvif/config.toml"),
        "[time]\ntimezone = \"CET-1CEST,M3.5.0,M10.5.0/3\"\n",
    )
    .unwrap();
    assert_eq!(resolve_timezone(dir.path(), "UTC"), "CET-1CEST,M3.5.0,M10.5.0/3");
}

#[test]
fn test_resolve_timezone_falls_back_when_the_config_is_malformed() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("onvif")).unwrap();
    std::fs::write(dir.path().join("onvif/config.toml"), "not toml {{{").unwrap();
    assert_eq!(resolve_timezone(dir.path(), "CET-1"), "CET-1");
}

#[test]
fn test_resolve_timezone_falls_back_when_the_config_is_missing() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(resolve_timezone(dir.path(), "CET-1"), "CET-1");
}
```

**Step 2: Run to verify it fails.**

**Step 3: Implement**

In `boot.rs`:

```rust
/// The timezone to hand services, preferring onvif-rust's config because that
/// is the file the WebUI writes. Any failure degrades to the `anyka.toml`
/// value: a malformed service config must never block boot.
pub fn resolve_timezone(update_root: &std::path::Path, fallback: &str) -> String {
    #[derive(serde::Deserialize)]
    struct Partial { time: Option<TimeSection> }
    #[derive(serde::Deserialize)]
    struct TimeSection { timezone: Option<String> }

    std::fs::read_to_string(update_root.join("onvif/config.toml"))
        .ok()
        .and_then(|s| toml::from_str::<Partial>(&s).ok())
        .and_then(|p| p.time)
        .and_then(|t| t.timezone)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| fallback.to_string())
}
```

At `boot.rs:245`, set `TZ` from `resolve_timezone(...)` rather than
`cfg.time.timezone`, then call `unsafe { libc::tzset() }` so the supervisor's
own `localtime_r` picks it up — POSIX does not require `localtime_r` to call
`tzset` itself. Rewrite the comment at lines 235-239: services **do** inherit
`TZ` now.

In `sys.rs`, add the variable to the spawn at line 128:

```rust
        cmd.args(&spec.args)
            .env_clear()
            .env("TZ", &spec.tz)
            .envs(&spec.env)
```

`.env` comes before `.envs` so a per-service `env` entry can still override it.
Add the `tz` field to `SpawnSpec` and populate it in `supervisor_loop.rs:54`.

**Step 4: Run to verify it passes, then commit**

```bash
cd cross-compile && $CARGO test --target x86_64-unknown-linux-gnu -p anyka-init
git add cross-compile/anyka-init/src/
git commit -m "feat(boot): export TZ to services from the onvif config"
```

---

## Task 11: anyka-init Log Timestamps

**Files:**
- Modify: `cross-compile/anyka-init/src/logging.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn test_local_timer_is_not_hardcoded_utc() {
    // SAFETY: single-threaded test, before any other thread reads the env.
    unsafe { std::env::set_var("TZ", "CET-1CEST,M3.5.0,M10.5.0/3") };
    unsafe { libc::tzset() };
    let mut out = String::new();
    LocalTimer.format_time(&mut tracing_subscriber::fmt::format::Writer::new(&mut out)).unwrap();
    assert!(out.ends_with("+0100") || out.ends_with("+0200"), "got {out:?}");
}
```

**Step 2: Run to verify it fails.**

**Step 3: Implement**

```rust
/// Stamps log lines in local time, using the `TZ` that `boot.rs` sets.
///
/// libc rather than chrono: this crate does not depend on chrono and is not
/// gaining a dependency for a timestamp.
#[derive(Debug, Clone, Copy)]
pub struct LocalTimer;

impl tracing_subscriber::fmt::time::FormatTime for LocalTimer {
    fn format_time(&self, w: &mut tracing_subscriber::fmt::format::Writer<'_>) -> std::fmt::Result {
        let t = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as libc::time_t)
            .unwrap_or(0);
        let mut tm: libc::tm = unsafe { std::mem::zeroed() };
        let mut buf = [0u8; 64];
        // SAFETY: localtime_r writes into our own `tm`; strftime bounds its
        // write by the buffer length and always NUL-terminates.
        let len = unsafe {
            libc::localtime_r(&t, &mut tm);
            libc::strftime(
                buf.as_mut_ptr() as *mut libc::c_char,
                buf.len(),
                c"%Y-%m-%dT%H:%M:%S%z".as_ptr(),
                &tm,
            )
        };
        write!(w, "{}", String::from_utf8_lossy(&buf[..len]))
    }
}
```

Add `.with_timer(LocalTimer)` to **both** `fmt::layer()` calls in `init` —
lines 62 and 67.

Ordering caveat: `logging::init` runs before `boot.rs` sets `TZ`, so the first
few lines are UTC. That is correct and not worth extra machinery; the timer
reads libc's zone on each call, so every line after P2 is local.

**Step 4: Run to verify it passes, then commit**

```bash
cd cross-compile && $CARGO test --target x86_64-unknown-linux-gnu -p anyka-init logging
git add cross-compile/anyka-init/src/logging.rs
git commit -m "feat(logging): stamp supervisor logs in local time"
```

---

## Task 12: snmp-agent Log Timestamps

`libc` is already a tokio dependency, so adding it here costs no extra
compilation.

**Files:**
- Modify: `cross-compile/snmp-agent/Cargo.toml`
- Modify: `cross-compile/snmp-agent/src/main.rs:7`

**Step 1:** Add `libc = "0.2"` under `[dependencies]`.

**Step 2:** Copy the `LocalTimer` from Task 11 into `snmp-agent/src/logging.rs`
(new file, ~20 lines) with the same test. Two call sites do not justify a
shared crate; duplicate it.

**Step 3:** Replace `tracing_subscriber::fmt::init();` at `main.rs:7`:

```rust
    tracing_subscriber::fmt().with_timer(logging::LocalTimer).init();
```

The crate's `tracing-subscriber` already enables the `fmt` feature, so
`with_timer` is available with no feature change.

**Step 4: Verify and commit**

```bash
cd cross-compile && $CARGO test --target x86_64-unknown-linux-gnu -p snmp-agent
git add cross-compile/snmp-agent/
git commit -m "feat(snmp): stamp agent logs in local time"
```

---

## Task 13: Confirm vendor-daemon Needs No C Change

`log.c:88` already calls `localtime_r`, but nothing in the daemon calls
`tzset`, and POSIX does not require `localtime_r` to call it. uClibc may do so
internally — **verify on hardware, do not assume**.

**Step 1:** Deploy and check `/mnt/logs/vendor_daemon.log` for a local-time stamp.

**Step 2:** If it is still UTC, add one line at the top of `main()` in
`cross-compile/vendor-daemon/src/main.c`:

```c
    tzset();   /* TZ comes from anyka-init's spawn env */
```

with `#include <time.h>`. Commit only if needed.

---

## Task 14: WebUI Timezone List

**Files:**
- Modify: `cross-compile/www/src/pages/settings/TimePage.tsx:46-55`
- Modify: `cross-compile/www/src/pages/settings/TimePage.test.tsx`

Replace the self-described "Stub list" of bare abbreviations with POSIX strings
carrying DST rules. Bare `CET` is an hour wrong in Poland from March to
October.

```ts
const TIMEZONES = [
  { value: 'UTC0', label: 'UTC' },
  { value: 'GMT0BST,M3.5.0/1,M10.5.0', label: 'London (GMT/BST)' },
  { value: 'CET-1CEST,M3.5.0,M10.5.0/3', label: 'Warsaw / Berlin / Paris (CET/CEST)' },
  { value: 'EET-2EEST,M3.5.0/3,M10.5.0/4', label: 'Helsinki / Athens (EET/EEST)' },
  { value: 'EST5EDT,M3.2.0,M11.1.0', label: 'New York (EST/EDT)' },
  { value: 'PST8PDT,M3.2.0,M11.1.0', label: 'Los Angeles (PST/PDT)' },
  { value: 'CST-8', label: 'China (CST)' },
  { value: 'JST-9', label: 'Japan (JST)' },
];
```

Add a test asserting every `value` round-trips through the picker and that the
default selection matches the value returned by `getSystemDateAndTime`.

**Verify and commit**

```bash
cd cross-compile/www && npx vitest run src/pages/settings/TimePage.test.tsx
git add cross-compile/www/src/pages/settings/
git commit -m "fix(www): use POSIX timezone strings with DST rules"
```

Note: `rtk prettier --check` has reported "All files formatted correctly" on a
real exit-1. Run the raw binary and read `$?`.

---

## Task 15: Full Gate and Hardware Verification

**Step 1: Host gate**

```bash
cd cross-compile
$CARGO fmt --check
$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
$CARGO test --target x86_64-unknown-linux-gnu
cd www && npx vitest run && npx tsc --noEmit
```

**Step 2: ARM build** — from `cross-compile/onvif-rust/`, not the workspace root:

```bash
cd cross-compile/onvif-rust && $CARGO build --release
```

Build `anyka-init` and `snmp-agent` the same way, from their own crate dirs.

**Step 3: Deploy** via the `anyka-firmware-upgrade` skill. Remember that a new
or changed config stanza is not carried by bundles where `anyka.toml` is
concerned; `[time]` lives in onvif's `config.toml`, which **is** in the bundle
as `config.template.toml` and will not overwrite the live file — so on first
deploy, add the `[time]` block to the live `/mnt/anyka_hack/onvif/config.toml`
by hand.

**Step 4: Verify on hardware**

1. Overlay shows local time, not UTC.
2. Change the zone in the WebUI; the overlay follows within ~1 s, with no restart.
3. `/mnt/logs/onvif.log` carries the new offset immediately.
4. `/mnt/logs/vendor_daemon.log` carries it after the daemon restarts.
5. `GetSystemDateAndTime` returns the real TZ and a `LocalDateTime` that differs from UTC.
6. Set `Manual` mode, confirm `state/ntp.disabled` exists, reboot, and confirm the clock was **not** stepped back by the boot sync.
7. Set `NTP` mode, confirm the marker is gone and the clock resyncs.

Check the logs with an explicit level — the shipped default filters warnings,
so a `grep -c` returning 0 proves nothing on its own.

---

## Out of Scope

`onvif-rust` runs from `slots/<active>/onvif/` but reads its config from the
flat `/mnt/anyka_hack/onvif/config.toml`. Pre-existing; see the design doc.
