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

## 🔒 Security

> **⚠️ CRITICAL**: Never use authentication without TLS in production!

ONVIF 24.12 uses WS-Security with SHA-1/MD5 digest authentication. While this provides replay protection, credentials can be intercepted if transmitted over unencrypted HTTP.

**See [TLS Setup Guide](wiki/TLS-Setup.md) for configuration instructions.**

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

### Vendor Setup

The vendor directory contains Anyka SDK headers and static libraries required for FFI binding generation and linking. This setup is **required** for cross-compilation to the ARM target.

#### Automated Setup (Recommended)

Run the preparation script from the repository root:

```bash
./scripts/prepare_vendor.sh
```

This script will:

- Create the `vendor/include/` and `vendor/lib/` directories
- Copy all SDK headers from `cross-compile/onvif/include/` to `vendor/include/`
- Copy required static libraries from `cross-compile/anyka_reference/IOT-ANYKA-PTZdaemon/libs/` to `vendor/lib/`
- Verify that all critical headers and libraries are present

The script is idempotent and can be safely run multiple times.

#### Manual Setup

If needed, you can manually set up the vendor directory:

1. **Headers**: Copy headers from `cross-compile/onvif/include/` to `vendor/include/`

   ```bash
   mkdir -p cross-compile/onvif-rust/vendor/include
   cp -r cross-compile/onvif/include/* cross-compile/onvif-rust/vendor/include/
   ```

2. **Libraries**: Copy static libraries from `cross-compile/anyka_reference/IOT-ANYKA-PTZdaemon/libs/` to `vendor/lib/`

   ```bash
   mkdir -p cross-compile/onvif-rust/vendor/lib
   cp cross-compile/anyka_reference/IOT-ANYKA-PTZdaemon/libs/lib*.a cross-compile/onvif-rust/vendor/lib/
   ```

#### Verification

The build script (`build.rs`) automatically verifies vendor setup during compilation. If vendor files are missing or incomplete, the build will:

- Print warnings indicating which files are missing
- Fall back to stub implementations (for native testing builds)
- Provide instructions to run `scripts/prepare_vendor.sh`

#### Troubleshooting

- **Missing headers**: Ensure `cross-compile/onvif/include/` contains all SDK headers (40+ files)
- **Missing libraries**: Ensure `cross-compile/anyka_reference/IOT-ANYKA-PTZdaemon/libs/` contains all required `.a` files
- **Stale files**: Re-run `scripts/prepare_vendor.sh` to refresh copied files
- **Build warnings**: Check the build output for specific missing files and verify source directories exist

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
