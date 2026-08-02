# Handover: GC1084 board (`.121`) — `ak_venc_open` returns NULL after VI attaches

**Date:** 2026-08-02
**Status:** open — blocks full modernization of the camera at `192.168.30.121`
**Prereq that is DONE:** the VI sensor-inversion fix (PR #56,
`fix/gc1084-vi-channel-mapping`). Deploy that build before reproducing this.

---

## One-paragraph summary

On the GC1084 board at `192.168.30.121`, the modern stack (`vendor-daemon` +
`onvif-rust`) now gets the video input channel up at 640×360 (the inversion fix
landed). The **next** step fails: the vendor daemon's `ak_venc_open(enc_grp=1)`
returns a NULL handle, so no encoder starts, RTSP (`:554`) never binds, and
`onvif-rust` tears the pipeline down and retries every few seconds. This note is
everything needed to pick that up cold.

## Environment / access

- Target: `192.168.30.121`, AK3918, kernel 3.4.35, 36 MB RAM, **GC1084** sensor,
  SD is **exfat**. Original "gerge" SD-exploit stack (config.sh → gergehack.sh →
  `libre_anyka_app -w 640 -h 360 -i 4 -u`).
- Reach it **only** via jumphost: `ssh root@192.168.3.137`, then from there
  `telnet 192.168.30.121 24` (passwordless root shell) and
  `ftp root:www123@192.168.30.121` (writable, rooted at `/`). Jumphost has
  `telnet`/`nc`/`ftp`/`busybox`, **no** `lftp`/`curl`.
- No serial console available. Boot init is untouched; recovery from a bad boot
  = pull the SD (someone must be on-site). **Keep WiFi on the vendor path** — the
  RTL8192CU is brought up by the vendor `wifi_manage.sh` from `anyka_cfg.ini`,
  NOT `.198`'s `wpa_supplicant` path. Do not let anyka-init take over WiFi here.

## What is already staged on `.121`

