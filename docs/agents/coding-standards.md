# Coding Standards & Guidelines

## C Code Standards

### Include Order (MANDATORY)
1. **System headers first** (e.g., `#include <stdio.h>`, `#include <stdlib.h>`)
2. **Third-party library headers** (e.g., `#include <curl/curl.h>`)
3. **Project-specific headers** (e.g., `#include "platform.h"`, `#include "common.h"`)

- Within each group, use alphabetical ordering
- Separate groups with blank lines
- Use include guards (`#ifndef HEADER_H` / `#define HEADER_H` / `#endif`) or `#pragma once`
- Avoid unnecessary includes and circular dependencies

### Return Code Constants (MANDATORY)

- **NEVER use magic numbers** for return codes (e.g., `return -1`, `return 0`)
- **ALWAYS use predefined constants** for all function return values
- **Module-specific constants** should be defined in the module's header file
- **Global constants** should be defined in `utils/error/error_handling.h`

**Examples**:
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

**Constant naming convention**:
- Success: `MODULE_SUCCESS` (e.g., `HTTP_AUTH_SUCCESS`, `ONVIF_SUCCESS`)
- Errors: `MODULE_ERROR_TYPE` (e.g., `HTTP_AUTH_ERROR_NULL`, `ONVIF_ERROR_INVALID`)

### Include Path Format (MANDATORY)

- **ALWAYS use relative paths from `src/` directory** for all project includes
- **CORRECT format**: `#include "services/common/video_config_types.h"`
- **INCORRECT format**: `#include "../../services/common/video_config_types.h"`

**Examples**:
```c
// ✅ CORRECT - Relative from src/
#include "services/common/onvif_types.h"
#include "utils/validation/common_validation.h"
#include "platform/platform.h"

// ❌ INCORRECT - Relative paths with ../
#include "../../services/common/onvif_types.h"
#include "../validation/common_validation.h"
```

## Code Organization

### Global Variable Naming (MANDATORY)

- **ALL global variables MUST start with `g_<module>_<variable_name>`**
- **Module prefix** should match the source file or functional area (e.g., `g_onvif_`, `g_platform_`, `g_media_`)
- **Variable name** should be descriptive and follow snake_case convention

**Examples**:
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

### Global Variable Placement (MANDATORY)

- **ALL global variables MUST be placed at the top of the file** after includes and before any function definitions
- **Grouping**: Group related global variables together with blank lines between groups
- **Initialization**: Initialize global variables at declaration when possible
- **Documentation**: Add comments explaining the purpose of each global variable

**Examples**:
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
```

### Source File Structure & Organization (MANDATORY)

**Directory Structure**:
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

**File Naming Conventions**:
- **Source files**: `onvif_<service>.c` (e.g., `onvif_device.c`)
- **Header files**: `onvif_<service>.h` (e.g., `onvif_device.h`)
- **Utility files**: `<category>_utils.c` (e.g., `memory_utils.c`)
- **Platform files**: `platform_<platform>.c` (e.g., `platform_anyka.c`)

## Utility Usage and Code Reuse (MANDATORY)

- **ALWAYS use existing utilities** - Check `src/utils/` directory for available utility functions before implementing new code
- **NO code duplication** - If similar functionality exists elsewhere, refactor to use shared utilities
- **Create utilities for common operations** - When implementing functionality that might be reused, create utility functions

**Utility function requirements**:
- Must be placed in appropriate `src/utils/` subdirectory
- Must have complete Doxygen documentation
- Must follow consistent naming conventions (e.g., `onvif_util_*` for ONVIF-specific utilities)
- Must include proper error handling and validation
- Must be thread-safe if applicable

## Code Duplication Prevention (MANDATORY)

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

## Documentation Standards (MANDATORY)

### Doxygen Documentation
- **ALL code changes MUST include updated Doxygen documentation**
- **Function documentation**: Every public function must have complete Doxygen comments including `@brief`, `@param`, `@return`, and `@note` where applicable
- **File headers**: Each source file must have a Doxygen file header with `@file`, `@brief`, `@author`, and `@date` tags
- **Structure/Enum documentation**: All public structures, enums, and typedefs must be documented with `@brief` and member descriptions
- **Documentation generation**: Use `doxygen Doxyfile` in the `cross-compile/onvif/` directory to generate updated documentation
- **Documentation validation**: Verify that all new/changed functions appear correctly in the generated HTML documentation

### File Header Standards (MANDATORY)

**Required format**:
```c
/**
 * @file filename.h
 * @brief Brief description of the file's purpose
 * @author kkrzysztofik
 * @date 2025
 */
```

**Header requirements**:
- `@file` - Must match the actual filename
- `@brief` - Concise description of the file's purpose and functionality
- `@author` - Must be "kkrzysztofik" for consistency
- `@date` - Must be "2025" for current year
- **Placement**: File header must be the first content in the file (after any include guards)
