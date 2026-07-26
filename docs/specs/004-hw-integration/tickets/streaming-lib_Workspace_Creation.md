# streaming-lib Workspace Creation

## Overview

Create the streaming-lib workspace member by extracting minimal components from xiu with proper attribution and licensing.

## Scope

**In Scope:**
- Create `cross-compile/streaming-lib/` directory structure
- Copy minimal xiu components:
  - `protocol/rtsp/` → `src/rtsp/`
  - `protocol/httpflv/` → `src/httpflv/`
  - `library/codec/h264/` → `src/codec/`
  - `library/container/flv/` → `src/container/`
  - `library/streamhub/` → `src/streamhub/`
  - `library/bytesio/` → `src/bytesio/`
  - `library/common/` → `src/common/`
- Create `Cargo.toml` (library-only, no binary)
- Add `LICENSE` (MIT from xiu)
- Add `NOTICE` (attribution to xiu contributors)
- Create `README.md` documenting fork and modifications
- Define public API in `lib.rs`:
  - `RtspServer`, `HttpFlvServer` traits
  - `FrameSource`, `StreamSession` types

**Out of Scope:**
- Integration with platform layer (T10, T11)
- Frame callback implementation (T10, T11)
- ONVIF integration (T12)

## Technical Details

**Component Mapping:**
```
xiu/protocol/rtsp/          → streaming-lib/src/rtsp/
xiu/protocol/httpflv/       → streaming-lib/src/httpflv/
xiu/library/codec/h264/     → streaming-lib/src/codec/
xiu/library/container/flv/  → streaming-lib/src/container/
xiu/library/streamhub/      → streaming-lib/src/streamhub/
```

**Attribution:**
Document in NOTICE file:
- Original project: xiu (https://github.com/harlanc/xiu)
- License: MIT
- Modifications: ARMv5TEJ patches, minimal extraction, Anyka integration

## Spec References

- spec:6f9ed714-572c-4c78-a67e-e1e363752b44/6f620a2e-4e66-4b20-b1d1-cd99217bdcba - Section 1.2 (streaming-lib Creation), Section 2.2 (Public API)
- spec:6f9ed714-572c-4c78-a67e-e1e363752b44/b45347e9-3eee-42d0-9e77-2c8cfa54db6f - streaming-lib structure, licensing

## Dependencies

- T1: Workspace Root Setup (needs workspace Cargo.toml with patches)

## Acceptance Criteria

- ✅ streaming-lib compiles as workspace member
- ✅ All xiu components copied with correct structure
- ✅ LICENSE file present (MIT)
- ✅ NOTICE file credits xiu contributors
- ✅ README documents fork, modifications, attribution
- ✅ Public API defined in lib.rs
- ✅ `cargo test` passes in streaming-lib
- ✅ No upstream xiu dependencies (all local)
- ✅ ARMv5TEJ patches inherited from workspace root