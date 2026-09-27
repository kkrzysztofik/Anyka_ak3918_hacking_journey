#!/bin/sh

while true
do
	killall -15 $1
	obj_app=`ps | grep $1 | grep -v grep |grep -v $0 | awk '{print $1}'`
	obj_upgarde=`ps | grep $2 | grep -v grep |grep -v $0 | awk '{print $2}'`
	if [ "$obj_app" != "" ]
	then
		sleep 1
	else
		echo "the pro has been killed"
		if [ "$obj_upgarde" != "" ]
		then
			echo "the pro has been killed"
		else
			echo "the pro has been killed"
		fi
	fi
done
