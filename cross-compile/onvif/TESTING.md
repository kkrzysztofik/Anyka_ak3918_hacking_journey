# ONVIF Testing Guide

Complete guide to running unit tests, integration tests, and E2E tests for the ONVIF server implementation.

## Overview

The project includes comprehensive testing infrastructure with 196 tests across 17 test suites:

- **Unit Tests**: 129 tests across 12 suites - Test individual components in isolation
- **Integration Tests**: 67 tests across 5 suites - Test service integration and SOAP functionality
- **E2E Tests**: Python-based tests for end-to-end validation with actual hardware

## Quick Start

### Prerequisites

```bash
# Install unit test dependencies (one-time setup)
cd cross-compile/onvif/tests
./install_dependencies.sh
```

### Running Tests

```bash
# From project root or tests directory - commands work in both locations

# Run all tests (196 tests)
make test

# Run unit tests only (129 tests)
make test-unit

# Run integration tests only (67 tests)
make test-integration

# List all available test suites
make test-list
```

## Test Suite Organization

### Unit Test Suites (12 suites, 129 tests)

| Suite Name           | Tests | Description                 |
| -------------------- | ----- | --------------------------- |
| `basic`              | 3     | Basic Framework Tests       |
| `memory-utils`       | 7     | Memory Management Utilities |
| `logging-utils`      | 6     | Logging Utilities           |
| `http-auth`          | 9     | HTTP Authentication         |
| `http-metrics`       | 8     | HTTP Metrics                |
| `gsoap-protocol`     | 26    | gSOAP Protocol              |
| `service-dispatcher` | 21    | Service Dispatcher          |
| `ptz-service`        | 8     | PTZ Service                 |
| `ptz-callbacks`      | 14    | PTZ Callbacks               |
| `media-utils`        | 11    | Media Utilities             |
| `media-callbacks`    | 8     | Media Callbacks             |
| `imaging-callbacks`  | 8     | Imaging Callbacks           |

### Integration Test Suites (5 suites, 67 tests)

| Suite Name            | Tests | Description                 |
| --------------------- | ----- | --------------------------- |
| `ptz-integration`     | 18    | PTZ Service Integration     |
| `media-integration`   | 19    | Media Service Integration   |
| `device-integration`  | 19    | Device Service Integration  |
| `imaging-integration` | 7     | Imaging Service Integration |
| `soap-errors`         | 4     | SOAP Error Handling         |

## Suite Filtering

### Run Specific Test Suite

```bash
# Run a single suite
make test SUITE=basic
make test SUITE=ptz-service
make test SUITE=http-auth

# Run multiple suites (comma-separated)
make test SUITE=ptz-service,ptz-callbacks,ptz-integration
make test SUITE=http-auth,http-metrics
```

### Filter by Type and Suite

```bash
# Run specific unit test suites
make test-unit SUITE=http-auth,http-metrics
make test-unit SUITE=ptz-service,ptz-callbacks

# Run specific integration test suites
make test-integration SUITE=ptz-integration
make test-integration SUITE=media-integration,device-integration
```

## Common Testing Workflows

### Test by Service Category

```bash
# PTZ service (all related tests: unit + integration)
make test SUITE=ptz-service,ptz-callbacks,ptz-integration

# Media service (all related tests)
make test SUITE=media-utils,media-callbacks,media-integration

# Device service
make test SUITE=device-integration

# Imaging service
make test SUITE=imaging-callbacks,imaging-integration

# HTTP functionality
make test-unit SUITE=http-auth,http-metrics
```

### Development Workflow

```bash
# 1. Make code changes to PTZ service
vim src/services/ptz/onvif_ptz.c

# 2. Run related tests
make test SUITE=ptz-service,ptz-integration

# 3. If tests pass, run broader test suite
make test-unit

# 4. Before committing, run all tests
make test

# 5. Optionally generate coverage report
make test-coverage-html
```

### Test-Driven Development (TDD)

```bash
# 1. Write test first
vim tests/src/unit/services/ptz/test_ptz_service.c

# 2. Run the test (should fail)
make test SUITE=ptz-service

# 3. Implement feature
vim src/services/ptz/onvif_ptz.c

# 4. Run test again (should pass)
make test SUITE=ptz-service

# 5. Refactor and verify
make test SUITE=ptz-service
```

## Advanced Usage

### Direct Test Runner Usage

For more control, use the test runner directly:

```bash
cd tests

# Run with specific options
./out/test_runner --suite=ptz-service
./out/test_runner --type=unit
./out/test_runner --type=integration
./out/test_runner --suite=ptz-service,ptz-callbacks
./out/test_runner --list
```

### Coverage Analysis

