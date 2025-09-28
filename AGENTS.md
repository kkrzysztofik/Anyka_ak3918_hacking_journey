# Agent Documentation for Anyka AK3918 Hacking Journey

## Project Overview
This repository contains comprehensive reverse-engineering work and custom firmware development for Anyka AK3918-based IP cameras. It includes cross-compilation tools, SD-card bootable payloads, root filesystem modifications, and detailed documentation for understanding and extending camera functionality.

The project focuses on creating a fully ONVIF 2.5 compliant implementation while maintaining compatibility with the existing camera hardware and providing a robust development environment for camera firmware modifications.

## Repository Structure & Key Components

### Primary Development Areas
- **`cross-compile/onvif/`** — **CURRENT FOCUS** - Complete ONVIF 2.5 implementation with Device, Media, PTZ, and Imaging services. Full SOAP-based web services stack for IP camera control and streaming.
- **`cross-compile/onvif/tests/`** — **MANDATORY** - Unit testing framework using CMocka for testing utility functions in isolation. All utility functions must have corresponding unit tests.
- **`cross-compile/`** — Source code and build scripts for individual camera applications (e.g., `libre_anyka_app`, `aenc_demo`, `ak_snapshot`, `ptz_daemon`). Each subproject contains its own Makefile or build script and handles specific camera functionality.
- **`SD_card_contents/anyka_hack/`** — SD card payload system with web UI for runtime testing. This directory contains everything needed to boot custom firmware from an SD card, making it the easiest and safest way to test changes on the actual device without modifying flash memory.
- **`newroot/`** and prepared squashfs images (`busyroot.sqsh4`, `busyusr.sqsh4`, `newroot.sqsh4`) — Prebuilt root filesystem images for flashing to device. These are compressed squashfs images that replace the camera's root filesystem when flashed.
- **`hack_process/`**, **`README.md`**, **`Images/`**, and **`UART_logs/`** — Comprehensive documentation and debugging resources. Contains detailed guides, captured images, UART serial logs, and step-by-step hacking procedures for understanding and modifying the camera firmware.

### Reference Implementation
- **`cross-compile/anyka_reference/akipc/`** — Complete reference implementation (chip/vendor-provided) that shows how the original camera firmware implements APIs, initialization, and configuration. This is the authoritative source for understanding camera behavior.
- Use as canonical example when:
  - Reverse-engineering how the camera starts services and uses device files
  - Matching IPC/CLI commands and config keys used by webUI and other apps
  - Verifying binary and library ABI expectations before replacing or reimplementing a system binary
  - Understanding the camera's service architecture and initialization sequence
  - Learning proper device file usage and hardware abstraction patterns

### Component Libraries
- **`cross-compile/anyka_reference/component/`** — Comprehensive collection of reusable pieces extracted from stock firmware: drivers, third-party libraries, and helper tools. Contains pre-compiled binaries and headers for all major camera subsystems.
- **`cross-compile/anyka_reference/platform/`** — Board/platform-specific glue code: sensor selection and initialization, board pin mappings, GPIO configurations, and other low-level hardware integration components.

## Development Workflow

### Quick Start Entry Points
- **Fast iteration**: Edit or add files under `cross-compile/<app>/` and test via the SD-card hack in `SD_card_contents/anyka_hack/`. This allows testing changes without flashing the device.
- **Key documentation**:
  - Top-level `README.md` — project goals, features, SD-card hack quick start, and important device commands for UART access and debugging.
  - `cross-compile/README.md` — cross-compile environment setup notes and toolchain configuration.

### Build Process
- **Use native cross-compilation tools**: `make -C cross-compile/<project>` - This ensures consistent builds in the WSL Ubuntu environment.
- **Test compilation** before committing changes - Always verify that code compiles successfully before making changes.
- **SD-card testing** is the primary development workflow for device testing - Copy compiled binaries to the SD card payload and boot the device with the SD card to test functionality.
- **Unit testing** is **MANDATORY** for all utility functions - Run `make test` to execute unit tests before committing changes.
- **Integration testing** can be done using the test suite in `integration-tests/` directory.

## Development Environment

### WSL Ubuntu Development
- **Primary OS**: Development is conducted on WSL2 with Ubuntu
- **Shell**: **MANDATORY** - All terminal commands MUST use bash syntax
- **Cross-compilation**: Use native cross-compilation tools and toolchain
- **Path handling**: Use Unix-style paths throughout the development environment

