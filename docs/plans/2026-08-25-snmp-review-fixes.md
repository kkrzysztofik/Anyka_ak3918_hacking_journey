# SNMP Review Fixes Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Close all nine defects and four plan gaps found reviewing `feat/snmp`, so the branch has green gates, RFC 3416 varbind semantics, and an `ifTable` built from real kernel data.

**Architecture:** A per-request `Snapshot` becomes the only thing the MIB layer can see, giving `ifTable`/`sysUpTime` a real data source and a seam to test through. Reload moves from a self-installed signal handler to an `mpsc` channel, which removes both the clippy failure and the test that kills its own test binary.

**Tech Stack:** Rust workspace member `snmp-agent` (tokio UDP), `onvif-rust` (axum + ONVIF SOAP), React 19 WebUI (Vitest + React Testing Library).

**Design:** `docs/plans/2026-08-25-snmp-review-fixes-design.md`

**Worktree:** `/home/kmk/dev/anyka-dev/.worktrees/snmp` on `feat/snmp`

---

## Toolchain preamble

Every task assumes this. Run it once per shell.

```bash
cd /home/kmk/dev/anyka-dev/.worktrees/snmp
source /home/kmk/dev/anyka-dev/setenv.sh
```

Three environment facts that will waste an hour if you miss them:

1. **Host tests** always need `--target x86_64-unknown-linux-gnu`. Without it cargo builds for ARM and the tests will not run.
2. **Clippy** needs the vendored toolchain's `bin` first on `PATH` or it dies with `E0514` (proc-macro built by a different compiler):
   ```bash
   export PATH="/home/kmk/dev/anyka-dev/toolchain/arm-anykav200-crosstool-ng/bin:$PATH"
   ```
3. **ARM release builds** must run from `cross-compile/onvif-rust/`, not the workspace root. That directory holds the generated `.cargo/config.toml` supplying the ARMv5TE linker; from the root, cargo silently links with the host toolchain.

The worktree has uncommitted test additions from a prior session. Leave them alone unless a task says otherwise; `git add` only the files each task names.

---

### Task 1: Stop `sighup_agent` signalling process groups

The pidfile is parsed straight into `libc::kill`. `kill(0, …)` signals our whole process group; `kill(-1, …)` signals every process the caller can signal, and `onvif-rust` runs as root on the camera. A truncated pidfile after a power cut is a known hazard on this hardware.

**Files:**
- Modify: `cross-compile/onvif-rust/src/config/snmp.rs`

**Step 1: Write the failing test**

Add to the `tests` module in `cross-compile/onvif-rust/src/config/snmp.rs`:

```rust
    #[test]
    fn test_sighup_agent_refuses_process_group_pids() {
        let dir = tempfile::tempdir().unwrap();
        for (name, body) in [("zero.pid", "0\n"), ("all.pid", "-1\n"), ("neg.pid", "-4242\n")] {
            let path = dir.path().join(name);
            std::fs::write(&path, body).unwrap();
            // Must be a no-op, never a kill(2) broadcast.
            assert!(sighup_agent(&path).is_ok(), "{name} must be ignored");
        }
    }
```

**Step 2: Run test to verify it fails**

```bash
cd /home/kmk/dev/anyka-dev/.worktrees/snmp/cross-compile
$CARGO test -p onvif-rust --target x86_64-unknown-linux-gnu --lib \
  config::snmp::tests::test_sighup_agent_refuses_process_group_pids
```

Expected: **FAIL**. `kill(0, SIGHUP)` succeeds and returns `Ok`, but `kill(-1, …)` / `kill(-4242, …)` return `EPERM` or `ESRCH` — the test may pass or fail depending on the host, which is exactly the non-determinism we are removing. Treat any run where the test binary itself dies as a failure.

**Step 3: Add the guard**

In `sighup_agent`, immediately after the `parse` and before the `unsafe` block:

```rust
    // kill(2) overloads its first argument: 0 means "my whole process group" and
    // negative values mean a process group or, for -1, every process we may
    // signal. onvif-rust is root on the camera, so a truncated pidfile must be
    // inert rather than a broadcast.
    if pid <= 1 {
        return Ok(());
    }
```

**Step 4: Run test to verify it passes**

```bash
$CARGO test -p onvif-rust --target x86_64-unknown-linux-gnu --lib config::snmp
```

Expected: PASS, all tests in the module.

**Step 5: Commit**

```bash
git add cross-compile/onvif-rust/src/config/snmp.rs
git commit -m "fix(snmp): refuse to SIGHUP pid 0 or a process group"
```

---

### Task 2: Delete the test that kills its own test binary

`test_config_path_override_and_sighup_agent` writes its own PID to a pidfile and calls `sighup_agent` on it. Default SIGHUP disposition is *terminate*; it survives only when another `#[tokio::test]` has already installed tokio's process-wide handler. On a cold run it loses that race and takes down the whole `onvif-rust` lib test binary:

```
process didn't exit successfully: .../onvif_rust-9ee43786c9f38ee9 (signal: 1, SIGHUP: hangup)
```

**Files:**
- Modify: `cross-compile/onvif-rust/src/config/snmp.rs`

**Step 1: Remove the self-signal**

In `test_config_path_override_and_sighup_agent`, delete these four lines and nothing else:

```rust
        let self_pid = dir.path().join("self.pid");
        std::fs::write(&self_pid, format!("{}\n", std::process::id())).unwrap();
        assert!(sighup_agent(&self_pid).is_ok());
```

The remaining cases — missing pidfile, unparseable pidfile, stale PID returning `ESRCH` — plus Task 1's guard cover every branch of `sighup_agent` without signalling anything real.

**Step 2: Verify the suite survives ten cold runs**

```bash
cd /home/kmk/dev/anyka-dev/.worktrees/snmp/cross-compile
for i in $(seq 1 10); do
  $CARGO test -p onvif-rust --target x86_64-unknown-linux-gnu > /tmp/onvif_$i.log 2>&1
  echo "run $i EXIT=$? sighup_kills=$(grep -c 'SIGHUP: hangup' /tmp/onvif_$i.log)"
done
```

Expected: ten lines of `EXIT=0 sighup_kills=0`.

Do **not** pipe `cargo test` into `tail`/`grep` when checking pass/fail — without `pipefail` the pipeline reports the exit status of the last command and hides a signal-killed run behind `0`. That is how this bug reached review.

**Step 3: Commit**

```bash
git add cross-compile/onvif-rust/src/config/snmp.rs
git commit -m "test(snmp): stop SIGHUPing the onvif-rust test binary"
```

---

### Task 3: Move reload from signal handler to channel

`run()` installs its own SIGHUP handler, so the only way to test reload is to signal the whole process. That forced the `run_test_lock()` serialization, which holds a `std::sync::MutexGuard` across `.await` and fails `clippy --all-targets`. Fix the cause: `run()` receives reload events, `main` owns the signal.

**Files:**
- Modify: `cross-compile/snmp-agent/src/server.rs`
- Modify: `cross-compile/snmp-agent/src/main.rs`

**Step 1: Change the `run` signature**

```rust
pub async fn run(
    config_path: PathBuf,
    pidfile: PathBuf,
    mut reload: tokio::sync::mpsc::Receiver<()>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
```

Delete the `use tokio::signal::unix::{SignalKind, signal};` import and the `let mut sighup = signal(SignalKind::hangup())?;` line. In the `select!`, replace the `_ = sighup.recv() => {` arm header with:

```rust
            _ = reload.recv() => {
```

The body is unchanged.

**Step 2: Move the signal into `main.rs`**

