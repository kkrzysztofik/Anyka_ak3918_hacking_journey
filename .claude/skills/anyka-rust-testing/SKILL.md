---
name: anyka-rust-testing
description: |
  Rust testing specialist for Anyka camera projects using mockall, tokio, and the project's testing framework.
  Triggers on: "write tests", "mock Platform", "mockall", "unit test Rust", "test coverage", "testing patterns".
version: 1.0.0
---

# Rust Testing for Anyka Camera Projects

Write unit tests for Rust code in the onvif-rust and streaming-lib projects using mockall, tokio, and project testing conventions. Follow these guidelines strictly.

## Cross-Compilation Awareness

The default target is ARM (`armv5te-unknown-linux-uclibceabi`). Always specify the x86_64 target for host-side testing:

```bash
cargo test --target x86_64-unknown-linux-gnu
cargo test --target x86_64-unknown-linux-gnu --lib  # Unit tests only
cargo test --target x86_64-unknown-linux-gnu test_name -- --nocapture  # Specific test with output
```

## Test Naming Convention

Follow the pattern: `test_<function>_<scenario>_<expected_outcome>`. Examples:

- `test_device_get_info_success`
- `test_device_get_info_unauthorized_returns_error`
- `test_media_create_profile_invalid_name_returns_validation_error`
- `test_brightness_set_value_out_of_range_returns_error`

## Test Module Structure

Place unit tests in inline `mod tests` blocks within the source file:

```rust
// src/onvif/device/handlers.rs
impl DeviceService {
    pub async fn get_device_info(&self) -> Result<DeviceInfo, OnvifError> {
        // implementation
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;

    fn create_test_service() -> DeviceService {
        // Test helper - reuse across tests
        DeviceService::new(MockPlatform::new(), test_config())
    }

    #[tokio::test]
    async fn test_get_device_info_success() {
        let service = create_test_service();
        let result = service.get_device_info().await;
        assert!(result.is_ok());
    }
}
```

## Mockall Patterns

### Define Traits with #[automock]

```rust
use mockall::{automock, predicate::*};
use async_trait::async_trait;

#[automock]
#[async_trait]
pub trait Platform {
    async fn init(&self) -> Result<(), PlatformError>;
    async fn get_device_info(&self) -> Result<DeviceInfo, PlatformError>;
    async fn set_brightness(&self, level: u8) -> Result<(), PlatformError>;
    async fn ptz_move(&self, pan: f32, tilt: f32, zoom: f32) -> Result<(), PlatformError>;
}
```

### Use mockall::mock! for External Traits

When you cannot use `#[automock]` on the trait definition:

```rust
mockall::mock! {
    pub Platform {}

    #[async_trait]
    impl Platform for Platform {
        async fn init(&self) -> Result<(), PlatformError>;
        async fn get_device_info(&self) -> Result<DeviceInfo, PlatformError>;
        async fn set_brightness(&self, level: u8) -> Result<(), PlatformError>;
    }
}
```

### Setting Expectations

```rust
#[tokio::test]
async fn test_brightness_setting() {
    let mut mock = MockPlatform::new();

    // Expect exactly one call with specific argument
    mock.expect_set_brightness()
        .with(eq(75))
        .times(1)
        .returning(|_| Ok(()));

    let result = mock.set_brightness(75).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_ptz_move_with_range() {
    let mut mock = MockPlatform::new();

    // Use predicate functions for ranges
    mock.expect_ptz_move()
        .with(
            predicate::in_iter(-180.0..=180.0),  // pan range
            predicate::in_iter(-90.0..=90.0),    // tilt range
            predicate::always()                   // any zoom
        )
        .times(1)
        .returning(|_, _, _| Ok(()));

    let result = mock.ptz_move(90.0, 45.0, 1.0).await;
    assert!(result.is_ok());
}
```

### Return Different Values on Successive Calls

```rust
#[tokio::test]
async fn test_retry_on_failure() {
    let mut mock = MockPlatform::new();

    mock.expect_get_device_info()
        .times(3)
        .returning({
            let mut count = 0;
            move || {
                count += 1;
                if count < 3 {
                    Err(PlatformError::Temporary)
                } else {
                    Ok(DeviceInfo::default())
                }
            }
        });

    // Test retry logic
}
```

## Error Testing

Test both success and error paths:

```rust
#[tokio::test]
async fn test_set_brightness_out_of_range() {
    let mut mock = MockPlatform::new();

    mock.expect_set_brightness()
        .with(eq(150))  // Invalid: > 100
        .times(1)
        .returning(|_| Err(PlatformError::InvalidParameter("brightness out of range".into())));

    let result = mock.set_brightness(150).await;

    assert!(result.is_err());
    match result {
        Err(PlatformError::InvalidParameter(msg)) => {
            assert!(msg.contains("out of range"));
        }
        _ => panic!("Expected InvalidParameter error"),
    }
}
```

## Test Helpers

Create reusable test fixtures in the test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> Config {
        Config {
            device_name: "Test Camera".to_string(),
            manufacturer: "Anyka".to_string(),
            model: "AK3918".to_string(),
            ..Default::default()
        }
    }

    fn create_test_user(level: UserLevel) -> User {
        User {
            username: format!("test_{:?}", level).to_lowercase(),
            password: "test123".to_string(),
            level,
        }
    }

    fn create_mock_platform_with_device_info() -> MockPlatform {
        let mut mock = MockPlatform::new();
        mock.expect_get_device_info()
            .returning(|| Ok(DeviceInfo {
                serial_number: "TEST123".to_string(),
                hardware_id: "HW001".to_string(),
                firmware_version: "1.0.0".to_string(),
            }));
        mock
    }
}
```

## Async Test Patterns

Use `#[tokio::test]` for async tests:

```rust
#[tokio::test]
async fn test_concurrent_operations() {
    let service = Arc::new(create_test_service());

    let handles: Vec<_> = (0..10)
        .map(|i| {
            let svc = Arc::clone(&service);
            tokio::spawn(async move {
                svc.get_device_info().await
            })
        })
        .collect();

    for handle in handles {
        assert!(handle.await.unwrap().is_ok());
    }
}
```

## Test Coverage

Run coverage reports with tarpaulin:

```bash
cargo tarpaulin --target x86_64-unknown-linux-gnu --out Html --output-dir coverage
cargo tarpaulin --target x86_64-unknown-linux-gnu --out Xml  # For CI
```

## Integration with Existing Tests

Before writing new tests, examine existing test patterns:

```bash
# Find existing tests in the project
grep -r "#\[tokio::test\]" cross-compile/onvif-rust/src/
grep -r "MockPlatform" cross-compile/onvif-rust/src/
```

## Common Assertions

```rust
// Value assertions
assert!(result.is_ok());
assert!(result.is_err());
assert_eq!(actual, expected);
assert_ne!(actual, unexpected);

// String assertions
assert!(string.contains("expected"));
assert!(string.starts_with("prefix"));

// Error matching
assert!(matches!(result, Err(OnvifError::AuthenticationFailed)));

// Collection assertions
assert!(vec.is_empty());
assert_eq!(vec.len(), 5);
assert!(vec.contains(&item));
```

## Reference

For detailed mockall patterns, see `references/mockall-patterns.md` in this skill directory.
