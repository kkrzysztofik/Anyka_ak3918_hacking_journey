# Anyka AK3918 Camera Firmware Constitution

## Core Principles

### I. Security First (NON-NEGOTIABLE)

All code must be secure by design with comprehensive protection against common attack vectors:

- **Input Validation**: Comprehensive validation and sanitization for all network inputs
- **Memory Safety**: Buffer overflow prevention using safe operations (strncpy over strcpy, proper bounds checking, memset/calloc initialization)
- **Threat Protection**: Defense against network attacks, authentication bypass attempts, memory corruption vulnerabilities, unauthorized stream access
- **Security Logging**: Event logging for all authentication attempts, failed requests, and security-relevant operations
- **Resource Cleanup**: Proper cleanup in all error paths to prevent resource leaks

### II. Standards Compliance

Strict adherence to industry standards for IP camera interoperability and protocol compliance:

- **ONVIF 24.12 Specification**: Complete implementation with ONVIF Core Specification requirements
- **HTTP/1.1 Compliance**: Full compliance with RFC 9112 (message syntax and routing) for HTTP server implementation
- **Profile S Compliance**: 100% pass rate on ONVIF Profile S test suite
- **Profile T Capabilities**: Documented Profile T capabilities for advanced features
- **RTSP RFC 2326**: Real-time streaming protocol standard compliance
- **WS-Discovery**: Web Services Discovery for automatic camera detection
- **Validation**: 100% pass rate on ONVIF conformance tests and HTTP compliance validation

### III. Open Source Transparency

Complete transparency in code, documentation, and development process:

- **Source Code**: All code is open source with clear licensing
- **API Documentation**: Comprehensive Doxygen documentation for all public APIs
- **Development Process**: Open feature branch workflow with full visibility
- **Code Review**: Mandatory review for all changes with security and performance analysis
- **Decision Documentation**: Architectural decisions and rationale clearly documented
- **Community**: Welcoming contribution process with clear guidelines

### IV. Hardware Compatibility

Maintain compatibility with target hardware while enabling future portability:

- **Anyka AK3918 SoC**: Primary target platform with full hardware support
- **Platform Abstraction**: Clean abstraction layer for hardware independence
- **ARM Architecture**: Cross-compilation support for ARM-based systems
- **Resource Constraints**: Design for embedded systems with limited RAM and flash storage
- **Hardware Access**: Direct hardware access through platform abstraction layer

### V. Developer Experience

Provide excellent tools, documentation, and workflow for contributors:

- **Code Quality Tools**: Linting (`lint_code.sh`), formatting (`format_code.sh`), static analysis (cppcheck, Clang analyzer, Snyk)
- **Testing Framework**: CMocka for unit and integration testing with comprehensive mocking support
- **Documentation**: Doxygen-generated API docs, README files, architecture guides, testing documentation
- **Development Environment**: SD-card based testing for safe iteration without flash modification
- **Build System**: GNU Make with clear targets, verbose output, and compilation database generation
- **Debugging Support**: Clear error messages, comprehensive logging, core dump analysis tools

### VI. Performance Optimization

Optimize for embedded systems with efficient resource usage:

- **Response Time**: Sub-second (<1s) response for all ONVIF operations under nominal load
- **Video Streaming**: Real-time H.264 streaming at 720p with bitrate variance <5%
- **Startup Time**: Camera operational within 30 seconds of power-on
- **Memory Usage**: Efficient memory management optimized for limited RAM
- **Concurrent Clients**: Support for multiple simultaneous ONVIF clients (1-10 per camera)
- **Resource Monitoring**: Real-time monitoring of CPU, memory, and network usage

### VII. Reliability & Production-Readiness

Create robust, production-ready firmware with consistent operation:

- **Uptime Requirement**: 99.9% uptime measured over 30-day soak tests
- **Error Handling**: Robust error handling in all code paths with proper cleanup
- **Network Recovery**: Automated recovery from network interruptions and service failures
- **Consistency**: Reliable behavior across different camera models and configurations
- **Monitoring**: Health checks, service availability monitoring, and alerting
- **Rollback Safety**: SD-card development environment enables safe rollback

### VIII. Educational Value

Make the codebase a learning resource for embedded systems and IoT security:

- **Code Organization**: Clear structure following established patterns
- **Documentation**: Well-documented architectural decisions and design rationale
- **Testing Examples**: Comprehensive unit and integration test examples
- **Best Practices**: Demonstration of embedded systems security and performance practices
- **Reference Implementation**: Complete ONVIF implementation as reference for others

## Technical Standards

### Language & Toolchain

- **Primary Language**: C (C99 standard) for embedded systems programming
- **Compiler**: GCC cross-compiler for ARM architecture (Anyka AK3918 SoC)
- **Build System**: GNU Make with cross-compilation support
- **Documentation**: Doxygen for API documentation generation

