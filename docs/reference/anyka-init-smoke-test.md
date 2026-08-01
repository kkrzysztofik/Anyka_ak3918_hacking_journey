# anyka-init Hardware Smoke Test

Manual checklist for validating the boot/runtime supervisor on a real camera.
Host-side tests cannot reach `reboot`, `insmod`, `clock_settime`, the vendor
boot chain, or the ONVIF auth path — everything below exists because it can
only be proven on hardware.

Design: `docs/plans/2026-08-01-boot-runtime-rust-design.md`
Plan: `docs/plans/2026-08-01-boot-runtime-rust.md`

## Preparing the card

```bash
cd <repo>
source ./setenv.sh
./scripts/build_sd_contents.sh
$EDITOR SD_card_contents/anyka_hack/anyka.toml   # set [wifi] ssid/password
./scripts/copy_sd_contents.sh --sd /path/to/mounted/card
sync
```

`[wifi].ssid` and `password` ship as `CHANGE_ME`. They parse fine — the
supervisor has no way to know they are placeholders — so a card deployed
unedited will boot, fail to associate, and sit there. Check this first when a
camera comes up unreachable.

## Checklist

Record PASS/FAIL and paste real output. Do not mark a row done from inference.

### 1. Boot and recovery channel

Insert the card, power on, wait ~30 s.

```
telnet <camera-ip> 24
```

**Expect:** a root shell. This is the P0 telnet started by
`/mnt/Factory/config.sh` *before* the binary is touched, so it must work even
when everything downstream is broken.

| Result | |
|---|---|
| Status | |
| Notes | |

### 2. Supervisor started

```sh
grep "anyka-init starting" /mnt/logs/anyka-init.log
```

**Expect:** one line with a version field.

**If empty:** the ELF never ran. Check `ls -l /mnt/anyka_hack/anyka-init.bin`
and that `/mnt/anyka_hack/lib/ld-uClibc.so.1` exists — the binary's interpreter
lives on the card, not in the rootfs.

| Result | |
|---|---|
| Status | |
| Notes | |

### 3. Services running

```sh
ps | grep -E "vendor-daemon|onvif-rust" | grep -v grep
```

**Expect:** both, each with a PID. Cross-check against the log:

```sh
grep started /mnt/logs/anyka-init.log
```

| Result | |
|---|---|
| Status | |
| Notes | |

### 4. Environment isolation

The regression this guards: two incompatible uClibc versions coexist on this
device, and a leaked `LD_LIBRARY_PATH` breaks every busybox applet a service
spawns. See the comment in `SD_card_contents/anyka_hack/onvif/onvif-rust`.

```sh
cat /proc/$(pidof vendor-daemon.bin)/environ | tr '\0' '\n'
```

**Expect:** exactly the two keys from `[services.vendor-daemon].env` —
`LD_LIBRARY_PATH=/mnt/anyka_hack/vendor-daemon/lib` and
`VENDOR_DAEMON_LOG_LEVEL=info`. Nothing else: no `PATH`, no `TZ`, no `HOME`,
nothing inherited from the vendor's `service.sh`.

```sh
cat /proc/$(pidof onvif-rust.bin)/environ | tr '\0' '\n'
```

**Expect:** empty. `[services.onvif]` declares no `env`, and `onvif-rust.bin`
has its RPATH embedded.

| Result | |
|---|---|
| Status | |
| Notes | |

### 5. Clock set before ONVIF starts

```sh
date
grep -n "stepped system clock" /mnt/logs/anyka-init.log
grep -n "started" /mnt/logs/anyka-init.log
```

**Expect:** the correct year, and the `stepped system clock` line **above** the
`service=onvif ... started` line. Ordering matters: `ws_security.rs:264-288`
expires its replay-nonce cache against `Utc::now()`, so a large step underneath
a live `onvif-rust` either purges the cache wholesale or freezes it.

**If the step line is missing:** the network was not up within
`[time].first_sync_timeout_sec` (15 s). Not a failure by itself — the resync
thread retries every `retry_interval_sec` (30 s) until it succeeds. Re-check
after a minute.

| Result | |
|---|---|
| Status | |
| Notes | |

### 6. Authenticated ONVIF request

The check that fails silently with the old fire-and-forget `ntpd`. Use any
ONVIF client with WS-UsernameToken credentials configured.

**Expect:** an authenticated call (e.g. `GetProfiles`) returns 200 with a real
body — not a `NotAuthorized` fault. `ws_security.rs:85` allows ±300 s of skew,
so this is a direct test of step 5 having worked.

