# Restart-Resilience Hardware Fixes Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Fix the three defects Task 17 hardware testing exposed — a daemon crash on client restart (S2), a circuit breaker that gives up on a merely-absent daemon (S3), and misleading comments about the epoch poller's role (S4).

**Architecture:** S3 is a Rust supervisor change (only count bring-up failures toward the breaker). S2 is a C daemon change (`vd_obj_close_all` performs the full safe SDK teardown so a SIGKILL'd client cannot leave an SDK `capture_thread` running against a freed VI). S4 is comment-only. Verified on the device at telnet `192.168.2.198:24`.

**Tech Stack:** Rust (tokio, vendored ARM toolchain), C99 (uClibc, ARMv5TE cross), vendor SDK (`libplat_vi.so`, `libmpi_venc.so`).

**Design doc:** `docs/plans/2026-07-31-restart-resilience-hardware-fixes-design.md` — read it first.

---

## Conventions used throughout this plan

Set once per shell session, from the repo root `/home/kmk/dev/anyka-dev`:

```bash
export CARGO=cross-compile/onvif-rust/../../toolchain/arm-anykav200-crosstool-ng/bin/cargo
export TOOLBIN=/home/kmk/dev/anyka-dev/toolchain/arm-anykav200-crosstool-ng/bin
export HOST=x86_64-unknown-linux-gnu
```

- **All** cargo commands use the vendored toolchain. Run from `cross-compile/onvif-rust/`.
- Host tests: `$CARGO test --target $HOST --lib -- <name>` (run from `cross-compile/onvif-rust/`).
- Before every Rust commit: `PATH=$TOOLBIN:$PATH $CARGO clippy --target $HOST --lib --tests -- -D warnings` and `$CARGO fmt --check`. The `PATH` prefix is mandatory or clippy dies with `E0514` (see memory `vendored-clippy-needs-path-prefix`).
- Daemon build: `make -C cross-compile/vendor-daemon` (clean rebuild needs `make -C cross-compile/vendor-daemon clean` first — the Makefile has no header dependency tracking).
- Device shell: `uv run python3 scripts/debugging/cam_exec.py '<cmd>'`. The device drops straight to a root shell; RTSP creds `admin:admin`; ONVIF HTTP on port 80 with a rate limiter.
- Device file transfer (no `lftp`): camera `nc -l -p PORT > /tmp/x &`, host `nc 192.168.2.198 PORT < file`, verify with `md5sum` both sides, then `mv` into place with `chmod +x`.

---

## Phase 1 — S3: breaker ignores absent-daemon failures (Rust)

Done first: pure Rust, TDD, no hardware needed to prove the logic.

### Task 1: Only count bring-up failures toward the circuit breaker

**Files:**
- Modify: `cross-compile/onvif-rust/src/platform/anyka/supervisor.rs` — `run_supervisor` at `:196-231` (the loop), tests in the same file's `mod tests`.

**Step 1: Write the failing test**

Add to `mod tests` in `supervisor.rs`, after `supervisor_gives_up_after_the_breaker_opens_on_attach_failures`:

```rust
    #[tokio::test(start_paused = true)]
    async fn an_absent_daemon_never_opens_the_breaker() {
        // attach() failing because the daemon is not there yet must be retried
        // forever: no SDK call happened, so there is no churn to bound. Only
        // bring-up (initialize) failures against a live daemon count.
        let target = std::sync::Arc::new(MockTarget::new(false, 0));
        let (tx, _rx) = watch::channel(Availability::Unavailable);

        // Run the supervisor with a hard timeout: with the fix it never returns
        // (retries forever), so a timeout is the pass signal.
        let stop = tokio::time::timeout(
            std::time::Duration::from_secs(600),
            run_supervisor(target.clone(), &tx),
        )
        .await;

        assert!(stop.is_err(), "supervisor gave up on an absent daemon");
        assert!(
            target.attach_calls.load(AtomicOrdering::SeqCst) > ATTACH_FAILURE_LIMIT as usize,
            "must keep retrying past the failure limit, not give up"
        );
        assert_ne!(*tx.borrow(), Availability::GivenUp);
    }
```

**Step 2: Run it to verify it fails**

```bash
cd cross-compile/onvif-rust
$CARGO test --target $HOST --lib -- an_absent_daemon_never_opens_the_breaker 2>&1 | tail -20
```

Expected: FAIL — the supervisor currently records a failure on every `attach()`
error, so the breaker opens at `ATTACH_FAILURE_LIMIT` and `run_supervisor` returns
`GivenUp`; the `timeout` completes `Ok`, so `stop.is_err()` is false.

**Step 3: Implement**

In `run_supervisor`, the `attach()` error arm currently reads (around `:202-207`):

```rust
        match target.attach().await {
            Err(e) => {
                tracing::warn!(error = %e, "attach failed");
                let _ = tx.send(Availability::Unavailable);
                breaker.record_failure();
                tokio::time::sleep(backoff.next()).await;
            }
```

Remove the `breaker.record_failure();` line from this arm and explain why:

```rust
        match target.attach().await {
            Err(e) => {
                // Do NOT count this toward the breaker. attach() fails here because
                // the daemon is absent, not yet listening, or restarting mid-attach
                // — no SDK call happened, so there is no VI_OPEN/VENC_OPEN churn to
                // bound. The breaker exists only for a live-but-wedged daemon, which
                // is the initialize() arm below. Counting absent-daemon failures
                // here is what made degraded boot give up on a late daemon (S3).
                tracing::warn!(error = %e, "attach failed; retrying (daemon absent)");
                let _ = tx.send(Availability::Unavailable);
                tokio::time::sleep(backoff.next()).await;
            }
```

Leave the `initialize()` error arm (around `:218`) untouched — it keeps
`breaker.record_failure()`.

**Step 4: Run to verify it passes**

```bash
$CARGO test --target $HOST --lib -- an_absent_daemon_never_opens_the_breaker 2>&1 | tail -8
```

Expected: PASS.

**Step 5: Run the full supervisor suite — the loop changed**

```bash
$CARGO test --target $HOST --lib -- supervisor 2>&1 | tail -20
```

Expected: all pass. `supervisor_gives_up_after_the_breaker_opens_on_attach_failures`
uses `MockTarget::new(false, 0)` (attach fails) and asserts `GivenUp` — this test
now encodes the OLD behaviour and **must be updated**, not the fix weakened. Change
it so attach succeeds but `initialize` always fails, which is the real path that
opens the breaker:

```rust
    #[tokio::test(start_paused = true)]
    async fn supervisor_gives_up_after_the_breaker_opens_on_initialize_failures() {
        // Breaker opens on repeated bring-up failure against a LIVE daemon
        // (attach succeeds, initialize wedges) — not on an absent one.
        let target = std::sync::Arc::new(MockTarget::new(true, 0));
        let (tx, _rx) = watch::channel(Availability::Unavailable);

        run_supervisor(target.clone(), &tx).await;

        assert_eq!(
            target.initialize_calls_or_init_calls(),
            ATTACH_FAILURE_LIMIT as usize
        );
        assert_eq!(*tx.borrow(), Availability::GivenUp);
    }
```

Use whatever the mock's init-call counter is actually named (`init_calls` in the
current `MockTarget`); the helper name above is illustrative. If a test named
`supervisor_gives_up_after_the_breaker_opens_on_attach_failures` already asserts
the live-daemon-init path, keep it and only add the absent-daemon test from Step 1.

