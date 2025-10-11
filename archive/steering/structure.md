# Project Structure

## Directory Organization

```
anyka-dev/
├── cross-compile/                    # Cross-compilation environment and source code
│   ├── onvif/                       # ONVIF 2.5 implementation (CURRENT FOCUS)
│   │   ├── src/                     # Source code organized by functionality
│   │   │   ├── core/                # Core system components
│   │   │   │   ├── config/          # Configuration management
│   │   │   │   ├── lifecycle/       # Service lifecycle management
│   │   │   │   └── main/            # Main daemon entry point
│   │   │   ├── platform/            # Platform abstraction layer
│   │   │   │   ├── adapters/        # Hardware-specific adapters
│   │   │   │   ├── platform_anyka.c # Anyka AK3918 implementation
│   │   │   │   └── platform.h       # Platform interface
│   │   │   ├── services/            # ONVIF service implementations
│   │   │   │   ├── device/          # Device service
│   │   │   │   ├── media/           # Media service
│   │   │   │   ├── ptz/             # PTZ service
│   │   │   │   ├── imaging/         # Imaging service
│   │   │   │   ├── snapshot/        # Snapshot service
│   │   │   │   └── common/          # Shared service types
│   │   │   ├── networking/          # Network protocol implementations
│   │   │   │   ├── http/            # HTTP/SOAP handling
│   │   │   │   ├── rtsp/            # RTSP streaming
│   │   │   │   ├── discovery/       # WS-Discovery
│   │   │   │   └── common/          # Shared networking utilities
│   │   │   ├── protocol/            # Protocol handling
│   │   │   │   ├── soap/            # SOAP processing
│   │   │   │   ├── xml/             # XML utilities
│   │   │   │   └── response/        # Response handling
│   │   │   └── utils/               # Utility functions (CRITICAL)
│   │   │       ├── memory/          # Memory management utilities
│   │   │       ├── string/          # String manipulation utilities
│   │   │       ├── error/           # Error handling utilities
│   │   │       ├── network/         # Network utility functions
│   │   │       ├── logging/         # Logging utilities
│   │   │       ├── validation/      # Input validation utilities
│   │   │       ├── security/        # Security utilities
│   │   │       ├── service/         # Service utilities
│   │   │       └── stream/          # Stream configuration utilities
│   │   ├── tests/                   # Unit testing framework (MANDATORY)
│   │   │   ├── src/                 # Test source files
│   │   │   │   ├── unit/            # Unit tests for utilities and modules
│   │   │   │   ├── integration/     # Integration tests for services
│   │   │   │   └── runner/          # Test runner implementation
│   │   │   ├── Makefile             # Test build configuration
│   │   │   └── coverage/            # Coverage reports
│   │   ├── out/                     # Build output directory
│   │   ├── docs/                    # Generated documentation
│   │   ├── scripts/                 # Build and utility scripts
│   │   │   ├── lint_code.sh         # Code linting script (MANDATORY)
│   │   │   └── format_code.sh       # Code formatting script (MANDATORY)
│   │   ├── Makefile                 # Build configuration
│   │   └── Doxyfile                 # Documentation configuration
│   ├── anyka_reference/             # Reference implementation
│   │   ├── akipc/                   # Complete vendor implementation
│   │   ├── component/               # Reusable components and libraries
│   │   └── platform/                # Platform-specific code
│   ├── libre_anyka_app/             # Custom camera application
│   ├── aenc_demo/                   # Audio encoding demo
│   ├── ak_snapshot/                 # Snapshot functionality
│   ├── ptz_daemon/                  # PTZ control daemon
│   └── gsoap/                       # gSOAP library and tools
├── SD_card_contents/                # SD card payload system
│   └── anyka_hack/                  # Web UI and runtime testing
│       ├── usr/bin/                 # Executable binaries
│       ├── www/                     # Web interface
│       └── scripts/                 # Utility scripts
├── newroot/                         # Root filesystem images
│   ├── busyroot.sqsh4               # BusyBox root filesystem
│   ├── busyusr.sqsh4                # BusyBox user filesystem
│   └── newroot.sqsh4                # Custom root filesystem
├── docs/                            # Project documentation
│   ├── agents/                      # Agent documentation
│   ├── refactoring/                 # Refactoring guides
│   └── test_plan/                   # Testing documentation
├── e2e/                             # E2E test suite (Python)
├── cross-compile/onvif/tests/       # Unit testing framework (CMocka)
├── scripts/                         # Build and utility scripts
├── toolchain/                       # Cross-compilation toolchain
├── .spec-workflow/                  # Spec-workflow MCP configuration
│   ├── steering/                    # Steering documents
│   ├── specs/                       # Specification documents
│   └── templates/                   # Document templates
└── README.md                        # Project overview
```

