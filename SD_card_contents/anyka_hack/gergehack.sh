#!/bin/sh
#
# gergehack.sh - Main orchestration script for Anyka camera hacking
#
# Description:
#   This is the main entry point for the anyka_hack system. It orchestrates
#   the startup of various services including WiFi configuration, PTZ daemon,
#   web interface, ONVIF server, and Libre Anyka application.
#
# Usage:
#   ./gergehack.sh
#   or source from rc.local or similar startup script
#
# Dependencies:
#   - common.sh (sourced automatically)
#   - init_logs.sh (sourced automatically)
#   - gergesettings.txt configuration file
#
# Configuration:
#   All configuration is read from /data/gergesettings.txt with fallback
#   to /mnt/anyka_hack/gergesettings.txt. See gergesettings.txt for available
#   configuration options.
#
# Services Started:
#   - WiFi management and credential updates
#   - PTZ daemon (if enabled)
#   - Web interface (legacy or ONVIF)
#   - ONVIF server (if enabled)
#   - Libre Anyka application (if enabled)
#   - System monitoring (if available)
#   - Periodic reboot (if enabled)
#
# Process Management:
#   - Uses PID files to prevent duplicate instances
#   - Automatic service restart on failure
#   - Graceful error handling and logging
#
# Author: Anyka Hack Project
# Version: 2.0
# Last Modified: $(date '+%Y-%m-%d')

# =============================================================================
# GLOBAL VARIABLES AND CONFIGURATION
# =============================================================================

LOG_FILE="gergehack.log"
CFG_FILE="/etc/jffs2/anyka_cfg.ini"

# =============================================================================
# FUNCTION DEFINITIONS
# =============================================================================

input_wifi_creds() {
  log INFO "Updating WiFi credentials"
  
  # Validate required configuration variables
  if ! validate_required_vars wifi_ssid wifi_password; then
    log ERROR "WiFi credentials not properly configured"
    return 1
  fi
  
  # Validate configuration file exists
  if ! validate_file_exists "$CFG_FILE" "configuration file"; then
    return 1
  fi
  
  local line_number=1
  local line_count=$(wc -l < "$CFG_FILE" 2>/dev/null || echo 0)
  local new_config_file="/mnt/anyka_hack/anyka_cfg.ini"
  
  # Create new configuration file
  : > "$new_config_file" || {
    log ERROR "Failed to create new configuration file: $new_config_file"
    return 1
  }
  
  while [ $line_number -le "$line_count" ]; do
    local line=$(readline $line_number "$CFG_FILE")
    case "$line" in
      ssid*) 
        log DEBUG "Set SSID line $line_number"
        echo "ssid = $wifi_ssid" >> "$new_config_file" || {
          log ERROR "Failed to write SSID to new config file"
          return 1
        }
        ;;
      password*) 
        log DEBUG "Set password line $line_number"
        echo "password = $wifi_password" >> "$new_config_file" || {
          log ERROR "Failed to write password to new config file"
          return 1
        }
        ;;
      *) 
        echo "$line" >> "$new_config_file" || {
          log ERROR "Failed to write line $line_number to new config file"
          return 1
        }
        ;;
    esac
    line_number=$((line_number + 1))
  done
  
  # Backup original configuration
  if ! mv "$CFG_FILE" "$CFG_FILE.old" 2>/dev/null; then
    log WARN "Could not backup original configuration file"
  fi
  
  # Install new configuration
  if ! safe_copy "$new_config_file" "$CFG_FILE" "WiFi configuration"; then
    # Try to restore backup if copy failed
    if [ -f "$CFG_FILE.old" ]; then
      mv "$CFG_FILE.old" "$CFG_FILE" 2>/dev/null || log ERROR "Failed to restore backup configuration"
    fi
    return 1
  fi
  
  log INFO "WiFi credentials updated successfully"
  return 0
}

