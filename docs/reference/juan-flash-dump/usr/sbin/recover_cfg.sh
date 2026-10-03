#!/bin/sh
# File:				recover_cfg.sh	
# Provides:         
# Description:      recover system config
# Author:			aj

#recover factory config ini
cp /usr/local/factory_cfg.ini /etc/jffs2/onvif_cfg.ini

#recover isp config ini
rm -rf /etc/jffs2/isp*.conf
