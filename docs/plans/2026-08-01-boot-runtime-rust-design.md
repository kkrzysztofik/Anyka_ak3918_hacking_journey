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

---

# Addendum: Wifi Bring-Up in Rust

Date: 2026-08-01
Status: brainstorming — design agreed in principle, not yet planned or built

The main design left wifi alone: P2 rewrites `[wireless]` credentials in
`/etc/jffs2/anyka_cfg.ini` and then shells out to `wifi_manage.sh start`. This
addendum records the investigation into replacing that chain outright, the
options weighed, and the decision.

## What the vendor chain actually does

```
P2  boot.rs
     apply_wifi()                    rewrite [wireless] ssid + password
     run_to_completion("/usr/sbin/wifi_manage.sh", ["start"])
       │
       └─ wifi_manage.sh:37   wifi_run.sh &  -> exit 0        [NON-BLOCKING]
            │
            └─ wifi_run.sh:185  awk [wireless] ssid out of anyka_cfg.ini
                 ├─ ssid EMPTY -> station_install(), then loop forever at 1 Hz
                 │                waiting for /tmp/wireless/gbk_ssid
                 └─ ssid SET   -> station_start()
                      station_install()   wifi_driver.sh uninstall; station
                                          poll /sys/class/net/wlan0, 30 s cap
                      wpa_start()         wifi_station.sh start
                      station_connect()   wifi_station.sh connect
                                           -> station_connect.sh <sec> <ssid> <psk>
                      check_wifi_config_update()   loop forever at 1 Hz
```

| Script | L | Filesystem |
|---|---|---|
| `/data/wifi_driver.sh` | 412 | jffs2 mtd7, writable |
| `/data/wifi_station.sh` | 421 | jffs2 mtd7, writable |
| `/usr/sbin/wifi_run.sh` | 242 | **squashfs mtd5, read-only** |
| `/usr/sbin/station_connect.sh` | 193 | **squashfs mtd5, read-only** |
| `/usr/sbin/wifi_manage.sh` | 48 | **squashfs mtd5, read-only** |
| | **1,316** | |

`rc.local:7` mounts `/usr` from squashfs, so in Mode A three of the five
scripts cannot be modified or deleted — only bypassed. "Replacing" them means
shipping a parallel implementation and never calling theirs; the originals stay
on the device either way.

### Driver dispatch

`wifi_driver.sh:41-47` reads `/etc/jffs2/hw.conf`, drops the 3-byte `HW=`
prefix with `tail -c +4`, then takes single characters at offsets 51 (chip
type) and 52 (GPIO enable polarity). `/mnt/Factory/newFactory/hw.conf`
overrides it when present.

| hw char | `WIFI_NAME` | insmod | rmmod |
|---|---|---|---|
| `1` | `ssv6x5x` | `ssv6x5x.ko stacfgpath=<cfg>/ak3916-wifi.cfg` | `ssv6x5x` |
| `2` | `rtl8188ftv_new` | `rtl8188fu.ko` | `rtl8188fu` |
| `3` | `rtl8189` | **no such function** | — |
| `4` | `atbm603x_HT20` | **name mismatch** | — |
| `7` | `rda5995` | `rdawfmac.ko` | `rdawfmac` |
| `d` | `txw801` | `txw801.ko fw_file=txw801x_USB.bin` + `sleep 2` | `hgics` |
| `e` | `rtl8731_8733` | `8733bu.ko` | `8733bu` |
| `f` | `ssv6115_wifi6` | `ssv6x5x_wifi6.ko stacfgpath=<cfg>/ak3916-wifi6.cfg` | `ssv6x5x` |
| `g` | `zt9101` | `ZT9101UV20.ko cfg=<drv>/wifi.cfg` | `ZT9101UV20` |
| `h` | `ssv6355_ble` | `ssv6355.ko stacfgpath=<cfg>/ssv6355-wifi.cfg` | `ssv6x5x` |

Note the `rmmod` names are not derivable from the `.ko` filenames: three
different SSV modules all unload as `ssv6x5x`, and `txw801.ko` unloads as
`hgics`.

