# Quality Gates - Anyka AK3918 Project

## Pre-Commit Checklist

### Rust Workspace (onvif-rust + streaming-lib + anyka-init)

**⚠️ Cross-compile note**: Use `--target x86_64-unknown-linux-gnu` for host-side operations. First load the vendored toolchain with `source ./setenv.sh` from the repo root (exports `$CARGO`, `$RUSTC`, `$RUSTDOC`). Never use bare `cargo` or `rustup`.

Commands from `cross-compile/` apply to the entire workspace.

```bash
cd cross-compile
$CARGO fmt && \
$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings && \
$CARGO test --target x86_64-unknown-linux-gnu
```

| Gate | Command | Requirement |
|------|---------|-------------|
| Formatting | `$CARGO fmt --check` | No changes needed (workspace) |
| Linting | `$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings` | Zero warnings (workspace) |
| Unit Tests | `$CARGO test --target x86_64-unknown-linux-gnu` | All pass (workspace) |
| Build (device) | `$CARGO build --release` | No errors (workspace) |
| Documentation | `$CARGO doc --target x86_64-unknown-linux-gnu --no-deps` | No warnings |

### WebUI Frontend (www)

```bash
cd cross-compile/www
npm run lint && npm run type-check && npm run test
```

| Gate | Command | Requirement |
|------|---------|-------------|
| Linting | `npm run lint` | Zero warnings |
| Type Check | `npm run type-check` | No errors |
| Unit Tests | `npm run test` | All pass |
| Build | `npm run build` | No errors |
| Formatting | `npm run prettier` | Consistent style |

## Code Review Checklist

### Rust Code

- [ ] **Naming**: snake_case for vars/functions, CamelCase for types
- [ ] **Error Handling**: No `unwrap()`/`expect()` in production paths
- [ ] **Unsafe Code**: Minimal, justified, documented with SAFETY comment
- [ ] **Async**: Uses `tokio::sync` primitives, no blocking calls
- [ ] **Logging**: Uses `tracing`, no `println!`
- [ ] **Testing**: New code has corresponding tests
- [ ] **Documentation**: Public APIs have doc comments

### WebUI Code

- [ ] **TypeScript**: Strict mode, no `any` types
- [ ] **Components**: Uses shadcn/ui, no custom primitives
- [ ] **Testing**: Uses `data-testid` selectors
- [ ] **Validation**: Zod schemas for forms and API responses
- [ ] **Error Handling**: React Query with proper error states
- [ ] **Accessibility**: Radix UI components maintain a11y

## Security Checklist

### Input Validation
- [ ] All user inputs validated using `validator` crate or Zod
- [ ] XML inputs checked for XXE and XML bombs
- [ ] Path inputs validated against directory traversal
- [ ] String lengths bounded appropriately

### Authentication
- [ ] Timing-safe credential comparison (`constant_time_eq`)
- [ ] Passwords hashed with Argon2
- [ ] Session management with nonce freshness
- [ ] Rate limiting on auth endpoints

### Memory Safety (Rust)
- [ ] No data races in concurrent code
- [ ] Proper synchronization with `Arc`, `Mutex`, `RwLock`
- [ ] Resource cleanup in error paths
- [ ] Memory limits respected (24MB embedded target)

### Network Security
- [ ] No information leakage in error messages
- [ ] HTTPS/TLS for production deployments
- [ ] Rate limiting implemented
- [ ] No hardcoded secrets

## CI/CD Gates

### Pull Request Requirements

| Check | Tool | Status |
|-------|------|--------|
| Rust Lint | clippy | Must pass |
| Rust Tests | `$CARGO test` | Must pass |
| WebUI Lint | ESLint | Must pass |
| WebUI Tests | Vitest | Must pass |
| Type Check | TypeScript | Must pass |
| Security Scan | Snyk | Review findings |
| Quality Gate | SonarCloud | Must pass |

### Branch Protection

- All PRs require at least 1 approval
- CI must pass before merge
- No direct pushes to `main`
- Squash merge preferred

## Performance Checklist (WebUI)

- [ ] Initial load < 3s on local network
- [ ] Page transitions < 500ms
- [ ] Gzip/Brotli compression enabled
- [ ] Code splitting for routes
- [ ] No console.log in production

## Documentation Checklist

- [ ] README updated if API changes
- [ ] Inline comments for complex logic
- [ ] Public APIs have doc comments
- [ ] CHANGELOG updated for releases

## Release Checklist

1. [ ] All CI checks pass
2. [ ] Version bumped in Cargo.toml / package.json
3. [ ] CHANGELOG updated
4. [ ] Security audit clean (`$CARGO audit`, `npm audit`)
5. [ ] Manual testing on target device
6. [ ] Documentation reviewed
7. [ ] Tag created and pushed
