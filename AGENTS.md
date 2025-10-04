# Agent Documentation for Anyka AK3918 Hacking Journey

## Project Overview

This repository contains comprehensive reverse-engineering work and custom firmware development for Anyka AK3918-based IP cameras. It includes cross-compilation tools, SD-card bootable payloads, root filesystem modifications, and detailed documentation for understanding and extending camera functionality.

The project focuses on creating a fully ONVIF 2.5 compliant implementation while maintaining compatibility with the existing camera hardware and providing a robust development environment for camera firmware modifications.

## Quick Reference (Essential Commands & Standards)

### Critical Standards

| Rule | ✅ Correct | ❌ Wrong |
|------|------------|----------|
| **Include paths** | `#include "services/common/onvif_types.h"` | `#include "../../services/..."` |
| **Global variables** | `static int g_onvif_device_count = 0;` | `static int device_count = 0;` |
| **Return codes** | `return ONVIF_SUCCESS;` | `return 0;`|
| **Test names** | `test_unit_memory_manager_init` | `test_memory_init`|
| **Mock functions** | `__wrap_platform_init()` | `mock_platform_init()`|

### Essential Commands

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

### Mock Pattern (CMocka)

```c
// Mock function
    platform_result_t __wrap_platform_init(void) {
      function_called();
      return (platform_result_t)mock();
    }

// Test usage
will_return(__wrap_platform_init, PLATFORM_SUCCESS);
expect_value(__wrap_platform_ptz_move, pan, 90.0f);
```

## Mandatory Document Loading Instructions

**CRITICAL**: When working on tasks covered by any of the linked documentation files below, the AGENT MUST:

1. **Load the relevant document** using the appropriate tool (e.g., `read_file`, `mcp_serena_read_memory`, or other file reading tools)
2. **Inform the user** that the document has been loaded and is being used to guide the work
3. **Follow the guidelines** contained in the loaded document throughout the task execution

This ensures consistent application of project standards and reduces context usage by referencing focused, purpose-built documentation modules.

## Documentation Structure

This documentation is organized into focused modules to reduce context usage:

- **[Project Overview](docs/agents/project-overview.md)** - Complete project description, architecture, and key components
- **[Development Workflow](docs/agents/development-workflow.md)** - Complete development process, examples, and common tasks
- **[Review Prompt](docs/agents/review-prompt.md)** - Comprehensive code review guidelines and checklist
- **[Agent Instructions](docs/agents/agent-instructions.md)** - Complete agent workflow and best practices
- **[Quality Assurance](docs/agents/quality-assurance.md)** - Testing, validation, and quality checklists
- **[Error Handling & Debugging](docs/agents/error-handling-debugging.md)** - Debugging guidelines and error handling strategies
- **[Coding Standards](docs/agents/coding-standards.md)** - Detailed coding guidelines and conventions
- **[Testing Guide](docs/agents/testing-guide.md)** - Comprehensive testing framework and mock usage
- **[Security Guidelines](docs/agents/security-guidelines.md)** - Security requirements and best practices
- **[Build System](docs/agents/build-system.md)** - Development environment and build process
- **[Review Prompt](docs/agents/review-prompt.md)** - Comprehensive code review guidelines and checklist
- **[Coredump Analysis](docs/agents/coredump-analysis-prompt.md)** - Debugging and crash analysis procedures

## Key Development Areas

- **`cross-compile/onvif/`** — **CURRENT FOCUS** - Complete ONVIF 2.5 implementation
- **`cross-compile/onvif/tests/`** — **MANDATORY** - Unit testing framework using CMocka
- **`SD_card_contents/anyka_hack/`** — SD card payload system for runtime testing
- **`cross-compile/anyka_reference/akipc/`** — Authoritative vendor reference code

## Essential Workflow

1. **Code** → Follow standards in [Coding Standards](docs/agents/coding-standards.md)
2. **Test** → Run unit tests: `make test`
3. **Quality** → Run linting: `./cross-compile/onvif/scripts/lint_code.sh --check`
4. **Document** → Update docs: `make -C cross-compile/onvif docs`
5. **Review** → Follow [Review Prompt](docs/agents/review-prompt.md)
6. **Deploy** → Test via SD card payload

> **📚 For complete details**: See [Development Workflow](docs/agents/development-workflow.md) and [Agent Instructions](docs/agents/agent-instructions.md)
