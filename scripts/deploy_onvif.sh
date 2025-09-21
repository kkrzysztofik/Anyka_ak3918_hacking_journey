#!/bin/bash

# Deploy ONVIF daemon binaries to device via FTP
# Usage: ./deploy_onvif.sh [device_ip] [username] [password]
#
# This script checks which ONVIF binaries are available and uploads only those that exist.
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
SOURCE_DIR="../cross-compile/onvif/out"
DEST_DIR="/mnt/anyka_hack/onvif/"

# Binary names
BINARY_RELEASE="onvifd"
BINARY_DEBUG="onvifd_debug"

echo "=== ONVIF Daemon Deployment Script ==="
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

# Check if source directory exists
if [ ! -d "$SOURCE_DIR" ]; then
    echo "ERROR: Source directory not found: $SOURCE_DIR"
    echo "Please ensure the ONVIF project is built first"
    exit 1
fi

# Check if ftp command is available
if ! command -v ftp &> /dev/null; then
    echo "ERROR: ftp command not found. Please install ftp client."
    echo "Install with: sudo apt-get install ftp"
    exit 1
fi

# Check if lftp is available (preferred for better error handling)
if command -v lftp &> /dev/null; then
    USE_LFTP=true
    echo "Using lftp for more reliable transfers"
else
    USE_LFTP=false
    echo "Using standard ftp (consider installing lftp for better reliability)"
fi

# Check which binaries are available
AVAILABLE_BINARIES=()
UPLOAD_COUNT=0

if [ -f "$SOURCE_DIR/$BINARY_RELEASE" ]; then
    AVAILABLE_BINARIES+=("$BINARY_RELEASE")
    echo "✓ Found release binary: $BINARY_RELEASE"
else
    echo "⚠ Release binary not found: $BINARY_RELEASE"
fi

if [ -f "$SOURCE_DIR/$BINARY_DEBUG" ]; then
    AVAILABLE_BINARIES+=("$BINARY_DEBUG")
    echo "✓ Found debug binary: $BINARY_DEBUG"
else
    echo "⚠ Debug binary not found: $BINARY_DEBUG"
fi

# Check if any binaries are available
if [ ${#AVAILABLE_BINARIES[@]} -eq 0 ]; then
    echo "ERROR: No ONVIF binaries found in $SOURCE_DIR"
    echo "Please build the project first using:"
    echo "  make -C cross-compile/onvif"
    exit 1
fi

echo ""
echo "Deploying ${#AVAILABLE_BINARIES[@]} available binary(ies) to device..."

# Upload each available binary
for binary in "${AVAILABLE_BINARIES[@]}"; do
    echo "Uploading $binary..."

    # Double-check that the binary file exists
    if [ ! -f "$SOURCE_DIR/$binary" ]; then
        echo "✗ Binary file not found: $SOURCE_DIR/$binary"
        continue
    fi

    if [ "$USE_LFTP" = true ]; then
        # Use lftp for more reliable transfers
        LFTP_OUTPUT=$(lftp -c "
            open ftp://$USERNAME:$PASSWORD@$DEVICE_IP
            mkdir -p $DEST_DIR
            cd $DEST_DIR
            put $SOURCE_DIR/$binary -o $binary
            chmod 755 $binary
            quit
        " 2>&1)
        LFTP_EXIT_CODE=$?

        if [ $LFTP_EXIT_CODE -eq 0 ] && ! echo "$LFTP_OUTPUT" | grep -q "error\|Error\|ERROR\|failed\|Failed\|FAILED"; then
            echo "✓ $binary uploaded successfully"
            ((UPLOAD_COUNT++))
        else
            echo "✗ Failed to upload $binary"
            echo "LFTP Error Details:"
            echo "$LFTP_OUTPUT" | grep -i "error\|failed" || echo "  (No specific error message found)"
        fi
    else
        # Use standard ftp
        cat > /tmp/ftp_upload_$binary.txt << EOF
open $DEVICE_IP
user $USERNAME $PASSWORD
binary
mkdir $DEST_DIR
cd $DEST_DIR
put $SOURCE_DIR/$binary $binary
chmod 755 $binary
quit
EOF

        # Capture FTP output to detect errors
        FTP_OUTPUT=$(ftp -n < /tmp/ftp_upload_$binary.txt 2>&1)
        FTP_EXIT_CODE=$?

        # Check for common FTP error patterns
        if [ $FTP_EXIT_CODE -eq 0 ] && ! echo "$FTP_OUTPUT" | grep -q "553 Error\|500 Unknown command\|550.*No such file\|550.*Permission denied"; then
            echo "✓ $binary uploaded successfully"
            ((UPLOAD_COUNT++))
        else
            echo "✗ Failed to upload $binary"
            echo "FTP Error Details:"
            echo "$FTP_OUTPUT" | grep -E "(553|500|550|Error|Unknown command)" || echo "  (No specific error message found)"
        fi

        # Clean up temporary FTP script
        rm -f /tmp/ftp_upload_$binary.txt
    fi
done

echo ""
echo "=== Deployment Complete ==="
echo "Successfully uploaded $UPLOAD_COUNT out of ${#AVAILABLE_BINARIES[@]} binary(ies)"

if [ $UPLOAD_COUNT -gt 0 ]; then
    echo "Binaries are now available on device at:"
    for binary in "${AVAILABLE_BINARIES[@]}"; do
        echo "  - $DEST_DIR/$binary"
    done
    echo ""
    echo "To execute the daemon, use:"
    if [ -f "$SOURCE_DIR/$BINARY_DEBUG" ]; then
        echo "  ./run_onvif.sh $DEVICE_IP $USERNAME $PASSWORD debug"
    fi
    if [ -f "$SOURCE_DIR/$BINARY_RELEASE" ]; then
        echo "  ./run_onvif.sh $DEVICE_IP $USERNAME $PASSWORD release"
    fi
else
    echo "No binaries were successfully uploaded"
    exit 1
fi
