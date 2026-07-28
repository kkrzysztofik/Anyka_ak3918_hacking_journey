# Vendor-Daemon Restart Resilience — Design

Date: 2026-07-29
Status: approved (design), implementation plan pending

## Problem

`onvif-rust` and `vendor-daemon` are separate processes joined by three channels:
a control socket, two frame-notification sockets, and a shared-memory frame ring.
Neither process can tell that the other has restarted. Today a `vendor-daemon`
restart leaves `onvif-rust` permanently black: it holds two dead frame sockets, a
stale mmap, and — worst — SDK handles that are raw pointers into a process that no
longer exists.

### Scope

| # | Requirement |
|---|---|
| R1 | Survive `vendor-daemon` restart while `onvif-rust` lives, **and** `onvif-rust` restart while `vendor-daemon` lives |
| R2 | RTSP sessions are dropped on peer loss; clients re-SETUP. No transparent resume |
| R3 | Both C and Rust may change; lockstep protocol bump is safe (both ship in one SD image) |
| R4 | Process respawn is **out of scope** — assumed handled externally |
| R5 | `onvif-rust` boots degraded without `vendor-daemon`; cold start and recovery share one code path |

## Findings that drive the design

**1. The ring carries no evidence of a restart.**
`vd_ring_create()` opens the ring with `O_CREAT|O_RDWR` — no `O_TRUNC`, no `unlink`
first (`vd_ring_buffer.h:228`). Same inode every restart. It then `memset`s the
header, rewrites an *identical* magic and version, and resets
`write_seq=0, read_seq=0, flags=0`. The client's existing mmap stays valid and
points at the same physical pages, silently re-zeroed underneath it. No SIGBUS, no
magic change, no error — and `flags=0` even clears the shutdown bit that
`shm_is_shutdown()` reads.

Consequence: an explicit epoch is the *only* possible signal. Approaches based on
detecting a changed magic or a failed mapping cannot work.

**2. SDK handles are raw pointers marshalled across a process boundary.**
`handle_vi_open()` returns `ak_vi_open()`'s pointer as a `u64`
(`handlers_vi.c:59-64`); `req_read_handle()` casts it straight back. There is no
session tag. After a daemon restart the client's handles are addresses in a dead
process, and sending them feeds garbage pointers into the vendor SDK — worse than
a clean failure.

**3. `AnykaIpc` is already shaped for in-place re-establishment.**
It is already `Arc`-shared five ways (`platform/anyka/mod.rs:161-185`), and every
resource it owns is already `Option` behind a lock: `frame_main_stream`,
`frame_sub_stream` (`Mutex<Option<UnixStream>>`) and `shm_reader`
(`Mutex<Option<ShmRingReader>>`). The control stream is owned by a dedicated
thread that already reconnects (`ipc/mod.rs:756`).

**4. Handle cleanup on the client side is hollow.**
Four of six handle Drops are no-ops in IPC mode (`video.rs:145,339`;
`audio.rs:71,156`), so `onvif-rust` never sends CLOSE and `vendor-daemon` leaks
VI/VENC on every client restart. Making Drop send IPC is a trap — it would block
on a socket, and it would never run at all under SIGKILL.

**5. The daemon is already partly ready for the reverse direction.**
On control-client hangup it calls `stop_push_slot(0/1)` + `release_control()`
(`main.c:499-505`). It just never closes VI/VENC.

## Approach

Chosen: **epoch-gated in-place re-attach**, driven by a single supervisor.

Rejected alternatives:

- *Per-channel independent reconnect* — dead on arrival given finding 1 (nothing
  to detect on the ring), and three independent state machines can disagree about
  which daemon generation they are talking to.
- *Reset-on-connect with no epoch* — relies entirely on I/O errors, so it is blind
  while idle: with push stopped and no RTSP client there is no frame traffic and
  no EOF to notice. The camera sits streaming nothing until something pokes a
  socket.
