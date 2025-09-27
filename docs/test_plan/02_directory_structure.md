# Test Directory Structure

## Target Organization

Reorganize tests from flat structure to categorized, agent-friendly layout:

```
integration-tests/
├── tests/
│   ├── unit/                    # Unit-style tests (mocked dependencies)
│   │   ├── __init__.py
│   │   ├── test_soap_parsing.py
│   │   ├── test_xml_validation.py
│   │   └── test_config_parsing.py
│   ├── integration/             # True integration tests (real ONVIF calls)
│   │   ├── __init__.py
│   │   ├── test_device_service.py
│   │   ├── test_media_service.py
│   │   ├── test_ptz_service.py
│   │   └── test_imaging_service.py
│   ├── performance/             # Performance and load tests
│   │   ├── __init__.py
│   │   ├── test_response_times.py
│   │   ├── test_concurrent_requests.py
│   │   └── test_memory_usage.py
│   └── compliance/              # ONVIF specification compliance
│       ├── __init__.py
│       ├── test_profile_s_compliance.py
│       ├── test_profile_t_compliance.py
│       └── test_soap_compliance.py
├── fixtures/                    # Shared test fixtures
│   ├── __init__.py
│   ├── device_fixtures.py       # Device connection and config fixtures
│   ├── soap_fixtures.py         # SOAP request/response fixtures
│   ├── performance_fixtures.py  # Performance testing fixtures
│   └── mock_fixtures.py         # Mock objects for unit tests
├── utils/                       # Test utilities
│   ├── __init__.py
│   ├── onvif_validator.py       # ONVIF spec validation utilities
│   ├── test_data_manager.py     # Test data setup/teardown
│   ├── soap_helpers.py          # SOAP request helpers
│   └── assertion_helpers.py     # Custom assertion utilities
├── config/                      # Test configuration
│   ├── __init__.py
│   ├── test_config.py          # Test configuration management
│   └── device_profiles.yaml    # Device configuration profiles
├── reports/                     # Test output and reports
│   ├── coverage/               # Coverage reports
│   ├── performance/            # Performance metrics
│   └── compliance/             # Compliance validation reports
└── scripts/                     # Validation and utility scripts
    ├── validate_structure.py   # Validate directory structure
    ├── migrate_existing_tests.py # Migrate old tests to new structure
    └── generate_test_matrix.py  # Generate test execution matrix
```

## Migration Strategy

### Phase 1: Create New Structure
```bash
# Create new directory structure
mkdir -p integration-tests/tests/{unit,integration,performance,compliance}
mkdir -p integration-tests/{fixtures,utils,config,reports,scripts}
mkdir -p integration-tests/reports/{coverage,performance,compliance}

# Create __init__.py files
find integration-tests -type d -exec touch {}/__init__.py \;
```

### Phase 2: Migrate Existing Tests
```bash
# Move existing tests to appropriate categories
mv test_device_service.py tests/integration/
mv test_media_service.py tests/integration/
mv test_ptz_service.py tests/integration/
mv test_performance.py tests/performance/test_response_times.py
```

### Phase 3: Consolidate Fixtures
```bash
# Merge conftest.py and fixtures.py content
python scripts/consolidate_fixtures.py
```

## File Naming Conventions

### Test Files
- **Unit tests**: `test_[component]_[aspect].py`
  - Example: `test_soap_parsing_validation.py`
- **Integration tests**: `test_[service]_service.py`
  - Example: `test_device_service.py`
- **Performance tests**: `test_[aspect]_performance.py`
  - Example: `test_response_time_performance.py`
- **Compliance tests**: `test_[profile]_compliance.py`
  - Example: `test_profile_s_compliance.py`

### Fixture Files
- **Service fixtures**: `[service]_fixtures.py`
  - Example: `device_fixtures.py`
- **Utility fixtures**: `[purpose]_fixtures.py`
  - Example: `mock_fixtures.py`

### Utility Files
- **Helpers**: `[purpose]_helpers.py`
  - Example: `soap_helpers.py`
- **Validators**: `[aspect]_validator.py`
  - Example: `onvif_validator.py`

## Pytest Configuration Update

Update `pyproject.toml` to work with new structure:

```toml
[tool.pytest.ini_options]
testpaths = ["tests"]
python_files = ["test_*.py"]
python_classes = ["Test*"]
python_functions = ["test_*"]
markers = [
    "unit: Unit tests with mocked dependencies",
    "integration: Integration tests with real ONVIF calls",
    "performance: Performance and load tests",
    "compliance: ONVIF specification compliance tests",
    "slow: Tests that take >10 seconds to run",
    "onvif_device: Device service tests",
    "onvif_media: Media service tests",
    "onvif_ptz: PTZ service tests",
    "onvif_imaging: Imaging service tests",
]
```

## Running Tests by Category

```bash
# Run all tests
pytest

# Run by category
pytest tests/unit/           # Unit tests only
pytest tests/integration/   # Integration tests only
pytest tests/performance/   # Performance tests only
pytest tests/compliance/    # Compliance tests only

# Run by marker
pytest -m "unit"            # Unit tests
pytest -m "integration"     # Integration tests
pytest -m "not slow"        # Skip slow tests
pytest -m "onvif_device"    # Device service tests only

# Combined markers
pytest -m "integration and not slow"  # Fast integration tests
pytest -m "compliance and onvif_device"  # Device compliance tests
```

## Validation Script

Create validation script to ensure structure is correct:

```python
# scripts/validate_structure.py
import os
from pathlib import Path

def validate_directory_structure():
    """Validate that directory structure matches specification."""
    base_path = Path("integration-tests")

    required_dirs = [
        "tests/unit",
        "tests/integration",
        "tests/performance",
        "tests/compliance",
        "fixtures",
        "utils",
        "config",
        "reports",
        "scripts"
    ]

    for dir_path in required_dirs:
        full_path = base_path / dir_path
        assert full_path.exists(), f"Missing required directory: {dir_path}"
        assert (full_path / "__init__.py").exists(), f"Missing __init__.py in {dir_path}"

    print("✅ Directory structure validation passed")

if __name__ == "__main__":
    validate_directory_structure()
```

## Benefits of New Structure

1. **Agent-Friendly**: Clear categorization helps agents understand what each test does
2. **Parallel Execution**: Different categories can run in parallel
3. **Selective Testing**: Run only relevant tests during development
4. **Clear Dependencies**: Separation between unit/integration/performance/compliance
5. **Easier Maintenance**: Related functionality grouped together
6. **Better Reporting**: Separate reports for different test types

## Implementation Order

1. Create directory structure using provided script
2. Move existing tests to appropriate categories
3. Consolidate duplicated fixtures
4. Update pytest configuration
5. Create validation script
6. Verify all tests still pass after migration

## Next Steps

After completing structure reorganization, proceed to [03_code_quality.md](03_code_quality.md) to implement linting and formatting standards.

## Related Documentation

- **Previous**: [01_current_issues.md](01_current_issues.md) - Issues this structure addresses
- **Next**: [03_code_quality.md](03_code_quality.md) - Quality tooling setup
- **Templates**: [templates/conftest_template.py](templates/conftest_template.py) - Configuration template for new structure
- **Reference**: [00_overview.md](00_overview.md) - Overall plan and context