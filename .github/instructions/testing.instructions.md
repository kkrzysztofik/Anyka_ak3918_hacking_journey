---
applyTo: "**/tests/**,**/*_test.rs,**/*test*.rs"
description: "Testing standards and best practices"
---

# Testing Guidelines

## Rust Testing

### Test Organization

- Unit tests: Inline `mod tests` in source files
- Integration tests: `tests/` directory
- Benchmarks: `benches/` with criterion

### Running Tests

```bash
# Host-side testing (required for this cross-compile project)
cargo test --target x86_64-unknown-linux-gnu

# Unit tests only
cargo test --target x86_64-unknown-linux-gnu --lib

# With output
cargo test --target x86_64-unknown-linux-gnu -- --nocapture
```

### Test Naming Convention

Use pattern: `test_<function>_<scenario>_<expected_outcome>`

Examples:
- `test_device_get_info_success`
- `test_auth_invalid_credentials_returns_unauthorized`
- `test_media_create_profile_empty_name_returns_error`

### Mocking with mockall

- Use `#[automock]` attribute on traits
- Use `mock!` macro for complex mocks
- Set expectations with `expect_<method>()`
- Use `with(eq())` for argument matching
- Use `times(1)` for call count verification
- Use `returning()` for return values

### Async Testing

- Use `#[tokio::test]` for async tests
- Mock async traits with `#[async_trait]`
- Test timeout and cancellation scenarios

### Test Quality

- Test both success and error paths
- Test edge cases and boundary conditions
- Test with realistic data
- Keep tests independent and isolated
- Use descriptive assertion messages

## WebUI Testing (TypeScript)

### Frameworks

- Vitest for test runner
- Testing Library for component tests
- MSW for API mocking

### Selectors

Always use `data-testid` attributes:
```typescript
screen.getByTestId('device-panel-save-button')
```

Never use role, text, or class selectors.
