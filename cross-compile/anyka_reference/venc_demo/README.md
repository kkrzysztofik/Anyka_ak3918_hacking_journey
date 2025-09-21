# Video encode demo

This is a working example of encoding h264 and JPEG from the YUV stream provided by the image sensor.

The compiler needs the old `libplat_thread.so` (V1.1.00) but running it only works with V2.0.00 (you can check version using `strings` on the `.so` file)

The JPEG encoder is a huge step forward and will be used to create a JPEG_snapshot app. This should be even better than the current BMP implementation.

H264 encoding will be tested more to make `.mp4` file recording possible

## Options

The app defaults to 1920x1080 which is not valid for many sensors, so the appropriate resolution needs to be set with arguments. The run script provides everything needed.

`venc_demo <number of frames> <main> <sub>`

example: `/mnt/anyka_hack/venc_demo/venc_demo.out 1000 8 4`

# Compiling

Make sure you have the [cross-compiler installed](https://gitea.raspiweb.com/Gerge/Anyka_ak3918_hacking_journey/src/branch/main/cross-compile).

Run: `./build.sh` (you may need to set execute permission first)