| Result | |
|---|---|
| Status | |
| Notes | |

### 7. Restart on crash

```sh
pidof vendor-daemon.bin              # note the PID
kill -9 $(pidof vendor-daemon.bin)
sleep 4
pidof vendor-daemon.bin              # must differ
tail -5 /mnt/logs/anyka-init.log
```

**Expect:** a new PID within ~3 s, and log lines `service exited` then
`started`. The reaper polls once per second and `backoff_min_sec` is 1, so
budget up to ~2 s plus scheduling.

| Result | |
|---|---|
| Status | |
| Notes | |

### 8. Backoff escalation

Point a service at a path that does not exist, then reboot:

```sh
vi /mnt/anyka_hack/anyka.toml        # [services.dropbear] enabled = true,
                                     # exec = "/mnt/anyka_hack/nope"
reboot
```

After a minute:

```sh
grep -E "start failed|started" /mnt/logs/anyka-init.log
```

**Expect:** repeated `start failed` for `dropbear` with gaps of roughly
1, 2, 4, 8, 16, 32, 60, 60 s. Other services keep running throughout.

Restore `enabled = false` afterwards.

| Result | |
|---|---|
| Status | |
| Notes | |

### 9. Bad config parks safely

```sh
cp /mnt/anyka_hack/anyka.toml /mnt/anyka_hack/anyka.toml.bak
echo "this is not toml" >> /mnt/anyka_hack/anyka.toml
reboot
```

**Expect:** camera boots, `telnet <ip> 24` still works, no services running,
and an error on the UART console naming the parse failure. The supervisor must
**not** substitute defaults — a guessed `wifi_ssid` joins the wrong network and
a guessed `sensor_module` loads the wrong kernel module.

```sh
cp /mnt/anyka_hack/anyka.toml.bak /mnt/anyka_hack/anyka.toml && reboot
```

| Result | |
|---|---|
| Status | |
| Notes | |

### 10. Safe mode after a reboot storm

```sh
mkdir -p /mnt/anyka_hack/state
echo '{"fast_reboots":3}' > /mnt/anyka_hack/state/boot.json
sync
reboot
```

**Expect:** `grep "SAFE MODE" /mnt/logs/anyka-init.log` hits, no services start,
telnet works. This is the bound on reboot-on-crash-loop: three consecutive fast
reboots and the camera parks in a diagnosable state instead of power-cycling
unattended forever.

```sh
rm /mnt/anyka_hack/state/boot.json && reboot
```

Then confirm the counter self-clears: after ~10 minutes of uptime,

```sh
cat /mnt/anyka_hack/state/boot.json
grep "storm-guard counter reset" /mnt/logs/anyka-init.log
```

**Expect:** `{"fast_reboots":0}` and the reset line.

| Result | |
|---|---|
| Status | |
| Notes | |

### 11. Log rotation

```sh
ls -l /mnt/logs/
```

**Expect:** no file exceeds `[log].max_bytes` (2 MB) by much, and `.1`/`.2`
generations appear once a service has restarted past the threshold.

Known ceiling, not a failure: service logs rotate only when the service is
(re)started. A stable, chatty service grows past `max_bytes` until its next
restart, because the supervisor holds its fd and renaming underneath a live
child leaves it writing to the renamed inode.

| Result | |
|---|---|
| Status | |
| Notes | |

### 12. Periodic reboot (only if enabled)

Skip unless `[reboot].enabled = true`. To test without waiting 12 hours, set
`interval_min = 2`, `jitter_max_sec = 30`, reboot, then:

```sh
grep "periodic reboot scheduled" /mnt/logs/anyka-init.log
```

**Expect:** a `delay_sec` between 120 and 150, and the camera reboots after it
elapses. Confirm the storm-guard counter is **not** incremented — a scheduled
reboot must never push a healthy camera toward safe mode:

```sh
cat /mnt/anyka_hack/state/boot.json
```

| Result | |
|---|---|
| Status | |
| Notes | |

### 13. Clean fallback with no card

Power off, remove the SD card, power on.

**Expect:** the camera boots stock firmware with the vendor app running. This
is the whole safety story of Mode A: `service.sh:179` tests `-d /mnt/Factory`,
finds nothing, and takes the branch that starts `anyka_ipc`. Nothing on the
camera's flash was modified, so there is no brick path.

| Result | |
|---|---|
| Status | |
| Notes | |

## Results

| Date | Firmware / camera | Tester | Outcome |
|------|-------------------|--------|---------|
| | | | not yet run |
