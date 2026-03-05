#!/bin/bash
# Build LLVM/Clang from source with cross-compilation support
# Target: ARMv5TE (armv5te) for Anyka AK3918 cameras

set -e  # Exit on error

# Script directory — must be set before sourcing common.sh
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/common.sh"

LLVM_BUILD_DIR="${BUILD_DIR}/.build/llvm-${LLVM_VERSION}-arm"

# Check dependencies
check_dependencies() {
    log_info "Checking build dependencies..."
    check_deps cmake ninja python3 git curl xz make gcc g++
    require_toolchain
    if [[ ! -d "${SYSROOT}" ]]; then
        log_error "Sysroot not found at: ${SYSROOT}"
        log_error "Please build the GCC toolchain first: ./build_toolchain.sh"
        exit 1
    fi
    log_info "All dependencies satisfied"
}

# Download LLVM source
download_llvm() {
    log_info "Downloading LLVM ${LLVM_VERSION} source..."

    if [[ -d "${LLVM_SRC_DIR}" ]]; then
        log_warn "LLVM source directory exists, skipping download"
        log_info "To re-download, remove: ${LLVM_SRC_DIR}"
        return 0
    fi

    cd "${BUILD_DIR}"

    local llvm_url="https://github.com/llvm/llvm-project/releases/download/llvmorg-${LLVM_VERSION}/llvm-project-${LLVM_VERSION}.src.tar.xz"
    local llvm_tarball="llvm-project-${LLVM_VERSION}.src.tar.xz"

    log_info "Downloading from: ${llvm_url}"
    
    # Create tarballs directory if it doesn't exist
    mkdir -p "${BUILD_DIR}/.build/tarballs"
    cd "${BUILD_DIR}/.build/tarballs"

    if [[ -f "${llvm_tarball}" ]]; then
        log_info "Tarball already exists, skipping download"
    else
        if ! curl -L -o "${llvm_tarball}" "${llvm_url}"; then
            log_error "Failed to download LLVM source"
            exit 1
        fi
        log_info "Download completed"
    fi

    # Extract
    log_info "Extracting LLVM source..."
    cd "${BUILD_DIR}"
    if ! tar -xf ".build/tarballs/${llvm_tarball}"; then
        log_error "Failed to extract LLVM source"
        exit 1
    fi

    # Rename to expected directory name
    if [[ -d "llvm-project-${LLVM_VERSION}.src" ]]; then
        mv "llvm-project-${LLVM_VERSION}.src" "${LLVM_SRC_DIR}"
    fi

    log_info "LLVM source extracted to: ${LLVM_SRC_DIR}"
}

