# Development Workflow - Anyka AK3918 Project

## Quick Start Entry Points

- **Fast iteration**: Edit or add files under `cross-compile/<app>/` and test via the SD-card hack in `SD_card_contents/anyka_hack/`. This allows testing changes without flashing the device.
- **Key documentation**:
  - Top-level `README.md` — project goals, features, SD-card hack quick start, and important device commands for UART access and debugging.
  - `cross-compile/README.md` — cross-compile environment setup notes and toolchain configuration.

## Build Process

- **Use native cross-compilation tools**: `make -C cross-compile/<project>` - This ensures consistent builds in the WSL Ubuntu environment.
- **Test compilation** before committing changes - Always verify that code compiles successfully before making changes.
- **SD-card testing** is the primary development workflow for device testing - Copy compiled binaries to the SD card payload and boot the device with the SD card to test functionality.
- **Unit testing** is **MANDATORY** for all utility functions:
  - Run all tests: `make test` (dynamic test count across 17 suites)
  - Run specific suites: `make test SUITE=ptz-service,http-auth`
  - List available suites: `make test-list`
  - Filter by type: `make test-unit` or `make test-integration`
- **E2E testing** can be done using the test suite in `e2e/` directory.

## Development Environment

### Quick Environment Summary

- **Primary OS**: WSL2 with Ubuntu
- **Shell**: **MANDATORY** - All commands use bash syntax
- **Cross-compilation**: Native tools for consistent builds
- **SD-card testing**: Primary development workflow for device testing

## Common Tasks & Examples

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

## Essential Commands

### Quick Commands

```bash
# Build & Test
make -C cross-compile/onvif          # Build
make test                           # All tests
make test-unit                      # Unit tests only

# Code Quality
./cross-compile/onvif/scripts/lint_code.sh --check
./cross-compile/onvif/scripts/format_code.sh --check

# Documentation
make -C cross-compile/onvif docs    # Generate docs
```

### Build System Features

The Makefile supports:

- **Debug/Release builds**: `make debug` vs `make release` with automatic BUILD_TYPE handling
- **Documentation generation**: `make docs` (MANDATORY after changes)
- **Static analysis**: Multiple tools including Clang, Cppcheck, Snyk via `make static-analysis`
- **Compile commands**: `make compile-commands` for clangd support
- **Cross-compilation**: Uses arm-anykav200-linux-uclibcgnueabi toolchain

## Agent Workflow & Best Practices (MANDATORY)

### Pre-Development Checklist

- [ ] **Understand the task** - Read requirements carefully and ask clarifying questions
- [ ] **Check existing code** - Review related files and utilities before implementing
- [ ] **Plan the approach** - Break down complex tasks into smaller, manageable steps
- [ ] **Identify dependencies** - Understand what other components might be affected
- [ ] **Review coding standards** - Ensure compliance with project guidelines

### Development Process

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

### Quality Assurance Checklist

- [ ] **Code Quality**: Follows all coding standards and best practices
- [ ] **Security**: Input validation, buffer management, authentication
- [ ] **Performance**: Efficient algorithms, memory usage, I/O operations
- [ ] **Maintainability**: Clear code structure, documentation, error handling
- [ ] **ONVIF Compliance**: Proper service implementation and protocol handling
- [ ] **Testing**: Comprehensive testing and validation
- [ ] **Documentation**: Complete and up-to-date documentation

### Common Pitfalls to Avoid

- **Code Duplication**: Always check for existing utilities before implementing new code
- **Memory Leaks**: Ensure all allocated resources are properly freed
- **Buffer Overflows**: Use safe string functions and bounds checking
- **Missing Error Handling**: Handle all error conditions gracefully
- **Incomplete Documentation**: Update documentation for all code changes
- **Security Vulnerabilities**: Validate all inputs and use secure coding practices
- **Performance Issues**: Profile critical code paths and optimize bottlenecks

## Memory & Context

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
