# E2E Test Plan Overview

## Purpose

This modular test plan ensures ONVIF Profile S & T compliance for the Anyka AK3918 camera implementation. The plan is broken into focused modules for agent-friendly processing.

## Module Structure

- **00_overview.md** - This file - high-level goals and navigation
- **01_current_issues.md** - Current problems that need fixing
- **02_directory_structure.md** - Test organization and file layout
- **03_code_quality.md** - Quality standards, linting, and tooling
- **04_device_tests.md** - Device service test specifications
- **05_performance_tests.md** - Performance requirements and metrics
- **06_compliance_framework.md** - ONVIF compliance validation
- **templates/** - Working code templates for implementation

## Quick Start for Agents

1. Read **01_current_issues.md** to understand what needs fixing
2. Use **02_directory_structure.md** to reorganize test files
3. Implement **03_code_quality.md** tooling setup
4. Refactor tests using **04_device_tests.md** and **05_performance_tests.md**
5. Add compliance validation from **06_compliance_framework.md**
6. Use **templates/** as starting points for new code

## Success Criteria

- 90% test coverage minimum
- Pylint score 8.0+
- All ONVIF Profile S mandatory features tested
- <2s average response times
- Zero code duplication in test fixtures

## Implementation Checkpoints

Each module contains validation scripts to verify correct implementation:

```bash
# Run validation for each checkpoint
python scripts/validate_checkpoint_1.py  # Basic structure
python scripts/validate_checkpoint_2.py  # Code quality
python scripts/validate_checkpoint_3.py  # Test refactoring
python scripts/validate_final.py         # Full compliance
```

## Agent Context Management

Each module is designed to fit within agent context limits (~2000 lines). Cross-references use relative links to related sections without duplicating content.

## Next Steps

Start with **01_current_issues.md** to understand the current state and what needs to be addressed.