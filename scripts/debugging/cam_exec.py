#!/usr/bin/env python3
"""Run shell commands on the Anyka camera over telnet and print their output.

The camera's telnet (port 24) is the only remote shell available. `telnet` is
interactive and `telnetlib` was removed in Python 3.13, so this speaks the few
bytes of the protocol we actually need: refuse every IAC negotiation, log in,
run commands, and read until a sentinel we printed ourselves.

Reuses the minimal Telnet client from camera_ntp_sync.py (the single telnet
implementation for this project).

Usage:
    cam_exec.py 'ps | grep vendor'
    cam_exec.py --timeout 30 'pidof onvif-rust'
    cam_exec.py 'uptime' 'free' 'df -h'        # multiple commands
    cam_exec.py --host 1.2.3.4 'cmd'

Single command: prints output, prints [exit=N] to stderr, and the process exit
code mirrors the remote command. Multiple commands: prints each under a
"===== cmd =====" header; exit code is 1 if any command failed.
"""

from __future__ import annotations

import argparse
import sys
import time
import uuid
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from camera_ntp_sync import Telnet  # noqa: E402


def read_until(tn: Telnet, needle: bytes, deadline: float) -> bytes:
    """Read cooked bytes until `needle` appears at least once, or the deadline passes."""
    buf = bytearray()
    while time.time() < deadline:
        try:
            chunk = tn.read_eager()
        except EOFError:
            break
        if chunk:
            buf += chunk
            if needle in buf:
                break
        else:
            time.sleep(0.05)
    return bytes(buf)


def run_one(tn: Telnet, cmd: str, deadline: float) -> tuple[str, str]:
    """Run one command, returning (output, exit_status).

    Keeps the sentinel value out of the echoed command line by defining it via a
    shell variable, then locates the *last* occurrence when parsing so residual
    echo cannot steal the exit status.
    """
    sentinel = f"__END_{uuid.uuid4().hex[:12]}__"
    tn.write(f"S={sentinel}; {cmd} 2>&1; echo \"$S\"$?\n".encode())

    raw = read_until(tn, sentinel.encode(), deadline)
    text = raw.decode("utf-8", "replace")

    idx = text.rfind(sentinel)
    if idx >= 0:
        body = text[:idx]
        tail = text[idx + len(sentinel):]
        status = tail.split()[0].strip() if tail.split() else "?"
    else:
        body, status = text, "TIMEOUT"
    return body.strip(), status


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("commands", nargs="+")
    ap.add_argument("--host", default="192.168.2.198")
    ap.add_argument("--port", type=int, default=24)
    ap.add_argument("--user", default="root")
    ap.add_argument("--password", default="")
    ap.add_argument("--timeout", type=float, default=25.0)
    args = ap.parse_args()

    multi = len(args.commands) > 1
    deadline = time.time() + args.timeout

    try:
        tn = Telnet(args.host, args.port, timeout=args.timeout)
    except OSError as e:
        print(f"Connection failed: {e}", file=sys.stderr)
        return 1

    try:
        # This device drops straight to a root shell after IAC negotiation; there is
        # no login prompt. Handle both shapes so the script keeps working if the
        # firmware ever grows one.
        banner = read_until(tn, b"# ", min(deadline, time.time() + 8))
        if b"ogin:" in banner:
            tn.write(args.user.encode() + b"\n")
            prompt = read_until(tn, b"assword:", min(deadline, time.time() + 5))
            if b"assword:" in prompt:
                tn.write(args.password.encode() + b"\n")
            read_until(tn, b"# ", min(deadline, time.time() + 8))

        # Kill terminal echo: the telnet server echoes our command back, and a long
        # command wraps across lines, which makes the echo impossible to strip
        # reliably by line count.
        tn.write(b"stty -echo\n")
        read_until(tn, b"# ", min(deadline, time.time() + 5))

        failed = 0
        for cmd in args.commands:
            if multi:
                print(f"\n===== {cmd} =====", flush=True)
            body, status = run_one(tn, cmd, deadline)
            print(body)
            print(f"[exit={status}]", file=sys.stderr)
            if status != "0":
                failed += 1

        return 1 if failed else 0
    except (OSError, EOFError) as e:
        print(f"Error: {e}", file=sys.stderr)
        return 1
    finally:
        tn.close()


if __name__ == "__main__":
    sys.exit(main())
