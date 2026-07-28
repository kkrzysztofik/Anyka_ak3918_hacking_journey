#!/usr/bin/env python3
"""
Connect to embedded camera via telnet and set correct date using NTP.
Usage: python3 camera_ntp_sync.py [host] [port]
Default: 192.168.2.198 24

Uses a minimal stdlib Telnet client: Python 3.13 removed telnetlib.
"""

from __future__ import annotations

import re
import select
import socket
import subprocess
import sys
import time

DEFAULT_HOST = "192.168.2.198"
DEFAULT_PORT = 24
NTP_SERVER = "pool.ntp.org"
TIMEOUT = 10
READ_WAIT = 1.5

# Telnet protocol (RFC 854) — enough to refuse option negotiation cleanly.
_IAC = 255
_DONT = 254
_DO = 253
_WONT = 252
_WILL = 251
_SB = 250
_SE = 240


class Telnet:
    """Minimal blocking Telnet client (stdlib only; replaces removed telnetlib)."""

    def __init__(self, host: str, port: int, timeout: float = TIMEOUT) -> None:
        self._sock = socket.create_connection((host, port), timeout=timeout)
        self._sock.setblocking(False)
        self._raw = bytearray()
        self._cooked = bytearray()
        self._eof = False

    def write(self, data: bytes) -> None:
        """Send data, doubling IAC bytes per Telnet rules."""
        payload = data.replace(bytes([_IAC]), bytes([_IAC, _IAC]))
        view = memoryview(payload)
        while view:
            _, writable, _ = select.select([], [self._sock], [], TIMEOUT)
            if not writable:
                raise TimeoutError("telnet write timed out")
            sent = self._sock.send(view)
            if sent == 0:
                raise OSError("telnet connection closed during write")
            view = view[sent:]

    def read_eager(self) -> bytes:
        """Return available cooked data without blocking; raise EOFError if closed."""
        self._fill()
        if self._cooked:
            data = bytes(self._cooked)
            self._cooked.clear()
            return data
        if self._eof:
            raise EOFError("telnet connection closed")
        return b""

    def read_very_eager(self) -> bytes:
        """Drain all currently available cooked data without blocking."""
        while True:
            before = len(self._cooked) + len(self._raw)
            self._fill()
            after = len(self._cooked) + len(self._raw)
            if after == before:
                break
        if self._cooked:
            data = bytes(self._cooked)
            self._cooked.clear()
            return data
        if self._eof:
            raise EOFError("telnet connection closed")
        return b""

    def close(self) -> None:
        """Close the underlying socket."""
        try:
            self._sock.close()
        except OSError:
            pass

    def _fill(self) -> None:
        """Pull any pending socket bytes and cook Telnet command sequences."""
        if not self._eof:
            ready, _, _ = select.select([self._sock], [], [], 0.0)
            if ready:
                try:
                    chunk = self._sock.recv(4096)
                except BlockingIOError:
                    chunk = b""
                if chunk:
                    self._raw.extend(chunk)
                else:
                    self._eof = True
        self._process_raw()

    def _process_raw(self) -> None:
        """Strip Telnet commands; refuse DO/WILL options."""
        i = 0
        out = bytearray()
        replies = bytearray()
        raw = self._raw
        while i < len(raw):
            byte = raw[i]
            if byte != _IAC:
                out.append(byte)
                i += 1
                continue
            if i + 1 >= len(raw):
                break
            cmd = raw[i + 1]
            if cmd == _IAC:
                out.append(_IAC)
                i += 2
                continue
            if cmd in (_DO, _DONT, _WILL, _WONT):
                if i + 2 >= len(raw):
                    break
                opt = raw[i + 2]
                if cmd == _DO:
                    replies.extend((_IAC, _WONT, opt))
                elif cmd == _WILL:
                    replies.extend((_IAC, _DONT, opt))
                i += 3
                continue
            if cmd == _SB:
                j = i + 2
                while j + 1 < len(raw):
                    if raw[j] == _IAC and raw[j + 1] == _SE:
                        j += 2
                        break
                    j += 1
                else:
                    break
                i = j
                continue
            i += 2
        del raw[:i]
        if out:
            self._cooked.extend(out)
        if replies:
            self._sock.setblocking(True)
            try:
                self._sock.sendall(replies)
            finally:
                self._sock.setblocking(False)


