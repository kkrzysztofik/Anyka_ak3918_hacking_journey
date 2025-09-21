# JPEG snapshot
This is the same as the original snapshot (with `.bmp` images), but using the hardware JPEG encoder insde the AK3918 SOC.

## Usage

`run_jpeg_snapshot.sh` script included which will load the kernel modules and start the app

The kernel module(s) needs to be adjusted to refer to the one for your sensor.
You can also edit the resolution arguments the snapshot app starts up with.

*Access in Browser*

http://IP:3000/snapshot.jpeg

## Options

The app does not have `--help`, but it takes arguments
- `-w <width>` [ default 1280 ]
- `-h <height>` [ default 720 ]

# Compiling

Make sure you have the [cross-compiler installed](https://gitea.raspiweb.com/Gerge/Anyka_ak3918_hacking_journey/src/branch/main/cross-compile).

Run: `./build.sh` (you may need to set execute permission first)


