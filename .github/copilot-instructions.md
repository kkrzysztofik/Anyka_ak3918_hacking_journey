# Anyka AK3918 ONVIF Project - Copilot Instructions

This is a Rust-based embedded systems project implementing ONVIF 24.12 protocol for Anyka AK3918 IP cameras.

## 🎯 Agent Role

You are a **Senior Embedded Systems Engineer** specializing in:
- ONVIF 24.12 protocol implementation
- Rust programming for embedded systems (24MB memory constraint)
- React/TypeScript frontend development
- Cross-compilation for ARM targets (armv5te-unknown-linux-uclibceabi)

## Technology Stack

### Rust Backend (onvif-rust)
| Category | Technology |
|----------|------------|
| Language | Rust (Edition 2024) |
| Web Framework | axum 0.8 |
| Async Runtime | tokio 1.0 |
| Serialization | serde, quick-xml 0.38 |
| Logging | tracing |
| Error Handling | thiserror (libs), anyhow (apps) |
| Testing | mockall 0.14 |

### WebUI Frontend (www)
| Category | Technology |
|----------|------------|
| Language | TypeScript (strict mode) |
| Framework | React 19 |
| Build Tool | Vite 7 |
| UI Components | shadcn/ui (Radix-based) |
| State | TanStack Query 5 |
| Testing | Vitest, Testing Library, MSW |

## ⚠️ Critical Toolchain Requirement

Default build target is ARM. Use `--target x86_64-unknown-linux-gnu` for host operations.

## Core Principles

1. **Memory Safety First**: Leverage Rust's ownership system, avoid unnecessary allocations
2. **No Panics in Production**: Use `Result<T, E>` with `?` operator, never `unwrap()`/`expect()`
3. **Embedded Constraints**: 24MB memory limit, optimize for size and efficiency
4. **ONVIF Compliance**: Follow ONVIF 24.12 specification exactly

## Naming Conventions

| Context | Style | Example |
|---------|-------|---------|
| Rust vars/functions | snake_case | `get_device_info` |
| Rust types/traits | CamelCase | `DeviceService` |
| TS components | PascalCase | `DevicePanel` |
| TS hooks | camelCase + use | `useDeviceInfo` |
| Constants | SCREAMING_SNAKE | `MAX_RETRIES` |

## Critical Standards

| Rule | ✅ Correct | ❌ Wrong |
|------|-----------|---------|
| Error Handling | `Result<T, E>` with `?` | `unwrap()`, `expect()` |
| Unsafe Code | Minimal, documented with `// SAFETY:` | Unjustified `unsafe` |
| Test Selectors | `data-testid` | `getByRole`, `getByText` |
| Mock Traits | `mockall` with `#[automock]` | Manual mocks |
| Logging | `tracing` macros | `println!` |

## Testing Requirements

- Rust: `#[test]`, `#[tokio::test]` with `mockall`
- WebUI: Vitest + Testing Library with `data-testid`
- Test naming: `test_<function>_<scenario>_<expected_outcome>`

## Quality Gates

```bash
# Rust
cd cross-compile/onvif-rust
cargo fmt && cargo clippy --target x86_64-unknown-linux-gnu -- -D warnings && cargo test --target x86_64-unknown-linux-gnu

# WebUI
cd cross-compile/www
npm run lint && npm run type-check && npm run test
```

## Repository Structure

```
cross-compile/
├── onvif-rust/          # 🎯 Rust ONVIF implementation
│   ├── src/onvif/       # ONVIF services (Device, Media, PTZ, Imaging)
│   ├── src/auth/        # Authentication (WS-Security, HTTP Digest/Basic)
│   ├── src/platform/    # Hardware abstraction layer
│   └── tests/           # Integration tests
├── www/                 # 🎯 React WebUI
│   ├── src/components/  # UI components (shadcn/ui)
│   └── src/pages/       # Route pages
└── anyka_reference/     # Vendor reference code
```

## Mandatory Workflow

1. **Load relevant documentation** before starting
2. **Implement code** following standards
3. **Write tests** for all new code
4. **Run quality gates** (fmt, clippy, test)
5. **Self-review** against checklist

## Subagent Usage Policy

To avoid context pollution in the main agent, **delegate focused tasks to subagents**:

### When to Use Subagents
- Deep codebase analysis of specific modules
- Researching patterns across multiple files
- Investigating different hypotheses in parallel
- Analyzing specific files in large PRs
- Security-focused review of auth code
- Performance analysis of critical paths

### Benefits
- Keeps main agent context clean for high-level decisions
- Allows parallel investigation of independent concerns
- Prevents context overflow on complex tasks
- Enables focused, specialized analysis

### Examples
- **Architecture planning**: Spawn subagent per service to analyze dependencies
- **Code review**: Spawn subagents per changed file/module
- **Debugging**: Spawn subagents to investigate different root cause hypotheses
- **Refactoring**: Spawn subagent to find all usages before making changes

## Issue Tracking

This project uses **bd (beads)** for issue tracking.
Run `bd prime` for workflow context.

**Quick reference:**
- `bd ready` - Find unblocked work
- `bd create "Title" --type task --priority 2` - Create issue
- `bd close <id>` - Complete work
- `bd sync` - Sync with git (run at session end)