# ONVIF Rust Architecture

This document describes the architecture of the Rust ONVIF implementation for Anyka AK3918-based IP cameras.

## Application Lifecycle

The application uses a "kinda-hybrid" lifecycle approach that combines explicit initialization/shutdown with Rust's ownership model:

- **Explicit `start()`**: Ordered, async, fallible initialization
- **Explicit `shutdown()`**: Coordinated, async, graceful shutdown
- **No global state**: All state owned by the `Application` struct
- **No `Drop` for async**: `Drop` only deallocates memory; async cleanup via `shutdown()`
- **Dependency injection**: Components receive dependencies, not global lookups
- **Optional components**: Handled at startup with degraded mode support

### Component Hierarchy

```
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

### Startup Sequence

The application initializes components in a specific order:

1. **Configuration** - Load and validate TOML configuration
2. **Platform** - Initialize hardware abstraction layer
3. **Services** - Initialize ONVIF services (Device, Media are required; PTZ, Imaging are optional)
4. **Network** - Start HTTP server and WS-Discovery
5. **Discovery** - Send WS-Discovery Hello message

```rust
// Example startup
let app = Application::start("/etc/onvif/config.toml").await?;
```

If optional components (PTZ, Imaging) fail to initialize, the application continues in **degraded mode** with those services unavailable.

### Shutdown Sequence

Shutdown occurs in reverse order with graceful coordination:

1. Send WS-Discovery Bye message
2. Stop accepting new HTTP connections
3. Broadcast shutdown signal to all tasks
4. Wait for in-flight requests (with timeout)
5. Shutdown services in reverse order
6. Shutdown platform
7. Return shutdown report

```rust
// Example shutdown
let report = app.shutdown().await;
match report.status {
    ShutdownStatus::Success => println!("Clean shutdown"),
    ShutdownStatus::Timeout => println!("Some tasks timed out"),
    ShutdownStatus::Error => println!("Errors: {:?}", report.errors),
}
```

### Health Checks

The application provides health status for observability:

```rust
let health = app.health();
println!("Status: {}", health.status);  // Healthy, Degraded, or Unhealthy
println!("Uptime: {:?}", health.uptime);
println!("Degraded services: {:?}", health.degraded_services);
```

## Module Structure

```
src/
├── app.rs              # Main Application struct
├── lib.rs              # Library root with module exports
├── main.rs             # Daemon entry point
├── lifecycle/
│   ├── mod.rs          # Lifecycle types (StartupError, ShutdownReport, etc.)
│   ├── startup.rs      # Startup sequence helpers
│   ├── shutdown.rs     # Shutdown coordination
│   └── health.rs       # Health status types
├── config/             # Configuration system (TOML)
├── platform/           # Hardware abstraction
├── onvif/              # ONVIF service implementations
├── auth/               # Authentication
├── security/           # Security hardening
├── discovery/          # WS-Discovery
├── users/              # User management
├── utils/              # Utilities
└── ffi/                # Anyka SDK FFI bindings
```

## Design Decisions

### Why Not Pure RAII?

Rust's `Drop` trait is synchronous, but our cleanup operations are async (HTTP server shutdown, WS-Discovery Bye, etc.). Using explicit `shutdown()` allows proper async cleanup.

### Why Not Global State?

Global state (static variables) makes testing difficult and prevents running multiple instances. The `Application` struct owns all state, enabling:

- Parallel test execution
- Multiple instances for integration testing
- Clear ownership and lifetime semantics

### Why Explicit Initialization Order?

The C implementation has a proven initialization order that ensures dependencies are satisfied. We maintain this order for reliability and debuggability.

### Why Optional Components?

PTZ and Imaging services depend on hardware that may not be available. Rather than failing completely, the application runs in degraded mode, providing core functionality (Device, Media services) while logging the unavailable components.

## Error Handling

### Startup Errors

```rust
pub enum StartupError {
    Config(String),     // Configuration loading/validation failed
    Platform(String),   // Hardware initialization failed
    Services(String),   // Required service initialization failed
    Network(String),    // HTTP server failed to start
    Io(std::io::Error), // File I/O error
}
```

### Runtime Errors

```rust
pub enum RuntimeError {
    HttpServer(String), // HTTP server error
    Service(String),    // Service error during request
    Signal(String),     // Signal handling error
}
```

## Testing

### Unit Tests

Each module contains unit tests using `#[cfg(test)]` modules. Run with:

```bash
cargo test --lib
```

### Integration Tests

Integration tests in `tests/` directory test full request/response cycles. Run with:

```bash
cargo test --test '*'
```

### Testing with Mocks

The platform abstraction uses traits with `#[mockall::automock]` for testing without hardware:

```rust
#[tokio::test]
async fn test_with_mock_platform() {
    let mut mock = MockPlatform::new();
    mock.expect_get_device_info()
        .returning(|| Ok(DeviceInfo { /* ... */ }));

    let app = Application::start_with_platform(mock, "config.toml").await?;
    // Test...
}
```

## Memory Management

The application targets embedded systems with limited memory (24MB hard limit). Key strategies:

- Avoid allocations in hot paths
- Use `Arc` sparingly (only for shared state)
- Implement memory monitoring (feature-gated)
- Reject requests when approaching memory limits

## Signal Handling

The application handles:

- **SIGINT** (Ctrl+C) - Graceful shutdown
- **SIGTERM** - Graceful shutdown (for systemd/container orchestration)

```rust
// In Application::run()
tokio::select! {
    _ = tokio::signal::ctrl_c() => { /* shutdown */ }
    _ = sigterm_signal() => { /* shutdown */ }
}
```
