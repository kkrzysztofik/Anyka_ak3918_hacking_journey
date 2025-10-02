# ONVIF Project Comprehensive Review Prompt

## Project Context
You are reviewing the **ONVIF 2.5 implementation for Anyka AK3918 cameras** - a complete cross-platform ONVIF daemon with Device, Media, PTZ, and Imaging services, RTSP streaming, and comprehensive platform abstraction. This is a production-ready C implementation targeting embedded camera systems.

## Review Scope & Objectives

### 🎯 **Primary Review Goals**
1. **Code Quality & Standards Compliance** - Ensure adherence to project coding standards and best practices
2. **Security Assessment** - Identify security vulnerabilities and hardening opportunities
3. **Performance Analysis** - Evaluate efficiency, memory usage, and optimization opportunities
4. **Architecture Review** - Assess design patterns, modularity, and maintainability
5. **ONVIF Compliance** - Verify proper implementation of ONVIF 2.5 specification
6. **Platform Integration** - Review Anyka AK3918 hardware abstraction and integration

### �� **Review Checklist**

#### **1. Code Quality & Standards**
- [ ] **Include Ordering** - System headers → Third-party → Project headers (alphabetical within groups)
- [ ] **Include Path Format** - **MANDATORY**: All project includes must use relative paths from `src/` directory
- [ ] **Indentation & Formatting** - Consistent 2-space indentation, proper spacing
- [ ] **Function Organization** - Definitions at top, execution logic at bottom
- [ ] **Variable Naming** - Consistent naming conventions throughout
- [ ] **Global Variable Naming** - **MANDATORY**: All global variables must start with `g_<module>_<variable_name>`
- [ ] **Global Variable Placement** - **MANDATORY**: All global variables must be placed at the top of the file after includes and before function definitions
- [ ] **Return Code Constants** - **MANDATORY**: NEVER use magic numbers (`return 0`, `return -1`), ALWAYS use predefined constants (`ONVIF_SUCCESS`, `HTTP_AUTH_ERROR_NULL`, etc.)
- [ ] **Doxygen Documentation** - Complete function documentation with @brief, @param, @return, @note
- [ ] **File Headers** - Consistent Doxygen file headers with @file, @brief, @author, @date tags
- [ ] **Utility Usage** - Check for code duplication, proper use of existing utilities in `src/utils/`
- [ ] **Memory Management** - Proper malloc/free pairs, no memory leaks
- [ ] **Error Handling** - Comprehensive error checking and propagation
- [ ] **Code Duplication Prevention** - No repeated code patterns, use shared utilities
- [ ] **Source File Structure** - Follow established directory organization and naming conventions
- [ ] **Linter Compliance** - Address all linter warnings and errors from automated analysis

#### **2. Security Analysis**
- [ ] **Input Validation** - All user inputs properly validated and sanitized
- [ ] **Buffer Management** - No buffer overflows, proper bounds checking
- [ ] **Null Pointer Safety** - All pointer dereferences checked for NULL
- [ ] **Format String Security** - Safe printf/scanf usage with proper format specifiers
- [ ] **Authentication** - Current auth implementation and security gaps
- [ ] **Network Security** - SOAP/HTTP/RTSP security considerations
- [ ] **Resource Exhaustion** - Protection against DoS attacks

#### **3. Performance & Efficiency**
- [ ] **Memory Usage** - Efficient allocation patterns, no memory leaks
- [ ] **Algorithm Complexity** - O(n) vs O(n²) considerations
- [ ] **Thread Safety** - Proper mutex usage, no race conditions
- [ ] **I/O Operations** - Efficient file/network operations
- [ ] **Resource Management** - Proper cleanup and resource handling

#### **4. Architecture & Design**
- [ ] **Platform Abstraction** - Clean separation between platform-specific and generic code
- [ ] **Service Modularity** - Well-defined service boundaries and interfaces
- [ ] **Configuration Management** - Proper INI file handling and runtime config
- [ ] **Error Handling Strategy** - Consistent error reporting and recovery
- [ ] **Threading Model** - Appropriate use of threads and synchronization

