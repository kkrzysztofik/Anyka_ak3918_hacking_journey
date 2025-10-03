# Anyka AK3918 ONVIF Project - Coding Standards and Conventions

## Project Overview
This repository contains comprehensive reverse-engineering work and custom firmware development for Anyka AK3918-based IP cameras. It includes cross-compilation tools, SD-card bootable payloads, root filesystem modifications, and detailed documentation for understanding and extending camera functionality.

## Development Environment (MANDATORY)
- **Primary OS**: WSL2 with Ubuntu
- **Shell**: **MANDATORY** - All terminal commands MUST use bash syntax
- **Cross-compilation**: Use native cross-compilation tools and toolchain
- **Path handling**: Use Unix-style paths throughout the development environment

## File Organization (MANDATORY)

### Include Order (strict ordering for all C files):
1. **System headers first** (e.g., `#include <stdio.h>`, `#include <stdlib.h>`)
2. **Third-party library headers** (e.g., `#include <curl/curl.h>`)
3. **Project-specific headers** (e.g., `#include "platform.h"`, `#include "common.h"`)

- Within each group, use alphabetical ordering
- Separate groups with blank lines
- Use include guards (`#ifndef HEADER_H` / `#define HEADER_H` / `#endif`) or `#pragma once`
- Avoid unnecessary includes and circular dependencies

### Include Path Format (MANDATORY for all project headers):
- **ALWAYS use relative paths from `src/` directory** for all project includes
- **CORRECT format**: `#include "services/common/video_config_types.h"`
- **INCORRECT format**: `#include "../../services/common/video_config_types.h"`
- **Rationale**: Consistent, maintainable, and IDE-friendly include paths
- **Enforcement**: Makefile and clangd configuration enforce this rule

### Source File Structure & Organization (MANDATORY):
```
src/
├── core/                    # Core system components
│   ├── config/             # Configuration management
│   ├── lifecycle/          # Service lifecycle management
│   └── main/               # Main daemon entry point
├── platform/               # Platform abstraction layer
│   ├── adapters/           # Hardware-specific adapters
│   ├── platform_anyka.c    # Anyka AK3918 implementation
│   └── platform.h          # Platform interface
├── services/               # ONVIF service implementations
│   ├── device/             # Device service
│   ├── media/              # Media service
│   ├── ptz/                # PTZ service
│   ├── imaging/            # Imaging service
│   ├── snapshot/           # Snapshot service
│   └── common/             # Shared service types
├── networking/             # Network protocol implementations
│   ├── http/               # HTTP/SOAP handling
│   ├── rtsp/               # RTSP streaming
│   ├── discovery/          # WS-Discovery
│   └── common/             # Shared networking utilities
├── protocol/               # Protocol handling
│   ├── soap/               # SOAP processing
│   ├── xml/                # XML utilities
│   └── response/           # Response handling
└── utils/                  # Utility functions (CRITICAL)
    ├── memory/             # Memory management utilities
    ├── string/             # String manipulation utilities
    ├── error/              # Error handling utilities
    ├── network/            # Network utility functions
    ├── logging/            # Logging utilities
    ├── validation/         # Input validation utilities
    ├── security/           # Security utilities
    ├── service/            # Service utilities
    └── stream/             # Stream configuration utilities
```

## Variable Naming Conventions (MANDATORY)

### Global Variable Naming Convention (MANDATORY):
- **ALL global variables MUST start with `g_<module>_<variable_name>`**
- **Module prefix** should match the source file or functional area (e.g., `g_onvif_`, `g_platform_`, `g_media_`)
- **Variable name** should be descriptive and follow snake_case convention
- **Examples**:
  ```c
  // ✅ CORRECT - Global variables with module prefix
  static int g_onvif_device_count = 0;
  static char g_platform_device_name[64] = {0};
  static struct media_profile* g_media_profiles[MAX_PROFILES] = {NULL};
  
  // ❌ INCORRECT - Global variables without module prefix
  static int device_count = 0;
  static char device_name[64] = {0};
  static struct media_profile* profiles[MAX_PROFILES] = {NULL};
  ```