```rust
use snmp_agent::server::{self, DEFAULT_PIDFILE};
use std::path::PathBuf;
use tokio::signal::unix::{SignalKind, signal};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    tracing_subscriber::fmt::init();
    let config_path = server::parse_args(std::env::args());
    tracing::info!(?config_path, "snmp-agent starting");

    let (tx, rx) = tokio::sync::mpsc::channel(1);
    match signal(SignalKind::hangup()) {
        Ok(mut sighup) => {
            tokio::spawn(async move {
                while sighup.recv().await.is_some() {
                    // Depth 1: a reload already queued subsumes this one.
                    let _ = tx.try_send(());
                }
            });
        }
        Err(e) => tracing::error!(error = %e, "SIGHUP unavailable; config reload disabled"),
    }

    if let Err(e) = server::run(config_path, PathBuf::from(DEFAULT_PIDFILE), rx).await {
        tracing::error!(error = %e, "snmp-agent exited");
        std::process::exit(1);
    }
}
```

`flavor = "current_thread"` lands here too: one UDP socket does not need a worker thread per core on a 36 MB device.

**Step 3: Rewrite the two `run()` tests to use the channel**

In `server.rs` tests, delete `run_test_lock()` and `sighup_self_from_pidfile()` entirely. In `test_run_serves_udp_get_reload_and_disable` and `test_run_starts_disabled_and_bind_failure_is_non_fatal`, delete every `let _guard = run_test_lock();` line, build a channel before spawning, and replace each `sighup_self_from_pidfile(&pidfile);` with `tx.send(()).await.unwrap();`:

```rust
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        let handle = tokio::spawn(async move {
            let _ = run(run_cfg, run_pid, rx).await;
        });
```

The `wait_for_file` + `sleep` pacing stays as-is.

**Step 4: Run tests and clippy**

```bash
cd /home/kmk/dev/anyka-dev/.worktrees/snmp/cross-compile
$CARGO test -p snmp-agent --target x86_64-unknown-linux-gnu
export PATH="/home/kmk/dev/anyka-dev/toolchain/arm-anykav200-crosstool-ng/bin:$PATH"
$CARGO clippy -p snmp-agent --target x86_64-unknown-linux-gnu --all-targets -- -D warnings
```

Expected: tests PASS; clippy clean. The two `await_holding_lock` errors at `server.rs:413` and `server.rs:515` are gone because the lock is gone.

**Step 5: Commit**

```bash
git add cross-compile/snmp-agent/src/server.rs cross-compile/snmp-agent/src/main.rs
git commit -m "refactor(snmp): reload over a channel instead of a self-installed signal"
```

---

### Task 4: Retry a failed bind instead of waiting for SIGHUP

The design promised "log + backoff retry". The implementation retries only on reload, so a port collision at boot leaves the agent silently unbound forever.

**Files:**
- Modify: `cross-compile/snmp-agent/src/server.rs`

**Step 1: Add the constant**

Near `DEFAULT_PIDFILE`:

```rust
/// How long to wait before retrying a bind that failed.
// ponytail: fixed interval, not exponential. Move to backoff only if a real
// deployment shows the retries themselves costing anything.
const BIND_RETRY: std::time::Duration = std::time::Duration::from_secs(30);
```

**Step 2: Add the retry arm to the `select!`**

Between the reload arm and the recv arm:

```rust
            _ = tokio::time::sleep(BIND_RETRY), if socket.is_none() && agent.config.enabled => {
                match bind_socket(agent.config.port).await {
                    Ok(s) => {
                        tracing::info!(port = agent.config.port, "snmp-agent bound on retry");
                        socket = Some(s);
                    }
                    Err(e) => tracing::debug!(error = %e, port = agent.config.port, "bind retry failed"),
                }
            }
```

Note: `agent` here is whatever the loop's config holder is named at this point in the sequence — it is still `state`/`LiveSources` until Task 8 renames it. Adapt the field access, not the logic.

**Step 3: Test it**

Add to the `server.rs` tests:

```rust
    #[tokio::test(start_paused = true)]
    async fn test_bind_retry_recovers_after_the_port_frees() {
        let dir = tempfile::tempdir().unwrap();
        let cfg_path = dir.path().join("snmp.toml");
        let pidfile = dir.path().join("retry.pid");

        let holder = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let port = holder.local_addr().unwrap().port();
        std::fs::write(
            &cfg_path,
            format!("enabled = true\nport = {port}\ncommunity = \"public\"\nsys_name = \"retry\"\n"),
        )
        .unwrap();

        let (_tx, rx) = tokio::sync::mpsc::channel(1);
        let handle = tokio::spawn(async move { let _ = run(cfg_path, pidfile, rx).await; });

        tokio::time::sleep(Duration::from_millis(50)).await; // initial bind fails
        drop(holder);
        tokio::time::advance(BIND_RETRY + Duration::from_secs(1)).await;
        tokio::task::yield_now().await;

        let client = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        client.send_to(&get_sysname_bytes("public"), ("127.0.0.1", port)).await.unwrap();
        let mut buf = [0u8; 2048];
        let (n, _) = tokio::time::timeout(Duration::from_secs(5), client.recv_from(&mut buf))
            .await
            .expect("agent must answer after the retry")
            .unwrap();
        assert!(SnmpMessage::parse(&buf[..n]).is_ok());

        handle.abort();
        let _ = handle.await;
    }
```

If paused time proves awkward against real sockets, temporarily shorten `BIND_RETRY` behind `#[cfg(test)]` rather than sleeping 30 real seconds.

**Step 4: Run**

```bash
$CARGO test -p snmp-agent --target x86_64-unknown-linux-gnu --lib server
```

Expected: PASS.

**Step 5: Commit**

```bash
git add cross-compile/snmp-agent/src/server.rs
git commit -m "fix(snmp): retry a failed bind instead of waiting for a reload"
```

---

### Task 5: Encode unsigned application types as unsigned

`Counter32`/`Gauge32`/`TimeTicks` route through `encode_integer`, which strips leading `0xff` bytes. The crate's own test asserts `encode_integer(-1) == vec![0xff]`, so `Counter32(0xFFFF_FFFF)` goes on the wire as one byte and any conformant receiver reads **255**.

**Files:**
- Modify: `cross-compile/snmp-agent/src/ber.rs`
- Modify: `cross-compile/snmp-agent/src/pdu.rs`

**Step 1: Write the failing tests**

In `ber.rs` tests:

```rust
    #[test]
    fn test_encode_unsigned_never_strips_ff() {
        assert_eq!(encode_unsigned(0), vec![0x00]);
        assert_eq!(encode_unsigned(200), vec![0x00, 0xc8]);
        assert_eq!(encode_unsigned(0x7fff_ffff), vec![0x7f, 0xff, 0xff, 0xff]);
        // The bug: encode_integer(-1) would give [0xff] and read back as 255.
        assert_eq!(encode_unsigned(u32::MAX), vec![0x00, 0xff, 0xff, 0xff, 0xff]);
        assert_eq!(encode_unsigned(0xffff_ff00), vec![0x00, 0xff, 0xff, 0xff, 0x00]);
    }
```

In `pdu.rs` tests:

```rust
    #[test]
    fn test_counter32_max_round_trips() {
        let oid = Oid::from_slice(&[1, 3, 6, 1, 2, 1, 2, 2, 1, 10, 1]).unwrap();
        let msg = SnmpMessage {
            version: SNMP_V2C_VERSION,
            community: "public".into(),
            pdu: Pdu {
                pdu_type: PduType::GetResponse,
                request_id: 1,
                error_status: 0,
                error_index: 0,
                variable_bindings: vec![VarBind {
                    name: oid,
                    value: SnmpValue::Counter32(u32::MAX),
                }],
            },
        };
        let again = SnmpMessage::parse(&msg.encode().unwrap()).unwrap();
        assert_eq!(again.pdu.variable_bindings[0].value, SnmpValue::Counter32(u32::MAX));
    }
```

**Step 2: Run — expect FAIL**

```bash
$CARGO test -p snmp-agent --target x86_64-unknown-linux-gnu --lib
```

