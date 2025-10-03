# Testing Guide

## Test Naming Convention (MANDATORY)

- **Unit tests**: Must follow `test_unit_<module>_<functionality>` pattern
- **Integration tests**: Must follow `test_integration_<service>_<functionality>` pattern

**Examples**:

- Unit test: `test_unit_memory_manager_init`, `test_unit_http_auth_verify_credentials_success`
- Integration test: `test_integration_ptz_absolute_move_functionality`, `test_integration_media_profile_operations`

## Unit Testing (MANDATORY)

**Test Suite**: Dynamic test count across 17 suites (unit and integration tests)

- **Run all tests**: `make test` (from project root or tests directory)
- **Run unit tests only**: `make test-unit`
- **Run integration tests only**: `make test-integration`
- **List available test suites**: `make test-list`
- **Run specific suite**: `make test SUITE=ptz-service`
- **Run multiple suites**: `make test SUITE=ptz-service,ptz-callbacks,ptz-integration`
- **Filter by type and suite**: `make test-unit SUITE=http-auth,http-metrics`

**Requirements**:

- All utility functions must have corresponding unit tests
- Tests must cover success cases, error cases, and edge conditions
- Use CMocka framework for isolated unit testing
- Generate coverage reports: `make test-coverage-html`
- Achieve high code coverage on utility functions
- Tests run on development machine (native compilation)

## CMocka-Based Mock Usage (MANDATORY)

### Mock Framework Requirements

- **ONLY CMocka mocks** are allowed for unit testing - no other mocking frameworks
- **Mock Implementation Pattern**: All mocks must use CMocka's `--wrap` linker mechanism with `__wrap_` prefix
- **Mock Structure**: Each mock file must follow the pattern `*_mock.c` and `*_mock.h`

### Mock Function Implementation

```c
// Mock function implementation
platform_result_t __wrap_platform_init(void) {
  function_called();                    // Track function calls
  return (platform_result_t)mock();    // Return configured value
}

// Mock function with parameter validation
platform_result_t __wrap_platform_ptz_move(float pan, float tilt) {
  check_expected(pan);                  // Validate pan parameter
  check_expected(tilt);                 // Validate tilt parameter
  function_called();                    // Track function calls
  return (platform_result_t)mock();    // Return configured value
}
```

### Mock Configuration in Tests

```c
// Configure mock return value
will_return(__wrap_platform_init, PLATFORM_SUCCESS);

// Configure mock parameter expectations
expect_value(__wrap_platform_ptz_move, pan, 90.0f);
expect_value(__wrap_platform_ptz_move, tilt, 45.0f);
will_return(__wrap_platform_ptz_move, PLATFORM_SUCCESS);

// Call function under test
platform_result_t result = ptz_adapter_absolute_move(90.0f, 45.0f, 50.0f);
assert_int_equal(result, PLATFORM_SUCCESS);
```

### Mock Call Verification

```c
// Verify function was called exactly once
assert_int_equal(1, platform_mock_get_ptz_init_call_count());

// Verify function was called with specific parameters
int pan, tilt, speed;
assert_int_equal(1, platform_mock_get_last_ptz_absolute_move(&pan, &tilt, &speed));
assert_int_equal(90, pan);
assert_int_equal(45, tilt);
```

### Mock State Management

```c
// Test setup
static int test_setup(void** state) {
  platform_mock_init();                    // Initialize mock state
  platform_ptz_mock_init();               // Initialize specific mock
  return 0;
}

// Test teardown
static int test_teardown(void** state) {
  platform_ptz_mock_cleanup();            // Cleanup specific mock
  platform_mock_cleanup();                // Cleanup mock state
  return 0;
}
```

### Mock Error Simulation

```c
// Simulate platform initialization failure
platform_mock_enable_ptz_error(PLATFORM_ERROR);
will_return(__wrap_platform_init, PLATFORM_ERROR);

platform_result_t result = ptz_adapter_init();
assert_int_equal(result, PLATFORM_ERROR);

// Disable error simulation for subsequent tests
platform_mock_disable_ptz_error();
```

### Mock Parameter Validation

```c
// For pointer parameters
check_expected_ptr(ptr);

// For value parameters
check_expected(value);

// For string parameters
check_expected_ptr(string);
```

