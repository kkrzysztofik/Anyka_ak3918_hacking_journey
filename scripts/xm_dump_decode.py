#!/usr/bin/env python3
"""Rebuild a flash image from U-Boot `md.b` serial output, and verify it.

U-Boot's `md.b` prints one 16-byte line per address:

    81000000: 27 05 19 56 00 00 00 00 00 00 00 00 00 00 00 00    '...V............

Usage:
    scripts/xm_dump_decode.py serial.log out.bin          # decode only
    scripts/xm_dump_decode.py serial.log out.bin --verify # + per-region CRC32

The CRCs are the ones measured on the XM535 board (see
docs/reference/xm535-x6e-weq-firmware.md). A region that does not match was
corrupted in transit and can be re-dumped on its own.
"""
import re
import sys
import zlib

LINE = re.compile(r"^([0-9a-fA-F]{8}):((?: [0-9a-fA-F]{2})+)\s")

# name, flash offset, length, expected crc32, volatile
# `volatile` regions are mounted read-write by the running firmware, so their
# contents change between boots and the CRC can never match.
REGIONS: list[tuple[str, int, int, str, bool]] = [
    ("boot", 0x000000, 0x2F800, "a65ab5df", False),
    ("kernel", 0x040000, 0x180000, "fff62b42", False),
    ("romfs", 0x1C0000, 0x140000, "c3fadac5", False),
    ("user", 0x300000, 0x470000, "eed7866d", False),
    ("custom", 0x770000, 0x40000, "b9edc4b6", False),
    ("mtd", 0x7B0000, 0x50000, "43730293", True),
]


def decode(lines: list[str]) -> tuple[bytearray, list[str]]:
    """Rebuild the image. Returns the bytes and a list of problem notes."""
    chunks: dict[int, bytes] = {}
    notes: list[str] = []
    for raw in lines:
        m = LINE.match(raw.strip("\r\n"))
        if not m:
            continue
        addr = int(m.group(1), 16)
        data = bytes.fromhex(m.group(2).replace(" ", ""))
        if addr in chunks and chunks[addr] != data:
            notes.append(f"conflicting data at {addr:#010x}")
        chunks[addr] = data

    if not chunks:
        return bytearray(), ["no md.b lines found"]

    # md.b prints absolute addresses; rebase to the first one dumped
    base = min(chunks)
    end = max(a + len(d) for a, d in chunks.items())
    img = bytearray(b"\xff" * (end - base))
    seen = bytearray(end - base)
    for addr, data in chunks.items():
        off = addr - base
        img[off : off + len(data)] = data
        seen[off : off + len(data)] = b"\x01" * len(data)

    # report holes so a dropped line is never silently filled with 0xff
    hole = None
    for i in range(end - base):
        if seen[i]:
            if hole is not None:
                notes.append(f"gap {base + hole:#010x}..{base + i - 1:#010x} ({i - hole} bytes)")
                hole = None
        elif hole is None:
            hole = i
    if hole is not None:
        notes.append(f"gap {base + hole:#010x}..{end - 1:#010x} ({end - base - hole} bytes)")
    return img, notes


def verify(img: bytearray, covered: tuple[int, int]) -> bool:
    """Check every region fully inside `covered`, the range this dump spans."""
    lo, hi = covered
    ok = True
    for name, off, length, want, volatile in REGIONS:
        if off + length <= lo or off >= hi:
            print(f"  {name:7} not in this dump")
            continue
        if off < lo or off + length > hi:
            print(f"  {name:7} PARTIAL - dump covers {lo:#x}..{hi:#x}")
            ok = False
            continue
        got = "%08x" % (zlib.crc32(img[off : off + length]) & 0xFFFFFFFF)
        if volatile:
            print(f"  {name:7} {off:#09x}..{off + length:#09x} crc={got} VOLATILE (live jffs2, was {want})")
            continue
        mark = "OK" if got == want else "MISMATCH"
        print(f"  {name:7} {off:#09x}..{off + length:#09x} crc={got} want={want} {mark}")
        ok &= got == want
    return ok


def parse_args(argv: list[str]) -> tuple[str, str, int, bool]:
    """Returns (log_path, out_path, base, verify).

    `--base <hex>` is the flash offset the lowest md.b address corresponds to.
    It lets a single region be re-dumped on its own: `sf read 0x81000000 0x300000
    0x470000` then `md.b 0x81000000 0x470000` and `--base 0x300000`.
    """
    positional: list[str] = []
    base, verify = 0, False
    i = 0
    while i < len(argv):
        arg = argv[i]
        if arg == "--verify":
            verify = True
        elif arg == "--base":
            i += 1
            if i >= len(argv):
                sys.exit("--base needs a value")
            base = int(argv[i], 16)
        elif arg.startswith("--base="):
            base = int(arg.split("=", 1)[1], 16)
        else:
            positional.append(arg)
        i += 1
    if len(positional) < 2:
        sys.exit(__doc__)
    return positional[0], positional[1], base, verify


def main() -> None:
    log_path, out_path, base, want_verify = parse_args(sys.argv[1:])

    try:
        with open(log_path, errors="replace") as f:
            lines = f.readlines()
    except OSError as exc:
        sys.exit(f"cannot read {log_path}: {exc}")

    img, notes = decode(lines)
    covered = (base, base + len(img))
    if base:
        img = bytearray(b"\xff" * base) + img

    try:
        with open(out_path, "wb") as f:
            f.write(img)
    except OSError as exc:
        sys.exit(f"cannot write {out_path}: {exc}")
    print(f"decoded {len(img)} bytes -> {out_path}")
    for note in notes[:20]:
        print(f"  ! {note}")
    if len(notes) > 20:
        print(f"  ! ... and {len(notes) - 20} more")

    if want_verify:
        print("region verification:")
        sys.exit(0 if verify(img, covered) and not notes else 1)


if __name__ == "__main__":
    main()