### Global Variable Placement (MANDATORY):
- **ALL global variables MUST be placed at the top of the file** after includes and before any function definitions
- **Grouping**: Group related global variables together with blank lines between groups
- **Initialization**: Initialize global variables at declaration when possible
- **Documentation**: Add comments explaining the purpose of each global variable

### Other Naming Conventions:
- **Functions**: snake_case with module prefix (e.g., `onvif_util_validate_token`)
- **Constants**: UPPER_CASE with module prefix (e.g., `ONVIF_SUCCESS`, `HTTP_AUTH_ERROR_NULL`)
- **Local Variables**: snake_case
- **File Naming**:
  - Source files: `onvif_<service>.c` (e.g., `onvif_device.c`)
  - Header files: `onvif_<service>.h` (e.g., `onvif_device.h`)
  - Utility files: `<category>_utils.c` (e.g., `memory_utils.c`)
  - Platform files: `platform_<platform>.c` (e.g., `platform_anyka.c`)

## Return Code Standards (MANDATORY)

### Return Code Constants (MANDATORY):
- **NEVER use magic numbers** for return codes (e.g., `return -1`, `return 0`)
- **ALWAYS use predefined constants** for all function return values
- **Module-specific constants** should be defined in the module's header file
- **Global constants** should be defined in `utils/error/error_handling.h`
- **Examples**:
  ```c
  // ✅ CORRECT - Using predefined constants
  return HTTP_AUTH_SUCCESS;
  return HTTP_AUTH_ERROR_NULL;
  return ONVIF_SUCCESS;
  return ONVIF_ERROR_INVALID;
  
  // ❌ INCORRECT - Using magic numbers
  return 0;
  return -1;
  return -2;
  ```
- **Constant naming convention**:
  - Success: `MODULE_SUCCESS` (e.g., `HTTP_AUTH_SUCCESS`, `ONVIF_SUCCESS`)
  - Errors: `MODULE_ERROR_TYPE` (e.g., `HTTP_AUTH_ERROR_NULL`, `ONVIF_ERROR_INVALID`)
- **Enforcement**: All code reviews must verify constant usage instead of magic numbers

## Test Naming Conventions (MANDATORY)

### Test Naming Convention (MANDATORY):
- **Unit tests**: Must follow `test_unit_<module>_<functionality>` pattern
- **Integration tests**: Must follow `test_integration_<service>_<functionality>` pattern
- **Examples**:
  - Unit test: `test_unit_memory_manager_init`, `test_unit_http_auth_verify_credentials_success`
  - Integration test: `test_integration_ptz_absolute_move_functionality`, `test_integration_media_profile_operations`
- **Enforcement**: Test runner uses naming convention to filter tests by category
- **Consistency**: All test functions must follow this naming pattern for proper categorization

### Unit Testing (MANDATORY):
- **Test Suite**: 196 tests across 17 suites (129 unit tests, 67 integration tests)
- **ALL utility functions** must have corresponding unit tests
- **Tests must cover** success cases, error cases, and edge conditions
- **Use CMocka framework** for isolated unit testing
- **Run tests** before committing any changes: `make test`
- **Suite filtering**: `make test SUITE=ptz-service,http-auth`
- **Type filtering**: `make test-unit` or `make test-integration`
- **List suites**: `make test-list`
- **Generate coverage reports** to ensure high test coverage: `make test-coverage-html`
- **Tests run on development machine** using native compilation (not cross-compilation)

## Documentation Standards (MANDATORY)

