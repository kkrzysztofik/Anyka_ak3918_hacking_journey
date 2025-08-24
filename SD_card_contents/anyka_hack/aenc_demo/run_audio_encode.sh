#! /bin/sh

LOG_FILE=aenc_demo.log
[ -f /mnt/anyka_hack/common.sh ] && . /mnt/anyka_hack/common.sh

log INFO 'Starting aenc_demo'
export LD_LIBRARY_PATH=/mnt/anyka_hack/aenc_demo/lib
/mnt/anyka_hack/aenc_demo/aenc_demo 8000 1 mp3 /mnt/ 10 7 mic
rc=$?
log INFO "aenc_demo exited code=$rc"
