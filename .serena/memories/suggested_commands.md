# Suggested Commands - Anyka AK3918 Project

## ⚠️ Cross-Compilation Note

This project cross-compiles for ARM (`armv5te-unknown-linux-uclibceabi`) by default.
**For host-side operations (test, lint, build for dev), you MUST specify x86_64 target.**

## Quick Reference

### Rust Backend (onvif-rust)

```bash
# Navigate to Rust project
cd cross-compile/onvif-rust

# === HOST-SIDE COMMANDS (for development) ===

# Build for host (x86_64)
cargo build --target x86_64-unknown-linux-gnu
cargo build --target x86_64-unknown-linux-gnu --release

# Test (MUST use host target)
cargo test --target x86_64-unknown-linux-gnu
cargo test --target x86_64-unknown-linux-gnu --lib          # Unit tests only
cargo test --target x86_64-unknown-linux-gnu test_name      # Specific test

# Linting (MUST use host target)
cargo clippy --target x86_64-unknown-linux-gnu -- -D warnings

# Formatting (target-independent)
cargo fmt                          # Format code
cargo fmt --check                  # Check formatting

# Documentation (target-independent)
cargo doc --no-deps                # Generate docs
cargo doc --no-deps --open         # Generate and open in browser

# Coverage (host target, requires tarpaulin)
cargo tarpaulin --target x86_64-unknown-linux-gnu --out Html

# === DEVICE-SIDE COMMANDS (cross-compile for ARM) ===

# Build for device (ARM) - DEFAULT target
cargo build --release              # Release build for device
cargo build                        # Debug build for device
```

### Pre-Commit (Rust)

```bash
cd cross-compile/onvif-rust
cargo fmt && \
cargo clippy --target x86_64-unknown-linux-gnu -- -D warnings && \
cargo test --target x86_64-unknown-linux-gnu
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

# Deploy to camera (uploads ARM binary)
./deploy_onvif.sh [device_ip] [username] [password]
./deploy_onvif.sh 192.168.1.100 admin admin  # Example

# Run on device
./run_onvif.sh [device_ip] [username] [password] [release|debug]
./run_onvif.sh 192.168.1.100 admin admin debug  # Example

# Collect crash dumps
./collect_coredump.sh [device_ip] [username] [password]

# Analyze crash dumps
gdb ../cross-compile/onvif/out/onvifd_debug ../debugging/coredump/core.*
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

### System Utilities

```bash
# File operations
ls -la                             # List files with details
find . -name "*.rs"                # Find Rust files
grep -r "pattern" --include="*.rs" # Search in Rust files

# Process management
ps aux | grep onvifd               # Check running daemon
kill -9 PID                        # Force kill process

# Network
curl -X POST http://IP:PORT/onvif/device_service  # Test ONVIF endpoint
netstat -tlnp                      # Check listening ports
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
| Doc | Any | (target-independent) |

## CI/CD Notes

- GitHub Actions runs tests/lint with `--target x86_64-unknown-linux-gnu`
- Coverage reports uploaded to SonarCloud
- Security scans via Snyk (SAST + SCA)
- Quality gates must pass before merge
