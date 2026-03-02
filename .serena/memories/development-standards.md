# Development Standards - Rust (onvif-rust)

## Code Formatting & Linting

### ⚠️ Cross-Compilation Note
Default target is ARM (`armv5te-unknown-linux-uclibceabi`). For host-side operations (test, lint), **you MUST specify x86_64 target**.

### Mandatory Before Commit
```bash
cd cross-compile/onvif-rust
cargo fmt                          # Format all code (target-independent)
cargo fmt --check                  # Verify formatting (CI)
cargo clippy --target x86_64-unknown-linux-gnu -- -D warnings  # Lint on host
cargo test --target x86_64-unknown-linux-gnu                   # Test on host
```

## Naming Conventions

| Element | Convention | Example |
|---------|------------|---------|
| Variables | snake_case | `device_info`, `user_count` |
| Functions | snake_case | `get_device_info()`, `validate_input()` |
| Types/Structs | CamelCase | `DeviceService`, `MediaProfile` |
| Traits | CamelCase | `PlatformAdapter`, `AuthProvider` |
| Constants | SCREAMING_SNAKE | `MAX_CONNECTIONS`, `DEFAULT_PORT` |
| Modules | snake_case | `device_service.rs`, `auth_handler.rs` |
| Enums | CamelCase | `UserLevel`, `StreamType` |
| Enum Variants | CamelCase | `UserLevel::Administrator` |

## Error Handling

### Required Patterns

```rust
// ❌ WRONG - Never use in production code
let value = result.unwrap();
let value = result.expect("should work");

// ✅ CORRECT - Use ? operator for propagation
let value = result?;

// ✅ CORRECT - Use match for explicit handling
match result {
    Ok(value) => process(value),
    Err(e) => {
        tracing::error!("Operation failed: {}", e);
        return Err(e.into());
    }
}

// ✅ CORRECT - Use if let for optional handling
if let Some(value) = optional {
    process(value);
}
```

### Error Types

```rust
// Library/Domain errors - use thiserror
use thiserror::Error;

#[derive(Error, Debug)]
pub enum OnvifError {
    #[error("Authentication failed")]
    AuthenticationFailed,
    
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
    
    #[error("Platform error: {0}")]
    Platform(#[from] PlatformError),
}

// Application errors - use anyhow with context
use anyhow::{Context, Result};

fn load_config() -> Result<Config> {
    let content = std::fs::read_to_string("config.toml")
        .context("Failed to read configuration file")?;
    toml::from_str(&content)
        .context("Failed to parse configuration")
}
```

## Async/Await Patterns

```rust
// ✅ CORRECT - Use tokio runtime
use tokio::sync::{Mutex, RwLock, mpsc};

// ✅ CORRECT - Async-aware synchronization
async fn update_state(state: &Arc<RwLock<AppState>>, data: Data) {
    let mut state = state.write().await;
    state.update(data);
}

// ❌ WRONG - Blocking in async context
use std::sync::Mutex;  // Blocks async runtime

// ✅ CORRECT - Use tokio channels for async communication
let (tx, mut rx) = mpsc::channel(100);
tokio::spawn(async move {
    while let Some(msg) = rx.recv().await {
        process(msg).await;
    }
});
```

## Logging

```rust
use tracing::{error, warn, info, debug, trace};

// Logging levels and usage:
error!("Critical failure requiring immediate attention: {}", e);
warn!("Unexpected but handled situation: {}", details);
info!("High-level operational event: startup, config change");
debug!("Detailed flow for debugging: request={:?}", req);
trace!("Extremely verbose low-level details");

// ❌ WRONG - Never use println! in production
println!("Debug: {}", value);

// ✅ CORRECT - Use structured logging
info!(user = %username, action = "login", "User authenticated");
```

## Module Organization

