#!/bin/sh
#
# common.sh - Common utility functions for anyka_hack scripts
#
# Description:
#   Provides lightweight logging, validation, and helper routines designed for
#   busybox and limited environments. This script should be sourced by other
#   anyka_hack scripts to provide common functionality.
#
# Usage:
#   . /mnt/anyka_hack/common.sh
#
# Dependencies:
#   - busybox or standard Unix utilities
#   - /mnt/logs directory (with fallback to /mnt/tmp)
#
# Environment Variables:
#   LOG_FILE    - Override default log file name (default: anyka_hack.log)
#   DEBUG       - Enable debug logging when set to 1 (default: 0)
#   VERBOSE     - Enable verbose logging when set to 1 (default: 0)
#
# Functions:
#   log(level, message...)     - Log message with timestamp and level
#   die(message...)           - Log error and exit with code 1
#   debug_log(message...)     - Log debug message (only if DEBUG=1)
#   verbose_log(message...)   - Log verbose message (only if VERBOSE=1)
#   validate_file_exists()    - Check if file exists and log error if not
#   validate_directory_writable() - Check if directory is writable
#   validate_required_vars()  - Validate required environment variables
#   validate_system_state()   - Validate essential system directories/files
#   validate_configuration()  - Validate configuration file and variables
#   safe_copy()              - Copy file with error handling
#   safe_mkdir()             - Create directory with error handling
#   load_camera_modules()    - Load camera kernel modules
#   ensure_mounted_sd()      - Mount SD card if not already mounted
#   rand_token()             - Generate random alphanumeric token
#   readline()               - Safely read specific line from file
#   is_process_running()     - Check if process is running via PID file
#   report_service_status()  - Report service status with PID and log info
#
# Author: Anyka Hack Project
# Version: 2.0
# Last Modified: $(date '+%Y-%m-%d')

# =============================================================================
# GLOBAL VARIABLES AND CONFIGURATION
# =============================================================================

SCRIPT_NAME="$(basename "$0")"
LOG_DIR="/mnt/logs"
TMP_DIR="/mnt/tmp"
LOG_FILE="${LOG_FILE:-anyka_hack.log}"

# Debug configuration
DEBUG_MODE="${DEBUG:-0}"
VERBOSE_MODE="${VERBOSE:-0}"

# =============================================================================
# UTILITY FUNCTIONS
# =============================================================================

# Enhanced debugging functions
debug_enabled() {
  [ "$DEBUG_MODE" = "1" ]
}

verbose_enabled() {
  [ "$VERBOSE_MODE" = "1" ]
}

# Enhanced logging with debug levels
debug_log() {
  if debug_enabled; then
    log DEBUG "$*"
  fi
}

verbose_log() {
  if verbose_enabled; then
    log INFO "$*"
  fi
}

# System state validation
validate_system_state() {
  local errors=0
  
  debug_log "Validating system state..."
  
  # Check essential directories
  for dir in /proc /sys /dev /mnt; do
    if [ ! -d "$dir" ]; then
      log ERROR "Essential directory missing: $dir"
      errors=$((errors + 1))
    else
      debug_log "Directory OK: $dir"
    fi
  done
  
  # Check essential files
  for file in /proc/stat /proc/meminfo /proc/loadavg; do
    if [ ! -r "$file" ]; then
      log ERROR "Essential file not readable: $file"
      errors=$((errors + 1))
    else
      debug_log "File OK: $file"
    fi
  done
  
  # Check log directory
  if ! validate_directory_writable "$LOG_DIR" "log directory"; then
    errors=$((errors + 1))
  fi
  
  if [ $errors -eq 0 ]; then
    debug_log "System state validation passed"
    return 0
  else
    log ERROR "System state validation failed with $errors errors"
    return 1
  fi
}

# =============================================================================
# LOGGING FUNCTIONS
# =============================================================================

log() {
  # Log a message with timestamp, level, and script name
  # Usage: log LEVEL MESSAGE...
  # Parameters:
  #   LEVEL   - Log level (INFO, WARN, ERROR, DEBUG)
  #   MESSAGE - Message to log (can be multiple arguments)
  # Returns: 0 on success, 1 on error
  # Notes: DEBUG messages are suppressed unless DEBUG=1 environment variable is set
  
  local lvl="$1"
  shift || true
  local ts=$(date '+%Y-%m-%d %H:%M:%S' 2>/dev/null || date)
  
  # Conditionally suppress DEBUG unless DEBUG=1
  if [ "$lvl" = "DEBUG" ] && [ "${DEBUG:-0}" != 1 ]; then
    return 0
  fi
  
  printf '%s [%s] [%s] %s\n' "$ts" "$lvl" "$SCRIPT_NAME" "$*" >> "$LOG_DIR/$LOG_FILE"
}

die() {
  log ERROR "$*"
  exit 1
}

# =============================================================================
# VALIDATION FUNCTIONS
# =============================================================================

validate_file_exists() {
  # validate_file_exists <file_path> <description>
  local file_path="$1"
  local description="${2:-file}"
  
  if [ ! -f "$file_path" ]; then
    log ERROR "$description not found: $file_path"
    return 1
  fi
  return 0
}

validate_directory_writable() {
  # validate_directory_writable <dir_path> <description>
  local dir_path="$1"
  local description="${2:-directory}"
  
  if [ ! -d "$dir_path" ]; then
    log ERROR "$description does not exist: $dir_path"
    return 1
  fi
  
  if [ ! -w "$dir_path" ]; then
    log ERROR "$description is not writable: $dir_path"
    return 1
  fi
  return 0
}

