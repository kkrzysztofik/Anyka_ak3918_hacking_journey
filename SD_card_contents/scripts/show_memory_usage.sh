#!/bin/sh
# Show processes sorted by RSS (KB), highest first. BusyBox/POSIX sh compatible.
# Usage: ./toprss.sh [N]
# Example: ./toprss.sh 30

TOP="${1:-20}"

# Print header
printf "%10s %8s %s\n" "RSS_KB" "PID" "COMMAND"

# Walk /proc, extract VmRSS (KB), PID, and command, then sort & trim
for p in /proc/[0-9]*; do
    pid=${p#/proc/}

    # RSS in KB (falls back to 0 if missing)
    rss_kb=$(awk '/^VmRSS:/ {print $2; found=1; exit} END{if(!found) print 0}' "$p/status" 2>/dev/null)
    [ -n "$rss_kb" ] || rss_kb=0

    # Command line (NUL-separated) → spaces; fallback to comm; final fallback to [pid]
    cmd=""
    if [ -r "$p/cmdline" ]; then
        cmd=$(tr '\000' ' ' < "$p/cmdline" 2>/dev/null)
    fi
    [ -n "$cmd" ] || cmd=$(cat "$p/comm" 2>/dev/null)
    [ -n "$cmd" ] || cmd="[$pid]"

    printf "%10s %8s %s\n" "$rss_kb" "$pid" "$cmd"
done | sort -nr -k1,1 | head -n "$TOP"
