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

    # Download LLVM source if needed.
    # GitHub releases use the ".src.tar.xz" suffix for source tarballs.
    if [[ ! -d "${llvm_src}" ]]; then
        log_info "Downloading LLVM ${LLVM_VERSION}..."
        mkdir -p "$(dirname "${llvm_src}")"
        local llvm_tarball="llvm-project-${LLVM_VERSION}.src.tar.xz"
        local llvm_url="https://github.com/llvm/llvm-project/releases/download/llvmorg-${LLVM_VERSION}/${llvm_tarball}"
        cd "${BUILD_DIR}"
        if [[ ! -f "${llvm_tarball}" ]]; then
            log_info "Fetching ${llvm_url}"
            wget "${llvm_url}" -O "${llvm_tarball}" || {
                rm -f "${llvm_tarball}"
                log_error "Failed to download LLVM ${LLVM_VERSION} from: ${llvm_url}"
                exit 1
            }
        fi
        log_info "Extracting LLVM source..."
        mkdir -p "${llvm_src}"
        tar --strip-components=1 -xf "${llvm_tarball}" -C "${llvm_src}"
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
        # Host compiler (native x86-64 gcc/g++) — NOT the ARM cross-compiler.
        # Use full paths: cmake resolves bare names relative to the build
        # directory, causing ar/ranlib to be looked up in the wrong place.
        -DCMAKE_C_COMPILER="$(command -v gcc)"
        -DCMAKE_CXX_COMPILER="$(command -v g++)"
        -DCMAKE_AR="$(command -v ar)"
        -DCMAKE_RANLIB="$(command -v ranlib)"
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
    # ct-ng installs the toolchain with read-only dirs (dr-xr-xr-x) to
    # prevent accidental modification.  We need write permission before
    # LLVM can install alongside it.
    log_info "Ensuring install prefix is writable..."
    chmod -R u+w "${llvm_install}"

    log_info "Installing LLVM..."
    ninja install

    mark_checkpoint "${STAGE_NAME}"

    log_info "=========================================="
    log_info "LLVM/Clang build completed!"
    log_info "Installation: ${INSTALL_DIR}"
    log_info "=========================================="
}
