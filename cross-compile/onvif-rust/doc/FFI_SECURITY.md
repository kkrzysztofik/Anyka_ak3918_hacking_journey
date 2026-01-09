# FFI Security Audit Guidelines

This document provides security audit guidelines and fuzzing targets for the FFI (Foreign Function Interface) code in `src/ffi/anyka_sdk.rs`.

## Overview

The FFI module provides safe Rust wrappers around the Anyka SDK C library. All unsafe operations are documented with SAFETY comments explaining why they are safe.

## Security Considerations

### Input Validation

All FFI functions must validate inputs before passing them to C functions:

1. **Enum Values**: Transmuted enum values must be validated to ensure they match valid C enum variants
2. **String Parameters**: C strings must be validated for null bytes and proper encoding
3. **Pointer Parameters**: Raw pointers must be checked for null before dereferencing
4. **Integer Bounds**: Integer parameters must be validated against hardware limits

### Bounds Checking

The following functions require bounds checking:

- `video_input_open()`: Device ID must be 0 (DEV0) - validated by `VideoDevice` type
- `ptz_turn()`: Direction must be valid enum variant - validated by `PtzDirection::to_direction_code()`
- `ptz_get_position()`: Motor must be valid enum variant - validated by `PtzMotor::to_device_id()`

### Error Code Mapping

SDK functions return negative values on error. All FFI wrappers check for `result < 0` and map to `AnykaError::SdkError`.

### Thread Safety

The SDK uses internal mutexes for thread safety:

- `VideoInput` is marked `Send` and `Sync` based on SDK's internal locking
- All SDK operations are protected by mutexes (documented in SAFETY comments)

## Fuzzing Targets

### High Priority Targets

1. **`video_input_open(device: VideoDevice)`**
   - Fuzz: Invalid device IDs (values outside 0)
   - Expected: Returns `ResourceUnavailable` error
   - Risk: Low (type system prevents invalid values)

2. **`ptz_turn(direction: PtzDirection)`**
   - Fuzz: Invalid direction codes
   - Expected: Type system prevents invalid values
   - Risk: Low (enum validation)

3. **`ptz_get_position(motor: PtzMotor)`**
   - Fuzz: Invalid motor IDs
   - Expected: Type system prevents invalid values
   - Risk: Low (enum validation)

4. **`ak_log(level: LogLevel, message: &str)`**
   - Fuzz: Messages with null bytes, invalid UTF-8
   - Expected: `CString::new()` returns error for null bytes
   - Risk: Medium (string validation)

### Medium Priority Targets

1. **String handling in `ak_log()`**
   - Test: Very long strings (>1MB)
   - Test: Strings with embedded nulls
   - Test: Invalid UTF-8 sequences
   - Expected: `CString::new()` handles validation

2. **Handle management**
   - Test: Double-close of handles
   - Test: Use-after-close
   - Expected: `Drop` implementation checks for null

### Low Priority Targets

1. **Enum transmute operations**
   - Test: Values outside enum range
   - Expected: Type system prevents invalid values
   - Risk: Very low (compile-time checked)

## Security Audit Checklist

- [x] All `unsafe` blocks have SAFETY comments
- [x] Input validation documented for each function
- [x] Bounds checking documented
- [x] Error handling documented
- [x] Thread safety documented (Send/Sync impls)
- [ ] Fuzzing harness created (optional, future work)
- [ ] Integration tests with invalid inputs
- [ ] Memory leak testing (handle cleanup)

## Known Limitations

1. **Transmute Usage**: Three functions use `std::mem::transmute` for enum conversion:
   - `video_input_open()`: `VideoDevice` → `video_dev_type`
   - `ptz_turn()`: `PtzDirection` → `ptz_turn_direction`
   - `ptz_get_position()`: `PtzMotor` → `ptz_device`

   All are safe because:
   - Type system ensures only valid enum values
   - Values are validated before transmute
   - NOSONAR annotations document safety

2. **Stub Implementations**: Stub implementations for testing don't perform actual FFI calls, reducing security risk in test environments.

## Recommendations

1. **Future Enhancement**: Consider replacing `transmute` with `#[repr(i32)]` and explicit matching (requires SDK enum definitions)

2. **Fuzzing**: Create fuzzing harness using `cargo-fuzz` or `libfuzzer` to test:
   - String inputs to `ak_log()`
   - Edge cases in enum conversions
   - Handle lifecycle management

3. **Static Analysis**: Use tools like `cargo-audit` and `cargo-clippy` to detect:
   - Unsafe code patterns
   - Potential memory safety issues
   - Undocumented unsafe blocks

## References

- [Rust FFI Guidelines](https://rust-lang.github.io/unsafe-code-guidelines/)
- [The Rustonomicon: FFI](https://doc.rust-lang.org/nomicon/ffi.html)
- [Anyka SDK Documentation](../../cross-compile/anyka_reference/)