Expected: `encode_unsigned` not found; the round-trip test fails once it compiles.

**Step 3: Implement**

In `ber.rs`:

```rust
/// Encode a u32 for the unsigned application types (Counter32/Gauge32/TimeTicks).
///
/// BER integers are two's complement, so a value with the top bit set needs a
/// leading zero to stay non-negative — this is what net-snmp emits. Routing
/// these through `encode_integer` instead strips leading `0xff` bytes and turns
/// `0xFFFF_FFFF` into `255` on the wire.
pub fn encode_unsigned(value: u32) -> Vec<u8> {
    let mut bytes = value.to_be_bytes().to_vec();
    while bytes.len() > 1 && bytes[0] == 0 {
        bytes.remove(0);
    }
    if bytes[0] & 0x80 != 0 {
        bytes.insert(0, 0);
    }
    bytes
}
```

In `pdu.rs`, change the three arms of `encode_value`:

```rust
        SnmpValue::Counter32(v) => ber::write_tlv(TAG_COUNTER32, &ber::encode_unsigned(*v), out),
        SnmpValue::Gauge32(v) => ber::write_tlv(TAG_GAUGE32, &ber::encode_unsigned(*v), out),
        SnmpValue::TimeTicks(t) => ber::write_tlv(TAG_TIMETICKS, &ber::encode_unsigned(*t), out),
```

And replace `decode_u32_app` so it reads unsigned rather than borrowing the signed decoder:

```rust
fn decode_u32_app(content: &[u8]) -> Result<u32, PduError> {
    // Up to 5 bytes: real agents pad values with the top bit set with a leading zero.
    if content.is_empty() || content.len() > 5 {
        return Err(PduError::Malformed);
    }
    if content.len() == 5 && content[0] != 0 {
        return Err(PduError::Malformed);
    }
    let mut value: u64 = 0;
    for &b in content {
        value = (value << 8) | u64::from(b);
    }
    u32::try_from(value).map_err(|_| PduError::Malformed)
}
```

**Step 4: Fix the now-wrong existing assertion**

`test_decode_value_rejects_unknown_tag_and_negative_counter` asserts `decode_u32_app(&[0xff])` is `Malformed`. Under unsigned decoding that is legitimately `255`. Replace that assertion with an over-long input and rename the test:

```rust
    #[test]
    fn test_decode_value_rejects_unknown_tag_and_oversized_unsigned() {
        assert!(matches!(decode_value(0x99, &[]), Err(PduError::Malformed)));
        assert_eq!(decode_u32_app(&[0xff]).unwrap(), 255);
        assert!(matches!(decode_u32_app(&[0x01, 0, 0, 0, 0]), Err(PduError::Malformed)));
        assert!(matches!(decode_u32_app(&[0, 0, 0, 0, 0, 0]), Err(PduError::Malformed)));
    }
```

**Step 5: Run — expect PASS, then commit**

```bash
$CARGO test -p snmp-agent --target x86_64-unknown-linux-gnu --lib
git add cross-compile/snmp-agent/src/ber.rs cross-compile/snmp-agent/src/pdu.rs
git commit -m "fix(snmp): encode Counter32/Gauge32/TimeTicks as unsigned"
```

---

### Task 6: Detect base-128 overflow

`checked_shl(7)` only rejects shift counts ≥ 32 — it never notices bits leaving the top of the word, so an oversized OID arc silently truncates to a different valid arc instead of erroring.

**Files:**
- Modify: `cross-compile/snmp-agent/src/ber.rs`

**Step 1: Write the failing test**

```rust
    #[test]
    fn test_decode_rejects_oversized_base128_arc() {
        // Six continuation bytes: more than 32 bits of payload.
        let bytes = [0x2b, 0x8f, 0xff, 0xff, 0xff, 0xff, 0x7f];
        assert_eq!(Oid::decode(&bytes), Err(BerError::InvalidOid));
    }
```

**Step 2: Run — expect FAIL** (it decodes to a truncated arc and returns `Ok`).

**Step 3: Fix `decode_base128`**

```rust
        value = value
            .checked_mul(128)
            .and_then(|v| v.checked_add(u32::from(b & 0x7f)))
            .ok_or(BerError::InvalidOid)?;
```

**Step 4: Run — expect PASS, then commit**

```bash
$CARGO test -p snmp-agent --target x86_64-unknown-linux-gnu --lib ber
git add cross-compile/snmp-agent/src/ber.rs
git commit -m "fix(snmp): detect base-128 arc overflow in OID decoding"
```

---

### Task 7: Add SNMPv2c exception values

RFC 3416 puts "no such thing" in the varbind, not in `error-status`. The agent instead fails the whole PDU with `noSuchName`, an SNMPv1 code — so a poller batching ten OIDs loses all ten when one is unsupported.

**Files:**
- Modify: `cross-compile/snmp-agent/src/pdu.rs`

**Step 1: Write the failing test**

```rust
    #[test]
    fn test_exception_values_round_trip() {
        for (value, tag) in [
            (SnmpValue::NoSuchObject, 0x80u8),
            (SnmpValue::NoSuchInstance, 0x81),
            (SnmpValue::EndOfMibView, 0x82),
        ] {
            let mut out = Vec::new();
            encode_value(&value, &mut out).unwrap();
            assert_eq!(out, vec![tag, 0x00], "{value:?} must be a zero-length TLV");
            assert_eq!(decode_value(tag, &[]).unwrap(), value);
        }
    }
```

**Step 2: Run — expect FAIL** (variants do not exist).

**Step 3: Implement**

Add to `SnmpValue`:

```rust
    /// v2c exception: the object does not exist in this MIB view (context tag [0]).
    NoSuchObject,
    /// v2c exception: the object exists but this instance does not (context tag [1]).
    NoSuchInstance,
    /// v2c exception: no object follows this OID (context tag [2]).
    EndOfMibView,
```

Add the tags beside `TAG_COUNTER32`:

```rust
const TAG_NO_SUCH_OBJECT: u8 = 0x80;
const TAG_NO_SUCH_INSTANCE: u8 = 0x81;
const TAG_END_OF_MIB_VIEW: u8 = 0x82;
```

Add to `decode_value`, above the `_ =>` arm:

```rust
        TAG_NO_SUCH_OBJECT if content.is_empty() => Ok(SnmpValue::NoSuchObject),
        TAG_NO_SUCH_INSTANCE if content.is_empty() => Ok(SnmpValue::NoSuchInstance),
        TAG_END_OF_MIB_VIEW if content.is_empty() => Ok(SnmpValue::EndOfMibView),
```

Add to `encode_value`:

```rust
        SnmpValue::NoSuchObject => ber::write_tlv(TAG_NO_SUCH_OBJECT, &[], out),
        SnmpValue::NoSuchInstance => ber::write_tlv(TAG_NO_SUCH_INSTANCE, &[], out),
        SnmpValue::EndOfMibView => ber::write_tlv(TAG_END_OF_MIB_VIEW, &[], out),
```

**Step 4: Run — expect PASS, then commit**

```bash
$CARGO test -p snmp-agent --target x86_64-unknown-linux-gnu --lib pdu
git add cross-compile/snmp-agent/src/pdu.rs
git commit -m "feat(snmp): add v2c noSuchObject/noSuchInstance/endOfMibView"
```

---

### Task 8: Build `IfRow` from sysfs

`ifOperStatus` is hardcoded to `up(1)`, so the one thing an NMS exists to detect — a link going down — is unreportable. MAC, MTU, ifType and ifAdminStatus are stubbed the same way, and `ifIndex` is positional so it renumbers when `wlan0` drops out.

**Files:**
- Modify: `cross-compile/snmp-agent/src/mib/interfaces.rs`

**Step 1: Write the failing tests**

