#!/bin/bash

# Execute onvif-rust on device via telnet
# Usage: ./run_onvif.sh [device_ip] [username] [password] [release|debug]
#
# The mode selects the config file (config.toml vs config_debug.toml).
# Device output is saved to the debugging/logs/ directory for analysis.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/common.sh
source "${SCRIPT_DIR}/common.sh"
PROJECT_ROOT="${ANYKA_REPO_ROOT}"

# Default values
DEFAULT_IP="192.168.1.100"
# Only echoed into the suggested deploy_onvif.sh command — telnet on :24 needs
# no login. Password stays out of the repo; see ANYKA_FTP_PASS.
DEFAULT_USER="root"
DEFAULT_PASS="${ANYKA_FTP_PASS:-}"
DEFAULT_MODE="release"

# Get parameters
DEVICE_IP="${1:-$DEFAULT_IP}"
USERNAME="${2:-$DEFAULT_USER}"
PASSWORD="${3:-$DEFAULT_PASS}"
MODE="${4:-$DEFAULT_MODE}"

# On-device paths
ONVIF_DIR="/mnt/anyka_hack/onvif"
BINARY_NAME="onvif-rust"
FULL_BINARY_PATH="$ONVIF_DIR/$BINARY_NAME"

if [ "$MODE" = "debug" ]; then
    CONFIG_FILE="$ONVIF_DIR/config_debug.toml"
else
    CONFIG_FILE="$ONVIF_DIR/config.toml"
fi

log_info "=== onvif-rust Execution Script ==="
log_info "Device IP:  $DEVICE_IP"
log_info "Mode:       $MODE (config: $CONFIG_FILE)"
log_info "Log output: $PROJECT_ROOT/debugging/logs/"
echo ""

if [ -z "$DEVICE_IP" ] || [ -z "$USERNAME" ]; then
    log_error "Usage: $0 [device_ip] [username] [password] [release|debug]"
    exit 1
fi

if [ "$MODE" != "release" ] && [ "$MODE" != "debug" ]; then
    log_error "Invalid mode '$MODE'. Use 'release' or 'debug'."
    exit 1
fi

anyka_check_commands telnet

log_info "Connecting to device and starting $BINARY_NAME..."

TELNET_SCRIPT=$(mktemp /tmp/telnet_run_onvif_rust.XXXXXX)
cat > "$TELNET_SCRIPT" << EOF
echo "Stopping any existing onvif-rust process..."
killall onvif-rust onvif-rust.bin 2>/dev/null || true
sleep 2

if [ ! -f "$FULL_BINARY_PATH" ]; then
    echo "ERROR: $FULL_BINARY_PATH not found. Deploy first: ./deploy_onvif.sh $DEVICE_IP $USERNAME"
    exit 1
fi

chmod +x "$FULL_BINARY_PATH"

# Core dump setup
ulimit -c unlimited
mkdir -p /mnt/coredumps 2>/dev/null || true
echo '/mnt/coredumps/core.%e.%p.%t' > /proc/sys/kernel/core_pattern 2>/dev/null || true

echo "Memory status:"
free -m

echo "Starting $BINARY_NAME ($MODE mode)..."
cd $ONVIF_DIR
./$BINARY_NAME $CONFIG_FILE
EOF

TELNET_OUTPUT=$(mktemp /tmp/telnet_output_onvif_rust.XXXXXX)

if timeout 30 telnet "$DEVICE_IP" 24 < "$TELNET_SCRIPT" > "$TELNET_OUTPUT" 2>&1; then
    log_success "Commands executed successfully"
else
    log_warn "Telnet exited (may be normal if daemon runs in foreground)"
fi

rm -f "$TELNET_SCRIPT"

if [ -s "$TELNET_OUTPUT" ]; then
    echo ""
    echo "=== Device Output ==="
    cat "$TELNET_OUTPUT"
    echo "=== End Device Output ==="

    LOG_DIR="$PROJECT_ROOT/debugging/logs"
    mkdir -p "$LOG_DIR"
    LOG_FILE="$LOG_DIR/onvif_execution_$(date +%Y%m%d_%H%M%S).log"
    cp "$TELNET_OUTPUT" "$LOG_FILE"
    log_info "Log saved to: $LOG_FILE"
fi

rm -f "$TELNET_OUTPUT"

echo ""
log_info "onvif-rust should now be running on device."
echo ""
echo "Useful follow-up commands:"
echo "  Check running:   telnet $DEVICE_IP 24  →  ps | grep onvif-rust"
echo "  Stop:            telnet $DEVICE_IP 24  →  killall onvif-rust"
echo "  Collect coredump: ./debugging/collect_coredump.sh $DEVICE_IP $USERNAME $PASSWORD"
echo "  View logs:        ls $PROJECT_ROOT/debugging/logs/"