### Bash Usage (MANDATORY)
- **ALL terminal commands MUST use bash syntax** - Standard Unix commands
- **File operations**: Use standard Unix commands like `ls`, `cp`, `rm`, `test`
- **Environment variables**: Use `$VARIABLE_NAME` syntax
- **Path separators**: Use forward slashes `/` for all paths
- **Build commands**: Use native make and cross-compilation tools

### Bash Command Examples
- **Native builds**: `make -C cross-compile/<project>`
- **File operations**:
  ```bash
  ls -la cross-compile/onvif
  cp source/file.c destination/file.c
  test -f cross-compile/onvif/out/onvifd
  ```
- **Environment variables**: `export BUILD_TYPE="release"`
- **Path separators**: Use forward slashes `/` for all paths

## Coding Standards & Guidelines

### C Code Standards
- **Include Order** (strict ordering for all C files):
  1. **System headers first** (e.g., `#include <stdio.h>`, `#include <stdlib.h>`)
  2. **Third-party library headers** (e.g., `#include <curl/curl.h>`)
  3. **Project-specific headers** (e.g., `#include "platform.h"`, `#include "common.h"`)
  - Within each group, use alphabetical ordering
  - Separate groups with blank lines
  - Use include guards (`#ifndef HEADER_H` / `#define HEADER_H` / `#endif`) or `#pragma once`
  - Avoid unnecessary includes and circular dependencies

- **Return Code Constants** (MANDATORY):
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

- **Include Path Format** (MANDATORY for all project headers):
  - **ALWAYS use relative paths from `src/` directory** for all project includes
  - **CORRECT format**: `#include "services/common/video_config_types.h"`
  - **INCORRECT format**: `#include "../../services/common/video_config_types.h"`
  - **Rationale**: Consistent, maintainable, and IDE-friendly include paths
  - **Enforcement**: Makefile and clangd configuration enforce this rule
  - **Examples**:
    ```c
    // ✅ CORRECT - Relative from src/
    #include "services/common/onvif_types.h"
    #include "utils/validation/common_validation.h"
    #include "platform/platform.h"

    // ❌ INCORRECT - Relative paths with ../
    #include "../../services/common/onvif_types.h"
    #include "../validation/common_validation.h"
    ```

### Code Organization
- **Consistent 2-space indentation**
- **Consistent variable naming**
- **Global variable naming convention** (MANDATORY):
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
  - **Rationale**: Prevents naming conflicts, improves code readability, and makes module boundaries clear

- **Global variable placement** (MANDATORY):
  - **ALL global variables MUST be placed at the top of the file** after includes and before any function definitions
  - **Grouping**: Group related global variables together with blank lines between groups
  - **Initialization**: Initialize global variables at declaration when possible
  - **Documentation**: Add comments explaining the purpose of each global variable
  - **Examples**:
    ```c
    // ✅ CORRECT - Global variables at top of file
    #include <stdio.h>
    #include <stdlib.h>
    #include "platform/platform.h"

    // Global state variables
    static int g_onvif_device_count = 0;
    static int g_discovery_running = 0; //NOLINT

    // Global configuration
    static char g_platform_device_name[64] = {0};
    static struct media_profile* g_media_profiles[MAX_PROFILES] = {NULL};

    // Function definitions start here
    int onvif_init(void) {
        // ...
    }

    // ❌ INCORRECT - Global variables mixed with functions
    #include <stdio.h>
    #include "platform/platform.h"

    int onvif_init(void) {
        // ...
    }

    static int g_device_count = 0;  // WRONG: Global variable after functions
    ```
  - **Rationale**: Improves code readability, makes global state visible at a glance, and follows C best practices
- **Proper spacing and formatting**
- **Functions ordered with definitions at the top of files and execution logic at the bottom**

### Source File Structure & Organization (MANDATORY)
- **Directory Structure** - Files must be placed in correct directories according to functionality:
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

- **File Naming Conventions**:
  - **Source files**: `onvif_<service>.c` (e.g., `onvif_device.c`)
  - **Header files**: `onvif_<service>.h` (e.g., `onvif_device.h`)
  - **Utility files**: `<category>_utils.c` (e.g., `memory_utils.c`)
  - **Platform files**: `platform_<platform>.c` (e.g., `platform_anyka.c`)

- **Module Boundaries** - Clear separation between different functional areas
- **Dependency Management** - Proper include dependencies and circular dependency prevention
- **File Size** - Reasonable file sizes with clear single responsibility (max ~1000 lines)

