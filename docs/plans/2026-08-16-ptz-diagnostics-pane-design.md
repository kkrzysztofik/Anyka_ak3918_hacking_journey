# PTZ Diagnostics Pane — Design

Date: 2026-08-16
Scope: `onvif-rust` (platform + `/api/diagnostics`) + `www` (DiagnosticsPage).
Follows `2026-08-15-ptz-disable-liveview-design.md`, which gated PTZ *advertising* on
`[ptz] enabled` but left the camera unable to explain a PTZ failure to anyone.

## Problem

When PTZ does not work, the WebUI cannot say why — by construction:

```rust
// platform/anyka/mod.rs, init_ptz_control
Err(e) => {
    tracing::error!("PTZ device failed to open, PTZ features will be unavailable: {}", e);
    None
}
```

The `PlatformResult` boundary added in `3e8f2b8c` carries `open /dev/ak-motor0: errno 19`
all the way up, and `init_ptz_control` drops it. `"PTZ"` never reaches
`degraded_services` either (it appears only in tests). Three consequences:

- A dead PTZ is indistinguishable from a disabled PTZ from outside the device.
- The calibration self-check result is logged at `warn` and discarded (`ptz_open`).
- `TurnOutcome.step_pos` — the one field that would expose the `MOTOR_GET_STATUS`
  silent no-op on V500 hardware — is read after every turn and thrown away in a
  `tracing::debug!`.

Diagnosing the PTZ `-1` era required telnet and a rebuild. This pane is the fix.

## Decisions

| Decision | Choice |
|---|---|
| Card content | Bring-up triage **and** live motion, one card, two row groups |
| Backend shape | New `ptz` block in the diagnostics snapshot, beside `vision` |
| Where the data hangs | `Platform::ptz_diagnostics()`, **not** `PTZControl` |
| Position source | Tracked degrees + last **cached** `step_pos` from `TurnOutcome` |
| Hardware I/O on the poll path | None. No ioctl, no driver lock |
| Disabled state | Collapses to the config row + a note, not eight em-dashes |
| `commands_completed` | Kept — one row, answers "did the command reach the motor?" |
| Also in scope | The fake-success motor ops; the missing loading-flash test |
| Deferred | Stripping PanTilt/Zoom spaces from the `GetNodes` PTZ node |

### Why `Platform`, not `PTZControl`

The codebase precedent is `ImagingControl::vision_diagnostics()`, and following it here
would be wrong. `imaging_control()` is always `Some`, so that trait can carry its own
diagnostics. `ptz_control()` is `None` **precisely when the interesting failure has
happened** — a `PTZControl::ptz_diagnostics()` would go blank exactly when PTZ breaks.
The bring-up outcome therefore hangs off the platform, which exists either way.

### Why cached `step_pos` and not a live read

`ptz_get_step_pos` takes the driver's main lock. A continuous sweep holds that lock for
up to 10 s (`PTZ_CONTINUOUS_TIMEOUT_SECS`), so a diagnostics poll landing mid-sweep would
stall `/api/diagnostics` for the duration. The actor already reads the step position at
the end of every turn; caching it costs nothing and cannot block.

The value is the mismatch: on V500 hardware `MOTOR_GET_STATUS` returns success and writes
nothing, so tracked degrees climb while `step_pos` stays pinned at 0. That divergence,
rendered side by side, *is* the diagnosis.

## Backend changes (`onvif-rust`)

1. **`platform/anyka/mod.rs` — `init_ptz_control`**
   Stop returning a bare `Option`. Keep the failure reason (reuse
   `lifecycle::startup::OptionalInitResult`, which already models
   `Failed { component, error }`). `AnykaPlatform` stores the outcome plus a **concrete**
   `Option<Arc<AnykaPTZControl>>` next to the existing trait object, so the diagnostics
   path can reach `PtzActorState` without downcasting.

2. **`hal/common/ptz.rs` — `PTZHandle`**
   Carry the `ptz_check_self` outcome as a field instead of logging and dropping it. The
   handle only exists when the open succeeded, which is exactly when a self-check result
   can exist.

3. **`platform/anyka/ptz_actor.rs` — `PtzActorState`**
   Add a cached last-step-position (pan, tilt, `Instant`) written from the `TurnOutcome`
   the actor already receives. `parking_lot::RwLock`, read lock-free from the poll path.

4. **`platform/common/traits.rs`**
   New `PtzDiagnostics` struct (`Serialize`) and a sync
   `Platform::ptz_diagnostics(&self) -> Option<PtzDiagnostics>` defaulting to `None`.

   ```rust
   pub struct PtzDiagnostics {
       pub enabled: bool,               // [ptz] enabled
       pub opened: bool,                // motor devices open
       pub init_error: Option<String>,  // "open /dev/ak-motor0: errno 19"
       pub self_check: Option<String>,  // None = not run; "ok"; or the failure
       pub position: Option<[f32; 3]>,  // tracked pan/tilt/zoom, dead-reckoned
       pub moving: bool,
       pub last_step_pos: Option<StepPos>, // { pan, tilt, age_ms }
       pub commands_completed: u32,
   }
   ```