**Step 6: fmt + clippy + commit**

```bash
$CARGO fmt
PATH=$TOOLBIN:$PATH $CARGO clippy --target $HOST --lib --tests -- -D warnings 2>&1 | tail -3
$CARGO fmt --check && echo fmt-clean
cd /home/kmk/dev/anyka-dev
git add cross-compile/onvif-rust/src/platform/anyka/supervisor.rs
git commit -m "fix(platform): keep the breaker closed while the daemon is merely absent

attach() fails at socket-connect when the daemon is not there yet; no SDK call
happens, so it must not count toward the circuit breaker. Only initialize()
failures (a live but wedged daemon) do. This is what let degraded boot give up
on a late-starting daemon (Task 17 scenario 3).

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

## Phase 2 — S2: daemon closes the SDK stream before the VI (C)

### Task 2: Full safe teardown in `vd_obj_close_all`

**Files:**
- Modify: `cross-compile/vendor-daemon/src/globals.c` — includes near `:1-11`, `g_obj_close_order` at `:150`, `vd_obj_close_one` at `:158`, `vd_obj_close_all` at `:192`.

**Background (coredump-confirmed):** the SDK's `capture_thread` in
`libmpi_venc.so` calls `ak_vi_release_frame` and is only stopped by
`ak_venc_cancel_stream`. `stop_push_slot` does **not** cancel it — it stops the
daemon's own push reader. So `vd_obj_close_all` closing VI while that thread runs
segfaults. The fix cancels the stream (bounded), closes VENC, turns VI capture
off, then closes VI.

There is no C unit-test harness; this task is verified by rebuild + Task 5 on
hardware.

**Step 1: Add a bounded cancel-stream helper**

`ak_venc_cancel_stream` can block indefinitely, so it must run on a detached
thread with a timeout — the same shape as `handle_venc_cancel_stream` in
`handlers_venc.c`. Add a self-contained helper to `globals.c` (do **not** refactor
the working handler; mild duplication is the safe call here).

First ensure the includes at the top of `globals.c` cover it. After the existing
`#include "vd_ring_buffer.h"` line, add:

