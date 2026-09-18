# Timezone Follow-Up: Correctness Fix, Dead Code, and Timer Shrink

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Fix one correctness defect in `SetSystemDateAndTime`, delete dead code, shrink two duplicated log timers by ~75 lines each, and close two test gaps left by `docs/plans/2026-09-18-osd-timezone.md`.

**Architecture:** No new files, no new dependencies, no design changes. Every task edits existing code that already builds and passes.

**Tech Stack:** Rust 2024 (vendored toolchain), libc, tracing-subscriber.

**Prior work:** `docs/plans/2026-09-18-osd-timezone-design.md` (design), `docs/plans/2026-09-18-osd-timezone.md` (the implemented plan).

---

## Read This First

You are editing working code. The full gate is currently **green**: host tests,
clippy, fmt, vitest, tsc, prettier, and ARM release builds for `onvif-rust`,
`anyka-init` and `snmp-agent` all pass. Your job is to keep it green.

Task 1 is a **correctness fix and is the only must-do**. Tasks 2-4 are cleanup.
Tasks 5-6 are small gaps. Do them in order. Commit after each.

Do **not** redesign anything. Do **not** extract a shared crate for the
duplicated timer in Tasks 3-4 — two copies of ~18 lines is deliberate; see the
design doc's "Rejected alternatives".

## Toolchain Setup (run this first, once per shell)

```bash
export PATH=/home/kmk/dev/anyka-dev/toolchain/arm-anykav200-crosstool-ng/bin:$PATH
export CARGO=/home/kmk/dev/anyka-dev/toolchain/arm-anykav200-crosstool-ng/bin/cargo
cd /home/kmk/dev/anyka-dev/cross-compile
```

The `PATH` prefix is **required**: without it clippy dies with `E0514`.

Commands you will reuse:

```bash
$CARGO test   --target x86_64-unknown-linux-gnu -p <crate>
$CARGO clippy --target x86_64-unknown-linux-gnu --all-targets -- -D warnings
$CARGO fmt
```

**Committing:** this repo's git index is usually fully staged (`M` appears in
the FIRST column of `git status`). A bare `git commit` sweeps in unrelated
binary changes. **Always pass an explicit pathspec to `git add`**, exactly as
written in each task.

End every commit message with:

```
Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
```

---

## Task 1: Reject Out-of-Range Clock Values Instead of Masking Them

**This is the must-fix.** `SetSystemDateAndTime` currently masks the timestamp
instead of validating it, so an out-of-range date silently sets the camera
clock to a wrong time **and returns success**. Measured:

```
2039-01-01   raw=2177452800   masked-> 29969152  (= 1970-12-14)
1950-01-01   raw=-631152000   masked->1516331648 (= 2018-01-19)
```

A wrong clock is not cosmetic here: `ws_security` rejects every authenticated
ONVIF request once skew exceeds ±300 s, so a bad manual set can lock the
operator out of the camera.

**Files:**
- Modify: `cross-compile/onvif-rust/src/onvif/device/ops/system.rs` (around line 303)

**Step 1: Write the failing test**

Add to the `mod tests` block in `system.rs`, next to the other
`test_set_system_date_and_time_*` tests:

```rust
#[test]
fn test_set_system_date_and_time_rejects_a_year_past_2038() {
    let _guard = tz_test_guard();
    let cfg = ConfigRuntime::new(AppConfig::default());
    let mut req = req_with_tz("UTC0");
    req.date_time_type = SetDateTimeType::Manual;
    req.utc_date_time = Some(DateTime {
        date: Date { year: 2039, month: 1, day: 1 },
        time: Time { hour: 0, minute: 0, second: 0 },
    });
    let err = handle_set_system_date_and_time(&cfg, req).unwrap_err();
    assert!(
        format!("{err:?}").contains("out of range"),
        "expected an out-of-range fault, got {err:?}"
    );
}
```

`tz_test_guard()` already exists in this test module (it serialises the global
tz cell). Use the same helper the neighbouring tests use — copy their first
line verbatim. If the neighbouring tests name it differently, match them.

**Step 2: Run it and confirm it FAILS**

```bash
$CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust --lib \
  system::tests::test_set_system_date_and_time_rejects_a_year_past_2038
```

Expected: **FAIL**. The current code masks 2039 into 1970 and returns `Ok`, so
`unwrap_err()` panics. If it passes, stop and re-read the file — you are not
looking at the code this task describes.

**Step 3: Make the change**

Find this in `handle_set_system_date_and_time`:

```rust
            // uclibc's time_t is 32-bit on the camera (1901..=2038, which
            // covers the camera's lifetime); host glibc is 64-bit. The u64
            // intermediate keeps the final cast a real type change on both.
            let tv_sec: libc::time_t = (dt.timestamp() as u64 & 0x7FFF_FFFF) as libc::time_t;
```

