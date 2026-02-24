# Approach B: Optimized Unix Socket IPC — Reduce Frame Copy & Allocation Overhead

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Date**: 2026-02-24
**Status**: Draft (Ready for implementation)
**Author**: Kilo (AI agent)

## Summary

Incrementally optimize the existing Unix domain socket IPC path between
`vendor-daemon` (C) and `onvif-rust` (Rust) to eliminate the double frame copy,
reduce per-frame heap allocations, and remove Mutex contention from the hot path.
No C-side daemon changes required for Phases 1-2. Minimal daemon changes for Phase 3.

## Target Device Profile

| Property | Value |
|----------|-------|
| **CPU** | ARM926EJ-S rev 5 (v5l) @ ~200MHz |
| **BogoMIPS** | 199.06 |
| **ISA Features** | swp, half, fastmult, edsp, java (NO NEON, NO VFP) |
| **RAM Total** | 36,540 KB |
| **RAM Free** | ~4,376 KB (19,580 KB cached, reclaimable) |
| **Kernel** | Linux 3.4.35 |
| **Page Size** | 4096 bytes |
| **libc** | uClibc |

## Current Architecture & Bottleneck Analysis

### Frame Data Path (Current — 2 copies)

```
vendor-daemon (C)
  │
  │ SDK DMA buffer
  │  → write(unix_socket, header+frame_data)    [kernel: 1 copy to socket buffer]
  ▼
onvif-rust / VendorIpc::venc_get_stream()
  │
  │ 1. read_exact() → Vec<u8>                   [COPY 1: kernel→userspace alloc]
  │    vendor_ipc.rs:506  → vec![0u8; resp_len]  (heap alloc)
  │    vendor_ipc.rs:935  → data[28..].to_vec()   (COPY 1a: slice→new Vec)
  │
  │ 2. Store in pending_frames HashMap            (HashMap insert)
  │ 3. Set video_stream.data pointer
  ▼
anyka.rs / unified_frame_read_loop()
  │
  │ 4. Frame { data: stream.data, size: stream.len }
  │    (pointer to VendorIpc's pending_frames Vec)
  ▼
StreamingBridge::route_frame()
  │
  │ 5. BytesMut::from(slice::from_raw_parts(...)) [COPY 2: Vec→BytesMut]
  │    bridge.rs:261
  │
  │ 6. Push to LowLatencyFrameQueue
  ▼
VendorIpc::venc_release_stream()
  │
  │ 7. HashMap remove (drop Vec<u8>)              (heap free)
  │ 8. Send release command to daemon
  ▼
```

### Bottleneck Quantification (ARM926EJ-S @ 200MHz)

| Source | Per-Frame Cost | At 50fps (main+sub) | CPU % |
|--------|---------------|---------------------|-------|
| **Copy 1** (socket→Vec) | ~25μs (50KB) | 1.25ms/s | 0.13% |
| **Copy 1a** (slice→to_vec) | ~25μs (50KB) | 1.25ms/s | 0.13% |
| **Copy 2** (Vec→BytesMut) | ~25μs (50KB) | 1.25ms/s | 0.13% |
| **Vec alloc** (resp_data) | ~3μs | 150μs/s | 0.015% |
| **Vec alloc** (frame_data) | ~3μs | 150μs/s | 0.015% |
| **HashMap insert/remove** | ~2μs | 100μs/s | 0.01% |
| **Mutex acquire** (stream) | ~1-5μs | 50-250μs/s | 0.005-0.025% |
| **Mutex acquire** (pending) | ~1-5μs | 50-250μs/s | 0.005-0.025% |
| **Syscalls** (~10/frame) | ~75μs | 3.75ms/s | 0.375% |
| **Total** | ~160μs | ~8ms/s | **~0.8%** |

Note: These are conservative estimates. On ARM926EJ-S without NEON, memcpy
throughput is ~150-200 MB/s. With cache misses on large frames, the actual
copy cost can be 2-3× higher. **Realistic total: 1.5-2.5% CPU.**

### Key Insight: Copy 1a and Copy 2 Are Eliminable

- **Copy 1** (kernel→userspace) is inherent to socket I/O — cannot be eliminated
  without shared memory (Approach A)
- **Copy 1a** (`data[28..].to_vec()`) — can be eliminated by reading the frame
  directly into the final buffer
- **Copy 2** (`BytesMut::from(...)` in bridge) — can be eliminated by passing
  `BytesMut` all the way from the IPC layer instead of raw `*const u8`

