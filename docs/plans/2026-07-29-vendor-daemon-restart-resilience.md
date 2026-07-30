# Vendor-Daemon Restart Resilience Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make `onvif-rust` and `vendor-daemon` survive each other's restart in either order, by stamping an epoch into the shared-memory ring and gating every IPC request on it.

**Architecture:** `vendor-daemon` generates a boot-random non-zero `epoch` at ring creation and reports it via a new `CMD_HELLO`. `onvif-rust` records that epoch at attach time and compares it against the live ring value before every control request, so a daemon restart makes all outstanding SDK handles inert without moving handle ownership. One supervisor task is the sole owner of attach/detach; it drives the same code path for cold start and recovery, with partial-attach rollback and a circuit breaker. On the reverse direction, `vendor-daemon` closes leaked VI/VENC objects when its control client changes.

**Tech Stack:** Rust (tokio, custom vendored toolchain), C99 (uClibc, ARMv5TE cross), POSIX shared memory over a regular file, Unix domain sockets.

**Design doc:** `docs/plans/2026-07-29-vendor-daemon-restart-resilience-design.md` — read it first. It explains *why* an epoch is the only workable signal (the ring file is reused across restarts with identical magic) and why handle ownership is deliberately not moved.

---

## Conventions used throughout this plan

Set these once per shell session:

```bash
cd /home/kmk/dev/anyka-dev
export CARGO=toolchain/arm-anykav200-crosstool-ng/bin/cargo
export HOST=x86_64-unknown-linux-gnu
```

- **All** cargo commands use the vendored toolchain. Never use a system `cargo`.
- Host-side tests always pass `--target $HOST`.
- Run a single test with: `$CARGO test --target $HOST --lib -- <test_name> --exact --nocapture`
- Before every commit: `$CARGO clippy --target $HOST -- -D warnings` and `$CARGO fmt --check`.
- Command output below is shown unfiltered. Long runs are easier to read piped
  through `2>&1 | tail -30`, or whatever summariser you have to hand.

**Field offsets in the ring header** (needed repeatedly, all `u32`):

```
magic 0  version 4  total_size 8  slot_count 12  slot_data_size 16
write_seq 20  read_seq 24  flags 28
overflow 32  eviction 36  socket_fallback 40  dropped 44
epoch 48   <-- new, carved out of _padding
_padding 52..64 (12 bytes remain)
```

---

## Phase 1 — Epoch in the ring header (C side)

### Task 1: Add `epoch` to the C ring header and stamp it at creation

**Files:**
- Modify: `cross-compile/vendor-daemon/include/vd_ring_buffer.h` (struct at :99, `vd_ring_create()` at :221)
- Create: `cross-compile/vendor-daemon/tests/test_ring_epoch.c`

**Step 1: Write the failing test**

The ring header is header-only `static inline`, so it compiles standalone on the host with no cross toolchain.

Create `cross-compile/vendor-daemon/tests/test_ring_epoch.c`:

```c
/* Host-compiled unit test for ring-header epoch generation. */
#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include "vd_ring_buffer.h"

int main(void)
{
    void *base_a;
    void *base_b;
    struct vd_ring_header *hdr;
    uint32_t epoch_a;
    uint32_t epoch_b;

    /* Fresh ring: epoch must be non-zero (0 is reserved for "detached"). */
    base_a = vd_ring_create();
    assert(base_a != NULL);
    hdr = vd_ring_get_header(base_a);
    assert(hdr->magic == VD_SHM_MAGIC);
    assert(hdr->version == VD_SHM_VERSION);
    epoch_a = hdr->epoch;
    assert(epoch_a != 0);

    /* Re-create over the SAME file, as a daemon restart does. The epoch must
     * change, because that is the only evidence of the restart the client has. */
    base_b = vd_ring_create();
    assert(base_b != NULL);
    hdr = vd_ring_get_header(base_b);
    epoch_b = hdr->epoch;
    assert(epoch_b != 0);
    assert(epoch_b != epoch_a);

    /* Header must still be exactly 64 bytes. */
    assert(sizeof(struct vd_ring_header) == 64);

    unlink(VD_SHM_PATH);
    printf("test_ring_epoch: PASS\n");
    return 0;
}
```

**Step 2: Run it to verify it fails**

```bash
gcc -std=c99 -D_GNU_SOURCE -Icross-compile/vendor-daemon/include \
    -o /tmp/test_ring_epoch cross-compile/vendor-daemon/tests/test_ring_epoch.c && /tmp/test_ring_epoch
```

Expected: compile error — `struct vd_ring_header has no member named 'epoch'`.

**Step 3: Implement**

In `vd_ring_buffer.h`, bump the version and add the field. Change:

```c
#define VD_SHM_VERSION     2
```

to:

```c
/* v3 adds the `epoch` field used to detect daemon restarts. */
#define VD_SHM_VERSION     3
```

In `struct vd_ring_header`, replace:

```c
    uint8_t  _padding[16];         /* Reduced from 32: pad to 64 bytes */
```

with:

```c
    /*
     * Daemon generation counter (version >= 3).  Re-randomised on every
     * vd_ring_create(), i.e. on every daemon start.  Never 0 -- 0 is reserved
     * by the client to mean "not attached".
     *
     * This exists because the ring file is REUSED across restarts (O_CREAT
     * without O_TRUNC, same inode) and the magic/version are rewritten
     * identically, so the mapping itself carries no evidence of a restart.
     */
    uint32_t epoch;
    uint8_t  _padding[12];         /* pad to 64 bytes */
```

Add `#include <time.h>` near the other includes if not already present, then in `vd_ring_create()` replace:

```c
    hdr->flags = 0;
```

with:

```c
    hdr->flags = 0;
    hdr->epoch = vd_ring_new_epoch();
```

And add this helper immediately above `vd_ring_create()`:

```c
/**
 * @brief Generate a fresh, non-zero epoch for this daemon generation.
 *
 * Drawn from /dev/urandom.  PID and CLOCK_MONOTONIC are a poor source on this
 * target: a reboot restarts the monotonic clock near zero and hands out the
 * same PIDs in the same boot order, so two generations at the same point in
 * two boots can land very close together.  The epoch's only job is to differ
 * from the previous generation, so it is drawn from real entropy instead.
 *
 * Retries on a zero draw -- 0 is reserved for "detached" on the client side.
 * Clamping to 1 instead would make 0 twice as likely as any other value, which
 * matters here only in that it is free to avoid.
 */
static inline uint32_t vd_ring_new_epoch(void)
{
    uint32_t epoch = 0;
    int fd;

    fd = open("/dev/urandom", O_RDONLY | O_CLOEXEC);
    if (fd >= 0) {
        while (epoch == 0) {
            if (read(fd, &epoch, sizeof(epoch)) != (ssize_t)sizeof(epoch)) {
                epoch = 0;
                break;
            }
        }
        close(fd);
    }

    if (epoch == 0) {
        /* Degraded: no entropy source.  Fall back to the PID/clock mix rather
         * than failing ring creation and taking the camera down for a missing
         * /dev/urandom.  Restart detection still works; only the collision
         * margin is worse.  No logging here on purpose -- this header is
         * compiled standalone by tests/test_ring_epoch.c and must not pull in
         * the daemon's log.h.  vd_ring_create() is the place to log it. */
        struct timespec ts;

        if (clock_gettime(CLOCK_MONOTONIC, &ts) != 0) {
            ts.tv_sec = 0;
            ts.tv_nsec = 0;
        }
        epoch = (uint32_t)getpid() * 2654435761u;
        epoch ^= (uint32_t)ts.tv_nsec;
        epoch ^= (uint32_t)ts.tv_sec << 16;
        if (epoch == 0) {
            epoch = 1u;
        }
    }

    return epoch;
}
```

