#! /bin/sh

LOG_FILE=ptz.log

# Initialize log directories if available
[ -f /mnt/anyka_hack/init_logs.sh ] && . /mnt/anyka_hack/init_logs.sh

# Source common utilities
[ -f /mnt/anyka_hack/common.sh ] && . /mnt/anyka_hack/common.sh

start_process() {
  log INFO 'Starting ptz daemon'
  export LD_LIBRARY_PATH=/mnt/anyka_hack/ptz/lib
  /mnt/anyka_hack/ptz/ptz_daemon_dyn &
  pid=$!
  log INFO "ptz_daemon_dyn started pid=$pid"
}

start_process >> /mnt/logs/ptz.log 2>&1
