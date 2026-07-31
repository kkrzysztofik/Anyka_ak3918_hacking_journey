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
│   │   ├── imaging/          # Imaging Service
│   │   └── snapshot/         # Snapshot Service (GetSnapshotUri)
│   ├── auth/                 # Authentication system
│   │   ├── mod.rs
│   │   ├── ws_security.rs    # WS-Security UsernameToken (SHA1)
│   │   ├── http_digest.rs    # HTTP Digest auth (RFC 2617)
│   │   └── credentials.rs    # Credential validation + auth bypass
│   ├── users/                # User management
│   │   ├── mod.rs
│   │   ├── storage.rs        # User storage/persistence
│   │   └── password.rs       # Argon2 password hashing
│   ├── discovery/            # WS-Discovery protocol
│   │   ├── mod.rs
│   │   └── ws_discovery.rs   # UDP multicast handler
│   ├── security/             # Security hardening
│   │   ├── mod.rs
│   │   ├── rate_limit.rs     # Per-IP rate limiting
│   │   ├── brute_force.rs    # Brute force protection
│   │   ├── xml_security.rs   # XXE/XML bomb detection
│   │   └── audit.rs          # Security event logging
│   ├── platform/             # Platform abstraction
│   │   ├── mod.rs
│   │   ├── anyka/            # Anyka SDK FFI bindings
│   │   ├── traits.rs         # Platform trait definitions
│   │   └── stubs.rs          # Hardware stubs for testing
│   ├── config/               # Configuration system
│   │   ├── mod.rs
│   │   ├── schema.rs         # Configuration schema
│   │   ├── runtime.rs        # Runtime configuration manager
│   │   └── storage.rs        # TOML file storage
│   ├── logging/              # Logging system
│   │   ├── mod.rs
│   │   └── platform.rs       # Platform log integration
│   ├── utils/                # Utility modules
│   │   ├── mod.rs
│   │   ├── validation.rs     # Input validation
│   │   └── memory.rs         # Memory management & profiling
│   └── ffi/                  # C FFI bindings
│       ├── mod.rs
│       ├── anyka_sdk.h       # Anyka SDK header bindings
│       └── anyka_sdk.rs      # Generated bindings
├── tests/                    # Integration tests
│   ├── onvif/                # ONVIF test specification tests
│   ├── platform/             # Platform abstraction tests
│   └── integration/          # Full integration tests
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
tokio = { version = "1", features = ["rt-multi-thread", "macros", "signal", "net", "sync"] }

# HTTP server & middleware
axum = "0.7"
tower = { version = "0.4", features = ["timeout", "limit"] }
tower-http = { version = "0.5", features = ["trace", "timeout", "limit"] }

# SOAP/XML handling - using quick-xml for performance and serde support
quick-xml = { version = "0.31", features = ["serialize"] }

# Configuration - TOML format (Rust standard)
toml = "0.8"
serde = { version = "1.0", features = ["derive"] }

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
tracing-log = "0.2"  # Bridge between log crate and tracing

# Authentication & Security
argon2 = "0.5"              # Password hashing (OWASP recommended)
sha1 = "0.10"               # WS-Security UsernameToken (ONVIF mandates SHA1)
md-5 = "0.10"               # HTTP Digest auth (RFC 2617)
hmac = "0.12"               # HMAC for HTTP Digest
base64 = "0.22"             # Auth header encoding
rand = "0.8"                # Nonce generation
constant_time_eq = "0.3"    # Timing-safe comparison

# Networking (WS-Discovery)
socket2 = "0.5"             # UDP multicast socket configuration

# Time & Identity
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1.0", features = ["v4"] }

# Concurrency
parking_lot = "0.12"        # Faster mutexes for embedded
dashmap = "5.5"             # Concurrent hashmap for rate limiting

# Utility
bytes = "1.5"               # Efficient byte handling

# FFI
libc = "0.2"

# Error handling
anyhow = "1.0"
thiserror = "1.0"

[dev-dependencies]
tokio-test = "0.4"
mockall = "0.12"
reqwest = { version = "0.11", features = ["json"] }  # Integration test HTTP client
wiremock = "0.5"            # HTTP mocking for tests
criterion = "0.5"           # Benchmarking

[build-dependencies]
cc = "1.0"
bindgen = "0.69"

[features]
default = []
memory-profiling = []       # Enable memory tracking (development only)
verbose-logging = []        # Enable HTTP request/response logging

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

## Application Lifecycle Architecture

### Design Principles

The lifecycle architecture follows these rules:

- **Explicit `start()`**: Ordered, async, fallible initialization
- **Explicit `shutdown()`**: Coordinated, async, graceful shutdown
- **No global state**: All state owned by `Application` struct
- **No `Drop` for async**: `Drop` only deallocates memory; async cleanup via `shutdown()`
- **Dependency injection**: Components receive dependencies, not global lookups
- **Optional components**: Handled at startup with degraded mode support

### Architecture Overview

```text
Application
├── start() → ordered async initialization
├── run() → main event loop with signal handling
├── shutdown() → coordinated async cleanup
└── Drop → memory deallocation only (no async, no side effects)

Components (owned by Application):
├── Config (non-optional, loaded first)
├── Platform (non-optional, hardware abstraction)
├── ServiceManager
│   ├── DeviceService (required)
│   ├── MediaService (required)
│   ├── PtzService (optional - degraded mode if unavailable)
│   └── ImagingService (optional - degraded mode if unavailable)
├── NetworkManager
│   ├── HttpServer (required)
│   ├── WsDiscovery (optional)
│   └── SnapshotHandler (optional)
└── ShutdownCoordinator (broadcast channel for graceful shutdown)
```

### Application Struct

