# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a comprehensive reverse-engineering project for Anyka AK3918-based IP cameras, featuring:

- **Complete ONVIF 2.5 implementation** with Device, Media, PTZ, and Imaging services
- **Platform abstraction layer** for hardware-agnostic operations
- **RTSP streaming** with H.264 video and audio support
- **Native cross-compilation environment** in WSL Ubuntu for consistent builds
- **SD card payload system** for safe firmware testing without flash modifications
- **Comprehensive unit testing framework** using CMocka for utility function testing

## Quick Reference (Essential Commands & Standards)

### Critical Standards

| Rule | ✅ Correct | ❌ Wrong |
|------|------------|----------|
| **Include paths** | `#include "services/common/onvif_types.h"` | `#include "../../services/..."`|
| **Global variables** | `static int g_onvif_device_count = 0;` | `static int device_count = 0;`|
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

## Core Architecture

The system uses a layered architecture:

```
Web Interface → CGI Scripts → HTTP/SOAP → ONVIF Services → Platform Abstraction → Anyka SDK → Hardware
```

**Key directories:**

- `cross-compile/onvif/` - **CURRENT FOCUS** - Complete ONVIF 2.5 implementation
- `cross-compile/onvif/tests/` - **MANDATORY** - Unit testing framework using CMocka
- `cross-compile/anyka_reference/akipc/` - Authoritative vendor reference code
- `SD_card_contents/anyka_hack/` - SD card payload for runtime testing
- `e2e/` - Python-based test suite with pytest

## Essential Development Workflow

1. **Plan**: Use TodoWrite tool for task management
2. **Code**: Follow [Coding Standards](docs/agents/coding-standards.md)
3. **Quality**: Run linting and formatting checks
4. **Test**: Run unit tests with CMocka mocks
5. **Document**: Update Doxygen documentation
6. **Review**: Follow [Review Prompt](docs/agents/review-prompt.md)
7. **Deploy**: Test via SD card payload

> **📚 For complete details**: See [Development Workflow](docs/agents/development-workflow.md) and [Agent Instructions](docs/agents/agent-instructions.md)

**Note**: The project focuses on defensive security only. All code must be secure and robust with proper input validation and error handling.
