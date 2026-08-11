# Project Context - Anyka AK3918 Hacking Journey

## Project Description

This repository contains comprehensive reverse-engineering work and custom firmware development for Anyka AK3918-based IP cameras. The project has three primary development focuses:

1. **`onvif-rust`** - Complete ONVIF 24.12 services stack in Rust
2. **`streaming-lib`** - RTSP/HTTP-FLV streaming library (workspace member)
3. **`www`** - Modern React-based camera web interface
4. **`vendor-daemon`** - C daemon bridging Anyka SDK via IPC

The project aims to create a fully ONVIF 24.12 compliant implementation while maintaining compatibility with the existing camera hardware and providing a robust development environment.

## Repository Structure

```
anyka-dev/
├── cross-compile/
│   ├── Cargo.toml           # Workspace root (members: onvif-rust, streaming-lib, anyka-init)
│   │
│   ├── onvif-rust/          # 🎯 PRIMARY - Rust ONVIF implementation
│   │   ├── src/
│   │   │   ├── onvif/       # ONVIF services (Device, Media, PTZ, Imaging)
│   │   │   │   ├── device/  # Device service
│   │   │   │   │   ├── ops/ # {system, network, discovery, users}.rs operations
│   │   │   │   │   ├── service.rs   # DeviceService handler
│   │   │   │   │   ├── state.rs     # Service state
│   │   │   │   │   ├── store.rs     # Device store
│   │   │   │   │   ├── types.rs     # Device types
│   │   │   │   │   ├── faults.rs    # Device faults
│   │   │   │   │   ├── validation.rs
│   │   │   │   │   └── user_types.rs
│   │   │   │   ├── media/   # Media service
│   │   │   │   ├── ptz/     # PTZ service
│   │   │   │   ├── imaging/ # Imaging service
│   │   │   │   ├── analytics/   # Analytics service
│   │   │   │   ├── events/      # Events service
│   │   │   │   ├── discovery/   # Discovery service
│   │   │   │   ├── common/      # dispatch.rs (dispatch_sync/dispatch_async)
│   │   │   │   ├── dispatcher/  # mod.rs (ServiceHandler trait), parse_body
│   │   │   │   ├── error/       # mod.rs (OnvifError)
│   │   │   │   ├── soap/        # build.rs, parse.rs, model.rs
│   │   │   │   ├── types/       # {common, device, media, imaging, ptz}
│   │   │   │   ├── auth_requirements.rs
│   │   │   │   ├── server.rs    # HTTP/SOAP server
│   │   │   │   ├── ws_security.rs
│   │   │   │   └── mod.rs
│   │   │   ├── hal/         # Hardware Abstraction Layer (IPC-based)
│   │   │   │   ├── common/  # {audio, imaging, ptz, sdk_types, video}.rs traits
│   │   │   │   ├── anyka/   # ipc/, ptz/, sdk.rs
│   │   │   │   └── stub/    # Test stubs
│   │   │   ├── streaming/   # Bridge to streaming-lib
│   │   │   │   ├── bridge.rs      # Frame delivery bridge
│   │   │   │   ├── config.rs      # Stream configuration
│   │   │   │   ├── helpers.rs     # Streaming utilities
│   │   │   │   ├── service.rs     # Streaming service
│   │   │   │   └── telemetry.rs   # Stream telemetry
│   │   │   ├── platform/    # Platform abstraction (business logic)
│   │   │   │   ├── anyka/  # {audio_encoder, audio_input, context, imaging,
│   │   │   │   │          #  lifecycle, network_info, night_mode, ptz_actor,
│   │   │   │   │          #  ptz_control, supervisor, video_encoder,
│   │   │   │   │          #  video_input}.rs, tests/
│   │   │   │   ├── common/ # traits.rs (Platform traits, #[cfg_attr(test, automock)])
│   │   │   │   ├── stub/  # Test stubs
│   │   │   │   └── mod.rs
│   │   │   ├── security/   # Auth home: audit.rs, brute_force.rs, rate_limit.rs, xml_security.rs
│   │   │   ├── config/     # {profiles, users} dirs; persistence, runtime, storage, types
│   │   │   ├── lifecycle/  # App lifecycle (startup, shutdown, health)
│   │   │   ├── logging/    # HTTP & platform logging
│   │   │   ├── validation/ # Input validation
│   │   │   ├── utils/      # {memory, validation}.rs shared utilities
│   │   │   ├── app.rs      # Application lifecycle
│   │   │   ├── lib.rs      # Library root, global allocator
│   │   │   └── main.rs     # Binary entry point (onvif-rust)
│   │   ├── tests/           # Integration tests
│   │   └── Cargo.toml
│   │
│   ├── streaming-lib/       # 🎯 RTSP/HTTP-FLV streaming library
│   │   ├── src/
│   │   │   ├── protocol/    # rtsp/, httpflv/
│   │   │   ├── codec/       # h264/ (sps, pps), test_fixtures
│   │   │   ├── container/   # demuxer, muxer
│   │   │   ├── hub/         # Stream hub
│   │   │   ├── io/          # bits/bytes readers + writers
│   │   │   ├── common/      # Shared types
│   │   │   ├── validation/  # aac_file_reader, h264_file_reader
│   │   │   ├── service.rs   # Streaming service
│   │   │   └── config.rs    # Streaming config
│   │   ├── tests/           # Streaming tests
│   │   └── Cargo.toml
│   │
│   ├── vendor-daemon/       # 🎯 C daemon for Anyka SDK
│   │   ├── src/main.c       # SDK operations via IPC
│   │   ├── include/         # IPC type headers
│   │   ├── lib/             # Compiled SDK libraries
│   │   └── Makefile
│   │
│   ├── www/                 # 🎯 React WebUI
│   │   ├── src/
│   │   │   ├── components/  # UI components (shadcn/ui based)
│   │   │   ├── pages/       # Route pages
│   │   │   ├── services/    # SOAP service clients
│   │   │   ├── hooks/       # React hooks
│   │   │   ├── lib/schemas/ # Zod validation schemas
│   │   │   ├── types/       # TypeScript type definitions
│   │   │   └── ...
│   │   └── package.json
│   │
│   ├── anyka_reference/     # Vendor reference code
│   └── patches/             # OpenSSL uClibc patches
│
├── validation/              # H.264 playback & RTSP RFC compliance suite
├── SD_card_contents/        # SD card payload system
├── scripts/                 # Deployment & debugging scripts
├── toolchain/               # Cross-compilation toolchains
├── docs/                    # Documentation
└── .github/workflows/       # CI/CD pipelines
```

