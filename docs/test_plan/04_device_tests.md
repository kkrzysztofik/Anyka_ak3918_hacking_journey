# Device Service Test Specifications

## Overview

Comprehensive test specifications for ONVIF Device service, focusing on eliminating code duplication and ensuring complete ONVIF Profile S/T compliance.

## Current Device Service Issues

### Problems to Fix
- **Code Duplication**: Fixtures repeated across test files
- **Missing Operations**: 5 required ONVIF operations not tested
- **Poor Error Handling**: Inconsistent error validation
- **No Compliance Validation**: Tests pass but may not be ONVIF compliant

### Current Test Coverage
```
✅ Tested Operations (10):
- GetDeviceInformation
- GetCapabilities
- GetSystemDateAndTime
- GetServices
- GetServiceCapabilities
- GetUsers (basic)
- GetNTP
- GetHostname
- GetDNS
- GetNetworkInterfaces

❌ Missing Operations (5):
- SetSystemDateAndTime
- CreateUsers/DeleteUsers
- SetNTP
- SetHostname
- SetNetworkInterfaces
```

## Refactored Test Structure

### Enhanced Device Fixtures

Create `fixtures/device_fixtures.py`:

```python
"""Device service test fixtures with zero duplication."""

import pytest
from dataclasses import dataclass
from typing import Dict, Any, Optional
from utils.soap_helpers import make_soap_request
from utils.onvif_validator import validate_onvif_response

@dataclass
class DeviceConfig:
    """Device configuration for testing."""
    name: str
    ip_address: str
    http_port: int = 8080
    username: str = "admin"
    password: str = "admin"
    timeout: int = 30

    # Expected device information
    expected_manufacturer: str = "Anyka"
    expected_model: str = "AK3918 Camera"
    expected_firmware_version: str = "1.0.0"

@pytest.fixture(scope="session")
def device_config():
    """Provide device configuration for all tests."""
    return DeviceConfig(
        name="test_camera",
        ip_address="192.168.1.100"
    )

@pytest.fixture(scope="session")
def device_client(device_config):
    """Create device client with connection validation."""
    from utils.device_client import ONVIFDeviceClient

    client = ONVIFDeviceClient(device_config)

    # Validate connection works
    response = client.get_device_information()
    assert response.status_code == 200, "Device connection failed"

    return client

@pytest.fixture
def soap_headers():
    """Standard SOAP headers for all requests."""
    return {
        'Content-Type': 'application/soap+xml',
        'SOAPAction': '""',
    }

@pytest.fixture
def device_operations():
    """Map of device operations to test."""
    return {
        'GetDeviceInformation': {
            'required': True,
            'profile_s': True,
            'profile_t': True,
            'response_fields': ['Manufacturer', 'Model', 'FirmwareVersion']
        },
        'GetCapabilities': {
            'required': True,
            'profile_s': True,
            'profile_t': True,
            'response_fields': ['Device', 'Media', 'PTZ']
        },
        'GetSystemDateAndTime': {
            'required': True,
            'profile_s': True,
            'profile_t': True,
            'response_fields': ['DateTimeType', 'DaylightSavings']
        },
        'SetSystemDateAndTime': {
            'required': False,
            'profile_s': False,
            'profile_t': True,
            'response_fields': []
        }
    }
```

### Enhanced Device Tests

Create `tests/integration/test_device_service.py`:

