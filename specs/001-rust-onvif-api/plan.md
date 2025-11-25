# Technical Implementation Plan: Rust ONVIF API Rewrite

**Feature Branch**: `001-rust-onvif-api`
**Created**: 2025-01-27
**Status**: Draft
**Based on**: [spec.md](./spec.md)

## Executive Summary

This plan outlines the technical approach for rewriting the ONVIF application from C to Rust, focusing on the ONVIF API layer first. The implementation will be based on ONVIF Device Test Specifications version 24.12 and will maintain compatibility with the existing platform abstraction, Anyka SDK integration, and configuration format.

## Architecture Overview

### High-Level Architecture

```text
┌─────────────────────────────────────────────────────────────┐
│                    ONVIF Rust Application                    │
├─────────────────────────────────────────────────────────────┤
│  ONVIF API Layer (Rust)                                      │
│  ├── HTTP/SOAP Server (axum + quick-xml)                    │
│  ├── Service Handlers (Device, Media, PTZ, Imaging)        │
│  └── ONVIF Protocol (SOAP XML parsing/serialization)        │
├─────────────────────────────────────────────────────────────┤
│  Platform Abstraction Layer (Rust FFI)                       │
│  ├── FFI Bindings to Anyka SDK (bindgen)                    │
│  ├── Platform Traits (Video, Audio, PTZ, Imaging)            │
│  └── Hardware Abstraction (stubs for testing)                │
├─────────────────────────────────────────────────────────────┤
│  Configuration System (Rust)                                  │
│  ├── TOML Parser (toml crate)                                │
│  ├── Schema Validation                                        │
│  └── Runtime Configuration Manager                           │
├─────────────────────────────────────────────────────────────┤
│  Logging System (Rust)                                        │
│  ├── Tracing Framework (tracing + tracing-subscriber)       │
│  ├── Tracing-Log Bridge (bidirectional logging)               │
│  └── Platform Log Integration (FFI to ak_print)              │
├─────────────────────────────────────────────────────────────┤
│  Anyka SDK (C - via FFI)                                      │
│  ├── Video Input/Encoder (ak_vi, ak_venc)                   │
│  ├── Audio Input/Encoder (ak_ai, ak_aenc)                   │
│  ├── PTZ Control (ak_drv_ptz)                               │
│  └── Imaging/VPSS (ak_vpss, ak_drv_irled)                  │
└─────────────────────────────────────────────────────────────┘
```

## Build System & Cross-Compilation

### Project Structure

```text
cross-compile/onvif-rust/
├── Cargo.toml                 # Main Rust project configuration
├── build.rs                   # Build script for FFI compilation
├── arm-unknown-linux-uclibcgnueabi.json  # Custom target spec
├── .cargo/
│   └── config.toml           # Cross-compilation configuration
├── src/
│   ├── main.rs               # Application entry point
│   ├── lib.rs                # Library root
│   ├── onvif/                # ONVIF API implementation
│   │   ├── mod.rs
│   │   ├── device/           # Device Service
│   │   ├── media/            # Media Service
│   │   ├── ptz/              # PTZ Service
│   │   └── imaging/          # Imaging Service
│   ├── platform/             # Platform abstraction
│   │   ├── mod.rs
│   │   ├── anyka/            # Anyka SDK FFI bindings
│   │   ├── traits.rs         # Platform trait definitions
│   │   └── stubs.rs          # Hardware stubs for testing
│   ├── config/               # Configuration system
│   │   ├── mod.rs
│   │   ├── schema.rs         # Configuration schema
│   │   ├── runtime.rs        # Runtime configuration manager
│   │   └── storage.rs        # TOML/INI file storage
│   ├── logging/              # Logging system
│   │   ├── mod.rs
│   │   └── platform.rs       # Platform log integration
│   └── ffi/                  # C FFI bindings
│       ├── mod.rs
│       ├── anyka_sdk.h       # Anyka SDK header bindings
│       └── anyka_sdk.rs      # Generated bindings
├── tests/                    # Integration tests
│   ├── onvif/                # ONVIF test specification tests
│   └── platform/             # Platform abstraction tests
└── scripts/                  # Build and deployment scripts
    ├── build.sh              # Build script
    └── verify_binary.sh      # Binary verification
```

