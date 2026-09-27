#!/bin/sh
# File:				update.sh
# Provides:
# Description:      update zImage&rootfs under dir1/dir2/...
# Author:			xc

#
# main:
#

n1_mode=$1
n1_port=$2
n1_file=$3
n1_size=$4

## 
#
# main:
#


# kill apps, MUST use force kill
/usr/sbin/kill_ipc.sh 
kill -9 `pidof IOTDaemon_start.sh`
kill -9 `pidof IOTDaemon`
kill -9 `pidof wpa_supplicant`
kill -9 `pidof hostapd`
kill -9 `pidof hostapd_rtl`
echo "rmmod watchdog and insmod watchdog..."
rmmod /usr/modules/ak39_top_wdt.ko
sleep 1
insmod /usr/modules/ak39_top_wdt.ko default_margin=10 nodeamon=1

echo "start copy lib bin to tmp before update....."

cp -rf /usr/bin/nk_upgarde /tmp
cp -rf /usr/bin/fuser /tmp

mkdir /tmp/root
DIR_ROOT_TMP=/tmp/root
dd if=/dev/root of=/dev/rootfstmp

echo "mkdir -p $DIR_ROOT_TMP"
mount -t squashfs /dev/rootfstmp $DIR_ROOT_TMP
/tmp/fuser -mk /usr
umount -l /usr
export LD_LIBRARY_PATH="$DIR_ROOT_TMP/lib"
export PATH="$DIR_ROOT_TMP/bin:$DIR_ROOT_TMP/sbin"

ulimit -s 384

#根据进程名杀死进程
echo "the start_upgarde start"
/tmp/nk_upgarde ${n1_mode} ${n1_port} ${n1_file} ${n1_size} 
echo "the start_upgarde end"
reboot -f

