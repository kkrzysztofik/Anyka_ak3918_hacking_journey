#!/bin/sh
# File:				update.sh
# Provides:
# Description:      update zImage&rootfs under dir1/dir2/...
# Author:			xc

#
# main:
#

echo "stop system service before update....."
killall -15 syslogd
killall -15 klogd
killall -15 tcpsvd

killall -9 udhcpc 
killall -9 udhcpd

echo "killall IOTDaemon_start.sh iot.Daemon hostapd"
killall IOTDaemon_start.sh iot.Daemon  hostapd

echo "the kill anyka_ipc start"
killall -9 anyka_ipc
killall -9 anyka_ipc_nostrip

echo 3 > /proc/sys/vm/drop_caches

echo "the kill anyka_ipc end"

