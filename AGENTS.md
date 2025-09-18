# Agent Documentation for Anyka AK3918 Hacking Journey

## Project Overview
This repository documents reverse-engineering and hacks for Anyka AK3918-based cameras. The codebase is a mix of cross-compile app sources, SD-card payloads, and notes for modifying device rootfs (squashfs `*.sqsh4`).

## Repository Structure & Key Components

### Primary Development Areas
- **`cross-compile/onvif/`** — **CURRENT FOCUS** - Complete ONVIF 2.5 implementation with Device, Media, PTZ, and Imaging services
- **`cross-compile/`** — Source and build scripts for individual apps (e.g., `libre_anyka_app`, `aenc_demo`, `ak_snapshot`, `ptz_daemon`). Each subproject usually has a `build.sh` or `Makefile`.
- **`SD_card_contents/anyka_hack/`** — Payload and web UI that run from SD card. This is the easiest test path and typical dev workflow for testing changes on device.
- **`newroot/`** and prepared squashfs images (`busyroot.sqsh4`, `busyusr.sqsh4`, `newroot.sqsh4`) — Prebuilt root/user fs images used for flashing.
- **`hack_process/`**, **`README.md`**, **`Images/`**, and **`UART_logs/`** — Docs and logs useful for debugging and reproducing device behavior.

### Reference Implementation
- **`akipc/`** — Reference implementation (chip/vendor-provided) that shows how the original camera firmware implements APIs, initialization, and configuration.
- Use as canonical example when:
  - Reverse-engineering how the camera starts services and uses device files
  - Matching IPC/CLI commands and config keys used by webUI and other apps
  - Verifying binary and library ABI expectations before replacing or reimplementing a system binary

### Component Libraries
- **`component/`** — Reusable pieces extracted from stock firmware: drivers, third-party libs, and helper tools.
- **`platform/`** — Board/platform-specific glue: sensor selection and initialization, board pin mappings, and other low-level integration.

## Development Workflow

### Quick Start Entry Points
- **Fast iteration**: Edit or add files under `cross-compile/<app>/` and test via the SD-card hack in `SD_card_contents/anyka_hack/`.
- **Key documentation**:
  - Top-level `README.md` — project goals, features, SD-card hack quick start and important device commands.
  - `cross-compile/README.md` — cross-compile environment notes and `setup.sh` usage.

### Build Process
- **Always use Docker for compilation**: `docker run --rm -v ${PWD}:/workspace anyka-cross-compile make -C /workspace/<project>`
- **Test compilation** before committing changes
- **SD-card testing** is the primary development workflow for device testing

## Development Environment

### Windows Development
- **Primary OS**: Development is conducted on Windows 10/11
- **Shell**: **MANDATORY** - All terminal commands MUST use PowerShell syntax
- **Docker**: Use Docker Desktop for Windows for containerized builds
- **Path handling**: Use Windows-style paths in PowerShell commands, but Docker volumes use Unix-style paths