## Architecture: IPC-Based Frame Delivery

The project uses a **push-only IPC architecture** where the vendor-daemon (C) handles low-level SDK operations and pushes frames to the Rust layer:

```
┌──────────────┐   Unix Socket    ┌──────────────┐
│ vendor-daemon │ ──────────────→ │  onvif-rust   │
│    (C/SDK)    │   IPC commands  │   (Rust)      │
│               │                 │               │
│  Anyka SDK    │  Shared Memory  │  hal/anyka    │
│  frame capture│ ──────────────→ │  /ipc         │
│  encoding     │  Ring Buffer    │  ipc/shm_ring │
│               │  (zero-copy)    │               │
│  Dual sockets │                 │  streaming/   │
│  main + sub   │  Frame notify   │  bridge       │
│  /tmp/vd-*.sock ─────────────→ │               │
└──────────────┘                 └──────────────┘
```

**Key Design Decisions**:
- **Push-only**: Daemon pushes frames; Rust never polls
- **Zero-copy**: Shared memory ring buffer avoids frame copies
- **Dual-channel**: Separate sockets for main (1280x720) and sub (640x360) streams
- **Error isolation**: C SDK crashes don't take down Rust process

## Platform vs HAL Boundary

The project enforces strict separation:
- **Platform** (`src/platform/`): Business logic, policy, state management
- **HAL** (`src/hal/`): Hardware access, FFI, unsafe code

**Detailed layering rules**: See [`.serena/memories/platform-hal-layering.md`](platform-hal-layering.md)

## Technology Stack