### Utility Usage and Code Reuse (MANDATORY)
- **ALWAYS use existing utilities** - Check `src/utils/` directory for available utility functions before implementing new code
- **NO code duplication** - If similar functionality exists elsewhere, refactor to use shared utilities
- **Create utilities for common operations** - When implementing functionality that might be reused, create utility functions
- **Utility function requirements**:
  - Must be placed in appropriate `src/utils/` subdirectory
  - Must have complete Doxygen documentation
  - Must follow consistent naming conventions (e.g., `onvif_util_*` for ONVIF-specific utilities)
  - Must include proper error handling and validation
  - Must be thread-safe if applicable

### Code Duplication Prevention (MANDATORY)
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

### Utility Function Examples
```c
// ❌ BAD - Duplicated string validation in multiple files
int validate_token(const char* token) {
    if (!token || strlen(token) == 0 || strlen(token) > 32) {
        return 0;
    }
    return 1;
}

// ✅ GOOD - Centralized utility function
/**
 * @brief Validate ONVIF token format and length
 * @param token Token string to validate (must not be NULL)
 * @return 1 if valid, 0 if invalid
 * @note Token must be 1-32 characters, non-empty
 */
int onvif_util_validate_token(const char* token) {
    if (!token) {
        return 0;
    }

    size_t len = strlen(token);
    if (len == 0 || len > ONVIF_TOKEN_MAX_LENGTH) {
        return 0;
    }

    return 1;
}
```

### Security Guidelines (MANDATORY)
- **Input Validation** - ALL user inputs must be properly validated and sanitized:
  - Check for NULL pointers before dereferencing
  - Validate string lengths and bounds
  - Sanitize XML/SOAP input to prevent injection attacks
  - Validate numeric ranges and types
  - Check for buffer overflows in all string operations

- **Buffer Management** - Prevent buffer overflows and underflows:
  - Use `strncpy()` instead of `strcpy()`
  - Always null-terminate strings
  - Check array bounds before access
  - Use fixed-size buffers with proper bounds checking
  - Avoid `scanf()` with unbounded format strings

- **Memory Security** - Prevent memory-related vulnerabilities:
  - Initialize all allocated memory with `memset()` or `calloc()`
  - Free all allocated resources in error paths
  - Check for use-after-free vulnerabilities
  - Validate pointer arithmetic safety
  - Use stack variables when possible, heap when necessary

- **Network Security** - Secure network communications:
  - Validate all network input before processing
  - Use proper error message handling (no information leakage)
  - Implement proper authentication and authorization
  - Use secure communication protocols (HTTPS, WSS)
  - Validate SOAP/XML input to prevent injection

- **Authentication & Authorization**:
  - Implement proper credential handling
  - Use secure password storage mechanisms
  - Validate session management
  - Ensure proper access controls for all endpoints
  - Implement rate limiting for authentication attempts

### ONVIF Compliance Requirements (MANDATORY)
- **Service Implementation** - Complete implementation of all required services:
  - **Device Service**: Device information, capabilities, system date/time
  - **Media Service**: Profile management, stream URIs, video/audio configurations
  - **PTZ Service**: Pan/tilt/zoom control, preset management, continuous move
  - **Imaging Service**: Image settings, brightness, contrast, saturation controls

- **SOAP Protocol** - Proper XML/SOAP request/response handling:
  - Correct XML namespace usage
  - Proper SOAP envelope structure
  - ONVIF-compliant error codes and messages
  - Proper content-type headers

- **WS-Discovery** - Correct multicast discovery implementation:
  - Proper UDP multicast socket handling
  - Correct discovery message format
  - Proper device announcement and probe responses

- **RTSP Streaming** - Proper H.264 stream generation and delivery:
  - Correct SDP format for stream descriptions
  - Proper RTP packetization
  - Correct stream URI generation
  - Proper authentication for RTSP streams

- **Error Codes** - ONVIF-compliant error reporting:
  - Use standard ONVIF error codes
  - Provide meaningful error messages
  - Handle all error conditions gracefully

### Platform Integration
- **Logging functionality** should be integrated into the platform_anyka abstraction and declared in platform.h, using standard function names instead of the ak_ prefix.
- **Rootfs considerations**: When modifying `busybox` or other low-level component binaries, remember the rootfs size and `mksquashfs` options matter.

### Documentation Standards (MANDATORY)
- **Doxygen Documentation**: **ALL code changes MUST include updated Doxygen documentation**
  - **Function documentation**: Every public function must have complete Doxygen comments including `@brief`, `@param`, `@return`, and `@note` where applicable
  - **File headers**: Each source file must have a Doxygen file header with `@file`, `@brief`, `@author`, and `@date` tags
  - **Structure/Enum documentation**: All public structures, enums, and typedefs must be documented with `@brief` and member descriptions
  - **Documentation generation**: **MANDATORY** - Use `doxygen Doxyfile` in the `cross-compile/onvif/` directory to generate updated documentation
  - **Documentation validation**: **MANDATORY** - Verify that all new/changed functions appear correctly in the generated HTML documentation
  - **Consistency**: Follow the existing Doxygen style used throughout the ONVIF project

