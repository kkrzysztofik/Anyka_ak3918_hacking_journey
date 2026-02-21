#!/usr/bin/env bash

# Prepare vendor directory for ONVIF Rust project (libre_anyka_app only)
# Usage: ./scripts/prepare_vendor.sh
#
# This script uses ONLY libre_anyka_app (older SDK). It does NOT build or use
# anything from the platform tree.
#
# - Copies SDK headers from libre_anyka_app/include to vendor/include.
# - Copies all required .so from libre_anyka_app/lib (or LIBRE_ANYKA_LIBS_PATH)
#   into vendor/lib for dynamic linking. That directory must contain the full
#   set of libs that libre_anyka_app links against (platform + device + app .so).
#
# The script is idempotent and can be safely run multiple times.

set -euo pipefail

# Script directory and project paths
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
WORKSPACE_DIR="$(cd "${SCRIPT_DIR}/../.." && pwd)"

# libre_anyka_app only (no platform paths)
LIBRE_APP_DIR="${WORKSPACE_DIR}/anyka_reference/libre_anyka_app"
LIBRE_APP_LIB="${LIBRE_APP_DIR}/lib"

# Vendor directories
VENDOR_DIR="${PROJECT_DIR}/vendor"
VENDOR_INCLUDE_DIR="${VENDOR_DIR}/include"
VENDOR_LIB_DIR="${VENDOR_DIR}/lib"

# Full set of .so that libre_anyka_app links (build.sh): use this order for copy/check
# Link line: -lakuio -lakispsdk -lakv_encode -lplat_common -lplat_thread -lplat_vi
#            -lplat_vpss -lplat_ipcsrv -lplat_venc_cb -lmpi_venc -lakstreamenc
#            -lakae -lakaudiocodec -lakmedia -lapp_net -lapp_rtsp -lmpi_aed -lmpi_aenc -lplat_ai -lplat_drv
REQUIRED_SO=(
    "libakuio.so"
    "libakispsdk.so"
    "libakv_encode.so"
    "libplat_common.so"
    "libplat_thread.so"
    "libplat_vi.so"
    "libplat_vpss.so"
    "libplat_ipcsrv.so"
    "libplat_venc_cb.so"
    "libmpi_venc.so"
    "libakstreamenc.so"
    "libakae.so"
    "libakaudiocodec.so"
    "libakmedia.so"
    "libapp_net.so"
    "libapp_rtsp.so"
    "libmpi_aed.so"
    "libmpi_aenc.so"
    "libplat_ai.so"
    "libplat_drv.so"
)
# Optional: versioned libakuio (e.g. libakuio.so.3.1.01) accepted as libakuio.so
LIBAKUIO_ALT="libakuio.so.3.1.01"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1" >&2
}

echo ""
log_info "=== ONVIF Rust Vendor Setup (libre_anyka_app only, no platform) ==="
echo ""

# Source of .so: LIBRE_ANYKA_LIBS_PATH overrides; else use libre_anyka_app/lib
LIBS_SOURCE="${LIBRE_ANYKA_LIBS_PATH:-$LIBRE_APP_LIB}"

if [ ! -d "$LIBRE_APP_DIR/include" ]; then
    log_error "libre_anyka_app include not found: $LIBRE_APP_DIR/include"
    exit 1
fi

if [ ! -d "$LIBS_SOURCE" ]; then
    log_error "Libs directory not found: $LIBS_SOURCE"
    log_info "Populate libre_anyka_app/lib/ with the full set of .so (same as used to build libre_anyka_app), or set: export LIBRE_ANYKA_LIBS_PATH=/path/to/dir"
    exit 1
fi

log_success "Source directories validated (libre_anyka_app only)"
echo ""

