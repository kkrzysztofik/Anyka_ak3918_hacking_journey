# Quality Gates - Anyka AK3918 Project

## Quality Assurance Checklist

**Before approving any code changes, verify:**

- [ ] **Bash syntax** used in all commands
- [ ] **Native builds** work correctly
- [ ] **Include ordering** follows standards
- [ ] **Include path format** uses relative paths from `src/` (no `../` patterns)
- [ ] **Global variable naming** follows `g_<module>_<variable_name>` convention
- [ ] **Return code constants** used instead of magic numbers
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
- [ ] **Code linting** passes without issues (`./cross-compile/onvif/scripts/lint_code.sh --check`)
- [ ] **Code formatting** is compliant (`./cross-compile/onvif/scripts/format_code.sh --check`)
- [ ] **Function ordering** follows guidelines (definitions at top, execution logic at bottom)
- [ ] **Global variables** are placed at top of files after includes
- [ ] **Static analysis** passes without critical issues
- [ ] **Memory analysis** shows no leaks or overflows
- [ ] **Security testing** validates input handling
- [ ] **ONVIF compliance** is maintained
- [ ] **File structure** follows directory organization
- [ ] **Tests** pass (if applicable)
- [ ] **Mock Testing** follows CMocka patterns (if unit tests)
- [ ] **Mock Framework** uses only CMocka (no other mocking frameworks)
- [ ] **Mock Implementation** uses `__wrap_` prefix and `--wrap` linker mechanism
- [ ] **Mock File Structure** follows `*_mock.c` and `*_mock.h` pattern
- [ ] **Mock Functions** use standard CMocka patterns (`function_called()`, `check_expected_*()`, `mock()`)
- [ ] **Mock Configuration** uses `will_return()` and `expect_*()` functions correctly
- [ ] **Mock State Management** has proper initialization and cleanup
- [ ] **Mock Parameter Validation** uses CMocka validation functions
- [ ] **Mock Call Verification** checks function calls and parameters
- [ ] **Mock Error Simulation** tests error conditions properly
- [ ] **Mock Documentation** has proper Doxygen documentation
- [ ] **Mock Isolation** ensures tests are independent
- [ ] **Mock Coverage** mocks all external dependencies
- [ ] **Mock Naming** follows `__wrap_<function_name>` convention
- [ ] **Mock File Organization** uses `tests/src/mocks/` directory
- [ ] **Mock Testing Requirements** avoids real hardware/network/file operations

## Code Quality Commands (MANDATORY)

### Code Linting

```bash
# Code Linting (MANDATORY)
./cross-compile/onvif/scripts/lint_code.sh                    # Lint all C files
./cross-compile/onvif/scripts/lint_code.sh --check            # Check for issues (exit 1 if found)
./cross-compile/onvif/scripts/lint_code.sh --file src/path/to/file.c  # Lint specific file
./cross-compile/onvif/scripts/lint_code.sh --changed          # Lint only changed files
./cross-compile/onvif/scripts/lint_code.sh --format           # Also check formatting
./cross-compile/onvif/scripts/lint_code.sh --severity error   # Fail only on errors
```

### Code Formatting

```bash
# Code Formatting (MANDATORY)
./cross-compile/onvif/scripts/format_code.sh                  # Format all C files
./cross-compile/onvif/scripts/format_code.sh --check          # Check formatting (exit 1 if issues)
./cross-compile/onvif/scripts/format_code.sh --files src/path/to/file.c  # Format specific files
./cross-compile/onvif/scripts/format_code.sh --dry-run        # Show what would be changed
```

### Function Order Compliance (MANDATORY)

- **Verify function ordering** follows project guidelines: definitions at top, execution logic at bottom
- **Check global variables** are placed at the top of files after includes
- **Validate include ordering**: system headers → third-party → project headers
- **Ensure proper spacing** and formatting between function groups
- **Use linting script** with `--format` flag to check function ordering compliance

## Testing Commands

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

