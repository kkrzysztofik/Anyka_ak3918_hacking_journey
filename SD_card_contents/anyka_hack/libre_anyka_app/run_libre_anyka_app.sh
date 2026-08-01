#! /bin/sh

# Settings previously came from gergesettings.txt. Override via the environment
# if you still launch this wrapper by hand.
: "${image_width:=1920}"
: "${image_height:=1080}"
: "${md_record_sec:=0}"
: "${extra_args:=}"
: "${sensor_kern_module:=/data/sensor/sensor_gc1084.ko}"

start_app() {
  echo 'restarting libre anyka app...'
  export LD_LIBRARY_PATH=/mnt/anyka_hack/libre_anyka_app/lib
  /mnt/anyka_hack/libre_anyka_app/libre_anyka_app -w $image_width -h $image_height -m $md_record_sec $extra_args
}

#load kernel modules for camera
insmod $sensor_kern_module
insmod /usr/modules/akcamera.ko
insmod /usr/modules/ak_info_dump.ko

start_app