#### **5. ONVIF Compliance**
- [ ] **Service Implementation** - Device, Media, PTZ, Imaging services completeness
- [ ] **SOAP Protocol** - Proper XML/SOAP request/response handling
- [ ] **WS-Discovery** - Correct multicast discovery implementation
- [ ] **RTSP Streaming** - Proper H.264 stream generation and delivery
- [ ] **Error Codes** - ONVIF-compliant error reporting

#### **6. Utilities Usage & Code Duplication Prevention**
- [ ] **Utility Function Usage** - Check if existing utilities in `src/utils/` are properly utilized
- [ ] **Code Duplication Detection** - Identify repeated code patterns that should be refactored
- [ ] **Shared Functionality** - Common operations should use centralized utility functions
- [ ] **DRY Principle** - Don't Repeat Yourself - eliminate duplicate implementations
- [ ] **Utility Documentation** - Ensure utility functions are properly documented
- [ ] **Utility Testing** - Verify utility functions work correctly across different contexts

#### **7. Source File Structure & Organization**
- [ ] **Directory Structure** - Files placed in correct directories according to functionality
- [ ] **Naming Conventions** - Consistent file and function naming patterns
- [ ] **Header Organization** - Proper separation of public/private interfaces
- [ ] **Module Boundaries** - Clear separation between different functional areas
- [ ] **Dependency Management** - Proper include dependencies and circular dependency prevention
- [ ] **File Size** - Reasonable file sizes with clear single responsibility

#### **8. Unit Testing & Test Naming Convention (MANDATORY)**
- [ ] **Unit Test Coverage** - ALL utility functions must have corresponding unit tests using CMocka framework
- [ ] **Unit Test Naming** - **MANDATORY**: Follow `test_unit_<module>_<functionality>` pattern (e.g., `test_unit_memory_manager_init`)
- [ ] **Integration Test Naming** - **MANDATORY**: Follow `test_integration_<service>_<functionality>` pattern (e.g., `test_integration_ptz_absolute_move`)
- [ ] **Test Cases** - Tests must cover success cases, error cases, and edge conditions
- [ ] **Coverage Reports** - Run `make test-coverage-html` to generate and verify coverage
- [ ] **Test Execution** - All tests must pass on development machine (native compilation)
- [ ] **CMocka Usage** - Proper use of CMocka assertions and mocking capabilities

#### **9. Build System & Code Quality Tools Integration**
- [ ] **compile_commands.json** - Verify exists and is current for clangd support
- [ ] **Linting Script** - Run `./cross-compile/onvif/scripts/lint_code.sh --check` and address all issues
- [ ] **Formatting Script** - Run `./cross-compile/onvif/scripts/format_code.sh --check` and fix formatting
- [ ] **Function Ordering** - Verify definitions at top, execution logic at bottom
- [ ] **Build Success** - Code must compile without errors using `make` command
- [ ] **Static Analysis** - Run `make static-analysis` and address critical issues
- [ ] **Documentation Build** - Run `make docs` and verify Doxygen output

#### **Include Path Format Guidelines (MANDATORY)**
- **MANDATORY**: All project includes must use relative paths from `src/` directory
- **CORRECT format**: `#include "services/common/video_config_types.h"`
- **INCORRECT format**: `#include "../../services/common/video_config_types.h"`
- **Rationale**:
  - Consistent and maintainable include paths
  - IDE-friendly (better autocomplete and navigation)
  - Reduces refactoring complexity when moving files
  - Prevents include path confusion
- **Enforcement**: Makefile and clangd configuration enforce this rule
- **Examples**:
  ```c
  // ✅ CORRECT - Relative from src/
  #include "services/common/onvif_types.h"
  #include "utils/validation/common_validation.h"
  #include "platform/platform.h"
  #include "networking/http/http_server.h"

  // ❌ INCORRECT - Relative paths with ../
  #include "../../services/common/onvif_types.h"
  #include "../validation/common_validation.h"
  #include "../../platform/platform.h"
  ```

#### **Global Variable Naming Guidelines (MANDATORY)**
- **MANDATORY**: All global variables must start with `g_<module>_<variable_name>`
- **Module prefix** should match the source file or functional area (e.g., `g_onvif_`, `g_platform_`, `g_media_`)
- **Variable name** should be descriptive and follow snake_case convention
- **Rationale**:
  - Prevents naming conflicts between modules
  - Improves code readability and maintainability
  - Makes module boundaries clear and explicit
  - Facilitates debugging and code navigation
