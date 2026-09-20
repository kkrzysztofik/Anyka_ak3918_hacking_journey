# The Time settings tab is mostly decorative

Date: 2026-09-20
Status: design approved, implementation not started

Successor to `docs/plans/2026-09-18-osd-timezone-design.md`, which built the
POSIX TZ parser, the `TIMEZONES` list and the `state/ntp.disabled` marker but
deliberately left the WebUI page, the NTP server list and sync status alone.

## Problem

`www/src/pages/settings/TimePage.tsx` renders a complete-looking Time tab:
a live clock, three sync modes, NTP server fields, a timezone picker. Most of
it is not connected to the camera.

The page can show a plausible clock while the camera's own clock sits at 1970,
and saving on a healthy NTP camera silently switches it to Manual.

## What is actually wired

Worth stating, because the backend is in better shape than the page suggests.

- `anyka-init/src/timesync.rs` is a real SNTP client: server list, step
  threshold, plausibility window, first-sync timeout, resync loop, and it
  honours the marker on every iteration.
- `onvif-rust` `handle_set_system_date_and_time` validates the POSIX TZ before
  any partial apply, steps the clock via `clock_settime` with a checked 32-bit
  `time_t` conversion, and toggles the marker.

## The gaps

| # | Gap | Location |
|---|-----|----------|
| 1 | "Device Time" is the **browser's** clock. The camera UTC from `getDateTime()` is fetched and discarded | `TimePage.tsx:58-73` |
| 2 | `GetSystemDateAndTime` hardcodes `date_time_type: Manual`; never reads `NtpMarker` | `system.rs:234` |
| 3 | Timezone never persists — no `request_save()`, unlike every sibling setter | `service.rs:422` |
| 4 | NTP server inputs are fiction: `pool.ntp.org`/`time.google.com` hardcoded in defaults *and* in `form.reset` | `TimePage.tsx:80,97` |
| 5 | `SetNTP` returns `ActionNotSupported` | `network.rs:507` |
| 6 | `GetNTP` returns empty on-camera: parses `/etc/ntp.conf` and `timesyncd.conf`, neither of which exists | `network_info.rs:271` |
| 7 | `fromDHCP: true` is a literal stub | `timeService.ts:118` |
| 8 | `DaylightSavings` always sent `false`, though the TZ string carries the rules | `timeService.ts:143` |
| 9 | `ntp_from_dhcp`/`ntp_primary`/`ntp_secondary` read by nothing | `types.rs:436-438` |
| 10 | "Computer" is a one-shot action shaped like a persistent mode; it round-trips back as Manual | `TimePage.tsx:140` |

Gaps 1–3 are the dangerous ones. The rest are cosmetic or dead weight.

Gap 1 is worse than cosmetic on this hardware. `ws_security` rejects requests
outside ±300 s skew, and `anyka.toml`'s own comment records the failure chain:
empty `resolv.conf` → hostname-only NTP never syncs → clock at 1970 → every
authenticated ONVIF call fails. The Time page is the one screen that should
make that visible and is currently guaranteed to hide it.

## Ownership model

Every piece of "time" has a different owner. Naming one owner each, and
letting no second process write it, is the whole design.

| Item | Single owner | How others reach it |
|------|--------------|---------------------|
| NTP servers | `anyka.toml [time].servers`, written **only** by anyka-init | control socket verb |
| Clock mode | `{update_root}/state/ntp.disabled` | already shared |
| Wall clock | `clock_settime` — onvif-rust for Manual, anyka-init for NTP | — |
| Sync status | `{update_root}/state/ntp.status` | plain file read |
| ONVIF/OSD timezone | onvif-rust's own config | — |

`onvif-rust` never opens `anyka.toml`. It sends `set-ntp` over
`/tmp/anyka-supervisor.sock` and the supervisor persists — the same shape as
`handle_toggle_service` and `[system].telnet`, reusing the client in
`diagnostics/services.rs` that already has a mock-server test suite.

No new `anyka.toml` keys: `servers` already exists in `TimeCfg`, so the older
anyka-init in the other A/B slot still parses the file. Sync status lives in
`state/`, not the config, specifically to keep that true.

## Design

### anyka-init

- Generalize `set_bool_in_text` into `set_value_in_text(text, section, key, raw)`
  taking a pre-encoded TOML scalar or array. It already preserves comments,
  indentation and CRLF, and re-parses via `verified()`. `set_bool_in_text`
  becomes a one-line wrapper so its two callers do not change.
- `Config::set_time_servers` → `persist_text`, exactly as `set_service_enabled`.
- One new control verb: `set-ntp <s1> <s2>…`. `handle_set_time` writes the file
  first, then in-memory state, matching `handle_toggle_service`'s visible order.
