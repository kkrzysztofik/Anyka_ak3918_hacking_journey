# Quality Gates - Anyka AK3918 Project (Rust)

## Pre-Commit Checklist

Before committing any code or marking a task as complete, you MUST verify the following:

### 1. Compilation

- [ ] Code compiles without errors (`cargo build`).
- [ ] Code compiles for the target architecture (if applicable).

### 2. Testing

- [ ] All unit tests pass (`cargo test`).
- [ ] All integration tests pass.
- [ ] New functionality has corresponding tests.

### 3. Code Quality

- [ ] Code is formatted (`cargo fmt`).
- [ ] No clippy warnings (`cargo clippy -- -D warnings`).
- [ ] No unnecessary `unwrap()` or `expect()` calls.
- [ ] Error handling is robust and idiomatic.

### 4. Documentation

- [ ] Public API is documented with doc comments (`///`).
- [ ] `README.md` is updated if necessary.
- [ ] `cargo doc` generates documentation without warnings.

### 5. Security

- [ ] No hardcoded secrets.
- [ ] Input validation is in place.
- [ ] Dependencies are secure (check `cargo audit` if available).

## Review Process

When reviewing code (self-review or peer review), focus on:

- **Safety**: Usage of `unsafe` (should be minimal and justified).
- **Concurrency**: Correct usage of async/await and synchronization primitives.
- **Performance**: Avoid unnecessary allocations or blocking operations.
- **Maintainability**: Clear naming, modular design, and lack of code duplication.