```rust
// src/app.rs
pub struct Application {
    config: ConfigRuntime,
    platform: Arc<dyn Platform>,
    services: ServiceManager,
    network: NetworkManager,
    shutdown_tx: broadcast::Sender<()>,
    started_at: Instant,
}

impl Application {
    /// Ordered async initialization - the ONLY way to create Application
    pub async fn start(config_path: &str) -> Result<Self, StartupError>;

    /// Run until shutdown signal received
    pub async fn run(&self) -> Result<(), RuntimeError>;

    /// Coordinated async shutdown - MUST be called before drop
    pub async fn shutdown(self) -> ShutdownReport;

    /// Health check for observability
    pub fn health(&self) -> HealthStatus;
}
```

### Startup Sequence

Initialization order (matches C implementation):

1. Load and validate configuration
2. Initialize platform abstraction (with hardware stubs for testing)
3. Initialize required services (Device, Media)
4. Initialize optional services (PTZ, Imaging) - log warning on failure, continue
5. Initialize network (HTTP server - required; WS-Discovery - optional)
6. Send WS-Discovery Hello message

```rust
// src/lifecycle/startup.rs
pub async fn startup_sequence(config_path: &str) -> Result<Application, StartupError> {
    tracing::info!("Starting ONVIF application...");

    // Phase 1: Configuration
    tracing::info!("Phase 1: Loading configuration...");
    let config = ConfigRuntime::load(config_path)
        .map_err(StartupError::Config)?;

    // Phase 2: Platform
    tracing::info!("Phase 2: Initializing platform...");
    let platform = create_platform(&config)
        .await
        .map_err(StartupError::Platform)?;

    // Phase 3: Services (required + optional)
    tracing::info!("Phase 3: Initializing services...");
    let (shutdown_tx, _) = broadcast::channel(1);
    let services = ServiceManager::new(&config, platform.clone(), shutdown_tx.subscribe())
        .await
        .map_err(StartupError::Services)?;

    // Phase 4: Network
    tracing::info!("Phase 4: Initializing network...");
    let network = NetworkManager::new(&config, services.clone(), shutdown_tx.subscribe())
        .await
        .map_err(StartupError::Network)?;

    // Phase 5: Discovery Hello
    if let Some(discovery) = &network.discovery {
        tracing::info!("Phase 5: Sending WS-Discovery Hello...");
        discovery.send_hello().await.ok(); // Non-fatal
    }

    tracing::info!("Application started successfully");
    Ok(Application {
        config,
        platform,
        services,
        network,
        shutdown_tx,
        started_at: Instant::now(),
    })
}
```

### Shutdown Coordination

Shutdown sequence:

1. Send WS-Discovery Bye message
2. Stop accepting new HTTP connections
3. Broadcast shutdown signal to all tasks
4. Wait for in-flight requests (with timeout)
5. Shutdown services in reverse order
6. Shutdown platform
7. Return shutdown report (success/timeout/errors)

```rust
// src/lifecycle/shutdown.rs
pub struct ShutdownCoordinator {
    shutdown_tx: broadcast::Sender<()>,
    timeout: Duration,
}

impl ShutdownCoordinator {
    pub fn new(shutdown_tx: broadcast::Sender<()>, timeout: Duration) -> Self {
        Self { shutdown_tx, timeout }
    }

    pub async fn initiate_shutdown(&self) -> ShutdownReport {
        let start = Instant::now();
        let mut report = ShutdownReport::new();

        tracing::info!("Initiating graceful shutdown...");

        // Broadcast shutdown signal to all listeners
        let _ = self.shutdown_tx.send(());

        // Wait for tasks with timeout
        match tokio::time::timeout(self.timeout, self.wait_for_tasks()).await {
            Ok(_) => {
                report.status = ShutdownStatus::Success;
                tracing::info!("All tasks completed gracefully");
            }
            Err(_) => {
                report.status = ShutdownStatus::Timeout;
                tracing::warn!("Shutdown timeout - some tasks may not have completed");
            }
        }

        report.duration = start.elapsed();
        report
    }
}
```

### Optional Component Handling

```rust
// src/onvif/services/manager.rs
pub struct ServiceManager {
    device: DeviceService,           // Required - startup fails if unavailable
    media: MediaService,             // Required - startup fails if unavailable
    ptz: Option<PtzService>,         // Optional - None if init failed
    imaging: Option<ImagingService>, // Optional - None if init failed
    degraded_services: Vec<String>,  // Track what's unavailable
}

impl ServiceManager {
    pub async fn new(
        config: &ConfigRuntime,
        platform: Arc<dyn Platform>,
        shutdown_rx: broadcast::Receiver<()>,
    ) -> Result<Self, ServiceInitError> {
        // Required services - fail if unavailable
        let device = DeviceService::new(config, platform.clone()).await?;
        let media = MediaService::new(config, platform.clone()).await?;

        // Optional services - log warning and continue
        let mut degraded_services = Vec::new();

        let ptz = match PtzService::new(config, platform.clone()).await {
            Ok(ptz) => Some(ptz),
            Err(e) => {
                tracing::warn!("PTZ service unavailable: {}", e);
                degraded_services.push("PTZ".to_string());
                None
            }
        };

        let imaging = match ImagingService::new(config, platform.clone()).await {
            Ok(imaging) => Some(imaging),
            Err(e) => {
                tracing::warn!("Imaging service unavailable: {}", e);
                degraded_services.push("Imaging".to_string());
                None
            }
        };

        Ok(Self {
            device,
            media,
            ptz,
            imaging,
            degraded_services,
        })
    }

    pub fn is_degraded(&self) -> bool {
        !self.degraded_services.is_empty()
    }
}
```

### Health Status

