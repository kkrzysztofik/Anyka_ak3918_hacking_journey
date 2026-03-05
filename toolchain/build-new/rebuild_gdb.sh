#!/bin/bash
#
# rebuild_gdb.sh - Rebuild GDB with custom dynamic linker path
#
# Description:
#   This script rebuilds GDB from the toolchain with /mnt/anyka_hack/lib/ld-uClibc.so.1
#   as the dynamic linker, matching the configuration used for xiu and other projects.
#
# Usage:
#   ./rebuild_gdb.sh
#
# Prerequisites:
#   - Cross-compiler toolchain must be built and available
#   - GDB source will be downloaded automatically if needed
#
# Author: Anyka Hack Project
# Version: 1.0

set -e

# Script directory \u2014 must be set before sourcing common.sh
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/common.sh"

GDB_TARBALL="gdb-${GDB_VERSION}.tar.xz"
GDB_DIR="${BUILD_DIR}/.build/gdb-${GDB_VERSION}"

# GDB is built for this target
TARGET="${TARGET_TUPLE}"
DYNAMIC_LINKER="/mnt/anyka_hack/lib/ld-uClibc.so.1"

log_info "=========================================="
log_info "Rebuilding GDB ${GDB_VERSION}"
log_info "Target: ${TARGET}"
log_info "Dynamic linker: ${DYNAMIC_LINKER}"
log_info "=========================================="

# Check if cross-compiler tools are available
if [[ ! -f "${CROSS_CC}" ]]; then
    log_error "Cross-compiler not found at ${CROSS_CC}"
    log_error "Please ensure the toolchain is built and installed"
    exit 1
fi

if [[ ! -f "${CROSS_CXX}" ]]; then
    log_error "Cross-compiler C++ not found at ${CROSS_CXX}"
    exit 1
fi

# Set up paths
export PATH="${INSTALL_DIR}/bin:${PATH}"

# Create build directory
mkdir -p "${BUILD_DIR}/.build"
cd "${BUILD_DIR}/.build"

# Download GDB source if not present
if [ ! -d "${GDB_DIR}" ]; then
    log_info "Downloading GDB ${GDB_VERSION}..."
    if [ ! -f "${GDB_TARBALL}" ]; then
        log_info "Downloading from sourceware.org..."
        wget "https://sourceware.org/pub/gdb/releases/${GDB_TARBALL}" || {
            log_error "Failed to download GDB source"
            exit 1
        }
    fi
    log_info "Extracting GDB source..."
    tar -xf "${GDB_TARBALL}"
fi

cd "${GDB_DIR}"

# ============================================
# Build 1: ARM GDB (runs on target device)
# ============================================
log_info ""
log_info "=========================================="
log_info "Building ARM GDB (for target device)..."
log_info "=========================================="

# Clean previous ARM build
if [ -d "build-arm" ]; then
    log_info "Cleaning previous ARM build..."
    rm -rf build-arm
fi

mkdir -p build-arm
cd build-arm

# Configure ARM GDB
# --host is where GDB runs (ARM), --target is what it debugs (ARM)
log_info "Configuring ARM GDB..."
../configure \
    --host="${TARGET}" \
    --target="${TARGET}" \
    --prefix="${INSTALL_DIR}" \
    --with-sysroot="${SYSROOT}" \
    --disable-werror \
    --enable-gdbmi=no \
    --with-python=no \
    --with-expat=no \
    --with-lzma=no \
    --with-zlib=no \
    --with-babeltrace=no \
    --with-debuginfod=no \
    --with-libunwind=no \
    --with-libbacktrace=no \
    --with-system-readline=no \
    --with-curses=no \
    --with-tcl=no \
    --with-tk=no \
    --with-gnu-ld \
    --with-gnu-as \
    CC="${INSTALL_DIR}/bin/${TARGET}-gcc" \
    CXX="${INSTALL_DIR}/bin/${TARGET}-g++" \
    AR="${INSTALL_DIR}/bin/${TARGET}-ar" \
    RANLIB="${INSTALL_DIR}/bin/${TARGET}-ranlib" \
    CFLAGS="-march=armv5te -mfloat-abi=soft -mtune=arm926ej-s" \
    CXXFLAGS="-march=armv5te -mfloat-abi=soft -mtune=arm926ej-s" \
    LDFLAGS="-Wl,--dynamic-linker=${DYNAMIC_LINKER}"

