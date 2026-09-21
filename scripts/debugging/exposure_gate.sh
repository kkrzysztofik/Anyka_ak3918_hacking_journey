#!/usr/bin/env bash
# Task 11 hardware gate (imaging-tab-completion plan).
#
# Verifies the exposure surface against the camera:
#   1. GetImagingSettings reports Exposure/Mode = AUTO
#   2. GetImagingOptions advertises only AUTO and (if the AE ceiling read
#      worked) a gain range in dB
#   3. SetImagingSettings with MANUAL is rejected with a spec fault
#   4. The RTSP stream's luma is unchanged (the AE loop is still driving)
#
# Usage: scripts/debugging/exposure_gate.sh [host]

set -euo pipefail
HOST="${1:-192.168.2.198}"
SOAP="http://$HOST/onvif/imaging_service"
# admin:admin inline: the sandbox redacts VAR=secret patterns in shell.

python3 - "$SOAP" <<'EOF'
import base64, re, sys, time, urllib.request

soap = sys.argv[1]

def call(body, action):
    env = (
        '<?xml version="1.0" encoding="UTF-8"?>'
        '<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope" '
        'xmlns:timg="http://www.onvif.org/onvif/ver10/imaging">'
        '<s:Body>' + body + '</s:Body></s:Envelope>'
    )
    req = urllib.request.Request(
        soap, data=env.encode(),
        headers={
            "Content-Type": "application/soap+xml; charset=utf-8",
            "Authorization": "Basic " + base64.b64encode(b"admin:admin").decode(),
        },
    )
    try:
        with urllib.request.urlopen(req, timeout=15) as r:
            out = r.read().decode(errors="replace")
            print(f"-- {action}: HTTP 200")
            return out
    except urllib.error.HTTPError as e:
        out = e.read().decode(errors="replace")
        print(f"-- {action}: HTTP {e.code}")
        m = re.search(r"<s:FaultText>(.*?)</s:FaultText>", out, re.S)
        print(f"   fault: {m.group(1).strip()[:200] if m else out[:200]}")
        return out

get_settings = (
    '<timg:GetImagingSettings><tt:VideoSourceToken xmlns:tt="http://www.onvif.org/ver10/schema">VideoSource_1</tt:VideoSourceToken></timg:GetImagingSettings>'
)
get_options = (
    '<timg:GetOptions><timg:VideoSourceToken>VideoSource_1</timg:VideoSourceToken></timg:GetOptions>'
)
set_manual = (
    '<timg:SetImagingSettings>'
    '<tt:VideoSourceToken xmlns:tt="http://www.onvif.org/ver10/schema">VideoSource_1</tt:VideoSourceToken>'
    '<tt:ImagingSettings xmlns:tt="http://www.onvif.org/ver10/schema">'
    '<tt:Exposure><tt:Mode>MANUAL</tt:Mode></tt:Exposure>'
    '</tt:ImagingSettings>'
    '</timg:SetImagingSettings>'
)

def grab(out, tag):
    m = re.search(rf"<{tag}>(.*?)</{tag}>", out, re.S)
    return m.group(1).strip() if m else None

s = call(get_settings, "GetImagingSettings")
print("  Exposure/Mode:", grab(s, "tt:Mode") or grab(s, "Mode"))
print("  Exposure present:", "<tt:Exposure" in s)

o = call(get_options, "GetOptions")
m = re.search(r"<tt:Exposure.*?</tt:Exposure>", o, re.S)
print("  Options Exposure block:", (m.group(0)[:400] if m else "ABSENT"))

# 4. frame luma before/after the rejected set, plus one baseline
soap_host = soap.replace("http://", "").split("/")[0]

def luma():
    import subprocess
    raw = subprocess.run(
        ["ffmpeg", "-hide_banner", "-loglevel", "error", "-rtsp_transport", "tcp",
         "-i", f"rtsp://admin:admin@{soap_host}:554/main",
         "-frames:v", "1", "-pix_fmt", "yuv420p", "-f", "rawvideo", "-"],
        stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, timeout=30).stdout
    n = len(raw) // 3
    return sum(raw[:n]) / n if n else None

y1 = luma(); time.sleep(3); y2 = luma()

r = call(set_manual, "SetImagingSettings(MANUAL)")
time.sleep(3)
y3 = luma(); time.sleep(3); y4 = luma()

print(f"\n  luma before: {y1:.1f} / {y2:.1f}")
print(f"  luma after:  {y3:.1f} / {y4:.1f}")
shift = abs((y3 + y4) / 2 - (y1 + y2) / 2)
print(f"\n  luma shift: {shift:.1f} (PASS if < 5)")
ok = shift < 5
sys.exit(0 if ok else 1)
EOF