```rust
// src/lifecycle/health.rs
#[derive(Debug, Clone)]
pub enum HealthState {
    Healthy,
    Degraded,
    Unhealthy,
}

#[derive(Debug, Clone)]
pub struct ComponentHealth {
    pub name: String,
    pub status: HealthState,
    pub message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct HealthStatus {
    pub status: HealthState,
    pub uptime: Duration,
    pub components: HashMap<String, ComponentHealth>,
    pub degraded_services: Vec<String>,
}

impl Application {
    pub fn health(&self) -> HealthStatus {
        let mut components = HashMap::new();

        // Check each component
        components.insert("config".to_string(), ComponentHealth {
            name: "Configuration".to_string(),
            status: HealthState::Healthy,
            message: None,
        });

        components.insert("platform".to_string(), ComponentHealth {
            name: "Platform".to_string(),
            status: HealthState::Healthy,
            message: None,
        });

        // Determine overall status
        let status = if self.services.is_degraded() {
            HealthState::Degraded
        } else {
            HealthState::Healthy
        };

        HealthStatus {
            status,
            uptime: self.started_at.elapsed(),
            components,
            degraded_services: self.services.degraded_services.clone(),
        }
    }
}
```

### Main Entry Point

```rust
// src/main.rs
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging first
    init_logging()?;

    // Single entry point - no global state
    let app = Application::start("/etc/onvif/config.toml").await?;

    // Run until shutdown signal
    if let Err(e) = app.run().await {
        tracing::error!("Runtime error: {}", e);
    }

    // Explicit graceful shutdown
    let report = app.shutdown().await;
    tracing::info!("Shutdown complete: {:?}", report);

    Ok(())
}
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

**Format Decision**: TOML format

**Rationale for TOML over YAML/JSON**:

- **Rust Ecosystem**: TOML is the standard for Rust projects (Cargo.toml, etc.)
- **Type Safety**: Better support for complex data structures and types
- **Human-Readable**: Clean, unambiguous syntax without indentation sensitivity
- **Comments**: Native support for comments (unlike JSON)
- **Nested Structures**: Better support for hierarchical configuration
- **Tooling**: Excellent Rust crates (`toml` crate) with strong type support
- **No Legacy Constraints**: No deployed devices, so no migration concerns

**Recommendation**: Use TOML format for new Rust implementation

- Better alignment with Rust ecosystem
- More maintainable for complex configurations
- Type-safe parsing with `toml` crate
- Future-proof for configuration evolution

**Key Sections**:

- `[onvif]` - ONVIF daemon settings
- `[network]` - Network configuration
- `[device]` - Device information
- `[server]` - HTTP server settings
- `[logging]` - Logging configuration
- `[media]` - Media service configuration
- `[ptz]` - PTZ service configuration
- `[imaging]` - Imaging service configuration
- `[stream_profile_1]` through `[stream_profile_4]` - Stream profiles
- `[[users]]` - User accounts array (up to 8 users)

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

- TOML file parsing (using `toml` crate)
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

#### 3.3 HTTP Request/Response Logging (NFR-011, NFR-012)

**Purpose**: Provide configurable HTTP request/response logging for debugging ONVIF traffic.

**Approach**:

- Implement an `axum` middleware (or tower layer) that logs HTTP method, path, response status code, and request latency for each request.
- When verbose HTTP logging is enabled, optionally log truncated, sanitized SOAP payloads (no full credentials or secrets).
- Read configuration from `ConfigRuntime` (for example a `[logging] http_verbose = true/false` flag) to toggle verbose HTTP logging at runtime.

**Implementation**:

- Add an HTTP logging layer in `src/onvif/server.rs` (or `src/logging/http.rs`) that:
  - Wraps the router with timing and metadata logging.
  - Uses `tracing` to emit structured events (method, path, status, latency, client IP).
  - Redacts or truncates sensitive headers and SOAP bodies by default.
- Ensure this behavior directly satisfies NFR-011 and NFR-012 from `spec.md`.

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
- `GetSnapshotUri` - Snapshot URIs (per FR-053 to FR-055)
- `GetVideoSources` - Video sources
- `GetAudioSources` - Audio sources
- `GetVideoSourceConfigurations` - Video source configs
- `GetVideoEncoderConfigurations` - Encoder configs
- All other operations from Media Device Test Specification

**Snapshot Service** (`src/onvif/snapshot/mod.rs`):

- `GetSnapshotUri` - Returns HTTP URI for JPEG snapshot
- Configurable resolution (width, height)
- Configurable JPEG quality (1-100)

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

#### 4.5 Input Validation

**Purpose**: Validate all incoming requests before processing (per FR-060 to FR-062).

**Implementation**:

```rust
// src/utils/validation.rs
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ValidationError {
    #[error("Invalid HTTP method: expected POST")]
    InvalidHttpMethod,
    #[error("Invalid content type: expected text/xml or application/soap+xml")]
    InvalidContentType,
    #[error("Invalid SOAP envelope structure")]
    InvalidSoapEnvelope,
    #[error("Missing SOAP action header")]
    MissingSoapAction,
    #[error("Invalid characters in input: {0}")]
    InvalidCharacters(String),
    #[error("Input exceeds maximum length: {0}")]
    InputTooLong(usize),
}

pub struct InputValidator {
    max_string_length: usize,
}

impl InputValidator {
    /// Validate HTTP request components
    pub fn validate_http_request(
        &self,
        method: &str,
        content_type: Option<&str>,
    ) -> Result<(), ValidationError> {
        if method != "POST" {
            return Err(ValidationError::InvalidHttpMethod);
        }

        match content_type {
            Some(ct) if ct.contains("text/xml") || ct.contains("application/soap+xml") => Ok(()),
            _ => Err(ValidationError::InvalidContentType),
        }
    }