**Step 4: Run the test to verify it passes**

```bash
gcc -std=c99 -D_GNU_SOURCE -Icross-compile/vendor-daemon/include \
    -o /tmp/test_ring_epoch cross-compile/vendor-daemon/tests/test_ring_epoch.c && /tmp/test_ring_epoch
```

Expected: `test_ring_epoch: PASS`

**Step 5: Verify the daemon still cross-compiles**

```bash
make -C cross-compile/vendor-daemon 2>&1
```

Expected: OK.

**Step 6: Commit**

```bash
git add cross-compile/vendor-daemon/include/vd_ring_buffer.h cross-compile/vendor-daemon/tests/test_ring_epoch.c
git commit -m "feat(vendor-daemon): stamp a per-generation epoch into the ring header

The ring file is reused across restarts with an identical magic rewritten
over it, so the mapping carries no evidence that the daemon is new. The
epoch is the only signal a client can use."
```

---

### Task 2: Mirror the epoch field on the Rust side

**Files:**
- Modify: `cross-compile/onvif-rust/src/hal/anyka/ipc/shm_ring.rs:69` (version const), `:125-159` (`RingHeader`), and add an accessor near `is_shutdown` at `:426`
- Test: same file, `mod tests`

**Step 1: Write the failing test**

Append to the `tests` module in `shm_ring.rs`:

```rust
#[test]
fn epoch_reads_back_the_value_the_daemon_stamped() {
    let reader = create_test_anon_reader();

    // create_test_anon_reader leaves epoch at 0 (freshly zeroed mmap), which is
    // exactly what "no epoch / v2 daemon" looks like.
    assert_eq!(reader.epoch(), 0, "zeroed ring must report epoch 0");

    // Stand in for the daemon stamping a generation.
    // SAFETY: offset 48 is inside the validated 64-byte header.
    unsafe {
        reader.base_ptr_for_test().add(48).cast::<u32>().write_volatile(0xDEAD_BEEF);
    }
    assert_eq!(reader.epoch(), 0xDEAD_BEEF);
}
```

**Step 2: Run it to verify it fails**

```bash
$CARGO test --target $HOST --lib -- epoch_reads_back_the_value_the_daemon_stamped --exact 2>&1
```

Expected: COMPILE_ERROR — no method `epoch`.

**Step 3: Implement**

Bump the version constant:

```rust
/// Version of the shared memory protocol (v3 adds the daemon `epoch`)
pub const VD_SHM_VERSION: u32 = 3;
```

In `RingHeader`, replace the `_padding` field:

```rust
    /// Daemon generation counter (version >= 3); 0 means "no epoch reported".
    pub epoch: u32,
    /// Padding to 64 bytes
    pub _padding: [u8; 12],
```

Add next to `is_shutdown()`:

```rust
    /// Read the daemon generation counter from the ring header (version >= 3).
    ///
    /// Returns 0 for a v1/v2 ring, or for a ring the daemon has not stamped yet —
    /// including the window in which `vd_ring_create()` has memset the header
    /// during a restart. 0 is never a valid generation, and the epoch gate treats
    /// it as a mismatch rather than as "no information".
    ///
    /// Uses `read_volatile`: the daemon rewrites this field concurrently on
    /// restart, and creating a `&u32` to it would violate strict aliasing.
    pub fn epoch(&self) -> u32 {
        // SAFETY: offset 48 is within the validated VD_SHM_HEADER_SIZE (64) region.
        unsafe { self.base.add(48).cast::<u32>().read_volatile() }
    }

    /// Raw base pointer, for tests that stand in for the daemon.
    #[cfg(test)]
    pub(in crate::hal::anyka::ipc) fn base_ptr_for_test(&self) -> *mut u8 {
        self.base
    }
```

**Note on `VD_SHM_VERSION_MIN`:** leave it at 1 — `ShmRingReader::open()` still maps an older ring rather than failing there. But be clear that this does **not** buy pre-v3 compatibility: `hello()` (Task 4) rejects a reported epoch of 0 outright, so a v1/v2 daemon cannot complete the attach handshake and the client stays detached. Pre-v3 daemons are rejected deliberately and explicitly.

This is the whole zero-epoch contract, and it must be read as one rule:

| where | epoch 0 means | behaviour |
| --- | --- | --- |
| `hello()` response | daemon is pre-v3 or misbehaving | reject the attach |
| ring, after a successful attach | daemon is re-creating the ring | refuse the request |
| `attached_epoch` | not attached | refuse the request |

There is deliberately **no** "0 means no information, carry on" path. Allowing one would open the exact hole this design exists to close: `vd_ring_create()` memsets the header on restart, so a v3 client polling mid-restart reads 0, and treating that as permissive would let handles from the dead generation through at the worst possible moment. Since `finish_attach` seeds `observed_epoch` before `attached_epoch`, there is no startup window where 0 is legitimate either.

**Step 4: Run the test to verify it passes**

```bash
$CARGO test --target $HOST --lib -- epoch_reads_back_the_value_the_daemon_stamped --exact 2>&1
```

Expected: PASS.

**Step 5: Run the whole shm_ring suite — the struct changed**

```bash
$CARGO test --target $HOST --lib -- shm_ring 2>&1
```

Expected: all pass. If any test asserts on `_padding` length or `VD_SHM_VERSION == 2`, update it — the version bump is intentional.

**Step 6: Commit**

```bash
git add cross-compile/onvif-rust/src/hal/anyka/ipc/shm_ring.rs
git commit -m "feat(ipc): read the vendor-daemon epoch from the ring header"
```

---

## Phase 2 — `CMD_HELLO` handshake

### Task 3: Add `CMD_HELLO` to the daemon

**Files:**
- Modify: `cross-compile/vendor-daemon/src/protocol.h:27` (enum), `cross-compile/vendor-daemon/src/dispatcher.c` (dispatch table)

**Step 1: Implement**

There is no C unit-test harness for the dispatcher, so this task is verified by the integration test in Task 14 and by the Rust `FakeDaemon` test in Task 4. Keep it small.

In `protocol.h`, after the error commands (200-201), add:

```c
    /* ---- Session ---------------------------------------------------------
     * CMD_HELLO is the client's attach handshake.  It is the only command a
     * client may send before the epoch gate is satisfied, so it must never
     * require an existing session.
     * Response: [u32 epoch][u32 shm_version] = 8 bytes.
     */
    CMD_HELLO                     = 300,
```

In `dispatcher.c`, add a handler and wire it into the dispatch switch:

```c
/*
 * handle_hello - IPC handler for CMD_HELLO.
 *
 * Reports this daemon generation's epoch and the shm protocol version so the
 * client can pin them for the lifetime of its attachment.
 *
 * Response: [u32 epoch][u32 shm_version]
 */
static int handle_hello(int fd)
{
    uint8_t resp[8];
    uint32_t epoch = 0;
    uint32_t version = VD_SHM_VERSION;

    if (g_ring_buffer != NULL) {
        epoch = vd_ring_get_header(g_ring_buffer)->epoch;
    }

    memcpy(&resp[0], &epoch, 4);
    memcpy(&resp[4], &version, 4);
    return send_response(fd, AK_SUCCESS, resp, sizeof(resp));
}
```

