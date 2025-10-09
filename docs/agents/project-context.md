# Project Context - Anyka AK3918 Hacking Journey

## Project Description

This repository contains comprehensive reverse-engineering work and custom firmware development for Anyka AK3918-based IP cameras. It includes cross-compilation tools, SD-card bootable payloads, root filesystem modifications, and detailed documentation for understanding and extending camera functionality.

The project focuses on creating a fully ONVIF 2.5 compliant implementation while maintaining compatibility with the existing camera hardware and providing a robust development environment for camera firmware modifications.

## Repository Structure & Key Components

### Primary Development Areas

- **`cross-compile/onvif/`** — **CURRENT FOCUS** - Complete ONVIF 2.5 implementation with Device, Media, PTZ, and Imaging services. Full SOAP-based web services stack for IP camera control and streaming.
- **`cross-compile/onvif/tests/`** — **MANDATORY** - Unit testing framework using CMocka for testing utility functions in isolation. All utility functions must have corresponding unit tests.
- **`cross-compile/`** — Source code and build scripts for individual camera applications (e.g., `libre_anyka_app`, `aenc_demo`, `ak_snapshot`, `ptz_daemon`). Each subproject contains its own Makefile or build script and handles specific camera functionality.
- **`SD_card_contents/anyka_hack/`** — SD card payload system with web UI for runtime testing. This directory contains everything needed to boot custom firmware from an SD card, making it the easiest and safest way to test changes on the actual device without modifying flash memory.
- **`newroot/`** and prepared squashfs images (`busyroot.sqsh4`, `busyusr.sqsh4`, `newroot.sqsh4`) — Prebuilt root filesystem images for flashing to device. These are compressed squashfs images that replace the camera's root filesystem when flashed.
- **`hack_process/`**, **`README.md`**, **`Images/`**, and **`UART_logs/`** — Comprehensive documentation and debugging resources. Contains detailed guides, captured images, UART serial logs, and step-by-step hacking procedures for understanding and modifying the camera firmware.

### Reference Implementation

- **`cross-compile/anyka_reference/akipc/`** — Complete reference implementation (chip/vendor-provided) that shows how the original camera firmware implements APIs, initialization, and configuration. This is the authoritative source for understanding camera behavior.
- Use as canonical example when:
  - Reverse-engineering how the camera starts services and uses device files
  - Matching IPC/CLI commands and config keys used by webUI and other apps
  - Verifying binary and library ABI expectations before replacing or reimplementing a system binary
  - Understanding the camera's service architecture and initialization sequence
  - Learning proper device file usage and hardware abstraction patterns

### Component Libraries

- **`cross-compile/anyka_reference/component/`** — Comprehensive collection of reusable pieces extracted from stock firmware: drivers, third-party libraries, and helper tools. Contains pre-compiled binaries and headers for all major camera subsystems.
- **`cross-compile/anyka_reference/platform/`** — Board/platform-specific glue code: sensor selection and initialization, board pin mappings, GPIO configurations, and other low-level hardware integration components.

## Core Architecture

The system uses a layered architecture:

```
Web Interface → CGI Scripts → HTTP/SOAP → ONVIF Services → Platform Abstraction → Anyka SDK → Hardware
```

### ONVIF Service Implementation

The ONVIF implementation follows a modular architecture:

**Core Services** (`src/services/`):

- **Device Service** - Device information, capabilities, system date/time
- **Media Service** - Profile management, stream URIs, video/audio configurations
- **PTZ Service** - Pan/tilt/zoom control, preset management, continuous movement
- **Imaging Service** - Image settings, brightness, contrast, saturation controls

**Platform Abstraction** (`src/platform/platform_anyka.c`):

- Hardware-agnostic interface for all camera operations
- Anyka SDK integration with proper resource management
- Video/audio processing, PTZ control, IR LED management

**Network Layer** (`src/networking/`):

- HTTP/SOAP server with ONVIF protocol compliance
- RTSP streaming with H.264 encoding
- WS-Discovery for automatic device discovery
- Connection management with thread pooling

## Project Goals

1. **ONVIF 2.5 Compliance** - Complete implementation of all required ONVIF services
2. **Hardware Abstraction** - Platform-independent camera control interface
3. **Security Focus** - Defensive security with proper input validation
4. **Performance Optimization** - Efficient resource usage for embedded systems
5. **Comprehensive Testing** - Full test coverage with unit and integration tests
6. **Documentation** - Complete documentation for all components and APIs
7. **Maintainability** - Clean, well-structured code following best practices

## Technology Stack

- **Target Platform**: Anyka AK3918 ARM-based IP camera
- **Cross-Compilation**: arm-anykav200-linux-uclibcgnueabi toolchain
- **Development Environment**: WSL Ubuntu with bash
- **Testing Framework**: CMocka for unit tests, pytest for E2E tests
- **Documentation**: Doxygen for code documentation
- **Version Control**: Git with comprehensive commit history
- **Build System**: Make-based with native cross-compilation support

## Development Environment

### Quick Environment Summary

- **Primary OS**: WSL2 with Ubuntu
- **Shell**: **MANDATORY** - All commands use bash syntax
- **Cross-compilation**: Native tools for consistent builds
- **SD-card testing**: Primary development workflow for device testing

### Build Process

- **Use native cross-compilation tools**: `make -C cross-compile/<project>` - This ensures consistent builds in the WSL Ubuntu environment.
- **Test compilation** before committing changes - Always verify that code compiles successfully before making changes.
- **SD-card testing** is the primary development workflow for device testing - Copy compiled binaries to the SD card payload and boot the device with the SD card to test functionality.

### Build System Features

The Makefile supports:

- **Debug/Release builds**: `make debug` vs `make release` with automatic BUILD_TYPE handling
- **Documentation generation**: `make docs` (MANDATORY after changes)
- **Static analysis**: Multiple tools including Clang, Cppcheck, Snyk via `make static-analysis`
- **Compile commands**: `make compile-commands` for clangd support
- **Cross-compilation**: Uses arm-anykav200-linux-uclibcgnueabi toolchain
