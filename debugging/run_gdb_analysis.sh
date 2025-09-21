#!/bin/sh

# GDB analysis script for Docker environment
# Usage:
#   From debugging directory: .\analyze_core_windows.ps1
#   From ONVIF directory: docker run --rm -v ${PWD}:/workspace anyka-cross-compile /workspace/run_gdb_analysis.sh [coredump_file] [binary_file]
#   Direct Docker: See comments in script for manual commands
#   Standalone from ONVIF: ./analyze_coredump_direct.sh
# This script is designed to work both as a standalone script and with the PowerShell wrapper

set -e  # Exit on any error

# Default values
COREDUMP_FILE="core.onvifd_debug.17401.15810"
BINARY_FILE="onvifd_debug"

# Parse command line arguments
if [ $# -ge 1 ]; then
    COREDUMP_FILE="$1"
fi
if [ $# -ge 2 ]; then
    BINARY_FILE="$2"
fi

# Validate input files exist
if [ ! -f "$COREDUMP_FILE" ]; then
    echo "Error: Coredump file '$COREDUMP_FILE' not found!"
    echo "Available core files:"
    ls -la core.* 2>/dev/null || echo "No core files found in current directory"
    exit 1
fi

if [ ! -f "$BINARY_FILE" ]; then
    echo "Error: Binary file '$BINARY_FILE' not found!"
    echo "Available binary files:"
    ls -la onvifd* 2>/dev/null || echo "No binary files found in current directory"
    exit 1
fi

echo "Analyzing coredump: $COREDUMP_FILE"
echo "Using binary: $BINARY_FILE"
echo "Working directory: $(pwd)"
echo ""

# Check if libraries exist
if [ ! -d "/workspace/lib" ]; then
    echo "Error: Library directory '/workspace/lib' not found!"
    echo "This script expects to be run via Docker with volume mount from PowerShell script"
    exit 1
fi

# Count available libraries
LIB_COUNT=$(ls -1 /workspace/lib/*.so* 2>/dev/null | wc -l)
echo "Available ARM libraries: $LIB_COUNT"

# Create symlinks for all required libraries
echo "Setting up library symlinks..."
mkdir -p /lib

# Essential system libraries (try to create symlinks)
for lib in ld-uClibc.so.0 libuClibc-0.9.33.2.so libc.so.0 libdl-0.9.33.2.so libpthread-0.9.33.2.so libm-0.9.33.2.so; do
    if [ -f "/workspace/lib/$lib" ]; then
        ln -sf "/workspace/lib/$lib" "/lib/$lib" 2>/dev/null || echo "Warning: Could not create symlink for $lib"
    else
        echo "Warning: Required library $lib not found in /workspace/lib/"
    fi
done

echo "Library setup complete"
echo ""

# Check if GDB can load the binary
echo "Testing binary loading..."
gdb --batch --ex "file $BINARY_FILE" --ex "quit" "$BINARY_FILE" >/dev/null 2>&1
if [ $? -ne 0 ]; then
    echo "Warning: GDB could not load the binary file properly"
    echo "This may be due to missing architecture support or library issues"
fi

echo "Starting comprehensive GDB analysis..."
echo "This may take a few minutes for large coredumps..."
echo ""

# Run comprehensive GDB analysis with error handling
gdb --batch \
    --ex "set substitute-path /workspace /mnt/anyka_hack/onvif" \
    --ex "set substitute-path /workspace/cross-compile/onvif /mnt/anyka_hack/onvif" \
    --ex "set solib-search-path /workspace/lib:/workspace:/lib:/usr/lib" \
    --ex "file $BINARY_FILE" \
    --ex "core-file $COREDUMP_FILE" \
    --ex "echo === COREDUMP ANALYSIS ===" \
    --ex "echo Coredump file: $COREDUMP_FILE" \
    --ex "echo Binary file: $BINARY_FILE" \
    --ex "echo Analysis date: $(date)" \
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
    "$BINARY_FILE"

# Check exit code
EXIT_CODE=$?
if [ $EXIT_CODE -eq 0 ]; then
    echo ""
    echo "GDB analysis completed successfully"
else
    echo ""
    echo "Warning: GDB analysis completed with exit code $EXIT_CODE"
    echo "This may indicate some issues with symbol loading or library paths"
    echo "Analysis results may still be valid"
fi

echo ""
echo "GDB analysis script completed"
echo ""
echo "=== ALTERNATIVE ANALYSIS METHODS ==="
echo ""
echo "If this script fails, try the direct Docker method from the ONVIF directory:"
echo "cd SD_card_contents/anyka_hack/onvif"
echo "docker run --rm -v \${PWD}:/workspace anyka-cross-compile sh -c \""
echo "mkdir -p /lib &&"
echo "ln -sf /workspace/lib/ld-uClibc.so.0 /lib/ld-uClibc.so.0 &&"
echo "ln -sf /workspace/lib/libuClibc-0.9.33.2.so /lib/libuClibc-0.9.33.2.so &&"
echo "ln -sf /workspace/lib/libc.so.0 /lib/libc.so.0 &&"
echo "ln -sf /workspace/lib/libdl-0.9.33.2.so /lib/libdl.so.0 &&"
echo "ln -sf /workspace/lib/libpthread-0.9.33.2.so /lib/libpthread.so.0 &&"
echo "ln -sf /workspace/lib/libm-0.9.33.2.so /lib/libm.so.0 &&"
echo "gdb --batch --ex 'set substitute-path /workspace /mnt/anyka_hack/onvif' \\"
echo "    --ex 'set solib-search-path /workspace/lib:/workspace:/lib:/usr/lib' \\"
echo "    --ex 'file onvifd_debug' --ex 'core-file $COREDUMP_FILE' \\"
echo "    --ex 'bt full' --ex 'info threads' --ex 'thread apply all bt' \\"
echo "    --ex 'info registers' --ex 'info frame' --ex 'list' \\"
echo "    --ex 'info args' --ex 'info locals' --ex 'info proc mappings' \\"
echo "    --ex 'info sharedlibrary' --ex 'x/20x \\\$pc-40' --ex 'x/20x \\\$sp-40' \\"
echo "    --ex 'quit' onvifd_debug"
echo "\""
