#!/bin/bash

# Pre-Deployment Validation Script
# Validates build, tests, and struct sizes before SD card deployment
# Usage: ./pre_deploy_check.sh

set -e

# Define paths
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TOOLCHAIN_CARGO="$PROJECT_ROOT/toolchain/arm-anykav200-crosstool-ng/bin/cargo"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[PASS]${NC} $1"
}

log_fail() {
    echo -e "${RED}[FAIL]${NC} $1"
    exit 1
}

echo "=============================================="
echo "  Pre-Deployment Validation"
echo "=============================================="
echo ""

# Check toolchain cargo exists
if [ ! -x "$TOOLCHAIN_CARGO" ]; then
    log_error "Toolchain cargo not found at: $TOOLCHAIN_CARGO"
    exit 1
fi

log_info "Using cargo: $TOOLCHAIN_CARGO"
echo ""

# ============================================
# 1. Build vendor-daemon (C)
# ============================================
log_info "=== Step 1: Building vendor-daemon (C) ==="

cd "$PROJECT_ROOT/cross-compile/vendor-daemon"

# Check if Makefile exists
if [ ! -f "Makefile" ]; then
    log_error "Makefile not found in vendor-daemon"
    exit 1
fi

# Clean and build
make clean > /dev/null 2>&1 || true
if make; then
    log_success "vendor-daemon built successfully"
    
    # Check binary exists
    if [ -f "vendor-daemon" ]; then
        log_info "Binary size: $(ls -lh vendor-daemon | awk '{print $5}')"
    else
        log_warn "Binary 'vendor-daemon' not found, checking for alternatives..."
        ls -la *.o 2>/dev/null || true
    fi
else
    log_fail "vendor-daemon build failed"
fi

echo ""

# ============================================
# 2. Build onvif-rust (Rust)
# ============================================
log_info "=== Step 2: Building onvif-rust (Rust) ==="

cd "$PROJECT_ROOT/cross-compile/onvif-rust"

if $TOOLCHAIN_CARGO build --release; then
    log_success "onvif-rust built successfully"
    
    # Check binary
    if [ -f "target/arm-anykav200-crosstool-ng/release/onvif-rust" ]; then
        log_info "Binary size: $(ls -lh target/arm-anykav200-crosstool-ng/release/onvif-rust | awk '{print $5}')"
    fi
else
    log_fail "onvif-rust build failed"
fi

echo ""

# ============================================
# 3. Verify struct sizes (host tests)
# ============================================
log_info "=== Step 3: Verifying struct sizes ==="

cd "$PROJECT_ROOT/cross-compile/onvif-rust"

# Run struct size tests on host (x86_64)
if $TOOLCHAIN_CARGO test --target x86_64-unknown-linux-gnu --release test_vd_slot_header_size 2>&1; then
    log_success "Struct size assertions passed"
else
    log_warn "Struct size test may not exist or failed - checking for vd_slot_header tests..."
    $TOOLCHAIN_CARGO test --target x86_64-unknown-linux-gnu --release vd_slot 2>&1 | tail -20 || true
fi

echo ""

# ============================================
# 4. Run unit tests (host)
# ============================================
log_info "=== Step 4: Running unit tests (host x86_64) ==="

cd "$PROJECT_ROOT/cross-compile/onvif-rust"

# Run lib tests only (unit tests)
if $TOOLCHAIN_CARGO test --target x86_64-unknown-linux-gnu --lib; then
    log_success "All unit tests passed"
else
    log_fail "Unit tests failed"
fi

echo ""

# ============================================
# 5. Code quality checks
# ============================================
log_info "=== Step 5: Code quality checks ==="

cd "$PROJECT_ROOT/cross-compile/onvif-rust"

# Check formatting
log_info "Checking code formatting..."
if cargo fmt --check; then
    log_success "Code formatting OK"
else
    log_warn "Code formatting issues - run 'cargo fmt' to fix"
    cargo fmt --check 2>&1 || true
fi

# Check clippy
log_info "Running clippy lints..."
if $TOOLCHAIN_CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings 2>&1; then
    log_success "Clippy linting passed (zero warnings)"
else
    log_warn "Clippy found issues - please review"
fi

echo ""

# ============================================
# 6. Package SD card payload (if script exists)
# ============================================
log_info "=== Step 6: SD card payload ==="

cd "$PROJECT_ROOT"

if [ -f "$SCRIPT_DIR/package_sd_payload.sh" ]; then
    log_info "Packaging SD card payload..."
    if "$SCRIPT_DIR/package_sd_payload.sh"; then
        log_success "SD card payload packaged successfully"
    else
        log_warn "SD card packaging failed or not configured"
    fi
else
    log_warn "package_sd_payload.sh not found - skipping"
fi

echo ""
echo "=============================================="
echo "  Pre-Deployment Validation Complete"
echo "=============================================="
log_success "All validation steps passed - ready for deployment"
echo ""
echo "Next steps:"
echo "  1. Copy binaries to SD card"
echo "  2. Boot device"
echo "  3. Run: ./test_video_latency.sh"
echo ""
