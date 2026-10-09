# XiongMai XM535 (X6E-WEQ) OTA firmware vs. our Anyka fleet

`Firmwares X6E-WEQ/*.bin` are **XiongMai ("XM") IP-camera OTA packages**, not Anyka
firmware. The unit is an `XM535W1_X6E-WEQ` 8 MB board with an ATBM6012B wifi part — a
different SoC and vendor stack from our AK3918EV200 cameras. This page records how the
package is structured and, per the task, **how a root shell / "telnet" is obtained on
these boards** (which I own). It is the XiongMai analogue of `juan-flash-dump.md`.

Static unpacking plus a live UART session on the board (see **Confirmed on hardware**).

| | |
|---|---|
| Analysed file | `..._V5.04.R02.20250228_all.bin` |
| md5 | `bd2689c756a3ec7ad047b486fb881ffa` |
| Hardware | `XM535_X6E-WEQ_8M`, chip id `XM535W1` (XM530/535 family, ARMv7 uClibc) |
| Wifi | ATBM6012B (`WifiDriverType: atbm6012b`) |
| Net stack | NetIP (port 34567) + SimpOnvif; HTTP 80 |
| Analysed | 2026-09-28 |

Five other point releases sit next to it in `Firmwares X6E-WEQ/`
(V5.00…V5.07). They share this layout; the telnet story below is identical across them.

## Package format

Unlike the Anyka dumps (raw SPI-NOR reads carved with `dd` at offsets), a XiongMai OTA
`.bin` is a **plain ZIP**. Each member is a partition image prefixed with a **64-byte
U-Boot legacy header** (magic `27 05 19 56`). `InstallDesc` is the JSON manifest that tells
the bootloader-side upgrader which member burns to which partition.

| Member | Real FS (after 64-byte header) | Mount |
|---|---|---|
| `u-boot.bin.img` | U-Boot | mtd0 `boot` |
| `u-boot.env.img` | U-Boot env (CRC + `key=val`) | — |
| `uImage.img` | kernel | mtd1 `kernel` |
| `romfs-x.cramfs.img` | **cramfs** | mtd2 `romfs` → `/` (ro) |
| `user-x.cramfs.img` | **squashfs** (`hsqs`, despite the name) | mtd3 `user` → `/usr` (ro) |
| `custom-x.cramfs.img` | **cramfs** | mtd4 `custom` → `/mnt/custom` (ro) |

`mtdparts` (from the u-boot env):
`256K(boot),1536K(kernel),1280K(romfs),4544K(user),256K(custom),320K(mtd)`.
mtd5 `mtd` (320K) is **JFFS2, the only writable partition**, mounted at `/mnt/mtd`.

### Extracting

```bash
F="Firmwares X6E-WEQ/General_IPC_XM535D1_X6E-WEQ_WIFIATBM6012BX_TB.at6012bx.Nat.dss.OnvifS_V5.04.R02.20250228_all.bin"
W=/tmp/xmfw; mkdir -p $W; unzip -o "$F" -d $W
# strip the 64-byte U-Boot header off each rootfs image
for f in romfs user custom; do dd if=$W/$f-x.cramfs.img of=$W/$f.fs bs=64 skip=1; done
fsck.cramfs --extract=$W/root   $W/romfs.fs     # rootfs
unsquashfs  -d $W/usr           $W/user.fs      # /usr (squashfs)
fsck.cramfs --extract=$W/custom $W/custom.fs    # /mnt/custom
strings $W/u-boot.env.img | grep =              # bootargs / tftp aliases
```

(`fsck.cramfs` prints "file extends past end of filesystem" because of the trailing pad;
it still extracts fully. `binwalk` is not needed.)

## Confirmed on hardware (UART, 2026-10-06)

Wired FT232R (host `/dev/ttyUSB0`, needs `dialout`/sudo) to the board header, GND tied,
115200 8N1. Console output is real; the earlier static reading of `inittab` was misleading:

```
U-Boot 2014.04 (Aug 18 2025 - 08:39:44)   CPU: XM535D1   DRAM: 64 MiB
MMC: arasan: 0   Net: PHY id1 937c id2 4024 dwmac.10010000
Press Ctrl+C to stop autoboot
SF: 1572864 bytes @ 0x40000 Read: OK
## Booting kernel from Legacy Image at 80007fc0 ...  Image Name: Linux-3.10.103+
   Data Size: 1465976 Bytes   Load/Entry: 80008000   XIP Kernel Image ... OK
```

Env (from `u-boot.env.img`). The **live chip env differs**: `mem=29M` not `35M`,
`ethact=dwmac.10010000`, `ethaddr=00:12:34:70:df:d2`, plus vendor vars `appCloudExAbility`,
`appNetIP`, `appProducerID=334`. `bootcmd`/`bootdelay`/tftp aliases match:

```text
bootdelay=1
bootcmd=sf probe 0;sf read 80007fc0 40000 180000;bootm 80007fc0
bootargs=mem=35M console=ttyAMA0,115200 root=/dev/mtdblock2 rootfstype=cramfs \
         mtdparts=xm_sfc:256K(boot),1536K(kernel),1280K(romfs),4544K(user),256K(custom),320K(mtd)
```

**The kernel console is fine — it was never dead.** A full boot prints, on the same pads at
the same 115200:

```text
Serial: AMBA PL011 UART driver
uart:0: ttyAMA0 at MMIO 0x10030000 (irq = 32) is a PL011 rev1
console [ttyAMA0] enabled
uart:1: ttyAMA1 at MMIO 0x10040000 (irq = 33) is a PL011 rev1
uart:2: ttyAMA2 at MMIO 0x10050000 (irq = 34) is a PL011 rev1
```

Kernel `3.10.103+` `#84 Wed Aug 6 09:41:19 CST 2025`, gcc 4.9.2 (Buildroot 2014.08), ARMv7
`410fc051` (Cortex-A5), machine `xm535`, single CPU. SPI-NOR is `XM25QH64D` 8 MiB
(`jedecid 204017`). Three PL011 UARTs exist; the console is the one at `0x10030000`.

The earlier "console dies at `booting the kernel ...`" reading was a **truncated paste**, and
the zero-byte baud sweep that appeared to confirm it was invalid: a second reader (an open
`picocom`) held the port, and two readers on one tty deliver each byte to only one of them.
Check `sudo fuser /dev/ttyUSB0` before trusting any capture.

**Do not use `init=/bin/sh` on this board.** Appending it to `bootargs` panics the kernel:

```text
VFS: Cannot open root device "(null)" or unknown-block(0,0): error -6
Please append a correct "root=" boot option; here are the available partitions:
1f00            8192 mtdblock0  (driver?)
Kernel panic - not syncing: VFS: Unable to mount root fs on unknown-block(0,0)
```

Only `mtdblock0` (the whole 8 MiB flash) is present at that point — with `init=` set the
kernel does not wait for the `mtdparts=`-split children to appear, so `root=/dev/mtdblock2`
never resolves. `setenv` is RAM-only, so a power cycle restores the real `bootargs`.

**Why the console is silent on a normal boot is still unexplained — do not trust any
explanation below until it is re-tested.** What is established:

- `bootargs` is clean: `echo "[${bootargs}]"` shows single spaces, no stray quotes or tabs.
- After a silent boot the board is fully up on the LAN (`ping`, 34567/554/80 open), so kernel
  and userspace run; only the console path is missing.
- An interactive `run bootcmd` with a hand-typed clean `bootargs` prints nothing after the
  decompressor. An interactive `run bootcmd` whose `bootargs` had been rewritten by `setenv`
  printed the entire kernel log — and in those runs `mtdparts=` did not take effect (only
  `mtdblock0` existed), producing the root panic above. Both of those runs had the value
  wrapped in literal `"…"` in the `setenv` command; **type `bootargs` without quotes.**
