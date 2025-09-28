#!/bin/sh

LOG_FILE=ffmpeg_restarter.log

# Initialize log directories if available
[ -f /mnt/anyka_hack/init_logs.sh ] && . /mnt/anyka_hack/init_logs.sh

# Source common utilities
[ -f /mnt/anyka_hack/common.sh ] && . /mnt/anyka_hack/common.sh

[ -f /data/gergesettings.txt ] && . /data/gergesettings.txt || log WARN "Missing settings file"

restart_app() {
  log INFO 'Attempting to (re)start libre_anyka_app'

  # Create temporary directory for PID files
  mkdir -p /mnt/tmp 2>/dev/null || true

  if [ -f /usr/bin/ptz_daemon_dyn ]; then
    SD_detect=$(mount | grep mmcblk0p1)
    if [ ${#SD_detect} -eq 0 ]; then
      md_record_sec=0 # disable recording if SD card is not mounted
      log WARN 'SD card not mounted; disabling recording'
    fi
    libre_anyka_app -w "$image_width" -h "$image_height" -m "$md_record_sec" $extra_args &
    APP_PID=$!
    echo $APP_PID > /mnt/tmp/libre_anyka_app.pid 2>/dev/null
  else
    /mnt/anyka_hack/libre_anyka_app/run_libre_anyka_app.sh &
    APP_PID=$!
    echo $APP_PID > /mnt/tmp/libre_anyka_app.pid 2>/dev/null
  fi
  log INFO "Restart issued pid=$APP_PID"
}

check_app() {
  appnum=$(top -n 1 | grep libre_anyka_app | grep -v 'grep')
  ffmpegnum=$(top -n 1 | grep wrap_mp4.sh | grep -v 'grep')
  if [ ${#appnum} -eq 0 ] && [ ${#ffmpegnum} -eq 0 ]; then
    log WARN 'libre_anyka_app and wrap_mp4.sh not detected; restarting'
    restart_app
  else
    log DEBUG 'Processes healthy'
  fi
}

if [ "${run_libre_anyka:-0}" -eq 1 ]; then
  while true; do
    check_app
    sleep 20
  done
fi
