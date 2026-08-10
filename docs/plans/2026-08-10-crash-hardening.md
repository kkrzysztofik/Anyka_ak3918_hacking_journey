# Crash Hardening Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make camera `192.168.30.121` recover from a wedged video pipeline without a human power-cycling it.

**Architecture:** Every component that cannot do its job exits, so anyka-init's existing restart machinery (backoff → crash-loop → reboot → storm-guard safe mode) is finally triggered. **Safety nets are built first (Tasks 1–6), crash-only behaviour second (Tasks 7–9)** — so that if the crash-only changes misbehave, the nets already catch it. Night-mode and observability follow.

**Tech Stack:** Rust (vendored toolchain, `armv5te-unknown-linux-uclibceabi` target, host tests on `x86_64-unknown-linux-gnu`), C99 for `vendor-daemon` (ARMv5TE / uClibc), busybox `sh` on device.

**Design doc:** `docs/plans/2026-08-10-crash-hardening-design.md`

---

## Before You Start

Read these first. They contain facts that are not obvious from the code:

- `docs/plans/2026-08-10-crash-hardening-design.md` — the diagnosis this plan implements
- `.serena/memories/development-standards.md`, `testing-framework.md`, `quality-gates.md`

**Every Rust command in this plan assumes you have run this first, from the repo root:**

```bash
source ./setenv.sh
```

That exports `$CARGO` (the vendored toolchain) and puts `toolchain/arm-anykav200-crosstool-ng/bin` first on `PATH`. Without the `PATH` ordering, `cargo clippy` fails with `E0514`.

**Verified working commands** (run from `cross-compile/`):

```bash
$CARGO test -p anyka-init --target x86_64-unknown-linux-gnu --lib      # 169 tests, ~0.3s
$CARGO test -p onvif-rust --target x86_64-unknown-linux-gnu --lib
$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
$CARGO fmt --check
```

**Device access** (`.121` is only reachable through a jumphost):

```bash
ssh root@192.168.3.137
telnet 192.168.30.121 24     # drops straight to a root shell, no login
```

Pace input to telnet — see `.serena/memories/` and the `anyka-remote-debugging` skill.

**Do not restart onvif-rust alone on the device.** It lands detached and dark. Always:

```bash
kill $(pidof onvif-rust.bin) $(pidof vendor-daemon.bin)
```

---

## Task 1: Reboot on kernel panic

The kernel currently has `/proc/sys/kernel/panic = 0`, meaning it **halts forever** on a panic instead of rebooting. Two sysctl writes fix it.

**Files:**
- Modify: `cross-compile/anyka-init/src/boot.rs`
- Test: same file, `#[cfg(test)] mod tests`

**Step 1: Write the failing test**

Add to the tests module in `boot.rs`:

```rust
#[test]
fn test_write_panic_sysctls_writes_both_values() {
    let dir = tempfile::tempdir().expect("tempdir");
    let kernel = dir.path().join("kernel");
    std::fs::create_dir_all(&kernel).expect("mkdir");
    std::fs::write(kernel.join("panic"), "0\n").expect("seed panic");
    std::fs::write(kernel.join("panic_on_oops"), "0\n").expect("seed oops");

    write_panic_sysctls(dir.path());

    assert_eq!(
        std::fs::read_to_string(kernel.join("panic")).expect("read panic"),
        "10"
    );
    assert_eq!(
        std::fs::read_to_string(kernel.join("panic_on_oops")).expect("read oops"),
        "1"
    );
}

#[test]
fn test_write_panic_sysctls_does_not_panic_when_the_knobs_are_absent() {
    let dir = tempfile::tempdir().expect("tempdir");
    // No kernel/ subdirectory at all — must not panic.
    write_panic_sysctls(dir.path());
}
```

**Step 2: Run test to verify it fails**

```bash
cd cross-compile && $CARGO test -p anyka-init --target x86_64-unknown-linux-gnu --lib write_panic_sysctls
```

Expected: FAIL, `cannot find function 'write_panic_sysctls'`.

**Step 3: Write minimal implementation**

Add to `boot.rs`:

```rust
/// Reboot 10s after a panic instead of halting forever.
///
/// This kernel ships `kernel.panic = 0`, so a panic leaves the camera dead
/// until someone power-cycles it. `proc_root` is a parameter only so the test
/// can point at a tempdir; production passes `/proc/sys`.
///
/// Best-effort: a missing knob is normal on other kernels and must not abort
/// boot.
pub fn write_panic_sysctls(proc_root: &std::path::Path) {
    for (knob, value) in [("kernel/panic", "10"), ("kernel/panic_on_oops", "1")] {
        let path = proc_root.join(knob);
        if let Err(e) = std::fs::write(&path, value) {
            tracing::warn!(knob, error = %e, "could not set panic sysctl");
        }
    }
}
```

