#! /bin/sh

LOG_FILE=gergehack.log

# Initialize log directories first
[ -f /mnt/anyka_hack/init_logs.sh ] && . /mnt/anyka_hack/init_logs.sh

# Source common utilities
[ -f /mnt/anyka_hack/common.sh ] && . /mnt/anyka_hack/common.sh

# import settings
[ -f /data/gergesettings.txt ] && . /data/gergesettings.txt || log WARN "Missing /data/gergesettings.txt; proceeding with defaults"

cfgfile=/etc/jffs2/anyka_cfg.ini

input_wifi_creds() {
  log INFO 'Updating WiFi credentials'
  i=1
  linecountlimit=$(wc -l < "$cfgfile" 2>/dev/null || echo 0)
  newcredfile=/mnt/anyka_hack/anyka_cfg.ini
  : > "$newcredfile"
  while [ $i -le "$linecountlimit" ]; do
    line=$(readline $i "$cfgfile")
    case "$line" in
      ssid*) log DEBUG "Set SSID line $i"; echo "ssid = $wifi_ssid" >> "$newcredfile" ;;
      password*) log DEBUG "Set password line $i"; echo "password = $wifi_password" >> "$newcredfile" ;;
      *) echo "$line" >> "$newcredfile" ;;
    esac
    i=$((i+1))
  done
  mv "$cfgfile" "$newcredfile.old" 2>/dev/null
  cp "$newcredfile" "$cfgfile" 2>/dev/null || log ERROR "Failed to update $cfgfile"
}

if [ "${run_telnet:-1}" -eq 0 ]; then
  # telnet is started by rc.local or time_zone.sh
  killall telnetd 2>/dev/null && log INFO "Disabled telnetd" || log DEBUG "telnetd not running"
fi

# check duplicate process
mkdir -p /mnt/tmp 2>/dev/null || true
if [ -f /mnt/tmp/exploit.txt ]; then
  log WARN 'gergehack already running'
