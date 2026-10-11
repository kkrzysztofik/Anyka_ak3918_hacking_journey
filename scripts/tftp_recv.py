#!/usr/bin/env python3
"""Minimal TFTP server that only accepts uploads (WRQ), to pull a flash dump
off a camera's U-Boot. Stdlib only; needs root for UDP 69.

    sudo scripts/tftp_recv.py /tmp/xm_backup
    # then at U-Boot>: tftp <addr> <name> <addr> <len>

Handles blksize negotiation. RFC 1350 is otherwise trimmed to what U-Boot sends.
"""
import os
import socket
import struct
import sys


def parse_rq(pkt: bytes) -> tuple[str, str, dict[str, str]]:
    """Return (filename, mode, options) from a WRQ packet."""
    fields: list[bytes] = pkt[2:].split(b"\x00")
    opts: dict[str, str] = {}
    for i in range(2, len(fields) - 1, 2):
        opts[fields[i].decode(errors="replace")] = fields[i + 1].decode(errors="replace")
    return fields[0].decode(errors="replace"), fields[1].decode(errors="replace"), opts


def serve_one(ctrl: socket.socket, pkt: bytes, addr: tuple[str, int], outdir: str) -> None:
    name, mode, opts = parse_rq(pkt)
    blksize = int(opts.get("blksize", "512"))
    safe = os.path.basename(name) or "upload.bin"
    path = os.path.join(outdir, safe)
    print(f"WRQ {addr} file={safe} mode={mode} opts={opts}", flush=True)

    if opts:  # acknowledge the option set with OACK, then wait for the client ACK
        oack = b"\x00\x03" + b"".join(
            k.encode() + b"\x00" + v.encode() + b"\x00" for k, v in opts.items()
        )
        ctrl.sendto(oack, addr)
        ctrl.settimeout(10)
        try:
            ctrl.recvfrom(1024)
        except socket.timeout:
            print("no ACK for OACK, aborting", flush=True)
            return

    # Data transfer moves to its own port (TID), as RFC 1350 requires.
    data = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    data.bind(("", 0))
    data.settimeout(30)
    data.sendto(struct.pack("!HH", 4, 0), addr)  # ACK 0

    total, expect = 0, 1
    with open(path, "wb") as f:
        while True:
            try:
                pkt, peer = data.recvfrom(blksize + 4)
            except socket.timeout:
                print(f"timeout after {total} bytes", flush=True)
                break
            if peer != addr:
                continue
            op = struct.unpack("!H", pkt[:2])[0]
            if op == 5:
                print(f"client error {struct.unpack('!H', pkt[2:4])[0]}: "
                      f"{pkt[4:-1].decode(errors='replace')}", flush=True)
                break
            if op != 1:
                continue
            blk = struct.unpack("!H", pkt[2:4])[0]
            if blk != expect:  # retransmit of the previous block: just re-ACK it
                data.sendto(struct.pack("!HH", 4, blk - 1), addr)
                continue
            payload = pkt[4:]
            f.write(payload)
            total += len(payload)
            data.sendto(struct.pack("!HH", 4, blk), addr)
            if len(payload) < blksize:
                break
            expect = (expect + 1) & 0xFFFF

    data.close()
    print(f"wrote {total} bytes -> {path}", flush=True)


def main() -> None:
    if len(sys.argv) < 2:
        sys.exit("usage: tftp_recv.py <output-dir> [port]")
    outdir = sys.argv[1]
    port = int(sys.argv[2]) if len(sys.argv) > 2 else 69
    os.makedirs(outdir, exist_ok=True)

    ctrl = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    ctrl.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    ctrl.bind(("", port))
    print(f"waiting for WRQ on :{port}, saving into {outdir}", flush=True)
    while True:
        pkt, addr = ctrl.recvfrom(1024)
        if len(pkt) > 4 and struct.unpack("!H", pkt[:2])[0] == 1:
            serve_one(ctrl, pkt, addr, outdir)


if __name__ == "__main__":
    main()