- **Enforcement**: Code review process and linter configuration enforce this rule
- **Examples**:
  ```c
  // ✅ CORRECT - Global variables with module prefix
  static int g_onvif_device_count = 0;
  static char g_platform_device_name[64] = {0};
  static struct media_profile* g_media_profiles[MAX_PROFILES] = {NULL};
  static pthread_mutex_t g_ptz_mutex = PTHREAD_MUTEX_INITIALIZER;

  // ❌ INCORRECT - Global variables without module prefix
  static int device_count = 0;
  static char device_name[64] = {0};
  static struct media_profile* profiles[MAX_PROFILES] = {NULL};
  static pthread_mutex_t ptz_mutex = PTHREAD_MUTEX_INITIALIZER;
  ```

#### **Return Code Constants Guidelines (MANDATORY)**
- **MANDATORY**: NEVER use magic numbers for return codes
- **Always use predefined constants** for all function return values
- **Module-specific constants** defined in module headers (e.g., `HTTP_AUTH_SUCCESS`, `HTTP_AUTH_ERROR_NULL`)
- **Global constants** defined in `utils/error/error_handling.h` (e.g., `ONVIF_SUCCESS`, `ONVIF_ERROR_INVALID`)
- **Rationale**:
  - Improves code readability and maintainability
  - Makes error handling explicit and self-documenting
  - Prevents confusion between different return value meanings
  - Enables consistent error handling across modules
- **Enforcement**: Code review process must verify constant usage instead of magic numbers
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
  return 1;
  ```

#### **Global Variable Placement Guidelines (MANDATORY)**
- **MANDATORY**: All global variables must be placed at the top of the file after includes and before any function definitions
- **Grouping**: Group related global variables together with blank lines between groups
- **Initialization**: Initialize global variables at declaration when possible
- **Documentation**: Add comments explaining the purpose of each global variable
- **Rationale**:
  - Improves code readability and maintainability
  - Makes global state visible at a glance
  - Follows C best practices and coding standards
  - Facilitates code review and debugging
- **Enforcement**: Code review process and linter configuration enforce this rule
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

#### **Utilities Usage Guidelines**
- **MANDATORY**: Check `src/utils/` directory before implementing new functionality
- **Code Duplication**: Any repeated code pattern MUST be refactored into utilities
- **Common Patterns**: String manipulation, memory management, validation, logging
- **Utility Naming**: Use consistent prefixes (e.g., `onvif_util_*`, `platform_util_*`)
- **Documentation**: All utility functions must have complete Doxygen documentation

#### **File Naming Conventions**
- **Source files**: `onvif_<service>.c` (e.g., `onvif_device.c`)
- **Header files**: `onvif_<service>.h` (e.g., `onvif_device.h`)
- **Utility files**: `<category>_utils.c` (e.g., `memory_utils.c`)
- **Platform files**: `platform_<platform>.c` (e.g., `platform_anyka.c`)

#### **Test Naming Convention Standards (MANDATORY)**
- **Unit Test Naming** - **MANDATORY**: All unit tests must follow `test_unit_<module>_<functionality>` pattern
  - **Examples**:
    - `test_unit_memory_manager_init` - Unit test for memory manager initialization
    - `test_unit_http_auth_verify_credentials_success` - Unit test for successful credential verification
    - `test_unit_validation_token_format` - Unit test for token format validation
- **Integration Test Naming** - **MANDATORY**: All integration tests must follow `test_integration_<service>_<functionality>` pattern
  - **Examples**:
    - `test_integration_ptz_absolute_move_functionality` - Integration test for PTZ absolute movement
    - `test_integration_media_profile_operations` - Integration test for media profile operations
    - `test_integration_device_capabilities_retrieval` - Integration test for device capabilities
- **Rationale**:
  - Test runner uses naming convention to filter and categorize tests
  - Enables selective test execution (unit vs integration)
  - Improves test organization and maintainability
  - Makes test purpose clear and explicit
- **Enforcement**: Test runner validates naming patterns and categorizes accordingly

#### **NOLINT Suppression Guidelines**
- **Use //NOLINT sparingly** - Only when linter suggestions conflict with legitimate design requirements
- **Common Valid Uses**:
  - Global variables that must be mutable for state management: `static int g_discovery_running = 0; //NOLINT`
  - Function parameters where order is semantically correct but linter suggests reordering
  - Intentional design patterns that trigger false positives