- *Moving SDK handle ownership into `AnykaIpc`* — priced at ~35 non-test
  `as_ptr()` call sites including the per-frame hot path
  (`video_encoder.rs:508`), 5 storage fields, and an inversion of the
  `hal/`→`platform/` layering. It buys a guarantee that is not obtainable anyway:
  the handles are `*mut c_void` minted in another process, so no ownership
  arrangement can make the type system know the pointer is live.

### Key idea

Invalidating the handles does not require **owning** them — it requires owning the
only road they can travel. `AnykaIpc` is the sole path to the daemon: all 39 HAL
methods are `impl …HalTrait for AnykaIpc` (`ipc/video.rs:19`, `audio.rs:14`,
`imaging.rs:20`), and every one funnels through `request_async` /
`request_blocking`. One epoch check there makes every outstanding handle inert
simultaneously, with no handle-struct change and no call-site churn.

## Components

### C side — `vendor-daemon`

**Epoch generation.** `struct vd_ring_header` has 16 spare bytes of `_padding`
inside its fixed 64-byte size (`vd_ring_buffer.h:113`). Claim 4 of them for
`uint32_t epoch`, bump `VD_SHM_VERSION` to 3. `vd_ring_create()` stamps a
boot-random non-zero epoch. Size is unchanged, so the `_Static_assert` stands.

**`CMD_HELLO`.** New control command returning `{epoch, version}`. This is the
client's attach handshake and the authoritative epoch source; the ring copy exists
so the client can poll while idle without a round-trip.

**Stale-command rejection.** The dispatcher rejects any command carrying a handle
whose epoch does not match the current one, returning a distinct status. Defence
in depth: the client already refuses to send, so this only catches bugs and
version skew.

**SDK self-reset on control-client change.** Extend the existing
`release_control()` + `stop_push_slot()` path (`main.c:499-505`) to close all
VI/VENC objects opened by the departing session. This covers every way the client
can vanish, including SIGKILL where no client-side Drop would run. Placing the
reset here rather than in client Drops is what makes R1's reverse direction work.

### Rust side — `onvif-rust`

**Epoch gate.** `AnykaIpc` gains `attached_epoch: AtomicU32`. `request_async` and
`request_blocking` compare it against the ring's epoch before writing a byte;
mismatch returns `HardwareUnavailable` immediately. The existing one-shot
`reconnect_and_retry` (`ipc/mod.rs:756`) is removed — a reconnect that silently
resumes with stale handles is precisely the hazard.

**Attach / detach on `AnykaIpc`.** Two new methods that swap the contents of the
three existing `Mutex<Option<_>>` fields and set/clear `attached_epoch`. The
struct keeps its identity, so the five `Arc<dyn …HalTrait>` clones held by
`AnykaVideoInput`, `AnykaVideoEncoder`, `AnykaAudioInput`, `AnykaAudioEncoder` and
`AnykaImagingControl` never learn anything changed. No new delegating wrapper, no
39 forwarding methods.

**Supervisor task.** One task, the **sole** constructor of an attachment. Owns the
attach state machine and publishes availability over a `watch` channel. Detection
sites — the ctrl owner thread, the frame reader, the epoch poller — only *report*;
they never attach. This is an invariant, not a convention: the daemon's
single-owner guards (`dispatcher.c:67`, `main.c:328`) *reject* concurrent
attachers rather than serialising them, so two simultaneous attaches produce a
half-attached mess.

**Degraded boot.** `AnykaIpc::new()` stops hard-failing when the frame sockets or
ring are absent (`ipc/mod.rs:479-511`), and the `pidof vendor-daemon.bin` guard in
`run_onvif_rust.sh:67` is removed. Cold start becomes "attach attempt #1", so
every boot exercises the recovery path.

## Data flow

### Attach (identical for cold start and recovery)

The daemon creates the ring first, then ctrl, then frame-main, then frame-sub
(`main.c:267,280,288,296`). Attaching in reverse order gives a free readiness
barrier: if frame-sub connects, everything else already exists.

```
connect frame-sub → connect frame-main → connect ctrl → CMD_HELLO(epoch)
  → open ring, verify ring epoch == HELLO epoch
  → set attached_epoch, publish Available
  → run existing Platform::initialize() (match_sensor → VI_OPEN → VPSS
    → VENC_OPEN → capture → push)
```