### Doxygen Documentation (MANDATORY):
- **ALL code changes MUST include updated Doxygen documentation**
- **Function documentation**: Every public function must have complete Doxygen comments including `@brief`, `@param`, `@return`, and `@note` where applicable
- **File headers**: Each source file must have a Doxygen file header with `@file`, `@brief`, `@author`, and `@date` tags
- **Structure/Enum documentation**: All public structures, enums, and typedefs must be documented with `@brief` and member descriptions
- **Documentation generation**: **MANDATORY** - Use `doxygen Doxyfile` in the `cross-compile/onvif/` directory to generate updated documentation
- **Documentation validation**: **MANDATORY** - Verify that all new/changed functions appear correctly in the generated HTML documentation
- **Consistency**: Follow the existing Doxygen style used throughout the ONVIF project

### File Header Standards (MANDATORY):
- **Consistent File Headers**: **ALL source and header files MUST have consistent Doxygen file headers**
- **Required format** (based on `onvif_constants.h` template):
  ```c
  /**
   * @file filename.h
   * @brief Brief description of the file's purpose
   * @author kkrzysztofik
   * @date 2025
   */
  ```
- **Header requirements**:
  - `@file` - Must match the actual filename
  - `@brief` - Concise description of the file's purpose and functionality
  - `@author` - Must be "kkrzysztofik" for consistency
  - `@date` - Must be "2025" for current year
- **Placement**: File header must be the first content in the file (after any include guards)
- **Consistency**: All files must follow this exact format and structure
- **Validation**: File headers are checked during code review and must be present in all new files

## Code Structure and Organization

### Code Organization:
- **Consistent 2-space indentation**
- **Consistent variable naming**
- **Proper spacing and formatting**
- **Functions ordered with definitions at the top of files and execution logic at the bottom**

### Function Order Compliance (MANDATORY):
- **Verify function ordering** follows project guidelines: definitions at top, execution logic at bottom
- **Check global variables** are placed at the top of files after includes
- **Validate include ordering**: system headers → third-party → project headers
- **Ensure proper spacing** and formatting between function groups
- **Use linting script** with `--format` flag to check function ordering compliance
- **Manual verification** may be required for complex files with many functions
- **All ordering issues MUST be resolved** before code can be approved

## Utility Usage and Code Reuse (MANDATORY)

### Utility Usage (MANDATORY):
- **ALWAYS use existing utilities** - Check `src/utils/` directory for available utility functions before implementing new code
- **NO code duplication** - If similar functionality exists elsewhere, refactor to use shared utilities
- **Create utilities for common operations** - When implementing functionality that might be reused, create utility functions
- **Utility function requirements**:
  - Must be placed in appropriate `src/utils/` subdirectory
  - Must have complete Doxygen documentation
  - Must follow consistent naming conventions (e.g., `onvif_util_*` for ONVIF-specific utilities)
  - Must include proper error handling and validation
  - Must be thread-safe if applicable

### Code Duplication Prevention (MANDATORY):
- **DRY Principle** - Don't Repeat Yourself - Any code that appears in multiple places MUST be refactored into utilities
- **Common patterns to avoid duplicating**:
  - String manipulation and validation
  - Memory allocation and cleanup
  - Error handling and logging
  - XML parsing and generation
  - Configuration file operations
  - Network operations and socket handling
  - File I/O operations
  - Mathematical calculations and conversions

## Security Guidelines (MANDATORY)

### Input Validation (MANDATORY):
- **ALL user inputs must be properly validated and sanitized**:
  - Check for NULL pointers before dereferencing
  - Validate string lengths and bounds
  - Sanitize XML/SOAP input to prevent injection attacks
  - Validate numeric ranges and types
  - Check for buffer overflows in all string operations

### Buffer Management (MANDATORY):
- **Prevent buffer overflows and underflows**:
  - Use `strncpy()` instead of `strcpy()`
  - Always null-terminate strings
  - Check array bounds before access
  - Use fixed-size buffers with proper bounds checking
  - Avoid `scanf()` with unbounded format strings

