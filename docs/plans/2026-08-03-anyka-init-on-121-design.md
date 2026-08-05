# anyka-init Cutover on Camera .121 — Design

Date: 2026-08-03
Status: **implemented 2026-08-03**

## Outcome

Cut over on the second attempt. The blocker was not in this design: `W11-CAM`
does not broadcast its SSID, and `wifi.rs::wpa_supplicant_conf()` emitted no
`scan_ssid=1`, so wpa_supplicant could only ever associate to a network it had
heard a beacon from. anyka-init discovered only the one mesh satellite that
does advertise the SSID, at -85 dBm, and failed to associate there on both
driver flags; the -49 dBm router BSSID was invisible to it. The vendor chain
succeeds on the same hardware because `station_connect.sh:62` runs
`wpa_cli -iwlan0 set_network $1 scan_ssid 1`. `.198` never hit this: its SSID
is broadcast.

With `scan_ssid=1` (commit `1e3eb071`) association took **2 seconds** on `wext`,
against a 45 s timeout it previously exhausted:

```text
00:01:03 starting wpa_supplicant driver="wext"
00:01:05 wpa_supplicant associated driver="wext"
00:01:06 wifi up chip="ssv6355_ble" ssid="W11-CAM" addr="192.168.30.121"
```

Observed, all as expected:

- No vendor fallback. `probed = Some("wext")`, so `main.rs:122` patched the
  service's `-D` flag and killed the P2 instance — the `.198` success path.
- Five supervised services, no restarts in the first 9 minutes, `boot.json`
  steady at `{"fast_reboots":0,"wifi_reboots":0}`, no `wifi link unhealthy`.
- The first attempt failed and **rolled itself back**: the deadman restored
  `config.sh.gerge.bak` and rebooted, and the camera returned on the vendor
  stack in ~5 minutes with nobody on-site. That mechanism is now proven.
- `:554` still does not bind. Pre-existing and explicitly out of scope.

Two corrections to what is written below: `camera.sh` is reached via
`rc.local:30`, not only through `anyka_ipc.sh` (F2 records this correctly), and
the earlier claim that a failed dry run was caused by a SIGHUP'd deadman was
wrong — it was stale `wpa_supplicant`/`udhcpc` processes holding `wlan0`, which
wedged the radio so that neither anyka-init nor the vendor chain could recover
without a reboot.

## Problem

Camera `192.168.30.121` still boots the shell hack: `/mnt/Factory/config.sh` →
`/data/gergehack.sh`. Camera `192.168.2.198` was cut over to the Rust supervisor
on 2026-08-02 and has run it for over a day. Two cameras, two boot paths, and
every fix has to be applied twice in two different idioms.

The cutover itself is small. What makes it non-trivial is the recovery profile:

| Constraint | Consequence |
|---|---|
| Reachable only via jumphost (`ssh root@192.168.3.137` → `telnet 192.168.30.121 24`) | No out-of-band channel |
| `getty` runs on `ttySAK0` but the header is not physically accessible | No serial console |
| `/mnt/Factory/config.sh` lives on the SD card | A bad edit needs the card pulled — someone on-site |

So the design is mostly about ordering the work such that the irreversible step
happens last and against a known-good configuration.

## Findings

Everything below was read off the live cameras, not inferred from `orig/`.

### F1 — The radio is the same chip as .198

The handover note `docs/reference/2026-08-02-gc1084-venc-open-null-handover.md`
(on branch `fix/gc1084-vi-channel-mapping`) states that `.121` runs an RTL8192CU
on the vendor path and that anyka-init must not own wifi. That is wrong. Six
independent layers agree it is the same `ssv6355_ble` part as `.198`:

| Evidence | Result |
|---|---|
| on-device `tail -c +4 /etc/jffs2/hw.conf`, `${HW_READ:51:1}` | `h` (record length 64) |
| `${HW_READ:52:1}` | `2` → HighLow |
| vendor `/data/wifi_driver.sh:91-92` | `h` → `WIFI_NAME="ssv6355_ble"` |
| vendor `/data/wifi_driver.sh:350` | `insmod ssv6355.ko stacfgpath=…/ssv6355-wifi.cfg` |
| `anyka-init` `wifi.rs:79-85` | same module, byte-identical `args` string |
| `lsmod` | `ssv6x5x` — matches the row's `rmmod` field |
| `dmesg` | loaded `/tmp/ko/ssv6355-wifi.cfg`, fw `/tmp/ko/ssv6355-sw_ble.bin` |
| `ps` | `wpa_supplicant -B -iwlan0 -Dwext` |

`/sys/user-gpio/wifi_en` reads `0` with the radio up, which is only consistent
with a high→low pulse — an independent confirmation of polarity `2`. The
vendor's `tail -c +4` also confirms the offset convention in `wifi.rs:127`: the
`HW=` prefix is stripped before indexing, which is the detail that would
silently select the wrong driver if it were off by three.

### F2 — camera.sh loads the camera modules on both cameras

