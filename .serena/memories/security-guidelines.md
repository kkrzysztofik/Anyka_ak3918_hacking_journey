# Security Guidelines (MANDATORY)

## Input Validation

**ALL user inputs must be properly validated and sanitized**:

- Use the `validator` crate with derive macros for struct field validation
- Validate string lengths and bounds using `String::len()` and range checks
- Sanitize XML/SOAP input to prevent XXE (XML External Entity) and XML bomb attacks
- Validate numeric ranges and types using `validator::Validate` trait
- Use `SecurityValidator` from `utils::validation` for XML content validation
- Validate paths to prevent directory traversal attacks using `PathValidator`

## Memory Safety

**Rust's ownership model prevents most memory vulnerabilities, but review for**:

- **Unsafe Code**: All `unsafe` blocks MUST be minimal, justified, and documented
- **Panic Safety**: Avoid `unwrap()` and `expect()` in production code paths - use `?` operator or proper error handling
- **Data Races**: Ensure proper synchronization in concurrent code using `Arc`, `Mutex`, `RwLock`, or `tokio::sync` primitives
- **Resource Leaks**: Ensure proper cleanup of resources (file handles, network connections) even in error paths
- **Memory Limits**: Respect embedded memory constraints (24MB limit enforced by `cap` allocator)

## Error Handling

**Proper error handling prevents information leakage and system failures**:

- Use `Result<T, E>` types for all fallible operations - NO panics in production code
- Use `thiserror` for library/domain errors to provide precise error types
- Use `anyhow` for application-level error handling with context
- Never expose internal error details to clients - map to ONVIF-compliant error codes
- Log errors with appropriate levels (`error!`, `warn!`, `debug!`) using `tracing`
- Ensure error paths don't leak sensitive information (passwords, file paths, internal state)

## Network Security

**Secure network communications**:

- Validate all network input before processing using `SecurityValidator`
- Use proper error message handling (no information leakage) - return generic ONVIF faults
- Implement proper authentication and authorization using `auth` module
- Validate SOAP/XML input to prevent XXE, XML bombs, and injection attacks
- Use HTTPS/TLS for all production deployments (especially for HTTP Basic auth)
- Implement rate limiting using `security::RateLimiter` to prevent abuse
- Use timing-safe comparisons for authentication (e.g., `constant_time_eq` crate)

## Authentication & Authorization

**Secure authentication implementation**:

- Use `auth::WsSecurityValidator` for WS-Security UsernameToken (SOAP requests)
- Use `auth::HttpDigestAuth` for HTTP Digest authentication (RFC 2617)
- Use `auth::HttpBasicAuth` only with HTTPS - credentials are base64 encoded, NOT encrypted
- Store passwords using Argon2 hashing (see `users::password` module)
- Validate session management and nonce/timestamp freshness
- Ensure proper access controls for all endpoints using `auth::UserLevel`
- Implement brute force protection using `security::BruteForceProtection`
- Use timing-safe credential comparison to prevent timing attacks

## Unsafe Code Guidelines

**Minimize and justify all unsafe code**:

- **Documentation Required**: Every `unsafe` block MUST have a comment explaining why it's safe
- **Bounded Scope**: Keep unsafe blocks as small as possible
- **Invariant Preservation**: Document all invariants that must hold for the unsafe code to be safe
- **FFI Safety**: When calling C code via FFI, validate all inputs before passing to unsafe functions
- **Review Required**: All unsafe code MUST be reviewed by senior developers

```rust
// ❌ BAD - Unjustified unsafe
unsafe {
    let ptr = raw_ptr as *mut u8;
    *ptr = 42;  // No documentation, no justification
}

// ✅ GOOD - Justified and documented unsafe
// SAFETY: This is safe because:
// 1. `raw_ptr` is guaranteed to be valid by the caller contract
// 2. The memory is owned by this function and not aliased
// 3. The pointer is properly aligned (verified in caller)
unsafe {
    let ptr = raw_ptr as *mut u8;
    *ptr = 42;
}
```

## ONVIF Compliance Requirements (MANDATORY)

### Service Implementation

Complete implementation of all required services:

- **Device Service**: Device information, capabilities, system date/time
- **Media Service**: Profile management, stream URIs, video/audio configurations
- **PTZ Service**: Pan/tilt/zoom control, preset management, continuous move
- **Imaging Service**: Image settings, brightness, contrast, saturation controls

### SOAP Protocol

Proper XML/SOAP request/response handling:

- Correct XML namespace usage (ONVIF 24.12 namespaces)
- Proper SOAP envelope structure using `quick-xml` serialization
- ONVIF-compliant error codes and messages (map Rust errors to ONVIF faults)
- Proper content-type headers (`application/soap+xml`)
- Validate all SOAP requests using `SecurityValidator` before processing

### WS-Discovery

Correct multicast discovery implementation:

- Proper UDP multicast socket handling using `tokio::net::UdpSocket`
- Correct discovery message format (ONVIF 24.12 compliant)
- Proper device announcement and probe responses
- Handle network errors gracefully without panicking

### RTSP Streaming

Proper H.264 stream generation and delivery:

- Correct SDP format for stream descriptions
- Proper RTP packetization
- Correct stream URI generation with authentication tokens
- Proper authentication for RTSP streams (HTTP Digest or Basic)