# Build ARM GDB and gdbserver
log_info "Building ARM GDB and gdbserver..."
make -j$(nproc) all-gdb all-gdbserver

# Install ARM GDB to a separate location
log_info "Installing ARM GDB..."
DESTDIR="${INSTALL_DIR}/arm-gdb" make install-gdb

# Install gdbserver
log_info "Installing gdbserver..."
make install-gdbserver

# Copy ARM GDB binary
mkdir -p "${INSTALL_DIR}/bin"
if [ -f "gdb/gdb" ]; then
    cp "gdb/gdb" "${INSTALL_DIR}/bin/${TARGET}-gdb-arm"
    chmod +x "${INSTALL_DIR}/bin/${TARGET}-gdb-arm"
    log_info "ARM GDB installed to: ${INSTALL_DIR}/bin/${TARGET}-gdb-arm"
fi

# Copy gdbserver to the expected location
mkdir -p "${INSTALL_DIR}/${TARGET}/debug-root/usr/bin"
if [ -f "gdbserver/gdbserver" ]; then
    cp "gdbserver/gdbserver" "${INSTALL_DIR}/${TARGET}/debug-root/usr/bin/gdbserver"
    chmod +x "${INSTALL_DIR}/${TARGET}/debug-root/usr/bin/gdbserver"
    log_info "gdbserver installed to: ${INSTALL_DIR}/${TARGET}/debug-root/usr/bin/gdbserver"
fi

# ============================================
# Build 2: Host GDB (runs on x86-64 host)
# ============================================
log_info ""
log_info "=========================================="
log_info "Building Host GDB (for x86-64 host)..."
log_info "=========================================="

cd "${GDB_DIR}"

# Clean previous host build
if [ -d "build-host" ]; then
    log_info "Cleaning previous host build..."
    rm -rf build-host
fi

mkdir -p build-host
cd build-host

# Configure host GDB
# --host is where GDB runs (x86-64), --target is what it debugs (ARM)
# For host build, we need to use native compiler, not cross-compiler
log_info "Configuring Host GDB..."

# Clear any cross-compilation environment variables and PATH
# We need to use native gcc, not the ARM cross-compiler
OLD_PATH="${PATH}"
export PATH="/usr/bin:/bin:${PATH}"
unset CC CXX AR RANLIB CFLAGS CXXFLAGS LDFLAGS

# Check for GMP and MPFR headers (they may be in different locations)
GMP_H_FOUND=""
MPFR_H_FOUND=""

# Check common locations for gmp.h
for path in /usr/include/gmp.h /usr/include/x86_64-linux-gnu/gmp.h /usr/local/include/gmp.h; do
    if [ -f "$path" ]; then
        GMP_H_FOUND="$path"
        break
    fi
done

# Check common locations for mpfr.h
for path in /usr/include/mpfr.h /usr/include/x86_64-linux-gnu/mpfr.h /usr/local/include/mpfr.h; do
    if [ -f "$path" ]; then
        MPFR_H_FOUND="$path"
        break
    fi
done

if [ -z "$GMP_H_FOUND" ] || [ -z "$MPFR_H_FOUND" ]; then
    log_error "GMP or MPFR development headers not found!"
    log_info "  Looking for gmp.h: ${GMP_H_FOUND:-not found}"
    log_info "  Looking for mpfr.h: ${MPFR_H_FOUND:-not found}"
    log_info "Please install: sudo apt-get install libgmp-dev libmpfr-dev"
    export PATH="${OLD_PATH}"
    exit 1
