# snmp-agent Ponytail Cleanup Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Cut ~300 lines of over-engineering from `snmp-agent` and its onvif-rust/WebUI mirrors without changing what an SNMP manager sees on the wire.

**Architecture:** Fourteen findings from a ponytail audit (2026-10-04), applied smallest-risk first. Tasks 1–11 stay inside `cross-compile/snmp-agent`. Tasks 12–15 remove the second SNMP on/off switch: `[services.snmp] enabled` in `anyka.toml` (anyka-init, toggled from Diagnostics → Processes) becomes the only switch, `snmp.toml` loses `enabled`, onvif-rust parses `snmp.toml` with the agent's own struct, and the Network page loses its Enable switch.

**Tech Stack:** Rust 2024 (tokio, serde/toml, tracing), axum (onvif-rust REST), React 19 + zod + vitest (WebUI).

---

## Conventions (read once)

- Host cargo runs from `cross-compile/` with the vendored toolchain:
  `CARGO=../toolchain/arm-anykav200-crosstool-ng/bin/cargo` and **always** `--target x86_64-unknown-linux-gnu`.
- Clippy needs the vendored bin dir first on `PATH`, or it dies with E0514:
  `PATH=$PWD/../toolchain/arm-anykav200-crosstool-ng/bin:$PATH $CARGO clippy --target x86_64-unknown-linux-gnu …`
- The ARM build runs from `cross-compile/onvif-rust/` (that is where `.cargo/config.toml` with the ARM linker lives), never from `cross-compile/`.
- This repo's index is often fully staged. **Every commit passes an explicit pathspec** (`git commit -m … -- <paths>`), never a bare `git commit`.
- Prefix git with `rtk`. Do not trust `rtk prettier`/`rtk diff` verdicts; run the raw binary and read `$?`.
- Commit messages end with the `Co-Authored-By:` trailer from the session's attribution instructions.
- Baseline (2026-10-04, `main` @ 194592ac): `snmp-agent` 58 lib + 1 bin + 3 integration tests; `onvif-rust --lib config::snmp` 7 tests. All green.

---

### Task 0: Worktree prerequisites and ARM size baseline

The worktree `.worktrees/snmp-agent-ponytail-cleanup` already has the toolchain symlink and `patches/*-full`. Two things remain.

**Step 1: WebUI deps**

Run: `cd cross-compile/www && npm ci`
Expected: completes without errors.

**Step 2: Record the ARM release size of snmp-agent (used in Task 16 to catch bloat from Task 11)**

```bash
cd cross-compile/onvif-rust
[ -f .cargo/config.toml ] || ./scripts/setup-cargo-config.sh
../../toolchain/arm-anykav200-crosstool-ng/bin/cargo build --release -p snmp-agent
ls -l ../target/armv5te-unknown-linux-uclibceabi/release/snmp-agent
```

Write the byte count into your notes. Nothing to commit.

---

### Task 1: Drop unused Cargo features

`main` is `#[tokio::main(flavor = "current_thread")]`, so `rt-multi-thread` is dead weight (`rt` is all it needs). The custom `LocalTimer` never touches the `time` crate, so tracing-subscriber's `time` feature is dead too.

**Files:**
- Modify: `cross-compile/snmp-agent/Cargo.toml:16,18`

**Step 1: Edit features**

```toml
tracing-subscriber = { version = "0.3", default-features = false, features = ["fmt", "std"] }
tokio = { version = "1", features = ["rt", "macros", "net", "signal", "time", "sync"] }
```

**Step 2: Verify**

Run: `$CARGO test --target x86_64-unknown-linux-gnu -p snmp-agent 2>&1 | grep "test result"`
Expected: `58 passed`, `1 passed`, `3 passed`, `0 passed` (doc), no failures.

**Step 3: Commit**

```bash
rtk git commit -m "chore(snmp-agent): drop unused tokio/tracing-subscriber features" -- snmp-agent/Cargo.toml Cargo.lock
```

(All commit commands run from `cross-compile/`, so `Cargo.lock` here is `cross-compile/Cargo.lock`. Naming an unchanged tracked file in the pathspec is harmless.)

---

### Task 2: `Oid` derives `Ord` (stdlib replaces two `oid_less`)

`Vec<u32>` already orders lexicographically, with a prefix sorting before its extensions. That is exactly SNMP's OID order, so the hand-rolled `oid_less` in two files can go.

**Files:**
- Modify: `cross-compile/snmp-agent/src/ber.rs:12-13` (derive + new test)
- Modify: `cross-compile/snmp-agent/src/mib/system.rs` (delete `oid_less`, use `<`)
- Modify: `cross-compile/snmp-agent/src/mib/interfaces.rs` (delete `oid_less`, use `<`)

**Step 1: Write the failing test** (append inside `mod tests` in `ber.rs`)

```rust
    #[test]
    fn test_oid_orders_like_snmp() {
        let o = |a: &[u32]| Oid(a.to_vec());
        // A prefix sorts before its extensions (GETNEXT on a group yields its first leaf).
        assert!(o(&[1, 3, 6, 1, 2, 1, 1]) < o(&[1, 3, 6, 1, 2, 1, 1, 1, 0]));
        assert!(o(&[1, 3, 6, 1, 2, 1, 1, 7, 0]) < o(&[1, 3, 6, 1, 2, 1, 2]));
        // Arcs compare numerically, not bytewise: column 2 before column 10.
        assert!(o(&[1, 3, 6, 1, 2, 1, 2, 2, 1, 2, 1]) < o(&[1, 3, 6, 1, 2, 1, 2, 2, 1, 10, 1]));
    }
```

**Step 2: Run it to verify it fails**

