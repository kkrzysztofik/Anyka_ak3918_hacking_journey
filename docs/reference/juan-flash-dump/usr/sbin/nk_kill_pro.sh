#!/bin/sh

for i in 1 2 3 4 5 6 7 8 9 
do
	echo "2 the pro has $i $1"
	#killall -10 $1
	#sleep 1
	killall -9 $1
	obj=`ps | grep $1 | grep -v grep |grep -v $0 | awk '{print $1}'`
	if [ "$obj" != "" ]
	then
		echo "the pro has sleep"
		sleep 1
		sync
	else
		echo "the pro has been killed"
		exit 0
	fi
done