### Build Configuration

**Cargo.toml** (based on rust-hello PoC):

```toml
[package]
name = "onvif-rust"
version = "0.1.0"
edition = "2024"
[dependencies]
# Async runtime
tokio = { version = "1", features = ["rt-multi-thread", "macros", "signal", "net"] }
# HTTP server
axum = "0.7"
# SOAP/XML handling - using quick-xml for performance and serde support
quick-xml = { version = "0.31", features = ["serialize"] }
# Configuration - TOML format (Rust standard) or INI (alternative)
toml = "0.8"  # Recommended: TOML for Rust ecosystem alignment
# ini = "1.3"  # Alternative: INI if compatibility required
serde = { version = "1.0", features = ["derive"] }
# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
tracing-log = "0.2"  # Bridge between log crate and tracing
# FFI
libc = "0.2"
# Error handling
anyhow = "1.0"
thiserror = "1.0"
# Testing
[dev-dependencies]
tokio-test = "0.4"
mockall = "0.12"

[build-dependencies]
cc = "1.0"
bindgen = "0.69"

[profile.release]
opt-level = "z"     # Optimize for size
lto = true          # Link-time optimization
codegen-units = 1   # Better optimization
strip = true        # Strip symbols
```

**Cross-Compilation Setup** (`.cargo/config.toml` - following xiu pattern):

```toml
[build]
target = "armv5te-unknown-linux-uclibceabi"
rustc = "/home/kmk/anyka-dev/toolchain/arm-anykav200-crosstool-ng/bin/rustc"

[target.armv5te-unknown-linux-uclibceabi]
# Use clang from new toolchain (not original gcc)
linker = "/home/kmk/anyka-dev/toolchain/arm-anykav200-crosstool-ng/bin/clang"
rustflags = [
  "-C", "link-arg=--target=arm-unknown-linux-uclibcgnueabi",
  "-C", "link-arg=--sysroot=/home/kmk/anyka-dev/toolchain/arm-anykav200-crosstool-ng/arm-unknown-linux-uclibcgnueabi/sysroot",
  "-C", "link-arg=-march=armv5te",
  "-C", "link-arg=-mfloat-abi=soft",
  "-C", "link-arg=-mtune=arm926ej-s",
  # Use dynamic linker from /mnt/anyka_hack/lib (xiu pattern)
  "-C", "link-arg=-Wl,--dynamic-linker=/mnt/anyka_hack/lib/ld-uClibc.so.1",
]

[env]
CC_armv5te_unknown_linux_uclibceabi = "/home/kmk/anyka-dev/toolchain/arm-anykav200-crosstool-ng/bin/arm-unknown-linux-uclibcgnueabi-gcc"
CXX_armv5te_unknown_linux_uclibceabi = "/home/kmk/anyka-dev/toolchain/arm-anykav200-crosstool-ng/bin/arm-unknown-linux-uclibcgnueabi-g++"
AR_armv5te_unknown_linux_uclibceabi = "/home/kmk/anyka-dev/toolchain/arm-anykav200-crosstool-ng/bin/arm-unknown-linux-uclibcgnueabi-ar"
```

## Subsystem Implementation Plans

### 1. Platform Abstraction Layer

#### 1.1 FFI Bindings to Anyka SDK

**Approach**: Use `bindgen` to generate Rust bindings from Anyka SDK headers.

**Key Headers to Bind**:

- `ak_vi.h` - Video input
- `ak_venc.h` - Video encoder
- `ak_ai.h` - Audio input
- `ak_aenc.h` - Audio encoder
- `ak_drv_ptz.h` - PTZ control
- `ak_vpss.h` - Video processing
- `ak_drv_irled.h` - IR LED control
- `ak_common.h` - Common types and error codes

**Implementation Steps**:

1. Create `build.rs` script to generate bindings using `bindgen`
2. Generate bindings for all Anyka SDK headers
3. Create safe Rust wrappers around unsafe FFI calls
4. Implement error handling for FFI operations
5. Add platform trait abstractions for testability

**Example Structure**:

```rust
// src/platform/anyka/mod.rs
pub mod vi;      // Video input bindings
pub mod venc;    // Video encoder bindings
pub mod ai;      // Audio input bindings
pub mod aenc;    // Audio encoder bindings
pub mod ptz;     // PTZ bindings
pub mod vpss;    // VPSS bindings
pub mod irled;   // IR LED bindings
```

#### 1.2 Platform Traits

**Purpose**: Define platform-agnostic interfaces for hardware operations.

**Key Traits**:

```rust
// src/platform/traits.rs
pub trait VideoInput {
    fn open(&mut self) -> Result<VideoInputHandle>;
    fn close(&mut self, handle: VideoInputHandle);
    fn get_resolution(&self, handle: VideoInputHandle) -> Result<Resolution>;
}

pub trait VideoEncoder {
    fn init(&mut self, config: VideoEncoderConfig) -> Result<VideoEncoderHandle>;
    fn get_stream(&self, handle: VideoEncoderHandle) -> Result<VideoStream>;
}

pub trait PTZControl {
    fn move_to_position(&mut self, pan: f32, tilt: f32, zoom: f32) -> Result<()>;
    fn get_position(&self) -> Result<PTZPosition>;
}

pub trait ImagingControl {
    fn set_brightness(&mut self, value: i32) -> Result<()>;
    fn set_contrast(&mut self, value: i32) -> Result<()>;
    // ... other imaging parameters
}
```

#### 1.3 Hardware Stubs

**Purpose**: Enable API testing without actual hardware (per FR-025).

**Implementation**:

- Create stub implementations of platform traits
- Return mock data for hardware queries
- Log operations for debugging
- Support configurable behavior (success/failure scenarios)

### 2. Configuration System

#### 2.1 Configuration Format

**Format Decision**: TOML format (recommended) or INI format (alternative)

**Rationale for TOML over INI/YAML/JSON**:

- **Rust Ecosystem**: TOML is the standard for Rust projects (Cargo.toml, etc.)
- **Type Safety**: Better support for complex data structures and types
- **Human-Readable**: Clean, unambiguous syntax without indentation sensitivity
- **Comments**: Native support for comments (unlike JSON)
- **Nested Structures**: Better support for hierarchical configuration
- **Tooling**: Excellent Rust crates (`toml` crate) with strong type support
- **No Legacy Constraints**: No deployed devices, so no migration concerns

**Rationale for INI (if chosen)**:

- **Simplicity**: Very simple format for basic key-value configurations
- **Familiarity**: Well-known format, easy to understand
- **Minimal Dependencies**: Lightweight parsing

**Recommendation**: Use TOML format for new Rust implementation

- Better alignment with Rust ecosystem
- More maintainable for complex configurations
- Type-safe parsing with `toml` crate
- Future-proof for configuration evolution

**Note**: If INI format is required for compatibility with existing C codebase or tooling, the `ini` crate provides good support, but TOML is recommended for a greenfield Rust implementation.

**Key Sections** (from existing C implementation):

- `[onvif]` - ONVIF daemon settings
- `[network]` - Network configuration
- `[device]` - Device information
- `[server]` - HTTP server settings
- `[logging]` - Logging configuration
- `[media]` - Media service configuration
- `[ptz]` - PTZ service configuration
- `[imaging]` - Imaging service configuration
- `[stream_profile_1]` through `[stream_profile_4]` - Stream profiles
- `[user_1]` through `[user_8]` - User accounts