- Three PL011 UARTs exist: `0x10030000` (ttyAMA0, the kernel console), `0x10040000` (ttyAMA1),
  `0x10050000` (ttyAMA2). U-Boot's `serial` and the kernel's `ttyAMA0` are not proven to be the
  same physical pad. `getty` sits on `ttyS000`/`ttyAMA0`, so if the board header is wired to a
  different UART, neither kernel messages nor the login prompt can ever appear on it — which
  matches every observation except the runs that did print the kernel log.

Untried discriminator: `console=ttyAMA1,115200` / `console=ttyAMA2,115200` in place of
`ttyAMA0` — if the kernel log then appears on the existing pads, the header is on uart:1/2.
**Tested: `console=ttyAMA1,115200` is also silent, so that hypothesis is dead.**

### Console: hypotheses tested and ruled out

`inittab` and `rcS` are byte-identical across V5.00 / V5.04 / V5.07, and nothing in userspace
sets `printk`/`dmesg -n`/`loglevel`. RX is proven working — the U-Boot `Password:` prompt
accepts typed input — so the pads are not one-way. Every successful boot leaves the board
fully alive on the LAN, so kernel and userspace both run.

Each of these was tried on the board and **all stayed silent** after
`Uncompressing Linux... done, booting the kernel.`:

| Tried | Result |
|---|---|
| `console=ttyAMA1,115200` (uart:1 at `0x10040000`) | silent |
| `ignore_loglevel` appended | silent — so it is not a loglevel filter |
| `mem=29M` removed entirely | silent — so `mem=` is not implicated |
| interactive `run bootcmd` vs cold autoboot | both silent |
| `bootargs` retyped literally (no quotes) | silent |

The **only** runs that ever printed the kernel log were the two whose `bootargs` had been
rewritten as `setenv bootargs "…"`, i.e. with literal `"` characters at both ends — and both
of those panicked on the root mount. No mechanism is known for why a stray quote would enable
console output. Treat this as unresolved; do not re-run the tests above without a new idea.

Practical consequence: **do not plan around a runtime serial console on this unit.** U-Boot's
console is reliable, and everything that matters (flash backup, reflashing) is reachable from
there — see **Backup and restore**.

### U-Boot console gate

`Ctrl+C` does **not** drop straight to `U-Boot>`; it prints `Password: ` and rejects wrong
input with `Sorry, please try again!` (no lockout, unlimited attempts).

```
U-Boot console password (this board): #Ux6@9V&4_Rz
```

> **This repo is public** (`kkrzysztofik/Anyka_ak3918_hacking_journey`). The line above is a
> device credential and becomes public on the next push. If it is a per-unit password rather
> than a firmware default, keep it out of tracked files.

The gate is driven by env hooks `bootstopkey` / `bootstopkey2` (and a separate `bootdelaykey`
/ `bootdelaykey2` boot menu, which also carries `## Switch baudrate to %d bps and press
ENTER ...`). **None of these keys exist in the shipped `u-boot.env.img` *or* in the live chip
env**, so the accepted value is compiled into `u-boot.bin`.

### Backup and restore

**There is no way to pull data out of this U-Boot.** Verified on the board:

- No `tftpput` in the binary.
- `tftp` is an alias for **`tftpboot`** — usage `tftpboot [loadAddress]
  [[hostIPaddr:]bootfilename]`, download only. There is no mainline `tftp` copy command, so the
  `saveAddress`/`length` upload form does not exist. Do not retry it.
- The link never comes up in U-Boot: `Waiting for PHY auto negotiation to complete...
  TIMEOUT!` / `No link.` with `ethact` resolving to `<NULL>`, even though Linux drives the same
  PHY (`937c4024`) fine. So networking is unavailable in the bootloader anyway.

`loadb`/`loadx`/`loady` are inbound only, and `md` paste is impractical for 8 MiB. **Over the
network, the only backup path is a running Linux shell**: `dd` each `/dev/mtd*`, then push it out
with the busybox `tftp` client (`tftp -p`, confirm with `tftp --help` on the device). The SD card
route below avoids both the network and Linux.

