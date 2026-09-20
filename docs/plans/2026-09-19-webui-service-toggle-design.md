# WebUI Service Enable/Disable — Design

- **Date:** 2026-09-19
- **Extends:** `2026-09-19-webui-process-control-design.md` (control socket, `/api/processes`, ProcessesCard)
- **Status:** approved in brainstorming session 2026-09-19; revised 2026-09-19 after a code review against `monitor.rs`, `netoverlay.rs` and `supervisor_loop.rs`
- **Branch:** `design/webui-process-control` (or a successor)

## Motivation and scope

The camera's supervised services (`udhcpc`, `wpa_supplicant`, `vendor-daemon`,
`onvif`, `snmp`, `dropbear`) are all-or-nothing today: `anyka.toml` declares
them, the supervisor starts every enabled one, and a crash-looping service
reboots the whole camera via the crash-loop cap. An admin with only the
browser — the normal case on a Wi-Fi camera whose telnet/FTP access is
locked down — cannot stop a misbehaving service.

This feature adds **enable/disable for any supervised service from the
Diagnostics → Processes card**, serving two uses with one mechanism:

1. **Crash-loop escape hatch** — a service keeps crashing; disabling it stops
   the reboots without shell or SD-card access.
2. **General admin toggle** — e.g. disable `udhcpc` when moving to static
   addressing (the shipped config's own comment prescribes exactly this),
   or stop `snmp` polling.

**Decisions recorded in the brainstorming session:**

- Every service is toggleable **except `wpa_supplicant`** (see §2.1); each
  carries its own consequence warning in the confirm dialog.
- State persists in `/mnt/anyka_hack/anyka.toml` (`[services.<name>]
  enabled =`), the file both A/B slots already read. Toggles survive
  reboots and A/B upgrades; a full SD payload push resets them to shipped
  defaults (a declared fresh-setup event).
  - This is the first writer of `anyka.toml` in the codebase, and it
    contradicts the invariant asserted in `netoverlay.rs:3-7` ("Nothing in
    this codebase writes it"). Chosen anyway, and deliberately: an older
    `anyka-init` in the other A/B slot honours `enabled = false` in this
    file, whereas it would ignore a new overlay file entirely and silently
    re-enable a service after a rollback. The `netoverlay.rs` comment is
    corrected as part of this work, and the line-level editor (§3) is what
    keeps the operator's comments and credentials intact.
- The mechanism is **Approach A**: the supervisor is the sole actor. The
  control socket gains `enable`/`disable` requests; only anyka-init writes
  the config file and only anyka-init mutates service state. onvif-rust is
  a thin client, as with `restart`.
- The trial's port requirement adapts to the enabled-service set (section 4),
  so disabling `onvif` does not wedge every future upgrade into a revert.

## 1. Control protocol

Two new request lines on the existing Unix socket
(`/tmp/anyka-supervisor.sock`), same conventions as `restart`:

```
enable  <name>\n   ->  ok\n | unknown\n | pending\n | error\n
disable <name>\n   ->  ok\n | unknown\n | pending\n | error\n
```

- `ok` — applied (or the no-op case: the service already had that state; no
  file write in the no-op case).
- `unknown` — malformed line, name not present in `[services]`, or a name
  that is not toggleable (`wpa_supplicant`, §2.1).
- `error` — **new reply word.** The config write failed (I/O or write
  error), or the loop is gone entirely. The action is then *not applied at
  all*: persistence is the first step, and a "disabled" that silently
  re-enables itself on the next reboot would defeat the escape-hatch use
  case.
- `pending` — the loop accepted the request but did not answer within the
  control thread's budget. **Not a failure.** `handle_toggle_service`
  persists and applies the change *before* it replies, and dropping the
  reply receiver cancels nothing, so a late answer means the toggle has most
  likely taken effect. Reporting it as an error would tell an admin that
  nothing changed while it changed a moment later. Maps to **202**, the same
  "accepted, not confirmed" contract as `ok`, with a body telling the
  operator to re-read the service list.

Unlike `restart`, these replies depend on work the supervisor loop has to
do, so the control thread waits for the loop to answer. **That wait must be
shorter than the client's socket timeout**, which is 2 s
(`diagnostics/services.rs:26`) — a server that waits longer turns every slow
reply into a connection error. The wait is 1 s (`CONTROL_REPLY_TIMEOUT`),
and the `status` path is bounded by the same constant: the control thread
serves connections one at a time, so an unbounded wait there would queue
every later request behind a single wedged loop.

`status` extends to one row per **configured** service (enabled and
disabled alike), same six-field TSV:

```
<name>\t<state>\t<pid>\t<uptime_s>\t<restarts>\t<retry_in_s>\n
```

`state` gains the value `disabled` (pid `-1`, all counters `0`). Disabled
rows are synthesized by the status handler; `from_svc_state` is unchanged.
This is what lets the UI list a disabled service so it can be re-enabled.

The onvif-rust client's `decode_status` already passes the `state` string
through verbatim, so no parser change is needed there.

## 2. Supervisor lifecycle (`supervisor_loop`)

New messages `Msg::DisableService(String)` and `Msg::EnableService(String)`
(sent by the control thread, handled by `dispatch_msg` like
`RestartService`). `SvcState` gains one variant, `Disabled`, and the service
**stays in the `services` vec** with that state — no insertion or removal,
so the index-based `by_pid` map is never disturbed and the crash-loop
`Reboot` path (which exits the whole process) is unreachable for it:

- `decide()` returns `{ action: None, next: Disabled }` for a `Disabled`
  service, without touching `hist`. That single guard covers the inert
  state: the per-tick stepper acts only on `Action::Start`, and the
  exit-report path records nothing of its own beyond the `by_pid` removal
  and one `warn!`. No `Disabled` guard is needed in `tick_services`.
- **`handle_service_exited` does need one guard, for a different reason.**
  `disable` SIGTERMs the child but its exit report arrives up to a
  reap-poll later, with the old pid still mapped in `by_pid`. Re-enabling
  inside that window starts a fresh pid; the stale exit would then be
  charged to the new instance — recording a restart it never had, knocking
  it into backoff, and letting the next tick spawn a *second* copy beside
  the one already running. So the handler ignores any exit whose pid is not
  the service's current pid. Fixing it there rather than in the disable path
  covers every producer of a stale mapping, not just this one.
- **disable** (name must exist in `cfg.services` and not be
  `wpa_supplicant`, else `unknown`):
  1. `set_service_enabled(path, name, false)` — file first (section 3).
     On failure reply `error` and change nothing.
  2. In-memory `cfg.services[name].enabled = false`.
  3. If the service is `Running`, `SIGTERM` its pid. Either way set
     `state = Disabled` (a `Backoff` service's pending start simply never
     fires). A late exit report for the killed pid finds the service in
     `Disabled` and does nothing.
  4. If the name is `vendor-daemon`, remove the video heartbeat file
     (§2.1). Best-effort: a failure here is logged, not fatal.
  5. Reply `ok`.
- **enable** (symmetric):
  1. `set_service_enabled(path, name, true)`; on failure `error`, nothing
     changes.
  2. In-memory `enabled = true`; set `state = Backoff { until: now,
     attempt: 0 }` — **the exact initial state `build_enabled_services`
     assigns at boot** — so the loop's next tick starts the service and it
     inherits the normal backoff/crash-loop policy from its first exit.
  3. Reply `ok`.
- **Idempotency:** disabling an already-disabled service (or enabling an
  enabled one — `enabled` in `cfg` is the test) is a no-op `ok`: no file
  write, no kill, no state change.
- **Status rows** (with section 1): the status handler lists one row per
  `cfg.services` entry; a service whose `cfg` entry is disabled renders as
  `disabled` (pid `-1`, counters `0`) whether or not it is in the vec, and
  otherwise renders from its vec state as today. `cfg` is the single source
  of truth because both the boot path (config load) and the runtime path
  (this section) write it before anything else.

At boot, disabled services are never inserted into the vec
(`build_enabled_services` is unchanged), and status still lists them via the
cfg rule above — the shipped config already relies on this for
dropbear.

## 2.1 The monitor is a second reboot authority

`decide()` is not the only thing that can reboot this camera. `monitor.rs`
runs its own thread with two escalation ladders that call `sys.reboot()`
**directly**, without consulting the supervisor:

- **video** (`monitor.rs:150`): `read_heartbeat` compares consecutive counter
  values from `[monitor] video_heartbeat_path`. Kill `vendor-daemon` and the
  file survives in `/tmp` holding its last value, so every tick reads a
  stalled counter → `RestartService` (2 ticks) → `KillService` (3) →
  **reboot** (5).
- **wifi** (`monitor.rs:80`): an unhealthy link escalates to
  `RestartSupplicant` and then **reboot**, up to `wifi_reboot_cap` (3).

Both would fire on a *deliberately* disabled service, rebooting the camera
that the admin just stopped rebooting — the exact failure this feature
exists to prevent, on the two services most likely to be crash-looping.

**Resolutions:**

- **vendor-daemon:** the disable path removes
  `cfg.monitor.video_heartbeat_path`. `read_heartbeat` then returns `None`,
  which the monitor already treats as "no signal yet, not a stall"
  (`*ticks = 0`). One line, no new state, no monitor change.
- **wpa_supplicant: not toggleable.** The wifi ladder is driven by link
  health, not by a file, so there is no equivalent one-line fix — it would
  need a disabled-set shared with the monitor thread. And the feature has no
  user: on a Wi-Fi camera, disabling the supplicant loses the device whether
  or not the monitor reboots it. The supervisor rejects
  `enable`/`disable wpa_supplicant` with `unknown`, and the UI renders no
  toggle for that row. This also deletes the boot-handover caveat that an
  earlier draft of this design carried.

## 3. Config persistence (`config.rs`)

`Config::set_service_enabled(path, name, enabled) -> Result<(), ConfigError>`:
a **line-level edit** of `/mnt/anyka_hack/anyka.toml`.

- Locate the `[services.<name>]` header line. Within the stanza (up to the
  next line beginning `[`), if an `enabled =` line exists, replace its
  value with `true`/`false`; otherwise insert `enabled = <value>` directly
  under the header line.
- Every other byte of the file — including comments inside the stanza — is
  preserved verbatim. No TOML round-trip: a re-serialization would reformat
  the whole file, and we only ever change one boolean.
- Missing stanza → error (the caller maps it to `unknown`).
- Write to `<path>.tmp`, `sync_all`, rename over the original — the same
  atomic pattern the applier (`active.tmp`) and the deadman restore already
  use on this filesystem.

Consequences, by construction:

- Both A/B slots read the same file, so a toggle applies identically to
  whichever slot next boots.
- `deny_unknown_fields` stays satisfied: `enabled` is an existing field of
  `ServiceCfg`; we never introduce a new key.
- A full payload push replaces the file and therefore resets toggles. That
  is a declared fresh-setup event (the `push_payload.sh` contract),
  documented here so the behavior is a decision, not an accident.

## 4. Adaptive trial (`update.rs`)

The trial requires every port in `[update] trial_ports` (default
`80/554/8080`) to stay bound. All three are owned by the **onvif** service
(the ONVIF/RTSP/HTTP-FLV endpoints all live in the onvif-rust binary —
`onvif/config.toml:105,183,186`). If an admin disables onvif, no trial
port can ever bind and *every* subsequent A/B update would fail its trial
and revert — precisely the situation where an admin most wants to upgrade
(broken onvif → fixed binary).

Resolution: when the production `Policy` is built (`main.rs:139`), its port
list is filtered through a static ownership map:

```
onvif -> {80, 554, 8080}
```

i.e. **`ports = cfg.update.trial_ports ∩ ports_owned_by(enabled services)`**.

`evaluate_trial` itself is **unchanged**: on an empty port list
`ports.iter().all(..)` is already vacuously true, so it confirms after the
normal hold. The filter is the whole change.

- onvif enabled → the set is unchanged; behavior is byte-identical to
  today, including cameras whose config overrides `trial_ports`.
- onvif disabled → the set is empty → the trial confirms after the hold
  without probing anything, and the update commits. The admin who disabled onvif accepts
  that its new binary is unverified until they re-enable it; a broken
  re-enabled service crash-loops and reboots via the existing cap, exactly
  as any newly started broken service already does.
- Other services own no trial ports, so toggling them never changes the
  trial. The map lives in `update.rs` next to `TRIAL_PORTS` with a comment
  citing `onvif/config.toml`.

## 5. HTTP API (`onvif-rust`)

Two routes beside the existing restart route, same auth table pattern
(administrator; 403 otherwise; inside the `auth_enabled` block):

```
POST /api/services/{name}/enable
POST /api/services/{name}/disable
```

Client (`diagnostics/services.rs`): one `request_toggle(path, name,
enabled)` mirroring `request_restart`, sending `enable <name>\n` /
`disable <name>\n` and requiring the trimmed reply `ok`.

- reply `ok` → **202** (applied, same "accepted, not confirmed" contract as
  restart)
- reply `pending` → **202** with an explanatory body (§1)
- reply `unknown` → **404**
- reply `error` → **503** "nothing changed"; supervisor unreachable → **503**
  "supervisor unreachable". Two bodies, because on a camera with no shell the
  body is the only thing that separates them.

One consequence for the existing restart route: `status` now lists disabled
services, so its "is this a known name" guard starts passing for them and a
restart of a disabled service would return 202 while the supervisor logs
"not running" and does nothing. It already has the status rows in hand, so
it checks the matched row's state and returns **409** for `disabled`.

## 6. WebUI (`ProcessesCard`)

- Each supervised-service row gains a **Disable** action; a disabled row
  renders dimmed with the `disabled` state badge and an **Enable** action.
- Both reuse the confirm-dialog pattern; the description is
  per-service consequence copy:
  - **onvif** — "Web, ONVIF and RTSP access end immediately. The camera
    stays up and keeps streaming, but it is reachable only via FTP (or the
    deadman's telnet after a failed boot). Re-enabling requires FTP or an
    SD-card edit."
  - **vendor-daemon** — "Video capture and encoding stop; streams go dead
    until it is re-enabled. The camera does not reboot — the video watchdog
    is stood down with it."
  - **udhcpc** — "Stops DHCP renewals, so a configured static address is no
    longer overwritten on renewal. Note the link watchdog still runs a
    one-shot `udhcpc` if the default route disappears."
  - **snmp** — "SNMP polling stops."
  - **dropbear** — (shipped disabled; standard copy.)
  - **wpa_supplicant** — no toggle is rendered (§2.1).
- The 10 s poll picks up state changes automatically.
- **onvif-disable flow:** unlike the restart-onvif flow, the card does
  **not** `waitForCameraBack` afterwards — the camera is not coming back on
  its own by design. It tolerates the page drop (the POST's own connection
  dies) and ends in a neutral "onvif disabled" note. Re-enabling onvif is
  only possible again once some other channel reaches the camera, so the
  dialog is the last word the user gets; it says so.

## 7. Edge cases and hazards

| Case | Behavior |
|---|---|
| Disable while in backoff | State becomes `Disabled` in place; the pending start can never fire because `decide()` returns `Action::None` for it. |
| Disable, then the SIGTERM'd process lingers | Its exit report finds the service in `Disabled` and records nothing: no `hist` entry, no backoff, no crash-loop counting. |
| Config write fails mid-disable | `error` reply; in-memory state untouched; the service keeps running exactly as before. |
| Toggle during an A/B apply/reboot window | The apply holds the state lock across its pointer writes; a concurrent toggle writes only `anyka.toml` (a different file) and its in-memory effect lands in whichever supervisor instance owns the process — the applier's instance re-reads nothing at runtime, and the next boot reads the file. No shared file is written by both. |
| Toggle of `wpa_supplicant` | Rejected with `unknown` → 404; no toggle is offered in the UI (§2.1). |
| vendor-daemon disabled, video watchdog | The heartbeat file is removed with it, so the monitor reads `None` and holds at zero ticks — no restart, no kill, no reboot (§2.1). |
| Restart requested for a disabled service | 409 from the HTTP layer (§5); the UI does not offer Restart on a disabled row. |
| onvif disabled, admin re-enables via FTP/SD later | Normal enable path; starts like a boot start. |
| Payload push after toggles | Toggles reset to shipped defaults (`dropbear` disabled, the rest enabled) — declared fresh-setup semantics. |

## 8. Testing

TDD per layer; all host-side (`x86_64-unknown-linux-gnu`), mock-based per
the project's testing standards.

- **anyka-init — config editor:** replace existing `enabled` line; insert
  missing line under header; preserve comments and all other bytes;
  unknown stanza → error; atomic tmp+rename (failure leaves the original
  intact).
- **anyka-init — lifecycle:** disable running → SIGTERM, `Disabled` state,
  listed as `disabled` by `status`, and a subsequent exit report changes
  nothing (no hist, no restart, no crash-loop count); disable in backoff →
  pending start never fires; enable → `Backoff { until: now, attempt: 0 }`
  then starts like a boot start; idempotent no-ops do not touch the file;
  file-write failure → `error` with state unchanged; `wpa_supplicant` →
  `unknown` with nothing written; disabling `vendor-daemon` removes the
  heartbeat file; `status` lists enabled + disabled services from cfg,
  including a boot-time disabled service (never in the vec).
- **anyka-init — adaptive trial:** onvif enabled → port list unchanged;
  onvif disabled → empty set; custom `trial_ports` filter correctly.
- **onvif-rust:** `request_toggle` round trip against the existing
  mock-server test harness (ok / unknown / error / unreachable); route
  tests: 202 on ok, 404 on unknown, 503 on error, 403 for non-admin, 409
  for a restart of a disabled service.
- **WebUI (Vitest/RTL, `data-testid` only):** row shows Disable for an
  enabled service and Enable (dimmed) for a disabled one; no toggle on the
  `wpa_supplicant` row; confirm dialog shows the per-service copy; onvif
  disable does not enter the reconnecting state.

## Out of scope

- **Reactive auto-disable** (supervisor disables a service itself when the
  crash-loop cap trips, instead of rebooting). Natural follow-up; this
  design's manual disable is its prerequisite, not a substitute.
- **Toggling `wpa_supplicant`** (§2.1). Would need a disabled-set shared
  with the monitor thread, and has no user on a Wi-Fi-only camera.
- **Gating `udhcpc` on `[wifi].dhcp`.** Nothing does this today, so a static
  address set from the Network page is already overwritten on renewal — a
  pre-existing bug this feature only gives the admin a lever against. Fix it
  on its own branch, where it belongs.
- Per-service runtime configuration beyond `enabled` (args, env).
- Any change to the web/FTP/telnet auth model.
- A second source of truth (override marker file) — rejected in
  brainstorming; `anyka.toml` is the single source.