`station_install()` also drives a power-enable GPIO before loading —
`/sys/user-gpio/wifi_en`, high-then-low when the offset-52 character is `2`,
low-then-high otherwise — then `insmod /usr/modules/otg-hs.ko`, then extracts
`/data/wifi_driver.tgz` and `/data/wifi_tool.tgz` into `/tmp/ko` and `/tmp`.
That extraction is why the loaded module lives at `/tmp/ko/ssv6355.ko` rather
than in `/usr/modules`.

### Association

Two different mechanisms, selected by hardware:

- **RTL8188 (`0bda:f179`/`0bda:f72b`) with `wpa_supplicant` 2.6** —
  `wifi_station.sh:51-54` `sed`s the SSID and PSK into *line numbers 3 and 4* of
  `/etc/jffs2/wpa_supplicant.conf`, then starts `wpa_supplicant -Dnl80211`.
- **Everything else** — `wpa_supplicant -B -Dwext` against the existing conf,
  and the credentials are applied at runtime by `station_connect.sh` through
  `wpa_cli add_network` / `set_network` / `select_network`.

## Findings

**W1 — Two of ten dispatch paths are dead.** `wifi_driver.sh:391` calls
`wifi_config_${WIFI_NAME} 1`, building a function name from a string. Type `3`
produces `wifi_config_rtl8189`, which is never defined. Type `4` produces
`wifi_config_atbm603x_HT20` while the function is spelled
`wifi_config_atbm603_HT20` — no `x`. Both resolve to "command not found", the
driver never loads, and `wifi_run.sh` then hangs on the empty-SSID branch. A
`match` over an enum makes this class of bug unrepresentable, and this is the
strongest argument for the rewrite.

**W2 — The default `hw.conf` cannot dispatch at all.** `service.sh:124` writes
`HW=12151005501110018000000000000000` when the file is absent: 32 characters
after the prefix, while the script indexes offset 51. `WIFI_NAME` ends up empty
and the dispatch becomes `wifi_config_ 1`. Any camera that lost its factory
`hw.conf` has no wifi and no diagnostic. A replacement needs an explicit chip
override in config for this case.

**W3 — Credentials are interpolated into `sh -c`.** `station_connect.sh:89-91`:

```sh
sh -c "wpa_cli -iwlan0 set_network $NET_ID ssid \"$SSID\""
sh -c "wpa_cli -iwlan0 set_network $NET_ID psk  \"$PSK\""
```

