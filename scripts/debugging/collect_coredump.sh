#!/bin/bash

# Collect onvif-rust core dumps from device via FTP
# Usage: ./collect_coredump.sh [device_ip] [username] [password]
#
# Searches /mnt/logs/ (primary) and /mnt/anyka_hack/onvif/ (fallback) for core.* files.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/common.sh
source "${SCRIPT_DIR}/../../scripts/common.sh"
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
# Primary: /mnt/coredumps (kernel core_pattern target for all processes)
# Fallback: /mnt/logs (old core_pattern location), /mnt/anyka_hack/onvif (legacy cwd)
SOURCE_DIRS=("/mnt/coredumps" "/mnt/logs" "/mnt/anyka_hack/onvif")
DEST_DIR="$PROJECT_ROOT/debugging/coredump"

# Core dump patterns
CORE_PATTERNS=("core.*" "core.*.*" "*.core")

log_info "=== onvif-rust Core Dump Collection ==="
log_info "Device IP:            $DEVICE_IP"
log_info "Search directories:   ${SOURCE_DIRS[*]}"
log_info "Local destination:    $DEST_DIR"
echo ""

if [ -z "$DEVICE_IP" ] || [ -z "$USERNAME" ] || [ -z "$PASSWORD" ]; then
    log_error "Usage: $0 [device_ip] [username] [password]"
    exit 1
fi

anyka_check_commands ftp

mkdir -p "$DEST_DIR"

CORE_FILES=""

for SOURCE_DIR in "${SOURCE_DIRS[@]}"; do
    log_info "Searching $SOURCE_DIR on device..."
    FTP_LIST=$(mktemp /tmp/ftp_list_cores.XXXXXX)
    cat > "$FTP_LIST" << EOF
open $DEVICE_IP
user $USERNAME $PASSWORD
binary
cd $SOURCE_DIR
ls -la
quit
EOF

    RAW=$(ftp -n < "$FTP_LIST" 2>/dev/null || true)
    rm -f "$FTP_LIST"

    FOUND=$(echo "$RAW" | grep -E "core\." | awk '{print $9}' | grep -v "^$" | \
            awk -v dir="$SOURCE_DIR" '{print dir "/" $0}' || true)

    if [ -n "$FOUND" ]; then
        CORE_FILES="${CORE_FILES}${FOUND}\n"
        log_info "Found in $SOURCE_DIR:"
        echo "$FOUND"
    else
        log_info "  (none)"
    fi
done

if [ -z "$CORE_FILES" ]; then
    log_warn "No core dumps found on device in any search directory."
    exit 0
fi
echo ""

# Download each core dump
DOWNLOAD_COUNT=0
TOTAL_SIZE=0

while IFS= read -r core_path; do
    [ -z "$core_path" ] && continue
    core_dir="$(dirname "$core_path")"
    core_file="$(basename "$core_path")"
    echo "Downloading $core_path..."

    FTP_DL=$(mktemp /tmp/ftp_download_onvif_rust.XXXXXX)
    cat > "$FTP_DL" << EOF
open $DEVICE_IP
user $USERNAME $PASSWORD
binary
cd $core_dir
get $core_file $DEST_DIR/$core_file
quit
EOF

        if ftp -n < "$FTP_DL"; then
            log_success "Downloaded $core_file"
            DOWNLOAD_COUNT=$((DOWNLOAD_COUNT + 1))

            # Get file size and add to total
            if [ -f "$DEST_DIR/$core_file" ]; then
                FILE_SIZE=$(ls -lh "$DEST_DIR/$core_file" | awk '{print $5}')
                FILE_SIZE_BYTES=$(stat -c%s "$DEST_DIR/$core_file" 2>/dev/null || echo "0")
                TOTAL_SIZE=$((TOTAL_SIZE + FILE_SIZE_BYTES))
                echo "  Size: $FILE_SIZE"
            fi
        else
            echo "✗ Failed to download $core_file"
        fi

        rm -f "$FTP_DL"
done < <(printf "%b" "$CORE_FILES")

echo ""
log_info "=== Collection Complete ==="
log_info "Downloaded $DOWNLOAD_COUNT core dump(s) to $DEST_DIR"
echo ""

if [ -d "$DEST_DIR" ] && ls "$DEST_DIR"/core.* &>/dev/null 2>&1; then
    echo "Collected core dumps:"
    ls -lh "$DEST_DIR"/core.* 2>/dev/null || true
    echo ""
    BINARY="$PROJECT_ROOT/cross-compile/onvif-rust/target/arm-anykav200-crosstool-ng/release/onvif-rust"
    echo "To analyze:"
    echo "  $SCRIPT_DIR/run_gdb_multiarch_analysis.sh <corefile> onvif-rust"
    echo "  or manually:"
    echo "  gdb-multiarch $BINARY $DEST_DIR/core.<filename>"
else
    log_warn "No core dumps were collected."
fi