### Memory Security (MANDATORY):
- **Prevent memory-related vulnerabilities**:
  - Initialize all allocated memory with `memset()` or `calloc()`
  - Free all allocated resources in error paths
  - Check for use-after-free vulnerabilities
  - Validate pointer arithmetic safety
  - Use stack variables when possible, heap when necessary

### Network Security (MANDATORY):
- **Secure network communications**:
  - Validate all network input before processing
  - Use proper error message handling (no information leakage)
  - Implement proper authentication and authorization
  - Use secure communication protocols (HTTPS, WSS)
  - Validate SOAP/XML input to prevent injection

### Authentication & Authorization (MANDATORY):
- Implement proper credential handling
- Use secure password storage mechanisms
- Validate session management
- Ensure proper access controls for all endpoints
- Implement rate limiting for authentication attempts

## Code Quality Validation (MANDATORY)

### Code Linting (MANDATORY):
- **Run linting script**: `./cross-compile/onvif/scripts/lint_code.sh` after implementation
- **Check specific files**: `./cross-compile/onvif/scripts/lint_code.sh --file src/path/to/file.c`
- **Check changed files**: `./cross-compile/onvif/scripts/lint_code.sh --changed`
- **Check only mode**: `./cross-compile/onvif/scripts/lint_code.sh --check` (exits 1 if issues found)
- **Severity levels**: Use `--severity error` to fail only on errors, `--severity warn` for warnings and errors
- **Format checking**: Use `--format` to also check code formatting compliance
- **All linting issues MUST be resolved** before code can be approved
- **Linting covers**: Code style, potential bugs, performance issues, security concerns, and best practices
- **Integration**: Linting is integrated with clangd-tidy for comprehensive analysis

### Code Formatting (MANDATORY):
- **Run formatting script**: `./cross-compile/onvif/scripts/format_code.sh` after implementation
- **Check specific files**: `./cross-compile/onvif/scripts/format_code.sh --files src/path/to/file.c`
- **Check only mode**: `./cross-compile/onvif/scripts/format_code.sh --check` (exits 1 if formatting issues found)
- **Dry run mode**: `./cross-compile/onvif/scripts/format_code.sh --dry-run` to see what would be changed
- **All formatting issues MUST be resolved** before code can be approved
- **Formatting covers**: Indentation, spacing, brace placement, line length, and consistent style
- **Integration**: Uses clang-format with project-specific configuration

### Static Analysis & Testing Requirements (MANDATORY):
- **Compilation Testing** - ALL code changes must compile successfully:
  - Use native build: `make -C cross-compile/onvif`
  - Fix all compiler warnings and errors
  - Verify no undefined behavior
  - Check for unused variables and functions

- **Static Analysis** - Run automated analysis tools:
  - Use tools like `cppcheck`, `clang-static-analyzer`, or `PVS-Studio`
  - Address all critical and high-severity issues
  - Review medium-severity issues for potential problems
  - Document any false positives

- **Memory Analysis** - Use memory debugging tools:
  - Run with `valgrind` or `AddressSanitizer` when possible
  - Check for memory leaks, buffer overflows, and use-after-free
  - Verify proper resource cleanup
  - Test with different input sizes and edge cases

- **Security Testing** - Validate security measures:
  - Test input validation with malicious inputs
  - Verify authentication and authorization mechanisms
  - Check for information leakage in error messages
  - Test network security with various attack vectors

- **Performance Testing** - Measure and optimize performance:
  - Profile critical functions for bottlenecks
  - Test with various load conditions
  - Measure memory usage and allocation patterns
  - Verify response times meet requirements

## NOLINT Usage Guidelines

### NOLINT Suppression (MANDATORY):
- **Use `//NOLINT` comments** for linter warnings when the suggested changes don't make sense from an implementation perspective
- **Example**: `static int g_discovery_running = 0; //NOLINT` for warnings about non-const global variables that need to be mutable
- **Example**: `int onvif_ptz_stop(const char *profile_token, int stop_pan_tilt, int stop_zoom) //NOLINT` for warnings about adjacent parameters of similar type that are easily swapped by mistake when changing the order doesn't make sense from implementation perspective
- **Rationale**: Some linter suggestions conflict with legitimate design requirements (e.g., global state variables that must be mutable, or function parameters where the current order is semantically correct)
- **Documentation**: Always add a comment explaining why NOLINT is used
- **Review**: NOLINT usage must be justified during code review

