# SNMP Integration Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Ship a read-only SNMPv2c agent (MIB-II `system` + basic `interfaces`) as a separate binary supervised by `anyka-init`, with `snmp.toml` + ONVIF + WebUI control.

**Architecture:** Hand-rolled minimal UDP agent (`snmp-agent`) owns port 161. `onvif-rust` writes `snmp.toml` and extends `Get/SetNetworkProtocols` with SNMP. WebUI edits enable/port/community. Init supervises the binary via `[services.snmp]`. SNMPv3 is a later module seam only.

**Tech Stack:** Rust workspace member (tokio UDP), existing `anyka-init` `ServiceCfg`, ONVIF device network ops, React Network page + `networkService`.

**Design:** `docs/plans/2026-08-25-snmp-integration-design.md`

**Worktree:** `/home/kmk/dev/anyka-dev/.worktrees/snmp` on `feat/snmp`

**Toolchain:** Always `source /home/kmk/dev/anyka-dev/setenv.sh` (extracted toolchain lives in main checkout). Host checks use `--target x86_64-unknown-linux-gnu`.

---

## Task 1: Scaffold `snmp-agent` workspace crate

**Files:**
- Create: `cross-compile/snmp-agent/Cargo.toml`
- Create: `cross-compile/snmp-agent/src/main.rs`
- Create: `cross-compile/snmp-agent/src/lib.rs`
- Modify: `cross-compile/Cargo.toml` (add workspace member)

**Step 1: Add workspace member and crate skeleton**

`cross-compile/Cargo.toml` — add `"snmp-agent"` to `members`.

`cross-compile/snmp-agent/Cargo.toml`:

```toml
[package]
name = "snmp-agent"
version = "0.1.0"
edition = "2024"
publish = false

[[bin]]
name = "snmp-agent"
path = "src/main.rs"

[dependencies]
serde = { version = "1", features = ["derive"] }
toml = "0.8"
tracing = "0.1"
tracing-subscriber = { version = "0.3", default-features = false, features = ["fmt", "std"] }
thiserror = "2"
tokio = { version = "1", features = ["rt-multi-thread", "macros", "net", "signal", "time", "sync"] }

[dev-dependencies]
tempfile = "3"
```

Pin versions to match workspace/`onvif-rust` where possible when editing.

`src/lib.rs`:

```rust
//! Read-only SNMPv2c agent (MIB-II system + interfaces).

pub mod config;
```

`src/main.rs`:

```rust
fn main() {
    tracing_subscriber::fmt::init();
    tracing::info!("snmp-agent starting");
}
```

Create empty `src/config.rs` with `// placeholder` until Task 2.

**Step 2: Verify it builds on host**

```bash
cd /home/kmk/dev/anyka-dev/.worktrees/snmp
source /home/kmk/dev/anyka-dev/setenv.sh
cd cross-compile
$CARGO build -p snmp-agent --target x86_64-unknown-linux-gnu
```

Expected: success.

**Step 3: Commit**

```bash
git add cross-compile/Cargo.toml cross-compile/snmp-agent
git commit -m "feat(snmp): scaffold snmp-agent workspace crate"
```

---

## Task 2: `snmp.toml` config load/defaults (TDD)

**Files:**
- Create: `cross-compile/snmp-agent/src/config.rs`
- Test in same file under `#[cfg(test)]`

**Step 1: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_default_config_values() {
        let c = SnmpConfig::default();
        assert!(c.enabled);
        assert_eq!(c.port, 161);
        assert_eq!(c.community, "public");
        assert_eq!(c.sys_contact, "");
        assert_eq!(c.sys_name, "");
        assert_eq!(c.sys_location, "");
    }

    #[test]
    fn test_load_missing_file_returns_defaults() {
        let c = SnmpConfig::load("/no/such/snmp.toml").expect("defaults");
        assert_eq!(c, SnmpConfig::default());
    }

    #[test]
    fn test_load_parses_toml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("snmp.toml");
        std::fs::write(
            &path,
            r#"
enabled = false
port = 1161
community = "monitor"
sys_contact = "ops@example"
sys_name = "cam-1"
sys_location = "lab"
"#,
        )
        .unwrap();
        let c = SnmpConfig::load(&path).unwrap();
        assert!(!c.enabled);
        assert_eq!(c.port, 1161);
        assert_eq!(c.community, "monitor");
        assert_eq!(c.sys_name, "cam-1");
    }

    #[test]
    fn test_load_rejects_port_zero() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("snmp.toml");
        std::fs::write(&path, "port = 0\n").unwrap();
        assert!(SnmpConfig::load(&path).is_err());
    }
}
```

**Step 2: Run tests — expect FAIL**

```bash
$CARGO test -p snmp-agent --target x86_64-unknown-linux-gnu --lib
```

**Step 3: Implement `SnmpConfig`**

```rust
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnmpConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_community")]
    pub community: String,
    #[serde(default)]
    pub sys_contact: String,
    #[serde(default)]
    pub sys_name: String,
    #[serde(default)]
    pub sys_location: String,
}