---

## Design: Phased Optimization

### Phase 1: Eliminate Double-Copy via Direct BytesMut Receive

**Goal**: Read frame data directly into `BytesMut` in `venc_get_stream`, then pass
the `BytesMut` through to `StreamingBridge` without a second copy.

**Impact**: Eliminates Copy 1a + Copy 2 = ~50% of copy overhead.
**Effort**: 2-3 days. **Risk**: Low.

#### 1.1 New Frame Container: `OwnedFrame`

Currently, the `Frame` struct holds a raw `*const u8` pointer that requires
the consumer to copy the data. Replace with an owned container that can carry
`BytesMut` through the pipeline.

```rust
// New: src/platform/frame.rs

/// Frame with owned data buffer — no copy needed downstream.
pub struct OwnedFrame {
    /// Owned encoded frame data (already in BytesMut).
    pub data: BytesMut,
    /// Timestamp in microseconds since epoch.
    pub timestamp: u64,
    /// Type of frame (I-frame, P-frame, etc.).
    pub frame_type: FrameType,
    /// Which stream this frame belongs to.
    pub stream_id: StreamId,
}

/// Extended callback trait that receives owned frames.
pub trait OwnedFrameCallback: Send + Sync {
    fn on_owned_frame(&self, frame: OwnedFrame);
}
```

#### 1.2 VendorIpc: Read Frame Data Directly Into BytesMut

Replace the current two-allocation flow in `venc_get_stream`:

**Current** (vendor_ipc.rs:462-514, 877-980):
```rust
// Step 1: Read entire response into Vec<u8>
let mut resp_data = vec![0u8; resp_len];      // alloc
stream.read_exact(&mut resp_data)?;            // read

// Step 2: Copy frame portion into another Vec<u8>  
let frame_data = data[28..expected_total].to_vec();  // alloc + copy
```

**Proposed**: Split the receive into header + frame data, reading frame data
directly into `BytesMut`:

```rust
/// Receive a frame response: reads the 28-byte header, then reads frame data
/// directly into a BytesMut buffer.
fn recv_frame_response(
    stream: &mut UnixStream,
) -> PlatformResult<(FrameMetadata, BytesMut)> {
    // 1. Read fixed-size response header (status + len)
    let mut hdr = [0u8; 8];  // status(4) + resp_len(4)
    stream.read_exact(&mut hdr)?;
    let status = i32::from_le_bytes(hdr[0..4].try_into().unwrap());
    let resp_len = u32::from_le_bytes(hdr[4..8].try_into().unwrap()) as usize;

    if status != AK_SUCCESS_I32 {
        // Drain any remaining data
        if resp_len > 0 {
            let mut discard = vec![0u8; resp_len.min(256)];
            let _ = stream.read_exact(&mut discard);
        }
        return Err(PlatformError::HardwareFailure(
            format!("IPC frame request failed: status {}", status)
        ));
    }

    // 2. Read 28-byte frame header
    let mut frame_hdr = [0u8; VENC_STREAM_HEADER_LEN];
    stream.read_exact(&mut frame_hdr)?;

    let frame_len = u32::from_le_bytes(frame_hdr[0..4].try_into().unwrap()) as usize;
    let timestamp = u64::from_le_bytes(frame_hdr[4..12].try_into().unwrap());
    let seq_no = u32::from_le_bytes(frame_hdr[12..16].try_into().unwrap());
    let frame_type_val = i32::from_le_bytes(frame_hdr[16..20].try_into().unwrap());
    let remote_token = u64::from_le_bytes(frame_hdr[20..28].try_into().unwrap());

    // 3. Read frame data DIRECTLY into BytesMut — no intermediate Vec
    let mut frame_data = BytesMut::zeroed(frame_len);
    stream.read_exact(&mut frame_data)?;

    let metadata = FrameMetadata {
        timestamp,
        seq_no,
        frame_type: Self::ipc_to_frame_type(frame_type_val),
        remote_token,
    };

    Ok((metadata, frame_data))
}
```

#### 1.3 New Dedicated Frame Fetch Method

Add a higher-level method that returns `OwnedFrame` with `BytesMut` data:

