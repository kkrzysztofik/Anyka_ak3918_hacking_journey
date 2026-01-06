# Project Context - Anyka AK3918 Hacking Journey

## Project Description

This repository contains comprehensive reverse-engineering work and custom firmware development for Anyka AK3918-based IP cameras. The project has two primary development focuses:

1. **`onvif-rust`** - Complete ONVIF 24.12 services stack rewrite in Rust
2. **`www`** - Modern React-based camera web interface

The project aims to create a fully ONVIF 24.12 compliant implementation while maintaining compatibility with the existing camera hardware and providing a robust development environment.

## Repository Structure

```
anyka-dev/
├── cross-compile/
│   ├── onvif-rust/          # 🎯 PRIMARY - Rust ONVIF implementation
│   │   ├── src/             # Source code
│   │   │   ├── onvif/       # ONVIF services (Device, Media, PTZ, Imaging)
│   │   │   ├── auth/        # Authentication (WS-Security, HTTP Digest/Basic)
│   │   │   ├── platform/    # Hardware abstraction layer
│   │   │   ├── security/    # Rate limiting, brute force protection
│   │   │   └── ...
│   │   ├── tests/           # Integration tests
│   │   └── Cargo.toml       # Dependencies
│   │
│   ├── www/                 # 🎯 PRIMARY - React WebUI
│   │   ├── src/
│   │   │   ├── components/  # UI components (shadcn/ui based)
│   │   │   ├── pages/       # Route pages
│   │   │   ├── services/    # API services
│   │   │   ├── hooks/       # React hooks
│   │   │   └── ...
│   │   └── package.json     # Dependencies
│   │
│   ├── xiu/                 # Media streaming server (Rust)
│   └── anyka_reference/     # Vendor reference code
│
├── SD_card_contents/        # SD card payload system
├── scripts/                 # Deployment & debugging scripts
├── toolchain/               # Cross-compilation toolchains
├── docs/                    # Documentation
└── .github/workflows/       # CI/CD pipelines
```

## Technology Stack

### Rust Backend (onvif-rust)
| Category | Technology |
|----------|------------|
| Language | Rust (Edition 2024) |
| Web Framework | axum 0.8 |
| Async Runtime | tokio 1.0 (multi-thread) |
| Serialization | serde, quick-xml 0.38 |
| Logging | tracing, tracing-subscriber |
| Error Handling | thiserror (libs), anyhow (apps) |
| Testing | mockall 0.14, wiremock |
| Target | armv5te-unknown-linux-uclibceabi |

### WebUI Frontend (www)
| Category | Technology |
|----------|------------|
| Language | TypeScript (strict mode) |
| Framework | React 19 |
| Build Tool | Vite 7 |
| Styling | TailwindCSS 4 |
| UI Components | shadcn/ui (Radix-based) |
| State Management | TanStack Query 5 |
| Form Handling | React Hook Form + Zod |
| Testing | Vitest, Testing Library, MSW |

### CI/CD
- **Platform**: GitHub Actions
- **Rust**: fmt, clippy, test, tarpaulin (coverage)
- **WebUI**: lint, type-check, test, coverage
- **Security**: Snyk (SAST + SCA), CodeQL
- **Quality**: SonarCloud analysis

## Target Platform

- **Chip**: Anyka AK3918 (ARM-based)
- **Architecture**: armv5te-unknown-linux-uclibceabi
- **Memory**: ~24MB available (memory-constrained)
- **OS**: Embedded Linux (uClibc)
- **Protocols**: ONVIF 24.12, RTSP, HTTP

## Key Development Patterns

### Rust
- Use `Result<T, E>` for all fallible operations
- No `unwrap()`/`expect()` in production code
- Use `tokio::sync` primitives for async code
- Minimal, documented `unsafe` blocks
- `mockall` for trait mocking in tests

### WebUI
- Strict TypeScript (no `any`)
- `data-testid` for all test selectors
- Zod schemas for all form validation
- MSW for API mocking in tests
- shadcn/ui components only (no custom primitives)

## Essential Commands

**⚠️ Cross-compile note**: Default Rust target is ARM. Use `--target x86_64-unknown-linux-gnu` for host-side operations (test, lint).

```bash
# Rust (host-side testing/linting)
cd cross-compile/onvif-rust
cargo fmt && \
cargo clippy --target x86_64-unknown-linux-gnu -- -D warnings && \
cargo test --target x86_64-unknown-linux-gnu

# Rust (build for device - ARM)
cargo build --release

# WebUI
cd cross-compile/www
npm run lint && npm run type-check && npm run test

# Deploy to device
cd scripts && ./deploy_onvif.sh 192.168.1.100 admin admin
```
