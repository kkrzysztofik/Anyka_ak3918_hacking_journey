#! /bin/sh

LOG_FILE=libre_anyka_app.log

# Initialize log directories if available
[ -f /mnt/anyka_hack/init_logs.sh ] && . /mnt/anyka_hack/init_logs.sh

# Source common utilities
[ -f /mnt/anyka_hack/common.sh ] && . /mnt/anyka_hack/common.sh

start_app() {
  log INFO 'Starting libre_anyka_app'
  export LD_LIBRARY_PATH=/mnt/anyka_hack/libre_anyka_app/lib
  /mnt/anyka_hack/libre_anyka_app/libre_anyka_app -w "$image_width" -h "$image_height" -m "$md_record_sec" $extra_args
  rc=$?
  if [ $rc -ne 0 ]; then
    log ERROR "libre_anyka_app exited with code $rc"
  else
    log INFO 'libre_anyka_app exited normally'
  fi
}

# import settings
[ -f /data/gergesettings.txt ] && . /data/gergesettings.txt || log WARN "Missing settings file"

# load kernel modules for camera
load_camera_modules "$sensor_kern_module"

start_app >> /mnt/logs/libre_anyka_app.log 2>&1
