#!/bin/bash
# Build script for ARMv5TE toolchain (Anyka AK3918 cameras)

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
log_info "Building ARMv5TE toolchain"
log_info "=========================================="

cd "${BUILD_DIR}"
./build_toolchain.sh || {
    log_error "ARMv5TE toolchain build failed"
    exit 1
}

log_info ""
log_info "=========================================="
log_info "Toolchain build completed successfully!"
log_info "=========================================="
log_info ""
log_info "ARMv5TE toolchain: ../arm-anykav200-crosstool-ng/"
log_info ""
log_info "Next steps:"
log_info "1. Build LLVM/Clang (if not already built)"
log_info "2. Bootstrap Rust:"
log_info "   ./bootstrap_rust.sh"
log_info ""
