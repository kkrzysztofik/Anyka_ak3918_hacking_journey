# PTZ Disable Reflects in Live View — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `[ptz] enabled = false` in the onvif-rust config stop ONVIF advertising PTZ, and make the WebUI live view grey out its PTZ controls with a "PTZ disabled" note.

**Architecture:** The backend already derives everything from `config_runtime.read().ptz.enabled`. Gate the three advertising surfaces (GetCapabilities, discovery scope, profile `PTZConfiguration`) on that flag; keep `/onvif/ptz_service` mounted so lamp `SendAuxiliaryCommand` keeps working. The WebUI detects "disabled" purely from the profiles query it already makes (no profile has `ptzConfiguration`), so no new SOAP service is added.

**Tech Stack:** Rust (onvif-rust, quick-xml types, host-side `x86_64-unknown-linux-gnu` tests) + TypeScript/React 19/Vitest (www).

**Spec:** `docs/plans/2026-08-15-ptz-disable-liveview-design.md`

## Global Constraints

- Rust: no `unwrap()`/`expect()`/`panic!()` in production paths; `tracing` for logs; tests in `#[cfg(test)] mod tests` next to code; test names `test_<component>_<scenario>_<outcome>`.
- Rust toolchain: `source ./setenv.sh` (repo root) then `$CARGO test --target x86_64-unknown-linux-gnu` (host-side).
- www: use `data-testid` selectors (no role/text/class selectors); shadcn primitives from `src/components/ui/`; strict TS (no `any`).
- www quality gates: `npm run lint`, `npm run type-check`, `npm run test` from `cross-compile/www`.
- Source of truth is `config_runtime.read().ptz.enabled` (default `true`). Lamp control and `GetServices` are NOT to be changed.
- Keep existing tests green; add/update tests for every behavior change.

---

### Task 1: GetCapabilities omits PTZ capability when disabled

**Files:**
- Modify: `cross-compile/onvif-rust/src/onvif/device/ops/system.rs` (fn `handle_get_capabilities`, and `mod tests`)

**Interfaces:**
- Consumes: `config: &Arc<ConfigRuntime>` (already a parameter).
- Produces: `GetCapabilitiesResponse` with `capabilities.ptz: None` when `ptz.enabled == false`.

- [ ] **Step 1: Write the failing tests**

In `system.rs`, in `mod tests` (which already has `use super::*; use std::sync::Arc;` and helper `create_test_config()` returning `Arc::new(ConfigRuntime::new(Default::default()))`), add after the existing `test_get_capabilities`:

```rust
#[test]
fn test_get_capabilities_ptz_disabled_omits_ptz() {
    let config = create_test_config();
    config.write().ptz.enabled = false;
    let response =
        handle_get_capabilities(&config, GetCapabilities { category: vec![] }).unwrap();
    assert!(response.capabilities.ptz.is_none());
}

#[test]
fn test_get_capabilities_ptz_enabled_includes_ptz() {
    let config = create_test_config();
    let response =
        handle_get_capabilities(&config, GetCapabilities { category: vec![] }).unwrap();
    assert!(response.capabilities.ptz.is_some());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `source ./setenv.sh && $CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust test_get_capabilities_ptz_disabled_omits_ptz` (from `cross-compile/onvif-rust`)
Expected: FAIL — the disabled test asserts `is_none()` but the response currently returns `Some`.

- [ ] **Step 3: Implement the gate**

In `handle_get_capabilities` (currently returns `ptz: Some(build_ptz_capabilities(&base_url))`), read the flag and use `then`:

```rust
let ptz_enabled = config.read().ptz.enabled;
let base_url = base_url(config);

Ok(GetCapabilitiesResponse {
    capabilities: crate::onvif::types::device::Capabilities {
        analytics: None,
        device: Some(build_device_capabilities(&base_url)),
        events: None,
        imaging: Some(build_imaging_capabilities(&base_url)),
        media: Some(build_media_capabilities(&base_url)),
        ptz: ptz_enabled.then(|| build_ptz_capabilities(&base_url)),
        extension: None,
    },
})
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `source ./setenv.sh && $CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust test_get_capabilities` 
Expected: PASS (all capabilities tests green).

- [ ] **Step 5: Commit**