    /// Validate SOAP envelope structure
    pub fn validate_soap_envelope(&self, xml: &str) -> Result<(), ValidationError> {
        if !xml.contains("Envelope") || !xml.contains("Body") {
            return Err(ValidationError::InvalidSoapEnvelope);
        }
        Ok(())
    }

    /// Sanitize user-provided strings
    pub fn sanitize_string(&self, input: &str) -> Result<String, ValidationError> {
        if input.len() > self.max_string_length {
            return Err(ValidationError::InputTooLong(self.max_string_length));
        }

        // Remove control characters except whitespace
        let sanitized: String = input
            .chars()
            .filter(|c| !c.is_control() || c.is_whitespace())
            .collect();

        // Check for potentially dangerous patterns
        if sanitized.contains("<!ENTITY") || sanitized.contains("javascript:") {
            return Err(ValidationError::InvalidCharacters(
                "Potentially dangerous content detected".to_string()
            ));
        }

        Ok(sanitized)
    }
}
```

#### 4.6 Connection Management

**Purpose**: Configure server resources and connection handling (per NFR-009, NFR-010).

**Configuration**:

```toml
# config.toml
[server]
# Authentication
auth_enabled = true

# Worker threads (1-32)
worker_threads = 4

# Connection limits
max_connections = 100
connection_timeout_secs = 30
request_timeout_secs = 60

# Request limits
max_payload_size = 1048576  # 1MB
max_header_size = 8192      # 8KB
```

**Implementation**:

```rust
// src/onvif/server.rs (extended)
use tower::ServiceBuilder;
use tower_http::timeout::TimeoutLayer;

pub struct ServerConfig {
    pub worker_threads: usize,
    pub max_connections: usize,
    pub connection_timeout: Duration,
    pub request_timeout: Duration,
    pub max_payload_size: usize,
}

impl ServerConfig {
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.worker_threads < 1 || self.worker_threads > 32 {
            return Err(ConfigError::InvalidValue("worker_threads must be 1-32"));
        }
        Ok(())
    }
}

impl OnvifServer {
    pub async fn start_with_config(&self, config: ServerConfig) -> Result<()> {
        let app = self.router.clone()
            .layer(
                ServiceBuilder::new()
                    .layer(TimeoutLayer::new(config.request_timeout))
                    .layer(axum::extract::DefaultBodyLimit::max(config.max_payload_size))
            );

        let listener = tokio::net::TcpListener::bind("0.0.0.0:80").await?;
        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal())
            .await?;

        Ok(())
    }
}
```

### 5. Authentication System

#### 5.1 WS-Security UsernameToken

**Purpose**: Implement ONVIF-mandated WS-Security authentication per ONVIF Core Specification.

**Implementation**:

```rust
// src/auth/ws_security.rs
use sha1::{Sha1, Digest};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use chrono::{DateTime, Utc};

pub struct WsSecurityValidator {
    max_timestamp_age_seconds: i64,
}

impl WsSecurityValidator {
    /// Validate UsernameToken from SOAP header
    /// Digest = Base64(SHA1(Nonce + Created + Password))
    pub fn validate_digest(
        &self,
        username: &str,
        nonce_b64: &str,
        created: &str,
        digest_b64: &str,
        password: &str,
    ) -> Result<bool, AuthError> {
        // Decode nonce from base64
        let nonce = BASE64.decode(nonce_b64)?;

        // Compute expected digest: SHA1(Nonce + Created + Password)
        let mut hasher = Sha1::new();
        hasher.update(&nonce);
        hasher.update(created.as_bytes());
        hasher.update(password.as_bytes());
        let expected = BASE64.encode(hasher.finalize());

        // Timing-safe comparison
        Ok(constant_time_eq::constant_time_eq(
            expected.as_bytes(),
            digest_b64.as_bytes()
        ))
    }

    /// Validate timestamp is not stale (prevents replay attacks)
    pub fn validate_timestamp(&self, created: &str) -> Result<bool, AuthError> {
        let created_time: DateTime<Utc> = created.parse()?;
        let age = Utc::now().signed_duration_since(created_time);
        Ok(age.num_seconds().abs() <= self.max_timestamp_age_seconds)
    }
}
```

#### 5.2 HTTP Digest Authentication

**Purpose**: Implement RFC 2617 HTTP Digest authentication as fallback.

**Implementation**:

```rust
// src/auth/http_digest.rs
use md5::{Md5, Digest};
use rand::Rng;

pub struct HttpDigestAuth {
    realm: String,
    nonce_validity_seconds: u64,
}

impl HttpDigestAuth {
    /// Generate WWW-Authenticate challenge header
    pub fn generate_challenge(&self) -> String {
        let nonce = self.generate_nonce();
        format!(
            r#"Digest realm="{}", nonce="{}", qop="auth", algorithm=MD5"#,
            self.realm, nonce
        )
    }

    /// Validate client response
    /// HA1 = MD5(username:realm:password)
    /// HA2 = MD5(method:uri)
    /// response = MD5(HA1:nonce:nc:cnonce:qop:HA2)
    pub fn validate_response(
        &self,
        username: &str,
        password: &str,
        realm: &str,
        nonce: &str,
        uri: &str,
        nc: &str,
        cnonce: &str,
        response: &str,
        method: &str,
    ) -> bool {
        let ha1 = md5_hex(&format!("{}:{}:{}", username, realm, password));
        let ha2 = md5_hex(&format!("{}:{}", method, uri));
        let expected = md5_hex(&format!("{}:{}:{}:{}:auth:{}", ha1, nonce, nc, cnonce, ha2));

        constant_time_eq::constant_time_eq(expected.as_bytes(), response.as_bytes())
    }