### File Header Standards (MANDATORY)
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

## Code Review Guidelines

### Code Review Process (MANDATORY)
When reviewing ONVIF project code, **ALWAYS** follow this comprehensive review process:

#### 1. Code Quality and Adherence to Best Practices
- **Coding Standards Compliance**:
  - Verify include ordering (system → third-party → project headers)
  - **MANDATORY: Check include path format** - All project includes must use relative paths from `src/`
  - Check 2-space indentation consistency
  - Validate function ordering (definitions at top, execution logic at bottom)
  - Ensure consistent variable naming conventions
  - **MANDATORY: Check global variable naming** - All global variables must start with `g_<module>_<variable_name>`
  - Verify proper spacing and formatting

- **Utility Usage and Code Reuse**:
  - **MANDATORY** - Check if existing utilities could be used instead of new code
  - Verify no code duplication exists
  - Ensure common operations use shared utility functions
  - Check if new functionality should be refactored into utilities
  - Validate utility functions follow naming conventions and documentation standards

- **Memory Management**:
  - Check for memory leaks (malloc/free pairs)
  - Verify proper error handling for memory allocation failures
  - Ensure all allocated resources are properly freed
  - Check for buffer overflows and underflows
  - Validate string operations and bounds checking

- **Error Handling**:
  - Verify all functions return appropriate error codes
  - Check for proper error propagation
  - Ensure graceful degradation on failures
  - Validate input parameter checking
  - Check for unhandled edge cases

#### 2. Potential Bugs and Edge Cases
- **Null Pointer Checks**:
  - Verify all pointer parameters are checked for NULL
  - Check for dereferencing before null checks
  - Validate pointer arithmetic safety

- **Buffer Management**:
  - Check for buffer overflows in string operations
  - Verify array bounds checking
  - Validate fixed-size buffer usage
  - Check for off-by-one errors

- **Thread Safety**:
  - Verify proper mutex usage
  - Check for race conditions
  - Validate shared resource access
  - Ensure atomic operations where needed

- **Resource Management**:
  - Check for resource leaks (file handles, sockets, etc.)
  - Verify proper cleanup in error paths
  - Ensure all resources are released on exit

#### 3. Performance Optimizations
- **Algorithm Efficiency**:
  - Check for unnecessary loops or nested iterations
  - Verify O(n) vs O(n²) complexity considerations
  - Look for redundant calculations
  - Check for early exit conditions
  - Optimize critical path functions
  - Use appropriate data structures for the use case

- **Memory Usage**:
  - Verify efficient memory allocation patterns
  - Check for memory fragmentation issues
  - Validate stack vs heap usage decisions
  - Look for unnecessary memory copies
  - Use memory pools for frequent allocations
  - Avoid memory leaks in long-running processes

- **I/O Operations**:
  - Check for blocking operations that could be optimized
  - Verify efficient data structure usage
  - Look for unnecessary system calls
  - Check for proper buffering strategies
  - Use non-blocking I/O where appropriate
  - Optimize file I/O operations

- **Threading and Concurrency**:
  - Verify proper thread synchronization
  - Check for deadlock conditions
  - Validate shared resource access patterns
  - Use appropriate locking mechanisms
  - Avoid unnecessary thread creation/destruction
  - Optimize critical section sizes

- **Network Performance**:
  - Optimize packet processing
  - Use efficient data serialization
  - Minimize network round trips
  - Implement proper connection pooling
  - Use appropriate buffer sizes for network operations

#### 4. Readability and Maintainability
- **Code Structure**:
  - Verify logical function organization
  - Check for appropriate function length (max ~50 lines)
  - Validate clear separation of concerns
  - Ensure consistent code style

- **Documentation**:
  - Verify complete Doxygen documentation
  - Check for clear function descriptions
  - Validate parameter and return value documentation
  - Ensure inline comments explain complex logic

- **Naming Conventions**:
  - Verify descriptive variable and function names
  - Check for consistent naming patterns
  - Validate appropriate use of constants vs variables
  - Ensure clear intent in naming

#### 5. Security Concerns
- **Input Validation**:
  - Check for proper input sanitization
  - Verify bounds checking on all inputs
  - Validate format string security
  - Check for injection vulnerabilities

- **Authentication and Authorization**:
  - Verify proper credential handling
  - Check for secure password storage
  - Validate session management
  - Ensure proper access controls

