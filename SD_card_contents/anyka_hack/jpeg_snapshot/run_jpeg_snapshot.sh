#! /bin/sh

LOG_FILE=jpeg_snapshot.log
[ -f /mnt/anyka_hack/common.sh ] && . /mnt/anyka_hack/common.sh

# Load sensor configuration
[ -f /data/gergesettings.txt ] && . /data/gergesettings.txt
sensor_module="${sensor_kern_module:-/data/sensor/sensor_gc1084.ko}"

load_camera_modules "$sensor_module"

log INFO 'Starting jpeg_snapshot'
export LD_LIBRARY_PATH=/mnt/anyka_hack/jpeg_snapshot/lib
/mnt/anyka_hack/jpeg_snapshot/jpeg_snapshot -w 320 -h 180
rc=$?
log INFO "jpeg_snapshot exited code=$rc"
