# Network Pane Write Support — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make every control on the WebUI Network pane actually write, by persisting network settings to a machine-owned overlay file that anyka-init merges and applies at boot.

**Architecture:** onvif-rust never runs `ifconfig` — it writes `/mnt/anyka_hack/network.toml`, a machine-owned overlay containing only the keys the user changed. anyka-init merges that overlay over `anyka.toml [wifi]` at boot and applies it through its existing, tested static/DHCP bring-up, inheriting the gateway-unreachable → DHCP rollback. A new middle rung quarantines an overlay that fails to associate and retries with the operator baseline. Ports are separate: they are onvif-rust's own listeners, written to its own `config.toml`.

**Tech Stack:** Rust (anyka-init, onvif-rust; vendored ARMv5 toolchain), axum, serde/toml 1.1, mockall, React 19 + TanStack Query + react-hook-form + zod, Vitest.

**Design doc:** `docs/plans/2026-08-19-network-pane-design.md`

---

## Before You Start

Every Rust command uses the vendored toolchain. From the repo root:

```bash
source ./setenv.sh
```

This exports `$CARGO` and puts the toolchain `bin/` first on `PATH`. Without it,
clippy dies with E0514 and the ARM build silently links against host Rust.

Two directory rules that will bite you:

- **Host tests** run from the crate dir with an explicit target:
  `$CARGO test --target x86_64-unknown-linux-gnu`
- **ARM builds** must run from `cross-compile/onvif-rust/`, not the workspace root.

WebUI commands run from `cross-compile/www/`: `npm test` (vitest run), `npm run lint`.

**Do not deploy to .127.** It is on the legacy stack and its deadman reverts.
Hardware validation in Task 18 targets **.198** (telnet 192.168.2.198:24).

---

## Phase 1 — anyka-init: overlay merge and rescue

This phase changes the supervisor that owns booting. Every task here is
test-first, and Task 3 in particular is the difference between a typo'd SSID
being a UI annoyance and being an SD-card pull.

### Task 1: NetworkOverlay type and merge

**Files:**
- Create: `cross-compile/anyka-init/src/netoverlay.rs`
- Modify: `cross-compile/anyka-init/src/lib.rs` (add `pub mod netoverlay;`)

The overlay is `Option`-per-key so that "not set" and "set to false" are
distinguishable. `dns` uses `Option<Vec<String>>` for the same reason: an
explicit empty list must be able to clear inherited servers.

**Step 1: Write the failing test**

Add to `cross-compile/anyka-init/src/netoverlay.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::WifiCfg;

    fn baseline() -> WifiCfg {
        toml::from_str(
            r#"
            ssid = "OperatorNet"
            password = "operatorpass"
            dhcp = true
            "#,
        )
        .expect("baseline must parse")
    }

    #[test]
    fn test_empty_overlay_leaves_baseline_untouched() {
        let mut cfg = baseline();
        NetworkOverlay::default().apply_to(&mut cfg);
        assert_eq!(cfg.ssid, "OperatorNet");
        assert!(cfg.dhcp);
        assert!(cfg.address.is_none());
    }

    #[test]
    fn test_overlay_switches_to_static_address() {
        let mut cfg = baseline();
        let overlay: NetworkOverlay = toml::from_str(
            r#"
            dhcp = false
            address = "192.168.2.50/24"
            gateway = "192.168.2.1"
            dns = ["192.168.2.1"]
            "#,
        )
        .expect("overlay must parse");

        overlay.apply_to(&mut cfg);

        assert!(!cfg.dhcp);
        assert_eq!(cfg.address.as_deref(), Some("192.168.2.50/24"));
        assert_eq!(cfg.gateway.as_deref(), Some("192.168.2.1"));
        assert_eq!(cfg.dns, vec!["192.168.2.1".to_string()]);
        // Credentials the overlay did not set must survive.
        assert_eq!(cfg.ssid, "OperatorNet");
        assert_eq!(cfg.password, "operatorpass");
    }

    #[test]
    fn test_overlay_replaces_credentials_only_when_present() {
        let mut cfg = baseline();
        let overlay: NetworkOverlay =
            toml::from_str(r#"ssid = "NewNet""#).expect("overlay must parse");

        overlay.apply_to(&mut cfg);

        assert_eq!(cfg.ssid, "NewNet");
        assert_eq!(
            cfg.password, "operatorpass",
            "an overlay that sets only ssid must not blank the baseline password"
        );
    }

    #[test]
    fn test_overlay_can_clear_dns_with_an_explicit_empty_list() {
        let mut cfg = baseline();
        cfg.dns = vec!["8.8.8.8".to_string()];
        let overlay: NetworkOverlay = toml::from_str("dns = []").expect("overlay must parse");

        overlay.apply_to(&mut cfg);

        assert!(
            cfg.dns.is_empty(),
            "an explicit empty list must clear, not be treated as absent"
        );
    }

    #[test]
    fn test_unknown_key_is_rejected() {
        let result: Result<NetworkOverlay, _> = toml::from_str(r#"chip = "ssv6355_ble""#);
        assert!(
            result.is_err(),
            "the overlay must not silently accept keys it does not apply"
        );
    }
}
```

**Step 2: Run test to verify it fails**

```bash
cd cross-compile/anyka-init
$CARGO test --target x86_64-unknown-linux-gnu netoverlay
```

Expected: FAIL to compile — `NetworkOverlay` is not defined.

**Step 3: Write minimal implementation**

Above the test module in `cross-compile/anyka-init/src/netoverlay.rs`:

```rust
//! Machine-owned network overlay.
//!
//! `anyka.toml` is the operator's file: hand-edited, comment-rich, and holding
//! the Wi-Fi credentials. Nothing in this codebase writes it. Runtime network
//! changes made from the WebUI land here instead, in a file that has no
//! comments to lose and no operator intent to clobber, and that a support
//! engineer can neutralise with a single `rm`.
//!
//! Every field is `Option` so that "the user never touched this" is
//! distinguishable from "the user set this to false / to an empty list".

use serde::{Deserialize, Serialize};

use crate::config::WifiCfg;

/// Overlay applied over `[wifi]` from `anyka.toml`.
#[derive(Debug, Default, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct NetworkOverlay {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub security: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dhcp: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gateway: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dns: Option<Vec<String>>,
}

impl NetworkOverlay {
    /// Merge this overlay onto a baseline `[wifi]` config, in place.
    ///
    /// Absent keys leave the baseline alone. This is why `dns` is an
    /// `Option<Vec<_>>` and not a bare `Vec<_>`: `dns = []` must be able to
    /// clear servers the baseline inherited.
    pub fn apply_to(&self, cfg: &mut WifiCfg) {
        if let Some(v) = &self.ssid {
            cfg.ssid = v.clone();
        }
        if let Some(v) = &self.password {
            cfg.password = v.clone();
        }
        if let Some(v) = &self.security {
            cfg.security = v.clone();
        }
        if let Some(v) = self.dhcp {
            cfg.dhcp = v;
        }
        if let Some(v) = &self.address {
            cfg.address = Some(v.clone());
        }
        if let Some(v) = &self.gateway {
            cfg.gateway = Some(v.clone());
        }
        if let Some(v) = &self.dns {
            cfg.dns = v.clone();
        }
    }
}
```

Register the module in `cross-compile/anyka-init/src/lib.rs` next to the other
`pub mod` lines:

```rust
pub mod netoverlay;
```

`WifiCfg` currently derives only `Deserialize`. The test builds one from a TOML
literal, so no derive change is needed — but its fields must be `pub` (they
already are, `config.rs:112-152`).

**Step 4: Run test to verify it passes**

```bash
$CARGO test --target x86_64-unknown-linux-gnu netoverlay
```

Expected: PASS, 5 tests.

**Step 5: Commit**