### PowerShell Usage (MANDATORY)
- **ALL terminal commands MUST use PowerShell syntax** - No bash/sh commands allowed
- **File operations**: Use PowerShell cmdlets like `Get-ChildItem`, `Copy-Item`, `Remove-Item`, `Test-Path`
- **Environment variables**: Use `$env:VARIABLE_NAME` syntax
- **Path separators**: Use backslashes `\` for Windows paths, forward slashes `/` for Docker/Unix paths
- **Docker commands**: Use PowerShell-compatible syntax: `docker run --rm -v ${PWD}:/workspace anyka-cross-compile make -C /workspace/<project>`

### PowerShell Command Examples
- **Docker builds**: `docker run --rm -v ${PWD}:/workspace anyka-cross-compile make -C /workspace/<project>`
- **File operations**: 
  ```powershell
  Get-ChildItem -Path "cross-compile\onvif" -Recurse
  Copy-Item "source\file.c" "destination\file.c"
  Test-Path "cross-compile\onvif\out\onvifd"
  ```
- **Environment variables**: `$env:BUILD_TYPE = "release"`
- **Path separators**: Use backslashes `\` for Windows paths, forward slashes `/` for Docker/Unix paths

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

### Code Organization
- **Consistent 2-space indentation**
- **Consistent variable naming**
- **Proper spacing and formatting**
- **Functions ordered with definitions at the top of files and execution logic at the bottom**

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

## Code Review Guidelines

### Code Review Process (MANDATORY)
When reviewing ONVIF project code, **ALWAYS** follow this comprehensive review process:

#### 1. Code Quality and Adherence to Best Practices
- **Coding Standards Compliance**:
  - Verify include ordering (system → third-party → project headers)
  - Check 2-space indentation consistency
  - Validate function ordering (definitions at top, execution logic at bottom)
  - Ensure consistent variable naming conventions
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

- **Memory Usage**:
  - Verify efficient memory allocation patterns
  - Check for memory fragmentation issues
  - Validate stack vs heap usage decisions
  - Look for unnecessary memory copies

- **I/O Operations**:
  - Check for blocking operations that could be optimized
  - Verify efficient data structure usage
  - Look for unnecessary system calls
  - Check for proper buffering strategies

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

### Code Review Checklist
**Before approving any code changes, verify:**

- [ ] **PowerShell syntax** used in all commands
- [ ] **Docker builds** work correctly
- [ ] **Include ordering** follows standards
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
1. **ALWAYS use PowerShell** for all terminal commands and file operations
2. **ALWAYS use Docker** for development tasks and building
3. **ALWAYS test compilation** using the Docker command before suggesting changes
4. **ALWAYS follow the include ordering** standards strictly
5. **MANDATORY: Update Doxygen documentation** for ALL code changes
6. **MANDATORY: Regenerate HTML documentation** after any code changes
7. **MANDATORY: Perform comprehensive code review** before approving changes
8. **MANDATORY: Use existing utilities** and avoid code duplication
9. **Reference the akipc tree** when reverse-engineering or implementing new features
10. **Use SD-card testing** as the primary development workflow

### Documentation Workflow (MANDATORY)
1. Make code changes
2. Update Doxygen comments for all modified functions
3. Update file headers if needed
4. Run documentation generation: `docker run --rm -v ${PWD}:/workspace anyka-cross-compile make -C /workspace/onvif docs`
5. Verify documentation in `cross-compile\onvif\docs\html\index.html`
6. Test compilation to ensure changes work
7. **Perform comprehensive code review**
8. Commit changes

### Common Tasks & Examples
- **ONVIF Development**: "I modified `cross-compile/onvif/src/services/ptz/onvif_ptz.c` to improve preset handling — please build using Docker, perform code review, and test PTZ functionality via ONVIF client."
- **Platform Updates**: "Update `cross-compile/onvif/src/platform/platform_anyka.c` to add better error handling for PTZ initialization failures — ensure proper code review for security and performance."
- **App Development**: "I changed `cross-compile/libre_anyka_app/main.c` to add logging — please build using Docker, review for memory management issues, and test by copying the new binary to `SD_card_contents/anyka_hack/usr/bin/` and booting an SD-card image."
- **Web UI Updates**: "Patch updates `www/js/app.js` for the web UI. After applying, pack the SD payload and test UI on device at `http://<device-ip>`; capture network requests and serial log if UI doesn't load."
- **Documentation Updates**: "I added new functions to `cross-compile/onvif/src/services/device/onvif_device.c` — please update the Doxygen documentation, regenerate the HTML docs, and perform security review."
- **Code Review**: "Please review the ONVIF project code considering code quality, potential bugs, performance optimizations, readability, and security concerns. Suggest improvements and explain reasoning for each suggestion."
- **Utility Refactoring**: "I found duplicated string validation code in multiple files — please refactor to use the existing `onvif_util_validate_token()` utility function and remove all duplicated implementations."

### PowerShell Command Examples
```powershell
# Build the project
docker run --rm -v ${PWD}:/workspace anyka-cross-compile make -C /workspace/onvif

# Generate documentation
docker run --rm -v ${PWD}:/workspace anyka-cross-compile make -C /workspace/onvif docs

# File operations
Get-ChildItem -Path "cross-compile\onvif\src" -Filter "*.c"
Copy-Item "cross-compile\onvif\out\onvifd" "SD_card_contents\anyka_hack\usr\bin\"

# Check if file exists
Test-Path "cross-compile\onvif\out\onvifd"

# View documentation
Start-Process "cross-compile\onvif\docs\html\index.html"

# Search for code duplication
Select-String -Path "cross-compile\onvif\src\**\*.c" -Pattern "strlen.*token.*32" -Context 2
```

### Memory & Context
- This project focuses on reverse-engineering Anyka AK3918 camera firmware
- Docker is the preferred development environment
- SD-card payload testing is the primary iteration method
- ONVIF 2.5 implementation is the current focus area
- Platform abstraction and logging integration are key architectural concerns
- **PowerShell is mandatory** for all terminal operations
- **Documentation updates are mandatory** for all code changes
- **Code review is mandatory** for all code changes
- **Utility usage is mandatory** - no code duplication allowed