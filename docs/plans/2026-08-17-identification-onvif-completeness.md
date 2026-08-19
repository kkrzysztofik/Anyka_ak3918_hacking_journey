# Identification Tab — ONVIF Completeness Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make device scopes, discovery mode, and hostname persist and actually reach WS-Discovery, then surface them on the Identification tab.

**Architecture:** `AppConfig` becomes the single source of truth for scopes and discovery mode; `DeviceState` is deleted. Every mutation writes config (persisted for free by the existing generation counter) and pushes to a late-bound `WsDiscoveryHandle` held in an `Arc<OnceLock<_>>`. The WebUI stops collapsing the scope list to two strings and edits it directly.

**Tech Stack:** Rust (tokio, parking_lot, serde/toml), React 19 + TanStack Query + react-hook-form + Zod, Vitest.

**Design doc:** `docs/plans/2026-08-17-identification-onvif-completeness-design.md`

---

## Orientation for the implementing engineer

Read this before Task 1. It will save you an hour of confusion.

**The toolchain is vendored.** Never use the system `cargo`. Set this once per shell:

```bash
cd /home/kmk/dev/anyka-dev
export CARGO=$PWD/toolchain/arm-anykav200-crosstool-ng/bin/cargo
export PATH=$PWD/toolchain/arm-anykav200-crosstool-ng/bin:$PATH   # clippy dies with E0514 without this
```

**Host tests run from `cross-compile/`, but the ARM build must run from `cross-compile/onvif-rust/`.** From the workspace root, cargo silently links with the host toolchain and you get a binary the camera cannot run.

```bash
cd cross-compile           && $CARGO test   --target x86_64-unknown-linux-gnu    # host tests
cd cross-compile           && $CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
cd cross-compile/onvif-rust && $CARGO build --release                            # ARM build
cd cross-compile/www       && npm test                                           # WebUI tests
```

**PR CI never cross-builds armv5te** — that job lives only in `release.yml`. A green PR does *not* prove the camera can build this. Run the ARM build locally before you open the PR.

**There are two copies of the scope handlers.** `src/onvif/device/ops/discovery.rs` is dead code (the file carries `#![cfg_attr(not(test), allow(dead_code))]`); the live path is the `apply_*` functions in `src/onvif/device/service.rs`. Task 1 deletes the dead copy so you never edit the wrong one. Do Task 1 first.

**Commit after every task.** Each task ends green.

---

## Phase 1 — Backend truth

### Task 1: Delete the dead scope handlers

Removes ~150 lines of duplicate logic that has already drifted from the live copy, so later tasks cannot edit the wrong file.

**Files:**
- Modify: `cross-compile/onvif-rust/src/onvif/device/ops/discovery.rs`

**Step 1: Confirm they are dead**

```bash
cd cross-compile/onvif-rust
for f in handle_set_scopes handle_add_scopes handle_remove_scopes handle_get_scopes handle_get_discovery_mode handle_set_discovery_mode; do
  echo "--- $f"; grep -rn "discovery_ops::$f" src/ --include=*.rs
done
```

Expected: no output under any heading. Only `default_scopes` and `handle_get_scopes_from_vec` have production callers.

**Step 2: Delete the dead functions and their tests**

From `ops/discovery.rs`, delete `handle_get_scopes`, `handle_set_scopes`, `handle_add_scopes`, `handle_remove_scopes`, `handle_get_discovery_mode`, `handle_set_discovery_mode`, and every `#[test]` in that file's `mod tests` that references them (including the `create_test_scopes` / `create_test_discovery_mode` fixtures if nothing else uses them).

**Keep:** `default_scopes()` and `handle_get_scopes_from_vec()`.

Also drop the now-unneeded imports (`parking_lot`, the `Set*`/`Add*`/`Remove*` request types) and, if the file no longer has dead code, the `#![cfg_attr(not(test), allow(dead_code))]` attribute at the top.

**Step 3: Verify**

```bash
cd cross-compile && $CARGO test --target x86_64-unknown-linux-gnu --lib
cd cross-compile && $CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
```

Expected: PASS, no warnings. If clippy reports unused imports in `discovery.rs`, remove them.

**Step 4: Commit**

```bash
git add cross-compile/onvif-rust/src/onvif/device/ops/discovery.rs
git commit -m "refactor(device): delete dead duplicate scope handlers"
```

---

### Task 2: Config schema for scopes, discovery mode, and identity

**Files:**
- Modify: `cross-compile/onvif-rust/src/config/types.rs` (`DeviceConfig` ~line 440, `DiscoverySettings` ~line 761)

