#!/bin/bash
set -e

# Get the directory where this script is located
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../../" && pwd)"

# Change to script directory to resolve relative paths correctly
cd "$SCRIPT_DIR"

# Find the crosstool in the repo (prefer -ng version if available)
if [ -d "$REPO_ROOT/toolchain/arm-anykav200-crosstool" ]; then
    TOOLCHAIN_PATH="$REPO_ROOT/toolchain/arm-anykav200-crosstool"
else
    echo "Error: Could not find crosstool in $REPO_ROOT/toolchain/"
    exit 1
fi

export PATH="$TOOLCHAIN_PATH/usr/bin:$PATH"

# Add additional include paths from platform/libapp
LIBAPP_INCLUDE="$REPO_ROOT/cross-compile/anyka_reference/platform/libapp/include"
LIBAPP_DVR_SRC="$REPO_ROOT/cross-compile/anyka_reference/platform/libapp/src/dvr"
LIBAPP_LIB="$REPO_ROOT/cross-compile/anyka_reference/platform/libapp/lib"

# Compile the application
arm-anykav200-linux-uclibcgnueabi-gcc main.c -std=c99 -D_POSIX_C_SOURCE=199309L \
    -Iinclude -I"$LIBAPP_INCLUDE" -I"$LIBAPP_DVR_SRC" -Llib -L"$LIBAPP_LIB" \
    -lakuio -lakispsdk -lakv_encode -lplat_common -lplat_thread -lplat_vi \
    -lplat_vpss -lplat_ipcsrv -lplat_venc_cb -lmpi_venc -lakstreamenc -lpthread \
    -lakae -lakaudiocodec -lakmedia -lakv_encode -lapp_net -lapp_rtsp -lmpi_aed \
    -lmpi_aenc -lplat_ai -lplat_drv -o libre_anyka_app
