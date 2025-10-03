# Quality Assurance - Anyka AK3918 Project

## Testing Strategy

### Testing Overview

- **Unit Tests**: CMocka-based testing for all utility functions
- **Integration Tests**: Service interaction testing
- **E2E Tests**: Python-based comprehensive test coverage
- **SD Card Testing**: Primary development workflow for device testing
- **Static Analysis**: Integrated tools for code quality

### Unit Testing (MANDATORY)

> **🧪 Testing Details**: See [Testing Guide](testing-guide.md) for comprehensive testing framework and CMocka usage.

#### Key Testing Requirements

- **Test Suite**: Dynamic test count across 17 suites (unit and integration tests)
- **ALL utility functions** must have corresponding unit tests
- **Tests must cover** success cases, error cases, and edge conditions
- **Use CMocka framework** for isolated unit testing
- **Run tests** before committing any changes: `make test`
- **Suite filtering**: `make test SUITE=ptz-service,http-auth`
- **Type filtering**: `make test-unit` or `make test-integration`
- **List suites**: `make test-list`
- **Generate coverage reports** to ensure high test coverage: `make test-coverage-html`
- **Tests run on development machine** using native compilation (not cross-compilation)

#### Test Naming Convention (MANDATORY)

- **Unit tests**: Must follow `test_unit_<module>_<functionality>` pattern
- **Integration tests**: Must follow `test_integration_<service>_<functionality>` pattern
- **Examples**:
  - Unit test: `test_unit_memory_manager_init`, `test_unit_http_auth_verify_credentials_success`
  - Integration test: `test_integration_ptz_absolute_move_functionality`, `test_integration_media_profile_operations`
- **Enforcement**: Test runner uses naming convention to filter tests by category
- **Consistency**: All test functions must follow this naming pattern for proper categorization

#### CMocka-Based Mock Usage (MANDATORY) - **STRICT REQUIREMENTS** for all unit tests

> **🧪 Testing Details**: See [Testing Guide](testing-guide.md) for comprehensive CMocka mock usage patterns.

##### Key Mock Requirements

- **Mock Framework**: **ONLY CMocka mocks** are allowed for unit testing
- **Mock Implementation**: Use `__wrap_` prefix with `--wrap` linker mechanism
- **Mock Structure**: Follow `*_mock.c` and `*_mock.h` pattern
- **Mock Functions**: Use standard CMocka patterns (`function_called()`, `check_expected_*()`, `mock()`)
- **Mock Configuration**: Use `will_return()` and `expect_*()` functions
- **Mock State Management**: Proper initialization and cleanup required
- **Mock Validation**: Verify calls, parameters, and return values
- **Mock Documentation**: Complete Doxygen documentation required

##### Essential Mock Pattern

```c
// Mock function implementation
platform_result_t __wrap_platform_init(void) {
    function_called();
    return (platform_result_t)mock();
}

// Test usage
will_return(__wrap_platform_init, PLATFORM_SUCCESS);
expect_value(__wrap_platform_ptz_move, pan, 90.0f);
```

### Static Analysis & Testing Requirements (MANDATORY)

#### Key Testing Requirements

- **Test Naming**: `test_unit_<module>_<functionality>` and `test_integration_<service>_<functionality>`
- **Unit Testing**: **MANDATORY** for all utility functions with CMocka framework
- **Mock Usage**: **ONLY CMocka mocks** with `__wrap_` prefix and `--wrap` linker mechanism
- **Code Quality**: Linting and formatting scripts are **MANDATORY** for all changes
- **Coverage**: Generate reports with `make test-coverage-html`

#### Essential Commands

```bash
# Testing
make test                           # All tests
make test-unit                      # Unit tests only
make test-integration               # Integration tests only
make test SUITE=ptz-service         # Specific suite

# Code Quality
./cross-compile/onvif/scripts/lint_code.sh --check
./cross-compile/onvif/scripts/format_code.sh --check
```

### Build System Features

The Makefile supports:

- **Debug/Release builds**: `make debug` vs `make release` with automatic BUILD_TYPE handling
- **Documentation generation**: `make docs` (MANDATORY after changes)
- **Static analysis**: Multiple tools including Clang, Cppcheck, Snyk via `make static-analysis`
- **Compile commands**: `make compile-commands` for clangd support
- **Cross-compilation**: Uses arm-anykav200-linux-uclibcgnueabi toolchain

### Quality Assurance Checklist

**Before approving any code changes, verify:**

