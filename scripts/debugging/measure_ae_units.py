#!/usr/bin/env python3
"""Measure the AK3918 AE raw-unit scales (Task 10, imaging-tab-completion plan).

Drives the camera's AE attribute through two isolated sweeps and records, per
step, the AE operating point (raw units) and the achieved frame luma (full Y
plane of one RTSP frame):

  A. exposure: gain pinned at 1x (a_gain_max=256), exp_time_max swept
     ascending — luma may only move through exposure, so luma vs exp_time
     yields µs-per-unit once fps is known.
  B. gain: exposure pinned low (exp_time_max=10), a_gain_max swept ascending
     — luma vs a_gain yields the dB mapping (256 = 1x expected, Q8).

The AE attribute as found on arrival is restored on every exit path.

Honesty rules (review 2026-09-24):
  * the RTSP connection uses CAMERA_USER/CAMERA_PASS, same as the HTTP side;
  * a frame that is not a complete 720p yuv420p buffer is rejected, never
    averaged (the Y plane is 2/3 of the frame — the old 1/3 split measured
    half of Y mixed with chroma);
  * if the AE ceiling write fails — including the endpoint being absent,
    which is the current state: /api/ae-debug was removed after the original
    measurement — the sweep stops immediately instead of recording the
    unchanged operating point under the requested ceiling. A supported
    measurement path (e.g. a diagnostics endpoint that writes
    a_gain_max/exp_time_max) must be restored before this script can be
    reused; its historical output lives in docs/reference/anyka-ae-units.md.

Usage:
  python3 scripts/debugging/measure_ae_units.py [--host 192.168.2.198]
Env:
  CAMERA_USER / CAMERA_PASS (default admin:admin) for HTTP and RTSP.
Output: a markdown table on stdout, ready for docs/reference/anyka-ae-units.md.
"""

import argparse
import json
import os
import subprocess
import sys
import time
import urllib.request
import base64

FADE_SETTLE_S = 4  # AE converges in a couple of frames; 4 s is generous
# 720p yuv420p: Y is 2/3 of the frame bytes (U and V 1/6 each).
FRAME = 1280 * 720 * 3 // 2


def http_json(host: str, path: str, method: str = "GET", body: dict | None = None):
    req = urllib.request.Request(
        f"http://{host}{path}",
        method=method,
        headers={"Content-Type": "application/json"},
    )
    if body is not None:
        req.data = json.dumps(body).encode()
    user = os.environ.get("CAMERA_USER", "admin")
    pw = os.environ.get("CAMERA_PASS", "admin")
    req.add_header(
        "Authorization",
        "Basic " + base64.b64encode(f"{user}:{pw}".encode()).decode(),
    )
    try:
        with urllib.request.urlopen(req, timeout=15) as r:
            return r.status, json.loads(r.read().decode())
    except urllib.error.HTTPError as e:
        return e.code, None