```rust
    fn fake_sysfs(root: &Path, name: &str, kv: &[(&str, &str)]) {
        let dir = root.join(name);
        std::fs::create_dir_all(&dir).unwrap();
        for (k, v) in kv {
            std::fs::write(dir.join(k), v).unwrap();
        }
    }

    /// Writes the shared `/proc/net/dev` fixture and a sysfs tree; returns both roots.
    fn fake_roots(dir: &Path, ifindexes: [&str; 3]) -> (PathBuf, PathBuf) {
        let proc_net_dev = dir.join("net-dev");
        std::fs::write(proc_net_dev.as_path(), include_str!("../../tests/fixtures/proc_net_dev.txt")).unwrap();
        let sys = dir.join("sys");
        fake_sysfs(&sys, "lo", &[
            ("ifindex", ifindexes[0]), ("operstate", "unknown"),
            ("address", "00:00:00:00:00:00"), ("mtu", "65536"),
            ("type", "772"), ("flags", "0x9"),
        ]);
        fake_sysfs(&sys, "eth0", &[
            ("ifindex", ifindexes[1]), ("operstate", "down"),
            ("address", "aa:bb:cc:dd:ee:ff"), ("mtu", "1500"),
            ("type", "1"), ("flags", "0x1002"),
        ]);
        fake_sysfs(&sys, "wlan0", &[
            ("ifindex", ifindexes[2]), ("operstate", "up"),
            ("address", "11:22:33:44:55:66"), ("mtu", "1500"),
            ("type", "1"), ("flags", "0x1003"),
        ]);
        (proc_net_dev, sys)
    }

    #[test]
    fn test_sysfs_reports_a_down_interface_as_down() {
        let dir = tempfile::tempdir().unwrap();
        let (proc_net_dev, sys) = fake_roots(dir.path(), ["1", "2", "3"]);
        let rows = load_interfaces(&proc_net_dev, &sys);

        assert_eq!(rows.len(), 3);
        // lo: loopback type, no address, unknown oper state, admin up (0x9 has IFF_UP)
        assert_eq!(rows[0].if_type, 24);
        assert_eq!(rows[0].oper_status, 4);
        assert_eq!(rows[0].phys_address, Vec::<u8>::new());
        assert_eq!(rows[0].mtu, 65536);
        assert_eq!(rows[0].admin_status, 1);
        // eth0 is DOWN — the whole reason this test exists
        assert_eq!(rows[1].oper_status, 2);
        assert_eq!(rows[1].admin_status, 2); // 0x1002 carries no IFF_UP
        assert_eq!(rows[1].if_type, 6);
        assert_eq!(rows[1].phys_address, vec![0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff]);
        // wlan0 is up
        assert_eq!(rows[2].oper_status, 1);
        assert_eq!(rows[2].admin_status, 1);
    }

    #[test]
    fn test_rows_sort_by_kernel_ifindex() {
        let dir = tempfile::tempdir().unwrap();
        let (proc_net_dev, sys) = fake_roots(dir.path(), ["1", "5", "3"]);
        let rows = load_interfaces(&proc_net_dev, &sys);
        assert_eq!(
            rows.iter().map(|r| (r.index, r.descr.as_str())).collect::<Vec<_>>(),
            vec![(1, "lo"), (3, "wlan0"), (5, "eth0")],
        );
    }

    #[test]
    fn test_missing_sysfs_falls_back_to_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let proc_net_dev = dir.path().join("net-dev");
        std::fs::write(&proc_net_dev, include_str!("../../tests/fixtures/proc_net_dev.txt")).unwrap();
        let rows = load_interfaces(&proc_net_dev, &dir.path().join("no-sysfs"));
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].index, 1);        // positional fallback
        assert_eq!(rows[0].oper_status, 4);  // unknown, never a fabricated "up"
        assert_eq!(rows[0].mtu, 1500);
    }
```

**Step 2: Run — expect FAIL** (fields and the two-argument `load_interfaces` do not exist).

**Step 3: Implement**

Replace `IfRow`, `parse_proc_net_dev`'s row construction, `load_interfaces` and `cell_value`:

```rust
/// One row of ifTable, from `/proc/net/dev` counters plus `/sys/class/net` metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IfRow {
    pub index: u32,
    pub descr: String,
    pub if_type: i32,
    pub mtu: i32,
    pub phys_address: Vec<u8>,
    pub admin_status: i32,
    pub oper_status: i32,
    pub in_octets: u32,
    pub out_octets: u32,
}
```

In `parse_proc_net_dev`, push rows with these defaults — never a fabricated `up`:

```rust
        rows.push(IfRow {
            index: (rows.len() as u32) + 1,
            descr: name,
            if_type: 1,      // other
            mtu: 1500,
            phys_address: Vec::new(),
            admin_status: 1, // ifAdminStatus has no "unknown"; up is the only sane default
            oper_status: 4,  // unknown
            in_octets,
            out_octets,
        });
```

Add the sysfs layer:

```rust
fn read_trim(path: &Path) -> Option<String> {
    std::fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

/// RFC 2863 ifOperStatus from the sysfs `operstate` string.
fn oper_status_code(s: &str) -> i32 {
    match s {
        "up" => 1,
        "down" => 2,
        "testing" => 3,
        "dormant" => 5,
        "notpresent" => 6,
        "lowerlayerdown" => 7,
        _ => 4, // unknown
    }
}

/// ifType from the sysfs ARPHRD value.
fn if_type_code(arphrd: u32) -> i32 {
    match arphrd {
        1 => 6,            // ARPHRD_ETHER -> ethernetCsmacd (wifi presents as this too)
        772 => 24,         // ARPHRD_LOOPBACK -> softwareLoopback
        801 | 802 | 803 => 71, // ARPHRD_IEEE80211* -> ieee80211
        _ => 1,            // other
    }
}

/// `aa:bb:cc:dd:ee:ff` -> six bytes. An all-zero address means "no address",
/// which RFC 2863 asks us to report as a zero-length octet string.
fn parse_mac(s: &str) -> Vec<u8> {
    let bytes: Vec<u8> = s
        .split(':')
        .filter_map(|b| u8::from_str_radix(b, 16).ok())
        .collect();
    if bytes.len() != 6 || bytes.iter().all(|&b| b == 0) {
        return Vec::new();
    }
    bytes
}

/// Overlay sysfs metadata onto a row. Every field degrades to its default.
fn enrich(row: &mut IfRow, sys_root: &Path) {
    let dir = sys_root.join(&row.descr);
    if let Some(v) = read_trim(&dir.join("ifindex")).and_then(|s| s.parse().ok()) {
        row.index = v;
    }
    if let Some(s) = read_trim(&dir.join("operstate")) {
        row.oper_status = oper_status_code(&s);
    }
    if let Some(s) = read_trim(&dir.join("address")) {
        row.phys_address = parse_mac(&s);
    }
    if let Some(v) = read_trim(&dir.join("mtu")).and_then(|s| s.parse().ok()) {
        row.mtu = v;
    }
    if let Some(v) = read_trim(&dir.join("type")).and_then(|s| s.parse::<u32>().ok()) {
        row.if_type = if_type_code(v);
    }
    if let Some(f) = read_trim(&dir.join("flags")) {
        if let Ok(bits) = u32::from_str_radix(f.strip_prefix("0x").unwrap_or(&f), 16) {
            row.admin_status = if bits & 0x1 != 0 { 1 } else { 2 }; // IFF_UP
        }
    }
}

/// Read ifTable rows, sorted by kernel ifIndex so an NMS keyed on it stays
/// stable when an interface disappears and returns.
pub fn load_interfaces(proc_net_dev: &Path, sys_root: &Path) -> Vec<IfRow> {
    let text = std::fs::read_to_string(proc_net_dev).unwrap_or_default();
    let mut rows = parse_proc_net_dev(&text);
    for row in &mut rows {
        enrich(row, sys_root);
    }
    rows.sort_by_key(|r| r.index);
    rows
}
```