### Architecture

**Layered Architecture with Platform Abstraction** (dependencies flow downward):

1. **Application Layer**: Camera daemon, SOAP/HTTP APIs, web interface bundles
2. **Network Layer**: HTTP/SOAP server (RFC 9112 compliant), RTSP streaming, WS-Discovery
3. **Services Layer**: ONVIF service implementations (Device, Media, PTZ, Imaging, Snapshot)
4. **Core Layer**: Configuration management, lifecycle management, service orchestration
5. **Utility Layer**: Memory management, string utilities, validation, logging, security
6. **Platform Layer**: Hardware abstraction for Anyka AK3918 SoC

**Dependency Rules**:

- Upper layers may depend on lower layers only
- Lower layers must never depend on upper layers
- Services depend on utilities; utilities are independent
- Application depends on platform; platform is hardware-specific
- No circular dependencies allowed

### Key Technologies

- **gSOAP 2.8+**: SOAP/XML processing for ONVIF services
- **CMocka**: Unit testing framework with mocking support (MANDATORY for all utilities)
- **Anyka SDK**: Hardware abstraction layer and device drivers
- **uClibc**: Embedded C library optimized for resource-constrained systems
- **BusyBox**: Lightweight Unix utilities for embedded Linux
- **gcov/lcov**: Code coverage analysis tools

### Memory Safety (MANDATORY)

- **Safe String Operations**: Use `strncpy()`, `snprintf()`, `strncat()` with proper size limits
- **Memory Initialization**: Use `memset()` or `calloc()` to initialize all allocated memory
- **Bounds Checking**: Validate all buffer operations with proper size checks
- **Resource Cleanup**: Free all allocated resources in success and error paths
- **NULL Checks**: Validate all pointers before dereferencing

### Code Quality Tools (MANDATORY)

All code must pass the following validations:

- **Linting**: `./cross-compile/onvif/scripts/lint_code.sh --check` (zero errors required)
- **Formatting**: `./cross-compile/onvif/scripts/format_code.sh --check` (zero issues required)
- **Static Analysis**: Cppcheck, Clang Static Analyzer for bug detection
- **Security Analysis**: Snyk Code for vulnerability detection
- **Coverage Analysis**: gcov/lcov for test coverage reporting (target: >80% coverage)

## Development Workflow

### Pre-Commit Validation (MANDATORY)

Every code change MUST pass these validation steps before commit:

1. **Code Linting**:

   ```bash
   ./cross-compile/onvif/scripts/lint_code.sh --check
   ```

   - Must pass with zero errors
   - Checks code quality issues and potential bugs

2. **Code Formatting**:

   ```bash
   ./cross-compile/onvif/scripts/format_code.sh --check
   ```

   - Must pass with zero issues
   - Use without `--check` flag to auto-format

3. **Function Ordering**:

   - Global variables at top after includes (format: `g_<module>_<variable_name>`)
   - Function declarations/definitions at top
   - Execution logic at bottom

4. **Unit Testing**:

   ```bash
   make test
   ```

   - ALL utility functions must have unit tests
   - Tests must pass with 100% success rate
   - Integration tests for service-level functionality

5. **Documentation**:

   ```bash
   make -C cross-compile/onvif docs
   ```

   - All public APIs must have complete Doxygen documentation
   - Update documentation for all changed APIs

### Naming Conventions (MANDATORY)

- **Global Variables**: `g_<module>_<variable_name>` (e.g., `g_onvif_device_count`, `g_platform_device_name`)

  - MUST be placed at top of file after includes
  - Module prefix should match the source file or functional area

- **Functions**: `onvif_<service>_<action>` (e.g., `onvif_device_get_info`, `onvif_media_get_profiles`)

- **Constants**:

  - Service constants: `ONVIF_<SERVICE>_<CONSTANT>` (e.g., `ONVIF_DEVICE_MAX_PROFILES`)
  - Return codes: `<MODULE>_SUCCESS`, `<MODULE>_ERROR_<TYPE>` (e.g., `HTTP_AUTH_SUCCESS`, `ONVIF_ERROR_INVALID`)
  - **NEVER use magic numbers for return codes** (e.g., avoid `return -1`, `return 0`)

- **Types**: `onvif_<service>_<type>_t` (e.g., `onvif_device_info_t`, `onvif_media_profile_t`)

- **Test Functions**:

  - Unit tests: `test_unit_<module>_<functionality>` (e.g., `test_unit_memory_manager_init`)
  - Integration tests: `test_integration_<service>_<functionality>` (e.g., `test_integration_http_message_format`)

