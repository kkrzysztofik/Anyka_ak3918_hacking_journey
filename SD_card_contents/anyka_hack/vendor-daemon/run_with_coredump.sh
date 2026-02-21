#!/bin/sh
# Wrapper script to run vendor-daemon with coredumps enabled
# BusyBox-compatible version
# Usage: ./run_with_coredump.sh

# Enable core dumps
ulimit -c unlimited 2>/dev/null

# Change to dump directory to ensure core dumps are saved there
# (if core_pattern is relative)
cd /mnt/anyka_hack/vendor-daemon 2>/dev/null || cd /tmp

# Verify core dump settings
if [ -r /proc/sys/kernel/core_pattern ]; then
    pattern=$(cat /proc/sys/kernel/core_pattern)
    echo "Core dump pattern: $pattern"
fi

ulimit_c=$(ulimit -c 2>/dev/null)
if [ "$ulimit_c" = "0" ] || [ -z "$ulimit_c" ]; then
    echo "Warning: Core dumps disabled (ulimit -c = $ulimit_c)"
    echo "Run: sudo ./enable_coredump.sh first"
else
    echo "Core dumps enabled (ulimit -c = $ulimit_c)"
fi

echo ""

# Set library path so vendor-daemon can find SDK shared libs
export LD_LIBRARY_PATH=/mnt/anyka_hack/lib:/mnt/anyka_hack/vendor-daemon/lib:$LD_LIBRARY_PATH

# Run vendor-daemon
exec /mnt/anyka_hack/vendor-daemon/vendor-daemon.bin
