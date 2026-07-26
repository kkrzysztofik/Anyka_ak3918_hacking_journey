# Dependency Bump Design (App + Validation)

**Date:** 2026-07-26  
**Status:** Approved (approach 2 — latest including majors)  
**Branch:** work on `integrate-video` (or short-lived `chore/deps-bump` off it)

## Goal

Bump all application and validation dependencies to the newest published versions for Rust and Node, including major upgrades, then fix breakage so quality gates pass.

## Scope

### In scope

| Area | Paths |
|------|--------|
| Rust workspace | `cross-compile/Cargo.toml`, `cross-compile/onvif-rust/Cargo.toml`, `cross-compile/streaming-lib/Cargo.toml`, lockfile under `cross-compile/` |
| Validation Rust | `validation/rust/Cargo.toml`, `validation/rust/Cargo.lock` |
| WebUI Node | `cross-compile/www/package.json`, `cross-compile/www/package-lock.json` |

### Out of scope

- `.opencode/` (editor plugins)
- `mock-server/` (no package dependencies today)
- Vendored Rust toolchain / crosstool / system packages
- Dependabot / CI workflow redesign
- Intentionally dropping the workspace `openssl-src` path patch

## Approach

1. **Rust manifests:** raise direct dependency version requirements to the latest crates.io releases (including incompatible majors) using `cargo upgrade --incompatible` (cargo-edit) or equivalent manual edits when the tool is unavailable.
2. **Rust lockfiles:** refresh with the vendored toolchain (`source ./setenv.sh`; use `$CARGO`).
3. **Workspace alignment:** keep shared crates consistent across `onvif-rust` and `streaming-lib` (e.g. `tokio`, `axum`, `rand`, `bytes`, `reqwest`, `socket2`) so the workspace resolves cleanly.
4. **Preserve patch:** leave `[patch.crates-io] openssl-src = { path = "patches/openssl-src-300.2.3+3.2.1-full" }` intact unless a newer patch is explicitly required for build; do not “upgrade away” uClibc support.
5. **Node:** `npx npm-check-updates -u` in `cross-compile/www`, then `npm install`. Keep or refresh `overrides` only if still needed for security/resolution.
6. **Code fixes:** update call sites for breaking API changes; no compatibility shims.
7. **Verify:** host Rust fmt/clippy/test for both Rust trees; www lint, type-check, and test.

## Constraints

- Use vendored cargo from `toolchain/arm-anykav200-crosstool-ng/` via `setenv.sh`.
- Host Rust commands must use `--target x86_64-unknown-linux-gnu`.
- Follow project quality gates (`development-standards`, `quality-gates`).
- No file deletion without explicit permission.
- Prefer revising manifests/lockfiles and call sites in place; no alternate `Cargo.toml.v2` files.

## Success criteria

- Manifests and lockfiles reflect newest available direct deps (modulo the openssl-src path patch).
- `cross-compile`: `cargo fmt --check`, `clippy -D warnings`, `test` all pass on host target.
- `validation/rust`: host `clippy` and `test` pass.
- `cross-compile/www`: `npm run lint`, `type-check`, and `test` pass.
- No new unwrap/expect in production paths introduced while fixing breaks.

## Risks

| Risk | Mitigation |
|------|------------|
| Major tower/axum/reqwest/vite/eslint breaks | Fix call sites; pin only if a crate is unusable on edition 2024 / our toolchain |
| openssl / native-tls / uClibc interaction | Keep path patch; prefer rustls where already used (onvif-rust tests, validation) |
| Toolchain MSRV vs newest crates | If a crate requires newer rustc than vendored toolchain, pin to newest crate that builds and document in the plan notes |
| Transient crates.io/npm registry failures | Retry with network; do not invent versions |

## Non-goals

- Runtime performance tuning from dependency upgrades
- Rewriting features solely to adopt new crate APIs beyond what the bump requires
- Bumping transitive deps beyond what the resolvers select after direct bumps
