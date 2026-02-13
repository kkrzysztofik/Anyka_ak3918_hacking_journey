#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage:
  measure_rtsp_fps.sh --onvif-bin <path> --h264-file <path> --aac-file <path> [options]

Options:
  --duration <seconds>         Measurement window (default: 60)
  --rtsp-port <port>           RTSP port (default: 8554)
  --httpflv-port <port>        HTTP-FLV port (default: 8080)
  --audio-sample-rate <hz>     AAC sample rate (default: 48000)
  --host <host>                RTSP host (default: 127.0.0.1)
  --fps-threshold <fps>        Pass threshold (default: 24.8)

Environment profile used for this measurement:
  ONVIF_VALIDATION_ENABLE_HTTPFLV=0
  ONVIF_VALIDATION_ENABLE_ONVIF_APP=0
USAGE
}

require_cmd() {
  local cmd="$1"
  if ! command -v "$cmd" >/dev/null 2>&1; then
    echo "error: required command not found: $cmd" >&2
    return 2
  fi
}

ONVIF_BIN=""
H264_FILE=""
AAC_FILE=""
DURATION=60
RTSP_PORT=8554
HTTPFLV_PORT=8080
AUDIO_SAMPLE_RATE=48000
HOST="127.0.0.1"
FPS_THRESHOLD=24.8

while [[ $# -gt 0 ]]; do
  case "$1" in
    --onvif-bin)
      ONVIF_BIN="$2"
      shift 2
      ;;
    --h264-file)
      H264_FILE="$2"
      shift 2
      ;;
    --aac-file)
      AAC_FILE="$2"
      shift 2
      ;;
    --duration)
      DURATION="$2"
      shift 2
      ;;
    --rtsp-port)
      RTSP_PORT="$2"
      shift 2
      ;;
    --httpflv-port)
      HTTPFLV_PORT="$2"
      shift 2
      ;;
    --audio-sample-rate)
      AUDIO_SAMPLE_RATE="$2"
      shift 2
      ;;
    --host)
      HOST="$2"
      shift 2
      ;;
    --fps-threshold)
      FPS_THRESHOLD="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "error: unknown argument: $1" >&2
      usage
      exit 2
      ;;
  esac
done

if [[ -z "$ONVIF_BIN" || -z "$H264_FILE" || -z "$AAC_FILE" ]]; then
  echo "error: --onvif-bin, --h264-file, and --aac-file are required" >&2
  usage
  exit 2
fi

if [[ ! -x "$ONVIF_BIN" ]]; then
  echo "error: onvif binary is not executable: $ONVIF_BIN" >&2
  exit 2
fi
if [[ ! -f "$H264_FILE" ]]; then
  echo "error: h264 file not found: $H264_FILE" >&2
  exit 2
fi
if [[ ! -f "$AAC_FILE" ]]; then
  echo "error: aac file not found: $AAC_FILE" >&2
  exit 2
fi

require_cmd ffprobe
require_cmd ffmpeg
require_cmd awk
require_cmd ps

RTSP_URL="rtsp://${HOST}:${RTSP_PORT}/stream1"
LOG_DIR="${TMPDIR:-/tmp}/onvif_rtsp_measure_$$"
mkdir -p "$LOG_DIR"
SERVER_LOG="$LOG_DIR/onvif.log"
PACKET_CSV="$LOG_DIR/video_packets.csv"
FFMPEG_LOG="$LOG_DIR/ffmpeg.log"

cleanup() {
  if [[ -n "${ONVIF_PID:-}" ]] && kill -0 "$ONVIF_PID" >/dev/null 2>&1; then
    kill "$ONVIF_PID" >/dev/null 2>&1 || true
    wait "$ONVIF_PID" >/dev/null 2>&1 || true
  fi
  return 0
}
trap cleanup EXIT

export ONVIF_VALIDATION_ENABLE_HTTPFLV=0
export ONVIF_VALIDATION_ENABLE_ONVIF_APP=0

