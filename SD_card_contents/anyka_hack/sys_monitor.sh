#!/bin/sh
#
# sys_monitor.sh - System resource monitoring utility
#
# Description:
#   Monitors system resources including CPU usage, memory, load average,
#   process count, and disk space. Logs metrics every minute to help
#   identify performance issues and resource constraints.
#
# Usage:
#   ./sys_monitor.sh
#   or call from gergehack.sh as background service
#
# Environment Variables:
#   DEBUG       - Enable debug logging when set to 1 (default: 0)
#   LOG_DIR     - Override log directory (default: /mnt/logs)
#   LOG_FILE    - Override log file name (default: sys_monitor.log)
#
# Metrics Collected:
#   - CPU usage percentage (calculated from /proc/stat)
#   - Free memory in KB (MemAvailable or MemFree)
#   - Load average (1-minute)
#   - Process count (numeric PIDs in /proc)
#   - Disk space for key mount points (/mnt, /tmp, /data, /etc/jffs2, /)
#
# Features:
#   - Robust error handling for missing /proc files
#   - Conditional debug logging to reduce log volume
#   - PID file management to prevent duplicates
#   - Graceful signal handling and cleanup
#   - Fallback mechanisms for log directory access
#
# Dependencies:
#   - init_logs.sh (sourced automatically)
#   - /proc filesystem for system metrics
#   - awk, head, wc, ls, df utilities
#   - /mnt/tmp directory for temporary files
#
# Author: Anyka Hack Project
# Version: 2.0
# Last Modified: $(date '+%Y-%m-%d')

# =============================================================================
# GLOBAL VARIABLES AND CONFIGURATION
# =============================================================================

LOG_DIR=${LOG_DIR:-/mnt/logs}
LOG_FILE=${LOG_FILE:-sys_monitor.log}

# =============================================================================
# FUNCTION DEFINITIONS
# =============================================================================

# Debug function to help troubleshoot (only logs if DEBUG=1)
debug_log() {
  if [ "${DEBUG:-0}" = "1" ]; then
    echo "$(date '+%Y-%m-%d %H:%M:%S') [DEBUG] $*" >> "$LOG_DIR/$LOG_FILE"
  fi
}

cleanup() {
  echo "$(date '+%Y-%m-%d %H:%M:%S') [INFO] sys_monitor stopping pid=$$" >> "$LOG_DIR/$LOG_FILE"
  rm -f /mnt/tmp/sys_monitor.pid /mnt/tmp/cpu1.$$ /mnt/tmp/cpu2.$$ 2>/dev/null
  exit 0
}

# Calculate CPU usage percentage
calculate_cpu_usage() {
  local cpu1_file="$1"
  local cpu2_file="$2"
  
  if [ ! -f "$cpu1_file" ] || [ ! -f "$cpu2_file" ]; then
    debug_log "CPU sample files missing"
    echo "0.0"
    return 1
  fi
  
  # Extract CPU times from /proc/stat (user, nice, system, idle, iowait, irq, softirq, steal)
  local cpu1_times=$(awk '{for(i=2;i<=NF;i++) total+=$i; idle=$5; print total,idle; exit}' "$cpu1_file" 2>/dev/null)
  local cpu2_times=$(awk '{for(i=2;i<=NF;i++) total+=$i; idle=$5; print total,idle; exit}' "$cpu2_file" 2>/dev/null)
  
  if [ -z "$cpu1_times" ] || [ -z "$cpu2_times" ]; then
    debug_log "Failed to extract CPU times"
    echo "0.0"
    return 1
  fi
  
  local total1=$(echo "$cpu1_times" | cut -d' ' -f1)
  local idle1=$(echo "$cpu1_times" | cut -d' ' -f2)
  local total2=$(echo "$cpu2_times" | cut -d' ' -f1)
  local idle2=$(echo "$cpu2_times" | cut -d' ' -f2)
  
  if [ -z "$total1" ] || [ -z "$idle1" ] || [ -z "$total2" ] || [ -z "$idle2" ]; then
    debug_log "Invalid CPU time values"
    echo "0.0"
    return 1
  fi
  
  local total_diff=$((total2 - total1))
  local idle_diff=$((idle2 - idle1))
  
  if [ "$total_diff" -gt 0 ]; then
    local cpu_usage=$(( (total_diff - idle_diff) * 100 / total_diff ))
    echo "$cpu_usage.0"
  else
    echo "0.0"
  fi
}

