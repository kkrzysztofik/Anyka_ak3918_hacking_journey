#!/usr/bin/env python3
"""Run a shell command on the Anyka camera over telnet and print its output.

The camera's telnet (port 24) is the only remote shell available. `telnet` is
interactive and `telnetlib` was removed in Python 3.13, so this speaks the few
bytes of the protocol we actually need: refuse every IAC negotiation, log in,
run one command, and read until a sentinel we printed ourselves.

Usage:
    cam_exec.py 'ps | grep vendor'
    cam_exec.py --timeout 30 'pidof onvif-rust'
"""

import argparse
import socket
import sys
import time
import uuid

IAC, DONT, WONT, WILL, DO = 255, 254, 252, 251, 253


def negotiate(sock: socket.socket, data: bytes) -> bytes:
    """Strip telnet IAC sequences, refusing every option, and return the text."""
    out = bytearray()
    i = 0
    while i < len(data):
        if data[i] != IAC:
            out.append(data[i])
            i += 1
            continue
        if i + 2 >= len(data):
            break
        cmd, opt = data[i + 1], data[i + 2]
        # Refuse everything: WILL->DONT, DO->WONT.
        if cmd == WILL:
            sock.sendall(bytes([IAC, DONT, opt]))
        elif cmd == DO:
            sock.sendall(bytes([IAC, WONT, opt]))
        i += 3
    return bytes(out)


def read_until(sock: socket.socket, needle: bytes, deadline: float, count: int = 1) -> bytes:
    """Read until `needle` has been seen `count` times, or the deadline passes.

    `count=2` is what the command path uses: the shell echoes our command line
    back before running it, so the sentinel always appears once in the echo
    before the real one.
    """
    buf = bytearray()
    while time.time() < deadline:
        sock.settimeout(max(0.2, deadline - time.time()))
        try:
            chunk = sock.recv(4096)
        except socket.timeout:
            continue
        if not chunk:
            break
        buf += negotiate(sock, chunk)
        if bytes(buf).count(needle) >= count:
            break
    return bytes(buf)


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("command")
    ap.add_argument("--host", default="192.168.2.198")
    ap.add_argument("--port", type=int, default=24)
    ap.add_argument("--user", default="root")
    ap.add_argument("--password", default="")
    ap.add_argument("--timeout", type=float, default=25.0)
    args = ap.parse_args()

    deadline = time.time() + args.timeout
    sock = socket.create_connection((args.host, args.port), timeout=args.timeout)

    # This device drops straight to a root shell after IAC negotiation; there is
    # no login prompt. Handle both shapes so the script keeps working if the
    # firmware ever grows one.
    banner = read_until(sock, b"# ", min(deadline, time.time() + 8))
    if b"ogin:" in banner:
        sock.sendall(args.user.encode() + b"\n")
        prompt = read_until(sock, b"assword:", min(deadline, time.time() + 5))
        if b"assword:" in prompt:
            sock.sendall(args.password.encode() + b"\n")
        read_until(sock, b"# ", min(deadline, time.time() + 8))

    # Kill terminal echo: the telnet server echoes our command back, and a long
    # command wraps across lines, which makes the echo impossible to strip
    # reliably by line count.
    sock.sendall(b"stty -echo\n")
    read_until(sock, b"# ", min(deadline, time.time() + 5))

    sentinel = f"__END_{uuid.uuid4().hex[:12]}__"
    # `2>&1` so stderr is not lost; the sentinel carries the exit status.
    sock.sendall(f"{args.command} 2>&1; echo {sentinel}$?\n".encode())

    raw = read_until(sock, sentinel.encode(), deadline)
    text = raw.decode("utf-8", "replace")

    if sentinel in text:
        body, _, tail = text.partition(sentinel)
        status = tail.split()[0].strip() if tail.split() else "?"
    else:
        body, status = text, "TIMEOUT"

    print(body.strip())
    print(f"[exit={status}]", file=sys.stderr)

    sock.close()
    return 0 if status == "0" else 1


if __name__ == "__main__":
    sys.exit(main())
