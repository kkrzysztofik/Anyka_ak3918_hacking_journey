# Boot/Runtime System in Rust — Design

Date: 2026-08-01
Status: approved (design), implementation plan pending

## Problem

The camera's boot and runtime layer is ~1,700 lines of untested POSIX shell
spread over nine files. It has no supervision: `gergehack.sh` is a one-shot
fan-out that starts services once and never reaps them. The only recovery
mechanism in the entire design is `periodic_reboot.sh` — reboot-as-supervision.

Concrete defects on the current path:

| # | Defect | Location |
|---|--------|----------|
| D1 | No supervision. A dead service stays dead until the next reboot | `gergehack.sh` |
| D2 | Config is `.`-sourced, so an SD card any user can write is unsandboxed root RCE at boot | `gergehack.sh:320` |
| D3 | `pgrep -f "vendor-daemon.bin"` idempotency matches zombies and the `grep` itself; a crash becomes permanent silent death claiming `exit 0` | `run_vendor_daemon.sh:104` |
| D4 | `cp` then `reboot` with a bare `sync` onto vfat — the window that corrupts FAT directory entries | `gergehack.sh:141-142`, `:171-172` |
| D5 | Every `LD_LIBRARY_PATH` export inherits and forwards, poisoning children across two incompatible uClibc versions | `run_vendor_daemon.sh:126`, cf. `onvif/onvif-rust:3-5` |
| D6 | Fallback chains swallow errors (`\|\| true`, `\|\| log DEBUG`), so failures are indistinguishable from success | `common.sh` throughout |
| D7 | `ntpd` is fire-and-forget and unlogged. A wrong clock silently rejects 100% of authenticated ONVIF requests | `gergehack.sh:357`, `ws_security.rs:85` |
| D8 | `load_camera_modules()` re-`insmod`s modules the vendor already loaded, and swallows the guaranteed failure | `common.sh:295-297` |

Replace it with one supervised Rust binary.

## Scope

| # | Requirement |
|---|-------------|
| R1 | Replace `gergehack.sh`, `common.sh`, `init_logs.sh`, `sys_monitor.sh`, `periodic_reboot.sh` and the three `run_*.sh` launchers with one binary |
| R2 | Supervise every service: reap, restart with backoff, cap crash loops |
| R3 | Support Mode A only (SD `Factory` exploit, unmodified rootfs). Drop `rootfs_modified=1` |
| R4 | Config is TOML, parsed — never evaluated. The supervisor is the only config reader |
| R5 | Ad-hoc diagnostic scripts stay shell and are not touched |
| R6 | Time sync moves in-process; no dependency on busybox `ntpd` |
| R7 | Policy logic must be testable on `x86_64` without hardware |

Non-goals: Mode B (modified rootfs), a control socket / status API, porting the
`ffmpeg` or `libre_anyka_app` wrappers, fixing `onvif-rust`'s `GetNTP` path.

## Findings that drive the design

**1. The boot chain, verified from vendor source.**

```
rcS: umask 022; telnetd &
 └─ rc.local
     ├─ mount /usr(squashfs) /etc/jffs2(jffs2 mtd6) /data(jffs2 mtd7)
     ├─ tcpsvd 0 21 ftpd &   syslogd -O /var/log/messages -s 200 -b 3 &
     ├─ camera.sh setup   -> sensor_*.ko from /etc/jffs2, /usr/modules,
     │                       /data/sensor_ko_and_isp_conf
     │                       akcamera.ko + ak_info_dump.ko
     │                       (mounts AND unmounts /mnt in passing)
     └─ service.sh start &                                   [BACKGROUNDED]
          mount /dev/mmcblk0p1 /mnt;  test -d /mnt/Factory -> FACTORY_TEST=1
          cmd_serverd (vendor, already up);  close_white_led;  killall telnetd
          core_pattern = /mnt/core_%e_%p_%t
          FACTORY_TEST=1 -> /mnt/Factory/config.sh   [foreground call, not exec]
```

Four consequences:

- `rc.local:34` backgrounds `service.sh`, and the `FACTORY_TEST=1` branch has
  nothing meaningful after the `config.sh` call (line 98 is an `echo`, lines
  141-148 are a check whose actions are all commented out). **A resident
  supervisor that never returns is safe.** No `&`, no PID file, no detach.
- `service.sh:85` runs `killall telnetd`, killing the `telnetd &` from `rcS:8`,
  immediately before calling `config.sh`. Restarting telnet is therefore
  **restoring a channel the vendor just tore down**, not belt-and-braces.