## Naming Conventions

### Files

- **Source Files**: `onvif_<service>.c` (e.g., `onvif_device.c`, `onvif_media.c`)
- **Header Files**: `onvif_<service>.h` (e.g., `onvif_device.h`, `onvif_media.h`)
- **Utility Files**: `<category>_utils.c` (e.g., `memory_utils.c`, `string_utils.c`)
- **Platform Files**: `platform_<platform>.c` (e.g., `platform_anyka.c`)
- **Unit Tests**: `test_unit_<module>_<functionality>.c` (e.g., `test_unit_memory_utils.c`, `test_unit_http_auth_verify_credentials_success.c`)
- **Integration Tests**: `test_integration_<service>_<functionality>.c` (e.g., `test_integration_ptz_absolute_move_functionality.c`, `test_integration_media_profile_operations.c`)

### Code

- **Functions**: `onvif_<service>_<action>` (e.g., `onvif_device_get_info`, `onvif_media_get_profiles`)
- **Global Variables**: `g_<module>_<variable_name>` (e.g., `g_onvif_device_count`, `g_platform_device_name`)
  - **MANDATORY**: ALL global variables MUST use this format
  - **MANDATORY**: Must be placed at top of file after includes
- **Constants**:
  - Service constants: `ONVIF_<SERVICE>_<CONSTANT>` (e.g., `ONVIF_DEVICE_MAX_PROFILES`, `ONVIF_MEDIA_DEFAULT_RESOLUTION`)
  - Return codes: `<MODULE>_SUCCESS`, `<MODULE>_ERROR_<TYPE>` (e.g., `HTTP_AUTH_SUCCESS`, `HTTP_AUTH_ERROR_NULL`, `ONVIF_SUCCESS`, `ONVIF_ERROR_INVALID`)
  - **MANDATORY**: NEVER use magic numbers for return codes (e.g., `return -1`, `return 0`)
- **Types**: `onvif_<service>_<type>_t` (e.g., `onvif_device_info_t`, `onvif_media_profile_t`)
- **Macros**: `ONVIF_<SERVICE>_<MACRO>` (e.g., `ONVIF_DEVICE_INIT`, `ONVIF_MEDIA_CLEANUP`)
- **Test Functions**:
  - Unit tests: `test_unit_<module>_<functionality>` (e.g., `test_unit_memory_manager_init`, `test_unit_http_auth_verify_credentials_success`)
  - Integration tests: `test_integration_<service>_<functionality>` (e.g., `test_integration_ptz_absolute_move_functionality`)

## Import Patterns

### Include Order (MANDATORY)

1. **System headers first** (e.g., `#include <stdio.h>`, `#include <stdlib.h>`)
2. **Third-party library headers** (e.g., `#include <curl/curl.h>`, `#include <gsoap/soapH.h>`)
3. **Project-specific headers** (e.g., `#include "platform.h"`, `#include "common.h"`)

### Module/Package Organization

- **Absolute imports from project root**: Use relative paths from `src/` directory
- **Relative imports within modules**: Use `../` for parent directory access
- **Dependency management**: Clear separation between layers to prevent circular dependencies
- **Include guards**: Use `#ifndef HEADER_H` / `#define HEADER_H` / `#endif` or `#pragma once`

### Include Path Format (MANDATORY)

- **ALWAYS use relative paths from `src/` directory** for all project includes
- **CORRECT format**: `#include "services/common/video_config_types.h"`
- **INCORRECT format**: `#include "../../services/common/video_config_types.h"`

## Code Structure Patterns

### Module/Class Organization

