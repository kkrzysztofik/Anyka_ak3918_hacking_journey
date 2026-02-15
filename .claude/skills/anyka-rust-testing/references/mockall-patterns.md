# Mockall Patterns Reference

## Expectation Methods

### times()
```rust
.times(1)           // Exactly once
.times(2..5)        // Between 2 and 4 times
.times(..)          // Any number of times
.times(0)           // Never called (useful for negative tests)
```

### with() - Argument Matching
```rust
use mockall::predicate::*;

.with(eq(42))                           // Exact match
.with(ne(0))                            // Not equal
.with(gt(10))                           // Greater than
.with(lt(100))                          // Less than
.with(ge(1), le(10))                    // Multiple predicates
.with(always())                         // Any value
.with(function(|x| x % 2 == 0))         // Custom predicate
.with(str::starts_with("prefix"))       // String predicates
.with(in_iter(vec![1, 2, 3]))           // Value in collection
```

### returning() - Return Values
```rust
.returning(|_| Ok(()))                  // Simple return
.returning(|x| Ok(x * 2))               // Transform input
.returning(move |_| Ok(captured_val))   // Capture values
.returning({                             // Stateful returns
    let mut count = 0;
    move |_| {
        count += 1;
        Ok(count)
    }
})
```

### Sequences
```rust
let mut seq = mockall::Sequence::new();

mock.expect_init()
    .times(1)
    .in_sequence(&mut seq)
    .returning(|| Ok(()));

mock.expect_get_info()
    .times(1)
    .in_sequence(&mut seq)
    .returning(|| Ok(Info::default()));
```

## Common Patterns for This Project

### Platform Mock with Multiple Expectations
```rust
fn create_initialized_platform() -> MockPlatform {
    let mut mock = MockPlatform::new();

    mock.expect_init()
        .times(1)
        .returning(|| Ok(()));

    mock.expect_get_device_info()
        .returning(|| Ok(DeviceInfo {
            manufacturer: "Anyka".to_string(),
            model: "AK3918".to_string(),
            firmware_version: "1.0.0".to_string(),
            serial_number: "TEST123".to_string(),
            hardware_id: "HW001".to_string(),
        }));

    mock
}
```

### Testing Error Recovery
```rust
#[tokio::test]
async fn test_retry_mechanism() {
    let mut mock = MockPlatform::new();
    let call_count = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&call_count);

    mock.expect_get_device_info()
        .times(3)
        .returning(move || {
            let count = counter.fetch_add(1, Ordering::SeqCst);
            if count < 2 {
                Err(PlatformError::Temporary)
            } else {
                Ok(DeviceInfo::default())
            }
        });

    let service = DeviceService::new(mock);
    let result = service.get_device_info_with_retry(3).await;

    assert!(result.is_ok());
    assert_eq!(call_count.load(Ordering::SeqCst), 3);
}
```

### Callback/Handler Mocking
```rust
mock.expect_on_packet_handler()
    .returning(|callback| {
        // Store or invoke the callback
        Box::new(callback)
    });
```

### Generic Trait Mocking
```rust
mockall::mock! {
    pub Storage<T: Clone + Send + 'static> {}

    impl<T: Clone + Send + 'static> Storage<T> for Storage<T> {
        fn store(&self, key: &str, value: T) -> Result<(), StorageError>;
        fn retrieve(&self, key: &str) -> Result<Option<T>, StorageError>;
    }
}
```

## Async Trait Mocking

Always use `async_trait` with mockall:

```rust
use async_trait::async_trait;
use mockall::{automock, predicate::*};

#[automock]
#[async_trait]
pub trait AsyncService {
    async fn process(&self, data: Vec<u8>) -> Result<ProcessResult, Error>;
}

#[tokio::test]
async fn test_async_processing() {
    let mut mock = MockAsyncService::new();

    mock.expect_process()
        .with(predicate::function(|data: &Vec<u8>| !data.is_empty()))
        .times(1)
        .returning(|data| Ok(ProcessResult { size: data.len() }));

    let result = mock.process(vec![1, 2, 3]).await;
    assert!(result.is_ok());
}
```

## Verification

Mocks automatically verify expectations when dropped. For explicit verification:

```rust
#[tokio::test]
async fn test_with_checkpoint() {
    let mut mock = MockPlatform::new();

    mock.expect_init()
        .times(1)
        .returning(|| Ok(()));

    // Perform operations
    mock.init().await.unwrap();

    // Explicit checkpoint - verifies all expectations met so far
    mock.checkpoint();

    // Continue with more expectations...
}
```
