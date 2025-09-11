#!/bin/sh
#
# periodic_reboot.sh - Periodic system reboot utility
#
# Description:
#   Reboots the camera system at configurable intervals to mitigate long-running
#   memory leaks and system instability. Includes jitter functionality to prevent
#   thundering herd problems in multi-camera deployments.
#
# Usage:
#   ./periodic_reboot.sh [INTERVAL_MINUTES]
#   or call from gergehack.sh with configuration
#
# Parameters:
#   INTERVAL_MINUTES - Reboot interval in minutes (optional, overrides config)
#
# Environment Variables:
#   REBOOT_INTERVAL_SEC        - Base interval in seconds (default: 21600 = 6h)
#   REBOOT_JITTER_MAX_SEC      - Max jitter in seconds (default: 0 = disabled)
#   ENABLE_PERIODIC_REBOOT     - Master switch (default: 1 = enabled)
#
# Features:
#   - Configurable reboot intervals
#   - Optional jitter to prevent simultaneous reboots
#   - PID file management to prevent duplicates
#   - Graceful signal handling (INT, TERM, EXIT)
#   - Comprehensive logging with progress updates
#   - Retry logic for failed reboot attempts
#
# Dependencies:
#   - common.sh (sourced automatically)
#   - init_logs.sh (sourced automatically)
#   - /mnt/tmp directory for PID files
#
# Author: Anyka Hack Project
# Version: 2.0
# Last Modified: $(date '+%Y-%m-%d')

# =============================================================================
# GLOBAL VARIABLES AND CONFIGURATION
# =============================================================================

LOG_FILE="periodic_reboot.log"
INTERVAL_SEC=${REBOOT_INTERVAL_SEC:-21600} # 6 hours
JITTER_MAX_SEC=${REBOOT_JITTER_MAX_SEC:-0}
PID_FILE="/mnt/tmp/periodic_reboot.pid"

# =============================================================================
# FUNCTION DEFINITIONS
# =============================================================================

compute_jitter() {
  # Outputs jitter (0..JITTER_MAX_SEC) using /dev/urandom if available, else pseudo.
  if [ "$JITTER_MAX_SEC" -le 0 ] 2>/dev/null; then
    echo 0
    return 0
  fi
  
  if [ -r /dev/urandom ]; then
    # Read 2 bytes, convert to integer, mod range
    local val=$(dd if=/dev/urandom bs=2 count=1 2>/dev/null | od -An -tu2 2>/dev/null | tr -d ' ')
    [ -z "$val" ] && val=0
  else
    # Fallback: simple time-based pseudo number
    local val=$$$(date +%S 2>/dev/null)
  fi
  
  echo $(( val % (JITTER_MAX_SEC + 1) ))
}

cleanup() {
  rm -f "$PID_FILE" 2>/dev/null
  log INFO "Exiting periodic_reboot"
  exit 0
}

# =============================================================================
# EXECUTION SECTION
# =============================================================================

# Initialize log directories & common helpers if present
[ -f /mnt/anyka_hack/init_logs.sh ] && . /mnt/anyka_hack/init_logs.sh
[ -f /mnt/anyka_hack/common.sh ] && . /mnt/anyka_hack/common.sh

# Check if periodic reboot is enabled
if [ "${ENABLE_PERIODIC_REBOOT:-1}" != 1 ]; then
  log INFO "Periodic reboot disabled via ENABLE_PERIODIC_REBOOT=${ENABLE_PERIODIC_REBOOT}"
  exit 0
fi

# Create temporary directory
mkdir -p /mnt/tmp 2>/dev/null || true

# Prevent duplicate instances
if [ -f "$PID_FILE" ]; then
  local old_pid=$(cat "$PID_FILE" 2>/dev/null)
  if [ -n "$old_pid" ] && kill -0 "$old_pid" 2>/dev/null; then
    log WARN "Already running (pid=$old_pid); exiting"
    exit 0
  fi
fi

echo $$ > "$PID_FILE" 2>/dev/null || log WARN "Could not write PID file $PID_FILE"

# Set up signal handlers
trap cleanup INT TERM EXIT

log INFO "Starting periodic_reboot pid=$$ base_interval=${INTERVAL_SEC}s jitter_max=${JITTER_MAX_SEC}s"

while true; do
  local jitter=$(compute_jitter)
  local sleep_sec=$INTERVAL_SEC
  
  if [ "$jitter" -gt 0 ] 2>/dev/null; then
    sleep_sec=$((INTERVAL_SEC + jitter))
  fi
  
  log INFO "Next reboot in ${sleep_sec}s (base=${INTERVAL_SEC}s jitter=${jitter}s)"

  # Sleep in smaller chunks to allow signal handling (avoid very long uninterruptible sleep)
  local remaining=$sleep_sec
  while [ "$remaining" -gt 0 ]; do
    local chunk=300
    [ "$remaining" -lt $chunk ] && chunk=$remaining
    
    # Log progress: INFO every hour boundary and for last 5 minutes, DEBUG otherwise
    if [ "$remaining" -le 300 ] 2>/dev/null || [ $((remaining % 3600)) -eq 0 ] 2>/dev/null; then
      log INFO "Reboot countdown: remaining=${remaining}s next_sleep=${chunk}s"
    else
      log DEBUG "Reboot countdown: remaining=${remaining}s next_sleep=${chunk}s"
    fi
    
    sleep $chunk || true
    remaining=$((remaining - chunk))
  done

  local uptime_s=$(awk '{print int($1)}' /proc/uptime 2>/dev/null || echo '?')
  log INFO "Rebooting now (uptime=${uptime_s}s)"
  
  # Sync filesystem before reboot
  if sync 2>/dev/null; then
    log DEBUG "Filesystem synced successfully"
  else
    log WARN "Filesystem sync failed, proceeding with reboot"
  fi
  
  # Attempt reboot with retry logic
  local reboot_attempts=0
  local max_reboot_attempts=3
  local reboot_retry_delay=60
  
  while [ $reboot_attempts -lt $max_reboot_attempts ]; do
    reboot_attempts=$((reboot_attempts + 1))
    log INFO "Reboot attempt $reboot_attempts/$max_reboot_attempts"
    
    if reboot 2>/dev/null; then
      # If reboot command succeeds, we should not reach here
      log INFO "Reboot command executed successfully"
      break
    else
      log WARN "Reboot command failed (attempt $reboot_attempts/$max_reboot_attempts)"
      if [ $reboot_attempts -lt $max_reboot_attempts ]; then
        log INFO "Retrying reboot in ${reboot_retry_delay}s"
        sleep $reboot_retry_delay
      else
        log ERROR "All reboot attempts failed, exiting"
        exit 1
      fi
    fi
  done
done
