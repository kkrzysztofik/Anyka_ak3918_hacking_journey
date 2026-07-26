# Fix Plan: async/hardware layer separation — `cross-compile/onvif-rust`

Source: architecture review of the request layer (axum/dispatcher/services) vs. the
action layer (PAL/HAL) on the AK3918 target (2 tokio workers, 16 blocking threads,
32MB RAM). Priority order below = execution order.

---

## Phase 1 — F1 (CRITICAL): PTZ actor + interruptible motor wait

**Problem:** `NativePtzDriver::turn()` (`src/hal/anyka/ptz/driver.rs:375-401`) blocks in
`select()`/`read()` up to 60s waiting for motor completion **while holding the driver
`Mutex`**, called synchronously from async trait methods in
`src/platform/anyka/ptz_control.rs`. One move parks worker 1; a concurrent `Stop`
parks worker 2 on the same mutex → whole server frozen; `Stop` cannot preempt a move.

**Tasks:**
1. `driver.rs`: make `wait_event` interruptible — replace the single 60s `select()`
   with a loop of ~100ms selects checking an `AtomicBool` stop flag; on flag set,
   issue `TURN_STOP` ioctl and return `Ok(interrupted)`.
2. `driver.rs`: split the stop path off the main mutex — keep motor fds reachable for
   `TURN_STOP` without acquiring the lock held by an in-flight `turn` (raw-fd stop or
   a separate stop-only mutex per motor; `AK_MOTOR_TURN_STOP` is safe to issue
   concurrently with a pending turn).
3. New PTZ actor: one `std::thread` owning both `MotorHandle`s, fed by a **bounded**
   `tokio::sync::mpsc` of `PtzCommand { MoveTo, Continuous, Stop }` with
   `tokio::sync::oneshot` replies.
   - Supersede semantics: on receipt, drain the queue; keep only the newest movement
     command; `Stop` always wins and triggers the interrupt flag for any in-flight turn.
4. `platform/anyka/ptz_control.rs`: async trait methods become thin
   `send + await oneshot` shims wrapped in `tokio::time::timeout` (return
   `PlatformError::Timeout` → SOAP fault instead of a frozen worker).
   - `move_to_position` returns once the command is *accepted* (ONVIF treats moves as
     asynchronous); completion updates tracked position.
5. Position truth: after each completed/interrupted turn, reconcile tracked position
   from `MOTOR_GET_STATUS` instead of dead-reckoning writes on ioctl submission.
6. Remove the now-redundant `spawn_blocking` stop in the 10s timeout task; route it
   through the actor's `Stop` command.

**Pass criteria:**
- `ContinuousMove` followed 100ms later by `Stop`: motor stops < 300ms; both requests
  return; server answers an unrelated `GetDeviceInformation` concurrently < 1s.
- Burst of 20 `ContinuousMove`s: only newest executes; no queue pile-up.
- Existing mockall tests in `ptz_control.rs` updated and green.

## Phase 2 — F2 (HIGH): IPC owner thread for the vendor-daemon control socket

**Problem:** `AnykaIpc` control socket = `Arc<Mutex<UnixStream>>` with 10s
read/write timeouts (`src/hal/anyka/ipc/mod.rs:284-289`), shared by video/audio/
imaging (`src/platform/anyka/mod.rs:96`). `AnykaImagingControl::set_settings` does 4
sequential sync RPCs inside an async fn (`src/platform/anyka/imaging.rs:151-170`) —
up to ~80s worst case on a worker thread, convoying streaming IDR requests behind it.

**Tasks:**
1. New IPC owner thread inside `AnykaIpc`: owns the `UnixStream`, consumes a bounded
   mpsc of `(IpcRequest, oneshot::Sender<IpcResponse>)`. Reconnect logic stays in the
   owner thread.
2. Public IPC methods become async `send + await oneshot` with `tokio::time::timeout`
   (slightly above `IPC_CTRL_TIMEOUT`).
3. Callers that are sync-by-need (the `venc-read` frame thread issuing IDR/venc
   commands) get a sync variant using `blocking_send` + `oneshot::blocking_recv` —
   fine, they are OS threads, not executor tasks.
