#!bin/sh
rmmod option
rmmod usb_wwan
rmmod rndis_host
rmmod cdc_subset
rmmod cdc_ether
rmmod usbnet
rmmod mii
#rmmod otg-hs
sleep 1
#insmod /usr/modules/otg-hs.ko
insmod /usr/modules/mii.ko
insmod /usr/modules/usbnet.ko
insmod /usr/modules/cdc_ether.ko
insmod /usr/modules/cdc_subset.ko
insmod /usr/modules/rndis_host.ko
insmod /usr/modules/usb_wwan.ko
insmod /usr/modules/option.ko