- [ ] **Bash syntax** used in all commands
- [ ] **Native builds** work correctly
- [ ] **Include ordering** follows standards
- [ ] **Include path format** uses relative paths from `src/` (no `../` patterns)
- [ ] **Global variable naming** follows `g_<module>_<variable_name>` convention
- [ ] **Return code constants** used instead of magic numbers
- [ ] **Doxygen documentation** is complete and updated
- [ ] **Utility functions** are used instead of duplicated code
- [ ] **No code duplication** exists
- [ ] **Memory management** is correct (no leaks)
- [ ] **Error handling** is comprehensive
- [ ] **Input validation** is present
- [ ] **Thread safety** is maintained
- [ ] **Performance** is acceptable
- [ ] **Security** concerns are addressed
- [ ] **Code style** is consistent
- [ ] **Code linting** passes without issues (`./cross-compile/onvif/scripts/lint_code.sh --check`)
- [ ] **Code formatting** is compliant (`./cross-compile/onvif/scripts/format_code.sh --check`)
- [ ] **Function ordering** follows guidelines (definitions at top, execution logic at bottom)
- [ ] **Global variables** are placed at top of files after includes
- [ ] **Static analysis** passes without critical issues
- [ ] **Memory analysis** shows no leaks or overflows
- [ ] **Security testing** validates input handling
- [ ] **ONVIF compliance** is maintained
- [ ] **File structure** follows directory organization
- [ ] **Tests** pass (if applicable)
- [ ] **Mock Testing** follows CMocka patterns (if unit tests)
- [ ] **Mock Framework** uses only CMocka (no other mocking frameworks)
- [ ] **Mock Implementation** uses `__wrap_` prefix and `--wrap` linker mechanism
- [ ] **Mock File Structure** follows `*_mock.c` and `*_mock.h` pattern
- [ ] **Mock Functions** use standard CMocka patterns (`function_called()`, `check_expected_*()`, `mock()`)
- [ ] **Mock Configuration** uses `will_return()` and `expect_*()` functions correctly
- [ ] **Mock State Management** has proper initialization and cleanup
- [ ] **Mock Parameter Validation** uses CMocka validation functions
- [ ] **Mock Call Verification** checks function calls and parameters
- [ ] **Mock Error Simulation** tests error conditions properly
- [ ] **Mock Documentation** has proper Doxygen documentation
- [ ] **Mock Isolation** ensures tests are independent
- [ ] **Mock Coverage** mocks all external dependencies
- [ ] **Mock Naming** follows `__wrap_<function_name>` convention
- [ ] **Mock File Organization** uses `tests/src/mocks/` directory
- [ ] **Mock Testing Requirements** avoids real hardware/network/file operations

### Testing Commands

```bash
# Install CMocka dependencies
./cross-compile/onvif/tests/install_dependencies.sh

# Run all unit tests
make test

# Run utility unit tests only
make test-utils

# Run tests with coverage analysis
make test-coverage

# Generate HTML coverage report
make test-coverage-html

# Generate coverage summary
make test-coverage-report

# Clean coverage files
make test-coverage-clean
```

### Linter Integration

#### Linter Analysis Commands

```bash
# Run comprehensive linter analysis (all files)
./cross-compile/onvif/scripts/lint_code.sh

# Check for linting issues (exits 1 if issues found)
./cross-compile/onvif/scripts/lint_code.sh --check

# Lint specific file
./cross-compile/onvif/scripts/lint_code.sh --file src/services/device/onvif_device.c

# Lint only changed files since last commit
./cross-compile/onvif/scripts/lint_code.sh --changed

# Also check code formatting compliance
./cross-compile/onvif/scripts/lint_code.sh --format

# Fail only on errors (not warnings/info/hints)
./cross-compile/onvif/scripts/lint_code.sh --severity error

# Dry run mode (show what would be linted)
./cross-compile/onvif/scripts/lint_code.sh --dry-run
```

#### Code Formatting Commands

```bash
# Format all C files
./cross-compile/onvif/scripts/format_code.sh

# Check formatting without making changes (exits 1 if issues)
./cross-compile/onvif/scripts/format_code.sh --check

# Format specific files
./cross-compile/onvif/scripts/format_code.sh --files src/core/main.c,src/services/device/onvif_device.c

# Dry run mode (show what would be changed)
./cross-compile/onvif/scripts/format_code.sh --dry-run
```

### Success Criteria

A successful quality assurance process should:

- ✅ **Identify all critical security vulnerabilities**
- ✅ **Ensure ONVIF 2.5 compliance**
- ✅ **Verify proper platform abstraction**
- ✅ **Confirm code quality standards adherence**
- ✅ **Address all linter errors and critical warnings** (`lint_code.sh --check` passes)
- ✅ **Verify code formatting compliance** (`format_code.sh --check` passes)
- ✅ **Validate all unit tests pass** (`make test` succeeds)
- ✅ **Verify test coverage** meets requirements (`make test-coverage-html`)
- ✅ **Confirm proper return code constant usage** (no magic numbers)
- ✅ **Validate test naming conventions** (unit/integration patterns followed)
- ✅ **Provide actionable improvement recommendations**
- ✅ **Validate build and documentation processes**
- ✅ **Verify compile_commands.json is current** for clangd support