# --- 1. Copy headers from libre_anyka_app only ---
log_info "Copying SDK headers from libre_anyka_app/include to vendor/include..."
mkdir -p "$VENDOR_INCLUDE_DIR"
cp -f "${LIBRE_APP_DIR}"/include/*.h "$VENDOR_INCLUDE_DIR/" 2>/dev/null || true
log_success "Vendor include populated from libre_anyka_app only"
echo ""

# --- 2. Copy all .so from libre_anyka_app lib (or LIBRE_ANYKA_LIBS_PATH) ---
log_info "Creating vendor lib directory and copying .so from $LIBS_SOURCE..."
mkdir -p "$VENDOR_LIB_DIR"
chmod 755 "$VENDOR_LIB_DIR"
COPIED=0
MISSING=()

for lib in "${REQUIRED_SO[@]}"; do
    src="$LIBS_SOURCE/$lib"
    if [ "$lib" = "libakuio.so" ]; then
        if [ -f "$src" ]; then
            cp -f "$src" "$VENDOR_LIB_DIR/"
            chmod 644 "$VENDOR_LIB_DIR/$lib"
            log_info "Copied: $lib"
            COPIED=$((COPIED + 1))
        elif [ -f "$LIBS_SOURCE/$LIBAKUIO_ALT" ]; then
            cp -f "$LIBS_SOURCE/$LIBAKUIO_ALT" "$VENDOR_LIB_DIR/$LIBAKUIO_ALT"
            chmod 644 "$VENDOR_LIB_DIR/$LIBAKUIO_ALT"
            if [ ! -L "$VENDOR_LIB_DIR/libakuio.so" ]; then
                ( cd "$VENDOR_LIB_DIR" && ln -sf "$LIBAKUIO_ALT" libakuio.so )
            fi
            log_info "Copied: $LIBAKUIO_ALT (as libakuio.so)"
            COPIED=$((COPIED + 1))
        else
            MISSING+=("$lib")
        fi
        continue
    fi
    if [ -f "$src" ]; then
        cp -f "$src" "$VENDOR_LIB_DIR/"
        chmod 644 "$VENDOR_LIB_DIR/$lib"
        log_info "Copied: $lib"
        COPIED=$((COPIED + 1))
    else
        MISSING+=("$lib")
    fi
done

# If we have libakuio.so.3.1.01 but no libakuio.so symlink yet
if [ -f "$VENDOR_LIB_DIR/$LIBAKUIO_ALT" ] && [ ! -L "$VENDOR_LIB_DIR/libakuio.so" ] && [ ! -f "$VENDOR_LIB_DIR/libakuio.so" ]; then
    ( cd "$VENDOR_LIB_DIR" && ln -sf "$LIBAKUIO_ALT" libakuio.so )
    log_info "Symlink: libakuio.so -> $LIBAKUIO_ALT"
fi

if [ ${#MISSING[@]} -gt 0 ]; then
    log_warn "Missing .so: ${MISSING[*]}"
    log_info "Ensure $LIBS_SOURCE contains the full set of libs that libre_anyka_app links against."
fi
log_success "Copied $COPIED .so to vendor/lib"
echo ""

# --- 3. Verify critical .so ---
CRITICAL_SO=("libplat_common.so" "libplat_vi.so" "libmpi_venc.so" "libakuio.so")
MISSING_CRITICAL=0
for lib in "${CRITICAL_SO[@]}"; do
    if [ "$lib" = "libakuio.so" ]; then
        if [ ! -f "$VENDOR_LIB_DIR/libakuio.so" ] && [ ! -L "$VENDOR_LIB_DIR/libakuio.so" ] && [ ! -f "$VENDOR_LIB_DIR/$LIBAKUIO_ALT" ]; then
            log_error "CRITICAL: Missing: libakuio.so (or $LIBAKUIO_ALT)"
            MISSING_CRITICAL=$((MISSING_CRITICAL + 1))
        fi
    elif [ ! -f "$VENDOR_LIB_DIR/$lib" ]; then
        log_error "CRITICAL: Missing: $lib"
        MISSING_CRITICAL=$((MISSING_CRITICAL + 1))
    fi
done

echo "=== Setup Complete ==="
if [ "$MISSING_CRITICAL" -gt 0 ]; then
    log_error "Build completed with CRITICAL errors ($MISSING_CRITICAL critical .so missing)"
    exit 1
fi
log_success "Vendor setup complete (libre_anyka_app only)."
echo ""
log_info "Build the project with:"
log_info "  cd ${PROJECT_DIR}"
log_info "  ./scripts/build.sh --release"
exit 0