Replace it with:

```rust
            // uclibc's time_t is 32-bit on the camera, 64-bit on host glibc.
            // A checked conversion rather than a mask: masking turned a 2039
            // request into 1970 and still answered Ok, which silently breaks
            // ws_security's ±300s skew check.
            let tv_sec: libc::time_t = dt.timestamp().try_into().map_err(|_| {
                OnvifError::invalid_arg(
                    "ter:InvalidArgVal",
                    "utc_date_time out of range for this platform",
                )
            })?;
```

Leave the `let ts = libc::timespec { tv_sec, tv_nsec: 0 };` line and everything
after it exactly as it is.

**Step 4: Run and confirm it PASSES**

```bash
$CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust --lib system::tests
```

Expected: all tests in that module pass, including the new one.

**Step 5: Commit**

```bash
cd /home/kmk/dev/anyka-dev
git add cross-compile/onvif-rust/src/onvif/device/ops/system.rs
git commit -m "fix(onvif): reject out-of-range clock values instead of masking

A masked timestamp turned a 2039 request into 1970 and still answered Ok.
A wrong clock breaks ws_security's 300s skew check, locking the operator out.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

## Task 2: Delete the Unused NtpMarker in anyka-init

`anyka-init` only ever **reads** the marker: production code calls
`ntp_disabled_marker_path` (`main.rs:81` and `main.rs:213`) and `sync_once`
takes a `&Path`. The `NtpMarker` struct's only consumer is its own test.
`onvif-rust` owns the writing side (`onvif-rust/src/time/ntp_marker.rs`), which
stays.

**Files:**
- Modify: `cross-compile/anyka-init/src/timesync.rs`

**Step 1: Confirm it is really unused**

```bash
cd /home/kmk/dev/anyka-dev/cross-compile
grep -rn "NtpMarker" anyka-init/src/
```

Expected: hits **only** in `timesync.rs` (the definition around line 190 and a
test around line 566). If you see a hit in any other file, **stop** — the
premise is wrong, skip this task and say so in your report.

**Step 2: Delete**

In `cross-compile/anyka-init/src/timesync.rs`, delete:

- the `pub struct NtpMarker { .. }` declaration and its doc comment
- the entire `impl NtpMarker { .. }` block (`new`, `ntp_enabled`, `disable`, `enable`)
- the test that exercises it (the one calling `NtpMarker::new`, around line 566)

**Keep** `pub fn ntp_disabled_marker_path(update_root: &Path) -> PathBuf` and
the test that uses it — production depends on both. Move the doc comment about
why this is a bare filename (`update.rs:143`) onto
`ntp_disabled_marker_path`, so the reasoning is not lost.

If the `PathBuf` import becomes unused, remove it; clippy will tell you.

**Step 3: Verify**

```bash
$CARGO test   --target x86_64-unknown-linux-gnu -p anyka-init
$CARGO clippy --target x86_64-unknown-linux-gnu --all-targets -- -D warnings
```

Both must pass.

**Step 4: Commit**

```bash
cd /home/kmk/dev/anyka-dev
git add cross-compile/anyka-init/src/timesync.rs
git commit -m "refactor(timesync): drop the write-side NtpMarker anyka-init never uses

Production only reads the path; onvif-rust owns writing the marker.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

## Task 3: Shrink the anyka-init Log Timer

The timer is 93 lines and hand-assembles the UTC offset because of a comment
claiming `%:z` is needed. It is not: **`%z` is standard C89/POSIX `strftime`**
and produces `+0100`. Only `%:z` (with the colon) is a GNU extension. One
`strftime` call replaces the manual assembly, the `String` round-trip, and the
`format_now(buf: &mut [u8]) -> usize` API.

Microseconds are dropped deliberately. This is a supervisor boot log; nothing
reads it at sub-second resolution.

**Files:**
- Modify: `cross-compile/anyka-init/src/logging.rs`

**Step 1: Replace the implementation**

Delete the whole `impl LocalTimer { pub fn format_now(..) .. }` block and the
existing `impl .. FormatTime for LocalTimer`. Replace both with:

