# Vendor Header & Library Preparation

## Overview

Consolidate Anyka SDK headers and static libraries into the vendor directory structure for FFI binding generation and linking.

## Scope

**In Scope:**
- Create `onvif-rust/vendor/include/` directory
- Symlink or copy headers from `cross-compile/onvif/include/`
- Create `onvif-rust/vendor/lib/` directory
- Copy static libraries (.a files) from `cross-compile/anyka_reference/IOT-ANYKA-PTZdaemon/libs/`:
  - `libplat_common.a`, `libplat_vi.a`, `libplat_ai.a`
  - `libmpi_venc.a`, `libmpi_aenc.a`
  - `libplat_drv.a` (PTZ control)
- Create `scripts/prepare_vendor.sh` automation script
- Verify critical headers exist (ak_vi.h, ak_venc.h, ak_ai.h, ak_aenc.h, ak_drv_ptz.h)
- Update `build.rs` to check vendor directory

**Out of Scope:**
- FFI binding generation (T2, T3)
- Workspace root creation (T1)

## Technical Details

**Header Sources:**
- Use existing `cross-compile/onvif/include/` (40+ headers already consolidated)
- Verify completeness against reference code

**Library Sources:**
- Static libraries from `cross-compile/anyka_reference/IOT-ANYKA-PTZdaemon/libs/`
- Required for static linking (as configured in build.rs)

## Spec References

- spec:6f9ed714-572c-4c78-a67e-e1e363752b44/6f620a2e-4e66-4b20-b1d1-cd99217bdcba - Section 1.5 (Vendor Header Preparation)
- spec:6f9ed714-572c-4c78-a67e-e1e363752b44/b45347e9-3eee-42d0-9e77-2c8cfa54db6f - Vendor header consolidation

## Dependencies

- T1: Workspace Root Setup (needs workspace structure)

## Acceptance Criteria

- ✅ `vendor/include/` contains all Anyka SDK headers
- ✅ `vendor/lib/` contains all required .a files
- ✅ `scripts/prepare_vendor.sh` runs without errors
- ✅ Critical headers verified (ak_vi.h, ak_venc.h, etc.)
- ✅ `build.rs` verification passes
- ✅ README documents vendor setup process
- ✅ Static linking configuration works