- **Documentation Required**: ALWAYS add comment explaining why NOLINT is used
- **Review Required**: All NOLINT usage must be justified during code review
- **Examples**:
  ```c
  // ✅ CORRECT - NOLINT with justification
  static int g_discovery_running = 0; //NOLINT - Mutable state required for discovery service

  int onvif_ptz_stop(const char *profile_token, int stop_pan_tilt, int stop_zoom) //NOLINT - Parameter order matches ONVIF spec

  // ❌ INCORRECT - NOLINT without justification
  static int g_device_count = 0; //NOLINT
  ```

#### **File Header Standards (MANDATORY)**
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

#### **Protocol Layer Review Guidelines (gSOAP/ONVIF)**
- [ ] **gSOAP API Usage** - Verify proper use of generated gSOAP functions instead of manual XML parsing
- [ ] **Type-Safe Deserialization** - Use `soap_read_*` functions for complete SOAP envelope processing
- [ ] **Request Structure Access** - Access structured request data instead of extracting individual tokens
- [ ] **Service Modularity** - Separate gSOAP implementations by service (Media, PTZ, Device, Imaging)
- [ ] **SOAP Context Initialization** - Proper configuration of `soap->is`, `soap->buflen`, `soap->bufidx`
- [ ] **Memory Management** - Proper cleanup of gSOAP-allocated structures using `soap_end()` and `soap_done()`
- [ ] **Error Handling** - Use ONVIF-compliant SOAP faults and error codes
- [ ] **XML Validation** - Let gSOAP handle XML validation instead of manual string parsing
- **Common Anti-Patterns to Avoid**:
  ```c
  // ❌ BAD - Manual XML token extraction
  onvif_gsoap_parse_profile_token(soap, &profile_token);

  // ✅ GOOD - Use gSOAP-generated deserialization
  struct _trt__GetStreamUri *request = soap_new__trt__GetStreamUri(soap, -1);
  if (soap_read__trt__GetStreamUri(soap, request) == SOAP_OK) {
      // Access structured data: request->ProfileToken
  }
  ```
- **Rationale**:
  - Eliminates ~800 lines of manual XML parsing code
  - Uses validated gSOAP-generated functions for ONVIF compliance
  - Provides type-safe access to request/response data
  - Reduces security vulnerabilities from manual string manipulation

### ��️ **Review Methodology**

#### **1. Linter Analysis Review**
- **Run linter analysis** using `read_lints` tool on the ONVIF project directory
- **Review linter output** for all warnings, errors, and style violations
- **Categorize issues** by severity (Critical/High/Medium/Low) and type
- **Address critical and high-severity issues** immediately
- **Document medium and low-severity issues** for future improvement
- **Verify linter configuration** is appropriate for the project standards
- **Check for false positives** and adjust linter rules if necessary

#### **2. Static Analysis Review**
- Review the existing static analysis report (`analysis-results/static-analysis-report.md`)
- Verify all critical issues are properly addressed
- Check for any new issues not caught by automated tools
- **Cross-reference with linter output** to ensure comprehensive coverage

#### **3. Code Review Process**
- **File-by-file review** of critical components
- **Cross-reference** with ONVIF 2.5 specification
- **Test compilation** using Docker: `docker run --rm -v ${PWD}:/workspace anyka-cross-compile make -C /workspace/onvif`
- **Documentation verification** - Ensure Doxygen docs are up-to-date

#### **4. Security Assessment**
- **Input validation** review across all service endpoints
- **Authentication** and authorization mechanisms
- **Network security** for SOAP/HTTP/RTSP protocols
- **Memory safety** and buffer management

#### **5. Performance Analysis**
- **Memory usage** patterns and potential leaks
- **Threading** and concurrency issues
- **I/O efficiency** for streaming and file operations
- **Algorithm optimization** opportunities

### 📊 **Review Output Format**