```bash
git add cross-compile/anyka-init/src/netoverlay.rs cross-compile/anyka-init/src/lib.rs
git commit -m "feat(anyka-init): network overlay type with per-key merge"
```

---

### Task 2: Load and apply the overlay in Config::load

**Files:**
- Modify: `cross-compile/anyka-init/src/config.rs:481-490` (`Config::load`)
- Modify: `cross-compile/anyka-init/src/netoverlay.rs`

`Config::load` takes a single path today. It gains a sibling loader that reads
the overlay from an explicit path so tests can drive it from a tempdir. A
missing overlay is the normal case and must not be an error; a *corrupt*
overlay must not be silently ignored, because a silently-ignored overlay looks
identical to a save that never happened.

**Step 1: Write the failing test**

Append to the `tests` module in `cross-compile/anyka-init/src/netoverlay.rs`:

```rust
    #[test]
    fn test_load_returns_default_when_the_file_is_absent() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("network.toml");

        let overlay = NetworkOverlay::load(&path).expect("absent overlay is not an error");

        assert_eq!(overlay, NetworkOverlay::default());
    }

    #[test]
    fn test_load_reads_a_present_overlay() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("network.toml");
        std::fs::write(&path, "dhcp = false\naddress = \"10.0.0.5/24\"\n").expect("write");

        let overlay = NetworkOverlay::load(&path).expect("overlay must load");

        assert_eq!(overlay.dhcp, Some(false));
        assert_eq!(overlay.address.as_deref(), Some("10.0.0.5/24"));
    }

    #[test]
    fn test_load_reports_a_corrupt_overlay_instead_of_ignoring_it() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("network.toml");
        std::fs::write(&path, "dhcp = = =").expect("write");

        assert!(
            NetworkOverlay::load(&path).is_err(),
            "a corrupt overlay must be loud; silently ignoring it is indistinguishable \
             from a save that never happened"
        );
    }
```

Confirm `tempfile` is already a dev-dependency of anyka-init:

```bash
grep -n tempfile cross-compile/anyka-init/Cargo.toml
```

If absent, add it under `[dev-dependencies]` using the same version the other
crates pin — check with `grep -rn 'tempfile' cross-compile/*/Cargo.toml`.

**Step 2: Run test to verify it fails**

```bash
$CARGO test --target x86_64-unknown-linux-gnu netoverlay
```

Expected: FAIL to compile — no function `NetworkOverlay::load`.

**Step 3: Write minimal implementation**

Add to `impl NetworkOverlay` in `netoverlay.rs`:

```rust
    /// Production location of the overlay, alongside `anyka.toml`.
    pub const DEFAULT_PATH: &'static str = "/mnt/anyka_hack/network.toml";

    /// Quarantine name used when a boot with this overlay fails to associate.
    pub const QUARANTINE_SUFFIX: &'static str = ".bad";

    /// Read the overlay from `path`.
    ///
    /// An absent file is the normal, unconfigured case and yields the default
    /// (all-absent) overlay. A present but unparseable file is an error: the
    /// caller must be able to tell "nothing configured" from "configuration
    /// present and broken".
    pub fn load(path: &std::path::Path) -> Result<Self, crate::config::ConfigError> {
        let src = match std::fs::read_to_string(path) {
            Ok(src) => src,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(source) => {
                return Err(crate::config::ConfigError::Read {
                    path: path.display().to_string(),
                    source,
                });
            }
        };
        Ok(toml::from_str(&src)?)
    }
```

Then wire it into `Config::load` (`config.rs:482`). Keep the existing signature
working and add the overlay-aware one, so no caller has to change at once:

```rust
    pub fn load(path: &str) -> Result<Self, ConfigError> {
        Self::load_with_overlay(path, std::path::Path::new(
            crate::netoverlay::NetworkOverlay::DEFAULT_PATH,
        ))
    }

    /// `Config::load`, with the overlay path taken as an argument so tests can
    /// point it at a tempdir.
    pub fn load_with_overlay(
        path: &str,
        overlay_path: &std::path::Path,
    ) -> Result<Self, ConfigError> {
        let src = std::fs::read_to_string(path).map_err(|source| ConfigError::Read {
            path: path.to_string(),
            source,
        })?;
        let mut cfg: Self = src.parse()?;
        crate::netoverlay::NetworkOverlay::load(overlay_path)?.apply_to(&mut cfg.wifi);
        cfg.validate()?;
        Ok(cfg)
    }
```

The overlay is applied **before** `validate()`, so a static address written by
the WebUI is validated on the same terms as one the operator typed.

**Step 4: Run test to verify it passes**

```bash
$CARGO test --target x86_64-unknown-linux-gnu
```

Expected: PASS, including the existing `config.rs` tests — confirm none regressed.

**Step 5: Commit**

```bash
git add cross-compile/anyka-init/src/netoverlay.rs cross-compile/anyka-init/src/config.rs
git commit -m "feat(anyka-init): merge network.toml overlay in Config::load"
```

---

### Task 3: Quarantine a failed overlay and retry with the baseline

**Files:**
- Modify: `cross-compile/anyka-init/src/wifi.rs:427-452` (`bring_up_with`)
- Modify: `cross-compile/anyka-init/src/netoverlay.rs`

This is the rescue rung. `gateway_reachable()` (`wifi.rs:679`) already covers a
bad static IP. It cannot cover a bad SSID: with no association there is no
gateway to probe. The new rung sits between that check and the vendor fallback.

**Step 1: Write the failing test**

Append to the `tests` module in `cross-compile/anyka-init/src/netoverlay.rs`:

```rust
    #[test]
    fn test_quarantine_renames_the_overlay_out_of_the_way() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("network.toml");
        std::fs::write(&path, "ssid = \"TypoNet\"\n").expect("write");

        NetworkOverlay::quarantine(&path);

        assert!(!path.exists(), "the failing overlay must not be used again");
        let bad = dir.path().join("network.toml.bad");
        assert!(bad.exists(), "the failing overlay must be kept for the UI to report");
        assert_eq!(
            std::fs::read_to_string(&bad).expect("read"),
            "ssid = \"TypoNet\"\n",
            "quarantine must preserve the content so the UI can show what failed"
        );
    }

    #[test]
    fn test_quarantine_on_an_absent_overlay_is_a_no_op() {
        let dir = tempfile::tempdir().expect("tempdir");
        // Must not panic and must not create a stray .bad file.
        NetworkOverlay::quarantine(&dir.path().join("network.toml"));
        assert!(!dir.path().join("network.toml.bad").exists());
    }
```

Then add the bring-up test to the `tests` module in
`cross-compile/anyka-init/src/wifi.rs`. Model it on the existing
`test_try_bring_up_with_happy_path_over_dhcp` (`wifi.rs:1163`) — reuse the same
`FakeSys` and tempdir `FsLayout` construction that test uses, including the
`HAPPY_ROUTE` fixture (`wifi.rs:1133`).

```rust
    #[test]
    fn test_a_failing_overlay_is_quarantined_and_the_baseline_is_retried() {
        // Arrange the same tempdir layout the happy-path test uses, but with
        // no carrier, so association fails and try_bring_up_with returns Err.
        // (Copy the setup block from test_try_bring_up_with_happy_path_over_dhcp
        // and omit the carrier write.)
        let dir = tempfile::tempdir().expect("tempdir");
        let overlay_path = dir.path().join("network.toml");
        std::fs::write(&overlay_path, "ssid = \"TypoNet\"\n").expect("write");

        // ... layout + FakeSys setup as in the happy-path test ...

        let mut cfg = /* WifiCfg with ssid = "TypoNet" from the overlay */;
        cfg.fallback_to_vendor = false; // isolate rung 2 from rung 3

        let outcome = bring_up_with_overlay(&sys, &cfg, &baseline_cfg, STORM, &layout, &overlay_path);

        assert!(!overlay_path.exists(), "the failing overlay must be quarantined");
        assert!(
            dir.path().join("network.toml.bad").exists(),
            "the quarantined overlay must be readable by the UI"
        );
        assert!(
            sys.commands().iter().any(|c| c.contains("OperatorNet")
                || c.contains("wpa_supplicant")),
            "the baseline config must actually be retried, not just recorded"
        );
    }
```

