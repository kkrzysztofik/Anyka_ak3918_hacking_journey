# Credits
this repo is based on the [original work of Ricardo](https://github.com/ricardojlrufino/anyka_v380ipcam_experiments/tree/master) and [MuhammedKalkan](https://github.com/MuhammedKalkan/Anyka-Camera-Firmware). The code has been modified a lot.

# Improvements and changes from Ricardo's work
- Loads image from memory and does not write to filesystem
- Uses hardware JPEG encoding for smaller filesize and faster process time
- Continuous and on-demand(first frame) encoding of images (encoding stops some time after no more previews are requested)
- Fixed resolution limitations (allows all resolutions up to 1280x720 as long as the size is a multiple of 4)
- Merged with RTSP stream app
- Added motion detection trigger
- H264 stream recording after motion trigger (mp4 not functional yet)
- using the h264 encoder buffer allows recording 2s video before the motion happened
- Improved capture loop for minimal workload

# Things to improve (todo list)

- mp4 file format (ffmpeg is used instead to convert h264 to mp4)
- detect available storage space (SD detection done, but no space detection)

# How it works

The JPEG encoder runs non-stop and generates a JPEG stream that is loaded into the two image buffers by capture loop.

There are two jpeg stream buffers (rotating between writing and ready to serve). This way an image can be served immediately on request.

Motion detection runs without frame capture, only when enabled (`-m` parameter) and when previous recording is nearly done (to check if recording needs to continue).

H264 encoded stream `.str` file recording while motion is observed (continues until motion stops).

The http server is waiting for requests in a separate thread.

RTSP stream runs on separate thread.

## Usage

`run_libre_anyka_app.sh` script included which will load the kernel modules and start the app

The kernel module(s) needs to be adjusted to refer to the one for your sensor.
You can also edit the arguments the app starts up with.

*Access in Browser*

http://IP:3000/snapshot.jpeg

*Access in VLC, ffplay, Motioneye or other DVR*

rtsp://IP:554/vs0 [MAIN] or rtsp://IP:554/vs1 [SUB]

*play recorded `.str` file*

`ffplay -autoexit 20240410-131647_1_.str`

## Options

The app does not have `--help`, but it takes arguments
- `-w <width>` [ default 1280 ]
- `-h <height>` [ default 720 ]
- `-m <motion_minimum_record_seconds>` [ default 0 disabled ]
- `-i <irinvert> [ default 1 ]`
- `-u "upside down" mode enable (rotates image 180 degree)`

irinvert can be a value 1-4 as follows
|  value  | black/white vs colour image mode | IR filter |
|:-------:|:--------------------------------:|:---------:|
|        1|                                  |           |
|        2|                                  |   inverted|
|        3|                          inverted|   inverted|
|        4|                          inverted|           |

If the colour image looks pink, that menas the IR filter needs to be inverted

If the camera has black/white image during daytime the image mode needs to be inverted

*NOTE: the width and height applies to the preview image. It is recommended to use smaller than 640x480 so that it uses the sub-stream. RTSP and video recording will always use main resolution*

# Compiling

Make sure you have the [cross-compiler installed](https://gitea.raspiweb.com/Gerge/Anyka_ak3918_hacking_journey/src/branch/main/cross-compile).

Run: `./build.sh` (you may need to set execute permission first)


