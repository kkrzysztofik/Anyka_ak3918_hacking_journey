# [SPLIT] Platform Layer - Media Encoder & Frame Callbacks → T8a + T8b

## ⚠️ THIS TICKET HAS BEEN SPLIT

This ticket was too complex and has been split into two focused tickets:

- **T8a: Platform Layer - Video Encoder Implementation** - Implements VideoEncoder trait with dual encoders
- **T8b: Platform Layer - Frame Callback Infrastructure** - Implements unified frame callbacks, extended lifetime, panic isolation

**Please work on T8a and T8b instead of this ticket.**

---

## Original Overview (For Reference)

Implement the VideoEncoder and AudioEncoder traits for Anyka AK3918 hardware with dual video encoders (1080p Main + 720p Sub), AAC audio encoder, and unified frame callback interface for both video and audio frames.

## Scope

**In Scope:**
- Implement `AnykaVideoEncoder` struct in `src/platform/anyka.rs`
- Implement `VideoEncoder` trait methods:
  - `init()` - Configure dual encoders (Main: 1080p@25fps, Sub: 720p@30fps)
  - `start()` - Start encoding
  - `stop()` - Stop encoding
  - `request_idr()` - Force I-frame generation
  - `set_bitrate()` - Adjust bitrate dynamically
- Implement unified frame callback registration (video + audio):
  - `register_frame_callback()` - Register callback for frame delivery
  - `unregister_frame_callback()` - Remove callback
  - Support both video frames (I/P/B) and audio packets
- Implement zero-copy frame delivery (read-only pointers)
- Implement extended frame lifetime mechanism:
  - `FrameHandle` struct with reference counting
  - `acquire_frame_ref()` method to extend SDK buffer lifetime
  - Safe zero-copy for FLV tags (buffer freed after send completes)
- Implement panic isolation (catch_unwind for callbacks)
- Integrate with FFI layer (`ffi::video` and `ffi::audio`)
- Unit tests with mockall

**Out of Scope:**
- Video input implementation (T5)
- Audio input implementation (T7)
- Audio encoder configuration (T7 handles AAC config)
- Streaming integration (T10, T11)
- Buffer pools (T6)

## Technical Details

**Dual Encoder Configuration:**
```rust
// Main encoder: 1080p@25fps, 4Mbps
let main_config = VideoEncoderConfig {
    resolution: Resolution::new(1920, 1080),
    framerate: 25,
    bitrate: 4000,
    encoding: VideoEncoding::H264,
};

// Sub encoder: 720p@30fps, 2Mbps
let sub_config = VideoEncoderConfig {
    resolution: Resolution::new(1280, 720),
    framerate: 30,
    bitrate: 2000,
    encoding: VideoEncoding::H264,
};
```

**Frame Callback Invocation:**
- Synchronous callbacks (< 2ms requirement)
- Zero-copy (read-only frame pointers)
- Panic isolation with `std::panic::catch_unwind`
- Unified mechanism for video and audio frames

**Extended Frame Lifetime:**
```rust
struct FrameHandle {
    sdk_buffer: *const u8,
    ref_count: Arc<AtomicUsize>,
}

impl Drop for FrameHandle {
    fn drop(&mut self) {
        if self.ref_count.fetch_sub(1, Ordering::Release) == 1 {
            unsafe { ak_venc_release_frame(self.sdk_buffer); }
        }
    }
}
```

## Spec References

- spec:6f9ed714-572c-4c78-a67e-e1e363752b44/6f620a2e-4e66-4b20-b1d1-cd99217bdcba - Section 3.1 (Platform Layer), Section 2.1 (Frame Delivery)
- spec:6f9ed714-572c-4c78-a67e-e1e363752b44/3339e6c7-c72c-49ba-8fda-bb6a8ce6150b - Flow 8 (frame delivery interface)

## Dependencies

- T2: FFI Video/Audio Wrappers (needs video and audio FFI layer)
- T7: Platform Audio (needs audio encoder for audio frame callbacks)

## Acceptance Criteria

- ✅ VideoEncoder trait fully implemented
- ✅ Dual video encoders configured (1080p + 720p)
- ✅ Frame callback registration works (video + audio)
- ✅ Zero-copy frame delivery implemented
- ✅ Extended frame lifetime (FrameHandle) implemented
- ✅ Audio frame callbacks integrated
- ✅ Panic isolation prevents callback crashes
- ✅ Callbacks invoked within timing constraint (< 2ms)
- ✅ Unit tests pass (100% with mocks)
- ✅ `cargo clippy` passes
