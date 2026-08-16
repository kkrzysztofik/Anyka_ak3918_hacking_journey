# PTZ Disable Reflects in Live View — Design

Date: 2026-08-15
Scope: `onvif-rust` (ONVIF advertising) + `www` (LiveViewPage). Source of truth is the
existing `[ptz] enabled` config flag (`config_runtime.read().ptz.enabled`, default `true`).

## Problem

`[ptz] enabled = false` skips PTZ *hardware* bring-up (`init_ptz_control` returns `None`,
no motor open, no calibration sweep), but the ONVIF layer and the WebUI never learn this:

- `GetCapabilities` still returns `ptz: Some(...)`.
- `GetProfiles` still attaches a `PTZConfiguration` to every profile.
- Discovery still advertises the `onvif://www.onvif.org/type/ptz` scope.
- `LiveViewPage` unconditionally renders the "Pan & Tilt" card.

So disabling PTZ in settings does not block the UI — the exact gap this feature closes.

## Decisions

| Decision | Choice |
|---|---|
| Source of truth | `config_runtime.read().ptz.enabled` (boot-time; no WebUI toggle) |
| Backend scope | Full ONVIF-correct: strip PTZ from capabilities, profiles, and discovery scope |
| Lamp control | Kept working — `/onvif/ptz_service` stays mounted and reachable for `SendAuxiliaryCommand` (IR/white light), which is independent of the motor |
| `GetServices` | **Still lists** the PTZ service (reachable for lamps); only motor *advertising* is stripped |
| Frontend detection | Profiles-based: "no profile has `ptzConfiguration`" ⇔ disabled (profiles query already in flight) |
| UI behavior | Grey out the card + "PTZ disabled" note (do not hide) |
| Frontend flash | Don't render the disabled card until the profiles query has resolved |

Rationale for keeping `GetServices` populated: the lamps ride the PTZ service via a
hardcoded WebUI endpoint, but `GetCapabilities.ptz = None` + profiles without a
`PTZConfiguration` are the two signals a well-behaved ONVIF client uses before issuing
motor commands, so motor clients already back off without touching the service list.

## Backend changes (`onvif-rust`)

All gated on `config_runtime.read().ptz.enabled`.

1. **`onvif/device/ops/system.rs` — `handle_get_capabilities`** (line ~102)
   Change `ptz: Some(build_ptz_capabilities(&base_url))` to
   `ptz: ptz_enabled.then(|| build_ptz_capabilities(&base_url))`.
   Read `ptz_enabled` from `config` (already a parameter).

2. **`onvif/device/ops/discovery.rs` — `default_scopes`**
   Add a `ptz_enabled: bool` parameter (or a `default_scopes_for(ptz_enabled)` variant);
   omit the `type/ptz` `Scope` when `false`. Update the caller in
   `onvif/device/state.rs::DeviceState::new`, which constructs the scope list at startup
   and has no config today — thread the flag from the `DeviceState` construction site
   (the implementer traces that one call site).

3. **`onvif/media/profile_manager.rs` — profile initialization**
   `initialize_profiles_from_config` (and `initialize_profiles_hardcoded`) currently pass
   a `default_ptz_config` into `defaults::create_profile`, which hardcodes
   `ptz_configuration: Some(ptz_config.clone())`. When `ptz.enabled` is false, omit it —
   e.g. pass `Option<PTZConfiguration>` through `create_profile` and set
   `ptz_configuration` from it (`None` ⇒ `None`). `ProfileManager` already holds
   `config: Option<Arc<ConfigRuntime>>`, so the flag is available.

4. **Verify (no change expected unless broken):** with no `PTZControl`, motor ops
   (`ContinuousMove`, `Stop`, `GotoHomePosition`, presets) should return a clean ONVIF
   fault rather than a raw error. Confirm and, only if it panics/leaks an internal error,
   add a tidy "PTZ not supported" fault.

Note: `onvif/media/state.rs` and `onvif/media/store.rs` also build profiles, but
`MediaService::handle_get_profiles` routes through `ProfileManager::get_profiles()`, so
path (3) is the authoritative GetProfiles source. The implementer confirms no second
live path reaches GetProfiles.

## Frontend changes (`www`)

`src/pages/LiveViewPage.tsx` only:

- Derive `hasPtz = profiles?.some((p) => p.ptzConfiguration) ?? false`.
- Gate the "Pan & Tilt" `SettingsCard` on `hasPtz`:
  - `true` → render exactly as today (D-pad, home, presets, speed slider).
  - `false` and `profiles` is resolved (`isSuccess`, not `isLoading`) → render the same
    card greyed out: controls `disabled`, `aria-disabled`, and a "PTZ disabled" note
    (e.g. a muted description line). No `moveMutation`/preset calls fire.
  - `false` and still loading → keep the current placeholder so the disabled card doesn't
    flash before the query resolves.
- Derive `profileToken` as
  `hasPtz ? (profiles?.find((p) => p.ptzConfiguration)?.token ?? '') : ''` so that with no
  PTZ profile the token is empty. The presets query is already guarded by
  `enabled: !!profileToken`, so it stays inert in the disabled branch (no `GetPresets`
  call). The D-pad/preset controls stay **rendered but `disabled`** (the card is greyed
  out, not emptied), so `handlePtzStart`/`handlePtzStop`/preset handlers never fire.
- ImagingPage lamp controls: **unchanged** — they still fall back to `profiles[0].token`
  and `sendAuxiliaryCommand` keeps working because the PTZ endpoint stays mounted.

## Data flow

```text
config [ptz] enabled = false
  → GetCapabilities:  ptz absent
  → GetProfiles:      profiles have no ptzConfiguration
  → discovery scope:  no type/ptz
  → WebUI getProfiles() → hasPtz = false → greyed "PTZ disabled" card (inert)
```

## Out of scope

- A WebUI toggle for `[ptz] enabled` (still boot-time config only).
- Disabling/hiding the Imaging lamp controls (lamps intentionally stay live).
- Adding a `getCapabilities()`/`getServices()` service to the WebUI (profiles already
  carry the signal).
- Runtime toggling without a reboot.

## Tests

**onvif-rust** (host-side, `$CARGO test --target x86_64-unknown-linux-gnu`):
1. `ptz.enabled=false` → `handle_get_capabilities` returns `ptz: None`; `true` → `Some`.
2. `ptz.enabled=false` → `default_scopes`/`DeviceState` omits `type/ptz`; `true` → present.
3. `ptz.enabled=false` → `ProfileManager` profiles carry no `ptz_configuration`; `true` → present.
4. Motor-op fault behavior with no `PTZControl` (if path 4 needs a change).

**www** (Vitest + `data-testid`):
1. `getProfiles` returns profiles with no `ptzConfiguration` → "Pan & Tilt" card renders
   disabled with "PTZ disabled" note; D-pad buttons are `disabled`.
2. `getProfiles` returns a profile with `ptzConfiguration` → card renders fully interactive
   (existing tests continue to pass).
3. Profiles still loading → disabled card does not flash.

## Ponytail notes

- `// ponytail: profiles-based detection; add getCapabilities() only if a client needs capability info profiles don't carry.`
- `// ponytail: grey-out reuses the existing SettingsCard; a dedicated EmptyState only if the note grows.`