### SD card is the way out that needs no network and no Linux

U-Boot has **no WiFi and no USB host** — only `dwmac`/`xmmac` + MDIO — so the bootloader's only
network is the on-board Ethernet, and that link currently fails to come up. But it does have the
`arasan` MMC driver plus `fatload`/`fatls` and `mmc write`, and `sf read/write/erase/update`.
So the flash can be dumped straight to a sacrificial SD card:

```text
=> mmc list
=> mmc rescan
=> mmc info
=> sf probe 0
=> sf read 0x81000000 0x0 0x800000
=> mmc dev 0
=> mmc write 0x81000000 0x0 0x4000
```

`0x4000` = 16384 512-byte blocks = 8 MiB, written to LBA 0, which **destroys the card's
partition table** — use a card you do not care about. Read it back on the host with
`dd if=/dev/sdX of=xm535-nor8m.bin bs=512 count=16384`.

The same route installs firmware without Ethernet: put the images on a FAT32 card and
`fatload mmc 0:1 0x81000000 <file>`, then `sf update 0x81000000 <offset> <len>` (`sf update`
erases and writes in one step).

What is possible is *verifying* that the stock OTA ZIP in `Firmwares X6E-WEQ/` is a faithful
copy of the chip. Partition offsets from `mtdparts`, and CRC32 of each V5.07 member (all
exactly fill their partition except `u-boot`, `0x2F800` bytes):

| Partition | Flash offset | Size | V5.07 member | crc32 |
|---|---|---|---|---|
| `boot` | `0x000000` | 256K | `u-boot.bin.img` (194560 B) | `f3a94ff5` |
| `kernel` | `0x040000` | 1536K | `uImage.img` | `955648fb` |
| `romfs` | `0x1C0000` | 1280K | `romfs-x.cramfs.img` | `73a116a4` |
| `user` | `0x300000` | 4544K | `user-x.cramfs.img` | `a814d90d` |
| `custom` | `0x770000` | 256K | `custom-x.cramfs.img` | `b5193d4f` |
| `mtd` | `0x7B0000` | 320K | *jffs2, unit-unique — not in any ZIP* | — |

Verify read-only with `sf read 0x81000000 <offset> <len>` then `crc32 0x81000000 <len>` on a
separate line (this U-Boot has no `#` comments and mishandles `;` chaining). Measured on the
board, 2026-10-06:

| Partition | Board crc32 | Matches any local package? |
|---|---|---|
| `boot` | `a65ab5df` | no (env lives in this region, so a mismatch here is expected) |
| `kernel` | `fff62b42` | no |
| `romfs` | `c3fadac5` | no |
| `user` | `eed7866d` | no |
| `custom` | `b9edc4b6` | no |
| `mtd` | `43730293` | n/a — unit-unique jffs2 |

**None of the six `Firmwares X6E-WEQ/` releases match the chip** (checked raw and 0xFF-padded
CRC32 for every member of every package). So those ZIPs are *not* a backup of this unit — the
board runs a release that is not in the repo. Until the exact release is identified and
downloaded, there is no stock image to fall back to, and a full-chip reflash (OpenIPC included)
is currently irreversible.

The only exfil route for the chip's real contents is a running Linux: `dd` each `/dev/mtd*`
and push it out with busybox `tftp -p` (the `tftp` applet is present). That makes a shell a
prerequisite for a backup, not a convenience.

The one thing with no backup is `/mnt/mtd` (jffs2, mtd5): MAC, IP, credentials, ability
config. It is not in any OTA package, and any full-chip reflash (e.g. an OpenIPC layout)
destroys it. Capture it from a Linux shell — see above.

### OpenIPC as an alternative

OpenIPC ships an **XM530 build (MVP stage)** but **no bootloader for XiongMai**, so the guided
install and the ready-made image are unavailable — it is hand-install from the stock U-Boot,
load address `0x81000000`, which is exactly what this U-Boot's tftp aliases already use.
`OpenIPC/defib` does **not** help here: `burn` speaks HiSilicon/Goke boot-ROM UART protocols
(Standard, V500, CV6xx) and `install` must first reach an OpenIPC U-Boot. It would buy a real
serial console plus dropbear/SSH at the cost of the whole XiongMai stack (`Sofia`, NetIP 34567,
their ONVIF, ISP/venc modules).

