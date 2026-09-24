#!/usr/bin/env bash
# White balance hardware gate (Task 9): proves MANUAL mode with a strongly
# red-biased CrGain produces a visible red cast, and AUTO corrects it.
#
# Every SOAP outcome is checked (a fault or curl failure fails the gate
# instead of printing and moving on), and the camera is restored to AUTO on
# every exit path, including interruption, via the EXIT trap.
#
# Usage: scripts/debugging/wb_gate.sh [host]
# Env:   CAMERA_USER / CAMERA_PASS (default admin:admin) for SOAP and RTSP.
#
# The U/V numbers are printed for the human who judges the cast: PASS means
# every write succeeded, the visual verdict is yours.

set -euo pipefail
HOST="${1:-192.168.2.198}"
SOAP="http://$HOST/onvif/imaging_service"
CAM_USER="${CAMERA_USER:-admin}"
CAM_PASS="${CAMERA_PASS:-admin}"
STREAM="rtsp://${CAM_USER}:${CAM_PASS}@${HOST}:554/main"

soap() { # $1 = mode  $2 = cr  $3 = cb  (omit cr/cb for AUTO)
  local wb="<tt:Mode>$1</tt:Mode>"
  if [ $# -ge 3 ] && [ -n "$2" ]; then
    wb="$wb<tt:CrGain>$2</tt:CrGain><tt:CbGain>$3</tt:CbGain>"
  fi
  cat > /tmp/wb.xml <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope" xmlns:timg="http://www.onvif.org/ver20/imaging/wsdl" xmlns:tt="http://www.onvif.org/ver10/schema">
<s:Body><timg:SetImagingSettings><timg:VideoSourceToken>VideoSource_1</tt:VideoSourceToken><tt:ImagingSettings><tt:WhiteBalance>$wb</tt:WhiteBalance></tt:ImagingSettings></s:Body>
</s:Envelope>
EOF
  local resp rc
  resp=$(curl -s -u "$CAM_USER:$CAM_PASS" -m 15 -H 'Content-Type: application/soap+xml' --data-binary @/tmp/wb.xml $SOAP)
  rc=$?
  if [ $rc -ne 0 ] || [ -z "$resp" ]; then
    echo "UNREACHABLE (curl rc=$rc)"
    return 1
  fi
  if echo "$resp" | grep -qi fault; then
    echo "FAULT: $(echo "$resp" | grep -o 'Fault[^<]*<[^>]*>[^<]*' | head -2 | tr -d '\n')"
    return 1
  fi
  echo "ok"
}

# Restore AUTO whenever we leave, for any reason. A failed restore here must
# not clobber the exit code of the check that triggered it, so the result is
# reported and the original status is preserved.
restore_auto() {
  local rc=$?
  if soap AUTO > /dev/null 2>&1; then
    echo "(restored to AUTO)"
  else
    echo "WARNING: could not restore AUTO — set it manually before the next gate" >&2
  fi
  exit $rc
}
trap restore_auto EXIT

measure() { # $1 = label; grabs one complete frame, prints mean U and V
  # yuv420p: Y is 2/3 of the frame, U and V 1/6 each. The old 1/3/3/3 split
  # only covered half of Y and mixed chroma into it; a short read is an
  # incomplete frame and is rejected, never averaged.
  ffmpeg -hide_banner -loglevel error -rtsp_transport tcp -i "$STREAM" -frames:v 1 -pix_fmt yuv420p -f rawvideo - 2>/dev/null |
    python3 -c "
import sys
d = sys.stdin.buffer.read()
f = 1280 * 720 * 3 // 2
if len(d) != f:
    sys.exit(f'$1: incomplete frame ({len(d)} != {f} bytes)')
n = f * 2 // 3
sixth = f // 6
y, u, v = d[:n], d[n:n + sixth], d[n + sixth:n + 2 * sixth]
print('$1: Y=%.1f U=%.1f (blue diff, 128=neutral) V=%.1f (red diff, 128=neutral)'
      % (sum(y)/len(y), sum(u)/len(u), sum(v)/len(v)))
"
}

echo "== firmware =="
curl -s -m 10 -u "$CAM_USER:$CAM_PASS" http://$HOST/api/diagnostics | python3 -c "import json,sys; print('fw:', json.load(sys.stdin).get('firmware_version'))"

echo "== 1. baseline (AUTO) =="
measure "AUTO  base"
sleep 2

echo "== 2. MANUAL cr=3.0 cb=1.0 (strong red bias) =="
if ! soap MANUAL 3.0 1.0; then
  echo "GATE FAIL: MANUAL set rejected — nothing to compare"
  exit 1
fi
sleep 6
measure "MANUAL"
sleep 2

echo "== 3. back to AUTO =="
if ! soap AUTO; then
  echo "GATE FAIL: AUTO restore rejected"
  exit 1
fi
sleep 10
measure "AUTO  after"

echo
echo "GATE: all writes succeeded — judge the red cast from the U/V numbers"
exit 0
