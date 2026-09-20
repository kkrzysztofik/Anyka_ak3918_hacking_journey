#!/bin/sh
# Exec target of the supervised wpa_supplicant service (anyka-init boot.rs
# rewires the service to this script at boot).
#
# Clears every process that squats on the ctrl socket before exec-ing the
# real supplicant: bring-up's detached instance, and the vendor
# wifi_run.sh respawn loop that refills a killed wpa within a second.
# The supervisor's own killall (P3) runs seconds before this script (P4):
# the respawn window in between is exactly how a second supplicant ended up
# exiting 255 forever on 192.168.2.198 (2026-09-20) and dragging the camera
# through its wifi deadman into the vendor boot path.
killall wpa_supplicant 2>/dev/null
killall wifi_run.sh 2>/dev/null
sleep 1
exec /tmp/wpa_supplicant "$@"
