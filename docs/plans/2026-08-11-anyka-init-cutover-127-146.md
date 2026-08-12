# anyka-init Cutover on `.127` and `.146` — Deploy Plan

**Date:** 2026-08-11
**Goal:** Move `192.168.3.127` and `192.168.30.146` off `gergehack.sh` onto the `anyka-init` Rust
supervisor stack (vendor-daemon + onvif-rust + WebUI), matching what `.198` and `.121` already run.

**Prior art:** `docs/plans/2026-08-03-anyka-init-on-121.md` is the proven runbook. This plan reuses
its task shape; the deltas are the per-camera config and one new wifi driver path.

---

## 1. Device survey (measured 2026-08-11, not assumed)

| | `.198` (reference) | `192.168.30.146` | `192.168.3.127` |
|---|---|---|---|
| Board | Cloud39EV2_AK3918E80PIN_MNBD | same | same |
| Sensor | gc1084 | gc1084 | gc1084 |
| `libakmedia.so` md5 | `45bc0d87…` | **identical** | **identical** |
| `libakstreamenc.so` md5 | `8df1c7c9…` | **identical** | **identical** |
| `libakispsdk.so` md5 | `1741e74b…` | **identical** | **identical** |
| `hw.conf` chip char (off. 51) | `h` | `h` | **`g`** |
| → anyka-init chip | `ssv6355_ble` | `ssv6355_ble` | **`zt9101`** |
| Polarity char (off. 52) | `2` → high_low | `2` → high_low | `2` → high_low |
| Live `lsmod` wifi module | ssv6x5x | ssv6x5x | ZT9101UV20 |
| SSID | kmk | W11-CAM | Orange_Swiatlowod_1AB0 |
| Gateway | 192.168.2.1 | 192.168.30.1 | 192.168.3.1 |
| Current stack | anyka-init | **gergehack** | **gergehack** |
| Port 8080 (ONVIF) | open | closed | closed |
| `/mnt` filesystem | — | exfat, 57.7 G free | exfat, 57.7 G free |
| `/etc/jffs2` free | — | 12 K / 64 K | 12 K / 64 K |
| RAM free | — | 11.5 M of 36.5 M | 8.2 M of 36.5 M |
| `ptz_daemon_dyn` | absent | present | present |
| `/usr/sbin/wifi_manage.sh` | present | present | present |
| `/tmp/wpa_supplicant` | present | present | present |
| Access | direct telnet :24 | **jumphost only** | direct telnet :24 |

**The finding that de-risks this whole job:** all three SDK libraries are byte-identical to `.198` on
both cameras. The `.121` "venc returns NULL / wrong libakstreamenc" class of blocker
(`modernization-121-blocked-on-vi-mapping`) does **not** apply here.

### Access paths

```bash
# .127 — direct
uv run python3 scripts/debugging/cam_exec.py --host 192.168.3.127 'cmd'

# .146 — via jumphost kmp-jumphost (192.168.3.137, also on 192.168.30.143)
ssh -N -L 12324:192.168.30.146:24 root@192.168.3.137 &
uv run python3 scripts/debugging/cam_exec.py --host 127.0.0.1 --port 12324 'cmd'
```

Jumphost has `rsync`, `ftp`, `python3`, `nc`, 29 G free — **but no `lftp`**. See Phase 2.

---

## 2. Risk register

