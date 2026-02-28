#!/bin/bash
# Build compiler-rt builtins separately using target GCC cross-compiler
# This is a second-stage build that runs after build_llvm.sh
# Target: ARMv5TE for Anyka AK3918 cameras

set -e  # Exit on error

# Script directory — must be set before sourcing common.sh
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/common.sh"

COMPILER_RT_SRC_DIR="${LLVM_SRC_DIR}/compiler-rt"
COMPILER_RT_BUILD_DIR="${BUILD_DIR}/.build/compiler-rt-${LLVM_VERSION}-arm"

# Global variable to store actual builtins library location
ACTUAL_BUILTINS_LIB=""

# Check dependencies
check_dependencies() {
    log_info "Checking build dependencies..."
    check_deps cmake ninja python3
    require_toolchain
    if [[ ! -d "${SYSROOT}" ]]; then
        log_error "Sysroot not found at: ${SYSROOT}"
        log_error "Please build the GCC toolchain first: ./build_toolchain.sh"
        exit 1
    fi
    if [[ ! -d "${COMPILER_RT_SRC_DIR}" ]]; then
        log_error "Compiler-rt source not found at: ${COMPILER_RT_SRC_DIR}"
        log_error "Please build LLVM first (which downloads the source): ./build_llvm.sh"
        exit 1
    fi
    log_info "All dependencies satisfied"
}

# Configure compiler-rt build
configure_compiler_rt() {
    log_info "Configuring compiler-rt builtins build for ${ARCH}..."

    # Remove any existing build directory to ensure clean configuration
    if [[ -d "${COMPILER_RT_BUILD_DIR}" ]]; then
        log_info "Removing existing build directory for clean configuration..."
        rm -rf "${COMPILER_RT_BUILD_DIR}"
    fi

    # Create build directory
    mkdir -p "${COMPILER_RT_BUILD_DIR}"
    cd "${COMPILER_RT_BUILD_DIR}"

    log_info "Running CMake configuration..."
    log_info "Target triple: ${TARGET_TUPLE}"
    log_info "Sysroot: ${SYSROOT}"
    log_info "Install directory: ${INSTALL_DIR}"
    log_info "Using TARGET compiler (${CROSS_CC}) to build compiler-rt builtins"
    log_info "Compiler-rt will be built for ${TARGET_TUPLE}"

    # Configure compiler-rt as a standalone build
    # We use the target GCC cross-compiler to build the builtins
    # This is a cross-compilation build, so we set CMAKE_SYSTEM_NAME and CMAKE_SYSTEM_PROCESSOR
    local cmake_target_flags
    cmake_target_flags=(
        -DCMAKE_SYSTEM_NAME=Linux
        -DCMAKE_SYSTEM_PROCESSOR=arm
        -DCMAKE_C_COMPILER_TARGET="${TARGET_TUPLE}"
        -DCMAKE_CXX_COMPILER_TARGET="${TARGET_TUPLE}"
    )

    # Configure with CMake
    # IMPORTANT: This is a cross-compilation build, so we use the target GCC
    # and set CMAKE_SYSROOT to the target sysroot
    cmake "${COMPILER_RT_SRC_DIR}" \
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
        -DCOMPILER_RT_BUILD_BUILTINS=ON \
        -DCOMPILER_RT_BUILD_SANITIZERS=OFF \
        -DCOMPILER_RT_BUILD_XRAY=OFF \
        -DCOMPILER_RT_BUILD_LIBFUZZER=OFF \
        -DCOMPILER_RT_BUILD_PROFILE=OFF \
        -DCOMPILER_RT_BUILD_MEMPROF=OFF \
        -DCOMPILER_RT_BUILD_ORC=OFF \
        -DCOMPILER_RT_DEFAULT_TARGET_ONLY=ON \
        -DCOMPILER_RT_USE_BUILTINS_LIBRARY=ON \
        -DCOMPILER_RT_USE_LLVM_UNWINDER=OFF \
        -DCOMPILER_RT_CAN_EXECUTE_TESTS=OFF \
        -DCOMPILER_RT_INCLUDE_TESTS=OFF \
        -DCOMPILER_RT_USE_LIBCXX=OFF \
        -DCOMPILER_RT_BUILD_CRT=ON \
        -DCOMPILER_RT_SYSROOT="${SYSROOT}" \
        -DCOMPILER_RT_OS_DIR="linux" \
        "${cmake_target_flags[@]}" || {
        log_error "CMake configuration failed"
        exit 1
    }

    log_info "CMake configuration completed"
}