What the bundle actually contains (`openipc.xm530-nor-lite.tgz` from the `nightly` release,
5.0 MB, plus two `.md5sum` files):

| File | Size | Notes |
|---|---|---|
| `uImage.xm530` | 1,198,400 B | legacy header, `load = ep = 0x80008000`, name `Linux-3.10.103+-xm530`, uncompressed |
| `rootfs.squashfs.xm530` | 3,874,816 B | `hsqs` squashfs, xz |

Their `sizes.xm530-lite.json` states the 8 MB layout budget: kernel cap 2048 KiB (1171 used),
rootfs cap 5120 KiB (3704 used). **That is a 2 MB kernel + 5 MB rootfs layout, which is not the
stock layout** (`1536K(kernel),1280K(romfs),4544K(user)`). So a hand install must also rewrite
`mtdparts`, `bootcmd` and `bootargs` in the stock environment — the stock `bootcmd` reads the
kernel from `0x40000` for `0x180000` bytes, which does not match OpenIPC's geometry. Generate
the authoritative command list from the openipc.org XM530 form (NOR 8M, 8 MB / 5 MB rootfs
layout); do not invent offsets.

Two blockers before any of that is reachable:

1. **U-Boot has no network link** (`Waiting for PHY auto negotiation to complete... TIMEOUT! /
   No link.`), and every install path — OpenIPC or stock restore — downloads via U-Boot tftp.
   Nothing can be flashed until `=> ping <serverip>` succeeds.
2. **No backup exists** (see **Backup and restore**), and `saveenv` would overwrite the stock
   environment. The stock env is captured in this document, and the V5.07 partition images are
   on disk, so a degraded restore is possible — different firmware version, `/mnt/mtd` config
   gone — but nothing restores this unit exactly as it is.

Live board answers on the LAN: **TCP 34567** (NetIP), **554** (RTSP), **80** (stock XiongMai
UI, `title="Web Viewer"`).

## Boot chain

`linuxrc -> bin/busybox` → `/etc/inittab`:

```
::sysinit:/etc/init.d/rcS
::respawn:/sbin/getty -L ttyS000 115200 vt100 -n root -I "Auto login as root ..."
```

`/etc/init.d/rcS` mounts the partitions, then:

```
/usr/etc/loadmod                 # tar xf modules.tar.lzma; ./loadxm530 -i   (XM530/535 SoC)
netinit
[ -f /mnt/custom/extapp.sh ] && /mnt/custom/extapp.sh &     # <-- only script hook
dvrHelper /lib/modules /usr/bin/app.sh 127.0.0.1 9578 1 &   # watchdog → app.sh → Sofia
```

`app.sh` just runs `/usr/bin/Sofia` (the 4.9 MB monolith: NetIP, ONVIF, RTSP, web, ISP).
`netinit`, `dvrHelper` and `telnetctrl` are the **same 21.6 KB multicall binary** dispatched
on `argv[0]` (like busybox).

## "Telnet" on this firmware

**There is no telnet server binary.** busybox v1.33.1 here was built with only
`getty`, `login`, `tftp` — no `telnetd`, `telnet`, `nc`/`netcat`, `inetd`, or `dropbear`
anywhere in the image. What XiongMai calls "telnet" is a **proprietary debug console**
inside `Sofia`/`dvrHelper`:

- `dvrHelper`'s `telnetctrl` applet calls `LibXmDvr_Debug_open` +
  `LibXmDvr_Debug_setDefaultPasswd` → `"Listening at port %d for debug."`