**Step 4: Run test to verify it passes**

```bash
cd cross-compile && $CARGO test -p anyka-init --target x86_64-unknown-linux-gnu --lib write_panic_sysctls
```

Expected: PASS, 2 tests.

**Step 5: Call it during boot**

Call it from `main.rs`, under the `// P2` marker, immediately before `boot::system_setup(...)`:

```rust
    // P2
    boot::write_panic_sysctls(std::path::Path::new("/proc/sys"));
    let probed = boot::system_setup(sysimpl.as_ref(), &cfg);
```

**Not** inside `system_setup`. That function takes an injected `Sys` specifically so it can be tested, and every other side effect in it routes through `sys` or a config-supplied path. A hardcoded `/proc/sys` write inside it means the host test suite mutates the real machine's kernel settings when run as root.

**Step 6: Full test + lint**

```bash
cd cross-compile && $CARGO test -p anyka-init --target x86_64-unknown-linux-gnu --lib
cd cross-compile && $CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
```

Expected: all pass.

**Step 7: Commit**

```bash
rtk git add cross-compile/anyka-init/src/boot.rs
rtk git commit -m "feat(anyka-init): reboot 10s after a kernel panic

kernel.panic was 0, so a panic halted the camera until someone
power-cycled it. Also sets panic_on_oops.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

## Task 2: Protect the supervisor from the OOM killer

anyka-init runs at `oom_score_adj = 0` while the OOM killer is demonstrably active on this box. If it is chosen, every service is orphaned and only a power cycle recovers.

**Files:**
- Modify: `cross-compile/anyka-init/src/boot.rs`
- Test: same file

**Step 1: Write the failing test**

```rust
#[test]
fn test_protect_from_oom_killer_writes_the_minimum_score() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("oom_score_adj");
    std::fs::write(&path, "0\n").expect("seed");

    protect_from_oom_killer(&path);

    assert_eq!(std::fs::read_to_string(&path).expect("read"), "-1000");
}
```

**Step 2: Run test to verify it fails**

```bash
cd cross-compile && $CARGO test -p anyka-init --target x86_64-unknown-linux-gnu --lib protect_from_oom
```

Expected: FAIL, function not found.

**Step 3: Write minimal implementation**

```rust
/// Make this process the OOM killer's last choice.
///
/// The supervisor is the one process whose death cannot be recovered from in
/// software: `/usr/sbin/service.sh` has no respawn loop, so if anyka-init dies
/// every service is orphaned. `path` is a parameter for testability;
/// production passes `/proc/self/oom_score_adj`.
pub fn protect_from_oom_killer(path: &std::path::Path) {
    if let Err(e) = std::fs::write(path, "-1000") {
        tracing::warn!(error = %e, "could not set oom_score_adj");
    }
}
```

**Step 4: Run test to verify it passes**

Expected: PASS.

**Step 5: Call it at startup**

Call it from `main.rs`, immediately **before** the Task 1 `write_panic_sysctls` line — protection matters most before any allocation happens:

```rust
    // P2
    boot::protect_from_oom_killer(std::path::Path::new("/proc/self/oom_score_adj"));
    boot::write_panic_sysctls(std::path::Path::new("/proc/sys"));
    let probed = boot::system_setup(sysimpl.as_ref(), &cfg);
```

**Not** inside `system_setup`, for the same reason as Task 1: it would make the host test suite mutate the real machine when run as root. `/proc/self/oom_score_adj` belongs to the supervisor process even more clearly than the panic knobs belong to "system setup".

**Step 6: Full test + lint, then commit**

```bash
rtk git add cross-compile/anyka-init/src/boot.rs
rtk git commit -m "feat(anyka-init): set oom_score_adj=-1000 on the supervisor

service.sh has no respawn loop, so an OOM-killed supervisor orphans
every service and needs a power cycle.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

## Task 3: Add SIGKILL escalation to service restarts

`supervisor_loop.rs:237` sends **SIGTERM only** and discards the result (`let _ = sys.kill(...)`). There is no escalation and no verification, so a restart request against a wedged daemon silently does nothing.

**Files:**
- Modify: `cross-compile/anyka-init/src/supervisor_loop.rs:32-38` (the `Msg` enum) and `:234-248` (the handler)
- Test: `cross-compile/anyka-init/tests/supervision.rs`

**Step 1: Add the message variant**

In the `Msg` enum:

```rust
    /// Escalation from the monitor when `RestartService` did not take. Sends
    /// SIGKILL. A task wedged in D state will not die even from this — the
    /// monitor's next rung is a reboot, which does not need the process to die.
    KillService(String),
```

