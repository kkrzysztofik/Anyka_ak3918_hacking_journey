#!/bin/sh

while [ 1 ]
do
	IOTDaemon -S "$1"
	sleep 1
done
