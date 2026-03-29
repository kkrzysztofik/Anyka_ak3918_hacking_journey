#!/bin/bash
# Common functions and path model for toolchain build system
# Sourced by build.sh and all stage modules

set -euo pipefail

# =============================================================================
# Path Model - Centralized for entire build system
# =============================================================================
# All paths are relative to the build-new/ root for portability
# and relative to absolute paths when invoked from elsewhere

# Root of the build system (where build.sh lives)
# Use values from environment if already set (by build.sh), otherwise compute
if [[ -z "${ROOT_DIR:-}" ]]; then
    ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
    export ROOT_DIR
fi

# Scripts directory (this file's directory)
if [[ -z "${SCRIPTS_DIR:-}" ]]; then
    SCRIPTS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    export SCRIPTS_DIR
fi

# Build output directory (all generated artifacts go here)
export BUILD_DIR="${ROOT_DIR}/build"

# Install directory (final toolchain destination)
# Default: sibling to build-new/ (arm-anykav200-crosstool-ng/)
# Set based on ARCH after ARCH is determined below

# =============================================================================
# Architecture Configuration
# =============================================================================
# Supported: armv5te (default), aarch64
export ARCH="${ARCH:-armv5te}"

# Target tuples based on architecture
case "${ARCH}" in
    armv5te)
        export TARGET_TUPLE="arm-unknown-linux-uclibcgnueabi"
        export RUST_TARGET="armv5te-unknown-linux-uclibceabi"
        export INSTALL_DIR="${ROOT_DIR}/../arm-anykav200-crosstool-ng"
        ;;
    aarch64)
        export TARGET_TUPLE="aarch64-unknown-linux-gnu"
        export RUST_TARGET="aarch64-unknown-linux-gnu"
        export INSTALL_DIR="${ROOT_DIR}/../aarch64-unknown-linux-gnu-toolchain"
        ;;
    *)
        echo "ERROR: Unknown ARCH '${ARCH}'. Supported: armv5te, aarch64" >&2
        exit 1
        ;;
esac

# Sysroot location
export SYSROOT="${INSTALL_DIR}/${TARGET_TUPLE}/sysroot"

# Cross-compiler tool paths
export CROSS_CC="${INSTALL_DIR}/bin/${TARGET_TUPLE}-gcc"
export CROSS_CXX="${INSTALL_DIR}/bin/${TARGET_TUPLE}-g++"
export CROSS_AR="${INSTALL_DIR}/bin/${TARGET_TUPLE}-ar"
export CROSS_RANLIB="${INSTALL_DIR}/bin/${TARGET_TUPLE}-ranlib"

# =============================================================================
# Version Configuration
# =============================================================================
export CROSSTOOL_NG_VERSION="1.28.0"
export LLVM_VERSION="22.1.2"
export RUST_VERSION="1.94.1"
export GDB_VERSION="17.1"

# =============================================================================
# Build Directory Subdirectories
# =============================================================================
export CT_NG_WORK_DIR="${BUILD_DIR}/.build"
export RUST_SRC_DIR="${BUILD_DIR}/rust/src"
export LLVM_SRC_DIR="${BUILD_DIR}/llvm-${LLVM_VERSION}/src"

# =============================================================================
# Logging Functions
# =============================================================================
log_info() {
    echo "[INFO] $*"
}

log_warn() {
    echo "[WARN] $*" >&2
}

log_error() {
    echo "[ERROR] $*" >&2
}

# =============================================================================
# Dependency Check Helpers
# =============================================================================
check_deps() {
    local missing=()
    for dep in "$@"; do
        if ! command -v "${dep}" &> /dev/null; then
            missing+=("${dep}")
        fi
    done
    if [[ ${#missing[@]} -gt 0 ]]; then
        log_error "Missing dependencies: ${missing[*]}"
        return 1
    fi
    return 0
}

require_toolchain() {
    if [[ ! -d "${INSTALL_DIR}" ]]; then
        log_error "Toolchain not installed at: ${INSTALL_DIR}"
        log_error "Run: ./build.sh (or ./build.sh --resume after partial build)"
        return 1
    fi
    if [[ ! -f "${CROSS_CC}" ]]; then
        log_error "Cross compiler not found: ${CROSS_CC}"
        return 1
    fi
    return 0
}

# =============================================================================
# Checkpoint System
# =============================================================================
CHECKPOINT_DIR="${BUILD_DIR}/.checkpoints"

mark_checkpoint() {
    local name="$1"
    mkdir -p "${CHECKPOINT_DIR}"
    echo "${ARCH}:$(date +%s)" > "${CHECKPOINT_DIR}/${name}"
    log_info "Checkpoint marked: ${name}"
}

has_checkpoint() {
    local name="$1"
    [[ -f "${CHECKPOINT_DIR}/${name}" ]]
}

clear_checkpoint() {
    local name="$1"
    rm -f "${CHECKPOINT_DIR}/${name}"
}

# =============================================================================
# Dry Run Mode
# =============================================================================
DRY_RUN="${DRY_RUN:-false}"

dry_run() {
    if [[ "${DRY_RUN}" == "true" ]]; then
        log_info "[DRY RUN] $*"
        return 0
    fi
    return 1
}

# =============================================================================
# Ensure Required Directories Exist
# =============================================================================
ensure_dirs() {
    mkdir -p "${BUILD_DIR}"
    mkdir -p "${BUILD_DIR}/.build"
    mkdir -p "$(dirname "${INSTALL_DIR}")"
}
