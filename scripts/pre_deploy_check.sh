#!/bin/bash

# Pre-Deployment Validation Script
# Validates build, tests, and struct sizes before SD card deployment
# Usage: ./pre_deploy_check.sh [--strict]
# Environment: STRICT_MODE=1 same as --strict (fail if SD packaging is missing or errors)

set -euo pipefail

WARN_COUNT=0
STRICT_MODE="${STRICT_MODE:-0}"
for _pre_arg in "$@"; do
    if [[ "${_pre_arg}" == "--strict" ]]; then
        STRICT_MODE=1
    fi
done
PACKAGING_FAILED=0

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/common.sh
source "${SCRIPT_DIR}/common.sh"
PROJECT_ROOT="${ANYKA_REPO_ROOT}"
TOOLCHAIN_CARGO="${ANYKA_CARGO}"

log_warn() {
    WARN_COUNT=$((WARN_COUNT + 1))
    echo -e "${YELLOW}[WARN]${NC} $1" >&2
}

log_fail() {
    echo -e "${RED}[FAIL]${NC} $1" >&2
    exit 1
}

echo "=============================================="
echo "  Pre-Deployment Validation"
echo "=============================================="
echo ""

anyka_require_vendored_cargo
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
        log_error "Binary 'vendor-daemon' not found after successful build — expected ./vendor-daemon"
        exit 1
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

if "$TOOLCHAIN_CARGO" build --release --target arm-anykav200-crosstool-ng; then
    log_success "onvif-rust built successfully"
    
    # Check binary
    if [ -f "target/arm-anykav200-crosstool-ng/release/onvif-rust" ]; then
        log_info "Binary size: $(ls -lh target/arm-anykav200-crosstool-ng/release/onvif-rust | awk '{print $5}')"
    else
        log_error "Release binary not found at target/arm-anykav200-crosstool-ng/release/onvif-rust — likely target/config mismatch; run: $TOOLCHAIN_CARGO build --release --target arm-anykav200-crosstool-ng"
        exit 1
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
if "$TOOLCHAIN_CARGO" test --target x86_64-unknown-linux-gnu --release test_vd_slot_header_size 2>&1; then
    log_success "Struct size assertions passed"
else
    log_fail "Struct size test test_vd_slot_header_size failed — fix layout/C header alignment before deploy"
fi

echo ""

# ============================================
# 4. Run unit tests (host)
# ============================================
log_info "=== Step 4: Running unit tests (host x86_64) ==="

cd "$PROJECT_ROOT/cross-compile/onvif-rust"

# Run lib tests only (unit tests)
if "$TOOLCHAIN_CARGO" test --target x86_64-unknown-linux-gnu --lib; then
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
if "$TOOLCHAIN_CARGO" fmt --check; then
    log_success "Code formatting OK"
else
    log_fail "Code formatting failed — run '$TOOLCHAIN_CARGO fmt' and re-check"
fi

# Check clippy
log_info "Running clippy lints..."
if "$TOOLCHAIN_CARGO" clippy --target x86_64-unknown-linux-gnu -- -D warnings 2>&1; then
    log_success "Clippy linting passed (zero warnings)"
else
    log_fail "Clippy found issues (-D warnings) — fix before deploy"
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
        PACKAGING_FAILED=1
    fi
else
    log_warn "package_sd_payload.sh not found - skipping"
    PACKAGING_FAILED=1
fi

echo ""
echo "=============================================="
echo "  Pre-Deployment Validation Complete"
echo "=============================================="
if [[ "${STRICT_MODE}" -eq 1 && "${PACKAGING_FAILED}" -eq 1 ]]; then
    log_error "Strict mode: SD card packaging is required (script must exist and succeed). Re-run without --strict or fix packaging."
    exit 1
fi
if [ "$WARN_COUNT" -gt 0 ]; then
    log_warn "Completed with $WARN_COUNT warning(s) — review messages above before deployment."
else
    log_success "All validation steps passed - ready for deployment"
fi
echo ""
echo "Next steps:"
echo "  1. Copy binaries to SD card"
echo "  2. Boot device"
echo "  3. Run: ./test_video_latency.sh"
echo ""