```c
#include <pthread.h>
#include <time.h>
```

Then add this helper immediately above `vd_obj_close_one` (before `:158`):

```c
/* Timeout for the SDK stream cancel during session cleanup, seconds. Mirrors
 * CANCEL_STREAM_TIMEOUT_SEC in handlers_venc.c. */
#define VD_OBJ_CANCEL_TIMEOUT_SEC 3

struct vd_cancel_arg {
    void        *handle;
    volatile int done;   /* set by the worker when ak_venc_cancel_stream returns */
};

static void *vd_cancel_worker(void *arg)
{
    struct vd_cancel_arg *ca = (struct vd_cancel_arg *)arg;
    (void)ak_venc_cancel_stream(ca->handle);
    __atomic_store_n(&ca->done, 1, __ATOMIC_RELEASE);
    return NULL;
}

/*
 * vd_cancel_stream_bounded - Cancel an SDK stream, giving up after a timeout.
 *
 * ak_venc_cancel_stream stops libmpi_venc's internal capture_thread, which must
 * happen before the VI it feeds on is closed (otherwise that thread calls
 * ak_vi_release_frame on freed state and the daemon segfaults). The call can
 * block indefinitely on a wedged encoder, so it runs on a detached thread and we
 * wait only VD_OBJ_CANCEL_TIMEOUT_SEC. On timeout we leak the small arg (the
 * detached thread may still touch it) and continue teardown: a leaked capture
 * thread is less bad than hanging the accept loop.
 */
static void vd_cancel_stream_bounded(void *handle)
{
    struct vd_cancel_arg *ca = (struct vd_cancel_arg *)malloc(sizeof(*ca));
    if (ca == NULL) {
        log_error("[obj] cancel_stream: malloc failed; skipping cancel ptr=%p", handle);
        return;
    }
    ca->handle = handle;
    ca->done = 0;

    pthread_t tid;
    if (pthread_create(&tid, NULL, vd_cancel_worker, ca) != 0) {
        log_error("[obj] cancel_stream: pthread_create failed ptr=%p", handle);
        free(ca);
        return;
    }
    pthread_detach(tid);

    struct timespec deadline;
    clock_gettime(CLOCK_MONOTONIC, &deadline);
    deadline.tv_sec += VD_OBJ_CANCEL_TIMEOUT_SEC;

    while (!__atomic_load_n(&ca->done, __ATOMIC_ACQUIRE)) {
        struct timespec now;
        clock_gettime(CLOCK_MONOTONIC, &now);
        if (now.tv_sec > deadline.tv_sec ||
            (now.tv_sec == deadline.tv_sec && now.tv_nsec >= deadline.tv_nsec)) {
            log_error("[obj] cancel_stream timed out after %ds ptr=%p (leaking arg)",
                      VD_OBJ_CANCEL_TIMEOUT_SEC, handle);
            return; /* intentional leak of ca; worker may still write it */
        }
        struct timespec ts = { .tv_sec = 0, .tv_nsec = 10 * 1000000L };
        nanosleep(&ts, NULL);
    }

    log_info("[obj] cancelled leaked stream ptr=%p", handle);
    free(ca);
}
```