# Update settings from SD card if available
update_settings_from_sd() {
  local src_file="/mnt/anyka_hack/gergesettings.txt"
  local dest_file="/data/gergesettings.txt"
  
  if [ ! -f "$src_file" ]; then
    log DEBUG "Source settings file not found: $src_file"
    return 0
  fi
  
  if [ ! -f "$dest_file" ]; then
    log DEBUG "Destination settings file not found: $dest_file"
    return 0
  fi
  
  if ! diff "$src_file" "$dest_file" >/dev/null 2>&1; then
    log INFO "Settings differ, updating from SD"
    if safe_copy "$src_file" "$dest_file" "settings file"; then
      log INFO "Settings updated successfully, rebooting"
      sync 2>/dev/null || true
      reboot
    else
      log ERROR "Failed to update settings file"
      return 1
    fi
  else
    log DEBUG "Settings files are identical"
  fi
  return 0
}

# Update gergehack script from SD card if available
update_script_from_sd() {
  local src_file="/mnt/anyka_hack/gergehack.sh"
  local dest_file="/data/gergehack.sh"
  
  if [ ! -f "$src_file" ]; then
    log DEBUG "Source script file not found: $src_file"
    return 0
  fi
  
  if [ ! -f "$dest_file" ]; then
    log DEBUG "Destination script file not found: $dest_file"
    return 0
  fi
  
  if ! diff "$src_file" "$dest_file" >/dev/null 2>&1; then
    log INFO "Script differs, updating from SD"
    if safe_copy "$src_file" "$dest_file" "gergehack script"; then
      log INFO "Script updated successfully, rebooting"
      sync 2>/dev/null || true
      reboot
    else
      log ERROR "Failed to update script file"
      return 1
    fi
  else
    log DEBUG "Script files are identical"
  fi
  return 0
}

# Check if WiFi credentials need updating
check_wifi_credentials() {
  local current_ssid=$(grep '^ssid' "$CFG_FILE" 2>/dev/null | sed 's/ //g' | cut -d'=' -f2)
  local current_pass=$(grep '^password' "$CFG_FILE" 2>/dev/null | sed 's/ //g' | cut -d'=' -f2)
  
  if [ -z "$current_ssid" ] || [ -z "$current_pass" ]; then
    log WARN "Could not read current WiFi credentials from $CFG_FILE"
    return 1
  fi
  
  if [ "$current_ssid" != "$wifi_ssid" ] || [ "$current_pass" != "$wifi_password" ]; then
    log INFO "WiFi credentials need updating (current: $current_ssid, new: $wifi_ssid)"
    return 0
  else
    log DEBUG "WiFi credentials are up to date"
    return 1
  fi
}

# Web interface functions
start_web_interface() {
  log INFO "Starting web interface"
  
  if [ ! -f /mnt/anyka_hack/web_interface/www/index.html ]; then
    log WARN "Web interface files not found, using fallback"
    busybox httpd -p 80 -h /data/www &
    return 0
  fi
  
  if [ -f /mnt/anyka_hack/web_interface/start_web_interface_onvif.sh ]; then
    log INFO "Starting ONVIF web interface"
    if /mnt/anyka_hack/web_interface/start_web_interface_onvif.sh &; then
      log INFO "ONVIF web interface started (pid=$!)"
      return 0
    else
      log ERROR "Failed to start ONVIF web interface"
      return 1
    fi
  fi
}

# ONVIF server functions
start_onvif_server() {
  log INFO "Starting ONVIF server"
  
  if [ -f /mnt/anyka_hack/onvif/onvifd ]; then
    if /mnt/anyka_hack/onvif/run_onvifd.sh &; then
      log INFO "ONVIF server started (pid=$!)"
      return 0
    else
      log ERROR "Failed to start ONVIF server"
      return 1
    fi
  else
    log WARN "ONVIF server binary not found at /mnt/anyka_hack/onvif/onvifd"
    return 1
  fi
}

# Monitoring functions
start_system_monitor() {
  local pid_file="/mnt/tmp/sys_monitor.pid"
  local log_file="/mnt/logs/sys_monitor.log"
  
  if [ ! -f /mnt/anyka_hack/sys_monitor.sh ]; then
    debug_log "sys_monitor.sh not present on SD"
    return 0
  fi
  
  if is_process_running "$pid_file" "sys_monitor"; then
    return 0
  fi
  
  if /mnt/anyka_hack/sys_monitor.sh >> "$log_file" 2>&1 &; then
    echo $! > "$pid_file"
    log INFO "Started sys_monitor.sh (pid=$!)"
    return 0
  else
    log ERROR "Failed to start sys_monitor.sh"
    return 1
  fi
}

