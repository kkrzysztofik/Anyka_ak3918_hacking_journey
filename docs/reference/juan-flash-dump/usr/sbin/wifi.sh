#! /bin/sh
### BEGIN INIT INFO
# File:				camera.sh	
# Provides:         camera service 
# Required-Start:   $
# Required-Stop:
# Default-Start:     
# Default-Stop:
# Short-Description:web service
# Author:			gao_wangsheng
# Email: 			gao_wangsheng@anyka.oa
# Date:				2012-8-8
### END INIT INFO

MODE=$1
PATH=$PATH:/bin:/sbin:/usr/bin:/usr/sbin
mode=hostapd
network=
usage()
{
	echo "Usage: $0 start|stop)"
	exit 3
}

hw_board_type=$(cat /sys/ak_info_dump/board_version)
enable_wifi()
{
	if [ "$hw_board_type" == "0x0" -o "$hw_board_type" == "0x1" ];then
		WIFI_ENABLE=$1
		echo $WIFI_ENABLE > /sys/user-gpio/gpio-wifi_power
	fi
}

stop()
{
	rmmod /usr/modules/otg-hs.ko
	echo "stop wifi device ......"
}

start ()
{	
	enable_wifi 0
	#sleep 1
	insmod /usr/modules/otg-hs.ko
	sleep 1
	enable_wifi 1
	#insmod /usr/modules/8188fu.ko
	#sleep 1
	echo "start reset wifi device ......"	
}

restart ()
{
	echo "restart ipc service......"
	stop
	start
}

#
# main:
#

case "$MODE" in
	start)
		start
		;;
	stop)
		stop
		;;
	restart)
		restart
		;;
	*)
		usage
		;;
esac
exit 0

