# Build System Guide

## Development Environment

### WSL Ubuntu Development
- **Primary OS**: Development is conducted on WSL2 with Ubuntu
- **Shell**: **MANDATORY** - All terminal commands MUST use bash syntax
- **Cross-compilation**: Use native cross-compilation tools and toolchain
- **Path handling**: Use Unix-style paths throughout the development environment

### Bash Usage (MANDATORY)
- **ALL terminal commands MUST use bash syntax** - Standard Unix commands
- **File operations**: Use standard Unix commands like `ls`, `cp`, `rm`, `test`
- **Environment variables**: Use `$VARIABLE_NAME` syntax
- **Path separators**: Use forward slashes `/` for all paths
- **Build commands**: Use native make and cross-compilation tools

## Build Process

### Native Cross-Compilation
- **Use native cross-compilation tools**: `make -C cross-compile/<project>` - This ensures consistent builds in the WSL Ubuntu environment
- **Test compilation** before committing changes - Always verify that code compiles successfully before making changes
- **SD-card testing** is the primary development workflow for device testing - Copy compiled binaries to the SD card payload and boot the device with the SD card to test functionality

### Build Commands

#### Build and Compilation
```bash
# Build the ONVIF project
make -C cross-compile/onvif

# Build with verbose output
make -C cross-compile/onvif VERBOSE=1

# Clean build artifacts
make -C cross-compile/onvif clean

# Build specific target
make -C cross-compile/onvif onvifd

# Generate documentation
make -C cross-compile/onvif docs
```

#### File Operations
```bash
# List all C source files
find cross-compile/onvif/src -name "*.c" -type f

# List all header files
find cross-compile/onvif/src -name "*.h" -type f

# Copy binary to SD card
cp cross-compile/onvif/out/onvifd SD_card_contents/anyka_hack/usr/bin/

# Copy with backup
cp -f cross-compile/onvif/out/onvifd SD_card_contents/anyka_hack/usr/bin/

# Check if file exists
test -f cross-compile/onvif/out/onvifd

# Get file size
ls -l cross-compile/onvif/out/onvifd

# Compare files
diff file1.c file2.c
```

#### Code Analysis Commands
```bash
# Search for code duplication patterns
grep -r "strlen.*token.*32" cross-compile/onvif/src/ -n -A2 -B2

# Find all malloc calls
grep -r "malloc(" cross-compile/onvif/src/ -n -A1 -B1

# Find all free calls
grep -r "free(" cross-compile/onvif/src/ -n -A1 -B1

# Search for TODO comments
grep -r "TODO\|FIXME\|XXX" cross-compile/onvif/src/ -n -A1 -B1

# Find function definitions
grep -r "^[a-zA-Z_][a-zA-Z0-9_]*\s+[a-zA-Z_][a-zA-Z0-9_]*\s*(" cross-compile/onvif/src/ -n

# Count lines of code
find cross-compile/onvif/src -name "*.c" -exec wc -l {} + | tail -1
```

#### Documentation Commands
```bash
# Generate documentation
make -C cross-compile/onvif docs

# View documentation in browser
xdg-open cross-compile/onvif/docs/html/index.html

# Check if documentation exists
test -f cross-compile/onvif/docs/html/index.html

# List documentation files
find cross-compile/onvif/docs -type f
```

#### Environment and Setup
```bash
# Check system status
ps aux | grep onvifd

# List running processes
ps aux

# Check environment variables
echo $PWD
echo $BUILD_TYPE

# Set environment variables
export BUILD_TYPE="debug"
export LOG_LEVEL="DEBUG"

# Check bash version
bash --version
```

#### Network and Device Testing
```bash
# Test network connectivity
nc -zv 192.168.1.100 80

# Ping device
ping -c 4 192.168.1.100

# Check if port is open
nc -zv 192.168.1.100 554

# Download file from device
wget -O response.xml "http://192.168.1.100/onvif/device_service"
```

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

## Development Workflow

### Quick Start Entry Points

- **Fast iteration**: Edit or add files under `cross-compile/<app>/` and test via the SD-card hack in `SD_card_contents/anyka_hack/`. This allows testing changes without flashing the device.
- **Key documentation**:
  - Top-level `README.md` — project goals, features, SD-card hack quick start, and important device commands for UART access and debugging.
  - `cross-compile/README.md` — cross-compile environment setup notes and toolchain configuration.

### SD-Card Testing Workflow

1. **Edit code** in `cross-compile/<project>/`
2. **Build** using `make -C cross-compile/<project>`
3. **Copy binary** to `SD_card_contents/anyka_hack/usr/bin/`
4. **Boot device** with SD card inserted
5. **Test functionality** on actual hardware
6. **Debug** using UART logs if needed

### Platform Integration

- **Logging functionality** should be integrated into the platform*anyka abstraction and declared in platform.h, using standard function names instead of the ak* prefix.
- **Rootfs considerations**: When modifying `busybox` or other low-level component binaries, remember the rootfs size and `mksquashfs` options matter.

## Common Tasks & Examples

- **ONVIF Development**: "I modified `cross-compile/onvif/src/services/ptz/onvif_ptz.c` to improve preset handling — please build using native make, run unit tests, perform code review, and test PTZ functionality via ONVIF client."
- **Platform Updates**: "Update `cross-compile/onvif/src/platform/platform_anyka.c` to add better error handling for PTZ initialization failures — ensure proper code review for security and performance."
- **App Development**: "I changed `cross-compile/libre_anyka_app/main.c` to add logging — please build using native make, run unit tests, review for memory management issues, and test by copying the new binary to `SD_card_contents/anyka_hack/usr/bin/` and booting an SD-card image."
- **Web UI Updates**: "Patch updates `www/js/app.js` for the web UI. After applying, pack the SD payload and test UI on device at `http://<device-ip>`; capture network requests and serial log if UI doesn't load."
- **Documentation Updates**: "I added new functions to `cross-compile/onvif/src/services/device/onvif_device.c` — please update the Doxygen documentation, regenerate the HTML docs, and perform security review."
