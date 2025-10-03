# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a comprehensive reverse-engineering project for Anyka AK3918-based IP cameras, featuring:

- **Complete ONVIF 2.5 implementation** with Device, Media, PTZ, and Imaging services
- **Platform abstraction layer** for hardware-agnostic operations
- **RTSP streaming** with H.264 video and audio support
- **Native cross-compilation environment** in WSL Ubuntu for consistent builds
- **SD card payload system** for safe firmware testing without flash modifications
- **Comprehensive unit testing framework** using CMocka for utility function testing

## Development Environment

**MANDATORY**: All development uses **WSL Ubuntu** with **bash syntax only** and **native cross-compilation toolchain**. The project includes comprehensive coding standards and mandatory documentation requirements detailed in `AGENTS.md`.

### Critical Coding Standards

**Return Code Constants (MANDATORY)**:

- **NEVER use magic numbers** for return codes (`return -1`, `return 0`)
- **ALWAYS use predefined constants** for all function return values
- **Module-specific constants** defined in module headers (e.g., `HTTP_AUTH_SUCCESS`, `HTTP_AUTH_ERROR_NULL`)
- **Global constants** defined in `utils/error/error_handling.h` (e.g., `ONVIF_SUCCESS`, `ONVIF_ERROR_INVALID`)
- **Enforcement**: All code reviews must verify constant usage instead of magic numbers

**Test Naming Convention (MANDATORY)**:

- **Unit tests**: Must follow `test_unit_<module>_<functionality>` pattern
- **Integration tests**: Must follow `test_integration_<service>_<functionality>` pattern
- **Examples**:
  - Unit test: `test_unit_memory_manager_init`, `test_unit_http_auth_verify_credentials_success`
  - Integration test: `test_integration_ptz_absolute_move_functionality`, `test_integration_media_profile_operations`
- **Enforcement**: Test runner uses naming convention to filter tests by category
- **Consistency**: All test functions must follow this naming pattern for proper categorization

**Unit Testing (MANDATORY)**:

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

### Core Architecture

The system uses a layered architecture:

```
Web Interface → CGI Scripts → HTTP/SOAP → ONVIF Services → Platform Abstraction → Anyka SDK → Hardware
```

**Key directories:**

- `cross-compile/onvif/` - **CURRENT FOCUS** - Complete ONVIF 2.5 implementation
- `cross-compile/onvif/tests/` - **MANDATORY** - Unit testing framework using CMocka
- `cross-compile/anyka_reference/akipc/` - Authoritative vendor reference code
- `SD_card_contents/anyka_hack/` - SD card payload for runtime testing
- `e2e/` - Python-based test suite with pytest

## Common Development Commands

### Building the ONVIF Server

```bash
# WSL native build (primary method)
cd cross-compile/onvif
make

# Debug build with symbols
make debug

# Run unit tests (MANDATORY) - 196 tests across 17 suites
make test
make test-unit              # Unit tests only (129 tests)
make test-integration       # Integration tests only (67 tests)
make test-list              # List all test suites
make test SUITE=ptz-service # Run specific suite

# Release build (optimized)
make release

# Generate documentation (MANDATORY after code changes)
make docs

# Static analysis
make static-analysis

# Individual analysis tools
make clang-analyze
make cppcheck-analyze
make snyk-analyze

# Clean build artifacts
make clean
```

### Code Quality Validation (MANDATORY)

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

# Function Order Compliance (MANDATORY verification)
# Verify function ordering follows guidelines: definitions at top, execution logic at bottom
# Check global variables are placed at top of files after includes
# Validate include ordering: system headers → third-party → project headers
```

### Unit Testing Commands (MANDATORY)

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

### E2E Testing Commands

```bash
# Run E2E tests (Python-based)
cd e2e
pip install -r requirements.txt
python run_tests.py

# Run specific test categories
python run_tests.py --category device
python run_tests.py --category ptz
python run_tests.py --category media

# Test with coverage
python run_tests.py --coverage
```

### SD Card Testing (Primary Workflow)

```bash
# Build and copy compiled binary to SD payload
cd cross-compile/onvif && make
cp out/onvifd ../../SD_card_contents/anyka_hack/usr/bin/

# Copy configuration
cp configs/anyka_cfg.ini ../../SD_card_contents/anyka_hack/onvif/

