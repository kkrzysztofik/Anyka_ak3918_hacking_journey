# Development Standards - Rust (onvif-rust)

## Code Formatting & Linting

### ⚠️ Cross-Compilation Note
Default target is ARM (`armv5te-unknown-linux-uclibceabi`). For host-side operations (test, lint), **you MUST specify x86_64 target**. Load the vendored toolchain first with `source ./setenv.sh` from the repo root (exports `$CARGO`, `$RUSTC`, `$RUSTDOC`). Never use bare `cargo`.

### Mandatory Before Commit
```bash
cd cross-compile/onvif-rust
$CARGO fmt                          # Format all code (target-independent)
$CARGO fmt --check                  # Verify formatting (CI)
$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings  # Lint on host
$CARGO test --target x86_64-unknown-linux-gnu                   # Test on host
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
    #[error("Action not supported: {0}")]
    ActionNotSupported(String),

    #[error("Request not well-formed: {0}")]
    WellFormed(String),

    #[error("Invalid argument: {subcode} - {reason}")]
    InvalidArgVal { subcode: String, reason: String },

    #[error("Hardware failure: {0}")]
    HardwareFailure(String),

    #[error("Not authorized: {0}")]
    NotAuthorized(String),

    #[error("Maximum number of users reached")]
    MaxUsers,

    #[error("Configuration conflict: {0}")]
    ConfigurationConflict(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Not found: {0}")]
    NotFound(String),
}

impl OnvifError {
    pub fn invalid_arg(reason: impl Into<String>) -> Self {
        OnvifError::InvalidArgVal { subcode: "ter:InvalidArgVal".into(), reason: reason.into() }
    }
    pub fn missing_arg(reason: impl Into<String>) -> Self {
        OnvifError::InvalidArgVal { subcode: "ter:MissingAttr".into(), reason: reason.into() }
    }
    pub fn out_of_range(reason: impl Into<String>) -> Self {
        OnvifError::InvalidArgVal { subcode: "ter:InvalidArgVal".into(), reason: reason.into() }
    }
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
├── main.rs                   # Binary entry point (onvif-rust)
├── app.rs                    # Application lifecycle
│
├── onvif/                    # ONVIF services
│   ├── device/               # Device service
│   │   ├── ops/              # {system, network, discovery, users}.rs
│   │   ├── service.rs        # DeviceService handler
│   │   ├── state.rs          # Service state
│   │   ├── store.rs          # Device store
│   │   ├── types.rs          # Device types
│   │   ├── faults.rs         # Device faults
│   │   ├── validation.rs     # Device validation
│   │   └── user_types.rs     # ONVIF user levels
│   ├── media/                # Media service
│   ├── ptz/                  # PTZ service
│   ├── imaging/              # Imaging service
│   ├── analytics/            # Analytics service
│   ├── events/               # Events service
│   ├── discovery/            # Discovery service
│   ├── common/               # dispatch.rs (dispatch_sync/dispatch_async)
│   ├── dispatcher/           # mod.rs (ServiceHandler trait), parse_body
│   ├── error/                # mod.rs (OnvifError)
│   ├── soap/                 # build.rs, parse.rs, model.rs
│   ├── types/                # {common, device, media, imaging, ptz}
│   ├── auth_requirements.rs  # Per-action auth levels
│   ├── server.rs             # HTTP/SOAP server
│   ├── ws_security.rs        # WS-Security processing
│   └── mod.rs
│
├── hal/                      # Hardware Abstraction Layer (IPC-based)
│   ├── common/               # {audio, imaging, ptz, sdk_types, video}.rs traits
│   ├── anyka/                # ipc/ (shm_ring.rs), ptz/, sdk.rs
│   └── stub/                 # Test stubs
│
├── streaming/                # Bridge to streaming-lib
│   ├── bridge.rs             # Frame delivery bridge
│   ├── config.rs             # Stream configuration
│   ├── helpers.rs            # Streaming utilities
│   ├── service.rs            # Streaming service
│   └── telemetry.rs          # Stream telemetry
│
├── platform/                 # Platform abstraction (business logic)
│   ├── anyka/                # audio_encoder, audio_input, context, imaging,
│   │                         # lifecycle, network_info, night_mode, ptz_actor,
│   │                         # ptz_control, supervisor, video_encoder,
│   │                         # video_input, tests/
│   ├── common/               # traits.rs (Platform traits, #[cfg_attr(test, automock)])
│   └── stub/                 # Test stubs
│
├── security/                 # Auth + security hardening
│   ├── audit.rs              # Security audit logging
│   ├── brute_force.rs        # Brute force protection
│   ├── rate_limit.rs         # Rate limiting
│   └── xml_security.rs       # XXE/XML bomb protection
│
├── discovery/                # WS-Discovery (UDP multicast)
├── config/                   # Configuration management, persistence & user management
│   ├── profiles/             # Media profiles
│   ├── users/                # User accounts, passwords & ONVIF user levels
│   ├── persistence.rs        # Persistent storage
│   ├── runtime.rs            # Runtime settings
│   ├── storage.rs            # Storage backend
│   └── types.rs              # Config types
├── lifecycle/                # App lifecycle (startup, shutdown, health)
├── logging/                  # HTTP & platform logging
├── validation/               # Input validation
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
| Serialization | serde, quick-xml 0.41 |
| Logging | tracing, tracing-subscriber, tracing-appender |
| Errors (lib) | thiserror 2.0 |
| Errors (app) | anyhow |
| Memory tracking | cap 0.1 (24MB hard limit) |
| Concurrency | parking_lot, portable-atomic |
| Bytes/buffers | bytes |
| Testing | mockall 0.15, wiremock 0.6, criterion 0.8 |
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
- Check for security advisories (`$CARGO audit`)

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

## Platform-HAL Layering

Platform layer (`src/platform/`) should minimize unsafe code, FFI calls, and raw pointer operations.
All hardware access should go through HAL traits from `hal/common/`.

**Detailed rules and current exceptions**: See [`.serena/memories/platform-hal-layering.md`](platform-hal-layering.md)

> ⚠️ Note: The layering is **directional guidance**, not strictly enforced. Known violations are tracked in the platform-hal-layering.md document.

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
/// Returns `OnvifError::InvalidArgVal` if input is invalid
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
# First: source ./setenv.sh to export $CARGO from the vendored toolchain
cd cross-compile
$CARGO fmt && \
$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings && \
$CARGO test --target x86_64-unknown-linux-gnu && \
$CARGO doc --target x86_64-unknown-linux-gnu --no-deps
```

**Note**: Commands run from `cross-compile/` apply to the entire workspace (onvif-rust + streaming-lib + anyka-init).

- [ ] Code formatted (`$CARGO fmt`)
- [ ] No clippy warnings (`$CARGO clippy -- -D warnings`)
- [ ] All tests pass (`$CARGO test`)
- [ ] No `unwrap()`/`expect()` in production code
- [ ] Public APIs documented
- [ ] New functionality has tests