#### 2.2 Configuration Schema

**Approach**: Define schema-driven configuration with validation.

**Implementation**:

```rust
// src/config/schema.rs
pub struct ConfigSchema {
    sections: Vec<ConfigSection>,
}

pub struct ConfigSection {
    name: String,
    parameters: Vec<ConfigParameter>,
}

pub struct ConfigParameter {
    key: String,
    value_type: ConfigValueType,
    default: ConfigValue,
    min: Option<i32>,
    max: Option<i32>,
    required: bool,
}

pub enum ConfigValueType {
    Int,
    String,
    Bool,
    Float,
}
```

#### 2.3 Runtime Configuration Manager

**Features**:

- Thread-safe configuration access
- Runtime updates without restart
- Schema validation on load/update
- Type-safe getters/setters
- Generation counters for change detection

**Implementation**:

```rust
// src/config/runtime.rs
pub struct ConfigRuntime {
    config: Arc<RwLock<ApplicationConfig>>,
    schema: ConfigSchema,
    generation: AtomicU64,
}

impl ConfigRuntime {
    pub fn get_int(&self, section: &str, key: &str) -> Result<i32>;
    pub fn set_int(&self, section: &str, key: &str, value: i32) -> Result<()>;
    pub fn get_string(&self, section: &str, key: &str) -> Result<String>;
    pub fn set_string(&self, section: &str, key: &str, value: &str) -> Result<()>;
}
```

#### 2.4 Configuration Storage

**Features**:

- INI file parsing (using `ini` crate)
- Atomic writes (temp file + rename)
- Error handling and validation
- Default value fallback

### 3. Logging System

#### 3.1 Logging Framework

**Approach**: Use Rust `tracing` framework with platform integration.

**Implementation**:

```rust
// src/logging/mod.rs
use tracing::{info, error, warn, debug};

// Platform log integration
pub fn init_logging() -> Result<()> {
    // Initialize tracing subscriber
    // Integrate with platform ak_print() via FFI
    // Set log levels from configuration
}
```

#### 3.2 Platform Log Integration (Unidirectional: C → Rust)

**Purpose**: Forward Anyka SDK `ak_print()` calls to Rust `tracing` logger.

**Rationale**:

- **Rust Logs Stay in Rust**: Rust code uses `tracing` directly, no forwarding to `ak_print()`
- **C Logs Forwarded to Rust**: C code using `ak_print()` (Anyka SDK) forwarded to Rust logger
- **Log Crate for Dependencies**: `tracing-log` bridge captures messages from cargo dependencies using `log` crate
- **Unified Output**: All logs (Rust tracing, C ak_print, cargo log crate) go through Rust logging system

**Implementation - Platform → Rust (ak_print to Rust logger)**:

```rust
// src/logging/platform.rs
use tracing_log::LogTracer;
use log::LevelFilter;

// FFI binding to ak_print() - we intercept this
extern "C" {
    fn ak_print(level: *const c_char, message: *const c_char);
}

// Wrapper function that forwards to Rust logger
#[no_mangle]
pub extern "C" fn ak_print_wrapper(level: *const c_char, message: *const c_char) {
    // Convert C strings to Rust strings
    let level_str = unsafe { CStr::from_ptr(level).to_string_lossy() };
    let msg_str = unsafe { CStr::from_ptr(message).to_string_lossy() };

    // Forward to tracing based on level
    match level_str.as_ref() {
        "ERROR" => tracing::error!("{}", msg_str),
        "WARNING" => tracing::warn!("{}", msg_str),
        "INFO" => tracing::info!("{}", msg_str),
        "DEBUG" => tracing::debug!("{}", msg_str),
        _ => tracing::info!("{}", msg_str),
    }
}

pub fn init_logging() -> Result<()> {
    // Initialize tracing-log bridge to capture log crate messages from dependencies
    LogTracer::builder()
        .with_max_level(LevelFilter::Debug)
        .init()?;

    // Set up tracing subscriber (standard Rust logging, no ak_print forwarding)
    let subscriber = tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer());

    tracing::subscriber::set_global_default(subscriber)?;

    // Note: Anyka SDK code will call ak_print(), which we intercept
    // via LD_PRELOAD or by linking our wrapper instead of original
    Ok(())
}
```

