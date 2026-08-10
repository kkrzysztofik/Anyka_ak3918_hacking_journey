# Crash Hardening — Design

Date: 2026-08-10
Status: proposed

## Problem

Camera `192.168.30.121` becomes unreachable roughly daily and only recovers
when a human power-cycles it. Diagnosed 2026-08-10; the full investigation is
in the `121-daily-crash-dusk-vi-collapse` memory.

The failure chain, from `/mnt/logs/vendor_daemon.log.1` (2026-08-09):

1. `19:27:56` — `ak_venc_get_stream` stops yielding frames; both push threads
   log `push_get_stream_error sdk_ret=-1`, `no_data_count` climbing 200/s.
2. `19:28:00` — `push.c:189 state=exit_no_data` at 1000. Both push threads
   exit **permanently**; venc closes, `vi_set_capture_off`. Nothing restarts
   capture.
3. An SDK-internal thread then spins on `ak_vi_get_frame` —
   `[ak_vi_get_frame:1339] must call after capture on` appears **1775 times**,
   each one a write to the vfat SD card.
4. `19:28:58` — onvif-rust's control socket drops. `release_control()` →
   `vd_obj_close_all()` tears down the VI and both VENCs. onvif-rust
   reattaches, the daemon rebuilds the whole pipeline (~7 s of blocking work
   in a single-threaded poll loop), bring-up fails, repeat: **9 rebuilds
   between 19:29:06 and 19:36:49**.
5. `19:37 → 10:00` — wedged. load1 3.5 → 15 at **40 % idle CPU** (≈10 threads
   in D state), and anyka-init's `thread::sleep(60)` (`monitor.rs:194`) taking
   **223 s**.
6. `10:00:16` — `vendor-daemon st=Signal(9)`; the box is power-cycled ~14 s
   later.

Reproduced live on 2026-08-10 11:15 by restarting onvif-rust alone.

### Why nothing recovered

Every mechanism needed to recover **already exists and was never triggered**:

- `anyka-init/src/supervise.rs` holds the entire restart policy as a pure,
  host-testable function: backoff, crash-loop cap, reboot.
- `storm.rs` guards against reboot storms by dropping to safe mode — telnet,
  logging and the monitor only — and waiting for a human.
- `platform/anyka/supervisor.rs` already has `Backoff` (500 ms → 15 s) and a
  `CircuitBreaker` that latches open at `ATTACH_FAILURE_LIMIT = 10`.

The gap is that **no component ever exits**. push.c kills its threads and
leaves the process alive. The breaker opens, publishes
`Availability::GivenUp`, and returns — and nothing consumes `GivenUp` to
terminate, so onvif-rust sits alive forever with no video. anyka-init only
watches for process exit, so it never learns anything is wrong.

Aug 9 produced 9 VI rebuilds; the breaker limit is 10. The numbers line up.

### The camera never reboots itself

`/proc/sys/kernel/panic = 0` — on panic this kernel halts forever rather than
rebooting. And every boot recorded in `anyka-init.log` falls in waking hours:

```
Aug 3 12:59   Aug 3 21:22   Aug 5 10:19   Aug 7 19:58   Aug 9 08:38   Aug 10 10:01
```

Not one overnight boot, despite the wedge starting at 19:26. A self-resetting
box would show random hours. This is a human power-cycling it on noticing.

Stated as a hypothesis from the boot-hour distribution, not proof — but it
matches the reported symptom exactly.

## Principle

> **A component that cannot do its job exits. Nothing limps, and nothing gives
> up while still alive.**

Every item below is an application of that. No new supervisor, no new config
subsystem, no hardware watchdog.

The design **removes** more than it adds. `release_control()`'s cleanup sweep
and `vd_obj_close_all()` exist only because "onvif-rust never sends CLOSE, and
under SIGKILL no Drop runs" (`dispatcher.c:146-166`). If the daemon exits
instead, the kernel closes `/dev/ion`, `/dev/video0` and `/dev/uio0` and does
that cleanup properly — demonstrably better than the sweep, which today leaves
the SDK dirty enough that the next `ak_venc_request_stream` returns null.

## Scope

### Spine