```bash
git add cross-compile/onvif-rust/src/onvif/device/ops/system.rs
git commit -m "feat(onvif): gate PTZ capability on ptz.enabled"
```

---

### Task 2: Discovery scope omits `type/ptz` when disabled

**Files:**
- Modify: `cross-compile/onvif-rust/src/onvif/device/ops/discovery.rs` (`default_scopes`, and `mod tests` helper `create_test_scopes`)
- Modify: `cross-compile/onvif-rust/src/onvif/device/state.rs` (`DeviceState::new` + new `with_ptz`)
- Modify: `cross-compile/onvif-rust/src/onvif/device/service.rs` (`with_config_and_platform`, line ~76)

**Interfaces:**
- Produces: `discovery::default_scopes(ptz_enabled: bool) -> Vec<Scope>`; `DeviceState::with_ptz(ptz_enabled: bool) -> Self` (and `DeviceState::new()` keeps its no-arg signature, delegating with `true`).

- [ ] **Step 1: Write the failing tests**

In `discovery.rs` `mod tests` (already has `use super::*;`), add:

```rust
#[test]
fn test_default_scopes_omits_ptz_when_disabled() {
    let scopes = default_scopes(false);
    assert!(!scopes
        .iter()
        .any(|s| s.scope_item == "onvif://www.onvif.org/type/ptz"));
}

#[test]
fn test_default_scopes_includes_ptz_when_enabled() {
    let scopes = default_scopes(true);
    assert!(scopes
        .iter()
        .any(|s| s.scope_item == "onvif://www.onvif.org/type/ptz"));
}
```

Also update the existing helper `create_test_scopes()` (currently `parking_lot::RwLock::new(default_scopes())`) to `parking_lot::RwLock::new(default_scopes(true))` so it compiles.

- [ ] **Step 2: Run tests to verify they fail**

Run: `source ./setenv.sh && $CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust default_scopes`
Expected: FAIL — `default_scopes` takes no arguments yet (`default_scopes(false)` is a compile error).

- [ ] **Step 3: Implement the gated scope list**

Change `default_scopes` in `discovery.rs` to take the flag and build the vector conditionally:

```rust
pub fn default_scopes(ptz_enabled: bool) -> Vec<Scope> {
    let mut scopes = vec![
        Scope {
            scope_def: ScopeDefinition::Fixed,
            scope_item: "onvif://www.onvif.org/type/video_encoder".to_string(),
        },
        Scope {
            scope_def: ScopeDefinition::Fixed,
            scope_item: "onvif://www.onvif.org/type/audio_encoder".to_string(),
        },
    ];
    if ptz_enabled {
        scopes.push(Scope {
            scope_def: ScopeDefinition::Fixed,
            scope_item: "onvif://www.onvif.org/type/ptz".to_string(),
        });
    }
    scopes.extend([
        Scope {
            scope_def: ScopeDefinition::Configurable,
            scope_item: "onvif://www.onvif.org/location/country/unknown".to_string(),
        },
        Scope {
            scope_def: ScopeDefinition::Configurable,
            scope_item: "onvif://www.onvif.org/name/OnvifCamera".to_string(),
        },
    ]);
    scopes
}
```

In `state.rs`, keep `DeviceState::new()` no-arg but delegate to a new constructor:

```rust
pub fn new() -> Self {
    Self::with_ptz(true)
}

pub fn with_ptz(ptz_enabled: bool) -> Self {
    Self {
        scopes: RwLock::new(crate::onvif::device::ops::discovery::default_scopes(ptz_enabled)),
        discovery_mode: RwLock::new(DiscoveryMode::Discoverable),
    }
}
```

In `service.rs` `with_config_and_platform`, replace `let state = Arc::new(DeviceState::new());` with:

```rust
let state = Arc::new(DeviceState::with_ptz(config.read().ptz.enabled));
```

(Leave `DeviceService::new()` — the no-config path — calling `DeviceState::new()`, which now means "enabled" by default.)

- [ ] **Step 4: Run tests to verify they pass**

Run: `source ./setenv.sh && $CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust default_scopes`
Expected: PASS. Then run the device module tests: `$CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust onvif::device` (or `--lib device`) — expect PASS (fix any remaining `default_scopes()`/`DeviceState::new()` call sites the compiler flags, all of which are test helpers).

