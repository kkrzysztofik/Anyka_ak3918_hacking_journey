# Project Context - Anyka AK3918 Hacking Journey (Rust Edition)

## Project Description

This repository contains comprehensive reverse-engineering work and custom firmware development for Anyka AK3918-based IP cameras. The primary focus is now on **`onvif-rust`**, a complete rewrite of the ONVIF services stack using modern Rust technologies to ensure memory safety, concurrency, and maintainability.

The project aims to create a fully ONVIF 24.12 compliant implementation while maintaining compatibility with the existing camera hardware.

## Repository Structure & Key Components

### Primary Development Areas

- **`cross-compile/onvif-rust/`** — **CURRENT FOCUS** - The Rust-based ONVIF implementation.
  - **`src/`** - Source code for the ONVIF services (Device, Media, PTZ, Imaging).
  - **`tests/`** - Integration tests ensuring ONVIF compliance.
  - **`Cargo.toml`** - Dependency management and build configuration.
- **`SD_card_contents/anyka_hack/`** — SD card payload system for runtime testing.
- **`cross-compile/`** — Contains legacy C projects and toolchains.

## Technology Stack

- **Language**: Rust (Edition 2024)
- **Target Platform**: Anyka AK3918 ARM-based IP camera (`armv5te-unknown-linux-uclibceabi`)
- **Web Framework**: `axum` (0.8)
- **Async Runtime**: `tokio` (1.0) with multi-thread scheduler
- **Serialization**: `serde`, `quick-xml`
- **Logging**: `tracing`, `tracing-subscriber`
- **Error Handling**: `anyhow` (apps), `thiserror` (libs)
- **Build System**: `cargo` with `cross` or custom toolchain configuration

## Development Environment

### Quick Environment Summary

- **Primary OS**: Linux / WSL2
- **Toolchain**: `arm-anykav200-crosstool-ng` (or equivalent Rust target support)
- **Build Command**: `cargo build`
- **Test Command**: `cargo test`

### Build Process

- **Standard Build**: `cargo build --release`
- **Cross Compilation**: Ensure `.cargo/config.toml` is configured for the target linker.
- **Documentation**: `cargo doc --no-deps --open`
