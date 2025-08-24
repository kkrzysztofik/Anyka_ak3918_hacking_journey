#! /bin/sh

# Common utility functions for anyka_hack scripts.
# Provides lightweight logging and helper routines. Designed to be safe on busybox / limited environments.

SCRIPT_NAME="$(basename "$0")"
LOG_DIR=/mnt/logs

# Ensure log directory exists with proper error handling
ensure_log_dir() {
  if [ ! -d "$LOG_DIR" ]; then
    if mkdir -p "$LOG_DIR" 2>/dev/null; then
      printf '%s [INFO] [%s] Created log directory %s\n' "$(date '+%Y-%m-%d %H:%M:%S' 2>/dev/null || date)" "$SCRIPT_NAME" "$LOG_DIR" >> "$LOG_DIR/directory_creation.log" 2>/dev/null || true
    else
      # Fallback: try to write to /tmp if /mnt/logs fails
      LOG_DIR=/tmp/logs
      mkdir -p "$LOG_DIR" 2>/dev/null || LOG_DIR=/tmp
    fi
  fi
}

ensure_log_dir

# Default log file name can be overridden by setting LOG_FILE before sourcing.
LOG_FILE=${LOG_FILE:-anyka_hack.log}

log() {
  # log LEVEL MESSAGE...
  # Levels: INFO WARN ERROR DEBUG
  lvl=$1; shift || true
  ts=$(date '+%Y-%m-%d %H:%M:%S' 2>/dev/null || date)
  # Conditionally suppress DEBUG unless DEBUG=1
  if [ "$lvl" = DEBUG ] && [ "${DEBUG:-0}" != 1 ]; then
    return 0
  fi
  printf '%s [%s] [%s] %s\n' "$ts" "$lvl" "$SCRIPT_NAME" "$*" >> "$LOG_DIR/$LOG_FILE"
}

die() { log ERROR "$*"; exit 1; }

load_camera_modules() {
  # load_camera_modules [sensor_module_path]
  sensor_module="$1"
  if [ -n "$sensor_module" ] && [ -f "$sensor_module" ]; then
    insmod "$sensor_module" 2>/dev/null || log DEBUG "Module $sensor_module may already be loaded"
  fi
  for m in /usr/modules/akcamera.ko /usr/modules/ak_info_dump.ko; do
    [ -f "$m" ] && insmod "$m" 2>/dev/null || log DEBUG "Module $m may already be loaded"
  done
}

ensure_mounted_sd() {
  # Attempt to mount SD card RW if not already mounted at /mnt
  mount | grep -q ' /mnt ' && return 0
  if [ -e /dev/mmcblk0p1 ]; then
    mount -rw /dev/mmcblk0p1 /mnt 2>/dev/null && log INFO "Mounted /dev/mmcblk0p1 to /mnt" && return 0
  fi
  if [ -e /dev/mmcblk0 ]; then
    mount -rw /dev/mmcblk0 /mnt 2>/dev/null && log INFO "Mounted /dev/mmcblk0 to /mnt" && return 0
  fi
  log WARN "SD card mount attempt failed"
  return 1
}

rand_token() {
  # Generate simple alphanumeric token (length ~32)
  dd if=/dev/urandom bs=32 count=1 2>/dev/null | tr -cd 'a-zA-Z0-9' | cut -c1-32
}

readline() {
  # Safe line reader: readline <line_number> <file>
  linetoread=$1; file=$2
  sed "${linetoread}q;d" "$file" 2>/dev/null
}
