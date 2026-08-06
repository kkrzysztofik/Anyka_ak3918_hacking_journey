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
# The camera's FTP account is root, not admin. The password is a secret: pass it
# as $3 or export ANYKA_FTP_PASS. Never hardcode it here.
DEFAULT_USER="root"
DEFAULT_PASS="${ANYKA_FTP_PASS:-}"

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
# Deploy target is the .bin only. `$DEST_DIR/onvif-rust` is a committed 336-byte
# launcher shell script (it execs onvif-rust.bin so RPATH resolves); uploading
# the binary over that name destroys the launcher.
DEST_BINARY="${BINARY_NAME}.bin"

log_info "=== onvif-rust Deployment Script ==="
log_info "Device IP: $DEVICE_IP"
log_info "Source: $SOURCE_DIR/$BINARY_NAME"
log_info "Destination: $DEST_DIR/$DEST_BINARY"
echo ""

if [ -z "$DEVICE_IP" ] || [ -z "$USERNAME" ] || [ -z "$PASSWORD" ]; then
    log_error "Usage: $0 [device_ip] [username] [password]"
    log_error "Password may also come from \$ANYKA_FTP_PASS."
    exit 1
fi

if [ ! -f "$SOURCE_DIR/$BINARY_NAME" ]; then
    log_error "Binary not found: $SOURCE_DIR/$BINARY_NAME"
    log_error "Build first: cd cross-compile/onvif-rust && \$CARGO build --release --target arm-anykav200-crosstool-ng"
    exit 1
fi

# Either client will do — the upload below prefers lftp and falls back to ftp.
# anyka_check_commands requires *all* of its arguments, which is wrong here.
if ! command -v lftp &>/dev/null && ! command -v ftp &>/dev/null; then
    log_error "Missing dependencies: need lftp or ftp"
    log_info "Install with: sudo apt-get install -y lftp"
    exit 1
fi

UPLOAD_OK=0

if command -v lftp &> /dev/null; then
    log_info "Using lftp..."
    LFTP_OUTPUT=$(lftp -c "
        open ftp://$USERNAME:$PASSWORD@$DEVICE_IP
        mkdir -p $DEST_DIR
        cd $DEST_DIR
        put $SOURCE_DIR/$BINARY_NAME -o $DEST_BINARY
        chmod 755 $DEST_BINARY
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
put $SOURCE_DIR/$BINARY_NAME $DEST_BINARY
chmod 755 $DEST_BINARY
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

# The FTP clients report success even when STOR failed, and overwriting a binary
# that is currently executing fails with ETXTBSY — which is the normal state here,
# since the supervisor keeps onvif-rust running. Compare sizes to catch it.
if [ $UPLOAD_OK -eq 1 ]; then
    LOCAL_SIZE=$(stat -c %s "$SOURCE_DIR/$BINARY_NAME")
    REMOTE_SIZE=$(printf 'open %s\nuser %s %s\nbinary\ncd %s\nls %s\nquit\n' \
        "$DEVICE_IP" "$USERNAME" "$PASSWORD" "$DEST_DIR" "$DEST_BINARY" \
        | ftp -n 2>/dev/null | awk -v f="$DEST_BINARY" '$NF ~ f {print $5}' | tail -1)
    if [ -n "$REMOTE_SIZE" ] && [ "$REMOTE_SIZE" != "$LOCAL_SIZE" ]; then
        log_error "Upload did not land: remote is $REMOTE_SIZE bytes, local is $LOCAL_SIZE."
        log_error "The binary is probably running (ETXTBSY). Stop it first:"
        log_error "  killall vendor-daemon.bin onvif-rust.bin   # via telnet :24, then re-run"
        UPLOAD_OK=0
    fi
fi

echo ""
if [ $UPLOAD_OK -eq 1 ]; then
    log_success "Deployment complete. Binary available at $DEST_DIR/$DEST_BINARY"
    echo ""
    echo "To run on device (password from \$ANYKA_FTP_PASS):"
    echo "  ./run_onvif.sh $DEVICE_IP $USERNAME"
else
    log_error "Deployment failed"
    exit 1
fi
