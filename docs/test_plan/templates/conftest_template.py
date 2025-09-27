"""
Template for pytest conftest.py - agents can copy and modify this template.

This file provides shared configuration and fixtures for all tests.
It should be placed in the root of the tests directory.

Key features:
- Pytest configuration
- Shared fixtures available to all tests
- Custom markers registration
- Test session setup/teardown
- Logging configuration
- Performance tracking
"""

import pytest
import logging
import json
from pathlib import Path
from datetime import datetime
from typing import Dict, Any, List

# Configure logging for tests
def setup_test_logging():
    """Configure logging for test execution."""
    log_format = '%(asctime)s - %(name)s - %(levelname)s - %(message)s'
    logging.basicConfig(
        level=logging.INFO,
        format=log_format,
        handlers=[
            logging.StreamHandler(),
            logging.FileHandler('reports/test_execution.log')
        ]
    )

# Pytest configuration hooks

def pytest_configure(config):
    """Configure pytest with custom settings."""
    # Ensure reports directory exists
    Path("reports").mkdir(exist_ok=True)
    Path("reports/coverage").mkdir(exist_ok=True)
    Path("reports/performance").mkdir(exist_ok=True)
    Path("reports/compliance").mkdir(exist_ok=True)

    # Setup test logging
    setup_test_logging()

    # Register custom markers
    config.addinivalue_line(
        "markers", "unit: Unit tests with mocked dependencies"
    )
    config.addinivalue_line(
        "markers", "integration: Integration tests with real ONVIF calls"
    )
    config.addinivalue_line(
        "markers", "performance: Performance and load tests"
    )
    config.addinivalue_line(
        "markers", "compliance: ONVIF specification compliance tests"
    )
    config.addinivalue_line(
        "markers", "slow: Tests that take >10 seconds to run"
    )
    config.addinivalue_line(
        "markers", "critical: Tests that must pass for service stability"
    )
    # Add service-specific markers
    config.addinivalue_line(
        "markers", "onvif_device: Device service tests"
    )
    config.addinivalue_line(
        "markers", "onvif_media: Media service tests"
    )
    config.addinivalue_line(
        "markers", "onvif_ptz: PTZ service tests"
    )
    config.addinivalue_line(
        "markers", "onvif_imaging: Imaging service tests"
    )

def pytest_collection_modifyitems(config, items):
    """Modify test collection to add automatic markers."""
    for item in items:
        # Auto-mark slow tests
        if hasattr(item.function, '__name__'):
            if 'performance' in item.function.__name__ or 'load' in item.function.__name__:
                item.add_marker(pytest.mark.slow)

        # Auto-mark critical tests
        if 'critical' in str(item.fspath) or 'critical' in item.function.__name__:
            item.add_marker(pytest.mark.critical)

def pytest_sessionstart(session):
    """Called after the Session object has been created."""
    logger = logging.getLogger(__name__)
    logger.info(f"Starting test session at {datetime.now()}")
    logger.info(f"Test collection: {len(session.items) if hasattr(session, 'items') else 'unknown'} tests")

def pytest_sessionfinish(session, exitstatus):
    """Called after whole test run finished."""
    logger = logging.getLogger(__name__)
    logger.info(f"Test session finished with exit status: {exitstatus}")

# Shared fixtures available to all tests

@pytest.fixture(scope="session")
def test_session_info():
    """Provide information about the current test session."""
    return {
        'start_time': datetime.now(),
        'session_id': f"test_session_{int(datetime.now().timestamp())}",
        'reports_dir': Path("reports")
    }

@pytest.fixture(scope="session")
def device_config():
    """Provide device configuration for all tests."""
    from config.test_config import DeviceTestConfig  # Replace with actual config class

    # Try to load from environment or config file
    config_file = Path("config/test_device_config.json")

    if config_file.exists():
        with open(config_file, 'r') as f:
            config_data = json.load(f)
            return DeviceTestConfig(**config_data)
    else:
        # Default configuration - modify for your setup
        return DeviceTestConfig(
            name="test_camera",
            ip_address="192.168.1.100",  # Replace with actual test device
            username="admin",
            password="admin"
        )

@pytest.fixture(scope="session")
def device_client(device_config):
    """Create device client for all tests."""
    from utils.device_client import ONVIFDeviceClient  # Replace with actual client

    client = ONVIFDeviceClient(device_config)

    # Validate connection before running tests
    try:
        response = client.get_device_information()
        if response.status_code != 200:
            pytest.exit(f"Cannot connect to test device at {device_config.ip_address}")
    except Exception as e:
        pytest.exit(f"Device connection failed: {e}")

    return client

@pytest.fixture(scope="session")
def device_capabilities(device_client):
    """Get device capabilities once for all tests."""
    try:
        response = device_client.get_capabilities()
        return device_client.parse_capabilities(response)
    except Exception as e:
        pytest.exit(f"Cannot retrieve device capabilities: {e}")

# Test data fixtures

@pytest.fixture
def test_data_dir(tmp_path):
    """Provide temporary directory for test data."""
    test_data = tmp_path / "test_data"
    test_data.mkdir()
    return test_data

