# HTTP-FLV Streaming Integration

## Overview

Integrate HTTP-FLV server from streaming-lib with the platform layer using zero-copy FLV tags and extended frame lifetime.

## Scope

**In Scope:**
- Create `src/streaming/httpflv.rs`
- Implement `HttpFlvFrameCallback` struct:
  - Frame callback implementation
  - Pre-allocated FLV tag pool (16 × 1KB = 16KB, headers only)
  - Zero-copy FLV tags (reference frame data, don't copy)
  - Extended frame lifetime management
  - Bounded send queue (16 frames max)
  - Oldest-frame eviction strategy
  - Fast FLV tag header creation (< 0.05ms)
- Implement async send task for network I/O
- Register callbacks with platform layer (Main + Sub streams)
- HTTP-FLV server initialization (port 8080)
- Client limit enforcement (4 max)
- Integration tests (HTTP-FLV streaming)

**Out of Scope:**
- Network buffer pool (T6)
- RTSP integration (T10)
- Main entry point (T13)
- Memory monitoring (T14)

## Technical Details

**Zero-Copy FLV Tags:**
```rust
struct FlvTagHandle {
    header: [u8; 16],           // FLV tag header only
    frame_ref: *const u8,       // Zero-copy reference
    _frame_handle: FrameHandle, // Extends SDK buffer lifetime
}
```

**Extended Lifetime:**
- Platform acquires frame reference before callbacks
- FLV tag holds reference (not copy)
- SDK buffer freed only after send completes
- Safe zero-copy without use-after-free

**Timing Breakdown:**
- Header creation: ~0.05ms (no frame copying)
- Queue send: ~0.1ms
- Total: < 0.2ms (well within constraint)

## Spec References

- spec:6f9ed714-572c-4c78-a67e-e1e363752b44/6f620a2e-4e66-4b20-b1d1-cd99217bdcba - Section 3.3 (Streaming Layer), HTTP-FLV callback, Section 1.3 (Frame Lifetime)
- spec:6f9ed714-572c-4c78-a67e-e1e363752b44/3339e6c7-c72c-49ba-8fda-bb6a8ce6150b - Flow 5 (HTTP-FLV streaming)

## Dependencies

- T5: streaming-lib Creation (needs HttpFlvServer)
- T6: Buffer Pool Infrastructure (needs FLV tag pool)
- T7: Platform Video Input (needs video input)
- T8a: Platform Video Encoder (needs video encoder)
- T8b: Platform Frame Callbacks (needs frame callbacks and extended lifetime)

## Acceptance Criteria

- ✅ HTTP-FLV server starts on port 8080
- ✅ Zero-copy FLV tags implemented (16KB pool)
- ✅ Extended frame lifetime prevents use-after-free
- ✅ Bounded send queue with eviction works
- ✅ Callback timing < 0.2ms
- ✅ 4 concurrent client limit enforced
- ✅ Integration tests pass (HTTP GET /live.flv)
- ✅ Browser streaming works (Chrome, Firefox, Safari, Edge)
- ✅ Latency < 3 seconds