> **Note for the implementer:** the exact assertion on `sys.commands()` depends
> on what the existing `FakeSys` records — read `wifi.rs:1100-1210` first and
> match the style of the surrounding tests rather than inventing a new
> accessor. If `FakeSys` exposes spawned argv, assert the retry spawned
> `wpa_supplicant` a second time.

**Step 2: Run test to verify it fails**

```bash
$CARGO test --target x86_64-unknown-linux-gnu wifi
```

Expected: FAIL to compile — no `bring_up_with_overlay`.

**Step 3: Write minimal implementation**

Add to `impl NetworkOverlay` in `netoverlay.rs`:

```rust
    /// Move a failed overlay aside so the next boot uses the baseline.
    ///
    /// Mirrors the quarantine-and-revert semantics of the A/B slot updates in
    /// `update.rs`: keep the last known-good, retain the failure for
    /// inspection. Best-effort by design — a rename that fails must not stop a
    /// boot that is already recovering.
    pub fn quarantine(path: &std::path::Path) {
        if !path.exists() {
            return;
        }
        let mut bad = path.as_os_str().to_owned();
        bad.push(Self::QUARANTINE_SUFFIX);
        if let Err(e) = std::fs::rename(path, std::path::Path::new(&bad)) {
            tracing::error!(error = %e, "failed to quarantine the network overlay");
        }
    }
```

In `wifi.rs`, add the overlay-aware wrapper and leave `bring_up_with` intact so
existing callers and tests keep working:

```rust
/// [`bring_up_with`], plus rung 2 of the rescue ladder.
///
/// `gateway_reachable` (see `assign_address`) already rescues a bad static
/// address. It cannot rescue bad credentials: with no association there is no
/// gateway to probe. So when bring-up fails outright *and* an overlay is what
/// produced `cfg`, quarantine the overlay and retry once with the operator's
/// baseline before falling through to the vendor chain.
pub fn bring_up_with_overlay(
    sys: &dyn Sys,
    cfg: &WifiCfg,
    baseline: &WifiCfg,
    storm_state_path: &str,
    layout: &FsLayout,
    overlay_path: &std::path::Path,
) -> Outcome {
    match try_bring_up_with(sys, cfg, layout) {
        Ok(outcome) => {
            if matches!(outcome, Outcome::Up { .. }) {
                let mut storm = crate::storm::StormState::load(storm_state_path);
                storm.wifi_reboots = 0;
                if let Err(e) = storm.save(storm_state_path) {
                    tracing::warn!(error = %e, "failed to clear wifi reboot counter");
                }
            }
            outcome
        }
        Err(e) => {
            tracing::error!(error = %e, "wifi bring-up failed");

            if overlay_path.exists() {
                tracing::error!(
                    "quarantining the network overlay and retrying with the operator baseline"
                );
                crate::netoverlay::NetworkOverlay::quarantine(overlay_path);
                match try_bring_up_with(sys, baseline, layout) {
                    Ok(outcome) => {
                        tracing::warn!("recovered on the baseline config");
                        return outcome;
                    }
                    Err(e) => tracing::error!(error = %e, "baseline retry also failed"),
                }
            }

            if cfg.fallback_to_vendor {
                fall_back(sys, layout)
            } else {
                tracing::error!("fallback_to_vendor is disabled; the camera may be unreachable");
                Outcome::Failed
            }
        }
    }
}
```

The caller in `boot.rs` (find it with `grep -n 'bring_up' cross-compile/anyka-init/src/boot.rs`)
must now pass both the merged config and the baseline. Load the baseline by
calling `Config::load_with_overlay` with a path that cannot exist — or, cleaner,
have `load_with_overlay` return the pre-merge `WifiCfg` alongside the merged
`Config`. Pick one and keep it consistent; the second is preferable because it
reads the file once.

**Step 4: Run test to verify it passes**

```bash
$CARGO test --target x86_64-unknown-linux-gnu
```

Expected: PASS. Existing `wifi.rs` tests must be untouched — `bring_up_with`
kept its signature.

**Step 5: Commit**

```bash
git add cross-compile/anyka-init/src/wifi.rs cross-compile/anyka-init/src/netoverlay.rs \
        cross-compile/anyka-init/src/boot.rs
git commit -m "feat(anyka-init): quarantine a failing overlay and retry the baseline"
```

---

## Phase 2 — onvif-rust: overlay writer, handlers, REST

### Task 4: Overlay reader/writer in onvif-rust

**Files:**
- Create: `cross-compile/onvif-rust/src/config/netoverlay.rs`
- Modify: `cross-compile/onvif-rust/src/config/mod.rs` (add `pub mod netoverlay;`)

onvif-rust needs its own copy of the overlay shape because the two crates do
not share a library. Keep the field set byte-identical to
`anyka-init/src/netoverlay.rs` — a mismatch means anyka-init's
`deny_unknown_fields` rejects a file onvif-rust just wrote, which surfaces as a
boot failure, not a save failure. Task 17 adds a guard test for exactly this.

**Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_then_read_round_trips() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("network.toml");

        let overlay = NetworkOverlay {
            dhcp: Some(false),
            address: Some("192.168.2.50/24".to_string()),
            gateway: Some("192.168.2.1".to_string()),
            dns: Some(vec!["192.168.2.1".to_string()]),
            ..Default::default()
        };
        overlay.write(&path).expect("write must succeed");

        assert_eq!(NetworkOverlay::read(&path).expect("read"), overlay);
    }

    #[test]
    fn test_absent_keys_are_not_serialised() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("network.toml");

        NetworkOverlay { dhcp: Some(true), ..Default::default() }
            .write(&path)
            .expect("write");

        let src = std::fs::read_to_string(&path).expect("read");
        assert!(src.contains("dhcp = true"));
        assert!(
            !src.contains("ssid"),
            "an overlay that never set ssid must not write an empty one; anyka-init \
             would merge it over the operator's real credentials"
        );
    }

    #[test]
    fn test_read_of_an_absent_file_is_the_default() {
        let dir = tempfile::tempdir().expect("tempdir");
        let overlay = NetworkOverlay::read(&dir.path().join("nope.toml")).expect("absent is ok");
        assert_eq!(overlay, NetworkOverlay::default());
    }

    #[test]
    fn test_quarantined_overlay_is_detected() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("network.toml");
        std::fs::write(dir.path().join("network.toml.bad"), "ssid = \"TypoNet\"\n")
            .expect("write");

        assert!(
            NetworkOverlay::last_failure(&path).is_some(),
            "the UI must be able to report that the previous settings failed"
        );
    }
}
```

**Step 2: Run test to verify it fails**

```bash
cd cross-compile/onvif-rust
$CARGO test --target x86_64-unknown-linux-gnu netoverlay
```

Expected: FAIL to compile.

**Step 3: Write minimal implementation**

```rust
//! Machine-owned network overlay, written here and consumed by anyka-init.
//!
//! The field set must stay identical to `anyka-init/src/netoverlay.rs`.
//! anyka-init parses this file with `deny_unknown_fields`, so a key added here
//! and not there turns a WebUI save into a failed boot.

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::file_ops::atomic_write;

/// Production location, alongside `anyka.toml` under the update root.
pub const DEFAULT_OVERLAY_PATH: &str = "/mnt/anyka_hack/network.toml";

#[derive(Debug, Default, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct NetworkOverlay {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub security: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dhcp: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gateway: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dns: Option<Vec<String>>,
}