```rust
impl VendorIpc {
    /// Fetch next encoded frame as an OwnedFrame with BytesMut data.
    ///
    /// This is the zero-extra-copy path: frame data is read from the socket
    /// directly into BytesMut, which can be passed through to the streaming
    /// pipeline without any additional copy.
    pub fn fetch_frame_owned(
        &self,
        stream_handle: *mut c_void,
    ) -> PlatformResult<Option<OwnedFrame>> {
        let handle_val = stream_handle as u64;
        let req_data = handle_val.to_le_bytes();

        let mut stream = self.stream.lock()
            .map_err(|e| PlatformError::HardwareFailure(
                format!("IPC mutex poisoned: {}", e)
            ))?;

        // Send request
        stream.write_all(&CMD_VENC_GET_STREAM.to_le_bytes())?;
        stream.write_all(&(8u32).to_le_bytes())?;  // req_len = 8
        stream.write_all(&req_data)?;
        stream.flush()?;

        // Receive frame directly into BytesMut
        let (metadata, frame_data) = Self::recv_frame_response(&mut *stream)?;

        // Store remote_token for release (keyed by stream_handle)
        // No need to store frame data — it's now owned by the caller
        self.pending_tokens.lock()
            .map_err(|e| PlatformError::HardwareFailure(
                format!("pending_tokens mutex poisoned: {}", e)
            ))?
            .insert(handle_val, metadata.remote_token);

        Ok(Some(OwnedFrame {
            data: frame_data,
            timestamp: metadata.timestamp.wrapping_mul(1000), // ms → μs
            frame_type: metadata.frame_type,
            stream_id: StreamId::VideoMain, // Set by caller
        }))
    }

    /// Release a frame (send remote_token back to daemon).
    pub fn release_frame_owned(
        &self,
        stream_handle: *mut c_void,
    ) -> PlatformResult<()> {
        let handle_val = stream_handle as u64;

        let remote_token = self.pending_tokens.lock()
            .map_err(|e| PlatformError::HardwareFailure(
                format!("pending_tokens mutex poisoned: {}", e)
            ))?
            .remove(&handle_val)
            .unwrap_or(0);

        let mut req_data = [0u8; 16];
        req_data[0..8].copy_from_slice(&handle_val.to_le_bytes());
        req_data[8..16].copy_from_slice(&remote_token.to_le_bytes());

        let (status, _) = self.send_request(CMD_VENC_RELEASE_STREAM, &req_data)?;
        if status != AK_SUCCESS_I32 {
            return Err(PlatformError::HardwareFailure(
                format!("Frame release failed: status {}", status)
            ));
        }
        Ok(())
    }
}
```

#### 1.4 Update unified_frame_read_loop

Replace the `venc_get_stream` + pointer-based `Frame` path with `fetch_frame_owned`:

```rust
// In drain_stream():

// OLD:
let mut stream = MaybeUninit::<video_stream>::uninit();
let ret = ffi.venc_get_stream(handle.as_ptr(), stream_ptr);
// ... unsafe pointer manipulation ...
let frame = Frame { data: stream_data.data as *const u8, ... };
invoke_callbacks_from_map(callbacks, &frame);
let _ = ffi.venc_release_stream(handle.as_ptr(), stream_data);

// NEW:
match ipc.fetch_frame_owned(handle.as_ptr()) {
    Ok(Some(owned_frame)) => {
        // Route directly — no copy needed in bridge
        invoke_owned_callbacks(callbacks, owned_frame);
        ipc.release_frame_owned(handle.as_ptr())?;
    }
    Ok(None) => { /* no data available */ }
    Err(e) => { /* error handling */ }
}
```

#### 1.5 Update StreamingBridge::route_frame

```rust
// OLD (bridge.rs:259-261):
fn route_frame(&self, frame: &Frame) {
    let data = BytesMut::from(unsafe {
        std::slice::from_raw_parts(frame.data, frame.size)
    }); // ← COPY 2 eliminated

// NEW:
fn route_owned_frame(&self, frame: OwnedFrame) {
    let data = frame.data;  // Move, no copy!
    let timestamp_ms = (frame.timestamp / 1000) as u32;
    // ... rest unchanged ...
}
```

#### 1.6 Backward Compatibility

Keep the existing `venc_get_stream` / `Frame` / `FrameCallback` APIs for now.
The new `fetch_frame_owned` / `OwnedFrame` / `OwnedFrameCallback` paths run
in parallel. Once stable, the old path can be deprecated.

#### 1.7 Changes to `pending_frames`

