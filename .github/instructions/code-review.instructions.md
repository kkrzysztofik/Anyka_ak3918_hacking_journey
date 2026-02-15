---
applyTo: "**"
description: "Code review standards and GitHub PR guidelines"
---

# Code Review Guidelines

## Before Requesting Review

Ensure all quality gates pass:

```bash
cargo fmt
cargo clippy --target x86_64-unknown-linux-gnu -- -D warnings
cargo test --target x86_64-unknown-linux-gnu
```

## Review Checklist

### Rust Code

- [ ] **Naming**: snake_case for vars/functions, CamelCase for types
- [ ] **Error Handling**: No `unwrap()`/`expect()` in production paths
- [ ] **Unsafe Code**: Minimal, justified, documented with SAFETY comment
- [ ] **Async**: Uses `tokio::sync` primitives, no blocking calls
- [ ] **Logging**: Uses `tracing`, no `println!`
- [ ] **Testing**: New code has corresponding tests
- [ ] **Documentation**: Public APIs have doc comments

### Security

- [ ] Input validation implemented
- [ ] No information leakage in errors
- [ ] Timing-safe credential comparison
- [ ] No hardcoded secrets

### Performance

- [ ] Memory-efficient for 24MB target
- [ ] No unnecessary allocations
- [ ] Appropriate async patterns

## Pull Request Standards

### PR Title

Use conventional commit format:
- `feat: add PTZ continuous move support`
- `fix: handle empty profile response`
- `refactor: extract auth middleware`
- `docs: update API documentation`
- `test: add media service unit tests`

### PR Description

Include:
- What changed and why
- Testing performed
- Breaking changes (if any)
- Related issues

### Review Etiquette

- Be constructive and specific
- Suggest alternatives, not just problems
- Approve when satisfied
- Use "Request changes" sparingly