impl NetworkOverlay {
    /// Read the overlay; an absent file yields the default.
    pub fn read(path: &Path) -> Result<Self, std::io::Error> {
        match std::fs::read_to_string(path) {
            Ok(src) => {
                toml::from_str(&src).map_err(|e| std::io::Error::other(e.to_string()))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e),
        }
    }

    /// Write the overlay atomically.
    ///
    /// A half-written overlay would fail anyka-init's `deny_unknown_fields`
    /// parse on every boot, so temp-plus-rename is load-bearing, not hygiene.
    pub fn write(&self, path: &Path) -> Result<(), std::io::Error> {
        let content =
            toml::to_string_pretty(self).map_err(|e| std::io::Error::other(e.to_string()))?;
        atomic_write(path, content.as_bytes(), None)
    }

    /// Contents of a quarantined overlay from the previous boot, if any.
    pub fn last_failure(path: &Path) -> Option<Self> {
        let mut bad = path.as_os_str().to_owned();
        bad.push(".bad");
        let src = std::fs::read_to_string(Path::new(&bad)).ok()?;
        toml::from_str(&src).ok()
    }
}
```

Note `atomic_write` already exists at `config/file_ops.rs:20` — reuse it, do
not write another one.

**Step 4: Run test to verify it passes**

```bash
$CARGO test --target x86_64-unknown-linux-gnu netoverlay
```

Expected: PASS, 4 tests.

**Step 5: Commit**

```bash
git add cross-compile/onvif-rust/src/config/netoverlay.rs cross-compile/onvif-rust/src/config/mod.rs
git commit -m "feat(onvif): network overlay reader and atomic writer"
```

---

### Task 5: Implement the NetworkInfo setter trait methods

**Files:**
- Modify: `cross-compile/onvif-rust/src/platform/anyka/network_info.rs`
- Reference: `cross-compile/onvif-rust/src/platform/common/traits.rs:766-789`

The three setters already exist as default methods returning
`PlatformError::NotSupported`. `AnykaNetworkInfo` overrides them to write the
overlay. It must **read-modify-write**: three separate ONVIF calls
(`SetNetworkInterfaces`, `SetDNS`, `SetNetworkDefaultGateway`) each touch a
different slice of the same file, and a blind write would drop the other two.

**Step 1: Write the failing test**

In the `tests` module of `network_info.rs`:

```rust
    #[tokio::test]
    async fn test_set_network_interface_writes_static_config() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("network.toml");
        let info = AnykaNetworkInfo::with_overlay_path(path.clone());

        info.set_network_interface("eth0", Some("192.168.2.50".into()), Some(24), false)
            .await
            .expect("set must succeed");

        let overlay = NetworkOverlay::read(&path).expect("read");
        assert_eq!(overlay.dhcp, Some(false));
        assert_eq!(
            overlay.address.as_deref(),
            Some("192.168.2.50/24"),
            "ONVIF sends address and prefix separately; the overlay stores CIDR \
             because that is what anyka-init's parse_cidr expects"
        );
    }

    #[tokio::test]
    async fn test_set_dns_preserves_an_existing_address() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("network.toml");
        let info = AnykaNetworkInfo::with_overlay_path(path.clone());

        info.set_network_interface("eth0", Some("192.168.2.50".into()), Some(24), false)
            .await
            .expect("set interface");
        info.set_dns(&["1.1.1.1".to_string()], &[]).await.expect("set dns");

        let overlay = NetworkOverlay::read(&path).expect("read");
        assert_eq!(overlay.dns, Some(vec!["1.1.1.1".to_string()]));
        assert_eq!(
            overlay.address.as_deref(),
            Some("192.168.2.50/24"),
            "SetDNS must not drop what SetNetworkInterfaces wrote"
        );
    }

    #[tokio::test]
    async fn test_set_dhcp_clears_the_static_address() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("network.toml");
        let info = AnykaNetworkInfo::with_overlay_path(path.clone());

        info.set_network_interface("eth0", Some("192.168.2.50".into()), Some(24), false)
            .await
            .expect("set static");
        info.set_network_interface("eth0", None, None, true)
            .await
            .expect("set dhcp");

        let overlay = NetworkOverlay::read(&path).expect("read");
        assert_eq!(overlay.dhcp, Some(true));
        assert!(
            overlay.address.is_none(),
            "a stale address left behind DHCP would be applied on the next \
             switch back to static and surprise the operator"
        );
    }
```

**Step 2: Run test to verify it fails**

```bash
$CARGO test --target x86_64-unknown-linux-gnu network_info
```

Expected: FAIL — no `with_overlay_path`, setters still return `NotSupported`.

**Step 3: Write minimal implementation**

`AnykaNetworkInfo` is currently a unit struct (`network_info.rs:14`). Give it
the overlay path so tests can redirect it:

```rust
pub(super) struct AnykaNetworkInfo {
    overlay_path: std::path::PathBuf,
}

impl AnykaNetworkInfo {
    pub(super) fn new() -> Self {
        Self {
            overlay_path: std::path::PathBuf::from(
                crate::config::netoverlay::DEFAULT_OVERLAY_PATH,
            ),
        }
    }

    #[cfg(test)]
    pub(super) fn with_overlay_path(overlay_path: std::path::PathBuf) -> Self {
        Self { overlay_path }
    }

    /// Read the overlay, hand it to `edit`, write it back.
    ///
    /// Read-modify-write because each ONVIF setter owns a different slice of
    /// the same file; a blind write would drop the other setters' work.
    fn update_overlay(
        &self,
        edit: impl FnOnce(&mut NetworkOverlay),
    ) -> PlatformResult<()> {
        let mut overlay = NetworkOverlay::read(&self.overlay_path)
            .map_err(|e| PlatformError::Io(e.to_string()))?;
        edit(&mut overlay);
        overlay
            .write(&self.overlay_path)
            .map_err(|e| PlatformError::Io(e.to_string()))
    }
}
```

Then override the three trait methods in the `impl NetworkInfo for
AnykaNetworkInfo` block:

```rust
    async fn set_network_interface(
        &self,
        _token: &str,
        ipv4_address: Option<String>,
        ipv4_prefix_length: Option<u8>,
        ipv4_dhcp: bool,
    ) -> PlatformResult<()> {
        self.update_overlay(|o| {
            o.dhcp = Some(ipv4_dhcp);
            o.address = if ipv4_dhcp {
                // Leaving a stale address behind DHCP means the next switch
                // back to static silently reuses an address nobody chose.
                None
            } else {
                ipv4_address.map(|a| format!("{a}/{}", ipv4_prefix_length.unwrap_or(24)))
            };
        })
    }

    async fn set_dns(
        &self,
        dns_servers: &[String],
        _search_domains: &[String],
    ) -> PlatformResult<()> {
        let servers = dns_servers.to_vec();
        self.update_overlay(|o| o.dns = Some(servers))
    }

    async fn set_gateway(&self, gateway: &str) -> PlatformResult<()> {
        let gateway = gateway.to_string();
        self.update_overlay(|o| o.gateway = Some(gateway))
    }
