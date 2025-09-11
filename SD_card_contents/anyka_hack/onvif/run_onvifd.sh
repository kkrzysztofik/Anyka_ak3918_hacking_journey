#!/bin/sh

LOG_FILE=onvifd.log

# Initialize log directories if available
[ -f /mnt/anyka_hack/init_logs.sh ] && . /mnt/anyka_hack/init_logs.sh

# Source common utilities
[ -f /mnt/anyka_hack/common.sh ] && . /mnt/anyka_hack/common.sh

start_onvifd() {
  log INFO 'Starting ONVIF server (onvifd)'
  
  # Set library path for ONVIF server
  export LD_LIBRARY_PATH=/mnt/anyka_hack/onvif/lib:$LD_LIBRARY_PATH
  
  # Check if configuration file exists, create default if not
  if [ ! -f /etc/jffs2/anyka_cfg.ini ]; then
    log INFO 'Creating default ONVIF configuration'
    mkdir -p /etc/jffs2
    cp /mnt/anyka_hack/onvif/anyka_cfg.ini /etc/jffs2/anyka_cfg.ini 2>/dev/null || {
      log WARN "Default config not found, creating minimal config"
      cat > /etc/jffs2/anyka_cfg.ini << EOF
[global]
user = admin
secret = admin

[onvif]
enabled = 1
http_port = 8080

[ptz]
invert_directions = 0
max_pan_speed = 1.0
max_tilt_speed = 1.0

[imaging]
default_brightness = 0
default_contrast = 0
default_saturation = 0
EOF
    }
  fi
  
  # Start ONVIF server
  /mnt/anyka_hack/onvif/onvifd
  rc=$?
  
  if [ $rc -ne 0 ]; then
    log ERROR "ONVIF server (onvifd) exited with code $rc"
  else
    log INFO 'ONVIF server (onvifd) exited normally'
  fi
}

# Check if ONVIF server is already running
if pgrep -f "onvifd" > /dev/null; then
  log INFO 'ONVIF server already running'
  exit 0
fi

# Import settings
[ -f /data/gergesettings.txt ] && . /data/gergesettings.txt || log WARN "Missing settings file"

# Load camera modules if needed
load_camera_modules "$sensor_kern_module"

# Start ONVIF server
start_onvifd >> /mnt/logs/onvifd.log 2>&1