### Mock Return Value Configuration

```c
// Single return value
will_return(__wrap_platform_init, PLATFORM_SUCCESS);

// Multiple return values (for multiple calls)
will_return(__wrap_platform_init, PLATFORM_SUCCESS);
will_return(__wrap_platform_init, PLATFORM_ERROR);

// Return values for output parameters
will_return(__wrap_platform_vi_open, (uintptr_t)mock_handle);
will_return(__wrap_platform_vi_open, PLATFORM_SUCCESS);
```

### Mock File Organization

```
tests/src/mocks/
├── platform_mock.c          # Platform abstraction mocks
├── platform_mock.h
├── network_mock.c           # Network function mocks
├── network_mock.h
├── config_mock.c            # Configuration mocks
├── config_mock.h
└── memory_mock.c            # Memory management mocks
```

### Mock Naming Conventions

- Mock implementation: `__wrap_<original_function_name>`
- Mock header: `<module>_mock.h`
- Mock source: `<module>_mock.c`
- Mock state functions: `<module>_mock_init()`, `<module>_mock_cleanup()`

### Mock Testing Requirements

- **ALL external dependencies** must be mocked (platform, network, file I/O, etc.)
- **NO real hardware calls** in unit tests
- **NO real network operations** in unit tests
- **NO real file system operations** in unit tests
- **Mock all platform abstraction functions** used by code under test
- **Mock all utility functions** that are not the primary focus of the test

### Mock Testing Best Practices

- **One mock per test**: Each test should focus on one specific function or behavior
- **Clear mock setup**: Use descriptive mock configuration that makes test intent clear
- **Mock isolation**: Each test should be independent and not rely on mock state from other tests
- **Mock cleanup**: Always clean up mock state between tests
- **Mock verification**: Always verify mock calls and parameters in assertions
- **Mock error testing**: Test both success and failure paths using mocks

## Compilation Testing (MANDATORY)

- Use native build: `make -C cross-compile/onvif`
- Fix all compiler warnings and errors
- Verify no undefined behavior
- Check for unused variables and functions

## Code Quality Commands (MANDATORY)

### Code Linting

```bash
# Code Linting (MANDATORY)
./cross-compile/onvif/scripts/lint_code.sh                    # Lint all C files
./cross-compile/onvif/scripts/lint_code.sh --check            # Check for issues (exit 1 if found)
./cross-compile/onvif/scripts/lint_code.sh --file src/path/to/file.c  # Lint specific file
./cross-compile/onvif/scripts/lint_code.sh --changed          # Lint only changed files
./cross-compile/onvif/scripts/lint_code.sh --format           # Also check formatting
./cross-compile/onvif/scripts/lint_code.sh --severity error   # Fail only on errors
```

### Code Formatting

```bash
# Code Formatting (MANDATORY)
./cross-compile/onvif/scripts/format_code.sh                  # Format all C files
./cross-compile/onvif/scripts/format_code.sh --check          # Check formatting (exit 1 if issues)
./cross-compile/onvif/scripts/format_code.sh --files src/path/to/file.c  # Format specific files
./cross-compile/onvif/scripts/format_code.sh --dry-run        # Show what would be changed
```

### Function Order Compliance (MANDATORY)

- **Verify function ordering** follows project guidelines: definitions at top, execution logic at bottom
- **Check global variables** are placed at the top of files after includes
- **Validate include ordering**: system headers → third-party → project headers
- **Ensure proper spacing** and formatting between function groups
- **Use linting script** with `--format` flag to check function ordering compliance

## Testing Commands

```bash
# Unit Testing (MANDATORY)
make test                    # Run all unit tests
make test-utils             # Run utility unit tests
make test-coverage          # Run tests with coverage
make test-coverage-html     # Generate HTML coverage report
make test-coverage-report   # Generate coverage summary
make test-valgrind          # Run tests with valgrind

# Install unit test dependencies
./cross-compile/onvif/tests/install_dependencies.sh

# Run static analysis (if available)
cppcheck cross-compile/onvif/src

# Check for memory leaks with valgrind (if available)
valgrind --leak-check=full cross-compile/onvif/out/onvifd
```