```

Check the exact `PlatformError` variant name before using `Io`:
`grep -n 'enum PlatformError' -A 20 cross-compile/onvif-rust/src/platform/common/*.rs`.

The interface token is ignored: `[wifi]` describes one interface, and
`anyka.toml` names it in `wifi.interface`. Say so in a comment rather than
pretending to honour it.

**Step 4: Run test to verify it passes**

```bash
$CARGO test --target x86_64-unknown-linux-gnu network_info
```

Expected: PASS, 3 tests.

**Step 5: Commit**

```bash
git add cross-compile/onvif-rust/src/platform/anyka/network_info.rs
git commit -m "feat(onvif): AnykaNetworkInfo writes the network overlay"
```

---

### Task 6: SetNetworkInterfaces handler and dispatch arm

**Files:**
- Modify: `cross-compile/onvif-rust/src/onvif/device/ops/network.rs`
- Modify: `cross-compile/onvif-rust/src/onvif/device/service.rs:482` (add the arm after `GetNetworkInterfaces`)
- Check: `cross-compile/onvif-rust/src/onvif/types/device.rs` for existing `SetNetworkInterfaces` request/response types
- Check: `cross-compile/onvif-rust/src/onvif/auth_requirements.rs` — the op must be listed as requiring auth

First confirm whether the SOAP types already exist:

```bash
grep -n "SetNetworkInterfaces" cross-compile/onvif-rust/src/onvif/types/device.rs
```

If absent, add request/response structs modelled on the neighbouring
`SetDNS`/`SetHostname` types, matching the XML the WebUI already sends
(`networkService.ts:156-165`).

**Step 1: Write the failing test**

In the `tests` module of `ops/network.rs`:

```rust
    #[tokio::test]
    async fn test_set_network_interfaces_rejects_a_malformed_address() {
        let platform = None;
        let request = SetNetworkInterfaces {
            interface_token: "eth0".to_string(),
            network_interface: /* IPv4 manual, address "not-an-ip", prefix 24 */,
        };

        let result = handle_set_network_interfaces(&platform, request).await;

        assert!(
            matches!(result, Err(OnvifError::InvalidArgVal { .. })),
            "a malformed address must fault, not be written to the overlay where \
             it would fail at boot instead"
        );
    }

    #[tokio::test]
    async fn test_set_network_interfaces_rejects_static_without_an_address() {
        let platform = None;
        let request = /* DHCP = false, no Manual block */;

        assert!(handle_set_network_interfaces(&platform, request).await.is_err());
    }

    #[tokio::test]
    async fn test_set_network_interfaces_without_a_platform_is_not_supported() {
        let platform = None;
        let request = /* valid DHCP = true request */;

        assert!(matches!(
            handle_set_network_interfaces(&platform, request).await,
            Err(OnvifError::ActionNotSupported(_))
        ));
    }
```

**Step 2: Run test to verify it fails**

```bash
$CARGO test --target x86_64-unknown-linux-gnu ops::network
```

Expected: FAIL to compile.

**Step 3: Write minimal implementation**

```rust
/// Handle SetNetworkInterfaces request.
///
/// Persists to the machine-owned overlay; anyka-init applies it at the next
/// boot. onvif-rust deliberately does not run `ifconfig` — the supervisor owns
/// the interface, and racing it is how a camera loses its only remote access.
pub async fn handle_set_network_interfaces(
    platform: &Option<Arc<dyn Platform>>,
    request: SetNetworkInterfaces,
) -> OnvifResult<SetNetworkInterfacesResponse> {
    let ipv4 = request.network_interface.ipv4.as_ref();
    let dhcp = ipv4.map(|v| v.dhcp).unwrap_or(true);
    let manual = ipv4.and_then(|v| v.manual.as_ref());

    let (address, prefix) = if dhcp {
        (None, None)
    } else {
        let m = manual.ok_or_else(|| {
            OnvifError::invalid_arg_val("NoConfig", "static addressing requires a Manual block")
        })?;
        validate_ipv4(&m.address)?;
        if !(1..=32).contains(&m.prefix_length) {
            return Err(OnvifError::invalid_arg_val(
                "NoConfig",
                "PrefixLength must be between 1 and 32",
            ));
        }
        (Some(m.address.clone()), Some(m.prefix_length as u8))
    };

    let network_info = platform
        .as_ref()
        .and_then(|p| p.network_info())
        .ok_or_else(|| OnvifError::ActionNotSupported("SetNetworkInterfaces".to_string()))?;

    network_info
        .set_network_interface(&request.interface_token, address, prefix, dhcp)
        .await
        .map_err(|e| OnvifError::Action(e.to_string()))?;

    tracing::info!(dhcp, "SetNetworkInterfaces: persisted; applies at next boot");

    // RebootNeeded is the honest answer: nothing has changed yet.
    Ok(SetNetworkInterfacesResponse { reboot_needed: true })
}
```

`validate_ipv4` may not exist. Check `device/faults.rs` — it already holds
`validate_hostname` (used at `ops/network.rs:70`), so add `validate_ipv4`
beside it with its own unit tests rather than inlining a regex here.

Match the exact `OnvifError` constructor names to what `onvif/error.rs`
actually provides.

Dispatch arm, after the `GetNetworkInterfaces` arm at `service.rs:482-494`:

```rust
            "SetNetworkInterfaces" => {
                dispatch_async(body_xml, |request: SetNetworkInterfaces| {
                    let platform = platform.clone();
                    async move {
                        network_ops::handle_set_network_interfaces(&platform, request).await
                    }
                })
                .await
            }
```

Add `SetNetworkInterfaces` to the import list at `service.rs:14-24` and to
`auth_requirements.rs` — a write op reachable unauthenticated is a finding, not
an oversight.

**Step 4: Run test to verify it passes**

```bash
$CARGO test --target x86_64-unknown-linux-gnu
```

Expected: PASS.

**Step 5: Commit**

```bash
git add cross-compile/onvif-rust/src/onvif/
git commit -m "feat(onvif): implement SetNetworkInterfaces against the overlay"
```

---

### Task 7: SetDNS and SetNetworkDefaultGateway

**Files:**
- Modify: `cross-compile/onvif-rust/src/onvif/device/ops/network.rs:325-334` (replace the `SetDNS` stub)
- Modify: `cross-compile/onvif-rust/src/onvif/device/service.rs:513` (pass `platform` into the closure)
- Modify: `cross-compile/onvif-rust/src/onvif/device/service.rs` (new `SetNetworkDefaultGateway` arm)

**Step 1: Write the failing test**

```rust
    #[tokio::test]
    async fn test_set_dns_is_no_longer_action_not_supported() {
        let platform = None;
        let request = SetDNS { from_dhcp: false, /* dns_manual: one IPv4 entry */ };

        let result = handle_set_dns(&platform, request).await;

        assert!(
            !matches!(result, Err(OnvifError::ActionNotSupported(_))),
            "SetDNS must reach the platform layer; the WebUI reported success for \
             this fault for months"
        );
    }

    #[tokio::test]
    async fn test_set_dns_rejects_a_malformed_server() {
        let platform = None;
        let request = SetDNS { from_dhcp: false, /* dns_manual: "999.1.1.1" */ };

        assert!(matches!(
            handle_set_dns(&platform, request).await,
            Err(OnvifError::InvalidArgVal { .. })
        ));
    }

    #[tokio::test]
    async fn test_set_dns_from_dhcp_clears_manual_servers() {
        // from_dhcp = true must write dns = [] so the overlay clears servers
        // the operator baseline supplied, rather than leaving them to be
        // merged back in at boot.
    }
