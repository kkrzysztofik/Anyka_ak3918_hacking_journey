#!/bin/sh

LOG_FILE=onvif_rust.log

# Initialize log directories if available
[ -f /mnt/anyka_hack/init_logs.sh ] && . /mnt/anyka_hack/init_logs.sh

# Source common utilities
[ -f /mnt/anyka_hack/common.sh ] && . /mnt/anyka_hack/common.sh

start_onvif_rust() {
  log INFO 'Starting ONVIF Rust server (onvif-rust)'

  # Set library path for shared libraries (common lib + onvif-specific lib)
  export LD_LIBRARY_PATH=/mnt/anyka_hack/lib:/mnt/anyka_hack/onvif/lib:$LD_LIBRARY_PATH

  # Check if configuration file exists
  if [ ! -f /mnt/anyka_hack/onvif/config.toml ]; then
    log WARN "Configuration file not found: /mnt/anyka_hack/onvif/config.toml"
    # Optionally create a default or exit
  fi

  # Start ONVIF Rust server with config path
  /mnt/anyka_hack/onvif/onvif-rust /mnt/anyka_hack/onvif/config.toml
  rc=$?

  if [ $rc -ne 0 ]; then
    log ERROR "ONVIF Rust server (onvif-rust) exited with code $rc"
  else
    log INFO 'ONVIF Rust server (onvif-rust) exited normally'
  fi
}

# Check if ONVIF Rust server is already running
if pgrep -f "onvif-rust" > /dev/null; then
  log INFO 'ONVIF Rust server already running'
  exit 0
fi

# Import settings
[ -f /data/gergesettings.txt ] && . /data/gergesettings.txt || log WARN "Missing settings file"

# Validate shared library directory
SHARED_LIB_DIR="/mnt/anyka_hack/lib"
if [ ! -d "${SHARED_LIB_DIR}" ]; then
  log ERROR "Shared library directory not found at: ${SHARED_LIB_DIR}"
  exit 1
fi

# Load camera modules if needed (may not be required for Rust version)
# load_camera_modules "$sensor_kern_module"

# Start ONVIF Rust server
start_onvif_rust >> /mnt/logs/onvif_rust.log 2>&1
