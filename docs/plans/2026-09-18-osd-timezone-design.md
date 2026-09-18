# OSD and log timestamps ignore the configured timezone

Date: 2026-09-18
Status: design approved, implementation not started

## Problem

The date/time overlay burned into the video shows UTC. So does every log file
on the camera. Setting a timezone in the WebUI does nothing.

## Root cause

`osd/renderer.rs:359` calls `format_datetime(Local::now(), ..)`. The code is
correct; `Local` is what resolves wrongly on this hardware.

- `anyka-init/src/boot.rs:245` sets `TZ`, but only for the supervisor process.
  The comment above it says so explicitly.
- `anyka-init/src/sys.rs:128` calls `.env_clear()` when spawning services, so
  `onvif-rust` starts with no `TZ`.
- chrono then falls back to reading `/etc/localtime`
  (`chrono-0.4.45/src/offset/local/tz_info/timezone.rs:43`). That file does not
  exist on this rootfs, so chrono returns UTC rather than an error.

The failure is silent by design, which is why nothing ever flagged it.

Three further defects share the cause:

- `onvif/device/ops/system.rs:213-221` hardcodes `tz: "UTC"` and reports local
  time as equal to UTC.
- `system.rs:234` returns `ActionNotSupported` for `SetSystemDateAndTime`,
  while `www/src/pages/settings/TimePage.tsx` ships a full timezone picker
  that calls it.
- No Rust binary sets `with_timer`, so tracing-subscriber's default
  `SystemTime` timer stamps every log line as RFC3339 UTC.

## Scope

Full: the timezone is settable at runtime via ONVIF and the WebUI, `Manual`
mode sets the wall clock and suspends NTP, and log timestamps across the
binaries follow the configured zone.

## Design

### 1. Authority and storage

`onvif-rust`'s `config.toml` gains a `[time]` section and is the single source
of truth for the timezone:

```toml
[time]
timezone = "CET-1CEST,M3.5.0,M10.5.0/3"   # serde default: "UTC"
```

This survives firmware upgrades: `scripts/build_bundle.sh:66` copies
`config.toml` into the bundle as `config.template.toml` precisely so it cannot
overwrite the live file, and `app.rs:626` reads the absolute flat path
`/mnt/anyka_hack/onvif/config.toml`, which the supervisor's `slot_path`
rewrite never touches.

NTP suspension is recorded as the *existence* of `{update.root}/state/ntp.disabled`,
following the `Trial` marker convention documented at `update.rs:143`: a
filename cannot be half-parsed, which matters on exFAT after a power cut.

### 2. POSIX TZ parser — `onvif-rust/src/time/tz.rs` (new)

```rust
pub struct PosixTz { std: FixedOffset, dst: Option<(FixedOffset, Rule, Rule)> }
pub fn parse(s: &str) -> Result<PosixTz, &'static str>;
impl PosixTz { pub fn offset_at(&self, utc: DateTime<Utc>) -> FixedOffset; }
```

Supports `STDoffset[DST[offset][,start[/time],end[/time]]]` with `Mm.w.d`
transition dates — the form every real zone uses. The `Jn` and `n` day forms
are rejected rather than silently misparsed.

A POSIX TZ string is self-contained: the DST rules travel inside the string, so
no zoneinfo database is needed on a 36 MB rootfs. This is also the format
ONVIF specifies for `TimeZone/TZ`, and the format `anyka.toml` already uses.

chrono has an equivalent parser but it is crate-private (`tz_info`).

### 3. Consumers in onvif-rust

`format_datetime` is already generic over `Tz` and needs no change. Both
existing `Local::now()` call sites move to one shared accessor on the runtime
config:

- `osd/renderer.rs:359` — the visible bug
- `logging/static_assets.rs:122`

Persistence is free: mutating `config.write()` bumps the generation and the
debounced, off-executor persistence service (`config/persistence.rs:96`)
flushes it. The renderer ticks once a second (`renderer.rs:264`), so a
timezone change appears on the overlay within 1 s without a restart.

### 4. ONVIF

- `GetSystemDateAndTime` (`system.rs:192`): report the real TZ, a real
  `LocalDateTime`, `DaylightSavings` derived from `offset_at`, and
  `DateTimeType` from the marker file.
- `SetSystemDateAndTime` (`system.rs:234`): validate and apply the TZ, persist
  it. For `Manual`, also step the clock and create `state/ntp.disabled`. For
  `NTP`, remove the marker.

