# Test Plan Templates

This directory contains working code templates that agents can copy and modify for implementing the E2E test plan.

## Available Templates

### Core Templates

- **[test_template.py](test_template.py)** - Standard test class structure with patterns for:
  - E2E tests with ONVIF operations
  - Performance testing with statistical analysis
  - Error handling and validation
  - Compliance testing patterns
  - CRUD operations testing

- **[fixture_template.py](fixture_template.py)** - Test fixtures with patterns for:
  - Device configuration and connection management
  - SOAP request/response handling
  - Test data setup and cleanup
  - Performance measurement fixtures
  - Mock fixtures for unit testing

- **[conftest_template.py](conftest_template.py)** - Pytest configuration with:
  - Shared fixtures for all tests
  - Custom markers and test categorization
  - Logging and reporting setup
  - Session management and cleanup

## Usage Instructions

### For Agents

1. **Copy** the appropriate template to your target directory
2. **Rename** the file to match your service (e.g., `test_media_service.py`)
3. **Replace** placeholder method calls with actual ONVIF client methods
4. **Update** markers and categories to match your service
5. **Modify** assertions and validations for specific requirements
6. **Add** service-specific fixtures and test data
7. **Test** that your implementation works correctly

### Template Patterns

Each template includes:
- ✅ **Standard Structure** - Consistent organization across all tests
- ✅ **Error Handling** - Proper exception handling and SOAP fault validation
- ✅ **Performance Tracking** - Built-in performance measurement
- ✅ **Compliance Validation** - ONVIF specification compliance checks
- ✅ **Cleanup Management** - Automatic resource cleanup
- ✅ **Documentation** - Clear docstrings and usage examples

### Customization Points

Look for these markers in templates to customize for your service:

```python
# Replace with actual service
device_client.operation_method()  # Replace with actual method
"OperationResponse"              # Replace with actual response name
['Field1', 'Field2']             # Replace with actual fields
@pytest.mark.onvif_device        # Change to appropriate service marker
```

## Alignment with Test Plan

These templates implement the patterns specified in:

- **[02_directory_structure.md](../02_directory_structure.md)** - File organization
- **[03_code_quality.md](../03_code_quality.md)** - Quality standards and tooling
- **[04_device_tests.md](../04_device_tests.md)** - Device test specifications
- **[05_performance_tests.md](../05_performance_tests.md)** - Performance testing patterns
- **[06_compliance_framework.md](../06_compliance_framework.md)** - Compliance validation

## Quality Standards

All templates follow:
- **Black formatting** (88-character lines)
- **Pylint compliance** (8.0+ score)
- **Type hints** where appropriate
- **Comprehensive docstrings**
- **Error handling** best practices

## Example Usage

### Creating a Media Service Test

```bash
# Copy template
cp templates/test_template.py tests/integration/test_media_service.py

# Edit to customize for Media service
# - Replace class name: TestONVIFServiceTemplate -> TestONVIFMediaService
# - Update markers: @pytest.mark.onvif_device -> @pytest.mark.onvif_media
# - Replace methods: device_client.operation_method() -> device_client.get_profiles()
# - Update response validation for Media service operations
```

### Creating Service-Specific Fixtures

```bash
# Copy fixture template
cp templates/fixture_template.py fixtures/media_fixtures.py

# Customize for Media service
# - Update ServiceConfigTemplate with media-specific fields
# - Add media service operations to service_operations fixture
# - Create media-specific mock responses
# - Add media service validation fixtures
```

## Validation

After creating tests from templates:

```bash
# Validate structure
python scripts/validate_structure.py

# Check code quality
black --check tests/
pylint tests/

# Run tests
pytest tests/integration/test_your_service.py -v
```

## Support

For issues with templates or implementation guidance, refer to:

- **[00_overview.md](../00_overview.md)** - Overall plan and navigation
- **[01_current_issues.md](../01_current_issues.md)** - Common problems and solutions
- Individual module documentation for specific guidance