- `camera.sh:78-79` already `insmod`s `akcamera.ko` and `ak_info_dump.ko` before
  `service.sh` runs. D8 confirmed: that code is dead.
- The `FACTORY_TEST=1` branch skips both `anyka_ipc` and the vendor `daemon`
  watchdog (`service.sh:121`, else-branch). Nothing external will kill us, and
  the `run_ipc` / `rootfs_modified` settings are unreachable.

**2. `config.sh` is exec'd by path, so an ELF works — but Rust output is not
self-contained.**

```
vendor-daemon.bin  ELF ARM EABI5, dynamic, interp /lib/ld-uClibc.so.0
dropbearmulti      ELF ARM EABI5, static
onvif-rust.bin     ELF ARM EABI5, dynamic, interp /mnt/anyka_hack/lib/ld-uClibc.so.1
```

The toolchain sysroot is `arm-unknown-linux-uclibcgnueabi`; its output needs a
*newer uClibc bundled on the SD card*, while stock system binaries use uClibc
0.9.33.2. An ELF at `/mnt/Factory/config.sh` would therefore run only if
`/mnt/anyka_hack/lib/` were intact — making the survival phase depend on a
second directory.

Exec-from-SD itself is sound: `/mnt` is mounted `mount -rw <dev> /mnt` with no
`noexec`; vfat takes its mode from the mounting process's umask, which `rcS:12`
sets to `022` → 0755; and ARM ELFs already execute from `/mnt` today.

**3. Two incompatible libcs coexist, and leaking a loader path breaks busybox.**
`SD_card_contents/anyka_hack/onvif/onvif-rust` is a wrapper whose comment records
the bug: *"Do NOT export LD_LIBRARY_PATH here; it poisons child processes
(busybox/date linked against stock uClibc 0.9.33.2)."* `env_clear()` plus a
per-service `env` map is the structural fix (D5).

**4. `onvif-rust` already tolerates peer restarts, so no dependency graph is
needed.** `docs/plans/2026-07-29-vendor-daemon-restart-resilience-design.md`
states R4 "process respawn is out of scope — assumed handled externally" and R5
"`onvif-rust` boots degraded without `vendor-daemon`; cold start and recovery
share one code path". This supervisor is that "externally", and R5 removes the
need for readiness gates, ordering constraints and restart cascades.

**5. A wrong clock breaks ONVIF auth outright.** `ws_security.rs:85` sets
`clock_skew_seconds: 300`; `:234-239` rejects any `Created` timestamp outside
±5 minutes of `Utc::now()`. At the epoch, every authenticated request fails.
`network_info.rs:147` only *reads* NTP config to answer `GetNTP` — nothing on
this device sets the clock in-process.

**6. Only three surviving scripts depend on the shared helpers.**
`verify_logs.sh`, `ffmpeg/app_restarter.sh`, `ffmpeg/wrap_mp4.sh` source
`common.sh`/`init_logs.sh`. The other diagnostics are self-contained.

## Decisions

| Axis | Decision |
|------|----------|
| Scope | Init + supervisor core; diagnostics stay shell |
| Process model | Long-running resident supervisor |
| Config | TOML, typed; supervisor is the only reader |
| Shell bridge | Env injected into children at spawn; no shell reads config |
| Runtime | `std` only, blocking threads, no tokio |
| Boot mode | Mode A only |
| Entry artifact | ~10-line shell wrapper at `Factory/config.sh` + ELF at `anyka_hack/anyka-init.bin` |
| Restart policy | Flat independent restarts; crash-loop cap → reboot, guarded |
| Time sync | In-process SNTP, replaces `ntpd` |
| Testing | `Sys` trait seam + `mockall`; host integration tests |
| Delivery | Big-bang port |

## Architecture

```
service.sh -> /mnt/Factory/config.sh          [/bin/sh, ~10 L, stock loader only]
                telnetd -p 24 -l /bin/sh &                              P0
                exec /mnt/anyka_hack/anyka-init.bin
                     │
              anyka-init.bin  [ELF ARM EABI5, RPATH]
                P1 CONFIG     parse /mnt/anyka_hack/anyka.toml
                              fail => log FATAL to file + stderr, PARK
                P2 SYSTEM     TZ; insmod [system].sensor_module if set & absent
                              wifi creds -> /etc/jffs2/anyka_cfg.ini if differs
                              wifi_manage.sh start
                              kill telnetd / tcpsvd per config
                P2.5 TIME     bounded synchronous first SNTP sync
                P3 SPAWN      per service: fork+exec, env_clear + envs,
                              log redirect, RLIMIT_CORE
                P4 SUPERVISE  never returns
```