Run: `$CARGO test --target x86_64-unknown-linux-gnu -p snmp-agent --lib test_oid_orders_like_snmp`
Expected: compile error `binary operation '<' cannot be applied to type 'Oid'`.

**Step 3: Implement**

In `ber.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Oid(pub Vec<u32>);
```

In both `mib/system.rs` and `mib/interfaces.rs`: delete the whole `fn oid_less(a: &Oid, b: &Oid) -> bool { … }` and replace each `if oid_less(oid, &candidate)` with `if oid < &candidate`.

**Step 4: Run tests**

Run: `$CARGO test --target x86_64-unknown-linux-gnu -p snmp-agent 2>&1 | grep "test result"`
Expected: `59 passed` lib; the others unchanged.

**Step 5: Commit**

```bash
rtk git commit -m "refactor(snmp-agent): derive Ord on Oid instead of hand-rolled oid_less" -- snmp-agent/src/ber.rs snmp-agent/src/mib/system.rs snmp-agent/src/mib/interfaces.rs
```

---

### Task 3: `PduType` carries its BER tag as the discriminant

**Files:**
- Modify: `cross-compile/snmp-agent/src/pdu.rs:11-47` and the call site in `encode_pdu`, plus `test_pdu_type_tags_cover_getnext_and_set`

**Step 1: Replace the five `PDU_*` consts, the enum and its `impl`** with:

```rust
/// SNMP PDU type; the discriminant is the context-specific BER tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PduType {
    GetRequest = 0xa0,
    GetNextRequest = 0xa1,
    GetResponse = 0xa2,
    SetRequest = 0xa3,
    /// SNMPv2c GetBulkRequest — `error_status`/`error_index` hold non-repeaters / max-repetitions.
    GetBulkRequest = 0xa5,
}

impl PduType {
    fn from_tag(tag: u8) -> Option<Self> {
        [
            Self::GetRequest,
            Self::GetNextRequest,
            Self::GetResponse,
            Self::SetRequest,
            Self::GetBulkRequest,
        ]
        .into_iter()
        .find(|t| *t as u8 == tag)
    }
}
```

**Step 2: Fix the call sites**

- `encode_pdu`: `ber::write_tlv(pdu.pdu_type.tag(), &body, &mut out);` → `ber::write_tlv(pdu.pdu_type as u8, &body, &mut out);`
- `test_pdu_type_tags_cover_getnext_and_set`: replace each `PduType::X.tag()` with `PduType::X as u8`.

**Step 3: Run tests** (this is a refactor; the existing tag test is the guard)

Run: `$CARGO test --target x86_64-unknown-linux-gnu -p snmp-agent 2>&1 | grep "test result"`
Expected: unchanged counts, no failures.

**Step 4: Commit**

```bash
rtk git commit -m "refactor(snmp-agent): PduType discriminant is its BER tag" -- snmp-agent/src/pdu.rs
```

---

### Task 4: Drop `SnmpMessage.version`

`parse` rejects anything other than v2c, so the field is always `SNMP_V2C_VERSION`. `encode` writes the constant instead.

**Files:**
- Modify: `cross-compile/snmp-agent/src/pdu.rs` (struct, `parse`, `encode`, tests)
- Modify: `cross-compile/snmp-agent/src/server.rs` (`handle_datagram`, `get_sysname_bytes`, import)
- Modify: `cross-compile/snmp-agent/tests/walk.rs` (two constructions, import)

**Step 1: Struct and codec** (`pdu.rs`)

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnmpMessage {
    pub community: String,
    pub pdu: Pdu,
}
```

In `parse`, keep the version check but don't store it, and end with `Ok(Self { community, pdu })`.
In `encode`: `ber::write_tlv(TAG_INTEGER, &ber::encode_integer(SNMP_V2C_VERSION), &mut inner);`
Make the constant private: `const SNMP_V2C_VERSION: i32 = 1;`

**Step 2: Remove every `version:` initializer and the one assertion**

```bash
cd cross-compile
sed -i '/^\s*version: SNMP_V2C_VERSION,$/d' snmp-agent/src/pdu.rs snmp-agent/src/server.rs snmp-agent/tests/walk.rs
sed -i '/assert_eq!(msg.version, SNMP_V2C_VERSION);/d' snmp-agent/src/pdu.rs
sed -i 's/PduType, SNMP_V2C_VERSION, SnmpMessage/PduType, SnmpMessage/' snmp-agent/src/server.rs snmp-agent/tests/walk.rs
grep -n "version" snmp-agent/src/server.rs snmp-agent/tests/walk.rs
```

Expected from the final grep: no hits.

**Step 3: Run tests**

Run: `$CARGO test --target x86_64-unknown-linux-gnu -p snmp-agent 2>&1 | grep -E "test result|^error"`
Expected: unchanged counts. `test_reject_non_v2c_version` still passes: v1 is still rejected.

**Step 4: Commit**

```bash
rtk git commit -m "refactor(snmp-agent): drop always-v2c SnmpMessage.version" -- snmp-agent/src/pdu.rs snmp-agent/src/server.rs snmp-agent/tests/walk.rs
```

---

### Task 5: Collapse `BerError` + `PduError` into one `Malformed`

`handle_datagram` discards every codec error with `.ok()?`; only tests ever looked at which variant came back.

**Files:**
- Modify: `cross-compile/snmp-agent/src/ber.rs`
- Modify: `cross-compile/snmp-agent/src/pdu.rs`

**Step 1: Mechanical rename**

```bash
cd cross-compile
sed -i -E 's/BerError::(InvalidOid|Truncated|UnexpectedTag|Unsupported)/Malformed/g; s/\bBerError\b/Malformed/g' snmp-agent/src/ber.rs snmp-agent/src/pdu.rs
sed -i -E 's/PduError::UnsupportedVersion\([a-z0-9]+\)/Malformed/g; s/PduError::Malformed/Malformed/g; s/\bPduError\b/Malformed/g' snmp-agent/src/pdu.rs
```

**Step 2: Hand-fix the two type definitions**

In `ber.rs`: delete `use thiserror::Error;` and replace the (now mangled) error enum with:

```rust
/// Any BER the agent cannot decode or encode. The datagram is silently
/// dropped, so nothing ever branches on *why*.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Malformed;
```

In `pdu.rs`: delete `use thiserror::Error;` and the whole former `PduError` enum block (now `pub enum Malformed { … }`). Its import line becomes `use crate::ber::{self, Malformed, Oid, TAG_INTEGER, …};`.

**Step 3: Run tests**

Run: `$CARGO test --target x86_64-unknown-linux-gnu -p snmp-agent 2>&1 | grep -E "test result|^error"`
Expected: unchanged counts. If a test still names a removed variant, the sed missed it; change it to `Malformed`.

**Step 4: Commit**

```bash
rtk git commit -m "refactor(snmp-agent): one Malformed error for the BER/PDU codec" -- snmp-agent/src/ber.rs snmp-agent/src/pdu.rs
```

---

### Task 6: Delete the `MibSources` trait; pass `&Snapshot`

The trait had one real implementation (`Snapshot`) and one test double (`FixedSources`) that was `Snapshot` with renamed fields.

**Files:**
- Modify: `cross-compile/snmp-agent/src/mib/mod.rs`
- Modify: `cross-compile/snmp-agent/src/mib/system.rs`
- Modify: `cross-compile/snmp-agent/src/mib/interfaces.rs`
- Modify: `cross-compile/snmp-agent/src/server.rs` (test import + one assertion)

**Step 1: `interfaces.rs` — delete the wrappers, rename the row functions**

Delete the two bottom wrappers `pub fn get(oid, sources: &dyn MibSources)` and `pub fn get_next(oid, sources: &dyn MibSources)`, and the `use crate::mib::MibSources;` import. Then:

```bash
cd cross-compile
sed -i 's/get_next_with_rows/get_next/g; s/get_with_rows/get/g' snmp-agent/src/mib/interfaces.rs
```

Update the doc comment on the new `get`: `/// Exact GET against ifTable rows.`

