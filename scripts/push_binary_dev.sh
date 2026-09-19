#!/bin/bash

# STAGE:   push — sends one binary to the device over FTP. Compiles nothing.
# UNIT:    a single binary (onvif-rust)
# USE FOR: DEV ITERATION ONLY. Not an upgrade path: no versioning, no A/B slot,
#          no trial window, no rollback. To ship a change use
#          ./scripts/build_bundle.sh then ./scripts/push_bundle.sh.
# NEXT:    ./scripts/run_binary_dev.sh <ip> <user>
#
# Usage: ./push_binary_dev.sh [device_ip] [username] [password]
#        Prefer exporting ANYKA_FTP_PASS over passing the password positionally;
#        an argv password is visible in `ps` and lands in shell history.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/common.sh
source "${SCRIPT_DIR}/common.sh"

# Anyone arriving here from muscle memory has not necessarily read the header.
log_warn "push_binary_dev.sh is DEV ONLY — no versioning, no A/B slot, no rollback."
log_warn "To ship a change: ./scripts/build_bundle.sh && ./scripts/push_bundle.sh"
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
    log_error "Build first: cd cross-compile && \$CARGO build --release --target armv5te-unknown-linux-uclibceabi -p onvif-rust"
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

# Temp-name upload + rename avoids ETXTBSY: STOR over a locked (running) binary
# returns 550, but writing to a fresh name then RENAME over it succeeds because
# the rename only replaces the directory entry, not the in-use inode.
TEMP_BINARY="${BINARY_NAME}.bin.new"

# Each transfer runs as its own step so rename and chmod can be judged
# separately: a failed rename means the binary never landed and is fatal, while
# the camera's FTP server answers `500 Unknown command` to chmod and the upload
# is still fine.
#
# Every capture uses `if ! VAR=$(...)`. A bare `VAR=$(cmd)` assignment takes the
# command's exit status, so under `set -e` a failed transfer would kill the
# script at the assignment and the diagnostics below would never print. Inside
# an `if` condition errexit is suspended, so the failure is ours to report.
ftp_fatal() {
    log_error "$1"
    shift
    printf '%s\n' "$@" | grep -iE "550|553|500|error|failed" || printf '%s\n' "$@"
    log_error "If the binary is running, stop it via telnet :24: killall onvif-rust.bin"
    exit 1
}

