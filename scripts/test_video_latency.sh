#!/bin/bash

# Device Integration Test Script
# Tests video latency fixes on the Anyka AK3918 device
# Usage: Run this script on the device via SSH or serial console

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

cleanup() {
    if [ -n "${ONVIF_PID:-}" ]; then
        kill "$ONVIF_PID" 2>/dev/null || true
    fi
    if [ -n "${DAEMON_PID:-}" ]; then
        kill "$DAEMON_PID" 2>/dev/null || true
    fi
}
trap cleanup EXIT INT TERM

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_step() {
    echo -e "${BLUE}[STEP]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[OK]${NC} $1"
}

# Wait up to $2 seconds for no matching pgrep -f process; returns 0 if gone.
wait_proc_gone() {
    local pattern=$1
    local timeout_sec=${2:-5}
    local i=0
    while pgrep -f "$pattern" >/dev/null 2>&1 && [ "$i" -lt "$timeout_sec" ]; do
        sleep 1
        i=$((i + 1))
    done
    ! pgrep -f "$pattern" >/dev/null 2>&1
}

# Configuration
LOG_DIR="/mnt/logs"
DAEMON_LOG="$LOG_DIR/vendor_daemon.log"
ONVIF_LOG="$LOG_DIR/onvif.log"
DAEMON_BIN="/mnt/vendor-daemon"
ONVIF_BIN="/mnt/onvif-rust"

# Default device IP (can be overridden)
DEVICE_IP="${1:-192.168.1.100}"

echo "=============================================="
echo "  Video Latency Integration Test"
echo "=============================================="
echo ""
log_info "Device IP: $DEVICE_IP"
log_info "Log directory: $LOG_DIR"
echo ""

# ============================================
# 1. Stop existing services
# ============================================
log_step "Stopping existing services..."

# Kill existing processes
pkill -f vendor-daemon 2>/dev/null || true
pkill -f onvif-rust 2>/dev/null || true
pkill -f onvifd 2>/dev/null || true

for proc in "vendor-daemon" "onvif-rust" "onvifd"; do
    if ! wait_proc_gone "$proc" 8; then
        log_error "Process still running after pkill (waited up to 8s): $proc"
        exit 1
    fi
done
log_info "Services stopped"

# ============================================
# 2. Prepare log directory
# ============================================
log_step "Preparing log directory..."

mkdir -p "$LOG_DIR"

# Clear old logs
rm -f "$DAEMON_LOG" "$ONVIF_LOG"

log_info "Log directory prepared: $LOG_DIR"

# ============================================
# 3. Check binaries exist
# ============================================
log_step "Checking binaries..."

if [ ! -x "$DAEMON_BIN" ]; then
    log_error "vendor-daemon not found at: $DAEMON_BIN"
    log_error "Please copy binary to SD card first"
    exit 1
fi
log_info "vendor-daemon: $DAEMON_BIN"

if [ ! -x "$ONVIF_BIN" ]; then
    log_error "onvif-rust not found at: $ONVIF_BIN"
    log_error "Please copy binary to SD card first"
    exit 1
fi
log_info "onvif-rust: $ONVIF_BIN"

echo ""

# ============================================
# 4. Start vendor-daemon
# ============================================
log_step "Starting vendor-daemon..."

pushd /mnt >/dev/null || {
    log_error "Cannot change directory to /mnt"
    exit 1
}
"$DAEMON_BIN" > "$DAEMON_LOG" 2>&1 &
DAEMON_PID=$!
log_info "vendor-daemon started (PID: $DAEMON_PID)"

sleep 3

# Check if daemon is running (PID-based — avoids fragile ps|grep races)
if [ -n "${DAEMON_PID:-}" ] && kill -0 "$DAEMON_PID" 2>/dev/null; then
    log_success "vendor-daemon is running"
else
    log_error "vendor-daemon failed to start"
    log_error "Check log: $DAEMON_LOG"
    popd >/dev/null || true
    exit 1
fi
popd >/dev/null || true

# ============================================
# 5. Start onvif-rust
# ============================================
log_step "Starting onvif-rust..."

"$ONVIF_BIN" > "$ONVIF_LOG" 2>&1 &
ONVIF_PID=$!
log_info "onvif-rust started (PID: $ONVIF_PID)"

sleep 3

# Check if onvif is running
if [ -n "${ONVIF_PID:-}" ] && kill -0 "$ONVIF_PID" 2>/dev/null; then
    log_success "onvif-rust is running"
else
    log_error "onvif-rust failed to start"
    log_error "Check log: $ONVIF_LOG"
    kill "$DAEMON_PID" 2>/dev/null || true
    exit 1
fi

echo ""

# ============================================
# 6. Verify RTSP port is listening
# ============================================
log_step "Checking RTSP port..."

sleep 2

RTSP_PORT_GREP='(^|[:.])8554($|[^0-9])'
if netstat -ln 2>/dev/null | grep -E -q "${RTSP_PORT_GREP}"; then
    log_success "RTSP port 8554 is listening"
elif ss -ln 2>/dev/null | grep -E -q "${RTSP_PORT_GREP}"; then
    log_success "RTSP port 8554 is listening"
else
    log_warn "RTSP port 8554 not detected yet - may take a moment"
    sleep 2
    if netstat -ln 2>/dev/null | grep -E -q "${RTSP_PORT_GREP}"; then
        log_success "RTSP port 8554 is now listening"
    elif ss -ln 2>/dev/null | grep -E -q "${RTSP_PORT_GREP}"; then
        log_success "RTSP port 8554 is now listening"
    else
        log_error "RTSP port 8554 not listening"
        log_error "Check onvif log: $ONVIF_LOG"
        exit 1
    fi
fi