#### **Overall Review Summary**
```markdown
## ONVIF Project Code Review Summary

### Build & Test Validation
- **Build Status**: [✅ Success / ❌ Failed]
- **Unit Tests**: [✅ All Passing / ❌ X failed / ⚠️ Not run]
- **Test Coverage**: [XX% line coverage, XX% function coverage]
- **Linting Status**: [✅ Clean / ❌ X issues / ⚠️ Not run]
- **Formatting Status**: [✅ Compliant / ❌ Issues found / ⚠️ Not run]
- **Static Analysis**: [✅ No critical issues / ❌ X critical, X high]

### Code Quality Metrics
- **Total Files Reviewed**: [count]
- **Return Code Constant Compliance**: [✅ 100% / ❌ XX violations]
- **Global Variable Naming**: [✅ Compliant / ❌ XX violations]
- **Include Path Format**: [✅ Compliant / ❌ XX violations]
- **Test Naming Convention**: [✅ Compliant / ❌ XX violations]
- **Code Duplication**: [✅ None detected / ⚠️ XX instances]
- **Doxygen Documentation**: [✅ Complete / ⚠️ XX functions missing]

### Security & Compliance
- **ONVIF 2.5 Compliance**: [✅ Compliant / ⚠️ Issues found]
- **Security Vulnerabilities**: [✅ None / ⚠️ XX low / ❌ XX high]
- **Memory Safety**: [✅ No leaks detected / ⚠️ Potential issues]
- **Input Validation**: [✅ Comprehensive / ⚠️ Gaps identified]
```

#### **Individual Issue Format**

For each issue found, provide:

```markdown
## 🔍 **Issue Category**: [Code Quality/Security/Performance/Architecture/ONVIF Compliance/Testing]

### **Issue**: [Brief description]
- **File**: `path/to/file.c`
- **Lines**: [line numbers]
- **Severity**: [Critical/High/Medium/Low]
- **CWE**: [if applicable]
- **Rule**: [Linter rule or coding standard violated]
- **Description**: [Detailed explanation]
- **Current Code**: [code snippet]
- **Recommendation**: [specific fix suggestion]
- **Example Fix**: [code example if applicable]
- **Impact**: [Performance/Security/Maintainability impact]
```

### 🔧 **Linter Output Integration**

#### **Linter Issue Categories**
- **Error** - Critical issues that must be fixed immediately (compilation failures, undefined behavior)
- **Warning** - Important issues that should be addressed (potential bugs, security concerns)
- **Info** - Code quality improvements and best practice suggestions
- **Hint** - Minor style suggestions and optimization opportunities

#### **Linter Severity Levels (clangd-tidy)**
- **--severity error** - Fail only on errors (critical issues)
- **--severity warn** - Fail on warnings and errors (recommended for code review)
- **--severity info** - Fail on info, warnings, and errors
- **--severity hint** - Fail on all issues including hints (strictest mode)

#### **Linter Output Format**
For each linter issue, provide:

```markdown
## 🔧 **Linter Issue**: [Issue Type]

### **Issue**: [Brief description from linter]
- **File**: `path/to/file.c`
- **Lines**: [line numbers]
- **Severity**: [Error/Warning/Info]
- **Linter Rule**: [specific linter rule violated]
- **Description**: [Detailed explanation of the issue]
- **Current Code**: [code snippet from linter output]
- **Recommendation**: [specific fix suggestion]
- **Example Fix**: [code example showing the fix]
```

#### **Linter Analysis Commands**
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

#### **Code Formatting Commands**
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

#### **Unit Testing Commands (MANDATORY)**
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

### 🎯 **Success Criteria**

A successful review should:
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

### 🚀 **Additional Considerations**

- **Bash Commands** - All terminal operations must use bash syntax (WSL Ubuntu environment)
- **Native Build Process** - Verify native cross-compilation builds work correctly (`make` command)
- **Documentation** - Ensure Doxygen documentation is complete and up-to-date (`make docs`)
- **Utility Functions** - Check for proper use of existing utilities to avoid code duplication
- **Memory Management** - Verify proper resource cleanup and leak prevention
- **Thread Safety** - Ensure all concurrent operations are properly synchronized
- **Linter Integration** - Run linter analysis as part of the review process and address all issues
- **Code Quality** - Ensure linter compliance maintains high code quality standards
- **Unit Testing** - ALL utility functions must have corresponding CMocka tests
- **SD Card Testing** - Validate SD card payload deployment for safe device testing
- **Return Code Constants** - NEVER use magic numbers, ALWAYS use predefined constants

