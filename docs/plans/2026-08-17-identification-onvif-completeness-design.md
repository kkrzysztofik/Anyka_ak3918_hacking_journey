# Identification Tab — ONVIF Completeness

**Date:** 2026-08-17
**Status:** Design approved, pending implementation plan

## Problem

The WebUI Identification tab renders real `GetDeviceInformation` / `GetScopes` data and
saves via `SetScopes`, but the device-identity feature set is both incomplete and
actively broken:

1. **`SetScopes` from the WebUI destroys other scopes.** `deviceService.setScopes()`
   sends exactly two items (name, location). The backend
   (`apply_set_scopes`, `onvif-rust/src/onvif/device/service.rs:240-281`) keeps *fixed*
   scopes and replaces **all** configurable ones with what was sent. One save wipes
   `location/country/unknown` and any scope an ONVIF client added.

   Note: `ops/discovery.rs` contains a second, near-identical implementation that is
   **dead code** (the file carries `#![cfg_attr(not(test), allow(dead_code))]`).
   `handle_set_scopes`, `handle_add_scopes`, `handle_remove_scopes`,
   `handle_get_discovery_mode`, and `handle_set_discovery_mode` have zero non-test
   callers; only `default_scopes()` and `handle_get_scopes_from_vec()` are live. The
   duplication is why the two copies have already drifted (see item 6).

2. **Scope and discovery-mode changes never reach the wire.** `DeviceState` (what
   `SetScopes` writes) and `WsDiscovery` (what announces Hello/ProbeMatch) are separate
   structures. `WsDiscoveryHandle::set_scopes` and `set_discovery_mode` have **only test
   callers**. Renaming the camera does not change what discovery tools display.

3. **`make_discovery_config()` never reads scopes from config.** It builds
   `DiscoveryConfig { endpoint_uuid, http_port, device_ip, hello_interval,
   ..Default::default() }` (`app.rs:1805`), so `scopes` silently falls through to the
   hardcoded `DEFAULT_SCOPES` constant on every boot.

4. **Nothing persists.** Scopes live in a `RwLock<Vec<Scope>>`. `config.device.scopes`
   exists on the struct but is written by nobody and read by nobody. Reboot reverts
   everything.

5. **Backend capability the UI never surfaces:** `AddScopes`, `RemoveScopes`,
   `GetDiscoveryMode`, `SetDiscoveryMode`, `GetHostname`, `SetHostname` are all
   implemented and dispatched, and no WebUI page calls any of them.

6. **`AddScopes` creates duplicates.** The live `apply_add_scopes`
   (`service.rs:283-305`) pushes unconditionally. The dead copy in `ops/discovery.rs:140`
   checks for an existing item first — the two implementations have drifted.

## Decision

Fix the backend so it tells the truth, then build the UI on top. A scopes editor over
the current backend would be a UI that lies.

### Approaches considered

| | Approach | Outcome |
|---|---|---|
| **A** | **Config is the single source of truth** (chosen) | Deletes a whole copy of the truth; persistence falls out of the existing generation counter for free |
| B | Keep `DeviceState`, fan out to all three copies on write | Smallest diff, but three copies still drift — exactly the failure mode that produced this bug |
| C | `WsDiscovery` reads `DeviceState` live | Couples discovery to the device service, still needs an explicit `metadata_version` signal, leaves persistence unsolved |

## Design

### 1. State model

**Delete `DeviceState` entirely.** The struct holds exactly two fields (`scopes`,
`discovery_mode`); both move to config. `DeviceService` already carries
`store.config: Arc<ConfigRuntime>`, so the read path needs no new plumbing.

**Config schema.** Neither `[device]` nor `[discovery]` appears in any deployed
`.deploy/*.toml`, so `#[serde(default)]` fills them — no migration on the four cameras.

```toml
[device]
scopes = ["onvif://www.onvif.org/name/Front%20Door",
          "onvif://www.onvif.org/location/Hallway"]   # String -> Vec<String>

[discovery]
mode = "Discoverable"                                  # new field
```

`scopes` stores **only configurable** scopes. Fixed scopes (`type/video_encoder`,
`type/audio_encoder`, and `type/ptz` gated on `ptz.enabled`) are recomputed at boot by
the existing `default_scopes()`. This keeps a PTZ config change automatically reflected
and stops a client from persisting a bogus fixed scope.

**Write path.** Every scope / mode mutation does two things:

```text
SetScopes / AddScopes / RemoveScopes / SetDiscoveryMode
  |- config.write().device.scopes = ...  -> generation++ -> ConfigPersistenceService flushes (free)
  '- discovery.get().set_scopes(...)     -> metadata_version++ (ONVIF Sec. 4.1)
```

**Late-bound discovery handle.** `OnvifServer::with_app_state` (`app.rs:1161`), which
builds `DeviceService`, runs *before* `start_discovery_phase` (`app.rs:1179`). So
`app.rs` creates an `Arc<OnceLock<WsDiscoveryHandle>>`, passes it through `AppState` into
`DeviceService`, and `start_discovery_phase` calls `.set()` on success.
`WsDiscoveryHandle` is `#[derive(Clone)]` over `Arc` internals, so this is cheap.

`OnceLock` over `RwLock<Option<T>>`: written once at startup, read on every SOAP
mutation, so reads are a plain atomic load and the type documents the intent. The
alternative — reordering startup so discovery precedes the server — is a zero-line change
but makes the camera announce Hello before the HTTP listener accepts.

**Seed path.** `make_discovery_config()` stops using `..Default::default()` for scopes
and seeds from config, so a rebooted camera announces what it was configured with.

### 2. SOAP op semantics

- **`GetScopes`** — merge derived fixed scopes with the configurable list from config.
- **`SetScopes`** — replaces the configurable set wholesale. This is the correct ONVIF
  semantic; the bug is the *client* sending a 2-item list. Validate-all-then-apply
  ordering stays, so a bad item causes no partial write.
- **`AddScopes`** — validate all, then append non-duplicates. Unchanged.
- **`RemoveScopes`** — **behavior change.** Removing a fixed scope currently returns an
  empty list silently, and `discovery.rs:449-483` asserts that no-op. Meanwhile
  `faults.rs:66` defines `fixed_scope()`, a `ter:InvalidArgVal/FixedScope` constructor
  with zero production callers. Wire it up and update the test. Confirm against the
  conformance tool before merge, since it changes an observable response.
- **`SetDiscoveryMode`** — writes `config.discovery.mode` and pushes to the handle.
  Distinct from `discovery.enabled`: `enabled = false` opens no socket at all, while
  `NonDiscoverable` keeps the service running, silent, and runtime-flippable. The UI
  toggle drives the latter.

**Name and location are not special.** They are the `name/` and `location/` scopes. The
form loads the full configurable list anyway, so saving sends **one `SetScopes` with the
complete list**, name/location substituted in place. An `AddScopes`/`RemoveScopes` pair
would be two non-atomic round trips with a window where the camera has no name scope;
the read-modify-write staleness window is identical either way.

### 3. WebUI

`deviceService.ts` — `getScopes()` is currently private and collapses the list to two
strings, which is what makes the destructive save invisible:

```ts
export interface Scope { scopeDef: 'Fixed' | 'Configurable'; scopeItem: string }

export async function getScopes(): Promise<Scope[]>                    // full list
export async function setScopes(scopeItems: string[]): Promise<void>   // complete configurable list
export async function getDiscoveryMode(): Promise<DiscoveryMode>
export async function setDiscoveryMode(m: DiscoveryMode): Promise<void>
export async function getHostname(): Promise<string>
export async function setHostname(name: string): Promise<void>
```

The signature change from `(name, location)` to `(scopeItems)` *is* the fix, expressed in
the type. Name/location extraction becomes pure helpers (`nameFromScopes`,
`locationFromScopes`) that keep percent-decoding in one place.

**Schema** — extend `identificationSchema` with `hostname`, `discoveryMode`, and a
`scopes` array. Scope items get a regex mirroring the Rust `validate_scope` (must start
with `onvif://www.onvif.org/`, no spaces or control characters) so bad entries show
inline instead of returning a SOAP fault.

**Layout** — existing cards stay; two new ones:

| Card | Contents |
|---|---|
| Device Configuration | Name, Location, **Hostname** (new) |
| **Discovery** (new) | `Switch` for Discoverable/NonDiscoverable plus an explanation of what stops |
| **Scopes** (new) | Table of all scopes, `Badge` for Fixed/Configurable, remove disabled on Fixed, add-row input |
| Hardware Information | unchanged |
| Network & System | unchanged |

The scopes table reuses the `<table>` + `Badge` + ghost-icon-button row pattern already in
`UserManagementPage.tsx` rather than introducing a list component.