## ONVIF Compliance Requirements (MANDATORY)

### Service Implementation (MANDATORY):
- **Complete implementation of all required services**:
  - **Device Service**: Device information, capabilities, system date/time
  - **Media Service**: Profile management, stream URIs, video/audio configurations
  - **PTZ Service**: Pan/tilt/zoom control, preset management, continuous move
  - **Imaging Service**: Image settings, brightness, contrast, saturation controls

### SOAP Protocol (MANDATORY):
- **Proper XML/SOAP request/response handling**:
  - Correct XML namespace usage
  - Proper SOAP envelope structure
  - ONVIF-compliant error codes and messages
  - Proper content-type headers

### WS-Discovery (MANDATORY):
- **Correct multicast discovery implementation**:
  - Proper UDP multicast socket handling
  - Correct discovery message format
  - Proper device announcement and probe responses

### RTSP Streaming (MANDATORY):
- **Proper H.264 stream generation and delivery**:
  - Correct SDP format for stream descriptions
  - Proper RTP packetization
  - Correct stream URI generation
  - Proper authentication for RTSP streams

### Error Codes (MANDATORY):
- **ONVIF-compliant error reporting**:
  - Use standard ONVIF error codes
  - Provide meaningful error messages
  - Handle all error conditions gracefully

## Development Workflow (MANDATORY)

### Pre-Development Checklist:
- [ ] **Understand the task** - Read requirements carefully and ask clarifying questions
- [ ] **Check existing code** - Review related files and utilities before implementing
- [ ] **Plan the approach** - Break down complex tasks into smaller, manageable steps
- [ ] **Identify dependencies** - Understand what other components might be affected
- [ ] **Review coding standards** - Ensure compliance with project guidelines

### Development Process:
1. **Code Implementation**:
   - Follow include ordering standards strictly
   - Use existing utilities to avoid code duplication
   - Implement proper error handling and validation
   - Add comprehensive Doxygen documentation
   - Use consistent naming conventions and formatting

2. **Code Quality Validation (MANDATORY)**:
   - **Run linting script**: `./cross-compile/onvif/scripts/lint_code.sh --check`
   - **Run formatting script**: `./cross-compile/onvif/scripts/format_code.sh --check`
   - **Fix all linting issues** before proceeding
   - **Fix all formatting issues** before proceeding
   - **Verify function ordering** compliance with project guidelines
   - **Check global variable placement** at top of files after includes

3. **Testing and Validation**:
   - Test compilation with native make
   - Run static analysis tools
   - Perform memory leak testing
   - Test with various input conditions
   - Verify ONVIF compliance

4. **Documentation and Review**:
   - Update Doxygen documentation
   - Regenerate HTML documentation
   - Perform comprehensive code review
   - Update any affected documentation
   - Test documentation generation

5. **Integration and Deployment**:
   - Test with SD-card payload
   - Verify functionality on target device
   - Check for regression issues
   - Update version control
   - Document any breaking changes

## Bash Command Reference (MANDATORY)

### Build and Compilation Commands:
```bash
# Build the ONVIF project
make -C cross-compile/onvif

# Build with verbose output
make -C cross-compile/onvif VERBOSE=1

# Clean build artifacts
make -C cross-compile/onvif clean

# Build specific target
make -C cross-compile/onvif onvifd

# Generate documentation
make -C cross-compile/onvif docs
```