if command -v lftp &> /dev/null; then
    log_info "Using lftp..."

    log_info "Step 1/3: connect + mkdir + upload to temp name"
    if ! LFTP_OUTPUT=$(lftp -c "
        set xfer:temp-extension .part
        open ftp://$USERNAME:$PASSWORD@$DEVICE_IP
        mkdir -p $DEST_DIR
        cd $DEST_DIR
        put $SOURCE_DIR/$BINARY_NAME -o $TEMP_BINARY
        quit
    " 2>&1) || printf '%s' "$LFTP_OUTPUT" | grep -qiE "550|553|error|failed"; then
        ftp_fatal "lftp STOR to temp name failed (550 = target locked or no perm):" "$LFTP_OUTPUT"
    fi

    log_info "Step 2/3: rename temp -> final (fatal if this fails)"
    if ! LFTP_OUTPUT=$(lftp -c "
        open ftp://$USERNAME:$PASSWORD@$DEVICE_IP
        cd $DEST_DIR
        rename -f $TEMP_BINARY $DEST_BINARY
        quit
    " 2>&1) || printf '%s' "$LFTP_OUTPUT" | grep -qiE "550|553|500|error|failed"; then
        ftp_fatal "lftp rename failed; ${TEMP_BINARY} may be left behind:" "$LFTP_OUTPUT"
    fi

    log_info "Step 3/3: chmod 755 (non-fatal)"
    if ! LFTP_OUTPUT=$(lftp -c "
        open ftp://$USERNAME:$PASSWORD@$DEVICE_IP
        cd $DEST_DIR
        chmod 755 $DEST_BINARY
        quit
    " 2>&1) || printf '%s' "$LFTP_OUTPUT" | grep -qiE "550|553|500|error|failed"; then
        log_warn "chmod was rejected; the binary is in place but may not be executable"
        log_warn "fix over telnet :24 if needed: chmod 755 ${DEST_DIR}/${DEST_BINARY}"
    fi

    log_success "onvif-rust uploaded and renamed into place"
    UPLOAD_OK=1
else
    log_info "Using ftp..."
    FTP_SCRIPT=$(mktemp /tmp/ftp_push_binary_dev.XXXXXX)
    # chmod is deliberately not in this batch: the camera's FTP server answers
    # `500 Unknown command`, and batching it here would make a cosmetic failure
    # indistinguishable from a failed upload or rename.
    cat > "$FTP_SCRIPT" << EOF
open $DEVICE_IP
user $USERNAME $PASSWORD
binary
mkdir $DEST_DIR
cd $DEST_DIR
put $SOURCE_DIR/$BINARY_NAME $TEMP_BINARY
rename -f $TEMP_BINARY $DEST_BINARY
quit
EOF

    log_info "Step 1/2: connect + upload temp + rename (fatal if either fails)"
    if ! FTP_OUTPUT=$(ftp -n < "$FTP_SCRIPT" 2>&1); then
        rm -f "$FTP_SCRIPT"
        ftp_fatal "ftp upload/rename failed:" "$FTP_OUTPUT"
    fi
    rm -f "$FTP_SCRIPT"

    if printf '%s' "$FTP_OUTPUT" | grep -qE "553|550"; then
        ftp_fatal "ftp upload/rename failed:" "$FTP_OUTPUT"
    fi

    log_info "Step 2/2: chmod 755 (non-fatal)"
    if ! printf 'open %s\nuser %s %s\ncd %s\nchmod 755 %s\nquit\n' \
            "$DEVICE_IP" "$USERNAME" "$PASSWORD" "$DEST_DIR" "$DEST_BINARY" \
            | ftp -n > /dev/null 2>&1; then
        log_warn "chmod was rejected; the binary is in place but may not be executable"
        log_warn "fix over telnet :24 if needed: chmod 755 ${DEST_DIR}/${DEST_BINARY}"
    fi

    log_success "onvif-rust uploaded and renamed into place"
    UPLOAD_OK=1
fi

# The FTP clients report success even when STOR failed, and overwriting a binary
# that is currently executing fails with ETXTBSY — which is the normal state here,
# since the supervisor keeps onvif-rust running. Compare sizes to catch it.
# Query the remote size with the same client that did the upload, so hosts with
# only lftp (or only ftp) verify without demanding the other binary.
if [ $UPLOAD_OK -eq 1 ]; then
    LOCAL_SIZE=$(stat -c %s "$SOURCE_DIR/$BINARY_NAME")

    if command -v lftp &>/dev/null; then
        REMOTE_SIZE=$(lftp -c "
            open ftp://$USERNAME:$PASSWORD@$DEVICE_IP
            cd $DEST_DIR
            du -b -s $DEST_BINARY
            quit
        " 2>/dev/null | awk '{print $1}' | tail -1)
    else
        REMOTE_SIZE=$(printf 'open %s\nuser %s %s\nbinary\ncd %s\nls %s\nquit\n' \
            "$DEVICE_IP" "$USERNAME" "$PASSWORD" "$DEST_DIR" "$DEST_BINARY" \
            | ftp -n 2>/dev/null | awk -v f="$DEST_BINARY" '$NF ~ f {print $5}' | tail -1)
    fi

    if [ -n "$REMOTE_SIZE" ] && [ "$REMOTE_SIZE" != "$LOCAL_SIZE" ]; then
        log_error "Upload did not land: remote is $REMOTE_SIZE bytes, local is $LOCAL_SIZE."
        log_error "The binary is probably running (ETXTBSY). Stop it first:"
        log_error "  killall vendor-daemon.bin onvif-rust.bin   # via telnet :24, then re-run"
        UPLOAD_OK=0
    elif [ -z "$REMOTE_SIZE" ]; then
        log_error "Could not verify remote size (listing unavailable). Treating upload as failed."
        UPLOAD_OK=0
    fi
fi

echo ""
if [ $UPLOAD_OK -eq 1 ]; then
    log_success "Deployment complete. Binary available at $DEST_DIR/$DEST_BINARY"
    echo ""
    echo "To run on device (password from \$ANYKA_FTP_PASS):"
    echo "  ./scripts/run_binary_dev.sh $DEVICE_IP $USERNAME"
else
    log_error "Deployment failed"
    exit 1
fi