```c
// 1. Include guards and file header
#ifndef ONVIF_DEVICE_H
#define ONVIF_DEVICE_H

/**
 * @file onvif_device.h
 * @brief ONVIF Device service implementation
 * @author kkrzysztofik
 * @date 2025
 */

// 2. System includes
#include <stdio.h>
#include <stdlib.h>

// 3. Third-party includes
#include <gsoap/soapH.h>

// 4. Project includes
#include "services/common/onvif_types.h"
#include "platform/platform.h"

// 5. Constants and configuration
#define ONVIF_DEVICE_MAX_PROFILES 16

// 6. Type definitions
typedef struct {
    char name[64];
    char model[32];
    char serial[32];
} onvif_device_info_t;

// 7. Global variables (if any)
extern int g_onvif_device_count;

// 8. Function declarations
int onvif_device_init(void);
int onvif_device_get_info(onvif_device_info_t *info);

#endif // ONVIF_DEVICE_H
```

### Function/Method Organization

```c
/**
 * @brief Initialize ONVIF device service
 * @return 0 on success, -1 on error
 * @note Must be called before using any device functions
 */
int onvif_device_init(void) {
    // 1. Input validation
    if (g_onvif_device_initialized) {
        return 0; // Already initialized
    }

    // 2. Resource allocation
    g_onvif_device_info = malloc(sizeof(onvif_device_info_t));
    if (!g_onvif_device_info) {
        return -1; // Memory allocation failed
    }

    // 3. Core logic
    memset(g_onvif_device_info, 0, sizeof(onvif_device_info_t));
    strncpy(g_onvif_device_info->name, "Anyka Camera",
            sizeof(g_onvif_device_info->name) - 1);

    // 4. Error handling
    if (platform_init() != 0) {
        free(g_onvif_device_info);
        g_onvif_device_info = NULL;
        return -1;
    }

    // 5. Success
    g_onvif_device_initialized = 1;
    return 0;
}
```

### File Organization Principles

- **One service per file**: Each ONVIF service has its own source and header files
- **Related functionality grouped**: Utilities are grouped by category (memory, string, etc.)
- **Public API at the top**: Function declarations in header files
- **Implementation details hidden**: Internal functions and variables are static

## Code Quality Validation (MANDATORY)

### Pre-Commit Validation

All code changes **MUST** pass the following validation steps before being committed:

1. **Code Linting** (MANDATORY):

   ```bash
   ./cross-compile/onvif/scripts/lint_code.sh --check
   ```

   - Checks for code quality issues and potential bugs
   - Must pass with zero errors before code can be approved
   - Options: `--file`, `--changed`, `--severity error`

2. **Code Formatting** (MANDATORY):

   ```bash
   ./cross-compile/onvif/scripts/format_code.sh --check
   ```

   - Verifies code follows project formatting standards
   - Must pass with zero issues before code can be approved
   - Use `./cross-compile/onvif/scripts/format_code.sh` to auto-format

3. **Function Ordering** (MANDATORY):

   - Verify definitions are at top of files
   - Execution logic at bottom
   - Global variables at top after includes

4. **Unit Testing** (MANDATORY):
   ```bash
   make test
   ```
   - ALL utility functions must have unit tests
   - Tests must pass before committing changes
   - Test naming: `test_unit_<module>_<functionality>` for unit tests
   - Test naming: `test_integration_<service>_<functionality>` for integration tests
   - Enforcement: Test runner uses naming convention to filter tests by category

## Code Organization Principles

1. **Single Responsibility**: Each file should have one clear purpose
2. **Modularity**: Code should be organized into reusable modules
3. **Testability**: Structure code to be easily testable
4. **Consistency**: Follow patterns established in the codebase
5. **Security**: All code must be secure by design
6. **Performance**: Optimize for embedded systems with limited resources
7. **Quality Validation**: All code must pass linting, formatting, and testing before approval

## Module Boundaries

### Core vs Services

- **Core**: Essential system components (config, lifecycle, main)
- **Services**: ONVIF service implementations (device, media, ptz, imaging)
- **Dependencies**: Services depend on core, core is independent

### Platform vs Application

- **Platform**: Hardware abstraction layer (platform_anyka.c)
- **Application**: ONVIF services and business logic
- **Dependencies**: Application depends on platform, platform is hardware-specific

### Utilities vs Services

- **Utilities**: Reusable functions (memory, string, validation)
  - **MANDATORY**: Always check `src/utils/` for existing functionality before implementing new code
  - **NO code duplication allowed** - reuse existing utilities
  - **ALL utility functions MUST have unit tests**
  - Located in `src/utils/` with categorized subdirectories:
    - `memory/` - Memory management utilities
    - `string/` - String manipulation utilities
    - `error/` - Error handling utilities (includes global return code constants)
    - `network/` - Network utility functions
    - `logging/` - Logging utilities
    - `validation/` - Input validation utilities
    - `security/` - Security utilities
    - `service/` - Service utilities
    - `stream/` - Stream configuration utilities