A credential containing `"`, `$`, a backtick or `\` breaks the quoting. This is
the mechanism behind the unexplained warning in
`SD_card_contents/anyka_hack/README.md` that "some special characters don't
work in wifi ssid names and passwords". Not a privilege boundary — the operator
owns the device — but the failure mode is the worst available: the camera
silently fails to associate and is unreachable *because* wifi is what failed.

**W4 — An empty SSID hangs forever, silently.** `wifi_run.sh:188` treats a blank
`ssid` as "wait for the phone app to provision us" and polls at 1 Hz for
`/tmp/wireless/gbk_ssid`, a file written only by `anyka_ipc` — which the
`FACTORY_TEST=1` branch of `service.sh` never starts. The shipped
`anyka_cfg.ini` has `ssid = ` blank by default, so this is the factory-reset
path. It is also why `apply_wifi` must run before `wifi_manage.sh start`.

**W5 — Our own `rewrite_wifi_cfg` is correct only by naming coincidence.** It
matches keys exactly `ssid`/`password` and is section-blind. The real
`anyka_cfg.ini` has those keys once each under `[wireless]`, and `[softap]` uses
`s_ssid`/`s_password`, so the exact-key filter happens to hit the right two
lines. A future firmware adding an `ssid` key under another section would break
it. It also only *replaces* existing lines and never inserts, so a config
missing the key silently produces W4.

**W6 — Wifi carries the recovery channel.** Every other component in this design
has an escape hatch that survives its own failure: bad config leaves telnet :24
up, a crash loop parks in safe mode with a shell, a broken binary is fixed by
pulling the SD card. All of those hatches are reachable *over the network*, and
the network is wifi. A wifi defect is the only one that removes its own
recovery path, leaving UART or physical retrieval. This inverts the usual
risk calculus and is the reason for R7 below.

## Options considered

**A — Harden around the vendor scripts.** Keep the chain; add credential
validation at TOML parse time, make the ini rewrite insert missing keys, and add
a wifi-up check that logs and retries with backoff. ~120 L, no risk to the
recovery channel, but leaves W1 and W2 unfixed because the dispatch stays in
shell.

**B — Take over association, keep vendor driver load.** Still call
`wifi_driver.sh station` for the chip-specific `insmod`, but skip `wifi_run.sh`
and drive `wpa_supplicant` ourselves. Removes W3 and W4 entirely and leaves the
untestable hardware dispatch with the vendor. ~250 L.

**C — Full rewrite including driver dispatch.** Reproduce all of it, including
`hw.conf` parsing and the ten `insmod` variants. Fixes W1–W5. Nine of ten
hardware paths cannot be exercised on the bench.

**D — Leave it alone.** Document the sharp edges in the smoke checklist.

**Decision: C, with a mandatory fallback (R7).** The deciding argument is W1: two
dispatch paths are already dead in vendor code and a further eight are one typo
away from the same silent failure, because the dispatch is string-built function
names. An exhaustive `match` removes the entire class. W2 and W3 are fixed on
the way.

## Proposed design

```
P2  wifi::bring_up(sys, cfg) -> Outcome
      1  resolve chip
           [wifi].chip = "auto" -> parse /etc/jffs2/hw.conf offsets 51,52
                                   (and /mnt/Factory/newFactory/hw.conf override)
           otherwise            -> pinned name, skips W2
      2  validate credentials   -> reject unquotable chars, ssid <= 32,
                                   WPA psk 8..=63; fail loud on the console
      3  power sequence         -> /sys/user-gpio/wifi_en, polarity from offset 52
      4  prepare               -> insmod /usr/modules/otg-hs.ko
                                  untar /data/wifi_driver.tgz -> /tmp/ko
                                  untar /data/wifi_tool.tgz    -> /tmp
      5  load driver            -> rmmod <chip.rmmod>; insmod <chip.module>
                                   + per-chip settle delay
      6  wait interface         -> /sys/class/net/wlan0, 30 s cap; ifconfig up
      7  write wpa_supplicant.conf  properly quoted, one network block
      8  start wpa_supplicant   -> as a supervised service, not a bare fork
      9  wait association       -> carrier/operstate, bounded
     10  udhcpc -i wlan0        -> or static from [ethernet] in the ini
     11  verify address
           ok      -> Outcome::Up { chip, ssid, addr, elapsed }
           timeout -> Outcome::FellBack, after invoking wifi_manage.sh start
```

Pure and therefore host-testable: `parse_hw_conf`, `Chip::from_hw_char`,
`Chip::from_name`, `Chip::module`, `validate_credentials`,
`wpa_supplicant_conf`, `ini_get`. That is the bulk of the risk surface — the
dispatch table, the credential rules and the generated config can all be proven
on x86_64 against transcribed vendor behaviour.

Thin and untestable off-device: the GPIO write, `insmod`, the tar extraction,
the interface poll and DHCP.

### Config additions

```toml
[wifi]
ssid = "..."
password = "..."
config_file = "/etc/jffs2/anyka_cfg.ini"   # still rewritten, for the vendor's benefit
security = "wpa"        # wpa | wep | open
chip = "auto"           # auto | ssv6355_ble | rtl8188ftv_new | ... (W2 escape hatch)
interface = "wlan0"
dhcp = true
connect_timeout_sec = 45
fallback_to_vendor = true
```

`config_file` stays because `anyka_ipc` and the WebUI still read
`anyka_cfg.ini`, and because the fallback path needs it populated.

## Risks

| # | Risk | Mitigation |
|---|------|------------|
| R7 | Nine of ten chip paths are untestable here, and a transcription error costs the only remote access to the camera | **Mandatory fallback**: if `bring_up` produces no carrier before `connect_timeout_sec`, invoke the vendor `wifi_manage.sh start` and log loudly. Makes a wrong dispatch entry recoverable rather than terminal. Not optional, not behind a flag by default |
| R8 | `wpa_supplicant` flags differ per driver (`-Dnl80211` for RTL8188+2.6, `-Dwext` elsewhere) | Probe order: try `nl80211`, fall back to `wext`. Record which worked in the log |
| R9 | The tar extraction and `otg-hs.ko` load have timing the vendor papers over with `sleep 3` / `sleep 2` | Keep the sleeps initially; replace with a poll on the expected artifact only once hardware confirms the timing |
| R10 | Rewriting `wpa_supplicant.conf` breaks the RTL8188 line-numbered `sed` path if the vendor fallback later runs | Fallback regenerates the vendor-shaped conf before delegating |
| R11 | Credential validation could lock a user out of a network whose PSK contains a rejected character | Reject only `"`, newline and NUL — characters that cannot survive the config-file grammar. Shell metacharacters (`$`, backtick, `\`, `;`, `&`) must be **accepted**, since they only broke the vendor's `sh -c` and are legal in a PSK |