**Step 1: Write the failing tests**

Add to the `mod tests` in `src/config/types.rs`:

```rust
#[test]
fn test_device_scopes_default_is_empty_list() {
    let config = AppConfig::default();
    assert!(config.device.scopes.is_empty());
}

#[test]
fn test_device_identity_defaults_are_empty_for_override_detection() {
    // Empty means "not overridden" so the platform descriptor wins.
    let config = AppConfig::default();
    assert_eq!(config.device.manufacturer, "");
    assert_eq!(config.device.model, "");
    assert_eq!(config.device.serial_number, "");
    assert_eq!(config.device.hardware_id, "");
}

#[test]
fn test_discovery_mode_defaults_to_discoverable() {
    let config = AppConfig::default();
    assert_eq!(config.discovery.mode, DiscoveryMode::Discoverable);
}

#[test]
fn test_device_scopes_round_trip_through_toml() {
    let toml_str = r#"
[device]
scopes = ["onvif://www.onvif.org/name/Front%20Door"]
"#;
    let config: AppConfig = toml::from_str(toml_str).unwrap();
    assert_eq!(config.device.scopes, vec!["onvif://www.onvif.org/name/Front%20Door"]);
}

#[test]
fn test_deployed_config_without_device_section_still_parses() {
    // No .deploy/*.toml carries [device] or [discovery]; serde(default) must cover it.
    let config: AppConfig = toml::from_str("[server]\nport = 80\n").unwrap();
    assert!(config.device.scopes.is_empty());
    assert_eq!(config.discovery.mode, DiscoveryMode::Discoverable);
}
```

**Step 2: Run to verify they fail**

```bash
cd cross-compile && $CARGO test --target x86_64-unknown-linux-gnu --lib config::types
```

Expected: compile error — `scopes` is a `String`, `discovery.mode` does not exist.

**Step 3: Change the schema**

In `DeviceConfig`:

```rust
    /// ONVIF configurable scopes. Fixed scopes are derived at boot, never stored.
    pub scopes: Vec<String>,
```

In `DeviceConfig::default()`, set `manufacturer`, `model`, `serial_number`, `hardware_id` to `String::new()` and `scopes` to `Vec::new()`. **Leave `hostname: "ipcam"` and `firmware_version` alone** — hostname is a real default, and firmware is never read for `GetDeviceInformation` (see Task 8).

In `DiscoverySettings`, add:

```rust
    /// Discovery mode. `NonDiscoverable` keeps the service running but silent.
    pub mode: DiscoveryMode,
```

with `mode: DiscoveryMode::Discoverable` in its `Default` impl, and `use crate::onvif::types::common::DiscoveryMode;` at the top. `DiscoveryMode` already derives `Serialize, Deserialize, Default, PartialEq` — no newtype needed.

**Step 4: Fix the fallout**

`grep -rn "device.scopes\|\.scopes" src/ --include=*.rs` and fix any site treating it as a `String`. Expect `log_loaded_config` and possibly `config/types.rs` field listings.

**Step 5: Verify**

```bash
cd cross-compile && $CARGO test --target x86_64-unknown-linux-gnu --lib
```

Expected: PASS, including the five new tests.

**Step 6: Commit**

```bash
git add cross-compile/onvif-rust/src/config/types.rs
git commit -m "feat(config): scopes as a list, discovery mode, empty identity defaults"
```

---

### Task 3: Move scopes and discovery mode into config, delete `DeviceState`

The biggest task. `DeviceState` has exactly two fields and both move to config.

**Files:**
- Delete: `cross-compile/onvif-rust/src/onvif/device/state.rs`
- Modify: `cross-compile/onvif-rust/src/onvif/device/mod.rs` (drop the `state` module)
- Modify: `cross-compile/onvif-rust/src/onvif/device/service.rs` (`DeviceService` struct, constructors, `apply_*`, dispatcher arms)
- Modify: `cross-compile/onvif-rust/src/onvif/device/ops/discovery.rs` (add the merge helper)

**Step 1: Write the failing tests**

In `ops/discovery.rs` `mod tests`:

```rust
#[test]
fn test_merged_scopes_combine_fixed_and_configured() {
    let configured = vec!["onvif://www.onvif.org/name/Cam".to_string()];
    let merged = merge_scopes(true, &configured);

    // Fixed scopes are derived, never stored.
    assert!(merged.iter().any(|s|
        s.scope_item == "onvif://www.onvif.org/type/ptz"
            && matches!(s.scope_def, ScopeDefinition::Fixed)));
    assert!(merged.iter().any(|s|
        s.scope_item == "onvif://www.onvif.org/name/Cam"
            && matches!(s.scope_def, ScopeDefinition::Configurable)));
}

#[test]
fn test_merged_scopes_omit_ptz_when_disabled() {
    let merged = merge_scopes(false, &[]);
    assert!(!merged.iter().any(|s| s.scope_item.ends_with("/type/ptz")));
}
```

