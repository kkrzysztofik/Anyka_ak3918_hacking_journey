# XM535 (X6E-WEQ) NOR flash dump — read-only regions

8 MiB SPI-NOR dump of a XiongMai `XM535D1_X6E-WEQ` board (`XM25QH64D`), captured
2026-10-11 through U-Boot's serial console (`sf read` + `md.b`), decoded and CRC-verified
against the `crc32` values U-Boot computed on the chip itself. See
[`../xm535-x6e-weq-firmware.md`](../xm535-x6e-weq-firmware.md) for the full analysis.

Each file is the **exact bytes the chip holds at that offset** — headerless, exactly as
`flwrite` left them. `kernel` keeps its own uImage header because the stock `bootcmd` hands
that address to `bootm`; `boot` starts with the ARM reset branch and `romfs`/`custom` with
cramfs magic.

| File | Offset | Length | Bytes | crc32 |
|---|---|---|---|---|
| `boot-region.bin` | `0x000000` | `0x2F800` | 194560 | `a65ab5df` |
| `kernel-region.bin` | `0x040000` | `0x180000` | 1572864 | `fff62b42` |
| `romfs-region.bin` | `0x1C0000` | `0x140000` | 1310720 | `c3fadac5` |
| `user-region.bin` | `0x300000` | `0x470000` | 4653056 | `eed7866d` |
| `custom-region.bin` | `0x770000` | `0x40000` | 262144 | `b9edc4b6` |

Not included: **`mtd`** (`0x7B0000`, `0x50000`) — jffs2, mounted read-write, so it changes
between boots and has no stable CRC. It also holds this unit's WiFi SSID/key material, an
`administrator` credential, `pap-secrets` and the device serial, so it is deliberately kept
out of this public repository. Capture it separately if you need it:

```
sf read 0x81000000 0x7B0000 0x50000
md.b 0x81000000 0x50000
```

Also not covered: `0x2F800`..`0x40000`, the U-Boot environment area (the env proper is at
`0x30000`). Its contents are recorded as `printenv` output in the reference doc.

## Restoring

From U-Boot, per region — download, erase, write, then read back to a *different* address so
the CRC tests what came off the chip rather than the buffer just written:

```
setenv ipaddr <camera-ip>
setenv serverip <host-ip>
tftp 0x81000000 custom-region.bin
sf probe 0
sf erase 0x770000 0x40000
sf write 0x81000000 0x770000 0x40000
sf read 0x82000000 0x770000 0x40000
crc32 0x82000000 0x40000
```

`sf erase` needs 4 KiB-aligned offsets *and* lengths, so for `boot` erase `0x0 0x30000`
(not `0x2F800`) and write the 194560-byte file. A TFTP server that sends a terminating
short block is required — 262144 is an exact multiple of 512, so without it U-Boot ACKs
every block and then waits forever; `scripts/tftp_serve.py` handles this.

## Reproducing

`scripts/tftp_serve.py <dir>` to serve, `md.b` output captured with
`picocom --logfile`, then `scripts/xm_dump_decode.py <log> <out> --base <offset> --verify`
to rebuild and check each region against the CRC table above.
