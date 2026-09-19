# WebUI Service Enable/Disable — Design

- **Date:** 2026-09-19
- **Extends:** `2026-09-19-webui-process-control-design.md` (control socket, `/api/processes`, ProcessesCard)
- **Status:** approved in brainstorming session 2026-09-19; spec pending user review
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

- All services are toggleable, including `onvif` and `vendor-daemon`; each
  carries its own consequence warning in the confirm dialog.
- State persists in `/mnt/anyka_hack/anyka.toml` (`[services.<name>]
  enabled =`), the file both A/B slots already read. Toggles survive
  reboots and A/B upgrades; a full SD payload push resets them to shipped
  defaults (a declared fresh-setup event).
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
enable  <name>\n   ->  ok\n | unknown\n | error\n
disable <name>\n   ->  ok\n | unknown\n | error\n
```

- `ok` — applied (or the no-op case: the service already had that state; no
  file write in the no-op case).
- `unknown` — malformed line, or name not present in `[services]`.
- `error` — **new reply word.** The config write failed (I/O or write
  error). The action is then *not applied at all*: persistence is the first
  step, and a "disabled" that silently re-enables itself on the next reboot
  would defeat the escape-hatch use case.

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

- `decide()` gains one arm: `(Disabled, _) → { action: None, next: Disabled }`.
  Two guard sites exist where the loop steps a service per tick or per exit
  report; both no-op on `Disabled` before touching `decide()` or `hist`.
- **disable** (name must exist in `cfg.services`, else `unknown`):
  1. `set_service_enabled(path, name, false)` — file first (section 3).
     On failure reply `error` and change nothing.
  2. In-memory `cfg.services[name].enabled = false`.
  3. If the service is `Running`, `SIGTERM` its pid. Either way set
     `state = Disabled` (a `Backoff` service's pending start simply never
     fires). A late exit report for the killed pid finds the service in
     `Disabled` and does nothing.
  4. Reply `ok`.
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

**wpa_supplicant caveat (documented, not special-cased):** the boot
handover (`boot.rs::hand_over_supplicant`) may have stood the supervised
supplicant down *for this boot only* (vendor owns the ctrl socket), having
left the file untouched. A runtime re-enable in that situation can hit the
exit-255 hazard until the next boot, where the handover re-probes and
self-heals; the crash-loop cap is the backstop in the meantime. The
wpa_supplicant warning text in the UI (section 6) carries this.

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

- onvif enabled → the set is unchanged; behavior is byte-identical to
  today, including cameras whose config overrides `trial_ports`.
- onvif disabled → the set is empty → the trial is satisfied immediately
  (no hold) and the update commits. The admin who disabled onvif accepts
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
- reply `unknown` → **404**
- reply `error`, or supervisor unreachable → **503**

## 6. WebUI (`ProcessesCard`)

- Each supervised-service row gains a **Disable** action; a disabled row
  renders dimmed with the `disabled` state badge and an **Enable** action.
- Both reuse the confirm-dialog pattern; the description is
  per-service consequence copy:
  - **onvif** — "Web, ONVIF and RTSP access end immediately. The camera
    stays up and keeps streaming, but it is reachable only via FTP (or the
    deadman's telnet after a failed boot). Re-enabling requires FTP or an
    SD-card edit."
  - **vendor-daemon** — "The video pipeline stops; the stream will show as
    stalled until it is re-enabled."
  - **wpa_supplicant** — "The Wi-Fi link may drop for the rest of this
    boot. Only toggle this from a wired connection."
  - **udhcpc** — "The address becomes static until the next renewal — this
    is the documented way to use static addressing."
  - **snmp** — "SNMP polling stops."
  - **dropbear** — (shipped disabled; standard copy.)
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
| Disable while in backoff | State becomes `Disabled` in place; the pending start can never fire because the per-tick stepper no-ops on `Disabled` before reaching `decide()`. |
| Disable, then the SIGTERM'd process lingers | Its exit report finds the service in `Disabled` and records nothing: no `hist` entry, no backoff, no crash-loop counting. |
| Config write fails mid-disable | `error` reply; in-memory state untouched; the service keeps running exactly as before. |
| Toggle during an A/B apply/reboot window | The apply holds the state lock across its pointer writes; a concurrent toggle writes only `anyka.toml` (a different file) and its in-memory effect lands in whichever supervisor instance owns the process — the applier's instance re-reads nothing at runtime, and the next boot reads the file. No shared file is written by both. |
| `wpa_supplicant` re-enabled after a vendor handover | May exit 255 until the next boot's handover re-probes (section 2). Crash-loop cap is the backstop. |
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
  file-write failure → `error` with state unchanged; `status` lists enabled
  + disabled services from cfg, including a boot-time disabled service
  (never in the vec).
- **anyka-init — adaptive trial:** onvif enabled → port list unchanged;
  onvif disabled → empty set → trial confirms immediately; custom
  `trial_ports` filter correctly.
- **onvif-rust:** `request_toggle` round trip against the existing
  mock-server test harness (ok / unknown / error / unreachable); route
  tests: 202 on ok, 404 on unknown, 503 on error, 403 for non-admin.
- **WebUI (Vitest/RTL, `data-testid` only):** row shows Disable for an
  enabled service and Enable (dimmed) for a disabled one; confirm dialog
  shows the per-service copy; onvif disable does not enter the
  reconnecting state.

## Out of scope

- **Reactive auto-disable** (supervisor disables a service itself when the
  crash-loop cap trips, instead of rebooting). Natural follow-up; this
  design's manual disable is its prerequisite, not a substitute.
- Per-service runtime configuration beyond `enabled` (args, env).
- Any change to the web/FTP/telnet auth model.
- A second source of truth (override marker file) — rejected in
  brainstorming; `anyka.toml` is the single source.
