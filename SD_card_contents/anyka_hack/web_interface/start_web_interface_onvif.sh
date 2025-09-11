#!/bin/sh
#
# Start web interface with ONVIF support
# This script ensures the ONVIF server is running before starting the web interface
#

[ -f /mnt/anyka_hack/common.sh ] && . /mnt/anyka_hack/common.sh

echo "Starting ONVIF-enabled web interface..."

# Check if ONVIF server is running
if ! pgrep -f "onvifd" > /dev/null; then
    echo "ONVIF server not running, starting it..."
    
    # Start ONVIF server using the dedicated script
    if [ -f /mnt/anyka_hack/onvif/run_onvifd.sh ]; then
        /mnt/anyka_hack/onvif/run_onvifd.sh &
        sleep 3
        echo "ONVIF server started via run_onvifd.sh"
    elif [ -f /mnt/anyka_hack/onvif/onvifd ]; then
        /mnt/anyka_hack/onvif/onvifd &
        sleep 2
        echo "ONVIF server started directly"
    else
        echo "Warning: ONVIF server binary not found at /mnt/anyka_hack/onvif/onvifd"
        echo "Please ensure the ONVIF server is compiled and available"
    fi
else
    echo "ONVIF server already running"
fi

# Check if snapshot service is running
if ! pgrep -f "jpeg_snapshot" > /dev/null; then
    echo "Snapshot service not running, starting it..."
    
    if [ -f /mnt/anyka_hack/jpeg_snapshot/jpeg_snapshot ]; then
        /mnt/anyka_hack/jpeg_snapshot/jpeg_snapshot &
        sleep 1
        echo "Snapshot service started"
    else
        echo "Warning: Snapshot service binary not found"
    fi
else
    echo "Snapshot service already running"
fi

# Start the web server
echo "Starting web server..."

# Use the ONVIF-enabled web interface
if [ -f /mnt/anyka_hack/web_interface/www/cgi-bin/webui_onvif ]; then
    # Copy web interface files to /data/www if it doesn't exist
    if [ ! -d /data/www ]; then
        mkdir -p /data/www
        cp -r /mnt/anyka_hack/web_interface/www/* /data/www/
    fi
    
    # Start busybox httpd
    if [ -f /mnt/anyka_hack/web_interface/busybox ]; then
        /mnt/anyka_hack/web_interface/busybox httpd -f -p 80 -h /data/www
    else
        echo "Error: busybox binary not found"
        exit 1
    fi
else
    echo "Error: ONVIF web interface files not found"
    exit 1
fi

echo "ONVIF web interface started successfully"
echo "Access at: http://$(ip route get 1 | awk '{print $NF;exit}')"
echo "ONVIF server running on port 8080"