`/etc/init.d/rc.local:30` calls `/usr/sbin/camera.sh setup`, before
`service.sh`. Both files are byte-identical across the two cameras
(`252230e920817751a7ff49f2ba24d61b`, `76950ff770a2e2f15ff451a316ca03ce`), and
`camera_setup()` ends with:

```sh
insmod /usr/modules/akcamera.ko
insmod /usr/modules/ak_info_dump.ko
```

`.198` proves this covers both: it has no `gergehack.sh`, its `anyka.toml` names
only the sensor, and `lsmod` still shows `akcamera` and `ak_info_dump` loaded.
The three insmods in `gergehack.sh` are therefore redundant, and so was the
earlier plan to move them into `config.sh`.

The sensor is the exception, and the asymmetry runs opposite to intuition.
`camera_setup()` searches `/tmp/sensor_ko_and_isp_conf` (unpacked from
`/etc/jffs2/sensor.tgz`) and `/data/sensor_ko_and_isp_conf`. `.121`'s sensor
lives in the latter; `.198`'s lives in `/data/sensor/`, which is on neither
path. `.198` is the camera with the gap, which is why its `[system]
sensor_module` is load-bearing and its config comment is correct.

### F3 — /mnt does not need a remount

`camera_setup()` ends with `umount /mnt`, so the card looked like it might be
left unmounted or read-only by the time `config.sh` runs. It is not: `.198`
runs the identical `camera.sh`, its `config.sh` contains no remount, and
anyka-init has been writing `/mnt/logs` and `state/boot.json` there for over a
day. Something between `rc.local` and `service.sh:91` remounts reliably.

### F4 — P1 parks before wifi exists

`main.rs:16` loads the config and calls `park()` on failure — an infinite sleep
that keeps the P0 telnet reachable. Wifi bring-up is P2, at `main.rs:62`. At
boot nothing has brought the link up yet: the `FACTORY_TEST` branch of
`service.sh` skips `anyka_ipc.sh start`, and `gergehack.sh` is what calls
`wifi_manage.sh start` today.

On `.198` parking is safe. On `.121` a config typo would park the supervisor
with telnet listening on an interface that has no link, and telnet you cannot
reach is not a recovery channel. This is the single highest-risk failure mode
of the cutover, and it is a phase-ordering issue, not a bug in `park()`.

### F5 — .121 runs services .198 does not

`ps` on `.121` shows `ptz_daemon_dyn` from `/mnt/anyka_hack/ptz/`, started by
`gergehack.sh`. `.198` has no PTZ hardware and no such service.
`/mnt/anyka_hack/ptz/run_ptz.sh` shows it needs
`LD_LIBRARY_PATH=/mnt/anyka_hack/ptz/lib`.

## Design

### D1 — config.sh

`.198`'s `config.sh` plus a two-stage deadman. Nothing else.

```sh
#!/bin/sh
telnetd -p 24 -l /bin/sh 2>/dev/null &

BIN=/mnt/anyka_hack/anyka-init.bin
[ -x "$BIN" ] || { echo "anyka-init: missing $BIN" >&2; exit 1; }

# Deadman, two stages: .121 is jumphost-only and P1 parks before P2 brings
# wifi up (F4), so a config error would strand it. Stage one hands wifi back
# to the vendor; if the link is still dead a minute later, stage two restores
# the vendor boot path (atomically) and reboots into it.
( sleep 180
  ifconfig wlan0 | grep -q "inet addr" && exit 0
  /usr/sbin/wifi_manage.sh start
  sleep 60
  ifconfig wlan0 | grep -q "inet addr" && exit 0
  RESTORE="${SELF}.restore.$$"
  if [ -r "$BAK" ] && cp "$BAK" "$RESTORE" && sync && mv "$RESTORE" "$SELF" && sync; then
    reboot
  else
    echo "anyka-init: vendor boot-path restore failed" >&2
  fi ) &

exec "$BIN"
```

`service.sh:85` runs `killall telnetd` immediately before calling `config.sh`,
killing the instance `rcS` started — restarting it here is what restores the
only remote channel. P0 stays shell because `anyka-init.bin` needs the bundled
`ld-uClibc.so.1`; if that were missing the kernel could not start the ELF at
all, and shell has no such dependency.

The deadman costs nothing when anyka-init is healthy — the `grep` simply
matches — and covers both the F4 park and the case where anyka-init's own wifi
bring-up fails *and* its internal `fallback_to_vendor` fails. It converts "pull
the SD card" into "wait three minutes".

### D2 — anyka.toml

`.deploy/anyka.toml` (the `.198` file) with these deltas, and nothing else:

```toml
[system]
sensor_module = "/data/sensor_ko_and_isp_conf/sensor_gc1084.ko"

[wifi]
ssid = "W11-CAM"
password = "<from /data/gergesettings.txt>"

[time]
servers = ["192.168.3.1", "0.ubuntu.pool.ntp.org"]

[services.ptz]
enabled = true
exec = "/mnt/anyka_hack/ptz/ptz_daemon_dyn"
log = "/mnt/logs/ptz.log"
env = { LD_LIBRARY_PATH = "/mnt/anyka_hack/ptz/lib" }
```