# Configure LLVM build
configure_llvm() {
    log_info "Configuring LLVM build for ARMv5TE..."

    # Remove any existing build directory to ensure clean configuration
    if [[ -d "${LLVM_BUILD_DIR}" ]]; then
        log_info "Removing existing build directory for clean configuration..."
        rm -rf "${LLVM_BUILD_DIR}"
    fi

    # Create build directory
    mkdir -p "${LLVM_BUILD_DIR}"
    cd "${LLVM_BUILD_DIR}"

    # Determine architecture-specific CMake flags
    # IMPORTANT: We are NOT cross-compiling LLVM itself - we're building native LLVM
    # that can generate code for the target architecture. So we should NOT set
    # CMAKE_SYSTEM_NAME or CMAKE_SYSTEM_PROCESSOR, as that makes CMake think we're
    # cross-compiling and adds --sysroot flags.
    local cmake_target_flags
    cmake_target_flags=(
            -DLLVM_TARGET_ARCH=ARM
            -DLLVM_DEFAULT_TARGET_TUPLE=arm-unknown-linux-uclibcgnueabi
        )

    log_info "Running CMake configuration..."
    log_info "Target triple: ${TARGET_TUPLE}"
    log_info "Sysroot: ${SYSROOT} (for runtime libraries only)"
    log_info "Install directory: ${INSTALL_DIR}"
    log_info "Using HOST compiler (gcc/g++) to build LLVM"
    log_info "Using TARGET compiler (${CROSS_CC}) for compiler-rt builtins"
    log_info "LLVM will be configured to TARGET ${TARGET_TUPLE}"

    # Configure with CMake
    # Note: We use HOST compiler (gcc/g++) to build LLVM itself
    # But configure LLVM to generate code for the TARGET architecture
    # IMPORTANT: Do NOT set CMAKE_SYSROOT here - it causes the host compiler
    # to use target headers, which fails. Sysroot is only for runtime libs.
    # We explicitly do NOT set CMAKE_SYSTEM_NAME/CMAKE_SYSTEM_PROCESSOR to avoid
    # CMake thinking we're cross-compiling (which adds --sysroot automatically).
    # Also explicitly unset any sysroot-related variables that might be inherited.
    # Clear any environment variables that might affect CMake's sysroot detection
    unset CMAKE_SYSROOT CMAKE_FIND_ROOT_PATH CMAKE_SYSROOT_COMPILE CMAKE_SYSROOT_LINK
    
    cmake "${LLVM_SRC_DIR}/llvm" \
        -G Ninja \
        -DCMAKE_BUILD_TYPE=Release \
        -DCMAKE_INSTALL_PREFIX="${INSTALL_DIR}" \
        -DCMAKE_C_COMPILER="gcc" \
        -DCMAKE_CXX_COMPILER="g++" \
        -DCMAKE_SYSROOT="" \
        -DCMAKE_FIND_ROOT_PATH="" \
        -DCMAKE_SYSROOT_COMPILE="" \
        -DCMAKE_SYSROOT_LINK="" \
        -DLLVM_ENABLE_PROJECTS="clang;lld;compiler-rt" \
        -DLLVM_ENABLE_RUNTIMES="" \
        -DLLVM_TARGETS_TO_BUILD="${LLVM_TARGET}" \
        -DLLVM_DEFAULT_TARGET_TUPLE="${TARGET_TUPLE}" \
        -DLLVM_ENABLE_ASSERTIONS=OFF \
        -DLLVM_ENABLE_EH=ON \
        -DLLVM_ENABLE_RTTI=ON \
        -DLLVM_ENABLE_TERMINFO=OFF \
        -DLLVM_ENABLE_ZLIB=ON \
        -DLLVM_INCLUDE_EXAMPLES=OFF \
        -DLLVM_INCLUDE_TESTS=OFF \
        -DLLVM_INCLUDE_DOCS=OFF \
        -DLLVM_INCLUDE_BENCHMARKS=OFF \
        -DLLVM_BUILD_LLVM_DYLIB=ON \
        -DLLVM_LINK_LLVM_DYLIB=ON \
        -DLLVM_ENABLE_PIC=ON \
        -DLLVM_ENABLE_BINDINGS=OFF \
        -DCLANG_DEFAULT_LINKER="lld" \
        -DCLANG_DEFAULT_RTLIB="compiler-rt" \
        -DCLANG_DEFAULT_CXX_STDLIB="libstdc++" \
        -DCLANG_DEFAULT_UNWINDLIB="libgcc" \
        -DCOMPILER_RT_BUILD_BUILTINS=OFF \
        -DCOMPILER_RT_BUILD_SANITIZERS=OFF \
        -DCOMPILER_RT_BUILD_XRAY=OFF \
        -DCOMPILER_RT_BUILD_LIBFUZZER=OFF \
        -DCOMPILER_RT_BUILD_PROFILE=OFF \
        -DCOMPILER_RT_BUILD_MEMPROF=OFF \
        -DCOMPILER_RT_BUILD_ORC=OFF \
        -DCOMPILER_RT_DEFAULT_TARGET_ONLY=OFF \
        -DCOMPILER_RT_USE_BUILTINS_LIBRARY=OFF \
        -DCOMPILER_RT_USE_LLVM_UNWINDER=OFF \
        -DCOMPILER_RT_CAN_EXECUTE_TESTS=OFF \
        -DCOMPILER_RT_INCLUDE_TESTS=OFF \
        -DCOMPILER_RT_USE_LIBCXX=OFF \
        -DCOMPILER_RT_BUILD_CRT=OFF \
        -DCOMPILER_RT_SYSROOT="${SYSROOT}" \
        -DCMAKE_C_COMPILER_TARGET="${TARGET_TUPLE}" \
        -DCMAKE_CXX_COMPILER_TARGET="${TARGET_TUPLE}" \
        "${cmake_target_flags[@]}" || {
        log_error "CMake configuration failed"
        exit 1
    }

    log_info "CMake configuration completed"
}

# Build LLVM
build_llvm() {
    log_info "Building LLVM (this will take 2-4 hours)..."

    cd "${LLVM_BUILD_DIR}"

    # Determine number of parallel jobs
    local num_jobs=$(nproc)
    log_info "Building with ${num_jobs} parallel jobs..."

    # Build with Ninja
    if ! ninja -j "${num_jobs}"; then
        log_error "LLVM build failed"
        exit 1
    fi

    log_info "LLVM build completed successfully"
}

