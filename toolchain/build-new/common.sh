#!/bin/bash
# shellcheck disable=SC2034  # Variables are intentionally used by scripts that source this file
# common.sh — Shared configuration and utilities for build-new toolchain scripts.
#
# USAGE:
#   Set SCRIPT_DIR before sourcing:
#     SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
#     source "${SCRIPT_DIR}/common.sh"

# Guard against double-sourcing
[[ -n "${_ANYKA_COMMON_SH:-}" ]] && return 0
readonly _ANYKA_COMMON_SH=1

# ── Colors ───────────────────────────────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# ── Logging ───────────────────────────────────────────────────────────────────
log_info() {
    local message="$1"
    echo -e "${GREEN}[INFO]${NC} ${message}"
}

log_warn() {
    local message="$1"
    echo -e "${YELLOW}[WARN]${NC} ${message}"
}

log_error() {
    local message="$1"
    echo -e "${RED}[ERROR]${NC} ${message}" >&2
}

# ── Directories ───────────────────────────────────────────────────────────────
# SCRIPT_DIR must be set by the caller before sourcing this file.
BUILD_DIR="${SCRIPT_DIR}"
INSTALL_DIR="${SCRIPT_DIR}/../arm-anykav200-crosstool-ng"

# ── ARMv5TE target constants ──────────────────────────────────────────────────
TARGET_TUPLE="arm-unknown-linux-uclibcgnueabi"
SYSROOT="${INSTALL_DIR}/${TARGET_TUPLE}/sysroot"
CROSS_CC="${INSTALL_DIR}/bin/${TARGET_TUPLE}-gcc"
CROSS_CXX="${INSTALL_DIR}/bin/${TARGET_TUPLE}-g++"
CROSS_AR="${INSTALL_DIR}/bin/${TARGET_TUPLE}-ar"
CROSS_RANLIB="${INSTALL_DIR}/bin/${TARGET_TUPLE}-ranlib"
LLVM_TARGET="ARM"
CMAKE_TARGET_ARCH="arm"

# ── Component versions ────────────────────────────────────────────────────────
LLVM_VERSION="22.1.2"
LLVM_SRC_DIR="${BUILD_DIR}/llvm-${LLVM_VERSION}"
GDB_VERSION="17.1"
# crosstool-NG: pinned git commit (binutils 2.46.0). Vendored packages: see vendor/crosstool-ng/.
CTNG_GIT_URL="https://github.com/crosstool-ng/crosstool-ng.git"
CTNG_GIT_REF="37190d5b1e8050832610ba5e899911c7a723d798"
CTNG_DIR="${BUILD_DIR}/crosstool-ng-src"
# Vendored under vendor/crosstool-ng/uClibc-ng/${UCLIBC_NG_VERSION}/ (must match that tree).
UCLIBC_NG_VERSION="1.0.57"
RUST_VERSION="1.94.1"
RUST_SRC_DIR="${BUILD_DIR}/rust"

# ── Utility functions ─────────────────────────────────────────────────────────
# check_deps <cmd> [cmd ...] — exit 1 if any command is not found in PATH.
check_deps() {
    local missing=()
    for cmd in "$@"; do
        if ! command -v "${cmd}" &> /dev/null; then
            missing+=("${cmd}")
        fi
    done
    if [[ ${#missing[@]} -ne 0 ]]; then
        log_error "Missing dependencies: ${missing[*]}"
        log_info "Install with: sudo apt-get install -y ${missing[*]}"
        exit 1
    fi
}

# require_toolchain — exit 1 if the GCC cross-compiler is not built yet.
require_toolchain() {
    if [[ ! -f "${CROSS_CC}" ]]; then
        log_error "GCC toolchain not found at: ${CROSS_CC}"
        log_error "Please build the GCC toolchain first: ./build_toolchain.sh"
        exit 1
    fi
}
