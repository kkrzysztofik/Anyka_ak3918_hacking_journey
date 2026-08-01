# Another exploit
The script `/usr/sbin/service.sh` which starts the `anyka_ipc` app on the default camera has some checks for updates and tests on the SD card.
If the `Factory` folder is present on the SD, then the camera thinks that there is a factory test script to run in there.

This script can be used to automate the hacking of the camera.

**This method does not modify boot firmware.** The worst that can happen if
the hack fails is that the camera keeps running the stock software. Note that
when Wi-Fi settings in `anyka.toml` differ from the camera's stored config, the
supervisor may rewrite `/etc/jffs2/anyka_cfg.ini` so association can succeed.

This is a tested and validated method on one of my cameras with factory reset firmware. Works out of the box, you never have to use the app as wifi will be set up.

1A) Simply copy the `anyka_hack` and `Factory` folder to the SD card.

1B) Edit wifi credentials (and any other settings) in `anyka_hack/anyka.toml` before first boot.

2) Put the SD card into the camera and power it on

## The camera will work with the hack applications when the SD card is plugged in.
## If there is no SD card with `Factory` folder, the camera will work as original without modifications.