### Code Quality Commands (MANDATORY):
```bash
# Code Linting (MANDATORY after implementation)
./cross-compile/onvif/scripts/lint_code.sh                    # Lint all C files
./cross-compile/onvif/scripts/lint_code.sh --check            # Check for issues (exit 1 if found)
./cross-compile/onvif/scripts/lint_code.sh --file src/path/to/file.c  # Lint specific file
./cross-compile/onvif/scripts/lint_code.sh --changed          # Lint only changed files
./cross-compile/onvif/scripts/lint_code.sh --format           # Also check formatting
./cross-compile/onvif/scripts/lint_code.sh --severity error   # Fail only on errors

# Code Formatting (MANDATORY after implementation)
./cross-compile/onvif/scripts/format_code.sh                  # Format all C files
./cross-compile/onvif/scripts/format_code.sh --check          # Check formatting (exit 1 if issues)
./cross-compile/onvif/scripts/format_code.sh --files src/path/to/file.c  # Format specific files
./cross-compile/onvif/scripts/format_code.sh --dry-run        # Show what would be changed
```

### Unit Testing Commands (MANDATORY):
```bash
# Install unit test dependencies
./cross-compile/onvif/tests/install_dependencies.sh

# Run all tests (196 tests across 17 suites)
make test

# Run all unit tests only (129 tests)
make test-unit

# Run all integration tests only (67 tests)
make test-integration

# List all available test suites
make test-list

# Run specific test suite
make test SUITE=ptz-service
make test SUITE=http-auth

# Run multiple test suites (comma-separated)
make test SUITE=ptz-service,ptz-callbacks,ptz-integration
make test-unit SUITE=http-auth,http-metrics

# Filter by type and suite
make test-unit SUITE=ptz-service,ptz-callbacks
make test-integration SUITE=ptz-integration,media-integration

# Run tests with coverage analysis
make test-coverage

# Generate HTML coverage report
make test-coverage-html

# Generate coverage summary
make test-coverage-report

# Clean coverage files
make test-coverage-clean
```

## Quality Assurance Checklist

### Before approving any code changes, verify:
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

## Common Pitfalls to Avoid

- **Code Duplication**: Always check for existing utilities before implementing new code
- **Memory Leaks**: Ensure all allocated resources are properly freed
- **Buffer Overflows**: Use safe string functions and bounds checking
- **Missing Error Handling**: Handle all error conditions gracefully
- **Incomplete Documentation**: Update documentation for all code changes
- **Security Vulnerabilities**: Validate all inputs and use secure coding practices
- **Performance Issues**: Profile critical code paths and optimize bottlenecks

## Agent Instructions

### When Working on This Project:
1. **ALWAYS use bash** for all terminal commands and file operations
2. **ALWAYS use native cross-compilation** for development tasks and building
3. **ALWAYS test compilation** using the native make command before suggesting changes
4. **ALWAYS follow the include ordering** standards strictly
5. **MANDATORY: Update Doxygen documentation** for ALL code changes
6. **MANDATORY: Regenerate HTML documentation** after any code changes
7. **MANDATORY: Perform comprehensive code review** before approving changes
8. **MANDATORY: Use existing utilities** and avoid code duplication
9. **Reference the `cross-compile/anyka_reference/akipc/` tree** when reverse-engineering or implementing new features
10. **Use SD-card testing** as the primary development workflow

### Documentation Workflow (MANDATORY):
1. Make code changes
2. Update Doxygen comments for all modified functions
3. Update file headers if needed
4. Run documentation generation: `make -C cross-compile/onvif docs`
5. Verify documentation in `cross-compile/onvif/docs/html/index.html`
6. Test compilation to ensure changes work
7. **Perform comprehensive code review**
8. Commit changes

### Error Handling & Debugging Guidelines (MANDATORY):
- **Error Code Strategy** - Use consistent error handling throughout
- **Debugging Tools** - Use appropriate debugging techniques
- **Logging Strategy** - Implement comprehensive logging
- **Testing Strategy** - Comprehensive testing approach

**Note**: The project focuses on defensive security only. All code must be secure and robust with proper input validation and error handling.