Under `/mnt/anyka_hack/` on the SD (non-destructive, boot untouched):
`onvif/onvif-rust.bin` (the PR #56 build), `onvif/onvif-rust` launcher,
`onvif/config.toml`, `onvif/www/`, `vendor-daemon/` + `vendor-daemon/lib/` (54
libs), `lib/` (minimal uClibc set: `ld-uClibc.so.1`, `libc.so.0`,
`libgcc_s.so.1` — `onvif-rust.bin`'s interpreter is
`/mnt/anyka_hack/lib/ld-uClibc.so.1`), and `anyka-init.bin`.

The device `config.toml` has `[stream_profile_1]` main = **640×360** (required:
the encoder main must match the clamped VI channel). Sub profiles are 640×480.

## Exact symptom

Fresh `vendor-daemon` + `onvif-rust` run, `vendor_daemon.log`:

```
[vi_set_capture_on] ... malloc_capture_buffers ... stream on   <- VI OK (fix works)
[ak_venc_open:1833] init enc_grp=1, enc_mode=0
[ak_venc_open:1890] VideoStream_Enc_Open!
ERROR src/handlers_venc.c:78: [venc] open failed (NULL handle)   <- first failure
[vi_set_capture_off] ... capture off ...
...(retry ~4s later)...
[venc_handle_init:581] group 1's handle has already opened       <- cascade on retry
[ak_venc_open:1833] init enc_grp=1 ...
ERROR handlers_venc.c:78: [venc] open failed (NULL handle)
```

`ps`: both `vendor-daemon.bin` and `onvif-rust.bin` alive. `netstat`: `:80`
listens, **`:554` does not**. `wlan0` stays up throughout.

## Established facts

- The **first** `ak_venc_open` on a freshly started daemon fails NULL for
  `enc_grp=1` — it is not only a retry artifact. The "already opened" line
  appears on the 2nd+ attempts (cascade after the first NULL leaves the group
  half-open).
- Only `enc_grp=1` appears in the log. Group index 0 is not logged (either it
  opens silently first, or onvif opens 1 first — unconfirmed).
- `.198` (different sensor, main VI = 1280×720) runs the identical `onvif-rust`
  encoder code and identical sub config (VI sub 640×360, sub encoder 640×480)
  and **streams fine**. So the sub 640×480-vs-360 resolution combo is NOT the
  cause by itself.
- What is genuinely different on `.121`: main VI = 640×360 (vs .198's 1280×720)
  and main encoder = 640×360 (vs 1280×720).

## Ruled out

- VI channel attr / sensor inversion — fixed, VI attaches, buffers allocate.
- Sub encoder res exceeding VI sub max — the fix leaves sub max at sensor size
  (1280×720), so 640×480 fits; and .198 has the same combo and works.

## Leading hypotheses (unranked)

1. **Leaked encoder group across ungraceful restart.** The dry-run loop killed
   `onvif-rust`/`vendor-daemon` (and earlier `libre_anyka_app`) with SIGTERM/KILL.
   Anyka encoder groups are kernel/HW resources that may not release on an
   unclean exit, so a later `ak_venc_open(1)` finds the group already held → NULL.
   **Test:** clean boot, prevent `libre_anyka_app` from ever opening the encoder
   (see below), start daemon+onvif **once**, observe the first open.
2. **Real encoder param issue at 640×360 main on this board.** e.g. group/mode
   mapping, a param the SDK rejects for a 640×360 main that it accepts at 720p.
   **Test:** get onvif's exact `venc_open` params (below) and compare to .198.
3. **enc_grp numbering / order.** onvif may open group 1 before group 0, or the
   daemon expects group 0 first. Check `handlers_venc.c` around line 78 and the
   `ak_venc_open` group/mode arguments the daemon received over IPC.

## First diagnostic to run (cheap, high value)

**Get onvif-rust's real log.** It writes to `/tmp/onvif.log`
(`[logging] file_path`, `console_enabled=false`), but that file was **absent**
during the dry-runs — so either onvif never reached logging init, or `/tmp` there
is unexpected. Fix this first: set `console_enabled=true` (or point
`file_path` at `/mnt/logs/onvif.log`) in the device `config.toml`, rerun, and
read onvif's own encoder-open params and error. Without onvif's side, you are
guessing at what it asked the daemon to open.

## Clean-room reproduction (isolates leak vs config)

1. Set `run_libre_anyka=0` (and ideally `run_web_interface=0`) in
   `/data/gergesettings.txt` on `.121`, then reboot. Now nothing grabs the
   encoder/`:80` at boot.
2. On the clean boot (no prior encoder opens):
   `LD_LIBRARY_PATH=/mnt/anyka_hack/vendor-daemon/lib nohup ./vendor-daemon/vendor-daemon.bin >/mnt/logs/vendor_daemon.log 2>&1 &`
   then `nohup ./onvif/onvif-rust >/mnt/logs/onvif.log 2>&1 &`.
3. If `ak_venc_open(1)` now **succeeds** → it was hypothesis 1 (leak); the real
   fix is graceful encoder teardown / group release on daemon or onvif exit
   (see the vendor-daemon restart-resilience work).
4. If it still fails NULL on a clean boot → hypothesis 2/3; dig into the
   `ak_venc_open` args (group, mode, res, bitrate, gop) vs a known-good `.198`
   capture.
5. **Restore** `run_libre_anyka=1` and reboot when done.

## Build / deploy loop (for reference)

- Toolchain: `source ./setenv.sh` sets `$CARGO`; target
  `armv5te-unknown-linux-uclibceabi`; artifacts land in `cross-compile/target/...`.
- Rebuild: `$CARGO build --release` (~5 min). Push just the binary:
  `scp` to jumphost `/tmp`, then jumphost `ftp` PUT to
  `/mnt/anyka_hack/onvif/onvif-rust.bin.new`; on `.121` `mv` it over
  `onvif-rust.bin` (avoids ETXTBSY on the running file) and restart.
- The camera is left DOWN while the modern stack runs; **reboot `.121` to
  restore** its original 640×360 stack (boot is untouched, comes back clean).

## Pointers

- Fix that got us here: PR #56 (`fix/gc1084-vi-channel-mapping`),
  `src/platform/anyka/video_input.rs` `set_channel_attr`.
- Daemon venc handler: `vendor-daemon` `src/handlers_venc.c:78`,
  `ak_venc_open` around `:1833`/`:1890`.
- onvif encoder init: `src/platform/anyka/mod.rs:336` (dual encoder init),
  `src/platform/anyka/video_encoder.rs`.
- Vendor-daemon restart-resilience / handle-leak context: see the Serena memory
  `vendor-daemon-restart-resilience-status`.
