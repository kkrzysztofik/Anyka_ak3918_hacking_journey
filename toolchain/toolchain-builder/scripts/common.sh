#!/bin/bash
# Common functions and path model for toolchain build system
# Sourced by build.sh and all stage modules

set -euo pipefail

# =============================================================================
# Path Model - Centralized for entire build system
# =============================================================================
# All paths are relative to the toolchain-builder/ root for portability
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
# Default: sibling to toolchain-builder/ (arm-anykav200-crosstool-ng/)
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
export UCLIBC_NG_VERSION="1.0.57"
export LLVM_VERSION="22.1.8"
export RUST_VERSION="1.97.1"
export GDB_VERSION="17.2"

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

# check_deps <cmd> [cmd ...] — exit 1 if any command is not found in PATH.
# For a friendlier error that also shows the apt install command, prefer
# check_all_build_deps (called once up-front from build.sh).
check_deps() {
    local missing=()
    for dep in "$@"; do
        if ! command -v "${dep}" &> /dev/null; then
            missing+=("${dep}")
        fi
    done
    if [[ ${#missing[@]} -gt 0 ]]; then
        log_error "Missing commands: ${missing[*]}"
        return 1
    fi
    return 0
}

# check_all_build_deps — verify every host package required by all build
# stages and emit a single actionable error with the full apt-get command.
#
# Covers:
#   Stage 1 (ct-ng)    : gcc g++ make gperf bison flex texinfo help2man gawk
#                        libtool-bin automake autoconf wget git file python3
#                        perl pkg-config libncurses-dev xz-utils
#   Stage 2 (LLVM)     : cmake ninja-build
#   Stage 3 (crt)      : cmake ninja-build  (same)
#   Stage 4 (Rust)     : rsync python3-dev  (system rustc/cargo checked separately)
#   Stage 5 (GDB)      : libgmp-dev libmpfr-dev
#
# Format: each entry is "command:apt-package" where command is what we look
# for in PATH and apt-package is what apt-get install expects.  When the
# command name matches the package name, only the command is listed.
check_all_build_deps() {
    # "command:apt-package" — colon separates when they differ
    local entries=(
        # ── Stage 1: crosstool-NG host build ─────────────────────────────
        "gcc"
        "g++"
        "make"
        "gperf"
        "bison"
        "flex"
        "makeinfo:texinfo"       # makeinfo is the binary; texinfo is the package
        "help2man"
        "gawk"
        "libtool:libtool-bin"    # libtool binary is in libtool-bin package
        "automake"
        "autoconf"
        "wget"
        "git"
        "file"
        "python3"
        "perl"
        "pkg-config"
        "xz:xz-utils"           # needed to unpack .tar.xz archives
        # libncurses-dev: no binary to check — verified via header below
        # ── Stage 2 & 3: LLVM / compiler-rt ─────────────────────────────
        "cmake"
        "ninja:ninja-build"      # ninja binary; ninja-build is the package
        # ── Stage 4: Rust bootstrap ───────────────────────────────────────
        "rsync"
        "python3-config:python3-dev"  # python3-config binary from python3-dev
        # ── Stage 5: GDB ─────────────────────────────────────────────────
        # libgmp-dev / libmpfr-dev: no binary — verified via headers below
    )

    local missing_pkgs=()

    for entry in "${entries[@]}"; do
        local cmd="${entry%%:*}"
        local pkg="${entry##*:}"
        if ! command -v "${cmd}" &>/dev/null; then
            missing_pkgs+=("${pkg}")
        fi
    done

    # ── Library-only deps (no binary in PATH) ────────────────────────────
    # Check for development headers as a proxy for the -dev packages.
    if ! printf '#include <ncurses.h>\nint main(){}\n' \
            | gcc -x c - -lncurses -o /dev/null 2>/dev/null; then
        missing_pkgs+=("libncurses-dev")
    fi

    if ! printf '#include <gmp.h>\nint main(){}\n' \
            | gcc -x c - -lgmp -o /dev/null 2>/dev/null; then
        missing_pkgs+=("libgmp-dev")
    fi

    if ! printf '#include <mpfr.h>\nint main(){}\n' \
            | gcc -x c - -lmpfr -o /dev/null 2>/dev/null; then
        missing_pkgs+=("libmpfr-dev")
    fi

    if [[ ${#missing_pkgs[@]} -eq 0 ]]; then
        log_info "All build dependencies satisfied."
        return 0
    fi

    log_error "Missing host packages required to build the toolchain:"
    log_error ""
    for pkg in "${missing_pkgs[@]}"; do
        log_error "  - ${pkg}"
    done
    log_error ""
    log_error "Install all missing packages with:"
    log_error ""
    log_error "  sudo apt-get install -y ${missing_pkgs[*]}"
    log_error ""
    return 1
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