```rust
/// Timestamps log lines in the process's zone, which `boot.rs` sets from `TZ`.
///
/// libc rather than chrono: this crate has no chrono dependency and is not
/// gaining one for a timestamp. `%z` is standard C89 strftime (`+0100`); only
/// `%:z` is a GNU extension, so no hand-assembly is needed.
pub struct LocalTimer;

impl tracing_subscriber::fmt::time::FormatTime for LocalTimer {
    fn format_time(&self, w: &mut tracing_subscriber::fmt::format::Writer<'_>) -> std::fmt::Result {
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        // time_t is 32-bit on the camera's uclibc, 64-bit on host glibc.
        let t: libc::time_t = secs.try_into().unwrap_or(libc::time_t::MAX);
        let mut tm: libc::tm = unsafe { std::mem::zeroed() };
        let mut out = [0u8; 32];
        // SAFETY: localtime_r fills our own `tm` and never reads it
        // uninitialised; on a broken clock it returns null and the zeroed
        // struct (1970) is formatted instead. strftime bounds its write by
        // out.len() and the format string is a NUL-terminated literal.
        let n = unsafe {
            libc::localtime_r(&t, &mut tm);
            libc::strftime(
                out.as_mut_ptr().cast::<libc::c_char>(),
                out.len(),
                c"%Y-%m-%dT%H:%M:%S%z".as_ptr(),
                &tm,
            )
        };
        write!(w, "{}", String::from_utf8_lossy(&out[..n]))
    }
}
```

**Step 2: Move `tzset` out of the per-line path**

The old code called `tzset()` on **every log line**. `TZ` is fixed for the life
of this process, so one call is enough — this is what
`vendor-daemon/src/main.c:167` already does.

In `pub fn init(..)`, immediately before the `tracing_subscriber::registry()`
call, add:

```rust
    // Parse TZ once. glibc caches the zone and will not re-read a changed TZ
    // on its own; the supervisor's zone never changes after boot.
    // Declared locally: the libc crate exposes tzset only on Windows.
    // SAFETY: tzset reads the process env and updates libc's own state.
    unsafe extern "C" {
        fn tzset();
    }
    unsafe { tzset() };
```

**Step 3: Fix the test**

The existing test calls `LocalTimer::format_now(&mut buf)`, which no longer
exists. Rewrite it to go through `FormatTime`:

```rust
#[test]
fn test_local_timer_uses_the_process_tz() {
    use tracing_subscriber::fmt::time::FormatTime;
    // SAFETY: single-threaded test; restore UTC at the end so no later test
    // in this binary sees a shifted zone.
    unsafe { std::env::set_var("TZ", "CET-1CEST,M3.5.0,M10.5.0/3") };
    unsafe extern "C" {
        fn tzset();
    }
    unsafe { tzset() };

    let mut s = String::new();
    LocalTimer
        .format_time(&mut tracing_subscriber::fmt::format::Writer::new(&mut s))
        .unwrap();
    assert!(
        s.ends_with("+0100") || s.ends_with("+0200"),
        "expected a CET/CEST offset, got {s}"
    );

    unsafe { std::env::set_var("TZ", "UTC") };
    unsafe { tzset() };
}
```

**Step 4: Verify**

```bash
$CARGO test   --target x86_64-unknown-linux-gnu -p anyka-init
$CARGO clippy --target x86_64-unknown-linux-gnu --all-targets -- -D warnings
$CARGO fmt
```

If the test fails because the offset is absent, your `%z` is not being honoured
— print `s` and check you used `c"..."` (a C string literal), not `"..."`.

**Step 5: Commit**

```bash
cd /home/kmk/dev/anyka-dev
git add cross-compile/anyka-init/src/logging.rs
git commit -m "refactor(logging): fold the supervisor timer into one strftime call

%z is standard C89 strftime, so the hand-assembled offset, the byte-buffer
API and the per-line tzset were all unearned.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

## Task 4: Shrink the snmp-agent Log Timer

`snmp-agent/src/main.rs` holds a byte-identical copy of the 93-line timer.
Apply exactly the same change.

**Files:**
- Modify: `cross-compile/snmp-agent/src/main.rs`

**Step 1:** Replace `LocalTimer` with the same implementation as Task 3, Step 1,
adjusting only the doc comment's first line to say `snmp-agent` and noting it
is duplicated from `anyka-init/src/logging.rs` on purpose.

**Step 2:** Call `tzset()` once in `main()`, immediately before
`tracing_subscriber::fmt()`. Same local `extern "C"` block as Task 3.

**Step 3:** Update the test the same way as Task 3, Step 3.

**Step 4: Verify**

```bash
$CARGO test   --target x86_64-unknown-linux-gnu -p snmp-agent
$CARGO clippy --target x86_64-unknown-linux-gnu --all-targets -- -D warnings
```

**Step 5: Commit**

```bash
cd /home/kmk/dev/anyka-dev
git add cross-compile/snmp-agent/src/main.rs
git commit -m "refactor(snmp-agent): fold the log timer into one strftime call

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

## Task 5: Test That the WebUI's Timezones Parse on the Camera

`cross-compile/www/src/utils/timezones.ts` offers 8 POSIX strings to the
operator. Nothing asserts the camera's parser accepts them. A typo there would
ship a zone the camera refuses, and no test would catch it.