```python
"""Enhanced Device service integration tests."""

import pytest
from utils.onvif_validator import validate_onvif_response, validate_soap_format
from utils.assertion_helpers import assert_required_fields, assert_field_types

class TestONVIFDeviceService:
    """Comprehensive Device service tests."""

    @pytest.mark.onvif_device
    @pytest.mark.integration
    @pytest.mark.critical
    def test_get_device_information(self, device_client, device_config):
        """Test GetDeviceInformation with complete validation.

        Validates:
        - SOAP request format
        - Response structure compliance
        - Required fields presence
        - Field type validation
        - ONVIF specification compliance
        """
        # Execute operation
        response = device_client.get_device_information()

        # Validate SOAP compliance
        validate_soap_format(response, "GetDeviceInformationResponse")

        # Parse response
        device_info = device_client.parse_device_information(response)

        # Validate required fields
        required_fields = ['Manufacturer', 'Model', 'FirmwareVersion',
                          'SerialNumber', 'HardwareId']
        assert_required_fields(device_info, required_fields)

        # Validate field types and constraints
        assert isinstance(device_info['Manufacturer'], str)
        assert len(device_info['Manufacturer']) <= 64, "Manufacturer name too long"
        assert device_info['Manufacturer'].strip(), "Manufacturer cannot be empty"

        # Validate expected values (if configured)
        if device_config.expected_manufacturer:
            assert device_info['Manufacturer'] == device_config.expected_manufacturer

    @pytest.mark.onvif_device
    @pytest.mark.integration
    @pytest.mark.critical
    def test_get_capabilities(self, device_client):
        """Test GetCapabilities with capability validation."""
        response = device_client.get_capabilities()
        validate_soap_format(response, "GetCapabilitiesResponse")

        capabilities = device_client.parse_capabilities(response)

        # Validate required capability categories
        required_categories = ['Device', 'Media']
        assert_required_fields(capabilities, required_categories)

        # Validate Device capabilities
        device_caps = capabilities['Device']
        assert 'XAddr' in device_caps, "Device XAddr required"
        assert device_caps['XAddr'].startswith('http'), "Invalid Device XAddr"

        # Validate Media capabilities
        media_caps = capabilities['Media']
        assert 'XAddr' in media_caps, "Media XAddr required"
        assert 'StreamingCapabilities' in media_caps

    @pytest.mark.onvif_device
    @pytest.mark.integration
    def test_get_system_date_time(self, device_client):
        """Test GetSystemDateAndTime with timezone validation."""
        response = device_client.get_system_date_time()
        validate_soap_format(response, "GetSystemDateAndTimeResponse")

        datetime_info = device_client.parse_system_date_time(response)

        # Validate datetime structure
        required_fields = ['DateTimeType', 'DaylightSavings', 'TimeZone']
        assert_required_fields(datetime_info, required_fields)

        # Validate datetime type
        assert datetime_info['DateTimeType'] in ['Manual', 'NTP']

        # Validate timezone format
        tz = datetime_info['TimeZone']
        assert 'TZ' in tz, "TimeZone must contain TZ field"

    @pytest.mark.onvif_device
    @pytest.mark.integration
    def test_get_services(self, device_client):
        """Test GetServices with service validation."""
        response = device_client.get_services(include_capability=False)
        validate_soap_format(response, "GetServicesResponse")

        services = device_client.parse_services(response)

        # Validate service list
        assert len(services) > 0, "At least one service required"

        for service in services:
            # Validate required service fields
            required_fields = ['Namespace', 'XAddr', 'Version']
            assert_required_fields(service, required_fields)

            # Validate service namespace format
            assert service['Namespace'].startswith('http://www.onvif.org')

            # Validate XAddr format
            assert service['XAddr'].startswith('http')

    @pytest.mark.onvif_device
    @pytest.mark.performance
    def test_device_operations_performance(self, device_client, device_operations):
        """Test performance of all device operations."""
        import time

        performance_results = {}

        for operation_name, operation_info in device_operations.items():
            if not operation_info['required']:
                continue

            times = []
            for _ in range(5):  # 5 iterations for average
                start_time = time.time()

                # Execute operation based on name
                if operation_name == 'GetDeviceInformation':
                    response = device_client.get_device_information()
                elif operation_name == 'GetCapabilities':
                    response = device_client.get_capabilities()
                elif operation_name == 'GetSystemDateAndTime':
                    response = device_client.get_system_date_time()

                end_time = time.time()
                times.append(end_time - start_time)

                # Validate response is successful
                assert response.status_code == 200

            avg_time = sum(times) / len(times)
            performance_results[operation_name] = {
                'avg_time': avg_time,
                'max_time': max(times),
                'min_time': min(times)
            }

            # Performance assertions
            assert avg_time < 2.0, f"{operation_name} too slow: {avg_time:.2f}s"

        # Log performance summary
        for op, stats in performance_results.items():
            print(f"{op}: avg={stats['avg_time']:.3f}s")

    @pytest.mark.onvif_device
    @pytest.mark.network
    def test_error_handling(self, device_client):
        """Test error handling for invalid operations."""
        # Test invalid SOAP action
        with pytest.raises(Exception) as exc_info:
            device_client.make_request("InvalidOperation", "<invalid/>")

        # Validate error format is SOAP fault
        error_response = str(exc_info.value)
        assert "soap:Fault" in error_response or "SOAP-ENV:Fault" in error_response

        # Test malformed request
        with pytest.raises(Exception):
            device_client.make_raw_request("not xml content")
```

## Test Utilities

### Enhanced SOAP Helpers

Create `utils/soap_helpers.py`:

```python
"""SOAP request/response helper utilities."""

import requests
import xml.etree.ElementTree as ET
from typing import Dict, Any, Optional

class SOAPHelper:
    """Helper class for SOAP operations."""

    @staticmethod
    def create_soap_envelope(body_content: str, namespaces: Dict[str, str] = None) -> str:
        """Create standard SOAP envelope with proper namespaces."""
        default_ns = {
            'soap': 'http://www.w3.org/2003/05/soap-envelope',
            'tds': 'http://www.onvif.org/ver10/device/wsdl'
        }

        if namespaces:
            default_ns.update(namespaces)

        ns_declarations = ' '.join([f'xmlns:{k}="{v}"' for k, v in default_ns.items()])

        return f'''<?xml version="1.0" encoding="UTF-8"?>
<soap:Envelope {ns_declarations}>
    <soap:Header/>
    <soap:Body>
        {body_content}
    </soap:Body>
</soap:Envelope>'''

    @staticmethod
    def parse_soap_response(response_text: str) -> ET.Element:
        """Parse SOAP response and return body element."""
        root = ET.fromstring(response_text)

        # Find SOAP body
        body = root.find('.//{http://www.w3.org/2003/05/soap-envelope}Body')
        if body is None:
            body = root.find('.//{http://schemas.xmlsoap.org/soap/envelope/}Body')

        if body is None:
            raise ValueError("No SOAP Body found in response")

        return body

def make_soap_request(url: str, soap_body: str, headers: Dict[str, str] = None) -> requests.Response:
    """Make SOAP request with proper error handling."""
    default_headers = {
        'Content-Type': 'application/soap+xml',
        'SOAPAction': '""',
    }

    if headers:
        default_headers.update(headers)

    try:
        response = requests.post(url, data=soap_body, headers=default_headers, timeout=30)
        response.raise_for_status()
        return response
    except requests.exceptions.RequestException as e:
        raise ConnectionError(f"SOAP request failed: {e}")
```