### Detection

| Signal | Source | Covers |
|---|---|---|
| ctrl I/O error | owner thread | active control traffic |
| frame socket EOF | frame reader | active streaming |
| ring epoch ≠ `attached_epoch` | poller, ~1 s | **idle** — push stopped, no RTSP client, no traffic to fail |

The poller is what the no-epoch alternative cannot do, and it is a single volatile
`u32` read of an already-mapped page.

### Detach

```
publish Unavailable → RTSP layer drops sessions, media service reports unavailable
  → run existing rollback_video_pipeline()
  → clear the three Mutex<Option<_>> fields, clear attached_epoch
  → back to attach with backoff
```

## Error handling

**Partial-attach rollback.** Attach is not "connect a socket" — it runs
`VI_MATCH_SENSOR → VI_OPEN → VPSS_INIT → VENC_OPEN → VENC_SET_RC → START_PUSH`.
An attempt that gets partway and fails must unwind what it opened, in reverse.
`rollback_video_pipeline()` (`platform/anyka/mod.rs:335`) and
`shutdown_video_pipeline()` already do exactly this and are reused unchanged.

Without rollback, a retry loop does open/close churn against a vendor SDK this
repo already has evidence of wedging: `PUSH_JOIN_TIMEOUT_SEC` exists because "a
thread parked inside a blocking SDK call cannot be interrupted" (`globals.h`), and
commit e3b1af9 is *"never let a wedged push thread swallow SIGTERM"*. Backoff
bounds the retry *rate*; only rollback bounds *cumulative* SDK damage.

**Circuit breaker.** After N consecutive failed attaches the supervisor stops
attaching, stays degraded, and logs loudly. This is not about CPU cost — a
connect attempt per second is noise on this box. It bounds two real failures:

- SDK wedge by repeated re-init, per above.
- Crash-loop amplification once respawn lands (R4, deferred): if attach is what
  kills the daemon, retrying forever means executing each fresh daemon in turn.

**Observability over silence.** A backoff that hides a permanent failure leaves the
camera dark with evidence only in logs. Attach state is surfaced through ONVIF —
R5's degraded boot already requires this — so a black stream reads as
"unavailable" rather than "connected, no frames".

**Not silently recovered:** epoch mismatch mid-request. It fails the request. The
supervisor handles the transition; individual requests do not retry across an
epoch boundary.

## Testing

**Unit — Rust.** The existing `FakeDaemon` harness in `ipc/mod.rs` tests is the
lever. Cases: epoch mismatch rejects before writing; attach is idempotent under
concurrent trigger reports; detach clears all three resources; circuit breaker
opens after N and stays open; partial attach failure rolls back.

**Unit — C.** Epoch is non-zero and differs across `vd_ring_create()` calls;
dispatcher rejects a handle command carrying a stale epoch; `release_control()`
closes VI/VENC.

**Integration.** Both directions of R1 against a real daemon:
`SIGKILL` the daemon mid-stream and confirm re-attach plus a fresh IDR; `SIGKILL`
`onvif-rust` and confirm the daemon resets and accepts the new client. Cold start
with no daemon present, daemon started afterwards. The ring-reuse behaviour
(finding 1) deserves an explicit test: restart the daemon and assert the client
detects it *without* any socket error, i.e. with the process idle.

**Regression risk to watch.** Removing `reconnect_and_retry` changes behaviour for
transient ctrl blips that previously recovered invisibly. That is intended — such
a blip is now either a real restart (epoch differs, full re-attach) or a genuine
I/O error (surfaced) — but the timeout tests that depend on single-attempt
reconnect semantics (`ipc/mod.rs:313-327`) will need revisiting.

## Open items

- Backoff schedule and circuit-breaker threshold N: pick from measured daemon
  restart time on hardware, not from a guess.
- Whether the epoch poll interval (~1 s) is worth making configurable.
- Respawn (R4) is deferred; the circuit breaker is what makes it safe to add later.
