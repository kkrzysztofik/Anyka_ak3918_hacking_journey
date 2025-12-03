# Memory Management Architecture

## Overview

This document describes the architecture decisions, design rationale, and implementation details of the memory management system for the ONVIF implementation on Anyka AK3918 embedded devices.

**Key Constraint**: ARMv5TE embedded system with ~32MB total available memory for ONVIF services (64MB system total).

**Specification Reference**: EC-016 - Graceful degradation under memory pressure

---

## Architecture Decision: `cap` Crate for Hard Limit Enforcement

### Selection Rationale

**Chosen Approach**: Use the [`cap`](https://crates.io/crates/cap) crate as the global allocator wrapper for hard-limit enforcement.

**Why `cap`?**

1. **Battle-tested**: Used in production blockchain validators (Solana, etc.) and embedded systems.
2. **Purpose-built**: Explicitly designed for hard-limit enforcement with atomic counters.
3. **Platform-agnostic**: Wraps `std::alloc::System`, making it compatible with uClibc on ARMv5TE.

### Implementation Details

The `cap` crate wraps the system allocator. We configure it with a hard limit (e.g., 32MB). If an allocation would exceed this limit, the allocator returns an error (which in Rust usually results in an abort/panic unless handled via `try_reserve`, but `cap` allows monitoring).

**Note**: In standard Rust, allocation failure is often fatal. However, `cap` allows us to:

1. Monitor usage in real-time.
2. Implement "soft limits" where we reject new heavy requests (like starting a new stream) *before* we hit the hard allocator limit.

### Monitoring Strategy

We implement a `MemoryMonitor` component that:

1. Checks current usage via `Cap::current()`.
2. Compares against a "Soft Limit" (e.g., 24MB).
3. If usage > Soft Limit, the system enters a "Degraded Mode":
    - New RTSP streams are rejected with 503 Service Unavailable.
    - Non-essential background tasks are paused.
    - Admin/Config requests are still allowed.

---

## Future Enhancements

1. **Per-service memory quotas**: Prevent a single service (e.g., Media) from starving the Device service.
2. **Adaptive limits**: Adjust limits based on system load or thermal status.
3. **Memory pooling**: Pre-allocate buffers for video frames to reduce fragmentation.
