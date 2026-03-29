#!/bin/bash
# Stage 2: Build LLVM/Clang with cross-compilation support
# Installs to ${INSTALL_DIR}

set -euo pipefail

source "${SCRIPTS_DIR}/common.sh"

STAGE_NAME="llvm"

stage_llvm() {
    log_info "=========================================="
    log_info "Stage 2: Building LLVM/Clang ${LLVM_VERSION}"
    log_info "Architecture: ${ARCH}"
    log_info "=========================================="

    if has_checkpoint "${STAGE_NAME}"; then
        log_info "Skipping - checkpoint exists"
        return 0
    fi

    require_toolchain

    local llvm_src="${LLVM_SRC_DIR}"
    local llvm_build="${BUILD_DIR}/.build/llvm-${LLVM_VERSION}"
    local llvm_install="${INSTALL_DIR}"

    # Download LLVM source if needed
    if [[ ! -d "${llvm_src}" ]]; then
        log_info "Downloading LLVM ${LLVM_VERSION}..."
        mkdir -p "$(dirname "${llvm_src}")"
        local llvm_project="llvm-project-${LLVM_VERSION}.tar.xz"
        cd "${BUILD_DIR}"
        if [[ ! -f "${llvm_project}" ]]; then
            wget -q "https://github.com/llvm/llvm-project/releases/download/llvmorg-${LLVM_VERSION}/${llvm_project}"
        fi
        mkdir -p "${llvm_src}"
        tar --strip-components=1 -xf "${llvm_project}" -C "${llvm_src}"
    fi

    # Create build directory
    mkdir -p "${llvm_build}"
    cd "${llvm_build}"

    # Configure LLVM
    # LLVM is a HOST tool (runs on x86-64, emits ARM code).
    # It MUST be built with the native compiler, NOT the cross-compiler.
    log_info "Configuring LLVM ${LLVM_VERSION} (host build targeting ${ARCH})..."

    # Select which LLVM backends to build based on target architecture.
    local llvm_targets
    case "${ARCH}" in
        armv5te)  llvm_targets="ARM" ;;
        aarch64)  llvm_targets="AArch64" ;;
        *)        llvm_targets="ARM;AArch64" ;;
    esac

    local cmake_args=(
        -G Ninja
        -DCMAKE_BUILD_TYPE=Release
        # Host compiler (native x86-64 gcc/g++) — NOT the ARM cross-compiler
        -DCMAKE_C_COMPILER=gcc
        -DCMAKE_CXX_COMPILER=g++
        -DCMAKE_AR=ar
        -DCMAKE_RANLIB=ranlib
        # Target backends to include in the built LLVM
        -DLLVM_TARGETS_TO_BUILD="${llvm_targets}"
        -DLLVM_DEFAULT_TARGET_TRIPLE="${TARGET_TUPLE}"
        # Build clang + lld; compiler-rt builtins are built separately in stage 3
        -DLLVM_ENABLE_PROJECTS="clang;lld"
        -DCLANG_ENABLE_ARCMOVE=OFF
        -DCLANG_ENABLE_STATIC_ANALYZER=OFF
        -DCLANG_ENABLE_OBJC_REWRITER=OFF
        -DLLVM_INCLUDE_BENCHMARKS=OFF
        -DLLVM_INCLUDE_TESTS=OFF
        -DLLVM_INCLUDE_DOCS=OFF
        -DCMAKE_INSTALL_PREFIX="${llvm_install}"
    )

    cmake "${llvm_src}/llvm" "${cmake_args[@]}"

    # Build LLVM
    log_info "Building LLVM (this may take 2-4 hours)..."
    ninja -j"$(nproc)"

    # Install LLVM
    log_info "Installing LLVM..."
    ninja install

    mark_checkpoint "${STAGE_NAME}"

    log_info "=========================================="
    log_info "LLVM/Clang build completed!"
    log_info "Installation: ${INSTALL_DIR}"
    log_info "=========================================="
}
