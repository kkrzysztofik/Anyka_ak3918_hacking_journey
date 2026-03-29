#!/bin/bash
# Stage 3: Build compiler-rt builtins for ARM target
# Installs to ${INSTALL_DIR}

set -euo pipefail

source "${SCRIPTS_DIR}/common.sh"

STAGE_NAME="compiler_rt"

stage_compiler_rt() {
    log_info "=========================================="
    log_info "Stage 3: Building compiler-rt builtins"
    log_info "Architecture: ${ARCH}"
    log_info "=========================================="

    if has_checkpoint "${STAGE_NAME}"; then
        log_info "Skipping - checkpoint exists"
        return 0
    fi

    require_toolchain

    if [[ "${ARCH}" != "armv5te" ]]; then
        log_info "Skipping - compiler-rt builtins only needed for ARM"
        return 0
    fi

    local compiler_rt_src="${LLVM_SRC_DIR}/compiler-rt/lib/builtins"
    local compiler_rt_build="${BUILD_DIR}/.build/compiler-rt-${LLVM_VERSION}-arm"

    if [[ ! -d "${compiler_rt_src}" ]]; then
        log_error "Compiler-rt source not found: ${compiler_rt_src}"
        log_error "LLVM must be built first (stage 2)"
        exit 1
    fi

    # Create build directory
    mkdir -p "${compiler_rt_build}"
    cd "${compiler_rt_build}"

    log_info "Configuring compiler-rt builtins..."

    local cmake_target_flags
    cmake_target_flags=(
        -DCMAKE_SYSTEM_NAME=Linux
        -DCMAKE_SYSTEM_PROCESSOR=arm
        -DCMAKE_C_COMPILER_TARGET="armv5te-unknown-linux-uclibcgnueabi"
        -DCMAKE_CXX_COMPILER_TARGET="armv5te-unknown-linux-uclibcgnueabi"
    )

    cmake "${compiler_rt_src}" \
        -G Ninja \
        -DCMAKE_BUILD_TYPE=Release \
        -DCMAKE_INSTALL_PREFIX="${INSTALL_DIR}" \
        -DCMAKE_C_COMPILER="${CROSS_CC}" \
        -DCMAKE_CXX_COMPILER="${CROSS_CXX}" \
        -DCMAKE_AR="${CROSS_AR}" \
        -DCMAKE_RANLIB="${CROSS_RANLIB}" \
        -DCMAKE_SYSROOT="${SYSROOT}" \
        -DCMAKE_FIND_ROOT_PATH="${SYSROOT}" \
        -DCMAKE_FIND_ROOT_PATH_MODE_PROGRAM=NEVER \
        -DCMAKE_FIND_ROOT_PATH_MODE_LIBRARY=ONLY \
        -DCMAKE_FIND_ROOT_PATH_MODE_INCLUDE=ONLY \
        -DCMAKE_FIND_ROOT_PATH_MODE_PACKAGE=ONLY \
        -DCOMPILER_RT_DEFAULT_TARGET_ONLY=ON \
        -DCOMPILER_RT_BAREMETAL_BUILD=OFF \
        -DLLVM_CONFIG_PATH="${INSTALL_DIR}/bin/llvm-config" \
        -DCOMPILER_RT_OS_DIR="linux" \
        "${cmake_target_flags[@]}"

    # Build
    log_info "Building compiler-rt builtins (this may take 30-60 minutes)..."
    ninja -j"$(nproc)" builtins

    # Install
    log_info "Installing compiler-rt builtins..."
    ninja install

    # Create per-target directory for LLVM 21+
    local llvm_major="${LLVM_VERSION%%.*}"
    local new_target_dir="${INSTALL_DIR}/lib/clang/${llvm_major}/lib/armv5te-unknown-linux-gnueabi"
    local old_style_lib
    old_style_lib="$(find "${INSTALL_DIR}/lib" -name "libclang_rt.builtins*.a" \
        -not -path "*/clang/*" 2>/dev/null | head -1)"
    if [[ -n "${old_style_lib}" ]]; then
        mkdir -p "${new_target_dir}"
        cp "${old_style_lib}" "${new_target_dir}/libclang_rt.builtins.a"
        log_info "Created per-target builtins at: ${new_target_dir}/libclang_rt.builtins.a"
    fi

    mark_checkpoint "${STAGE_NAME}"

    log_info "=========================================="
    log_info "compiler-rt builtins build completed!"
    log_info "=========================================="
}
