# Platform Layer - Audio Implementation

## Overview

Implement AudioInput and AudioEncoder traits for Anyka AK3918 hardware with AAC encoding.

## Scope

**In Scope:**
- Implement `AnykaAudioInput` struct in `src/platform/anyka.rs`
- Implement `AudioInput` trait methods:
  - `open()` - Initialize audio input (microphone)
  - `close()` - Release audio input
  - `set_volume()` - Adjust input volume
  - `get_sample_rate()` - Query sample rate
- Implement `AnykaAudioEncoder` struct
- Implement `AudioEncoder` trait methods:
  - `init()` - Configure AAC encoder (512KB footprint)
  - `start()` - Start encoding
  - `stop()` - Stop encoding
  - `set_config()` - Adjust encoder parameters
- Integrate with FFI layer (`ffi::audio`)
- Error handling and resource cleanup
- Unit tests with mockall

**Out of Scope:**
- Video implementation (T5, T6)
- Frame callback registration (T6 handles unified callbacks)
- PTZ/Imaging implementation (T8, T9)
- Streaming integration (T10, T11)
- Buffer pools (T6)

## Technical Details

**AAC Encoder Configuration:**
```rust
let audio_config = AudioEncoderConfig {
    codec: AudioCodec::AAC,
    sample_rate: 16000,
    channels: 1,
    bitrate: 64,
};
```

**Memory Footprint:**
- Target: 512KB (as per memory budget)
- AAC encoder state + buffers

## Spec References

- spec:6f9ed714-572c-4c78-a67e-e1e363752b44/6f620a2e-4e66-4b20-b1d1-cd99217bdcba - Section 3.1 (Platform Layer), audio implementation
- spec:6f9ed714-572c-4c78-a67e-e1e363752b44/3339e6c7-c72c-49ba-8fda-bb6a8ce6150b - Flow 1 (initialization)
- spec:6f9ed714-572c-4c78-a67e-e1e363752b44/b45347e9-3eee-42d0-9e77-2c8cfa54db6f - AAC-only audio constraint

## Dependencies

- T2: FFI Video/Audio Wrappers (needs audio FFI layer)

**Note:** T6 (Media Encoder & Frame Callbacks) depends on T7 for audio encoder integration.

## Acceptance Criteria

- ✅ AudioInput trait fully implemented
- ✅ AudioEncoder trait fully implemented
- ✅ AAC encoding configured correctly
- ✅ Memory footprint ≤ 512KB
- ✅ Error handling propagates SDK errors
- ✅ Resource cleanup on drop
- ✅ Unit tests pass (100% with mocks)
- ✅ `cargo clippy` passes