Replace `pending_frames: Mutex<HashMap<u64, PendingFrame>>` (stores Vec<u8>)
with `pending_tokens: Mutex<HashMap<u64, u64>>` (stores only remote_token).
This eliminates the HashMap storing large frame data.

**Files changed:**
- `src/hal/vendor_ipc.rs` — Add `fetch_frame_owned`, `release_frame_owned`, `recv_frame_response`
- `src/platform/frame.rs` — Add `OwnedFrame`, `OwnedFrameCallback`
- `src/platform/anyka.rs` — Update `unified_frame_read_loop` to use owned path
- `src/streaming/bridge.rs` — Add `route_owned_frame` method

---

### Phase 2: Reduce Per-Frame Allocations with Buffer Pool

**Goal**: Reuse `BytesMut` buffers instead of allocating new ones per frame.
**Impact**: Eliminates ~100 malloc/free per second, reduces memory fragmentation.
**Effort**: 1-2 days. **Risk**: Low.

#### 2.1 Simple BytesMut Pool

```rust
/// Fixed-capacity pool of reusable BytesMut buffers.
///
/// Designed for the frame receive path where buffers are allocated
/// frequently with similar sizes. Reduces malloc pressure on uClibc.
pub struct BytesMutPool {
    pool: Mutex<Vec<BytesMut>>,
    default_capacity: usize,
    max_pool_size: usize,
}

impl BytesMutPool {
    pub fn new(default_capacity: usize, max_pool_size: usize) -> Self {
        Self {
            pool: Mutex::new(Vec::with_capacity(max_pool_size)),
            default_capacity,
            max_pool_size,
        }
    }

    /// Get a buffer from the pool, or allocate a new one.
    pub fn get(&self, min_capacity: usize) -> BytesMut {
        if let Some(mut buf) = self.pool.lock().pop() {
            buf.clear();
            if buf.capacity() >= min_capacity {
                return buf;
            }
            // Buffer too small — drop it, allocate fresh
        }
        BytesMut::with_capacity(min_capacity.max(self.default_capacity))
    }

    /// Return a buffer to the pool for reuse.
    pub fn put(&self, buf: BytesMut) {
        let mut pool = self.pool.lock();
        if pool.len() < self.max_pool_size {
            pool.push(buf);
        }
        // else: drop buf (pool full)
    }
}
```

#### 2.2 Integration with Frame Receive

```rust
// In recv_frame_response:
fn recv_frame_response(
    stream: &mut UnixStream,
    pool: &BytesMutPool,
) -> PlatformResult<(FrameMetadata, BytesMut)> {
    // ... header parsing same as Phase 1 ...

    // Get buffer from pool instead of allocating
    let mut frame_data = pool.get(frame_len);
    frame_data.resize(frame_len, 0);
    stream.read_exact(&mut frame_data)?;

    Ok((metadata, frame_data))
}
```

#### 2.3 Return-to-Pool After Streaming

After the streaming pipeline consumes a frame (RTSP packet sent, HTTP-FLV chunk
written), the `BytesMut` can be returned to the pool. This requires adding a
pool reference to the frame queue or using a custom `Drop` implementation.

**Simple approach**: Add a `recycle` method to `LowLatencyFrameQueue`:

```rust
impl LowLatencyFrameQueue {
    /// Dequeue a frame, returning the buffer to a pool after processing.
    pub async fn recv_with_recycle(
        &self,
        pool: &BytesMutPool,
    ) -> QueuedFrame {
        let frame = self.recv().await;
        // Caller processes frame.data, then:
        // pool.put(frame.data) when done
        frame
    }
}
```

**Files changed:**
- `src/hal/vendor_ipc.rs` — Use pool in `recv_frame_response`
- `src/streaming/bridge.rs` — Add pool integration (new `BytesMutPool` struct)
- `src/platform/anyka.rs` — Create pool in platform init, pass to frame loop

**Pool sizing** (for 24MB budget):
- Default buffer: 64 KB (covers most P-frames)
- Max pool size: 8 buffers (512 KB total)
- I-frames > 64KB get fresh allocations (infrequent, ~1/sec per stream)

---

### Phase 3: Separate Frame and Control Sockets

**Goal**: Eliminate `Arc<Mutex<UnixStream>>` contention between the high-frequency
frame reader and low-frequency control commands (brightness, resolution, etc.).
**Impact**: Reduces worst-case frame latency when control commands coincide.
**Effort**: 2-3 days. **Risk**: Medium (requires daemon socket change).

