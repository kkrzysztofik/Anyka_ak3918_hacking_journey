#!/usr/bin/env python3
"""Capture /proc/meminfo, loadavg, and status from device via telnet for test fixtures."""
import telnetlib
import sys

HOST = "192.168.2.198"
PORT = 24
TIMEOUT = 15
MARKER_BEGIN = "__ANYKA_BEGIN__"
MARKER_END = "__ANYKA_END__"


def run_cmd(tn: telnetlib.Telnet, cmd: str) -> str:
    tn.write(cmd.encode("ascii") + b"\n")
    data = tn.read_until(MARKER_END.encode("ascii"), timeout=10).decode("ascii", errors="replace")
    start = data.find(MARKER_BEGIN)
    end = data.find(MARKER_END)
    if start != -1 and end != -1:
        return data[start + len(MARKER_BEGIN) : end].strip()
    return data.strip()


def main() -> int:
    out_dir = sys.argv[1] if len(sys.argv) > 1 else "tests/fixtures"
    try:
        tn = telnetlib.Telnet(HOST, PORT, TIMEOUT)
    except Exception as e:
        print(f"Connect failed: {e}", file=sys.stderr)
        return 1

    try:
        # skip login banner
        tn.read_until(b"$ ", timeout=5)
        tn.read_until(b"# ", timeout=2)

        # meminfo
        cmd = f"echo {MARKER_BEGIN} && cat /proc/meminfo && echo {MARKER_END}"
        meminfo = run_cmd(tn, cmd)
        with open(f"{out_dir}/meminfo.txt", "w") as f:
            f.write(meminfo)

        # loadavg
        cmd = f"echo {MARKER_BEGIN} && cat /proc/loadavg && echo {MARKER_END}"
        loadavg = run_cmd(tn, cmd)
        with open(f"{out_dir}/loadavg.txt", "w") as f:
            f.write(loadavg)

        # pgrep to get a pid (use 1 if nothing)
        cmd = f"echo {MARKER_BEGIN} && (pgrep -f onvif-rust || echo 1) && echo {MARKER_END}"
        pgrep_out = run_cmd(tn, cmd)
        pid = "1"
        for line in pgrep_out.splitlines():
            line = line.strip()
            if line.isdigit():
                pid = line
                break

        # status for that pid
        cmd = f"echo {MARKER_BEGIN} && cat /proc/{pid}/status 2>/dev/null && echo {MARKER_END}"
        status = run_cmd(tn, cmd)
        with open(f"{out_dir}/proc_status.txt", "w") as f:
            f.write(status)

        with open(f"{out_dir}/pgrep.txt", "w") as f:
            f.write(pgrep_out)

        print(f"Wrote {out_dir}/meminfo.txt, loadavg.txt, proc_status.txt, pgrep.txt")
    finally:
        tn.close()
    return 0


if __name__ == "__main__":
    sys.exit(main())
