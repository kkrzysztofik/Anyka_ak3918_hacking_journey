# This is a modified version of
https://github.com/kuhnchris/IOT-ANYKA-PTZdaemon

# Available commands

The command names differ from the original to improve clarity, but this means the old web_ui is not compatible.

NOTE: some of the commands (set_ar, set_deg, set_speed...) are commented out because they are not so useful as the `init_ptz` call already sets them, if you need them just enable and recompile.

### PTZ commands
- `init_ptz` homes the camera and starts tracking the position in degrees
- `up` `down` `left` `right` relative motion for the web_ui
- `right_up` `right_down` `left_up` `left_down` relative diagonal motion commands for the web_ui
- `t2p [xdegrees] [ydegrees]` turn to position
### IR commands
- `init_ir`
- `get_ir_in_lev` get the input lever of the sensor
- `get_ir_thres` get the current setting for IR auto trigger (just a guess)
- `set_ir_thres [num]`
- `set_ir_cut [0|1]` control the IR shutter (used by web_ui)
- `set_ir_checkmode`
### Other commands
- `q` quit the daemon
- `ping` returns pong

# Compiling

Make sure you have the [cross-compiler installed](https://gitea.raspiweb.com/Gerge/Anyka_ak3918_hacking_journey/src/branch/main/cross-compile).

Run: `./build.sh` (you may need to set execute permission first)

The build script uses dynamic library linking by default which results in smaller size. This allows running from flash by modifying `/usr` partition if needed.

## The Simple Dirty Way
There is no need for special compiler if using static executable. See this example below. (also in build script commented out)

This is a summary of how the ptz_daemon was compiled. Install the tools:
```
sudo apt install gcc-arm-linux-gnueabi g++-arm-linux-gnueabi binutils-arm-linux-gnueabi
```
I decided to compile everything I can as static (there is plenty of space on the SD card, so the larger size is worth the convenience on not having to look for all the libraries in the camera system). This makes distribution easy, just download and run it on the camera. I have to mention that I had to look for `.a` [static libraries](https://github.com/ricardojlrufino/anyka_v380ipcam_experiments/tree/master/akv300-extract/libplat/lib) that I can compile into it.

I used the following command (with static libs copied to libs/ folder):

`arm-linux-gnueabi-g++ ptz_daemon_cpp.cpp -L./libs -ldl -lplat_drv -lplat_common -lplat_thread -lpthread -D_GLIBCXX_USE_CXX11_ABI=0 -static -o ptz_daemon`

Then it is a matter of copy and run on the camera. (because the executable is static, there are no dependencies to worry about)

## PTZ?

**P**an **T**ilt **Z**oom - basically the camera controls.