# Deploy to SD card and test on actual device
# Safe testing - leaves original firmware intact
```

## Code Architecture & Standards

### ONVIF Service Implementation

The ONVIF implementation follows a modular architecture:

**Core Services** (`src/services/`):

- **Device Service** - Device information, capabilities, system date/time
- **Media Service** - Profile management, stream URIs, video/audio configurations
- **PTZ Service** - Pan/tilt/zoom control, preset management, continuous movement
- **Imaging Service** - Image settings, brightness, contrast, saturation controls

**Platform Abstraction** (`src/platform/platform_anyka.c`):

- Hardware-agnostic interface for all camera operations
- Anyka SDK integration with proper resource management
- Video/audio processing, PTZ control, IR LED management

**Network Layer** (`src/networking/`):

- HTTP/SOAP server with ONVIF protocol compliance
- RTSP streaming with H.264 encoding
- WS-Discovery for automatic device discovery
- Connection management with thread pooling

### Mandatory Coding Standards

**CRITICAL REQUIREMENTS** (see `AGENTS.md` for complete details):

1. **Include Path Format**: Always use relative paths from `src/` directory

   ```c
   // ✅ CORRECT
   #include "services/common/onvif_types.h"
   // ❌ INCORRECT
   #include "../../services/common/onvif_types.h"
   ```

2. **Global Variable Naming**: Must use `g_<module>_<variable_name>` format

   ```c
   static int g_onvif_device_count = 0;
   static char g_platform_device_name[64] = {0};
   ```

3. **Utility Usage**: MANDATORY - Always check `src/utils/` for existing functionality before implementing new code. NO code duplication allowed.

4. **Code Quality Validation**: **MANDATORY** - Run linting and formatting checks after implementation

   - **Linting**: `./cross-compile/onvif/scripts/lint_code.sh --check`
   - **Formatting**: `./cross-compile/onvif/scripts/format_code.sh --check`
   - **Function Ordering**: Verify definitions at top, execution logic at bottom
   - **Global Variables**: Must be placed at top of files after includes
   - **All issues MUST be resolved** before code can be approved

5. **Doxygen Documentation**: ALL code changes must include complete documentation:

   ```c
   /**
    * @brief Brief description of function purpose
    * @param param Description of parameter
    * @return Description of return value
    * @note Additional notes about usage
    */
   ```

6. **File Headers**: All files must have consistent headers:
   ```c
   /**
    * @file filename.h
    * @brief Brief description of file purpose
    * @author kkrzysztofik
    * @date 2025
    */
   ```

### Build System Features

The Makefile supports:

- **Debug/Release builds**: `make debug` vs `make release` with automatic BUILD_TYPE handling
- **Documentation generation**: `make docs` (MANDATORY after changes)
- **Static analysis**: Multiple tools including Clang, Cppcheck, Snyk via `make static-analysis`
- **Compile commands**: `make compile-commands` for clangd support
- **Cross-compilation**: Uses arm-anykav200-linux-uclibcgnueabi toolchain

## Testing Strategy

### E2E Tests (Python)

Located in `e2e/` directory with comprehensive test coverage:

- Device service functionality tests
- PTZ movement and preset tests
- Media profile and streaming tests
- Imaging parameter adjustment tests
- Error handling and edge cases

### SD Card Testing

Primary development workflow for device testing:

1. Compile ONVIF binary with Docker
2. Copy to SD card payload directory
3. Boot device with SD card (leaves original firmware intact)
4. Test functionality via web interface and ONVIF clients

### Static Analysis

Integrated tools for code quality:

- **Clang Static Analyzer**: Memory leaks, null pointer dereferences
- **Cppcheck**: Bug detection, style issues
- **Snyk Code**: Security vulnerability scanning

## Security & Performance

### Security Requirements

- **Input validation**: All user inputs must be validated and sanitized
- **Buffer management**: Use safe string functions, proper bounds checking
- **Memory security**: Initialize all allocated memory, proper cleanup
- **Network security**: Validate all network input, secure protocols

### Performance Considerations

- **Embedded optimization**: Code optimized for Anyka AK3918 resource constraints
- **Memory efficiency**: Careful memory allocation patterns
- **Real-time processing**: Video/audio streaming with minimal latency
- **Resource cleanup**: Proper resource management using platform abstraction

## Reference Documentation

- `AGENTS.md` - Complete coding standards, security guidelines, and development workflow
- `README.md` - Project features, installation, and usage instructions
- `cross-compile/onvif/docs/html/index.html` - Generated Doxygen documentation
- `hack_process/README.md` - Detailed reverse engineering process

## Development Workflow

1. **Plan**: Use TodoWrite tool for task management and tracking
2. **Code**: Follow mandatory standards in `AGENTS.md`
3. **Code Quality Validation**: **MANDATORY** - Run linting and formatting checks
   - `./cross-compile/onvif/scripts/lint_code.sh --check`
   - `./cross-compile/onvif/scripts/format_code.sh --check`
   - Verify function ordering compliance
   - Check global variable placement
4. **Build**: Test compilation with native build environment
5. **Unit Test**: **MANDATORY** - Run unit tests for all utility functions
6. **Document**: Update Doxygen documentation for all changes
7. **Analyze**: Run static analysis tools to ensure code quality
8. **Test**: Use E2E tests and SD card testing
9. **Review**: Comprehensive code review covering security and performance

**Note**: The project focuses on defensive security only. All code must be secure and robust with proper input validation and error handling.