**Step 2: Write the failing test**

In `tests/supervision.rs`, mirroring the existing restart-request test:

```rust
#[test]
fn test_kill_service_sends_sigkill_to_the_named_service() {
    // Follow the arrangement used by the existing
    // test_run_restart_message_for_unknown_service_is_ignored test in
    // supervisor_loop.rs: build a MockSys, expect kill() with SIGKILL.
    // Assert the signal argument is libc::SIGKILL, not SIGTERM.
}
```

Fill this in following the existing mock setup in `supervisor_loop.rs::run_tests`.

**Step 3: Run test to verify it fails**

```bash
cd cross-compile && $CARGO test -p anyka-init --target x86_64-unknown-linux-gnu kill_service
```

Expected: FAIL — `KillService` unhandled / no matching expectation.

**Step 4: Implement the handler**

Next to the existing `Msg::RestartService` arm:

```rust
            Ok(Msg::KillService(name)) => match services.iter().find(|s| s.name == name) {
                Some(svc) => match svc.state.pid() {
                    Some(pid) => {
                        tracing::warn!(service = %name, pid, "SIGTERM did not take; sending SIGKILL");
                        let _ = sys.kill(pid, libc::SIGKILL);
                    }
                    None => tracing::info!(
                        service = %name,
                        "kill requested but the service is not running"
                    ),
                },
                None => tracing::warn!(service = %name, "kill requested for unknown service"),
            },
```

**Step 5: Run tests**

```bash
cd cross-compile && $CARGO test -p anyka-init --target x86_64-unknown-linux-gnu
```

Expected: PASS.

**Step 6: Commit**

```bash
rtk git add cross-compile/anyka-init/src/supervisor_loop.rs cross-compile/anyka-init/tests/supervision.rs
rtk git commit -m "feat(anyka-init): add SIGKILL escalation for service restarts

RestartService sent SIGTERM once and discarded the result, so a restart
request against a wedged daemon silently did nothing.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

## Task 4: Write a video heartbeat from vendor-daemon

push.c already tracks `frames_pushed` and logs it every 300 frames. Write it to tmpfs at the same point so anyka-init can observe frame flow.

**Files:**
- Modify: `cross-compile/vendor-daemon/src/push.c:418-424`
- Modify: `cross-compile/vendor-daemon/src/globals.h` (path constant)

**No unit test.** This is one `fopen`/`fprintf`/`fclose` and the logic being protected lives in the Rust ladder (Task 5), which is exhaustively tested on the host. Verified on device in Task 15. `tests/test_ring_epoch.c` remains the only C test.

**Step 1: Add the path constant**

In `globals.h`, near `PUSH_NO_DATA_EXIT_THRESHOLD`:

```c
/* Liveness beacon read by anyka-init's monitor. tmpfs, so no SD writes. */
#define PUSH_HEARTBEAT_PATH "/tmp/vd_heartbeat"
```

**Step 2: Write the counter**

In `push_frame_thread`, inside the existing every-300-frames block at `push.c:418`:

```c
        if (frames_pushed > 0 && (frames_pushed % 300) == 0) {
            log_info("[push] stream=%u frames=%llu no_data=%llu",
                     state->stream_id,
                     (unsigned long long)frames_pushed,
                     (unsigned long long)no_data_count);

            /* Only the main stream drives liveness: if it stalls we are
             * broken regardless of what the sub stream is doing. */
            if (state->stream_id == 0) {
                FILE *hb = fopen(PUSH_HEARTBEAT_PATH, "w");
                if (hb) {
                    fprintf(hb, "%llu\n", (unsigned long long)frames_pushed);
                    fclose(hb);
                }
            }
        }
```

Confirm `<stdio.h>` is already included in `push.c`; add it if not.

**Step 3: Build**

```bash
source ./setenv.sh
cd cross-compile/vendor-daemon && make release
```

Expected: builds clean, no new warnings.

**Step 4: Commit**

```bash
rtk git add cross-compile/vendor-daemon/src/push.c cross-compile/vendor-daemon/src/globals.h
rtk git commit -m "feat(vendor-daemon): write a frame-count heartbeat to tmpfs

anyka-init's monitor needs a work-product liveness signal; process
liveness is not enough because a wedged daemon stays alive.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

## Task 5: Video-liveness decision function (pure)

The decision logic, separate from any I/O, mirroring `netstat::decide`.

**Files:**
- Modify: `cross-compile/anyka-init/src/monitor.rs`

**Why not a new module:** the wifi decision lives in `netstat.rs` because it is network logic with several helpers. This is one function and one enum — a new file would be scaffolding.

**Step 1: Write the failing tests**

In `monitor.rs` tests:

