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

IAC, DONT, WONT, WILL, DO, SB, SE = 255, 254, 252, 251, 253, 250, 240


def negotiate(sock: socket.socket, data: bytes) -> bytes:
    """Strip telnet IAC sequences, refusing every option, and return the text.

    Incomplete IAC sequences at the end of a recv chunk are discarded; a later
    chunk may drop a split sequence rather than buffering across reads.
    """
    out = bytearray()
    i = 0
    while i < len(data):
        if data[i] != IAC:
            out.append(data[i])
            i += 1
            continue
        if i + 1 >= len(data):
            # Incomplete IAC at end of chunk — discard the dangling byte.
            break
        cmd = data[i + 1]
        if cmd == IAC:
            # IAC IAC is one literal 0xFF.
            out.append(IAC)
            i += 2
            continue
        if cmd == SB:
            # Skip subnegotiation through the terminating IAC SE.
            j = i + 2
            while j + 1 < len(data):
                if data[j] == IAC and data[j + 1] == SE:
                    i = j + 2
                    break
                j += 1
            else:
                # Incomplete SB payload — discard remainder of chunk.
                break
            continue
        if i + 2 >= len(data):
            break
        opt = data[i + 2]
        # Refuse everything: WILL->DONT, DO->WONT.
        if cmd == WILL:
            sock.sendall(bytes([IAC, DONT, opt]))
        elif cmd == DO:
            sock.sendall(bytes([IAC, WONT, opt]))
        i += 3
    return bytes(out)


def read_until(sock: socket.socket, needle: bytes, deadline: float) -> bytes:
    """Read until `needle` appears at least once, or the deadline passes.

    Callers that need the final (post-echo) occurrence locate it themselves
    after this returns; this helper only waits for the first sighting.
    """
    buf = bytearray()
    while time.time() < deadline:
        sock.settimeout(max(0.2, deadline - time.time()))
        try:
            chunk = sock.recv(4096)
        except TimeoutError:
            continue
        if not chunk:
            break
        buf += negotiate(sock, chunk)
        if needle in buf:
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
    with socket.create_connection((args.host, args.port), timeout=args.timeout) as sock:
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

        # Keep the sentinel value out of the echoed command line by defining it
        # via a shell variable, then locate the *last* occurrence when parsing so
        # residual echo cannot steal the exit status.
        sentinel = f"__END_{uuid.uuid4().hex[:12]}__"
        sock.sendall(
            f"S={sentinel}; {args.command} 2>&1; echo \"$S\"$?\n".encode()
        )

        raw = read_until(sock, sentinel.encode(), deadline)
        text = raw.decode("utf-8", "replace")

        idx = text.rfind(sentinel)
        if idx >= 0:
            body = text[:idx]
            tail = text[idx + len(sentinel) :]
            status = tail.split()[0].strip() if tail.split() else "?"
        else:
            body, status = text, "TIMEOUT"

        print(body.strip())
        print(f"[exit={status}]", file=sys.stderr)

        return 0 if status == "0" else 1


if __name__ == "__main__":
    sys.exit(main())