def frame_luma(host: str) -> float | None:
    """Mean Y of one complete RTSP frame; None when the frame is unusable."""
    user = os.environ.get("CAMERA_USER", "admin")
    pw = os.environ.get("CAMERA_PASS", "admin")
    try:
        raw = subprocess.run(
            ["ffmpeg", "-hide_banner", "-loglevel", "error", "-rtsp_transport", "tcp",
             "-i", f"rtsp://{user}:{pw}@{host}:554/main",
             "-frames:v", "1", "-pix_fmt", "yuv420p", "-f", "rawvideo", "-"],
            stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, timeout=30,
        ).stdout
        if len(raw) != FRAME:
            print(f"  frame luma: incomplete frame ({len(raw)} != {FRAME} bytes)",
                  file=sys.stderr)
            return None
        y = raw[: FRAME * 2 // 3]
        return sum(y) / len(y)
    except Exception as e:  # noqa: BLE001 - a dropped frame is data, not a crash
        print(f"  frame luma failed: {e}", file=sys.stderr)
        return None


def step(host: str, a_gain_max, exp_time_max, label: str) -> dict:
    st, r = http_json(host, "/api/ae-debug", "PUT",
                      {k: v for k, v in
                       [("a_gain_max", a_gain_max), ("exp_time_max", exp_time_max)]
                       if v is not None})
    if st != 200 or r is None or "error" in r:
        # Fail fast: a failed write means the next measurement would record
        # the UNCHANGED operating point under the requested ceiling, which
        # would silently corrupt the sweep. If this is the first step, the
        # message below explains that the endpoint is gone, not that one
        # write raced.
        sys.exit(
            f"AE ceiling write failed for {label} (HTTP {st}): "
            + (f"{r}" if r else "endpoint absent")
            + "\n/api/ae-debug was removed after the original measurement; "
              "restore a supported AE-ceiling write path before reusing this "
              "script (historical data: docs/reference/anyka-ae-units.md)."
        )
    time.sleep(FADE_SETTLE_S)
    st, diag = http_json(host, "/api/diagnostics")
    if st != 200 or diag is None:
        sys.exit(f"/api/diagnostics failed (HTTP {st}) for {label}")
    vision = diag.get("vision") or {}
    info = vision.get("ae_run_info") or {}
    luma = frame_luma(host)
    if luma is None:
        sys.exit(f"luma unreadable for {label}: the sweep's luma evidence "
                 "would be incomplete")
    row = {
        "label": label,
        "a_gain_max_set": a_gain_max,
        "exp_time_max_set": exp_time_max,
        "a_gain": info.get("a_gain"),
        "d_gain": info.get("d_gain"),
        "isp_d_gain": info.get("isp_d_gain"),
        "exp_time": info.get("exp_time"),
        "avg_lumi": info.get("avg_lumi"),
        "frame_y": luma,
    }
    print(
        f"  {label:28s} exp={row['exp_time']} a_gain={row['a_gain']} "
        f"isp_d={row['isp_d_gain']} avg_lumi={row['avg_lumi']} frameY={luma}",
        flush=True,
    )
    return row


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--host", default="192.168.2.198")
    ap.add_argument("--out", default="ae_sweep.tsv")
    args = ap.parse_args()
    host = args.host

    st, diag = http_json(host, "/api/diagnostics")
    if st != 200 or diag is None:
        sys.exit("camera unreachable")
    vision = diag.get("vision") or {}
    orig = {
        "a_gain_max": vision.get("ae_a_gain_max"),
        "exp_time_max": vision.get("ae_exp_time_max"),
    }
    print(f"original ceilings: {orig}", file=sys.stderr)

    rows = []
    try:
        # Phase A: pin gain at 1x (Q8 256), sweep exposure ceiling ascending.
        print("phase A: exposure sweep (a_gain_max=256)", file=sys.stderr)
        for exp_max in [40, 100, 200, 400, 800, 1600, 3000, 5000]:
            rows.append(step(host, 256, exp_max, f"A exp_max={exp_max}"))
        # Phase B: pin exposure low, sweep gain ceiling ascending.
        print("phase B: gain sweep (exp_time_max=10)", file=sys.stderr)
        for gain_max in [256, 512, 1024, 2048, 4096, 8192, 16384]:
            rows.append(step(host, gain_max, 10, f"B gain_max={gain_max}"))
    finally:
        # Restore what we found on every exit path: a sweep interrupted
        # mid-run must not leave the AE pinned at a foreign ceiling.
        print("restoring original ceilings", file=sys.stderr)
        st, r = http_json(host, "/api/ae-debug", "PUT",
                          {k: v for k, v in orig.items() if v is not None})
        if st != 200:
            print(f"  WARNING: restore write failed (HTTP {st}); the camera "
                  f"may still hold a swept ceiling until reboot", file=sys.stderr)

    cols = list(rows[0].keys())
    with open(args.out, "w") as f:
        f.write("\t".join(cols) + "\n")
        for r in rows:
            f.write("\t".join(str(r[c]) for c in cols) + "\n")
    print(f"\nwrote {args.out}")
    print("\n| " + " | ".join(cols) + " |")
    print("|" + "---|" * len(cols))
    for r in rows:
        print("| " + " | ".join(str(r[c]) for c in cols) + " |")


if __name__ == "__main__":
    main()