| ID | Change | Location |
|----|--------|----------|
| S1 | Control-client loss ⇒ daemon exits. Deletes the leaked-object sweep. | `vendor-daemon/src/main.c:500`, `dispatcher.c:160` |
| S2 | `exit_no_data` ⇒ `_exit(1)`, threshold raised to 30 s | `push.c:188`, `globals.h:66` |
| S3 | `Availability::GivenUp` ⇒ exit non-zero | `platform/anyka/supervisor.rs:207` + consumer |
| S4 | Video-liveness ladder | `push.c:419` + `anyka-init/src/monitor.rs` |

**S2 threshold.** `PUSH_NO_DATA_EXIT_THRESHOLD (1000) × PUSH_POLL_SLEEP_MS (5)`
is **5 s**, while push.c's own comment notes ISP day/night stalls of
0.5–2 s — only 2.5× margin. Making that fatal unchanged would risk a daemon
restart every dusk. Raise to `6000` (30 s) as part of making it fatal: far
beyond any legitimate stall, still 30 s instead of 14 hours. This is a
hardware-timing calibration knob, not a magic number.

**S2 exit mechanics.** `_exit(1)`, not `exit(1)` — `exit` runs atexit handlers
concurrently with live threads.

**S4 signal.** push.c already tracks `frames_pushed` and logs it every 300
frames (`push.c:419`). Write it to `/tmp/vd_heartbeat` at the same point:
tmpfs, one line, no SD writes.

The monitor **compares consecutive values rather than checking mtime.**
`supervise.rs` warns that the wall clock steps by decades once timesync lands,
so any mtime-based liveness would either fire instantly or never. Content
comparison is immune.

Push threads start on `CMD_VENC_START_PUSH` (19) from onvif-rust at bring-up,
**not** per RTSP client, so frames flow whenever the pipeline is up regardless
of client count. But a no-frames window is legitimate at startup and when
streaming is disabled: **an absent counter is not a stall**, and ticks reset
after any restart.

Ladder shape copies the existing wifi ladder in `monitor.rs`:

```
stalled >= video_stall_restart_ticks (2)  -> SIGTERM
stalled >= video_stall_kill_ticks    (3)  -> SIGKILL
stalled >= video_stall_reboot_ticks  (5)  -> Reboot
```

Three rungs, not two, because `RestartService` today is
`let _ = sys.kill(pid, libc::SIGTERM)` — SIGTERM only, result discarded, no
escalation and no verification (`supervisor_loop.rs:237`). Against a wedged
daemon that silently does nothing, which would make every liveness event cost
a full reboot.

### Survivability

| ID | Change | Rationale |
|----|--------|-----------|
| V1 | `kernel.panic = 10`, `panic_on_oops = 1` at boot | Kernel currently halts forever on panic. Two sysctl writes, no false-positive risk. |
| V2 | anyka-init sets its own `oom_score_adj = -1000` | Runs at `0` today while the OOM killer is demonstrably active on this box. |
| V3 | Respawn loop for anyka-init in `config.sh` | `/usr/sbin/service.sh` has no respawn. If the supervisor dies, every service is orphaned — a guaranteed power cycle. |

V2 and V3 protect the component the entire design rests on.

### Night mode

| ID | Change | Rationale |
|----|--------|-----------|
| N1 | `day_threshold`/`night_threshold` become `Option<i32>` with no defaults; unset ⇒ ain0 fallback disabled, `tick()` holds | A raw ADC value is board-specific: `.198` reads 648–670, `.121` reads 548–639. Shipping `.198`'s numbers as a code default is why `.121` classified **night at noon**. |
| N2 | Reconcile night state from GPIO at startup (read `IR_LED`) | In-memory state dies with the process; the GPIO does not. Fixes an orphaned illuminator from any cause. |
| N3 | `POLL_INTERVAL` 2 s → 10 s | 30 IPC round-trips/min through a single-threaded poll loop to read a light level that changes twice a day. |

**AE thresholds keep their defaults** (`ae_day_threshold = 28`,
`ae_night_threshold = 8`). AE correctly reported day on `.121` all afternoon
using `.198`'s numbers, so that value is portable across these boards. Only
the raw ADC is not. The rule is applied where the evidence supports it.

**Deliberately not changed:** the `record_change`-on-ISP-failure behaviour
(`night_mode.rs:369-387`). The existing comment defends it: not recording
means re-driving the ircut solenoid every tick, which is physically worse.
N1 removes the spurious night decision and the spine turns IPC failures into
restarts, so the path that made it harmful is gone; N2 cleans up the residue.

### Observability

