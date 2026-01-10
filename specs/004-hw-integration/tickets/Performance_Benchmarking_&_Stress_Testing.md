# Performance Benchmarking & Stress Testing

## Overview

Validate performance characteristics, memory efficiency, and long-term stability on AK3918 hardware.

## Scope

**In Scope:**
- Memory profiling:
  - Baseline memory usage (no clients)
  - Per-client memory usage (1, 2, 3, 4 clients)
  - Peak memory usage under load
  - Validate < 24MB budget with 4 concurrent clients
- Performance benchmarking:
  - RTSP latency measurement (target: < 100ms)
  - HTTP-FLV latency measurement (target: < 3 seconds)
  - PTZ response time (target: < 200ms)
  - Frame callback timing (target: < 2ms RTSP, < 0.2ms FLV)
- 24-hour stress test:
  - Continuous streaming with 4 clients
  - Memory leak detection (usage should be stable)
  - Resource leak detection (file descriptors, SDK handles)
  - Error recovery validation
- Browser compatibility validation:
  - Chrome (desktop + mobile)
  - Firefox (desktop + mobile)
  - Safari (desktop + iOS)
  - Edge (desktop)
- Generate performance report with metrics

**Out of Scope:**
- Feature development (all features complete)
- Hardware integration tests (T15)

## Technical Details

**Memory Profiling Tools:**
- `/proc/self/status` (VmRSS tracking)
- Periodic snapshots every 10 seconds
- Graph memory usage over time

**Latency Measurement:**
- Timestamp at encoder output
- Timestamp at client reception
- Calculate end-to-end latency

**Stress Test Validation:**
- Memory usage stable (no growth over 24 hours)
- No file descriptor leaks (`lsof` monitoring)
- No SDK handle leaks (all resources cleaned up)

## Spec References

- spec:6f9ed714-572c-4c78-a67e-e1e363752b44/b45347e9-3eee-42d0-9e77-2c8cfa54db6f - Success Metrics (all validation criteria)
- spec:6f9ed714-572c-4c78-a67e-e1e363752b44/3339e6c7-c72c-49ba-8fda-bb6a8ce6150b - Flow 9 (concurrent operations)

## Dependencies

- T13: Main Entry Point (needs complete system)
- T14: Memory Management (needs monitoring)
- T15: Hardware Integration Testing (needs working system)

## Acceptance Criteria

- ✅ Memory usage < 24MB with 4 concurrent clients
- ✅ RTSP latency < 100ms (measured)
- ✅ HTTP-FLV latency < 3 seconds (measured)
- ✅ PTZ response < 200ms (measured)
- ✅ Callback timing < 2ms RTSP, < 0.2ms FLV (measured)
- ✅ 24-hour stress test: zero memory leaks
- ✅ 24-hour stress test: zero resource leaks
- ✅ Browser streaming validated (Chrome, Firefox, Safari, Edge)
- ✅ Performance report generated with all metrics
- ✅ All success metrics from Epic Brief validated
