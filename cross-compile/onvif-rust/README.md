# Anyka AK3918 ONVIF Services (Rust)

> **Status**: Active Development (Alpha)
> **Target**: Anyka AK3918 (ARM926EJ-S)
> **Compliance**: ONVIF Profile S/T (Targeting v24.12)

This project is a complete rewrite of the ONVIF services stack for the Anyka AK3918 IP camera, built using **Rust** for safety, performance, and maintainability. It replaces the legacy C implementation with a modern, asynchronous stack based on `tokio` and `axum`.

## 🚀 Features

- **Modern Stack**: Built on `tokio` (Async Runtime) and `axum` (Web Server).
- **Memory Safe**: Leverages Rust's ownership model to prevent common embedded pitfalls (buffer overflows, use-after-free).
- **ONVIF 24.12**: Targeting the latest ONVIF specifications.
- **Implemented Services**:
  - **Device**: System configuration, network, users.
  - **Media**: Video profiles, RTSP stream URI generation.
  - **PTZ**: Pan/Tilt/Zoom control.
  - **Imaging**: Image settings (Brightness, Contrast).

## 📚 Documentation

- **[Architecture Guide](doc/ARCHITECTURE.md)**: High-level design, module structure, and data flow.
- **[Developer Guide](doc/DEVELOPER_GUIDE.md)**: Setup, build instructions, and contribution guidelines.
- **[Memory Management](doc/MEMORY_MANAGEMENT.md)**: Memory allocation strategies, constraints, and embedded system considerations.
- **[Requirements](doc/REQUIREMENTS.md)**: Functional requirements and ONVIF service specifications.
- **[Testing](doc/TESTING.md)**: Testing strategy, unit tests, integration tests, and test execution.

## 🛠️ Quick Start

### Prerequisites

- Rust (Stable)
- `arm-anykav200-crosstool-ng` toolchain

### Build for Host (Testing)

```bash
cargo build
cargo test
```

### Build for Target (Anyka AK3918)

```bash
cargo build --release --target armv5te-unknown-linux-uclibceabi
```

## 🤝 Contributing

Please refer to the [Developer Guide](doc/DEVELOPER_GUIDE.md) for details on adding new services and running tests. Ensure all code follows the project's [Development Standards](../../.serena/memories/development-standards.md).

## 📄 License

This project is part of the Anyka Hacking Journey. See the root repository for license details.
