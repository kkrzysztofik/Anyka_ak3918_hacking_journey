# Code Quality Standards

## 📝 Coding Standards Compliance

### MANDATORY Requirements from AGENTS.md

#### 1. Include Ordering (Critical)
```c
// CORRECT order:
// 1. System headers first
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// 2. Third-party library headers
#include <curl/curl.h>
#include <gsoap/soapH.h>

// 3. Project-specific headers (relative from src/)
#include "platform/platform.h"
#include "services/common/onvif_types.h"
#include "utils/memory/memory_manager.h"
```

#### 2. Include Path Format (MANDATORY)
```c
// ✅ CORRECT - Relative from src/
#include "services/common/onvif_types.h"
#include "utils/validation/input_validation.h"
#include "platform/platform.h"

// ❌ INCORRECT - Relative paths with ../
#include "../../services/common/onvif_types.h"
#include "../validation/input_validation.h"
```

#### 3. Global Variable Naming (MANDATORY)
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

#### 4. Global Variable Placement (MANDATORY)
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

### Documentation Standards (MANDATORY)

#### File Headers (MANDATORY)
```c
/**
 * @file onvif_device.c
 * @brief ONVIF Device Service implementation for Anyka AK3918
 * @author kkrzysztofik
 * @date 2025
 */
```

#### Function Documentation (MANDATORY)
```c
/**
 * @brief Handle ONVIF GetDeviceInformation request
 * @param config Service handler configuration (must not be NULL)
 * @param request HTTP request structure (must not be NULL)
 * @param response Response structure to populate (must not be NULL)
 * @param gsoap_ctx gSOAP context for XML processing (must not be NULL)
 * @return 0 on success, -1 on error
 * @note Uses direct HTTP request access per refactoring guide
 *       Follows security and validation standards from AGENTS.md
 */
static int handle_get_device_information(
    const service_handler_config_t *config,
    const http_request_t *request,
    onvif_response_t *response,
    onvif_gsoap_context_t *gsoap_ctx) {
    // Implementation...
}
```

### Code Organization Standards

#### Function Ordering
```c
// ✅ CORRECT - Definitions at top, execution logic at bottom
static int validate_request(const http_request_t *request);
static int build_response(onvif_response_t *response, const char *content);
static int handle_device_info(const http_request_t *request, onvif_response_t *response);

// Main execution logic at bottom
int onvif_device_handler(const http_request_t *request, onvif_response_t *response) {
    if (validate_request(request) != 0) {
        return -1;
    }

    // ... execution logic ...

    return build_response(response, content);
}
```

#### Consistent Indentation
- **2-space indentation** throughout
- **Consistent brace placement**
- **Proper spacing around operators**

## 🔧 Code Quality Tools

### Static Analysis Commands
```bash
# Check for coding standards violations
grep -r "#include.*\.\./" cross-compile/onvif/src/ --include="*.c" --include="*.h" && \
echo "❌ Found relative include paths - MUST fix per AGENTS.md" || \
echo "✅ Include paths compliant"

# Check for missing Doxygen documentation
find cross-compile/onvif/src/services/ -name "*.c" -exec grep -L "@brief" {} \; | head -5

# Check for non-compliant global variable naming
grep -r "^static.*[^g_][a-zA-Z_][a-zA-Z0-9_]*\s*=" cross-compile/onvif/src/ --include="*.c" | \
grep -v "g_[a-zA-Z_][a-zA-Z0-9_]*" | head -5
```

### Compilation Testing
```bash
# Test compilation with WSL native build
cd cross-compile/onvif
make clean
make 2>&1 | grep -E "(error|Error)" | head -5

# Build with verbose output for debugging
make VERBOSE=1
```

### Documentation Generation
```bash
# Generate documentation
make docs

# Check if documentation was generated
test -f docs/html/index.html && echo "✅ Documentation updated"

# Check for documentation errors
make docs 2>&1 | grep -E "(error|Error|warning)"
```

## 🚫 Code Duplication Prevention

### MANDATORY: Use Existing Utilities

#### Check for Existing Utilities First
```bash
# List available utilities
find src/utils/ -name "*.h" -exec basename {} \; | sort

# Check for existing functionality before implementing
grep -r "function_name" src/utils/ --include="*.c" --include="*.h"
```

#### Common Duplication Patterns to Avoid
```c
// ❌ BAD - Duplicated string validation
int validate_token(const char* token) {
    if (!token || strlen(token) == 0 || strlen(token) > 32) {
        return 0;
    }
    return 1;
}

// ✅ GOOD - Use existing utility
#include "utils/validation/input_validation.h"
int validate_token(const char* token) {
    return onvif_util_validate_token(token);
}
```

