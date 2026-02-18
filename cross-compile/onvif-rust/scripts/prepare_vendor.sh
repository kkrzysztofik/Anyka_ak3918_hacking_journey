#!/usr/bin/env bash

# Prepare vendor directory for ONVIF Rust project
# Usage: ./scripts/prepare_vendor.sh
#
# This script builds the Anyka platform libraries (libplat, libmpi, uiolib)
# from source and copies the resulting static libraries to the vendor directory
# for Rust FFI integration.
#
# The script is idempotent and can be safely run multiple times.

set -euo pipefail

# Script directory and project paths
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
WORKSPACE_DIR="$(cd "${SCRIPT_DIR}/../.." && pwd)"
REPO_ROOT="$(cd "${WORKSPACE_DIR}/.." && pwd)"

# Relative paths from workspace
PLATFORM_DIR="${WORKSPACE_DIR}/anyka_reference/platform"
UIOLIB_DIR="${PLATFORM_DIR}/uiolib"

# Vendor directories
VENDOR_DIR="${PROJECT_DIR}/vendor"
VENDOR_LIB_DIR="${VENDOR_DIR}/lib"

# Build output locations
PLATFORM_LIBPLAT_DIR="${PLATFORM_DIR}/libplat/lib"
PLATFORM_LIBMPI_DIR="${PLATFORM_DIR}/libmpi/lib"
UIOLIB_BUILD_DIR="${UIOLIB_DIR}/BUILD_libakuio_SO"

# Required libraries (as specified in build.rs)
REQUIRED_LIBS=(
    "libplat_common.a"
    "libplat_thread.a"
    "libplat_vi.a"
    "libplat_vpss.a"
    "libplat_ipcsrv.a"
    "libplat_venc_cb.a"
    "libplat_ai.a"
    "libplat_drv.a"
    "libmpi_venc.a"
    "libmpi_aenc.a"
    "libmpi_aed.a"
    "libakuio.a"
    "libakispsdk.a"
    "libakv_encode.a"
    "libakstreamenc.a"
    "libakaudiocodec.a"
    "libakmedialib.a"
    "libakae.a"
)

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
log_info "=== ONVIF Rust Vendor Setup (Platform Libs) ==="
echo ""

# Validate platform directory exists
if [ ! -d "$PLATFORM_DIR" ]; then
    log_error "Platform directory not found: $PLATFORM_DIR"
    exit 1
fi

if [ ! -f "$PLATFORM_DIR/Makefile" ]; then
    log_error "Platform Makefile not found: $PLATFORM_DIR/Makefile"
    exit 1
fi

if [ ! -d "$UIOLIB_DIR" ]; then
    log_error "uiolib directory not found: $UIOLIB_DIR"
    exit 1
fi

if [ ! -f "$UIOLIB_DIR/Makefile" ]; then
    log_error "uiolib Makefile not found: $UIOLIB_DIR/Makefile"
    exit 1
fi

log_success "Source directories validated"
echo ""

# Create vendor library directory
log_info "Creating vendor directory structure..."
mkdir -p "$VENDOR_LIB_DIR"
chmod 755 "$VENDOR_LIB_DIR"
log_success "Vendor lib directory created: $VENDOR_LIB_DIR"
echo ""

# Build platform libraries (libplat and libmpi)
log_info "Building platform libraries (libplat, libmpi)..."
log_info "Running: make -C $PLATFORM_DIR lib COMPILE_SO=n"
echo ""

if ! make -C "$PLATFORM_DIR" lib COMPILE_SO=n; then
    log_error "Failed to build platform libraries"
    exit 1
fi

log_success "Platform libraries built successfully"
echo ""

# Build uiolib
log_info "Building uiolib..."
log_info "Running: make -C $UIOLIB_DIR"
echo ""

if ! make -C "$UIOLIB_DIR"; then
    log_error "Failed to build uiolib"
    exit 1
fi

log_success "uiolib built successfully"
echo ""

# Copy libraries to vendor
log_info "Copying built libraries to vendor directory..."
echo ""

COPIED_COUNT=0
MISSING_LIBS=()

for lib in "${REQUIRED_LIBS[@]}"; do
    dest_lib="$VENDOR_LIB_DIR/$lib"
    source_lib=""

    # Determine source location based on library name
    if [[ "$lib" == libplat_* ]]; then
        source_lib="$PLATFORM_LIBPLAT_DIR/$lib"
    elif [[ "$lib" == libmpi_* ]]; then
        source_lib="$PLATFORM_LIBMPI_DIR/$lib"
    elif [[ "$lib" == libakuio.a ]]; then
        source_lib="$UIOLIB_BUILD_DIR/$lib"
    else
        # Other SDK libraries (akispsdk, etc.) - not built here
        # These may be in other vendor directories or pre-built
        if [ ! -f "$dest_lib" ]; then
            MISSING_LIBS+=("$lib")
            log_warn "Library not found (will be needed at build time): $lib"
        else
            log_info "Library already in vendor: $lib"
            COPIED_COUNT=$((COPIED_COUNT + 1))
        fi
        continue
    fi

    if [ ! -f "$source_lib" ]; then
        MISSING_LIBS+=("$lib")
        log_warn "Library not found in build output: $source_lib"
        continue
    fi

    # Copy library
    cp -f "$source_lib" "$dest_lib"
    chmod 644 "$dest_lib"
    log_info "Copied: $lib"
    COPIED_COUNT=$((COPIED_COUNT + 1))
done

echo ""
log_success "Copied $COPIED_COUNT library file(s)"
echo ""

# Verify critical libraries were copied
CRITICAL_LIBS=(
    "libplat_common.a"
    "libplat_vi.a"
    "libmpi_venc.a"
    "libakuio.a"
)

MISSING_CRITICAL=0
for lib in "${CRITICAL_LIBS[@]}"; do
    if [ ! -f "$VENDOR_LIB_DIR/$lib" ]; then
        log_error "CRITICAL: Missing library: $lib"
        MISSING_CRITICAL=$((MISSING_CRITICAL + 1))
    fi
done

echo ""
echo "=== Setup Complete ==="
echo "Libraries copied: $COPIED_COUNT"
echo "Total required: ${#REQUIRED_LIBS[@]}"

if [ "$MISSING_CRITICAL" -gt 0 ]; then
    log_error "Build completed with CRITICAL errors ($MISSING_CRITICAL critical libraries missing)"
    exit 1
elif [ ${#MISSING_LIBS[@]} -gt 0 ]; then
    log_warn "Build completed with warnings (${#MISSING_LIBS[@]} libraries missing)"
    echo ""
    log_info "Some non-critical libraries are missing. These may be:"
    log_info "  - Pre-built SDK libraries (libakispsdk.a, etc.)"
    log_info "  - Already in vendor/ from previous builds"
    exit 1
else
    log_success "All vendor libraries are ready!"
    echo ""
    log_info "You can now build the project with:"
    log_info "  cd ${PROJECT_DIR}"
    log_info "  ./scripts/build.sh --release"
    exit 0
fi
