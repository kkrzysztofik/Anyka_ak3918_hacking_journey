# Platform Layer - Video Encoder Implementation

## Overview

Implement the VideoEncoder trait for Anyka AK3918 hardware with dual video encoders (1080p Main + 720p Sub) for simultaneous high-quality and bandwidth-optimized streaming.

## Scope

**In Scope:**
- Implement `AnykaVideoEncoder` struct in `src/platform/anyka.rs`
- Implement `VideoEncoder` trait methods:
  - `init()` - Configure dual encoders (Main: 1080p@25fps, Sub: 720p@30fps)
  - `start()` - Start encoding on both channels
  - `stop()` - Stop encoding on both channels
  - `request_idr()` - Force I-frame generation
  - `set_bitrate()` - Adjust bitrate dynamically
  - `get_config()` - Query current encoder configuration
- Dual encoder configuration using Anyka SDK video channels:
  - VIDEO_CHN_MAIN: 1920x1080@25fps, 4Mbps, H.264
  - VIDEO_CHN_SUB: 1280x720@30fps, 2Mbps, H.264
- Integrate with FFI layer (`ffi::video`)
- Error handling and resource cleanup
- Unit tests with mockall

**Out of Scope:**
- Video input implementation (T7)
- Frame callback registration (T8b)
- Audio encoder (T9)
- Streaming integration (T12, T13)

## Technical Details

**Dual Encoder Configuration:**
```rust
// Main encoder: 1080p@25fps, 4Mbps
let main_config = VideoEncoderConfig {
    channel: VideoChannel::Main,
    resolution: Resolution::new(1920, 1080),
    framerate: 25,
    bitrate: 4000,
    gop_size: 50,
    encoding: VideoEncoding::H264,
};

// Sub encoder: 720p@30fps, 2Mbps
let sub_config = VideoEncoderConfig {
    channel: VideoChannel::Sub,
    resolution: Resolution::new(1280, 720),
    framerate: 30,
    bitrate: 2000,
    gop_size: 60,
    encoding: VideoEncoding::H264,
};
```

**SDK Integration:**
- Use `ak_venc_open()` for each channel
- Configure rate control with `ak_venc_set_rc()`
- Start encoding with `ak_venc_start()`
- Request I-frames with `ak_venc_request_idr()`

**Memory Footprint:**
- Target: 6-8MB for both encoders
- SDK manages internal buffers
- Platform layer provides configuration only

## Spec References

- spec:6f9ed714-572c-4c78-a67e-e1e363752b44/6f620a2e-4e66-4b20-b1d1-cd99217bdcba - Section 3.1 (Platform Layer), video encoder
- spec:6f9ed714-572c-4c78-a67e-e1e363752b44/3339e6c7-c72c-49ba-8fda-bb6a8ce6150b - Flow 1 (initialization)

## Dependencies

- T3: FFI Video/Audio Wrappers (needs video FFI layer)
- T7: Platform Video Input (needs video input initialized)

## Acceptance Criteria

- ✅ VideoEncoder trait fully implemented
- ✅ Dual video encoders configured (1080p + 720p)
- ✅ Both encoders start/stop correctly
- ✅ I-frame requests work (verified with frame inspection)
- ✅ Bitrate adjustment works dynamically
- ✅ Error handling propagates SDK errors
- ✅ Resource cleanup on drop (no leaks)
- ✅ Memory footprint ≤ 8MB (measured)
- ✅ Unit tests pass (100% with mocks)
- ✅ `cargo clippy` passes with zero warnings