fi

log_info "Found GMP header: ${GMP_H_FOUND}"
log_info "Found MPFR header: ${MPFR_H_FOUND}"
log_info "Using system GMP and MPFR from /usr"

# Determine include paths
GMP_INCLUDE_DIR=$(dirname "${GMP_H_FOUND}")
MPFR_INCLUDE_DIR=$(dirname "${MPFR_H_FOUND}")

# Build CFLAGS with include paths
HOST_CFLAGS="-I${GMP_INCLUDE_DIR}"
if [ "${GMP_INCLUDE_DIR}" != "${MPFR_INCLUDE_DIR}" ]; then
    HOST_CFLAGS="${HOST_CFLAGS} -I${MPFR_INCLUDE_DIR}"
fi

../configure \
    --host="x86_64-unknown-linux-gnu" \
    --target="${TARGET}" \
    --prefix="${INSTALL_DIR}" \
    --with-sysroot="${SYSROOT}" \
    --with-gmp=/usr \
    --with-mpfr=/usr \
    --disable-werror \
    --enable-gdbmi=no \
    --with-python=no \
    --with-expat=no \
    --with-lzma=no \
    --with-zlib=no \
    --with-babeltrace=no \
    --with-debuginfod=no \
    --with-libunwind=no \
    --with-libbacktrace=no \
    --with-system-readline=no \
    --with-curses=no \
    --with-tcl=no \
    --with-tk=no \
    --with-gnu-ld \
    --with-gnu-as \
    CC="gcc" \
    CXX="g++" \
    AR="ar" \
    RANLIB="ranlib" \
    CFLAGS="${HOST_CFLAGS}" \
    CPPFLAGS="${HOST_CFLAGS}"

# Restore PATH
export PATH="${OLD_PATH}"

# Build host GDB
log_info "Building Host GDB..."
make -j$(nproc) all-gdb

# Install host GDB
log_info "Installing Host GDB..."
make install-gdb

# Verify host GDB
if [ -f "${INSTALL_DIR}/bin/${TARGET}-gdb" ]; then
    log_info "Host GDB installed to: ${INSTALL_DIR}/bin/${TARGET}-gdb"
fi

log_info ""
log_info "=========================================="
log_info "✓ GDB rebuild complete!"
log_info "=========================================="
log_info ""
log_info "Host GDB (x86-64, for connecting to gdbserver):"
log_info "  ${INSTALL_DIR}/bin/${TARGET}-gdb"
if [ -f "${INSTALL_DIR}/bin/${TARGET}-gdb" ]; then
    file "${INSTALL_DIR}/bin/${TARGET}-gdb"
fi
log_info ""
log_info "ARM GDB (for running on target device):"
log_info "  ${INSTALL_DIR}/bin/${TARGET}-gdb-arm"
if [ -f "${INSTALL_DIR}/bin/${TARGET}-gdb-arm" ]; then
    file "${INSTALL_DIR}/bin/${TARGET}-gdb-arm"
    log_info "  Interpreter:"
    readelf -l "${INSTALL_DIR}/bin/${TARGET}-gdb-arm" 2>/dev/null | grep -A 1 "interpreter" || log_info "    (statically linked or not found)"
fi
log_info ""
log_info "gdbserver (for running on target device):"
log_info "  ${INSTALL_DIR}/${TARGET}/debug-root/usr/bin/gdbserver"
if [ -f "${INSTALL_DIR}/${TARGET}/debug-root/usr/bin/gdbserver" ]; then
    file "${INSTALL_DIR}/${TARGET}/debug-root/usr/bin/gdbserver"
    log_info "  Interpreter:"
    readelf -l "${INSTALL_DIR}/${TARGET}/debug-root/usr/bin/gdbserver" 2>/dev/null | grep -A 1 "interpreter" || log_info "    (statically linked or not found)"
fi