Update `cell_value`:

```rust
        3 => SnmpValue::Integer(row.if_type),
        4 => SnmpValue::Integer(row.mtu),
        5 => SnmpValue::Gauge32(0), // sysfs `speed` is EINVAL on wifi and loopback
        6 => SnmpValue::OctetString(row.phys_address.clone()),
        7 => SnmpValue::Integer(row.admin_status),
        8 => SnmpValue::Integer(row.oper_status),
```

Delete the old single-argument `load_interfaces` and the `get`/`get_next` wrappers that took `_sources` and read `/proc/net/dev` directly — Task 9 replaces them.

**Step 4: Run — expect PASS**

```bash
$CARGO test -p snmp-agent --target x86_64-unknown-linux-gnu --lib mib::interfaces
```

**Step 5: Commit**

```bash
git add cross-compile/snmp-agent/src/mib/interfaces.rs
git commit -m "feat(snmp): build ifTable rows from sysfs, sorted by kernel ifIndex"
```

---

### Task 9: Per-request snapshot and RFC varbind semantics

The MIB currently re-reads `/proc/net/dev` once per varbind, so a 30-varbind walk observes thirty different instants. Give it one snapshot per datagram, and while the trait is changing, switch `handle_varbinds` to per-varbind exceptions.

**Files:**
- Modify: `cross-compile/snmp-agent/src/mib/mod.rs`
- Modify: `cross-compile/snmp-agent/src/mib/interfaces.rs`

**Step 1: Write the failing tests**

In `mib/mod.rs` tests — replace `FixedSources` with a snapshot-backed one carrying rows, then:

```rust
    #[test]
    fn test_get_unknown_oid_returns_exception_not_pdu_error() {
        let binds = vec![
            VarBind { name: Oid::from_slice(&[1, 3, 6, 1, 2, 1, 1, 1, 0]).unwrap(), value: SnmpValue::Null },
            VarBind { name: Oid::from_slice(&[1, 3, 6, 1, 2, 1, 99, 1, 0]).unwrap(), value: SnmpValue::Null },
            VarBind { name: Oid::from_slice(&[1, 3, 6, 1, 2, 1, 1, 99, 0]).unwrap(), value: SnmpValue::Null },
        ];
        let (status, index, out) = handle_varbinds(PduType::GetRequest, &binds, &sources());
        assert_eq!(status, ERR_NO_ERROR, "one bad OID must not fail the whole PDU");
        assert_eq!(index, 0);
        assert!(matches!(out[0].value, SnmpValue::OctetString(_)), "good varbind still answered");
        assert_eq!(out[1].value, SnmpValue::NoSuchObject);   // unknown group
        assert_eq!(out[2].value, SnmpValue::NoSuchInstance); // known group, bad instance
    }

    #[test]
    fn test_getnext_past_the_end_returns_end_of_mib_view() {
        let binds = vec![VarBind {
            name: Oid::from_slice(&[1, 3, 6, 1, 2, 1, 2, 2, 1, 16, 99]).unwrap(),
            value: SnmpValue::Null,
        }];
        let (status, _, out) = handle_varbinds(PduType::GetNextRequest, &binds, &sources());
        assert_eq!(status, ERR_NO_ERROR);
        assert_eq!(out[0].value, SnmpValue::EndOfMibView);
    }
```

**Step 2: Run — expect FAIL.**

**Step 3: Implement the trait and snapshot**

In `mib/mod.rs`:

```rust
use crate::mib::interfaces::IfRow;

pub trait MibSources {
    fn config(&self) -> &SnmpConfig;
    fn uptime_ticks(&self) -> u32;
    fn interfaces(&self) -> &[IfRow];
}

/// One consistent view of the device, captured per datagram.
///
/// Capturing once means a multi-varbind walk observes a single instant instead
/// of re-reading `/proc` for every varbind.
pub struct Snapshot {
    pub config: SnmpConfig,
    pub uptime_ticks: u32,
    pub ifaces: Vec<IfRow>,
}

impl MibSources for Snapshot {
    fn config(&self) -> &SnmpConfig { &self.config }
    fn uptime_ticks(&self) -> u32 { self.uptime_ticks }
    fn interfaces(&self) -> &[IfRow] { &self.ifaces }
}
```

Delete `ERR_NO_SUCH_NAME` — it is an SNMPv1 code with no v2c use — and rewrite the dispatcher:

```rust
pub fn handle_varbinds(
    pdu_type: PduType,
    binds: &[VarBind],
    sources: &dyn MibSources,
) -> (i32, i32, Vec<VarBind>) {
    if pdu_type == PduType::SetRequest {
        return (ERR_NOT_WRITABLE, 1, binds.to_vec());
    }

    let mut out = Vec::with_capacity(binds.len());
    for vb in binds {
        // RFC 3416: a missing object is an exception *in the varbind*, so one
        // bad OID does not cost the caller the other nine.
        let (name, value) = if pdu_type == PduType::GetRequest {
            resolve_get(&vb.name, sources)
                .unwrap_or_else(|| (vb.name.clone(), miss_kind(&vb.name)))
        } else {
            resolve_get_next(&vb.name, sources)
                .unwrap_or_else(|| (vb.name.clone(), SnmpValue::EndOfMibView))
        };
        out.push(VarBind { name, value });
    }
    (ERR_NO_ERROR, 0, out)
}

/// `noSuchInstance` when we serve the group but not that instance, else `noSuchObject`.
fn miss_kind(oid: &crate::ber::Oid) -> SnmpValue {
    const SERVED: [[u32; 7]; 2] = [[1, 3, 6, 1, 2, 1, 1], [1, 3, 6, 1, 2, 1, 2]];
    if oid.0.len() > 7 && SERVED.iter().any(|g| oid.0[..7] == *g) {
        SnmpValue::NoSuchInstance
    } else {
        SnmpValue::NoSuchObject
    }
}
```

In `mib/interfaces.rs`, make `get`/`get_next` read the snapshot instead of the filesystem:

```rust
pub fn get(oid: &Oid, sources: &dyn MibSources) -> Option<(Oid, SnmpValue)> {
    get_with_rows(oid, sources.interfaces())
}

pub fn get_next(oid: &Oid, sources: &dyn MibSources) -> Option<(Oid, SnmpValue)> {
    get_next_with_rows(oid, sources.interfaces())
}
```

Delete `test_live_proc_get_if_number_smoke` — it existed only to exercise the hardcoded `/proc` path that no longer exists.

**Step 4: Run — expect PASS**

```bash
$CARGO test -p snmp-agent --target x86_64-unknown-linux-gnu --lib mib
```

**Step 5: Commit**

```bash
git add cross-compile/snmp-agent/src/mib
git commit -m "feat(snmp): per-request MIB snapshot and RFC 3416 varbind exceptions"
```

---

### Task 10: Wire the Agent, drop the lock, filter PDU types

Three things land together because they touch the same function. `handle_datagram` answers inbound `GetResponse` packets, so one spoofed-source datagram makes the agent talk to itself forever on a 400 MHz core. `sysUpTime` is process uptime, so a supervised restart looks like a device reboot. And `Arc<RwLock<LiveSources>>` guards nothing — the loop is single-tasked.

**Files:**
- Modify: `cross-compile/snmp-agent/src/server.rs`

**Step 1: Write the failing tests**

```rust
    #[test]
    fn test_get_response_is_never_answered() {
        let agent = test_agent();
        let mut req = get_sysname_bytes("public");
        // Flip the PDU tag from GetRequest [0] to GetResponse [2].
        let i = req.iter().position(|&b| b == 0xa0).expect("pdu tag");
        req[i] = 0xa2;
        assert!(
            handle_datagram(&req, &agent).is_none(),
            "answering a response lets a spoofed source loop us against ourselves"
        );
    }

    #[test]
    fn test_uptime_comes_from_proc_uptime() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("uptime"), "12345.67 98765.43\n").unwrap();
        let agent = Agent::with_roots(SnmpConfig::default(), dir.path().into(), dir.path().join("sys"));
        assert_eq!(agent.snapshot().uptime_ticks(), 1_234_567);
    }
```

