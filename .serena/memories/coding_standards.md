# Coding Standards and Conventions

## File Organization (MANDATORY)
- **Include Order** (strict ordering):
  1. System headers (`#include <stdio.h>`)
  2. Third-party library headers (`#include <curl/curl.h>`)
  3. Project-specific headers (`#include "platform.h"`)
  - Alphabetical ordering within each group
  - Separate groups with blank lines

## Include Path Format (MANDATORY)
- **ALWAYS use relative paths from `src/` directory** for project includes
- **CORRECT**: `#include "services/common/onvif_types.h"`
- **INCORRECT**: `#include "../../services/common/onvif_types.h"`
- **Enforcement**: Makefile and clangd configuration enforce this rule

## Variable Naming Conventions (MANDATORY)
- **Global Variables**: `g_<module>_<variable_name>` format
  - Examples: `g_onvif_device_count`, `g_platform_device_name`
  - **Placement**: ALL global variables at top of file after includes
- **Functions**: snake_case with module prefix
- **Constants**: UPPER_CASE with module prefix
- **Local Variables**: snake_case

## Return Code Standards (MANDATORY)
- **NEVER use magic numbers** for return codes (`return -1`, `return 0`)
- **ALWAYS use predefined constants** for all function return values
- **Module-specific constants** in module headers
- **Global constants** in `utils/error/error_handling.h`
- **Examples**: 
  - Success: `HTTP_AUTH_SUCCESS`, `ONVIF_SUCCESS`
  - Errors: `HTTP_AUTH_ERROR_NULL`, `ONVIF_ERROR_INVALID`

## Test Naming Conventions (MANDATORY)
- **Unit Test Files**: `test_unit_<module>_<functionality>.c`
  - Examples: `test_unit_memory_utils.c`, `test_unit_http_auth_verify_credentials_success.c`
- **Integration Test Files**: `test_integration_<service>_<functionality>.c`
  - Examples: `test_integration_ptz_absolute_move_functionality.c`
- **Test Functions**:
  - Unit: `test_unit_<module>_<functionality>`
  - Integration: `test_integration_<service>_<functionality>`
- **Enforcement**: Test runner uses naming convention to filter tests by category

## Documentation Standards (MANDATORY)
- **Doxygen Documentation**: ALL functions must have complete documentation
- **File Headers**: Consistent format with `@file`, `@brief`, `@author`, `@date`
- **Required Tags**: `@brief`, `@param`, `@return`, `@note` where applicable
- **Documentation Generation**: `make docs` after any changes

## Code Structure
- **Indentation**: 2 spaces consistently
- **Functions**: Definitions at top, execution logic at bottom
- **Global Variables**: At top of file after includes
- **Error Handling**: Comprehensive with proper error propagation
- **Memory Management**: Proper allocation/deallocation with error path cleanup

## Code Quality Validation (MANDATORY)
- **Linting**: All code must pass `./cross-compile/onvif/scripts/lint_code.sh --check`
- **Formatting**: All code must pass `./cross-compile/onvif/scripts/format_code.sh --check`
- **Function Ordering**: Verify definitions at top, execution logic at bottom
- **Global Variable Placement**: Verify all globals at top after includes
- **Enforcement**: Must pass before code can be approved

## Security Requirements (MANDATORY)
- **Input Validation**: ALL user inputs validated and sanitized
- **Buffer Management**: Safe string functions, bounds checking
- **Memory Security**: Initialize allocated memory, proper cleanup
- **Network Security**: Validate all network input

## Utility Usage (MANDATORY)
- **NO code duplication** - Always check `src/utils/` before implementing
- **Create utilities** for common operations
- **Thread-safe** utility functions where applicable
- **Complete documentation** and unit tests for all utilities

## NOLINT Usage
- Use `//NOLINT` for legitimate design requirements where linter suggestions conflict
- Always document reasoning for NOLINT usage
- Examples: Global state variables, semantic function parameter ordering