Dispatch it **before** the `acquire_control()` check at `dispatcher.c:176` — a client must be able to say hello without owning control, otherwise a second client can never discover that it lost the race.

**Step 2: Verify it builds**

```bash
make -C cross-compile/vendor-daemon 2>&1
```

Expected: OK.

**Step 3: Commit**

```bash
git add cross-compile/vendor-daemon/src/protocol.h cross-compile/vendor-daemon/src/dispatcher.c
git commit -m "feat(vendor-daemon): add CMD_HELLO reporting epoch and shm version"
```

---

### Task 4: Client-side `hello()`

**Files:**
- Modify: `cross-compile/onvif-rust/src/hal/anyka/ipc/mod.rs` (constants near `:276`, method near `:885`)
- Test: same file, `mod tests`

**Step 1: Write the failing test**

```rust
#[tokio::test]
async fn hello_parses_epoch_and_version_from_the_daemon() {
    let daemon = test_helpers::FakeDaemon::start(|cmd_id, _req| {
        if cmd_id == CMD_HELLO {
            let mut resp = Vec::with_capacity(8);
            resp.extend_from_slice(&0x1234_5678u32.to_le_bytes());
            resp.extend_from_slice(&3u32.to_le_bytes());
            (AK_SUCCESS_I32, resp)
        } else {
            (AK_FAILED_I32, vec![])
        }
    });
    let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();

    let (epoch, version) = ipc.hello().await.unwrap();

    assert_eq!(epoch, 0x1234_5678);
    assert_eq!(version, 3);
}

#[tokio::test]
async fn hello_rejects_a_zero_epoch() {
    // A daemon that reports epoch 0 is either pre-v3 or broken. Either way we
    // must not pin 0, because 0 is our own "detached" sentinel.
    let daemon = test_helpers::FakeDaemon::start(|_cmd_id, _req| {
        let mut resp = Vec::with_capacity(8);
        resp.extend_from_slice(&0u32.to_le_bytes());
        resp.extend_from_slice(&3u32.to_le_bytes());
        (AK_SUCCESS_I32, resp)
    });
    let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();

    let err = ipc.hello().await.unwrap_err();

    assert!(
        matches!(err, PlatformError::HardwareUnavailable(_)),
        "expected HardwareUnavailable, got {err:?}"
    );
}
```

**Step 2: Run to verify it fails**

```bash
$CARGO test --target $HOST --lib -- hello_ 2>&1
```

Expected: COMPILE_ERROR — no `CMD_HELLO`, no `hello`.

**Step 3: Implement**

Add near the other command constants:

```rust
/// Attach handshake. Returns `[u32 epoch][u32 shm_version]`.
///
/// The only command exempt from the epoch gate — it is how the epoch is learned.
const CMD_HELLO: i32 = 300;

/// Sentinel meaning "not attached to any daemon generation".
///
/// The daemon guarantees a non-zero epoch, so 0 can never collide with a real one.
const EPOCH_DETACHED: u32 = 0;
```

Add `"HELLO"` to `cmd_name()`. Then:

```rust
    /// Perform the attach handshake and learn this daemon generation's epoch.
    ///
    /// Returns `(epoch, shm_version)`. Exempt from the epoch gate by construction:
    /// `epoch_gate` short-circuits on `CMD_HELLO`.
    pub(crate) async fn hello(&self) -> PlatformResult<(u32, u32)> {
        let (status, resp) = self.request_async(CMD_HELLO, &[]).await?;
        if status != AK_SUCCESS_I32 {
            return Err(PlatformError::HardwareUnavailable(format!(
                "vendor daemon rejected CMD_HELLO with status {status}"
            )));
        }
        if resp.len() < 8 {
            return Err(PlatformError::HardwareFailure(format!(
                "CMD_HELLO response too short: {} bytes (want 8)",
                resp.len()
            )));
        }
        let epoch = u32::from_le_bytes([resp[0], resp[1], resp[2], resp[3]]);
        let version = u32::from_le_bytes([resp[4], resp[5], resp[6], resp[7]]);
        if epoch == EPOCH_DETACHED {
            return Err(PlatformError::HardwareUnavailable(
                "vendor daemon reported epoch 0; it is pre-v3 or misbehaving".to_string(),
            ));
        }
        Ok((epoch, version))
    }
```

**Step 4: Run to verify it passes**

```bash
$CARGO test --target $HOST --lib -- hello_ 2>&1
```

Expected: both PASS.

**Step 5: Commit**

```bash
git add cross-compile/onvif-rust/src/hal/anyka/ipc/mod.rs
git commit -m "feat(ipc): add CMD_HELLO attach handshake"
```

---

## Phase 3 — The epoch gate

### Task 5: Gate every request on the epoch

This is the core of the design: it makes stale SDK handles inert without moving handle ownership.

**Files:**
- Modify: `cross-compile/onvif-rust/src/hal/anyka/ipc/mod.rs` — struct at `:396`, constructors at `:468/:528/:562`, `request_async` at `:890`, `request_blocking` at `:912`

**Step 1: Write the failing test**

```rust
#[tokio::test]
async fn request_is_refused_without_writing_when_the_epoch_moved() {
    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
    use std::sync::Arc as StdArc;

    let seen = StdArc::new(AtomicUsize::new(0));
    let seen_in_daemon = StdArc::clone(&seen);
    let daemon = test_helpers::FakeDaemon::start(move |_cmd_id, _req| {
        seen_in_daemon.fetch_add(1, AtomicOrdering::SeqCst);
        (AK_SUCCESS_I32, vec![])
    });
    let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();

    // Attached to generation 7, but the ring now reports generation 8.
    ipc.set_epochs_for_test(7, 8);

    let err = ipc.request_async(CMD_VI_CLOSE, &[0u8; 8]).await.unwrap_err();

    assert!(matches!(err, PlatformError::HardwareUnavailable(_)));
    assert_eq!(
        seen.load(AtomicOrdering::SeqCst),
        0,
        "a stale handle must never reach the daemon"
    );
}

#[tokio::test]
async fn request_is_refused_when_detached() {
    let daemon = test_helpers::FakeDaemon::start(|_c, _r| (AK_SUCCESS_I32, vec![]));
    let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
    ipc.set_epochs_for_test(EPOCH_DETACHED, 0);

    let err = ipc.request_async(CMD_VI_CLOSE, &[0u8; 8]).await.unwrap_err();

    assert!(matches!(err, PlatformError::HardwareUnavailable(_)));
}

#[tokio::test]
async fn request_passes_when_epochs_agree() {
    let daemon = test_helpers::FakeDaemon::start(|_c, _r| (AK_SUCCESS_I32, vec![]));
    let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
    ipc.set_epochs_for_test(7, 7);

    let (status, _) = ipc.request_async(CMD_VI_CLOSE, &[0u8; 8]).await.unwrap();

    assert_eq!(status, AK_SUCCESS_I32);
}

#[tokio::test]
async fn observed_epoch_zero_does_not_block_requests() {
    // A v2 daemon reports no epoch. That is a version skew, not an outage:
    // degrade to today's behaviour rather than refusing everything.
    let daemon = test_helpers::FakeDaemon::start(|_c, _r| (AK_SUCCESS_I32, vec![]));
    let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
    ipc.set_epochs_for_test(7, 0);

    let (status, _) = ipc.request_async(CMD_VI_CLOSE, &[0u8; 8]).await.unwrap();

    assert_eq!(status, AK_SUCCESS_I32);
}

#[test]
fn hello_is_exempt_from_the_gate() {
    let daemon = test_helpers::FakeDaemon::start(|_c, _r| {
        let mut resp = Vec::new();
        resp.extend_from_slice(&9u32.to_le_bytes());
        resp.extend_from_slice(&3u32.to_le_bytes());
        (AK_SUCCESS_I32, resp)
    });
    let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
    ipc.set_epochs_for_test(EPOCH_DETACHED, 0);

    // Must succeed while detached, or attach could never happen.
    let (status, _) = ipc.request_blocking(CMD_HELLO, &[]).unwrap();

    assert_eq!(status, AK_SUCCESS_I32);
}
```

