#!/bin/sh

# init_logs.sh - Initialize log directories and ensure they exist
# This script should be called early in the boot process to ensure consistent logging

MAIN_LOG_DIR=/mnt/logs
FALLBACK_LOG_DIR=/mnt/tmp/logs
FALLBACK_BASE_DIR=/mnt/tmp
mkdir -p /mnt/tmp 2>/dev/null || true

init_log_directories() {
  # Try to create the main log directory first
  if mkdir -p "$MAIN_LOG_DIR" 2>/dev/null; then
    printf '[INFO] Created log directory %s\n' "$MAIN_LOG_DIR"
    return 0
  fi
  
  # If main directory creation fails, try fallback
  if mkdir -p "$FALLBACK_LOG_DIR" 2>/dev/null; then
    printf '[WARN] Main log dir failed, using fallback %s\n' "$FALLBACK_LOG_DIR"
    return 0
  fi
  
  # Final fallback to /tmp
  printf '[WARN] Log directory creation failed, using %s\n' "$FALLBACK_BASE_DIR"
  return 1
}

# Create essential log directories
init_log_directories

# Log the initialization
timestamp="$(date '+%Y-%m-%d %H:%M:%S' 2>/dev/null || date)"
log_file="$MAIN_LOG_DIR/init.log"
[ ! -w "$MAIN_LOG_DIR" ] && log_file="$FALLBACK_LOG_DIR/init.log"
[ ! -w "$FALLBACK_LOG_DIR" ] && log_file="$FALLBACK_BASE_DIR/init.log"

printf '%s [INFO] [init_logs] Log directories initialized\n' "$timestamp" >> "$log_file" 2>/dev/null || true
printf '%s [INFO] [init_logs] Main log directory: %s\n' "$timestamp" "$MAIN_LOG_DIR" >> "$log_file" 2>/dev/null || true

# Export the log directory for other scripts
export LOG_DIR="$MAIN_LOG_DIR"
[ ! -w "$MAIN_LOG_DIR" ] && export LOG_DIR="$FALLBACK_LOG_DIR"
[ ! -w "$FALLBACK_LOG_DIR" ] && export LOG_DIR="$FALLBACK_BASE_DIR"

printf '[INFO] Log initialization complete. LOG_DIR=%s\n' "$LOG_DIR"