**Step 2: `system.rs`**

```bash
sed -i 's/sources: &dyn MibSources/sources: \&Snapshot/g; s/use crate::mib::MibSources;/use crate::mib::Snapshot;/' snmp-agent/src/mib/system.rs
```

In `value_for`: `let cfg = sources.config();` → `let cfg = &sources.config;` and `sources.uptime_ticks()` → `sources.uptime_ticks`.

**Step 3: `mod.rs`**

Delete `pub trait MibSources { … }` and `impl MibSources for Snapshot { … }`. Then:

```bash
sed -i 's/sources: &dyn MibSources/sources: \&Snapshot/g' snmp-agent/src/mib/mod.rs
```

and make the resolvers pass the rows:

```rust
fn resolve_get(oid: &Oid, sources: &Snapshot) -> Option<(Oid, SnmpValue)> {
    system::get(oid, sources).or_else(|| interfaces::get(oid, &sources.ifaces))
}

fn resolve_get_next(oid: &Oid, sources: &Snapshot) -> Option<(Oid, SnmpValue)> {
    system::get_next(oid, sources).or_else(|| interfaces::get_next(oid, &sources.ifaces))
}
```

(add `use crate::ber::Oid;` at the top and drop the `crate::ber::` / `crate::pdu::` path prefixes inside these two functions).

In `mod tests`, delete `struct FixedSources` and its `impl MibSources`. Replace the helper and the one inline construction:

```rust
    fn sources() -> Snapshot {
        Snapshot {
            config: SnmpConfig {
                sys_contact: "ops@example".into(),
                sys_name: "cam-1".into(),
                sys_location: "lab".into(),
                ..Default::default()
            },
            uptime_ticks: 42,
            ifaces: interfaces::parse_proc_net_dev(include_str!(
                "../../tests/fixtures/proc_net_dev.txt"
            )),
        }
    }
```

```rust
        let empty_name = Snapshot {
            config: SnmpConfig {
                sys_name: String::new(),
                ..Default::default()
            },
            uptime_ticks: 1,
            ifaces: Vec::new(),
        };
```

Drop the now-unused `use crate::mib::interfaces::IfRow;` from `mod tests`; the top-level one stays because `Snapshot` needs it.

**Step 4: `server.rs` tests**

Delete `use crate::mib::MibSources;`; change `agent.snapshot().uptime_ticks()` → `agent.snapshot().uptime_ticks`.

**Step 5: Run tests**

Run: `$CARGO test --target x86_64-unknown-linux-gnu -p snmp-agent 2>&1 | grep -E "test result|^error"`
Expected: unchanged counts.

**Step 6: Commit**

```bash
rtk git commit -m "refactor(snmp-agent): drop single-impl MibSources trait, pass &Snapshot" -- snmp-agent/src/mib snmp-agent/src/server.rs
```

---

### Task 7: Generate the system scalars; `COLUMNS` as a const

**Files:**
- Modify: `cross-compile/snmp-agent/src/mib/system.rs`
- Modify: `cross-compile/snmp-agent/src/mib/interfaces.rs`

**Step 1: `system.rs`** — delete `fn system_scalars()` and replace `get_next` with:

```rust
/// Lexicographic next system scalar after `oid` (sysDescr.0 … sysServices.0).
pub fn get_next(oid: &Oid, sources: &Snapshot) -> Option<(Oid, SnmpValue)> {
    let next = (1..=7)
        .map(|arc| Oid(vec![1, 3, 6, 1, 2, 1, 1, arc, 0]))
        .find(|candidate| oid < candidate)?;
    let value = value_for(&next, sources)?;
    Some((next, value))
}
```