- **Services**: ONVIF service implementations
- **Dependencies**: Services depend on utilities, utilities are independent

### Public vs Internal

- **Public API**: Functions declared in header files
- **Internal**: Static functions and private implementation details
- **Dependencies**: Public API can depend on internal, internal cannot depend on public

### Stable vs Experimental

- **Stable**: Production-ready code with full testing
- **Experimental**: New features under development
- **Dependencies**: Stable code cannot depend on experimental

## Code Size Guidelines

### File Size

- **Maximum lines per file**: ~1000 lines
- **Recommended size**: 200-500 lines for maintainability
- **Large files**: Split into logical modules

### Function Size

- **Maximum lines per function**: ~50 lines
- **Recommended size**: 10-30 lines for readability
- **Large functions**: Split into smaller, focused functions

### Complexity Guidelines

- **Cyclomatic complexity**: Maximum 10 per function
- **Nesting depth**: Maximum 4 levels
- **Parameter count**: Maximum 5 parameters per function

## SD Card Development Structure

### SD Card Payload Organization

```
SD_card_contents/anyka_hack/
├── usr/bin/                 # Executable binaries
│   ├── onvifd              # Main ONVIF daemon
│   ├── libre_anyka_app     # Custom camera app
│   └── ptz_daemon          # PTZ control daemon
├── www/                     # Web interface
│   ├── index.html          # Main web page
│   ├── js/                 # JavaScript files
│   └── css/                # Stylesheets
├── scripts/                 # Utility scripts
│   ├── start.sh            # Startup script
│   └── stop.sh             # Shutdown script
└── config/                  # Configuration files
    ├── onvif.conf          # ONVIF configuration
    └── camera.conf         # Camera settings
```

### Separation of Concerns

- **SD card payload**: Isolated from core firmware
- **Web interface**: Independent of ONVIF services
- **Configuration**: Separate from executable code
- **Scripts**: Utility functions for development

## Documentation Standards

### Code Documentation

- **Doxygen comments**: All public functions must have complete documentation
- **File headers**: Every source file must have Doxygen file header
- **Inline comments**: Complex logic should include explanatory comments
- **API documentation**: Generated HTML documentation from source code

### Documentation Structure

```
docs/
├── agents/                  # Agent documentation
│   ├── spec-workflow-*.md  # Spec-workflow MCP documentation
│   └── review-prompt.md    # Code review guidelines
├── refactoring/            # Refactoring guides
├── test_plan/              # Testing documentation
└── README.md               # Project overview
```

### Documentation Requirements

- **Public APIs**: Must have complete Doxygen documentation
- **Complex logic**: Must include inline comments explaining the approach
- **Configuration**: Must document all configuration options
- **Build process**: Must document build and deployment procedures
- **Security**: Must document security considerations and best practices

## Build System Structure

### Makefile Organization

```
cross-compile/onvif/
├── Makefile                # Main build configuration
├── src/                    # Source code
├── out/                    # Build output
├── docs/                   # Generated documentation
└── Doxyfile                # Documentation configuration
```

### Build Targets

- **all**: Build all components
- **onvifd**: Build main ONVIF daemon
- **debug**: Build with debug symbols
- **release**: Build with optimizations
- **docs**: Generate Doxygen documentation (MANDATORY after code changes)
- **test**: Run all unit tests (MANDATORY before commits)
- **test-utils**: Run utility unit tests
- **test-coverage**: Run tests with coverage analysis
- **test-coverage-html**: Generate HTML coverage report
- **static-analysis**: Run all static analysis tools
- **clang-analyze**: Run Clang static analyzer
- **cppcheck-analyze**: Run Cppcheck analysis
- **snyk-analyze**: Run Snyk security analysis
- **clean**: Clean build artifacts
- **compile-commands**: Generate compile_commands.json for clangd

### Build Dependencies

- **Cross-compilation toolchain**: ARM GCC compiler
- **Libraries**: gSOAP, libcurl, libxml2, OpenSSL
- **Tools**: Make, Doxygen, cppcheck
- **Platform**: WSL2 Ubuntu development environment