### Utility Function Requirements
- Must be placed in appropriate `src/utils/` subdirectory
- Must have complete Doxygen documentation
- Must follow consistent naming conventions
- Must include proper error handling and validation
- Must be thread-safe if applicable

## 🔍 Code Review Checklist

### Pre-Review Checklist
- [ ] **Bash syntax** used in all commands
- [ ] **Native builds** work correctly
- [ ] **Include ordering** follows standards
- [ ] **Include path format** uses relative paths from `src/`
- [ ] **Global variable naming** follows `g_<module>_<variable_name>` convention
- [ ] **Doxygen documentation** is complete and updated
- [ ] **Utility functions** are used instead of duplicated code
- [ ] **No code duplication** exists

### Code Quality Review
- [ ] **Memory management** is correct (no leaks)
- [ ] **Error handling** is comprehensive
- [ ] **Input validation** is present
- [ ] **Thread safety** is maintained
- [ ] **Performance** is acceptable
- [ ] **Security** concerns are addressed
- [ ] **Code style** is consistent

### Documentation Review
- [ ] **File headers** follow standard format
- [ ] **Function documentation** is complete
- [ ] **Parameter descriptions** are accurate
- [ ] **Return value descriptions** are clear
- [ ] **Notes and warnings** are included where needed

## 🛠️ Code Quality Fixes

### Fix Include Path Violations
```bash
# Find non-compliant include paths
grep -r "#include.*\.\./" cross-compile/onvif/src/ --include="*.c" --include="*.h"

# Replace with compliant paths
sed -i 's|#include "\.\./\.\./|#include "|g' cross-compile/onvif/src/services/device/onvif_device.c
```

### Fix Global Variable Naming
```bash
# Find non-compliant global variables
grep -r "^static.*[^g_][a-zA-Z_][a-zA-Z0-9_]*\s*=" cross-compile/onvif/src/ --include="*.c"

# Replace with compliant naming
sed -i 's/static int device_count/static int g_device_count/g' cross-compile/onvif/src/services/device/onvif_device.c
```

### Add Missing Documentation
```c
// Add file header if missing
/**
 * @file filename.c
 * @brief Brief description of the file's purpose
 * @author kkrzysztofik
 * @date 2025
 */

// Add function documentation
/**
 * @brief Brief description of function purpose
 * @param param1 Description of parameter 1
 * @param param2 Description of parameter 2
 * @return Description of return value
 * @note Any important notes or warnings
 */
```

## 📊 Code Quality Metrics

### Current State Measurement
```bash
# Count documentation compliance
echo "Functions with documentation:" >> quality_metrics.txt
find cross-compile/onvif/src/services/ -name "*.c" -exec grep -l "@brief" {} \; | wc -l >> quality_metrics.txt

# Count include path compliance
echo "Non-compliant include paths:" >> quality_metrics.txt
grep -r "#include.*\.\./" cross-compile/onvif/src/ --include="*.c" --include="*.h" | wc -l >> quality_metrics.txt

# Count global variable compliance
echo "Non-compliant global variables:" >> quality_metrics.txt
grep -r "^static.*[^g_][a-zA-Z_][a-zA-Z0-9_]*\s*=" cross-compile/onvif/src/ --include="*.c" | wc -l >> quality_metrics.txt
```

### Target State Goals
- **Documentation compliance**: 100% of functions documented
- **Include path compliance**: 0 non-compliant paths
- **Global variable compliance**: 100% compliant naming
- **Code duplication**: 0 instances
- **Compilation warnings**: 0 warnings

## ✅ Success Criteria

- [ ] **Consistent response handling** across all services
- [ ] **MANDATORY**: All functions have complete Doxygen documentation
- [ ] **MANDATORY**: All includes use relative paths from `src/`
- [ ] **MANDATORY**: Global variables use `g_<module>_<variable_name>` naming
- [ ] **MANDATORY**: Documentation updated with `make docs` after changes
- [ ] **Proper cleanup** in all error paths
- [ ] **Compilation** with zero warnings using WSL native build
- [ ] **MANDATORY**: No code duplication - use existing utilities only

## 🔧 Troubleshooting

### Common Issues

**Compilation Errors**
```bash
# Missing includes - use WSL native build per CLAUDE.md
cd cross-compile/onvif
make 2>&1 | grep -E "(error|Error)"

# Check for existing utilities before creating new ones
find src/utils/ -name "*.h" -exec basename {} \; | sort
```

**Documentation Generation Issues**
```bash
# Check for documentation errors
cd cross-compile/onvif && make docs 2>&1 | grep -E "(error|Error|warning)"

# Verify documentation was generated
test -f docs/html/index.html && echo "✅ Documentation updated"
```

**Code Quality Violations**
```bash
# Check for all coding standard violations
bash scripts/check_coding_standards.sh

# Fix violations automatically where possible
bash scripts/fix_coding_standards.sh
```
