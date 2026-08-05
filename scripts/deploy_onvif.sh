#!/bin/bash

# Deploy onvif-rust binary to device via FTP
# Usage: ./deploy_onvif.sh [device_ip] [username] [password]

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/common.sh
source "${SCRIPT_DIR}/common.sh"
PROJECT_ROOT="${ANYKA_REPO_ROOT}"

# Default values
DEFAULT_IP="192.168.1.100"
DEFAULT_USER="admin"
DEFAULT_PASS="admin"

# Get parameters
DEVICE_IP="${1:-$DEFAULT_IP}"
USERNAME="${2:-$DEFAULT_USER}"
PASSWORD="${3:-$DEFAULT_PASS}"

# Source and destination paths
# Workspace target dir, not per-crate: cross-compile/ is a cargo workspace, and
# onvif-rust/.cargo/config.toml:16-18 pins armv5te-unknown-linux-uclibceabi.
SOURCE_DIR="$PROJECT_ROOT/cross-compile/target/armv5te-unknown-linux-uclibceabi/release"
DEST_DIR="/mnt/anyka_hack/onvif"
BINARY_NAME="onvif-rust"

log_info "=== onvif-rust Deployment Script ==="
log_info "Device IP: $DEVICE_IP"
log_info "Source: $SOURCE_DIR/$BINARY_NAME"
log_info "Destination: $DEST_DIR/$BINARY_NAME"
echo ""

if [ -z "$DEVICE_IP" ] || [ -z "$USERNAME" ] || [ -z "$PASSWORD" ]; then
    log_error "Usage: $0 [device_ip] [username] [password]"
    exit 1
fi

if [ ! -f "$SOURCE_DIR/$BINARY_NAME" ]; then
    log_error "Binary not found: $SOURCE_DIR/$BINARY_NAME"
    log_error "Build first: cd cross-compile/onvif-rust && \$CARGO build --release --target arm-anykav200-crosstool-ng"
    exit 1
fi

anyka_check_commands lftp ftp

UPLOAD_OK=0

if command -v lftp &> /dev/null; then
    log_info "Using lftp..."
    LFTP_OUTPUT=$(lftp -c "
        open ftp://$USERNAME:$PASSWORD@$DEVICE_IP
        mkdir -p $DEST_DIR
        cd $DEST_DIR
        put $SOURCE_DIR/$BINARY_NAME -o $BINARY_NAME
        put $SOURCE_DIR/$BINARY_NAME -o ${BINARY_NAME}.bin
        chmod 755 $BINARY_NAME
        quit
    " 2>&1)
    LFTP_EXIT_CODE=$?

    if [ $LFTP_EXIT_CODE -eq 0 ] && ! echo "$LFTP_OUTPUT" | grep -qi "error\|failed"; then
        log_success "onvif-rust uploaded successfully"
        UPLOAD_OK=1
    else
        log_error "lftp upload failed:"
        echo "$LFTP_OUTPUT" | grep -i "error\|failed" || true
    fi
else
    log_info "Using ftp..."
    FTP_SCRIPT=$(mktemp /tmp/ftp_deploy_onvif_rust.XXXXXX)
    cat > "$FTP_SCRIPT" << EOF
open $DEVICE_IP
user $USERNAME $PASSWORD
binary
mkdir $DEST_DIR
cd $DEST_DIR
put $SOURCE_DIR/$BINARY_NAME $BINARY_NAME
put $SOURCE_DIR/$BINARY_NAME ${BINARY_NAME}.bin
chmod 755 $BINARY_NAME
quit
EOF

    FTP_OUTPUT=$(ftp -n < "$FTP_SCRIPT" 2>&1)
    FTP_EXIT_CODE=$?
    rm -f "$FTP_SCRIPT"

    if [ $FTP_EXIT_CODE -eq 0 ] && ! echo "$FTP_OUTPUT" | grep -qE "553 Error|500 Unknown|550"; then
        log_success "onvif-rust uploaded successfully"
        UPLOAD_OK=1
    else
        log_error "ftp upload failed:"
        echo "$FTP_OUTPUT" | grep -E "(553|500|550|Error)" || true
    fi
fi

echo ""
if [ $UPLOAD_OK -eq 1 ]; then
    log_success "Deployment complete. Binary available at $DEST_DIR/$BINARY_NAME"
    echo ""
    echo "To run on device:"
    echo "  ./run_onvif.sh $DEVICE_IP $USERNAME $PASSWORD"
else
    log_error "Deployment failed"
    exit 1
fi