# Check and fix installation directory permissions
check_install_permissions() {
    log_info "Checking installation directory permissions..."

    # Check if installation directory exists
    if [[ ! -d "${INSTALL_DIR}" ]]; then
        log_info "Creating installation directory: ${INSTALL_DIR}"
        mkdir -p "${INSTALL_DIR}" || {
            log_error "Failed to create installation directory: ${INSTALL_DIR}"
            log_error "You may need to run: sudo mkdir -p ${INSTALL_DIR}"
            exit 1
        }
    fi

    # Always fix permissions recursively to ensure everything is writable
    # This is safer than just checking, since directories might have been created by root
    if command -v sudo &> /dev/null; then
        log_info "Fixing ownership and permissions recursively for ${INSTALL_DIR}..."
        if sudo chown -R "${USER}:${USER}" "${INSTALL_DIR}" 2>/dev/null && \
           sudo chmod -R u+w "${INSTALL_DIR}" 2>/dev/null; then
            log_info "Ownership and permissions fixed successfully"
        else
            log_warn "Could not fix permissions with sudo (may require password)"
            log_warn "Checking if directory is already writable..."
        fi
    fi

    # Verify we can actually write by creating a test file
    local test_file="${INSTALL_DIR}/.write_test_$$"
    if touch "${test_file}" 2>/dev/null && rm -f "${test_file}" 2>/dev/null; then
        log_info "Write test passed - installation directory is writable"
    else
        log_error "Cannot write to installation directory: ${INSTALL_DIR}"
        log_error "Please fix permissions manually:"
        log_error "  sudo chown -R ${USER}:${USER} ${INSTALL_DIR}"
        log_error "Or if you don't have sudo access, contact your system administrator"
        exit 1
    fi

    # Ensure common subdirectories exist and are writable
    local subdirs=("bin" "lib" "include" "share")
    for subdir in "${subdirs[@]}"; do
        local dir_path="${INSTALL_DIR}/${subdir}"
        if [[ ! -d "${dir_path}" ]]; then
            log_info "Creating subdirectory: ${dir_path}"
            mkdir -p "${dir_path}" || {
                log_error "Failed to create subdirectory: ${dir_path}"
                exit 1
            }
        fi
        
        # Test write to subdirectory
        local test_subfile="${dir_path}/.write_test_$$"
        if ! touch "${test_subfile}" 2>/dev/null || ! rm -f "${test_subfile}" 2>/dev/null; then
            log_error "Cannot write to subdirectory: ${dir_path}"
            if command -v sudo &> /dev/null; then
                log_info "Attempting to fix permissions for ${dir_path}..."
                sudo chown -R "${USER}:${USER}" "${dir_path}" && \
                sudo chmod -R u+w "${dir_path}" || {
                    log_error "Failed to fix permissions for ${dir_path}"
                    log_error "Please run: sudo chown -R ${USER}:${USER} ${dir_path} && sudo chmod -R u+w ${dir_path}"
                    exit 1
                }
            else
                log_error "Please fix permissions manually:"
                log_error "  sudo chown -R ${USER}:${USER} ${dir_path}"
                exit 1
            fi
        fi
    done

    log_info "Installation directory permissions verified and OK"
}

# Install LLVM
install_llvm() {
    log_info "Installing LLVM..."

    # Check and fix permissions before installation
    check_install_permissions

    cd "${LLVM_BUILD_DIR}"

    # Install with Ninja
    if ! ninja install; then
        log_error "LLVM installation failed"
        log_error "If you see permission errors, try:"
        log_error "  sudo chown -R ${USER}:${USER} ${INSTALL_DIR}"
        exit 1
    fi

    log_info "LLVM installed successfully"
}

# Verify installation
verify_installation() {
    log_info "Verifying LLVM installation for ARMv5TE..."

    local llvm_config="${INSTALL_DIR}/bin/llvm-config"
    local clang="${INSTALL_DIR}/bin/clang"
    local clangxx="${INSTALL_DIR}/bin/clang++"
    local lld="${INSTALL_DIR}/bin/lld"

    if [[ ! -f "${llvm_config}" ]]; then
        log_error "llvm-config not found at: ${llvm_config}"
        exit 1
    fi

    if [[ ! -f "${clang}" ]]; then
        log_error "clang not found at: ${clang}"
        exit 1
    fi

    log_info "Testing llvm-config version..."
    "${llvm_config}" --version

    log_info "Testing clang version..."
    "${clang}" --version

    log_info "Testing target architecture..."
    local clang_target=$("${clang}" -dumpmachine 2>/dev/null || echo "unknown")
    log_info "Clang target: ${clang_target}"

    if [[ -f "${clangxx}" ]]; then
        log_info "Testing clang++ version..."
        "${clangxx}" --version
    fi

    if [[ -f "${lld}" ]]; then
        log_info "Testing lld version..."
        "${lld}" --version 2>/dev/null || "${lld}" -v 2>/dev/null || true
    fi

    log_info "LLVM verification completed"
}

# Main execution
main() {
    log_info "Starting LLVM build for ARMv5TE"
    log_info "Build directory: ${BUILD_DIR}"
    log_info "Install directory: ${INSTALL_DIR}"
    log_info "Target triple: ${TARGET_TUPLE}"
    log_info "LLVM version: ${LLVM_VERSION}"

    check_dependencies
    download_llvm
    configure_llvm
    build_llvm
    install_llvm
    verify_installation

    log_info "=========================================="
    log_info "LLVM build completed successfully!"
    log_info "Installation location: ${INSTALL_DIR}"
    log_info "=========================================="
    log_info "LLVM tools available at:"
    log_info "  ${INSTALL_DIR}/bin/clang"
    log_info "  ${INSTALL_DIR}/bin/clang++"
    log_info "  ${INSTALL_DIR}/bin/llvm-config"
    log_info "  ${INSTALL_DIR}/bin/lld"
    log_info ""
    log_info "You can now bootstrap Rust:"
    log_info "  ./bootstrap_rust.sh"
}

# Run main function
main "$@"
