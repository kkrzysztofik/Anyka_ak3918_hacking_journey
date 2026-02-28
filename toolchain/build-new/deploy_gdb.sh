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

# Script directory \u2014 must be set before sourcing common.sh
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/common.sh"

PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
TARGET="${TARGET_TUPLE}"
SD_GDB_DIR="${PROJECT_ROOT}/SD_card_contents/anyka_hack/gdb"

log_info "=========================================="
log_info "Deploying GDB to SD card directory"
log_info "=========================================="

# Verify ARM GDB and gdbserver exist (these are for the target device)
ARM_GDB_BINARY="${INSTALL_DIR}/bin/${TARGET}-gdb-arm"
GDB_SERVER="${INSTALL_DIR}/${TARGET}/debug-root/usr/bin/gdbserver"

if [ ! -f "${ARM_GDB_BINARY}" ]; then
    log_error "ARM GDB not found at ${ARM_GDB_BINARY}"
    log_info "Please run rebuild_gdb.sh first"
    exit 1
fi

if [ ! -f "${GDB_SERVER}" ]; then
    log_error "gdbserver not found at ${GDB_SERVER}"
    log_info "Please run rebuild_gdb.sh first"
    exit 1
fi

# Verify ARM GDB dynamic linker path
log_info "Verifying ARM GDB dynamic linker path..."
ARM_GDB_INTERP=$(readelf -l "${ARM_GDB_BINARY}" 2>/dev/null | grep "Requesting program interpreter" | sed 's/.*\[\(.*\)\].*/\1/' || log_info "")
if [ -n "${ARM_GDB_INTERP}" ]; then
    log_info "ARM GDB interpreter: ${ARM_GDB_INTERP}"
    if [ "${ARM_GDB_INTERP}" != "/mnt/anyka_hack/lib/ld-uClibc.so.1" ]; then
        log_warn "ARM GDB interpreter is ${ARM_GDB_INTERP}, expected /mnt/anyka_hack/lib/ld-uClibc.so.1"
    else
        log_info "✓ ARM GDB has correct interpreter path"
    fi
else
    log_info "ARM GDB appears to be statically linked (no interpreter)"
fi

# Create SD card directory
mkdir -p "${SD_GDB_DIR}"

# Copy ARM GDB (for running on target device)
log_info "Copying ARM GDB to ${SD_GDB_DIR}/gdb..."
cp "${ARM_GDB_BINARY}" "${SD_GDB_DIR}/gdb"
chmod +x "${SD_GDB_DIR}/gdb"

# Copy gdbserver
log_info "Copying gdbserver to ${SD_GDB_DIR}/gdbserver..."
cp "${GDB_SERVER}" "${SD_GDB_DIR}/gdbserver"
chmod +x "${SD_GDB_DIR}/gdbserver"

# Verify copy
if [ -f "${SD_GDB_DIR}/gdb" ] && [ -f "${SD_GDB_DIR}/gdbserver" ]; then
    log_info "=========================================="
    log_info "✓ GDB deployment successful!"
    log_info "=========================================="
    log_info ""
    log_info "ARM GDB (for target device):"
    log_info "  Location: ${SD_GDB_DIR}/gdb"
    log_info "  Size: $(du -h "${SD_GDB_DIR}/gdb" | cut -f1)"
    log_info ""
    log_info "gdbserver (for target device):"
    log_info "  Location: ${SD_GDB_DIR}/gdbserver"
    log_info "  Size: $(du -h "${SD_GDB_DIR}/gdbserver" | cut -f1)"
    log_info ""
    log_info "Host GDB (for x86-64 host, connects to gdbserver):"
    log_info "  Location: ${INSTALL_DIR}/bin/${TARGET}-gdb"
    log_info ""
    log_info "Usage on target device:"
    log_info "  /mnt/anyka_hack/gdb/gdb [program]          # Direct debugging"
    log_info "  /mnt/anyka_hack/gdb/gdbserver :1234 [program]  # Remote debugging"
    log_info ""
    log_info "Usage on host:"
    log_info "  ${INSTALL_DIR}/bin/${TARGET}-gdb [program]"
    log_info "  (gdb) target remote <device_ip>:1234"
else
    log_error "Failed to copy GDB binaries to SD card directory"
    exit 1
fi