### Rust Backend (onvif-rust + streaming-lib)
| Category | Technology |
|----------|------------|
| Language | Rust (Edition 2024) |
| Web Framework | axum 0.8 |
| Async Runtime | tokio 1.0 (multi-thread) |
| Serialization | serde, quick-xml 0.41 |
| Logging | tracing, tracing-subscriber, tracing-appender |
| Error Handling | thiserror 2.0 (libs), anyhow 1.0 (apps) |
| Memory Tracking | cap 0.1 (24MB hard limit) |
| Concurrency | parking_lot 0.12, portable-atomic 1.14 |
| Streaming | streaming-lib (workspace, RTSP/HTTP-FLV) |
| Testing | mockall 0.15, wiremock 0.6, criterion 0.8 |
| Target | armv5te-unknown-linux-uclibceabi |

### Vendor Daemon (C)
| Category | Technology |
|----------|------------|
| Language | C |
| SDK | Anyka AK3918 proprietary SDK |
| IPC | Unix domain sockets + shared memory |
| Build | Makefile with ARM cross-compiler |

### WebUI Frontend (www)
| Category | Technology |
|----------|------------|
| Language | TypeScript (strict mode) |
| Framework | React 19.2 |
| Build Tool | Vite 7.3 |
| Styling | TailwindCSS 4.1 |
| UI Components | shadcn/ui (Radix-based) |
| State Management | TanStack Query 5.90 |
| Routing | React Router 7.13 |
| Form Handling | React Hook Form 7.71 + Zod 4.3 |
| XML Parsing | fast-xml-parser 5.3 |
| Testing | Vitest 4.0, Testing Library 16.3 (vi.mock service mocking) |

### CI/CD
- **Platform**: GitHub Actions
- **Host CI container**: `kkrzysztofik/anyka-cross-compile:rust-1.97.1-ci`
- **Cross/release container**: `kkrzysztofik/anyka-cross-compile:rust-1.97.1`
- **Rust**: fmt, clippy, test, cargo-llvm-cov (coverage)
- **WebUI**: lint, type-check, test, coverage
- **Security**: `cargo audit` (RustSec, blocking) + Dependabot (dependency updates), CodeQL + SonarQube (SAST, advisory)
- **ARM Build**: Cross-compile for armv5te target

## Target Platform

- **Chip**: Anyka AK3918 (ARM-based)
- **Architecture**: armv5te-unknown-linux-uclibceabi
- **Memory**: ~24MB available (hard-limited via `cap` allocator)
- **OS**: Embedded Linux (uClibc)
- **Protocols**: ONVIF 24.12, RTSP, HTTP-FLV

## Workspace Configuration

```toml
# cross-compile/Cargo.toml
[workspace]
members = ["onvif-rust", "streaming-lib", "anyka-init"]
resolver = "2"

[profile.release]
opt-level = 3    # Speed over size (5-10% CPU improvement on ARMv5TEJ)
lto = true
codegen-units = 1
strip = true
```

## Key Development Patterns

### Rust
- Use `Result<T, E>` for all fallible operations
- No `unwrap()`/`expect()` in production code
- Use `tokio::sync` primitives for async code
- Minimal, documented `unsafe` blocks
- `mockall` for trait mocking in tests
- IPC via `hal/anyka/ipc/` (never direct FFI)
- Zero-copy frames via `hal/anyka/ipc/shm_ring.rs`

### WebUI
- Strict TypeScript (no `any`)
- `data-testid` for all test selectors
- Zod 4 schemas for all form validation
- vi.mock service module mocking in tests
- shadcn/ui components only (no custom primitives)

## Essential Commands

**⚠️ Cross-compile note**: Default Rust target is ARM. Use `--target x86_64-unknown-linux-gnu` for host-side operations (test, lint).

```bash
# Load the vendored Rust toolchain (exports $CARGO, $RUSTC, $RUSTDOC)
source ./setenv.sh

# Rust (host-side testing/linting) — runs across workspace
cd cross-compile
$CARGO fmt && \
$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings && \
$CARGO test --target x86_64-unknown-linux-gnu

# Rust (build for device - ARM)
$CARGO build --release

# Vendor daemon (ARM cross-compile)
cd cross-compile/vendor-daemon && make

# WebUI
cd cross-compile/www
npm run lint && npm run type-check && npm run test

# Deploy to device
cd scripts && ./deploy_onvif.sh 192.168.2.198 admin admin
```
