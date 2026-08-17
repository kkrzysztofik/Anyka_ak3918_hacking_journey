# PTZ Preset UI and Diagnostics Truth — Design

Date: 2026-08-16
Scope: `www` (LiveViewPage, DiagnosticsPage, ptzService, diagnosticsService) +
`onvif-rust` (`hal`, `platform/anyka`, `platform/common/traits.rs`).

Follows `2026-08-16-ptz-diagnostics-pane-design.md`, which built the PTZ card, and
`2026-08-16-ptz-position-tracking-design.md`, which added dead reckoning.

**Supersedes one decision** from the diagnostics-pane design: the all-zero
`last_step_pos` heuristic ("the single piece of real logic") is replaced by a fact the
backend reports. See "Why the heuristic goes" below.

## Problem

Two surfaces present PTZ dishonestly, in opposite directions.

**LiveView presets lie about capability.** `displayPresets` (`LiveViewPage.tsx:242`)
hardcodes `[1, 2, 3]`, so a fourth preset on the device is invisible while the backend
allows 255 (`onvif/ptz/types.rs:30`). The per-row control is a `Settings2` gear that
deletes on click with no confirmation. "+ Add Preset" names presets `Preset ${length+1}`,
which collides after any delete. There is no way to rename or re-point a preset.

**The diagnostics card lies about provenance.** Measured and dead-reckoned values render
in the same monospace white column, distinguished only by a `(tracked)` suffix inside one
value string. `Step pos` — the sole measured field — is permanently `0 / 0` on V500
hardware, where `MOTOR_GET_STATUS` returns success and writes nothing. The
`showNoReadback` guard (`DiagnosticsPage.tsx:197-212`) only fires once dead reckoning has
left home, so a freshly homed camera shows a confident `0 / 0` with no caveat at all. The
`position` tuple carries zoom; the card drops it.

## Decisions

| Decision | Choice |
|---|---|
| Preset list | Dynamic, from the device; no fixed slots |
| Preset edit | One dialog for create **and** update, disclosing the position rewrite |
| Standalone rename | **No** — ONVIF has no `RenamePreset` and `SetPreset` rewrites position |
| Delete | Trash icon behind an `AlertDialog` confirm |
| Position readout | New `GetStatus` SOAP client, fetched on dialog open, not polled |
| Readback truth | Backend-reported `step_readback`, not a frontend heuristic |
| Detection point | The probe already at `hal/anyka/ptz/driver.rs:474-486` |
| Trait change | `ptz_check_self` return type, not a new trait method |
| Card layout | Two labelled groups: Measured / Estimated |
| New UI primitives | None — `dialog`, `alert-dialog`, `input`, `button` all exist |

### Why no standalone rename

`store.rs:374` is `presets.insert(preset_token, PresetData { name, position })`. A
`SetPreset` carrying an existing token replaces the position as well as the name, and
ONVIF 24.12 defines no rename operation. A "rename" button implemented the obvious way
would silently re-point the preset at wherever the camera is currently aimed — a data-loss
bug that looks like a text edit.

The honest surface is one action, labelled for what it does: **Update**, which saves the
current position under a possibly-edited name, with the position stated in the dialog
before the user commits. Users who want a pure rename simply do not move the camera first.

This is what earns the `GetStatus` plumbing. The position line is not decoration; it is
the disclosure that makes the operation safe.

### Why the heuristic goes

The diagnostics-pane design chose `enabled && opened && commands_completed > 0 &&
last_step_pos == (0,0)` as the no-readback signature, later tightened with a
`trackedAwayFromHome` clause. Both are inference from a symptom, and the tightening
narrowed the false-positive window at the cost of staying silent in the very state a user
first meets the camera in: freshly homed, tracked position `0,0`, step pos `0,0`,
warning suppressed.

The driver already knows the answer. `calibrate()` performs a post-calibration
`get_status()` for exactly this purpose — "this distinguishes a working
`MOTOR_GET_STATUS` from one that returns success while writing nothing"
(`driver.rs:474-475`) — and then discards the verdict into `tracing::info!`. Promoting a
log line to a field replaces inference with observation, and deletes UI logic rather than
adding to it.

