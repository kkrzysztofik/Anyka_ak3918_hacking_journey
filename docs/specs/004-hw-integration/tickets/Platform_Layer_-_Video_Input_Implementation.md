# Platform Layer - Video Input Implementation

## Overview

Implement the VideoInput trait for Anyka AK3918 hardware, providing camera sensor access and configuration.

## Scope

**In Scope:**
- Implement `AnykaVideoInput` struct in `src/platform/anyka.rs`
- Implement `VideoInput` trait methods:
  - `open()` - Initialize video input device
  - `close()` - Release video input device
  - `get_resolution()` - Query sensor resolution
  - `set_channel_attr()` - Configure video channels
- Integrate with FFI layer (`ffi::video`)
- Error handling and resource cleanup
- Unit tests with mockall

**Out of Scope:**
- Video encoder implementation (T8a)
- Audio implementation (T9)
- Frame callback registration (T8b)
- Streaming integration (T12)

## Technical Details

**Video Input Initialization:**
```rust
impl VideoInput for AnykaVideoInput {
    async fn open(&self) -> PlatformResult<()> {
        let handle = ffi::video::vi_open()?;
        self.handle.store(handle);
        Ok(())
    }
}
```

**Channel Configuration:**
- Main channel: 1920x1080
- Sub channel: 1280x720
- Uses SDK's VIDEO_CHN_MAIN, VIDEO_CHN_SUB

## Spec References

- spec:6f9ed714-572c-4c78-a67e-e1e363752b44/6f620a2e-4e66-4b20-b1d1-cd99217bdcba - Section 3.1 (Platform Layer)
- spec:6f9ed714-572c-4c78-a67e-e1e363752b44/3339e6c7-c72c-49ba-8fda-bb6a8ce6150b - Flow 1 (initialization)

## Dependencies

- T2: FFI Video/Audio Wrappers (needs video FFI layer)

## Acceptance Criteria

- ✅ VideoInput trait fully implemented
- ✅ Dual channel configuration works (Main + Sub)
- ✅ Error handling propagates SDK errors
- ✅ Resource cleanup on drop
- ✅ Unit tests pass (100% with mocks)
- ✅ `cargo clippy` passes
- ✅ Integration with FFI layer verified
