#!/usr/bin/env python3
"""Capture /proc/meminfo, loadavg, and status from device via SSH for test fixtures."""

from __future__ import annotations

import os
import shlex
import subprocess
import sys
import tempfile
from pathlib import Path

HOST = os.getenv("ANYKA_DEVICE_HOST", "192.168.2.198")
PORT = int(os.getenv("ANYKA_DEVICE_SSH_PORT", "22"))
USER = os.getenv("ANYKA_DEVICE_USER", "root")
PASSWORD = os.getenv("ANYKA_DEVICE_PASSWORD", "")
TIMEOUT = int(os.getenv("ANYKA_DEVICE_TIMEOUT_SEC", "15"))

MARKER_BEGIN = "__ANYKA_BEGIN__"
MARKER_END = "__ANYKA_END__"
MARKER_BEGIN_CMD = 'echo __ANYKA_""BEGIN__'
MARKER_END_CMD = 'echo __ANYKA_""END__'


def _extract_marked(text: str) -> str:
    start = text.find(MARKER_BEGIN)
    end = text.find(MARKER_END)
    if start != -1 and end != -1 and end > start:
        return text[start + len(MARKER_BEGIN) : end].strip()
    return text.strip()


def _ssh_base_command() -> list[str]:
    return [
        "ssh",
        "-o",
        "StrictHostKeyChecking=no",
        "-o",
        "UserKnownHostsFile=/dev/null",
        "-o",
        "PreferredAuthentications=password",
        "-o",
        "PubkeyAuthentication=no",
        "-o",
        "NumberOfPasswordPrompts=1",
        "-o",
        f"ConnectTimeout={TIMEOUT}",
        "-p",
        str(PORT),
        f"{USER}@{HOST}",
    ]


def run_cmd(command: str) -> str:
    env = os.environ.copy()
    askpass_path: Path | None = None
    try:
        if PASSWORD:
            askpass_file = tempfile.NamedTemporaryFile(
                mode="w",
                encoding="utf-8",
                prefix="anyka_askpass_",
                suffix=".sh",
                delete=False,
            )
            askpass_path = Path(askpass_file.name)
            askpass_file.write(f"#!/bin/sh\nprintf %s {shlex.quote(PASSWORD)}\n")
            askpass_file.close()
            askpass_path.chmod(0o700)
            env["SSH_ASKPASS_REQUIRE"] = "force"
            env["SSH_ASKPASS"] = str(askpass_path)
            env["DISPLAY"] = "anyka-fixtures"

        proc = subprocess.run(
            _ssh_base_command() + [command],
            env=env,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            timeout=TIMEOUT + 15,
            check=False,
        )
        combined = (proc.stdout or "") + (proc.stderr or "")
        if proc.returncode != 0:
            raise RuntimeError(
                f"ssh failed ({proc.returncode}) for {USER}@{HOST}:{PORT}: {combined.strip()}"
            )
        return _extract_marked(combined)
    finally:
        if askpass_path is not None:
            try:
                askpass_path.unlink()
            except OSError:
                pass


def main() -> int:
    out_dir = Path(sys.argv[1]) if len(sys.argv) > 1 else Path("tests/fixtures")
    out_dir.mkdir(parents=True, exist_ok=True)

    try:
        meminfo_cmd = f"{MARKER_BEGIN_CMD} && cat /proc/meminfo && {MARKER_END_CMD}"
        meminfo = run_cmd(meminfo_cmd)
        (out_dir / "meminfo.txt").write_text(meminfo, encoding="utf-8")

        loadavg_cmd = f"{MARKER_BEGIN_CMD} && cat /proc/loadavg && {MARKER_END_CMD}"
        loadavg = run_cmd(loadavg_cmd)
        (out_dir / "loadavg.txt").write_text(loadavg, encoding="utf-8")

        pgrep_cmd = (
            f"{MARKER_BEGIN_CMD} && "
            "(pgrep -f onvif-rust 2>/dev/null || pidof onvif-rust 2>/dev/null || echo 1) "
            f"&& {MARKER_END_CMD}"
        )
        pgrep_out = run_cmd(pgrep_cmd)
        pid = "1"
        for token in pgrep_out.split():
            if token.isdigit():
                pid = token
                break

        status_cmd = (
            f"{MARKER_BEGIN_CMD} && cat /proc/{pid}/status 2>/dev/null && {MARKER_END_CMD}"
        )
        status = run_cmd(status_cmd)
        (out_dir / "proc_status.txt").write_text(status, encoding="utf-8")
        (out_dir / "pgrep.txt").write_text(pgrep_out, encoding="utf-8")
    except Exception as exc:  # noqa: BLE001 - script-level error path
        print(f"Capture failed: {exc}", file=sys.stderr)
        return 1

    print(f"Wrote {out_dir}/meminfo.txt, loadavg.txt, proc_status.txt, pgrep.txt")
    return 0


if __name__ == "__main__":
    sys.exit(main())
