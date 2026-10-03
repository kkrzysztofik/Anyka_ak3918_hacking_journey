# JUAN SPI-NOR flash dump vs. our stock firmware

`juan-flash-dump.bin` is a raw 8 MB SPI-NOR read of an AK3918EV200 camera that never ran
our hack. It is the same chip as our fleet but a **different OEM** (JUAN / Juanvision) on
a **newer SDK revision**: Anyka's AKv200 SDK V5.1.3.2, July 2024. Our cameras run an
Ants/Yi build. This page records what the dump contains and where it differs from
`orig/` (the gitignored capture of our stock rootfs).

| | |
|---|---|
| md5 | `3daa299a1a9c4df06c484002c472175f` |
| sha256 | `9bc153e1ef840dedbce64e1ee7fe5d099786dc8ea889d736f7e24f5f478cb6ac` |
| Analysed | 2026-09-27 |

The image is unmodified, so its config partition still carries that unit's MAC, serial,
`sn.bin`, `resolv.conf` and password hashes.

## Flash layout

| Offset | Size | Content |
|---|---|---|
| `0x000000` | 256 KB | U-Boot, `AKSH_240` boot header |
| `0x040000` | 128 KB | erased |
| `0x060000` | 128 KB | binary blob (not a text env) |
| `0x080000` | ~1.46 MB | uImage, `Linux-3.4.35`, uncompressed wrapper around an xz piggy at +0x3868 |
| `0x200000` | ~845 KB | xz squashfs, rootfs |
| `0x300000` | ~4.7 MB | xz squashfs, `/usr` (`mtdblock5`) |
| `0x780000` | 512 KB | JFFS2, `/etc/jffs2` (`mtdblock6`) |

There is no `/data` partition. Ours mounts `mtdblock7` there.

### Extracting

```bash
D=docs/reference/juan-flash-dump.bin; W=/tmp/akdump; mkdir -p $W
dd if=$D of=$W/kernel.uImage bs=64K skip=8 count=24
dd if=$D of=$W/root.sqsh bs=1M skip=2 count=1
dd if=$D of=$W/usr.sqsh  bs=1M skip=3 count=5
dd if=$D of=$W/cfg.jffs2 bs=64K skip=$((0x78))
unsquashfs -d $W/root $W/root.sqsh
unsquashfs -d $W/usr  $W/usr.sqsh
uvx jefferson -d $W/cfg $W/cfg.jffs2
```

The kernel is an xz stream at offset `0x3868` past the 64-byte uImage header;
`lzma.LZMADecompressor()` unpacks it to a 4.4 MB `vmlinux`.

The unpacked filesystems are committed under `juan-flash-dump/` as `rootfs/`, `usr/`
and `jffs2/`, produced by the commands above. Git doesn't store empty directories
(the mount points) or permission bits beyond the executable bit. Three files were
deleted after extraction: `usr/local/rsa_private_key.pem`, `jffs2/shadow` and
`jffs2/sn.bin`. They still exist inside `juan-flash-dump.bin`.

## Differences from our stock firmware

| Area | Ours (`orig/`) | Dump |
|---|---|---|
| Kernel | 3.4.35, `chensheng@ants-szfir` | 3.4.35, Anyka Jenkins `ipc_system_AKv300_AKv200`, 2024-03-27 |
| Toolchain | gcc 4.8.5 / uClibc 0.9.33.2 | identical |
| Wifi | SSV6355 (`ssv6355.ko`) / zt9101 | ATBM6031x (`atbm6031x.ko`, `atbm_iw`, `hostapd`) |
| Sensors | GC1084 | h63, sc1245, sc1345, sc1346 (`.ko` + `isp_*.conf`) |
| Motor | built into the kernel, `/dev/ak-motor{0,1}` (misc) | loadable `ak_motor.ko` + `aw9523.ko` GPIO expander, `/dev/motor%d` |
| Extra modules | — | `ak39_top_wdt`, `akmci`, USB LTE (`option`, `usb_wwan`, `cdc_*`, `rndis_host`), `bridge` |
| Main app | `anyka_ipc` 648 KB + Yi P2P/cloud `.so`s | `anyka_ipc` 5.5 MB |
| Protocols in the app | vendor cloud only | N1 (80), RTSP (554), ONVIF (8888), CORSEE (12306), P2P (60002), GB28181, RTMP, HLS (`xsdk.json`) |
| Cloud | Yi | Juanvision (`ngw.dvr163.com`, `*.kp2p.dvr163.com`, `stun.msndvr.com`); empty Tuya and LinkVisual slots |
| Telnet | `telnetd &` in `rcS` (killed later by `service.sh`) | removed from `rcS` |
| FTP / syslog | `tcpsvd 0 21 ftpd`, `syslogd`, `klogd` | all commented out |
| `rc.local` | — | also sets `eth0 192.168.1.123` and stages the watchdog `.ko`, `reboot` and `nk_update.sh` into `/tmp` |
| Root password | `$1$www12345$…` | `/etc/passwd` DES `ABgia2Z.lfFhA`; `/etc/jffs2/shadow` `$1$6AHjBnTn$…` |
| Extra files | — | `/usr/local/rsa_private_key.pem`, `IOTDaemon` (respawn loop), `daemon_server` |

### Boot and SD-card hooks

Our hack gets in through `service.sh`. When `FACTORY_TEST=1` it runs
`/mnt/Factory/config.sh`, which is `SD_card_contents/Factory/config.sh`. The dump's
`service.sh` has none of the mode flags: `start_service` only calls `anyka_ipc.sh start`.

