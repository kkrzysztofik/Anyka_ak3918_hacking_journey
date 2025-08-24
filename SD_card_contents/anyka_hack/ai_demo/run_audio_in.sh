#! /bin/sh

LOG_FILE=ai_demo.log
[ -f /mnt/anyka_hack/common.sh ] && . /mnt/anyka_hack/common.sh

# Load sensor configuration
[ -f /data/gergesettings.txt ] && . /data/gergesettings.txt
sensor_module="${sensor_kern_module:-/data/sensor/sensor_gc1084.ko}"

load_camera_modules "$sensor_module"
log INFO 'Starting ai_demo'
export LD_LIBRARY_PATH=/mnt/anyka_hack/ai_demo/lib
/mnt/anyka_hack/ai_demo/ai_demo
rc=$?
log INFO "ai_demo exited code=$rc"
