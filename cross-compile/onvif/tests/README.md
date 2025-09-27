# ONVIF Project Unit Testing

## Overview
This directory contains **unit tests only** for the ONVIF project using the CMocka testing framework. These tests focus on testing individual utility functions in isolation. Integration tests are handled by a separate project.

## Framework Choice: CMocka
- **Lightweight**: Minimal overhead for embedded systems
- **Native compilation**: Runs on development machine (WSL Ubuntu)
- **Mocking**: Built-in mock support for isolated testing
- **Memory checking**: Automatic memory leak detection
- **C99 compatible**: Matches project standards
- **Hardware independence**: Tests actual utility functions, mocks only platform_anyka.c

## Directory Structure
```
tests/
├── README.md                 # This file
├── INSTALL.md               # Installation guide
├── install_dependencies.sh  # Dependency installation script
├── coverage_report.sh       # Coverage report generator
├── Makefile                 # Makefile for test builds
├── test_runner.c            # Main test runner
├── mocks/                   # Mock implementations
│   ├── platform_mock.c      # Mock for platform_anyka.c
│   └── network_mock.c       # Mock for network functions
└── unit/                    # Unit tests by module
    └── utils/               # Utility function tests
        ├── test_memory_utils.c
        ├── test_string_utils.c
        ├── test_validation_utils.c
        ├── test_error_handling.c
        ├── test_network_utils.c
        ├── test_logging_utils.c
        ├── test_security_utils.c
        └── test_stream_utils.c
```

## Building and Running Tests

### Prerequisites
```bash
# Install CMocka (Ubuntu/Debian)
sudo apt-get install libcmocka-dev

# Or run the installation script
./install_dependencies.sh
```

### Build Tests
```bash
# Build all tests
make -C tests

# Build specific test
make -C tests test_utils

# Build with debug symbols
make -C tests debug
```

### Run Tests
```bash
# Run all unit tests
make -C tests test

# Run utility unit tests
./tests/out/test_utils

# Run with verbose output
./tests/out/test_utils -v

# Run with memory checking
valgrind ./tests/out/test_utils
```

## Test Writing Guidelines

### Basic Test Structure
```c
#include <stdarg.h>
#include <stddef.h>
#include <setjmp.h>
#include <cmocka.h>

/**
 * @brief Test function example
 * @param state Test state (unused)
 * @return 0 on success
 */
static void test_example_function(void **state) {
    (void) state; // Suppress unused parameter warning

    // Arrange
    const char* input = "test_string";
    char output[32];

    // Act
    int result = example_function(input, output, sizeof(output));

    // Assert
    assert_int_equal(result, 0);
    assert_string_equal(output, "expected_output");
}

/**
 * @brief Test suite setup
 * @param state Test state
 * @return 0 on success
 */
static int setup_test_suite(void **state) {
    // Initialize test environment
    return 0;
}

/**
 * @brief Test suite teardown
 * @param state Test state
 * @return 0 on success
 */
static int teardown_test_suite(void **state) {
    // Cleanup test environment
    return 0;
}

/**
 * @brief Main test runner
 * @return Number of test failures
 */
int main(void) {
    const struct CMUnitTest tests[] = {
        cmocka_unit_test(test_example_function),
        // Add more tests here
    };

    return cmocka_run_group_tests(tests, setup_test_suite, teardown_test_suite);
}
```

### Mock Usage
```c
#include <cmocka.h>

// Mock function
int __wrap_platform_function(int param) {
    check_expected(param);
    return mock_type(int);
}

// Test using mock
static void test_with_mock(void **state) {
    (void) state;

    // Setup mock expectations
    expect_value(__wrap_platform_function, param, 42);
    will_return(__wrap_platform_function, 0);

    // Test the function that calls the mock
    int result = function_under_test();

    assert_int_equal(result, 0);
}
```

## Test Categories

### Unit Tests (This Project)
- **Memory Utils**: Memory management, allocation, deallocation
- **String Utils**: String manipulation, validation, formatting
- **Validation Utils**: Input validation, token validation, format checking
- **Error Handling**: Error logging, error propagation, cleanup
- **Network Utils**: IP validation, port validation, URL parsing
- **Logging Utils**: Log levels, formatting, initialization
- **Security Utils**: Input sanitization, encoding, validation
- **Stream Utils**: Stream configuration, validation, parsing

### Integration Tests (Separate Project)
- **ONVIF Compliance**: Full protocol compliance testing
- **End-to-End**: Complete workflow testing
- **Service Integration**: ONVIF service interactions
- **Performance**: Load and stress testing

## Continuous Integration
Tests are integrated into the main build system and can be run as part of CI/CD pipelines.

## Coverage Analysis
Enhanced coverage reporting with HTML reports and summary statistics:

### Basic Coverage
```bash
# Build with coverage
make -C tests coverage

# Run tests to generate coverage data
make -C tests test

# Generate coverage report
gcov tests/out/*.gcda
```

### HTML Coverage Reports
```bash
# Generate HTML coverage report
make -C tests coverage-html

# View in browser
xdg-open tests/coverage/html/index.html
```

### Coverage Summary
```bash
# Generate coverage summary
make -C tests coverage-report
```

### Coverage Management
```bash
# Clean coverage files
make -C tests coverage-clean

# From main project directory
make test-coverage-html    # Generate HTML report
make test-coverage-report  # Generate summary
make test-coverage-clean   # Clean coverage files
```

## Best Practices
1. **Test Isolation**: Each test should be independent
2. **Mock External Dependencies**: Use mocks for platform calls
3. **Comprehensive Assertions**: Test both success and failure cases
4. **Memory Management**: Verify no memory leaks
5. **Edge Cases**: Test boundary conditions and error paths
6. **Documentation**: Document test purpose and expected behavior