**Step 2: Run to verify it fails**

```bash
$CARGO test --target $HOST --lib -- epoch 2>&1
```

Expected: COMPILE_ERROR — no `set_epochs_for_test`.

**Step 3: Implement**

Add two fields to `AnykaIpc`:

```rust
    /// Daemon generation this client attached to, or [`EPOCH_DETACHED`].
    ///
    /// Set once by `attach`, cleared by `detach`. Every outstanding SDK handle is
    /// implicitly tagged with this value: the handles are raw pointers minted inside
    /// the daemon process, so they are only meaningful for the generation that minted
    /// them.
    attached_epoch: AtomicU32,
    /// Latest epoch observed in the ring header, refreshed by the supervisor's poller.
    ///
    /// Kept as an atomic rather than read from the mmap on demand so the request path
    /// never contends with the frame reader for the `shm_reader` mutex.
    /// 0 means the ring is not stamped — while detached, or because the daemon is
    /// re-creating it. Never a usable generation: the gate refuses on 0 like any
    /// other mismatch.
    observed_epoch: AtomicU32,
```

Initialise both to `EPOCH_DETACHED` in all three constructors (`new`, `new_with_path`, `from_parts_for_test`).

Add the gate and the test seam:

```rust
    /// Refuse a request whose SDK handles belong to a dead daemon generation.
    ///
    /// This is the single chokepoint every handle must pass through: all 39 HAL
    /// trait methods are implemented on `AnykaIpc` and funnel into `request_async` /
    /// `request_blocking`. Checking here makes every outstanding handle inert at
    /// once, which is why handle ownership does not need to move.
    fn epoch_gate(&self, cmd_id: i32) -> PlatformResult<()> {
        if cmd_id == CMD_HELLO {
            return Ok(());
        }
        let attached = self.attached_epoch.load(Ordering::Acquire);
        if attached == EPOCH_DETACHED {
            return Err(PlatformError::HardwareUnavailable(
                "not attached to a vendor daemon".to_string(),
            ));
        }
        let observed = self.observed_epoch.load(Ordering::Acquire);
        // No exemption for observed == 0. Once attached, `attached` is a non-zero
        // v3 epoch (hello() rejects 0) and finish_attach seeds `observed` before
        // `attached`, so there is no legitimate "not yet polled" window. A zero
        // read therefore means the daemon is re-creating the ring right now —
        // vd_ring_create() memsets the header — which is precisely when stale
        // handles must be refused, not waved through.
        if observed != attached {
            return Err(PlatformError::HardwareUnavailable(format!(
                "vendor daemon restarted (attached epoch {attached}, observed {observed}); \
                 handles from the previous generation are stale"
            )));
        }
        Ok(())
    }

    /// Force both epochs, standing in for attach and the poller during tests.
    #[cfg(test)]
    pub(crate) fn set_epochs_for_test(&self, attached: u32, observed: u32) {
        self.attached_epoch.store(attached, Ordering::Release);
        self.observed_epoch.store(observed, Ordering::Release);
    }
```

Call `self.epoch_gate(cmd_id)?;` as the **first** line of both `request_async` and `request_blocking`, before the `job_tx` lookup.

Add `use std::sync::atomic::AtomicU32;` to the imports.

**Step 4: Run to verify it passes**

```bash
$CARGO test --target $HOST --lib -- epoch 2>&1
```

Expected: all PASS.

**Step 5: Run the full suite — this gate touches every IPC caller**

```bash
$CARGO test --target $HOST 2>&1
```

Expected: **failures.** Existing tests construct `AnykaIpc` and issue requests without attaching, so the gate now refuses them. Fix each by calling `set_epochs_for_test(1, 1)` after construction. Do **not** weaken the gate to make tests pass — the tests are asserting the old, unsafe behaviour.

**Step 6: Commit**

```bash
git add cross-compile/onvif-rust/src/hal/anyka/ipc/mod.rs
git commit -m "feat(ipc): refuse requests carrying handles from a dead daemon generation

The SDK handles are raw pointers minted inside vendor-daemon. After a restart
they address freed memory in a dead process, and forwarding them feeds garbage
into the vendor SDK. Gating at the single request chokepoint invalidates all of
them at once without moving handle ownership through ~35 call sites."
```

---

### Task 6: Remove the one-shot reconnect-and-retry

**Files:**
- Modify: `cross-compile/onvif-rust/src/hal/anyka/ipc/mod.rs:313-353` (`CtrlConnect`), `:708-750` (`process_job`), `:756-806` (`reconnect_and_retry`)

**Why:** a reconnect that silently resumes with the same handles is precisely the hazard the epoch gate exists to prevent. After this task, a control-socket I/O error is reported to the supervisor, which re-attaches deliberately.

**Step 1: Write the failing test**

```rust
#[tokio::test]
async fn ctrl_io_error_does_not_silently_reconnect() {
    // A daemon that accepts, then hangs up. The old code reconnected to the
    // production socket and retried; that must no longer happen.
    let daemon = test_helpers::FakeDaemon::start_then_hangup();
    let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
    ipc.set_epochs_for_test(1, 1);

    let err = ipc.request_async(CMD_VI_CLOSE, &[0u8; 8]).await.unwrap_err();

    assert!(
        matches!(err, PlatformError::HardwareFailure(_) | PlatformError::Timeout),
        "expected the I/O error to surface, got {err:?}"
    );
}
```

Add the helper to `test_helpers`:

```rust
    /// Spawns a daemon that accepts one connection then immediately closes it,
    /// simulating a daemon that died mid-request.
    pub fn start_then_hangup() -> Self {
        let counter = TEST_DAEMON_COUNTER.fetch_add(1, Ordering::SeqCst);
        let socket_path = format!(
            "/tmp/test-vendor-daemon-hangup-{}-{}.sock",
            std::process::id(),
            counter
        );
        let _ = std::fs::remove_file(&socket_path);
        let listener = UnixListener::bind(&socket_path).unwrap();
        let path_clone = socket_path.clone();
        let handle = std::thread::spawn(move || {
            if let Ok((stream, _)) = listener.accept() {
                drop(stream);
            }
            let _ = std::fs::remove_file(&path_clone);
        });
        std::thread::sleep(std::time::Duration::from_millis(50));
        Self {
            socket_path,
            _listener_thread: handle,
        }
    }
```

**Step 2: Run to verify it fails**

```bash
$CARGO test --target $HOST --lib -- ctrl_io_error_does_not_silently_reconnect --exact 2>&1
```

