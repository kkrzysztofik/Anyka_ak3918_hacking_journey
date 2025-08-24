#! /bin/sh

LOG_FILE=web_interface.log

# Initialize log directories if available
[ -f /mnt/anyka_hack/init_logs.sh ] && . /mnt/anyka_hack/init_logs.sh

# Source common utilities
[ -f /mnt/anyka_hack/common.sh ] && . /mnt/anyka_hack/common.sh

[ -f /data/gergesettings.txt ] && . /data/gergesettings.txt

restart_process() {
  log INFO 'Starting web interface busybox httpd'
  /mnt/anyka_hack/web_interface/busybox httpd -p 80 -h /mnt/anyka_hack/web_interface/www
  log INFO 'httpd launched'
}

sync_cgi() {
  src_dir=/mnt/anyka_hack/web_interface/www/cgi-bin
  dest_dir=/data/www/cgi-bin
  for path in "$src_dir"/*; do
    filename="${path##*/}"
    if [ -f "$dest_dir/$filename" ]; then
      if ! diff "$src_dir/$filename" "$dest_dir/$filename" >/dev/null 2>&1; then
        cp "$src_dir/$filename" "$dest_dir/$filename" && log INFO "Updated $filename"
      else
        log DEBUG "$filename unchanged"
      fi
    else
      cp "$src_dir/$filename" "$dest_dir/$filename" && log INFO "Installed $filename"
    fi
  done
}

update_webui() {
  if [ -f /data/www/index.html ]; then
    sync_cgi
  else
    if [ "${rootfs_modified:-0}" -eq 1 ]; then
      mkdir -p /data/www
      cp -r /mnt/anyka_hack/web_interface/www/cgi-bin /data/www/
      cp /mnt/anyka_hack/web_interface/www/styles.css /data/www/
      cp /mnt/anyka_hack/web_interface/www/index.html /data/www/
      log INFO 'Installed web UI to rootfs'
    else
      log DEBUG 'Rootfs not modified; serving UI from SD'
    fi
  fi
}

update_webui >> /mnt/logs/start_web_interface.log 2>&1
/mnt/anyka_hack/ffmpeg/app_restarter.sh  >> /mnt/logs/start_web_interface.log 2>&1 &
restart_process  >> /mnt/logs/start_web_interface.log 2>&1