**Features**:

- **Platform → Rust Only**: C code `ak_print()` calls forwarded to Rust `tracing`
- **Log Crate Bridge**: `tracing-log` captures messages from cargo dependencies using `log` crate
- **Rust Code**: Uses `tracing` directly, no forwarding to C
- **Unified Output**: All logs go through Rust `tracing` system
- **Thread Safety**: Safe concurrent logging from Rust and C code

**Dependencies**:

- `tracing-log`: Bridge between `log` crate (used by cargo dependencies) and `tracing` framework
- `tracing-subscriber`: Subscriber implementation for Rust logging

### 4. ONVIF API Implementation

#### 4.1 HTTP/SOAP Server

**Framework**: `axum` for HTTP server, `quick-xml` with `serde` for SOAP serialization.

**Structure**:

```rust
// src/onvif/server.rs
pub struct OnvifServer {
    router: Router,
    config: Arc<ConfigRuntime>,
    platform: Arc<dyn Platform>,
}

impl OnvifServer {
    pub fn new(config: Arc<ConfigRuntime>, platform: Arc<dyn Platform>) -> Self;
    pub async fn start(&self) -> Result<()>;
}
```

**Routes**:

- `POST /onvif/device_service` - Device Service
- `POST /onvif/media_service` - Media Service
- `POST /onvif/ptz_service` - PTZ Service
- `POST /onvif/imaging_service` - Imaging Service

#### 4.2 SOAP Request/Response Handling

**Approach**: Use `quick-xml` with `serde` for ONVIF SOAP XML serialization/deserialization.

**Rationale for quick-xml over yaserde**:

- **Performance**: `quick-xml` is ~50x faster than alternatives, critical for embedded systems
- **Serde Support**: Native `serde` integration via `serialize` feature
- **Active Maintenance**: Well-maintained with active development
- **Flexibility**: Lower-level API allows fine-grained control for ONVIF-specific requirements
- **Size**: Smaller binary footprint important for embedded targets

**Implementation**:

```rust
// src/onvif/soap.rs
use quick_xml::{de::from_str, se::to_string};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "soap:Envelope")]
pub struct SoapEnvelope<T> {
    #[serde(rename = "@xmlns:soap")]
    pub xmlns_soap: String,
    #[serde(rename = "soap:Body")]
    pub body: SoapBody<T>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SoapBody<T> {
    #[serde(flatten)]
    pub content: T,
}
```

#### 4.3 Service Handlers

**Device Service** (`src/onvif/device/mod.rs`):

- `GetDeviceInformation` - Device info
- `GetCapabilities` - Service capabilities
- `GetSystemDateAndTime` - System time
- `GetServices` - Service endpoints
- `SystemReboot` - System reboot
- All other operations from Base Device Test Specification

**Media Service** (`src/onvif/media/mod.rs`):

- `GetProfiles` - Media profiles
- `GetStreamUri` - Stream URIs
- `GetVideoSources` - Video sources
- `GetAudioSources` - Audio sources
- `GetVideoSourceConfigurations` - Video source configs
- `GetVideoEncoderConfigurations` - Encoder configs
- All other operations from Media Device Test Specification

**PTZ Service** (`src/onvif/ptz/mod.rs`):

