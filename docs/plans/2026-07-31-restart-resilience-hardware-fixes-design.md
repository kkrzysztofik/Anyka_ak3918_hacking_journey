# Restart-Resilience Hardware Fixes — Design (S2/S3/S4)

**Context:** Tasks 1–16 of `2026-07-29-vendor-daemon-restart-resilience.md` were
implemented and passed all host-side gates. Task 17 (hardware verification on the
device at 192.168.2.198) then exposed defects that only appear on real hardware.
This document designs the fixes for the three that need work.

Hardware run already produced one committed fix (`ec85013`, the re-attach flap:
stale peer-loss reports drained after re-attach). What remains: **S2** (daemon
crash), **S3** (breaker gives up on an absent daemon), **S4** (poller role).

Scenarios S1 (daemon restart) and S5 (breaker opens) already pass on hardware.

---

## S2 — Daemon SIGSEGV on client restart (correctness)

### Root cause (coredump-confirmed)

Killing `onvif-rust` with SIGKILL runs no client-side Drop, so the daemon must
clean up. Task 14's `vd_obj_close_all` closes VI/VENC but skips streams, on the
assumption (stated in a code comment) that `stop_push_slot` already cancels them.
It does not: `stop_push_slot` only stops the daemon's *own* push-reader thread
(`ak_venc_get_stream`). The SDK's internal `capture_thread` in `libmpi_venc.so`,
started by `ak_venc_request_stream`, is only stopped by `ak_venc_cancel_stream`.

Coredump backtrace (`core.venc_capture.3106`, SIGSEGV):

```
#0  ak_vi_release_frame ()  from libplat_vi.so
#1  capture_thread ()       from libmpi_venc.so
```

The SDK `capture_thread` survives `vd_obj_close_all`, and its next
`ak_vi_release_frame` dereferences the VI state that `ak_vi_close` just freed.

### Fix

`vd_obj_close_all` (`vendor-daemon/src/globals.c`) performs the full safe
teardown that `onvif-rust` does on clean shutdown, in order:

1. **STREAM** → `ak_venc_cancel_stream(handle)`, wrapped in the existing detached
   thread + `CANCEL_STREAM_TIMEOUT_SEC` pattern from `handlers_venc.c` so a wedged
   cancel cannot hang the accept loop. Stops the SDK `capture_thread`.
2. **VENC** → `ak_venc_close(handle)`
3. **VI** → `ak_vi_capture_off(handle)`, then `ak_vi_close(handle)`

The registry already tracks STREAM/VENC/VI handles, so no new state. The VI
teardown becomes capture_off-then-close rather than a single close. The incorrect
"stop_push_slot already cancels the streams" comment is corrected.

### Verification

Integration only (no C unit harness). Re-run S2: `kill -9 onvif-rust`, restart it,
confirm the daemon does **not** crash (no new core, daemon PID stable) and the new
client attaches and streams. The crash reproduces reliably, so a clean run is the
proof.

---

## S3 — Breaker gives up on an absent daemon (resilience)

### Root cause

`run_supervisor` counts every failure toward the circuit breaker, including
`attach()` failing because the daemon is simply not there yet (frame-sub socket
`ECONNREFUSED`). No SDK call happens in that path. The breaker exists to bound
VI_OPEN/VENC_OPEN churn against a *present* daemon; counting "daemon absent"
toward give-up makes degraded boot fragile — a daemon that starts late enough
trips `attach_given_up` and requires manual intervention.

### Fix

In `platform/anyka/supervisor.rs`, only `initialize()` failures call
`breaker.record_failure()`:

```
loop:
  attach() Err            -> publish(Unavailable); backoff; (NO record_failure)
  attach() Ok, init Err   -> rollback; detach; record_failure; backoff
  attach() Ok, init Ok    -> publish(Available); reset breaker; wait_for_loss
```

An absent, late, or mid-restart daemon is retried forever with backoff. A
live-but-wedged daemon still trips the breaker at `ATTACH_FAILURE_LIMIT`. The
`ATTACH_FAILURE_LIMIT` / `BACKOFF_MAX` placeholders are left as-is: they now only
govern the SDK-churn case they were written for, and are no longer what breaks
degraded boot.

### Verification

Unit test: `attach()` fails N > `ATTACH_FAILURE_LIMIT` times → breaker stays
closed, retries continue, no `GivenUp`. On-device S3: start `onvif-rust` with no
daemon, start the daemon 30 s+ later, confirm attach succeeds and `onvif-rust`
never restarted.

---

## S4 — Poller is a backstop, not the sole idle detector (documentation)

### Finding

Task 11 assumed the epoch poller is the only detector while idle ("push stopped,
no RTSP client, no frame traffic"). On hardware, push stays active regardless of
RTSP clients, so a daemon kill trips frame-socket errors within milliseconds and
the poller's 1 s tick never wins. The poller is not broken; its premise is.

### Decision

No behavior change. Keep the poller (~1 volatile read/sec): it still catches
races the socket path misses — a daemon restart that re-stamps the ring before
the frame reader notices the old socket died, and a half-dead daemon whose socket
never EOFs. Detection is layered:

- frame-socket EOF/error — milliseconds — **primary** when push is active
- control-socket error — on the next control request
- epoch poller — 1 s — **backstop** for restart races / non-EOF death

Fix the misleading comments in `supervisor.rs` (Task 11 block) and
`watch_epoch_until_loss` to describe this layering.

`idle-stop-push` (stop push when the last subscriber leaves, restart on the next)
is explicitly **out of scope**. It is a separate feature justified by CPU savings,
with its own cost (first-client start latency), and is not required for restart
resilience. If added later it would make the poller the sole idle detector, which
is what Task 11 originally imagined.

### Verification

Covered by re-running S1/S4 on device: confirm layered detection still recovers a
daemon restart cleanly (no flap, epoch changes, RTSP resumes).

---

## Out of scope

- `idle-stop-push` (see S4).
- Tuning `ATTACH_FAILURE_LIMIT` / `BACKOFF_MAX` from measured data — the plan's
  open item; deferred until there is a reason to change the SDK-churn budget.
- Analyzing the second coredump (`core.venc_capture.2001`, from the pre-fix S1
  flap) — the S2 core is decisive and the flap is already fixed.
