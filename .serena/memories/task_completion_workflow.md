# Task Completion Workflow

## Mandatory Steps for Task Completion

### 1. Code Development Phase
- **Follow include ordering** standards strictly
- **Use existing utilities** - check `src/utils/` before implementing new code
- **Implement proper error handling** and validation
- **Add comprehensive Doxygen documentation** for all new/modified functions
- **Use consistent naming conventions** and formatting
- **Follow security guidelines** with input validation

### 2. Code Quality Validation Phase (MANDATORY)
```bash
# Code linting (MANDATORY - must pass before approval)
./cross-compile/onvif/scripts/lint_code.sh --check

# Code formatting (MANDATORY - must pass before approval)
./cross-compile/onvif/scripts/format_code.sh --check

# Auto-format if needed
./cross-compile/onvif/scripts/format_code.sh

# Verify function ordering
# - Definitions at top of files
# - Execution logic at bottom
# - Global variables at top after includes
```

### 3. Build and Test Phase (MANDATORY)
```bash
# Test compilation
make -C cross-compile/onvif clean
make -C cross-compile/onvif

# Run unit tests (MANDATORY for utility functions)
make -C cross-compile/onvif test

# Run utility-specific tests
make -C cross-compile/onvif test-utils

# Generate coverage report
make -C cross-compile/onvif test-coverage-html

# Memory leak detection
make -C cross-compile/onvif test-valgrind
```

### 4. Documentation Phase (MANDATORY)
```bash
# Update Doxygen documentation
make -C cross-compile/onvif docs

# Verify documentation generated correctly
test -f cross-compile/onvif/docs/html/index.html
```

### 5. Static Analysis Phase
```bash
# Run static analysis
make -C cross-compile/onvif static-analysis

# Individual tools
make -C cross-compile/onvif clang-analyze
make -C cross-compile/onvif cppcheck-analyze
make -C cross-compile/onvif snyk-analyze
```

### 6. Integration Testing Phase
```bash
# Copy binary to SD card payload
cp cross-compile/onvif/out/onvifd SD_card_contents/anyka_hack/usr/bin/

# Copy configuration files
cp cross-compile/onvif/configs/anyka_cfg.ini SD_card_contents/anyka_hack/onvif/

# Test on actual device using SD card boot (primary testing method)
```

### 7. Final Verification Checklist
- [ ] **Code linting passed** with zero errors (MANDATORY)
- [ ] **Code formatting passed** with zero issues (MANDATORY)
- [ ] **Function ordering verified** (definitions at top, logic at bottom)
- [ ] **Global variables at top** after includes
- [ ] **Test naming follows convention** (`test_unit_*` or `test_integration_*`)
- [ ] **Code compiles** without warnings or errors
- [ ] **Unit tests pass** (if utility functions modified)
- [ ] **Documentation generated** successfully
- [ ] **Static analysis** shows no critical issues
- [ ] **Memory analysis** shows no leaks (if applicable)
- [ ] **Integration tests** pass (if applicable)
- [ ] **Security review** completed for input validation
- [ ] **Code review** completed for quality and standards compliance

## Development Environment Requirements
- **Operating System**: WSL2 Ubuntu (MANDATORY)
- **Shell**: bash syntax only (MANDATORY)
- **Cross-compilation**: Native toolchain, no Docker unless specified
- **Path handling**: Unix-style paths with forward slashes

## Common Task Types and Specific Requirements

### ONVIF Service Development
- Update service implementations in `cross-compile/onvif/src/services/`
- Ensure ONVIF 2.5 compliance
- Test with ONVIF clients
- Update platform abstraction if needed

### Utility Function Development
- **MANDATORY unit tests** for all utility functions
- Place in appropriate `src/utils/` subdirectory
- Complete Doxygen documentation
- Thread-safe implementation where applicable
- Test naming: `test_unit_<module>_<functionality>`

### Integration Test Development
- Place in `cross-compile/onvif/tests/src/integration/`
- Test naming: `test_integration_<service>_<functionality>`
- Test complete workflows and service interactions

### Platform Integration
- Modify `cross-compile/onvif/src/platform/platform_anyka.c`
- Ensure proper resource management
- Test hardware integration
- Update platform interface if needed

### Web Interface Updates
- Modify files in `www/cgi-bin/`
- Test both ONVIF and legacy interfaces
- Ensure proper SOAP request handling
- Verify network security

## Error Handling During Task Completion
- **Linting Errors**: Fix all errors before proceeding (MANDATORY)
- **Formatting Issues**: Auto-format or fix manually before proceeding (MANDATORY)
- **Compilation Errors**: Fix immediately, do not proceed without clean build
- **Test Failures**: Investigate and fix before proceeding
- **Documentation Issues**: Regenerate docs and verify completeness
- **Static Analysis Issues**: Address critical and high-severity findings
- **Memory Issues**: Fix all leaks and overflow vulnerabilities before completion