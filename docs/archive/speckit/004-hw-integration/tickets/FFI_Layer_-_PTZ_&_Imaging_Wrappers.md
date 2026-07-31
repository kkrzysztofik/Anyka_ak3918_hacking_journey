# FFI Layer - PTZ & Imaging Wrappers

## Overview

Implement safe Rust wrappers around Anyka SDK PTZ and imaging functions using the RAII pattern for automatic resource management.

## Scope

**In Scope:**
- Create `src/ffi/ptz.rs` with safe wrappers:
  - `ak_drv_ptz_open()`, `ak_drv_ptz_close()`
  - `ak_drv_ptz_turn()`, `ak_drv_ptz_get_step_pos()`
  - `ak_drv_ptz_set_cruise_mode()`
  - RAII handle: `PTZHandle`
- Create `src/ffi/imaging.rs` with safe wrappers:
  - Imaging SDK calls (brightness, contrast, saturation, sharpness)
  - IR filter control, WDR settings
  - Sensor parameter adjustments
  - RAII handle: `ImagingHandle`
- Error code conversion
- Null pointer safety checks
- Unit tests with mockall

**Out of Scope:**
- Video/Audio wrappers (T2)
- Platform trait implementations (T8, T9)
- Hardware testing (T13)

## Technical Details

**PTZ Coordinate Conversion:**
- Degrees → motor steps conversion
- Range validation (±180° pan, ±90° tilt)

**Imaging Parameter Mapping:**
- ONVIF ranges (0-100) → SDK register values

## Spec References

- spec:6f9ed714-572c-4c78-a67e-e1e363752b44/6f620a2e-4e66-4b20-b1d1-cd99217bdcba - Section 3.2 (FFI Layer), PTZ/imaging examples
- spec:6f9ed714-572c-4c78-a67e-e1e363752b44/3339e6c7-c72c-49ba-8fda-bb6a8ce6150b - Flow 6 (PTZ), Flow 7 (imaging)

## Dependencies

- T1: Workspace & Vendor Setup (needs headers/libs)

## Acceptance Criteria

- ✅ All PTZ FFI wrappers implemented with RAII
- ✅ All imaging FFI wrappers implemented with RAII
- ✅ Coordinate conversion functions tested
- ✅ Error handling converts SDK errors properly
- ✅ Unit tests pass (100% success with mocks)
- ✅ `cargo clippy` passes with zero warnings
- ✅ `cargo doc` generates documentation