| Risk | Camera | Severity | Mitigation |
|---|---|---|---|
| **`zt9101` driver path never exercised on hardware** — anyka-init's wifi table has the row, and `/tmp/ko/{ZT9101UV20.ko,wifi.cfg,ZT9101_fw_r2636.bin}` are all present, but every previous cutover used `ssv6355_ble`. | `.127` | **High** | Do `.146` first. Dry run (Phase 3) proves it before anything irreversible. Deadman in `Factory/config.sh` restores gergehack + reboots if no IP 180 s after boot. |
| Camera reachable only via jumphost; a lost boot = on-site SD pull | `.146` | High | Deadman. Cutover is the last step, after a green dry run. |
| `copy_sd_contents.sh --ftp` uploads `Factory/` too — that **is** the irreversible cutover | both | **High** | Do not use it as-is. Phase 2 uploads `anyka_hack/` only; `Factory/config.sh` goes last, by hand. |
| gergehack's `libre_anyka_app` currently serves RTSP; our stack replaces it | both | Medium | Recoverable over telnet — the deadman only guards the network, not video. Verify RTSP in Phase 5. |
| `/etc/jffs2` is 81 % full (12 K free); udhcpc rewrites `resolv.conf` per lease | both | Medium | `[time] servers` lists an IP first, exactly as `.121` does. Already handled by the config template. |
| 86 M payload over 2.4 GHz to an ARM926 | both | Low | Budget 10–30 min per camera. Upload during the dry-run phase, not under time pressure. |
| `.127` NTP: gergehack points at `192.168.1.1`, but the camera's own gateway is `192.168.3.1` | `.127` | Low | Use `192.168.3.1` + pool fallback. |

**Recommended order: `.146` first, then `.127`.** `.146` is a byte-for-byte twin of the already-proven
`.121` (identical `hw.conf`, identical libs, same SSID/network); its config delta is comments only.
`.127` carries the one untested code path, so it goes second, once the process itself is known good.

---

## 3. Build artifact (already done)

`bash ./scripts/build_sd_contents.sh` completed clean. Verified:

```
anyka-init.bin       1.5M  ELF 32-bit LSB ARM EABI5, interp /mnt/anyka_hack/lib/ld-uClibc.so.1
onvif-rust.bin       8.3M  ELF 32-bit LSB ARM EABI5, interp /mnt/anyka_hack/lib/ld-uClibc.so.1
vendor-daemon.bin   83.9K  ELF 32-bit LSB ARM EABI5, interp /lib/ld-uClibc.so.0
onvif/www/                 25 files, 872 kB raw / 254 kB gzip / 223 kB brotli
```

Payload: `SD_card_contents/anyka_hack` 86 M, `SD_card_contents/Factory` 12 K.

**This build includes uncommitted working-tree changes** (`Sparkline.tsx`, `DiagnosticsPage.tsx` and
their tests, branch `feat/diagnostics-view`). Their tests pass: 46/46. Commit before cutover so the
deployed bytes are reproducible from git.

---

## Phase 0 — Pre-flight (both cameras, ~10 min)

1. Commit or stash the `feat/diagnostics-view` working-tree changes so the payload is traceable.
2. Re-run the full gate on the vendored toolchain:
   ```bash
   source ./setenv.sh
   cd cross-compile/onvif-rust
   $CARGO fmt --check
   $CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
   $CARGO test --target x86_64-unknown-linux-gnu
   ```
   (`vendored-clippy-needs-path-prefix`: the toolchain bin dir must be first on `PATH`.)
3. Confirm both cameras are still up and note their current uptime, so a surprise reboot mid-deploy
   is visible.

## Phase 1 — Write the per-camera configs (~20 min, no device writes)

`.deploy/` is gitignored on purpose — these files carry real PSKs. No commit.

### `.deploy/anyka-146.toml`

Copy `.deploy/anyka-121.toml` verbatim, then change **only**:
- header comment → `192.168.30.146`
- the commented-out static-IP block → `192.168.30.146/24`

Everything else is already correct for this camera and is proven on `.121`:
`sensor_module = "/data/sensor_ko_and_isp_conf/sensor_gc1084.ko"` (symlink verified present),
`ssid = "W11-CAM"`, `chip = "ssv6355_ble"`, `gpio_polarity = "high_low"`,
`servers = ["192.168.3.1", "0.ubuntu.pool.ntp.org"]`, `[services.ptz]` enabled,
`[services.wpa_supplicant] exec = "/tmp/wpa_supplicant"`.

Verify the PSK against the device before trusting it:
```bash
cam_exec … 'grep wifi_pass /data/gergesettings.txt'   # do not paste into a transcript
```

### `.deploy/anyka-127.toml`

Copy `.deploy/anyka-121.toml`, then change:

