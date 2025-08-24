#! /bin/sh

LOG_FILE=venc_demo.log

# Initialize log directories if available
[ -f /mnt/anyka_hack/init_logs.sh ] && . /mnt/anyka_hack/init_logs.sh

# Source common utilities
[ -f /mnt/anyka_hack/common.sh ] && . /mnt/anyka_hack/common.sh

# Load sensor configuration
[ -f /data/gergesettings.txt ] && . /data/gergesettings.txt
sensor_module="${sensor_kern_module:-/data/sensor/sensor_gc1084.ko}"

load_camera_modules "$sensor_module"
log INFO 'Starting venc_demo'
export LD_LIBRARY_PATH=/mnt/anyka_hack/venc_demo/lib
/mnt/anyka_hack/venc_demo/venc_demo.out 1000 8 4
rc=$?
log INFO "venc_demo exited code=$rc"