- **Network Security**:
  - Check for secure communication protocols
  - Verify proper error message handling (no information leakage)
  - Validate input from network sources
  - Check for proper SSL/TLS usage

- **Memory Security**:
  - Check for stack overflow vulnerabilities
  - Verify proper buffer management
  - Validate pointer arithmetic safety
  - Check for use-after-free vulnerabilities

### Static Analysis & Testing Requirements (MANDATORY)
- **Unit Testing** - **MANDATORY** for all utility functions:
  - Run unit tests: `make test` or `make test-utils`
  - All utility functions must have corresponding unit tests
  - Tests must cover success cases, error cases, and edge conditions
  - Use CMocka framework for isolated unit testing
  - Generate coverage reports: `make test-coverage-html`
  - Achieve high code coverage on utility functions
  - Tests run on development machine (native compilation)

- **Compilation Testing** - ALL code changes must compile successfully:
  - Use native build: `make -C cross-compile/onvif`
  - Fix all compiler warnings and errors
  - Verify no undefined behavior
  - Check for unused variables and functions

- **Code Linting (MANDATORY)** - **MANDATORY** for all code changes:
  - **Run linting script**: `./cross-compile/onvif/scripts/lint_code.sh` after implementation
  - **Check specific files**: `./cross-compile/onvif/scripts/lint_code.sh --file src/path/to/file.c`
  - **Check changed files**: `./cross-compile/onvif/scripts/lint_code.sh --changed`
  - **Check only mode**: `./cross-compile/onvif/scripts/lint_code.sh --check` (exits 1 if issues found)
  - **Severity levels**: Use `--severity error` to fail only on errors, `--severity warn` for warnings and errors
  - **Format checking**: Use `--format` to also check code formatting compliance
  - **All linting issues MUST be resolved** before code can be approved
  - **Linting covers**: Code style, potential bugs, performance issues, security concerns, and best practices
  - **Integration**: Linting is integrated with clangd-tidy for comprehensive analysis

- **Code Formatting (MANDATORY)** - **MANDATORY** for all code changes:
  - **Run formatting script**: `./cross-compile/onvif/scripts/format_code.sh` after implementation
  - **Check specific files**: `./cross-compile/onvif/scripts/format_code.sh --files src/path/to/file.c`
  - **Check only mode**: `./cross-compile/onvif/scripts/format_code.sh --check` (exits 1 if formatting issues found)
  - **Dry run mode**: `./cross-compile/onvif/scripts/format_code.sh --dry-run` to see what would be changed
  - **All formatting issues MUST be resolved** before code can be approved
  - **Formatting covers**: Indentation, spacing, brace placement, line length, and consistent style
  - **Integration**: Uses clang-format with project-specific configuration

- **Function Order Compliance (MANDATORY)** - **MANDATORY** verification after implementation:
  - **Verify function ordering** follows project guidelines: definitions at top, execution logic at bottom
  - **Check global variables** are placed at the top of files after includes
  - **Validate include ordering**: system headers → third-party → project headers
  - **Ensure proper spacing** and formatting between function groups
  - **Use linting script** with `--format` flag to check function ordering compliance
  - **Manual verification** may be required for complex files with many functions
  - **All ordering issues MUST be resolved** before code can be approved

- **Static Analysis** - Run automated analysis tools:
  - Use tools like `cppcheck`, `clang-static-analyzer`, or `PVS-Studio`
  - Address all critical and high-severity issues
  - Review medium-severity issues for potential problems
  - Document any false positives
  - **NOLINT Suppression** - Use `//NOLINT` comments for linter warnings when the suggested changes don't make sense from an implementation perspective:
    - **Example**: `static int g_discovery_running = 0; //NOLINT` for warnings about non-const global variables that need to be mutable
    - **Example**: `int onvif_ptz_stop(const char *profile_token, int stop_pan_tilt, int stop_zoom) //NOLINT` for warnings about adjacent parameters of similar type that are easily swapped by mistake when changing the order doesn't make sense from implementation perspective
    - **Rationale**: Some linter suggestions conflict with legitimate design requirements (e.g., global state variables that must be mutable, or function parameters where the current order is semantically correct)
    - **Documentation**: Always add a comment explaining why NOLINT is used
    - **Review**: NOLINT usage must be justified during code review

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

### Code Review Checklist
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

### Code Review Examples

