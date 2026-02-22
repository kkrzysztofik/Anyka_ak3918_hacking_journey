# Design: Remove Direct FFI, Transition to IPC-Only Architecture

**Date**: 2026-02-22  
**Status**: Implemented  
**Author**: Kilo (AI agent)

## Summary

Removed all direct C FFI bindings (extern "C" blocks, bindgen, vendor SDK linking) from the onvif-rust codebase. The project now exclusively uses IPC communication with vendor-daemon via Unix domain socket for all hardware access except PTZ, which uses a native Rust ioctl driver.

## Motivation

The codebase had three competing access paths to hardware:

1. **Direct FFI** — `extern "C"` calls to Anyka SDK `.so` libraries via bindgen
2. **Vendor IPC** — Unix socket communication with `vendor-daemon` C bridge process
3. **Rust PTZ Driver** — Native `libc::ioctl` to `/dev/ak-motor*` kernel devices

The Direct FFI path required:
- 20 vendor `.so` files (2.4MB) committed to the repo
- 34 C header files for bindgen generation
- `bindgen` and `cc` build dependencies
- Complex `#[cfg]` gates (`use_stubs`, `use_vendor_ipc`) creating three compilation modes
- `unsafe extern "C"` blocks in video.rs, audio.rs, imaging.rs, ipc.rs, anyka_sdk.rs

With vendor-daemon IPC proven stable, the Direct FFI path was dead weight adding complexity, binary size, and maintenance burden.

## Changes

### Phase 1: Remove Direct FFI Code

| Item | Action |
|------|--------|
| `vendor/include/` (34 C headers) | **Deleted** |
| `vendor/lib/` (20 `.so` files, 2.4MB) | **Deleted** |
| `ffi/ipc.rs` (command server) | **Deleted** — SDK command server not needed in IPC mode |
| `build.rs` | **Simplified** — removed bindgen, vendor lib linking, header checks |
| `Cargo.toml` | **Removed** `bindgen`, `cc` build-deps; removed `use_vendor_ipc` feature |
| `video.rs`, `audio.rs`, `imaging.rs` | **Removed** all `extern "C"` blocks and `#[cfg(not(use_stubs))]` impls |
| `anyka_sdk.rs` | **Removed** FFI init code, kept type definitions |
| `platform/anyka.rs` | **Removed** direct FFI paths, command server code; made VendorIpc unconditional |

### Phase 2: Simplify Feature Flags (merged into Phase 1)

**Before**: 3 compilation modes controlled by `use_stubs` + `use_vendor_ipc`
**After**: 2 modes controlled by `use_stubs` only

| Mode | Condition | Implementation |
|------|-----------|---------------|
| ARM (device) | `use_stubs = false` | VendorIpc (Unix socket) + Rust PTZ driver |
| x86_64 (host testing) | `use_stubs = true` | Stub implementations |

### Phase 3: Rename `src/ffi/` → `src/hal/`

The module no longer contains FFI code. Renamed to Hardware Abstraction Layer:

| Old Name | New Name |
|----------|----------|
| `src/ffi/` | `src/hal/` |
| `VideoFfiTrait` | `VideoHalTrait` |
| `AudioFfiTrait` | `AudioHalTrait` |
| `ImagingFfiTrait` | `ImagingHalTrait` |
| `PtzFfiTrait` | `PtzHalTrait` |
| `RealVideoFfi` | `StubVideoHal` |
| `NativePtzFfi` | `NativePtzHal` |
| `REAL_VIDEO_FFI` | `DEFAULT_VIDEO_HAL` |
| `default_ptz_ffi()` | `default_ptz_hal()` |

## Architecture After

```
onvif-rust (Rust)
├── src/hal/              (was src/ffi/)
│   ├── vendor_ipc.rs     ── Unix socket ──► vendor-daemon (C) ──► Anyka SDK
│   ├── ptz_driver.rs     ── libc::ioctl ──► /dev/ak-motor0, /dev/ak-motor1
│   ├── ptz.rs            ── PtzHalTrait abstraction
│   ├── video.rs          ── VideoHalTrait + stubs (real impl via VendorIpc)
│   ├── audio.rs          ── AudioHalTrait + stubs
│   ├── imaging.rs        ── ImagingHalTrait + stubs + conversion functions
│   ├── anyka_sdk.rs      ── Type definitions only
│   └── mod.rs            ── Module facade
├── src/platform/
│   ├── anyka.rs          ── AnykaPlatform (VendorIpc + PTZ driver)
│   ├── hw_ptz.rs         ── HardwarePTZControl
│   └── stubs.rs          ── StubPlatform for testing
```

## Impact

- **74 files changed**, 15,664 lines deleted, 216 lines inserted
- **~2.4MB** of vendor `.so` files removed from repo
- **34 C headers** removed
- **2 build dependencies** removed (bindgen, cc)
- **1 feature flag** removed (`use_vendor_ipc`)
- **1,751 tests passing**, 0 regressions

## Related Issues

- `anyka-dev-rfy`: "Remove vendor/lib and vendor/include from onvif-rust repo" — **Closed by this work**
