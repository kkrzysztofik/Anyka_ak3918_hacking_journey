#!/bin/sh

LOG_FILE=wrap_mp4.log

# Initialize log directories if available
[ -f /mnt/anyka_hack/init_logs.sh ] && . /mnt/anyka_hack/init_logs.sh

# Source common utilities
[ -f /mnt/anyka_hack/common.sh ] && . /mnt/anyka_hack/common.sh

restart_app() {
  [ -f /data/gergesettings.txt ] && . /data/gergesettings.txt
  log INFO 'Restarting libre_anyka_app after wrapping (disabled by default)'
  if [ -f /usr/bin/ptz_daemon_dyn ]; then
    SD_detect=$(mount | grep mmcblk0p1)
    if [ ${#SD_detect} -eq 0 ]; then
      md_record_sec=0
      log WARN 'SD card not mounted; disabling recording'
    fi
    libre_anyka_app -w "$image_width" -h "$image_height" -m "$md_record_sec" $extra_args &
  else
    /mnt/anyka_hack/libre_anyka_app/run_libre_anyka_app.sh &
  fi
  log INFO "Restarted libre_anyka_app pid=$!"
}

stop_app() {
  appnum=$(top -n 1 | grep libre_anyka_app | grep -v 'grep')
  for i in $appnum; do
    kill "$i" 2>/dev/null && log INFO "Stopped libre_anyka_app pid=$i" || log DEBUG "Could not kill $i"
    break
  done
}

stop_app
h264files=$(ls /mnt/video_encode/*.str 2>/dev/null)
mkdir -p /mnt/anyka_hack/web_interface/www/video

for i in $h264files; do
  filename="${i%.str}"
  filename="${filename##*/}"
  log INFO "Processing $filename"
  if [ ! -f "/mnt/anyka_hack/web_interface/www/video/$filename.mp4" ]; then
    cp "/mnt/video_encode/$filename.str" "/mnt/video_encode/$filename.h264" 2>/dev/null || { log ERROR "Copy failed for $filename"; continue; }
    /mnt/anyka_hack/ffmpeg/ffmpeg -threads 1 -i "/mnt/video_encode/$filename.h264" -c:v copy -f mp4 "/mnt/anyka_hack/web_interface/www/video/$filename.mp4" >> /mnt/logs/ffmpeg_convert.log 2>&1
    if [ -f "/mnt/anyka_hack/web_interface/www/video/$filename.mp4" ]; then
      rm "/mnt/video_encode/$filename.str" 2>/dev/null && log INFO "Converted and removed original $filename.str"
    else
      log ERROR "Failed to create mp4 for $filename"
    fi
  else
    log DEBUG "$filename.mp4 already exists; skipping"
  fi
done

#restart_app (left disabled intentionally)