# =============================================================================
# EXECUTION SECTION
# =============================================================================

# Initialize log directories if available
[ -f /mnt/anyka_hack/init_logs.sh ] && . /mnt/anyka_hack/init_logs.sh

# Ensure preferred log dir exists (attempt creation if missing)
if [ ! -d "$LOG_DIR" ]; then
  mkdir -p "$LOG_DIR" 2>/dev/null || true
fi

# Primary write test; if it fails try one more time after mkdir
if ! touch "$LOG_DIR/$LOG_FILE" 2>/dev/null; then
  # Retry once after brief delay in case /mnt just mounted
  sleep 1
  touch "$LOG_DIR/$LOG_FILE" 2>/dev/null || {
    # Final fallback to /mnt/tmp (created earlier by caller / gergehack) to avoid ram-only /tmp
    LOG_DIR="/mnt/tmp"
    mkdir -p "$LOG_DIR" 2>/dev/null || true
    LOG_FILE="sys_monitor.log"
    echo "$(date '+%Y-%m-%d %H:%M:%S') [WARN] Falling back to $LOG_DIR/$LOG_FILE (original log dir not writable)" >&2
  }
fi

echo "$(date '+%Y-%m-%d %H:%M:%S') [INFO] sys_monitor starting pid=$$, LOG_DIR=$LOG_DIR" >> "$LOG_DIR/$LOG_FILE"

# Check essential /proc files
debug_log "Checking /proc filesystem availability..."
for proc_file in /proc/stat /proc/meminfo /proc/loadavg /proc/mounts; do
  if [ -r "$proc_file" ]; then
    debug_log "$proc_file is readable"
  else
    debug_log "ERROR: $proc_file is not readable"
  fi
done

# Set up signal handlers
trap cleanup INT TERM EXIT

# Create temporary directory and PID file
mkdir -p /mnt/tmp 2>/dev/null || true
echo $$ > /mnt/tmp/sys_monitor.pid
debug_log "PID file created: /mnt/tmp/sys_monitor.pid"

# Test basic functionality before entering main loop
debug_log "Testing basic functionality..."

# Test /proc/stat read
if head -n1 /proc/stat > /mnt/tmp/test_cpu.$$ 2>/dev/null; then
  debug_log "/proc/stat read test: OK"
  rm -f /mnt/tmp/test_cpu.$$
else
  debug_log "ERROR: Cannot read /proc/stat"
fi

# Test /proc/meminfo read  
if awk '/MemFree/ {print $2; exit}' /proc/meminfo > /mnt/tmp/test_mem.$$ 2>/dev/null; then
  debug_log "/proc/meminfo read test: OK"
  rm -f /mnt/tmp/test_mem.$$
else
  debug_log "ERROR: Cannot read /proc/meminfo"
fi

debug_log "Entering main monitoring loop..."