The dump's only SD-card hook is in `anyka_ipc.sh`. It mounts `mmcblk0p1` at `/mnt/tf`,
and if `/mnt/tf/debug.ini` exists together with `/mnt/tf/anyka_ipc_nostrip`, it
bind-mounts that file over `/usr/bin/anyka_ipc` and runs it as root. It also enables
cores to `/mnt/tf/core/`. That is the equivalent entry point on this firmware. It has
not been tried on hardware.

### Vendor libraries

Both `anyka_ipc` binaries statically link the MPI/platform layer. `strings` shows the
embedded versions:

| Module | Ours | Dump | Repo `vendor-daemon/lib` |
|---|---|---|---|
| `libplat_vi` | V2.3.02 | V2.3.03 | V2.3.01 |
| `libplat_ao` | V2.4.03 | V2.4.04 | V1.2.02 |
| `libplat_ai` | V2.9.01 | V2.9.01 | V2.5.03 |
| `libplat_thread` | V2.1.00 | V2.1.00 | V2.0.00 |
| `libplat_common` | V2.1.05 | V2.1.05 | V2.1.05 |
| `libplat_drv_ptz` | V1.0.01 | V1.0.01 | — |
| `libmpi_venc` | V2.2.06 | V2.2.06 | V2.2.06 |
| `libmpi_osd` | V2.1.00 | V2.1.00 | V1.1.03 |
| `libmpi_md` | — | V4.0.16 | — |

Shared libraries in `/usr/lib`:

| Library | Ours | Dump |
|---|---|---|
| `libakstreamenc.so` | V1.10.08 `8df1c7c9` | identical |
| `libakispsdk.so` | V3.1.00 `1741e74b` | identical |
| `libakaudiofilter.so` | `938d71ff` | identical |
| `libakmedia.so` | V1.19.00_svn5741 | V1.19.20 |

`libplat_ao` is still static-only here, so a shared build of it still doesn't exist.
The dump also pairs `libakstreamenc` V1.10.08 with `libmpi_venc` V2.2.06 without
trouble. That works only because the venc layer is compiled in; the shared
`libmpi_venc.so` we ship was built before the V1.10.08 version gate.

## Motor driver: `ak_motor.ko`

This is the most useful part of the dump. `modinfo`: `vermagic=3.4.35 mod_unload ARMv5`,
`alias=platform:ak-motor`.

`ak_motor_ioctl` dispatches only on the "Anycloud V500 Porting" command family. Every
command is encoded `_IOW('m', nr, int)`, even where a struct pointer is passed:

| nr | Command word | Handler |
|---|---|---|
| `0x00` | `0x40046D00` | `copy_from_user` 24 bytes, i.e. `MOTOR_PARM` |
| `0x20` | `0x40046D20` | move |
| `0x21` | `0x40046D21` | move |
| `0x40` | `0x40046D40` | turn |
| `0x41` | `0x40046D41` | turn |
| `0x42` | `0x40046D42` | `motor_stop` |
| `0x43` | `0x40046D43` | `copy_to_user` 36 bytes, i.e. `MOTOR_GET_STATUS` |
| `0x60` | `0x40046D60` | `motor_reset` |
| `0x61` | `0x40046D61` | `turn_middle` |
| `0x62` | `0x40046D62` | cruise |
| `0x63` | `0x40046D63` | boundary |

The legacy family (nr 11-16: `SET_ANG_SPEED`, `TURN_CLKWISE`, …) is absent. That is the
exact complement of the driver built into our kernel, which implements only the legacy
family and silently accepts V500 commands as no-ops.

### Struct layouts

Recovered from the handler's loads and stores:

```c
struct motor_parm {           /* MOTOR_PARM, 24 bytes */
    int pos;
    int speed_step;
    int steps_one_circle;
    int total_steps;
    int boundary_steps;
    int phase_mode;           /* 8 = 8-entry half-step table, else 4-entry */
};

struct motor_message {        /* MOTOR_GET_STATUS, 36 bytes */
    int status;               /* 0 when runtime state == 2, else 1 */
    int pos;
    int speed_step;
    int speed_angle;          /* speed_step * 360 / steps_one_circle */
    int steps_one_circle;
    int total_steps;
    int boundary_steps;
    int attach_timer;
    int phase_mode;
};
```

`MotorParm` and `MotorMessage` in `cross-compile/onvif-rust/src/hal/anyka/ptz/driver.rs`
match these field for field but stop one `int` short, at 20 and 32 bytes. Against this
driver, `MOTOR_GET_STATUS` would write 4 bytes past our struct. On our own kernel this
has no effect, because the V500 handler doesn't exist there.

### Loading it on our cameras

Not tried. The module imports `akv200_board_version`, `aw9523b_gpio_set` (exported by
the dump's `aw9523.ko`), and `ak39_timer_probe/start/stop/remove`. It also registers the
same `ak-motor` platform driver name as our built-in one. To check whether it could
resolve, run `grep -E 'akv200_board_version|ak39_timer_probe' /proc/kallsyms` on a
camera.

## Config partition

`/etc/jffs2` holds JSON (`config/*.json`, `metadata/*.json`, `factory.json`) plus
`WifiMac`, `BurnSN`, `sn.bin`, `passwd`, `shadow` and `resolv.conf`. `oem.json` names
the OEM `"JUAN"`; `p2p.json` and `cloud.json` are unprovisioned.
