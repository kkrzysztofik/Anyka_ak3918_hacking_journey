# Suggested Commands - Anyka AK3918 Project

> **Toolchain:** this project uses a custom Rust toolchain vendored at
> `toolchain/arm-anykav200-crosstool-ng/`. Run `source ./setenv.sh` (repo root)
> before any Rust command — it exports `$CARGO`/`$RUSTC`/`$RUSTDOC` and prepends
> the toolchain `bin/` to `PATH`. All commands below use `$CARGO`; system Rust
> tools cause compilation/doctest failures from version/target mismatches.

## ⚠️ Cross-Compilation Note

This project cross-compiles for ARM (`armv5te-unknown-linux-uclibceabi`) by default.
**For host-side operations (test, lint, build for dev), you MUST specify x86_64 target.**

The Rust project is a **workspace** with members: `onvif-rust` and `streaming-lib`.
Commands run from `cross-compile/` apply to the entire workspace.

## Quick Reference

### Rust Workspace (onvif-rust + streaming-lib)

```bash
# Navigate to workspace root
cd cross-compile

# === HOST-SIDE COMMANDS (for development) ===

# Build entire workspace for host (x86_64)
$CARGO build --target x86_64-unknown-linux-gnu
$CARGO build --target x86_64-unknown-linux-gnu --release

# Test entire workspace (MUST use host target)
$CARGO test --target x86_64-unknown-linux-gnu
$CARGO test --target x86_64-unknown-linux-gnu --lib          # Unit tests only
$CARGO test --target x86_64-unknown-linux-gnu test_name      # Specific test

# Test specific workspace member
$CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust
$CARGO test --target x86_64-unknown-linux-gnu -p streaming-lib

# Linting (MUST use host target)
$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings

# Formatting (target-independent)
$CARGO fmt                          # Format all workspace code
$CARGO fmt --check                  # Check formatting (CI)

# Documentation (host target)
$CARGO doc --target x86_64-unknown-linux-gnu --no-deps   # Generate docs
$CARGO doc --target x86_64-unknown-linux-gnu --no-deps --open  # Generate and open

# Coverage (host target, requires cargo-llvm-cov; match CI rust-coverage job)
IGNORE_REGEX='(/xiu/|/patches/|/anyka_reference/|/onvif/|webrtc-util|webrtc-ice|webrtc-sctp|/rtp-|tokio-metrics|openssl-src)'
$CARGO llvm-cov --target x86_64-unknown-linux-gnu --workspace \
  --all-features \
  --ignore-filename-regex "$IGNORE_REGEX" \
  --cobertura --output-path coverage/cobertura.xml

# === DEVICE-SIDE COMMANDS (cross-compile for ARM) ===

# Build for device (ARM) - DEFAULT target
$CARGO build --release              # Release build for device
$CARGO build                        # Debug build for device
```

### Pre-Commit (Rust - Workspace)

```bash
cd cross-compile
$CARGO fmt && \
$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings && \
$CARGO test --target x86_64-unknown-linux-gnu
```

### Vendor Daemon (C)

```bash
# Build vendor daemon for ARM
cd cross-compile/vendor-daemon
make                               # Build with ARM cross-compiler
make clean                         # Clean build artifacts
```

### WebUI Frontend (www)

```bash
# Navigate to frontend project
cd cross-compile/www

# Install dependencies
npm ci                             # Clean install (CI-style)
npm install                        # Install/update dependencies

# Development
npm run dev                        # Start dev server
VITE_API_TARGET=http://IP:PORT npm run dev  # Connect to specific camera

# Build
npm run build                      # Production build

# Test
npm run test                       # Run Vitest tests
npm run test:coverage              # Tests with coverage report

# Code Quality (MANDATORY before commit)
npm run lint                       # ESLint check
npm run lint:fix                   # Auto-fix lint issues
npm run type-check                 # TypeScript validation
npm run prettier                   # Format code
```

### Pre-Commit (WebUI)

```bash
cd cross-compile/www
npm run lint && npm run type-check && npm run test
```

### Device Deployment & Debugging

```bash
# Navigate to scripts directory
cd scripts

# Deploy to camera (uploads ARM binaries); default device IP is 192.168.2.198
./deploy_onvif.sh [device_ip] [username] [password]
./deploy_onvif.sh 192.168.2.198 admin admin  # Example

# Run on device
./run_onvif.sh [device_ip] [username] [password] [release|debug]

# Copy payload to SD card
./copy_sd_contents.sh

# Collect crash dumps
./debugging/collect_coredump.sh [device_ip] [username] [password]

# Device shell over telnet (port 24)
./debugging/cam_exec.py '<command>'
```

### Git Workflow

```bash
# Always check status before starting work
git status
git branch

# Create feature branch
git checkout -b feature/your-feature-name

# Stage and commit
git add -p                         # Stage interactively
git commit -m "feat: description"  # Conventional commit

# Push and create PR
git push -u origin feature/your-feature-name
```

## Target Summary

| Operation | Target | Command Flag |
|-----------|--------|--------------|
| Test | Host (x86_64) | `--target x86_64-unknown-linux-gnu` |
| Lint (clippy) | Host (x86_64) | `--target x86_64-unknown-linux-gnu` |
| Coverage | Host (x86_64) | `--target x86_64-unknown-linux-gnu` |
| Build for dev | Host (x86_64) | `--target x86_64-unknown-linux-gnu` |
| Build for device | ARM (default) | (no flag needed) |
| Format | Any | (target-independent) |
| Doc | Host (x86_64) | `--target x86_64-unknown-linux-gnu` |

## CI/CD Notes

- GitHub Actions runs tests/lint with `--target x86_64-unknown-linux-gnu`
- Host CI container: `kkrzysztofik/anyka-cross-compile:rust-1.97.1-ci` (lint/coverage)
- Cross/release container: `kkrzysztofik/anyka-cross-compile:rust-1.97.1`
- Coverage reports uploaded to SonarCloud
- Dependency audit via `cargo audit` (RustSec DB): vulnerabilities fail the build,
  informational advisories (unmaintained/unsound/yanked) only warn. Covers both
  lockfiles; it parses `Cargo.lock` only, so no target flag is needed.
  - `(cd cross-compile && $CARGO audit)`
  - `(cd validation/rust && $CARGO audit)`
- SAST via CodeQL (default setup) and SonarCloud; Dependabot manages dependency updates
- Quality gates must pass before merge