All 8 currently parse — this closes the gap, it does not fix a bug.

**Files:**
- Modify: `cross-compile/onvif-rust/src/time/tz.rs`

**Step 1: Add the test**

Append to the `mod tests` block in `tz.rs`:

```rust
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
        assert!(parse(tz).is_ok(), "WebUI offers {tz:?}, which the camera rejects");
    }
}
```

**Step 2: Verify the list still matches the WebUI**

```bash
cd /home/kmk/dev/anyka-dev/cross-compile
grep "value:" www/src/utils/timezones.ts
```

Every `value:` must appear in the test array. If they differ, **use the values
from `timezones.ts`** — that file is the source of truth for this test.

**Step 3: Run**

```bash
$CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust --lib time::tz
```

Expected: PASS. If any zone fails, **do not delete it from the test** — report
which one, because that means the WebUI is offering a broken value.

**Step 4: Commit**

```bash
cd /home/kmk/dev/anyka-dev
git add cross-compile/onvif-rust/src/time/tz.rs
git commit -m "test(time): assert every WebUI timezone parses on the camera

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

## Task 6: Log Why NTP Was Skipped

`sync_once` returns `None` silently when the manual-clock marker is present.
The camera's default log level is `error`, so a field diagnosis of "why is the
clock not syncing" currently has nothing to find.

**Files:**
- Modify: `cross-compile/anyka-init/src/timesync.rs` (the guard at the top of `sync_once`)

**Step 1: Add the line**

```rust
    if ntp_disabled.is_file() {
        tracing::info!(
            marker = %ntp_disabled.display(),
            "NTP disabled by operator (manual clock); not stepping the clock"
        );
        return None;
    }
```

**Step 2: Verify**

```bash
$CARGO test --target x86_64-unknown-linux-gnu -p anyka-init
```

**Step 3: Commit**

```bash
cd /home/kmk/dev/anyka-dev
git add cross-compile/anyka-init/src/timesync.rs
git commit -m "feat(timesync): say why an NTP sync was skipped

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

## Task 7: Full Gate

Run everything. All must exit 0.

```bash
cd /home/kmk/dev/anyka-dev/cross-compile
$CARGO fmt --check                                              && echo FMT_OK
$CARGO clippy --target x86_64-unknown-linux-gnu --all-targets -- -D warnings && echo CLIPPY_OK
$CARGO test --target x86_64-unknown-linux-gnu                   && echo TEST_OK
```

WebUI (unchanged by this plan, but confirm nothing regressed). **Run the raw
binaries, not `npx`** — `npx` is intercepted here and has reported success on a
real failure:

```bash
cd /home/kmk/dev/anyka-dev/cross-compile/www
./node_modules/.bin/tsc --noEmit        ; echo "tsc exit: $?"
./node_modules/.bin/prettier --check src/ ; echo "prettier exit: $?"
```

ARM cross-builds. CI never does these — `armv5te` lives only in `release.yml` —
so a break here reaches `main` unnoticed. Each **must** run from its own crate
directory, not the workspace root, or cargo silently links with the host
toolchain:

```bash
cd /home/kmk/dev/anyka-dev/cross-compile/onvif-rust  && $CARGO build --release; echo "onvif: $?"
cd /home/kmk/dev/anyka-dev/cross-compile/anyka-init  && $CARGO build --release; echo "init: $?"
cd /home/kmk/dev/anyka-dev/cross-compile/snmp-agent  && $CARGO build --release; echo "snmp: $?"
```

---

## Report Back

State, with the actual command output:

1. Which tasks you completed, and any you skipped with the reason.
2. The exit code of every command in Task 7.
3. Whether Task 2's grep confirmed `NtpMarker` was unused.
4. Whether any zone in Task 5 failed to parse.

Do **not** claim success without pasting the output. If a gate fails, stop and
report rather than working around it.

---

## Explicitly Out of Scope

- **Extracting a shared crate** for the duplicated timer. Two ~18-line copies
  is the accepted trade; see the design doc.
- **The `0x7FFF_FFFF` mask in the two log timers.** After Task 3/4 it is gone
  from the timers anyway. Post-2038 log stamps are inherent on 32-bit `time_t`
  and not worth further work.
- **The supervisor/child TZ fallback mismatch.** `boot.rs:236` falls back to
  `anyka.toml`'s zone for the supervisor, while `supervisor_loop.rs:75` passes
  `None` to children so they keep UTC. This is a real inconsistency on a camera
  whose onvif config lacks `[time]`, but it needs a decision about which
  behaviour is right, so it is not a mechanical fix. Leave it; it is filed for
  a later session.
- **Hardware verification.** Deploying and checking the overlay on a real
  camera belongs to whoever runs the rollout, not to this plan.