In `service.rs` `mod tests`:

```rust
#[tokio::test]
async fn test_set_scopes_persists_to_config_and_bumps_generation() {
    let service = test_service();          // helper: DeviceService over an in-memory ConfigRuntime
    let before = service.store.config.generation();

    apply_set_scopes(&service, SetScopes {
        scopes: vec!["onvif://www.onvif.org/name/Cam".to_string()],
    })
    .await
    .unwrap();

    assert_eq!(service.store.config.read().device.scopes,
               vec!["onvif://www.onvif.org/name/Cam"]);
    assert!(service.store.config.generation() > before,
            "generation must bump so ConfigPersistenceService flushes");
}

#[tokio::test]
async fn test_set_scopes_does_not_store_fixed_scopes() {
    let service = test_service();
    apply_set_scopes(&service, SetScopes {
        scopes: vec!["onvif://www.onvif.org/name/Cam".to_string()],
    })
    .await
    .unwrap();

    // Config holds configurable scopes only; fixed ones are derived.
    assert!(!service.store.config.read().device.scopes
        .iter().any(|s| s.contains("/type/")));

    // ...but GetScopes still reports them.
    let response = service.handle_get_scopes(GetScopes {}).await.unwrap();
    assert!(response.scopes.iter().any(|s|
        matches!(s.scope_def, ScopeDefinition::Fixed)));
}

#[tokio::test]
async fn test_set_discovery_mode_persists_to_config() {
    let service = test_service();
    service.handle_set_discovery_mode(SetDiscoveryMode {
        discovery_mode: DiscoveryMode::NonDiscoverable,
    })
    .await
    .unwrap();

    assert_eq!(service.store.config.read().discovery.mode,
               DiscoveryMode::NonDiscoverable);
}

#[tokio::test]
async fn test_fixed_scopes_follow_ptz_config_without_stored_state() {
    let service = test_service_with_ptz(false);
    let response = service.handle_get_scopes(GetScopes {}).await.unwrap();
    assert!(!response.scopes.iter().any(|s| s.scope_item.ends_with("/type/ptz")));
}
```

**Step 2: Run to verify they fail**

```bash
cd cross-compile && $CARGO test --target x86_64-unknown-linux-gnu --lib device::
```

Expected: compile errors — `merge_scopes` and `test_service` do not exist.

**Step 3: Implement**

In `ops/discovery.rs`:

```rust
/// Merge derived fixed scopes with the configured configurable ones.
///
/// Fixed scopes are never stored: they follow device capabilities, so a PTZ
/// config change is reflected with no migration and a client cannot persist a
/// bogus fixed scope.
pub fn merge_scopes(ptz_enabled: bool, configured: &[String]) -> Vec<Scope> {
    let mut scopes: Vec<Scope> = default_scopes(ptz_enabled)
        .into_iter()
        .filter(|s| matches!(s.scope_def, ScopeDefinition::Fixed))
        .collect();

    scopes.extend(configured.iter().map(|item| Scope {
        scope_def: ScopeDefinition::Configurable,
        scope_item: item.clone(),
    }));

    scopes
}
```

In `service.rs`:
- Delete the `state: DeviceStateRef` field and every `DeviceState::new()` / `with_ptz()` call.
- Change `apply_set_scopes` / `apply_add_scopes` / `apply_remove_scopes` and the discovery-mode handlers to take `&DeviceService` (or `&Arc<ConfigRuntime>`) instead of `&DeviceStateRef`.
- Read the scope list with `merge_scopes(config.read().ptz.enabled, &config.read().device.scopes)` — take the read guard once, clone the `Vec<String>`, drop the guard before any `await`.
- Write with `config.write().device.scopes = new_configurable;` — `ConfigRuntime::write()` bumps the generation counter, which is the entire persistence mechanism.
- Update the dispatcher arms (`service.rs:515-580`) that currently `state.clone()`.

Note the `parking_lot` guards in `ConfigRuntime` are **not** async — never hold one across an `await`. Scope the guard in a block and clone what you need out of it.

Delete `state.rs` and its `mod state;` line.

**Step 4: Verify**

```bash
cd cross-compile && $CARGO test --target x86_64-unknown-linux-gnu
cd cross-compile && $CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
```