### Error Codes

ONVIF-compliant error reporting:

- Use standard ONVIF error codes (map `thiserror` types to ONVIF faults)
- Provide meaningful error messages (but no internal details)
- Handle all error conditions gracefully using `Result` types
- Never panic on invalid input - return appropriate ONVIF fault

## Security Testing

**Validate security measures**:

- Test input validation with malicious inputs (XXE, XML bombs, path traversal)
- Verify authentication and authorization mechanisms
- Check for information leakage in error messages
- Test network security with various attack vectors
- Test rate limiting and brute force protection
- Verify panic safety - ensure no panics in production code paths
- Test concurrent access patterns for data races

## Common Security Patterns

### Input Validation Pattern

```rust
// ❌ BAD - No input validation, potential panic
fn process_username(username: &str) -> String {
    username.to_uppercase()  // Panics if username is invalid UTF-8
}

// ✅ GOOD - Proper input validation
use validator::Validate;

#[derive(Validate)]
struct UsernameInput {
    #[validate(length(min = 1, max = 64))]
    #[validate(regex = "USERNAME_PATTERN")]
    username: String,
}

fn process_username(input: UsernameInput) -> Result<String, ValidationError> {
    input.validate()?;
    Ok(input.username.to_uppercase())
}
```

### XML/SOAP Security

```rust
// ❌ BAD - No XML security validation
fn parse_soap_request(xml: &str) -> Result<SoapRequest, Error> {
    quick_xml::de::from_str(xml)  // Vulnerable to XXE attacks
}

// ✅ GOOD - XML security validation
use onvif_rust::utils::validation::SecurityValidator;

fn parse_soap_request(xml: &str) -> Result<SoapRequest, Error> {
    let validator = SecurityValidator::default();
    validator.check_xml_security(xml)?;  // Check for XXE, XML bombs
    quick_xml::de::from_str(xml)
}
```

### Error Handling Pattern

```rust
// ❌ BAD - Panic in production code
fn get_device_info() -> DeviceInfo {
    let config = load_config().unwrap();  // Panics on error
    DeviceInfo::from_config(&config)
}

// ✅ GOOD - Proper error handling
fn get_device_info() -> Result<DeviceInfo, OnvifError> {
    let config = load_config()?;  // Propagate error
    Ok(DeviceInfo::from_config(&config))
}

// ✅ ALSO GOOD - Handle error with context
fn get_device_info() -> Result<DeviceInfo, OnvifError> {
    let config = load_config()
        .context("Failed to load device configuration")?;
    Ok(DeviceInfo::from_config(&config))
}
```

### Authentication

```rust
// ❌ BAD - Timing-vulnerable comparison
fn validate_password(input: &str, stored: &str) -> bool {
    input == stored  // Timing attack vulnerable
}

// ✅ GOOD - Timing-safe comparison
use constant_time_eq::constant_time_eq;

fn validate_password(input: &str, stored: &str) -> bool {
    constant_time_eq(input.as_bytes(), stored.as_bytes())
}

// ✅ ALSO GOOD - Use Argon2 for password hashing
use argon2::{Argon2, PasswordHash, PasswordVerifier};

fn validate_password(input: &str, hash: &str) -> Result<bool, argon2::Error> {
    let parsed = PasswordHash::new(hash)?;
    Argon2::default().verify_password(input.as_bytes(), &parsed)
}
```

### Concurrent Access

```rust
// ❌ BAD - Data race potential
use std::sync::Mutex;

static COUNTER: Mutex<i32> = Mutex::new(0);

async fn increment() {
    let mut count = COUNTER.lock().unwrap();  // Blocks async runtime
    *count += 1;
}

// ✅ GOOD - Async-safe synchronization
use tokio::sync::Mutex;

static COUNTER: Mutex<i32> = Mutex::new(0);

async fn increment() {
    let mut count = COUNTER.lock().await;  // Async-aware
    *count += 1;
}

// ✅ ALSO GOOD - Use Arc for shared ownership
use std::sync::Arc;
use tokio::sync::RwLock;

struct AppState {
    config: Arc<RwLock<Config>>,
}

async fn update_config(state: &AppState, new_config: Config) -> Result<(), Error> {
    let mut config = state.config.write().await;
    *config = new_config;
    Ok(())
}
```

## Security Checklist

**Before deploying any code, verify**:

- [ ] **Input Validation**: All inputs are validated using `validator` or `SecurityValidator`
- [ ] **Unsafe Code**: All `unsafe` blocks are justified, documented, and minimal
- [ ] **Panic Safety**: No `unwrap()` or `expect()` in production code paths
- [ ] **Error Handling**: All errors use `Result` types and don't leak sensitive information
- [ ] **Memory Safety**: No data races, proper synchronization in concurrent code
- [ ] **Network Security**: Secure communication protocols, rate limiting implemented
- [ ] **Authentication**: Proper credential handling with timing-safe comparison
- [ ] **Authorization**: Access controls for all endpoints using `UserLevel`
- [ ] **XML Security**: XXE and XML bomb protection using `SecurityValidator`
- [ ] **ONVIF Compliance**: Proper service implementation and error codes
- [ ] **Security Testing**: Malicious input testing completed, no panics in production paths
