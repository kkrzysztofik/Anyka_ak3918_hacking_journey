#!/usr/bin/env python3
"""Minimal read-only TFTP server (RRQ) for handing files to a camera's U-Boot.

    sudo scripts/tftp_serve.py /tmp/opc
    # at U-Boot>:
    #   setenv serverip <host-ip>
    #   tftp 0x81000000 uImage.xm530

Deliberately refuses TFTP options (no OACK), so every transfer is plain
512-byte blocks -- which U-Boot falls back to happily. Stdlib only; UDP 69
needs root.
"""
import os
import socket
import struct
import sys

OP_RRQ, OP_DATA, OP_ACK, OP_ERROR = 1, 3, 4, 5
BLKSIZE = 512


def parse_rrq(pkt: bytes) -> str:
    """Filename from an RRQ packet. Options are ignored on purpose."""
    fields: list[bytes] = pkt[2:].split(b"\x00")
    return fields[0].decode(errors="replace")


def serve_rrq(ctrl: socket.socket, pkt: bytes, addr: tuple[str, int], root: str) -> None:
    name = parse_rrq(pkt)
    root_real = os.path.realpath(root)
    path = os.path.realpath(os.path.join(root_real, os.path.basename(name)))
    if not path.startswith(root_real + os.sep):
        ctrl.sendto(struct.pack("!HH", OP_ERROR, 2) + b"access violation\x00", addr)
        print(f"RRQ {addr} {name!r} -> outside {root_real}", flush=True)
        return
    try:
        with open(path, "rb") as f:
            data = f.read()
    except OSError as exc:
        ctrl.sendto(struct.pack("!HH", OP_ERROR, 1) + b"file not found\x00", addr)
        print(f"RRQ {addr} {name!r} -> {exc}", flush=True)
        return
    # TFTP ends on a *short* block, so a size that is an exact multiple of the
    # block size needs a final empty block or the client waits forever.
    blocks = len(data) // BLKSIZE + 1
    print(f"RRQ {addr} {name} {len(data)} B / {blocks} blocks", flush=True)

    # Transfer moves to its own port, as RFC 1350 requires.
    port = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    port.bind(("", 0))
    port.settimeout(2)

    for blk in range(1, blocks + 1):
        payload = data[(blk - 1) * BLKSIZE : blk * BLKSIZE]
        pkt_out = struct.pack("!HH", OP_DATA, blk & 0xFFFF) + payload
        acked = False
        for _ in range(10):
            port.sendto(pkt_out, addr)
            try:
                resp, peer = port.recvfrom(128)
            except TimeoutError:
                continue
            if peer != addr:
                continue
            op = struct.unpack("!H", resp[:2])[0]
            if op == OP_ACK and struct.unpack("!H", resp[2:4])[0] == (blk & 0xFFFF):
                acked = True
                break
            if op == OP_ERROR:
                print(f"client error on block {blk}", flush=True)
                port.close()
                return
        if not acked:
            print(f"no ACK for block {blk}, giving up", flush=True)
            port.close()
            return

    port.close()
    print(f"sent {len(data)} bytes -> {path}", flush=True)


def main() -> None:
    if len(sys.argv) < 2:
        sys.exit("usage: tftp_serve.py <directory> [port]")
    root = sys.argv[1]
    port = 69
    if len(sys.argv) > 2:
        if not sys.argv[2].isdigit():
            sys.exit(f"bad port: {sys.argv[2]}")
        port = int(sys.argv[2])
    if not os.path.isdir(root):
        sys.exit(f"not a directory: {root}")

    ctrl = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    ctrl.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    ctrl.bind(("", port))
    print(f"serving {root} on :{port}", flush=True)
    while True:
        pkt, addr = ctrl.recvfrom(1024)
        if len(pkt) > 2 and struct.unpack("!H", pkt[:2])[0] == OP_RRQ:
            serve_rrq(ctrl, pkt, addr, root)


if __name__ == "__main__":
    main()