**Step 2: `interfaces.rs`** — replace `fn column_ids() -> [u32; 10] { [...] }` with

```rust
/// Columnar OIDs: ifIndex(1), ifDescr(2), ifType(3), ifMtu(4), ifSpeed(5),
/// ifPhysAddress(6), ifAdminStatus(7), ifOperStatus(8), ifInOctets(10), ifOutOctets(16).
const COLUMNS: [u32; 10] = [1, 2, 3, 4, 5, 6, 7, 8, 10, 16];
```

and change `column_ids()` → `COLUMNS` at both uses (`for col in COLUMNS`, `COLUMNS.contains(&col)`).

**Step 3: Run tests** — `test_system_walk_order` and the walk integration test guard ordering.

Run: `$CARGO test --target x86_64-unknown-linux-gnu -p snmp-agent 2>&1 | grep -E "test result|^error"`
Expected: unchanged counts.

**Step 4: Commit**

```bash
rtk git commit -m "refactor(snmp-agent): generate system scalar OIDs, const ifTable columns" -- snmp-agent/src/mib
```

---

### Task 8: Refuse SET once, before any snapshot exists

`bare_snapshot()` existed only so a SET could reach `handle_varbinds` without reading `/proc`. Answer SET in `handle_datagram` instead.

**Files:**
- Modify: `cross-compile/snmp-agent/src/server.rs` (`handle_datagram`, delete `bare_snapshot`)
- Modify: `cross-compile/snmp-agent/src/mib/mod.rs` (`handle_varbinds`, delete `test_set_returns_not_writable`)

**Step 1: `server.rs`** — delete `fn bare_snapshot`, and replace the `let snapshot = …` and `let (error_status, …) = …` blocks with:

```rust
    let (error_status, error_index, variable_bindings) = match msg.pdu.pdu_type {
        // Refused before any varbind is resolved, so /proc and sysfs are never read for it.
        PduType::SetRequest => (mib::ERR_NOT_WRITABLE, 1, msg.pdu.variable_bindings),
        PduType::GetBulkRequest => mib::handle_getbulk(
            msg.pdu.error_status,
            msg.pdu.error_index,
            &msg.pdu.variable_bindings,
            &agent.snapshot(),
        ),
        t => mib::handle_varbinds(t, &msg.pdu.variable_bindings, &agent.snapshot()),
    };
```

**Step 2: `mib/mod.rs`** — remove the SET early-return from `handle_varbinds`, change its doc to `/// Resolve GET / GETNEXT for the fixed OID map.`, and delete `test_set_returns_not_writable`. Coverage stays: `server::tests::test_set_is_refused_without_reading_the_device` asserts `notWritable`/index 1 with unreadable roots, and the walk test sends a SET.

**Step 3: Run tests**

Run: `$CARGO test --target x86_64-unknown-linux-gnu -p snmp-agent 2>&1 | grep -E "test result|^error"`
Expected: lib count drops by 1 (58), others unchanged.

**Step 4: Commit**

```bash
rtk git commit -m "refactor(snmp-agent): refuse SET before building a snapshot" -- snmp-agent/src/server.rs snmp-agent/src/mib/mod.rs
```

---

### Task 9: Drop the process-uptime fallback

`/proc/uptime` always exists on Linux. The fallback reported *process* uptime, which the doc comment itself says a monitoring system misreads as a device reboot.

**Files:**
- Modify: `cross-compile/snmp-agent/src/server.rs` (`Agent` struct, `with_roots`, `uptime_ticks`, `use std::time::Instant`)

**Step 1: Edit**

Delete the `started: Instant` field, its initializer, and `use std::time::Instant;`. Then:

```rust
    fn uptime_ticks(&self) -> u32 {
        proc_uptime_ticks(&self.proc_root.join("uptime")).unwrap_or(0)
    }
```

No new test: the old branch can't be told apart from `0` in a test (elapsed is ~0 ticks), and `test_uptime_comes_from_proc_uptime` covers the real path.

**Step 2: Run tests, then commit**

Run: `$CARGO test --target x86_64-unknown-linux-gnu -p snmp-agent 2>&1 | grep -E "test result|^error"`

```bash
rtk git commit -m "refactor(snmp-agent): sysUpTime comes only from /proc/uptime" -- snmp-agent/src/server.rs
```

---

### Task 10: Delete `--config` parsing and the pidfile helpers

The shipped `anyka.toml` passes `--config /mnt/anyka_hack/snmp.toml`, which is already `DEFAULT_CONFIG_PATH`. Tests call `run()` with a path directly. Cameras keep their own `anyka.toml` (bundles never carry it), so the stale `args` there becomes harmless: nothing reads it.

**Files:**
- Modify: `cross-compile/snmp-agent/src/server.rs` (delete `parse_args`, `write_pidfile`, `remove_pidfile` and their two tests; inline into `run`)
- Modify: `cross-compile/snmp-agent/src/main.rs`
- Modify: `SD_card_contents/anyka_hack/anyka.toml:122` (drop the `args = [...]` line under `[services.snmp]`; `args` defaults to empty)

**Step 1: `server.rs`** — delete `parse_args`, `test_parse_args_config_flag`, `write_pidfile`, `remove_pidfile` and `test_write_and_remove_pidfile`, and drop `DEFAULT_CONFIG_PATH` from the `use crate::config::…` line. In `run`:

```rust
    std::fs::write(&pidfile, format!("{}\n", std::process::id()))?;
    struct PidGuard(PathBuf);
    impl Drop for PidGuard {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }
    let _pid_guard = PidGuard(pidfile);
```