### 📦 **SD Card Testing Workflow Validation**

When reviewing code changes, verify the SD card testing workflow can be followed:

```bash
# 1. Build the ONVIF binary
cd cross-compile/onvif && make

# 2. Verify binary exists
test -f out/onvifd && echo "Build successful"

# 3. Copy to SD card payload
cp out/onvifd ../../SD_card_contents/anyka_hack/usr/bin/

# 4. Copy configuration if changed
cp configs/anyka_cfg.ini ../../SD_card_contents/anyka_hack/onvif/

# 5. Deploy to SD card and test on actual device
# (Safe testing - leaves original firmware intact)
```

**Validation Checklist**:
- [ ] Binary builds successfully without errors
- [ ] File size is reasonable for embedded system
- [ ] Dependencies are properly linked
- [ ] Configuration files are up-to-date
- [ ] SD card payload structure is maintained

### 🔧 **Code Duplication Detection & Utility Usage**

#### **Common Duplication Patterns to Look For**
- **String Operations**: Repeated string validation, manipulation, or formatting
- **Memory Management**: Repeated malloc/free patterns or error handling
- **Input Validation**: Similar validation logic across different services
- **Error Handling**: Repeated error checking and reporting patterns
- **Configuration Access**: Repeated config file parsing or value retrieval
- **Logging**: Repeated logging patterns with similar formatting

#### **Utility Usage Examples**

**❌ BAD - Code Duplication:**
```c
// In onvif_device.c
if (!token || strlen(token) == 0 || strlen(token) > 32) {
    return ONVIF_ERROR_INVALID_PARAMETER;
}

// In onvif_media.c
if (!profile_token || strlen(profile_token) == 0 || strlen(profile_token) > 32) {
    return ONVIF_ERROR_INVALID_PARAMETER;
}
```

**✅ GOOD - Using Utility:**
```c
// In onvif_device.c
if (!onvif_util_validate_token(token)) {
    return ONVIF_ERROR_INVALID_PARAMETER;
}

// In onvif_media.c
if (!onvif_util_validate_token(profile_token)) {
    return ONVIF_ERROR_INVALID_PARAMETER;
}
```

#### **Utility Function Requirements**
- **Location**: Must be in appropriate `src/utils/<category>/` directory
- **Naming**: Use consistent prefix (e.g., `onvif_util_*`, `platform_util_*`)
- **Documentation**: Complete Doxygen documentation with examples
- **Error Handling**: Proper error codes and validation
- **Thread Safety**: Document thread safety requirements
- **Testing**: Verify utility works across different contexts

### 📋 **Linter Integration Workflow**

#### **Step 1: Run Linter Analysis**
```bash
# Run comprehensive linter analysis
read_lints cross-compile/onvif/

# Focus on specific areas if needed
read_lints cross-compile/onvif/src/services/
read_lints cross-compile/onvif/src/platform/
```

#### **Step 2: Categorize and Prioritize Issues**
1. **Critical Issues** - Syntax errors, compilation failures
2. **High Priority** - Security warnings, memory leaks, undefined behavior
3. **Medium Priority** - Style violations, code quality issues
4. **Low Priority** - Info messages, suggestions for improvement

#### **Step 3: Document and Track Issues**
- Create issue tracking for each linter finding
- Cross-reference with existing static analysis
- Prioritize fixes based on severity and impact
- Document false positives and rule adjustments

#### **Step 4: Implement Fixes**
- Address critical and high-priority issues immediately
- Batch medium-priority fixes for efficiency
- Document low-priority issues for future improvement
- Verify fixes don't introduce new issues

#### **Step 5: Validate and Re-test**
- Re-run linter analysis after fixes
- Verify all critical issues are resolved
- Check for any new issues introduced
- Update documentation and review process

---

**Remember**: This is a production-ready ONVIF implementation for embedded camera systems. Focus on security, reliability, and ONVIF compliance while maintaining the high code quality standards established in the project. Linter analysis is a critical component of ensuring code quality and should be integrated into every review process.