| Key | Value | Why |
|---|---|---|
| header comment | `192.168.3.127`, directly reachable, no jumphost | |
| `[system] sensor_module` | `/data/sensor/sensor_gc1084.ko` | real persistent file, verified present (matches `.198`) |
| `[wifi] ssid` | `Orange_Swiatlowod_1AB0` | from `iwconfig`/`gergesettings.txt` |
| `[wifi] password` | read from `/data/gergesettings.txt` | |
| **`[wifi] chip`** | **`zt9101`** | `hw.conf` offset 51 = `g`; row insmods `/tmp/ko/ZT9101UV20.ko cfg=/tmp/ko/wifi.cfg`, rmmod name `ZT9101UV20` — all three confirmed present and live |
| `[wifi] gpio_polarity` | `high_low` | offset 52 = `2` |
| `[time] servers` | `["192.168.3.1", "0.ubuntu.pool.ntp.org"]` | its own gateway; gergehack's `192.168.1.1` is on a different subnet |
| static-IP comment block | `192.168.3.127/24`, gw `192.168.3.1` | |

Keep `[services.ptz] enabled = true` (`ptz_daemon_dyn` present on both).

Grep both files for `CHANGE_ME` before proceeding.

## Phase 2 — Upload the payload, **excluding `Factory/`** (~15–30 min per camera)

Until `Factory/config.sh` is replaced, a power cycle is a **complete rollback**. Keep it that way.

### `.127` (direct)

```bash
lftp -c "
set ftp:ssl-allow no
open -u root,\"\$ANYKA_FTP_PASS\" 192.168.3.127
mkdir -p /mnt/anyka_hack
mirror -R --verbose --no-perms --no-umask \
  SD_card_contents/anyka_hack /mnt/anyka_hack
bye"
```

Then upload the config separately:
`.deploy/anyka-127.toml` → `/mnt/anyka_hack/anyka.toml`.

### `.146` (via jumphost — no `lftp` there)

Stage on the jumphost, then push from it:

```bash
rsync -a SD_card_contents/anyka_hack/ root@192.168.3.137:/tmp/payload/
ssh root@192.168.3.137 'apt-get install -y lftp'      # simplest: reuse the same mirror command
ssh root@192.168.3.137 'lftp -c "
  set ftp:ssl-allow no
  open -u root,PASS 192.168.30.146
  mkdir -p /mnt/anyka_hack
  mirror -R --no-perms --no-umask /tmp/payload /mnt/anyka_hack
  bye"'
```

If installing `lftp` on the jumphost is unwanted, a ~15-line `python3 ftplib` recursive walk does the
same job with what is already there.

**Verify after upload, on the camera:**
```bash
md5sum /mnt/anyka_hack/anyka-init.bin /mnt/anyka_hack/onvif/onvif-rust.bin \
       /mnt/anyka_hack/vendor-daemon/vendor-daemon.bin
ls -l /mnt/anyka_hack/anyka.toml
```
Compare against local `md5sum`. A silent short FTP write is the classic failure here
(`deploy_onvif.sh` has a whole size-check block for exactly this reason).

## Phase 3 — Dry run: prove the config on the live camera (~10 min per camera)

Nothing here is irreversible. A power cycle still restores gergehack.

1. **Parse check.** There is no on-device dry-parse: `anyka-init` takes **no CLI arguments**, hardcodes
   `/mnt/anyka_hack/anyka.toml` (`main.rs:16`), and on a config error calls `park()` — it stays alive
   forever with recovery telnet up but spawns nothing. So:
   - *Before upload:* diff the **key set** of the new file against known-good `.deploy/anyka-121.toml`.
     A typo'd key name is what serde rejects, and this catches it on the host:
     ```bash
     diff <(grep -oE '^\s*[a-z_]+\s*=' .deploy/anyka-121.toml | tr -d ' ' | sort -u) \
          <(grep -oE '^\s*[a-z_]+\s*=' .deploy/anyka-146.toml | tr -d ' ' | sort -u)
     ```
   - *During the dry run:* a config failure shows as `anyka-init.bin` alive in `ps` with **no**
     `vendor-daemon.bin` / `onvif-rust.bin` children after ~15 s, plus a `config load failed:` line
     on the boot console. Treat that as an abort → `reboot`.
