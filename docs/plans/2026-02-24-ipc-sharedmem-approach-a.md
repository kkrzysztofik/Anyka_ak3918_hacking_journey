# Approach A: Shared Memory Ring Buffer for Zero-Copy Frame Transfer

**Date**: 2026-02-24
**Status**: Draft (Future — Phase 2 after Approach B)
**Author**: Kilo (AI agent)

## Summary

Replace Unix domain socket frame data transfer between `vendor-daemon` (C) and
`onvif-rust` (Rust) with a POSIX shared memory ring buffer. Control commands
remain on the Unix socket. This eliminates all frame data copies from the IPC
path, achieving true zero-copy frame delivery on the ARM926EJ-S target.

## Target Device Profile

| Property | Value |
|----------|-------|
| **CPU** | ARM926EJ-S rev 5 (v5l) @ ~200MHz |
| **BogoMIPS** | 199.06 |
| **ISA Features** | swp, half, fastmult, edsp, java (NO NEON, NO VFP) |
| **RAM** | 36,540 KB total, ~4,376 KB free |
| **Kernel** | Linux 3.4.35 |
| **Page Size** | 4096 bytes |
| **libc** | uClibc |
| **Hardware** | Cloud39EV2_AK3918E80PIN_MNBD |

## Motivation

### Current Frame Data Path (2 copies)

```
vendor-daemon (C)
  │  SDK buffer → write() to Unix socket  ← copy 1 (kernel buffer)
  │
  ▼
onvif-rust (Rust)
  │  read() from socket → Vec<u8>         ← copy 1 received (userspace alloc)
  │  Vec<u8> → BytesMut                   ← copy 2 (bridge.rs:261)
  │  BytesMut → streaming pipeline
  ▼
RTSP / HTTP-FLV server
```

At 25fps main (720p, ~50KB avg) + 25fps sub (VGA, ~15KB avg):
- **Copy throughput**: ~6.5 MB/s of memcpy
- **CPU cost**: ~3.3% on ARM926EJ-S @ 200MHz (memcpy ~200MB/s without NEON)
- **Allocation cost**: 50 malloc+free pairs/sec × 2 buffers = 100 alloc cycles/sec

### Proposed Frame Data Path (0 copies)

```
vendor-daemon (C)
  │  SDK buffer → memcpy to shared ring buffer  ← 1 copy (daemon side, unavoidable)
  │  send 12-byte notification on socket         ← lightweight metadata only
  │
  ▼
onvif-rust (Rust)
  │  read 12-byte notification from socket
  │  mmap'd pointer → Bytes::from_static-like    ← 0 copies (shared memory)
  │  Bytes → streaming pipeline
  ▼
RTSP / HTTP-FLV server
```

**Savings**: Eliminates ~3.3% CPU overhead + 100 alloc/free cycles per second.

## Architecture

### High-Level Design

```
┌──────────────────────┐                     ┌──────────────────────┐
│   vendor-daemon (C)  │                     │   onvif-rust (Rust)  │
│                      │                     │                      │
│  ┌────────────────┐  │   /tmp/vd-shm.dat   │  ┌────────────────┐  │
│  │ Ring Buffer    │◄─┼─── mmap (RW) ────►──┼─►│ Ring Buffer    │  │
│  │ Writer         │  │   (shared memory)   │  │ Reader         │  │
│  └────────────────┘  │                     │  └────────────────┘  │
│                      │                     │                      │
│  ┌────────────────┐  │  Unix socket (ctrl) │  ┌────────────────┐  │
│  │ Notifier       │──┼──────────────────►──┼─►│ Ctrl Client    │  │
│  └────────────────┘  │  (12-byte metadata) │  └────────────────┘  │
└──────────────────────┘                     └──────────────────────┘
```

### Shared Memory Layout

Total size: 1 MB (configurable, page-aligned)

