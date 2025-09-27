#!/bin/bash

# Collect core dumps from device via FTP
# Usage: ./collect_coredump.sh [device_ip] [username] [password]
#
# This script downloads core dump files from the device for debugging purposes.
# It uses native cross-compilation tools as per project guidelines.

set -e

# Default values
DEFAULT_IP="192.168.1.100"
DEFAULT_USER="admin"
DEFAULT_PASS="admin"

# Get parameters
DEVICE_IP="${1:-$DEFAULT_IP}"
USERNAME="${2:-$DEFAULT_USER}"
PASSWORD="${3:-$DEFAULT_PASS}"

# Source and destination paths
SOURCE_DIR="/mnt/anyka_hack/onvif/"
DEST_DIR="../debugging/coredump"

# Core dump patterns
CORE_PATTERNS=("core.*" "core.*.*" "*.core")

echo "=== Core Dump Collection Script ==="
echo "Device IP: $DEVICE_IP"
echo "Username: $USERNAME"
echo "Source Directory: $SOURCE_DIR"
echo "Destination Directory: $DEST_DIR"
echo ""

# Validate input parameters
if [ -z "$DEVICE_IP" ] || [ -z "$USERNAME" ] || [ -z "$PASSWORD" ]; then
    echo "ERROR: Invalid parameters provided"
    echo "Usage: $0 [device_ip] [username] [password]"
    exit 1
fi

# Check if ftp command is available
if ! command -v ftp &> /dev/null; then
    echo "ERROR: ftp command not found. Please install ftp client."
    echo "Install with: sudo apt-get install ftp"
    exit 1
fi

# Create destination directory if it doesn't exist
mkdir -p "$DEST_DIR"

# Create FTP script to list core dumps
cat > /tmp/ftp_list_cores.txt << EOF
open $DEVICE_IP
user $USERNAME $PASSWORD
binary
cd $SOURCE_DIR
ls -la
quit
EOF

echo "Searching for core dumps on device..."

# List core dumps on device with better error handling
# First try to get the raw listing to debug
echo "Raw FTP listing:"
ftp -n < /tmp/ftp_list_cores.txt 2>/dev/null || echo "FTP command failed"

echo ""
echo "Processing core dump files..."

# Improved pattern matching for core dumps
CORE_FILES=$(ftp -n < /tmp/ftp_list_cores.txt 2>/dev/null | grep -E "core\." | awk '{print $9}' | grep -v "^$" || true)

# If the above doesn't work, try alternative patterns
if [ -z "$CORE_FILES" ]; then
    echo "Trying alternative pattern matching..."
    CORE_FILES=$(ftp -n < /tmp/ftp_list_cores.txt 2>/dev/null | grep -E "core" | awk '{print $9}' | grep -v "^$" || true)
fi

# If still no files, try without awk (in case of different ls output format)
if [ -z "$CORE_FILES" ]; then
    echo "Trying without awk processing..."
    CORE_FILES=$(ftp -n < /tmp/ftp_list_cores.txt 2>/dev/null | grep -E "core" | sed 's/.*[[:space:]]//' | grep -v "^$" || true)
fi

if [ -z "$CORE_FILES" ]; then
    echo "No core dumps found on device"
    echo "Available files in $SOURCE_DIR:"
    ftp -n < /tmp/ftp_list_cores.txt 2>/dev/null | grep -v "^total" | grep -v "^$" || true
    rm -f /tmp/ftp_list_cores.txt
    exit 0
fi

echo "Found core dumps:"
echo "$CORE_FILES"
echo ""

# Download each core dump
DOWNLOAD_COUNT=0
TOTAL_SIZE=0

for core_file in $CORE_FILES; do
    if [ -n "$core_file" ]; then
        echo "Downloading $core_file..."

        # Create FTP script for downloading
        cat > /tmp/ftp_download_$core_file.txt << EOF
open $DEVICE_IP
user $USERNAME $PASSWORD
binary
cd $SOURCE_DIR
get $core_file $DEST_DIR/$core_file
quit
EOF

        if ftp -n < /tmp/ftp_download_$core_file.txt; then
            echo "✓ Downloaded $core_file"
            ((DOWNLOAD_COUNT++))

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

        # Clean up temporary FTP script
        rm -f /tmp/ftp_download_$core_file.txt
    fi
done

# Clean up temporary files
rm -f /tmp/ftp_list_cores.txt

echo ""
echo "=== Collection Complete ==="
echo "Successfully downloaded $DOWNLOAD_COUNT core dump(s)"
if [ $TOTAL_SIZE -gt 0 ]; then
    TOTAL_SIZE_MB=$((TOTAL_SIZE / 1024 / 1024))
    echo "Total size: ${TOTAL_SIZE_MB}MB"
fi
echo "Core dumps saved to: $DEST_DIR"
echo ""

# List collected files
if [ -d "$DEST_DIR" ] && [ "$(ls -A $DEST_DIR 2>/dev/null)" ]; then
    echo "Collected core dumps:"
    ls -lh "$DEST_DIR"/core* 2>/dev/null || true
    echo ""
    echo "To analyze core dumps with GDB:"
    echo "  gdb ../cross-compile/onvif/out/onvifd_debug $DEST_DIR/core.*"
    echo "  or"
    echo "  gdb ../cross-compile/onvif/out/onvifd $DEST_DIR/core.*"
    echo ""
    echo "GDB commands for analysis:"
    echo "  (gdb) bt          # Show backtrace"
    echo "  (gdb) info threads # Show all threads"
    echo "  (gdb) thread 1    # Switch to thread 1"
    echo "  (gdb) bt          # Show backtrace for thread 1"
    echo "  (gdb) quit        # Exit GDB"
else
    echo "No core dumps were collected"
fi