```

**Step 2: Run test to verify it fails**

```bash
$CARGO test --target x86_64-unknown-linux-gnu ops::network
```

Expected: FAIL — `handle_set_dns` takes one argument and returns `ActionNotSupported`.

**Step 3: Write minimal implementation**

Replace the stub:

```rust
/// Handle SetDNS request.
///
/// Persists to the overlay; applied by anyka-init at next boot.
pub async fn handle_set_dns(
    platform: &Option<Arc<dyn Platform>>,
    request: SetDNS,
) -> OnvifResult<SetDNSResponse> {
    // from_dhcp writes an explicit empty list rather than omitting the key:
    // an omitted key is "unset", which merges the baseline's servers straight
    // back in and makes the toggle look inert.
    let servers: Vec<String> = if request.from_dhcp {
        Vec::new()
    } else {
        request
            .dns_manual
            .iter()
            .map(|a| a.ipv4_address.clone().unwrap_or_default())
            .collect()
    };
    for s in &servers {
        validate_ipv4(s)?;
    }

    let network_info = platform
        .as_ref()
        .and_then(|p| p.network_info())
        .ok_or_else(|| OnvifError::ActionNotSupported("SetDNS".to_string()))?;

    network_info
        .set_dns(&servers, &request.search_domain)
        .await
        .map_err(|e| OnvifError::Action(e.to_string()))?;

    Ok(SetDNSResponse {})
}
```

Add `handle_set_network_default_gateway` in the same shape, calling
`set_gateway`. Update both dispatch arms to clone and pass `platform`, matching
the `GetDNS` arm at `service.rs:505-511`.

Verify the real field names on `SetDNS` in `types/device.rs` — `search_domain`
and `dns_manual` are the likely spellings but confirm before writing.

**Step 4: Run test to verify it passes**

```bash
$CARGO test --target x86_64-unknown-linux-gnu
```

Expected: PASS.

**Step 5: Commit**

```bash
git add cross-compile/onvif-rust/src/onvif/
git commit -m "feat(onvif): implement SetDNS and SetNetworkDefaultGateway"
```

---

### Task 8: SetNetworkProtocols writes the listener ports

**Files:**
- Modify: `cross-compile/onvif-rust/src/onvif/device/ops/network.rs:485-497`
- Modify: `cross-compile/onvif-rust/src/onvif/device/service.rs:548`

Ports are onvif-rust's own (`ServerConfig.port`, `rtsp_port` at
`config/types.rs:486` and `:595`), so this one writes `config.toml`, not the
overlay. HTTPS is rejected: there is no TLS listener anywhere in the codebase,
and accepting the value would be the same lie the pane tells today.

**Step 1: Write the failing test**

```rust
    #[tokio::test]
    async fn test_set_network_protocols_updates_http_and_rtsp_ports() {
        let config = create_test_config();
        let request = /* HTTP enabled port 8080, RTSP enabled port 8554 */;

        handle_set_network_protocols(&config, request).await.expect("must succeed");

        assert_eq!(config.read().server.port, 8080);
        assert_eq!(config.read().media.rtsp_port, 8554);
    }

    #[tokio::test]
    async fn test_set_network_protocols_rejects_https() {
        let config = create_test_config();
        let request = /* HTTPS enabled port 443 */;

        assert!(
            handle_set_network_protocols(&config, request).await.is_err(),
            "there is no TLS listener; accepting the value would be the same lie \
             the stub told"
        );
    }

    #[tokio::test]
    async fn test_set_network_protocols_rejects_port_zero() {
        let config = create_test_config();
        let request = /* HTTP enabled port 0 */;
        assert!(handle_set_network_protocols(&config, request).await.is_err());
    }
```

Confirm the real path of `rtsp_port` first — `config/types.rs:595` is inside
some config section; `grep -n 'rtsp_port' cross-compile/onvif-rust/src/config/types.rs`
and check which struct owns it.

**Step 2: Run test to verify it fails**

```bash
$CARGO test --target x86_64-unknown-linux-gnu ops::network
```

Expected: FAIL — signature mismatch, returns `ActionNotSupported`.

**Step 3: Write minimal implementation**

```rust
/// Handle SetNetworkProtocols request.
///
/// Ports belong to this process's own listeners, so they go to `config.toml`,
/// not to the anyka-init overlay. Both listeners bind at startup, so the change
/// takes effect at the next restart.
pub async fn handle_set_network_protocols(
    config: &Arc<ConfigRuntime>,
    request: SetNetworkProtocols,
) -> OnvifResult<SetNetworkProtocolsResponse> {
    for proto in &request.network_protocols {
        let port = *proto.port.first().ok_or_else(|| {
            OnvifError::invalid_arg_val("NoConfig", "protocol entry carries no port")
        })?;
        if !(1..=65535).contains(&port) {
            return Err(OnvifError::invalid_arg_val(
                "NoConfig",
                "port must be between 1 and 65535",
            ));
        }
        match proto.name {
            NetworkProtocolType::HTTP => config.write().server.port = port as u16,
            NetworkProtocolType::RTSP => config.write().media.rtsp_port = port as u16,
            NetworkProtocolType::HTTPS => {
                return Err(OnvifError::ActionNotSupported(
                    "HTTPS: no TLS listener exists".to_string(),
                ));
            }
        }
    }
    Ok(SetNetworkProtocolsResponse {})
}
```

`ConfigRuntime` writes need to reach disk. Check how the identification work
persisted its mutations (`59b2aaf9 fix(device): persist identification mutations
to config.toml`) and follow the same pattern — likely a `ConfigStorage::save`
call or an existing dirty-flag mechanism. Do **not** invent a second path.

**Step 4: Run test to verify it passes**

```bash
$CARGO test --target x86_64-unknown-linux-gnu
```

Expected: PASS.

**Step 5: Commit**

```bash
git add cross-compile/onvif-rust/src/onvif/
git commit -m "feat(onvif): SetNetworkProtocols writes HTTP and RTSP ports"
```

---

### Task 9: GET and PUT /api/network

**Files:**
- Create: `cross-compile/onvif-rust/src/diagnostics/network.rs`
- Modify: `cross-compile/onvif-rust/src/diagnostics/mod.rs`
- Modify: `cross-compile/onvif-rust/src/onvif/server.rs:628-653` (register the routes)

This endpoint exists because ONVIF cannot express two things the pane needs:
*pending* state (saved but not applied), and Wi-Fi credentials without the
`Dot11Configuration` XML surface. Both routes mount inside the existing
authenticated `/api` router, alongside `/diagnostics` and `/update`.

**Step 1: Write the failing test**

Model the request construction on the existing endpoint tests at
`server.rs:1696` and `:1768`.

```rust
    #[tokio::test]
    async fn test_get_network_requires_auth() {
        // Same shape as the /api/diagnostics auth test at server.rs:1768.
        // Expected: 401 without credentials.
    }

    #[tokio::test]
    async fn test_put_network_requires_auth() {
        // A write endpoint reachable unauthenticated is a finding, not an oversight.
        // Expected: 401 without credentials.
    }

    #[tokio::test]
    async fn test_get_network_reports_pending_and_last_failure() {
        // With an overlay present, `pending` mirrors it.
        // With a .bad file present, `last_failure` is populated.
    }

    #[tokio::test]
    async fn test_put_network_rejects_an_empty_ssid() {
        // A blank SSID cannot associate and rung 2 would fire on every boot.
        // Expected: 400.
    }
```

**Step 2: Run test to verify it fails**

```bash
$CARGO test --target x86_64-unknown-linux-gnu network
```

Expected: FAIL — routes not registered, handlers absent.

**Step 3: Write minimal implementation**

```rust
//! `/api/network` — overlay state for the WebUI.
//!
//! Exists because ONVIF cannot express "saved but not yet applied", and
//! because Wi-Fi credentials over SOAP would mean the whole
//! `Dot11Configuration` type surface for one form.

use axum::{Json, extract::State, http::StatusCode, response::Response};
use serde::{Deserialize, Serialize};

use crate::config::netoverlay::NetworkOverlay;

#[derive(Debug, Serialize)]
pub struct NetworkStateResponse {
    /// Overlay contents: what will be applied at the next boot.
    pub pending: NetworkOverlay,
    /// Whether an overlay exists at all.
    pub has_pending: bool,
    /// A previous overlay that failed to bring the network up.
    pub last_failure: Option<NetworkOverlay>,
}

/// GET /api/network
pub async fn handle_get_network(/* State(...) */) -> Json<NetworkStateResponse> {
    // Read the overlay path from server state, not a hardcoded constant, so
    // the tests can redirect it.
    todo!("read overlay + .bad, return both")
}

