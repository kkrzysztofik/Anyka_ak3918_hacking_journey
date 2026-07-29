#!/usr/bin/env python3
"""Run shell commands on the camera over telnet and print their output.

Usage:
    uv run --no-project python3 scripts/debugging/camera_shell.py 'cmd' ['cmd2' ...]
    uv run --no-project python3 scripts/debugging/camera_shell.py --host 1.2.3.4 'uptime'

Reuses the minimal Telnet client from camera_ntp_sync.py (Python 3.13 dropped telnetlib).
"""

from __future__ import annotations

import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from camera_ntp_sync import (  # noqa: E402
    DEFAULT_HOST,
    DEFAULT_PORT,
    Telnet,
    _try_login,
    read_until_prompt,
)


def main(argv: list[str]) -> int:
    host, port = DEFAULT_HOST, DEFAULT_PORT
    while argv and argv[0] in ("--host", "--port"):
        flag, value, argv = argv[0], argv[1], argv[2:]
        if flag == "--host":
            host = value
        else:
            port = int(value)
    if not argv:
        print(__doc__)
        return 2

    tn = Telnet(host, port)
    try:
        _try_login(tn, read_until_prompt(tn, timeout=5))
        read_until_prompt(tn, timeout=3)
        for cmd in argv:
            print(f"\n===== {cmd} =====", flush=True)
            tn.write(cmd.encode() + b"\n")
            time.sleep(0.3)
            print(read_until_prompt(tn, timeout=15), flush=True)
    finally:
        tn.close()
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
