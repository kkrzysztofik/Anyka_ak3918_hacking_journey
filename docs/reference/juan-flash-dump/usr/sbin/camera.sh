#! /bin/sh
### BEGIN INIT INFO
# File:				camera.sh	
# Provides:         
# Required-Start:   $
# Required-Stop:
# Default-Start:     
# Default-Stop:
# Short-Description: install or remove camera drivers
# Author:			
# Email: 			
# Date:				2015-9-28
### END INIT INFO

MODE=$1

usage()
{
	echo "Usage: $0 setup | remove"
}


camera_remove()
{
	rmmod /usr/modules/ak_info_dump.ko
	rmmod /usr/modules/akcamera.ko
	rmmod /usr/modules/akmci.ko
	/usr/sbin/wifi.sh stop
	find /usr/modules -maxdepth 1 -type f -name "sensor_*.ko" -exec rmmod {} \;
	find /etc/jffs2 -maxdepth 1 -type f -name "sensor_*.ko" -exec rmmod {} \;
}

camera_setup()
{
	insmod /usr/modules/akmci.ko
	insmod /usr/modules/ak39_top_wdt.ko
	insmod /usr/modules/mii.ko
	insmod /usr/modules/usbnet.ko
	insmod /usr/modules/cdc_ether.ko
	insmod /usr/modules/cdc_subset.ko
	insmod /usr/modules/rndis_host.ko
	insmod /usr/modules/usbserial.ko
	insmod /usr/modules/usb_wwan.ko
	insmod /usr/modules/option.ko
	find /etc/jffs2 -maxdepth 1 -type f -name "sensor_*.ko" -exec insmod {} \;
	find /usr/modules -maxdepth 1 -type f -name "sensor_*.ko" -exec insmod {} \;
	insmod /usr/modules/akcamera.ko
	insmod /usr/modules/aw9523.ko
	insmod /usr/modules/ak_motor.ko
	insmod /usr/modules/ak_info_dump.ko
	/usr/sbin/wifi.sh start &
}

case "$MODE" in
	setup)
		camera_setup
		;;
	remove)
		camera_remove
		;;
	*)
		usage
		;;
esac
exit 0

