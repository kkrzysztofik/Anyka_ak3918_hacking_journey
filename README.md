# Anyka AK3918 Hacking Journey

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

## Quick Start

### ONVIF Rust Implementation

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

> **⚠️ CRITICAL**: Always use the custom toolchain's cargo binary for target builds.

## Documentation

**📚 For comprehensive documentation, please visit the [GitHub Wiki](https://github.com/kkrzysztofik/Anyka_ak3918_hacking_journey/wiki).**

The wiki contains detailed information on:

- **Home** - Project overview and navigation
- **ONVIF Rust Implementation** - Complete ONVIF 24.12 implementation details
- **Development Environment** - Toolchain setup and build instructions
- **Web Interface** - Web UI documentation
- **Development Guide** - Development workflow and best practices
- **Troubleshooting** - Common issues and solutions
- **Static Analysis Tools** - Code quality and security analysis
- **Legacy Applications** - Legacy tools and applications
- **Resources** - External links and quick start guides
- **Future Enhancements** - Planned features and roadmap

## Key Features

- **ONVIF Rust Implementation** - Modern, memory-safe ONVIF 24.12 server
- **Memory Safety** - Rust's ownership model prevents buffer overflows and use-after-free errors
- **Asynchronous Architecture** - High-performance async I/O using `tokio` and `axum`
- **Comprehensive Testing** - Unit tests with `mockall` and integration test framework
- **Web Interfaces** - Both ONVIF and legacy interfaces with clear separation

## Status

- **Current Status**: Active Development (Alpha)
- **Target Platform**: Anyka AK3918 (ARM926EJ-S, 32MB RAM)
- **ONVIF Compliance**: Profile S/T (Targeting v24.12)

> **⚠️ Current Limitation**: The ONVIF Rust implementation currently uses **stub implementations** for the Platform Abstraction Layer. Real Anyka hardware integration is not yet implemented. All ONVIF API calls are handled by stub/mock implementations for testing and development purposes.

## Quick Reference

The ONVIF server implements:

- **Device Service**: System information, network configuration, user management
- **Media Service**: Video profiles, RTSP stream URI generation, encoder configuration
- **PTZ Service**: Pan/Tilt/Zoom control with preset management
- **Imaging Service**: Image parameter adjustment (brightness, contrast, saturation, sharpness)

---

**For detailed documentation, setup instructions, troubleshooting, and development guides, please see the [GitHub Wiki](https://github.com/kkrzysztofik/Anyka_ak3918_hacking_journey/wiki).**