Expected: COMPILE_ERROR, then FAIL once the helper exists (the current code reconnects).

**Step 3: Implement**

- Delete `reconnect_and_retry` entirely.
- In `process_job`, replace the `if let Err(ref e) = result { … reconnect_and_retry … }` block with a warn-and-report:

```rust
        if let Err(ref e) = result {
            warn!(
                cmd_id = job.cmd_id,
                cmd_name,
                elapsed_ms = started.elapsed().as_millis(),
                error = %e,
                "IPC request failed; reporting peer loss to the supervisor"
            );
            // Do NOT reconnect here. A reconnect that resumes with the same
            // handles is exactly the hazard the epoch gate exists to prevent;
            // re-attaching is the supervisor's job and only its job.
        }
```

- Simplify `CtrlConnect`: `reconnect()` and the doc comment about production-path reconnects go away. `connect()` stays.

**Step 4: Run to verify it passes**

```bash
$CARGO test --target $HOST --lib -- ctrl_io_error 2>&1
```

Expected: PASS.

**Step 5: Fix the timeout tests that depended on reconnect semantics**

```bash
$CARGO test --target $HOST --lib -- timeout 2>&1
```

The tests at `:2309`, `:2353`, `:2399`, `:2428` use `start_with_delay` and assert single-attempt timing. Their comment at `:313-327` explains they relied on reconnects failing fast against absent production paths. That reasoning is now moot; the timing assertions should get simpler, not looser. Update the comments to match reality.

**Step 6: Commit**

```bash
git add cross-compile/onvif-rust/src/hal/anyka/ipc/mod.rs
git commit -m "refactor(ipc): drop the silent one-shot ctrl reconnect

A reconnect that resumes with the previous generation's handles is the hazard
the epoch gate prevents. Re-attaching is now the supervisor's job alone."
```

---

## Phase 4 — Attach and detach

### Task 7: `detach()`

**Files:**
- Modify: `cross-compile/onvif-rust/src/hal/anyka/ipc/mod.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn detach_clears_every_resource_and_the_epoch() {
    let daemon = test_helpers::FakeDaemon::start(|_c, _r| (AK_SUCCESS_I32, vec![]));
    let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
    ipc.set_epochs_for_test(5, 5);

    ipc.detach();

    assert_eq!(ipc.attached_epoch_for_test(), EPOCH_DETACHED);
    assert_eq!(ipc.observed_epoch_for_test(), EPOCH_DETACHED);
    assert!(ipc.frame_main_stream.lock().unwrap().is_none());
    assert!(ipc.frame_sub_stream.lock().unwrap().is_none());
    assert!(ipc.shm_reader.lock().unwrap().is_none());
}

#[test]
fn detach_is_idempotent() {
    let daemon = test_helpers::FakeDaemon::start(|_c, _r| (AK_SUCCESS_I32, vec![]));
    let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
    ipc.set_epochs_for_test(5, 5);

    ipc.detach();
    ipc.detach(); // must not panic or poison a mutex

    assert_eq!(ipc.attached_epoch_for_test(), EPOCH_DETACHED);
}
```

**Step 2: Run to verify it fails**

```bash
$CARGO test --target $HOST --lib -- detach_ 2>&1
```

Expected: COMPILE_ERROR.

**Step 3: Implement**

```rust
    /// Tear down the current attachment.
    ///
    /// Clears the epoch first: from this instant every in-flight request is refused
    /// by [`Self::epoch_gate`], so no stale handle can race the teardown. Then drops
    /// the frame sockets and unmaps the ring.
    ///
    /// Idempotent — the supervisor calls it on every failed attach attempt as well as
    /// on peer loss. Uses the poisoned-lock recovery path deliberately: a panicked
    /// frame reader must not make the connection permanently un-teardownable.
    pub(crate) fn detach(&self) {
        self.attached_epoch.store(EPOCH_DETACHED, Ordering::Release);
        self.observed_epoch.store(EPOCH_DETACHED, Ordering::Release);

        let mut main = self
            .frame_main_stream
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        *main = None;
        drop(main);

        let mut sub = self
            .frame_sub_stream
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        *sub = None;
        drop(sub);

        let mut shm = self.shm_reader.lock().unwrap_or_else(|e| e.into_inner());
        *shm = None;
        drop(shm);

        tracing::info!(event = "ipc_detached", "IPC detached from vendor daemon");
    }

    #[cfg(test)]
    pub(crate) fn attached_epoch_for_test(&self) -> u32 {
        self.attached_epoch.load(Ordering::Acquire)
    }

    #[cfg(test)]
    pub(crate) fn observed_epoch_for_test(&self) -> u32 {
        self.observed_epoch.load(Ordering::Acquire)
    }
```

**Step 4: Run to verify it passes**

```bash
$CARGO test --target $HOST --lib -- detach_ 2>&1
```

**Step 5: Commit**

```bash
git add cross-compile/onvif-rust/src/hal/anyka/ipc/mod.rs
git commit -m "feat(ipc): add detach() to release an attachment"
```

---

### Task 8: `attach()`

**Files:**
- Modify: `cross-compile/onvif-rust/src/hal/anyka/ipc/mod.rs`

**Ordering matters.** The daemon creates the ring first, then ctrl, then frame-main, then frame-sub (`main.c:267,280,288,296`). Connecting frame-sub *first* is therefore a free readiness barrier: if it accepts, everything else already exists. Attaching in creation order would race a still-initialising daemon and burn retry budget.

**Step 1: Write the failing test**

```rust
#[tokio::test]
async fn attach_rejects_a_ring_epoch_that_disagrees_with_hello() {
    // Daemon restarted between HELLO and the ring being mapped: the two epochs
    // disagree, so the attachment is already stale and must not be pinned.
    let daemon = test_helpers::FakeDaemon::start(|_c, _r| {
        let mut resp = Vec::new();
        resp.extend_from_slice(&11u32.to_le_bytes());
        resp.extend_from_slice(&3u32.to_le_bytes());
        (AK_SUCCESS_I32, resp)
    });
    let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();

    let reader = shm_ring::tests::create_test_anon_reader();
    // SAFETY: offset 48 is inside the validated header.
    unsafe {
        reader.base_ptr_for_test().add(48).cast::<u32>().write_volatile(12);
    }

    let err = ipc.finish_attach_for_test(reader).await.unwrap_err();

    assert!(matches!(err, PlatformError::HardwareUnavailable(_)));
    assert_eq!(ipc.attached_epoch_for_test(), EPOCH_DETACHED);
}

#[tokio::test]
async fn attach_pins_the_epoch_when_hello_and_ring_agree() {
    let daemon = test_helpers::FakeDaemon::start(|_c, _r| {
        let mut resp = Vec::new();
        resp.extend_from_slice(&11u32.to_le_bytes());
        resp.extend_from_slice(&3u32.to_le_bytes());
        (AK_SUCCESS_I32, resp)
    });
    let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();

    let reader = shm_ring::tests::create_test_anon_reader();
    unsafe {
        reader.base_ptr_for_test().add(48).cast::<u32>().write_volatile(11);
    }

    ipc.finish_attach_for_test(reader).await.unwrap();

    assert_eq!(ipc.attached_epoch_for_test(), 11);
    assert_eq!(ipc.observed_epoch_for_test(), 11);
}
```

**Step 2: Run to verify it fails**

```bash
$CARGO test --target $HOST --lib -- attach_ 2>&1
```