#### 3.1 Dual Socket Architecture

```
onvif-rust                              vendor-daemon
┌─────────────────┐                     ┌─────────────────┐
│                  │                     │                  │
│ Frame Socket ────┼─── /tmp/vd-frame ──┼─► Frame Handler  │
│ (no Mutex,       │    .sock            │   (dedicated)    │
│  single reader)  │                     │                  │
│                  │                     │                  │
│ Control Socket ──┼─── /tmp/vd-ctrl ───┼─► Ctrl Handler   │
│ (Mutex OK,       │    .sock            │   (existing)     │
│  infrequent)     │                     │                  │
└─────────────────┘                     └─────────────────┘
```

#### 3.2 Daemon Changes (C)

The daemon currently accepts one connection on `/tmp/vendor-daemon.sock`.
Change to accept two:

```c
// New: listen on two sockets
int frame_sock = create_unix_socket("/tmp/vd-frame.sock");
int ctrl_sock  = create_unix_socket("/tmp/vd-ctrl.sock");

// Frame socket: only handles CMD_VENC_GET_STREAM, CMD_VENC_RELEASE_STREAM
// Ctrl socket: handles everything else (vi_open, set_brightness, etc.)
```

**Backward compatibility**: Keep the existing single-socket path. If the daemon
detects both sockets are connected, use dual mode. If only the old socket is
connected, use legacy mode.

#### 3.3 Rust Client Changes

```rust
pub struct VendorIpc {
    /// Dedicated frame I/O socket — no Mutex needed since only the
    /// unified_frame_read_loop thread accesses it.
    frame_stream: UnixStream,  // NOT behind Mutex

    /// Control socket for setup/teardown/config commands.
    ctrl_stream: Mutex<UnixStream>,

    /// Remote tokens awaiting release.
    pending_tokens: Mutex<HashMap<u64, u64>>,
}
```