#### Example 1: Utility Usage Review
```c
// ❌ BAD - Duplicated string validation
int validate_profile_token(const char* token) {
    if (!token || strlen(token) == 0 || strlen(token) > 32) {
        return 0;
    }
    return 1;
}

int validate_encoder_token(const char* token) {
    if (!token || strlen(token) == 0 || strlen(token) > 32) {
        return 0;
    }
    return 1;
}

// ✅ GOOD - Using shared utility
#include "utils/validation/input_validation.h"

int validate_profile_token(const char* token) {
    return onvif_util_validate_token(token);
}

int validate_encoder_token(const char* token) {
    return onvif_util_validate_token(token);
}
```

#### Example 2: Function Review
```c
// ❌ BAD - Missing error handling, no documentation
int process_data(char* input) {
    char buffer[100];
    strcpy(buffer, input);
    return strlen(buffer);
}

// ✅ GOOD - Proper error handling, documentation, validation
/**
 * @brief Process input data safely with bounds checking
 * @param input Input string to process (must not be NULL)
 * @return Length of processed data on success, -1 on error
 * @note Input is validated and buffer overflow is prevented
 */
int process_data(const char* input) {
    if (!input) {
        return -1;  // Input validation
    }

    char buffer[100];
    if (strlen(input) >= sizeof(buffer)) {
        return -1;  // Buffer overflow prevention
    }

    strncpy(buffer, input, sizeof(buffer) - 1);
    buffer[sizeof(buffer) - 1] = '\0';  // Null termination

    return (int)strlen(buffer);
}
```

#### Example 3: Memory Management Review
```c
// ❌ BAD - Memory leak, no error handling
void create_profile() {
    struct media_profile* profile = malloc(sizeof(struct media_profile));
    profile->token = "MainProfile";
    // Missing free() - memory leak
}

// ✅ GOOD - Proper memory management
/**
 * @brief Create a new media profile with proper memory management
 * @return Pointer to created profile on success, NULL on failure
 * @note Caller must free the returned profile using free()
 */
struct media_profile* create_profile() {
    struct media_profile* profile = malloc(sizeof(struct media_profile));
    if (!profile) {
        return NULL;  // Error handling
    }

    memset(profile, 0, sizeof(struct media_profile));
    strncpy(profile->token, "MainProfile", sizeof(profile->token) - 1);
    profile->token[sizeof(profile->token) - 1] = '\0';

    return profile;
}
```

### Review Response Format
When providing code review feedback, use this format:

```markdown
## Code Review: [File Name]

### 🔍 **Code Quality Issues**
- **Issue**: [Description]
- **Location**: [File:Line]
- **Severity**: [High/Medium/Low]
- **Reasoning**: [Why this is a problem]
- **Suggestion**: [How to fix it]

### 🐛 **Potential Bugs**
- **Issue**: [Description]
- **Location**: [File:Line]
- **Risk**: [High/Medium/Low]
- **Reasoning**: [Why this could cause problems]
- **Suggestion**: [How to prevent it]

### ⚡ **Performance Concerns**
- **Issue**: [Description]
- **Location**: [File:Line]
- **Impact**: [High/Medium/Low]
- **Reasoning**: [Why this affects performance]
- **Suggestion**: [How to optimize]

### 🔒 **Security Issues**
- **Issue**: [Description]
- **Location**: [File:Line]
- **Risk**: [High/Medium/Low]
- **Reasoning**: [Why this is a security concern]
- **Suggestion**: [How to secure it]

### 🔧 **Utility Usage Issues**
- **Issue**: [Code duplication or missing utility usage]
- **Location**: [File:Line]
- **Suggestion**: [Use existing utility or create new one]

### **Documentation Issues**
- **Issue**: [Missing or incomplete documentation]
- **Location**: [File:Line]
- **Suggestion**: [What documentation to add]

### ✅ **Positive Aspects**
- [What was done well]
- [Good practices observed]
- [Particularly well-implemented features]
```

## Agent Instructions

### When Working on This Project
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

### Documentation Workflow (MANDATORY)
1. Make code changes
2. Update Doxygen comments for all modified functions
3. Update file headers if needed
4. Run documentation generation: `make -C cross-compile/onvif docs`
5. Verify documentation in `cross-compile/onvif/docs/html/index.html`
6. Test compilation to ensure changes work
7. **Perform comprehensive code review**
8. Commit changes

### Error Handling & Debugging Guidelines (MANDATORY)
- **Error Code Strategy** - Use consistent error handling throughout:
  - Define clear error codes for all failure conditions
  - Use ONVIF-compliant error codes where applicable
  - Provide meaningful error messages for debugging
  - Log errors with appropriate severity levels
  - Handle errors gracefully without crashing

