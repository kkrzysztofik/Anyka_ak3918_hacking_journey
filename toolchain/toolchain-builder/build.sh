#!/bin/bash
#
# Unified Toolchain Build Entrypoint
# 
# Builds complete cross-compilation toolchain for Anyka AK3918 (ARMv5TE) or ARM64 targets.
# Single command to build everything: GCC, LLVM, Rust, GDB.
#
# Usage:
#   ./build.sh                        # Build all stages (armv5te default)
#   ./build.sh --arch aarch64         # Build for ARM64 instead
#   ./build.sh --no-rust              # Skip Rust (faster iteration)
#   ./build.sh --no-gdb               # Skip GDB rebuild
#   ./build.sh --resume               # Resume from last checkpoint
#   ./build.sh --clean                 # Clear all checkpoints and rebuild
#   ./build.sh --dry-run               # Show what would be built without building
#   ./build.sh --help                  # Show this help
#
# Checkpoints:
#   Stages that complete successfully are checkpointed. Use --resume to skip
#   completed stages on subsequent runs. Use --clean to force full rebuild.
#
# Environment:
#   ARCH=armv5te   # or aarch64
#   DRY_RUN=true   # echo actions without executing
#

set -euo pipefail

# Script directory and project root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
export readonly ROOT_DIR="${SCRIPT_DIR}"
export readonly SCRIPTS_DIR="${SCRIPT_DIR}/scripts"

# =============================================================================
# Usage and Help
# =============================================================================
show_help() {
    cat << EOF
Unified Toolchain Build System
==============================

USAGE:
    $(basename "$0") [OPTIONS]

OPTIONS:
    --arch ARCH       Target architecture: armv5te (default) or aarch64
    --no-rust        Skip Rust bootstrap stage
    --no-gdb         Skip GDB rebuild stage
    --resume         Resume from last checkpoint (default behavior)
    --clean          Clear all checkpoints and rebuild from scratch
    --dry-run        Show what would be built without executing
    -h, --help       Show this help message

EXAMPLES:
    # Full build (armv5te default)
    ./build.sh

    # Build for ARM64
    ./build.sh --arch aarch64

    # Build without Rust (faster if you only need GCC+LLVM)
    ./build.sh --no-rust

    # Resume interrupted build
    ./build.sh --resume

    # Force clean rebuild
    ./build.sh --clean

    # Dry run to see what would happen
    ./build.sh --dry-run

CHECKPOINTS:
    Build progress is saved to ${ROOT_DIR}/build/.checkpoints/
    Remove specific checkpoints with: rm ${ROOT_DIR}/build/.checkpoints/*

EOF
}

# =============================================================================
# Parse Arguments
# =============================================================================
ARCH="armv5te"
SKIP_RUST=false
SKIP_GDB=false
CLEAN=false
RESUME=false
DRY_RUN=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --arch)
            ARCH="$2"
            shift 2
            ;;
        --no-rust)
            SKIP_RUST=true
            shift
            ;;
        --no-gdb)
            SKIP_GDB=true
            shift
            ;;
        --clean)
            CLEAN=true
            shift
            ;;
        --resume)
            RESUME=true
            shift
            ;;
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        -h|--help)
            show_help
            exit 0
            ;;
        *)
            echo "ERROR: Unknown option: $1" >&2
            show_help
            exit 1
            ;;
    esac
done

export ARCH
export DRY_RUN

# =============================================================================
# Main Build Pipeline
# =============================================================================
main() {
    echo "=============================================="
    echo "Unified Toolchain Build"
    echo "=============================================="
    echo "Architecture: ${ARCH}"
    echo "Dry run:      ${DRY_RUN}"
    echo "Clean:        ${CLEAN}"
    echo "Skip Rust:    ${SKIP_RUST}"
    echo "Skip GDB:     ${SKIP_GDB}"
    echo "=============================================="

    # Source common functions
    source "${SCRIPTS_DIR}/common.sh"

    # Check all host dependencies upfront so the user gets one clear message
    # listing every missing package and the full apt-get install command,
    # rather than discovering missing tools mid-build hours later.
    check_all_build_deps

    # Handle clean
    if [[ "${CLEAN}" == "true" ]]; then
        echo ""
        echo ">>> Clearing all checkpoints..."
        rm -rf "${ROOT_DIR}/build/.checkpoints"
        echo "Checkpoints cleared."
    fi

    # Ensure directories exist
    ensure_dirs

    # Stage 1: GCC toolchain (crosstool-NG)
    source "${SCRIPTS_DIR}/stages/01_toolchain.sh"
    stage_toolchain

    # Stage 2: LLVM/Clang
    source "${SCRIPTS_DIR}/stages/02_llvm.sh"
    stage_llvm

    # Stage 3: compiler-rt builtins (ARM only)
    source "${SCRIPTS_DIR}/stages/03_compiler_rt.sh"
    stage_compiler_rt

    # Stage 4: Rust (optional)
    if [[ "${SKIP_RUST}" == "false" ]]; then
        source "${SCRIPTS_DIR}/stages/04_rust.sh"
        stage_rust
    else
        echo ""
        echo ">>> Skipping Rust (--no-rust)"
    fi

    # Stage 5: GDB (optional)
    if [[ "${SKIP_GDB}" == "false" ]]; then
        source "${SCRIPTS_DIR}/stages/05_gdb.sh"
        stage_gdb
    else
        echo ""
        echo ">>> Skipping GDB (--no-gdb)"
    fi

    echo ""
    echo "=============================================="
    echo "BUILD COMPLETE"
    echo "=============================================="
    echo "Toolchain installed to: ${INSTALL_DIR}"
    echo ""
    echo "Key binaries:"
    echo "  ${INSTALL_DIR}/bin/${TARGET_TUPLE}-gcc"
    echo "  ${INSTALL_DIR}/bin/clang"
    if [[ "${SKIP_RUST}" == "false" ]]; then
        echo "  ${INSTALL_DIR}/bin/rustc"
        echo "  ${INSTALL_DIR}/bin/cargo"
    fi
    if [[ "${SKIP_GDB}" == "false" ]]; then
        echo "  ${INSTALL_DIR}/bin/${TARGET_TUPLE}-gdb"
    fi
    echo "=============================================="
}

main "$@"