Expected: PASS. `DeviceState` should appear nowhere: `grep -rn "DeviceState" cross-compile/onvif-rust/src/` returns nothing.

**Step 5: Commit**

```bash
git add -A cross-compile/onvif-rust/src/onvif/device/
git commit -m "refactor(device): config is the single source of truth for scopes"
```

---

### Task 4: Push scope and mode changes to WS-Discovery

This is the task that fixes the actual bug. `DeviceService` is built at `app.rs:1161`, *before* discovery starts at `app.rs:1179`, so the handle must be late-bound.

**Files:**
- Modify: `cross-compile/onvif-rust/src/app.rs` (`AppState` ~line 83, builder, `start_discovery_phase` ~line 918)
- Modify: `cross-compile/onvif-rust/src/onvif/server.rs:435-452` (service construction)
- Modify: `cross-compile/onvif-rust/src/onvif/device/service.rs`

**Step 1: Write the failing test**

This is the test whose absence caused the bug — it must cross the seam.

```rust
#[tokio::test]
async fn test_set_scopes_reaches_discovery_and_bumps_metadata_version() {
    let discovery = WsDiscovery::new(DiscoveryConfig::default());
    let (handle, _task) = discovery.run_service().await.unwrap();

    let slot = Arc::new(OnceLock::new());
    slot.set(handle.clone()).unwrap();
    let service = test_service_with_discovery(slot);

    let before_version = handle.metadata_version();

    apply_set_scopes(&service, SetScopes {
        scopes: vec!["onvif://www.onvif.org/name/Renamed".to_string()],
    })
    .await
    .unwrap();

    let announced = handle.scopes().await;
    assert!(announced.iter().any(|s| s.contains("name/Renamed")),
            "SetScopes must change what WS-Discovery announces");
    assert!(handle.metadata_version() > before_version,
            "ONVIF Sec. 4.1 requires metadata_version to increment on config change");
}

#[tokio::test]
async fn test_set_scopes_succeeds_when_discovery_is_disabled() {
    // discovery.enabled = false, or a degraded startup, leaves the slot empty.
    let service = test_service_with_discovery(Arc::new(OnceLock::new()));

    apply_set_scopes(&service, SetScopes {
        scopes: vec!["onvif://www.onvif.org/name/Cam".to_string()],
    })
    .await
    .expect("missing discovery handle is not a fault");

    assert_eq!(service.store.config.read().device.scopes,
               vec!["onvif://www.onvif.org/name/Cam"]);
}
```

`WsDiscoveryHandle` may not expose `metadata_version()` / `scopes()` as public accessors yet — add them if missing (they read the `Arc<AtomicU32>` and `Arc<RwLock<DiscoveryConfig>>` it already holds).

**Step 2: Run to verify it fails**

```bash
cd cross-compile && $CARGO test --target x86_64-unknown-linux-gnu --lib test_set_scopes_reaches_discovery
```

Expected: FAIL — the announced scopes still contain only `DEFAULT_SCOPES`.

**Step 3: Implement**

Add to `AppState` (follow the existing optional-handle fields like `config_persistence`):

```rust
    /// WS-Discovery handle, populated after the discovery phase starts.
    ///
    /// The server (and therefore DeviceService) is constructed before discovery,
    /// so this is late-bound. Empty is legitimate: discovery may be disabled or
    /// may have failed into degraded mode.
    discovery_handle: Arc<OnceLock<WsDiscoveryHandle>>,
```

with a builder setter and a `discovery_handle()` accessor. In `start_discovery_phase`, on the `Ok` branch:

```rust
let _ = app_state.discovery_handle().set(disc.clone());
```

`set()` returns `Err` if already set; ignoring it is correct — startup runs once.

Pass it into `DeviceService::with_config_and_platform` and store it on the struct. Then in each mutation helper, after the config write:

```rust
if let Some(handle) = self.discovery_handle.get() {
    let announced: Vec<String> = merge_scopes(ptz_enabled, &configurable)
        .into_iter()
        .map(|s| s.scope_item)
        .collect();
    handle.set_scopes(announced).await;   // bumps metadata_version
} else {
    tracing::debug!("No WS-Discovery handle; scope change persisted for next boot");
}
```

Same shape for `SetDiscoveryMode` calling `handle.set_discovery_mode(mode).await`.

**Step 4: Verify**

```bash
cd cross-compile && $CARGO test --target x86_64-unknown-linux-gnu
```

Expected: PASS.

**Step 5: Commit**

```bash
git add -A cross-compile/onvif-rust/src/
git commit -m "fix(device): scope and discovery-mode changes now reach WS-Discovery"
```

---