- `GetConfigurations` - PTZ configurations
- `AbsoluteMove` - Absolute positioning
- `RelativeMove` - Relative movement
- `ContinuousMove` - Continuous movement
- `Stop` - Stop movement
- `GetPresets` - List presets
- `SetPreset` - Save preset
- `GotoPreset` - Recall preset
- All other operations from PTZ Device Test Specification

**Imaging Service** (`src/onvif/imaging/mod.rs`):

- `GetImagingSettings` - Current settings
- `SetImagingSettings` - Update settings
- `GetOptions` - Available options
- All other operations from Imaging Device Test Specification

#### 4.4 Service Dispatcher

**Purpose**: Route SOAP requests to appropriate service handlers.

**Implementation**:

```rust
// src/onvif/dispatcher.rs
pub struct ServiceDispatcher {
    services: HashMap<String, Box<dyn ServiceHandler>>,
}

pub trait ServiceHandler {
    fn handle_operation(&self, operation: &str, request: &[u8]) -> Result<Vec<u8>>;
}
```

### 5. Testing Strategy

#### 5.1 Unit Tests

**Location**: `src/**/*.rs` (inline with `#[cfg(test)]` modules)

**Framework**: Rust's built-in `#[test]` with `tokio-test` for async tests

**Coverage**:

- **Service Handlers**: Mock platform traits, test business logic
- **Configuration System**: Test schema validation, getters/setters, INI parsing
- **SOAP Serialization/Deserialization**: Test XML round-trips, namespace handling
- **Error Handling**: Test error propagation, ONVIF fault generation
- **Platform Abstraction**: Test trait implementations with mock hardware

**Example Structure**:

```rust
// src/onvif/device/mod.rs
#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;
    use crate::platform::traits::MockPlatform;

    #[tokio::test]
    async fn test_get_device_information() {
        let mut mock_platform = MockPlatform::new();
        mock_platform.expect_get_device_info()
            .returning(|| Ok(DeviceInfo { /* ... */ }));

        let service = DeviceService::new(Arc::new(mock_platform));
        let response = service.handle_get_device_information().await.unwrap();

        assert!(response.contains("manufacturer"));
        assert!(response.contains("model"));
    }
}
```

**Mocking Strategy**:

- Use `mockall` crate for platform trait mocking
- Create mock implementations of `VideoInput`, `PTZControl`, etc.
- Test service handlers in isolation from hardware

#### 5.2 Integration Tests

**Location**: `tests/integration/` directory

**Framework**: `tokio-test` for async, full HTTP server testing

**Coverage**:

- **Full ONVIF Request/Response Cycles**: End-to-end SOAP over HTTP
- **Platform Abstraction with Stubs**: Test with hardware stubs (no real hardware access)
- **Configuration Loading/Saving**: Test configuration file round-trips
- **Concurrent Request Handling**: Test thread safety, concurrent clients
- **Error Scenarios**: Test malformed requests, missing parameters

**Important**: Integration tests use **stub platform implementations**, not real hardware. This ensures:

- Tests can run in CI/CD without hardware
- Deterministic test behavior
- Fast test execution
- No hardware dependencies

**Example Structure**:

```rust
// tests/integration/device_service.rs
use onvif_rust::*;
use onvif_rust::platform::stubs::StubPlatform;

#[tokio::test]
async fn test_device_service_integration() {
    // Start test server with stub platform (no real hardware)
    let stub_platform = StubPlatform::new()
        .with_device_info(DeviceInfo {
            manufacturer: "TestManufacturer".to_string(),
            model: "TestModel".to_string(),
            // ... configure stub responses
        });

    let server = start_test_server(stub_platform).await;

    // Send actual HTTP POST with SOAP envelope
    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:8080/onvif/device_service")
        .body(soap_request_body)
        .send()
        .await
        .unwrap();

    // Verify SOAP response
    assert_eq!(response.status(), 200);
    let body = response.text().await.unwrap();
    assert!(body.contains("GetDeviceInformationResponse"));
}
```

**Test Infrastructure**:

- Test server with stub platform (no hardware access)
- SOAP request/response helpers
- ONVIF test specification test case generators
- Stub platform implementations for all hardware interfaces

#### 5.3 ONVIF Test Specification Compliance Tests

**Location**: `tests/onvif/` directory

**Approach**: Implement test cases directly from ONVIF Device Test Specifications

**Structure**:

```rust
// tests/onvif/base_device_test_spec.rs
// Test cases from ONVIF Base Device Test Specification version 24.12

#[tokio::test]
async fn test_get_device_information_basic() {
    // Test Case: GET DEVICE INFORMATION - Basic
    // From Base Device Test Specification section X.X
    // Pre-requisite: Device service is supported
    // Steps:
    //   1. Send GetDeviceInformation request
    //   2. Verify response contains Manufacturer, Model, FirmwareVersion, SerialNumber
    // Expected: Valid GetDeviceInformationResponse
}

#[tokio::test]
async fn test_get_capabilities_all_services() {
    // Test Case: GET CAPABILITIES - All Services
    // Verify all service endpoints are returned
}

#[tokio::test]
async fn test_get_system_date_and_time_utc() {
    // Test Case: GET SYSTEM DATE AND TIME - UTC
    // Verify UTC time format and accuracy
}
```

**Test Organization**:

- One test file per ONVIF test specification document
- Test cases organized by specification section
- Comments reference specific test case IDs from specifications
- Pre-requisites and expected results documented

**Validation**:

- SOAP XML schema validation
- ONVIF namespace URI verification
- Response structure matches specification
- Error responses match ONVIF fault format

## Implementation Phases

### Phase 1: Foundation (Weeks 1-2)

- [ ] Set up Rust project structure
- [ ] Configure cross-compilation (based on rust-hello PoC)
- [ ] Generate Anyka SDK FFI bindings
- [ ] Implement basic platform abstraction traits
- [ ] Create hardware stubs for testing
- [ ] Implement configuration system (INI parser, schema, runtime)
- [ ] Implement logging system with platform integration

### Phase 2: ONVIF Core (Weeks 3-4)

- [ ] Implement HTTP/SOAP server (axum + quick-xml)
- [ ] Implement SOAP request/response handling
- [ ] Implement service dispatcher
- [ ] Implement Device Service (all operations from test spec)
- [ ] Implement basic error handling and fault responses

### Phase 3: Media & PTZ Services (Weeks 5-6)

- [ ] Implement Media Service (all operations from test spec)
- [ ] Implement PTZ Service (all operations from test spec)
- [ ] Integrate with platform abstraction for hardware operations
- [ ] Implement state management for concurrent requests

### Phase 4: Imaging Service & Polish (Weeks 7-8)

- [ ] Implement Imaging Service (all operations from test spec)
- [ ] Implement all remaining operations from test specifications
- [ ] Add comprehensive error handling
- [ ] Optimize performance and memory usage

### Phase 5: Testing & Validation (Weeks 9-10)

- [ ] Implement test cases from ONVIF Device Test Specifications
- [ ] Run ONVIF test tool validation
- [ ] Performance testing and optimization
- [ ] Documentation and deployment preparation

## Technical Decisions

### 1. Rust Edition

**Decision**: Use Rust 2024 edition
**Rationale**:

- **Latest Stable**: Rust 2024 edition was released in version 1.85.0 ([Rust Edition Guide](https://doc.rust-lang.org/edition-guide/rust-2024/index.html))
- **Modern Features**: Includes latest language improvements and best practices
- **Future-Proof**: Ensures long-term compatibility and access to newest Rust features
- **RFC 3501**: Officially specified and stable

### 2. Async Runtime

**Decision**: Use `tokio` for async runtime
**Rationale**: Industry standard, excellent performance, good ecosystem

### 3. HTTP Framework

**Decision**: Use `axum` for HTTP server
**Rationale**: Modern, type-safe, built on tokio, excellent performance

### 4. SOAP/XML Handling

**Decision**: Use `quick-xml` with `serde` for SOAP serialization
**Rationale**:

- **Performance**: ~50x faster than alternatives, critical for embedded systems
- **Serde Support**: Native integration with Rust's serialization framework
- **Active Maintenance**: Well-maintained with active development
- **Flexibility**: Lower-level API allows fine-grained control for ONVIF requirements
- **Size**: Smaller binary footprint important for embedded targets

### 5. Configuration Format

**Decision**: Use TOML format (recommended) or INI (alternative)
**Rationale**:

- **TOML Recommended**: Rust ecosystem standard, better type safety, supports complex structures
- **No Legacy Constraints**: No deployed devices, so no migration concerns
- **Future-Proof**: Better alignment with Rust tooling and best practices
- **INI Alternative**: Available if compatibility with existing C tooling is required

### 6. FFI Approach

**Decision**: Use `bindgen` for automatic binding generation
**Rationale**: Reduces manual work, ensures type safety, easier maintenance

### 8. Logging Integration

**Decision**: Unidirectional logging (C → Rust) with `tracing-log` bridge
**Rationale**:

- **Rust Logs Stay in Rust**: Rust code uses `tracing` directly, no forwarding to C
- **C Logs Forwarded**: C code `ak_print()` calls forwarded to Rust `tracing` logger
- **Log Crate Bridge**: `tracing-log` captures messages from cargo dependencies using `log` crate
- **Unified Output**: All logs (Rust tracing, C ak_print, cargo log) go through Rust logging system
- **No Hardware Dependency**: Rust logging doesn't depend on C platform logging

### 7. Testing Strategy

**Decision**: Three-tier testing approach (unit, integration, ONVIF spec compliance)
**Rationale**:

- **Unit Tests**: Fast, isolated testing with mocks (`mockall`)
- **Integration Tests**: End-to-end testing with stub platform
- **ONVIF Spec Tests**: Direct implementation of test cases from specifications
- **Coverage**: Ensures both code correctness and ONVIF compliance

## Dependencies & Constraints

### Build Dependencies

- Rust toolchain (nightly for `-Z build-std` if needed)
- Anyka cross-compilation toolchain
- `bindgen` for FFI binding generation
- `cc` crate for C compilation

### Runtime Dependencies

- Anyka SDK libraries (via FFI)
- uClibc runtime (target platform)
- System libraries (pthread, etc.)

### Constraints

- ARMv5TEJ architecture (no VFP/NEON)
- Soft-float ABI
- uClibc (not glibc)
- Limited memory resources
- Must pass ONVIF 24.12 test specifications

## Risk Mitigation

### Risk 1: FFI Complexity

**Mitigation**: Start with simple FFI bindings, gradually expand, comprehensive testing

### Risk 2: SOAP/XML Complexity

**Mitigation**: Use proven libraries (quick-xml with serde), extensive test cases from specifications

### Risk 3: Performance

**Mitigation**: Profile early, optimize hot paths, use release optimizations

### Risk 4: Memory Usage

**Mitigation**: Use `#![no_std]` where possible, optimize allocations, use `Arc`/`Rc` wisely

### Risk 5: Test Specification Compliance

**Mitigation**: Implement test cases early, validate against ONVIF test tool continuously

## Success Metrics

- ✅ All ONVIF operations from test specifications implemented
- ✅ 100% of applicable test cases from test specifications pass
- ✅ Binary size comparable to C implementation
- ✅ Performance meets or exceeds C implementation
- ✅ Memory usage within acceptable limits
- ✅ Cross-compilation works reliably
- ✅ Configuration format compatible with existing system

## Next Steps

1. Review and approve this plan
2. Set up project structure and build system
3. Begin Phase 1 implementation
4. Establish CI/CD for cross-compilation
5. Set up test infrastructure for ONVIF test specifications