**Step 2: Run — expect FAIL.**

**Step 3: Implement**

Replace `LiveSources` with `Agent`:

```rust
/// Owns the config and the filesystem roots the MIB is built from.
pub struct Agent {
    pub config: SnmpConfig,
    proc_root: PathBuf,
    sys_class_net: PathBuf,
    started: Instant,
}

impl Agent {
    pub fn new(config: SnmpConfig) -> Self {
        Self::with_roots(config, PathBuf::from("/proc"), PathBuf::from("/sys/class/net"))
    }

    pub fn with_roots(config: SnmpConfig, proc_root: PathBuf, sys_class_net: PathBuf) -> Self {
        Self { config, proc_root, sys_class_net, started: Instant::now() }
    }

    /// System uptime in hundredths of a second.
    ///
    /// `/proc/uptime`, not process uptime: anyka-init restarts this binary on
    /// crash, and an NMS reads a sysUpTime reset as a device reboot.
    fn uptime_ticks(&self) -> u32 {
        proc_uptime_ticks(&self.proc_root.join("uptime")).unwrap_or_else(|| {
            let e = self.started.elapsed();
            (e.as_secs() * 100 + u64::from(e.subsec_millis()) / 10).min(u64::from(u32::MAX)) as u32
        })
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            config: self.config.clone(),
            uptime_ticks: self.uptime_ticks(),
            ifaces: interfaces::load_interfaces(
                &self.proc_root.join("net/dev"),
                &self.sys_class_net,
            ),
        }
    }
}

fn proc_uptime_ticks(path: &Path) -> Option<u32> {
    let text = std::fs::read_to_string(path).ok()?;
    let secs: f64 = text.split_whitespace().next()?.parse().ok()?;
    if !secs.is_finite() || secs < 0.0 {
        return None;
    }
    Some((secs * 100.0).min(f64::from(u32::MAX)) as u32)
}
```

Rewrite `handle_datagram`:

```rust
pub fn handle_datagram(bytes: &[u8], agent: &Agent) -> Option<Vec<u8>> {
    let msg = SnmpMessage::parse(bytes).ok()?;

    // Only ever answer requests. Answering a GetResponse lets one packet with a
    // spoofed source address make the agent respond to itself forever.
    if !matches!(
        msg.pdu.pdu_type,
        PduType::GetRequest | PduType::GetNextRequest | PduType::SetRequest
    ) {
        return None;
    }

    if msg.community != agent.config.community {
        // Wrong community: silent drop (no scanner oracle), and no /proc read.
        return None;
    }

    let snapshot = agent.snapshot();
    let (error_status, error_index, variable_bindings) =
        mib::handle_varbinds(msg.pdu.pdu_type, &msg.pdu.variable_bindings, &snapshot);

    SnmpMessage {
        version: SNMP_V2C_VERSION,
        community: msg.community,
        pdu: Pdu {
            pdu_type: PduType::GetResponse,
            request_id: msg.pdu.request_id,
            error_status,
            error_index,
            variable_bindings,
        },
    }
    .encode()
    .ok()
}
```

In `run()`, replace `Arc<RwLock<LiveSources>>` with a plain `let mut agent = Agent::new(SnmpConfig::load(&config_path)?);`. Drop the `Arc`, `RwLock` and `tokio::sync::RwLock` imports. Reload assigns `agent.config = new_cfg.clone();` with no guard dance; the serve arm calls `handle_datagram(&buf[..n], &agent)` directly.

Add a `test_agent()` helper to the tests and delete `test_udp_get_sysname_ephemeral` — it binds its own socket and calls `handle_datagram` by hand, so it exercises nothing `test_run_serves_udp_get_reload_and_disable` does not.

**Step 4: Run tests and clippy**

```bash
$CARGO test -p snmp-agent --target x86_64-unknown-linux-gnu
export PATH="/home/kmk/dev/anyka-dev/toolchain/arm-anykav200-crosstool-ng/bin:$PATH"
$CARGO clippy -p snmp-agent --target x86_64-unknown-linux-gnu --all-targets -- -D warnings
```

Expected: PASS and clean.

**Step 5: Commit**

```bash
git add cross-compile/snmp-agent/src/server.rs
git commit -m "fix(snmp): drop response PDUs, read /proc/uptime, remove the unshared lock"
```

---

### Task 11: End-to-end walk test

Every finding above survived a green unit suite. This test is the shape that would have caught them: it drives a real agent over a real socket and asserts the wire behaviour.

**Files:**
- Create: `cross-compile/snmp-agent/tests/walk.rs`

**Step 1: Write the test**

```rust
//! End-to-end walk against a running agent. No net-snmp dependency, so it gates CI.

use snmp_agent::ber::Oid;
use snmp_agent::config::SnmpConfig;
use snmp_agent::pdu::{Pdu, PduType, SNMP_V2C_VERSION, SnmpMessage, SnmpValue, VarBind};
use std::time::Duration;
use tokio::net::UdpSocket;

async fn spawn_agent() -> (u16, tokio::task::JoinHandle<()>, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let cfg = dir.path().join("snmp.toml");
    let pid = dir.path().join("agent.pid");

    let probe = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let port = probe.local_addr().unwrap().port();
    drop(probe);
    std::fs::write(
        &cfg,
        format!("enabled = true\nport = {port}\ncommunity = \"public\"\nsys_name = \"walk-cam\"\n"),
    )
    .unwrap();

    let (_tx, rx) = tokio::sync::mpsc::channel(1);
    let handle = tokio::spawn(async move { let _ = snmp_agent::server::run(cfg, pid, rx).await; });
    tokio::time::sleep(Duration::from_millis(150)).await;
    (port, handle, dir)
}

fn request(pdu_type: PduType, oid: Oid) -> Vec<u8> {
    SnmpMessage {
        version: SNMP_V2C_VERSION,
        community: "public".into(),
        pdu: Pdu {
            pdu_type,
            request_id: 42,
            error_status: 0,
            error_index: 0,
            variable_bindings: vec![VarBind { name: oid, value: SnmpValue::Null }],
        },
    }
    .encode()
    .unwrap()
}

async fn ask(sock: &UdpSocket, port: u16, bytes: &[u8]) -> Option<SnmpMessage> {
    sock.send_to(bytes, ("127.0.0.1", port)).await.unwrap();
    let mut buf = [0u8; 2048];
    match tokio::time::timeout(Duration::from_millis(600), sock.recv_from(&mut buf)).await {
        Ok(Ok((n, _))) => Some(SnmpMessage::parse(&buf[..n]).expect("agent must emit valid BER")),
        _ => None,
    }
}

#[tokio::test]
async fn test_full_walk_is_ordered_and_terminates() {
    let (port, handle, _dir) = spawn_agent().await;
    let client = UdpSocket::bind("127.0.0.1:0").await.unwrap();

    let mut cursor = Oid::from_slice(&[1, 3]).unwrap();
    let mut seen: Vec<(Oid, SnmpValue)> = Vec::new();
    for _ in 0..500 {
        let resp = ask(&client, port, &request(PduType::GetNextRequest, cursor.clone()))
            .await
            .expect("GETNEXT must be answered");
        assert_eq!(resp.pdu.error_status, 0, "v2c reports misses in the varbind");
        let vb = resp.pdu.variable_bindings.into_iter().next().unwrap();
        if vb.value == SnmpValue::EndOfMibView {
            break;
        }
        assert!(cursor.0 < vb.name.0, "walk must strictly ascend: {:?} -> {:?}", cursor.0, vb.name.0);
        cursor = vb.name.clone();
        seen.push((vb.name, vb.value));
    }

    assert!(seen.len() >= 7, "at least the system group should be present");
    assert_eq!(seen[0].0.0, vec![1, 3, 6, 1, 2, 1, 1, 1, 0], "walk starts at sysDescr.0");
    assert!(
        seen.iter().any(|(o, _)| o.0.starts_with(&[1, 3, 6, 1, 2, 1, 2, 2, 1, 8])),
        "ifOperStatus column must be walked"
    );

    handle.abort();
}

#[tokio::test]
async fn test_unknown_oid_set_and_response_handling() {
    let (port, handle, _dir) = spawn_agent().await;
    let client = UdpSocket::bind("127.0.0.1:0").await.unwrap();

    // Unknown OID: noError plus an exception in the varbind.
    let resp = ask(&client, port, &request(PduType::GetRequest, Oid::from_slice(&[1, 3, 6, 1, 2, 1, 99, 1, 0]).unwrap()))
        .await
        .expect("GET must be answered");
    assert_eq!(resp.pdu.error_status, 0);
    assert_eq!(resp.pdu.variable_bindings[0].value, SnmpValue::NoSuchObject);

    // SET is refused and changes nothing.
    let resp = ask(&client, port, &request(PduType::SetRequest, Oid::from_slice(&[1, 3, 6, 1, 2, 1, 1, 5, 0]).unwrap()))
        .await
        .expect("SET must be answered");
    assert_eq!(resp.pdu.error_status, 17, "notWritable");
    let resp = ask(&client, port, &request(PduType::GetRequest, Oid::from_slice(&[1, 3, 6, 1, 2, 1, 1, 5, 0]).unwrap()))
        .await
        .unwrap();
    assert_eq!(resp.pdu.variable_bindings[0].value, SnmpValue::OctetString(b"walk-cam".to_vec()));

    // A response PDU draws silence — otherwise a spoofed source loops us forever.
    let mut bytes = request(PduType::GetRequest, Oid::from_slice(&[1, 3, 6, 1, 2, 1, 1, 1, 0]).unwrap());
    let i = bytes.iter().position(|&b| b == 0xa0).unwrap();
    bytes[i] = 0xa2;
    assert!(ask(&client, port, &bytes).await.is_none(), "must not answer a GetResponse");

    // Wrong community draws silence too.
    let mut wrong = request(PduType::GetRequest, Oid::from_slice(&[1, 3, 6, 1, 2, 1, 1, 1, 0]).unwrap());
    let c = wrong.windows(6).position(|w| w == b"public").unwrap();
    wrong[c] = b'X';
    assert!(ask(&client, port, &wrong).await.is_none(), "must not answer a bad community");

    handle.abort();
}
```