2. **Stop what the supervisor will own:**
   ```bash
   killall gergehack.sh sys_monitor.sh app_restarter.sh 2>/dev/null
   killall libre_anyka_app ptz_daemon_dyn 2>/dev/null
   killall wpa_supplicant udhcpc 2>/dev/null
   ```
3. **Arm the deadman by hand**, then launch detached (copy the `( sleep 180 … ) &` block out of
   `SD_card_contents/Factory/config.sh` and run it in the shell first — do **not** skip this, it is
   the only thing between a bad wifi bring-up and an on-site visit):
   ```bash
   ( /mnt/anyka_hack/anyka-init.bin >/mnt/logs/init-dryrun.log 2>&1 & ) &
   ```
4. **Wait 90 s, reconnect, and check:**
   - `ifconfig wlan0` still has the same `inet addr`
   - `ps` shows `anyka-init.bin`, `vendor-daemon.bin`, `onvif-rust.bin`, `wpa_supplicant`, `udhcpc`
   - `lsmod` shows the expected module (`ZT9101UV20` on `.127`, `ssv6x5x` on `.146`)
   - `date` is not 1970 (NTP synced)
   - port 8080 answers: `curl -s -o /dev/null -w '%{http_code}' http://<ip>:8080/onvif/device_service`
   - RTSP 554 answers
   - `free` shows headroom (36 M total — this is the tightest resource)
5. **Abort path:** if anything fails, `reboot`. gergehack comes back untouched.

**Gate: do not proceed to Phase 4 until Phase 3 is green.** For `.127` this is the step that
either validates the `zt9101` path or kills the deploy.

## Phase 4 — Cutover (irreversible; ~5 min per camera)

