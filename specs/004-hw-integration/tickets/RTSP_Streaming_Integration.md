# RTSP Streaming Integration

## Overview

Integrate RTSP server from streaming-lib with the platform layer using bounded send queues and zero-copy frame delivery.

## Scope

**In Scope:**
- Create `src/streaming/rtsp.rs`
- Implement `RtspFrameCallback` struct:
  - Frame callback implementation
  - Pre-allocated RTP packet pool (16 × 64KB = 1MB)
  - Bounded send queue (16 frames max)
  - Oldest-frame eviction strategy
  - Fast H.264 packetization (< 1ms)
- Implement async send task for network I/O
- Register callbacks with platform layer (Main + Sub streams)
- RTSP server initialization (port 554)
- Client limit enforcement (4 max)
- Integration tests (RTSP session establishment)

**Out of Scope:**
- Network buffer pool (T6)
- HTTP-FLV integration (T11)
- Main entry point (T13)
- Memory monitoring (T14)

## Technical Details

**Bounded Queue with Eviction:**
```rust
if let Err(packet) = self.send_queue.try_send(packet_buffer) {
    // Evict oldest frame
    if let Ok(old_packet) = rx.try_recv() {
        drop(old_packet);  // Return to pool
    }
    self.send_queue.try_send(packet).ok();
}
```

**Timing Breakdown:**
- Packetization: ~1ms (H.264 NAL parsing + RTP fragmentation)
- Queue send: ~0.1ms
- Total: < 2ms (within relaxed constraint)

## Spec References

- spec:6f9ed714-572c-4c78-a67e-e1e363752b44/6f620a2e-4e66-4b20-b1d1-cd99217bdcba - Section 3.3 (Streaming Layer), RTSP callback
- spec:6f9ed714-572c-4c78-a67e-e1e363752b44/3339e6c7-c72c-49ba-8fda-bb6a8ce6150b - Flow 4 (RTSP streaming)

## Dependencies

- T5: streaming-lib Creation (needs RtspServer)
- T6: Buffer Pool Infrastructure (needs RTP packet pool)
- T7: Platform Video Input (needs video input)
- T8a: Platform Video Encoder (needs video encoder)
- T8b: Platform Frame Callbacks (needs frame callback infrastructure)

## Acceptance Criteria

- ✅ RTSP server starts on port 554
- ✅ Frame callbacks work with bounded queues
- ✅ Oldest-frame eviction prevents memory explosion
- ✅ RTP packet pool (1MB) works correctly
- ✅ Callback timing < 2ms
- ✅ 4 concurrent client limit enforced
- ✅ Integration tests pass (RTSP DESCRIBE, SETUP, PLAY)
- ✅ Latency < 100ms end-to-end