```
Offset 0x0000: Ring Buffer Header (64 bytes, cache-line aligned)
  ┌─────────────────────────────────────────────────────────────┐
  │ magic: u32          = 0x56444653 ("VDFS")                   │
  │ version: u32        = 1                                     │
  │ total_size: u32     = 1048576                               │
  │ slot_count: u32     = 16                                    │
  │ slot_data_size: u32 = 65024 (64KB - 512 header)             │
  │ write_index: u32    (atomically updated by daemon)          │
  │ read_index: u32     (atomically updated by Rust client)     │
  │ flags: u32          (shutdown, overflow indicators)         │
  │ _padding: [u8; 32]  (fill to 64 bytes / cache line)        │
  └─────────────────────────────────────────────────────────────┘

Offset 0x0040: Slot 0 Header (64 bytes)
  ┌─────────────────────────────────────────────────────────────┐
  │ state: u32          = EMPTY(0) | WRITING(1) | READY(2)     │
  │ frame_len: u32                                              │
  │ timestamp_us: u64                                           │
  │ seq_no: u32                                                 │
  │ frame_type: u32     (0=P, 1=I, 2=B, 3=Pi)                  │
  │ stream_id: u32      (0=main, 1=sub, 2=audio)               │
  │ checksum: u32       (CRC32 of frame data, optional)         │
  │ _padding: [u8; 24]  (fill to 64 bytes / cache line)        │
  └─────────────────────────────────────────────────────────────┘

Offset 0x0080: Slot 0 Data (slot_data_size bytes)
  ┌─────────────────────────────────────────────────────────────┐
  │ [frame_len bytes of encoded H.264/AAC data]                 │
  │ [remainder unused]                                          │
  └─────────────────────────────────────────────────────────────┘

Offset 0x10080: Slot 1 Header (64 bytes)
  ... (repeats for slot_count slots)
```

### Slot Size Calculation

For 720p H.264 at 25fps:
- Average P-frame: ~15-30 KB
- Average I-frame: ~80-150 KB (need oversized slot or I-frame spanning)
- Audio AAC frame: ~500 bytes

**Option 1**: Fixed 64 KB slots × 16 = 1 MB total. I-frames that exceed 64 KB are
split across multiple slots (fragmentation) or fall back to socket transfer.

**Option 2**: Variable-size allocation within ring buffer (more complex, less cache-friendly).

**Recommendation**: Fixed 128 KB slots × 8 = 1 MB total. Accommodates most I-frames
without fragmentation. For the rare I-frame > 128 KB, use socket fallback.

### Notification Protocol (Unix Socket)

The socket only carries lightweight 12-byte frame notifications:

```
Frame Notification (12 bytes):
  slot_index: u32    — which ring buffer slot contains the frame
  frame_len: u32     — actual frame data length within the slot
  flags: u32         — bit 0: is_last_fragment, bit 1: socket_fallback
```

Control commands (open/close/set_params) continue using the existing binary
protocol unchanged.

## ARM926EJ-S Cache Coherence Considerations

**CRITICAL**: The ARM926EJ-S has separate instruction and data caches but
**no hardware cache coherence** between processes sharing memory.

### Memory Ordering

The ARM9 family uses a **weakly ordered** memory model. Shared memory access
between the daemon (writer) and onvif-rust (reader) requires explicit barriers:

**Daemon (C) — after writing frame data:**
```c
// Write frame data to slot
memcpy(slot->data, sdk_buffer, frame_len);

// Data memory barrier — ensure all writes are visible
__asm__ __volatile__("" ::: "memory");  // Compiler barrier
// ARM926EJ-S: drain write buffer
__asm__ __volatile__("mcr p15, 0, %0, c7, c10, 4" :: "r"(0) : "memory");

// Mark slot as READY (atomic store with release semantics)
__atomic_store_n(&slot->state, SLOT_READY, __ATOMIC_RELEASE);
```

**Rust client — before reading frame data:**
```rust
// Load slot state with acquire semantics
let state = slot_state.load(Ordering::Acquire);
if state != SLOT_READY { continue; }

// ARM926EJ-S: invalidate D-cache for the slot region (optional — may not be
// needed if the kernel's mmap implementation handles this via page table flags)
// The safest approach is MAP_SHARED + msync(MS_INVALIDATE) but that's expensive.

// Read frame data — guaranteed visible after Acquire fence
let data = &shared_mem[slot_offset..slot_offset + frame_len];
```

### Cache Management Strategy

On kernel 3.4.35 with ARM926EJ-S, the recommended approach:

1. **Use `MAP_SHARED`** on both sides — the kernel manages cache coherence for
   shared file mappings by mapping pages as uncacheable or write-through.

2. **If performance of uncached access is too slow** (unlikely for 5 MB/s), use
   `madvise(MADV_DONTNEED)` on consumed slots to hint the kernel to drop cached
   pages.

3. **Atomic operations** via `__atomic_*` builtins (C) and `std::sync::atomic` (Rust)
   — ARM926EJ-S supports SWP instruction for atomic exchange. The kernel provides
   `__kuser_cmpxchg` helper at `0xffff0fc0` for compare-and-swap.