## Resolved questions

### Q1 — `wpa_supplicant` lifecycle: supervised service. RESOLVED

How the vendor does it today:

```
START    wifi_station.sh start
           RTL8188 + wpa_s 2.6:  wpa_supplicant -iwlan0 -Dnl80211 -c <conf> >>/tmp/wpa_log &
           everything else:      wpa_supplicant -B -iwlan0 -Dwext -f /tmp/wpa_log -c <conf>
         Either way it detaches and is reparented to init.
WAIT     wifi_run.sh:106            pgrep; empty -> "init failed, exit start wifi"
         station_connect.sh:57-63   unbounded `while not in ps: sleep 1`
MONITOR  wifi_station.sh:141-176, only during the connect attempt:
           180 iterations at 1 Hz of `ps | grep wpa_supplicant`,
           `wpa_cli status | grep wpa_state`, and grep of /tmp/wpa_log for
           "4-Way Handshake failed" / "Invalid WEP key" -> return 3
AFTER    check_wifi_config_update() loops forever at 1 Hz but never checks
         whether wpa_supplicant is still alive
STOP     wifi_station.sh:396-399    killall wpa_supplicant; killall udhcpc
RESTART  never
```

So the incumbent design is **detached, unsupervised, watched only until
association completes, then forgotten** — D1 applied to wifi. When
`wpa_supplicant` dies after association the interface keeps its address and the
camera looks healthy until the AP deauths or the lease lapses, then drops off
the network with no log line and no recovery short of a reboot.

**Decision: `wpa_supplicant` becomes a `[services.*]` entry** under the normal
backoff and crash-loop policy. Three findings make this straightforward:

- **Drop `-B`.** A self-daemonizing process exits its parent immediately, which
  the supervisor would read as an instant crash and backoff-loop on forever.
  Foreground operation is already proven on this hardware: `wifi_station.sh:60`
  runs it without `-B` on the RTL8188 path.
- **The driver coupling does not bite.** `wifi_driver.sh uninstall` is only
  called from `station_install` at the start of bring-up; nothing unloads the
  module at runtime. Restarting `wpa_supplicant` alone is a valid recovery in
  steady state.
- **Bad-password diagnostics come free.** With the process supervised, its
  stdout already lands in its own log via the existing spawn plumbing, so the
  `4-Way Handshake failed` signal the vendor greps for becomes a distinct loud
  error instead of a generic 180 s timeout.

### Q3 — `anyka_cfg.ini` ownership: sole writer. RESOLVED

`anyka-init` is the only component that writes it, so no ordering rule is
needed.

- `onvif-rust` has zero references to `anyka_cfg`. `set_network_interface` is a
  default trait method (`platform/common/traits.rs:593`) returning
  `PlatformError::NotSupported`, and the Anyka platform does not override it —
  it structurally cannot write network configuration.
- The WebUI has no references.
- All other matches are vendor SDK reference source under
  `cross-compile/anyka_reference/` (never deployed) or vendor rootfs scripts.
- The only runtime writer would be `anyka_ipc` (`ak_config.c:20`), which the
  `FACTORY_TEST=1` branch of `service.sh` never starts.

## Open questions

1. Static IP: `wifi_run.sh:57-67` supports `[ethernet] ipaddr/netmask/gateway`
   from the ini. Carry that forward, or require DHCP and drop the option?
2. Is `hw.conf` offset 51 stable across the camera revisions in circulation, or
   should chip detection prefer `lsusb` IDs where the chip is on USB?
