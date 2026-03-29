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
    log_info "Configuring LLVM ${LLVM_VERSION}..."

    local cmake_args=(
        -G Ninja
        -DCMAKE_BUILD_TYPE=Release
        -DLLVM_TARGETS_TO_BUILD="ARM;AArch64"
        -DLLVM_DEFAULT_TARGET_TRIPLE="${TARGET_TUPLE}"
        -DCMAKE_C_COMPILER="${CROSS_CC}"
        -DCMAKE_CXX_COMPILER="${CROSS_CXX}"
        -DCMAKE_AR="${CROSS_AR}"
        -DCMAKE_RANLIB="${CROSS_RANLIB}"
        -DCMAKE_SYSROOT="${SYSROOT}"
        -DCMAKE_FIND_ROOT_PATH="${SYSROOT}"
        -DCMAKE_FIND_ROOT_PATH_MODE_PROGRAM=NEVER
        -DCMAKE_FIND_ROOT_PATH_MODE_LIBRARY=ONLY
        -DCMAKE_FIND_ROOT_PATH_MODE_INCLUDE=ONLY
        -DLLVM_ENABLE_PROJECTS="clang;lld;compiler-rt"
        -DLLVM_ENABLE_RUNTIMES="compiler-rt"
        -DCLANG_ENABLE_ARCMOVE=OFF
        -DCLANG_ENABLE_STATIC_ANALYZER=OFF
        -DCLANG_ENABLE_OBJC_REWRITER=OFF
        -DLLVM_INCLUDE_BENCHMARKS=OFF
        -DLLVM_INCLUDE_TESTS=OFF
        -DLLVM_INCLUDE_DOCS=OFF
        -DCMAKE_INSTALL_PREFIX="${llvm_install}"
    )

    # ARM-specific configuration
    if [[ "${ARCH}" == "armv5te" ]]; then
        cmake_args+=(
            -DLLVM_TARGET_ARCH=ARM
            -DLLVM_TARGETS_TO_BUILD=ARM
        )
    fi

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
