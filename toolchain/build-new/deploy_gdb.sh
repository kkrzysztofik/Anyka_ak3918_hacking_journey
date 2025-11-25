#!/bin/bash
#
# deploy_gdb.sh - Copy rebuilt GDB to SD card directory
#
# Description:
#   This script copies the rebuilt GDB and gdbserver to the SD card directory
#   for testing on the device.
#
# Usage:
#   ./deploy_gdb.sh
#
# Prerequisites:
#   - GDB must be rebuilt using rebuild_gdb.sh
#
# Author: Anyka Hack Project
# Version: 1.0

set -e

# Get script directory and project root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
INSTALL_DIR="${PROJECT_ROOT}/toolchain/arm-anykav200-crosstool-ng"
TARGET="arm-unknown-linux-uclibcgnueabi"
SD_GDB_DIR="${PROJECT_ROOT}/SD_card_contents/anyka_hack/gdb"

echo "=========================================="
echo "Deploying GDB to SD card directory"
echo "=========================================="

# Verify ARM GDB and gdbserver exist (these are for the target device)
ARM_GDB_BINARY="${INSTALL_DIR}/bin/${TARGET}-gdb-arm"
GDB_SERVER="${INSTALL_DIR}/${TARGET}/debug-root/usr/bin/gdbserver"

if [ ! -f "${ARM_GDB_BINARY}" ]; then
    echo "ERROR: ARM GDB not found at ${ARM_GDB_BINARY}"
    echo "Please run rebuild_gdb.sh first"
    exit 1
fi

if [ ! -f "${GDB_SERVER}" ]; then
    echo "ERROR: gdbserver not found at ${GDB_SERVER}"
    echo "Please run rebuild_gdb.sh first"
    exit 1
fi

# Verify ARM GDB dynamic linker path
echo "Verifying ARM GDB dynamic linker path..."
ARM_GDB_INTERP=$(readelf -l "${ARM_GDB_BINARY}" 2>/dev/null | grep "Requesting program interpreter" | sed 's/.*\[\(.*\)\].*/\1/' || echo "")
if [ -n "${ARM_GDB_INTERP}" ]; then
    echo "ARM GDB interpreter: ${ARM_GDB_INTERP}"
    if [ "${ARM_GDB_INTERP}" != "/mnt/anyka_hack/lib/ld-uClibc.so.1" ]; then
        echo "WARNING: ARM GDB interpreter is ${ARM_GDB_INTERP}, expected /mnt/anyka_hack/lib/ld-uClibc.so.1"
    else
        echo "✓ ARM GDB has correct interpreter path"
    fi
else
    echo "ARM GDB appears to be statically linked (no interpreter)"
fi

# Create SD card directory
mkdir -p "${SD_GDB_DIR}"

# Copy ARM GDB (for running on target device)
echo "Copying ARM GDB to ${SD_GDB_DIR}/gdb..."
cp "${ARM_GDB_BINARY}" "${SD_GDB_DIR}/gdb"
chmod +x "${SD_GDB_DIR}/gdb"

# Copy gdbserver
echo "Copying gdbserver to ${SD_GDB_DIR}/gdbserver..."
cp "${GDB_SERVER}" "${SD_GDB_DIR}/gdbserver"
chmod +x "${SD_GDB_DIR}/gdbserver"

# Verify copy
if [ -f "${SD_GDB_DIR}/gdb" ] && [ -f "${SD_GDB_DIR}/gdbserver" ]; then
    echo "=========================================="
    echo "✓ GDB deployment successful!"
    echo "=========================================="
    echo ""
    echo "ARM GDB (for target device):"
    echo "  Location: ${SD_GDB_DIR}/gdb"
    echo "  Size: $(du -h "${SD_GDB_DIR}/gdb" | cut -f1)"
    echo ""
    echo "gdbserver (for target device):"
    echo "  Location: ${SD_GDB_DIR}/gdbserver"
    echo "  Size: $(du -h "${SD_GDB_DIR}/gdbserver" | cut -f1)"
    echo ""
    echo "Host GDB (for x86-64 host, connects to gdbserver):"
    echo "  Location: ${INSTALL_DIR}/bin/${TARGET}-gdb"
    echo ""
    echo "Usage on target device:"
    echo "  /mnt/anyka_hack/gdb/gdb [program]          # Direct debugging"
    echo "  /mnt/anyka_hack/gdb/gdbserver :1234 [program]  # Remote debugging"
    echo ""
    echo "Usage on host:"
    echo "  ${INSTALL_DIR}/bin/${TARGET}-gdb [program]"
    echo "  (gdb) target remote <device_ip>:1234"
else
    echo "ERROR: Failed to copy GDB binaries to SD card directory"
    exit 1
fi