### Task 5: Seed discovery scopes from config at boot

Without this, a reboot reverts to `DEFAULT_SCOPES` and Task 4's fix survives only until power-cycle.

**Files:**
- Modify: `cross-compile/onvif-rust/src/app.rs:1805` (`make_discovery_config`)

**Step 1: Write the failing test**

```rust
#[test]
fn test_make_discovery_config_seeds_scopes_from_config() {
    let mut app_config = AppConfig::default();
    app_config.device.scopes = vec!["onvif://www.onvif.org/name/Persisted".to_string()];
    app_config.ptz.enabled = false;
    let runtime = Arc::new(ConfigRuntime::new(app_config));

    let discovery_config = App::make_discovery_config(&runtime, 80);

    assert!(discovery_config.scopes.iter().any(|s| s.contains("name/Persisted")),
            "..Default::default() must not swallow configured scopes");
    assert!(!discovery_config.scopes.iter().any(|s| s.ends_with("/type/ptz")));
}

#[test]
fn test_make_discovery_config_seeds_mode_from_config() {
    let mut app_config = AppConfig::default();
    app_config.discovery.mode = DiscoveryMode::NonDiscoverable;
    let runtime = Arc::new(ConfigRuntime::new(app_config));

    assert_eq!(App::make_discovery_config(&runtime, 80).discovery_mode,
               DiscoveryMode::NonDiscoverable);
}
```

**Step 2: Run to verify it fails**

```bash
cd cross-compile && $CARGO test --target x86_64-unknown-linux-gnu --lib make_discovery_config
```

Expected: FAIL — scopes are the hardcoded `DEFAULT_SCOPES`.

**Step 3: Implement**

Replace the `..Default::default()` tail with explicit fields:

```rust
        DiscoveryConfig {
            endpoint_uuid,
            http_port,
            device_ip,
            hello_interval: Duration::from_secs(hello_interval_secs),
            scopes: merge_scopes(ptz_enabled, &configured_scopes)
                .into_iter()
                .map(|s| s.scope_item)
                .collect(),
            discovery_mode,
        }
```

Read `ptz_enabled`, `configured_scopes`, and `discovery_mode` from the guard `c` before the existing `drop(c)`. Constructing every field explicitly is deliberate: `..Default::default()` is what let `scopes` go unwired in the first place, and it would hide the next new field just as silently.

**Step 4: Verify**

```bash
cd cross-compile && $CARGO test --target x86_64-unknown-linux-gnu
```

**Step 5: Commit**

```bash
git add cross-compile/onvif-rust/src/app.rs
git commit -m "fix(discovery): seed announced scopes and mode from config"
```

---

### Task 6: `RemoveScopes` faults on a fixed scope

**Files:**
- Modify: `cross-compile/onvif-rust/src/onvif/device/service.rs` (`apply_remove_scopes`)

**Step 1: Write the failing test**

```rust
#[tokio::test]
async fn test_remove_scopes_rejects_fixed_scope_with_fault() {
    let service = test_service();
    let error = apply_remove_scopes(&service, RemoveScopes {
        scope_item: vec!["onvif://www.onvif.org/type/video_encoder".to_string()],
    })
    .await
    .expect_err("removing a fixed scope must fault, not silently no-op");

    assert!(matches!(error,
        OnvifError::InvalidArgVal { ref subcode, .. } if subcode == "FixedScope"));
}

#[tokio::test]
async fn test_remove_scopes_reports_removed_configurable_items() {
    let service = test_service();
    apply_set_scopes(&service, SetScopes {
        scopes: vec!["onvif://www.onvif.org/name/Cam".to_string()],
    }).await.unwrap();

    let response = apply_remove_scopes(&service, RemoveScopes {
        scope_item: vec!["onvif://www.onvif.org/name/Cam".to_string()],
    }).await.unwrap();

    assert_eq!(response.scope_item, vec!["onvif://www.onvif.org/name/Cam"]);
    assert!(service.store.config.read().device.scopes.is_empty());
}
```

**Step 2: Run to verify it fails**

Expected: FAIL — the call currently returns `Ok` with an empty `scope_item`.

**Step 3: Implement**

In `apply_remove_scopes`, before mutating, check each requested item against the derived fixed scopes and return `crate::onvif::device::faults::fixed_scope(item)` if it matches. That helper already exists at `faults.rs:66` and until now had zero production callers. Validate all items before removing any, matching the validate-then-apply ordering of `apply_set_scopes`.

**Step 4: Verify**

```bash
cd cross-compile && $CARGO test --target x86_64-unknown-linux-gnu
```