- [ ] **Step 5: Commit**

```bash
git add cross-compile/onvif-rust/src/onvif/device/ops/discovery.rs \
        cross-compile/onvif-rust/src/onvif/device/state.rs \
        cross-compile/onvif-rust/src/onvif/device/service.rs
git commit -m "feat(onvif): omit type/ptz discovery scope when disabled"
```

---

### Task 3: Profiles omit `PTZConfiguration` when disabled

**Files:**
- Modify: `cross-compile/onvif-rust/src/onvif/media/profile_manager.rs` (`initialize_profiles_from_config`, and `mod tests`)

**Interfaces:**
- Consumes: `config: &Arc<ConfigRuntime>` (already available in `initialize_profiles_from_config`).
- Produces: profiles returned by `ProfileManager::get_profiles()` have `ptz_configuration: None` when `ptz.enabled == false`. (The hardcoded no-config path stays enabled — no config means default `true`.)

- [ ] **Step 1: Write the failing tests**

In `profile_manager.rs` `mod tests` (ensure `use std::sync::Arc;` and `use crate::config::ConfigRuntime;` are in the `use` block), add:

```rust
#[test]
fn test_profiles_omit_ptz_configuration_when_disabled() {
    let config = Arc::new(ConfigRuntime::new(Default::default()));
    config.write().ptz.enabled = false;
    let manager = ProfileManager::with_config(config);
    let profiles = manager.get_profiles();
    assert!(!profiles.is_empty());
    for profile in profiles {
        assert!(profile.ptz_configuration.is_none());
    }
}

#[test]
fn test_profiles_include_ptz_configuration_when_enabled() {
    let config = Arc::new(ConfigRuntime::new(Default::default()));
    let manager = ProfileManager::with_config(config);
    let profiles = manager.get_profiles();
    assert!(!profiles.is_empty());
    for profile in profiles {
        assert!(profile.ptz_configuration.is_some());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `source ./setenv.sh && $CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust test_profiles_omit_ptz_configuration_when_disabled`
Expected: FAIL — profiles currently always carry `Some` PTZ configuration.

- [ ] **Step 3: Implement the gate**

In `initialize_profiles_from_config`, read the flag once at the top (next to `let mut profile_count = 0;`):

```rust
let ptz_enabled = config.read().ptz.enabled;
```

Then, in the loop, make the created profile mutable and clear the config when disabled. Change this block (currently around line 288):

```rust
let profile = Self::create_profile_from_config(
    &profile_config.name,
    profile_config.width,
    profile_config.height,
    profile_count,
    profile_config.audio_enabled,
    video_encoder_config,
    audio_encoder_config,
    &default_ptz_config,
);
self.profiles.write().insert(profile.token.clone(), profile);
```

to:

```rust
let mut profile = Self::create_profile_from_config(
    &profile_config.name,
    profile_config.width,
    profile_config.height,
    profile_count,
    profile_config.audio_enabled,
    video_encoder_config,
    audio_encoder_config,
    &default_ptz_config,
);
if !ptz_enabled {
    profile.ptz_configuration = None;
}
self.profiles.write().insert(profile.token.clone(), profile);
```

(Leave `initialize_profiles_hardcoded` untouched — it has no config, so PTZ stays enabled.)

- [ ] **Step 4: Run tests to verify they pass**

Run: `source ./setenv.sh && $CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust test_profiles_omit_ptz_configuration_when_disabled test_profiles_include_ptz_configuration_when_enabled`
Expected: PASS. Then `$CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust onvif::media` — expect PASS.

- [ ] **Step 5: Commit**

```bash
git add cross-compile/onvif-rust/src/onvif/media/profile_manager.rs
git commit -m "feat(onvif): omit PTZConfiguration from profiles when disabled"
```

---

### Task 4: Live view greys out PTZ controls when unsupported

**Files:**
- Modify: `cross-compile/www/src/pages/LiveViewPage.tsx`
- Test: `cross-compile/www/src/pages/LiveViewPage.test.tsx`

**Interfaces:**
- Consumes: `getProfiles` (already imported from `@/services/profileService`).
- Produces: a `data-testid="liveview-ptz-fieldset"` wrapping the PTZ cards, `disabled` when no profile has `ptzConfiguration`; a `data-testid="liveview-ptz-disabled-note"` line in the Pan & Tilt card.

- [ ] **Step 1: Write the failing test**

In `LiveViewPage.test.tsx`, add a test that overrides the `getProfiles` mock to return profiles with **no** `ptzConfiguration`:

```tsx
it('should disable PTZ controls when no profile has a PTZ configuration', async () => {
  const { getProfiles } = await import('@/services/profileService');
  vi.mocked(getProfiles).mockResolvedValueOnce([
    { token: 'ProfileToken1', name: 'MainStream' },
    { token: 'ProfileToken2', name: 'SubStream' },
  ]);
  renderWithProviders(<LiveViewPage />);
  await screen.findByTestId('liveview-ptz-title');
  expect(screen.getByTestId('liveview-ptz-fieldset')).toBeDisabled();
  expect(screen.getByTestId('liveview-ptz-disabled-note')).toHaveTextContent('PTZ is disabled');
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `npm run test -- LiveViewPage` (from `cross-compile/www`)
Expected: FAIL — `liveview-ptz-fieldset` does not exist yet.

- [ ] **Step 3: Implement the disabled state**

In `LiveViewPage.tsx`:

1. Capture `isSuccess` from the profiles query (currently `const { data: profiles } = useQuery({...})`):

```tsx
const { data: profiles, isSuccess } = useQuery({
  queryKey: ['profiles'],
  queryFn: getProfiles,
});
```

1. Replace the current `profileToken` derivation with a `hasPtz` flag plus an empty-token-when-disabled `profileToken`:

```tsx
const hasPtz = !!profiles?.some((p) => p.ptzConfiguration);
const ptzDisabled = isSuccess && !hasPtz;
const profileToken = hasPtz
  ? (profiles?.find((p) => p.ptzConfiguration)?.token ?? '')
  : '';
```

1. Wrap the right-column controls so the two PTZ cards (Pan & Tilt, Presets) sit inside a native `<fieldset disabled>`. Replace the wrapper `<div className="flex flex-col gap-4">` that opens the "Right Column: Controls" section with:

```tsx
<fieldset
  disabled={ptzDisabled}
  className={cn('m-0 flex flex-col gap-4 border-0 p-0', ptzDisabled && 'opacity-50')}
  data-testid="liveview-ptz-fieldset"
>
```

and change its matching closing `</div>` to `</fieldset>`.

1. Add the note inside the Pan & Tilt card header, after the existing `SettingsCardDescription` (the block that already holds `liveview-ptz-title` / `liveview-ptz-description`):

```tsx
{ptzDisabled && (
  <p className="text-xs text-zinc-500" data-testid="liveview-ptz-disabled-note">
    PTZ is disabled on this device
  </p>
)}
```

Note: when `hasPtz` is false, `profileToken` is `''`, so the existing `!profileToken` guards in `handlePtzStart`/`handlePtzStop`/`handleHome`/`handleGotoPreset`/`handleAddPreset`/`handleRemovePreset` already make every PTZ action a no-op, and the presets query (`enabled: !!profileToken`) does not fire.

- [ ] **Step 4: Run the test suite to verify it passes**

Run: `npm run test -- LiveViewPage` (from `cross-compile/www`)
Expected: PASS — new test green, all existing `LiveViewPage` tests still green (they mock a PTZ-capable profile, so `hasPtz` is true and the fieldset stays enabled).

- [ ] **Step 5: Commit**

```bash
git add cross-compile/www/src/pages/LiveViewPage.tsx cross-compile/www/src/pages/LiveViewPage.test.tsx
git commit -m "feat(www): disable PTZ controls in live view when unsupported"
```

---

## Final validation (after all tasks)

- [ ] Rust: `source ./setenv.sh && $CARGO fmt --check && $CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings && $CARGO test --target x86_64-unknown-linux-gnu`
- [ ] www: `npm run lint && npm run type-check && npm run test`
- [ ] Verification (no code change expected): with `ptz.enabled = false`, confirm motor ops (`ContinuousMove`/`Stop`/`GotoHomePosition`/presets) return a clean ONVIF fault rather than a raw/panic error. Only add a tidy "PTZ not supported" fault if the current behavior is broken. This is out of scope for the four tasks above; log a follow-up only if needed.
