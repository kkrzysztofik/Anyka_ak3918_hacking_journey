# Anyka_ak3918_hacking_journey

Reverse engineering and hacking journey for Chinese IP cameras based on Anyka AK3918 SoC

## Credits

This project was originally developed by **Gerge** (https://gitea.raspiweb.com/Gerge/Anyka_ak3918_hacking_journey) and has been continued with additional improvements and fixes. The original work involved extensive reverse engineering of the Anyka AK3918 platform and development of custom firmware and applications.

## Recent Updates

- **Docker Cross-Compilation Environment**: Added Docker-based cross-compilation setup for easier development on Windows and other platforms
- **ONVIF Implementation**: Complete ONVIF server implementation with PTZ, imaging, and media services
- **Code Quality Improvements**: Fixed compilation errors, improved code structure, and enhanced error handling
- **Build System**: Streamlined build process with proper Makefiles and build scripts
- **Web Interface Reorganization**: Separated ONVIF and legacy interfaces for better organization and user experience

## Summary

This is a simplified README with the latest features. For detailed hacking process and development information, see the [hack process documentation](hack_process/README.md).

### Working Features
- **RTSP stream** (720p on http://IP:554/vs0)
- **BMP snapshot** (up to 720p) on port 3000
- **JPEG snapshot**
- **H264 video recording** (no mp4 yet, but ffmpeg converts it)
- **Audio playback**
- **Audio recording to mp3**
- **PTZ movement**
- **IR shutter**
- **Combined web interface** with PTZ and IR on port 80
- **ONVIF web interface** with advanced PTZ, imaging, and preset controls
- **Motion detection**
- **ONVIF server** with full PTZ, imaging, and media services
- **Docker cross-compilation** environment for easy development

The camera can now be connected to a video recorder or monitor software such as MotionEye, and supports standard ONVIF protocols for integration with professional surveillance systems.

# Development Environment

## Docker Cross-Compilation (Recommended)

For easy development on Windows and other platforms, a Docker-based cross-compilation environment is provided:

```bash
# Build the Docker image
cd cross-compile
./docker-build.sh  # Linux/Mac
# or
.\docker-build.ps1  # Windows PowerShell

# Compile the ONVIF server
docker run --rm -v ${PWD}:/workspace anyka-cross-compile make -C /workspace/onvif

# Interactive development shell
docker run -it --rm -v ${PWD}:/workspace anyka-cross-compile
```

The Docker environment includes:
- Ubuntu 16.04 base with 32-bit library support
- Pre-installed Anyka ARM toolchain
- All necessary build dependencies
- Cross-compilation ready for AK3918 target

## Traditional Setup

For the original Ubuntu 16.04 setup, see the [hack process documentation](hack_process/README.md).

# Quick Start SD Card Hack

This hack runs only when the SD card is inserted leaving the camera unmodified. It is beginner friendly, and requires zero coding/terminal skills. See more [here](SD_card_contents/Factory)

It is unlikely that this can cause any harm to your camera as the system remains original, but no matter how small the risk it is never zero (unless you have the exact same camera). Try any of these hacks at your own risk.

The SD card hack is a safe way to test compatibility with the camera and to see if all features are working.

# Web Interface

The project includes two web interfaces with clear separation and easy switching between them.

## Web Interface Structure

```
www/cgi-bin/
├── header                    # Common header for all interfaces
├── footer                    # Common footer for all interfaces
├── webui_onvif              # Main ONVIF interface (recommended)
├── onvif_imaging            # ONVIF imaging controls
├── onvif_presets            # ONVIF PTZ preset management
└── legacy/                  # Legacy web UI implementation
    ├── webui                # Original web interface
    ├── events               # Event management
    ├── login                # Login page
    ├── login_validate.sh    # Login validation
    ├── settings             # Settings page
    ├── settings_submit.sh   # Settings submission
    ├── system               # System information
    ├── video                # Video playback
    ├── del_video.sh         # Video deletion
    └── pwd_change           # Password change
```

## Interface Comparison

### ONVIF Interface (Recommended)
- **Location**: `/cgi-bin/webui_onvif`
- **Features**:
  - ONVIF protocol compliance
  - Advanced PTZ controls
  - Imaging parameter adjustment
  - PTZ preset management
  - Real-time service status
  - Better integration with surveillance software

### Legacy Interface
- **Location**: `/cgi-bin/legacy/webui`
- **Features**:
  - Original ptz_daemon integration
  - libre_anyka_app integration
  - Basic PTZ controls
  - Event management
  - Settings configuration

## Navigation

### Main Entry Point
- **URL**: `http://[CAMERA_IP]/`
- **Behavior**: Redirects to ONVIF interface by default
- **Options**: Provides links to both interfaces

### ONVIF Interface Navigation
- Home → Imaging → Presets → Settings → System → Events
- Includes link to Legacy Interface

### Legacy Interface Navigation
- Home → Settings → System → Events
- Includes link to ONVIF Interface

## Access URLs

### ONVIF Interface
- Main: `http://[CAMERA_IP]/cgi-bin/webui_onvif`
- Imaging: `http://[CAMERA_IP]/cgi-bin/onvif_imaging`
- Presets: `http://[CAMERA_IP]/cgi-bin/onvif_presets`

### Legacy Interface
- Main: `http://[CAMERA_IP]/cgi-bin/legacy/webui`
- Events: `http://[CAMERA_IP]/cgi-bin/legacy/events`
- Settings: `http://[CAMERA_IP]/cgi-bin/legacy/settings`
- System: `http://[CAMERA_IP]/cgi-bin/legacy/system`

# ONVIF Server

A complete ONVIF server implementation with:

## Features

### Device Management
- Device information, capabilities, and discovery
- WS-Discovery for automatic device discovery on the network

### Media Services
- Video stream configuration and management
- RTSP streaming with H.264 video and audio

### PTZ Services
- Pan-tilt-zoom control with preset management
- Relative and absolute movement commands
- Preset storage and recall (up to 5 positions)

### Imaging Services
- Brightness, contrast, and saturation adjustment
- Real-time parameter adjustment
- Quick preset configurations

## Architecture

The ONVIF web interface communicates with the ONVIF server using HTTP POST requests with SOAP XML payloads:

```
Web Browser → CGI Script → HTTP POST → ONVIF Server → Hardware
```

### ONVIF Services Used

1. **PTZ Service** (`/onvif/ptz_service`)
   - `RelativeMove`: Move camera in specified direction
   - `Stop`: Stop all camera movement
   - `SetPreset`: Store current position as preset
   - `GotoPreset`: Move to saved preset position
   - `RemovePreset`: Delete saved preset

2. **Imaging Service** (`/onvif/imaging_service`)
   - `GetImagingSettings`: Retrieve current imaging parameters
   - `SetImagingSettings`: Apply new imaging parameters

3. **Device Service** (`/onvif/device_service`)
   - `GetDeviceInformation`: Check server status and capabilities

## Installation

1. **Copy ONVIF Server**: Ensure `onvifd` binary is compiled and available at `/mnt/anyka_hack/onvif/onvifd`

2. **Copy Web Interface**: The web interface files are already included in the SD card contents

3. **Start Services**: Use the provided startup script:
   ```bash
   /mnt/anyka_hack/web_interface/start_web_interface_onvif.sh
   ```

4. **Access Interface**: Open browser to `http://[CAMERA_IP]`

## Configuration

The ONVIF server uses the configuration file at `/etc/jffs2/anyka_cfg.ini`:

```ini
[global]
user = admin
secret = admin

[onvif]
enabled = 1
http_port = 8080

[ptz]
invert_directions = 0
max_pan_speed = 1.0
max_tilt_speed = 1.0

[imaging]
default_brightness = 0
default_contrast = 0
default_saturation = 0
```

Both interfaces share the same configuration files:
- `/data/gergesettings.txt`: Main settings
- `/mnt/tmp/token.txt`: Authentication token

## Current Status
- ✅ **Compilation**: All projects compile successfully with Docker environment
- ✅ **Basic ONVIF**: Device discovery, media, and PTZ services working
- ⚠️ **RTSP Streaming**: Basic implementation complete, some protocol compliance issues remain
- ⚠️ **Security**: No authentication implemented (as noted in original design)
- 🔧 **Known Issues**: See [REVIEW.md](REVIEW.md) for detailed technical analysis and improvement suggestions

## Dependencies

- **ONVIF Server**: Must be running on port 8080 (configurable)
- **Snapshot Service**: For live camera preview (port 3000)
- **Busybox HTTPD**: For web server functionality
- **curl**: For HTTP requests to ONVIF server

## Advantages over Legacy Interface

1. **Standard Protocol**: Uses industry-standard ONVIF protocol
2. **Better Integration**: Compatible with ONVIF-compliant software
3. **More Features**: Imaging controls and preset management
4. **Real-time Status**: Live monitoring of service status
5. **Future-proof**: Easily extensible with additional ONVIF services

## Troubleshooting

### ONVIF Server Not Responding
- Check if `onvifd` process is running: `ps | grep onvifd`
- Verify port 8080 is not blocked: `netstat -ln | grep 8080`
- Check configuration file: `/etc/jffs2/anyka_cfg.ini`

### PTZ Controls Not Working
- Ensure PTZ hardware is properly initialized
- Check ONVIF server logs for errors
- Verify profile token is correct (default: "MainProfile")

### Imaging Controls Not Working
- Check if imaging service is enabled in ONVIF server
- Verify video source token is correct
- Ensure camera supports imaging adjustments

# Legacy Applications

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

More info about the [app](SD_card_contents/anyka_hack/libre_anyka_app) and [source](cross-compile/libre_anyka_app).

**Note: the RTSP stream and snapshots are not protected by password. Do not expose to the internet!**

## SSH

[Dropbear](SD_card_contents/anyka_hack/dropbear) can give ssh access if telnet is not your preference.

## Play Sound

**Extracted from camera.**

More info about the [app](SD_card_contents/anyka_hack/ak_adec_demo)

## Record Sound

**mp3 recording works.**

More info about the [app](SD_card_contents/anyka_hack/aenc_demo) and [source](cross-compile/aenc_demo).


## Setup without SD

After the root and usr partitions are modified with the desired apps, the following steps are needed to get it working:

copy the webpage www folder to `/etc/jffs2/www` for httpd to serve from flash

The latest `gergehack.sh` is already capable of running fully local files, so update if needed and check sensor module settings.

This gives the following functions without SD card running on the camera:
- webUI on port 80
- usual ftp, telnet functions, and ntp time sync
- RTSP stream and snapshots for UI with libre_anyka_app
- ptz movement

# Info Links

These are the most important links here (this is where 99% of the info and resources come from):

https://gitea.raspiweb.com/Gerge/Anyka_ak3918_hacking_journey

https://github.com/helloworld-spec/qiwen/tree/main/anycloud39ev300 (explanation in chinese, good reference)

https://github.com/ricardojlrufino/anyka_v380ipcam_experiments/tree/master (ak_snapshot original)

https://github.com/kuhnchris/IOT-ANYKA-PTZdaemon (ptz daemon original)

https://github.com/MuhammedKalkan/Anyka-Camera-Firmware (Muhammed's RTSP app + library, and more discussions)

https://github.com/e27-camera-hack/E27-Camera-Hack/discussions/1 (discussion where most of this was worked on)

# Development

To modify the web interface:

1. **Edit CGI Scripts**: Modify the shell scripts in `www/cgi-bin/`
2. **Update SOAP Requests**: Modify the XML payloads for ONVIF requests
3. **Add New Features**: Create new CGI scripts for additional ONVIF services
4. **Test Changes**: Use the startup script to test modifications

## Future Enhancements

- **Authentication**: Add ONVIF authentication support
- **More Presets**: Increase preset storage capacity
- **Advanced Imaging**: Add more imaging parameters (sharpness, white balance, etc.)
- **Recording Controls**: Add video recording start/stop controls
- **Event Handling**: Add motion detection and event management