while true; do
  debug_log "Starting monitoring cycle"
  
  # Take two samples one second apart to compute recent CPU usage
  if ! head -n1 /proc/stat > /mnt/tmp/cpu1.$$ 2>/dev/null; then
    echo "$(date '+%Y-%m-%d %H:%M:%S') [ERROR] Failed to read /proc/stat (first sample)" >> "$LOG_DIR/$LOG_FILE"
    debug_log "Failed to read /proc/stat (first sample)"
    sleep 60
    continue
  fi
  
  sleep 1
  
  if ! head -n1 /proc/stat > /mnt/tmp/cpu2.$$ 2>/dev/null; then
    echo "$(date '+%Y-%m-%d %H:%M:%S') [ERROR] Failed to read /proc/stat (second sample)" >> "$LOG_DIR/$LOG_FILE"
    debug_log "Failed to read /proc/stat (second sample)"
    rm -f /mnt/tmp/cpu1.$$
    sleep 60
    continue
  fi

  # Calculate CPU usage percentage
  local cpu_pct=$(calculate_cpu_usage /mnt/tmp/cpu1.$$ /mnt/tmp/cpu2.$$)
  
  if [ -z "$cpu_pct" ] || [ "$cpu_pct" = "0.0" ]; then
    debug_log "CPU calculation failed or returned 0.0"
  fi

  # Get free memory in KB (prefer MemAvailable, fallback to MemFree)
  local mem_kb=$(awk '($1=="MemAvailable:"){print $2; found=1; exit} ($1=="MemFree:" && !found){mf=$2} END{ if(!found){ if(mf) print mf; else print 0 } }' /proc/meminfo 2>/dev/null)
  if [ -z "$mem_kb" ]; then 
    mem_kb=0
    debug_log "Memory calculation failed, using 0"
  fi

  # Load average (first field)
  local loadavg=$(awk '{print $1; exit}' /proc/loadavg 2>/dev/null)
  if [ -z "$loadavg" ]; then 
    loadavg="0.00"
    debug_log "Load average read failed, using 0.00"
  fi

  # Process count (count numeric entries in /proc) - simplified for busybox
  local proc_count=$(ls -1 /proc 2>/dev/null | grep '^[0-9][0-9]*$' | wc -l 2>/dev/null)
  if [ -z "$proc_count" ]; then 
    proc_count=0
    debug_log "Process count failed, using 0"
  fi

  # Simplified mount space check - now includes /mnt /tmp /data /etc/jffs2 and root
  local mounts_free=""
  for mp in /mnt /tmp /data /etc/jffs2 /; do
    if [ -d "$mp" ]; then
      local free_kb=$(df -k "$mp" 2>/dev/null | awk 'NR==2 {print $4; exit}')
      if [ -n "$free_kb" ] && [ "$free_kb" -gt 0 ] 2>/dev/null; then
        if [ -z "$mounts_free" ]; then
          mounts_free="${mp}=${free_kb}"
        else
          mounts_free="${mounts_free},${mp}=${free_kb}"
        fi
      fi
    fi
  done
  
  if [ -z "$mounts_free" ]; then
    mounts_free="none=0"
    debug_log "Mount space check failed"
  fi

  local ts="$(date '+%Y-%m-%d %H:%M:%S')"
  # Individual explicit free space metrics for key persistent areas (0 if missing)
  local data_free_kb=$(echo "$mounts_free" | tr ',' '\n' | awk -F'=| ' '/\/data=/{print $2; found=1; exit} END{if(!found) print 0}')
  local jffs2_free_kb=$(echo "$mounts_free" | tr ',' '\n' | awk -F'=| ' '/\/etc\/jffs2=/{print $2; found=1; exit} END{if(!found) print 0}')

  local log_line="$ts [INFO] CPU=${cpu_pct}% FREE_MEM_KB=${mem_kb} LOADAVG=${loadavg} PROC_COUNT=${proc_count} DATA_FREE_KB=${data_free_kb} JFFS2_FREE_KB=${jffs2_free_kb} MOUNTS_FREE=${mounts_free}"
  
  echo "$log_line" >> "$LOG_DIR/$LOG_FILE"
  debug_log "Metrics logged successfully"
  
  # Clean up temp files
  rm -f /mnt/tmp/cpu1.$$ /mnt/tmp/cpu2.$$ 2>/dev/null

  # Sleep remaining time to make interval ~60s (we already slept 1s during sampling)
  debug_log "Sleeping for 59 seconds"
  sleep 59
done