**Step 2: Make STREAM close actually cancel, and VI close turn capture off**

Replace the `vd_obj_close_one` function (currently at `:158`). The current version
`return`s for `VD_OBJ_KIND_STREAM` and calls only `ak_vi_close` for VI. New version:

```c
static void vd_obj_close_one(uint8_t kind, void *ptr)
{
    int ret = 0;

    switch (kind) {
    case VD_OBJ_KIND_STREAM:
        /* Cancel the SDK stream so libmpi_venc's capture_thread stops before the
         * VI it reads is closed. stop_push_slot() only stops the daemon's own
         * push reader, NOT this thread. */
        vd_cancel_stream_bounded(ptr);
        return;
    case VD_OBJ_KIND_VENC:
        ret = ak_venc_close(ptr);
        break;
    case VD_OBJ_KIND_AENC:
        ret = ak_aenc_close(ptr);
        break;
    case VD_OBJ_KIND_VI:
        /* capture_off before close: the safe teardown onvif-rust does on clean
         * shutdown, moved here so a SIGKILLed client gets it too. */
        (void)ak_vi_capture_off(ptr);
        ret = ak_vi_close(ptr);
        break;
    case VD_OBJ_KIND_AI:
        ret = ak_ai_close(ptr);
        break;
    default:
        return;
    }

    if (ret != 0)
        log_warn("[obj] close kind=%u ptr=%p returned %d (continuing)",
                 (unsigned)kind, ptr, ret);
    else
        log_info("[obj] closed leaked kind=%u ptr=%p", (unsigned)kind, ptr);
}
```

The `g_obj_close_order` already lists `VD_OBJ_KIND_STREAM` first, then `VENC`,
`AENC`, `VI`, `AI` — which is exactly the required order (cancel stream → close
VENC → capture_off + close VI). Leave it unchanged.

**Step 3: Correct the stale comment**

Find the comment near `release_control` in `dispatcher.c` and the one that
previously claimed `stop_push_slot` cancels streams. In `globals.c` the old
`VD_OBJ_KIND_STREAM: /* ... stop_push_slot() already cancels the streams ... */`
text is gone (replaced in Step 2). Verify no remaining comment claims the push
slots cancel the SDK stream:

```bash
grep -rn "already cancels the streams\|stop_push_slot() already" cross-compile/vendor-daemon/src/
```

Expected: no matches.

**Step 4: Clean rebuild (no header dep tracking)**

```bash
make -C cross-compile/vendor-daemon clean
make -C cross-compile/vendor-daemon 2>&1 | grep -iE "warning|error"; echo "exit=${PIPESTATUS[0]}"
ls -l cross-compile/vendor-daemon/build/vendor-daemon.bin
```

Expected: `exit=0`, no warnings, binary present.

**Step 5: Commit**

