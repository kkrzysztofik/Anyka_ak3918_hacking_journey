#!/bin/bash
# Verification script for aarch64 toolchain
# Tests GCC installation and cross-compilation capability

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
INSTALL_DIR="${SCRIPT_DIR}/../aarch64-unknown-linux-gnu-toolchain"
TARGET_TUPLE="aarch64-unknown-linux-gnu"
SYSROOT="${INSTALL_DIR}/${TARGET_TUPLE}/sysroot"

GCC="${INSTALL_DIR}/bin/${TARGET_TUPLE}-gcc"
GXX="${INSTALL_DIR}/bin/${TARGET_TUPLE}-g++"

if [ ! -f "${GCC}" ]; then
    log_error "GCC not found: ${GCC}"
    exit 1
fi

log_info "aarch64 toolchain verification"
log_info "Installation location: ${INSTALL_DIR}"
log_info "Target: ${TARGET_TUPLE}"
if [ -d "${SYSROOT}" ]; then
    log_info "Sysroot: ${SYSROOT}"
else
    log_warn "Sysroot not found: ${SYSROOT}"
fi

# Test 1: Check GCC version
log_info "Test 1: Checking GCC version..."
"${GCC}" --version

# Test 2: Check target architecture
log_info "Test 2: Checking target architecture..."
MACHINE=$("${GCC}" -dumpmachine)
log_info "Target machine: ${MACHINE}"

if echo "${MACHINE}" | grep -q "aarch64"; then
    log_info "Target architecture is aarch64 - OK"
else
    log_error "Target architecture is not aarch64: ${MACHINE}"
    exit 1
fi

# Test 3: Compile a simple C program
log_info "Test 3: Compiling test C program..."
TEST_DIR=$(mktemp -d)
TEST_SRC="${TEST_DIR}/test.c"
TEST_BIN="${TEST_DIR}/test"

cat > "${TEST_SRC}" << 'EOF'
#include <stdio.h>
int main() {
    printf("Hello from aarch64!\n");
    return 0;
}
EOF

"${GCC}" -o "${TEST_BIN}" "${TEST_SRC}" 2>&1 || {
    log_error "Failed to compile test C program"
    rm -rf "${TEST_DIR}"
    exit 1
}

if [ -f "${TEST_BIN}" ]; then
    log_info "Test binary compiled successfully"

    # Check file type
    if command -v file &> /dev/null; then
        FILE_TYPE=$(file "${TEST_BIN}")
        log_info "Binary type: ${FILE_TYPE}"

        if echo "${FILE_TYPE}" | grep -qi "aarch64\|arm64"; then
            log_info "Binary is aarch64 architecture - OK"
        else
            log_warn "Binary architecture may be incorrect"
        fi
    fi
else
    log_error "Test binary was not created"
    rm -rf "${TEST_DIR}"
    exit 1
fi

# Test 4: Compile a simple C++ program
log_info "Test 4: Compiling test C++ program..."
TEST_CPP_SRC="${TEST_DIR}/test.cpp"
TEST_CPP_BIN="${TEST_DIR}/test_cpp"

cat > "${TEST_CPP_SRC}" << 'EOF'
#include <iostream>
int main() {
    std::cout << "Hello from aarch64 C++!" << std::endl;
    return 0;
}
EOF

if "${GXX}" -o "${TEST_CPP_BIN}" "${TEST_CPP_SRC}" 2>&1; then
    if [ -f "${TEST_CPP_BIN}" ]; then
        log_info "Test C++ binary compiled successfully"
    fi
else
    log_warn "Failed to compile test C++ program (may be expected if libstdc++ not available)"
fi

# Test 5: Check glibc version
log_info "Test 5: Checking glibc version..."
if [ -f "${SYSROOT}/lib/libc.so.6" ] || [ -f "${SYSROOT}/usr/lib/libc.so.6" ]; then
    log_info "glibc library found in sysroot"
else
    log_warn "glibc library not found in sysroot (may be expected if not yet built)"
fi

# Cleanup
rm -rf "${TEST_DIR}"

log_info "=========================================="
log_info "aarch64 toolchain verification completed!"
log_info "=========================================="
log_info "Toolchain is ready to use!"
log_info "GCC: ${GCC}"
log_info "G++: ${GXX}"