else
  echo "exploit" > /mnt/tmp/exploit.txt
  ensure_mounted_sd
  log INFO '_________________________________'
  log INFO '|       Gerge Hacked This       |'
  log INFO '---------------------------------'
  #get new settings from SD card if available
  if [ -f /mnt/anyka_hack/gergesettings.txt ] && [ -f /data/gergesettings.txt ]; then
    if ! diff /mnt/anyka_hack/gergesettings.txt /data/gergesettings.txt >/dev/null 2>&1; then
      log INFO 'Updating settings from SD and rebooting'
      cp /mnt/anyka_hack/gergesettings.txt /data/gergesettings.txt && reboot
    fi
  fi
  if [ -f /mnt/anyka_hack/gergehack.sh ] && [ -f /data/gergehack.sh ]; then
    if ! diff /mnt/anyka_hack/gergehack.sh /data/gergehack.sh >/dev/null 2>&1; then
      log INFO 'Updating gergehack from SD and rebooting'
      cp /mnt/anyka_hack/gergehack.sh /data/gergehack.sh && reboot
    fi
  fi
  setssid=$(grep '^ssid' "$cfgfile" | sed 's/ //g') #get ssid line and remove spaces
  setpass=$(grep '^password' "$cfgfile" | sed 's/ //g') #get password line and remove spaces
  setssid=${setssid##*=} #keep the part after the =
  setpass=${setpass##*=} #keep the part after the =
  if [ "$setssid" != "$wifi_ssid" ] || [ "$setpass" != "$wifi_password" ]; then
    input_wifi_creds
  fi
  log INFO 'Start network service'
  /usr/sbin/wifi_manage.sh start 2>/dev/null || log WARN "wifi_manage failed"
  ntpd -n -N -p "$time_source" &
  export TZ="$time_zone"
  if [ "${run_ftp:-1}" -eq 0 ]; then
    killall tcpsvd 2>/dev/null && log INFO "Disabled FTP service" || log DEBUG "FTP service not running"
  fi
  # load kernel modules for camera
  load_camera_modules "$sensor_kern_module"
  if [ "${run_ptz_daemon:-0}" -eq 1 ]; then
    log INFO 'Start PTZ'
    if [ -f /usr/bin/ptz_daemon_dyn ]; then
      ptz_daemon_dyn &
    else
      /mnt/anyka_hack/ptz/run_ptz.sh &
    fi
    if [ "${ptz_init_on_boot:-0}" -eq 1 ]; then
      sleep 10
      echo "init_ptz" > /mnt/tmp/ptz.daemon
    fi
  fi
  if [ "${run_web_interface:-0}" -eq 1 ]; then
    log INFO 'Start web interface'
    if [ -f /mnt/anyka_hack/web_interface/www/index.html ]; then
      # Use ONVIF web interface if enabled and available, otherwise use legacy
      if [ "${use_onvif_web_interface:-0}" -eq 1 ] && [ -f /mnt/anyka_hack/web_interface/start_web_interface_onvif.sh ]; then
        log INFO 'Starting ONVIF web interface'
        /mnt/anyka_hack/web_interface/start_web_interface_onvif.sh &
      else
        log INFO 'Starting legacy web interface'
        /mnt/anyka_hack/web_interface/start_web_interface.sh &
      fi
    else
      busybox httpd -p 80 -h /data/www &
    fi
  fi
  if [ "${run_onvif_server:-0}" -eq 1 ]; then
    log INFO 'Start ONVIF server'
    if [ -f /mnt/anyka_hack/onvif/onvifd ]; then
      /mnt/anyka_hack/onvif/run_onvifd.sh &
    else
      log WARN "ONVIF server binary not found at /mnt/anyka_hack/onvif/onvifd"
    fi
  fi
  if [ "${run_libre_anyka:-0}" -eq 1 ]; then
    log INFO 'Start Libre Anyka'
    if [ -f /usr/bin/ptz_daemon_dyn ]; then
      SD_detect=$(mount | grep mmcblk0p1)
      if [ ${#SD_detect} -eq 0 ]; then
        md_record_sec=0 # disable recording if SD card is not mounted
      fi
      libre_anyka_app -w "$image_width" -h "$image_height" -m "$md_record_sec" $extra_args &
    else
      /mnt/anyka_hack/libre_anyka_app/run_libre_anyka_app.sh &
    fi
  fi

  # Start lightweight system monitor if available (logs CPU and free memory every minute)
  if [ -f /mnt/anyka_hack/sys_monitor.sh ]; then
    if [ -f /mnt/tmp/sys_monitor.pid ] && kill -0 "$(cat /mnt/tmp/sys_monitor.pid)" 2>/dev/null; then
      log DEBUG "sys_monitor already running pid=$(cat /mnt/tmp/sys_monitor.pid)"
    else
      /mnt/anyka_hack/sys_monitor.sh >> /mnt/logs/sys_monitor.log 2>&1 &
      log INFO "Started sys_monitor.sh"
    fi
  else
    log DEBUG "sys_monitor.sh not present on SD"
  fi

  # Start periodic reboot helper (optional) if enabled and script present.
  # Requires variables (optional) in gergesettings.txt:
  #   enable_periodic_reboot=1            # master switch
  #   periodic_reboot_minutes=720         # interval in minutes (default 720 = 12h)
  if [ "${enable_periodic_reboot:-0}" -eq 1 ]; then
    if [ -f /mnt/anyka_hack/periodic_reboot.sh ]; then
      if [ -f /mnt/tmp/periodic_reboot.pid ] && kill -0 "$(cat /mnt/tmp/periodic_reboot.pid)" 2>/dev/null; then
        log DEBUG "periodic_reboot already running pid=$(cat /mnt/tmp/periodic_reboot.pid)"
      else
        REBOOT_INTERVAL_MIN=${periodic_reboot_minutes:-720}
        /mnt/anyka_hack/periodic_reboot.sh "$REBOOT_INTERVAL_MIN" >> /mnt/logs/periodic_reboot.log 2>&1 &
  echo $! > /mnt/tmp/periodic_reboot.pid
        log INFO "Started periodic_reboot.sh interval=${REBOOT_INTERVAL_MIN}m"
      fi
    else
      log WARN "periodic_reboot.sh not present on SD (enable_periodic_reboot=1)"
    fi
  fi
  if [ "${run_ipc:-1}" -eq 0 ] && [ "${rootfs_modified:-0}" -eq 0 ]; then
    while true; do
      sleep 30
    done
  fi
fi