# Build compiler-rt
build_compiler_rt() {
    log_info "Building compiler-rt builtins (this may take 30-60 minutes)..."

    cd "${COMPILER_RT_BUILD_DIR}"

    # Determine number of parallel jobs
    local num_jobs=$(nproc)
    log_info "Building with ${num_jobs} parallel jobs..."

    # Build with Ninja
    if ! ninja -j "${num_jobs}"; then
        log_error "Compiler-rt build failed"
        exit 1
    fi

    log_info "Compiler-rt build completed successfully"
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

    # Ensure common subdirectories exist and are writable (especially lib/clang for compiler-rt)
    local subdirs=("bin" "lib" "include" "share" "lib/clang")
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

# Install compiler-rt
install_compiler_rt() {
    log_info "Installing compiler-rt builtins..."

    # Check and fix permissions before installation
    check_install_permissions

    cd "${COMPILER_RT_BUILD_DIR}"

    # Install with Ninja
    if ! ninja install; then
        log_error "Compiler-rt installation failed"
        log_error "If you see permission errors, try:"
        log_error "  sudo chown -R ${USER}:${USER} ${INSTALL_DIR}"
        exit 1
    fi

    log_info "Compiler-rt installed successfully"
}

# Verify installation
verify_installation() {
    log_info "Verifying compiler-rt installation for ARMv5TE..."

    # Check for builtins library in expected location
    local builtins_lib
    builtins_lib="${INSTALL_DIR}/lib/clang/${LLVM_VERSION}/lib/linux/libclang_rt.builtins-arm.a"

    local found_lib=""
    if [[ -f "${builtins_lib}" ]]; then
        found_lib="${builtins_lib}"
        log_info "Builtins library found at expected location: ${found_lib}"
    else
        log_warn "Builtins library not found at expected location: ${builtins_lib}"
        log_warn "Searching for alternative locations..."
        
        # Try to find the library
        found_lib=$(find "${INSTALL_DIR}" -name "libclang_rt.builtins*.a" 2>/dev/null | head -1)
        if [[ -n "${found_lib}" ]]; then
            log_info "Found builtins library at: ${found_lib}"
            log_warn "Note: Library is in a different location than expected"
            log_warn "This is normal when building compiler-rt standalone"
        else
            log_error "Could not find builtins library anywhere in ${INSTALL_DIR}"
            exit 1
        fi
    fi

    if [[ -n "${found_lib}" ]]; then
        log_info "Library size: $(du -h "${found_lib}" | cut -f1)"
        # Store the actual location for the success message (global variable)
        ACTUAL_BUILTINS_LIB="${found_lib}"
    fi

    log_info "Compiler-rt verification completed"
}

# Main execution
main() {
    log_info "Starting compiler-rt builtins build for ARMv5TE"
    log_info "Build directory: ${BUILD_DIR}"
    log_info "Install directory: ${INSTALL_DIR}"
    log_info "Target triple: ${TARGET_TUPLE}"
    log_info "LLVM version: ${LLVM_VERSION}"

    check_dependencies
    configure_compiler_rt
    build_compiler_rt
    install_compiler_rt
    verify_installation

    log_info "=========================================="
    log_info "Compiler-rt builtins build completed successfully!"
    log_info "Installation location: ${INSTALL_DIR}"
    log_info "=========================================="
    log_info "Builtins library installed to:"
    if [[ -n "${ACTUAL_BUILTINS_LIB}" ]]; then
        log_info "  ${ACTUAL_BUILTINS_LIB}"
    else
        log_info "  ${INSTALL_DIR}/lib/clang/${LLVM_VERSION}/lib/linux/libclang_rt.builtins-arm.a"
    fi
    log_info ""
    log_info "You can now bootstrap Rust:"
    log_info "  ./bootstrap_rust.sh"
}

# Run main function
main "$@"