def read_until_prompt(tn: Telnet, timeout: int = 5) -> str:
    """Read until we see something that looks like a prompt or login."""
    data: bytes = b""
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        try:
            chunk: bytes = tn.read_eager()
            if chunk:
                data += chunk
                text: str = chunk.decode("utf-8", errors="replace")
                # Prompt: # or $ at EOL, or "login:", "Password:"
                if (
                    re.search(r"[\#\$]\s*$", text)
                    or "login:" in text.lower()
                    or "password:" in text.lower()
                ):
                    break
            else:
                time.sleep(0.1)
        except (EOFError, OSError):
            break
    return data.decode("utf-8", errors="replace")


def _try_login(tn: Telnet, initial: str) -> None:
    """If device showed login prompt, send root / empty password."""
    if "login:" not in initial.lower():
        return
    tn.write(b"root\n")
    time.sleep(0.5)
    login_out: str = tn.read_very_eager().decode("utf-8", errors="replace")
    print(login_out)
    if "password:" in login_out.lower():
        tn.write(b"\n")
        time.sleep(0.5)


def _run_ntp_commands(tn: Telnet) -> bool:
    """Run ntpd/rdate commands; return True if time was set (year not 1970)."""
    cmds_try: list[tuple[str, str, int]] = [
        (f"/usr/sbin/ntpd -q -g -p {NTP_SERVER}\n", "ntpd one-shot (pool.ntp.org)", 12),
        ("/usr/sbin/ntpd -q -g -p time.google.com\n", "ntpd (time.google.com)", 12),
        ("/usr/sbin/rdate -s time.nist.gov\n", "rdate time.nist.gov", 3),
    ]
    for cmd, desc, wait_s in cmds_try:
        print(f"\n>>> {desc}: {cmd.strip()}")
        tn.write(cmd.encode())
        time.sleep(wait_s)
        out: str = tn.read_very_eager().decode("utf-8", errors="replace")
        print(out)
        tn.write(b"date\n")
        time.sleep(1.5)
        date_out: str = tn.read_very_eager().decode("utf-8", errors="replace")
        print("Current date:", date_out)
        if "1970" not in date_out and re.search(r"20\d{2}", date_out):
            print("(Time set successfully via NTP.)")
            return True
    return False


def _set_date_from_host(tn: Telnet) -> None:
    """Set camera date from this host's time (fallback when NTP unreachable)."""
    result = subprocess.run(
        ["date", "-u", "+%a %b %e %H:%M:%S %Z %Y"],
        capture_output=True,
        text=True,
        timeout=2,
        check=False,
    )
    if result.returncode != 0 or not result.stdout or not result.stdout.strip():
        return
    date_str: str = result.stdout.strip()
    print(f"\n>>> Setting date from this host: {date_str}")
    tn.write(f'date -s "{date_str}"\n'.encode())
    time.sleep(1.5)
    print(tn.read_very_eager().decode("utf-8", errors="replace"))
    tn.write(b"date\n")
    time.sleep(1)
    print(tn.read_very_eager().decode("utf-8", errors="replace"))
    print("(Time set from host.)")


def main() -> int:
    """Connect via telnet, sync time with NTP (or set from host if NTP fails)."""
    host: str = sys.argv[1] if len(sys.argv) > 1 else DEFAULT_HOST
    port: int = int(sys.argv[2]) if len(sys.argv) > 2 else DEFAULT_PORT

    print(f"Connecting to {host}:{port}...")
    try:
        tn: Telnet = Telnet(host, port, timeout=TIMEOUT)
    except OSError as e:
        print(f"Connection failed: {e}")
        return 1

    try:
        time.sleep(READ_WAIT)
        initial: str = read_until_prompt(tn, timeout=5)
        print("--- Device output ---")
        print(initial)
        print("---")

        _try_login(tn, initial)
        time.sleep(READ_WAIT)
        read_until_prompt(tn, timeout=5)

        if not _run_ntp_commands(tn):
            try:
                _set_date_from_host(tn)
            except OSError as e:
                print(f"\nCould not set date from host: {e}")

    except (OSError, EOFError) as e:
        print(f"Error: {e}")
        return 1
    finally:
        try:
            tn.close()
        except OSError:
            pass

    print("\nDone.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
