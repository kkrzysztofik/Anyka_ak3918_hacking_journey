# SNMP Review Fixes — Design

Date: 2026-08-25
Status: implemented
Branch / worktree: `feat/snmp` @ `.worktrees/snmp`
Follows: `docs/plans/2026-08-25-snmp-integration-design.md`, `docs/plans/2026-08-25-snmp-integration.md`

## Problem

A review of the shipped `feat/snmp` implementation found nine defects plus four
plan-vs-implementation gaps. The agent works — `snmpwalk` returns the system
group and `ifTable`, `snmpset` is correctly inert — but the branch is not
mergeable:

- `cargo clippy -p snmp-agent --all-targets -- -D warnings` fails.
- `cargo test -p onvif-rust` was killed by `SIGHUP` on a cold run.
- Unit tests were green while `ifOperStatus` was hardcoded to `up`.

That last point is the theme. The bugs that survived were the ones with no seam
to test through.

## Goals

1. Green quality gates, including `--all-targets` clippy.
2. No test that can kill its own test binary.
3. RFC 3416 varbind semantics for GET / GETNEXT.
4. `ifTable` and `sysUpTime` from real kernel data.
5. A verification shape that would have failed on every finding above.

## Non-goals

Unchanged from the original design: no traps, no SET, no SNMPv3, no private MIB,
no `ifSpeed` (the sysfs file returns `EINVAL` on wireless and loopback, and this
camera is wifi-primary — reporting `0`/unknown is the honest answer).

`sysObjectID` stays `.1.3.6.1.4.1.0.1`. Enterprise `0` is reserved, but there is
no correct value without a registered PEN; substituting a different squat would
trade one placeholder for another. The comment gets sharpened instead.

## Findings addressed

| # | Finding | Severity |
| --- | --- | --- |
| 1 | `clippy --all-targets` fails: `MutexGuard` held across `await` | blocking |
| 2 | `test_config_path_override_and_sighup_agent` SIGHUPs its own PID, killing the onvif-rust test binary | blocking |
| 3 | `sighup_agent` will `kill(0, …)` (process group) or `kill(-1, …)` (every process, as root) from a corrupt pidfile | blocking |
| 4 | `ifOperStatus` hardcoded `up(1)`; MAC / MTU / ifType / ifAdminStatus stubbed | correctness |
| 5 | GET of an unknown OID fails the whole PDU with v1 `noSuchName` instead of a per-varbind `noSuchObject` | correctness |
| 6 | Agent answers inbound `GetResponse`, enabling a spoofed-source packet loop | correctness |
| 7 | `Counter32`/`Gauge32`/`TimeTicks` encoded signed; `0xFFFF_FFFF` goes on the wire as `255` | correctness |
| 8 | `sysUpTime` is process uptime, so a supervised restart looks like a device reboot | correctness |
| 9 | `checked_shl(7)` does not detect base-128 value overflow | minor |
| — | No WebUI test asserts the SNMP card renders or saves | gap |
| — | Community string reaches the HTTP log; `sanitize_soap_body` masks XML only | gap |
| — | Bind failure retries only on SIGHUP, contrary to the design's backoff | gap |
| — | Config struct duplicated across crates with no drift guard | ponytail |
| — | `rt-multi-thread`, `Arc<RwLock>`, a test that tests nothing, dead bundle lines | ponytail |

## Architecture: the snapshot seam

Findings 4 and 8, the ignored `_sources` parameter, the repeated `/proc` reads,
and the pointless lock are one problem. `MibSources` becomes the only thing the
MIB layer can see; nothing under `mib/` touches the filesystem.

```rust
pub trait MibSources {
    fn config(&self) -> &SnmpConfig;
    fn uptime_ticks(&self) -> u32;
    fn interfaces(&self) -> &[IfRow];   // new
}

pub struct Agent { config: SnmpConfig, proc_root: PathBuf, sys_root: PathBuf }

pub struct Snapshot { config: SnmpConfig, uptime_ticks: u32, ifaces: Vec<IfRow> }
impl MibSources for Snapshot { /* three getters, no IO */ }
```

