# Memory Management & Monitoring

## Overview

Implement fast memory monitoring (100ms intervals) with proactive client rejection to prevent OOM crashes on the memory-constrained AK3918 platform.

## Scope

**In Scope:**
- Enforce memory limit at startup (24MB via cap crate)
- Implement 100ms memory monitoring task
- Implement three-tier threshold system:
  - 18MB: OK (accept clients)
  - 20MB: Pressure (reject new clients)
  - 22MB: Critical (reject new clients, log error)
- Implement proactive client rejection mechanism
- Implement `check_memory_before_allocation()` for large allocations
- Integrate with streaming layer (client acceptance)
- Memory usage tracking via `/proc/self/status`
- Atomic flags for client acceptance control
- Unit tests for monitoring logic

**Out of Scope:**
- Buffer pool implementation (T6)
- Hardware integration tests (T15)
- Performance benchmarking (T16)

## Technical Details

**Monitoring Task:**
```rust
async fn monitor_memory_usage() {
    loop {
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        let usage_mb = get_current_memory_usage() / (1024 * 1024);
        
        if usage_mb > 22 {
            error!("Memory critical: {}MB / 24MB", usage_mb);
            ACCEPT_NEW_CLIENTS.store(false, Ordering::SeqCst);
        } else if usage_mb > 20 {
            warn!("Memory pressure: {}MB / 24MB", usage_mb);
            ACCEPT_NEW_CLIENTS.store(false, Ordering::SeqCst);
        } else if usage_mb < 18 {
            ACCEPT_NEW_CLIENTS.store(true, Ordering::SeqCst);
        }
    }
}
```

**Client Rejection:**
- Streaming servers check `ACCEPT_NEW_CLIENTS` before accepting
- RTSP: Return 453 Not Enough Bandwidth
- HTTP-FLV: Return HTTP 503 Service Unavailable

## Spec References

- spec:6f9ed714-572c-4c78-a67e-e1e363752b44/6f620a2e-4e66-4b20-b1d1-cd99217bdcba - Section 1.3 (Memory Management), monitoring implementation
- spec:6f9ed714-572c-4c78-a67e-e1e363752b44/3339e6c7-c72c-49ba-8fda-bb6a8ce6150b - Flow 9 (concurrent operations), Flow 10 (error recovery)

## Dependencies

- T13: Main Entry Point (needs main.rs to spawn monitoring task)

## Acceptance Criteria

- ✅ Memory limit enforced at startup (24MB via cap crate)
- ✅ Process crashes if memory limit exceeded (hard limit)
- ✅ Monitoring runs at 100ms intervals
- ✅ Memory usage tracked accurately via /proc/self/status
- ✅ Three-tier thresholds work correctly
- ✅ Client rejection at 20MB threshold
- ✅ Atomic flags prevent race conditions
- ✅ Unit tests pass (threshold logic)
- ✅ No performance impact from monitoring
- ✅ Logging provides visibility into memory state
