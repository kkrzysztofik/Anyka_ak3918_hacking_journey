# Anyka AK3918 IP Camera Project Overview

## Project Purpose
This is a comprehensive reverse-engineering project for Chinese IP cameras based on Anyka AK3918 SoC, featuring:
- **Complete ONVIF 2.5 implementation** with Device, Media, PTZ, and Imaging services
- **Platform abstraction layer** for hardware-agnostic operations
- **RTSP streaming** with H.264 video and audio support
- **Native cross-compilation environment** in WSL Ubuntu for consistent builds
- **SD card payload system** for safe firmware testing without flash modifications
- **Comprehensive unit testing framework** using CMocka for utility function testing

## Core Architecture
The system uses a layered architecture:
```
Web Interface → CGI Scripts → HTTP/SOAP → ONVIF Services → Platform Abstraction → Anyka SDK → Hardware
```

## Key Features
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

## Development Focus
- **Current Focus**: `cross-compile/onvif/` - Complete ONVIF 2.5 implementation
- **Security**: Defensive security only - robust input validation and error handling
- **Platform**: WSL Ubuntu with native cross-compilation using bash syntax exclusively
- **Testing**: SD card payload system for safe device testing without flash modifications