    fn generate_nonce(&self) -> String {
        let random_bytes: [u8; 16] = rand::thread_rng().gen();
        base64::engine::general_purpose::STANDARD.encode(random_bytes)
    }
}

fn md5_hex(input: &str) -> String {
    let mut hasher = Md5::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}
```

#### 5.3 Authentication Bypass Configuration

**Purpose**: Support development/testing without authentication (per FR-048a).

**Configuration**:

```toml
# config.toml
[server]
auth_enabled = true  # Set to false to disable all authentication
```

**Implementation**:

```rust
// src/auth/credentials.rs
pub struct AuthConfig {
    pub enabled: bool,
}

impl AuthConfig {
    /// Check if request should bypass authentication
    pub fn should_authenticate(&self) -> bool {
        self.enabled
    }
}
```

### 6. User Management System

#### 6.1 User Storage

**Purpose**: Store and manage up to 8 user accounts with secure password storage.

**Implementation**:

```rust
// src/users/storage.rs
use parking_lot::RwLock;
use std::collections::HashMap;

pub const MAX_USERS: usize = 8;

#[derive(Debug, Clone, PartialEq)]
pub enum UserLevel {
    Administrator,
    Operator,
    User,
}

#[derive(Debug, Clone)]
pub struct UserAccount {
    pub username: String,
    pub password_hash: String,  // Argon2id hash
    pub level: UserLevel,
}

pub struct UserStorage {
    users: RwLock<HashMap<String, UserAccount>>,
}

impl UserStorage {
    pub fn get_user(&self, username: &str) -> Option<UserAccount> {
        self.users.read().get(username).cloned()
    }

    pub fn create_user(&self, account: UserAccount) -> Result<(), UserError> {
        let mut users = self.users.write();
        if users.len() >= MAX_USERS {
            return Err(UserError::MaxUsersReached);
        }
        if users.contains_key(&account.username) {
            return Err(UserError::UserExists);
        }
        users.insert(account.username.clone(), account);
        Ok(())
    }

    pub fn delete_user(&self, username: &str) -> Result<(), UserError> {
        let mut users = self.users.write();
        users.remove(username).ok_or(UserError::UserNotFound)?;
        Ok(())
    }

    pub fn list_users(&self) -> Vec<UserAccount> {
        self.users.read().values().cloned().collect()
    }
}
```

#### 6.2 Password Security

**Purpose**: Secure password hashing using Argon2id (OWASP recommended).

**Implementation**:

```rust
// src/users/password.rs
use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rand::rngs::OsRng;

pub struct PasswordManager;

impl PasswordManager {
    /// Hash password using Argon2id
    pub fn hash_password(password: &str) -> Result<String, PasswordError> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let hash = argon2
            .hash_password(password.as_bytes(), &salt)?
            .to_string();
        Ok(hash)
    }

    /// Verify password against stored hash (timing-safe)
    pub fn verify_password(password: &str, hash: &str) -> Result<bool, PasswordError> {
        let parsed_hash = PasswordHash::new(hash)?;
        Ok(Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok())
    }
}
```

#### 6.3 User Persistence

**Purpose**: Persist user accounts to TOML configuration file.

**Configuration Format**:

```toml
# users.toml
[[users]]
username = "admin"
password_hash = "$argon2id$v=19$m=19456,t=2,p=1$..."
level = "Administrator"

[[users]]
username = "operator"
password_hash = "$argon2id$v=19$m=19456,t=2,p=1$..."
level = "Operator"
```

### 7. WS-Discovery Protocol

#### 7.1 UDP Multicast Handler

**Purpose**: Implement WS-Discovery for ONVIF device discovery (per FR-049 to FR-052).

**Implementation**:

```rust
// src/discovery/ws_discovery.rs
use socket2::{Domain, Protocol, Socket, Type};
use std::net::{Ipv4Addr, SocketAddrV4};
use tokio::net::UdpSocket;

const WS_DISCOVERY_MULTICAST: Ipv4Addr = Ipv4Addr::new(239, 255, 255, 250);
const WS_DISCOVERY_PORT: u16 = 3702;

pub struct WsDiscovery {
    socket: UdpSocket,
    device_uuid: String,
    scopes: Vec<String>,
    xaddr: String,
    discoverable: bool,
}

impl WsDiscovery {
    pub async fn new(device_uuid: String, xaddr: String) -> Result<Self, DiscoveryError> {
        // Create UDP socket with multicast support
        let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
        socket.set_reuse_address(true)?;
        socket.bind(&SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, WS_DISCOVERY_PORT).into())?;
        socket.join_multicast_v4(&WS_DISCOVERY_MULTICAST, &Ipv4Addr::UNSPECIFIED)?;

        let socket = UdpSocket::from_std(socket.into())?;