```bash
git add cross-compile/vendor-daemon/src/globals.c
git commit -m "fix(vendor-daemon): cancel the SDK stream and stop capture before closing VI

Task 17 scenario 2 crashed the daemon: coredump shows libmpi_venc's internal
capture_thread calling ak_vi_release_frame after vd_obj_close_all freed the VI.
stop_push_slot only stops the daemon's own push reader, not that SDK thread, so
the previous 'skip streams' cleanup left it running.

vd_obj_close_all now does the full safe teardown a SIGKILLed client cannot:
ak_venc_cancel_stream (bounded by a detached thread + timeout, since it can
hang) -> ak_venc_close -> ak_vi_capture_off -> ak_vi_close.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

## Phase 3 — S4: correct the poller's documented role (comments only)

### Task 3: Reframe the epoch poller as a backstop

**Files:**
- Modify: `cross-compile/onvif-rust/src/platform/anyka/supervisor.rs` — the `poll_epoch_once` doc comment and the `watch_epoch_until_loss` doc comment.

**Step 1: Rewrite the `poll_epoch_once` doc**

The current comment claims the poller is "the only thing that detects a restart
while idle... no frame traffic." That is false in this build (push stays active).
Replace the doc block above `poll_epoch_once` with:

```rust
/// One tick of the epoch poller: refresh from the ring, publish loss if it moved.
///
/// A detection *backstop*, not the primary detector. In this build push stays
/// active regardless of RTSP clients, so a daemon restart trips frame-socket
/// errors within milliseconds and the frame reader / control owner report it
/// first. The poller still earns its keep on two races those miss: a daemon that
/// restarts fast enough to re-stamp the ring before the frame reader notices the
/// old socket died, and a half-dead daemon whose socket never EOFs. Cost is one
/// volatile u32 read of an already-mapped page per tick.
```

**Step 2: Rewrite the `watch_epoch_until_loss` doc**

Replace the doc block above `watch_epoch_until_loss` with one describing the
layering (keep the existing detail about the drain, which is correct):

```rust
/// Wait for peer loss, from whichever detector notices first.
///
/// Detection is layered, fastest first:
/// - frame-socket EOF/error — milliseconds — the primary detector while push is
///   active (which, in this build, is always);
/// - control-socket error — on the next control request;
/// - epoch poller — 1 s — a backstop for restart races and non-EOF daemon death.
///
/// The `reports` channel carries the first two; the ticker drives the poller.
/// (`idle-stop-push`, which would make the poller the sole idle detector, is
/// deliberately not implemented — see the S4 design.)
```

**Step 3: fmt + clippy (no test change; comments only)**

```bash
cd cross-compile/onvif-rust
$CARGO fmt --check && echo fmt-clean
PATH=$TOOLBIN:$PATH $CARGO clippy --target $HOST --lib --tests -- -D warnings 2>&1 | tail -3
```

Expected: clean (comments do not change codegen; clippy confirms nothing broke).

**Step 4: Commit**

```bash
cd /home/kmk/dev/anyka-dev
git add cross-compile/onvif-rust/src/platform/anyka/supervisor.rs
git commit -m "docs(platform): describe the epoch poller as a backstop, not the sole idle detector

Task 17 scenario 4 showed push stays active with no RTSP client, so socket errors
detect a daemon restart first and the poller never wins the race. The poller is
still a real backstop (fast-restart ring re-stamp, non-EOF death); the comments
claiming it is the only idle detector were wrong.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

## Phase 4 — Host verification and hardware re-test

### Task 4: Full host gate

**Step 1: Whole suite + gates**

```bash
cd cross-compile/onvif-rust
$CARGO test --target $HOST 2>&1 | grep -E "test result:|FAILED" | head
$CARGO fmt --check && echo fmt-clean
PATH=$TOOLBIN:$PATH $CARGO clippy --target $HOST --lib --tests -- -D warnings 2>&1 | tail -3
$CARGO build --release 2>&1 | tail -2
make -C /home/kmk/dev/anyka-dev/cross-compile/vendor-daemon clean
make -C /home/kmk/dev/anyka-dev/cross-compile/vendor-daemon 2>&1 | grep -iE "warning|error"; echo "make_exit=${PIPESTATUS[0]}"
```

Expected: all tests pass, fmt clean, clippy clean, release build finishes, daemon
builds with `make_exit=0` and no warnings.

No commit (verification only).

### Task 5: Deploy and re-run the hardware scenarios

**REQUIRED SUB-SKILL:** use `superpowers:verification-before-completion` before
claiming any scenario passes. Evidence before assertions.

**Step 1: Transfer the two rebuilt binaries**