```
src/
├── lib.rs                    # Library root, global allocator (cap 24MB)
├── main.rs                   # Binary entry point
├── app.rs                    # Application lifecycle
│
├── onvif/                    # ONVIF services
│   ├── device/               # Device service (handlers, types, faults)
│   ├── media/                # Media service
│   ├── ptz/                  # PTZ service
│   ├── imaging/              # Imaging service
│   ├── types/                # Shared ONVIF type definitions
│   ├── dispatcher.rs         # SOAP action routing
│   ├── server.rs             # HTTP/SOAP server
│   ├── soap.rs               # SOAP envelope handling
│   └── ws_security.rs        # WS-Security processing
│
├── hal/                      # Hardware Abstraction Layer (IPC-based)
│   ├── anyka_sdk.rs          # SDK type definitions
│   ├── vendor_ipc.rs         # Unix socket IPC client
│   ├── video.rs              # Video HAL
│   ├── audio.rs              # Audio HAL
│   ├── imaging.rs            # Imaging HAL
│   ├── ptz.rs                # PTZ HAL
│   └── ptz_driver.rs         # PTZ motor driver
│
├── streaming/                # Bridge to streaming-lib
│   ├── bridge.rs             # Frame delivery bridge
│   ├── config.rs             # Stream configuration
│   ├── helpers.rs            # Streaming utilities
│   └── service.rs            # Streaming service
│
├── platform/                 # Platform abstraction
│   ├── traits.rs             # Platform trait definitions
│   ├── anyka.rs              # Anyka implementation
│   ├── frame.rs              # Frame ownership & stream ID
│   ├── hw_ptz.rs             # Hardware PTZ
│   ├── stubs.rs              # Test stubs
│   └── validation.rs         # Platform validation
│
├── auth/                     # Authentication
│   ├── ws_security.rs        # WS-Security
│   ├── http_digest.rs        # HTTP Digest
│   ├── http_basic.rs         # HTTP Basic
│   └── credentials.rs        # Credential management
│
├── security/                 # Security hardening
│   ├── xml_security.rs       # XXE/XML bomb protection
│   ├── rate_limit.rs         # Rate limiting
│   ├── brute_force.rs        # Brute force protection
│   └── audit.rs              # Security audit logging
│
├── discovery/                # WS-Discovery (UDP multicast)
├── config/                   # Configuration management, persistence & user management
│   └── users/                # User accounts, passwords & ONVIF user levels
├── lifecycle/                # App lifecycle (startup, shutdown, health)
├── logging/                  # HTTP & platform logging
├── net/                      # Network utilities (IP detection)
├── validation/               # H.264 playback & stream validation
│
└── utils/                    # Shared utilities
    ├── validation.rs         # Input validation
    └── memory.rs             # Memory management
```

## Dependencies

### Preferred Crates
| Purpose | Crate |
|---------|-------|
| Async runtime | tokio |
| Web framework | axum 0.8, tower, tower-http |
| Serialization | serde, quick-xml 0.39 |
| Logging | tracing, tracing-subscriber, tracing-appender |
| Errors (lib) | thiserror 2.0 |
| Errors (app) | anyhow |
| Memory tracking | cap 0.1 (24MB hard limit) |
| Concurrency | parking_lot, dashmap, portable-atomic |
| Bytes/buffers | bytes |
| Testing | mockall 0.14, wiremock 0.6, criterion 0.8 |
| Validation | validator |
| Time | chrono |
| UUID | uuid |
| Streaming | streaming-lib (workspace path dep) |
| CLI | clap (derive) |
| Crypto | argon2, sha1, md-5, hmac, constant_time_eq |

### Adding Dependencies
- Prefer well-maintained, widely-used crates
- Avoid heavy dependencies for simple tasks
- Keep `Cargo.toml` organized and sorted
- Check for security advisories (`cargo audit`)

## Unsafe Code

```rust
// ❌ WRONG - Unjustified unsafe
unsafe {
    let ptr = raw_ptr as *mut u8;
    *ptr = 42;
}

// ✅ CORRECT - Documented and justified
// SAFETY: This is safe because:
// 1. `raw_ptr` is guaranteed valid by caller contract
// 2. Memory is owned by this function and not aliased
// 3. Pointer is properly aligned (verified in caller)
unsafe {
    let ptr = raw_ptr as *mut u8;
    *ptr = 42;
}
```

## Documentation

```rust
/// Brief description of the function.
///
/// More detailed explanation if needed.
///
/// # Arguments
///
/// * `param1` - Description of first parameter
/// * `param2` - Description of second parameter
///
/// # Returns
///
/// Description of return value
///
/// # Errors
///
/// Returns `OnvifError::InvalidRequest` if input is invalid
///
/// # Examples
///
/// ```
/// let result = get_device_info(&platform).await?;
/// ```
pub async fn get_device_info(platform: &impl Platform) -> Result<DeviceInfo, OnvifError> {
    // implementation
}
```

## Pre-Commit Checklist

```bash
# Run all checks across workspace (note: x86_64 target for host-side operations)
cd cross-compile
cargo fmt && \
cargo clippy --target x86_64-unknown-linux-gnu -- -D warnings && \
cargo test --target x86_64-unknown-linux-gnu && \
cargo doc --no-deps
```

**Note**: Commands run from `cross-compile/` apply to the entire workspace (onvif-rust + streaming-lib).

- [ ] Code formatted (`cargo fmt`)
- [ ] No clippy warnings (`cargo clippy -- -D warnings`)
- [ ] All tests pass (`cargo test`)
- [ ] No `unwrap()`/`expect()` in production code
- [ ] Public APIs documented
- [ ] New functionality has tests
