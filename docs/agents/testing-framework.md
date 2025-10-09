# Testing Framework - Anyka AK3918 Project

## Test Naming Convention (MANDATORY)

- **Unit tests**: Must follow `test_unit_<module>_<functionality>` pattern
- **Integration tests**: Must follow `test_integration_<service>_<functionality>` pattern

**Examples**:

- Unit test: `test_unit_memory_manager_init`, `test_unit_http_auth_verify_credentials_success`
- Integration test: `test_integration_ptz_absolute_move_functionality`, `test_integration_media_profile_operations`

## Unified Test Runner System

The project uses a sophisticated unified test runner that consolidates **196 tests across 17 test suites** into a single executable. This system provides flexible filtering, comprehensive output logging, and efficient test execution.

### Test Suite Structure

The unified runner organizes tests into the following **17 test suites**:

#### Unit Test Suites (12 suites)

- **memory-utils** - Memory Management Utilities
- **logging-utils** - Logging Utilities
- **http-auth** - HTTP Authentication
- **http-metrics** - HTTP Metrics
- **gsoap-protocol** - gSOAP Protocol
- **gsoap-response** - gSOAP Response Generation
- **service-dispatcher** - Service Dispatcher
- **ptz-service** - PTZ Service
- **ptz-callbacks** - PTZ Callbacks
- **ptz-adapter** - PTZ Adapter
- **media-utils** - Media Utilities
- **media-callbacks** - Media Callbacks
- **imaging-callbacks** - Imaging Callbacks

#### Integration Test Suites (5 suites)

- **ptz-integration** - PTZ Service Integration
- **media-integration** - Media Service Integration
- **device-integration** - Device Service Integration
- **imaging-integration** - Imaging Service Integration
- **soap-errors** - SOAP Error Handling

### OUT.log Output Mechanism

The unified test runner automatically creates an `OUT.log` file in the tests directory that captures:

- **Dual Output**: All test output is written to both console and `OUT.log` simultaneously
- **Complete Test Logs**: Full test execution details, debug output, and results
- **Persistent Records**: Test results are preserved for analysis and debugging
- **Real-time Monitoring**: Output appears in both console and log file in real-time

**Output Location**: `cross-compile/onvif/tests/OUT.log`

## Test Execution Commands

### Basic Test Execution

```bash
# Run all 196 tests across 17 suites (output to console + OUT.log)
make test

# Run only unit tests (12 suites)
make test-unit

# Run only integration tests (5 suites)
make test-integration

# List all available test suites with test counts
make test-list
```

### Suite-Specific Execution

```bash
# Run specific suite
make test SUITE=ptz-service

# Run multiple suites (comma-separated)
make test SUITE=ptz-service,ptz-callbacks,ptz-integration

# Run unit tests for specific suites
make test-unit SUITE=http-auth,http-metrics

# Run integration tests for specific suites
make test-integration SUITE=ptz-integration,media-integration
```

### Advanced Filtering

```bash
# Run all PTZ-related tests (unit + integration)
make test SUITE=ptz-service,ptz-callbacks,ptz-adapter,ptz-integration

# Run all gSOAP-related tests
make test SUITE=gsoap-protocol,gsoap-response

# Run all media-related tests
make test SUITE=media-utils,media-callbacks,media-integration
```

### Direct Runner Usage

```bash
# Direct runner with advanced options
./out/test_runner --type=unit --suite=ptz-service
./out/test_runner --type=integration --suite=ptz-integration,media-integration
./out/test_runner --list
./out/test_runner --help
```

## Unit Testing (MANDATORY)

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

```text
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

## Testing Commands

### Unified Test Runner Commands

```bash
# Basic test execution (MANDATORY)
make test                    # Run all 196 tests across 17 suites
make test-unit              # Run only unit tests (12 suites)
make test-integration       # Run only integration tests (5 suites)
make test-list              # List all available test suites with counts

# Suite-specific execution
make test SUITE=ptz-service                    # Run specific suite
make test SUITE=ptz-service,ptz-callbacks      # Run multiple suites
make test-unit SUITE=http-auth,http-metrics    # Unit tests for specific suites
make test-integration SUITE=ptz-integration    # Integration tests for specific suites

# Coverage and analysis
make test-coverage          # Run tests with coverage
make test-coverage-html     # Generate HTML coverage report
make test-coverage-report   # Generate coverage summary
make test-valgrind          # Run tests with valgrind

# Direct runner usage
./out/test_runner --type=unit --suite=ptz-service
./out/test_runner --type=integration --suite=ptz-integration,media-integration
./out/test_runner --list
./out/test_runner --help
```

### Test Infrastructure Commands

```bash
# Install unit test dependencies
./cross-compile/onvif/tests/install_dependencies.sh

# Run static analysis (if available)
cppcheck cross-compile/onvif/src

# Check for memory leaks with valgrind (if available)
valgrind --leak-check=full cross-compile/onvif/out/onvifd
```

### Test Runner Help Options

The unified test runner provides comprehensive help and filtering options:

```bash
# Show help with all available options
./out/test_runner --help

# List all test suites with descriptions and test counts
./out/test_runner --list

# Filter by test type
./out/test_runner --type=unit          # Unit tests only
./out/test_runner --type=integration   # Integration tests only
./out/test_runner --type=all           # All tests (default)

# Filter by suite name (supports multiple suites)
./out/test_runner --suite=ptz-service
./out/test_runner --suite=ptz-service,media-utils,http-auth

# Combined filtering
./out/test_runner --type=unit --suite=ptz-service,ptz-callbacks
```
