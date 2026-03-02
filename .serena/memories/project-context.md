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
│   ├── Cargo.toml           # Workspace root (members: onvif-rust, streaming-lib)
│   │
│   ├── onvif-rust/          # 🎯 PRIMARY - Rust ONVIF implementation
│   │   ├── src/
│   │   │   ├── onvif/       # ONVIF services (Device, Media, PTZ, Imaging)
│   │   │   │   ├── device/  # Device service handlers, types, faults
│   │   │   │   ├── media/   # Media service
│   │   │   │   ├── ptz/     # PTZ service
│   │   │   │   ├── imaging/ # Imaging service
│   │   │   │   ├── types/   # Shared ONVIF type definitions
│   │   │   │   ├── dispatcher.rs  # SOAP action routing
│   │   │   │   ├── server.rs      # HTTP/SOAP server
│   │   │   │   ├── soap.rs        # SOAP envelope handling
│   │   │   │   └── ws_security.rs # WS-Security processing
│   │   │   ├── hal/         # Hardware Abstraction Layer (IPC-based)
│   │   │   │   ├── anyka_sdk.rs   # SDK type definitions
│   │   │   │   ├── vendor_ipc.rs  # Unix socket IPC client (2,132 LOC)
│   │   │   │   ├── video.rs       # Video HAL
│   │   │   │   ├── audio.rs       # Audio HAL
│   │   │   │   ├── imaging.rs     # Imaging HAL
│   │   │   │   ├── ptz.rs         # PTZ HAL
│   │   │   │   └── ptz_driver.rs  # PTZ motor driver
│   │   │   ├── streaming/   # Bridge to streaming-lib
│   │   │   │   ├── bridge.rs      # Frame delivery bridge
│   │   │   │   ├── config.rs      # Stream configuration
│   │   │   │   ├── helpers.rs     # Streaming utilities
│   │   │   │   └── service.rs     # Streaming service
│   │   │   ├── platform/    # Platform abstraction
│   │   │   │   ├── traits.rs      # Platform trait definitions
│   │   │   │   ├── anyka.rs       # Anyka implementation (5,709 LOC)
│   │   │   │   ├── frame.rs       # Frame ownership & stream ID
│   │   │   │   ├── hw_ptz.rs      # Hardware PTZ
│   │   │   │   ├── stubs.rs       # Test stubs
│   │   │   │   └── validation.rs  # Platform validation
│   │   │   ├── auth/        # Authentication (WS-Security, HTTP Digest/Basic)
│   │   │   ├── security/    # Rate limiting, brute force, XML security, audit
│   │   │   ├── discovery/   # WS-Discovery (UDP multicast)
│   │   │   ├── config/      # Configuration management, persistence & user management
│   │   │   ├── lifecycle/   # App lifecycle (startup, shutdown, health)
│   │   │   ├── logging/     # HTTP & platform logging
│   │   │   ├── net/         # Network utilities (IP detection)
│   │   │   ├── validation/  # H.264 playback & stream validation
│   │   │   ├── utils/       # Shared utilities
│   │   │   ├── app.rs       # Application lifecycle
│   │   │   ├── lib.rs       # Library root, global allocator
│   │   │   └── main.rs      # Binary entry point
│   │   ├── tests/           # Integration tests (25+ suites)
│   │   └── Cargo.toml
│   │
│   ├── streaming-lib/       # 🎯 RTSP/HTTP-FLV streaming library
│   │   ├── src/
│   │   │   ├── rtsp/        # RTSP protocol implementation
│   │   │   ├── httpflv/     # HTTP-FLV muxing
│   │   │   └── ...
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
│  Anyka SDK    │  Shared Memory  │  hal/         │
│  frame capture│ ──────────────→ │  vendor_ipc   │
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

## Technology Stack

### Rust Backend (onvif-rust + streaming-lib)
| Category | Technology |
|----------|------------|
| Language | Rust (Edition 2024) |
| Web Framework | axum 0.8 |
| Async Runtime | tokio 1.0 (multi-thread) |
| Serialization | serde, quick-xml 0.39 |
| Logging | tracing, tracing-subscriber, tracing-appender |
| Error Handling | thiserror 2.0 (libs), anyhow 1.0 (apps) |
| Memory Tracking | cap 0.1 (24MB hard limit) |
| Concurrency | parking_lot 0.12, dashmap 6.1, portable-atomic 1.13 |
| Streaming | streaming-lib (workspace, RTSP/HTTP-FLV) |
| Testing | mockall 0.14, wiremock 0.6, criterion 0.8 |
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
| Testing | Vitest 4.0, Testing Library 16.3, MSW 2.12 |

### CI/CD
- **Platform**: GitHub Actions
- **Container**: `kkrzysztofik/anyka-cross-compile:rust-1.91.1`
- **Rust**: fmt, clippy, test, tarpaulin (coverage)
- **WebUI**: lint, type-check, test, coverage
- **Security**: Snyk (SAST + SCA), SonarQube
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
members = ["onvif-rust", "streaming-lib"]
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
- IPC via `hal/vendor_ipc.rs` (never direct FFI)
- Zero-copy frames via `hal/anyka/ipc/shm_ring.rs`

### WebUI
- Strict TypeScript (no `any`)
- `data-testid` for all test selectors
- Zod 4 schemas for all form validation
- MSW for API mocking in tests
- shadcn/ui components only (no custom primitives)

## Essential Commands

**⚠️ Cross-compile note**: Default Rust target is ARM. Use `--target x86_64-unknown-linux-gnu` for host-side operations (test, lint).

```bash
# Rust (host-side testing/linting) — runs across workspace
cd cross-compile
cargo fmt && \
cargo clippy --target x86_64-unknown-linux-gnu -- -D warnings && \
cargo test --target x86_64-unknown-linux-gnu

# Rust (build for device - ARM)
cargo build --release

# Vendor daemon (ARM cross-compile)
cd cross-compile/vendor-daemon && make

# WebUI
cd cross-compile/www
npm run lint && npm run type-check && npm run test

# Deploy to device
cd scripts && ./deploy_onvif.sh 192.168.1.100 admin admin
```