`[services.ptz]` is the only structural addition over `.198`. Everything else —
`chip`, `gpio_polarity`, `fallback_to_vendor`, the `/tmp/wpa_supplicant` exec
path, the supervisor and monitor blocks — carries over unchanged.

`sensor_module` is kept even though `camera.sh` may already cover it on this
camera (F2). The insmod is idempotent — a second load logs `Code(1)` and is
ignored — and it is what the field is for.

`timezone` stays the POSIX `CET-1CEST,M3.5.0,M10.5.0/3` from `.198`, not the
`GMT+02:00` in `gergesettings.txt`. Under POSIX that string means UTC−2; the
vendor's usage is sign-inverted. It affects the supervisor process only.

The file lives at `.deploy/anyka-121.toml` in the repo, untracked, beside the
existing `.deploy/anyka.toml` — it carries the real PSK.

### D3 — What gergehack.sh was carrying

| `gergehack.sh` did | becomes |
|---|---|
| `wifi_manage.sh start` | anyka-init P2 |
| `ntpd -p 192.168.3.1` | `[time] servers` |
| `export TZ=GMT+02:00` | `[time] timezone`, as POSIX |
| 3 × `insmod` | `camera.sh` (F2) + `[system] sensor_module` |
| `ptz_daemon_dyn` | `[services.ptz]` |
| `web_interface` httpd `:80` | dropped — onvif-rust serves `:80` and its own `www/` |
| `libre_anyka_app` | dropped — vendor-daemon + onvif-rust replace it, and it holds `/dev/video0` |
| `ffmpeg/app_restarter.sh` | dropped with it |
| `sys_monitor.sh` | `[monitor]` |
| `periodic_reboot.sh` | `[reboot] enabled = false` |
| gergesettings self-update-then-`reboot` | gone |

### D4 — Rollback

Three tiers, cheapest first:

1. **Before the swap** — power cycle. `config.sh` is untouched, the vendor stack
   returns. No card pull, no laptop.
2. **After the swap, wifi lost** — the two-stage deadman fires at T+180 s and
   `wifi_manage.sh start` restores the link; telnet is already listening.
3. **After the swap, hard failure** — if `wlan0` is still dead T+60 s after the
   vendor retry, the deadman automatically restores `config.sh.gerge.bak` over
   `config.sh` (atomic copy + rename) and reboots; the camera returns on the
   vendor stack in ~5 minutes. If even that did not help, pull the card and
   restore `config.sh.gerge.bak` by hand — on-site, and the only tier that
   requires it.

`[monitor] wifi_reboot_cap = 3` and `[supervisor] storm_guard_max_reboots = 3`
bound any reboot loop. Safe mode forces `telnet = true` (`main.rs:58`) before
parking, so the storm guard cannot lock itself out.

## Validation

### Dry run

`config.sh` stays untouched throughout, which is what makes tier-1 rollback
available and keeps the SD card out of it.

1. Stage `/mnt/anyka_hack/anyka.toml`; confirm it parses by running
   `anyka-init.bin` and checking it gets past P1.
2. Stop `vendor-daemon`, `onvif-rust`, `ptz_daemon_dyn`, `ntpd`, `wifi_run.sh`.
   Leave `telnetd`.
3. Arm the deadman line by hand — this exercises it.
4. Launch detached: `nohup /mnt/anyka_hack/anyka-init.bin > … 2>&1 &`.
5. Disconnect, wait ~90 s, reconnect.

The run kills its own telnet session: P2 rmmods and reinserts `ssv6355.ko`, so
the link drops mid-run. That is why it must be detached and the result read
after reconnecting rather than watched live. Abort is `reboot` — `config.sh` is
still the old one.

### Pass criteria

Checked after the dry run, then again after the cold boot:

- `wlan0` holds an IP; telnet reachable through the jumphost
- log shows `wifi up` with `chip=ssv6355_ble`, `driver=wext`
- `lsmod` lists `sensor_gc1084`, `akcamera`, `ak_info_dump`, `ssv6x5x`
- `vendor-daemon.bin`, `onvif-rust.bin`, `ptz_daemon_dyn` all running;
  `fuser /dev/video0` returns vendor-daemon only
- both encoder groups logging `grp_type=1` and `grp_type=2`
- `:80` bound by onvif-rust
- `state/boot.json` reads `{"fast_reboots":0,"wifi_reboots":0}`

### Cutover

Only after the dry run passes: back up `config.sh` to `config.sh.gerge.bak`,
write the new `config.sh`, reboot, re-check the same criteria from cold.

## Out of scope

The open `:554` bind failure and the `ak_vi_get_frame` / `H264EncStrmEncode`
frame starvation on `.121` are untouched by this cutover, and the pass criteria
deliberately do not require them. Gating a boot-path change on a pre-existing
streaming bug would block the deploy on an unrelated fix.

`/mnt/anyka_hack/lib/` on `.121` holds 4 files against `.198`'s 11. The binary
runs — it reached P1 and reported the missing config — so nothing it needs is
absent, but the directory is thinner than the reference and worth reconciling
separately.