`handle_datagram` takes `&Agent` and captures one `Snapshot` per datagram.

Two consequences beyond testability. A 30-varbind walk now observes one instant
instead of thirty, so counters cannot move mid-walk. And `LiveSources` /
`Arc<RwLock<…>>` disappear: the loop is single-tasked and nothing was ever
shared.

### `IfRow`

```rust
pub struct IfRow {
    pub index: u32,            // /sys/class/net/<n>/ifindex, positional fallback
    pub descr: String,
    pub if_type: i32,          // ARPHRD: 1 -> 6 ether, 772 -> 24 loopback, else 1
    pub mtu: i32,
    pub phys_address: Vec<u8>, // "aa:bb:.." -> 6 bytes, empty if unreadable
    pub admin_status: i32,     // flags & IFF_UP -> up(1) / down(2)
    pub oper_status: i32,      // up1 down2 testing3 unknown4 dormant5 …
    pub in_octets: u32,
    pub out_octets: u32,
}
```

Rows sort by kernel ifindex, so an NMS keyed on `ifIndex` stays stable when
`wlan0` drops out and returns. Every sysfs read degrades to a documented default
rather than failing — missing file means `oper_status: 4` (unknown), empty MAC,
positional index. The original error table already required "skip iface or
`genErr`; never panic".

`sysUpTime` moves to `/proc/uptime`, falling back to process start if unreadable.

## Protocol conformance

**Exception values.** `SnmpValue` gains `NoSuchObject` (`[0]` 0x80),
`NoSuchInstance` (`[1]` 0x81), `EndOfMibView` (`[2]` 0x82) — context-tagged and
zero-length. `handle_varbinds` deletes `ERR_NO_SUCH_NAME` (an SNMPv1 code) and
always answers `noError` for GET / GETNEXT, putting the exception in the
offending varbind:

```rust
GetRequest     => resolve_get(..).unwrap_or_else(|| (name, miss_kind(&vb.name))),
GetNextRequest => resolve_get_next(..).unwrap_or((name, EndOfMibView)),
```

`miss_kind` returns `NoSuchInstance` when the OID sits under a group we serve
(`…2.1.1`, `…2.1.2`) and `NoSuchObject` otherwise. A poller batching ten OIDs
now gets nine values and one exception instead of losing all ten.

**PDU-type filter.** One guard in `handle_datagram`, ahead of the community
check:

```rust
if !matches!(pdu_type, GetRequest | GetNextRequest | SetRequest) { return None; }
```

`GetResponse` stays in the enum because we emit it, but is never accepted. A
packet with the source spoofed to the camera's own `ip:161` no longer bounces
forever against a 400 MHz core. A spoofed `SetRequest` draws exactly one
`notWritable` reply, which the filter drops on arrival.

**Unsigned application types.**

```rust
fn encode_unsigned(v: u32) -> Vec<u8> {
    let mut b = v.to_be_bytes().to_vec();
    while b.len() > 1 && b[0] == 0 { b.remove(0); }
    if b[0] & 0x80 != 0 { b.insert(0, 0); }  // stay non-negative, as net-snmp does
    b
}
```

The current path strips leading `0xff`, which is why `Counter32(0xFFFF_FFFF)`
encodes to one byte and reads back as 255. The paired decode change accepts the
five-byte leading-zero form that real agents emit.

**Base-128 overflow.** `checked_shl(7)` becomes `checked_mul(128)`.
`checked_shl` only rejects shift counts ≥ 32; it never noticed bits leaving the
top of the word.

## Reload without signals

The clippy failure and the SIGHUP flake share a root cause: `run()` installs the
signal handler itself, so testing reload meant signalling the whole process,
which forced the `Mutex` serialization clippy rejects and killed the onvif-rust
suite when the snmp test won the race against tokio's handler registration.