```rust
fn video_policy() -> VideoPolicy {
    VideoPolicy {
        restart_after_ticks: 2,
        kill_after_ticks: 3,
        reboot_after_ticks: 5,
    }
}

#[test]
fn test_video_decide_does_nothing_while_frames_advance() {
    assert_eq!(video_decide(0, &video_policy()), VideoAction::Nothing);
}

#[test]
fn test_video_decide_restarts_at_the_restart_threshold() {
    assert_eq!(video_decide(2, &video_policy()), VideoAction::Restart);
}

#[test]
fn test_video_decide_escalates_to_kill_then_reboot() {
    assert_eq!(video_decide(3, &video_policy()), VideoAction::Kill);
    assert_eq!(video_decide(4, &video_policy()), VideoAction::Kill);
    assert_eq!(video_decide(5, &video_policy()), VideoAction::Reboot);
    assert_eq!(video_decide(50, &video_policy()), VideoAction::Reboot);
}

#[test]
fn test_video_decide_below_the_first_threshold_is_nothing() {
    assert_eq!(video_decide(1, &video_policy()), VideoAction::Nothing);
}
```

**Step 2: Run to verify failure**

```bash
cd cross-compile && $CARGO test -p anyka-init --target x86_64-unknown-linux-gnu --lib video_decide
```

Expected: FAIL, types not found.