If `Oid`, `SnmpValue` or `server::run` are not reachable from outside the crate, add the missing `pub` to `lib.rs`'s module list rather than duplicating types in the test.

**Step 2: Run**

```bash
$CARGO test -p snmp-agent --target x86_64-unknown-linux-gnu --test walk
```

Expected: both tests PASS.

**Step 3: Commit**

```bash
git add cross-compile/snmp-agent/tests/walk.rs
git commit -m "test(snmp): end-to-end walk, exception, SET and reflection coverage"
```

---

### Task 12: Config key parity test

`SnmpConfig` (agent) and `SnmpSettings` (onvif-rust) are the same six fields declared twice, held together by a comment. `config/netoverlay.rs` already solved this for its twin in `anyka-init` with a parity test; copy the pattern rather than adding a workspace dependency from an ONVIF server onto an SNMP agent crate.

**Files:**
- Modify: `cross-compile/onvif-rust/src/config/snmp.rs`

**Step 1: Add the test**

```rust
    /// The agent parses this file with its own struct. If either side gains a
    /// field, this fails instead of the mismatch reaching a camera.
    #[test]
    fn test_keys_match_snmp_agent_config() {
        let toml = toml::to_string(&SnmpSettings::default()).unwrap();
        let keys: Vec<String> = toml
            .lines()
            .filter_map(|l| l.split('=').next())
            .map(|k| k.trim().to_string())
            .filter(|k| !k.is_empty())
            .collect();
        assert_eq!(
            keys,
            ["enabled", "port", "community", "sys_contact", "sys_name", "sys_location"],
            "keys changed: update snmp-agent/src/config.rs SnmpConfig to match"
        );
    }
```

Read `cross-compile/onvif-rust/src/config/netoverlay.rs` first and mirror its assertion style if it differs.

**Step 2: Run — expect PASS**

```bash
$CARGO test -p onvif-rust --target x86_64-unknown-linux-gnu --lib config::snmp
```

**Step 3: Commit**

```bash
git add cross-compile/onvif-rust/src/config/snmp.rs
git commit -m "test(snmp): guard snmp.toml key parity across the two crates"
```

---

### Task 13: Redact the community string from HTTP logs

`sanitize_soap_body` masks XML `<Password>` elements only, so the JSON body of `PUT /api/snmp` — and the `GET /api/snmp` response — carry the community into `/mnt/logs` verbatim when `http_verbose` is on. The original plan's execution notes said never to log it.

**Files:**
- Modify: `cross-compile/onvif-rust/src/logging/http.rs`

**Step 1: Read the existing masks**

```bash
grep -n "fn sanitize_soap_body" -A 40 cross-compile/onvif-rust/src/logging/http.rs
```

**Step 2: Write the failing test**

Add beside the existing sanitizer tests:

```rust
    #[test]
    fn test_sanitize_masks_json_community() {
        let body = r#"{"enabled":true,"port":161,"community":"s3cret","sys_name":"cam"}"#;
        let out = sanitize_soap_body(body);
        assert!(!out.contains("s3cret"), "community must never reach the log: {out}");
        assert!(out.contains("\"port\":161"), "unrelated fields survive: {out}");
    }
```

**Step 3: Run — expect FAIL.**

**Step 4: Implement**

Extend `sanitize_soap_body` with a JSON mask alongside the XML ones. The function already runs on both request and response bodies, so one edit closes both directions. Match the surrounding implementation style — if the existing masks are regex-based, add a `"community"\s*:\s*"[^"]*"` pattern; if they are string scans, add the equivalent scan.

**Step 5: Run — expect PASS, then commit**

```bash
$CARGO test -p onvif-rust --target x86_64-unknown-linux-gnu --lib logging::http
git add cross-compile/onvif-rust/src/logging/http.rs
git commit -m "fix(snmp): redact the community string from HTTP body logs"
```

---

### Task 14: WebUI tests for the SNMP card

Task 9 of the original plan promised `NetworkPage` coverage. The branch added only `vi.mock` plumbing — not one test asserts the card renders or saves.

**Files:**
- Modify: `cross-compile/www/src/pages/settings/NetworkPage.test.tsx`

**Step 1: Write the tests**

Add inside the existing `describe('NetworkPage')` block:

```tsx
  it('should render SNMP settings from the fetched config', async () => {
    vi.mocked(getSnmpConfig).mockResolvedValue({
      enabled: true,
      port: 1161,
      community: 'monitor',
      sys_contact: '',
      sys_name: '',
      sys_location: '',
    });

    await renderNetworkPage();

    expect(screen.getByTestId('network-snmp-port-input')).toHaveValue(1161);
    expect(screen.getByTestId('network-snmp-community-input')).toHaveValue('monitor');
  });

  it('should save SNMP settings on confirmation', async () => {
    const user = userEvent.setup();
    await renderNetworkPage();

    await makeFormDirty(user, 'network-snmp-port-input', '2161');
    await user.click(screen.getByTestId('network-save-button'));
    await user.click(await screen.findByTestId('network-confirm-save-button'));

    await waitFor(() => {
      expect(putSnmpConfig).toHaveBeenCalledWith(
        expect.objectContaining({ enabled: true, port: 2161, community: 'public' }),
      );
    });
  });

  it('should block saving when the community is empty', async () => {
    const user = userEvent.setup();
    await renderNetworkPage();

    await fillFormField(user, 'network-snmp-community-input', '');
    await user.click(screen.getByTestId('network-save-button'));

    expect(await screen.findByText('Community must not be empty')).toBeInTheDocument();
    expect(putSnmpConfig).not.toHaveBeenCalled();
  });
```

