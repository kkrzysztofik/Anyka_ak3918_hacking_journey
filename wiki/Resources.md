# Resources

## Quick Start SD Card Hack

This hack runs only when the SD card is inserted, leaving boot firmware unmodified.
It is beginner friendly: copy two folders, edit Wi-Fi settings, power on.

1. Build (or use a pre-built tree) and copy `anyka_hack/` + `Factory/` to the card —
   see [Factory README](https://github.com/kkrzysztofik/Anyka_ak3918_hacking_journey/tree/main/SD_card_contents/Factory).
2. **Edit** `anyka_hack/anyka.toml` — set `[wifi] ssid` and `password` before first boot.
3. Insert the card and power on. Recovery telnet is on port **24**.

Full operator guide: [[Boot-Runtime-Supervisor]].

It is unlikely that this can cause any harm to your camera as the system remains original, but no matter how small the risk it is never zero (unless you have the exact same camera). Try any of these hacks at your own risk. When Wi-Fi settings differ from the stored vendor config, the supervisor may rewrite `/etc/jffs2/anyka_cfg.ini`.

The SD card hack is a safe way to test compatibility with the camera and to see if all features are working.

## Info Links

These are the most important links here (this is where 99% of the info and resources come from):

- <https://gitea.raspiweb.com/Gerge/Anyka_ak3918_hacking_journey>
- <https://github.com/helloworld-spec/qiwen/tree/main/anycloud39ev300> (explanation in chinese, good reference)
- <https://github.com/ricardojlrufino/anyka_v380ipcam_experiments/tree/master> (ak_snapshot original)
- <https://github.com/kuhnchris/IOT-ANYKA-PTZdaemon> (ptz daemon original)
- <https://github.com/MuhammedKalkan/Anyka-Camera-Firmware> (Muhammed's RTSP app + library, and more discussions)
- <https://github.com/e27-camera-hack/E27-Camera-Hack/discussions/1> (discussion where most of this was worked on)

## Traditional Setup

For the original Ubuntu 16.04 setup and other legacy applications, see the [hack process documentation](https://github.com/kkrzysztofik/Anyka_ak3918_hacking_journey/blob/main/hack_process/README.md).

## See Also

- [[Home]] - Main wiki entry point
- [[Boot-Runtime-Supervisor]] - `anyka-init` and `anyka.toml`
- [[Development-Environment]] - Development setup instructions
- [[Legacy-Applications]] - Legacy tools and applications
