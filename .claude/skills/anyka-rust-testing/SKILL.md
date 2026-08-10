---
name: anyka-rust-testing
description: Use when writing or debugging Rust tests for onvif-rust and streaming-lib (mockall, tokio, unit/integration tests, host-side test commands, test naming, error-path tests).
version: 2.0.0
---

# Rust Testing for Anyka Camera Projects

Write tests for `cross-compile/onvif-rust` and `cross-compile/streaming-lib` using Rust's built-in test framework, `tokio`, and `mockall`. Follow the project conventions below.

## Toolchain & Running Tests

Use the vendored toolchain. **Never bare `cargo`**:

```bash
source ./setenv.sh                      # exports $CARGO, $RUSTC, sets CARGO_HOME
cd cross-compile/onvif-rust
$CARGO test --target x86_64-unknown-linux-gnu
$CARGO test --target x86_64-unknown-linux-gnu --lib        # unit tests only
$CARGO test --target x86_64-unknown-linux-gnu test_name -- --nocapture
$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
$CARGO fmt --check
```

## Test Naming Convention

`test_<component>_<scenario>_<expected_outcome>`:

- `test_service_handler_unknown_action_device`
- `test_service_handler_invalid_xml`
- `test_apply_set_scopes_keeps_fixed_replaces_configurable`
- `test_sps_fixtures_have_correct_nal_type`

## Where Tests Live

- **Unit tests**: inline `#[cfg(test)] mod tests` next to the code (use `super::*`).
- **Integration tests**: `tests/` (e.g. `tests/dispatcher_extended.rs`, `tests/namespace_serialization_tests.rs`, `tests/security_inputs.rs`).
- **Async tests**: `#[tokio::test]`.
- Test helper modules/fixtures: `src/codec/test_fixtures.rs` (SPS/PPS), `tests/fixtures/`, `tests/data/`.

## Mockall Patterns

### `#[cfg_attr(test, automock)]` on trait definitions (project standard)

The platform traits in `src/platform/common/traits.rs` use `#[cfg_attr(test, automock)]` so mocks exist only in test builds:

```rust
#[cfg_attr(test, automock)]
#[async_trait]
pub trait VideoInput: Send + Sync {
    async fn open(&self) -> PlatformResult<()>;
    async fn get_resolution(&self) -> PlatformResult<Resolution>;
}
```

Mock name is `MockVideoInput`. Traits using `#[automock]` need `Send + Sync` and async methods require `#[async_trait]` (mockall generates the mock via `mock!` internally).

### Setting expectations

```rust
let mut mock = MockPlatform::new();

mock.expect_set_brightness()
    .with(eq(75))
    .times(1)
    .returning(|_| Ok(()));

mock.expect_ptz_move()
    .with(predicate::in_iter(-180.0..=180.0), predicate::in_iter(-90.0..=90.0), predicate::always())
    .times(1)
    .returning(|_, _, _| Ok(()));
```

### Sequential returns (retry logic)

```rust
mock.expect_get_device_info()
    .times(3)
    .returning({
        let mut count = 0;
        move || {
            count += 1;
            if count < 3 { Err(PlatformError::Temporary) } else { Ok(DeviceInfo::default()) }
        }
    });
```

### Async trait mocks

For traits that can't use `#[automock]` (external/third-party), use `mockall::mock!` with `#[async_trait]`:

```rust
mockall::mock! {
    pub Platform {}
    #[async_trait]
    impl Platform for Platform {
        async fn init(&self) -> Result<(), PlatformError>;
        async fn get_device_info(&self) -> Result<DeviceInfo, PlatformError>;
    }
}
```

## Error-Path Testing

Test success and failure paths, and match on the exact error variant:

```rust
#[tokio::test]
async fn test_service_handler_invalid_xml() {
    let service = create_test_service();
    let result = service.handle_operation("GetDeviceInformation", "<InvalidXml><Broken").await;
    assert!(matches!(result, Err(OnvifError::WellFormed(_))));
}

#[tokio::test]
async fn test_service_handler_unknown_action_device() {
    let service = create_test_service();
    let result = service.handle_operation("UnknownAction", "<test/>").await;
    assert!(matches!(result, Err(OnvifError::ActionNotSupported(_))));
}
```

See the `onvif-service-impl` skill for the full `OnvifError` variant list (`WellFormed`, `InvalidArgVal{subcode, reason}`, `ActionNotSupported`, `NotFound`, etc.).

## Test Helpers

Create small helper fns in `mod tests` and reuse:

```rust
fn create_test_service() -> DeviceService {
    let mut mock = MockPlatform::new();
    mock.expect_get_device_info()
        .returning(|| Ok(DeviceInfo { serial_number: "TEST".into(), ..Default::default() }));
    DeviceService::with_config_and_platform(
        Arc::new(UserStorage::new()), test_config(), Arc::new(mock))
}
```

## Common Assertions

```rust
assert!(result.is_ok());
assert!(matches!(result, Err(OnvifError::NotAuthorized(_))));
assert_eq!(packets[0].header.marker, 1);
assert!(packets[packets.len()-1].payload[1] & 0x40 == 0x40);  // FU_END bit
```

## Reference

Load `.serena/memories/testing-framework.md` and `.serena/memories/development-standards.md` before writing tests. See the `rtsp-rtp-streaming` skill for packer/unpacker test patterns (mock IO + `on_packet_handler`).