## Test Data Management

Create `utils/test_data_manager.py`:

```python
"""Test data setup and cleanup management."""

import json
import tempfile
from pathlib import Path
from datetime import datetime
from typing import Dict, Any, List

class TestDataManager:
    """Manages test data lifecycle with cleanup."""

    def __init__(self):
        self.test_data_dir = Path(tempfile.gettempdir()) / "onvif_test_data"
        self.test_data_dir.mkdir(exist_ok=True)
        self._created_files: List[Path] = []

    def create_device_test_data(self, device_name: str, data: Dict[str, Any]) -> Path:
        """Create test data file for device."""
        test_data = {
            'device_name': device_name,
            'created_at': datetime.now().isoformat(),
            'data': data
        }

        data_file = self.test_data_dir / f"{device_name}_test_data.json"
        with open(data_file, 'w') as f:
            json.dump(test_data, f, indent=2)

        self._created_files.append(data_file)
        return data_file

    def cleanup_test_data(self):
        """Clean up all created test data."""
        for file_path in self._created_files:
            try:
                if file_path.exists():
                    file_path.unlink()
            except OSError:
                pass  # Ignore cleanup failures

        self._created_files.clear()

    def __enter__(self):
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        self.cleanup_test_data()
```

## Validation Scripts

Create `scripts/validate_device_tests.py`:

```python
#!/usr/bin/env python3
"""Validate device test implementation."""

import subprocess
import sys
from pathlib import Path

def check_test_coverage():
    """Check device test coverage."""
    result = subprocess.run([
        "pytest", "tests/integration/test_device_service.py",
        "--cov=tests", "--cov-report=term-missing"
    ], capture_output=True, text=True)

    if "100%" not in result.stdout:
        print("❌ Device test coverage not 100%")
        return False

    print("✅ Device test coverage: 100%")
    return True

def check_required_operations():
    """Check all required operations are tested."""
    device_test_file = Path("tests/integration/test_device_service.py")

    if not device_test_file.exists():
        print("❌ Device test file missing")
        return False

    content = device_test_file.read_text()

    required_tests = [
        "test_get_device_information",
        "test_get_capabilities",
        "test_get_system_date_time",
        "test_get_services",
        "test_device_operations_performance",
        "test_error_handling"
    ]

    missing_tests = []
    for test_name in required_tests:
        if test_name not in content:
            missing_tests.append(test_name)

    if missing_tests:
        print(f"❌ Missing tests: {missing_tests}")
        return False

    print("✅ All required device tests present")
    return True

def main():
    """Run device test validation."""
    print("🔍 Validating device test implementation")

    checks = [
        check_required_operations,
        check_test_coverage
    ]

    passed = sum(1 for check in checks if check())
    total = len(checks)

    print(f"\n📊 Validation Results: {passed}/{total} passed")

    return 0 if passed == total else 1

if __name__ == "__main__":
    sys.exit(main())
```

## Implementation Checklist

- [ ] Create enhanced device fixtures in `fixtures/device_fixtures.py`
- [ ] Refactor `tests/integration/test_device_service.py` using new patterns
- [ ] Implement SOAP helpers in `utils/soap_helpers.py`
- [ ] Add test data manager in `utils/test_data_manager.py`
- [ ] Create validation script `scripts/validate_device_tests.py`
- [ ] Run validation to ensure 100% coverage
- [ ] Verify all tests pass with new structure

## Next Steps

After completing device test refactoring, proceed to [05_performance_tests.md](05_performance_tests.md) for enhanced performance testing implementation.

## Related Documentation

- **Previous**: [03_code_quality.md](03_code_quality.md) - Quality standards for refactoring
- **Next**: [05_performance_tests.md](05_performance_tests.md) - Performance testing framework
- **Issues Addressed**: [01_current_issues.md](01_current_issues.md#critical-issues) - Code duplication and missing operations
- **Templates**:
  - [templates/test_template.py](templates/test_template.py) - Test structure patterns
  - [templates/fixture_template.py](templates/fixture_template.py) - Fixture patterns
- **Directory Structure**: [02_directory_structure.md](02_directory_structure.md) - Where to place refactored tests