```bash
cd /home/kmk/dev/anyka-dev
ONVIF=cross-compile/target/armv5te-unknown-linux-uclibceabi/release/onvif-rust
VD=cross-compile/vendor-daemon/build/vendor-daemon.bin
for f in "$ONVIF" "$VD"; do md5sum "$f"; done
# onvif
uv run python3 scripts/debugging/cam_exec.py '(nc -l -p 9101 > /tmp/onvif.new &); sleep 1; echo ready'
nc -w 60 192.168.2.198 9101 < "$ONVIF"
# daemon
uv run python3 scripts/debugging/cam_exec.py '(nc -l -p 9102 > /tmp/vd.new &); sleep 1; echo ready'
nc -w 60 192.168.2.198 9102 < "$VD"
uv run python3 scripts/debugging/cam_exec.py 'md5sum /tmp/onvif.new /tmp/vd.new'
```

Confirm both md5sums match the host. Then install and restart clean:

```bash
uv run python3 scripts/debugging/cam_exec.py '
pkill -9 -f run_onvif_rust; kill -9 $(pidof onvif-rust.bin) 2>/dev/null
pkill -9 -f run_vendor_daemon; kill -9 $(pidof vendor-daemon.bin) 2>/dev/null
sleep 2
mv /tmp/onvif.new /mnt/anyka_hack/onvif/onvif-rust.bin && chmod +x /mnt/anyka_hack/onvif/onvif-rust.bin
mv /tmp/vd.new /mnt/anyka_hack/vendor-daemon/vendor-daemon.bin && chmod +x /mnt/anyka_hack/vendor-daemon/vendor-daemon.bin
rm -f /mnt/coredumps/core.* 2>/dev/null
: > /mnt/logs/onvif_rust.log; : > /mnt/logs/vendor_daemon.log
echo installed'
```

**Step 2: Baseline — both up, streaming**

```bash
uv run python3 scripts/debugging/cam_exec.py '
setsid /mnt/anyka_hack/vendor-daemon/run_vendor_daemon.sh >/dev/null 2>&1 &
sleep 5
setsid /mnt/anyka_hack/onvif/run_onvif_rust.sh >/dev/null 2>&1 &
sleep 14
echo "vd=$(pidof vendor-daemon.bin) onvif=$(pidof onvif-rust.bin) available=$(grep -ac attach_available /mnt/logs/onvif_rust.log)"'
ffprobe -rtsp_transport tcp -v error -show_entries stream=codec_name,width,height -of default=noprint_wrappers=1 "rtsp://admin:admin@192.168.2.198:554/main" 2>&1 | head -3
```

Expected: both PIDs present, `available=1`, ffprobe prints `codec_name=h264`.

**Step 3: Scenario 2 (the fix) — kill onvif, restart, daemon must NOT crash**

```bash
uv run python3 scripts/debugging/cam_exec.py '
VDPID=$(pidof vendor-daemon.bin)
kill -9 $(pidof onvif-rust.bin); pkill -9 -f run_onvif_rust; sleep 3
echo "daemon cleanup log:"; grep -aE "obj\]|released|cancelled" /mnt/logs/vendor_daemon.log | tail -6 | sed "s/\x1b\[[0-9;]*m//g" | cut -c1-120
: > /mnt/logs/onvif_rust.log
setsid /mnt/anyka_hack/onvif/run_onvif_rust.sh >/dev/null 2>&1 &
sleep 15
echo "vd_same=$([ \"$(pidof vendor-daemon.bin)\" = \"$VDPID\" ] && echo yes || echo NO-CRASHED)"
echo "onvif=$(pidof onvif-rust.bin) available=$(grep -ac attach_available /mnt/logs/onvif_rust.log)"
echo "new cores: $(ls /mnt/coredumps/core.* 2>/dev/null | wc -l)"'
ffprobe -rtsp_transport tcp -v error -select_streams v -read_intervals "%+#1" -show_entries frame=pict_type -show_entries stream=codec_name -of default=noprint_wrappers=1 "rtsp://admin:admin@192.168.2.198:554/main" 2>&1 | head -3
```

