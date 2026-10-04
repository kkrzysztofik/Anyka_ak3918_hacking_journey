# Flash rootfs cleanup: replace the vendor boot path on both board types

Date: 2026-10-04
Status: design approved (approach 1: clip pilot, then in-system)

## Goal

Replace each camera's root squashfs with a minimal one that boots our stack from
SD directly. That drops the vendor's insecure services and our own factory-mode
hacks. Unlock the U-Boot serial console while we are at it. The cameras stay
SD-dependent. Kernel, U-Boot and `/usr` are not touched in this project.

Boards in scope:
- **Cloud39EV2**: .198, .121, .146, .127 (Yi stock, `/mnt/Factory/config.sh` hook)
- **JUAN**: .148 (Juanvision stock, `debug.ini` + `anyka_ipc_nostrip` hook)

Recovery hardware: SPI programmer with SOIC-8 clip, plus UART.

Out of scope: dropbear / dropping telnet 24 (separate project), stripping `/usr`,
kernel changes.

## Flash map (Cloud39EV2)

From Anyka's partition table captured in `orig/sys/kernel/partition_table/`:

| mtd | name | offset | size | content |
|---|---|---|---|---|
| 0 | (U-Boot) | 0x000000 | 196 KB | bootloader |
| 1 | KERNEL | 0x031000 | 1.5 MB | uImage |
| 2 | MAC | 0x1b1000 | 4 KB | per-unit MAC |
| 3 | ENV | 0x1b2000 | 4 KB | U-Boot env, 591 bytes used |
| 4 | A | 0x1b3000 | 1 MB | root squashfs (952 KB used) |
| 5 | B | 0x2b3000 | ~3 MB | `/usr` squashfs |
| 6 | C | 0x5a8000 | 64 KB | `/etc/jffs2` |
| 7 | D | 0x5b8000 | ~2.2 MB | `/data` jffs2 |

JUAN layout is in `docs/reference/juan-flash-dump.md`. Its env location is
unconfirmed (`0x060000` holds a binary blob, not a text env); `/proc/mtd` and the
unit's own backup settle it.

The vendor U-Boot source (`anyka_reference/component/uboot/include/configs/ak3918.h`)
compiles `CONFIG_BOOTDELAY 3`, so the 0 delay on our units most likely comes from
the saved environment.

## Rootfs contract

The new rootfs is a small, stable stage that rarely changes:

1. mount `/proc`, `/sys`, `/dev` (mdev), `/usr`, the jffs2 partitions, the SD card
2. `exec /mnt/anyka_hack/boot.sh` if it exists
3. otherwise stay up with only the UART console (recovery)

`boot.sh` replaces `Factory/config.sh`: telnet 24, slot choice, respawn loop,
deadman. Boot logic stays updatable on SD without reflashing.

Each board's rootfs is built by **subtracting from its own stock squashfs**:
busybox, uClibc and the module ABI stay as shipped; only scripts change.
`mksquashfs -comp xz -b 131072`, must fit the partition.

| Removed | Cloud39EV2 | JUAN |
|---|---|---|
| Passwordless telnetd | `rcS` `telnetd &` | already gone |
| Root FTP writable at `/` | `rc.local` `tcpsvd … ftpd -w /` | already commented out |
| Vendor app and cloud | `service.sh` → `anyka_ipc`, `cloudAPI`, `cmd_serverd`, `daemon`, `mdns` | `anyka_ipc.sh` → `anyka_ipc` (N1, ONVIF 8888, P2P 60002, GB28181, cloud), `IOTDaemon` |
| Factory/debug backdoors | `FACTORY_TEST`, `/mnt/debug`, `/mnt/update`, `tf_burn_id.sh` | `debug.ini` bind-mount hook |
| Vendor plumbing | `camera.sh setup`, loop-vfat ramdisk | `eth0 192.168.1.123`, watchdog staging |
| UART | `getty` with unknown hash → our root hash | same |

JUAN specifics: SD mounts at `/mnt/tf`, the `/mnt/anyka_hack` symlink stays
(RPATH). Removing `anyka_ipc` removes what loads the ATBM wifi, so JUAN cutover is
blocked on phase 1 of `2026-09-28-juan-board-support-design.md`.

## Flashing procedure

Neither busybox has `flashcp`/`flash_eraseall`. Both have `dd` and `md5sum`, and
partitions are 4 KB-aligned. One method on both boards: `dd` to `/dev/mtdblockN`,
then a read-back md5. No vendor `updater`.

### Pilot: .198 with the clip

1. Clip-read 8 MB three times; md5s must match. Per-unit golden backup (MAC and
   ENV are unit-specific; never share an image).
2. Carve ENV on the host and determine its real format (standard
   `crc32 + key=val`, or Anyka-wrapped). Patch `bootdelay=3`, fix the checksum.
3. Build the rootfs, check its size.
4. Splice both into a copy of the dump, clip-write, clip-read, diff.
5. Boot on UART: U-Boot interrupts, kernel boots, `boot.sh` runs, stream is up.
   Confirm U-Boot has `sf` and `mmc`/`fatload` for the recovery path below.

### Every later unit: in-system over telnet 24

1. `cat /dev/mtdblock{0..7}` to SD, pull to host with an md5 manifest.
2. Copy busybox to `/tmp` and run everything after this from there: mtdblock4 is
   the mounted root, and a page fault into it mid-write crashes the shell.
3. `dd` rootfs to mtdblock4, `sync`, read back, compare md5.
4. Reboot only on a match. On mismatch the old system is still in RAM, so retry.
5. ENV in a separate write and a separate reboot, after the rootfs is proven.

## Deadman change (prerequisite)

`config.sh`'s stage-2 fallback restores `config.sh.gerge.bak`, the vendor boot
path. The cleanup deletes that path, so `boot.sh` replaces stage 2:

- stage 1 unchanged: `wifi_manage.sh start` (on `/usr`, still present)
- stage 2: flip `active` to the other slot and reboot; a boot counter in
  `/mnt/anyka_hack` caps it at two flips, after that park with telnet 24

This is SD-side only and ships to every camera before any flash write.

## Recovery ladder

| Broken | Recovery |
|---|---|
| `boot.sh` / slot content | edit SD; remotely, the deadman slot flip |
| SD card | rootfs falls back to UART console |
| rootfs (mtd4) | UART → U-Boot → `fatload mmc` backup rootfs, `sf erase` / `sf write` |
| ENV or U-Boot | clip, write the unit's own backup |

## Per-unit pass criteria

- read-back md5 matches before reboot
- after reboot: telnet 24, RTSP `/main` + `/sub`, FLV decode, WebUI via curl
- nothing listening on 21, 23, 8888, 60002 (`netstat -ltn`); no vendor processes
- second reboot, confirmed by uptime reset

## Rollout

1. SD-side `boot.sh` + new deadman on all units (no flash writes)
2. .198 pilot (clip + UART)
3. .148 JUAN, after JUAN plan phase 1
4. .146, then .121, each after a week of .198 clean
5. .127 last (lone zt9101 board)