/// PUT /api/network
pub async fn handle_put_network(
    /* State(...), */ Json(body): Json<NetworkOverlay>,
) -> Result<StatusCode, (StatusCode, String)> {
    // Validate before writing: an SSID that cannot associate costs a boot
    // cycle through rung 2, and a malformed CIDR costs the DHCP fallback.
    todo!("validate, read-modify-write the overlay, 204")
}
```

Never serialise the stored password back out in the GET response. Return a
boolean `has_password` instead and have the WebUI leave the field blank when a
password is already set. Add a test asserting the password never appears in the
GET body.

Registration in `server.rs`, next to `/update` (which is likewise gated on
`auth_enabled`):

```rust
            if state.auth_enabled {
                api = api.route("/update", put(crate::diagnostics::update::handle_update));
                api = api.route(
                    "/network",
                    get(crate::diagnostics::network::handle_get_network)
                        .put(crate::diagnostics::network::handle_put_network)
                        .layer(timeout()),
                );
            }
```

**Step 4: Run test to verify it passes**

```bash
$CARGO test --target x86_64-unknown-linux-gnu
```

Expected: PASS.

**Step 5: Commit**

```bash
git add cross-compile/onvif-rust/src/diagnostics/ cross-compile/onvif-rust/src/onvif/server.rs
git commit -m "feat(onvif): /api/network exposes pending and failed overlay state"
```

---

## Phase 3 — WebUI

### Task 10: Real gateway and real ports in networkService

**Files:**
- Modify: `cross-compile/www/src/services/networkService.ts:89` (hardcoded `gateway: ''`)
- Modify: `cross-compile/www/src/services/networkService.ts:133-137` (`getNetworkConfig`)
- Test: `cross-compile/www/src/services/networkService.test.ts`

**Step 1: Write the failing test**

```typescript
describe('getNetworkConfig', () => {
  it('populates the gateway from GetNetworkDefaultGateway', async () => {
    // mock GetNetworkDefaultGatewayResponse with IPv4Address 192.168.2.1
    const config = await getNetworkConfig();
    expect(config.interfaces[0].gateway).toBe('192.168.2.1');
  });

  it('reads real ports from GetNetworkProtocols', async () => {
    // mock HTTP port 80, RTSP port 554
    const config = await getNetworkConfig();
    expect(config.protocols).toEqual({ http: 80, rtsp: 554 });
  });

  it('leaves the gateway empty when the device reports none', async () => {
    const config = await getNetworkConfig();
    expect(config.interfaces[0].gateway).toBe('');
  });
});
```

Follow the existing mocking style in `networkService.test.ts` — do not invent a
new fixture pattern.

**Step 2: Run test to verify it fails**

```bash
cd cross-compile/www
npm test -- networkService
```

Expected: FAIL — gateway is `''`, `protocols` undefined.

**Step 3: Write minimal implementation**

Add `getNetworkDefaultGateway()` and `getNetworkProtocols()` alongside
`getDNS()`, then widen `getNetworkConfig`'s `Promise.all` and stitch the gateway
onto the first interface. Extend `NetworkConfig` with
`protocols: { http: number; rtsp: number }`.

**Step 4: Run test to verify it passes**

```bash
npm test -- networkService
```

Expected: PASS.

**Step 5: Commit**

```bash
git add cross-compile/www/src/services/networkService.ts cross-compile/www/src/services/networkService.test.ts
git commit -m "feat(webui): read real gateway and ports in networkService"
```

---

### Task 11: Overlay REST client

**Files:**
- Modify: `cross-compile/www/src/services/networkService.ts`
- Test: `cross-compile/www/src/services/networkService.test.ts`

Add `getNetworkOverlay()` and `putNetworkOverlay()` against `/api/network`.
Reuse whatever authenticated-fetch helper the firmware upload already uses for
`PUT /api/update` — find it with
`grep -rn '/api/update' cross-compile/www/src`. Do not hand-roll a second auth
header path.

Tests: shape of the GET response, that a PUT sends only changed keys, and that
a non-2xx response rejects rather than resolving.

**Commit:** `feat(webui): overlay client for /api/network`

---

### Task 12: Remove the duplicated hostname and discovery controls

**Files:**
- Modify: `cross-compile/www/src/pages/settings/NetworkPage.tsx:74-79, 105-110, 130-135, 276-341`
- Modify: `cross-compile/www/src/pages/settings/NetworkPage.test.tsx`

Delete the `hostname` and `onvifDiscovery` schema fields, defaults, reset
values, and both form cards. Replace with a line linking to Settings ›
Identification, where both already work for real.

**Step 1: Write the failing test**

```typescript
it('does not render a hostname input; Identification owns it', () => {
  const { queryByTestId } = renderWithProviders(<NetworkPage />);
  expect(queryByTestId('network-hostname-input')).toBeNull();
});

it('does not render an ONVIF discovery switch; Identification owns it', () => {
  const { queryByTestId } = renderWithProviders(<NetworkPage />);
  expect(queryByTestId('network-onvif-discovery-switch')).toBeNull();
});

it('links to the Identification pane instead', () => {
  const { getByTestId } = renderWithProviders(<NetworkPage />);
  expect(getByTestId('network-identification-link')).toHaveAttribute(
    'href',
    '/settings/identification',
  );
});
```

Confirm the real route path in `cross-compile/www/src/router/index.tsx` before
asserting it.

**Steps 2-5:** run, implement, re-run, commit
(`refactor(webui): drop duplicated hostname and discovery from Network`).

---

### Task 13: Wi-Fi Network card

**Files:**
- Modify: `cross-compile/www/src/pages/settings/NetworkPage.tsx`
- Modify: `cross-compile/www/src/pages/settings/NetworkPage.test.tsx`

New `SettingsCard` above IP Configuration with `ssid`, `password`, `security`.
Follow the existing card structure (`NetworkPage.tsx:261-274`) and the design
tokens already in the file — do not introduce new colours.

Schema additions:

```typescript
  ssid: z.string().min(1, 'SSID is required').max(32, 'SSID must be 32 characters or fewer'),
  password: z.string().max(63, 'WPA passphrases are at most 63 characters'),
  security: z.enum(['wpa', 'wep', 'none']),
```

Tests: SSID required, the password field renders blank when `has_password` is
true and only submits when the user typed something, and the 63-character WPA
bound.

**Commit:** `feat(webui): Wi-Fi credentials card on the Network pane`

---

### Task 14: Pending badges and the failure banner

**Files:**
- Modify: `cross-compile/www/src/pages/settings/NetworkPage.tsx`
- Modify: `cross-compile/www/src/pages/settings/NetworkPage.test.tsx`

A `useQuery` on `getNetworkOverlay` drives both. A card shows a "Pending
reboot" badge when the overlay holds a value differing from the live one; a
`last_failure` renders a dismissible banner above the form.

**Step 1: Write the failing test**

```typescript
it('badges IP Configuration as pending when the overlay differs from live', async () => {
  // live: dhcp true; overlay: dhcp false, address 192.168.2.50/24
  const { findByTestId } = renderWithProviders(<NetworkPage />);
  expect(await findByTestId('network-ip-pending-badge')).toBeInTheDocument();
});

it('shows no badge when the overlay matches the live config', async () => {
  const { queryByTestId } = renderWithProviders(<NetworkPage />);
  await waitFor(() => expect(queryByTestId('network-ip-pending-badge')).toBeNull());
});

