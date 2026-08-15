#!/bin/sh
# Monitor thread states during shutdown
# BusyBox-compatible version
# Usage: Run this script, then trigger shutdown in another terminal
# Press Ctrl+C to stop monitoring

# Find PID - BusyBox compatible
DAEMON_PID=$(pidof vendor-daemon.bin 2>/dev/null)
if [ -z "$DAEMON_PID" ]; then
    DAEMON_PID=$(ps | grep -v grep | grep vendor-daemon.bin | awk '{print $1}' | head -1)
fi

if [ -z "$DAEMON_PID" ]; then
    echo "Error: vendor-daemon process not found"
    exit 1
fi

echo "Monitoring shutdown for PID $DAEMON_PID..."
echo "Press Ctrl+C to stop"
echo ""

while [ -d "/proc/$DAEMON_PID" ]; do
    # Date - BusyBox compatible
    if command -v date >/dev/null 2>&1; then
        timestamp=$(date +%H:%M:%S 2>/dev/null || echo "N/A")
    else
        timestamp="N/A"
    fi
    echo "=== $timestamp ==="
    
    stuck_count=0
    thread_count=0
    
    for tid in /proc/$DAEMON_PID/task/*; do
        if [ ! -d "$tid" ]; then
            continue
        fi
        
        thread_count=$((thread_count + 1))
        
        # Read state - BusyBox compatible
        if [ -r "$tid/stat" ]; then
            state=$(awk '{print $3}' "$tid/stat" 2>/dev/null)
            if [ -z "$state" ]; then
                state=$(cut -d' ' -f3 "$tid/stat" 2>/dev/null)
            fi
        else
            continue
        fi
        
        if [ "$state" = "D" ]; then
            stuck_count=$((stuck_count + 1))
            tid_num=$(basename "$tid")
            
            if [ -r "$tid/comm" ]; then
                name=$(cat "$tid/comm" 2>/dev/null)
            else
                name="unknown"
            fi
            
            if [ -r "$tid/wchan" ]; then
                wchan=$(cat "$tid/wchan" 2>/dev/null)
            else
                wchan="unknown"
            fi
            
            echo "WARNING STUCK: TID=$tid_num ($name) waiting on $wchan"
        fi
    done
    
    if [ $stuck_count -eq 0 ]; then
        echo "OK: No stuck threads"
    fi
    echo "Total threads: $thread_count"
    echo ""
    sleep 1
done

echo "Process exited"