- **Debugging Tools** - Use appropriate debugging techniques:
  - Enable debug logging in development builds
  - Use `gdb` or similar debugger for complex issues
  - Add debug prints for critical code paths
  - Use memory debugging tools (valgrind, AddressSanitizer)
  - Test with various input conditions and edge cases

- **Logging Strategy** - Implement comprehensive logging:
  - Use different log levels (DEBUG, INFO, WARN, ERROR, FATAL)
  - Log all significant events and state changes
  - Include context information in log messages
  - Use structured logging for better analysis
  - Rotate log files to prevent disk space issues

- **Testing Strategy** - Comprehensive testing approach:
  - Unit tests for individual functions
  - Integration tests for service interactions
  - Performance tests for critical paths
  - Security tests for input validation
  - Regression tests for bug fixes

### Common Tasks & Examples
- **ONVIF Development**: "I modified `cross-compile/onvif/src/services/ptz/onvif_ptz.c` to improve preset handling — please build using native make, run unit tests, perform code review, and test PTZ functionality via ONVIF client."
- **Platform Updates**: "Update `cross-compile/onvif/src/platform/platform_anyka.c` to add better error handling for PTZ initialization failures — ensure proper code review for security and performance."
- **App Development**: "I changed `cross-compile/libre_anyka_app/main.c` to add logging — please build using native make, run unit tests, review for memory management issues, and test by copying the new binary to `SD_card_contents/anyka_hack/usr/bin/` and booting an SD-card image."
- **Web UI Updates**: "Patch updates `www/js/app.js` for the web UI. After applying, pack the SD payload and test UI on device at `http://<device-ip>`; capture network requests and serial log if UI doesn't load."
- **Documentation Updates**: "I added new functions to `cross-compile/onvif/src/services/device/onvif_device.c` — please update the Doxygen documentation, regenerate the HTML docs, and perform security review."
- **Unit Testing**: "I added new utility functions to `cross-compile/onvif/src/utils/validation/input_validation.c` — please create corresponding unit tests in `cross-compile/onvif/tests/unit/utils/test_validation_utils.c`, run the tests, and generate coverage report."
- **Code Review**: "Please review the ONVIF project code considering code quality, potential bugs, performance optimizations, readability, and security concerns. Suggest improvements and explain reasoning for each suggestion."
- **Utility Refactoring**: "I found duplicated string validation code in multiple files — please refactor to use the existing `onvif_util_validate_token()` utility function, update unit tests, and remove all duplicated implementations."
- **Security Audit**: "Please perform a comprehensive security review of the ONVIF implementation, focusing on input validation, buffer management, and authentication mechanisms."
- **Performance Optimization**: "The RTSP streaming performance is slow — please analyze the code for bottlenecks and suggest optimizations."
- **Memory Leak Investigation**: "The daemon is consuming increasing memory over time — please investigate and fix any memory leaks."

### Bash Command Reference (MANDATORY)

#### Build and Compilation Commands
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

#### File Operations
```bash
# List all C source files
find cross-compile/onvif/src -name "*.c" -type f

# List all header files
find cross-compile/onvif/src -name "*.h" -type f

# Copy binary to SD card
cp cross-compile/onvif/out/onvifd SD_card_contents/anyka_hack/usr/bin/

# Copy with backup
cp -f cross-compile/onvif/out/onvifd SD_card_contents/anyka_hack/usr/bin/

# Check if file exists
test -f cross-compile/onvif/out/onvifd

# Get file size
ls -l cross-compile/onvif/out/onvifd

# Compare files
diff file1.c file2.c
```

#### Code Analysis Commands
```bash
# Search for code duplication patterns
grep -r "strlen.*token.*32" cross-compile/onvif/src/ -n -A2 -B2

# Find all malloc calls
grep -r "malloc(" cross-compile/onvif/src/ -n -A1 -B1

# Find all free calls
grep -r "free(" cross-compile/onvif/src/ -n -A1 -B1

# Search for TODO comments
grep -r "TODO\|FIXME\|XXX" cross-compile/onvif/src/ -n -A1 -B1

# Find function definitions
grep -r "^[a-zA-Z_][a-zA-Z0-9_]*\s+[a-zA-Z_][a-zA-Z0-9_]*\s*(" cross-compile/onvif/src/ -n

# Count lines of code
find cross-compile/onvif/src -name "*.c" -exec wc -l {} + | tail -1
```

#### Documentation Commands
```bash
# Generate documentation
make -C cross-compile/onvif docs

# View documentation in browser
xdg-open cross-compile/onvif/docs/html/index.html

# Check if documentation exists
test -f cross-compile/onvif/docs/html/index.html

# List documentation files
find cross-compile/onvif/docs -type f
```