validate_required_vars() {
  # validate_required_vars <var1> [var2] [var3] ...
  local missing_vars=""
  
  for var in "$@"; do
    if [ -z "${!var}" ]; then
      missing_vars="${missing_vars}${missing_vars:+ }$var"
    fi
  done
  
  if [ -n "$missing_vars" ]; then
    log ERROR "Required configuration variables not set: $missing_vars"
    return 1
  fi
  return 0
}

validate_configuration() {
  local config_file="$1"
  local required_vars="$2"
  local errors=0
  
  debug_log "Validating configuration from $config_file"
  
  # Check if config file exists and is readable
  if [ ! -f "$config_file" ]; then
    log WARN "Configuration file not found: $config_file"
    return 1
  fi
  
  if [ ! -r "$config_file" ]; then
    log ERROR "Configuration file not readable: $config_file"
    return 1
  fi
  
  # Validate required variables
  if [ -n "$required_vars" ]; then
    for var in $required_vars; do
      if [ -z "${!var}" ]; then
        log ERROR "Required configuration variable not set: $var"
        errors=$((errors + 1))
      else
        debug_log "Configuration OK: $var=${!var}"
      fi
    done
  fi
  
  if [ $errors -eq 0 ]; then
    debug_log "Configuration validation passed"
    return 0
  else
    log ERROR "Configuration validation failed with $errors errors"
    return 1
  fi
}

# =============================================================================
# FILE OPERATION FUNCTIONS
# =============================================================================

safe_copy() {
  # safe_copy <src> <dest> <description>
  local src="$1"
  local dest="$2"
  local description="${3:-file}"
  
  if [ ! -f "$src" ]; then
    log ERROR "Source $description not found: $src"
    return 1
  fi
  
  if cp "$src" "$dest" 2>/dev/null; then
    log INFO "Successfully copied $description: $src -> $dest"
    return 0
  else
    log ERROR "Failed to copy $description: $src -> $dest"
    return 1
  fi
}

safe_mkdir() {
  # safe_mkdir <dir_path> <description>
  local dir_path="$1"
  local description="${2:-directory}"
  
  if mkdir -p "$dir_path" 2>/dev/null; then
    log INFO "Successfully created $description: $dir_path"
    return 0
  else
    log ERROR "Failed to create $description: $dir_path"
    return 1
  fi
}

# =============================================================================
# SYSTEM FUNCTIONS
# =============================================================================

load_camera_modules() {
  # load_camera_modules [sensor_module_path]
  local sensor_module="$1"
  
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

# =============================================================================
# UTILITY FUNCTIONS
# =============================================================================

rand_token() {
  # Generate simple alphanumeric token (length ~32)
  dd if=/dev/urandom bs=32 count=1 2>/dev/null | tr -cd 'a-zA-Z0-9' | cut -c1-32
}

readline() {
  # Safe line reader: readline <line_number> <file>
  local linetoread="$1"
  local file="$2"
  sed "${linetoread}q;d" "$file" 2>/dev/null
}

# =============================================================================
# PROCESS MANAGEMENT FUNCTIONS
# =============================================================================

is_process_running() {
  local pid_file="$1"
  local process_name="$2"
  
  if [ -f "$pid_file" ]; then
    local pid=$(cat "$pid_file" 2>/dev/null)
    if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
      debug_log "$process_name is running (pid=$pid)"
      return 0
    else
      debug_log "$process_name PID file exists but process not running"
      rm -f "$pid_file" 2>/dev/null
    fi
  fi
  
  debug_log "$process_name is not running"
  return 1
}

report_service_status() {
  local service_name="$1"
  local pid_file="$2"
  local log_file="$3"
  
  if is_process_running "$pid_file" "$service_name"; then
    local pid=$(cat "$pid_file" 2>/dev/null)
    log INFO "$service_name is running (pid=$pid)"
    
    if [ -n "$log_file" ] && [ -f "$log_file" ]; then
      local log_size=$(wc -c < "$log_file" 2>/dev/null || echo "0")
      debug_log "$service_name log file size: ${log_size} bytes"
    fi
    return 0
  else
    log WARN "$service_name is not running"
    return 1
  fi
}

# =============================================================================
# EXECUTION SECTION
# =============================================================================

# Ensure log directory exists with proper error handling
ensure_log_dir() {
  if [ ! -d "$LOG_DIR" ]; then
    if mkdir -p "$LOG_DIR" 2>/dev/null; then
      printf '%s [INFO] [%s] Created log directory %s\n' "$(date '+%Y-%m-%d %H:%M:%S' 2>/dev/null || date)" "$SCRIPT_NAME" "$LOG_DIR" >> "$LOG_DIR/directory_creation.log" 2>/dev/null || true
    else
      # Fallback: try to write to /mnt/tmp/logs if /mnt/logs fails
      LOG_DIR="/mnt/tmp/logs"
      if mkdir -p "$LOG_DIR" 2>/dev/null; then
        printf '%s [WARN] [%s] Using fallback log directory %s\n' "$(date '+%Y-%m-%d %H:%M:%S' 2>/dev/null || date)" "$SCRIPT_NAME" "$LOG_DIR" >> "$LOG_DIR/directory_creation.log" 2>/dev/null || true
      else
        # Final fallback to /mnt/tmp
        LOG_DIR="/mnt/tmp"
        printf '%s [ERROR] [%s] Using final fallback directory %s\n' "$(date '+%Y-%m-%d %H:%M:%S' 2>/dev/null || date)" "$SCRIPT_NAME" "$LOG_DIR" >> "$LOG_DIR/directory_creation.log" 2>/dev/null || true
      fi
    fi
  fi
}

# Initialize log directory
ensure_log_dir

# Create temporary directory
mkdir -p "$TMP_DIR" 2>/dev/null || true
