# Credits
this repo is based on the [original work of Ricardo](https://github.com/ricardojlrufino/anyka_v380ipcam_experiments/tree/master)

# Improvements and changes from Ricardo's work
- Loads image from memory and does not write to filesystem (as suggested by Ricardo's TODO comments)
- Uses BMP565 for smaller filesize
- Simplified single step YUV -> RGB565/BMP565 conversion (instead of YUV -> RGB24 -> RGB565) for speeeeeeed
- continuous encoding of images instead of on HTTP request (loading time is only sending file and not encoding)
- fixed resolution limitations (allows all resolutions up to 1280x720 as long as the size is an even number)

# How it works

The `capture_loop` continuously takes YUV input and creates RGB565 images

There are two RGB565 buffers (rotating between writing and ready to serve). This way an image can be served immediately on request.

The http server is waiting for requests in a separate thread.

# Benchmarks

Tested resolutions (16:9 aspect ratio):
- 160x90 load time 25-30ms (filesize:29kB)
- 320x180 load time 65-120ms (filesize:115kB)
- 640x360 load time 285-360ms (filesize:460kB)
- 854x480 load time 560-640ms (filesize:820kB)
- 1280x720 load time 1.3-1.5s (filesize:1.84MB)

NOTE: other aspect ratios also work (example 640x480), but because the crop area does not change the image is distorted. This can be fixed by cropping the sensor input with `attr.crop.width` and `attr.crop.height`.

## Usage

`run_snapshot.sh` script included which will load the kernel modules and start the app

The kernel module(s) needs to be adjusted to refer to the one for your sensor.
You can also edit the resolution arguments the snapshot app starts up with.

*Access in Browser*

http://IP:3000/snapshot.bmp

## Options

The app does not have `--help`, but it takes arguments
- `-w <width>` [ default 1280 ]
- `-h <height>` [ default 720 ]

# Compiling

Make sure you have the [cross-compiler installed](https://gitea.raspiweb.com/Gerge/Anyka_ak3918_hacking_journey/src/branch/main/cross-compile).

Run: `./build.sh` (you may need to set execute permission first)


