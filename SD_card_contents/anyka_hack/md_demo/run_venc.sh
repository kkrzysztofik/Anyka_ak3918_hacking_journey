#! /bin/sh

LOG_FILE=md_demo.log
[ -f /mnt/anyka_hack/common.sh ] && . /mnt/anyka_hack/common.sh

# Load sensor configuration
[ -f /data/gergesettings.txt ] && . /data/gergesettings.txt
sensor_module="${sensor_kern_module:-/data/sensor/sensor_gc1084.ko}"

load_camera_modules "$sensor_module"
log INFO 'Starting md_demo'
export LD_LIBRARY_PATH=/mnt/anyka_hack/md_demo/lib
/mnt/anyka_hack/md_demo/md_demo
rc=$?
log INFO "md_demo exited code=$rc"