- `Sofia` exposes the NetIP op `OPTelnetControl` / `OnTelnetControl` with sub-commands
  `TelnetDebugOpen`, `TelnetDebugOpenForever`, `TelnetDebugClose`, gated by the ability flag
  `Ability.TelnetDebugMode`. Opening it drops a token in `/var/NetIPTokenInfo`, touches
  `/var/netiptelnetdebug.txt`, and spawns the debug shell. `/var` is ramfs, so it does not
  survive a reboot.

`Ability.TelnetDebugMode` is **not present** in the shipped `custom/CustomConfig/Ability.custom`,
i.e. the feature is off by default and is meant to be turned on by an authenticated NetIP
command from the vendor tooling.

### Getting a root shell (owned hardware)

Ranked easiest → most invasive.

1. **UART auto-login — the working path, confirmed on hardware.** `inittab` runs
   `getty … -n root`, i.e. it auto-logins **root on the serial console**
   (`console=ttyAMA0,115200`, UART pads on the board; `ttyS000` and `ttyAMA0` are the same
   node — `/etc/init.d/dnode`, first line of `rcS`, mknods both as `c 204 64`). Let the board
   boot untouched, then press Enter on the console. Needs zero modification. If a password is
   ever prompted, the hash is `root:$1$RYIwEiRA$d5iRRVQ5ZeRTrJwGjRy.B0` in `/etc/passwd`
   (MD5-crypt; `/etc/shadow` is empty; not any of the usual XM defaults). For a *network*
   shell, use this once to set up 2 or 3.

2. **Trigger XM's own debug console over the network.** Send the NetIP `OPTelnetControl`
   → `TelnetDebugOpenForever` command to Sofia on **TCP 34567**, authenticated as the device
   admin. Community "dvrip"/Sofia clients implement this. It is the vendor-intended enable
   path and requires no flash write, but it is XM's raw debug shell, not RFC-854 telnet.

3. **U-Boot serial recovery.** `bootdelay=1` — interrupt U-Boot on the console and use its
   built-in tftp aliases (`serverip 192.168.1.107`, `ipaddr 192.168.1.10`):
   `dr`/`du`/`dc` = tftp+`flwrite` romfs/user/custom, `tk` = tftp-boot a kernel. This lets you
   flash a modified partition without the OTA path.

4. **Persistent real telnetd (permanent).** The only firmware-baked script hook is
   `/mnt/custom/extapp.sh`, and `/mnt/custom` is read-only cramfs, so persistence means
   reflashing:
   - cross-compile `dropbear` (or a busybox with `telnetd`) for XM530/535 — ARM EABI5,
     `/lib/ld-uClibc.so.0`;
   - repack `custom-x.cramfs.img` to add `extapp.sh` (which starts your telnetd) plus the
     binary, re-add the 64-byte U-Boot header (`mkimage -A arm -T firmware`), fix the CRC;
   - burn just mtd4 via U-Boot `dc`, or rebuild the ZIP with `InstallDesc` and OTA it.

   The writable `/mnt/mtd` (jffs2) is a fine place to *store* the binary, but nothing sourced
   at boot reads from it, so the launch hook still has to live in `custom` (or in `romfs`'s
   `rcS`).

## Differences from our Anyka stack (quick orientation)

| Area | Our AK3918 (`orig/`) | XM535 X6E-WEQ |
|---|---|---|
| Vendor / SoC | Anyka AKv200, MIPS-less ARMv5 | XiongMai XM530/535, ARMv7 uClibc |
| Packaging | raw NOR image, carve with `dd` | ZIP of U-Boot-headered partition images |
| rootfs | squashfs | cramfs (romfs) + squashfs (/usr) |
| init | our Rust supervisor / vendor `anyka_ipc` | busybox init → `Sofia` monolith |
| Main app | `onvif-rust` + vendor daemon | single `Sofia` (NetIP 34567, ONVIF, RTSP, web) |
| Serial | — | **root auto-login on UART** (`getty -n root`) |
| Telnet | busybox telnetd path | none shipped; proprietary `OPTelnetControl` debug shell |
| Writable cfg | `/data` / `anyka.toml` | `/mnt/mtd` jffs2, `/mnt/mtd/Config/*` |