`MotorMessage::default()` is all zeros going into the ioctl. `steps_one_circle == 0` on
return is physically impossible for a driver that wrote anything, which makes the
sentinel reliable rather than probabilistic.

## Backend changes (`onvif-rust`)

1. **`hal/common/ptz.rs` — new verdict type**

   ```rust
   /// Whether the kernel's own position accounting is live on this motor driver.
   ///
   /// V500 boards accept MOTOR_GET_STATUS and write nothing, so a step position
   /// read back from them is a zero, not a measurement.
   pub enum StepReadback { Working, Unsupported, Unknown }
   ```

2. **`hal/anyka/ptz/driver.rs` — `calibrate` / `check_self`**
   Return the verdict instead of logging it. Per motor, from the existing probe:
   - `Err(_)` → `Unknown` (the ioctl refused outright)
   - `Ok(msg)` with `pos == 0 && steps_one_circle == 0 && total_steps == 0` → `Unsupported`
   - otherwise → `Working`

   `check_self` combines both motors worst-of: any `Unsupported` makes the whole readback
   story unusable, so reporting per-axis would imply a precision the consumer cannot act on.

3. **`hal/common/ptz.rs` — `PtzHalTrait` and `PTZHandle`**
   `fn ptz_check_self(&self, pin_type: ptz_feedback_pin) -> PlatformResult<StepReadback>`.
   Changing the return type beats adding a method: no new driver state, no interior
   mutability, and the stub plus `automock` updates are mechanical. `ptz_open` stores the
   verdict on `PTZHandle` beside `self_check_error`, with a `step_readback()` accessor.

   A self-check `Err` still yields `self_check_error` and leaves the readback `Unknown` —
   the sweep failing tells us nothing about the status ioctl.

4. **`platform/common/traits.rs` — `PtzDiagnostics`**
   New field `pub step_readback: StepReadback`, `#[serde(rename_all = "lowercase")]`, so
   the JSON is `"working" | "unsupported" | "unknown"`. `PtzDiagnostics::disabled()`
   defaults it to `Unknown`.

5. **`platform/anyka/ptz_control.rs:205`**
   Read it off the handle alongside `self_check`.

No new I/O on the poll path — the verdict was computed once at open, and this design
keeps the pane's original "no ioctl, no driver lock" rule.

## Frontend changes (`www`)

### `services/ptzService.ts` — GetStatus

`soapBodies.getStatus(profileToken)` plus:

```ts
export interface PTZStatus { pan: number; tilt: number; zoom: number }
export async function getPtzStatus(profileToken: string): Promise<PTZStatus>
```

Parsed from `GetStatusResponse` → `PTZStatus.Position.PanTilt/@x,@y` and `Zoom/@x`,
via the existing `safeString`/`soapRequest` pattern. Absent elements yield `0`, matching
how `getPresets` tolerates a missing `Preset`.

### `pages/LiveViewPage.tsx` — presets

Delete `displayPresets` (`:242-245`). Map `presets ?? []`; empty state renders "No presets
saved" rather than three placeholder buttons that do nothing.

Each row: the name button issues `GotoPreset`; a pencil opens the dialog seeded with that
preset; a trash opens the `AlertDialog`. Both icon buttons carry `aria-label`s naming the
preset. The row disables while its own mutation is in flight — tracked by the token under
mutation, not a single page-wide boolean, so deleting one preset does not freeze the rest.

One `PresetDialog`, two modes:

```text
Update preset                      Save preset
Name:  [Front gate________]        Name:  [Preset 4__________]
⚠ This also re-saves the           Position: 42.0° / −8.5° (estimated)
  position as 42.0° / −8.5°
  (estimated)
        [Cancel] [Update]                  [Cancel] [Save]
```

Both call `setPreset(profileToken, name, token?)`. The suggested name on create derives
from the highest existing `PresetN` token, not `presets.length`.

The `getPtzStatus` query is `enabled` only while the dialog is open, and refetches on
open. Polling continuously would cost a SOAP round trip per interval for a number visible
in one modal.

