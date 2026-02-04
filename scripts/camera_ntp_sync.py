#!/usr/bin/env python3
"""
Connect to embedded camera via telnet and set correct date using NTP.
Usage: python3 camera_ntp_sync.py [host] [port]
Default: 192.168.2.198 24
"""

from __future__ import annotations

import re
import subprocess
import sys
import telnetlib
import time
import warnings

# telnetlib deprecated in 3.11, removed in 3.13; suppress at runtime
warnings.filterwarnings("ignore", category=DeprecationWarning, message=".*telnetlib.*")

DEFAULT_HOST = "192.168.2.198"
DEFAULT_PORT = 24
NTP_SERVER = "pool.ntp.org"
TIMEOUT = 10
READ_WAIT = 1.5


def read_until_prompt(tn: telnetlib.Telnet, timeout: int = 5) -> str:
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


def _try_login(tn: telnetlib.Telnet, initial: str) -> None:
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


def _run_ntp_commands(tn: telnetlib.Telnet) -> bool:
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


def _set_date_from_host(tn: telnetlib.Telnet) -> None:
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
        tn: telnetlib.Telnet = telnetlib.Telnet(host, port, timeout=TIMEOUT)
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
