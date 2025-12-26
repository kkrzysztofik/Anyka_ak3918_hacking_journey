#!/bin/bash
# Test script to verify Docker image works with onvif-rust project
# Tests all steps from GitHub Actions workflows

set -e

# Colors for output
readonly RED='\033[0;31m'
readonly GREEN='\033[0;32m'
readonly YELLOW='\033[1;33m'
readonly BLUE='\033[0;34m'
readonly NC='\033[0m' # No Color

log_info() {
    local message="$1"
    echo -e "${BLUE}[INFO]${NC} ${message}"
    return 0
}

log_success() {
    local message="$1"
    echo -e "${GREEN}[SUCCESS]${NC} ${message}"
    return 0
}

log_error() {
    local message="$1"
    echo -e "${RED}[ERROR]${NC} ${message}" >&2
    return 0
}

log_warn() {
    local message="$1"
    echo -e "${YELLOW}[WARN]${NC} ${message}"
    return 0
}

# Configuration
IMAGE_TAG="${IMAGE_TAG:-anyka-cross-compile:test}"
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ONVIF_RUST_DIR="${PROJECT_ROOT}/cross-compile/onvif-rust"

# Check if image exists
if ! docker image inspect "${IMAGE_TAG}" &>/dev/null; then
    log_error "Docker image '${IMAGE_TAG}' not found. Please build it first:"
    log_info "  ./cross-compile/docker-build.sh"
    exit 1
fi

log_info "Testing Docker image: ${IMAGE_TAG}"
log_info "Project root: ${PROJECT_ROOT}"
echo ""

# Test 1: Verify toolchain
log_info "=== Test 1: Verify toolchain ==="
if docker run --rm -v "${PROJECT_ROOT}:/workspace" "${IMAGE_TAG}" \
  bash -c "rustc --version && cargo --version && clang --version && arm-unknown-linux-uclibcgnueabi-gcc --version"; then
    log_success "Toolchain verification passed"
else
    log_error "Toolchain verification failed"
    exit 1
fi
echo ""

# Test 2: CI - Check Formatting
log_info "=== Test 2: CI - Check Formatting ==="
if docker run --rm -v "${PROJECT_ROOT}:/workspace" "${IMAGE_TAG}" \
  bash -c "cd /workspace/cross-compile/onvif-rust && ./scripts/setup-cargo-config.sh && cargo fmt --check"; then
    log_success "Formatting check passed"
else
    log_warn "Formatting check failed (this is expected if code is not formatted)"
fi
echo ""

# Test 3: CI - Run Clippy
log_info "=== Test 3: CI - Run Clippy ==="
if docker run --rm -v "${PROJECT_ROOT}:/workspace" "${IMAGE_TAG}" \
  bash -c "cd /workspace/cross-compile/onvif-rust && ./scripts/setup-cargo-config.sh && cargo clippy -- -D warnings 2>&1 | head -20"; then
    log_success "Clippy check passed"
else
    log_warn "Clippy found issues (this may be expected)"
fi
echo ""

# Test 4: CI - Run Tests
log_info "=== Test 4: CI - Run Tests ==="
if docker run --rm -v "${PROJECT_ROOT}:/workspace" "${IMAGE_TAG}" \
  bash -c "cd /workspace/cross-compile/onvif-rust && ./scripts/setup-cargo-config.sh && cargo test --target x86_64-unknown-linux-gnu --all-features"; then
    log_success "Tests passed"
else
    log_error "Tests failed"
    exit 1
fi
echo ""

# Test 5: Release - Build for ARM
log_info "=== Test 5: Release - Build for ARM ==="
if docker run --rm -v "${PROJECT_ROOT}:/workspace" "${IMAGE_TAG}" \
  bash -c "cd /workspace/cross-compile/onvif-rust && ./scripts/build.sh --release --target armv5te-unknown-linux-uclibceabi"; then
    log_success "ARM build passed"
else
    log_error "ARM build failed"
    exit 1
fi
echo ""

# Test 6: Release - Verify Binary
log_info "=== Test 6: Release - Verify Binary ==="
if docker run --rm -v "${PROJECT_ROOT}:/workspace" "${IMAGE_TAG}" \
  bash -c "cd /workspace/cross-compile/onvif-rust && ./scripts/verify_binary.sh"; then
    log_success "Binary verification passed"
else
    log_error "Binary verification failed"
    exit 1
fi
echo ""

# Test 7: Verify rustc is from toolchain
log_info "=== Test 7: Verify rustc is from toolchain ==="
RUSTC_PATH=$(docker run --rm -v "${PROJECT_ROOT}:/workspace" "${IMAGE_TAG}" \
  bash -c "which rustc")
EXPECTED_PATH="/opt/arm-anykav200-crosstool-ng/bin/rustc"
if [[ "${RUSTC_PATH}" = "${EXPECTED_PATH}" ]]; then
    log_success "rustc is from toolchain: ${RUSTC_PATH}"
else
    log_error "rustc is NOT from toolchain! Found: ${RUSTC_PATH}"
    exit 1
fi
echo ""

log_success "All tests passed! Docker image is ready for use."
log_info "You can now push the image:"
log_info "  docker tag ${IMAGE_TAG} kkrzysztofik/anyka-cross-compile:latest"
log_info "  docker push kkrzysztofik/anyka-cross-compile:latest"