1. **Back up the vendor boot path** (the deadman's restore source — must exist *before* the swap):
   ```bash
   cp /mnt/Factory/config.sh /mnt/Factory/config.sh.gerge.bak && sync
   ```
   Verify it is the gergehack version and is readable.
2. Upload `SD_card_contents/Factory/config.sh` → `/mnt/Factory/config.sh`.
3. **Verify what actually landed, before rebooting:** `cat` it back, confirm it contains the deadman
   block and `BIN=${ANYKA_INIT_BIN:-/mnt/anyka_hack/anyka-init.bin}`, and `sh -n` it on the camera.
4. `sync; reboot`.
5. Wait 120 s, reconnect, re-run every Phase 3 Step 4 check.
6. Confirm gergehack is genuinely gone: no `gergehack.sh`, `libre_anyka_app`, or `sys_monitor.sh`
   in `ps`.

## Phase 5 — Verify and record

- WebUI: `http://<ip>:8080/` — check it actually serves (per `webui-static-root-must-be-absolute`,
  verify with `curl`, not `ls`; watch for `static_root` and the rate-limit-0 deny-all trap).
- RTSP: run the validation harness (`anyka-validation` skill) against both cameras.
- Per `ak3918-stream-timing-measurement`: ignore loadavg, read `top`'s idle %; discount the fixed
  ~2 s RTSP startup on short samples.
- Let each camera sit for one full day/night cycle before declaring success — `.121`'s dusk crash
  (`121-daily-crash-dusk-vi-collapse`) only showed up over a diurnal cycle.
- Update `.deploy/` notes, write memories for the two new cutovers, and note the `zt9101` path is now
  hardware-proven (or not).

---

## Rollback

| When | How |
|---|---|
| Before Phase 4 | Power cycle. Complete rollback, no action needed. |
| After Phase 4, network alive | Restore `config.sh.gerge.bak` over `config.sh`, `sync`, `reboot`. |
| After Phase 4, no network | Deadman fires automatically at T+180 s: retries `wifi_manage.sh start`, waits 60 s, then restores `config.sh.gerge.bak` and reboots. Camera is back on gergehack in ~5 min. |
| Deadman also fails | On-site SD card pull. `.146` has no serial console and no non-jumphost access. |

---

## Outcome (2026-08-11/12)

> **Superseded on 2026-08-12 — see "Update 2026-08-12" at the end of this file.**
> Two conclusions below were wrong: the streaming hang was *not* a lock-ordering
> deadlock (it was a missing `users.toml`), and the `zt9101` path is *not*
> hardware-proven (it hard-resets the camera).

**`.146`: cut over, partially working. `.127`: not started (deliberate stop).**

Done on `.146`: payload pushed via `tar | nc` (md5-verified), `.deploy/anyka-146.toml` written and
verified against the device's own SSID/PSK hashes, `config.sh.gerge.bak` created, `config.sh`
replaced (md5 `1c40209060518ebf3aa0adae092f2505`), rebooted. It now boots `anyka-init` → onvif-rust
+ vendor-daemon + ptz, WiFi/DHCP stable, NTP synced, gergehack gone.

Working: WebUI `http://192.168.30.146/` → **200**; ONVIF on **port 80** → 401/400 (alive).
Broken: **RTSP 554 and HTTP-FLV 8080 never bind.** onvif-rust logs **zero** `Application started`
lines and sits at 0:00 CPU (healthy `.121` shows 237:13 over a day). Phase 4 spawns HTTP as a
background task, so port 80 works while Phase 6 (streaming) is never reached. `[discovery]
enabled = false` did **not** fix it, so the hang is not WS-Discovery. Root cause unresolved —
needs `superpowers:systematic-debugging`, not more probing.

**`.127`: dry run passed the WiFi gate, failed the streaming gate, held before cutover.**

Two useful results:

1. **`zt9101` is now hardware-proven.** The dry run brought the camera back at **T+120s, before the
   deadman's 180s**, so `anyka-init`'s own zt9101 path did it, not the vendor fallback:
   `ZT9101UV20` live in `lsmod`, `wlan0` holding 192.168.3.127, gergehack's httpd gone, onvif-rust
   owning port 80, vendor-daemon healthy at 0:12 CPU. The chip table row is correct as written.
2. **The streaming hang is NOT camera-specific — it reproduces identically on `.127`**: zero
   `Application started`, zero streaming log lines, 554/8080 dead. So cutting over would have
   produced a second video-less camera. Held at the gate; `config.sh` never touched, and `.127`
   was rebooted back to gergehack.

**New diagnostic** (captured while `.127` was hung): onvif-rust's main thread sits in
`futex_wait_queue_me` — blocked on a lock, *not* on I/O. The other four threads are a normal idle
tokio runtime (`epoll_wait` + two idle workers). That points at a lock-ordering deadlock in the
startup path; the config `RwLock` is the obvious suspect, since `StreamingConfig::from_config`
takes a read lock and Phase 5 runs just before it. Start there, not at the network layer.

### Payload integrity: `tar | nc` is not self-verifying

A full md5 manifest check (`md5sum -c`, 267 files) found silent corruption on **both** cameras —
files written the right size but with NUL content, an exFAT flush artifact:

- `.127`: 1 file — `libre_anyka_app/run_libre_anyka_app.sh` (657 NUL bytes)
- `.146`: 2 files — the same launcher plus `libre_anyka_app/lib/libakae.so`

Everything the anyka-init stack depends on (`anyka-init.bin`, `onvif-rust.bin`,
`vendor-daemon.bin`, `lib/`, `onvif/www/`) verified clean on both, so corruption does **not**
explain the streaming hang. **Always run the manifest check after a `tar | nc` push** — spot-checking
a few binaries is not enough.

### Trap: the repo launcher bricks gergehack's video

`SD_card_contents/anyka_hack/libre_anyka_app/run_libre_anyka_app.sh` hardcodes `image_width=1920 /
image_height=1080` (upstream dropped the `gergesettings.txt` sourcing, since anyka-init cameras have
no gergehack). Overwriting a *gergehack* camera's launcher with it kills RTSP: the gc1084 rejects
1080p —
`vi_check_channel_attr: main channel argument error, w: 1920, h: 1080` → `ak_vi_set_channel_attr
FAIL` → **SIGSEGV**. `.127` needed a one-line restore (`. /data/gergesettings.txt`, giving back
`-w 640 -h 360 -m 0 -i 4 -u`) before its video came back. Push the payload only to cameras you are
actually cutting over, or repair this file if you abort.

Rollback for `.146` (restores gergehack + its working RTSP):
```sh
cp /mnt/Factory/config.sh.gerge.bak /mnt/Factory/config.sh && sync && reboot
```

Process corrections learned the hard way, now in memory: never `killall busybox`
([[never-killall-busybox-on-camera]]); log dry-run scripts to `/tmp`, since a failed
`exec >/mnt/...` leaves busybox ash running and a second `anyka-init` will fight the first over
vendor-daemon's single-client IPC slots; and ONVIF is port 80, not 8080
([[onvif-rust-ports-and-startup-phases]]).

## Open question

**Is there on-site physical access to either camera?** The deadman covers the wifi-loss case, but it
is the last line of defence and it has never had to fire on a `zt9101` camera. If nobody can reach
`.127` or `.146` to pull an SD card, the `.127` cutover in particular should wait until the `zt9101`
path has been proven on a camera someone can physically touch.

---

## Update 2026-08-12

### `.146`: fixed and streaming. Root cause was a missing `users.toml`.

Not a deadlock. `server.auth_enabled = true` plus an absent
`users.toml` makes `build_stream_auth` (`app.rs:1379`) return `Err`, so
`start_streaming` logs a warning and returns `None` — **RTSP and HTTP-FLV never
bind, and everything else comes up normally.** The error message in the source
says exactly this; nobody saw it because the shipped `[logging] level = "error"`
filters `warn!`. The only reason any log line appeared at all is that the
night-mode module is pinned to INFO (`32e7dba1`).

That also explains the two red herrings recorded above:

- "zero `Application started` lines" — startup took the *degraded* branch
  (`app.rs:1146`), which also logged at `warn!`.
- "main thread in `futex_wait_queue_me`" — a normal idle tokio park, not a lock.

`.121` and `.198` only ever worked because they picked up a `users.toml` during
earlier hand-deploys. **The SD payload has never contained one**, so every
camera built from `build_sd_contents.sh` loses RTSP/HTTP-FLV on first boot.

Fix applied to `.146`: copied `.121`'s `users.toml` (md5 `e51fdd97…`, 72 bytes)
to `/mnt/anyka_hack/onvif/users.toml`, restarted the daemon pair. Verified:

| Check | Result |
|---|---|
| RTSP `/main` | h264 1280x720 @ 15 fps |
| RTSP `/sub` | h264 640x360 |
| WebUI `http://192.168.30.146/` | 200 |
| HTTP-FLV 8080 | bound (401 without credentials, as designed) |

A copy now lives in `.deploy/users.toml` (gitignored, alongside the wifi PSKs).

### `.127`: cutover **blocked**. The `zt9101` path hard-resets the camera.

The claim above that "`zt9101` is now hardware-proven" does not survive a
second look — the same Aug-11 `anyka-init.log` that was read as success shows
`wpa_supplicant` exiting `Code(255)` in a tight loop and memory falling to
3 MB.

Two instrumented dry runs on 2026-08-12 both **hard-reset the camera ~25 s after
`anyka-init` started**, with `config.sh` untouched, so both times it came back on
gergehack. Round 2 logged to `/mnt/logs/dryrun127.log` (persistent — round 1's
`/tmp` log was wiped by the reset). Last samples before the reset:

```
394.64 starting anyka-init
       ERROR anyka_init::boot: sensor module load failed … insmod … File exists
396.68 SAMPLE free=15660 load=7.12 init=2058 onvif= vd= wlan=0
402.02 SAMPLE free=15600 load=6.63 init=2058 onvif= vd= wlan=0
417.90 SAMPLE free=15428 load=7.07 init=2058 onvif= vd= wlan=0
<reset>
```

`anyka-init` is alive throughout; `onvif-rust` and `vendor-daemon` are **never
reached**; `wlan0` never gets an address. Memory is not the constraint (15 MB
free). The 16 s gap before the final sample means the box stalled, then reset —
during wifi bring-up, which on this board means `rmmod`/`insmod` of a live
`ZT9101UV20` that the vendor stack still holds. Ruled out as causes: the
watchdog (`/dev/watchdog` is open by nobody), `periodic_reboot.sh` (6 h
interval), the monitor's wifi reboot (needs 10 ticks = 10 min), `[reboot]`
(disabled), and both vendor wifi scripts (no reboot/halt logic).

**Do not cut `.127` over.** With `config.sh` swapped, every boot hits this same
path and the deadman becomes the only thing standing between the camera and a
reboot loop. `.127` is left on gergehack, healthy, `config.sh` never touched and
no `config.sh.gerge.bak` created.

### Two traps found while preparing `.127`

1. **`.deploy/anyka-127.toml` carried the wrong wifi PSK.** The copy already on
   the camera matched `/data/gergesettings.txt`; the local one did not. Uploading
   the local file "to be safe" before the cutover would have taken the wifi down
   and forced the deadman. The local file has been replaced with the device's
   proven copy (old one kept as `.deploy/anyka-127.toml.stale-psk.bak`).
   **Compare PSK hashes against the device before every upload.**
2. **`SD_card_contents/anyka_hack/onvif/onvif-rust.bin` no longer matches what is
   deployed** (local 8750468 B vs 8738172 B on both cameras). The local tree was
   rebuilt after the deploy. `.146` and `.127` carry the *same* binary, and that
   is the one now proven to stream. Do not push the local rebuild without
   re-validating it.

### Repo fix

`app.rs`: the two `start_streaming` failure branches and the degraded-startup
line now log at `error!` instead of `warn!`, so a camera that silently loses
RTSP says so at the shipped log level. Uncommitted — see the session notes about
the concurrent branch checkout.

**Still open:** ship a `users.toml` (or generate one at first boot) as part of
the payload, so a fresh camera does not lose streaming by default.

---

## Update 2026-08-12 (later): `.127` cutover attempted and rolled back

**zt9101 is cleared.** The hard reset was a dry-run artifact: `wifi.rs:492` power-cycles the radio
~4 s before `wifi.rs:524` rmmods it, so a dry run yanks power from a chip the vendor's live
`ZT9101UV20` still owns. `rmmod ZT9101UV20` first, then start anyka-init → **no reset, 2 h+ stable,
RTSP `/main` 1280x720 and `/sub` 640x360 both verified, WebUI 200.** All gates green, so the
cutover proceeded.

**It failed at the real boot, for an unrelated reason.** `dmesg` shows
`[EXFAT] mounted → trying to unmount → unmounted → mounted`. `service.sh:174` mounts `/mnt`,
`service.sh:91` runs `/mnt/Factory/config.sh`, and the card is then unmounted and remounted. Our
`config.sh` backgrounds `while :; do "$BIN"; sleep 5; done &` — script and binary both on `/mnt` —
so the unmount kills the loop. The camera came up with **wifi and telnet fine and nothing else
running**: no anyka-init, no onvif-rust, no gergehack. **The deadman is blind to this** — it only
checks for a wlan0 address, which was present.

The vendor's own `config.sh` avoids this by copying gergehack to `/data` and running it from jffs2,
never from `/mnt`. `/data` has ~12 K free, so the 1.5 MB binary cannot simply move there.

Rolled back (`cp config.sh.gerge.bak config.sh && sync && reboot`); `.127` is on gergehack with
`libre_anyka_app` encoding and ports 554/80 up. `config.sh.gerge.bak` now exists, so a future
cutover is one step.

**Before retrying:** make `config.sh` survive the SD unmount, and check whether `.146`/`.121`/`.198`
show the same unmount in `dmesg` — if they don't, find out what differs rather than assuming the
fix is universal. Also worth hardening the deadman: "wlan0 has an address" is not a liveness test,
since this failure mode passes it.

Two mechanical notes: telnet dies on a ~4900-char line, so push files as base64 in ~280-byte
chunks, decode to a `.new`, verify md5, then `mv` — never redirect straight onto `config.sh`. And
never start anyka-init by hand while the vendor wifi stack is live; it triggers the same reset.

### Correction (same day): the SD unmount is NOT the cause

The section above blamed the exFAT unmount for killing the supervisor loop. **That is wrong**,
checked directly:

| | `.198` | `.146` | `.127` |
|---|---|---|---|
| `/mnt` filesystem | **vfat** | exfat (SD64G) | exfat (SD64G) |
| `[EXFAT] mount/unmount/mount` in dmesg | absent (vfat) | **present** | present |
| `{config.sh}` loop shell alive | — | **yes, pid 439 after 22 h** | no |
| Boots anyka-init | yes | yes | **no** |

`.146` runs the same card and the same unmount sequence and its supervisor shell survives, so the
unmount alone strands nothing.

What remains is `config.sh:59` — `if [ ! -x "$BIN" ]; then … exit 1; fi`. On `.127` config.sh
provably ran (telnetd came up) and the deadman armed, yet no loop shell existed, and that `exit 1`
is the only path out before the loop. The `-x` test evidently landed in the window where `/mnt` is
the bare tmpfs. The `while` loop would itself have tolerated a briefly-absent binary — the fatal
guard is what makes it permanent. `.127`'s ZT9101 being a **USB** part is a plausible reason its
boot timing differs.

Not yet directly observed (the guard's stderr goes to the serial console). Confirm by instrumenting
`config.sh` with `logger` calls — syslog lands in `/var/log/messages`, which survives because this
failure does not reset the box — recording `[ -x $BIN ]` and `mount | grep /mnt` at that instant.

Proposed fix: bounded wait for the binary instead of an immediate `exit 1`.

### Correction 2 (2026-08-12, instrumented boot): `.127` is cut over and running

Both earlier theories were wrong. The instrumented `config.sh` logged
`entry BIN=present` / `BIN available after 0s` — **the `-x` guard never fired**, and the SD unmount
does not kill the loop (`.146` runs the same exfat card with the same unmount and its `{config.sh}`
shell is alive after 22 h; `.198` is vfat and not comparable).

`.127` now boots anyka-init cleanly, with the **native zt9101 path** (`driver="wext"`), no vendor
fallback and no reset. `Application started successfully in 469ms`. Working: WebUI 200, ONVIF,
RTSP `/sub`, HTTP-FLV `/live/main.flv` at h264 1280x720.

**Why the first two post-cutover boots produced no `config.sh` process at all is still
unexplained.** Do not claim the bounded wait fixed it.

**Remaining defect — RTSP `main` never registers.** `/main` 404s to RTSP while `/live/main.flv`
serves the same video. The hub logs `transceiver_run_success` for `live/main`, `sub` and
`live/sub`, but **never for `main`**. Immediately above it:

```
Applying Anyka VI max attrs (vendor-ipc-legacy-mapping): main_max=640x360, sub_max=1280x720
Aligned encoder configurations to VI layout:            main=1280x720,   sub=640x360
```

main is configured 1280x720 but capped at 640x360 — the max attrs are swapped. `profiles.toml` is
byte-identical to `.146`'s, so this is the `vendor-ipc-legacy-mapping` quirk, not config drift.
Start there. A client at 192.168.3.6 polls RTSP every 10 s, so this is user-visible.

### Correction 3: the VI max-attr mapping is NOT the bug

`video_input.rs:138-146` inverts `main_max`/`sub_max` **on purpose**, mirroring the proven
libre_anyka_app C workaround ("in vendor IPC mode, `main.max_*` drives sub-channel validation").
`.146` runs the identical code with a working main stream. **Do not change it** — that would break
the cameras that work.

The phantom "main never registers" came from a truncated `tail -18`, which cut the line where
`main`'s own registration appears (publish order is main-RTSP first). Untruncated, all four
transceivers succeed on both good and bad starts:

```
transceiver_run_success ... stream_name: main
transceiver_run_success ... stream_name: live/main
transceiver_run_success ... stream_name: sub
transceiver_run_success ... stream_name: live/sub
```

**Actual behaviour, measured.** RTSP `/main` availability is decided **per process start** and is
permanent for that process — retried at t+30/60/90/120 s it never recovers. Restarting onvif-rust
is the only lever; roughly 2 of 5 observed starts came up good. FLV `live/main` works in *both*
cases, so only RTSP DESCRIBE differs. A good start logs
`hub: request stream_id=... main` then `DESCRIBE stream_path=main sdp_media_count=1`.

**Next step:** capture a *failing* instance at `level = "info"` and diff those lines against a good
one — does `request stream_id` appear at all, and is `sdp_media_count` 0? Unresolved.