Expected: PASS. If Task 1 missed a stale test asserting the old no-op, delete it.

**Step 5: Commit**

```bash
git add cross-compile/onvif-rust/src/onvif/device/service.rs
git commit -m "fix(device): RemoveScopes faults on fixed scopes per ONVIF"
```

---

### Task 7: `AddScopes` stops creating duplicates

**Files:**
- Modify: `cross-compile/onvif-rust/src/onvif/device/service.rs` (`apply_add_scopes`)

**Step 1: Write the failing test**

```rust
#[tokio::test]
async fn test_add_scopes_is_idempotent() {
    let service = test_service();
    let request = || AddScopes { scope_item: vec!["onvif://www.onvif.org/name/Cam".to_string()] };

    apply_add_scopes(&service, request()).await.unwrap();
    apply_add_scopes(&service, request()).await.unwrap();

    assert_eq!(service.store.config.read().device.scopes.len(), 1,
               "adding an existing scope must not duplicate it");
}
```

**Step 2: Run to verify it fails**

Expected: FAIL, `assert_eq!` gets 2.

**Step 3: Implement**

Guard the push with a containment check, mirroring what the (now-deleted) dead copy did:

```rust
    for item in request.scope_item {
        if !configurable.contains(&item) {
            configurable.push(item);
        }
    }
```

**Step 4: Verify + commit**

```bash
cd cross-compile && $CARGO test --target x86_64-unknown-linux-gnu
git add cross-compile/onvif-rust/src/onvif/device/service.rs
git commit -m "fix(device): AddScopes no longer duplicates existing scopes"
```

---

### Task 8: Device identity config override

Today every camera reports `SerialNumber = AK3918-001`, because the hardcoded platform descriptor always wins over config.

**Files:**
- Modify: `cross-compile/onvif-rust/src/onvif/device/ops/system.rs:42-79`

**Step 1: Write the failing tests**

```rust
#[tokio::test]
async fn test_device_information_uses_platform_when_config_is_empty() {
    let config = test_config();                      // identity fields default to ""
    let platform = mock_platform_with_serial("AK3918-001");

    let response = handle_get_device_information(&Some(platform), &config).await.unwrap();

    assert_eq!(response.serial_number, "AK3918-001");
}

#[tokio::test]
async fn test_device_information_config_overrides_platform() {
    let config = test_config();
    config.write().device.serial_number = "CAM-198".to_string();
    let platform = mock_platform_with_serial("AK3918-001");

    let response = handle_get_device_information(&Some(platform), &config).await.unwrap();

    assert_eq!(response.serial_number, "CAM-198",
               "operators must be able to give each camera a unique serial");
}

#[tokio::test]
async fn test_device_information_firmware_always_reports_build_version() {
    let config = test_config();
    config.write().device.firmware_version = "9.9.9".to_string();
    let platform = mock_platform_with_serial("AK3918-001");

    let response = handle_get_device_information(&Some(platform), &config).await.unwrap();

    assert_eq!(response.firmware_version, crate::build_version(),
               "config must not be able to misreport the running build");
}
```

**Step 2: Run to verify they fail**

Expected: the override test FAILS — config is ignored when a platform is present.

**Step 3: Implement**

Add a field-wise override helper and apply it to the platform result:

```rust
/// Overlay non-empty config values over the platform descriptor.
///
/// Empty means "not overridden", so a camera with no `[device]` section keeps
/// reporting exactly what the platform says. `firmware_version` is deliberately
/// excluded: `build_version()` stays authoritative.
fn apply_identity_overrides(mut info: DeviceInfo, config: &Arc<ConfigRuntime>) -> DeviceInfo {
    let c = config.read();
    for (field, override_value) in [
        (&mut info.manufacturer, &c.device.manufacturer),
        (&mut info.model, &c.device.model),
        (&mut info.serial_number, &c.device.serial_number),
        (&mut info.hardware_id, &c.device.hardware_id),
    ] {
        if !override_value.is_empty() {
            *field = override_value.clone();
        }
    }
    info
}
```

Apply it to both branches of `handle_get_device_information` (platform result and `device_info_from_config` fallback). Because Task 2 made the config defaults empty, `device_info_from_config` now needs built-in constants for the no-platform case — add a `DEFAULT_DEVICE_INFO` in this module rather than reaching into `AnykaPlatform`, which is target-gated.

**Step 4: Verify + commit**

```bash
cd cross-compile && $CARGO test --target x86_64-unknown-linux-gnu
git add cross-compile/onvif-rust/src/onvif/device/ops/system.rs
git commit -m "feat(device): allow per-camera identity override from config"
```