- `resync_loop` takes the config path and re-parses `[time]` each iteration,
  keeping the previous value on error. No shared state, no threading changes.
- `resync_loop` writes `state/ntp.status` after each attempt, one TSV line:
  `last_unix \t server \t delta_s \t s1,s2,…`. Also written when the thread
  starts, so the server list is known before the first sync completes.
  With `[time] enabled = false` there is no thread and no file; the WebUI then
  honestly reports NTP as off.

### onvif-rust

Five edits, no new infrastructure.

1. `handle_get_system_date_and_time` — report NTP/Manual from
   `NtpMarker::ntp_enabled()` instead of the hardcoded `Manual`. (gap 2)
2. `service.rs:422` — add the missing `request_save()`. (gap 3)
3. `handle_set_ntp` — validate, then send `set-ntp`. Drop
   `ActionNotSupported`. (gap 5)
4. `get_ntp_info` — read `state/ntp.status`; keep the `/etc/ntp.conf` and
   `timesyncd.conf` parsers for the stub platform only. `from_dhcp` becomes an
   honest `false`: udhcpc here never supplies NTP. (gaps 6, 7)
5. Delete `NetworkConfig.ntp_from_dhcp/ntp_primary/ntp_secondary` (gap 9); add
   a `time` field to the existing `/api/diagnostics` snapshot.

### WebUI

- Tick from `offsetMs = cameraUtc − Date.now()` captured at fetch, render in
  the camera's zone, and turn red below year 2020. (gap 1)
- Two mode cards, NTP and Manual, driven by the real `dateTimeType`.
  "Use this computer's time" becomes a button inside the Manual card that
  fills the date and time inputs. (gap 10)
- Servers: a `<textarea>`, one per line. The "NTP from DHCP" switch is deleted
  outright — it is structurally false here and nothing can make it true.
- Sync status read from the diagnostics snapshot.
- `timeService.ts`: real `getNtp`/`setNtp`, drop the `fromDHCP` stub and both
  hardcoded server strings, stop sending `DaylightSavings: false`. (gaps 4, 8)

## Trust boundary

Server strings land in a TOML array inside the operator's hand-edited file.
`handle_set_ntp` rejects quotes, whitespace and newlines before the socket
call, and `set_value_in_text` re-parses after writing via the existing
`verified()`. This is the one place in the change worth spending code on.

## Rejected alternatives

- **onvif-rust writes `anyka.toml` directly.** Needs `set_value_in_text` and
  `persist_text` duplicated across crates, and puts two writers on one file.
  The control socket already exists and is tested.
- **`Arc<RwLock<TimeShared>>` + generation counter + chunked sleep**, to apply
  server changes instantly. Invented threading for a rare config action; the
  per-iteration re-parse gets the same result in four lines.
- **`set-timezone` control verb.** `anyka.toml [time].timezone` affects only
  the supervisor's own log timestamps — `Sys::spawn` calls `env_clear` and
  onvif-rust carries its own zone. Pushing it changes nothing a user sees.
- **`GET /api/time` + `POST /api/time/sync`.** A new route, state struct and
  Extension layer for one field the diagnostics snapshot already carries.
- **`useFieldArray` for the server list.** A textarea is three lines and does
  not truncate a three-server config the way two fixed inputs would.

## Consequences

- New servers apply within 6 h (the next resync) rather than instantly, with
  no reboot. Nothing degrades meanwhile: the clock stays synced on the old
  servers. Stated in the UI, and marked with a `ponytail:` comment on the
  re-parse naming the ceiling.
- The WebUI timezone affects onvif-rust only. anyka-init's log timestamps keep
  using whatever `anyka.toml` says, so the two can visibly disagree in logs.
- anyka-init does not validate POSIX TZ with onvif-rust's parser, and its
  default is `"GMT+00:00"` where the WebUI list uses `"UTC0"`. Both parse, and
  onvif-rust validates before send, so only a hand-edited `anyka.toml` can
  hold a string onvif-rust rejects.

## Testing

Trimmed to checks that fail if the logic breaks.

- `set_value_in_text` array round-trip with comments and CRLF preserved.
- `handle_set_ntp` rejects quotes, whitespace and newlines in server strings.
- `GetSystemDateAndTime` reports NTP with the marker absent, Manual with it
  present.
- WebUI: fake timers plus a camera UTC three hours off — the panel must show
  the camera's time, not the browser's.
- On-device: set a bogus manual time and confirm the panel shows the camera's
  wrong time rather than the browser's right one. This is the check that
  proves gap 1 is really dead.
