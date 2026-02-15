# Buffer Pool Infrastructure

## Overview

Implement pre-allocated buffer pools for network connections, RTP packets, and FLV tags to prevent dynamic allocations and ensure memory budget compliance.

## Scope

**In Scope:**
- Create `src/platform/buffer_pools.rs` module
- Implement `NetworkBufferPool`:
  - Pre-allocate 4 buffers × 320KB = 1.3MB
  - RAII `BufferHandle` with automatic return to pool
  - Thread-safe acquisition/release
- Implement `PacketBufferPool` (for RTSP):
  - Pre-allocate 16 buffers × 64KB = 1MB
  - Reusable RTP packet buffers
  - RAII `PacketHandle` with automatic return
- Implement `FlvTagBufferPool` (for HTTP-FLV):
  - Pre-allocate 16 buffers × 1KB = 16KB
  - Header-only buffers (zero-copy frame references)
  - RAII `FlvTagHandle` with automatic return
- Pool exhaustion handling:
  - NetworkBufferPool returns `None` when empty
  - PacketBufferPool evicts the oldest frame
  - FlvTagBufferPool drops the current frame
- Unit tests for all pools (acquire, release, exhaustion)

**Pool Exhaustion Strategy:**
- NetworkBufferPool exhaustion → Reject new client connections (return HTTP 503 or RTSP 453)
- PacketBufferPool exhaustion → Drop oldest frame from send queue (eviction strategy)
- FlvTagBufferPool exhaustion → Drop current frame (log warning)

**Performance Requirements:**
- Acquisition time: < 10μs (to not impact callback timing)
- Example uses `Mutex` for simplicity; prefer lock-free structures (e.g., `crossbeam::queue::SegQueue`) if contention is observed
- Thread-safe with minimal contention

**Integration Points:**
- Provide `get_network_pool()`, `get_packet_pool()`, `get_flv_pool()` accessors
- Initialize pools in main.rs before streaming layer
- Cleanup pools on shutdown (verify all buffers returned)

**Out of Scope:**
- Frame callback implementation (T8b)
- Full streaming integration (T12, T13) is out of scope; basic integration validation required
- Memory monitoring (T15)

## Technical Details

**Network Buffer Pool:**
```rust
pub struct NetworkBufferPool {
    buffers: Vec<Box<[u8; 320 * 1024]>>,
    available: Arc<Mutex<Vec<usize>>>,
}

pub struct BufferHandle<'a> {
    pool: &'a NetworkBufferPool,
    idx: usize,
}

impl NetworkBufferPool {
  pub fn try_acquire(&self) -> Option<BufferHandle<'_>> {
    let mut available = self.available.lock().ok()?;
    available.pop().map(|idx| BufferHandle { pool: self, idx })
  }

  pub fn acquire(&self) -> Option<BufferHandle<'_>> {
    self.try_acquire()
  }

  pub fn get_buffer(&self, handle: &BufferHandle<'_>) -> &[u8; 320 * 1024] {
    &self.buffers[handle.idx]
  }

  pub fn get_buffer_mut(&self, handle: &mut BufferHandle<'_>) -> &mut [u8; 320 * 1024] {
    &mut self.buffers[handle.idx]
  }
}

impl BufferHandle<'_> {
  pub fn as_slice(&self) -> &[u8] {
    &self.pool.buffers[self.idx][..]
  }

  pub fn as_mut_slice(&mut self) -> &mut [u8] {
    &mut self.pool.buffers[self.idx][..]
  }
}

impl Drop for BufferHandle<'_> {
    fn drop(&mut self) {
    match self.pool.available.lock() {
      Ok(mut guard) => guard.push(self.idx),
      Err(poison) => poison.into_inner().push(self.idx),
    }
    }
}

// When sharing across threads, wrap the pool in Arc
// let pool = Arc::new(NetworkBufferPool::new());
```

**Memory Allocation:**
- Network: 1.3MB (4 × 320KB)
- RTP packets: 1MB (16 × 64KB)
- FLV tags: 16KB (16 × 1KB)
- **Total: 2.3MB pre-allocated**

**Pool Exhaustion:**
- NetworkBufferPool returns `None` when empty (caller rejects client)
- PacketBufferPool evicts oldest frame from send queue
- FlvTagBufferPool drops current frame (log warning)

## Spec References

- spec:6f9ed714-572c-4c78-a67e-e1e363752b44/6f620a2e-4e66-4b20-b1d1-cd99217bdcba - Section 1.3 (Memory Management), buffer pool implementation
- spec:6f9ed714-572c-4c78-a67e-e1e363752b44/b45347e9-3eee-42d0-9e77-2c8cfa54db6f - Memory constraints (24MB budget)

## Dependencies

- T1: Workspace Root Setup (needs workspace structure)

## Acceptance Criteria

- ✅ NetworkBufferPool implemented with 4 × 320KB buffers
- ✅ PacketBufferPool implemented with 16 × 64KB buffers
- ✅ FlvTagBufferPool implemented with 16 × 1KB buffers
- ✅ RAII handles return buffers to pool automatically
- ✅ Thread-safe acquisition/release (no race conditions)
- ✅ Pool exhaustion handling documented per pool
- ✅ Pool exhaustion strategy documented and tested
- ✅ Acquisition time < 10μs (measured with benchmarks)
- ✅ Concurrent access tests pass (no race conditions under load)
- ✅ API design supports future T12/T13 integration (no actual integration required)
- ✅ All buffers returned on shutdown (no leaks detected)
- ✅ Total pre-allocated memory: 2.3MB
- ✅ Unit tests pass (acquire, release, exhaustion scenarios)
- ✅ `cargo clippy` passes with zero warnings
- ✅ Documentation generated
