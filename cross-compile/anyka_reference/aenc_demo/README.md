# Audio encode demo

This is a working example of encoding mp3 from the microphone input. There are other file-formats as well, but mp3 is the most useful.

Currently volume control, noise reduction and auto gain control does not work. The problematic features are simply commented out or set to disabled.

## Options

The app shows a help menu when called without arguments.

`aenc_demo [sample rate] [channel] [type] [save path] [time(secs)] [volume] [mic/linein]`

example: `aenc_demo 8000 1 mp3 /mnt/ 10 7 mic`

# Compiling

Make sure you have the [cross-compiler installed](https://gitea.raspiweb.com/Gerge/Anyka_ak3918_hacking_journey/src/branch/main/cross-compile).

Run: `./build.sh` (you may need to set execute permission first)