        Ok(Self {
            socket,
            device_uuid,
            scopes: vec![
                "onvif://www.onvif.org/type/video_encoder".to_string(),
                "onvif://www.onvif.org/type/ptz".to_string(),
            ],
            xaddr,
            discoverable: true,
        })
    }

    /// Send Hello message on startup
    pub async fn send_hello(&self) -> Result<(), DiscoveryError> {
        let hello = self.build_hello_message();
        self.socket.send_to(
            hello.as_bytes(),
            (WS_DISCOVERY_MULTICAST, WS_DISCOVERY_PORT),
        ).await?;
        Ok(())
    }

    /// Send Bye message on shutdown
    pub async fn send_bye(&self) -> Result<(), DiscoveryError> {
        let bye = self.build_bye_message();
        self.socket.send_to(
            bye.as_bytes(),
            (WS_DISCOVERY_MULTICAST, WS_DISCOVERY_PORT),
        ).await?;
        Ok(())
    }

    /// Listen for Probe messages and respond
    pub async fn run(&self) -> Result<(), DiscoveryError> {
        let mut buf = [0u8; 4096];
        loop {
            let (len, addr) = self.socket.recv_from(&mut buf).await?;
            if !self.discoverable {
                continue;  // Silently ignore when NonDiscoverable (EC-014)
            }

            let message = std::str::from_utf8(&buf[..len])?;
            if self.is_probe_message(message) {
                let response = self.build_probe_match();
                self.socket.send_to(response.as_bytes(), addr).await?;
            }
        }
    }

    fn build_hello_message(&self) -> String {
        // WS-Discovery Hello SOAP envelope
        format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope"
               xmlns:wsa="http://schemas.xmlsoap.org/ws/2004/08/addressing"
               xmlns:wsd="http://schemas.xmlsoap.org/ws/2005/04/discovery">
  <soap:Header>
    <wsa:Action>http://schemas.xmlsoap.org/ws/2005/04/discovery/Hello</wsa:Action>
    <wsa:MessageID>urn:uuid:{}</wsa:MessageID>
  </soap:Header>
  <soap:Body>
    <wsd:Hello>
      <wsa:EndpointReference>
        <wsa:Address>urn:uuid:{}</wsa:Address>
      </wsa:EndpointReference>
      <wsd:Types>dn:NetworkVideoTransmitter</wsd:Types>
      <wsd:Scopes>{}</wsd:Scopes>
      <wsd:XAddrs>{}</wsd:XAddrs>
    </wsd:Hello>
  </soap:Body>
</soap:Envelope>"#,
            uuid::Uuid::new_v4(),
            self.device_uuid,
            self.scopes.join(" "),
            self.xaddr
        )
    }
}
```

### 8. Security Hardening

#### 8.1 Rate Limiting

**Purpose**: Protect against DoS attacks with per-IP request limiting (per NFR-005).

**Implementation**:

```rust
// src/security/rate_limit.rs
use dashmap::DashMap;
use std::time::{Duration, Instant};

pub struct RateLimiter {
    requests: DashMap<String, RequestCount>,
    max_requests_per_minute: u32,
    window_duration: Duration,
}

struct RequestCount {
    count: u32,
    window_start: Instant,
}

impl RateLimiter {
    pub fn new(max_requests_per_minute: u32) -> Self {
        Self {
            requests: DashMap::new(),
            max_requests_per_minute,
            window_duration: Duration::from_secs(60),
        }
    }

    /// Check if request should be allowed
    pub fn check_rate_limit(&self, client_ip: &str) -> bool {
        let mut entry = self.requests.entry(client_ip.to_string()).or_insert(RequestCount {
            count: 0,
            window_start: Instant::now(),
        });

        // Reset window if expired
        if entry.window_start.elapsed() > self.window_duration {
            entry.count = 0;
            entry.window_start = Instant::now();
        }

        entry.count += 1;
        entry.count <= self.max_requests_per_minute
    }
}
```

#### 8.2 Brute Force Protection

**Purpose**: Block IPs after repeated authentication failures (per FR-076).

**Implementation**:

```rust
// src/security/brute_force.rs
use dashmap::DashMap;
use std::time::{Duration, Instant};

pub struct BruteForceProtection {
    failures: DashMap<String, FailureRecord>,
    max_failures: u32,           // Default: 5
    failure_window: Duration,    // Default: 60 seconds
    block_duration: Duration,    // Default: 300 seconds
}

struct FailureRecord {
    count: u32,
    first_failure: Instant,
    blocked_until: Option<Instant>,
}

impl BruteForceProtection {
    /// Record authentication failure
    pub fn record_failure(&self, client_ip: &str) {
        let mut entry = self.failures.entry(client_ip.to_string()).or_insert(FailureRecord {
            count: 0,
            first_failure: Instant::now(),
            blocked_until: None,
        });

        // Reset if outside failure window
        if entry.first_failure.elapsed() > self.failure_window {
            entry.count = 0;
            entry.first_failure = Instant::now();
        }

        entry.count += 1;

        // Block if threshold exceeded
        if entry.count >= self.max_failures {
            entry.blocked_until = Some(Instant::now() + self.block_duration);
            tracing::warn!("Blocked IP {} for {} seconds due to brute force",
                client_ip, self.block_duration.as_secs());
        }
    }

    /// Check if IP is currently blocked
    pub fn is_blocked(&self, client_ip: &str) -> bool {
        if let Some(entry) = self.failures.get(client_ip) {
            if let Some(blocked_until) = entry.blocked_until {
                return Instant::now() < blocked_until;
            }
        }
        false
    }
}
```

#### 8.3 XML Security

**Purpose**: Prevent XXE and XML bomb attacks (per NFR-007, EC-017).

**Implementation**:

```rust
// src/security/xml_security.rs

pub struct XmlSecurityValidator {
    max_entity_expansions: usize,
    max_payload_size: usize,  // Default: 1MB
}

impl XmlSecurityValidator {
    /// Validate XML payload before parsing
    pub fn validate(&self, xml: &str) -> Result<(), XmlSecurityError> {
        // Check payload size
        if xml.len() > self.max_payload_size {
            return Err(XmlSecurityError::PayloadTooLarge);
        }

        // Detect XXE attempts (external entity references)
        if xml.contains("<!ENTITY") || xml.contains("SYSTEM") || xml.contains("PUBLIC") {
            tracing::warn!("XXE attack detected");
            return Err(XmlSecurityError::XxeDetected);
        }

        // Detect XML bomb patterns (entity expansion)
        let entity_count = xml.matches("<!ENTITY").count();
        if entity_count > self.max_entity_expansions {
            tracing::warn!("XML bomb detected: {} entities", entity_count);
            return Err(XmlSecurityError::XmlBombDetected);
        }

        Ok(())
    }
}
```

#### 8.4 Security Event Logging

**Purpose**: Audit trail for security events (per FR-082).

**Implementation**:

```rust
// src/security/audit.rs
use tracing::{warn, info};