**Step 3: Implement**

```rust
    /// Establish frame sockets, ring mapping and epoch against a live daemon.
    ///
    /// Connects in *reverse* creation order. The daemon creates the ring, then the
    /// control socket, then frame-main, then frame-sub, so a successful frame-sub
    /// connect proves the rest already exists. Attaching in creation order would
    /// race a still-initialising daemon and waste the retry budget.
    ///
    /// On any failure the partial attachment is rolled back via [`Self::detach`],
    /// so a failed attempt never leaves half a connection behind.
    pub(crate) async fn attach(&self) -> PlatformResult<u32> {
        let result = self.try_attach().await;
        if result.is_err() {
            self.detach();
        }
        result
    }

    async fn try_attach(&self) -> PlatformResult<u32> {
        // Readiness barrier: frame-sub is the last thing the daemon creates.
        let frame_sub = UnixStream::connect(FRAME_SUB_SOCKET_PATH).map_err(|e| {
            PlatformError::HardwareUnavailable(format!(
                "sub frame socket {FRAME_SUB_SOCKET_PATH} not ready: {e}"
            ))
        })?;
        let frame_main = UnixStream::connect(FRAME_MAIN_SOCKET_PATH).map_err(|e| {
            PlatformError::HardwareUnavailable(format!(
                "main frame socket {FRAME_MAIN_SOCKET_PATH} not ready: {e}"
            ))
        })?;
        *self
            .frame_sub_stream
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(frame_sub);
        *self
            .frame_main_stream
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(frame_main);

        // Control last of the three, per the documented order. `finish_attach`
        // issues CMD_HELLO through the owner thread, which replies
        // HardwareUnavailable while its stream is `None` (Task 9), so the owner
        // must be holding a live stream before the handshake runs.
        let ctrl = UnixStream::connect(CTRL_SOCKET_PATH).map_err(|e| {
            PlatformError::HardwareUnavailable(format!(
                "control socket {CTRL_SOCKET_PATH} not ready: {e}"
            ))
        })?;
        // Must complete before finish_attach: `hello()` goes through the same
        // owner thread, and if installation were merely queued the handshake
        // could reach the owner first and be refused as HardwareUnavailable.
        self.give_ctrl_stream(ctrl).await?;

        let reader = ShmRingReader::open()?.ok_or_else(|| {
            PlatformError::HardwareUnavailable("shared memory ring not present".to_string())
        })?;
        self.finish_attach(reader).await
    }

    /// Handshake and epoch agreement, given an already-opened ring reader.
    ///
    /// Split out from `try_attach` so tests can supply an anonymous ring.
    async fn finish_attach(&self, reader: ShmRingReader) -> PlatformResult<u32> {
        let (epoch, version) = self.hello().await?;

        let ring_epoch = reader.epoch();
        if ring_epoch != epoch {
            return Err(PlatformError::HardwareUnavailable(format!(
                "daemon generation changed mid-attach (HELLO {epoch}, ring {ring_epoch}); retrying"
            )));
        }

        *self.shm_reader.lock().unwrap_or_else(|e| e.into_inner()) = Some(reader);
        // observed before attached: the gate reads attached first, so this ordering
        // can never leave a window where attached is set but observed is stale.
        self.observed_epoch.store(epoch, Ordering::Release);
        self.attached_epoch.store(epoch, Ordering::Release);

        tracing::info!(
            event = "ipc_attached",
            epoch,
            shm_version = version,
            "IPC attached to vendor daemon"
        );
        Ok(epoch)
    }

    #[cfg(test)]
    pub(crate) async fn finish_attach_for_test(
        &self,
        reader: ShmRingReader,
    ) -> PlatformResult<u32> {
        let result = self.finish_attach(reader).await;
        if result.is_err() {
            self.detach();
        }
        result
    }
```

**Step 4: Run to verify it passes**

```bash
$CARGO test --target $HOST --lib -- attach_ 2>&1
```

**Step 5: Commit**

```bash
git add cross-compile/onvif-rust/src/hal/anyka/ipc/mod.rs
git commit -m "feat(ipc): add attach() with reverse-order readiness barrier

The daemon creates ring, ctrl, frame-main, frame-sub in that order, so
connecting frame-sub first proves the rest exists."
```

---

### Task 9: Let `AnykaIpc::new()` succeed while detached

**Files:**
- Modify: `cross-compile/onvif-rust/src/hal/anyka/ipc/mod.rs:468-524`

**Step 1: Write the failing test**

```rust
#[test]
fn new_detached_succeeds_without_a_daemon() {
    // R5: cold start and recovery share one path, so construction must not
    // require a live daemon. Attaching is the supervisor's job.
    let ipc = AnykaIpc::new_detached().expect("construction must not need a daemon");

    assert_eq!(ipc.attached_epoch_for_test(), EPOCH_DETACHED);
    assert!(ipc.frame_main_stream.lock().unwrap().is_none());
    assert!(ipc.shm_reader.lock().unwrap().is_none());
}
```

**Step 2: Run to verify it fails**

Expected: COMPILE_ERROR.

**Step 3: Implement**

Add a constructor that builds the owner thread with a *lazily* connected control stream and leaves every resource `None`. Keep `new()` as a thin wrapper that constructs detached and then attaches, so existing production callers are unchanged for now (Task 12 removes that wrapper).

```rust
    /// Construct an unattached client.
    ///
    /// No daemon needs to exist. Every resource is `None` and the epoch is
    /// [`EPOCH_DETACHED`], so [`Self::epoch_gate`] refuses every request until the
    /// supervisor attaches. This is what lets cold start and recovery share one path.
    pub fn new_detached() -> PlatformResult<Self> { /* … */ }
```

**Note:** the control-socket owner thread currently requires a connected `UnixStream` up front (`spawn_owner` at `:672`). Change `run_owner` to hold `Option<UnixStream>` and reply `HardwareUnavailable` to any job while it is `None`. Expose `give_ctrl_stream(UnixStream)` and `drop_ctrl_stream()` so the owner's stream can be installed and cleared without respawning the thread — `try_attach` (Task 8) calls the former, `detach` the latter. This is the one structural change in the phase — do it here, not spread across tasks. Task 8's `try_attach` is written against `give_ctrl_stream`, so if you are running the tasks strictly in order, stub it as `unimplemented!()` in Task 8 and fill it in here.

**`give_ctrl_stream` must be acknowledged, not fire-and-forget.** Send it as a job on the same queue the requests use and await the owner's reply, so installation is ordered ahead of the `CMD_HELLO` that `finish_attach` issues immediately afterwards. Handing the stream over on a side channel — or storing it in a field the owner picks up "eventually" — leaves a window where the handshake reaches the owner while its stream is still `None` and attach fails spuriously on a healthy daemon. `drop_ctrl_stream()` can stay fire-and-forget: it only ever makes subsequent requests fail sooner.

**Step 4: Run the full suite**

```bash
$CARGO test --target $HOST 2>&1
```

**Step 5: Commit**

```bash
git add cross-compile/onvif-rust/src/hal/anyka/ipc/mod.rs
git commit -m "feat(ipc): allow construction without a live vendor daemon"
```

---

## Phase 5 — The supervisor

### Task 10: Availability channel and supervisor skeleton

