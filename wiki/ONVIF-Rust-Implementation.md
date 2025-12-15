# ONVIF Rust Implementation

The project features a complete, modern ONVIF 24.12 implementation written in **Rust** for the Anyka AK3918 IP camera. This is the primary ONVIF stack, replacing the legacy C implementation with a memory-safe, asynchronous architecture built on `tokio` and `axum`.

## Status

- **Current Status**: Active Development (Alpha)
- **Target Platform**: Anyka AK3918 (ARM926EJ-S, 32MB RAM)
- **ONVIF Compliance**: Profile S/T (Targeting v24.12)

> **⚠️ Current Limitation**: The ONVIF Rust implementation currently uses **stub implementations** for the Platform Abstraction Layer. Real Anyka hardware integration is not yet implemented. All ONVIF API calls are handled by stub/mock implementations for testing and development purposes. Hardware integration with the Anyka AK3918 platform is planned for future development.

## Features

- **Modern Stack**: Built on `tokio` (async runtime) and `axum` (web server)
- **Memory Safe**: Leverages Rust's ownership model to prevent common embedded pitfalls (buffer overflows, use-after-free)
- **ONVIF 24.12**: Targeting the latest ONVIF specifications
- **Implemented Services**:
  - **Device Service**: System configuration, network interfaces, users, device information
  - **Media Service**: Video profiles, RTSP stream URI generation, encoder configuration
  - **PTZ Service**: Pan/Tilt/Zoom control with preset management
  - **Imaging Service**: Image settings (brightness, contrast, saturation, sharpness)

## Quick Start

### ONVIF Rust Prerequisites

- Rust (Stable channel)
- `arm-anykav200-crosstool-ng` toolchain (located at `/home/kmk/anyka-dev/toolchain/arm-anykav200-crosstool-ng/`)

For detailed toolchain setup, see [[Development-Environment]].

### Build for Host (Testing)

```bash
cd cross-compile/onvif-rust
cargo build
cargo test
```

### Build for Target (Anyka AK3918)

> **⚠️ CRITICAL**: Always use the custom toolchain's cargo binary

```bash
cd cross-compile/onvif-rust
export CARGO=/home/kmk/anyka-dev/toolchain/arm-anykav200-crosstool-ng/bin/cargo
$CARGO build --release --target armv5te-unknown-linux-uclibceabi
```

The resulting binary will be at: `target/armv5te-unknown-linux-uclibceabi/release/onvif-rust`

## Documentation

For detailed information, please refer to the comprehensive developer documentation:

- **[README](https://github.com/kkrzysztofik/Anyka_ak3918_hacking_journey/blob/main/cross-compile/onvif-rust/README.md)** - Project overview and quick start
- **[Architecture Guide](https://github.com/kkrzysztofik/Anyka_ak3918_hacking_journey/blob/main/cross-compile/onvif-rust/doc/ARCHITECTURE.md)** - High-level design, module structure, and data flow
- **[Developer Guide](https://github.com/kkrzysztofik/Anyka_ak3918_hacking_journey/blob/main/cross-compile/onvif-rust/doc/DEVELOPER_GUIDE.md)** - Setup, build instructions, and contribution guidelines
- **[Memory Management](https://github.com/kkrzysztofik/Anyka_ak3918_hacking_journey/blob/main/cross-compile/onvif-rust/doc/MEMORY_MANAGEMENT.md)** - Memory allocation strategies and embedded system considerations
- **[Requirements](https://github.com/kkrzysztofik/Anyka_ak3918_hacking_journey/blob/main/cross-compile/onvif-rust/doc/REQUIREMENTS.md)** - Functional requirements and ONVIF service specifications
- **[Testing Strategy](https://github.com/kkrzysztofik/Anyka_ak3918_hacking_journey/blob/main/cross-compile/onvif-rust/doc/TESTING.md)** - Testing framework, unit tests, and integration tests

## ONVIF Server

The ONVIF server is implemented in Rust and provides a complete, memory-safe ONVIF 24.12 implementation. For detailed information about the ONVIF server architecture, features, and usage, please refer to the comprehensive documentation listed above.

## Quick Reference

The Rust-based ONVIF server implements:

- **Device Service**: System information, network configuration, user management
- **Media Service**: Video profiles, RTSP stream URI generation, encoder configuration
- **PTZ Service**: Pan/Tilt/Zoom control with preset management
- **Imaging Service**: Image parameter adjustment (brightness, contrast, saturation, sharpness)

## Troubleshooting

For common issues and solutions, see [[Troubleshooting]].

## See Also

- [[Development-Environment]] - Toolchain setup and build instructions
- [[Development-Guide]] - Development workflow and best practices
- [[Web-Interface]] - Web UI that communicates with the ONVIF server
- [[Troubleshooting]] - Common issues and solutions
