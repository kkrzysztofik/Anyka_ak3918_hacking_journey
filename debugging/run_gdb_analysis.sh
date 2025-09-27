#!/bin/bash

# Simplified GDB analysis script for ONVIF coredumps
# Usage: ./run_gdb_analysis_simple.sh [coredump_file] [binary_file]
# Example: ./run_gdb_analysis_simple.sh core.onvifd_debug.18851 onvifd_debug

set -e

# Default values
COREDUMP_FILE="core.onvifd_debug.18851"
BINARY_FILE="onvifd_debug"

# Parse arguments
if [ $# -ge 1 ]; then
    COREDUMP_FILE="$1"
fi
if [ $# -ge 2 ]; then
    BINARY_FILE="$2"
fi

# Get script location and project root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Set paths
ONVIF_DIR="$PROJECT_ROOT/cross-compile/onvif"
SD_LIB_DIR="$PROJECT_ROOT/SD_card_contents/anyka_hack/onvif/lib"
BINARY_DIR="$ONVIF_DIR/out"
COREDUMP_DIR="$SCRIPT_DIR/coredump"

# Set up shared library search path for GDB
TOOLCHAIN_LIB_DIR="$PROJECT_ROOT/toolchain/arm-anykav200-crosstool/usr/lib"
SOLIB_SEARCH_PATH="$SD_LIB_DIR:$ONVIF_DIR/out:$TOOLCHAIN_LIB_DIR"

# Find coredump file
if [ -f "$COREDUMP_FILE" ]; then
    COREDUMP_PATH="$COREDUMP_FILE"
elif [ -f "$COREDUMP_DIR/$COREDUMP_FILE" ]; then
    COREDUMP_PATH="$COREDUMP_DIR/$COREDUMP_FILE"
else
    echo "Error: Coredump file '$COREDUMP_FILE' not found!"
    exit 1
fi

# Find binary file
if [ -f "$BINARY_FILE" ]; then
    BINARY_PATH="$BINARY_FILE"
elif [ -f "$BINARY_DIR/$BINARY_FILE" ]; then
    BINARY_PATH="$BINARY_DIR/$BINARY_FILE"
else
    echo "Error: Binary file '$BINARY_FILE' not found!"
    exit 1
fi

# Check for GDB
HOST_GDB="$PROJECT_ROOT/toolchain/arm-anykav200-crosstool/usr/bin/arm-anykav200-linux-uclibcgnueabi-gdb"
SYSTEM_GDB="gdb"

# Set up library paths for 32-bit GDB
PYTHON_LIB_PATH="/usr/local/python2.6/lib"
I386_LIB_PATH="/usr/lib/i386-linux-gnu"
GDB_LIBRARY_PATH="$PYTHON_LIB_PATH:$I386_LIB_PATH"

if [ -f "$HOST_GDB" ]; then
    GDB_CMD="$HOST_GDB"
    GDB_ENV="LD_LIBRARY_PATH=$GDB_LIBRARY_PATH"
    echo "Using host GDB from toolchain with 32-bit library support"
    echo "Library path: $GDB_LIBRARY_PATH"
elif command -v $SYSTEM_GDB &> /dev/null; then
    GDB_CMD="$SYSTEM_GDB"
    GDB_ENV=""
    echo "Using system GDB (may have limited ARM support)"
else
    echo "Error: No GDB found!"
    exit 1
fi

echo "=== Simplified GDB Core Dump Analysis ==="
echo "Coredump: $COREDUMP_PATH"
echo "Binary: $BINARY_PATH"
echo "Libraries: $SD_LIB_DIR"
echo ""

# Run GDB analysis
echo "Running GDB analysis..."
if [ -n "$GDB_ENV" ]; then
    env $GDB_ENV $GDB_CMD --batch \
        --ex "set substitute-path /workspace $ONVIF_DIR" \
        --ex "set substitute-path /mnt/anyka_hack/onvif $ONVIF_DIR" \
        --ex "set substitute-path /workspace/cross-compile/onvif $ONVIF_DIR" \
        --ex "set solib-search-path $SOLIB_SEARCH_PATH" \
        --ex "file $BINARY_PATH" \
        --ex "core-file $COREDUMP_PATH" \
        --ex "echo === COREDUMP ANALYSIS ===" \
        --ex "echo Coredump file: $COREDUMP_PATH" \
        --ex "echo Binary file: $BINARY_PATH" \
        --ex "echo Analysis date: $(date)" \
        --ex "echo Project root: $PROJECT_ROOT" \
        --ex "echo" \
        --ex "bt full" \
        --ex "echo" \
        --ex "echo === THREAD ANALYSIS ===" \
        --ex "info threads" \
        --ex "echo" \
        --ex "thread apply all bt" \
        --ex "echo" \
        --ex "echo === REGISTER ANALYSIS ===" \
        --ex "info registers" \
        --ex "echo" \
        --ex "echo === FRAME ANALYSIS ===" \
        --ex "info frame" \
        --ex "echo" \
        --ex "echo === SOURCE CONTEXT ===" \
        --ex "list" \
        --ex "echo" \
        --ex "echo === ARGUMENTS AND LOCALS ===" \
        --ex "info args" \
        --ex "info locals" \
        --ex "echo" \
        --ex "echo === MEMORY ANALYSIS ===" \
        --ex "info proc mappings" \
        --ex "echo" \
        --ex "echo === SHARED LIBRARIES ===" \
        --ex "info sharedlibrary" \
        --ex "echo" \
        --ex "echo === MEMORY DUMP ===" \
        --ex "x/20x \$pc-40" \
        --ex "x/20x \$sp-40" \
        --ex "echo" \
        --ex "echo === ANALYSIS COMPLETE ===" \
        --ex "quit" \
        "$BINARY_PATH"
else
    $GDB_CMD --batch \
        --ex "set substitute-path /workspace $ONVIF_DIR" \
        --ex "set substitute-path /mnt/anyka_hack/onvif $ONVIF_DIR" \
        --ex "set substitute-path /workspace/cross-compile/onvif $ONVIF_DIR" \
        --ex "set solib-search-path $SOLIB_SEARCH_PATH" \
        --ex "file $BINARY_PATH" \
        --ex "core-file $COREDUMP_PATH" \
        --ex "echo === COREDUMP ANALYSIS ===" \
        --ex "echo Coredump file: $COREDUMP_PATH" \
        --ex "echo Binary file: $BINARY_PATH" \
        --ex "echo Analysis date: $(date)" \
        --ex "echo Project root: $PROJECT_ROOT" \
        --ex "echo" \
        --ex "bt full" \
        --ex "echo" \
        --ex "echo === THREAD ANALYSIS ===" \
        --ex "info threads" \
        --ex "echo" \
        --ex "thread apply all bt" \
        --ex "echo" \
        --ex "echo === REGISTER ANALYSIS ===" \
        --ex "info registers" \
        --ex "echo" \
        --ex "echo === FRAME ANALYSIS ===" \
        --ex "info frame" \
        --ex "echo" \
        --ex "echo === SOURCE CONTEXT ===" \
        --ex "list" \
        --ex "echo" \
        --ex "echo === ARGUMENTS AND LOCALS ===" \
        --ex "info args" \
        --ex "info locals" \
        --ex "echo" \
        --ex "echo === MEMORY ANALYSIS ===" \
        --ex "info proc mappings" \
        --ex "echo" \
        --ex "echo === SHARED LIBRARIES ===" \
        --ex "info sharedlibrary" \
        --ex "echo" \
        --ex "echo === MEMORY DUMP ===" \
        --ex "x/20x \$pc-40" \
        --ex "x/20x \$sp-40" \
        --ex "echo" \
        --ex "echo === ANALYSIS COMPLETE ===" \
        --ex "quit" \
        "$BINARY_PATH"
fi

echo ""
echo "=== Analysis Complete ==="
echo "For interactive debugging:"
if [ -n "$GDB_ENV" ]; then
    echo "  env $GDB_ENV $GDB_CMD $BINARY_PATH"
else
    echo "  $GDB_CMD $BINARY_PATH"
fi
echo "  (gdb) set solib-search-path $SOLIB_SEARCH_PATH"
echo "  (gdb) core-file $COREDUMP_PATH"
echo "  (gdb) bt full"
