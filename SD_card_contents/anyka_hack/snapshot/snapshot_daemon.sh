#! /bin/sh

LOG_FILE=snapshot_daemon.log

# Initialize log directories if available
[ -f /mnt/anyka_hack/init_logs.sh ] && . /mnt/anyka_hack/init_logs.sh

# Source common utilities
[ -f /mnt/anyka_hack/common.sh ] && . /mnt/anyka_hack/common.sh

restart_process() {
  log INFO 'Restarting snapshot service'
  export LD_LIBRARY_PATH=/mnt/anyka_hack/snapshot/lib
  /mnt/anyka_hack/snapshot/ak_snapshot &
  #! /bin/sh

  LOG_FILE=snapshot_daemon.log
  [ -f /mnt/anyka_hack/common.sh ] && . /mnt/anyka_hack/common.sh

  restart_process() {
    log INFO 'Restarting snapshot process'
    export LD_LIBRARY_PATH=/mnt/anyka_hack/snapshot/lib
    /mnt/anyka_hack/snapshot/ak_snapshot &
    log INFO "ak_snapshot pid=$!"
  }

  check_process_health() {
    myresult=$( top -n 1 | grep snapshot | grep -v grep | grep -v daemon )
    log DEBUG "snapshot check: ${myresult}"
    if [ ${#myresult} -lt 5 ]; then
      restart_process
    fi
  }

  # Load sensor configuration
  [ -f /data/gergesettings.txt ] && . /data/gergesettings.txt
  sensor_module="${sensor_kern_module:-/data/sensor/sensor_gc1084.ko}"
  
  # load kernel modules for camera
  load_camera_modules "$sensor_module"

  while true; do
    check_process_health
    sleep 5
  done