**Pass criteria:** `vd_same=yes` (daemon did not crash), `new cores: 0`,
`available=1`, ffprobe prints a keyframe and `codec_name=h264`. The daemon log
shows `cancelled leaked stream` and `closed leaked kind=...`.

**Step 4: Scenario 3 (the fix) — onvif first, daemon 60 s later**

```bash
uv run python3 scripts/debugging/cam_exec.py '
pkill -9 -f run_onvif_rust; kill -9 $(pidof onvif-rust.bin) 2>/dev/null
pkill -9 -f run_vendor_daemon; kill -9 $(pidof vendor-daemon.bin) 2>/dev/null
rm -f /tmp/vd-*.sock /tmp/vendor-frame-ring.shm; sleep 2
: > /mnt/logs/onvif_rust.log
setsid /mnt/anyka_hack/onvif/run_onvif_rust.sh >/dev/null 2>&1 &
sleep 60
echo "after 60s no daemon: onvif=$(pidof onvif-rust.bin) given_up=$(grep -ac attach_given_up /mnt/logs/onvif_rust.log)"'
```

Expected mid-point: `onvif` PID present, `given_up=0` (breaker still closed after
60 s of an absent daemon — the S3 fix). Then start the daemon:

```bash
uv run python3 scripts/debugging/cam_exec.py '
ONVIFPID=$(pidof onvif-rust.bin)
setsid /mnt/anyka_hack/vendor-daemon/run_vendor_daemon.sh >/dev/null 2>&1 &
sleep 18
echo "onvif_same=$([ \"$(pidof onvif-rust.bin)\" = \"$ONVIFPID\" ] && echo yes || echo NO)"
echo "available=$(grep -ac attach_available /mnt/logs/onvif_rust.log) given_up=$(grep -ac attach_given_up /mnt/logs/onvif_rust.log)"'
ffprobe -rtsp_transport tcp -v error -show_entries stream=codec_name -of default=noprint_wrappers=1 "rtsp://admin:admin@192.168.2.198:554/main" 2>&1 | head -2
```

**Pass criteria:** `onvif_same=yes` (no restart), `given_up=0`, `available>=1`,
ffprobe prints `codec_name=h264`.

**Step 5: Scenario 1 + 4 regression — daemon restart while onvif runs**

```bash
uv run python3 scripts/debugging/cam_exec.py '
E0=$(od -An -tx4 -j48 -N4 /tmp/vendor-frame-ring.shm|tr -d " ")
kill -9 $(pidof vendor-daemon.bin); sleep 3
setsid /mnt/anyka_hack/vendor-daemon/run_vendor_daemon.sh >/dev/null 2>&1 &
sleep 15
E1=$(od -An -tx4 -j48 -N4 /tmp/vendor-frame-ring.shm|tr -d " ")
echo "epoch $E0 -> $E1 (must differ)"
echo "onvif=$(pidof onvif-rust.bin) available=$(grep -ac attach_available /mnt/logs/onvif_rust.log)"
sleep 8
echo "stable check: attached=$(grep -ac ipc_attached /mnt/logs/onvif_rust.log) available=$(grep -ac attach_available /mnt/logs/onvif_rust.log)"'
ffprobe -rtsp_transport tcp -v error -show_entries stream=codec_name -of default=noprint_wrappers=1 "rtsp://admin:admin@192.168.2.198:554/main" 2>&1 | head -2
```

**Pass criteria:** epoch differs, `onvif` PID unchanged, `available` incremented
by exactly 1 (no flap — regression check on `ec85013`), ffprobe streams.

**Step 6: Update the status memory and finish**

Record the hardware results in
`/home/kmk/.claude/projects/-home-kmk-dev-anyka-dev/memory/vendor-daemon-restart-resilience-status.md`
and, if all scenarios pass, invoke `superpowers:finishing-a-development-branch`.

---

## Open items carried forward

- `ATTACH_FAILURE_LIMIT` / `BACKOFF_MAX` remain placeholders. They now only bound
  the live-but-wedged-daemon case; tune from measured data if that case ever
  matters in practice.
- `idle-stop-push` remains out of scope (S4 design).
