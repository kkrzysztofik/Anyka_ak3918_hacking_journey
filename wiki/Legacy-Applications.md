# Legacy Applications

**Note**: The applications in this section are separate legacy utilities and are not part of the ONVIF Rust implementation. These are standalone tools that may be used independently or alongside the ONVIF server.

## Legacy Web Interface

I created a combined web interface using the features from `ptz_daemon`, `libre_anyka_app`, and `busybox httpd`. The webpage is based on [another Chinese camera hack for Goke processors](https://github.com/dc35956/gk7102-hack).

![web_interface](Images/web_interface.png)

With the most recent update of webui the interface is a lot nicer and has all settings and features of the camera available without needing to edit config files manually.

![web_interface](Images/web_interface_settings.png)

**Note: the WebUI has a login process using md5 password hash and a token, but this is not secure by any means. Do not expose to the internet!**

## Libre Anyka App

This is an app in development aimed to combine most features of the camera and make it small enough to run from flash without the SD card.

Currently contains features:

- JPEG snapshot
- RTSP stream
- Motion detection trigger
- H264 `.str` file recording to SD card (ffmpeg converts this to mp4 in webUI)

Does not have:

- sound (only RTSP stream has sound)

More info about the [app](https://github.com/kkrzysztofik/Anyka_ak3918_hacking_journey/tree/main/SD_card_contents/anyka_hack/libre_anyka_app) and [source](https://github.com/kkrzysztofik/Anyka_ak3918_hacking_journey/tree/main/cross-compile/libre_anyka_app).

**Note: the RTSP stream and snapshots are not protected by password. Do not expose to the internet!**

## SSH

[Dropbear](https://github.com/kkrzysztofik/Anyka_ak3918_hacking_journey/tree/main/SD_card_contents/anyka_hack/dropbear) can give ssh access if telnet is not your preference.

## Play Sound

**Extracted from camera.**

More info about the [app](https://github.com/kkrzysztofik/Anyka_ak3918_hacking_journey/tree/main/SD_card_contents/anyka_hack/ak_adec_demo)

## Record Sound

**mp3 recording works.**

More info about the [app](https://github.com/kkrzysztofik/Anyka_ak3918_hacking_journey/tree/main/SD_card_contents/anyka_hack/aenc_demo) and [source](https://github.com/kkrzysztofik/Anyka_ak3918_hacking_journey/tree/main/cross-compile/aenc_demo).

## Setup without SD

After the root and usr partitions are modified with the desired apps, the following steps are needed to get it working:

copy the webpage www folder to `/etc/jffs2/www` for httpd to serve from flash

The latest `gergehack.sh` is already capable of running fully local files, so update if needed and check sensor module settings.

This gives the following functions without SD card running on the camera:

- webUI on port 80
- usual ftp, telnet functions, and ntp time sync
- RTSP stream and snapshots for UI with libre_anyka_app
- ptz movement

## See Also

- [[Web-Interface]] - Current web interface documentation
- [[Resources]] - Additional resources and links
