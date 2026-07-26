#!/bin/bash
# Stage 5: Rebuild GDB with custom dynamic linker path
# Installs to ${INSTALL_DIR}

set -euo pipefail

source "${SCRIPTS_DIR}/common.sh"

STAGE_NAME="gdb"

stage_gdb() {
    log_info "=========================================="
    log_info "Stage 5: Rebuilding GDB ${GDB_VERSION}"
    log_info "Target: ${TARGET_TUPLE}"
    log_info "=========================================="

    if has_checkpoint "${STAGE_NAME}"; then
        log_info "Skipping - checkpoint exists"
        return 0
    fi

    require_toolchain

    local gdb_src="${BUILD_DIR}/gdb-${GDB_VERSION}"
    local gdb_tarball="gdb-${GDB_VERSION}.tar.xz"

    # Download GDB source if needed
    if [[ ! -d "${gdb_src}" ]]; then
        log_info "Downloading GDB ${GDB_VERSION}..."
        cd "${BUILD_DIR}"
        if [[ ! -f "${gdb_tarball}" ]]; then
            local gdb_url="https://sourceware.org/pub/gdb/releases/${gdb_tarball}"
            log_info "Fetching ${gdb_url}"
            wget "${gdb_url}" -O "${gdb_tarball}" || {
                rm -f "${gdb_tarball}"
                log_error "Failed to download GDB ${GDB_VERSION}"
                exit 1
            }
        fi
        tar -xf "${gdb_tarball}"
    fi

    # Build ARM GDB (runs on target)
    log_info "Building ARM GDB for target device..."
    build_gdb_arm

    # Build host GDB (runs on x86-64)
    log_info "Building host GDB..."
    build_gdb_host

    mark_checkpoint "${STAGE_NAME}"

    log_info "=========================================="
    log_info "GDB rebuild completed!"
    log_info "Host GDB: ${INSTALL_DIR}/bin/${TARGET_TUPLE}-gdb"
    log_info "ARM GDB:  ${INSTALL_DIR}/bin/${TARGET_TUPLE}-gdb-arm"
    log_info "=========================================="
}

build_gdb_arm() {
    local gdb_src="${BUILD_DIR}/gdb-${GDB_VERSION}"
    local build_dir="${gdb_src}/build-arm"

    mkdir -p "${build_dir}"
    cd "${build_dir}"

    local dynamic_linker="/mnt/anyka_hack/lib/ld-uClibc.so.1"

    ../configure \
        --host="${TARGET_TUPLE}" \
        --target="${TARGET_TUPLE}" \
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
        CC="${CROSS_CC}" \
        CXX="${CROSS_CXX}" \
        AR="${CROSS_AR}" \
        RANLIB="${CROSS_RANLIB}" \
        CFLAGS="-march=armv5te -mfloat-abi=soft -mtune=arm926ej-s" \
        CXXFLAGS="-march=armv5te -mfloat-abi=soft -mtune=arm926ej-s" \
        LDFLAGS="-Wl,--dynamic-linker=${dynamic_linker}"

    make -j"$(nproc)" all-gdb all-gdbserver

    DESTDIR="${INSTALL_DIR}/arm-gdb" make install-gdb
    make install-gdbserver

    mkdir -p "${INSTALL_DIR}/bin"
    if [[ -f "gdb/gdb" ]]; then
        cp "gdb/gdb" "${INSTALL_DIR}/bin/${TARGET_TUPLE}-gdb-arm"
        chmod +x "${INSTALL_DIR}/bin/${TARGET_TUPLE}-gdb-arm"
    fi

    mkdir -p "${INSTALL_DIR}/${TARGET_TUPLE}/debug-root/usr/bin"
    if [[ -f "gdbserver/gdbserver" ]]; then
        cp "gdbserver/gdbserver" "${INSTALL_DIR}/${TARGET_TUPLE}/debug-root/usr/bin/gdbserver"
        chmod +x "${INSTALL_DIR}/${TARGET_TUPLE}/debug-root/usr/bin/gdbserver"
    fi
}

build_gdb_host() {
    local gdb_src="${BUILD_DIR}/gdb-${GDB_VERSION}"
    local build_dir="${gdb_src}/build-host"

    mkdir -p "${build_dir}"
    cd "${build_dir}"

    # Check for GMP/MPFR — headers may be in arch-specific subdirs on Debian/Ubuntu
    local gmp_h
    gmp_h="$(find /usr/include -name "gmp.h" 2>/dev/null | head -1)"
    local mpfr_h="/usr/include/mpfr.h"
    if [[ -z "${gmp_h}" ]] || [[ ! -f "${mpfr_h}" ]]; then
        log_error "GMP or MPFR development headers not found"
        log_error "Install: sudo apt-get install libgmp-dev libmpfr-dev"
        exit 1
    fi

    local host_cflags="-I/usr/include"
    [[ -d /usr/include/x86_64-linux-gnu ]] && host_cflags+=" -I/usr/include/x86_64-linux-gnu"

    ../configure \
        --host="x86_64-unknown-linux-gnu" \
        --target="${TARGET_TUPLE}" \
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
        CFLAGS="${host_cflags}" \
        CPPFLAGS="${host_cflags}"

    make -j"$(nproc)" all-gdb
    make install-gdb
}
