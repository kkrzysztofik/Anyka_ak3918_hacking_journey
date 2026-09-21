#!/usr/bin/env bash
# White balance hardware gate (Task 9): proves MANUAL mode with a strongly
# red-biased CrGain produces a visible red cast, and AUTO corrects it.
set -u
HOST=192.168.2.198
SOAP=http://$HOST/onvif/imaging_service
STREAM=rtsp://admin:admin@$HOST:554/main

soap() { # $1 = mode  $2 = cr  $3 = cb  (omit cr/cb for AUTO)
  local wb="<tt:Mode>$1</tt:Mode>"
  if [ $# -ge 3 ] && [ -n "$2" ]; then
    wb="$wb<tt:CrGain>$2</tt:CrGain><tt:CbGain>$3</tt:CbGain>"
  fi
  cat > /tmp/wb.xml <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope" xmlns:timg="http://www.onvif.org/ver20/imaging/wsdl" xmlns:tt="http://www.onvif.org/ver10/schema">
<s:Body><timg:SetImagingSettings><timg:VideoSourceToken>VideoSource_1</timg:VideoSourceToken><timg:ImagingSettings><tt:WhiteBalance>$wb</tt:WhiteBalance></timg:ImagingSettings></timg:SetImagingSettings></s:Body>
</s:Envelope>
EOF
  local resp rc
  resp=$(curl -s -u admin:admin -m 15 -H 'Content-Type: application/soap+xml' --data-binary @/tmp/wb.xml $SOAP)
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

measure() { # $1 = label; grabs one frame, prints mean U (blue diff) and V (red diff)
  ffmpeg -hide_banner -loglevel error -rtsp_transport tcp -i "$STREAM" -frames:v 1 -pix_fmt yuv420p -f rawvideo - 2>/dev/null |
    python3 -c "
import sys
d = sys.stdin.buffer.read()
n = len(d) // 3
y, u, v = d[:n], d[n:2*n], d[2*n:3*n]
print('$1: U=%.1f (blue diff, 128=neutral) V=%.1f (red diff, 128=neutral)' % (sum(u)/len(u), sum(v)/len(v)))
"
}

echo "== firmware =="
curl -s -m 10 -u admin:admin http://$HOST/api/diagnostics | python3 -c "import json,sys; print('fw:', json.load(sys.stdin).get('firmware_version'))"

echo "== 1. baseline (AUTO) =="
measure "AUTO  base"
sleep 2

echo "== 2. MANUAL cr=3.0 cb=1.0 (strong red bias) =="
soap MANUAL 3.0 1.0
sleep 6
measure "MANUAL"
sleep 2

echo "== 3. back to AUTO =="
soap AUTO
sleep 10
measure "AUTO  after"