"$ONVIF_BIN" \
  --validation-mode \
  --h264-file "$H264_FILE" \
  --aac-file "$AAC_FILE" \
  --audio-sample-rate "$AUDIO_SAMPLE_RATE" \
  --rtsp-port "$RTSP_PORT" \
  --httpflv-port "$HTTPFLV_PORT" \
  --loop-playback >"$SERVER_LOG" 2>&1 &
ONVIF_PID=$!

echo "onvif_pid=$ONVIF_PID"

echo "waiting for RTSP stream: $RTSP_URL"
for _ in $(seq 1 60); do
  if ffprobe -v error -rtsp_transport tcp -show_entries stream=codec_name -of default=nk=1:nw=1 "$RTSP_URL" >/dev/null 2>&1; then
    break
  fi
  sleep 1
done

if ! kill -0 "$ONVIF_PID" >/dev/null 2>&1; then
  echo "error: onvif-rust exited early" >&2
  tail -n 80 "$SERVER_LOG" >&2 || true
  exit 1
fi

ffprobe -v error \
  -rtsp_transport tcp \
  -read_intervals "0%+${DURATION}" \
  -select_streams v:0 \
  -show_entries packet=pts_time,size \
  -of csv=p=0 \
  "$RTSP_URL" >"$PACKET_CSV"

ffmpeg -hide_banner -nostats -loglevel info \
  -rtsp_transport tcp \
  -t "$DURATION" \
  -i "$RTSP_URL" \
  -f null - >"$FFMPEG_LOG" 2>&1 || true

read -r avg_fps mean_delta stddev_delta packet_count stream_duration < <(
  awk -F',' '
    $1 == "N/A" || $1 == "" { next }
    {
      t = $1 + 0.0
      c++
      if (c == 1) {
        first = t
        prev = t
      } else {
        d = t - prev
        prev = t
        sum += d
        sumsq += d * d
      }
      last = t
    }
    END {
      if (c < 2 || last <= first) {
        print "0 0 0 0 0"
        exit
      }
      duration = last - first
      fps = c / duration
      n = c - 1
      mean = sum / n
      var = (sumsq / n) - (mean * mean)
      if (var < 0) {
        var = 0
      }
      stddev = sqrt(var)
      print fps, mean, stddev, c, duration
    }
  ' "$PACKET_CSV"
)

cpu_threads_line=$(ps -o %cpu=,nlwp= -p "$ONVIF_PID" 2>/dev/null | awk '{print $1, $2}' || echo "0 0")
proc_cpu=$(echo "$cpu_threads_line" | awk '{print $1}')
proc_threads=$(echo "$cpu_threads_line" | awk '{print $2}')
proc_rss_kb=$(awk '/^VmRSS:/ {print $2}' "/proc/$ONVIF_PID/status" 2>/dev/null || echo 0)

drop_count=$(awk -F'[: ]+' '/drop=/ {for (i=1; i<=NF; i++) if ($i ~ /^drop=/) {split($i,a,"="); print a[2]}}' "$FFMPEG_LOG" | tail -n 1)
if [[ -z "$drop_count" ]]; then
  drop_count=0
fi

echo "metrics:"
echo "  avg_fps=${avg_fps}"
echo "  packet_count=${packet_count}"
echo "  stream_duration_s=${stream_duration}"
echo "  interframe_mean_s=${mean_delta}"
echo "  interframe_stddev_s=${stddev_delta}"
echo "  ffmpeg_drop_count=${drop_count}"
echo "  process_cpu_percent=${proc_cpu}"
echo "  process_threads=${proc_threads}"
echo "  process_rss_kb=${proc_rss_kb}"
echo "  logs_dir=${LOG_DIR}"

if ! awk -v fps="$avg_fps" -v threshold="$FPS_THRESHOLD" '
  BEGIN {
    if (fps + 0.0 >= threshold + 0.0) {
      exit 0
    }
    exit 1
  }
'; then
  echo "FAIL: avg_fps=${avg_fps} is below threshold=${FPS_THRESHOLD}" >&2
  exit 1
fi

echo "PASS: avg_fps=${avg_fps} >= threshold=${FPS_THRESHOLD}"