**Step 2: `main.rs`**

```rust
use snmp_agent::config::DEFAULT_CONFIG_PATH;
use snmp_agent::server::{self, DEFAULT_PIDFILE};
…
    let config_path = PathBuf::from(DEFAULT_CONFIG_PATH);
```

**Step 3: Run tests** — the `run()` integration tests wait on the pidfile, so writing it is still covered.

Run: `$CARGO test --target x86_64-unknown-linux-gnu -p snmp-agent 2>&1 | grep -E "test result|^error"`
Expected: lib count drops by 2 (56).

**Step 4: Validate the TOML edit** (there is no `--check-config`)

Run: `/usr/bin/python3 -c "import tomllib;print(tomllib.load(open('../SD_card_contents/anyka_hack/anyka.toml','rb'))['services']['snmp'])"`
If the `python3 -c` hook blocks this, put the same two lines in `/tmp/check_toml.py` and run `/usr/bin/python3 /tmp/check_toml.py`.
Expected: a dict with `enabled`, `exec`, `log` and no `args`.

**Step 5: Commit**

```bash
rtk git commit -m "refactor(snmp-agent): drop --config flag and one-line pidfile helpers" -- snmp-agent/src/server.rs snmp-agent/src/main.rs ../SD_card_contents/anyka_hack/anyka.toml
```

---

### Task 11: Reuse anyka-init's `LocalTimer`

`main.rs` holds the third copy of `LocalTimer` (onvif-rust and anyka-init have the others). `anyka_init::logging::LocalTimer` is `pub`, and the `anyka_init` library is tokio-free. Once the copy is gone, `libc` has no user left in snmp-agent.

**Files:**
- Modify: `cross-compile/snmp-agent/Cargo.toml`
- Modify: `cross-compile/snmp-agent/src/main.rs`

**Step 1: Cargo.toml** — add `anyka-init = { path = "../anyka-init" }`. Then confirm libc is unused outside `LocalTimer`:

Run: `grep -rn "libc" snmp-agent/src`
Expected: hits only inside the `LocalTimer` impl in `main.rs`. If so, remove `libc = "0.2"` from `[dependencies]`.