P0 lives in shell deliberately. It is the one phase that must not fail, and shell
has no dependency on the bundled loader. A missing or mismatched binary becomes a
logged message rather than `execve` failing with nothing running.

P1 failure deliberately does **not** fall through with defaults. Defaulting
`wifi_ssid` or `sensor_module` would join a wrong network and `insmod` a wrong
kernel module. Halting with telnet up is the safe stop.

### Crate layout

New workspace member `cross-compile/anyka-init/`.

```
src/main.rs      phase sequencing
   config.rs     TOML -> Config (serde), defaults, validation
   sys.rs        trait Sys + RealSys (libc); the only unsafe in the crate
   supervise.rs  SvcState, pure decide(), backoff, cap, storm guard
   boot.rs       P2: tz, sensor module, wifi creds, service kills
   timesync.rs   P2.5 + resync thread; SNTP query + pure parse_response()
   monitor.rs    /proc sampling thread
   logging.rs    tracing subscriber -> size-capped file
```

Build artifact `anyka-init`, deployed to
`SD_card_contents/anyka_hack/anyka-init.bin`.

### Threading

```
reaper   loop { waitpid(-1, 0) -> tx.send(Exited{pid, status}) }   blocking
signals  signal_hook::Signals[TERM,INT] -> tx.send(Shutdown)
monitor  loop { sample /proc -> log; sleep(interval) }
timesync loop { sntp; sleep(resync_interval) }
main     loop { match rx.recv_timeout(next_backoff_deadline) { .. } }
```

Moving the blocking `waitpid` into its own thread and turning child exits into
channel messages makes `recv_timeout(next_deadline)` the single wait point — no
SIGCHLD handler, no self-pipe, no `WNOHANG` polling. Thread stacks pinned to
64 KiB via `Builder::stack_size`; there is no reason to reserve Rust's 2 MiB
default four times over on a 36 MB device.

## Config schema

```toml
[log]
dir = "/mnt/logs";  level = "info";  max_bytes = 2000000;  keep = 2

[system]
# /data/sensor/ is NOT on any of camera.sh's three search paths
# (/etc/jffs2, /usr/modules, /data/sensor_ko_and_isp_conf), which is why
# this insmod is load-bearing and not a duplicate of camera.sh:37-38.
sensor_module = "/data/sensor/sensor_gc1084.ko"
telnet = false      # kill the P0 recovery telnetd once booted
ftp = true

[wifi]
ssid = "..."; password = "..."; config_file = "/etc/jffs2/anyka_cfg.ini"

[time]
enabled = true
servers = ["0.ubuntu.pool.ntp.org", "1.ubuntu.pool.ntp.org"]
timezone = "GMT+01:00"
first_sync_timeout_sec = 15
retry_interval_sec     = 30
resync_interval_sec    = 21600
step_threshold_sec     = 2
min_plausible_unix     = 1767225600   # 2026-01-01
max_plausible_unix     = 2524608000   # 2050-01-01

[supervisor]
backoff_min_sec = 1;   backoff_max_sec = 60
crashloop_count = 10;  crashloop_window_sec = 600
storm_guard_max_reboots = 3
storm_guard_state = "/mnt/anyka_hack/state/boot.json"

[monitor] enabled = true;  interval_sec = 60
[reboot]  enabled = false; interval_min = 720; jitter_max_sec = 0

[services.vendor-daemon]
enabled = true
exec = "/mnt/anyka_hack/vendor-daemon/vendor-daemon.bin"
log  = "/mnt/logs/vendor_daemon.log"
core_dump = true
env = { LD_LIBRARY_PATH = "/mnt/anyka_hack/vendor-daemon/lib",
        VENDOR_DAEMON_LOG_LEVEL = "info" }

[services.onvif]
enabled = true
exec = "/mnt/anyka_hack/onvif/onvif-rust.bin"
log  = "/mnt/logs/onvif.log"
core_dump = true

[services.dropbear]
enabled = false
exec = "/mnt/anyka_hack/dropbear/dropbearmulti"
args = ["dropbear", "-F", "-E", "-p", "22"]
log  = "/mnt/logs/dropbear.log"
```

Service tables have exactly six keys: `enabled, exec, args, env, log, core_dump`.
No per-service restart policy, no user/group, no readiness probe — policy is
global, everything runs as root, and finding 4 removes the need for probes.