pub fn log_auth_failure(client_ip: &str, username: &str, reason: &str) {
    warn!(
        target: "security",
        event = "auth_failure",
        client_ip = %client_ip,
        username = %username,
        reason = %reason,
        "Authentication failure"
    );
}

pub fn log_ip_blocked(client_ip: &str, duration_secs: u64) {
    warn!(
        target: "security",
        event = "ip_blocked",
        client_ip = %client_ip,
        duration_secs = %duration_secs,
        "IP address blocked"
    );
}

pub fn log_attack_detected(client_ip: &str, attack_type: &str) {
    warn!(
        target: "security",
        event = "attack_detected",
        client_ip = %client_ip,
        attack_type = %attack_type,
        "Security attack detected"
    );
}
```

#### 8.5 HTTP/SOAP Input Hardening (NFR-006, EC-017)

**Purpose**: Provide centralized validation for URLs/paths and user-controllable strings to satisfy NFR-006 and support EC-017.

**Implementation**:

- Extend `src/utils/validation.rs` with helpers that:
  - Canonicalize and validate HTTP request paths, reject `..` segments and other traversal patterns, and enforce confinement to a configured root.
  - Apply additional string validation rules in `InputValidator::sanitize_string()` to detect and reject common XSS-style payloads in SOAP headers and bodies.
- Integrate these helpers into the HTTP/SOAP server pipeline in `src/onvif/server.rs` so every incoming request passes through:
  - Basic HTTP method/content-type validation.
  - Path traversal checks.
  - String sanitization for user-controllable fields.
- Clearly document that these behaviors implement NFR-006 and complement the XML-focused protections in `XmlSecurityValidator` for EC-017.

### 9. CI/CD Pipeline

#### 9.1 Pull Request Workflow

**Purpose**: Validate code quality on every push and pull request (per FR-031 to FR-037).

**GitHub Actions Workflow** (`.github/workflows/ci.yml`):

```yaml
name: CI

on:
  push:
    branches: [001-rust-onvif-api]
  pull_request:
    branches: [001-rust-onvif-api]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6

      - name: Install Rust
        uses: dtolnay/rust-action@stable

      - name: Cache cargo
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}

      - name: Run tests
        run: cargo test --all-features

      - name: Clippy
        run: cargo clippy -- -D warnings

      - name: Format check
        run: cargo fmt --check

  coverage:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6

      - name: Install Rust
        uses: dtolnay/rust-action@stable

      - name: Install cargo-tarpaulin
        run: cargo install cargo-tarpaulin

      - name: Generate coverage
        run: |
          cargo tarpaulin --out Html --out Lcov --out Xml \
            --output-dir coverage \
            --fail-under 80

      - name: Upload HTML coverage (human readable)
        uses: actions/upload-artifact@v4
        with:
          name: coverage-html
          path: coverage/tarpaulin-report.html

      - name: Upload LCOV coverage (machine readable)
        uses: actions/upload-artifact@v4
        with:
          name: coverage-lcov
          path: coverage/lcov.info

      - name: Upload Cobertura coverage (GitHub PR annotations)
        uses: actions/upload-artifact@v4
        with:
          name: coverage-cobertura
          path: coverage/cobertura.xml
```

#### 9.2 Release Workflow

**Purpose**: Cross-compile and release on git tags (per FR-038 to FR-041).

**GitHub Actions Workflow** (`.github/workflows/release.yml`):

```yaml
name: Release

on:
  push:
    tags: ['v*']

jobs:
  build:
    runs-on: ubuntu-latest
    container:
      image: ghcr.io/your-org/anyka-rust-toolchain:latest

    steps:
      - uses: actions/checkout@v6

      - name: Cross-compile for ARM
        run: |
          cargo build --release --target armv5te-unknown-linux-uclibceabi

      - name: Create SD_card_content structure
        run: |
          mkdir -p SD_card_content/anyka_hack/bin
          cp target/armv5te-unknown-linux-uclibceabi/release/onvif-rust \
             SD_card_content/anyka_hack/bin/

      - name: Create ZIP archive
        run: zip -r onvif-rust-${{ github.ref_name }}.zip SD_card_content/

      - name: Upload Release
        uses: softprops/action-gh-release@v1
        with:
          files: onvif-rust-${{ github.ref_name }}.zip
```

### 10. Memory Management

#### 10.1 Memory Constraints

**Purpose**: Enforce memory limits for embedded target (per NFR-001 to NFR-004).

**Configuration**:

```rust
// src/utils/memory.rs
pub const MEMORY_SOFT_LIMIT: usize = 16 * 1024 * 1024;  // 16MB
pub const MEMORY_HARD_LIMIT: usize = 24 * 1024 * 1024;  // 24MB

pub struct MemoryMonitor {
    current_usage: AtomicUsize,
}

impl MemoryMonitor {
    /// Check if request can be processed within memory limits
    pub fn check_available(&self, estimated_size: usize) -> Result<(), MemoryError> {
        let current = self.current_usage.load(Ordering::Relaxed);

        if current >= MEMORY_HARD_LIMIT {
            return Err(MemoryError::HardLimitExceeded);
        }

        if current + estimated_size > MEMORY_HARD_LIMIT {
            return Err(MemoryError::WouldExceedLimit);
        }

        if current > MEMORY_SOFT_LIMIT {
            tracing::warn!("Memory usage {} exceeds soft limit", current);
        }

        Ok(())
    }
}
```

#### 10.2 Memory Profiling (Development)

**Purpose**: Track memory allocations during development (feature-gated).

**Implementation**:

```rust
// src/utils/memory.rs (continued)
#[cfg(feature = "memory-profiling")]
pub struct AllocationTracker {
    allocations: DashMap<usize, AllocationInfo>,
    peak_usage: AtomicUsize,
}