| ID | Change | Rationale |
|----|--------|-----------|
| O1 | `bridge.rs:551` IDR-missing-SPS/PPS warn fires once per stream, not per frame | This single line makes info-level logging cost **184 MB/day** (measured). |
| O2 | Shipped default `file_path` → SD path, `level = "warn"` | Today's shipped default is `/tmp/onvif.log` with console off — no logs at all. |

O2 lands **after** O1 is measured. Gate: 10 minutes at `warn` must project to
under ~1 MB/day.

## Out of scope

- **Hardware watchdog.** `/dev/watchdog` exists and nothing holds it, but
  `ak_drv_wdt.c:31` documents "driver limit feed_time max to 8 second". At an
  8 s cap the feed cannot be conditional on application health without false
  reboots — a VI rebuild alone blocks ~7 s. An unconditional feed only detects
  a kernel that has stopped scheduling userspace, which we have never
  observed: anyka-init was scheduling fine for the whole 14.5 h outage. V1
  covers the panic case for free. Revisit only if a genuine kernel hang is
  observed.
- **`panic_on_oom`.** An OOM kill that frees memory is survivable; panicking
  on it is heavier than the problem.
- **Log append-vs-truncate.** Service logs are already rotated on every start
  (`logging::rotate_if_needed` in the `Action::Start` path) — which is exactly
  how the Aug 9 evidence survived in `.1`.
- **Hung-task detector.** `CONFIG_DETECT_HUNG_TASK` is not compiled into this
  kernel; verified on device, the sysctls do not exist.

## Failure flow

```
frames stop       -> S2 push no_data(30s)  -> _exit(1)              ~30s
bring-up fails    -> S3 breaker GivenUp    -> exit(1)               ~2min
control lost      -> S1 daemon exits                                immediate
none of the above -> S4 liveness ladder    -> TERM/KILL/reboot      2-5min
supervisor dies   -> V2 oom_score_adj + V3 respawn
kernel panics     -> V1 kernel.panic=10    -> reboot                10s
```

Each rung is strictly stronger than the one above. A persistent fault escalates
through backoff (30/60 s) → `crashloop_count` → reboot → three fast reboots →
storm guard → safe mode with telnet, waiting for a human. **No power cycle at
any point.**

## Testing

- **Policy is pure and host-tested.** The video-liveness ladder is table-driven
  tests over tick sequences in `monitor.rs`, mirroring the existing
  `test_apply_wifi_actions_*`; escalation goes in `tests/supervision.rs`.
- **N1**: one test — uncalibrated ain0 ⇒ hold.
- **S3**: the exit must be injectable. The supervisor publishes `GivenUp`, a
  thin consumer decides to exit, and the consumer's decision is unit-tested —
  not `process::exit` buried in the loop.
- **C changes stay untested in-tree.** S1/S2 are exit calls and a constant, and
  the heartbeat is one write. The logic lives in the Rust ladder and that is
  where the tests go; `tests/test_ring_epoch.c` remains the only C test.
- **Device acceptance**, the exact scenarios that failed on 2026-08-10:
  1. Kill vendor-daemon alone → both daemons recover streaming with no human
     action.
  2. Kill onvif-rust alone → the pair recovers. Today this leaves it detached
     and dark.
  3. Both survive a dusk transition.

## Open items

- **What stopped `ak_venc_get_stream` at 19:27:56** is still unknown. `[isp]
  set_ir_filter` is `log_debug`, invisible at the daemon's info level. A 20 s
  watcher is running on `.121` (`/mnt/logs/watch121.sh` →
  `/mnt/logs/watch121.log`) sampling `ir ain free cached dirty wb nproc load
  rss_onvif rss_vd`. The evening `Signal(9)` cluster (Aug 4 17:36/19:34,
  Aug 5 18:09/19:40/21:54 UTC) **predates** the IR-LED binary deployed Aug 7
  21:20, so night-mode control is not the sole trigger.
- **Why the box was unreachable** is not established. `min_free_kbytes = 2048`
  means MemFree at ~2–3 MB is the kernel's watermark by design, not
  exhaustion, which weakens the "could not fork" explanation — with 20 MB of
  clean page cache, fork should succeed. The watcher forks four processes
  every 20 s; if it keeps writing through the next wedge, fork was never the
  problem.
- **Calibrate `.121`'s ain0** from tonight's watcher data (N1 makes an
  uncalibrated board hold rather than guess, so this is a follow-up, not a
  blocker).
