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

**Unit Testing (MANDATORY)**:
- **ALL utility functions** must have corresponding unit tests
- **Tests must cover** success cases, error cases, and edge conditions
- **Use CMocka framework** for isolated unit testing
- **Run tests** before committing any changes: `make test`
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
- `integration-tests/` - Python-based test suite with pytest

## Common Development Commands

### Building the ONVIF Server

```bash
# WSL native build (primary method)
cd cross-compile/onvif
make

# Debug build with symbols
make debug

# Run unit tests (MANDATORY)
make test
make test-utils

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

### Unit Testing Commands (MANDATORY)

```bash
# Install unit test dependencies
./cross-compile/onvif/tests/install_dependencies.sh

# Run all unit tests
make test

# Run utility unit tests
make test-utils

# Run tests with coverage analysis
make test-coverage

# Generate HTML coverage report
make test-coverage-html

# Generate coverage summary
make test-coverage-report

# Run tests with valgrind (memory checking)
make test-valgrind

# Clean coverage files
make test-coverage-clean
```

### Integration Testing Commands

```bash
# Run integration tests (Python-based)
cd integration-tests
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

4. **Doxygen Documentation**: ALL code changes must include complete documentation:
   ```c
   /**
    * @brief Brief description of function purpose
    * @param param Description of parameter
    * @return Description of return value
    * @note Additional notes about usage
    */
   ```

5. **File Headers**: All files must have consistent headers:
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

### Integration Tests (Python)
Located in `integration-tests/` directory with comprehensive test coverage:
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
3. **Build**: Test compilation with native build environment
4. **Unit Test**: **MANDATORY** - Run unit tests for all utility functions
5. **Document**: Update Doxygen documentation for all changes
6. **Analyze**: Run static analysis tools to ensure code quality
7. **Test**: Use integration tests and SD card testing
8. **Review**: Comprehensive code review covering security and performance

**Note**: The project focuses on defensive security only. All code must be secure and robust with proper input validation and error handling.