**Step 3: Implement**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VideoPolicy {
    pub restart_after_ticks: u32,
    pub kill_after_ticks: u32,
    pub reboot_after_ticks: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoAction {
    Nothing,
    Restart,
    Kill,
    Reboot,
}

/// Escalation ladder for a stalled video pipeline.
///
/// Ordered strongest-first, like `netstat::decide`, so absolute thresholds
/// cannot skip a rung. Three rungs rather than two because `RestartService`
/// sends SIGTERM, which a wedged daemon can ignore; the reboot rung does not
/// need the process to die at all.
pub fn video_decide(stalled_ticks: u32, p: &VideoPolicy) -> VideoAction {
    if stalled_ticks >= p.reboot_after_ticks {
        return VideoAction::Reboot;
    }
    if stalled_ticks >= p.kill_after_ticks {
        return VideoAction::Kill;
    }
    if stalled_ticks >= p.restart_after_ticks {
        return VideoAction::Restart;
    }
    VideoAction::Nothing
}
```

**Step 4: Run to verify pass, then commit**

```bash
rtk git add cross-compile/anyka-init/src/monitor.rs
rtk git commit -m "feat(anyka-init): add the pure video-liveness escalation ladder

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

## Task 6: Wire the video ladder into the monitor

**Files:**
- Modify: `cross-compile/anyka-init/src/config.rs:168-186` (`MonitorCfg`)
- Modify: `cross-compile/anyka-init/src/monitor.rs` (`apply_video_actions`, `tick`, `run`)
- Modify: `SD_card_contents/anyka_hack/anyka.toml` (document the new keys)

**Step 1: Add config knobs**

In `MonitorCfg`:

```rust
    #[serde(default = "d_true")]
    pub video: bool,
    #[serde(default = "d_video_restart_ticks")]
    pub video_restart_after_ticks: u32,
    #[serde(default = "d_video_kill_ticks")]
    pub video_kill_after_ticks: u32,
    #[serde(default = "d_video_reboot_ticks")]
    pub video_reboot_after_ticks: u32,
    #[serde(default = "d_video_heartbeat")]
    pub video_heartbeat_path: String,
```

with defaults `2`, `3`, `5`, `"/tmp/vd_heartbeat"`. Follow the existing `d_*` helper style at the bottom of `config.rs`.

**Step 2: Write the failing tests**

```rust
#[test]
fn test_apply_video_actions_absent_heartbeat_is_not_a_stall() {
    // Push threads start on CMD_VENC_START_PUSH from onvif-rust, so there is
    // a legitimate no-frames window at startup and when streaming is off.
    // An absent counter must never escalate.
    let mut ticks = 3;
    let (tx, rx) = std::sync::mpsc::channel();
    let sys = MockSys::new(); // no reboot expectation: must not be called

    apply_video_actions(&sys, &cfg_with_video(), &tx, None, &mut ticks);

    assert_eq!(ticks, 0, "absent heartbeat resets rather than escalates");
    assert!(rx.try_recv().is_err(), "no message must be sent");
}

#[test]
fn test_apply_video_actions_advancing_counter_resets_ticks() {
    let mut ticks = 4;
    let (tx, _rx) = std::sync::mpsc::channel();
    let sys = MockSys::new();

    apply_video_actions(&sys, &cfg_with_video(), &tx, Some(1000), &mut ticks);

    assert_eq!(ticks, 0);
}

#[test]
fn test_apply_video_actions_stalled_counter_escalates_to_restart() {
    let (tx, rx) = std::sync::mpsc::channel();
    let sys = MockSys::new();
    let cfg = cfg_with_video();
    let mut ticks = 0;

    // Same value three times: first call seeds, next two are stalls.
    apply_video_actions(&sys, &cfg, &tx, Some(500), &mut ticks);
    apply_video_actions(&sys, &cfg, &tx, Some(500), &mut ticks);
    apply_video_actions(&sys, &cfg, &tx, Some(500), &mut ticks);

    assert!(matches!(
        rx.try_recv(),
        Ok(Msg::RestartService(ref s)) if s == "vendor-daemon"
    ));
}

#[test]
fn test_apply_video_actions_reboots_when_the_stall_persists() {
    let (tx, _rx) = std::sync::mpsc::channel();
    let mut sys = MockSys::new();
    sys.expect_reboot().times(1).returning(|| Ok(()));
    let cfg = cfg_with_video();
    let mut ticks = 5;

    apply_video_actions(&sys, &cfg, &tx, Some(500), &mut ticks);
}
```

You will need a `last: &mut Option<u64>` to hold the previous counter — thread it through like `ticks`.

**Step 3: Run to verify failure**

```bash
cd cross-compile && $CARGO test -p anyka-init --target x86_64-unknown-linux-gnu --lib apply_video_actions
```

**Step 4: Implement**

```rust
/// Read the heartbeat counter. `None` means "no signal yet", which is not a
/// stall — see the test.
fn read_heartbeat(path: &str) -> Option<u64> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
}

/// Video escalation for one tick.
///
/// Compares consecutive counter values rather than checking mtime: P2.5 steps
/// the wall clock by decades, so any mtime-based liveness would either fire
/// instantly or never (see the note in `supervise.rs`).
pub fn apply_video_actions(
    sys: &dyn Sys,
    cfg: &MonitorCfg,
    tx: &Sender<Msg>,
    frames: Option<u64>,
    last: &mut Option<u64>,
    ticks: &mut u32,
) {
    let Some(frames) = frames else {
        *ticks = 0;
        *last = None;
        return;
    };
    if *last != Some(frames) {
        *ticks = 0;
        *last = Some(frames);
    } else {
        *ticks = ticks.saturating_add(1);
        tracing::warn!(frames, ticks = *ticks, "video frames stalled");
    }

    let policy = VideoPolicy {
        restart_after_ticks: cfg.video_restart_after_ticks,
        kill_after_ticks: cfg.video_kill_after_ticks,
        reboot_after_ticks: cfg.video_reboot_after_ticks,
    };
    match video_decide(*ticks, &policy) {
        VideoAction::Nothing => {}
        VideoAction::Restart => {
            let _ = tx.send(Msg::RestartService("vendor-daemon".into()));
        }
        VideoAction::Kill => {
            let _ = tx.send(Msg::KillService("vendor-daemon".into()));
        }
        VideoAction::Reboot => {
            tracing::error!(ticks = *ticks, "video stalled past the reboot threshold; rebooting");
            if let Err(e) = sys.reboot() {
                tracing::error!(error = %e, "reboot() returned without rebooting");
            }
        }
    }
}
```

Call it from `tick`, guarded by `cfg.video`, threading `last` and a second tick counter through `run` exactly as `ticks` is threaded today.

**Step 5: Run all tests + clippy + fmt**

```bash
cd cross-compile && $CARGO test -p anyka-init --target x86_64-unknown-linux-gnu
cd cross-compile && $CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
cd cross-compile && $CARGO fmt --check
```

**Step 6: Document the keys** in `SD_card_contents/anyka_hack/anyka.toml` under `[monitor]`, with a comment explaining that an absent heartbeat is not a stall.

**Step 7: Commit**

```bash
rtk git add cross-compile/anyka-init/src/monitor.rs cross-compile/anyka-init/src/config.rs SD_card_contents/anyka_hack/anyka.toml
rtk git commit -m "feat(anyka-init): escalate on a stalled video pipeline

The supervisor only watched for process exit, so a daemon that stayed
alive holding a dead pipeline was invisible to it for 14.5 hours.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

## Task 7: Make `exit_no_data` fatal, with a safe threshold

**Files:**
- Modify: `cross-compile/vendor-daemon/src/globals.h:66`
- Modify: `cross-compile/vendor-daemon/src/push.c:188-195`

**Step 1: Raise the threshold**

`PUSH_NO_DATA_EXIT_THRESHOLD (1000) × PUSH_POLL_SLEEP_MS (5)` is **5 seconds**, while push.c's own comment notes ISP day/night stalls of 0.5–2 s — only 2.5× margin. Making that fatal unchanged risks a daemon restart every dusk.

```c
/* 6000 * PUSH_POLL_SLEEP_MS = 30s. Fatal, so the margin over a legitimate
 * ISP day/night stall (0.5-2s per the note in push.c) has to be large.
 * Calibration knob: lower it only with dusk evidence. */
#define PUSH_NO_DATA_EXIT_THRESHOLD 6000
```

**Step 2: Exit the process**

Replace `state->active = 0; break;` at `push.c:194-195` with:

```c
                /* Crash-only: the pipeline is dead and nothing in this process
                 * can rebuild it. Exiting hands recovery to anyka-init, whose
                 * backoff/crash-loop/storm-guard policy already exists. The
                 * kernel closing /dev/ion, /dev/video0 and /dev/uio0 cleans the
                 * SDK state better than vd_obj_close_all() does. */
                _exit(1);
```

`_exit`, not `exit` — `exit` runs atexit handlers concurrently with live threads. Ensure `<unistd.h>` is included.

**Step 3: Build**

```bash
source ./setenv.sh && cd cross-compile/vendor-daemon && make release
```

**Step 4: Commit**

```bash
rtk git add cross-compile/vendor-daemon/src/push.c cross-compile/vendor-daemon/src/globals.h
rtk git commit -m "fix(vendor-daemon): exit when the stream stops delivering frames

Previously both push threads exited and the process limped on holding a
dead pipeline, invisible to the supervisor. Threshold raised 5s -> 30s
because it is now fatal.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

## Task 8: Exit on control-client loss, delete the object sweep

**Files:**
- Modify: `cross-compile/vendor-daemon/src/main.c:497-510`
- Modify: `cross-compile/vendor-daemon/src/dispatcher.c:143-167`
- Modify: `cross-compile/vendor-daemon/src/dispatcher.h`

**Step 1: Exit instead of sweeping**

In `main.c`, replace the `ret == -1` arm body:

```c
                    if (ret == -1) {
                        /* Crash-only: this daemon exists to serve one control
                         * client. The sweep that used to run here could not
                         * fully clean the SDK -- the next client's
                         * ak_venc_request_stream returned null. Exiting lets
                         * the kernel close /dev/ion, /dev/video0 and /dev/uio0,
                         * and anyka-init restarts the pair. */
                        log_info("[daemon] control client fd=%d disconnected; exiting", client_fd);
                        _exit(1);
                    }
```

**Step 2: Delete `release_control`**

Remove it from `dispatcher.c` and `dispatcher.h`, and remove the remaining call at `main.c:525`. If `vd_obj_close_all()` has no other callers after this, delete it too.

```bash
rg -n "release_control|vd_obj_close_all" cross-compile/vendor-daemon/src/
```

Only remove `vd_obj_close_all` if that search shows no surviving caller. **Do not delete any file** — only these functions.

**Step 3: Build and check for unused-symbol warnings**

```bash
source ./setenv.sh && cd cross-compile/vendor-daemon && make clean && make release
```

**Step 4: Commit**

```bash
rtk git add cross-compile/vendor-daemon/src/
rtk git commit -m "fix(vendor-daemon): exit on control-client loss

Replaces the leaked-object sweep, which existed only because onvif-rust
never sends CLOSE and left the SDK dirty enough that the next
ak_venc_request_stream returned null. Net code reduction.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

## Task 9: Exit onvif-rust when the attach breaker gives up

The breaker already latches open at `ATTACH_FAILURE_LIMIT = 10`, publishes `Availability::GivenUp` and returns — but nothing consumes `GivenUp`, so the process stays alive and dark forever.

**Files:**
- Modify: `cross-compile/onvif-rust/src/platform/anyka/supervisor.rs` (or its consumer)
- Test: same file

**Design constraint:** the exit must be **injectable**. Do not put `process::exit` inside the supervisor loop — that would make the existing `test_supervisor_init_failures_opens_breaker_gives_up` unrunnable. Add a pure decision function and unit-test that; the caller performs the exit.

**Step 1: Write the failing test**

```rust
#[test]
fn test_given_up_is_fatal_but_unavailable_is_not() {
    assert!(availability_is_fatal(Availability::GivenUp));
    assert!(!availability_is_fatal(Availability::Unavailable));
    assert!(!availability_is_fatal(Availability::Available));
}
```

**Step 2: Run to verify failure**

```bash
cd cross-compile && $CARGO test -p onvif-rust --target x86_64-unknown-linux-gnu --lib availability_is_fatal
```

**Step 3: Implement**

```rust
/// `GivenUp` means the breaker latched open: the daemon is alive but the
/// pipeline cannot be built. Staying alive in that state leaves the camera
/// dark with no supervisor visibility, so it is fatal. `Unavailable` is
/// transient and must not be.
pub fn availability_is_fatal(a: Availability) -> bool {
    matches!(a, Availability::GivenUp)
}
```

**Step 4: Wire the exit**

Find where the `watch::Receiver<Availability>` is consumed (starts at `platform/anyka/mod.rs:284`). Add a task that watches for a fatal value and terminates the process with a non-zero status after logging:

```rust
tracing::error!(
    event = "attach_given_up_fatal",
    "vendor-daemon attach gave up; exiting so the supervisor can restart the pair"
);
std::process::exit(1);
```

**Step 5: Run the full onvif-rust suite**

```bash
cd cross-compile && $CARGO test -p onvif-rust --target x86_64-unknown-linux-gnu
cd cross-compile && $CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
```

Expected: all pass, including the pre-existing `test_supervisor_init_failures_opens_breaker_gives_up`.

**Step 6: Commit**

```bash
rtk git add cross-compile/onvif-rust/src/
rtk git commit -m "fix(onvif-rust): exit when the attach breaker gives up

The breaker latched open and published GivenUp, but nothing consumed it,
so the process stayed alive with no video and the supervisor never saw
an exit.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

## Task 10: Require explicit calibration for the ain0 fallback

A raw ADC value is board-specific: `.198` reads 648–670, `.121` reads 548–639. Shipping `.198`'s numbers as a code default is why `.121` classified **night at noon**.

**Files:**
- Modify: `cross-compile/onvif-rust/src/config/types.rs:615-649`
- Modify: `cross-compile/onvif-rust/src/platform/anyka/night_mode.rs:412-431`
- Test: both files

**Step 1: Write the failing test** in `night_mode.rs`:

```rust
#[tokio::test]
async fn test_tick_holds_when_ae_fails_and_ain0_is_uncalibrated() {
    // AE unavailable past the streak limit, thresholds unset: the fallback
    // must not run. An unavailable AE reading is not evidence about light,
    // and guessing with another board's numbers put the IR illuminator on at
    // midday.
    // Arrange a NightMode whose cfg has day_threshold: None,
    // night_threshold: None and an ffi whose get_ae_luma returns None.
    // Assert no GPIO write and no set_ir_filter call occur.
}
```

**Step 2: Run to verify failure.**

**Step 3: Implement**

In `types.rs`, change both fields to `Option<i32>` and **remove them from `Default`**:

```rust
    /// Raw `ain0` reading at or above which the board is in daylight.
    ///
    /// No default: this is a raw ADC value and is board-specific (.198 reads
    /// 648-670, .121 reads 548-639). An uncalibrated board holds instead of
    /// guessing. See wiki/IR-Night-Mode-Calibration.md.
    pub day_threshold: Option<i32>,
    /// Raw `ain0` reading at or below which the board is in darkness.
    pub night_threshold: Option<i32>,
```

In `night_mode.rs`, in the `None` arm of `tick()`:

```rust
                let (Some(day), Some(night)) = (self.cfg.day_threshold, self.cfg.night_threshold)
                else {
                    return;
                };
                let Some(raw) = read_light_sensor(&self.paths) else {
                    return;
                };
                classify(
                    raw,
                    Thresholds {
                        day,
                        night,
                        ldr_high_is_day: self.cfg.ldr_high_is_day,
                    },
                )
```

**AE thresholds keep their defaults** (`28`/`8`). AE correctly reported day on `.121` all afternoon using `.198`'s numbers, so that value is portable across these boards; only the raw ADC is not.

**Step 4: Update the shipped templates** — comment out `day_threshold`/`night_threshold` in `SD_card_contents/anyka_hack/onvif/config.toml` with a note that they must be calibrated per board.

**Step 5: Run tests, clippy, fmt. Commit.**

```bash
rtk git commit -m "fix(onvif-rust): require explicit ain0 calibration for night mode

.198's raw ADC thresholds shipped as code defaults, so .121 classified
night at noon whenever AE luma was briefly unavailable.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

## Task 11: Reconcile night state from GPIO at startup

In-memory state dies with the process; the GPIO does not. Observed on device: IR illuminator on while the ISP sat at `fps 15` (day).

**Files:**
- Modify: `cross-compile/onvif-rust/src/platform/anyka/night_mode.rs` (`new`, around line 268)
- Test: same file

**Step 1: Write the failing test**

```rust
#[test]
fn test_initial_state_follows_the_ir_led_gpio() {
    // paths.node(Node::IrLed) contains "1" -> initial state is Night.
    // Contains "0" -> Day. Absent -> Day.
}
```

**Step 2–4:** implement a `read_initial_state(paths: &NodePaths) -> DayNight` helper, use it in `new()` in place of the current fixed initial state, run tests, commit.

```bash
rtk git commit -m "fix(onvif-rust): seed night state from the IR_LED GPIO at startup

GPIO state survives a process restart but the in-memory state did not,
leaving the illuminator orphaned on in daylight.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

## Task 12: Slow the night-mode poll

**Files:** `cross-compile/onvif-rust/src/platform/anyka/night_mode.rs:237`

```rust
/// AUTO poll cadence.
///
/// Every tick is an IPC round-trip through the daemon's single-threaded poll
/// loop. 2s meant 30 round-trips a minute, forever, to read a light level that
/// changes twice a day.
const POLL_INTERVAL: Duration = Duration::from_secs(10);
```

Check whether any existing test asserts the 2 s cadence and update it. Run tests, commit.

---

## Task 13: Stop the per-frame SPS/PPS warning

Measured on device: this one line makes info-level logging cost **184 MB/day**, which would itself cause the failure it is meant to observe.

**Files:**
- Modify: `cross-compile/onvif-rust/src/streaming/bridge.rs:551`
- Test: same file

**Step 1:** write a test asserting the warning is emitted once per stream, not once per frame — e.g. a counter/flag on the bridge state that suppresses repeats, reset when a stream restarts.

**Step 2–4:** implement, run, commit.

```bash
rtk git commit -m "fix(onvif-rust): log the missing-SPS/PPS warning once per stream

At frame rate this single line cost 184 MB/day of SD writes, making
info-level logging unusable on this device.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

## Task 14: Ship working logging defaults

**Gated on Task 13.** Do not start until the measurement below passes.

**Step 1: Measure on device**

Set `level = "warn"` and a `/mnt/logs/...` `file_path` on `.121`, then:

```bash
ls -la /mnt/logs/onvif-debug.log.*     # note size
# wait 10 minutes
ls -la /mnt/logs/onvif-debug.log.*     # note size again
```

Growth must project to **under ~1 MB/day**. If not, return to Task 13.

**Step 2:** change the shipped defaults in `SD_card_contents/anyka_hack/onvif/config.toml` and the `config/types.rs` default from `/tmp/onvif.log` to the SD path with `level = "warn"`. Today's shipped default produces no logs at all.

**Step 3:** run tests, commit.

---

## Task 15: Respawn the supervisor, and device acceptance

**Files:**
- Modify: `SD_card_contents/anyka_hack/` copy of `config.sh` (the device copy is `/mnt/Factory/config.sh`; rollback is `config.sh.gerge.bak`)

**Step 1: Add a respawn loop**

Replace the single invocation of `anyka-init.bin` with:

```sh
# service.sh has no respawn. If the supervisor dies, every service is
# orphaned and only a power cycle recovers. anyka-init's own reboot path
# uses libc::reboot and does not return, so this loop cannot fight it.
while :; do
    /mnt/anyka_hack/anyka-init.bin
    sleep 5
done &
```

**Step 2: Deploy** the rebuilt `anyka-init.bin`, `vendor-daemon.bin`, `onvif-rust.bin` and `config.sh` per the `anyka-embedded-build` skill. Back up whatever you overwrite.

**Step 3: Acceptance tests on `.121`** — the exact scenarios that failed on 2026-08-10:

```bash
# 1. vendor-daemon alone
kill $(pidof vendor-daemon.bin)
# expect: both daemons back, streaming, within ~60s, no human action

# 2. onvif-rust alone  (today this leaves it detached and dark)
kill $(pidof onvif-rust.bin)
# expect: the pair recovers

# 3. supervisor death
kill $(pidof anyka-init.bin)
# expect: respawned within ~5s

# 4. liveness ladder
#    confirm /tmp/vd_heartbeat advances, then verify a stall escalates
cat /tmp/vd_heartbeat; sleep 30; cat /tmp/vd_heartbeat
```

**Step 4: Watch a dusk transition** (~19:26 UTC on `.121`) and confirm no restart loop. Check `/mnt/logs/watch121.log` and `/mnt/logs/vendor_daemon.log`.

**Step 5: Commit** the SD-card changes.

---

## After Implementation

- Run `/code-review` on the branch.
- Update `.serena/memories/` and the `121-daily-crash-dusk-vi-collapse` memory with the outcome.
- Remove the temporary watcher from `.121` (`/mnt/logs/watch121.sh`) once the built-in liveness ladder is confirmed working — it is duplicate instrumentation at that point.
- **Still open, not addressed by this plan:** what stopped `ak_venc_get_stream` at 19:27:56 on 2026-08-09. This plan makes that failure recoverable, not impossible. The watcher data should be reviewed before closing the investigation.