fn default_enabled() -> bool { true }
fn default_port() -> u16 { 161 }
fn default_community() -> String { "public".to_string() }

impl Default for SnmpConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            port: 161,
            community: default_community(),
            sys_contact: String::new(),
            sys_name: String::new(),
            sys_location: String::new(),
        }
    }
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("read snmp config: {0}")]
    Io(#[from] std::io::Error),
    #[error("parse snmp config: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("invalid snmp port {0}")]
    InvalidPort(u16),
}

impl SnmpConfig {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(path)?;
        let cfg: Self = toml::from_str(&text)?;
        if cfg.port == 0 {
            return Err(ConfigError::InvalidPort(cfg.port));
        }
        Ok(cfg)
    }
}
```

Default path constant (device): `/mnt/anyka_hack/snmp.toml` — export as `DEFAULT_CONFIG_PATH`.

**Step 4: Run tests — expect PASS**

**Step 5: Commit**

```bash
git add cross-compile/snmp-agent
git commit -m "feat(snmp): load snmp.toml with public/161 defaults"
```

---

## Task 3: Minimal SNMPv2c BER helpers (TDD)

**Files:**
- Create: `cross-compile/snmp-agent/src/ber.rs`
- Create: `cross-compile/snmp-agent/src/pdu.rs`
- Modify: `cross-compile/snmp-agent/src/lib.rs`

Focus: only what GET/GETNEXT/response/error need — not a full ASN.1 stack.

**Step 1: Failing tests for OID encode/decode and a GET request parse**

Cover at least:
- `Oid::from_slice(&[1,3,6,1,2,1,1,1,0])` round-trip
- Parse a captured or hand-built SNMPv2c GET for `sysDescr.0` with community `public`
- Reject version != 1 (v2c wire version is 1)

**Step 2: Implement minimal TLV readers/writers** (`INTEGER`, `OCTET STRING`, `NULL`, `OID`, `SEQUENCE`, SNMP PDU context tags).

**Step 3: Tests pass; commit**

```bash
git commit -m "feat(snmp): minimal BER/PDU encode-decode for v2c GET"
```

---

## Task 4: MIB-II `system` + GETNEXT order (TDD)

**Files:**
- Create: `cross-compile/snmp-agent/src/mib/mod.rs`
- Create: `cross-compile/snmp-agent/src/mib/system.rs`
- Modify: `lib.rs`

**Step 1: Tests**

- GET `sysUpTime.0` returns TimeTicks from injected uptime
- GETNEXT from `1.3.6.1.2.1.1` yields `sysDescr.0`
- Walk order: descr → objectID → uptime → contact → name → location → services → end of system
- SET path returns `notWritable` error status

**Step 2: Implement fixed OID table + value provider trait** taking `&SnmpConfig` + uptime fn.

**Step 3: Commit**

```bash
git commit -m "feat(snmp): MIB-II system group GET/GETNEXT"
```

---

## Task 5: MIB-II `interfaces` from `/proc` fixtures (TDD)

**Files:**
- Create: `cross-compile/snmp-agent/src/mib/interfaces.rs`
- Create: `cross-compile/snmp-agent/tests/fixtures/proc_net_dev.txt`

**Step 1: Tests with fixture** matching embedded `/proc/net/dev` shape (lo + eth0/wlan0). Assert `ifNumber`, `ifDescr.N`, `ifOperStatus`, octet counters.

**Step 2: Implement parser + ifTable columnar GET/GETNEXT** chained after system in the global walk order.

**Step 3: Commit**

```bash
git commit -m "feat(snmp): MIB-II ifTable from /proc/net/dev"
```

---

## Task 6: UDP server, community check, SIGHUP reload

**Files:**
- Create: `cross-compile/snmp-agent/src/server.rs`
- Modify: `cross-compile/snmp-agent/src/main.rs`

**Behavior:**
- Bind `0.0.0.0:port` when `enabled`; when disabled, run loop without bind (or unbound) and wait for SIGHUP
- Wrong community → silent drop
- Bad PDU → drop
- SET → `notWritable` response
- SIGHUP → reload config; rebind on port change; on bind failure keep old socket + error log
- Optional pidfile `/var/run/snmp-agent.pid` or `/tmp/snmp-agent.pid` for onvif-rust SIGHUP

**Tests:** bind ephemeral port in tokio test; send crafted GET; assert response bytes / sysName.

**Commit:**

```bash
git commit -m "feat(snmp): UDP agent loop with SIGHUP config reload"
```

---

## Task 7: Wire `anyka-init` + SD payload service

**Files:**
- Modify: `SD_card_contents/anyka_hack/anyka.toml` — add `[services.snmp]`
- Modify: deploy scripts / copy path as needed so `snmp-agent.bin` lands next to onvif (follow existing onvif/vendor-daemon pattern under `SD_card_contents/anyka_hack/snmp/` or similar)
- Modify: `cross-compile/anyka-init` tests only if a fixture anyka.toml must include the service (optional)

**Service stanza:**

```toml
[services.snmp]
enabled = true
exec = "/mnt/anyka_hack/snmp/snmp-agent.bin"
args = ["--config", "/mnt/anyka_hack/snmp.toml"]
log = "/mnt/logs/snmp-agent.log"
core_dump = true
```

Confirm `ServiceCfg` supports `args` (it does for udhcpc). Add CLI flag parsing in agent for `--config`.

**Commit:**

```bash
git commit -m "feat(snmp): supervise snmp-agent from anyka-init"
```

---

## Task 8: ONVIF `NetworkProtocolType::SNMP` + `snmp.toml` persistence

**Files:**
- Modify: `cross-compile/onvif-rust/src/onvif/types/device.rs` — add `SNMP` variant to `NetworkProtocolType`
- Modify: `cross-compile/onvif-rust/src/onvif/device/ops/network.rs` — get/set handlers
- Create: `cross-compile/onvif-rust/src/config/snmp.rs` (load/store `snmp.toml`, mirror `netoverlay` atomic write style)
- Wire path constant `/mnt/anyka_hack/snmp.toml`
- SIGHUP helper (read pidfile; ignore if missing)

**Note:** Official ONVIF `NetworkProtocolType` is HTTP/HTTPS/RTSP only. Treat `SNMP` as a **vendor extension** enumeration value (serde rename `"SNMP"`). Document in code comment.

**Get:** include SNMP entry from `snmp.toml` alongside HTTP/RTSP.

**Set:**
- Allow `enabled=false` for SNMP (unlike HTTP/RTSP)
- Persist community? Design: community is WebUI/config primarily; ONVIF protocol object has Name/Enabled/Port only — **community stays in `snmp.toml` / WebUI**, not in SetNetworkProtocols body unless we add a vendor extension later
- Port + enabled from ONVIF update `snmp.toml`; SIGHUP agent

**Tests:** extend existing network protocol unit tests in `network.rs`.

**Commit:**

```bash
git commit -m "feat(snmp): ONVIF Get/SetNetworkProtocols SNMP extension"
```

---

## Task 9: WebUI Network page — SNMP controls

**Files:**
- Modify: `cross-compile/www/src/services/networkService.ts` — extend `NetworkProtocols` with `snmp: { enabled, port }` (and community via small REST or fold into existing diagnostics-style overlay if cleaner)
- Prefer: extend ONVIF get/set for port/enabled **plus** a tiny `GET/PATCH /api/snmp` for community/sys* (keeps community out of SOAP). If that splits surface too much, single REST resource for all of `snmp.toml` and keep ONVIF port/enabled in sync from the same store.

**Practical v1 (recommended in this plan):**  
- REST `GET/PATCH /api/snmp` in `onvif-rust` (like network overlay) owning full `snmp.toml`  
- ONVIF Get/SetNetworkProtocols reads/writes the same file for enabled/port only  
- WebUI uses REST for the SNMP card (enable, port, community) — simpler than stuffing community into SOAP

**Files for REST:** `cross-compile/onvif-rust/src/diagnostics/` or `src/api/snmp.rs` + router registration (follow network overlay pattern).

**WebUI:**
- `NetworkPage.tsx` — SNMP section under ports
- `NetworkPage.test.tsx` + `networkService` tests
- Security note under community field: default `public` is insecure

**Commit:**

```bash
git commit -m "feat(www): SNMP enable/port/community on Network page"
```

---

## Task 10: Cross-build, SD image, device smoke docs

**Files:**
- Modify: build/deploy scripts that copy `onvif-rust.bin` to also build/copy `snmp-agent`
- Modify: wiki or `SD_card_contents/anyka_hack/README.md` — short SNMP section (community `public`, port 161, `snmpwalk` example)

**Step 1: ARM release build**

```bash
source /home/kmk/dev/anyka-dev/setenv.sh
cd cross-compile
$CARGO build --release -p snmp-agent
```

**Step 2: Deploy via existing SD/deploy path; on device:**

```bash
snmpwalk -v2c -c public <cam-ip> 1.3.6.1.2.1.1
snmpwalk -v2c -c public <cam-ip> 1.3.6.1.2.1.2
```

**Step 3: Commit script/docs updates**

```bash
git commit -m "docs(snmp): deploy path and snmpwalk smoke instructions"
```

---

## Task 11: Quality gates

```bash
source /home/kmk/dev/anyka-dev/setenv.sh
cd cross-compile
$CARGO fmt
$CARGO clippy --target x86_64-unknown-linux-gnu -p snmp-agent -p onvif-rust -p anyka-init -- -D warnings
$CARGO test --target x86_64-unknown-linux-gnu -p snmp-agent -p onvif-rust
cd www && npm run lint && npm run type-check && npm run test
```

Fix until green. Then `ponytail-review` / request code review per `AGENTS.md`.

---

## Execution notes

- YAGNI: no traps, no SET, no v3 crypto, no private MIB.
- Wrong community = silent drop.
- Never log the community string in info/debug without redaction.
- Prefer atomic write for `snmp.toml` (write temp + rename), same spirit as `network.toml`.