```bash
# Build with coverage
make test-coverage

# Generate HTML report (opens in browser)
make test-coverage-html

# View coverage summary
make test-coverage-report

# Clean coverage files
make test-coverage-clean
```

### Memory Analysis

```bash
# Run tests with valgrind (memory leak detection)
make test-valgrind

# Check specific suite for memory leaks
cd tests
valgrind --leak-check=full ./out/test_runner --suite=ptz-service
```

## Test Naming Convention

All test functions must follow these naming patterns:

### Unit Tests

Pattern: `test_unit_<module>_<functionality>`

Examples:

- `test_unit_memory_manager_init`
- `test_unit_http_auth_verify_credentials_success`
- `test_unit_ptz_get_nodes_null_params`

### Integration Tests

Pattern: `test_integration_<service>_<functionality>`

Examples:

- `test_integration_ptz_absolute_move_functionality`
- `test_integration_media_profile_operations`
- `test_integration_device_get_device_info_soap`

## Writing New Tests

### Unit Test Template

```c
/**
 * @brief Test description
 */
static void test_unit_module_functionality(void** state) {
    (void)state;  // Unused parameter

    // Setup
    int result;

    // Execute
    result = function_under_test(param1, param2);

    // Verify
    assert_int_equal(result, EXPECTED_VALUE);
}
```

### Integration Test Template

```c
/**
 * @brief Integration test description
 */
void test_integration_service_functionality(void** state) {
    (void)state;

    // Setup service
    onvif_service_init();

    // Execute SOAP operation
    struct soap soap;
    soap_init(&soap);
    int result = service_operation(&soap, params);

    // Verify response
    assert_int_equal(result, ONVIF_SUCCESS);

    // Cleanup
    soap_destroy(&soap);
    soap_end(&soap);
    onvif_service_cleanup();
}
```

### Adding Tests to Suite

1. Write test function following naming convention
2. Add forward declaration to suite wrapper file
3. Add test to suite array in wrapper
4. Rebuild: `make clean && make`
5. Verify: `make test-list`

## Continuous Integration

### Pre-Commit Testing

```bash
# Before committing code changes
make test                        # Run all tests
make test-coverage-html          # Check coverage
./scripts/lint_code.sh --check  # Lint code
./scripts/format_code.sh --check # Check formatting
```

### CI/CD Pipeline

```yaml
# Example GitHub Actions workflow
- name: Run Unit Tests
  run: |
    cd cross-compile/onvif
    make test

- name: Generate Coverage
  run: |
    make test-coverage-html

- name: Upload Coverage
  uses: codecov/codecov-action@v3
```

## E2E Testing

For end-to-end testing with actual hardware, see the [E2E README](../../e2e/README.md).

Quick E2E test commands:

```bash
# From project root
cd e2e

# Install dependencies
pip install -r requirements.txt

# Configure device (edit config.py or set env vars)
export ONVIF_TEST_DEVICE_0_IP="192.168.1.100"

# Run E2E tests
python -m pytest
python -m pytest -m "onvif_device"
python -m pytest -m "onvif_ptz"
```

## Troubleshooting

### Build Issues

```bash
# Clean and rebuild
make clean
make

# Rebuild just tests
cd tests
make clean
make
```

### Test Failures

```bash
# Run with verbose output
./out/test_runner --suite=failing-suite

# Check test logs
less tests/logs/test_output.log

# Run specific test in debugger
gdb --args ./out/test_runner --suite=ptz-service
```

### Missing Dependencies

```bash
# Reinstall test dependencies
cd tests
./install_dependencies.sh

# Install CMocka manually
sudo apt-get install libcmocka-dev
```

## Test Metrics

Current test statistics:

- **Total Tests**: 196
- **Unit Tests**: 129 (65.8%)
- **Integration Tests**: 67 (34.2%)
- **Test Suites**: 17
- **Code Coverage**: Run `make test-coverage-report` to see current coverage

## Best Practices

1. **Run tests frequently** - After every code change
2. **Write tests first** - Follow TDD when adding features
3. **Test edge cases** - Include NULL checks, boundary conditions
4. **Keep tests fast** - Unit tests should run in milliseconds
5. **Use descriptive names** - Follow naming convention strictly
6. **Clean up resources** - Prevent memory leaks in tests
7. **Mock external dependencies** - Use mocks for platform/network calls
8. **Verify coverage** - Aim for 80%+ coverage on new code
9. **Document tests** - Include clear docstrings
10. **Run all tests before commit** - Ensure nothing breaks

## References

- [E2E README](../../e2e/README.md) - End-to-end testing guide
- [tests/Makefile](Makefile) - Test build system
- [CMocka Documentation](https://cmocka.org/) - Testing framework docs
