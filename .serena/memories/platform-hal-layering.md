# Platform-HAL Layering Architecture

> **DIRECTIONAL GUIDANCE**: This document defines the architectural boundary between the Platform layer and the Hardware Abstraction Layer (HAL). These are aspirational goals being actively refactored toward, not currently enforced everywhere.

## Current Reality

**⚠️ IMPORTANT**: The current codebase has known violations of these rules. This document serves as:
1. Directional guidance for new code
2. A tracking document for known violations being cleaned up
3. Not a strictly enforced boundary (yet)

See [Transitional Exceptions](#transitional-exceptions) for known violations.

## Architectural Principle

The project enforces a strict separation between **Platform** and **HAL** layers to ensure:
- **Memory safety**: All unsafe code isolated to HAL
- **Testability**: Platform layer can be tested without hardware
- **Portability**: HAL can be swapped for different hardware targets
- **Maintainability**: Business logic separated from hardware details

## Layer Definitions

### Platform Layer (`src/platform/`)

The Platform layer contains **business logic, policy, state management, and coordination**. It SHOULD be:

- ✅ **Primarily safe Rust** — Minimal `unsafe`, primarily for `unsafe impl Send/Sync` marker traits
- ✅ **Trait-based** — Depends on HAL traits only
- ✅ **Testable** — Can run with mock HAL implementations
- ✅ **Policy-driven** — Encodes camera business logic

**Examples of Platform responsibilities:**
- ONVIF service implementations (Device, Media, PTZ, Imaging)
- Stream configuration and profile management
- Authentication and authorization logic
- Network configuration and IP address management
- Application lifecycle management

**What Platform SHOULD NOT contain:**
- ⚠️ `unsafe` blocks (except marker trait implementations - see exceptions below)
- ❌ FFI declarations (`extern "C"`)
- ❌ Raw pointer casts (`as *mut`, `as *const`) in hot paths
- ❌ Direct hardware register access
- ❌ libc or raw system calls

#### Acceptable Exceptions in Platform

The following `unsafe` patterns ARE currently acceptable in platform:

1. **`unsafe impl Send/Sync`** — Marker trait implementations for types that are actually Send+Sync
   ```rust
   // platform/common/frame.rs - This is acceptable
   unsafe impl Send for Frame {}
   unsafe impl Sync for Frame {}
   ```

2. **FFI through vendor-daemon bridge** — Some platform code directly calls vendor-daemon IPC
   (being refactored to go through HAL)

### HAL Layer (`src/hal/`)

The HAL layer contains **hardware access, FFI, and low-level I/O**. It is the **only** place where:

- ✅ `unsafe` blocks are permitted
- ✅ FFI calls to vendor-daemon
- ✅ Raw pointer operations
- ✅ Unix socket communication
- ✅ Shared memory ring buffer access

**HAL structure:**
```
src/hal/
├── common/          # Platform-independent HAL traits
│   ├── mod.rs
│   ├── video.rs     # Video capture traits
│   ├── audio.rs     # Audio capture traits
│   ├── imaging.rs   # Imaging settings traits
│   ├── ptz.rs       # PTZ control traits
│   └── sdk_types.rs # SDK type definitions
│
├── anyka/           # Anyka AK3918 implementation
│   ├── mod.rs
│   ├── sdk.rs       # SDK FFI bindings
│   ├── ipc/         # Unix socket IPC client
│   │   ├── mod.rs
│   │   ├── video.rs
│   │   ├── audio.rs
│   │   ├── imaging.rs
│   │   └── shm_ring.rs  # Shared memory ring buffer
│   └── ptz/         # PTZ motor drivers
│
└── stub/            # Test stubs (no-op implementations)
    ├── mod.rs
    ├── video.rs
    ├── audio.rs
    ├── imaging.rs
    └── ptz.rs
```

## Transitional Exceptions

⚠️ **Known violations being actively refactored**. These will be addressed in future PRs:

| File | Violation | Status | Notes |
|------|-----------|--------|-------|
| `src/platform/common/frame.rs` | `unsafe impl Send/Sync` | **Acceptable** | Marker traits for frame types |
| `src/platform/anyka/video_encoder.rs` | `unsafe` blocks | In progress | Being moved to HAL |
| `src/platform/anyka/ptz_control.rs` | Direct IPC calls | Planned | Refactor to use HAL traits |

**Rationale for accepting `unsafe impl Send/Sync`**: These are marker traits that simply assert thread-safety properties. When the underlying type genuinely is Send+Sync (e.g., contains only Send+Sync fields), the implementation is sound. This pattern is common in safe Rust codebases and does not introduce unsoundness when used correctly.

## Directional Rules

### Rule 1: Platform Should Not Call FFI Directly

```rust
// ❌ WRONG - Platform calling FFI directly
// src/platform/anyka/video_input.rs
impl VideoInput for AnykaPlatform {
    async fn get_frame(&self) -> Result<Frame, PlatformError> {
        // Direct socket call in platform - FORBIDDEN
        let fd = unsafe { libc::socket(...) };  // VIOLATION
    }
}

// ✅ CORRECT - Platform delegates to HAL
// src/platform/anyka/video_input.rs
impl VideoInput for AnykaPlatform {
    async fn get_frame(&self) -> Result<Frame, PlatformError> {
        // Call HAL trait - platform stays pure
        self.hal.get_frame().await
    }
}
```

### Rule 2: Platform Should Use HAL Traits Only

```rust
// ✅ CORRECT - Platform depends on HAL trait
// src/platform/anyka/mod.rs
pub struct AnykaPlatform {
    hal: Arc<dyn hal::VideoCapture>,  // HAL trait, not implementation
}

// ❌ WRONG - Platform depends on HAL implementation
use crate::hal::anyka::AnykaVideoCapture;  // VIOLATION
pub struct AnykaPlatform {
    hal: AnykaVideoCapture,  // Concrete type, not trait
}
```

### Rule 3: HAL Exposes Safe Trait Interfaces

```rust
// ✅ CORRECT - HAL trait provides safe interface
// src/hal/common/video.rs
#[async_trait]
pub trait VideoCapture: Send + Sync {
    /// Capture a video frame from the hardware
    async fn capture_frame(&self, stream_id: StreamId) -> Result<VideoFrame, HalError>;
    
    /// Get current encoding parameters
    async fn get_encoding(&self) -> Result<EncodingParams, HalError>;
}

// Implementation in HAL (anyka/ or stub/)
// src/hal/anyka/ipc/video.rs
pub struct AnykaVideoCapture { /* unsafe code hidden */ }

#[async_trait]
impl VideoCapture for AnykaVideoCapture {
    async fn capture_frame(&self, stream_id: StreamId) -> Result<VideoFrame, HalError> {
        // HAL implementation can use unsafe internally
        unsafe { self.uncached_capture_frame(stream_id).await }
    }
}
```

### Rule 4: Raw Pointers Should Be Avoided in Platform

```rust
// ❌ WRONG - Raw pointer in platform
// src/platform/anyka/some_file.rs
fn process_buffer(ptr: *const u8, len: usize) {  // VIOLATION
    // Platform should never see raw pointers
}

// ✅ CORRECT - Safe slice in platform
// src/platform/anyka/some_file.rs
fn process_buffer(data: &[u8]) {  // Safe Rust
    // Platform works with safe references
}
```

### Rule 5: IPC Should Live in HAL

```rust
// ❌ WRONG - Unix socket in platform
// src/platform/anyka/network_info.rs
fn get_ip() -> std::io::Result<String> {
    use std::os::unix::net::UnixStream;
    let _socket = UnixStream::connect("/tmp/vd-ctrl.sock"?);  // VIOLATION
}

// ✅ CORRECT - IPC in HAL
// src/hal/anyka/ipc/mod.rs
pub struct VendorIpc {
    // Unix socket communication lives here
}
```

## Dependency Graph

```
┌─────────────────────────────────────────────────────────────────┐
│                         ONVIF Services                          │
│        (src/onvif/*/service.rs, src/onvif/device/ops/*)         │
└─────────────────────────────┬───────────────────────────────────┘
                              │ depends on
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Platform Layer                              │
│                 (src/platform/anyka/*.rs)                       │
│                                                                  │
│  - Pure safe Rust                                               │
│  - Business logic and state                                    │
│  - Depends on HAL traits only                                   │
│  - NO unsafe, FFI, or raw pointers                            │
└─────────────────────────────┬───────────────────────────────────┘
                              │ implements traits from
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                        HAL Traits                                │
│                    (src/hal/common/*.rs)                         │
│                                                                  │
│  - Safe trait definitions                                      │
│  - Hardware capability abstraction                            │
└─────────────────────────────┬───────────────────────────────────┘
                              │ implemented by
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    HAL Implementations                          │
│              (src/hal/anyka/*.rs, src/hal/stub/*.rs)           │
│                                                                  │
│  - ANYKA: Real hardware (unsafe allowed)                       │
│  - STUB:  Test implementations (no-op)                         │
│  - FFI to vendor-daemon                                         │
│  - Shared memory IPC                                           │
│  - Unix socket communication                                   │
└─────────────────────────────────────────────────────────────────┘
```

## Testing Strategy

### Platform Tests with Stub HAL

```rust
// src/platform/anyka/tests/video_input_tests.rs
use crate::hal::stub::StubVideoCapture;

#[tokio::test]
async fn test_platform_with_stub_hal() {
    // Platform can be tested with stub HAL - no unsafe needed
    let stub_hal = Arc::new(StubVideoCapture::new());
    let platform = AnykaPlatform::new(stub_hal);
    
    let frame = platform.get_frame().await;
    assert!(frame.is_ok());
}
```

### HAL Tests with Real Implementation

```rust
// src/hal/anyka/ipc/tests/mod.rs
#[tokio::test]
async fn test_ipc_socket_communication() {
    // HAL tests can use unsafe - they're testing hardware
    let ipc = VendorIpc::new().await;
    unsafe { ipc.send_command(Cmd::GetDeviceInfo).await }
}
```

## Verification Commands

> ⚠️ These commands help identify deviations from the target architecture. Some violations are expected (see Transitional Exceptions above).

```bash
# Check Platform for unsafe blocks (excluding unsafe impl Send/Sync)
grep -rn "unsafe {" cross-compile/onvif-rust/src/platform/ || echo "✅ No unsafe blocks in platform"

# Check for unsafe impl Send/Sync (acceptable in platform)
grep -rn "unsafe impl" cross-compile/onvif-rust/src/platform/ || echo "✅ No unsafe impl in platform"

# Check Platform has no FFI declarations  
grep -r 'extern "C"' cross-compile/onvif-rust/src/platform/ || echo "✅ PASS: No FFI in platform"

# Check Platform has no raw pointer casts
grep -r "as \*mut\|as \*const" cross-compile/onvif-rust/src/platform/ || echo "✅ PASS: No raw pointers in platform"

# Check Platform has no libc calls
grep -r "libc::" cross-compile/onvif-rust/src/platform/ || echo "✅ PASS: No libc in platform"

# Verify HAL contains the implementation
ls cross-compile/onvif-rust/src/hal/anyka/ipc/
```

## Common Violations and Fixes

| Violation | Location | Status | Notes |
|-----------|----------|--------|-------|
| `unsafe impl Send/Sync` | `src/platform/common/frame.rs` | **Acceptable** | Marker trait implementations |
| `unsafe` blocks | `src/platform/anyka/video_encoder.rs` | In progress | Being moved to HAL |
| Direct IPC calls | `src/platform/anyka/ptz_control.rs` | Planned | Refactor to use HAL traits |

## Enforcement

This architecture is **directional guidance**. Current status:

- ❌ NOT strictly enforced at code review time
- ✅ New code SHOULD follow these guidelines
- ✅ Known violations are tracked in Transitional Exceptions above
- ✅ Refactoring PRs welcome to close the gap

---

**Related Documentation:**
- [Development Standards](development-standards.md) - General coding rules
- [Project Context](project-context.md) - Architecture overview
- [Review Prompt](review-prompt.md) - Code review checklist