**Files:**
- Create: `cross-compile/onvif-rust/src/platform/anyka/supervisor.rs`
- Modify: `cross-compile/onvif-rust/src/platform/anyka/mod.rs:29` (module decl), `:154-211` (`with_isp_config`)

**Step 1: Write the failing test**

```rust
#[tokio::test(start_paused = true)]
async fn backoff_grows_and_caps() {
    let mut b = Backoff::new();
    assert_eq!(b.next(), Duration::from_millis(500));
    assert_eq!(b.next(), Duration::from_secs(1));
    assert_eq!(b.next(), Duration::from_secs(2));
    assert_eq!(b.next(), Duration::from_secs(4));
    assert_eq!(b.next(), Duration::from_secs(8));
    assert_eq!(b.next(), BACKOFF_MAX);
    assert_eq!(b.next(), BACKOFF_MAX, "must cap, not grow unbounded");
    b.reset();
    assert_eq!(b.next(), Duration::from_millis(500));
}

#[test]
fn circuit_breaker_opens_after_the_threshold_and_stays_open() {
    let mut cb = CircuitBreaker::new();
    for _ in 0..ATTACH_FAILURE_LIMIT - 1 {
        cb.record_failure();
        assert!(!cb.is_open(), "must not trip early");
    }
    cb.record_failure();
    assert!(cb.is_open(), "must trip at the limit");
    cb.record_failure();
    assert!(cb.is_open(), "must stay open");
}

#[test]
fn circuit_breaker_resets_on_success() {
    let mut cb = CircuitBreaker::new();
    cb.record_failure();
    cb.record_failure();
    cb.record_success();
    for _ in 0..ATTACH_FAILURE_LIMIT - 1 {
        cb.record_failure();
    }
    assert!(!cb.is_open(), "success must clear the count");
}
```

**Step 2: Run to verify it fails**

```bash
$CARGO test --target $HOST --lib -- supervisor 2>&1
```

**Step 3: Implement**

Create `supervisor.rs` with:

```rust
//! Sole owner of the vendor-daemon attachment.
//!
//! Detection sites (control owner thread, frame reader, epoch poller) only *report*
//! peer loss; they never attach. This is an invariant, not a convention: the daemon's
//! single-owner guards reject a concurrent second attacher rather than serialising it
//! (`dispatcher.c:67` for control, `main.c:328` for the frame sockets), so two
//! simultaneous attaches leave a half-attached mess rather than one winner.

/// First retry delay; doubles up to [`BACKOFF_MAX`].
const BACKOFF_START: Duration = Duration::from_millis(500);
/// Cap on the retry delay.
const BACKOFF_MAX: Duration = Duration::from_secs(15);
/// Consecutive attach failures before the breaker opens.
///
/// This does not exist to save CPU — a connect attempt per second is noise on this
/// box. It bounds cumulative damage to the vendor SDK from repeated
/// VI_OPEN/VENC_OPEN churn (see `PUSH_JOIN_TIMEOUT_SEC` in the daemon's `globals.h`
/// and commit e3b1af9), and it stops a future respawn loop from being amplified into
/// a crash loop if attach is what kills the daemon.
const ATTACH_FAILURE_LIMIT: u32 = 10;
/// How often the ring epoch is polled while attached.
const EPOCH_POLL_INTERVAL: Duration = Duration::from_secs(1);

/// What the rest of the application observes about the attachment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Availability {
    /// Attached and the hardware pipeline is initialised.
    Available,
    /// Not attached; the supervisor is retrying.
    Unavailable,
    /// The breaker is open. No further attach attempts without intervention.
    GivenUp,
}
```

Plus `Backoff` and `CircuitBreaker` as small structs matching the tests, and a `watch::Sender<Availability>`.

**Step 4: Run to verify it passes**

**Step 5: Commit**

```bash
git add cross-compile/onvif-rust/src/platform/anyka/supervisor.rs cross-compile/onvif-rust/src/platform/anyka/mod.rs
git commit -m "feat(platform): add attach supervisor scaffolding with backoff and breaker"
```

---

### Task 11: The epoch poller

**Files:**
- Modify: `cross-compile/onvif-rust/src/platform/anyka/supervisor.rs`, `cross-compile/onvif-rust/src/hal/anyka/ipc/mod.rs`

**Why this task carries the design.** The poller is the only thing that detects a restart while idle. With push stopped and no RTSP client there is no frame traffic, so no socket ever errors and no EOF ever arrives — the camera would sit streaming nothing indefinitely. It is a single volatile `u32` read of an already-mapped page.

**Step 1: Write the failing test**

```rust
#[tokio::test]
async fn poller_reports_loss_when_the_ring_epoch_changes() {
    let ipc = /* attached AnykaIpc with an anonymous ring at epoch 11 */;
    let (tx, mut rx) = watch::channel(Availability::Available);

    // Daemon restarts: stamp a new generation into the ring.
    unsafe { /* write 12 at offset 48 */ }

    poll_epoch_once(&ipc, &tx);

    assert_eq!(*rx.borrow_and_update(), Availability::Unavailable);
}

#[tokio::test]
async fn poller_is_quiet_while_the_epoch_holds() {
    let ipc = /* attached at epoch 11, ring still 11 */;
    let (tx, mut rx) = watch::channel(Availability::Available);

    poll_epoch_once(&ipc, &tx);

    assert_eq!(*rx.borrow_and_update(), Availability::Available);
}
```

**Step 2-4: fail → implement → pass**

Add to `AnykaIpc`:

```rust
    /// Refresh `observed_epoch` from the ring and report whether it still matches.
    ///
    /// Uses `try_lock`: the frame reader holds `shm_reader` during a read, and the
    /// poller must never block behind it. Contention means "no new information this
    /// tick", which is correct — the next tick is 1 s away.
    pub(crate) fn refresh_observed_epoch(&self) -> bool {
        let Ok(guard) = self.shm_reader.try_lock() else {
            return true; // no information; do not report loss on lock contention
        };
        let Some(reader) = guard.as_ref() else {
            return false; // detached
        };
        let live = reader.epoch();
        self.observed_epoch.store(live, Ordering::Release);
        let attached = self.attached_epoch.load(Ordering::Acquire);
        live == EPOCH_DETACHED || live == attached
    }
```

**Step 5: Commit**

```bash
git commit -m "feat(platform): poll the ring epoch to detect an idle daemon restart"
```

---

### Task 12: Wire the supervisor loop to platform init and rollback

**Files:**
- Modify: `cross-compile/onvif-rust/src/platform/anyka/supervisor.rs`, `cross-compile/onvif-rust/src/platform/anyka/mod.rs:154-211,386-395`

**Reuse, do not reimplement.** `Platform::initialize()` (`mod.rs:386`) already runs the four bring-up stages, and `rollback_video_pipeline()` (`mod.rs:335`) plus `shutdown_video_pipeline()` already unwind them in reverse. The supervisor calls these; it does not grow its own copy.

The loop:

```
loop {
    if breaker.is_open() { publish(GivenUp); return; }
    match ipc.attach().await {
        Err(e) => { publish(Unavailable); breaker.record_failure(); sleep(backoff.next()); }
        Ok(epoch) => match platform.initialize().await {
            Err(e) => {
                // Partial bring-up MUST be unwound. Without this, each retry does
                // another VI_OPEN/VENC_OPEN cycle against an SDK that wedges.
                platform.rollback_video_pipeline().await;
                ipc.detach();
                breaker.record_failure();
                sleep(backoff.next());
            }
            Ok(()) => {
                breaker.record_success(); backoff.reset(); publish(Available);
                wait_for_loss(&ipc, &mut loss_rx).await;   // poller or reported error
                publish(Unavailable);
                platform.rollback_video_pipeline().await;
                ipc.detach();
            }
        }
    }
}
```

