#!/usr/bin/env bash

# Prepare vendor directory for ONVIF Rust project
# Usage: ./scripts/prepare_vendor.sh
#
# This script sets up the vendor directory structure required for FFI binding
# generation and static linking. It copies headers and static libraries from
# source directories to the vendor directory for build isolation.
#
# The script is idempotent and can be safely run multiple times.

set -euo pipefail

# Get repository root directory
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

# Source and destination paths
ONVIF_INCLUDE_DIR="cross-compile/onvif/include"
REFERENCE_LIBS_DIR="cross-compile/anyka_reference/IOT-ANYKA-PTZdaemon/libs"
VENDOR_DIR="cross-compile/onvif-rust/vendor"
VENDOR_INCLUDE_DIR="$VENDOR_DIR/include"
VENDOR_LIB_DIR="$VENDOR_DIR/lib"

# Critical headers to verify
CRITICAL_HEADERS=(
    "ak_vi.h"
    "ak_venc.h"
    "ak_ai.h"
    "ak_aenc.h"
    "ak_drv_ptz.h"
    "ak_common.h"
    "ak_vpss.h"
    "ak_drv_irled.h"
)

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

echo "=== ONVIF Rust Vendor Setup ==="
echo ""

# Validate source directories exist
if [ ! -d "$ONVIF_INCLUDE_DIR" ]; then
    echo "✗ ERROR: Source include directory not found: $ONVIF_INCLUDE_DIR"
    exit 1
fi

if [ ! -d "$REFERENCE_LIBS_DIR" ]; then
    echo "✗ ERROR: Source libraries directory not found: $REFERENCE_LIBS_DIR"
    exit 1
fi

echo "✓ Source directories validated"
echo ""

# Create vendor directory structure
echo "Creating vendor directory structure..."
mkdir -p "$VENDOR_LIB_DIR"
chmod 755 "$VENDOR_LIB_DIR"
echo "✓ Vendor directories created"
echo ""

# Copy headers from consolidated include directory
echo "Copying header files..."
# Remove existing vendor include directory to ensure clean copy
if [ -d "$VENDOR_INCLUDE_DIR" ]; then
    rm -rf "$VENDOR_INCLUDE_DIR"
fi
mkdir -p "$VENDOR_INCLUDE_DIR"

# Copy all .h files preserving directory structure
# Use absolute paths to avoid path resolution issues
ONVIF_INCLUDE_ABS="$(cd "$ONVIF_INCLUDE_DIR" && pwd)"
VENDOR_INCLUDE_ABS="$(cd "$VENDOR_INCLUDE_DIR" && pwd)"

# Use find with -print0 and read -d '' for robust handling of filenames
while IFS= read -r -d '' file; do
    # Calculate relative path from source directory
    rel_path="${file#$ONVIF_INCLUDE_ABS/}"
    target_path="$VENDOR_INCLUDE_ABS/$rel_path"
    target_dir="$(dirname "$target_path")"
    mkdir -p "$target_dir"
    cp -f "$file" "$target_path"
    chmod 644 "$target_path"
done < <(find "$ONVIF_INCLUDE_ABS" -type f -name "*.h" -print0)

# Count copied headers
COPIED_HEADER_COUNT=$(find "$VENDOR_INCLUDE_DIR" -type f -name "*.h" | wc -l)
echo "✓ Copied $COPIED_HEADER_COUNT header file(s)"
echo ""

# Verify critical headers
echo "Verifying critical headers..."
MISSING_HEADERS=()
for header in "${CRITICAL_HEADERS[@]}"; do
    if [ ! -f "$VENDOR_INCLUDE_DIR/$header" ]; then
        MISSING_HEADERS+=("$header")
    fi
done

if [ ${#MISSING_HEADERS[@]} -gt 0 ]; then
    echo "⚠ WARNING: Missing critical headers:"
    for header in "${MISSING_HEADERS[@]}"; do
        echo "  - $header"
    done
else
    echo "✓ All critical headers present"
fi
echo ""

# Copy static libraries
echo "Copying static libraries..."
COPIED_COUNT=0
MISSING_LIBS=()

for lib in "${REQUIRED_LIBS[@]}"; do
    source_lib="$REFERENCE_LIBS_DIR/$lib"
    dest_lib="$VENDOR_LIB_DIR/$lib"

    if [ ! -f "$source_lib" ]; then
        MISSING_LIBS+=("$lib")
        echo "⚠ WARNING: Library not found in source: $lib"
        continue
    fi

    # Copy library (overwrite if exists)
    cp -f "$source_lib" "$dest_lib"
    chmod 644 "$dest_lib"
    ((COPIED_COUNT++))
done

echo "✓ Copied $COPIED_COUNT library file(s)"
echo ""

# Verify required libraries
if [ ${#MISSING_LIBS[@]} -gt 0 ]; then
    echo "⚠ WARNING: Missing required libraries:"
    for lib in "${MISSING_LIBS[@]}"; do
        echo "  - $lib"
    done
    echo ""
fi

# Summary
echo "=== Setup Complete ==="
echo "Headers copied: $COPIED_HEADER_COUNT"
echo "Libraries copied: $COPIED_COUNT"
echo ""

if [ ${#MISSING_HEADERS[@]} -eq 0 ] && [ ${#MISSING_LIBS[@]} -eq 0 ]; then
    echo "✓ All vendor files are ready"
    echo ""
    echo "You can now build the project with:"
    echo "  cd cross-compile/onvif-rust"
    echo "  cargo build --release --target armv5te-unknown-linux-uclibceabi"
    exit 0
else
    echo "⚠ Setup completed with warnings (see above)"
    echo "Some files may be missing. Build may fail or use stubs."
    exit 1
fi
