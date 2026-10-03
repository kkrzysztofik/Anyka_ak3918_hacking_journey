#!/bin/sh

while true
do
	pid="`pidof  daemon_server`"
	if [ "$pid" -gt "1" ]; then
		#echo "OK:$pid"
		sleep 5
	else
		/opt/bin/daemon_server
		daemon_server
	fi

	sleep 1
done