@pytest.fixture
def performance_tracker():
    """Track performance metrics across tests."""
    class PerformanceTracker:
        def __init__(self):
            self.metrics = {}

        def record_metric(self, test_name: str, metric_name: str, value: float):
            if test_name not in self.metrics:
                self.metrics[test_name] = {}
            self.metrics[test_name][metric_name] = value

        def get_summary(self) -> Dict[str, Any]:
            return {
                'total_tests': len(self.metrics),
                'metrics': self.metrics,
                'generated_at': datetime.now().isoformat()
            }

        def save_results(self, file_path: Path):
            with open(file_path, 'w') as f:
                json.dump(self.get_summary(), f, indent=2)

    tracker = PerformanceTracker()

    yield tracker

    # Save performance results at end of session
    results_file = Path("reports/performance/session_metrics.json")
    tracker.save_results(results_file)

# Error handling fixtures

@pytest.fixture
def capture_errors():
    """Capture and track errors during test execution."""
    errors = []

    class ErrorCapture:
        def add_error(self, error_info: Dict[str, Any]):
            errors.append(error_info)

        def get_errors(self) -> List[Dict[str, Any]]:
            return errors.copy()

        def has_errors(self) -> bool:
            return len(errors) > 0

    return ErrorCapture()

# Validation fixtures

@pytest.fixture
def soap_validator():
    """Provide SOAP response validator."""
    from utils.onvif_validator import validate_soap_format, validate_onvif_response

    class SOAPValidator:
        @staticmethod
        def validate_response(response, expected_operation: str):
            """Validate SOAP response format and content."""
            validate_soap_format(response, f"{expected_operation}Response")
            validate_onvif_response(response, expected_operation)

        @staticmethod
        def validate_error_response(response):
            """Validate SOAP fault response."""
            assert "soap:Fault" in response.text or "SOAP-ENV:Fault" in response.text
            assert response.status_code >= 400

    return SOAPValidator()

# Configuration management fixtures

@pytest.fixture
def environment_config():
    """Provide environment-specific configuration."""
    import os

    env = os.getenv('TEST_ENVIRONMENT', 'development')

    configs = {
        'development': {
            'strict_timing': False,
            'performance_thresholds': {'response_time': 5.0},
            'skip_slow_tests': True
        },
        'testing': {
            'strict_timing': True,
            'performance_thresholds': {'response_time': 2.0},
            'skip_slow_tests': False
        },
        'ci': {
            'strict_timing': True,
            'performance_thresholds': {'response_time': 3.0},
            'skip_slow_tests': False
        }
    }

    return configs.get(env, configs['development'])

# Skip conditions

@pytest.fixture
def skip_if_no_capability(device_capabilities):
    """Factory fixture to skip tests based on device capabilities."""
    def _skip_if_missing(capability_name: str):
        if capability_name not in device_capabilities:
            pytest.skip(f"Device does not support {capability_name}")
    return _skip_if_missing

@pytest.fixture
def skip_if_slow(environment_config):
    """Skip slow tests in development environment."""
    if environment_config.get('skip_slow_tests', False):
        pytest.skip("Skipping slow test in development environment")

# Cleanup fixtures

@pytest.fixture
def cleanup_registry():
    """Registry for cleanup functions to run after test completion."""
    cleanup_functions = []

    class CleanupRegistry:
        def register(self, cleanup_func):
            cleanup_functions.append(cleanup_func)

        def cleanup_all(self):
            for cleanup_func in reversed(cleanup_functions):
                try:
                    cleanup_func()
                except Exception as e:
                    logging.warning(f"Cleanup function failed: {e}")

    registry = CleanupRegistry()

    yield registry

    # Run all registered cleanup functions
    registry.cleanup_all()

# Reporting fixtures

@pytest.fixture(scope="session")
def test_results_collector():
    """Collect test results for reporting."""
    class TestResultsCollector:
        def __init__(self):
            self.results = []

        def add_result(self, test_name: str, status: str, duration: float, details: Dict[str, Any] = None):
            result = {
                'test_name': test_name,
                'status': status,
                'duration': duration,
                'timestamp': datetime.now().isoformat(),
                'details': details or {}
            }
            self.results.append(result)

        def get_summary(self) -> Dict[str, Any]:
            passed = sum(1 for r in self.results if r['status'] == 'passed')
            failed = sum(1 for r in self.results if r['status'] == 'failed')
            skipped = sum(1 for r in self.results if r['status'] == 'skipped')

            return {
                'total_tests': len(self.results),
                'passed': passed,
                'failed': failed,
                'skipped': skipped,
                'pass_rate': (passed / len(self.results)) * 100 if self.results else 0,
                'results': self.results
            }

        def save_results(self, file_path: Path):
            with open(file_path, 'w') as f:
                json.dump(self.get_summary(), f, indent=2)

    collector = TestResultsCollector()

    yield collector

    # Save results at end of session
    results_file = Path("reports/test_session_results.json")
    collector.save_results(results_file)

# Usage Instructions for Agents:
#
# 1. Copy this template to tests/conftest.py
# 2. Update device_config fixture with your actual test device settings
# 3. Replace placeholder imports with actual client and config classes
# 4. Add any additional shared fixtures needed for your tests
# 5. Modify markers to match your service categories
# 6. Update skip conditions based on your device capabilities
# 7. Add service-specific configuration as needed
# 8. Test that pytest can load configuration correctly
# 9. Verify all fixtures work with your test setup
# 10. Add any project-specific hooks or configurations