`makeFormDirty` and `fillFormField` come from `@/test/componentTestHelpers` and are already imported.

**Step 2: Run — expect FAIL for the right reasons** (assertions, not missing imports)

```bash
cd /home/kmk/dev/anyka-dev/.worktrees/snmp/cross-compile/www
npx vitest run src/pages/settings/NetworkPage.test.tsx
```

**Step 3: Adjust until green**

These test the existing UI, so failures mean the assertions are wrong, not the component — except the empty-community case, which depends on the zod message in `networkSchema` reading exactly `'Community must not be empty'`. Align the test with the schema, not the other way round.

**Step 4: Commit**

```bash
git add cross-compile/www/src/pages/settings/NetworkPage.test.tsx
git commit -m "test(www): cover the SNMP card render, save and validation"
```

---

### Task 15: Drop the dead bundle lines

`build_bundle.sh` copies `snmp.toml` into the bundle root, which `stage_and_flip` extracts to `slots/<x>/snmp.toml`. Nothing reads it — the agent is launched with the flat `/mnt/anyka_hack/snmp.toml`, and `slot_path` correctly leaves that path alone because `snmp.toml` matches no `BUNDLED` component.

**Files:**
- Modify: `scripts/build_bundle.sh`

**Step 1: Remove the copy**

Delete this line and leave the `mkdir -p "${STAGE}/snmp"` / `snmp-agent.bin` copy in place:

```bash
cp "${SRC}/snmp.toml"             "${STAGE}/snmp.toml"
```

**Step 2: Verify the bundle still builds**

```bash
cd /home/kmk/dev/anyka-dev/.worktrees/snmp
./scripts/build_bundle.sh
tar -tf bundle.tar | grep -E 'snmp' || true
```

Expected: `snmp/snmp-agent.bin` present, `snmp.toml` absent.

**Step 3: Commit**

```bash
git add scripts/build_bundle.sh
git commit -m "chore(snmp): stop staging snmp.toml into a slot nothing reads"
```

---

### Task 16: Quality gates and device verification

**Step 1: Full host gates**

```bash
cd /home/kmk/dev/anyka-dev/.worktrees/snmp
source /home/kmk/dev/anyka-dev/setenv.sh
export PATH="/home/kmk/dev/anyka-dev/toolchain/arm-anykav200-crosstool-ng/bin:$PATH"
cd cross-compile

$CARGO fmt
$CARGO clippy --target x86_64-unknown-linux-gnu --all-targets \
  -p snmp-agent -p onvif-rust -p anyka-init -- -D warnings
$CARGO test --target x86_64-unknown-linux-gnu -p snmp-agent -p onvif-rust -p anyka-init
```

Check `$?` after each command directly. Do not read a verdict out of an RTK-filtered or `tail`-piped run — `rtk git status` reported `ok` for a worktree with seven modified files during the review, and a `tail` pipe hid a signal-killed test binary behind exit 0.

**Step 2: Cold-run stability**

```bash
cd /home/kmk/dev/anyka-dev/.worktrees/snmp/cross-compile
# Rebuild the onvif-rust package if HTTP log sanitizer tests look stale:
# $CARGO test -p onvif-rust --target x86_64-unknown-linux-gnu logging::http
for i in $(seq 1 10); do
  $CARGO test -p onvif-rust --target x86_64-unknown-linux-gnu > /tmp/cold_$i.log 2>&1
  echo "run $i EXIT=$? sighup=$(grep -c 'SIGHUP' /tmp/cold_$i.log)"
done
```

Expected: ten × `EXIT=0 sighup=0`.

**Step 3: WebUI gates**

```bash
cd /home/kmk/dev/anyka-dev/.worktrees/snmp/cross-compile/www
npm run lint && npm run type-check && npm run test
npx prettier --check src   # run the raw binary; the RTK wrapper has reported success on a real failure
echo "prettier exit=$?"
```

**Step 4: ARM build**

```bash
cd /home/kmk/dev/anyka-dev/.worktrees/snmp/cross-compile/onvif-rust
$CARGO build --release -p snmp-agent
file ../target/armv5te-unknown-linux-uclibceabi/release/snmp-agent
```

Expected: `ELF 32-bit LSB executable, ARM, EABI5 … interpreter /mnt/anyka_hack/lib/ld-uClibc.so.1, stripped`.

**Step 5: Local snmpwalk against the host build**

`net-snmp` is installed on this machine. This is the check that caught the hardcoded `ifOperStatus`:

```bash
cd /home/kmk/dev/anyka-dev/.worktrees/snmp/cross-compile
$CARGO build -p snmp-agent --target x86_64-unknown-linux-gnu
printf 'enabled = true\nport = 16161\ncommunity = "public"\nsys_name = "verify"\n' > /tmp/verify.toml
setsid ./target/x86_64-unknown-linux-gnu/debug/snmp-agent --config /tmp/verify.toml \
  > /tmp/verify.log 2>&1 < /dev/null &
sleep 1

snmpwalk -v2c -c public -t 2 127.0.0.1:16161 1.3.6.1.2.1.1
snmpwalk -v2c -c public -t 2 127.0.0.1:16161 1.3.6.1.2.1.2.2.1.8   # ifOperStatus

# Cross-check a down interface against the kernel
for i in /sys/class/net/*/; do echo "$(basename $i) = $(cat $i/operstate)"; done

snmpset -v2c -c public -t 2 127.0.0.1:16161 1.3.6.1.2.1.1.5.0 s pwned   # expect notWritable
# Stop the verification agent by its recorded PID (do not delete files without permission):
# kill "$SNMP_VERIFY_PID"
```

Expected: any interface the kernel reports as `down` shows `INTEGER: 2`, not `1`. `snmpset` fails with `notWritable` and `sysName` is unchanged.

**Step 6: Deploy and verify on hardware**

```bash
cd /home/kmk/dev/anyka-dev/.worktrees/snmp
./scripts/build_sd_contents.sh
```

Then deploy per the `anyka-firmware-upgrade` skill and, from the laptop:

```bash
snmpwalk -v2c -c public <camera-ip> 1.3.6.1.2.1.1
snmpwalk -v2c -c public <camera-ip> 1.3.6.1.2.1.2
```

Confirm `ifOperStatus` for a disconnected interface reads `down(2)`, and that `sysUpTime` survives an `snmp-agent` restart without resetting to zero.

**Step 7: Update the design status and commit**

Mark `docs/plans/2026-08-25-snmp-review-fixes-design.md` status as `implemented`, note the device verification result, then:

```bash
git add docs/plans/2026-08-25-snmp-review-fixes-design.md
git commit -m "docs(snmp): record review-fix implementation and device verification"
```

**Step 8: Request code review** per `AGENTS.md` before merging.

---

## Execution notes

- The worktree carries uncommitted test additions from an earlier session. `git add` only the files each task names; never `git add -A`.
- Findings 4 and 8 are the two that a green unit suite hid. When a task says "expect FAIL", confirm it fails for the stated reason before implementing — a test that passes before the fix is testing the wrong thing.
- Never claim a gate is green from filtered output. Run the raw binary and read `$?`.
- `sysObjectID` stays `.1.3.6.1.4.1.0.1`. Enterprise `0` is reserved, but there is no correct value without a registered PEN; sharpen the comment rather than substituting a different squat.
