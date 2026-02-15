# Platform Layer - Frame Callback Infrastructure

## Overview

Implement unified frame callback infrastructure for zero-copy frame delivery from video and audio encoders to streaming servers, with extended frame lifetime management and panic isolation.

## Scope

**In Scope:**
- Implement unified frame callback registration in `src/platform/anyka.rs`:
  - `register_frame_callback()` - Register callback for frame delivery
  - `unregister_frame_callback()` - Remove callback
  - Support both video frames (I/P/B) and audio packets
  - Support multiple callbacks per stream (RTSP + HTTP-FLV)
- Implement zero-copy frame delivery:
  - Read-only frame pointers (no memcpy)
  - Frame data remains in SDK buffers
  - Callbacks receive `&Frame` reference
- Implement extended frame lifetime mechanism:
  - `FrameHandle` struct with Arc-based reference counting
  - `acquire_frame_ref()` method to extend SDK buffer lifetime
  - SDK buffer freed only when all references dropped
  - Safe zero-copy for FLV tags (buffer lives until send completes)
- Implement panic isolation:
  - Wrap callback invocation with `std::panic::catch_unwind`
  - Log panic with callback ID and error message
  - Unregister panicked callback automatically
  - Continue invoking other callbacks
- Callback timing instrumentation:
  - Measure callback duration
  - Log warning if callback exceeds 2ms
  - Expose metrics for monitoring
- Integrate with video encoder (T8a) and audio encoder (T9)
- Unit tests with mockall (including panic scenarios)

**Out of Scope:**
- Video encoder implementation (T8a)
- Audio encoder implementation (T9)
- Streaming server implementation (T12, T13)
- Buffer pools (T6)

## Technical Details

**Frame Callback Interface:**
```rust
pub trait FrameCallback: Send + Sync {
    fn on_frame(&self, frame: &Frame);
}

pub struct Frame {
    pub data: *const u8,      // Read-only pointer (zero-copy)
    pub size: usize,
    pub timestamp: u64,       // Microseconds since epoch
    pub frame_type: FrameType,
    pub stream_id: StreamId,
}
```

**Extended Frame Lifetime:**
```rust
pub struct FrameHandle {
    sdk_buffer: *const u8,
    ref_count: Arc<AtomicUsize>,
}

impl Drop for FrameHandle {
    fn drop(&mut self) {
        if self.ref_count.fetch_sub(1, Ordering::Release) == 1 {
            // Last reference - release SDK buffer
            unsafe { ak_venc_release_frame(self.sdk_buffer); }
        }
    }
}

// Platform maintains active frames
struct ActiveFrames {
    frames: HashMap<*const u8, Arc<AtomicUsize>>,
}
```

**Panic Isolation:**
```rust
fn invoke_callbacks(&self, frame: &Frame) {
    for (id, callback) in &self.callbacks {
        let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
            callback.on_frame(frame);
        }));
        
        if result.is_err() {
            error!("Callback {} panicked, unregistering", id);
            self.unregister_callback(id);
        }
    }
}
```

**Timing Instrumentation:**
```rust
let start = Instant::now();
callback.on_frame(frame);
let duration = start.elapsed();

if duration > Duration::from_millis(2) {
    warn!("Callback {} took {:?} (> 2ms)", id, duration);
}
```

## Spec References

- spec:6f9ed714-572c-4c78-a67e-e1e363752b44/6f620a2e-4e66-4b20-b1d1-cd99217bdcba - Section 2.1 (Frame Delivery), Section 1.3 (Extended Lifetime)
- spec:6f9ed714-572c-4c78-a67e-e1e363752b44/3339e6c7-c72c-49ba-8fda-bb6a8ce6150b - Flow 8 (frame delivery interface)

## Dependencies

- T8a: Platform Video Encoder (needs video encoder producing frames)
- T9: Platform Audio (needs audio encoder producing frames)

## Acceptance Criteria

- ✅ Frame callback registration works (video + audio)
- ✅ Multiple callbacks per stream supported (RTSP + HTTP-FLV)
- ✅ Zero-copy frame delivery implemented (no memcpy)
- ✅ Extended frame lifetime (FrameHandle) implemented
- ✅ FrameHandle prevents use-after-free (validated with msan)
- ✅ Panic isolation tested (callback panic doesn't crash system)
- ✅ Panicked callbacks automatically unregistered
- ✅ Callback timing measured and logged
- ✅ Callbacks invoked within timing constraint (< 2ms average)
- ✅ Integration with T8a and T9 validated
- ✅ Unit tests pass (100% with mocks, including panic scenarios)
- ✅ `cargo clippy` passes with zero warnings