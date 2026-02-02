# Codex Instructions (onvif-rust)

This directory contains the Rust ONVIF 24.12 implementation for Anyka AK3918-class devices (memory-constrained target).

## Role

Act as a Senior Embedded Rust Engineer focused on:
- ONVIF SOAP/XML correctness (quick-xml)
- Security hardening (auth + XML safety)
- Low-allocation, memory-aware design (24MB budget)
- Testability via traits + `mockall`

## Mandatory docs to load (before changes)

When working in this subtree, load and follow:
- `.serena/memories/development-standards.md` (Rust conventions, host-target rules)
- `.serena/memories/testing-framework.md` (mocking + test patterns)
- `.serena/memories/quality-gates.md` (review checklist and required gates)
- `.serena/memories/security-guidelines.md` (input validation, XML safety, auth rules)

## Toolchain & targets

This repo vendors a cross toolchain. Use its cargo for all commands:

```bash
export CARGO=../../toolchain/arm-anykav200-crosstool-ng/bin/cargo
```

Host-side quality gates (required for this cross project):

```bash
cd cross-compile/onvif-rust
$CARGO fmt
$CARGO fmt --check
$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
$CARGO test --target x86_64-unknown-linux-gnu
```

## Non-negotiable rules (summary)

- No `unwrap()` / `expect()` / `panic!()` in production paths.
- Document every `unsafe` with a `// SAFETY:` justification.
- Use `tracing` (no `println!`).
- Prefer borrowing over cloning; avoid allocations on hot paths.
- Tests must be added/updated for any behavior change.

## Testing conventions

- Unit tests: `mod tests` next to code.
- Integration tests: `tests/`.
- Async tests: `#[tokio::test]`.
- Test naming: `test_<component>_<scenario>_<expected_outcome>`.
- Mocking: `mockall` (`#[automock]` on traits or `mock!` for async traits).