**Step 2: main.rs** — delete `pub struct LocalTimer`, its `impl FormatTime`, and the whole `#[cfg(test)] mod tests` (anyka-init's `logging.rs` has the same TZ test). Keep the `tzset()` block. Use:

```rust
        .with_timer(anyka_init::logging::LocalTimer)
```

**Step 3: Run tests**

Run: `$CARGO test --target x86_64-unknown-linux-gnu -p snmp-agent 2>&1 | grep -E "test result|^error"`
Expected: the bin test target now reports `0 passed`; lib unchanged.

**Step 4: Commit**

```bash
rtk git commit -m "refactor(snmp-agent): use anyka-init's LocalTimer instead of a third copy" -- snmp-agent/Cargo.toml snmp-agent/src/main.rs Cargo.lock
```

---

### Task 12: Agent loses `enabled`; anyka-init is the only on/off switch

**Files:**
- Modify: `cross-compile/snmp-agent/src/config.rs`
- Modify: `cross-compile/snmp-agent/src/server.rs` (`run`, `apply_reload`, tests)
- Modify: `cross-compile/snmp-agent/tests/walk.rs` (TOML strings)

**Step 1: Write the failing test** (`config.rs` `mod tests`)

```rust
    #[test]
    fn test_legacy_enabled_false_no_longer_excuses_empty_community() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("snmp.toml");
        // Pre-cleanup files still carry `enabled`; it is ignored, not a parse error.
        std::fs::write(&path, "enabled = false\ncommunity = \"\"\n").unwrap();
        assert!(matches!(
            SnmpConfig::load(&path),
            Err(ConfigError::EmptyCommunity)
        ));
    }
```

**Step 2: Run to verify it fails**

Run: `$CARGO test --target x86_64-unknown-linux-gnu -p snmp-agent --lib test_legacy_enabled_false`
Expected: compile error, no variant `EmptyCommunity`.

**Step 3: `config.rs`**

- Delete `default_enabled`, the `enabled` field and its `Default` initializer.
- Struct doc: `/// SNMPv2c agent settings. Whether the agent runs at all is [services.snmp] enabled in anyka.toml, owned by anyka-init.`
- Replace `EmptyCommunityWhenEnabled` with `#[error("community must not be empty")] EmptyCommunity`.
- `load` ends with `config.validate()?; Ok(config)`, using the new:

```rust
    /// The rules [`SnmpConfig::load`] enforces; onvif-rust runs them before writing.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.port == 0 {
            return Err(ConfigError::InvalidPort);
        }
        if self.community.is_empty() {
            return Err(ConfigError::EmptyCommunity);
        }
        Ok(())
    }
```

- Tests: drop `assert!(c.enabled);` and `assert!(!c.enabled);`. Leave `enabled = false` in `test_load_parses_toml`'s TOML, since it proves the legacy key still parses. Delete `test_load_rejects_enabled_without_community`, which the new test supersedes.

**Step 4: `server.rs` — `run` and `apply_reload`**

`apply_reload` becomes:

```rust
/// Apply a freshly loaded config. A new port is bound before `agent.config` is
/// replaced, so a failed rebind keeps serving on the old one.
async fn apply_reload(agent: &mut Agent, socket: &mut Option<UdpSocket>, new_cfg: SnmpConfig) {
    if socket.is_some() && agent.config.port == new_cfg.port {
        agent.config = new_cfg;
        tracing::info!("snmp-agent config reloaded (same bind)");
        return;
    }
    match bind_socket(new_cfg.port).await {
        Ok(s) => {
            tracing::info!(port = new_cfg.port, "snmp-agent rebound");
            *socket = Some(s);
            agent.config = new_cfg;
        }
        Err(e) => {
            tracing::error!(
                error = %e,
                port = new_cfg.port,
                "rebind failed; keeping previous socket when bound"
            );
            // No live socket: commit the requested config so BIND_RETRY
            // keeps trying the new port instead of stale last-good values.
            if socket.is_none() {
                agent.config = new_cfg;
            }
        }
    }
}
```

In `run`, the initial bind loses its `if agent.config.enabled { … } else { "disabled" }` wrapper:

```rust
    let mut socket = match bind_socket(agent.config.port).await {
        Ok(s) => {
            tracing::info!(port = agent.config.port, "snmp-agent listening");
            Some(s)
        }
        Err(e) => {
            tracing::error!(error = %e, port = agent.config.port, "bind failed; will retry");
            None
        }
    };
```

In the loop, delete `let enabled = agent.config.enabled;` and change the retry guard to `if socket.is_none()`.

**Step 5: `server.rs` tests**

- `test_run_serves_udp_get_reload_and_disable`: delete the `// Disable on reload.` and `// Re-enable (rebind after socket cleared).` blocks, up to just before `handle.abort();`. Rename it `test_run_serves_udp_get_and_reload`.
- `test_run_starts_disabled_and_bind_failure_is_non_fatal`: delete the first (disabled) half, up to and including the first `handle.abort(); let _ = handle.await;`. Rename it `test_bind_failure_is_non_fatal`.
- Strip the legacy key from the remaining test TOML:

```bash
cd cross-compile
sed -i 's/enabled = true\\n//' snmp-agent/src/server.rs snmp-agent/tests/walk.rs
grep -n "enabled" snmp-agent/src/server.rs snmp-agent/tests/walk.rs
```

Expected from the grep: no hits.

**Step 6: Run tests**

Run: `$CARGO test --target x86_64-unknown-linux-gnu -p snmp-agent 2>&1 | grep -E "test result|^error"`
Expected: all green. Integration tests: 3 passed.

**Step 7: Commit**

```bash
rtk git commit -m "refactor(snmp-agent)!: drop snmp.toml enabled; anyka-init owns on/off" -- snmp-agent/src/config.rs snmp-agent/src/server.rs snmp-agent/tests/walk.rs
```

---

### Task 13: onvif-rust parses `snmp.toml` with the agent's own struct

Removes `SnmpSettings` (a field-for-field copy plus the drift test that guarded it), `SnmpView` (another copy), `validate_patch` (the shared `validate()` covers it), and `config_path()`/`set_config_path_for_test`, which only their own test used. onvif-rust already depends on every snmp-agent dependency except `anyka-init`, which Task 11 added (bin-only); that crate is now compiled transitively for onvif-rust too — harmless, no cycle, but not free.

**Files:**
- Modify: `cross-compile/onvif-rust/Cargo.toml` (add `snmp-agent = { path = "../snmp-agent" }` to `[dependencies]`)
- Modify: `cross-compile/onvif-rust/src/config/snmp.rs`
- Modify: `cross-compile/onvif-rust/src/diagnostics/snmp.rs`

**Step 1: Write the failing tests** — replace the `mod tests` of `config/snmp.rs` with the three below. Keep `test_sighup_agent_refuses_process_group_pids` verbatim. Rewrite `test_config_path_override_and_sighup_agent` as `test_sighup_agent_tolerates_missing_and_stale_pidfiles` by deleting its first five lines (the `set_config_path_for_test`/`config_path` part).

```rust
    #[test]
    fn test_update_at_round_trips_through_the_agent_loader() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("snmp.toml");
        update_at(&path, |s| {
            s.port = 1161;
            s.community = "monitor".into();
            s.sys_contact = "ops".into();
        })
        .unwrap();
        let got = SnmpSettings::load(&path).unwrap();
        assert_eq!(got.port, 1161);
        assert_eq!(got.community, "monitor");
        assert_eq!(got.sys_contact, "ops");
    }

    #[test]
    fn test_update_at_rejects_invalid_edits_and_keeps_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("snmp.toml");
        update_at(&path, |_| {}).unwrap();
        let before = std::fs::read_to_string(&path).unwrap();
        assert!(matches!(update_at(&path, |s| s.port = 0), Err(ConfigError::InvalidPort)));
        assert!(matches!(
            update_at(&path, |s| s.community.clear()),
            Err(ConfigError::EmptyCommunity)
        ));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), before);
    }

    #[test]
    fn test_update_at_drops_the_legacy_enabled_key() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("snmp.toml");
        std::fs::write(&path, "enabled = false\ncommunity = \"public\"\n").unwrap();
        update_at(&path, |_| {}).unwrap();
        assert!(!std::fs::read_to_string(&path).unwrap().contains("enabled"));
    }
```

**Step 2: Run to verify they fail**

Run: `$CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust --lib config::snmp`
Expected: compile errors (`update_at` not found, `ConfigError` not in scope).

**Step 3: Rewrite `config/snmp.rs` above `sighup_agent`** (keep `sighup_agent` verbatim):

```rust
//! Machine-owned SNMP agent settings (`snmp.toml`).
//!
//! The struct and its validation are the agent's own (`snmp_agent::config`),
//! so the two sides cannot drift. On/off is `[services.snmp]` in `anyka.toml`.
//! SNMP is exposed via REST `/api/snmp` and this file — not ONVIF NetworkProtocolType.

use std::path::Path;
use std::sync::{Mutex, PoisonError};

pub use snmp_agent::config::{ConfigError, SnmpConfig as SnmpSettings};
pub use snmp_agent::server::DEFAULT_PIDFILE;

use super::file_ops::atomic_write;

/// Serializes all snmp.toml read-modify-write updates.
static UPDATE_LOCK: Mutex<()> = Mutex::new(());

/// Load, edit, validate and atomically rewrite `snmp.toml`.
pub fn update_at(path: &Path, edit: impl FnOnce(&mut SnmpSettings)) -> Result<(), ConfigError> {
    let _guard = UPDATE_LOCK.lock().unwrap_or_else(PoisonError::into_inner);
    let mut cfg = SnmpSettings::load(path)?;
    edit(&mut cfg);
    cfg.validate()?;
    let content = toml::to_string_pretty(&cfg).map_err(std::io::Error::other)?;
    atomic_write(path, content.as_bytes(), None)?;
    Ok(())
}
```

This deletes `DEFAULT_CONFIG_PATH`, `path_override`, `update_lock`, `config_path`, `set_config_path_for_test`, the struct, its defaults, `ERR_EMPTY_COMMUNITY_WHEN_ENABLED`, `read` and `write`. Also delete the old `test_default_round_trip`, `test_missing_file_defaults`, `test_update_at_enabled_without_community_returns_error`, `test_read_write_reject_port_zero_and_update_at` and `test_keys_match_snmp_agent_config`.

**Step 4: Rewrite `diagnostics/snmp.rs`**

- Import: `use crate::config::snmp::{self, ConfigError, SnmpSettings};`
- Delete `SnmpView` and its `From` impl, `validate_patch`, and `enabled` from `SnmpPatch` and `apply_patch`.
- Handlers:

```rust
/// GET /api/snmp
pub async fn handle_get_snmp(
    Extension(state): Extension<Arc<SnmpApiState>>,
) -> Result<Json<SnmpSettings>, (StatusCode, String)> {
    SnmpSettings::load(&state.config_path)
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// PUT /api/snmp
pub async fn handle_put_snmp(
    Extension(state): Extension<Arc<SnmpApiState>>,
    Json(patch): Json<SnmpPatch>,
) -> Result<StatusCode, (StatusCode, String)> {
    snmp::update_at(&state.config_path, |s| apply_patch(s, patch)).map_err(|e| {
        let status = match e {
            ConfigError::InvalidPort | ConfigError::EmptyCommunity => StatusCode::BAD_REQUEST,
            ConfigError::Io(_) | ConfigError::Parse(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, e.to_string())
    })?;

    snmp::sighup_agent(Path::new(&state.pidfile)).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("snmp settings saved but agent reload failed: {e}"),
        )
    })?;
    Ok(StatusCode::NO_CONTENT)
}
```

- Tests:
  - Delete `test_validate_rejects_empty_community` and `test_validate_rejects_port_zero`.
  - `test_apply_patch_merges_fields`: drop `enabled: Some(false),` and `assert!(!s.enabled);`.
  - `test_get_and_put_snmp_handlers`: drop `assert!(get.enabled);`, `enabled: Some(false),`, `assert!(!stored.enabled);` and the two `SnmpView` lines. Use `SnmpSettings::load(&config_path)` instead of `read`. Change the final port-0 check to `assert_eq!(bad.unwrap_err().0, StatusCode::BAD_REQUEST);`.
  - Replace `test_put_snmp_enabled_without_community_returns_bad_request` with:

```rust
    #[tokio::test]
    async fn test_put_snmp_empty_community_returns_bad_request() {
        let dir = tempfile::tempdir().unwrap();
        let state = Arc::new(SnmpApiState {
            config_path: dir.path().join("snmp.toml"),
            pidfile: dir.path().join("missing.pid"),
        });
        let err = handle_put_snmp(
            Extension(state),
            Json(SnmpPatch {
                community: Some(String::new()),
                ..Default::default()
            }),
        )
        .await
        .expect_err("empty community");
        assert_eq!(err.0, StatusCode::BAD_REQUEST);
    }
```

**Step 5: Run tests**

Run: `$CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust --lib snmp 2>&1 | grep -E "test result|^error|FAILED"`
Expected: all pass (the filter `snmp` matches both `config::snmp` and `diagnostics::snmp`).

Then: `grep -rn "SnmpView\|validate_patch\|ERR_EMPTY_COMMUNITY\|SnmpSettings::read\|\.write(&" onvif-rust/src | grep -i snmp`
Expected: no hits.

**Step 6: Commit**

```bash
rtk git commit -m "refactor(onvif-rust): parse snmp.toml with snmp-agent's SnmpConfig" -- onvif-rust/Cargo.toml onvif-rust/src/config/snmp.rs onvif-rust/src/diagnostics/snmp.rs Cargo.lock
```

---

### Task 14: WebUI — the Network page loses its Enable switch

Diagnostics → Processes already enables and disables `snmp` through anyka-init. The Network page keeps port and community.

**Files:**
- Modify: `cross-compile/www/src/services/networkService.ts:44-51`
- Modify: `cross-compile/www/src/pages/settings/NetworkPage.tsx`
- Test: `cross-compile/www/src/pages/settings/NetworkPage.test.tsx`
- Test: `cross-compile/www/src/services/networkService.test.ts:360`

**Step 1: Write the failing test** (`NetworkPage.test.tsx`, next to the other SNMP tests)

```ts
  it('test_snmp_card_points_to_processes_instead_of_a_switch', async () => {
    await renderNetworkPage();

    expect(screen.queryByTestId('network-snmp-enabled-switch')).not.toBeInTheDocument();
    expect(screen.getByText(/Diagnostics → Processes/)).toBeInTheDocument();
  });
```

**Step 2: Run to verify it fails**

Run: `cd cross-compile/www && npx vitest run src/pages/settings/NetworkPage.test.tsx -t points_to_processes`
Expected: FAIL; the switch is still in the document.

**Step 3: Implement**

`networkService.ts`: delete `enabled: boolean;` from `SnmpConfig`.

`NetworkPage.tsx`:
1. Schema: delete `snmpEnabled: z.boolean(),`. Change the community line to
   `snmpCommunity: z.string().max(64).refine((s) => s.trim() !== '', 'Community must not be empty'),`
   and delete the `if (data.snmpEnabled && !data.snmpCommunity.trim()) { … }` block from `superRefine`.
2. Delete each remaining `snmpEnabled` line: in `defaultValues`, the `form.reset` effect, the `form.setValue` effect, `snmpChanged` (`values.snmpEnabled !== snmp.enabled ||`), the `putSnmpConfig` body (`enabled: values.snmpEnabled,`) and `handleReset`.
3. JSX: delete the whole `<FormField … name="snmpEnabled" … />`. Change the card description to
   `Read-only SNMPv2c agent; changes apply without reboot. Turn it on or off under Diagnostics → Processes.`
   Move the security note under the community input, just before `<FormMessage data-testid="network-snmp-community-error" />`:
   ```tsx
                            <FormDescription className="text-[#636366]">
                              Default community &quot;public&quot; is insecure on untrusted networks
                            </FormDescription>
   ```
4. Check: `grep -n "snmpEnabled\|snmp.enabled\|snmp?.enabled" src/pages/settings/NetworkPage.tsx` returns no hits.

Tests: delete `enabled: true,` at `NetworkPage.test.tsx:91` and `:407`, and `networkService.test.ts:360` (the SNMP mocks only; lines 101/167/179 are network interfaces, so leave them). Change `:431` to `expect.objectContaining({ port: 2161, community: 'public' }),`.

**Step 4: Run tests**

Run: `npx vitest run src/pages/settings/NetworkPage.test.tsx src/services/networkService.test.ts`
Expected: all pass, including the new test and `test_save_empty_community_blocks_submit_and_skips_putSnmpConfig`.

**Step 5: Static checks** (raw prettier; read the exit code)

Run: `npm run type-check && npm run lint && npx prettier --check src; echo "exit=$?"`
Expected: `exit=0`.

**Step 6: Commit**

```bash
rtk git commit -m "feat(webui)!: SNMP on/off lives in Diagnostics → Processes only" -- www/src/services/networkService.ts www/src/pages/settings/NetworkPage.tsx www/src/pages/settings/NetworkPage.test.tsx www/src/services/networkService.test.ts
```

---

### Task 15: Payload defaults and README

**Files:**
- Modify: `SD_card_contents/anyka_hack/snmp.toml` (delete `enabled = true`)
- Modify: `SD_card_contents/anyka_hack/snmp/README.md`

**Step 1:** Remove the `enabled = true` line from `snmp.toml`. Append to the README:

```markdown
On/off is `[services.snmp] enabled` in `/mnt/anyka_hack/anyka.toml` (WebUI:
Diagnostics → Processes). `snmp.toml` holds only port, community and the
sys* strings; a leftover `enabled` key is ignored.
```

**Step 2: Commit**

```bash
rtk git commit -m "docs(snmp): on/off lives in anyka.toml, not snmp.toml" -- ../SD_card_contents/anyka_hack/snmp.toml ../SD_card_contents/anyka_hack/snmp/README.md
```

---

### Task 16: Verification

**Step 1: Host tests, workspace-wide** (onvif-rust and snmp-agent now depend on anyka-init)

Run: `$CARGO test --target x86_64-unknown-linux-gnu --workspace 2>&1 | grep -E "test result|FAILED|^error"`
Expected: every `test result: ok`.

**Step 2: Clippy**

Run: `PATH=$PWD/../toolchain/arm-anykav200-crosstool-ng/bin:$PATH $CARGO clippy --target x86_64-unknown-linux-gnu -p snmp-agent -p onvif-rust --all-targets -- -D warnings`
Expected: no warnings.

**Step 3: fmt**

Run: `$CARGO fmt --all -- --check; echo "exit=$?"`
Expected: `exit=0`.

**Step 4: ARM builds** (PR CI never cross-builds ARM, so this is the only check)

```bash
cd onvif-rust
../../toolchain/arm-anykav200-crosstool-ng/bin/cargo build --release -p snmp-agent
ls -l ../target/armv5te-unknown-linux-uclibceabi/release/snmp-agent
../../toolchain/arm-anykav200-crosstool-ng/bin/cargo build --release -p onvif-rust
```

Expected: both build. Compare the snmp-agent size with Task 0. A notable *increase* means the `anyka_init` dependency pulled code in, so investigate before shipping.

**Step 5: Size of the cut**

Run: `rtk git diff --stat main -- cross-compile SD_card_contents`
Expected: net deletions in the region of −300 lines.

**Step 6: Fleet check — do this before any camera gets the bundle, not as part of the PR**

After upgrade, a camera that had SNMP turned off *only* via `snmp.toml` (`enabled = false`) starts answering. On each camera (.198, .121, .146, 30.127 via the jumphost, .148), run:

```sh
grep enabled /mnt/anyka_hack/snmp.toml
```

Wherever it prints `enabled = false`, disable `snmp` in Diagnostics → Processes **before** deploying. Default community `public` must not come up on a camera where someone deliberately switched SNMP off.

**Step 7: Request review**

Use superpowers:requesting-code-review, then superpowers:finishing-a-development-branch.

---

## Out of scope (noted, not done)

- onvif-rust's own `LocalTimer` copy in `src/logging/mod.rs` could also use `anyka_init::logging::LocalTimer`, but that is outside the snmp-agent audit.
- `SnmpPatch` still mirrors the field list with `Option`s; PUT is a partial update, and a generic merge would cost more than it saves.
