#!/bin/sh
#
# Start web interface with ONVIF support
# This script only manages the web server startup
#

[ -f /mnt/anyka_hack/common.sh ] && . /mnt/anyka_hack/common.sh

echo "Starting ONVIF-enabled web interface..."

# Start the web server
echo "Starting web server..."

# Use the ONVIF-enabled web interface
if [ -f /mnt/anyka_hack/web_interface/www/cgi-bin/webui_onvif ]; then
    # Copy web interface files to /data/www if it doesn't exist
    if [ ! -d /data/www ]; then
        mkdir -p /data/www 2>/dev/null || true
        cp -r /mnt/anyka_hack/web_interface/www/* /data/www/ 2>/dev/null || true
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

echo "Web interface started successfully"
echo "Access at: http://$(ip route get 1 | awk '{print $NF;exit}')"
