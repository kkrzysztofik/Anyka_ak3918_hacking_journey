# Code Quality Standards and Tooling

## Overview

Implement comprehensive code quality framework for integration tests with automated tooling and enforcement.

## Quality Standards

### Code Quality Metrics
- **Pylint Score**: 8.0 minimum (out of 10)
- **Test Coverage**: 90% minimum
- **Type Safety**: 100% basedpyright compliance
- **Documentation**: All public functions must have docstrings
- **Line Length**: 88 characters maximum

### Test Quality Standards
- **Test Isolation**: Each test must be independent
- **Test Reliability**: No flaky tests (>99% pass rate)
- **Performance**: Individual tests <30 seconds
- **Naming**: Descriptive test names following conventions

## Tool Configuration

### pyproject.toml Configuration

```toml
[tool.black]
line-length = 88
target-version = ['py39']
include = '\.pyi?$'
extend-exclude = '''
/(
  \.eggs
  | \.git
  | \.hg
  | \.tox
  | \.venv
  | build
  | dist
)/
'''

[tool.isort]
profile = "black"
multi_line_output = 3
line_length = 88
known_first_party = ["tests", "fixtures", "utils"]
sections = ["FUTURE", "STDLIB", "THIRDPARTY", "FIRSTPARTY", "LOCALFOLDER"]

[tool.pylint.main]
load-plugins = ["pylint.extensions.docparams"]

[tool.pylint.messages_control]
max-line-length = 88
disable = [
    "C0114",  # missing-module-docstring
    "C0116",  # missing-function-docstring
    "R0903",  # too-few-public-methods
    "R0913",  # too-many-arguments
    "W0613",  # unused-argument (common in pytest fixtures)
]

[tool.pylint.design]
max-args = 8
max-locals = 20
max-returns = 6
max-branches = 15

[tool.pytest.ini_options]
testpaths = ["tests"]
python_files = ["test_*.py"]
python_classes = ["Test*"]
python_functions = ["test_*"]
addopts = [
    "--strict-markers",
    "--strict-config",
    "--verbose",
    "--tb=short",
    "--cov=tests",
    "--cov=fixtures",
    "--cov=utils",
    "--cov-report=html:reports/coverage/html",
    "--cov-report=term-missing",
    "--html=reports/test_report.html",
    "--self-contained-html",
    "--junitxml=reports/junit.xml",
    "-ra",
    "--maxfail=5",
    "--timeout=300",
    "--disable-warnings",
    "--color=yes",
]
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
    "critical: Tests that must pass for service stability",
    "network: Tests requiring network access",
]

[tool.coverage.run]
source = ["tests", "fixtures", "utils"]
omit = [
    "tests/test_*.py",
    "*/test_*.py",
    "setup.py",
    "venv/*",
    ".venv/*",
]

[tool.coverage.report]
exclude_lines = [
    "pragma: no cover",
    "def __repr__",
    "raise AssertionError",
    "raise NotImplementedError",
    "if __name__ == .__main__.:",
]
```

### Pre-commit Hooks Configuration

Create `.pre-commit-config.yaml`:

```yaml
repos:
  - repo: https://github.com/pre-commit/pre-commit-hooks
    rev: v4.4.0
    hooks:
      - id: trailing-whitespace
      - id: end-of-file-fixer
      - id: check-yaml
      - id: check-added-large-files
      - id: check-merge-conflict
      - id: debug-statements
      - id: check-docstring-first
      - id: check-json
      - id: check-toml
      - id: check-xml

  - repo: https://github.com/psf/black
    rev: 23.3.0
    hooks:
      - id: black
        language_version: python3.9
        args: [--line-length=88]

  - repo: https://github.com/pycqa/isort
    rev: 5.12.0
    hooks:
      - id: isort
        args: [--profile=black, --line-length=88]

  - repo: https://github.com/pycqa/flake8
    rev: 6.0.0
    hooks:
      - id: flake8
        additional_dependencies:
          - flake8-docstrings
          - flake8-bugbear
          - flake8-comprehensions
        args: [--max-line-length=88, --extend-ignore=E203,W503]

  - repo: https://github.com/pycqa/pylint
    rev: v2.17.4
    hooks:
      - id: pylint
        args: [--fail-under=8.0]

  - repo: https://github.com/asottile/pyupgrade
    rev: v3.7.0
    hooks:
      - id: pyupgrade
        args: [--py39-plus]

  - repo: https://github.com/hadialqattan/pycln
    rev: v2.1.3
    hooks:
      - id: pycln
        args: [--config=pyproject.toml]
```

## Setup Commands

### Install Quality Tools

```bash
# Install development dependencies
pip install black isort pylint flake8 pytest-cov pytest-html pre-commit

# Install flake8 extensions
pip install flake8-docstrings flake8-bugbear flake8-comprehensions

# Install pre-commit hooks
pre-commit install

# Install additional tools
pip install pyupgrade pycln
```

### Quality Check Scripts

Create `scripts/run_quality_checks.py`:

```python
#!/usr/bin/env python3
"""Run all code quality checks."""

import subprocess
import sys
from pathlib import Path

def run_command(cmd, description):
    """Run a command and report results."""
    print(f"\n🔍 {description}")
    print(f"Running: {' '.join(cmd)}")

    result = subprocess.run(cmd, capture_output=True, text=True)

    if result.returncode == 0:
        print(f"✅ {description} passed")
        if result.stdout:
            print(result.stdout)
    else:
        print(f"❌ {description} failed")
        if result.stderr:
            print(result.stderr)
        if result.stdout:
            print(result.stdout)

    return result.returncode == 0

def main():
    """Run all quality checks."""
    print("🚀 Running comprehensive code quality checks")

    checks = [
        (["black", "--check", "."], "Black formatting check"),
        (["isort", "--check-only", "."], "Import sorting check"),
        (["flake8", "."], "Flake8 linting"),
        (["pylint", "tests", "fixtures", "utils"], "Pylint analysis"),
        (["pytest", "--cov", "--cov-fail-under=90"], "Test coverage check"),
    ]

    passed = 0
    total = len(checks)

    for cmd, description in checks:
        if run_command(cmd, description):
            passed += 1

    print(f"\n📊 Quality Check Results: {passed}/{total} passed")

    if passed == total:
        print("🎉 All quality checks passed!")
        return 0
    else:
        print("❌ Some quality checks failed")
        return 1

if __name__ == "__main__":
    sys.exit(main())
```

## Quality Gates

### Pre-commit Quality Gate
All commits must pass:
- Black formatting
- Import sorting (isort)
- Flake8 linting
- Basic pre-commit hooks

### CI/CD Quality Gate
All pull requests must pass:
- Pylint score ≥8.0
- Test coverage ≥90%
- All tests passing
- No security vulnerabilities

### Release Quality Gate
All releases must pass:
- Full test suite (including slow tests)
- Performance benchmarks
- ONVIF compliance validation
- Security audit

## Fixing Common Issues

### Black Formatting Issues
```bash
# Auto-fix formatting
black .

# Check what would be changed
black --diff .
```

### Import Sorting Issues
```bash
# Auto-fix imports
isort .

# Check what would be changed
isort --diff .
```

### Pylint Issues
```bash
# Run pylint with detailed output
pylint tests fixtures utils --output-format=colorized

# Generate pylint report
pylint tests fixtures utils --output-format=json > reports/pylint.json
```

### Coverage Issues
```bash
# Generate detailed coverage report
pytest --cov --cov-report=html --cov-report=term-missing

# View coverage report
open reports/coverage/html/index.html
```

## Documentation Standards

### Docstring Format
```python
def validate_onvif_response(response, expected_operation):
    """Validate ONVIF SOAP response format and content.

    Args:
        response: HTTP response object from ONVIF request
        expected_operation: Expected ONVIF operation name

    Returns:
        bool: True if response is valid ONVIF format

    Raises:
        ONVIFValidationError: If response format is invalid

    Example:
        >>> response = make_onvif_request("GetDeviceInformation")
        >>> is_valid = validate_onvif_response(response, "GetDeviceInformation")
        >>> assert is_valid
    """
```

### Test Function Documentation
```python
@pytest.mark.onvif_device
@pytest.mark.integration
def test_device_service_get_capabilities(device_client):
    """Test Device service GetCapabilities operation.

    Validates that GetCapabilities returns proper ONVIF capability
    structure with all required capability categories.

    Test covers:
    - SOAP request format validation
    - Response parsing and structure
    - Required capability categories presence
    - Optional capability categories handling
    """
```

## Validation Commands

```bash
# Run all quality checks
python scripts/run_quality_checks.py

# Run specific checks
black --check .
isort --check-only .
flake8 .
pylint tests fixtures utils
pytest --cov --cov-fail-under=90

# Fix formatting issues
black .
isort .

# Generate quality reports
pylint tests fixtures utils --output-format=json > reports/pylint.json
pytest --cov --cov-report=html --html=reports/test_report.html
```

## Integration with CI/CD

### GitHub Actions Workflow
```yaml
name: Quality Checks
on: [push, pull_request]

jobs:
  quality:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v3
    - uses: actions/setup-python@v4
      with:
        python-version: '3.9'

    - name: Install dependencies
      run: |
        pip install -r requirements-dev.txt

    - name: Run quality checks
      run: python scripts/run_quality_checks.py
```

## Next Steps

After implementing code quality framework, proceed to [04_device_tests.md](04_device_tests.md) for test-specific refactoring guidelines.

## Related Documentation

- **Previous**: [02_directory_structure.md](02_directory_structure.md) - Directory structure setup
- **Next**: [04_device_tests.md](04_device_tests.md) - Device test refactoring
- **Issues Addressed**: [01_current_issues.md](01_current_issues.md#medium-priority-issues) - Code quality problems
- **Templates**: All templates in [templates/](templates/) follow these quality standards
- **Reference**: [00_overview.md](00_overview.md#success-criteria) - Quality goals