#[cfg(feature = "memory-profiling")]
struct AllocationInfo {
    size: usize,
    file: &'static str,
    line: u32,
}

#[cfg(feature = "memory-profiling")]
impl AllocationTracker {
    pub fn log_stats(&self) {
        let total: usize = self.allocations.iter().map(|e| e.size).sum();
        let peak = self.peak_usage.load(Ordering::Relaxed);

        tracing::info!(
            "Memory stats: current={}KB, peak={}KB, allocations={}",
            total / 1024,
            peak / 1024,
            self.allocations.len()
        );
    }
}
```

### 11. Testing Strategy

The testing and CI workflows in this section form the Rust-specific quality gates defined in `.specify/memory/constitution-rust.md` and are intended as the direct equivalents of the main constitution’s CMocka, gcov/lcov, lint, and format validation steps.

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

## Error Handling Matrix

The following table maps edge cases (EC-*) from the specification to their corresponding ONVIF fault codes and HTTP status codes:

| Edge Case | Scenario | ONVIF Fault | HTTP Status | Implementation |
|-----------|----------|-------------|-------------|----------------|
| EC-001 | Invalid operation name | `ter:ActionNotSupported` | 500 | Return fault in SOAP response |
| EC-002 | Malformed SOAP XML | `ter:WellFormed` | 400 | Validate before parsing |
| EC-003 | Invalid profile token | `ter:InvalidArgVal/ter:NoProfile` | 400 | Check profile registry |
| EC-004 | Concurrent requests | N/A | 200 | Thread pool + mutex protection |
| EC-005 | Hardware unavailable | `ter:HardwareFailure` | 500 | Platform trait returns error |
| EC-006 | Missing required params | `ter:InvalidArgVal/ter:InvalidArgs` | 400 | Schema validation |
| EC-007 | Unimplemented endpoint | N/A | 404 | Router returns Not Found |
| EC-008 | Oversized payload | N/A | 413 | Check before parsing |
| EC-009 | Concurrent config modify | N/A | 200 | Mutex serialization |
| EC-010 | System time unavailable | N/A | 200 | Return `DateTimeType=Manual` |
| EC-011 | Auth failure | `ter:NotAuthorized` | 401 | WWW-Authenticate header |
| EC-012 | Brute force detected | N/A | 429 | IP blocklist check |
| EC-013 | Max users reached | `ter:MaxUsers` | 400 | Check user count |
| EC-014 | Discovery disabled | N/A | N/A | Silent ignore (no response) |
| EC-015 | Config storage full | `ter:ConfigurationConflict` | 500 | Log warning, keep in-memory |
| EC-016 | Memory limit exceeded | N/A | 503 | Memory monitor check |
| EC-017 | XML attack detected | N/A | 400 | Immediate connection close |

**Implementation Pattern**:

```rust
// src/onvif/error.rs
use thiserror::Error;

#[derive(Error, Debug)]
pub enum OnvifError {
    #[error("Action not supported: {0}")]
    ActionNotSupported(String),

    #[error("Malformed XML: {0}")]
    WellFormed(String),

    #[error("Invalid argument: {0}")]
    InvalidArgVal { subcode: String, reason: String },

    #[error("Hardware failure: {0}")]
    HardwareFailure(String),

    #[error("Not authorized")]
    NotAuthorized,

    #[error("Maximum users reached")]
    MaxUsers,

    #[error("Configuration conflict: {0}")]
    ConfigurationConflict(String),
}

impl OnvifError {
    /// Convert to SOAP fault XML response
    pub fn to_soap_fault(&self) -> String {
        match self {
            OnvifError::ActionNotSupported(msg) => soap_fault(
                "soap:Receiver",
                "ter:ActionNotSupported",
                msg,
            ),
            OnvifError::WellFormed(msg) => soap_fault(
                "soap:Sender",
                "ter:WellFormed",
                msg,
            ),
            OnvifError::InvalidArgVal { subcode, reason } => soap_fault(
                "soap:Sender",
                &format!("ter:InvalidArgVal/ter:{}", subcode),
                reason,
            ),
            OnvifError::NotAuthorized => soap_fault(
                "soap:Sender",
                "ter:NotAuthorized",
                "Authentication required",
            ),
            // ... other mappings
        }
    }

    /// Get appropriate HTTP status code
    pub fn http_status(&self) -> u16 {
        match self {
            OnvifError::ActionNotSupported(_) => 500,
            OnvifError::WellFormed(_) => 400,
            OnvifError::InvalidArgVal { .. } => 400,
            OnvifError::HardwareFailure(_) => 500,
            OnvifError::NotAuthorized => 401,
            OnvifError::MaxUsers => 400,
            OnvifError::ConfigurationConflict(_) => 500,
        }
    }
}

fn soap_fault(code: &str, subcode: &str, reason: &str) -> String {
    format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope"
               xmlns:ter="http://www.onvif.org/ver10/error">
  <soap:Body>
    <soap:Fault>
      <soap:Code>
        <soap:Value>{}</soap:Value>
        <soap:Subcode>
          <soap:Value>{}</soap:Value>
        </soap:Subcode>
      </soap:Code>
      <soap:Reason>
        <soap:Text xml:lang="en">{}</soap:Text>
      </soap:Reason>
    </soap:Fault>
  </soap:Body>
</soap:Envelope>"#, code, subcode, reason)
}
```

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
