#!/bin/bash
# Build script for ARMv5TE toolchain (Anyka AK3918 cameras)

set -e  # Exit on error

# Script directory — must be set before sourcing common.sh
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/common.sh"

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