- **Files**:
  - Source: `onvif_<service>.c` (e.g., `onvif_device.c`)
  - Headers: `onvif_<service>.h` (e.g., `onvif_device.h`)
  - Utils: `<category>_utils.c` (e.g., `memory_utils.c`)
  - Tests: `test_unit_<module>_<functionality>.c`, `test_integration_<service>_<functionality>.c`

### Include Order (MANDATORY)

All source files must follow this include order:

1. **System headers** (e.g., `#include <stdio.h>`, `#include <stdlib.h>`, `#include <string.h>`)
2. **Third-party library headers** (e.g., `#include <curl/curl.h>`, `#include <gsoap/soapH.h>`)
3. **Project-specific headers** (e.g., `#include "platform/platform.h"`, `#include "services/common/onvif_types.h"`)

**Include Path Format**:

- **ALWAYS use relative paths from `src/` directory** for project includes
- **CORRECT**: `#include "services/common/video_config_types.h"`
- **INCORRECT**: `#include "../../services/common/video_config_types.h"`

### Code Organization Principles

1. **Single Responsibility**: Each file has one clear purpose (one service per file)
2. **Modularity**: Code organized into reusable modules with clear interfaces
3. **Testability**: Structure code to be easily testable with dependency injection
4. **Consistency**: Follow patterns established in the codebase
5. **Utility Reuse**: ALWAYS check `src/utils/` for existing functionality before implementing new code (NO code duplication allowed)
6. **Test Coverage**: ALL utility functions MUST have unit tests

### Code Size Guidelines

- **File Size**: Maximum ~1000 lines; recommended 200-500 lines for maintainability
- **Function Size**: Maximum ~50 lines; recommended 10-30 lines for readability
- **Cyclomatic Complexity**: Maximum 10 per function
- **Nesting Depth**: Maximum 4 levels
- **Parameter Count**: Maximum 5 parameters per function

### Testing Requirements (MANDATORY)

- **Unit Tests**: All utility functions must have comprehensive unit tests using CMocka
- **Integration Tests**: All service-level functionality must have integration tests
- **HTTP Compliance Tests**: HTTP server must pass RFC 9112 compliance validation
- **ONVIF Conformance Tests**: 100% pass rate on ONVIF Profile S test suite
- **E2E Tests**: Python-based end-to-end tests with physical cameras
- **Coverage Target**: Minimum 80% code coverage with lcov reporting
- **Mock Usage**: Use CMocka mocking framework for isolated unit testing

### Module Boundaries

**Core vs Services**:

- Core: Essential system components (config, lifecycle, main)
- Services: ONVIF implementations (device, media, ptz, imaging)
- Rule: Services depend on core; core is independent

**Platform vs Application**:

- Platform: Hardware abstraction layer (platform_anyka.c)
- Application: ONVIF services and business logic
- Rule: Application depends on platform; platform is hardware-specific

**Utilities vs Services**:

- Utilities: Reusable functions (memory, string, validation, security)
- Services: ONVIF service implementations
- Rule: Services depend on utilities; utilities are independent

**Public vs Internal**:

- Public API: Functions declared in header files
- Internal: Static functions and private implementation details
- Rule: Public API can depend on internal; internal cannot depend on public

**Stable vs Experimental**:

- Stable: Production-ready code with full testing
- Experimental: New features under development
- Rule: Stable code cannot depend on experimental

## Governance

### Constitutional Authority

- This constitution supersedes all other development practices and guidelines
- All code changes, pull requests, and reviews MUST verify compliance with constitutional principles
- Non-compliance is grounds for change rejection
- Questions of interpretation are resolved by referring to steering documents: `.spec-workflow/steering/product.md`, `tech.md`, `structure.md`

### Compliance Verification

All code contributions must pass:

1. **Pre-commit validation gates** (linting, formatting, function ordering)
2. **Unit testing requirements** (100% pass rate, coverage targets)
3. **Security review** (input validation, memory safety, threat protection)
4. **Standards compliance** (ONVIF 24.12, HTTP/1.1 RFC 9112, Profile S)
5. **Documentation completeness** (Doxygen comments, API docs, rationale)

### Amendment Process

Constitutional amendments require:

1. **Documentation**: Proposed changes documented with clear rationale
2. **Impact Analysis**: Assessment of changes to existing code and workflows
3. **Approval**: Review and approval from project maintainers
4. **Migration Plan**: Strategy for updating existing code to comply
5. **Version Update**: Constitution version increment and changelog entry

### Version History

**Version**: 1.0.0 | **Ratified**: 2025-10-10 | **Last Amended**: 2025-10-10

**Changelog**:

- 1.0.0 (2025-10-10): Initial constitution ratified, synthesizing principles from steering documents (product.md, tech.md, structure.md)
