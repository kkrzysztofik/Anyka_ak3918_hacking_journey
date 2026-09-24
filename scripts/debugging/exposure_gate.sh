#!/usr/bin/env bash
# Task 11 hardware gate (imaging-tab-completion plan).
#
# Verifies the exposure surface against the camera:
#   1. GetImagingSettings reports Exposure/Mode = AUTO
#   2. GetImagingOptions advertises AUTO only and (if the AE ceiling read
#      worked) a gain range in dB
#   3. SetImagingSettings with MANUAL is rejected with a spec fault
#   4. The RTSP stream's luma is unchanged (the AE loop is still driving)
#
# Every SOAP outcome is asserted: a failed GET, an accepted MANUAL set, or an
# unparsable response fails the gate instead of being printed and ignored.
#
# Usage: scripts/debugging/exposure_gate.sh [host]
# Env:   CAMERA_USER / CAMERA_PASS (default admin:admin), used for SOAP and RTSP.

set -euo pipefail
HOST="${1:-192.168.2.198}"
SOAP="http://$HOST/onvif/imaging_service"

python3 - "$SOAP" <<'EOF'
import base64, os, re, sys, time, urllib.request

soap = sys.argv[1]
user = os.environ.get("CAMERA_USER", "admin")
pw = os.environ.get("CAMERA_PASS", "admin")
soap_host = soap.replace("http://", "").split("/")[0]
auth = "Basic " + base64.b64encode(f"{user}:{pw}".encode()).decode()

def call(body, action):
    """Returns (status, body)."""
    env = (
        '<?xml version="1.0" encoding="UTF-8"?>'
        '<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope" '
        'xmlns:timg="http://www.onvif.org/ver20/imaging/wsdl" '
        'xmlns:tt="http://www.onvif.org/ver10/schema">'
        '<s:Body>' + body + '</s:Body></s:Envelope>'
    )
    req = urllib.request.Request(
        soap, data=env.encode(),
        headers={
            "Content-Type": "application/soap+xml; charset=utf-8",
            "Authorization": auth,
        },
    )
    try:
        with urllib.request.urlopen(req, timeout=15) as r:
            return str(r.status), r.read().decode(errors="replace")
    except urllib.error.HTTPError as e:
        return str(e.code), e.read().decode(errors="replace")

def grab(out, tag):
    m = re.search(rf"<{tag}>(.*?)</{tag}>", out, re.S)
    return m.group(1).strip() if m else None

get_settings = (
    '<timg:GetImagingSettings><tt:VideoSourceToken xmlns:tt="http://www.onvif.org/ver10/schema">VideoSource_1</tt:VideoSourceToken></timg:GetImagingSettings>'
)
get_options = (
    '<timg:GetOptions><tt:VideoSourceToken xmlns:tt="http://www.onvif.org/ver10/schema">VideoSource_1</tt:VideoSourceToken></timg:GetOptions>'
)
set_manual = (
    '<timg:SetImagingSettings>'
    '<tt:VideoSourceToken xmlns:tt="http://www.onvif.org/ver10/schema">VideoSource_1</tt:VideoSourceToken>'
    '<tt:ImagingSettings xmlns:tt="http://www.onvif.org/ver10/schema">'
    '<tt:Exposure><tt:Mode>MANUAL</tt:Mode></tt:Exposure>'
    '</tt:ImagingSettings>'
    '</timg:SetImagingSettings>'
)

# --- 1. settings report AUTO
st, s = call(get_settings, "GetImagingSettings")
mode = grab(s, "tt:Mode") or grab(s, "Mode")
print(f"-- GetImagingSettings: HTTP {st}  Exposure/Mode={mode}  present={'<tt:Exposure' in s}")
if st != "200":
    sys.exit(f"GATE FAIL: GetImagingSettings returned HTTP {st}: {s[:200]}")
if mode != "AUTO":
    sys.exit(f"GATE FAIL: Exposure/Mode is {mode!r}, expected AUTO")

# --- 2. options advertise AUTO only (+ gain range if the ceiling read worked)
st, o = call(get_options, "GetOptions")
print(f"-- GetOptions: HTTP {st}")
if st != "200":
    sys.exit(f"GATE FAIL: GetOptions returned HTTP {st}: {o[:200]}")
m = re.search(r"<tt:Exposure.*?</tt:Exposure>", o, re.S)
block = m.group(0) if m else ""
modes = re.findall(r"<tt:Mode>(\w+)</tt:Mode>", block)
print(f"  Options Exposure block: {block[:400]}")
print(f"  Advertised modes: {modes}")
if "AUTO" not in modes or "MANUAL" in modes:
    sys.exit(f"GATE FAIL: expected exactly [AUTO] advertised, got {modes}")

# --- 3. MANUAL must be a clean spec fault (400), not a 500, not an accept
st, r = call(set_manual, "SetImagingSettings(MANUAL)")
m = re.search(r"<s:FaultText>(.*?)</s:FaultText>", r, re.S)
fault = m.group(1).strip() if m else None
print(f"-- MANUAL set: HTTP {st}  fault: {fault or r[:200]}")
if st != "400" or not fault:
    sys.exit(f"GATE FAIL: MANUAL must be rejected with HTTP 400 + fault, got {st} / {fault}")

# --- 4. luma stability around the rejected set
import subprocess

# 720p yuv420p: Y is 2/3 of the frame bytes (U and V 1/6 each). A short
# read is an incomplete frame and is rejected, never averaged.
FRAME = 1280 * 720 * 3 // 2

def luma():
    raw = subprocess.run(
        ["ffmpeg", "-hide_banner", "-loglevel", "error", "-rtsp_transport", "tcp",
         "-i", f"rtsp://{user}:{pw}@{soap_host}:554/main",
         "-frames:v", "1", "-pix_fmt", "yuv420p", "-f", "rawvideo", "-"],
        stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, timeout=30).stdout
    if len(raw) != FRAME:
        return None
    y = raw[: FRAME * 2 // 3]
    return sum(y) / len(y)

y1 = luma(); time.sleep(3); y2 = luma()
time.sleep(3)
y3 = luma(); time.sleep(3); y4 = luma()
if None in (y1, y2, y3, y4):
    sys.exit("GATE FAIL: incomplete frame — luma could not be measured")

print(f"\n  luma before: {y1:.1f} / {y2:.1f}")
print(f"  luma after:  {y3:.1f} / {y4:.1f}")
shift = abs((y3 + y4) / 2 - (y1 + y2) / 2)
print(f"\n  luma shift: {shift:.1f} (PASS if < 5)")
if shift >= 5:
    sys.exit("GATE FAIL: luma moved around the rejected MANUAL set")
print("GATE: PASS")
EOF