**Save semantics** — the discovery `Switch` applies immediately as its own mutation
(toggles behind a Save button feel broken). Name/location/hostname/scopes go through the
form's Save, which issues one `SetScopes` with the complete configurable list, plus
`SetHostname` only when that field is dirty.

**Rename** — "Reset to Default" re-reads current device values into the form; it does not
reset to defaults. Relabel to "Discard Changes". Wiring it to the real
`SetSystemFactoryDefault` op would be a different and much more dangerous button.

**Out of scope** — the fake status-card tiles (`Uptime: --`, `Speed: 100 Mbps`,
`Channel: Auto`, hardcoded `ONVIF 24.12`). Noted for a later pass;
`diagnosticsService.uptime.system_s` already exists if uptime is wanted.

### 4. Device identity override

`GetDeviceInformation` reports Manufacturer / Model / Serial Number / Hardware ID from
`AnykaPlatform::device_descriptor()` (`src/platform/anyka/mod.rs:189-198`) — hardcoded
constants. `device_info_from_config()` exists and reads `config.device.*`, but on real
hardware it is only the error fallback, since `platform.get_device_info()` succeeds. So
editing `[device]` in `anyka.toml` currently does nothing, and all four cameras report
`SerialNumber = AK3918-001`.

ONVIF has no `SetDeviceInformation`, so this is not a UI field. The fix is a config
override:

- Change the `manufacturer`, `model`, `serial_number`, and `hardware_id` defaults in
  `DeviceConfig::default()` to **empty strings**, so "unset" is representable. Today they
  default to values that *differ* from the platform constants (`"AK3918 Camera"` vs
  `"AK3918"`, `"ak3918"` vs `"ak3918-hw"`), so a naive "config wins" rule would silently
  change what every camera reports.
- `handle_get_device_information` starts from the platform descriptor (or built-in
  constants when no platform is present) and overrides each field with the config value
  **only when non-empty**.
- `firmware_version` is deliberately excluded: `build_version()` stays authoritative, per
  the existing comment at `ops/system.rs:60-63`. Config must not override the running
  build's identity.

Deployed configs have no `[device]` section, so behavior is unchanged until an operator
opts in by setting a field.

Deriving a unique serial from the interface MAC was considered and deferred — the config
override unblocks per-camera identity without adding a boot-time dependency on network
enumeration.

### 5. Testing

The bug is an integration-seam bug: both sides had passing unit tests over their own
`RwLock`, and no test asserted that a `SetScopes` request changes what goes out on
multicast. The tests that matter cross the seam.

**Rust:**
- `SetScopes` -> `WsDiscoveryHandle` reflects new scopes **and** `metadata_version`
  incremented. This is the test whose absence caused the bug.
- `make_discovery_config()` seeds `scopes` from config — regression guard on the
  `..Default::default()` hole.
- `SetScopes` -> `config.device.scopes` updated and generation counter bumped
  (persistence proof without touching disk).
- Fixed scopes recomputed from `ptz.enabled` at boot.
- Handle absent (`OnceLock` empty) -> mutation still succeeds and persists.
- `RemoveScopes` on a fixed scope -> `ter:FixedScope` fault; existing test flips.
- `AddScopes` with an already-present item does not duplicate it.
- Device identity: empty config field -> platform value reported; non-empty config field
  -> config value reported; `firmware_version` always `build_version()` regardless of
  config.

**WebUI:**
- `setScopes` regression: seed a scope list containing something that is neither name nor
  location, save the form, assert it survives. That single test is the bug.
- `nameFromScopes` / `locationFromScopes` including `%20` decoding.
- Page: fixed scopes render a badge with remove disabled; add/remove rows; discovery
  `Switch` fires its mutation immediately; hostname sent only when dirty.

**Verification gotchas:**
- Host tests run from `cross-compile/`, but the ARM build must run from
  `cross-compile/onvif-rust/` or cargo silently links with the host toolchain.
- `cargo clippy` needs the vendored toolchain `bin` dir first on `PATH` or it fails with
  `E0514`.
- PR CI never cross-builds armv5te — that lives only in `release.yml`. A green PR does
  not prove the camera can build this; run the ARM build locally before merge.

**On-device confirmation:** rename the camera in the WebUI, verify ONVIF Device Manager
shows the new name without a reboot, and that it survives one.