5. **`diagnostics/state.rs`**
   `Snapshot` gains `ptz: Option<PtzDiagnostics>`, filled from
   `platform.ptz_diagnostics()`. Sync — no `.await`, unlike `vision`.

6. **`onvif/ptz/ops/{movement,status}.rs` — the fake-success fix**
   Today, with `ptz_control == None`, `ContinuousMove`/`AbsoluteMove`/`RelativeMove`/
   `GotoHomePosition` take a "no hardware" branch that writes the requested position into
   ONVIF state and returns `Ok`. On a real device with `ptz.enabled = false` that is a
   fabricated success: the client gets `200` and a `GetStatus` position that moved.

   The branch exists for unit tests. Move the simulation where it belongs: give
   `create_test_service()` a `StubPTZControl` (already public, `platform/stub/mod.rs:789`)
   so `None` unambiguously means "this device has no PTZ", and return a `NotSupported`
   fault from that branch.

## Frontend changes (`www`)

- `services/diagnosticsService.ts`: `ptz` field on `Diagnostics` plus an `isPtz` type
  guard, tolerating a missing/`null` key so a snapshot without it still validates.
- `pages/DiagnosticsPage.tsx`: a `PtzCard` mirroring `VisionCard` — same `Card`/`dl`
  shape, same `—` for unknown — added to the existing two-column grid. Two row groups
  split by a border: bring-up (config, motors, self-check, last error) above, motion
  (pan/tilt, step pos + age, moving, commands) below.
- The single piece of real logic: when `enabled && opened && commands_completed > 0` and
  `last_step_pos` is all-zero, render the no-readback warning. That combination is the
  silent-no-op signature.
- When `enabled` is false the card collapses to the config row plus a "PTZ disabled in
  configuration" note.

## Data flow

```text
[ptz] enabled ─┬─ false → PtzDiagnostics { enabled: false, opened: false, .. }
               └─ true  → init_ptz_control
                            ├─ open fails → { opened: false, init_error: "…errno 19" }
                            └─ open ok    → PTZHandle{self_check} + Arc<PtzActorState>
                                              ↓ (every turn, already read today)
                                            TurnOutcome.step_pos → cached + timestamped
/api/diagnostics → Snapshot.ptz → PtzCard
```

## Error handling

- Every field except `enabled`/`opened`/`moving`/`commands_completed` is `Option`; a
  missing reading renders `—` and never fails the snapshot, matching `vision`.
- `ptz_diagnostics()` performs no I/O, so it has no failure mode of its own and returns
  `Option`, not `Result`.
- A platform without PTZ support returns `None` and the card is not rendered.

## Testing

**onvif-rust** (host, `--target x86_64-unknown-linux-gnu`):
1. `enabled = false` → `enabled: false, opened: false, init_error: None`.
2. Open failure → `init_error` retains the device path *and* the errno (the same
   assertion style as `test_ptz_open_propagates_driver_error_detail`).
3. Self-check failure → surfaces in `self_check`, and `opened` stays `true`.
4. Cached step position and `commands_completed` advance after a mocked turn.
5. `Snapshot.ptz` is `None` without a platform.
6. Motor ops with `ptz_control == None` return a `NotSupported` fault, not `Ok`.

**www** (Vitest, `data-testid`):
1. Full `ptz` block renders every row.
2. `enabled: false` → collapsed card with the note, no motion rows.
3. `init_error` present → the errno line renders.
4. Zero `last_step_pos` with `commands_completed > 0` → no-readback warning.
5. `ptz: null` → card absent, page still renders.
6. LiveViewPage: profiles still loading → the disabled PTZ card does not flash
   (the design test from 2026-08-15 that was never written).

## Out of scope

- Stripping PanTilt/Zoom spaces from the PTZ node in `GetNodes`/`GetNode`/
  `GetConfigurations` when disabled. The node must survive for lamp
  `AuxiliaryCommands`, so this needs an ONVIF-conformance judgement of its own.
- Any PTZ control from the diagnostics page — read-only pane.
- A WebUI toggle for `[ptz] enabled` (still boot-time config).
- Hardware step→degree calibration. The pane reports the divergence; it does not correct
  it.

## Ponytail notes

- `// ponytail: cached step_pos from TurnOutcome; live ptz_get_step_pos only if a
  stale-by-one-turn reading proves insufficient — it costs the driver lock.`
- `// ponytail: sync ptz_diagnostics(); make it async only when a field needs real I/O.`
- `commands_completed` and the collapsed-when-disabled card are both reversible one-row
  judgement calls, defaulted rather than debated.
