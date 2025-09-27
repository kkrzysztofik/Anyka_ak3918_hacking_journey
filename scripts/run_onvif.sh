#!/bin/bash

# Execute ONVIF daemon on device via telnet
# Usage: ./run_onvif.sh [device_ip] [username] [password] [release|debug]
#
# This script connects to the device via telnet and executes the ONVIF daemon.
# It uses native cross-compilation tools as per project guidelines.
# Device output is automatically saved to ../debugging/logs/ for debugging.

set -e

# Default values
DEFAULT_IP="192.168.1.100"
DEFAULT_USER="admin"
DEFAULT_PASS="admin"
DEFAULT_MODE="debug"

# Get parameters
DEVICE_IP="${1:-$DEFAULT_IP}"
USERNAME="${2:-$DEFAULT_USER}"
PASSWORD="${3:-$DEFAULT_PASS}"
MODE="${4:-$DEFAULT_MODE}"

# Binary paths on device
BINARY_PATH="/mnt/anyka_hack/onvif/"
BINARY_RELEASE="onvifd"
BINARY_DEBUG="onvifd_debug"

echo "=== ONVIF Daemon Execution Script ==="
echo "Device IP: $DEVICE_IP"
echo "Username: $USERNAME"
echo "Mode: $MODE"
echo "Log Output: ../debugging/logs/"
echo ""

# Validate input parameters
if [ -z "$DEVICE_IP" ] || [ -z "$USERNAME" ] || [ -z "$PASSWORD" ]; then
    echo "ERROR: Invalid parameters provided"
    echo "Usage: $0 [device_ip] [username] [password] [release|debug]"
    echo ""
    echo "Device output is automatically saved to ../debugging/logs/"
    echo ""
    echo "Examples:"
    echo "  $0                                    # Use defaults (debug mode)"
    echo "  $0 192.168.1.100 admin admin release # Run release mode"
    echo "  $0 192.168.1.100 admin admin debug   # Run debug mode"
    exit 1
fi

# Validate mode
if [ "$MODE" != "release" ] && [ "$MODE" != "debug" ]; then
    echo "ERROR: Invalid mode '$MODE'. Use 'release' or 'debug'"
    echo "Usage: $0 [device_ip] [username] [password] [release|debug]"
    exit 1
fi

# Select binary based on mode
if [ "$MODE" = "release" ]; then
    BINARY_NAME="$BINARY_RELEASE"
else
    BINARY_NAME="$BINARY_DEBUG"
fi

FULL_BINARY_PATH="$BINARY_PATH/$BINARY_NAME"

# Check if telnet command is available
if ! command -v telnet &> /dev/null; then
    echo "ERROR: telnet command not found. Please install telnet client."
    echo "Install with: sudo apt-get install telnet"
    exit 1
fi

echo "Connecting to device and executing $BINARY_NAME..."

# Create telnet script with improved error handling
cat > /tmp/telnet_run_onvif.txt << EOF
# Kill any existing onvifd processes
echo "Stopping existing ONVIF daemon processes..."
killall onvifd 2>/dev/null || true
killall onvifd_debug 2>/dev/null || true

# Wait for processes to terminate
sleep 2

# Check if binary exists
if [ ! -f "$FULL_BINARY_PATH" ]; then
    echo "ERROR: Binary $FULL_BINARY_PATH not found on device"
    echo "Please deploy the binary first using:"
    echo "  ./deploy_onvif.sh $DEVICE_IP $USERNAME $PASSWORD"
    exit 1
fi

# Make sure binary is executable
chmod +x "$FULL_BINARY_PATH"

# Set up core dump handling
echo "Configuring core dump handling..."
ulimit -c unlimited
echo "/mnt/anyka_hack/onvif/core.%e.%p" > /proc/sys/kernel/core_pattern

# Check available memory
echo "System memory status:"
free -m

# Start the daemon
echo "Starting $BINARY_NAME in $MODE mode..."
cd $BINARY_PATH
./$BINARY_NAME
EOF

# Execute via telnet with timeout and capture output
echo "Executing commands on device..."
TELNET_OUTPUT="/tmp/telnet_output_$$.txt"

# Capture telnet output to file
if timeout 30 telnet $DEVICE_IP 24 < /tmp/telnet_run_onvif.txt > "$TELNET_OUTPUT" 2>&1; then
    echo "✓ Commands executed successfully"
    TELNET_EXIT_CODE=0
else
    echo "✗ Failed to execute commands on device or connection timed out"
    echo "This may be normal if the daemon is running in foreground mode"
    TELNET_EXIT_CODE=$?
fi

# Display captured output
if [ -f "$TELNET_OUTPUT" ] && [ -s "$TELNET_OUTPUT" ]; then
    echo ""
    echo "=== Device Output ==="
    cat "$TELNET_OUTPUT"
    echo ""
    echo "=== End Device Output ==="

    # Always save to log file
    LOG_DIR="../debugging/logs"
    mkdir -p "$LOG_DIR"
    TIMESTAMP=$(date +"%Y%m%d_%H%M%S")
    LOG_FILE="$LOG_DIR/onvif_execution_${TIMESTAMP}.log"

    echo "Saving device output to: $LOG_FILE"
    cp "$TELNET_OUTPUT" "$LOG_FILE"
    echo "Log saved successfully"
else
    echo "No output captured from device"
fi

# Clean up output file
rm -f "$TELNET_OUTPUT"

# Clean up temporary file
rm -f /tmp/telnet_run_onvif.txt

echo ""
echo "=== Execution Complete ==="
echo "ONVIF daemon ($MODE mode) should now be running on device"
echo "Device output has been saved to: ../debugging/logs/"
echo ""
echo "To check if it's running:"
echo "  telnet $DEVICE_IP"
echo "  ps | grep onvifd"
echo ""
echo "To stop the daemon:"
echo "  telnet $DEVICE_IP"
echo "  killall onvifd"
echo ""
echo "To collect core dumps:"
echo "  ./collect_coredump.sh $DEVICE_IP $USERNAME $PASSWORD"
echo ""
echo "To view logs (if available):"
echo "  telnet $DEVICE_IP"
echo "  dmesg | tail -20"
echo ""
echo "To view saved execution logs:"
echo "  ls -la ../debugging/logs/"
echo "  cat ../debugging/logs/onvif_execution_*.log"