start_periodic_reboot() {
  local pid_file="/mnt/tmp/periodic_reboot.pid"
  local log_file="/mnt/logs/periodic_reboot.log"
  
  if [ "${enable_periodic_reboot:-0}" -ne 1 ]; then
    debug_log "Periodic reboot disabled"
    return 0
  fi
  
  if [ ! -f /mnt/anyka_hack/periodic_reboot.sh ]; then
    log WARN "periodic_reboot.sh not present on SD (enable_periodic_reboot=1)"
    return 1
  fi
  
  if is_process_running "$pid_file" "periodic_reboot"; then
    return 0
  fi
  
  local reboot_interval_min=${periodic_reboot_minutes:-720}
  if /mnt/anyka_hack/periodic_reboot.sh "$reboot_interval_min" >> "$log_file" 2>&1 &; then
    echo $! > "$pid_file"
    log INFO "Started periodic_reboot.sh interval=${reboot_interval_min}m (pid=$!)"
    return 0
  else
    log ERROR "Failed to start periodic_reboot.sh"
    return 1
  fi
}

# =============================================================================
# EXECUTION SECTION
# =============================================================================

# Initialize log directories first
[ -f /mnt/anyka_hack/init_logs.sh ] && . /mnt/anyka_hack/init_logs.sh

# Source common utilities
[ -f /mnt/anyka_hack/common.sh ] && . /mnt/anyka_hack/common.sh

# Import settings with validation
if [ -f /data/gergesettings.txt ]; then
  . /data/gergesettings.txt
  validate_configuration "/data/gergesettings.txt" "wifi_ssid wifi_password time_zone"
else
  log WARN "Missing /data/gergesettings.txt; proceeding with defaults"
fi

# Validate system state
validate_system_state

# Disable telnet if configured
if [ "${run_telnet:-1}" -eq 0 ]; then
  killall telnetd 2>/dev/null && log INFO "Disabled telnetd" || log DEBUG "telnetd not running"
fi

# Check for duplicate process
mkdir -p /mnt/tmp 2>/dev/null || true
if [ -f /mnt/tmp/exploit.txt ]; then
  log WARN "gergehack already running"
else
  echo "exploit" > /mnt/tmp/exploit.txt
  ensure_mounted_sd
  log INFO "_________________________________"
  log INFO "|       Gerge Hacked This       |"
  log INFO "---------------------------------"
  
  # Perform updates
  update_settings_from_sd
  update_script_from_sd
  
  # Check and update WiFi credentials
  if check_wifi_credentials; then
    input_wifi_creds
  fi
  
  # Start network service
  log INFO "Start network service"
  /usr/sbin/wifi_manage.sh start 2>/dev/null || log WARN "wifi_manage failed"
  ntpd -n -N -p "$time_source" &
  export TZ="$time_zone"
  
  # Disable FTP if configured
  if [ "${run_ftp:-1}" -eq 0 ]; then
    killall tcpsvd 2>/dev/null && log INFO "Disabled FTP service" || log DEBUG "FTP service not running"
  fi
  
  # Load kernel modules for camera
  load_camera_modules "$sensor_kern_module"

  
  if [ "${run_web_interface:-0}" -eq 1 ]; then
    start_web_interface
  fi
  
  if [ "${run_onvif_server:-0}" -eq 1 ]; then
    start_onvif_server
  fi

  
  # Start monitoring services
  start_system_monitor
  start_periodic_reboot
  
  # Report service status
  report_service_status "sys_monitor" "/mnt/tmp/sys_monitor.pid" "/mnt/logs/sys_monitor.log"
  report_service_status "periodic_reboot" "/mnt/tmp/periodic_reboot.pid" "/mnt/logs/periodic_reboot.log"
  
  if [ "${run_ipc:-0}" -eq 0 ] && [ "${rootfs_modified:-0}" -eq 0 ]; then
    log INFO "IPC disabled and rootfs not modified, entering maintenance mode"
    while true; do
      sleep 30
    done
  fi
fi