**Tests:** drive the loop with a mock platform whose `initialize` fails N times then succeeds; assert `rollback_video_pipeline` was called once per failure, that the breaker opens at the limit, and that `Availability` transitions are `Unavailable → Available → Unavailable`.

**Commit:**

```bash
git commit -m "feat(platform): drive attach, init, rollback and detach from one supervisor"
```

---

### Task 13: Report peer loss from the detection sites

**Files:**
- Modify: `cross-compile/onvif-rust/src/hal/anyka/ipc/mod.rs` (`process_job` error arm from Task 6; `read_push_notification` at `:583`)

Both sites send on an `mpsc::Sender<PeerLoss>` handed in at construction. Neither attaches. Assert this in a test: after a reported loss, the number of attach calls is still exactly one.

**Commit:**

```bash
git commit -m "feat(ipc): report ctrl errors and frame EOF to the supervisor"
```

---

## Phase 6 — Daemon-side reverse direction

### Task 14: Close leaked VI/VENC when the control client changes

**Files:**
- Modify: `cross-compile/vendor-daemon/src/dispatcher.c:116-130` (`release_control`), `cross-compile/vendor-daemon/src/globals.c`/`.h` (session handle registry)

**Why here and not in Rust Drop.** Four of six client-side handle Drops are deliberate no-ops (`video.rs:145,339`, `audio.rs:71,156`), so `onvif-rust` never sends CLOSE and the daemon leaks VI/VENC on every client restart. Making Drop send IPC would block on a socket — and it would not run at all under SIGKILL, which is exactly the case that matters. Resetting on the daemon side covers every way the client can vanish.

Track handles opened by the current control session in a small fixed-size registry, and close them in `release_control()` next to the existing `stop_push_slot(0/1)` calls (`main.c:499-505`).

**Verify:** integration only (Task 16).

**Commit:**

```bash
git commit -m "fix(vendor-daemon): close VI/VENC when the control client goes away

Client-side handle Drops are no-ops in IPC mode and never run under SIGKILL,
so cleanup has to live here to cover every way the client can vanish."
```

---

### Task 15: Reject stale-epoch handle commands in the dispatcher

**Files:**
- Modify: `cross-compile/vendor-daemon/src/dispatcher.c:176` (`req_read_handle`), `handlers_vi.c:59-64`, `handlers_venc.c` (every site returning a raw pointer as a handle)

Defence in depth: the client already refuses to send, so this only catches bugs and version skew. Return a distinct status the client can log.

**The handle must carry its epoch, or this check cannot work.** Today
`handle_vi_open()` returns `ak_vi_open()`'s pointer straight through as a `u64`
and `req_read_handle()` casts it back — no session tag (see design finding 2).
A bare registry of live pointers is *not* sufficient: after a restart the
allocator can hand out the same address, so a stale handle from the previous
generation would look valid. Bind the epoch into the handle itself:

- Stop returning pointers. Keep tables of open SDK objects, one per resource
  kind, built empty at daemon start so they hold nothing from a previous
  generation. Return an opaque `u64` token laid out as:

  ```
  bits 63..32  epoch       (32)  daemon generation
  bits 31..16  slot_gen    (16)  bumped every time the slot is reused
  bits 15..4   slot_index  (12)  up to 4096 live objects per kind
  bits  3..0   kind        (4)   VI, VENC, ... — selects the table
  ```

- **`slot_gen` is what stops an epoch-local ABA.** Epoch alone is not enough:
  close a VI on slot 3 and open another within the *same* generation, and a
  stale token for the old occupant still matches epoch and index. Each table
  entry stores its own `slot_gen`, incremented on every allocation of that
  slot, so a token from the previous occupant no longer matches.
- `req_read_handle()` validates in order: `kind` selects the table and must
  match what the command expects (a VENC token handed to a VI command is
  rejected, not dereferenced); `epoch` must equal the daemon's current epoch;
  `slot_index` must be in bounds; the entry must be live; and `slot_gen` must
  equal the entry's. Only then does it yield the pointer.
- Reject with a status distinct from the generic invalid-argument one — the
  client logs "stale epoch" rather than a confusing argument error. Add
  `VD_STATUS_STALE_EPOCH` alongside the existing status codes. A `slot_gen` or
  `kind` mismatch uses it too: from the client's side these all mean "this
  handle is no longer yours".

This also removes the raw-pointer marshalling across the process boundary,
which is worth doing on its own.

**Commit:**

```bash
git commit -m "feat(vendor-daemon): reject handle commands from a stale epoch"
```

---

## Phase 7 — Degraded boot

### Task 16: Remove the startup guards and surface availability

**Files:**
- Modify: `SD_card_contents/anyka_hack/onvif/run_onvif_rust.sh:64-72`, `cross-compile/onvif-rust/src/onvif/media/service.rs`

Delete the `pidof vendor-daemon.bin` abort — with degraded boot it is actively wrong, and it is what stopped cold start and recovery from sharing a path. Wire `Availability` into the media service so DESCRIBE/SETUP report unavailable instead of handing out a stream that will never produce frames. A backoff that hides a permanent failure leaves the camera dark with evidence only in logs; this is what makes it visible.

**Commit:**

```bash
git commit -m "feat: boot degraded without vendor-daemon and surface attach state"
```

---

## Phase 8 — Integration verification on hardware

### Task 17: Both directions of R1 on the device

**REQUIRED SUB-SKILL:** use `superpowers:verification-before-completion` before claiming any of this passes. Evidence before assertions.

| # | Scenario | Pass criterion |
|---|---|---|
| 1 | `kill -9 vendor-daemon.bin` mid-stream, respawn manually | `onvif-rust` logs `ipc_detached` then `ipc_attached` with a **different** epoch; RTSP client re-SETUPs and gets a fresh IDR |
| 2 | `kill -9 onvif-rust`, restart it | daemon logs the control client leaving, closes VI/VENC, accepts the new client; stream works |
| 3 | Start `onvif-rust` with **no** daemon, start daemon 30 s later | device service answers throughout; media reports unavailable, then available; no restart of `onvif-rust` |
| 4 | **Idle detection.** No RTSP client, push stopped. Restart the daemon. | loss is detected by the poller within ~2 s, with **no** socket error in the log. This is the case the design exists for |
| 5 | Wedge the SDK so attach fails repeatedly | breaker opens at `ATTACH_FAILURE_LIMIT`, `Availability::GivenUp` is logged, attempts stop |

Deploy with `scripts/build_sd_contents.sh` and `scripts/copy_sd_contents.sh`. Device is at telnet `192.168.2.198:24`.

**Do not** claim completion until scenario 4 has been observed with a clean log — it is the one that distinguishes this design from the cheaper alternatives, and the one most likely to be silently broken.

---

## Open items carried from the design

- `BACKOFF_MAX` and `ATTACH_FAILURE_LIMIT` are placeholders. Measure real daemon restart time on hardware in Task 17 and set them from that, not from a guess.
- Whether `EPOCH_POLL_INTERVAL` deserves to be configurable.
- Process respawn (R4) stays out of scope. The circuit breaker is what makes it safe to add later.