---

### Task 9: Backend gate — ARM build

Do not proceed to the WebUI until this passes. CI will not catch it for you.

```bash
cd cross-compile && $CARGO test --target x86_64-unknown-linux-gnu
cd cross-compile && $CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
cd cross-compile && $CARGO fmt --check
cd cross-compile/onvif-rust && $CARGO build --release      # MUST run from this directory
```

Expected: all green, and the release build produces an armv5te binary. Verify:

```bash
file cross-compile/onvif-rust/target/armv5te-unknown-linux-uclibcgnueabi/release/onvif-rust
```

Expected: `ELF 32-bit LSB ... ARM`. If it says x86-64, you ran cargo from the wrong directory.

---

## Phase 2 — WebUI

### Task 10: `deviceService` exposes the real scope list

**Files:**
- Modify: `cross-compile/www/src/services/deviceService.ts`
- Test: `cross-compile/www/src/services/deviceService.test.ts`

**Step 1: Write the failing test**

This one test *is* the bug. Everything else in this phase is UI.

```ts
it('should preserve scopes it does not manage when saving', async () => {
  // A scope no form field represents — added by an ONVIF client, or the default
  // location/country scope. The old two-argument setScopes() destroyed these.
  const existing = [
    { scopeDef: 'Configurable', scopeItem: 'onvif://www.onvif.org/name/Old' },
    { scopeDef: 'Configurable', scopeItem: 'onvif://www.onvif.org/location/country/unknown' },
    { scopeDef: 'Fixed', scopeItem: 'onvif://www.onvif.org/type/video_encoder' },
  ];

  await setScopes(scopesForSave(existing, { name: 'New', location: 'Hall' }));

  const sentBody = vi.mocked(soapRequest).mock.calls[0][1];
  expect(sentBody).toContain('location/country/unknown');
  expect(sentBody).toContain('name/New');
  expect(sentBody).not.toContain('type/video_encoder');  // fixed scopes are never sent
});

it('should parse scope definitions from GetScopes', async () => {
  vi.mocked(soapRequest).mockResolvedValue({
    Scopes: [
      { ScopeDef: 'Fixed', ScopeItem: 'onvif://www.onvif.org/type/ptz' },
      { ScopeDef: 'Configurable', ScopeItem: 'onvif://www.onvif.org/name/Front%20Door' },
    ],
  });

  const scopes = await getScopes();

  expect(scopes).toHaveLength(2);
  expect(scopes[0].scopeDef).toBe('Fixed');
});

it.each([
  ['onvif://www.onvif.org/name/Front%20Door', 'Front Door'],
  ['onvif://www.onvif.org/name/Cam', 'Cam'],
])('should decode %s to %s', (scopeItem, expected) => {
  expect(nameFromScopes([{ scopeDef: 'Configurable', scopeItem }])).toBe(expected);
});

it('should return empty string when no name scope is present', () => {
  expect(nameFromScopes([])).toBe('');
});
```

**Step 2: Run to verify it fails**

```bash
cd cross-compile/www && npm test -- src/services/deviceService.test.ts
```

Expected: FAIL — `getScopes` is not exported and `setScopes` takes two strings.

**Step 3: Implement**

```ts
export interface Scope {
  scopeDef: 'Fixed' | 'Configurable';
  scopeItem: string;
}

export async function getScopes(): Promise<Scope[]>;
export async function setScopes(scopeItems: string[]): Promise<void>;
export function nameFromScopes(scopes: Scope[]): string;
export function locationFromScopes(scopes: Scope[]): string;
export function scopesForSave(scopes: Scope[], values: { name: string; location: string }): string[];
```

`scopesForSave` takes the configurable scopes, replaces the `name/` and `location/` entries with percent-encoded values, keeps everything else untouched, and drops fixed scopes. Keep `getDeviceIdentification()` working by composing it from `getDeviceInformation()` + `getScopes()` + the helpers.

**Step 4: Verify + commit**

```bash
cd cross-compile/www && npm test -- src/services/deviceService.test.ts
git add cross-compile/www/src/services/deviceService.ts cross-compile/www/src/services/deviceService.test.ts
git commit -m "fix(webui): stop destroying unmanaged scopes on save"
```

---

### Task 11: Discovery mode and hostname service functions

**Files:**
- Modify: `cross-compile/www/src/services/deviceService.ts` + test

**Step 1: Write the failing tests**