The frame reader thread calls `fetch_frame_owned` and `release_frame_owned`
directly on `frame_stream` without any locking. Control commands from ONVIF
handlers use `ctrl_stream` with Mutex (fine — they're infrequent).

**Files changed:**
- `cross-compile/vendor-daemon/src/main.c` — Dual socket listener
- `src/hal/vendor_ipc.rs` — Split into frame_stream + ctrl_stream
- `src/platform/anyka.rs` — Pass frame socket to reader thread

---

### Phase 4: Reduce Encode Helper Allocations (Opportunistic)

**Goal**: Replace per-call `Vec::new()` in `encode_*` helpers with stack buffers.
**Impact**: Minor (~0.01% CPU) but improves code quality.
**Effort**: 0.5 days. **Risk**: None.

#### 4.1 Stack-Based Encoding

```rust
// OLD:
fn encode_i32(val: i32) -> Vec<u8> {
    val.to_le_bytes().to_vec()  // heap alloc for 4 bytes!
}

fn encode_encode_param(param: &encode_param) -> Vec<u8> {
    let mut data = Vec::new();  // heap alloc
    data.extend_from_slice(&param.width.to_le_bytes());
    // ... 11 more fields ...
    data
}

// NEW: Use fixed-size arrays or write directly to socket
fn write_i32(stream: &mut UnixStream, val: i32) -> io::Result<()> {
    stream.write_all(&val.to_le_bytes())
}

fn write_encode_param(stream: &mut UnixStream, param: &encode_param) -> io::Result<()> {
    // Write each field directly — no intermediate allocation
    stream.write_all(&param.width.to_le_bytes())?;
    stream.write_all(&param.height.to_le_bytes())?;
    // ... 10 more fields ...
    Ok(())
}

// Or use iovec/writev for a single syscall:
fn write_encode_param_vectored(stream: &mut UnixStream, param: &encode_param) -> io::Result<()> {
    let bufs: [IoSlice; 12] = [
        IoSlice::new(&param.width.to_le_bytes()),
        IoSlice::new(&param.height.to_le_bytes()),
        // ...
    ];
    stream.write_all_vectored(&mut bufs)?;  // Single syscall
    Ok(())
}
```

**Files changed:**
- `src/hal/vendor_ipc.rs` — Replace `encode_*` helpers

---

## Implementation Order & Verification

### Task Sequence

| Task | Phase | Files | Verification |
|------|-------|-------|-------------|
| 1 | P1 | frame.rs | Add `OwnedFrame`, `OwnedFrameCallback` types |
| 2 | P1 | vendor_ipc.rs | Add `FrameMetadata`, `recv_frame_response`, `fetch_frame_owned`, `release_frame_owned` |
| 3 | P1 | vendor_ipc.rs | Replace `pending_frames` with `pending_tokens` |
| 4 | P1 | anyka.rs | Update `drain_stream` to use `fetch_frame_owned` |
| 5 | P1 | bridge.rs | Add `route_owned_frame` method |
| 6 | P1 | - | Tests: fake daemon + owned frame round-trip |
| 7 | P2 | bridge.rs | Add `BytesMutPool` |
| 8 | P2 | vendor_ipc.rs | Use pool in `recv_frame_response` |
| 9 | P2 | - | Tests: pool alloc/reuse verification |
| 10 | P3 | vendor-daemon C | Add dual socket listener |
| 11 | P3 | vendor_ipc.rs | Split frame_stream / ctrl_stream |
| 12 | P3 | anyka.rs | Pass frame socket to reader |
| 13 | P4 | vendor_ipc.rs | Replace `encode_*` with direct writes |

### Quality Gates (Per Task)

```bash
export CARGO=../../toolchain/arm-anykav200-crosstool-ng/bin/cargo

# 1. Format
$CARGO fmt

# 2. Clippy (host)
$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings

# 3. Tests (host)
$CARGO test --target x86_64-unknown-linux-gnu

# 4. ARM build
$CARGO build --release

# 5. Docs
$CARGO doc --no-deps
```

### On-Device Validation

After each phase, deploy to device and verify:

```bash
# Stream health check (should show 25fps, no frame drops)
curl -s http://camera:8080/api/stream/health

# FPS measurement
./scripts/measure_rtsp_fps.sh --duration 30

# Memory usage (should not increase)
cat /proc/$(pidof onvif-rust)/status | grep VmRSS
```

---

## Risk Assessment

| Risk | Phase | Impact | Mitigation |
|------|-------|--------|-----------|
| BytesMut::zeroed slower than vec![0u8; n] | P1 | Perf regression | Benchmark both; use `BytesMut::with_capacity` + `unsafe set_len` if needed |
| OwnedFrame breaks existing callback chain | P1 | Regression | Keep old path, run both in parallel until stable |
| Pool exhaustion under load | P2 | Frame drops | Pool fallback to fresh alloc; size pool for 2× expected pipeline depth |
| Daemon dual-socket backward compat | P3 | Breaks old binaries | Feature flag on daemon; fallback to single socket |
| Stack buffer overflow in encode | P4 | Crash | Fixed-size buffers with compile-time size checks |

## Expected Results

| Metric | Before | Phase 1 | Phase 2 | Phase 3 | Phase 4 |
|--------|--------|---------|---------|---------|---------|
| Frame copies (Rust) | 2 | 1 | 1 | 1 | 1 |
| Allocs per frame | 3 | 1 | 0-1 | 0-1 | 0-1 |
| Mutex acquisitions (hot) | 2/frame | 1/frame | 1/frame | 0/frame | 0/frame |
| Syscalls per frame | ~10 | ~8 | ~8 | ~6 | ~5 |
| Est. CPU savings | — | ~0.5% | ~0.1% | ~0.2% | ~0.05% |
| **Cumulative CPU savings** | — | **~0.5%** | **~0.6%** | **~0.8%** | **~0.85%** |

On the ARM926EJ-S at 200MHz, 0.85% CPU translates to ~1.7 million fewer
instructions per second freed up for ONVIF XML processing, RTSP packetization,
and other work.

## Future: Approach A (Shared Memory)

If profiling after Phase 3 shows that the remaining copy (kernel→userspace
socket read) is still a bottleneck, proceed to Approach A (shared memory ring
buffer) as documented in `2026-02-24-ipc-sharedmem-approach-a.md`. Approach A
would eliminate the final copy, achieving true zero-copy frame delivery.

---

## Appendix: Device Syscall Availability Verification

Confirmed on target (kernel 3.4.35):
```
process_vm_readv    ✅ (available per /proc/kallsyms)
process_vm_writev   ✅
mmap / munmap       ✅
epoll               ✅
eventfd             ✅
splice / vmsplice   ✅ (kernel 2.6.17+)
sendfile            ✅
memfd_create        ❌ (needs 3.17+)
io_uring            ❌ (needs 5.1+)
```
