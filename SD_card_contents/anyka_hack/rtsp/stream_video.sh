#! /bin/sh

LOG_FILE=rtsp.log
[ -f /mnt/anyka_hack/common.sh ] && . /mnt/anyka_hack/common.sh

start_process() {
  log INFO 'Starting rtsp service'
  export LD_LIBRARY_PATH=/mnt/anyka_hack/rtsp/lib
  /mnt/anyka_hack/rtsp/rtsp &
  log INFO "rtsp pid=$!"
}

# Load sensor configuration
[ -f /data/gergesettings.txt ] && . /data/gergesettings.txt
sensor_module="${sensor_kern_module:-/data/sensor/sensor_gc1084.ko}"

# load camera modules
load_camera_modules "$sensor_module"

start_process