4. **Avoid `msync()` per frame** — too expensive at 50fps. Rely on `MAP_SHARED`
   coherence instead.

### kernel 3.4.35 Syscall Availability

| Syscall | Available | Notes |
|---------|-----------|-------|
| `mmap` / `munmap` | ✅ | Core of this design |
| `open` + `ftruncate` | ✅ | Create backing file in `/tmp` |
| `shm_open` | ✅ | If `/dev/shm` mounted (check on device) |
| `shmget` / `shmat` | ✅ | SysV alternative |
| `process_vm_readv` | ✅ | Available per `/proc/kallsyms` |
| `eventfd` | ✅ | Alternative to socket notification |
| `memfd_create` | ❌ | Needs 3.17+ |

**Recommended backing**: `open("/tmp/vendor-frame-ring.shm", O_RDWR|O_CREAT)`
+ `ftruncate()` + `mmap(MAP_SHARED)`. Simple, works on all 3.x kernels.

## Implementation Outline

### C Daemon Changes (`vendor-daemon`)

1. **Ring buffer init**: At startup, create and mmap the shared file
2. **Frame write path**: When SDK delivers a frame, copy to next available slot,
   update `write_index`, send 12-byte notification on socket
3. **Slot recycling**: Monitor `read_index` to know when slots are consumed
4. **Overflow handling**: If writer catches up to reader, either block (backpressure)
   or overwrite oldest (lossy)

### Rust Client Changes (`vendor_ipc.rs`)

1. **Shared memory init**: On connection, mmap the same backing file
2. **Frame receive**: Read 12-byte notification, access slot data via mmap pointer
3. **BytesMut integration**: Create `Bytes` from shared memory slice without copy
   (requires careful lifetime management — slot must not be recycled while `Bytes` is live)
4. **Slot release**: Advance `read_index` after streaming pipeline consumes the frame

### Lifetime Management Challenge

The critical design challenge is ensuring a shared memory slot is not overwritten
while downstream consumers (RTSP packetizer, HTTP-FLV muxer) still reference it.

**Options:**
1. **Copy-on-read**: Copy frame data into `BytesMut` when reading from shared memory.
   Defeats the purpose but is safe. Only saves 1 of 2 copies.
2. **Reference counting per slot**: Add an `Arc<AtomicUsize>` per slot. Consumer
   clones the Arc, writer checks refcount before recycling.
3. **Lease-based**: Consumer must "check out" a slot and "check in" when done.
   Writer skips checked-out slots. Requires enough slots to absorb pipeline latency.

**Recommendation**: Option 3 (lease-based) with 8-16 slots. At 25fps, a slot
holds data for 40ms. Pipeline latency is typically <20ms, so 8 slots provides
8×40ms = 320ms of buffering — more than sufficient.

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|-----------|
| ARM cache coherence bugs | Data corruption, visual artifacts | Use MAP_SHARED (kernel-managed); extensive testing on device |
| Ring buffer overflow | Frame loss | Back-pressure via notification socket; sufficient slot count |
| Slot lifetime violation | Use-after-overwrite | Lease-based design with reader fencing |
| Complex debugging | Hard to reproduce issues | Add CRC32 checksums; IPC debug mode |
| kernel 3.4.35 mmap quirks | Mapping failures | Fallback to pure socket mode |
| Memory pressure (1MB extra) | OOM on 4MB free | Reduce slot count; only 512KB minimum |

## Estimated Impact

| Metric | Current | After Approach A |
|--------|---------|------------------|
| Frame copies (Rust side) | 2 | 0-1 (lease) or 1 (copy-on-read) |
| Syscalls per frame | ~10 | ~2 (notification read + slot advance) |
| CPU overhead (frame I/O) | ~4-6% | ~0.5-1% |
| RAM for frame buffers | ~200KB transient | 1MB fixed (ring buffer) |
| Latency (frame to stream) | ~2-5ms | ~0.5-1ms |

## Prerequisites

- **Approach B must be implemented first** — Approach B's changes (async I/O,
  separate sockets, buffer pool) provide foundation and fallback path
- **Device testing infrastructure** — Need reliable on-device testing for
  shared memory validation
- **vendor-daemon source available** — ✅ Confirmed

## Dependencies

- Approach B (optimized socket) — implement first as baseline and fallback
- vendor-daemon modification — C-side ring buffer writer
- On-device kernel testing — validate mmap behavior on 3.4.35

---

*This document describes a future optimization. Implement Approach B first for
immediate gains with lower risk. Approach A can be pursued once B is stable
and if profiling shows frame copy overhead remains significant.*