## Review Process

### Automated Analysis (REQUIRED)

```bash
# Run linter analysis - MUST complete successfully
read_lints cross-compile/onvif/

# Verify build success - MUST pass
cd cross-compile/onvif && make clean && make

# Check test execution - MUST pass
make test
```

### Critical Standards Validation (REQUIRED)

- [ ] **Return Code Constants**: NO magic numbers (`return 0`, `return -1`) - MUST use predefined constants
- [ ] **Global Variable Naming**: MUST follow `g_<module>_<variable_name>` pattern
- [ ] **Include Path Format**: MUST use relative paths from `src/` directory
- [ ] **Test Naming Convention**: MUST follow `test_unit_<module>_<functionality>` pattern
- [ ] **File Headers**: MUST have consistent Doxygen headers with @file, @brief, @author, @date

### Security Assessment (REQUIRED)

- [ ] **Input Validation**: All user inputs properly sanitized
- [ ] **Buffer Management**: No buffer overflows or bounds violations
- [ ] **Memory Safety**: No leaks or double-free issues
- [ ] **Authentication**: Security gaps in auth implementation

## Success Criteria

A successful quality assurance process should:

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

## Review Output Format

### Executive Summary (200 words max)

```markdown
## ONVIF Code Review Summary

**Build Status**: [✅ Success / ❌ Failed]
**Critical Issues**: [X] found
**Security Vulnerabilities**: [X] high, [X] medium
**Standards Compliance**: [✅ Compliant / ❌ X violations]
**ONVIF Compliance**: [✅ Compliant / ⚠️ Issues found]

**Recommendation**: [APPROVE / REJECT / CONDITIONAL APPROVAL]
```

### Critical Issues Only (1,500 words max)

For each critical issue, provide:

````markdown
## 🚨 **CRITICAL ISSUE**: [Brief Description]

**File**: `path/to/file.c:line`
**Severity**: [Critical/High]
**Rule Violated**: [Specific coding standard]
**Impact**: [Security/Functionality/Compliance impact]

**Current Code**:

```c
[Code snippet]
```
````

**Required Fix**:

```c
[Corrected code]
```

**Rationale**: [Why this fix is necessary]

````

### Standards Violations Summary (300 words max)

```markdown
## 📋 **Standards Compliance Report**

| Standard | Status | Violations | Examples |
|----------|--------|------------|----------|
| Return Code Constants | [✅/❌] | [X] | `return 0` in file.c:123 |
| Global Variable Naming | [✅/❌] | [X] | `device_count` should be `g_onvif_device_count` |
| Include Path Format | [✅/❌] | [X] | `#include "../../services/..."` |
| Test Naming Convention | [✅/❌] | [X] | `test_init` should be `test_unit_memory_manager_init` |
````

## Constraints & Limitations

### What to IGNORE (Focus on Critical Only)

- Minor style violations (spacing, indentation)
- Documentation completeness (unless critical)
- Performance optimizations (unless blocking)
- Code duplication (unless security-related)
- Minor architectural improvements

### What to PRIORITIZE (Must Address)

- Security vulnerabilities (buffer overflows, injection attacks)
- Memory leaks and resource management
- ONVIF specification violations
- Critical coding standards violations
- Build failures and compilation errors

### Response Length Limits

- **Total Response**: 2,000-3,000 words maximum
- **Executive Summary**: 200 words maximum
- **Critical Issues**: 1,500 words maximum
- **Standards Summary**: 300 words maximum

## Framework Version Constraints

**MANDATORY**: Use only the following verified versions:

- **ONVIF Specification**: 2.5 (confirmed)
- **gSOAP Version**: 2.8.x (as specified in project)
- **CMocka Version**: 1.1.x (as specified in tests)
- **Anyka Platform**: AK3918 (confirmed hardware target)

**DO NOT**:

- Assume or guess framework versions
- Reference unspecified library versions
- Use outdated or incorrect version numbers