**Deleted keys:** `rootfs_modified`, `run_ipc` (unreachable, finding 1);
`extra_modules` (D8); `run_web_interface`, `image_width`, `image_height`,
`md_record_sec`, `extra_args` (move into the owning service's `env`); `ssh_*`
(collapse into `services.dropbear.args`).

## Supervision semantics

```rust
enum SvcState { Running { pid: Pid, since: Instant },
                Backoff { until: Instant, attempt: u32 } }

enum Action  { Spawn, Sleep(Duration), Reboot(Reason), SafeMode, None }

fn decide(st: &SvcState, hist: &RestartHistory, ev: Event, now: Instant) -> Action
```

- **Backoff** `min(backoff_min << (attempt-1), backoff_max)` → 1,2,4,…,60,60.
- **Stability reset** `attempt = 0` once a service has run longer than
  `backoff_max`. The threshold sits above the ceiling on purpose, so a service
  dying every 59 s still escalates instead of resetting forever.
- **Crash-loop cap** sliding window: `>= crashloop_count` restarts within
  `crashloop_window_sec` → `Reboot`.
- **All clocks monotonic.** P2.5 steps the wall clock by decades; backoff or
  window logic built on `SystemTime` would either fire instantly or never.

### Reboot-storm guard

Reboot-on-cap without a guard is an unattended camera power-cycling forever.

```
/mnt/anyka_hack/state/boot.json   { fast_reboots: u8 }

on crash-loop reboot: fast_reboots += 1; write tmp + rename + sync; reboot
on start:             fast_reboots >= 3  ->  SAFE MODE
                                             skip P3 entirely, log FATAL
                                             telnet + logging + monitor only
monitor tick:         uptime > 10 min    ->  fast_reboots = 0
```

Bounded at three crash-loop reboots, then the camera parks in a diagnosable state
with a shell rather than cycling forever.

### Shutdown

SIGTERM/SIGINT → SIGTERM all children → wait 5 s → SIGKILL stragglers → exit.
Nothing on the camera sends these (finding 1), but host integration tests do.

## Time sync

Replaces `ntpd -n -N -p <server> &`.

P2.5 attempts a bounded synchronous first sync (retry every
`retry_interval_sec` until `first_sync_timeout_sec`), then proceeds to P3
regardless. Wifi association usually completes in a few seconds, so the common
path hands `onvif-rust` a correct clock at spawn. A slow network costs 15 s of
boot and the background thread corrects it later.

Stepping the clock *before* spawning `onvif-rust` also matters because
`ws_security.rs:264-288` expires its replay-nonce cache against `Utc::now()`; a
large step underneath a live process either purges the cache wholesale or
freezes it.

Implementation splits I/O from validation:

```rust
fn query(server: &str, timeout: Duration) -> Result<[u8; 48]>          // ~15 L
fn parse_response(pkt: &[u8; 48], sent_nonce: u64, bounds: &Bounds)    // pure
    -> Result<SystemTime, NtpError>
```

An NTP response is unauthenticated UDP from the network. `parse_response`
rejects:

| Check | Rejects |
|---|---|
| `len == 48`, mode `== 4` | malformed / non-server replies |
| `LI != 3` | server advertising itself unsynchronised |
| `stratum` in `1..=15` | `0` kiss-o'-death, `16` unsynced |
| originate timestamp `== sent_nonce` | **off-path spoofing.** We send a random 64-bit value from `/dev/urandom` as our transmit timestamp; a real server echoes it verbatim |
| transmit timestamp `!= 0` | empty replies |
| result within `[min_plausible, max_plausible]` | a broken or hostile server pushing the clock to 1970 or 2106 — re-breaking the ONVIF auth this phase exists to fix |

Clock is **stepped**, not slewed, behind `Sys::set_realtime()`, only when
`|delta| > step_threshold_sec`, and every step is logged with the delta.

## Logging

```
anyka-init   -> /mnt/logs/anyka-init.log     tracing, size-capped
             -> stderr for FATAL / SafeMode  (reaches the boot console)
services     -> [services.X].log             fd opened by supervisor,
                                             O_APPEND, dup2 at spawn
```

Rotation is **size-based, never time-based**. `tracing-appender`'s
`Rotation::DAILY` names files from wall-clock time, and P2.5 steps the clock from
the epoch to the real date mid-boot — producing `anyka-init.log.1970-01-01` for
the boot record and a discontinuity at every boundary.

Service logs rotate **at spawn only**:

```
spawn(svc):
  if size(svc.log) > log.max_bytes: rename .log -> .log.1 (keep N)
  open O_APPEND|O_CREAT -> dup2 to child stdout + stderr
```

This is a deliberate ceiling. The supervisor holds the child's fd, so renaming
the file underneath a running child leaves the child writing to the renamed
inode; doing it properly requires reopening and `dup2`ing, which is only possible
at exec time. It self-corrects for a crash-looping service (rotates on every
restart) and leaks for a stable, chatty one. Marked with a `ponytail:` comment
naming the ceiling.

## Error handling

| Failure | Response |
|---|---|
| P1 config parse/validate | log FATAL to file **and** stderr, park. Never substitute defaults |
| P2 `insmod` sensor | log ERROR, continue. Video dead; ONVIF/SSH/logs still useful for diagnosis |
| P2 wifi cfg rewrite | backup → write → verify → restore-on-failure (port existing logic) |
| P2 `wifi_manage.sh` | log WARN, continue |
| P2.5 no sync within timeout | log WARN, continue; background thread retries |
| P3 exec fails (ENOENT/EACCES) | treated as an immediate child exit → normal backoff, no special case |
| panic in supervisor | panic hook writes log + stderr, then dies. Children are orphaned but keep running — degrading exactly to today's permanent behaviour |

No `unwrap`/`expect` outside tests. `thiserror` for `ConfigError` and `NtpError`,
`anyhow` at phase boundaries.

## Testing

**Unit, x86_64, `MockSys`.** `decide()` is total and therefore exhaustively
testable: backoff sequence; stability reset only above `backoff_max`; cap fires
at the window boundary and not one restart earlier; storm-guard transitions
0→1→2→3→SafeMode and the uptime reset. Plus config parse (valid / unknown key /
bad type / missing required / not-TOML); env map construction including
`env_clear`; `anyka_cfg.ini` rewrite as a pure string transform with golden
input/output; `parse_response` against byte fixtures for every rejection row
above.

**Integration, x86_64, `RealSys`.** Real children under a compressed test config:
`/bin/false` → immediate exit → backoff observed; `/bin/sleep 300` → stable →
attempt resets; a script exiting N times then staying up → restart count matches;
SIGTERM → all children reaped, clean exit.

**Hardware smoke checklist.** SD in → telnet :24 reachable • `kill -9
vendor-daemon.bin` → respawn within backoff • corrupt `anyka.toml` → parks,
telnet alive, FATAL on console • clock correct before `onvif-rust` spawns •
authenticated ONVIF request succeeds • force 3 crash-loop reboots → SafeMode •
**pull SD → stock camera boots**.

**Gates.** `$CARGO test --target x86_64-unknown-linux-gnu` ·
`$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings` · cross build
via `toolchain/arm-anykav200-crosstool-ng/bin/cargo`.

## Net change

```
DELETED                                          L
  anyka_hack/gergehack.sh                      396
  anyka_hack/common.sh                         402
  anyka_hack/init_logs.sh                       54
  anyka_hack/sys_monitor.sh                    261
  anyka_hack/periodic_reboot.sh                171
  vendor-daemon/run_vendor_daemon.sh           163
  dropbear/start_dropbear.sh                   143
  onvif/run_onvif_rust.sh                       92
  gergesettings.txt                             42
                                            ─────
                                            1 724

REWRITTEN
  Factory/config.sh                 38 -> ~10
  verify_logs.sh, ffmpeg/app_restarter.sh, ffmpeg/wrap_mp4.sh
    inline the 4-line log() fallback already used at
    run_vendor_daemon.sh:53-59                 +12

RETAINED, untouched (diagnostics, per R5)   ~1 300

ADDED
  cross-compile/anyka-init/    ~1 000-1 200 L Rust + ~480 L tests
```

## Risks

| # | Risk | Mitigation |
|---|------|------------|
| K1 | Binary built for the wrong arch → `execve` ENOEXEC → ash interprets the ELF as a script | P0 wrapper `[ -x ]` check; hardware smoke test gate; pull SD to recover |
| K2 | `/mnt/anyka_hack/lib/` missing → loader not found → nothing runs | P0 telnet starts before the binary is touched, so the camera stays reachable |
| K3 | Storm guard state file corrupted by power loss on vfat | Unparseable state ⇒ treat as `fast_reboots = 0`; worst case is three extra reboots |
| K4 | Big-bang cutover means one all-or-nothing hardware validation | Full smoke checklist before merge; Mode A means pulling the SD always restores a working camera |
| K5 | Stable, chatty service grows its log unbounded between spawns | Documented ceiling; monitor thread logs a WARN past a hard limit |