it('warns when the previous Wi-Fi settings failed and were reverted', async () => {
  // last_failure: { ssid: 'TypoNet' }
  const { findByTestId } = renderWithProviders(<NetworkPage />);
  expect(await findByTestId('network-failure-banner')).toHaveTextContent('TypoNet');
});
```

**Steps 2-5:** run, implement, re-run, commit
(`feat(webui): pending badges and overlay failure banner`).

---

### Task 15: Ports card — HTTP and RTSP, with an honest confirm dialog

**Files:**
- Modify: `cross-compile/www/src/pages/settings/NetworkPage.tsx:476-548, 592-623`
- Modify: `cross-compile/www/src/pages/settings/NetworkPage.test.tsx`

Delete the HTTPS input and its schema field. Bind HTTP and RTSP to the values
from Task 10. When the HTTP port changes, the confirm dialog must spell out the
URL the user will need after the reboot — a port change is otherwise
indistinguishable from a hang.

**Step 1: Write the failing test**

```typescript
it('does not render an HTTPS port input; no TLS listener exists', () => {
  const { queryByTestId } = renderWithProviders(<NetworkPage />);
  expect(queryByTestId('network-https-port-input')).toBeNull();
});

it('spells out the new URL when the HTTP port changes', async () => {
  // change http port 80 -> 8080, submit
  const { findByTestId } = renderWithProviders(<NetworkPage />);
  expect(await findByTestId('network-confirm-dialog')).toHaveTextContent(':8080');
});

it('rejects a port outside 1-65535', async () => { /* ... */ });
```

**Steps 2-5:** run, implement, re-run, commit
(`feat(webui): writable HTTP and RTSP ports, HTTPS input removed`).

---

### Task 16: Stop reporting success for failed saves

**Files:**
- Modify: `cross-compile/www/src/pages/settings/NetworkPage.tsx:140-163`
- Modify: `cross-compile/www/src/pages/settings/NetworkPage.test.tsx`

This is the defect that made the whole pane untrustworthy: `onSuccess` fired a
"Network settings saved" toast for calls that returned SOAP faults. With the
handlers implemented, the mutation must also stop swallowing partial failure —
the current `mutationFn` awaits `setNetworkInterface` then `setDNS` with no
handling for the first succeeding and the second faulting.

**Step 1: Write the failing test**

```typescript
it('reports failure when a save faults', async () => {
  vi.mocked(setDNS).mockRejectedValue(new Error('ActionNotSupported'));
  // submit and confirm
  await waitFor(() => expect(toast.error).toHaveBeenCalled());
  expect(toast.success).not.toHaveBeenCalled();
});

it('reports which part failed when the interface saves but DNS does not', async () => {
  vi.mocked(setNetworkInterface).mockResolvedValue(undefined);
  vi.mocked(setDNS).mockRejectedValue(new Error('boom'));
  await waitFor(() =>
    expect(toast.error).toHaveBeenCalledWith(
      expect.stringContaining('DNS'),
      expect.anything(),
    ),
  );
});

it('says the change applies after reboot, not that it is live', async () => {
  // success path must not claim the device is already reconfigured
  await waitFor(() =>
    expect(toast.success).toHaveBeenCalledWith(
      expect.anything(),
      expect.objectContaining({ description: expect.stringContaining('reboot') }),
    ),
  );
});
```

**Steps 2-5:** run, implement, re-run, commit
(`fix(webui): surface network save failures instead of reporting success`).

---

## Phase 4 — Verification

### Task 17: Cross-crate guard and full quality gates

**Files:**
- Modify: `cross-compile/onvif-rust/src/config/netoverlay.rs` (add the guard test)

The two `NetworkOverlay` definitions must not drift. anyka-init parses with
`deny_unknown_fields`, so a key added on the onvif-rust side and not the other
turns a WebUI save into a failed boot — a failure that only shows up on
hardware, long after CI is green.

**Step 1: Write the guard test**

```rust
    #[test]
    fn test_serialised_keys_match_the_anyka_init_schema() {
        // anyka-init/src/netoverlay.rs parses with deny_unknown_fields. Any key
        // added here and not there turns a save into a failed boot.
        let all = NetworkOverlay {
            ssid: Some("s".into()),
            password: Some("p".into()),
            security: Some("wpa".into()),
            dhcp: Some(false),
            address: Some("10.0.0.1/24".into()),
            gateway: Some("10.0.0.254".into()),
            dns: Some(vec!["10.0.0.254".into()]),
        };
        let src = toml::to_string_pretty(&all).expect("serialise");
        let mut keys: Vec<&str> = src
            .lines()
            .filter_map(|l| l.split('=').next())
            .map(str::trim)
            .filter(|k| !k.is_empty() && !k.starts_with('['))
            .collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            ["address", "dhcp", "dns", "gateway", "password", "security", "ssid"],
            "overlay keys changed: update anyka-init/src/netoverlay.rs to match, \
             or anyka-init will reject the file at boot"
        );
    }
```

**Step 2: Run the full gates**

```bash
source ./setenv.sh

cd cross-compile/onvif-rust
$CARGO fmt --check
$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
$CARGO test --target x86_64-unknown-linux-gnu

cd ../anyka-init
$CARGO fmt --check
$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
$CARGO test --target x86_64-unknown-linux-gnu

cd ../www
npm run lint
npm test
```

Expected: all pass, zero warnings.

**Step 3: Build for ARM**

PR CI never cross-builds armv5te — `release.yml` is the only workflow that
does, so a break here merges green and is discovered on the camera. Build it
yourself, from the crate directory:

```bash
cd cross-compile/onvif-rust
$CARGO build --release
cd ../anyka-init
$CARGO build --release
```

Expected: both link. If either falls back to the host toolchain, you ran it from
the workspace root.

**Step 4: Commit**

```bash
git add cross-compile/onvif-rust/src/config/netoverlay.rs
git commit -m "test(onvif): guard the overlay schema against anyka-init drift"
```

---

### Task 18: Hardware validation on .198

**Target: 192.168.2.198 only.** Not .127 — it runs the legacy stack and its
deadman reverts. Telnet is port 24, no login, and needs the input paced.

Deploy per @anyka-firmware-upgrade, then walk these in order. The first two are
the ones that matter: they prove the rescue ladder, and a rescue path that has
never fired is a rescue path that does not work.

| # | Scenario | Steps | Expected |
|---|---|---|---|
| 1 | Static IP, good gateway | Set a static address on the current subnet, save, reboot | Camera returns on the new address; badge clears |
| 2 | **Static IP, wrong gateway** | Set gateway `192.168.99.1`, save, reboot | `gateway_reachable` fails → udhcpc → camera returns on a DHCP address |
| 3 | **Wrong SSID** | Set SSID `TypoNet`, save, reboot | Rung 2: `network.toml` → `network.toml.bad`, baseline retried, camera returns; WebUI shows the failure banner |
| 4 | DNS change | Set primary DNS, save, reboot | `/etc/resolv.conf` reflects it |
| 5 | HTTP port change | 80 → 8080, save, reboot | WebUI reachable on `:8080`; confirm dialog had said so |
| 6 | Factory reset | `rm /mnt/anyka_hack/network.toml`, reboot | Operator baseline from `anyka.toml` restored |

Verify each over telnet before trusting the UI:

```bash
cat /mnt/anyka_hack/network.toml
ls -la /mnt/anyka_hack/network.toml.bad
ifconfig wlan0
cat /etc/resolv.conf
route -n
```

Have physical access to the SD card before running scenario 3. It is the one
that exercises the path with no software rescue behind it.

**Step: Record the results**

Append a results table to `docs/plans/2026-08-19-network-pane-design.md` under a
new "Hardware validation" heading and commit.

---

## Definition of Done

- [ ] All six ONVIF/REST write paths reach storage; none returns `ActionNotSupported` except HTTPS, deliberately
- [ ] No control on the pane writes a value the backend discards
- [ ] Hostname and ONVIF discovery appear on exactly one pane
- [ ] Failed saves produce error toasts, never success toasts
- [ ] Pending badges reflect real overlay-versus-live difference
- [ ] Both rescue rungs verified on hardware (scenarios 2 and 3)
- [ ] `fmt`, `clippy -D warnings`, and all three test suites pass
- [ ] Both crates cross-build for armv5te
- [ ] Code review requested per @superpowers:requesting-code-review
