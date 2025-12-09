# Rust Implementation Addendum to Anyka AK3918 Camera Firmware Constitution

**Version**: 1.0.0 | **Created**: 2025-01-28 | **Applies to**: `cross-compile/onvif-rust/`

## Overview

This document is a **Rust-specific addendum** to the main [constitution.md](constitution.md). It defines technical standards, tooling, and conventions for the Rust ONVIF API implementation while inheriting all core principles from the parent constitution.

**Parent Constitution Reference**: All principles from the main constitution apply unless explicitly overridden below:

- **I. Security First** - Fully applies (Rust's memory safety enhances this)
- **II. Standards Compliance** - Fully applies (ONVIF 24.12, Profile S)
- **III. Open Source Transparency** - Fully applies
- **IV. Hardware Compatibility** - Fully applies (ARM cross-compilation)
- **V. Developer Experience** - Rust-specific tooling defined below
- **VI. Performance Optimization** - Fully applies
- **VII. Reliability & Production-Readiness** - Fully applies
- **VIII. Educational Value** - Fully applies

---

## Rust-Specific Technical Standards

### Language & Toolchain

| Aspect | Standard |
|--------|----------|
| **Primary Language** | Rust (2024 edition) |
| **Compiler** | rustc via Cargo with cross-compilation support |
| **Target Architecture** | `armv5te-unknown-linux-uclibceabi` (Anyka AK3918 SoC) |
| **Build System** | Cargo with `.cargo/config.toml` for cross-compilation |
| **Documentation** | rustdoc for API documentation generation |

### Core Dependencies

| Category | Crate | Purpose | Replaces (C) |
|----------|-------|---------|--------------|
| **Async Runtime** | `tokio` | Async I/O, task scheduling | pthread |
| **HTTP Framework** | `axum` | HTTP server, routing, middleware | Custom HTTP server |
| **SOAP/XML** | `quick-xml` + `serde` | SOAP XML serialization/deserialization | gSOAP 2.8+ |
| **Configuration** | `toml` + `serde` | TOML configuration parsing | Custom INI parser |
| **Logging** | `tracing` + `tracing-subscriber` | Structured logging | ak_print / custom |
| **Authentication** | `argon2`, `sha1`, `md-5` | Password hashing, WS-Security | Custom crypto |
| **Concurrency** | `parking_lot`, `dashmap` | Fast mutexes, concurrent maps | pthread mutexes |
| **Error Handling** | `thiserror`, `anyhow` | Error types and propagation | errno / return codes |

### Testing Framework

| Aspect | Tool | Replaces (C) |
|--------|------|--------------|
| **Unit Tests** | Rust native `#[test]` + `#[tokio::test]` | CMocka |
| **Mocking** | `mockall` crate | CMocka mocks with `__wrap_` functions |
| **Integration Tests** | `tests/` directory with `reqwest` | Custom test harness |
| **Benchmarking** | `criterion` crate | Custom benchmarks |
| **Coverage** | `cargo-tarpaulin` | gcov/lcov |

### FFI Bindings

| Aspect | Approach |
|--------|----------|
| **Binding Generation** | `bindgen` for Anyka SDK headers |
| **Build Script** | `build.rs` with `cc` crate for C compilation |
| **Safety Wrappers** | Safe Rust wrappers around all unsafe FFI calls |
| **Platform Traits** | Trait-based abstraction for hardware operations |

---

## Rust Naming Conventions

### Crate & Module Names

- **Crate name**: `onvif-rust` (kebab-case)
- **Module names**: `snake_case` (e.g., `ws_discovery`, `http_digest`)

### Types & Traits

- **Structs**: `PascalCase` (e.g., `DeviceService`, `MediaProfile`)
- **Enums**: `PascalCase` with `PascalCase` variants (e.g., `UserLevel::Administrator`)
- **Traits**: `PascalCase` (e.g., `VideoInput`, `PTZControl`)
- **Type aliases**: `PascalCase` (e.g., `Result<T> = std::result::Result<T, OnvifError>`)

### Functions & Variables

- **Functions**: `snake_case` (e.g., `handle_get_device_information()`)
- **Methods**: `snake_case` (e.g., `config.get_string()`)
- **Variables**: `snake_case` (e.g., `max_payload_size`)
- **Constants**: `SCREAMING_SNAKE_CASE` (e.g., `MEMORY_HARD_LIMIT`)
- **Statics**: `SCREAMING_SNAKE_CASE` with `lazy_static!` or `once_cell`

### Test Functions

- **Unit tests**: `test_<function_name>_<scenario>` (e.g., `test_validate_digest_success`)
- **Integration tests**: `test_<service>_<operation>` (e.g., `test_device_get_information`)

### Files & Directories

- **Source files**: `snake_case.rs` (e.g., `ws_security.rs`, `rate_limit.rs`)
- **Module directories**: `snake_case/` with `mod.rs` (e.g., `onvif/device/mod.rs`)
- **Test files**: `tests/<category>/<test_name>.rs`

---

## Code Quality Tools (MANDATORY)

All Rust code MUST pass these validation steps:

### 1. Formatting

```bash
cargo fmt --check
```

- Must pass with zero issues
- Use `cargo fmt` to auto-format

### 2. Linting

```bash
cargo clippy -- -D warnings
```

- Must pass with zero warnings (warnings treated as errors)
- Address all clippy suggestions

### 3. Testing

```bash
cargo test --all-features
```

- All tests must pass with 100% success rate
- Target: 80% minimum code coverage

### 4. Documentation

```bash
cargo doc --no-deps
```

- All public APIs must have rustdoc comments
- Use `///` for item documentation
- Use `//!` for module-level documentation

---

## Rust-Specific Code Organization

### Project Structure

```
cross-compile/onvif-rust/
├── Cargo.toml              # Package manifest
├── build.rs                # Build script for FFI
├── .cargo/
│   └── config.toml         # Cross-compilation config
├── src/
│   ├── main.rs             # Entry point
│   ├── lib.rs              # Library root
│   ├── onvif/              # ONVIF services
│   ├── auth/               # Authentication
│   ├── security/           # Security hardening
│   ├── platform/           # Hardware abstraction
│   ├── config/             # Configuration
│   ├── logging/            # Logging
│   ├── users/              # User management
│   ├── discovery/          # WS-Discovery
│   ├── utils/              # Utilities
│   └── ffi/                # FFI bindings
├── tests/                  # Integration tests
│   ├── onvif/              # ONVIF service tests
│   └── integration/        # Full integration tests
└── benches/                # Benchmarks
```

### Module Organization Rules

1. **One responsibility per module**: Each module has a single, clear purpose
2. **Public API in `mod.rs`**: Re-export public items from submodules
3. **Internal items private**: Use `pub(crate)` or keep private unless needed externally
4. **Error types per module**: Define error enums with `thiserror`
5. **Traits for abstraction**: Use traits for testability and hardware abstraction

### Import Order

```rust
// 1. Standard library
use std::collections::HashMap;
use std::sync::Arc;

// 2. External crates
use axum::{Router, routing::post};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

// 3. Crate modules
use crate::config::ConfigRuntime;
use crate::platform::Platform;
```

---

## Memory Safety (MANDATORY)

Rust's ownership system provides memory safety by default. Additional requirements:

- **No `unsafe` without justification**: Document all `unsafe` blocks with safety comments
- **FFI boundaries**: Wrap all FFI calls in safe abstractions
- **Error propagation**: Use `Result<T, E>` for fallible operations, never panic in library code
- **Resource cleanup**: Use RAII patterns; implement `Drop` where needed
- **Thread safety**: Use `Send`/`Sync` bounds appropriately; prefer `Arc<RwLock<T>>` for shared state

---

## Async/Await Guidelines

- **Tokio runtime**: Use `#[tokio::main]` for entry point
- **Async all the way**: Don't block in async contexts; use `tokio::spawn_blocking` for CPU-bound work
- **Cancellation safety**: Document cancellation behavior for async functions
- **Timeouts**: Use `tokio::time::timeout` for operations that may hang

---

## Error Handling Patterns

### Error Types

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ServiceError {
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    #[error("Hardware failure: {0}")]
    HardwareFailure(#[from] PlatformError),

    #[error("Configuration error: {0}")]
    ConfigError(#[from] ConfigError),
}
```

### Result Propagation

- Use `?` operator for error propagation
- Convert errors at boundaries with `map_err` or `From` implementations
- Log errors at the point of handling, not propagation

---

## Governance

### Applicability

This addendum applies **only** to code within `cross-compile/onvif-rust/` directory. The main constitution applies to all other code (C implementation, scripts, etc.).

### Conflict Resolution

If this addendum conflicts with the main constitution:

1. For Rust code: This addendum takes precedence
2. For shared concerns (security, standards): Main constitution principles apply
3. When unclear: Consult project maintainers

### Amendment Process

Same as main constitution - requires documentation, impact analysis, and maintainer approval.

---

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.0.0 | 2025-01-28 | Initial Rust addendum created for ONVIF API rewrite |
