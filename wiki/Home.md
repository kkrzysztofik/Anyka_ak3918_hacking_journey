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

## Quick Navigation

### Core Documentation

- [[ONVIF-Rust-Implementation]] - Complete ONVIF 24.12 implementation details
- [[Development-Environment]] - Toolchain setup and build instructions
- [[Web-Interface]] - Web UI documentation and navigation

### Development

- [[Development-Guide]] - Development workflow and best practices
- [[Static-Analysis-Tools]] - Code quality and security analysis tools
- [[Troubleshooting]] - Common issues and solutions

### Additional Resources

- [[Legacy-Applications]] - Legacy tools and applications
- [[Resources]] - External links and quick start guides
- [[Future-Enhancements]] - Planned features and roadmap

## Summary

This project provides a complete, modern ONVIF 24.12 implementation for Anyka AK3918 IP cameras, along with comprehensive development tools and documentation.

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

## Quick Start

For detailed setup instructions, see:

- [[Development-Environment]] - Toolchain and development setup
- [[ONVIF-Rust-Implementation]] - ONVIF server build and deployment
- [[Resources]] - Quick start SD card hack

---

**Note**: For the most up-to-date documentation, please refer to the individual wiki pages linked above.
