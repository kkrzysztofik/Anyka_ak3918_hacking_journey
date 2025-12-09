# Testing Framework - Anyka AK3918 Project (Rust)

## Testing Strategy

The project relies on a strong automated testing culture using Rust's built-in testing framework.

### Test Types

1. **Unit Tests**:
    - Located within the source file (usually in a `mod tests` block at the bottom).
    - Test individual functions and logic in isolation.
    - Use `#[test]` or `#[tokio::test]`.

2. **Integration Tests**:
    - Located in the `tests/` directory.
    - Test the public API of the crate or interactions between multiple modules.
    - Each file in `tests/` is compiled as a separate crate.

### Mocking

- Use **`mockall`** for mocking traits and structs in unit tests.
- Use **`wiremock`** for mocking HTTP services if needed (though `axum` testing often avoids this).

### Running Tests

- **Run All Tests**: `cargo test`
- **Run Specific Test**: `cargo test test_name`
- **Run Ignored Tests**: `cargo test -- --ignored`
- **Fail Fast**: `cargo test -- --nocapture` (to see output immediately)

### Test Naming

- Tests should have descriptive names indicating what is being tested and the expected outcome.
  - `fn test_device_get_info_success()`
  - `fn test_media_create_profile_invalid_name()`

### Continuous Integration

- All tests must pass in CI.
- Do not commit broken tests.