#### Code Quality Commands (MANDATORY)
```bash
# Code Linting (MANDATORY)
./cross-compile/onvif/scripts/lint_code.sh                    # Lint all C files
./cross-compile/onvif/scripts/lint_code.sh --check            # Check for issues (exit 1 if found)
./cross-compile/onvif/scripts/lint_code.sh --file src/path/to/file.c  # Lint specific file
./cross-compile/onvif/scripts/lint_code.sh --changed          # Lint only changed files
./cross-compile/onvif/scripts/lint_code.sh --format           # Also check formatting
./cross-compile/onvif/scripts/lint_code.sh --severity error   # Fail only on errors

# Code Formatting (MANDATORY)
./cross-compile/onvif/scripts/format_code.sh                  # Format all C files
./cross-compile/onvif/scripts/format_code.sh --check          # Check formatting (exit 1 if issues)
./cross-compile/onvif/scripts/format_code.sh --files src/path/to/file.c  # Format specific files
./cross-compile/onvif/scripts/format_code.sh --dry-run        # Show what would be changed
```

#### Testing and Debugging Commands
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

# Run with debug output
./cross-compile/onvif/out/onvifd --debug

# Check binary dependencies
ldd cross-compile/onvif/out/onvifd
```

#### Environment and Setup
```bash
# Check system status
ps aux | grep onvifd

# List running processes
ps aux

# Check environment variables
echo $PWD
echo $BUILD_TYPE

# Set environment variables
export BUILD_TYPE="debug"
export LOG_LEVEL="DEBUG"

# Check bash version
bash --version
```

#### Network and Device Testing
```bash
# Test network connectivity
nc -zv 192.168.1.100 80

# Ping device
ping -c 4 192.168.1.100

# Check if port is open
nc -zv 192.168.1.100 554

# Download file from device
wget -O response.xml "http://192.168.1.100/onvif/device_service"
```

### Agent Workflow & Best Practices (MANDATORY)

#### Pre-Development Checklist
- [ ] **Understand the task** - Read requirements carefully and ask clarifying questions
- [ ] **Check existing code** - Review related files and utilities before implementing
- [ ] **Plan the approach** - Break down complex tasks into smaller, manageable steps
- [ ] **Identify dependencies** - Understand what other components might be affected
- [ ] **Review coding standards** - Ensure compliance with project guidelines

#### Development Process
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

#### Quality Assurance Checklist
- [ ] **Code Quality**: Follows all coding standards and best practices
- [ ] **Security**: Input validation, buffer management, authentication
- [ ] **Performance**: Efficient algorithms, memory usage, I/O operations
- [ ] **Maintainability**: Clear code structure, documentation, error handling
- [ ] **ONVIF Compliance**: Proper service implementation and protocol handling
- [ ] **Testing**: Comprehensive testing and validation
- [ ] **Documentation**: Complete and up-to-date documentation

#### Common Pitfalls to Avoid
- **Code Duplication**: Always check for existing utilities before implementing new code
- **Memory Leaks**: Ensure all allocated resources are properly freed
- **Buffer Overflows**: Use safe string functions and bounds checking
- **Missing Error Handling**: Handle all error conditions gracefully
- **Incomplete Documentation**: Update documentation for all code changes
- **Security Vulnerabilities**: Validate all inputs and use secure coding practices
- **Performance Issues**: Profile critical code paths and optimize bottlenecks

### Memory & Context
- This project focuses on reverse-engineering Anyka AK3918 camera firmware and developing custom implementations
- **Reference implementation**: `cross-compile/anyka_reference/akipc/` contains the authoritative vendor code for understanding camera behavior
- **Component libraries**: `cross-compile/anyka_reference/component/` and `platform/` provide reusable drivers and hardware abstraction
- Native cross-compilation tools are used for consistent builds in WSL Ubuntu
- SD-card payload testing is the primary iteration method for safe device testing
- ONVIF 2.5 implementation is the current focus area with full service compliance
- Platform abstraction and logging integration are key architectural concerns
- **Bash is mandatory** for all terminal operations in WSL Ubuntu development environment
- **Documentation updates are mandatory** for all code changes with Doxygen compliance
- **Code review is mandatory** for all code changes with comprehensive security and performance analysis
- **Utility usage is mandatory** - no code duplication allowed, always use shared utilities
- **Security is paramount** - all code must be secure and robust with proper input validation
- **ONVIF compliance is critical** - must follow ONVIF 2.5 specification for all services
- **Performance matters** - optimize for embedded camera systems with efficient resource usage
- **Testing is essential** - comprehensive testing before deployment including integration tests