### `pages/DiagnosticsPage.tsx` — the card

Delete `showNoReadback` and `trackedAwayFromHome` (`:197-212`) entirely.

**Measured** — Config, Motors, Self-check, Step readback, Moving, Commands.
**Estimated** (group header carries "dead-reckoned, not measured" once) — Pan, Tilt,
Zoom, Last motion.

Three changes beyond regrouping:

- `step_readback === 'unsupported'` hides the Step pos row and renders
  `Step readback: unsupported — driver accepts the ioctl, writes nothing`. A permanently
  zero row is noise impersonating data.
- `last_step_pos` conflates a position with a timestamp. The position is measured and
  goes to Measured only when readback works; `age_ms` is always real — it is when the last
  turn finished — and goes to Estimated as "Last motion".
- Zoom renders. `position` has been a 3-tuple since the tracking work; the card dropped
  element `[2]`.

`services/diagnosticsService.ts`: `step_readback` on the `ptz` type and in `isPtz`,
accepting a missing key so a snapshot from an older bundle still validates — same
tolerance the `ptz` block itself already has (`:142-143`).

## Data flow

```text
ptz_open
  └─ ptz_check_self ──► StepReadback ──► PTZHandle.step_readback
                                            │
/api/diagnostics ──► PtzDiagnostics.step_readback ──► PtzCard
                                                        ├─ working     → show Step pos
                                                        └─ unsupported → show the reason

LiveView dialog open ──► GetStatus ──► "re-saves the position as 42.0° / −8.5°"
             [Update] ──► SetPreset(token, name, current position)
```

## Error handling

- `getPtzStatus` failing leaves the dialog usable: the position line renders `—` and the
  warning drops to "This also re-saves the current position." Saving a preset must not be
  blocked by a failed read of a number that is only advisory.
- Preset mutations already surface via `toast.error`; that stays. The delete confirm is
  the new guard, since the destructive action currently has none.
- `step_readback` absent from an older snapshot is treated as `unknown`, which renders the
  Step pos row as it does today. No regression on a mixed-version bundle.

## Testing

**onvif-rust** (host, `--target x86_64-unknown-linux-gnu`):
1. Mocked `get_status` returning a populated `MotorMessage` → `Working`.
2. All-zero `MotorMessage` → `Unsupported`.
3. `get_status` erroring → `Unknown`, and `opened` stays `true`.
4. Self-check `Err` → `self_check_error` set, `step_readback == Unknown`.
5. The verdict reaches `PtzDiagnostics` through `ptz_control.rs`.

**www** (Vitest, `data-testid`):
1. Four presets render four rows — the regression test for the `[1,2,3]` cap.
2. No presets → empty state, no placeholder rows.
3. Trash opens the confirm; `removePreset` is not called until confirmed.
4. Pencil opens the dialog seeded with the preset name and the disclosure line.
5. Create suggests a non-colliding name after a delete.
6. `getPtzStatus` rejecting → dialog still saves.
7. `step_readback: 'unsupported'` → reason rendered, Step pos row absent.
8. `step_readback: 'working'` → Step pos row present with age.
9. Zoom row renders from `position[2]`.
10. Freshly homed camera (`position: [0,0,0]`, `commands_completed: 0`) with
    `unsupported` → still warns. This is the case the old heuristic missed.

## Out of scope

- Preset reordering or drag-and-drop.
- Per-preset thumbnails.
- An on-video position overlay. Add it when someone asks to see the aim without opening a
  dialog; the `getPtzStatus` client this design adds is what it would build on.
- Correcting the step→degree calibration. This reports provenance; it does not fix the
  underlying V500 no-op, which no amount of userspace code can.
- A `RenamePreset` extension to the ONVIF surface.

## Ponytail notes

- `// ponytail: worst-of across both motors; per-axis readback only if pan and tilt ever
  ship with different motor drivers.`
- `// ponytail: GetStatus fetched on dialog open, not polled — a live overlay would need
  an interval, and nothing asks for one yet.`
- Replacing the heuristic is a net deletion in the UI: two derived booleans and a
  conditional paragraph go, one rendered field arrives.