4. Delete the `Mutex<UnixStream>` once all callers are migrated.

**Pass criteria:**
- With vendor daemon paused (SIGSTOP), `SetImagingSettings` returns a SOAP fault in
  ~10s and the server keeps answering other requests throughout.
- No regression in frame throughput (venc-read path unchanged in behavior).

## Phase 3 — F3 (HIGH): move all sync persistence off the executor

**Problem:** sync SD-card writes (incl. `sync_all`) inside async handlers:
- `src/onvif/imaging/store.rs:446-492` (`persist_settings`)
- `src/onvif/media/profile_manager.rs:957` (`persist_all` → `ProfileStorage::save`)
- `src/onvif/device/ops/users.rs:144,191,253` (`save_to_toml`)

**Tasks:**
1. Reuse the existing debounced pattern (`src/config/persistence.rs`): give imaging
   store, profile manager, and user storage a `request_save()`-style handle whose
   background task snapshots state under the lock and writes on a blocking thread.
   Either generalize `ConfigPersistenceService` over a `Snapshot + Save` closure or
   add three small instances — prefer generalizing only if it stays < ~50 lines.
2. Wrap the actual `fs` writes (including the config service's own
   `ConfigStorage::save`) in `tokio::task::spawn_blocking`.
3. Keep atomic write pattern (tmp + `sync_all` + `rename`) — it is correct, just off
   the executor.

**Pass criteria:** `SetVideoEncoderConfiguration` p99 latency independent of SD-card
fsync latency (verify with `strace`/timing on device); settings still survive power
cycle.

## Phase 4 — F6 (MEDIUM): bounded fanout to streamhub

**Problem:** bridge→streamhub uses `mpsc::UnboundedSender<FrameData>`
(`src/streaming/service.rs:231-248`, also validation fanout in `src/main.rs`). A
stalled streamhub loop accumulates frames unbounded → `cap` hard limit →
`handle_alloc_error` abort on 32MB.

**Tasks:**
1. Replace unbounded fanout channels with the existing drop-oldest policy — reuse
   `LowLatencyFrameQueue` (`src/streaming/bridge.rs`) or a bounded mpsc with
   `try_send` + drop-oldest-on-full and a dropped-frame counter in telemetry.

**Pass criteria:** with streamhub artificially stalled 5s, RSS stays flat and stream
recovers with a gap instead of the process aborting.

## Phase 5 — F5 (MEDIUM): cancellation + state truth (mostly falls out of Phase 1)

**Tasks:**
1. `src/onvif/ptz/ops/movement.rs:59`: stop setting `state.set_position(&position)`
   before the hardware move; update state from the platform's reconciled position
   (or on command-accepted, mark `moving` and let `GetStatus` read live position).
2. Verify `TimeoutLayer` now actually fires during hardware ops (it will, once
   Phases 1–2 remove sync blocking from polls).

## Phase 6 — F8 (LOW): observability gap

**Tasks:**
1. Health monitor: track `stream_health` frame-counter liveness at runtime (not just
   startup) so a dead `venc-read` thread flips health to degraded.

---

## Explicitly deferred (YAGNI)

- Generic actor framework — three concrete owner threads (PTZ, IPC, persistence) are
  the whole need.
- Live-apply of `SetVideoEncoderConfiguration` to the hardware encoder — currently
  config-only-until-restart. **Constraint for whoever wires it:** the encoder restart
  path (`stop_streaming`, `src/platform/anyka/video_encoder.rs:1354+`: grace-period
  sleeps + thread joins) must run behind an owner thread, never in a handler.
- Runtime thread-count tuning — 2 workers is right once blocking work is off the
  executor.

## Global pass criteria (regression gate for the whole plan)

Concurrent load on device: 1 RTSP subscriber + WebUI polling + `ContinuousMove`/`Stop`
loop + `SetImagingSettings` loop for 10 min → no request > 15s, no frozen executor
(watchdog never fires), RSS stable, motors respond to Stop < 300ms.