```ts
it('should parse the discovery mode', async () => {
  vi.mocked(soapRequest).mockResolvedValue({ DiscoveryMode: 'NonDiscoverable' });
  await expect(getDiscoveryMode()).resolves.toBe('NonDiscoverable');
});

it('should send the discovery mode', async () => {
  await setDiscoveryMode('NonDiscoverable');
  expect(vi.mocked(soapRequest).mock.calls[0][1]).toContain('NonDiscoverable');
});

it('should parse and send the hostname', async () => {
  vi.mocked(soapRequest).mockResolvedValue({ HostnameInformation: { Name: 'ipcam' } });
  await expect(getHostname()).resolves.toBe('ipcam');
});
```

**Step 2-4:** Implement the four functions against `ENDPOINTS.device` following the existing `soapRequest` pattern, run `npm test -- src/services/deviceService.test.ts`, commit as `feat(webui): discovery mode and hostname service calls`.

---

### Task 12: Schema

**Files:**
- Modify: `cross-compile/www/src/lib/schemas/identification.ts` + test

Extend `identificationSchema` with `hostname`, `discoveryMode`, and `scopes`. Mirror the Rust `validate_scope` rules so bad input is caught inline rather than as a SOAP fault:

```ts
const scopeItemSchema = z
  .string()
  .min(1, 'Scope cannot be empty')
  .max(1024, 'Scope is too long')
  .startsWith('onvif://www.onvif.org/', 'Scope must start with onvif://www.onvif.org/')
  .refine((s) => !/[\s -]/.test(s), 'Scope cannot contain spaces or control characters');
```

Test each rejection case and one acceptance case. Commit as `feat(webui): identification schema for scopes, hostname, discovery`.

---

### Task 13: Scopes card

**Files:**
- Modify: `cross-compile/www/src/pages/settings/IdentificationPage.tsx` + test

Add a `SettingsCard` between Device Configuration and Hardware Information containing a table of all scopes. Reuse the `<table>` + `Badge` + ghost-icon-button row pattern already in `UserManagementPage.tsx` — do not introduce a list component.

Tests to write first:

```tsx
it('should render fixed scopes as non-removable', async () => { /* remove button disabled */ });
it('should add a scope row', async () => { /* type + click add, row appears */ });
it('should remove a configurable scope row', async () => { /* row disappears */ });
it('should send the full configurable list on save', async () => {
  // The regression guard, at the page level this time.
});
```

Follow the existing `data-testid` convention: `identification-scope-row-<item>`, `identification-scope-add-input`, `identification-scope-remove-<item>`.

Commit as `feat(webui): scopes editor on the identification tab`.

---

### Task 14: Discovery card and hostname field

**Files:**
- Modify: `cross-compile/www/src/pages/settings/IdentificationPage.tsx` + test

- A `Switch` (already in `components/ui/switch.tsx`) for Discoverable / NonDiscoverable, applying **immediately** as its own mutation — a toggle sitting behind a Save button reads as broken. Include a one-line explanation of what NonDiscoverable stops.
- A Hostname `FormField` in the Device Configuration card, sent via `SetHostname` **only when dirty**.
- Relabel "Reset to Default" to "Discard Changes". It re-reads current device values into the form; it has never reset anything to defaults, and the label is more dangerous now that the form carries real scope-editing power.

Tests: switch fires its mutation without a save click; hostname omitted from the save when untouched; discard restores loaded values.

Commit as `feat(webui): discovery mode toggle and hostname field`.

---

## Phase 3 — Verification

### Task 15: Full gate

```bash
cd cross-compile          && $CARGO test --target x86_64-unknown-linux-gnu
cd cross-compile          && $CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
cd cross-compile          && $CARGO fmt --check
cd cross-compile/onvif-rust && $CARGO build --release
cd cross-compile/www      && npm test && npm run lint && npm run type-check && npm run build
```

All must pass. Report actual output — if something fails, say so with the output rather than summarizing it as passing.

### Task 16: On-device confirmation

Scopes and discovery mode are exactly what ONVIF discovery tools consume, so host tests cannot prove this works. Deploy to a camera (see the `anyka-firmware-upgrade` skill) and confirm:

1. Rename the camera in the WebUI → ONVIF Device Manager shows the new name **without a reboot** (proves the WsDiscovery push works).
2. Reboot → the new name survives (proves persistence + the `make_discovery_config` seed).
3. Add a custom scope via an ONVIF client, then save the Identification form → the custom scope is still there (proves the destructive-save fix on real hardware).
4. Toggle NonDiscoverable → the camera stops appearing in discovery scans.
5. Set `serial_number` in that camera's `anyka.toml`, restart → `GetDeviceInformation` reports it.

Check `/mnt/logs` for the dry-run output, not `/tmp` — a reset wipes `/tmp`.
