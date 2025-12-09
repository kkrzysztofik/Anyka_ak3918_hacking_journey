# Anyka_ak3918_hacking_journey

Reverse engineering and hacking journey for Chinese IP cameras based on Anyka AK3918 SoC

## Credits

This project was originally developed by **Gerge** (<https://gitea.raspiweb.com/Gerge/Anyka_ak3918_hacking_journey>) and has been continued with additional improvements and fixes. The original work involved extensive reverse engineering of the Anyka AK3918 platform and development of custom firmware and applications.

## Recent Updates

- **ONVIF Rust Rewrite**: Complete rewrite of ONVIF services in Rust for memory safety and modern async architecture
- **ONVIF 24.12 Compliance**: Targeting latest ONVIF specifications with Profile S/T support
- **Modern Stack**: Built on `tokio` and `axum` for high-performance asynchronous I/O
- **Comprehensive Testing**: Unit tests with `mockall` and integration test framework
- **Code Quality**: Rust's ownership model prevents common embedded pitfalls
- **Web Interface**: ONVIF and legacy interfaces with clear separation

## ONVIF Rust Implementation

The project features a complete, modern ONVIF 24.12 implementation written in **Rust** for the Anyka AK3918 IP camera. This is the primary ONVIF stack, replacing the legacy C implementation with a memory-safe, asynchronous architecture built on `tokio` and `axum`.

### Status

- **Current Status**: Active Development (Alpha)
- **Target Platform**: Anyka AK3918 (ARM926EJ-S, 32MB RAM)
- **ONVIF Compliance**: Profile S/T (Targeting v24.12)

> **⚠️ Current Limitation**: The ONVIF Rust implementation currently uses **stub implementations** for the Platform Abstraction Layer. Real Anyka hardware integration is not yet implemented. All ONVIF API calls are handled by stub/mock implementations for testing and development purposes. Hardware integration with the Anyka AK3918 platform is planned for future development.

### Features

- **Modern Stack**: Built on `tokio` (async runtime) and `axum` (web server)
- **Memory Safe**: Leverages Rust's ownership model to prevent common embedded pitfalls (buffer overflows, use-after-free)
- **ONVIF 24.12**: Targeting the latest ONVIF specifications
- **Implemented Services**:
  - **Device Service**: System configuration, network interfaces, users, device information
  - **Media Service**: Video profiles, RTSP stream URI generation, encoder configuration
  - **PTZ Service**: Pan/Tilt/Zoom control with preset management
  - **Imaging Service**: Image settings (brightness, contrast, saturation, sharpness)

### Quick Start

#### ONVIF Rust Prerequisites

- Rust (Stable channel)
- `arm-anykav200-crosstool-ng` toolchain (located at `/home/kmk/anyka-dev/toolchain/arm-anykav200-crosstool-ng/`)

#### Build for Host (Testing)

```bash
cd cross-compile/onvif-rust
cargo build
cargo test
```

#### Build for Target (Anyka AK3918)

> **⚠️ CRITICAL**: Always use the custom toolchain's cargo binary

```bash
cd cross-compile/onvif-rust
export CARGO=/home/kmk/anyka-dev/toolchain/arm-anykav200-crosstool-ng/bin/cargo
$CARGO build --release --target armv5te-unknown-linux-uclibceabi
```

The resulting binary will be at: `target/armv5te-unknown-linux-uclibceabi/release/onvif-rust`

### Documentation

For detailed information, please refer to the comprehensive developer documentation:

- **[README](cross-compile/onvif-rust/README.md)** - Project overview and quick start
- **[Architecture Guide](cross-compile/onvif-rust/doc/ARCHITECTURE.md)** - High-level design, module structure, and data flow
- **[Developer Guide](cross-compile/onvif-rust/doc/DEVELOPER_GUIDE.md)** - Setup, build instructions, and contribution guidelines
- **[Memory Management](cross-compile/onvif-rust/doc/MEMORY_MANAGEMENT.md)** - Memory allocation strategies and embedded system considerations
- **[Requirements](cross-compile/onvif-rust/doc/REQUIREMENTS.md)** - Functional requirements and ONVIF service specifications
- **[Testing Strategy](cross-compile/onvif-rust/doc/TESTING.md)** - Testing framework, unit tests, and integration tests

## Summary

This is a simplified README with the latest features. For detailed hacking process and development information, see the [hack process documentation](hack_process/README.md).

### Working Features

- **ONVIF Rust Implementation** - Modern, memory-safe ONVIF 24.12 server with Device, Media, PTZ, and Imaging services
- **Memory Safety** - Rust's ownership model prevents buffer overflows and use-after-free errors
- **Asynchronous Architecture** - High-performance async I/O using `tokio` and `axum`
- **PTZ Control** - Pan-tilt-zoom functionality with preset management (currently using stub implementation)
- **Imaging Services** - Image parameter adjustment (brightness, contrast, saturation, sharpness) (currently using stub implementation)
- **Platform Abstraction** - Clean hardware abstraction layer for portability and testing (currently using `StubPlatform` for development)
- **Comprehensive Testing** - Unit tests with `mockall` and integration test framework
- **Web Interfaces** - Both ONVIF and legacy interfaces with clear separation

### Key Technical Achievements

- **Memory Safety**: Rust's ownership model eliminates entire classes of embedded bugs
- **ONVIF 24.12 Compliance**: Full implementation targeting Profile S/T specifications
- **Modern Async Stack**: High-performance asynchronous I/O with `tokio` and `axum`
- **Platform Abstraction**: Clean trait-based interface for hardware operations
- **Comprehensive Testing**: Unit and integration tests with mocking support
- **Resource Management**: Proper cleanup and resource handling through Rust's RAII
- **Error Handling**: Type-safe error handling with `Result` types

The camera can now be connected to professional surveillance software such as MotionEye, Blue Iris, or any ONVIF-compliant system, providing full integration with industry-standard protocols.

## Development Environment

### Building the Custom Toolchain

The project uses a custom cross-compilation toolchain built with crosstool-NG that includes GCC, LLVM/Clang, and Rust support for the Anyka AK3918 target.

#### Toolchain Build Prerequisites

- **System Requirements**:
  - Linux or WSL environment
  - At least 10GB free disk space
  - 4GB+ RAM recommended
  - 2-4 hours build time (depending on CPU)

- **Build Dependencies**:

```bash
sudo apt-get update
sudo apt-get install -y \
    build-essential \
    libncurses-dev \
    gperf \
    bison \
    flex \
    texinfo \
    help2man \
    gawk \
    libtool-bin \
    automake \
    autoconf \
    wget \
    git \
    file \
    python3 \
    python3-dev \
    cmake \
    ninja-build \
    curl \
    unzip \
    xz-utils \
    perl \
    pkg-config
```

### Building the Toolchain

#### Step 1: Navigate to the build directory

```bash
cd /home/kmk/anyka-dev/toolchain/build-new
```

#### Step 2: Build the GCC toolchain

```bash
./build_toolchain.sh
```

This will:

- Download and build crosstool-NG 1.28.0
- Configure for ARMv5TEJ with uClibc-ng
- Build GCC 15.2, Binutils 2.45, and GDB 16.3
- Install to `../arm-anykav200-crosstool-ng/usr/`

#### Step 3: Build LLVM/Clang (Required for Rust)

```bash
./build_llvm.sh
```

This builds LLVM 18.1.8 with Clang and LLD for cross-compilation support.

#### Step 4: Bootstrap Rust from Source

```bash
./bootstrap_rust.sh
```

This will:

- Clone Rust source code
- Add `armv5te-unknown-linux-uclibceabi` target specification
- Configure Rust to use the custom LLVM
- Build Rust compiler and std library for the target
- Install to `../arm-anykav200-crosstool-ng/`

#### Step 5: Verify the installation

```bash
./verify_rust.sh
```

After building, the toolchain will be available at:

- `../arm-anykav200-crosstool-ng/bin/rustc`
- `../arm-anykav200-crosstool-ng/bin/cargo`
- `../arm-anykav200-crosstool-ng/bin/clang`

For detailed toolchain build instructions, see the [Toolchain Build README](toolchain/build-new/README.md).

## Rust Development Setup

The ONVIF Rust project uses the custom Rust toolchain for cross-compilation to the Anyka AK3918 target.

### Rust Development Prerequisites

- **Custom Toolchain**: Built using the steps above (located at `/home/kmk/anyka-dev/toolchain/arm-anykav200-crosstool-ng/`)

### Rust Development Quick Start

```bash
cd cross-compile/onvif-rust

# Use the custom toolchain's cargo
export CARGO=/home/kmk/anyka-dev/toolchain/arm-anykav200-crosstool-ng/bin/cargo

# Build for host (testing)
$CARGO build
$CARGO test

# Build for target (Anyka AK3918)
$CARGO build --release --target armv5te-unknown-linux-uclibceabi
```

For detailed setup instructions, see the [Developer Guide](cross-compile/onvif-rust/doc/DEVELOPER_GUIDE.md).

## Traditional Setup

For the original Ubuntu 16.04 setup and other legacy applications, see the [hack process documentation](hack_process/README.md).

## Quick Start SD Card Hack

This hack runs only when the SD card is inserted leaving the camera unmodified. It is beginner friendly, and requires zero coding/terminal skills. See more in the [SD card factory documentation](SD_card_contents/Factory)

It is unlikely that this can cause any harm to your camera as the system remains original, but no matter how small the risk it is never zero (unless you have the exact same camera). Try any of these hacks at your own risk.

The SD card hack is a safe way to test compatibility with the camera and to see if all features are working.

## Web Interface

The project includes two web interfaces with clear separation and easy switching between them. The web interfaces communicate with the ONVIF server (Rust implementation) via HTTP/SOAP requests.

### Web Interface Structure

```text
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

### Legacy Interface URLs

- Main: `http://[CAMERA_IP]/cgi-bin/legacy/webui`
- Events: `http://[CAMERA_IP]/cgi-bin/legacy/events`
- Settings: `http://[CAMERA_IP]/cgi-bin/legacy/settings`
- System: `http://[CAMERA_IP]/cgi-bin/legacy/system`

## ONVIF Server

The ONVIF server is implemented in Rust and provides a complete, memory-safe ONVIF 24.12 implementation. For detailed information about the ONVIF server architecture, features, and usage, please refer to the [ONVIF Rust Implementation](#onvif-rust-implementation) section above and the comprehensive documentation:

- **[ONVIF Rust README](cross-compile/onvif-rust/README.md)** - Project overview and features
- **[Architecture Guide](cross-compile/onvif-rust/doc/ARCHITECTURE.md)** - System architecture and design
- **[Developer Guide](cross-compile/onvif-rust/doc/DEVELOPER_GUIDE.md)** - Build and deployment instructions
- **[Requirements](cross-compile/onvif-rust/doc/REQUIREMENTS.md)** - ONVIF service specifications

## Quick Reference

The Rust-based ONVIF server implements:

- **Device Service**: System information, network configuration, user management
- **Media Service**: Video profiles, RTSP stream URI generation, encoder configuration
- **PTZ Service**: Pan/Tilt/Zoom control with preset management
- **Imaging Service**: Image parameter adjustment (brightness, contrast, saturation, sharpness)

## Troubleshooting

### ONVIF Server Not Responding

- Check if the `onvif-rust` process is running: `ps | grep onvif-rust`
- Verify the server port is not blocked: `netstat -ln | grep 8080`
- Check server logs for error messages
- Ensure the Rust binary is properly compiled and deployed

### PTZ Controls Not Working

- **Note**: Currently using stub implementation - hardware integration is not yet implemented
- Ensure PTZ hardware is properly initialized (when hardware integration is available)
- Check ONVIF server logs for errors
- Verify profile token is correct (default: "MainProfile")
- Ensure the platform abstraction layer is correctly configured

### Imaging Controls Not Working

- **Note**: Currently using stub implementation - hardware integration is not yet implemented
- Check if imaging service is enabled in the ONVIF server
- Verify video source token is correct
- Ensure camera supports imaging adjustments (when hardware integration is available)
- Check platform abstraction layer for hardware support

## Legacy Applications

**Note**: The applications in this section are separate legacy utilities and are not part of the ONVIF Rust implementation. These are standalone tools that may be used independently or alongside the ONVIF server.

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

## Info Links

These are the most important links here (this is where 99% of the info and resources come from):

<https://gitea.raspiweb.com/Gerge/Anyka_ak3918_hacking_journey>

<https://github.com/helloworld-spec/qiwen/tree/main/anycloud39ev300> (explanation in chinese, good reference)

<https://github.com/ricardojlrufino/anyka_v380ipcam_experiments/tree/master> (ak_snapshot original)

<https://github.com/kuhnchris/IOT-ANYKA-PTZdaemon> (ptz daemon original)

<https://github.com/MuhammedKalkan/Anyka-Camera-Firmware> (Muhammed's RTSP app + library, and more discussions)

<https://github.com/e27-camera-hack/E27-Camera-Hack/discussions/1> (discussion where most of this was worked on)

## Development

### ONVIF Rust Development

For developing the ONVIF Rust implementation:

1. **Setup Environment**: Follow the [Developer Guide](cross-compile/onvif-rust/doc/DEVELOPER_GUIDE.md)
2. **Build and Test**: Use `cargo build` and `cargo test` with the custom toolchain
3. **Code Quality**: Run `cargo clippy` and `cargo fmt` for linting and formatting
4. **Add Services**: Implement new ONVIF services following the existing patterns

## Web Interface Development

To modify the web interface:

1. **Edit CGI Scripts**: Modify the shell scripts in `www/cgi-bin/`
2. **Update SOAP Requests**: Modify the XML payloads for ONVIF requests (communicates with Rust ONVIF server)
3. **Add New Features**: Create new CGI scripts for additional ONVIF services
4. **Test Changes**: Use the startup script to test modifications

## Static Analysis Tools

The project includes comprehensive static analysis tools integrated into the Anyka cross-compile environment to ensure code quality, security, and reliability.

## Available Tools

### 1. Clang Static Analyzer

- **Purpose**: Advanced symbolic execution and path-sensitive analysis
- **Detects**: Memory leaks, buffer overflows, null pointer dereferences, uninitialized variables
- **Output**: HTML reports with detailed analysis paths

### 2. Cppcheck

- **Purpose**: Bug detection and code quality analysis
- **Detects**: Undefined behavior, memory leaks, buffer overflows, style issues
- **Output**: XML and HTML reports

### 3. Snyk Code

- **Purpose**: Advanced security vulnerability scanning and code analysis
- **Detects**: Security vulnerabilities, code quality issues, dependency vulnerabilities
- **Output**: JSON and SARIF reports
- **Authentication**: Requires SNYK_TOKEN for full functionality

## Usage Methods

### Method 1: PowerShell Script (Recommended)

Use the provided PowerShell script for easy analysis:

```powershell
# Run all static analysis tools
.\static-analysis.ps1

# Run specific tool
.\static-analysis.ps1 -Tool clang
.\static-analysis.ps1 -Tool cppcheck
.\static-analysis.ps1 -Tool snyk

# Run with Snyk authentication token
.\static-analysis.ps1 -Tool snyk -SnykToken "your-api-token-here"

# Verbose output
.\static-analysis.ps1 -Verbose

# Custom output directory
.\static-analysis.ps1 -OutputDir "my-analysis-results"
```

**Note**: For the ONVIF Rust project, use Rust's built-in tools:

- `cargo clippy -- -D warnings` for linting
- `cargo fmt --check` for formatting
- `cargo test` for testing

## Output Files

After running analysis, results are saved in the `analysis-results/` directory:

```text
analysis-results/
├── clang/                    # Clang Static Analyzer results
│   └── index.html           # Main HTML report
├── cppcheck-results.xml     # Cppcheck XML output
├── cppcheck-html/           # Cppcheck HTML report
│   └── index.html
├── snyk-results.json        # Snyk JSON output
└── snyk-results.sarif       # Snyk SARIF output
```

## Viewing Results

### Clang Static Analyzer

- Open `analysis-results/clang/index.html` in your browser
- Shows detailed analysis paths with source code highlighting
- Click on issues to see the execution path that leads to the problem

### Cppcheck

- Open `analysis-results/cppcheck-html/index.html` in your browser
- Shows categorized issues with severity levels
- Includes suggestions for fixes

### Snyk Code

- Open `analysis-results/snyk-results.json` for programmatic analysis
- Open `analysis-results/snyk-results.sarif` for SARIF-compatible tools
- Shows security vulnerabilities with detailed remediation guidance
- Includes severity levels and CVE information when available

## Snyk Authentication Setup

### Getting Your Snyk API Token

1. **Create a free Snyk account**: Visit <https://app.snyk.io/>
2. **Navigate to Account Settings**:
   - Click on your profile → Account Settings
   - Go to General Settings → API Token
3. **Copy your token**: Click "click to show" to reveal your API token
4. **Store securely**: Never commit this token to source control

### Authentication Methods

#### Method 1: Environment Variable (Recommended)

```powershell
# Set environment variable for current session
$env:SNYK_TOKEN = "your-api-token-here"

# Or set permanently (Windows)
[Environment]::SetEnvironmentVariable("SNYK_TOKEN", "your-api-token-here", "User")
```

#### Method 2: Script Parameter

```powershell
# Pass token directly to script
.\static-analysis.ps1 -Tool snyk -SnykToken "your-api-token-here"
```

### Authentication Behavior

- **With Token**: Full Snyk functionality, cloud-based analysis, latest vulnerability database
- **Without Token**: Limited offline mode, basic analysis only
- **Script Warnings**: The script will warn you if no token is provided

## Integration with Development Workflow

### Pre-commit Analysis

Add to your development workflow:

```powershell
# Before committing changes
.\static-analysis.ps1 -Tool all
# Review results and fix issues before committing
```

To install repository-provided git hooks that run local validations (e.g., Rust `cargo fmt` for `onvif-rust`), run:

```bash
scripts/install-git-hooks.sh
```

To revert this change run:

```bash
git config --unset core.hooksPath
```

### CI/CD Integration

The tools can be integrated into CI/CD pipelines:

```yaml
# For ONVIF Rust project
- name: Run Rust Linting
  run: |
    cd cross-compile/onvif-rust
    cargo clippy -- -D warnings
    cargo fmt --check
```

### IDE Integration

- **VS Code**: Install C/C++ extension for real-time analysis
- **CLion**: Built-in static analysis support
- **Vim/Neovim**: Use ALE or coc.nvim with clangd

## Common Issues and Solutions

### Missing Include Files

If you see "missing include" warnings:

- These are often false positives for system headers
- Use `--suppress=missingIncludeSystem` flag for cppcheck
- The tools focus on your source code, not system dependencies

### Memory Analysis

For embedded systems like Anyka AK3918:

- Pay attention to memory leak warnings
- Check for buffer overflow vulnerabilities
- Verify proper resource cleanup

### Security Analysis

Focus on:

- Input validation issues
- Buffer overflow vulnerabilities
- Unsafe string operations
- Authentication and authorization flaws

## Customization

### Adding Custom Rules

Snyk uses built-in security rules, but you can configure severity thresholds and exclusions:

```bash
# Set severity threshold
snyk code test --severity-threshold=medium

# Exclude specific files or directories
snyk code test --exclude=test/,vendor/
```

### Suppressing False Positives

Add comments to suppress specific warnings:

```c
// cppcheck-suppress nullPointer
if (ptr == NULL) return;

// snyk: disable-next-line
strcpy(dest, src);  // Known safe in this context
```

## Best Practices

1. **Run analysis regularly** - Integrate into your development workflow
2. **Fix high-severity issues first** - Address security and memory issues immediately
3. **Review false positives** - Understand why tools flag certain code
4. **Use multiple tools** - Each tool has different strengths
5. **Keep tools updated** - Use latest versions for better analysis

### Static Analysis Troubleshooting

#### Permission Issues

```powershell
# Fix file permissions if needed
Get-ChildItem -Path "analysis-results" -Recurse | ForEach-Object { $_.Attributes = "Normal" }
```

### Memory Issues

```powershell
# Run analysis on smaller subsets if memory is limited
.\static-analysis.ps1 -Tool cppcheck  # Start with cppcheck (lightest)
```

## Further Reading

- [Clang Static Analyzer Documentation](https://clang.llvm.org/docs/analyzer/)
- [Cppcheck Manual](http://cppcheck.sourceforge.net/manual.pdf)
- [Snyk Documentation](https://docs.snyk.io/)
- ONVIF Project Coding Standards (see project documentation)

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
