#!/bin/sh
# Complete thread analysis for vendor-daemon process
# BusyBox-compatible version
# Usage: ./check_threads.sh

# Find PID - BusyBox compatible
DAEMON_PID=$(pidof vendor-daemon.bin 2>/dev/null)
if [ -z "$DAEMON_PID" ]; then
    # Fallback: use ps and grep
    DAEMON_PID=$(ps | grep -v grep | grep vendor-daemon.bin | awk '{print $1}' | head -1)
fi

if [ -z "$DAEMON_PID" ]; then
    echo "Error: vendor-daemon process not found"
    exit 1
fi

echo "=== Thread Analysis for PID $DAEMON_PID ==="
echo ""

for tid in /proc/$DAEMON_PID/task/*; do
    if [ ! -d "$tid" ]; then
        continue
    fi
    
    tid_num=$(basename "$tid")
    
    # Read thread name
    if [ -r "$tid/comm" ]; then
        name=$(cat "$tid/comm" 2>/dev/null)
    else
        name="unknown"
    fi
    
    # Read thread state - BusyBox compatible (use cut if awk fails)
    if [ -r "$tid/stat" ]; then
        state=$(awk '{print $3}' "$tid/stat" 2>/dev/null)
        if [ -z "$state" ]; then
            # Fallback: use cut
            state=$(cut -d' ' -f3 "$tid/stat" 2>/dev/null)
        fi
        utime=$(awk '{print $14}' "$tid/stat" 2>/dev/null)
        if [ -z "$utime" ]; then
            utime=$(cut -d' ' -f14 "$tid/stat" 2>/dev/null)
        fi
        stime=$(awk '{print $15}' "$tid/stat" 2>/dev/null)
        if [ -z "$stime" ]; then
            stime=$(cut -d' ' -f15 "$tid/stat" 2>/dev/null)
        fi
    else
        state="unknown"
        utime="0"
        stime="0"
    fi
    
    # Read wait channel
    if [ -r "$tid/wchan" ]; then
        wchan=$(cat "$tid/wchan" 2>/dev/null)
    else
        wchan="unknown"
    fi
    
    echo "TID: $tid_num"
    echo "  Name: $name"
    echo "  State: $state"
    echo "  CPU Time: user=${utime}ms, sys=${stime}ms"
    echo "  Wait Channel: $wchan"
    
    if [ "$state" = "D" ]; then
        echo "  WARNING: UNINTERRUPTIBLE SLEEP (STUCK)"
        echo "  Stack trace:"
        if [ -r "$tid/stack" ]; then
            head -15 "$tid/stack" 2>/dev/null | sed 's/^/    /'
        else
            echo "    (stack trace not available)"
        fi
    elif [ "$state" = "S" ]; then
        echo "  OK: Interruptible sleep (normal)"
    elif [ "$state" = "R" ]; then
        echo "  OK: Running"
    fi
    echo ""
done