```rust
pub async fn run(config_path: PathBuf, pidfile: PathBuf, mut reload: mpsc::Receiver<()>)

// main.rs owns the signal
let mut sighup = signal(SignalKind::hangup())?;
let (tx, rx) = mpsc::channel(1);
tokio::spawn(async move { while sighup.recv().await.is_some() { let _ = tx.try_send(()); } });
```

Tests send on the channel. No `kill` subprocess, no process-wide side effect, no
test lock, no clippy error. The onvif-rust self-SIGHUP case is deleted; its
ESRCH and bad-parse siblings stay and gain a `pid <= 1` case.

`sighup_agent` grows that guard:

```rust
if pid <= 1 { return Ok(()); }   // 0 = our process group, -1 = every process
```

A truncated pidfile is not hypothetical here — half-written files on exFAT after
a power cut are a known hazard on this hardware, and onvif-rust runs as root.

## Remaining gaps

- **WebUI:** three `NetworkPage` tests — card renders fetched values, submit
  calls `putSnmpConfig` with the right payload, empty community blocks with the
  zod message.
- **Log redaction:** `sanitize_soap_body` learns a JSON `"community":"…"` mask
  beside its XML password masks. It already runs on request and response bodies,
  so one edit closes both directions.
- **Bind retry:** a fixed 30 s retry arm in the `select!`, with a `ponytail:`
  comment naming the fixed interval as the ceiling.
- **Config drift:** a key-parity test in `onvif-rust/src/config/snmp.rs`,
  mirroring what `config/netoverlay.rs` already does for its twin in
  `anyka-init`. Keeping both structs avoids a workspace dependency from an ONVIF
  server onto an SNMP agent crate.
- **Ponytail:** `flavor = "current_thread"`; delete
  `test_udp_get_sysname_ephemeral` (it binds its own socket and calls
  `handle_datagram` by hand, exercising nothing the real server test does not);
  drop the three dead `snmp.toml` lines from `build_bundle.sh`, which stage into
  `slots/<x>/snmp.toml` where nothing reads them.

## Testing

`tests/walk.rs` drives a real agent over loopback and asserts:

- strictly ascending OIDs across a full walk
- `endOfMibView` termination
- `noSuchObject` on an unknown OID, with `error-status = noError`
- `notWritable` on SET, and state unchanged
- silence for an inbound `GetResponse`

Self-contained — no `net-snmp` dependency, so it gates CI.

`interfaces.rs` gains a fixture sysfs tree (tempdir with
`eth0/{ifindex,operstate,address,mtu,type,flags}`), making the down-interface
case a unit test rather than something only a live walk catches.

## Success criteria

1. `cargo clippy --all-targets -- -D warnings` green for `snmp-agent`,
   `onvif-rust`, `anyka-init`.
2. `cargo test -p onvif-rust` survives ten consecutive cold runs.
3. `snmpwalk` of `1.3.6.1.2.1.2` reports a down interface as `down(2)`.
4. A `GetResponse` sent to the agent draws no reply.
5. WebUI lint, type-check, and tests green with real SNMP coverage.

## Verification (2026-08-25)

Host gates: fmt/clippy/tests green for `snmp-agent` / `onvif-rust` / `anyka-init`;
ten cold `onvif-rust` runs `EXIT=0 sighup=0`; WebUI lint/type-check/1018 tests/
prettier green; ARM `snmp-agent` is EABI5 uClibc-stripped.

Host `snmpwalk` (127.0.0.1:16161): system group + `ifOperStatus` matched kernel
`operstate`; `snmpset` returned `notWritable` and left `sysName` unchanged.

Device (192.168.2.198): deployed release `snmp-agent.bin` over nc and restarted.
`ifOperStatus` matched sysfs (`lo=unknown(4)`, `wlan0=up(1)`, `p2p0=down(2)`).
`sysUpTime` stayed ~6h across an agent kill/respawn (reads `/proc/uptime`).
