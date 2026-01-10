# Platform Layer - Imaging & Network Implementation

## Overview

Implement ImagingControl and NetworkInfo traits for Anyka AK3918 hardware, providing camera imaging settings and network configuration.

## Scope

**In Scope:**
- Implement `AnykaImagingControl` struct in `src/platform/anyka.rs`
- Implement `ImagingControl` trait methods:
  - `get_settings()` - Query current imaging settings
  - `set_brightness()`, `set_contrast()`, `set_saturation()`, `set_sharpness()`
  - `set_ir_filter()` - Control IR cut filter
  - `set_wdr()` - Configure WDR (Wide Dynamic Range)
- Implement `AnykaNetworkInfo` struct
- Implement `NetworkInfo` trait methods:
  - `get_interfaces()` - List network interfaces
  - `detect_local_ip()` - Detect camera IP address
  - `get_hostname()` - Query system hostname
- Integrate with FFI layer (`ffi::imaging`)
- Read network info from Linux system files
- Unit tests with mockall

**Out of Scope:**
- Video/Audio implementation (T5, T6, T7)
- PTZ implementation (T8)
- ONVIF integration (T12)

## Technical Details

**Imaging Parameter Mapping:**
- ONVIF range (0-100) → SDK register values
- Settings persist to configuration file

**Network Info Sources:**
- `/proc/net/dev` - Network interfaces
- `/etc/hostname` - System hostname
- IP detection via socket binding

## Spec References

- spec:6f9ed714-572c-4c78-a67e-e1e363752b44/6f620a2e-4e66-4b20-b1d1-cd99217bdcba - Section 3.1 (Platform Layer), imaging/network
- spec:6f9ed714-572c-4c78-a67e-e1e363752b44/3339e6c7-c72c-49ba-8fda-bb6a8ce6150b - Flow 7 (imaging settings)

## Dependencies

- T3: FFI PTZ/Imaging Wrappers (needs imaging FFI layer)

## Acceptance Criteria

- ✅ ImagingControl trait fully implemented
- ✅ All imaging settings work (brightness, contrast, etc.)
- ✅ Settings apply immediately (visible in stream)
- ✅ NetworkInfo trait fully implemented
- ✅ IP detection works correctly
- ✅ Unit tests pass (100% with mocks)
- ✅ `cargo clippy` passes