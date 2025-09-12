# Anyka_ak3918_hacking_journey

Reverse engineering and hacking journey for Chinese IP cameras based on Anyka AK3918 SoC

## Credits

This project was originally developed by **Gerge** (https://gitea.raspiweb.com/Gerge/Anyka_ak3918_hacking_journey) and has been continued with additional improvements and fixes. The original work involved extensive reverse engineering of the Anyka AK3918 platform and development of custom firmware and applications.

## Recent Updates

- **Complete Platform Abstraction Layer**: Full hardware abstraction implementation for Anyka AK3918 with proper SDK integration
- **ONVIF Server Implementation**: Complete ONVIF server with Device, Media, PTZ, and Imaging services
- **RTSP Streaming**: Robust RTSP server with H.264 video and audio streaming support
- **Docker Cross-Compilation Environment**: Added Docker-based cross-compilation setup for easier development on Windows and other platforms
- **Code Quality Improvements**: Fixed compilation errors, improved code structure, and enhanced error handling
- **Build System**: Streamlined build process with proper Makefiles and build scripts
- **Web Interface Reorganization**: Separated ONVIF and legacy interfaces for better organization and user experience

## Summary

This is a simplified README with the latest features. For detailed hacking process and development information, see the [hack process documentation](hack_process/README.md).

### Working Features
- **Complete Platform Abstraction Layer** - Full hardware abstraction for Anyka AK3918
- **ONVIF Server** - Complete implementation with Device, Media, PTZ, and Imaging services
- **RTSP Streaming** - H.264 video and audio streaming with proper resource management
- **PTZ Control** - Full pan-tilt-zoom functionality with preset management
- **Imaging Services** - Real-time video effects and day/night switching
- **IR LED Management** - Automatic night vision control
- **Configuration Management** - INI-style configuration with load/save functionality
- **Web Interfaces** - Both ONVIF and legacy interfaces with clear separation
- **Motion Detection** - Real-time motion detection capabilities
- **Audio Support** - Audio recording and playback functionality
- **Docker Cross-Compilation** - Easy development environment for all platforms

### Key Technical Achievements
- **Hardware Abstraction**: Clean, platform-agnostic interface for all hardware operations
- **ONVIF Compliance**: Full implementation of ONVIF Device, Media, PTZ, and Imaging services
- **RTSP Integration**: Robust streaming with proper video/audio synchronization
- **Resource Management**: Proper cleanup and resource handling using Anyka SDK
- **Error Handling**: Comprehensive error handling with standardized return codes
- **Logging System**: Unified logging interface with timestamp formatting

The camera can now be connected to professional surveillance software such as MotionEye, Blue Iris, or any ONVIF-compliant system, providing full integration with industry-standard protocols.

# Platform Abstraction Layer

The project features a comprehensive platform abstraction layer (`cross-compile/onvif/src/platform/platform_anyka.c`) that provides unified hardware access for the Anyka AK3918 platform.

## Core Components

### Video Processing
- **Video Input (VI)**: Device management, sensor resolution detection, day/night switching
- **Video Processing Subsystem (VPSS)**: Real-time effects (brightness, contrast, saturation, sharpness, hue)
- **Video Encoder**: H.264/H.265/MJPEG encoding with configurable parameters

### Audio Processing
- **Audio Input (AI)**: Audio capture and device management
- **Audio Encoder**: AAC, G.711, PCM encoding with synchronized streaming

### Control Systems
- **PTZ Control**: Complete pan-tilt-zoom functionality with coordinate mapping
- **IR LED Management**: Automatic night vision control and mode switching
- **Configuration Management**: INI-style configuration with load/save functionality

### System Services
- **Logging System**: Unified logging interface using Anyka's `ak_print()` function
- **Resource Management**: Proper cleanup and resource handling
- **Error Handling**: Standardized error codes and comprehensive error reporting

## API Design

The platform abstraction provides a clean, hardware-agnostic interface:

```c
// Video encoder stream for RTSP
platform_result_t platform_venc_get_stream(platform_venc_handle_t handle, 
                                          platform_venc_stream_t *stream, 
                                          uint32_t timeout_ms);

// PTZ control
platform_result_t platform_ptz_move_to_position(int pan_deg, int tilt_deg);

// Configuration management
const char* platform_config_get_string(const char *section, const char *key, 
                                      const char *default_value);
```

## Benefits

- **Hardware Independence**: Clean separation between application logic and hardware specifics
- **Maintainability**: Easy to modify or extend hardware support
- **Error Handling**: Consistent error reporting across all platform functions
- **Resource Management**: Proper cleanup and resource handling
- **Testing**: Easier unit testing with abstracted interfaces

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

A complete ONVIF server implementation with full platform abstraction integration:

## Features

### Device Management
- Device information, capabilities, and discovery
- WS-Discovery for automatic device discovery on the network
- Hardware abstraction for device-specific operations

### Media Services
- Video stream configuration and management
- RTSP streaming with H.264 video and synchronized audio
- Real-time stream generation using platform video encoder
- Proper resource management and frame handling

### PTZ Services
- Pan-tilt-zoom control with preset management
- Relative and absolute movement commands
- Preset storage and recall (up to 5 positions)
- Hardware abstraction for PTZ operations

### Imaging Services
- Brightness, contrast, saturation, sharpness, and hue adjustment
- Real-time parameter adjustment using VPSS effects
- Day/night mode switching with automatic IR LED control
- Quick preset configurations

## Architecture

The system uses a layered architecture with clear separation of concerns:

```
Web Browser → CGI Script → HTTP POST → ONVIF Server → Platform Abstraction → Anyka SDK → Hardware
```

### Component Layers

1. **Web Interface Layer**: CGI scripts and web pages for user interaction
2. **ONVIF Service Layer**: SOAP/HTTP server implementing ONVIF protocols
3. **Platform Abstraction Layer**: Hardware-agnostic interface for all operations
4. **SDK Integration Layer**: Direct integration with Anyka SDK functions
5. **Hardware Layer**: Physical camera hardware and peripherals

### Data Flow

- **Configuration**: INI files → Platform abstraction → Hardware settings
- **Video Streams**: Hardware → SDK → Platform abstraction → RTSP server → Network
- **PTZ Commands**: Web interface → ONVIF server → Platform abstraction → Hardware
- **Imaging Controls**: Web interface → ONVIF server → Platform abstraction → VPSS effects

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
- ✅ **Platform Abstraction**: Complete hardware abstraction layer implemented
- ✅ **ONVIF Server**: Full implementation with Device, Media, PTZ, and Imaging services
- ✅ **RTSP Streaming**: Robust implementation with H.264 video and audio streaming
- ✅ **PTZ Control**: Complete pan-tilt-zoom functionality with preset management
- ✅ **Imaging Services**: Real-time video effects and day/night switching
- ✅ **Configuration Management**: INI-style configuration with load/save functionality
- ✅ **Resource Management**: Proper cleanup and resource handling
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
6. **Hardware Abstraction**: Clean separation between application and hardware
7. **Robust Error Handling**: Comprehensive error reporting and recovery
8. **Resource Management**: Proper cleanup and resource handling
9. **Maintainability**: Easier to modify and extend functionality
10. **Professional Grade**: Suitable for integration with professional surveillance systems

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

- **Authentication**: Add ONVIF authentication support (HTTP Digest, WS-Security)
- **More Presets**: Increase preset storage capacity beyond current 5 positions
- **Advanced Imaging**: Add more imaging parameters (white balance, exposure, etc.)
- **Recording Controls**: Add video recording start/stop controls
- **Event Handling**: Add motion detection and event management
- **Profile S Compliance**: Full ONVIF Profile S compliance for advanced features
- **Audio Streaming**: Enhanced audio streaming with multiple codec support
- **Multi-Stream**: Support for multiple video streams simultaneously
- **Advanced PTZ**: More sophisticated PTZ features (tours, patterns, etc.)
- **Configuration UI**: Web-based configuration interface for platform settings