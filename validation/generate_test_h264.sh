#!/bin/bash
# Generate a test H.264 file for RTSP validation

OUTPUT_FILE="${1:-test_video.h264}"
DURATION="${2:-10}"
FPS="${3:-25}"
RESOLUTION="${4:-1920x1080}"

log_info() { echo "[INFO] $*"; }
log_error() { echo "[ERROR] $*" >&2; }

if ! command -v ffmpeg &> /dev/null; then
    log_error "ffmpeg not found. Install with: sudo apt-get install ffmpeg"
    exit 1
fi

log_info "Generating test H.264 file..."
log_info "  Output: $OUTPUT_FILE"
log_info "  Duration: ${DURATION}s"
log_info "  FPS: $FPS"
log_info "  Resolution: $RESOLUTION"

ffmpeg -f lavfi -i "testsrc=s=${RESOLUTION}:d=${DURATION}:r=${FPS}" \
  -c:v libx264 -preset fast -crf 23 \
  -bsf h264_mp4toannexb \
  -y "${OUTPUT_FILE}" 2>&1 | grep -E "frame=|bitrate=" || true

if [ -f "$OUTPUT_FILE" ]; then
    file_size=$(du -h "$OUTPUT_FILE" | cut -f1)
    log_info "Generated test H.264 file: ${OUTPUT_FILE} (${file_size})"
else
    log_error "Failed to generate H.264 file"
    exit 1
fi
