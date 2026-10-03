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
usage()
{
	echo "Usage: $0 start|stop)"
	exit 3
}

stop()
{
	echo "we don't stop ipc service......"
}

start ()
{
	echo "start ipc service......"
	DEBUG_INI_PATH="/mnt/tf/debug.ini"
	PDTEST_INI_PATH="/mnt/tf/production_test.ini"
	MOUNT_DIR=/mnt/tf
	MMC_DEV=/dev/mmcblk0
	MMC_FS=/dev/mmcblk0p1
	LOG_REDIRECT=0
	umask 077
	ulimit -s 1024
	echo 3 > /proc/sys/vm/drop_caches
	usleep 300000
	if [ -e $MMC_DEV ];then
		echo "exist mmcblk0"
		if [ -e $MMC_FS ];then
			echo "exist mmcblk0p1"
			mkdir $MOUNT_DIR
			mount $MMC_FS $MOUNT_DIR
		fi
	fi

	if [ -f "$DEBUG_INI_PATH" ];then

		if [ -e $MOUNT_DIR/anyka_ipc_nostrip ];then
			if [ ! -e $MOUNT_DIR/do_not_debug.ini ];then
				mount --bind $MOUNT_DIR/anyka_ipc_nostrip /usr/bin/anyka_ipc
				ulimit -c unlimited
				ulimit -a
				echo "/mnt/tf/core/%e_%t.core" > /proc/sys/kernel/core_pattern
			fi
		fi

		if [ -f "$PDTEST_INI_PATH" ];then
			echo "production test mode, no redirect log"
		else
			LOG_REDIRECT=1
		fi

		if [ -f "/mnt/tf/log_no_redirect" ];then
			LOG_REDIRECT=0
		fi

	fi

	pid=`pgrep anyka_ipc`
    if [ "$pid" = "" ]
    then
		if [ $LOG_REDIRECT == 1 ];then
			REBOOTTIMES=1;
			if [ -f "$MOUNT_DIR/.reboot" ];then
				REBOOTTIMES=`cat "$MOUNT_DIR/.reboot"`;let REBOOTTIMES+=1;
			fi

			echo "$REBOOTTIMES" > "$MOUNT_DIR/.reboot";
			cat /proc/kmsg >> $MOUNT_DIR/kmsg.$REBOOTTIMES.log & 2>&1

			CURTIME=`date +%y%m%d_%H%M%S`
			LOG_FILE=ipc_$CURTIME.$REBOOTTIMES.log
			echo $LOG_FILE
			touch "$MOUNT_DIR/$LOG_FILE"
			if [ -f "$MOUNT_DIR/$LOG_FILE" ];then
				anyka_ipc >> "$MOUNT_DIR/$LOG_FILE" 2>&1
			else
				anyka_ipc
			fi
		else
			anyka_ipc
		fi
		sync
		echo "anyka_ipc exit"
	fi
	touch /etc/jffs2/error_reboot
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

