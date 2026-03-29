# Workspace Root & ARMv5TEJ Patch Setup

## Overview

Create the workspace root configuration with ARMv5TEJ compatibility patches to enable cross-compilation for the Anyka AK3918 platform. This is the foundation ticket that unblocks all other development work.

## Scope

**In Scope:**
- Create `cross-compile/Cargo.toml` workspace root
- Configure workspace members: `onvif-rust`, `streaming-lib`
- Apply ARMv5TEJ patches from `xiu/patches/`:
  - `webrtc-util`, `webrtc-ice`, `webrtc-sctp`
  - `rtp`, `tokio-metrics`
  - `openssl-src` (uClibc target support)
- Update `onvif-rust/Cargo.toml` to reference workspace
- Document workspace structure in README

**Out of Scope:**
- Vendor header/library preparation (separate ticket)
- FFI implementation
- streaming-lib creation

## Technical Details

**Workspace Structure:**
```
cross-compile/
├── Cargo.toml          # NEW: Workspace root
├── onvif-rust/
│   └── Cargo.toml      # Updated: workspace member
└── streaming-lib/      # Will be created in T4
    └── Cargo.toml
```

**Patch Configuration:**
All patches reference `xiu/patches/` directory with portable-atomic for ARMv5TEJ compatibility (no 64-bit atomics).

## Spec References

- spec:6f9ed714-572c-4c78-a67e-e1e363752b44/6f620a2e-4e66-4b20-b1d1-cd99217bdcba - Section 1.2 (Workspace Structure)
- spec:6f9ed714-572c-4c78-a67e-e1e363752b44/b45347e9-3eee-42d0-9e77-2c8cfa54db6f - Platform Constraints (ARMv5TEJ)

## Dependencies

None (foundation ticket)

## Acceptance Criteria

- ✅ `cross-compile/Cargo.toml` exists with workspace configuration
- ✅ All 6 ARMv5TEJ patches applied correctly
- ✅ `onvif-rust/Cargo.toml` updated as workspace member
- ✅ `cargo build` succeeds in workspace root
- ✅ Patches inherited by workspace members
- ✅ README documents workspace structure
- ✅ No compilation errors related to atomics or openssl