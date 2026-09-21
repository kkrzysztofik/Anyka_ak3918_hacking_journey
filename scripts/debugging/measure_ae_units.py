#!/usr/bin/env python3
"""Measure the AK3918 AE raw-unit scales (Task 10, imaging-tab-completion plan).

Drives the camera's temporary /api/ae-debug endpoint through two isolated
sweeps and records, per step, the AE operating point (raw units) and the
achieved frame luma (Y plane of one RTSP frame):

  A. exposure: gain pinned at 1x (a_gain_max=256), exp_time_max swept
     ascending — luma may only move through exposure, so luma vs exp_time
     yields µs-per-unit once fps is known.
  B. gain: exposure pinned low (exp_time_max=10), a_gain_max swept ascending
     — luma vs a_gain yields the dB mapping (256 = 1x expected, Q8).

The AE attribute as found on arrival is restored at the end.

Usage:
  python3 scripts/debugging/measure_ae_units.py [--host 192.168.2.198]
Output: a markdown table on stdout, ready for docs/reference/anyka-ae-units.md.
"""

import argparse
import json
import subprocess
import sys
import time
import urllib.request
import base64

FADE_SETTLE_S = 4  # AE converges in a couple of frames; 4 s is generous


def http_json(host: str, path: str, method: str = "GET", body: dict | None = None):
    req = urllib.request.Request(
        f"http://{host}{path}",
        method=method,
        headers={"Content-Type": "application/json"},
    )
    if body is not None:
        req.data = json.dumps(body).encode()
    # admin:admin is the stock credential; pass CAMERA_USER/CAMERA_PASS to override.
    import os
    user = os.environ.get("CAMERA_USER", "admin")
    pw = os.environ.get("CAMERA_PASS", "admin")
    req.add_header(
        "Authorization",
        "Basic " + base64.b64encode(f"{user}:{pw}".encode()).decode(),
    )
    try:
        with urllib.request.urlopen(req, timeout=15) as r:
            return json.loads(r.read().decode())
    except urllib.error.HTTPError as e:
        print(f"  HTTP {e.code} on {method} {path}: {e.read().decode()[:200]}", file=sys.stderr)
        return None


def frame_luma(host: str) -> float | None:
    """Mean Y of one RTSP frame."""
    try:
        raw = subprocess.run(
            ["ffmpeg", "-hide_banner", "-loglevel", "error", "-rtsp_transport", "tcp",
             "-i", f"rtsp://admin:admin@{host}:554/main",
             "-frames:v", "1", "-pix_fmt", "yuv420p", "-f", "rawvideo", "-"],
            stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, timeout=30,
        ).stdout
        n = len(raw) // 3
        if n == 0:
            return None
        y = raw[:n]
        return sum(y) / len(y)
    except Exception as e:  # noqa: BLE001 - a dropped frame is data, not a crash
        print(f"  frame luma failed: {e}", file=sys.stderr)
        return None


def step(host: str, a_gain_max, exp_time_max, label: str) -> dict:
    body = {}
    if a_gain_max is not None:
        body["a_gain_max"] = a_gain_max
    if exp_time_max is not None:
        body["exp_time_max"] = exp_time_max
    r = http_json(host, "/api/ae-debug", "PUT", body)
    if r is None:
        print(f"  set failed for {label}", file=sys.stderr)
    time.sleep(FADE_SETTLE_S)
    diag = http_json(host, "/api/diagnostics")
    vision = (diag or {}).get("vision") or {}
    info = vision.get("ae_run_info") or {}
    luma = frame_luma(host)
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

    diag = http_json(host, "/api/diagnostics")
    if diag is None:
        sys.exit("camera unreachable")
    vision = diag.get("vision") or {}
    orig = {
        "a_gain_max": vision.get("ae_a_gain_max"),
        "exp_time_max": vision.get("ae_exp_time_max"),
    }
    print(f"original ceilings: {orig}", file=sys.stderr)

    rows = []
    # Phase A: pin gain at 1x (Q8 256), sweep exposure ceiling ascending.
    print("phase A: exposure sweep (a_gain_max=256)", file=sys.stderr)
    for exp_max in [40, 100, 200, 400, 800, 1600, 3000, 5000]:
        rows.append(step(host, 256, exp_max, f"A exp_max={exp_max}"))
    # Phase B: pin exposure low, sweep gain ceiling ascending.
    print("phase B: gain sweep (exp_time_max=10)", file=sys.stderr)
    for gain_max in [256, 512, 1024, 2048, 4096, 8192, 16384]:
        rows.append(step(host, gain_max, 10, f"B gain_max={gain_max}"))

    # Restore what we found.
    print("restoring original ceilings", file=sys.stderr)
    step(host, orig["a_gain_max"], orig["exp_time_max"], "restore")

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
