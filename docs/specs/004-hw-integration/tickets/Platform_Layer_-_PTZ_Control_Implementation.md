# Platform Layer - PTZ Control Implementation

## Overview

Implement the PTZControl trait for Anyka AK3918 hardware, providing pan/tilt/zoom motor control.

## Scope

**In Scope:**
- Implement `AnykaPTZControl` struct in `src/platform/anyka.rs`
- Implement `PTZControl` trait methods:
  - `absolute_move()` - Move to absolute position
  - `relative_move()` - Move relative to current position
  - `continuous_move()` - Start continuous movement
  - `stop()` - Stop movement
  - `get_position()` - Query current position
  - `get_status()` - Query movement status
- Coordinate conversion (degrees ↔ motor steps)
- Range validation (±180° pan, ±90° tilt)
- Response time optimization (< 200ms target)
- Integrate with FFI layer (`ffi::ptz`)
- Unit tests with mockall

**Out of Scope:**
- Video/Audio implementation (T5, T6, T7)
- Imaging implementation (T9)
- ONVIF integration (T12)

## Technical Details

**Coordinate Conversion:**
```rust
fn degrees_to_steps(degrees: f32, steps_per_degree: f32) -> i32 {
    (degrees * steps_per_degree) as i32
}
```

**Movement Sequence:**
1. Validate position within range
2. Convert degrees to steps
3. Call SDK PTZ functions
4. Wait for movement completion
5. Return success/error

## Spec References

- spec:6f9ed714-572c-4c78-a67e-e1e363752b44/6f620a2e-4e66-4b20-b1d1-cd99217bdcba - Section 3.1 (Platform Layer), PTZ implementation
- spec:6f9ed714-572c-4c78-a67e-e1e363752b44/3339e6c7-c72c-49ba-8fda-bb6a8ce6150b - Flow 6 (PTZ control)

## Dependencies

- T3: FFI PTZ/Imaging Wrappers (needs PTZ FFI layer)

## Acceptance Criteria

- ✅ PTZControl trait fully implemented
- ✅ All movement types work (absolute, relative, continuous)
- ✅ Coordinate conversion accurate (±2° tolerance)
- ✅ Response time < 200ms
- ✅ Range validation prevents out-of-bounds
- ✅ Unit tests pass (100% with mocks)
- ✅ `cargo clippy` passes