echo ""

# ============================================
# 7. Initial log analysis
# ============================================
log_step "Initial log analysis..."

echo ""
log_info "=== vendor-daemon startup logs ==="
head -20 "$DAEMON_LOG" 2>/dev/null || log_warn "No daemon logs yet"

echo ""
log_info "=== onvif-rust startup logs ==="
head -20 "$ONVIF_LOG" 2>/dev/null || log_warn "No onvif logs yet"

echo ""

# ============================================
# 8. Wait for manual VLC test
# ============================================
log_step "Manual VLC Test Required"
echo ""
echo "=============================================="
echo "  CONNECT VLC TO TEST THE STREAM"
echo "=============================================="
echo ""
echo "1. Open VLC on your computer"
echo "2. Media → Open Network Stream"
echo "3. URL: rtsp://$DEVICE_IP:8554/stream"
echo "4. Click Play"
echo ""
echo "Monitor for:"
echo "  - Stream starts within 2 seconds"
echo "  - No 'late video' errors"
echo "  - No 'picture is too late' warnings"
echo ""
echo "Let the stream run for at least 30 seconds, then press ENTER to continue..."
echo ""
if [ -t 0 ]; then
    read -r
else
    log_warn "Non-interactive stdin: skipping ENTER wait (sleeping 30s to match minimum stream run, then continuing)"
    sleep 30
fi

# ============================================
# 9. Collect diagnostic information
# ============================================
log_step "Collecting diagnostic information..."

echo ""
log_info "=== Timestamp Normalization Check ==="
if [ -f "$DAEMON_LOG" ]; then
    anchor_count=$(grep -c "timestamp_anchor" "$DAEMON_LOG" 2>/dev/null || echo "0")
    log_info "Timestamp anchors found: $anchor_count"
    
    echo "Sample anchor events:"
    grep "timestamp_anchor" "$DAEMON_LOG" | head -4 || echo "  (none found)"
    
    echo ""
    echo "First 10 slot timestamps:"
    grep -o "slot_ts_ms=[0-9]*" "$DAEMON_LOG" | head -10 | sed 's/slot_ts_ms=/  /' || echo "  (none found)"
else
    log_warn "Daemon log not found"
fi

echo ""
log_info "=== RTP Send Performance Check ==="
if [ -f "$ONVIF_LOG" ]; then
    slow_count=$(grep -c "rtp_send_slow" "$ONVIF_LOG" 2>/dev/null || echo "0")
    log_info "Slow RTP sends: $slow_count (expect 0)"
    
    echo ""
    echo "Frame send times (first 10):"
    grep -o "frame_send_ms=[0-9]*" "$ONVIF_LOG" | head -10 | sed 's/frame_send_ms=/  /' || echo "  (none found)"
    
    echo ""
    echo "Sample packet counts:"
    grep "payload_len=" "$ONVIF_LOG" | head -5 | grep -o "packet_count=[0-9]*" | sed 's/packet_count=/  /' || echo "  (none found)"
else
    log_warn "ONVIF log not found"
fi

echo ""

# ============================================
# 10. Stop services
# ============================================
log_step "Stopping services..."

if [ -n "${ONVIF_PID:-}" ]; then
    kill "$ONVIF_PID" 2>/dev/null || true
fi
if [ -n "${DAEMON_PID:-}" ]; then
    kill "$DAEMON_PID" 2>/dev/null || true
fi
sleep 1

# Force kill if still running
pkill -f vendor-daemon 2>/dev/null || true
pkill -f onvif-rust 2>/dev/null || true

for proc in "vendor-daemon" "onvif-rust"; do
    if ! wait_proc_gone "$proc" 8; then
        log_error "Process still running after stop (waited up to 8s): $proc"
        exit 1
    fi
done

if [ -n "${ONVIF_PID:-}" ]; then
    for _i in $(seq 1 8); do
        if ! kill -0 "$ONVIF_PID" 2>/dev/null; then
            break
        fi
        sleep 1
    done
    if kill -0 "$ONVIF_PID" 2>/dev/null; then
        log_error "ONVIF_PID=$ONVIF_PID still running after stop"
        exit 1
    fi
fi
if [ -n "${DAEMON_PID:-}" ]; then
    for _i in $(seq 1 8); do
        if ! kill -0 "$DAEMON_PID" 2>/dev/null; then
            break
        fi
        sleep 1
    done
    if kill -0 "$DAEMON_PID" 2>/dev/null; then
        log_error "DAEMON_PID=$DAEMON_PID still running after stop"
        exit 1
    fi
fi

log_info "Services stopped"

# ============================================
# 11. Save logs with timestamp
# ============================================
log_step "Saving logs..."

TIMESTAMP=$(date +%Y%m%d_%H%M%S)
LOG_PREFIX="/tmp/video_latency_test_$TIMESTAMP"

cp "$DAEMON_LOG" "${LOG_PREFIX}_daemon.log" 2>/dev/null || true
cp "$ONVIF_LOG" "${LOG_PREFIX}_onvif.log" 2>/dev/null || true

log_info "Logs saved to:"
log_info "  ${LOG_PREFIX}_daemon.log"
log_info "  ${LOG_PREFIX}_onvif.log"

echo ""
echo "=============================================="
echo "  Integration Test Complete"
echo "=============================================="
echo ""
log_info "To analyze logs (paths relative to repo root, or use this script's directory):"
echo "  python3 \"${SCRIPT_DIR}/analyze_test_logs.py\" \\"
echo "    ${LOG_PREFIX}_daemon.log \\"
echo "    ${LOG_PREFIX}_onvif.log"
echo ""
echo "Review VALIDATION_CHECKLIST.md for full pass/fail criteria"
echo ""
