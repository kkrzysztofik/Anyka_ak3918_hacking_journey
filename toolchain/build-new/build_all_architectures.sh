#!/bin/bash
# Build script for all supported architectures
# Builds both ARMv5TE and aarch64 toolchains

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BUILD_DIR="${SCRIPT_DIR}"

# Logging functions
log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_info "=========================================="
log_info "Building toolchains for all architectures"
log_info "=========================================="

# Build ARMv5TE toolchain
log_info ""
log_info "=========================================="
log_info "Step 1: Building ARMv5TE toolchain"
log_info "=========================================="
cd "${BUILD_DIR}"
ARCH=armv5te ./build_toolchain.sh || {
    log_error "ARMv5TE toolchain build failed"
    exit 1
}

# Build aarch64 toolchain
log_info ""
log_info "=========================================="
log_info "Step 2: Building aarch64 toolchain"
log_info "=========================================="
cd "${BUILD_DIR}"
ARCH=aarch64 ./build_toolchain.sh || {
    log_error "aarch64 toolchain build failed"
    exit 1
}

log_info ""
log_info "=========================================="
log_info "Toolchain builds completed successfully!"
log_info "=========================================="
log_info ""
log_info "ARMv5TE toolchain: ../arm-anykav200-crosstool-ng/"
log_info "aarch64 toolchain:  ../aarch64-unknown-linux-gnu-toolchain/"
log_info ""
log_info "Next steps:"
log_info "1. Build LLVM/Clang (if not already built)"
log_info "2. Bootstrap Rust for each architecture:"
log_info "   ARCH=armv5te ./bootstrap_rust.sh"
log_info "   ARCH=aarch64 ./bootstrap_rust.sh"
log_info ""
