#!/bin/sh
# Enable kernel coredumps for debugging
# BusyBox-compatible version
# Usage: ./enable_coredump.sh [dump_dir]

# Default dump directory (on device)
DUMP_DIR="${1:-/mnt/coredumps}"

echo "=== Enabling Kernel Coredumps ==="
echo ""

# Check if running as root
if [ "$(id -u)" -ne 0 ]; then
    echo "Error: This script must be run as root"
    exit 1
fi

# Create dump directory if it doesn't exist
if [ ! -d "$DUMP_DIR" ]; then
    echo "Creating dump directory: $DUMP_DIR"
    mkdir -p "$DUMP_DIR" 2>/dev/null
    if [ $? -ne 0 ]; then
        echo "Warning: Failed to create $DUMP_DIR, using /tmp instead"
        DUMP_DIR="/tmp/coredumps"
        mkdir -p "$DUMP_DIR" 2>/dev/null || {
            echo "Error: Cannot create dump directory"
            exit 1
        }
    fi
fi

echo "Dump directory: $DUMP_DIR"
echo ""

# Enable core dumps for setuid processes
if [ -w /proc/sys/fs/suid_dumpable ]; then
    echo "Enabling core dumps for setuid processes..."
    echo "2" > /proc/sys/fs/suid_dumpable
    echo "  /proc/sys/fs/suid_dumpable = $(cat /proc/sys/fs/suid_dumpable)"
else
    echo "Warning: Cannot write to /proc/sys/fs/suid_dumpable"
fi

# Set core dump pattern with full path
if [ -w /proc/sys/kernel/core_pattern ]; then
    echo "Setting core dump pattern..."
    # Pattern: /mnt/coredumps/core.%e.%p.%t (full path, executable name, PID, timestamp)
    echo "$DUMP_DIR/core.%e.%p.%t" > /proc/sys/kernel/core_pattern
    echo "  /proc/sys/kernel/core_pattern = $(cat /proc/sys/kernel/core_pattern)"
else
    echo "Warning: Cannot write to /proc/sys/kernel/core_pattern"
fi

# Set core dump size limit (unlimited = -1)
if [ -w /proc/sys/kernel/core_pipe_limit ]; then
    echo "Setting core dump pipe limit..."
    echo "4" > /proc/sys/kernel/core_pipe_limit
    echo "  /proc/sys/kernel/core_pipe_limit = $(cat /proc/sys/kernel/core_pipe_limit)"
else
    echo "Warning: Cannot write to /proc/sys/kernel/core_pipe_limit"
fi

# Enable kernel oops/panic logging
if [ -w /proc/sys/kernel/printk ]; then
    echo "Enabling kernel printk logging..."
    # Set console log level (8 = all messages)
    echo "8 4 1 7" > /proc/sys/kernel/printk
    echo "  /proc/sys/kernel/printk = $(cat /proc/sys/kernel/printk)"
else
    echo "Warning: Cannot write to /proc/sys/kernel/printk"
fi

# Enable panic on oops (for debugging)
if [ -w /proc/sys/kernel/panic_on_oops ]; then
    echo "Setting panic_on_oops..."
    echo "1" > /proc/sys/kernel/panic_on_oops
    echo "  /proc/sys/kernel/panic_on_oops = $(cat /proc/sys/kernel/panic_on_oops)"
else
    echo "Warning: Cannot write to /proc/sys/kernel/panic_on_oops"
fi

# Set panic timeout (seconds before reboot on panic)
if [ -w /proc/sys/kernel/panic ]; then
    echo "Setting panic timeout (0 = no reboot)..."
    echo "0" > /proc/sys/kernel/panic
    echo "  /proc/sys/kernel/panic = $(cat /proc/sys/kernel/panic)"
else
    echo "Warning: Cannot write to /proc/sys/kernel/panic"
fi

# Set ulimit for core dumps (unlimited)
echo "Setting ulimit for core dumps..."
ulimit -c unlimited 2>/dev/null
if [ $? -eq 0 ]; then
    echo "  ulimit -c = $(ulimit -c)"
else
    echo "  Warning: Cannot set ulimit (may require shell restart)"
fi

# Try to enable kdump if available (kernel crash dumps)
if command -v kdump >/dev/null 2>&1; then
    echo "kdump is available, attempting to enable..."
    # This would require kdump configuration
    echo "  Note: kdump requires additional configuration"
elif [ -d /sys/kernel/kexec_crash_loaded ]; then
    echo "Kexec crash kernel support detected"
    if [ -r /sys/kernel/kexec_crash_loaded ]; then
        echo "  kexec_crash_loaded = $(cat /sys/kernel/kexec_crash_loaded)"
    fi
else
    echo "Kexec/kdump not available (normal for embedded systems)"
fi

echo ""
echo "=== Summary ==="
echo "Core dumps will be saved to: $DUMP_DIR"
echo ""
echo "To test core dump generation:"
echo "  kill -SEGV \$(pidof onvif-rust)"
echo ""
echo "To view current settings:"
echo "  cat /proc/sys/kernel/core_pattern"
echo "  cat /proc/sys/fs/suid_dumpable"
echo "  ulimit -c"
echo ""
echo "To find core dumps:"
echo "  find $DUMP_DIR -name 'core.*' -ls"
echo ""
echo "Done!"
