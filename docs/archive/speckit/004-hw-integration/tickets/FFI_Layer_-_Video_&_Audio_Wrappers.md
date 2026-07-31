# FFI Layer - Video & Audio Wrappers

## Overview

Implement safe Rust wrappers around Anyka SDK video and audio functions using the RAII pattern for automatic resource management.

## Scope

**In Scope:**
- Create `src/ffi/video.rs` with safe wrappers:
  - Video Input: `ak_vi_open()`, `ak_vi_close()`, `ak_vi_get_sensor_resolution()`, `ak_vi_set_channel_attr()`
  - Video Encoder: `ak_venc_open()`, `ak_venc_close()`, `ak_venc_set_rc()`, `ak_venc_request_idr()`
  - RAII handles: `VideoInputHandle`, `VideoEncoderHandle`
- Create `src/ffi/audio.rs` with safe wrappers:
  - Audio Input: `ak_ai_open()`, `ak_ai_close()`, `ak_ai_set_volume()`
  - Audio Encoder: `ak_aenc_open()`, `ak_aenc_close()`, `ak_aenc_set_config()`
  - RAII handles: `AudioInputHandle`, `AudioEncoderHandle`
- Error code conversion: `AK_SUCCESS`/`AK_FAILED` → `Result<T, PlatformError>`
- Null pointer safety checks
- Unit tests with mockall for each wrapper

**Out of Scope:**
- PTZ/Imaging wrappers (T3)
- Platform trait implementations (T5, T6, T7)
- Hardware testing (T13)

## Technical Details

**RAII Pattern Example:**
```rust
pub struct VideoEncoderHandle {
    handle: i32,
}

impl Drop for VideoEncoderHandle {
    fn drop(&mut self) {
        unsafe { ak_venc_close(self.handle); }
    }
}
```

**Error Conversion:**
```rust
fn check_result(ret: i32) -> PlatformResult<()> {
    match ret {
        AK_SUCCESS => Ok(()),
        _ => Err(PlatformError::SdkError(ret)),
    }
}
```

## Spec References

- spec:6f9ed714-572c-4c78-a67e-e1e363752b44/6f620a2e-4e66-4b20-b1d1-cd99217bdcba - Section 3.2 (FFI Layer), video/audio examples
- spec:6f9ed714-572c-4c78-a67e-e1e363752b44/3339e6c7-c72c-49ba-8fda-bb6a8ce6150b - Flow 1 (hardware initialization)

## Dependencies

- T1: Workspace & Vendor Setup (needs headers/libs)

## Acceptance Criteria

- ✅ All video FFI wrappers implemented with RAII
- ✅ All audio FFI wrappers implemented with RAII
- ✅ Error handling converts SDK errors properly
- ✅ Null pointer checks in place
- ✅ Unit tests pass (100% success with mocks)
- ✅ `cargo clippy` passes with zero warnings
- ✅ `cargo doc` generates documentation
- ✅ No unsafe code outside FFI boundary