### 5. Supervisor

- `timesync.rs:192` — guard the top of `sync_once`: if `state/ntp.disabled`
  exists, skip and log why. `sync_once` is the single chokepoint called by both
  `first_sync` (P2.5 boot) and `resync_loop`, so one guard covers both. A guard
  at the `set_realtime` call site instead would let a reboot silently
  re-clobber a manually set clock.
- `sys.rs:128` — add `.env("TZ", tz)` to the spawn.
- TZ source at boot: read `[time].timezone` from
  `/mnt/anyka_hack/onvif/config.toml`, falling back to `anyka.toml`'s value on
  any read or parse error. A malformed service config must never block boot.
- `boot.rs:235-246` — the comment asserting that services do not inherit `TZ`
  becomes false and is rewritten.

### 6. Log timestamps

| Binary | Change | Mechanism |
|---|---|---|
| vendor-daemon | none | already calls `localtime_r` (`log.c:88`); only lacked `TZ` |
| onvif-rust | `FormatTime` on the live `PosixTz` | follows a runtime change, stays consistent with the OSD |
| anyka-init | `FormatTime` via `libc::localtime_r` | uses the `TZ` it already sets at `boot.rs:245` |
| snmp-agent | same, plus `libc = "0.2"` | `libc` is already a tokio dependency, so no extra compilation |

`onvif-rust` deliberately does not use the `TZ` env var for its own stamps:
env is fixed at spawn, so env-based timestamps would disagree with the OSD the
moment an operator changed the zone. The other binaries are boot-constant, so
the env route is correct for them.

The ~15-line `localtime_r` timer is duplicated in `anyka-init` and
`snmp-agent` rather than extracted; two call sites do not justify a shared
crate.

### 7. WebUI

`TimePage.tsx:46`'s self-described "Stub list" of seven bare abbreviations is
replaced with POSIX TZ strings carrying DST rules (`CET` becomes
`CET-1CEST,M3.5.0,M10.5.0/3`). Bare `CET` would be an hour wrong in Poland
from March to October. The manual date/time inputs at lines 127 and 132 stay
and now work.

## Testing

Host-side:

- parser across DST boundaries in both directions, including the spring-forward
  02:00-03:00 gap where naive `Mm.w.d` implementations break
- non-DST zones, and malformed input rejected rather than defaulted
- `sync_once` skips when the marker exists, syncs when it does not
- `SetSystemDateAndTime` accept and reject paths; `GetSystemDateAndTime`
  reporting a non-UTC zone

On hardware:

- change the zone in the WebUI, confirm the overlay text changes without a
  restart
- reboot, confirm the zone survived and that a `Manual` clock was not stepped
  back by the boot sync

## Risks and limits

- A WebUI timezone change is immediate for the OSD, ONVIF responses and
  `onvif.log`, but reaches `vendor_daemon.log` and the supervisor only at their
  next restart. Writing `anyka.toml` from a SOAP handler was rejected as the
  larger risk: a bad write costs a boot.
- `state/` lives on the SD card. If the card drops off the bus the marker
  vanishes and NTP re-enables — the fail-safe direction.
- `step_threshold_sec = 2` means a manual clock within 2 s of NTP is
  indistinguishable from a synced one. Cosmetic.

## Out of scope

`onvif-rust` runs from `slots/<active>/onvif/` but reads its config from the
flat `/mnt/anyka_hack/onvif/config.toml`. That mismatch is pre-existing and is
what the `# ... which is the DEBUG variant already on this card` comment at
`anyka.toml:119` hints at. Worth its own investigation.

## Rejected alternatives

- **`unsafe { set_var("TZ") }` + `Local::now()`** — one line, but setting env
  from a SOAP handler races any concurrent `getenv` in a multi-threaded tokio
  process. Safe only at startup, which defeats runtime settability.
- **`chrono-tz` with IANA names** — no parser to write, but a new dependency
  carrying a tz database, and it sends `Europe/Warsaw` where ONVIF specifies a
  POSIX string.
- **Fixed UTC offset, no DST** — about ten lines, wrong by an hour for half
  the year.
- **A shared `state/time.toml`** — proposed and cut. It was justified by the
  belief that upgrades overwrite `config.toml`; `build_bundle.sh:66` shows they
  do not. It would also have put parsed state on exFAT, against the convention
  at `update.rs:143`.
- **A four-level fallback chain** — cut to two; a serde default covers the
  rest.
