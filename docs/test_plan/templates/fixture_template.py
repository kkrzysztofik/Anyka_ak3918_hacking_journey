"""
Template for test fixtures - agents can copy and modify this template.

This template provides standard patterns for:
- Device connection fixtures
- Configuration management
- Test data setup/cleanup
- Performance measurement fixtures
- Mock fixtures for unit tests
"""

import pytest
import tempfile
import json
from pathlib import Path
from dataclasses import dataclass
from typing import Dict, Any, Optional, List
from utils.test_data_manager import TestDataManager

# Configuration fixtures

@dataclass
class ServiceConfigTemplate:
    """Template for service-specific configuration."""
    name: str
    ip_address: str
    http_port: int = 8080
    rtsp_port: int = 554
    username: str = "admin"
    password: str = "admin"
    timeout: int = 30

    # Service-specific expected values - modify for your service
    expected_service_field: str = "expected_value"
    expected_capability: str = "capability_name"

    # Performance thresholds - adjust for your service
    max_response_time: float = 2.0
    max_operation_time: float = 5.0
    min_success_rate: float = 95.0

    def __post_init__(self):
        """Validate configuration after initialization."""
        if not self.name or not self.name.strip():
            raise ValueError("Service name cannot be empty")

        if not self.ip_address or not self.ip_address.strip():
            raise ValueError("IP address cannot be empty")

        if self.http_port <= 0 or self.http_port > 65535:
            raise ValueError(f"Invalid HTTP port: {self.http_port}")

@pytest.fixture(scope="session")
def service_config():
    """Provide service configuration for all tests."""
    return ServiceConfigTemplate(
        name="test_service",
        ip_address="192.168.1.100"  # Replace with actual test device IP
    )

# Device client fixtures

@pytest.fixture(scope="session")
def device_client(service_config):
    """Create device client with connection validation."""
    from utils.device_client import ONVIFDeviceClient  # Replace with actual client

    client = ONVIFDeviceClient(service_config)

    # Validate connection works
    try:
        response = client.get_device_information()
        assert response.status_code == 200, "Device connection failed"
    except Exception as e:
        pytest.skip(f"Cannot connect to device: {e}")

    return client

@pytest.fixture(scope="function")
def fresh_client(service_config):
    """Create a fresh client for tests that need isolation."""
    from utils.device_client import ONVIFDeviceClient  # Replace with actual client

    client = ONVIFDeviceClient(service_config)

    # Validate connection
    response = client.get_device_information()
    assert response.status_code == 200, "Fresh client connection failed"

    return client

# SOAP request fixtures

@pytest.fixture
def soap_headers():
    """Standard SOAP headers for all requests."""
    return {
        'Content-Type': 'application/soap+xml',
        'SOAPAction': '""',
        'User-Agent': 'ONVIF Test Client'
    }

@pytest.fixture
def soap_namespaces():
    """Standard SOAP namespaces for service operations."""
    return {
        'soap': 'http://www.w3.org/2003/05/soap-envelope',
        'tds': 'http://www.onvif.org/ver10/device/wsdl',
        'trt': 'http://www.onvif.org/ver10/media/wsdl',
        'tptz': 'http://www.onvif.org/ver20/ptz/wsdl',
        'timg': 'http://www.onvif.org/ver20/imaging/wsdl',
        # Add more namespaces as needed for specific services
    }

# Service operations fixtures

@pytest.fixture
def service_operations():
    """Map of service operations to test - modify for your service."""
    return {
        'GetServiceInformation': {  # Replace with actual operation
            'required': True,
            'profile_s': True,
            'profile_t': True,
            'response_fields': ['Field1', 'Field2', 'Field3'],  # Replace with actual fields
            'max_response_time': 2.0
        },
        'GetServiceCapabilities': {  # Replace with actual operation
            'required': True,
            'profile_s': True,
            'profile_t': True,
            'response_fields': ['Capability1', 'Capability2'],  # Replace with actual fields
            'max_response_time': 1.0
        },
        'SetServiceConfiguration': {  # Replace with actual operation
            'required': False,
            'profile_s': False,
            'profile_t': True,
            'response_fields': [],
            'max_response_time': 3.0
        }
    }

# Test data management fixtures

@pytest.fixture(scope="function")
def test_data_manager():
    """Provide test data manager with automatic cleanup."""
    manager = TestDataManager()

    yield manager

    # Automatic cleanup
    manager.cleanup_test_data()

@pytest.fixture
def sample_test_data():
    """Provide sample test data for operations - modify for your service."""
    return {
        'test_item_1': {
            'name': 'Test Item 1',
            'property1': 'value1',
            'property2': 42,
            'property3': True
        },
        'test_item_2': {
            'name': 'Test Item 2',
            'property1': 'value2',
            'property2': 100,
            'property3': False
        },
        'invalid_item': {
            'name': '',  # Invalid empty name
            'property1': 'x' * 1000,  # Too long value
            'property2': -1  # Invalid negative value
        }
    }

# Configuration backup/restore fixtures

@pytest.fixture
def backup_service_config(device_client):
    """Backup and restore service configuration - modify for your service."""
    try:
        # Backup original configuration
        original_config = device_client.get_service_configuration()  # Replace with actual method
    except Exception as e:
        pytest.skip(f"Cannot backup service configuration: {e}")

    yield original_config

    # Restore original configuration
    try:
        device_client.set_service_configuration(original_config)  # Replace with actual method
    except Exception as e:
        print(f"Warning: Could not restore service configuration: {e}")

# Performance measurement fixtures

@pytest.fixture
def performance_thresholds(service_config):
    """Provide performance thresholds for testing."""
    return {
        'max_avg_response_time': service_config.max_response_time,
        'max_p95_response_time': service_config.max_response_time * 1.5,
        'min_success_rate': service_config.min_success_rate,
        'max_std_dev': 1.0,
        'min_throughput': 5.0  # operations per second
    }

@pytest.fixture
def performance_collector():
    """Provide performance metrics collector."""
    from utils.performance_metrics import PerformanceCollector

    collector = PerformanceCollector("TestOperation")  # Replace with actual operation name

    yield collector

    # Log final performance summary
    if collector.metrics.response_times:
        print(f"Performance Summary: avg={collector.metrics.avg_response_time:.3f}s, "
              f"success_rate={collector.metrics.success_rate:.1f}%")

# Mock fixtures for unit testing

@pytest.fixture
def mock_device_response():
    """Mock device response for unit tests - modify for your service."""
    return """<?xml version="1.0" encoding="UTF-8"?>
<soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope">
    <soap:Body>
        <tds:GetServiceInformationResponse xmlns:tds="http://www.onvif.org/ver10/device/wsdl">
            <tds:ServiceInformation>
                <tds:Field1>Test Value 1</tds:Field1>
                <tds:Field2>Test Value 2</tds:Field2>
                <tds:Field3>Test Value 3</tds:Field3>
            </tds:ServiceInformation>
        </tds:GetServiceInformationResponse>
    </soap:Body>
</soap:Envelope>"""

@pytest.fixture
def mock_error_response():
    """Mock error response for error handling tests."""
    return """<?xml version="1.0" encoding="UTF-8"?>
<soap:Envelope xmlns:soap="http://www.w3.org/2003/05/soap-envelope">
    <soap:Body>
        <soap:Fault>
            <soap:Code>
                <soap:Value>soap:Sender</soap:Value>
                <soap:Subcode>
                    <soap:Value>ter:InvalidArgVal</soap:Value>
                </soap:Subcode>
            </soap:Code>
            <soap:Reason>
                <soap:Text xml:lang="en">Invalid argument value</soap:Text>
            </soap:Reason>
        </soap:Fault>
    </soap:Body>
</soap:Envelope>"""

@pytest.fixture
def mock_device_client(mock_device_response, mock_error_response):
    """Mock device client for unit tests."""
    from unittest.mock import Mock
    import requests

    client = Mock()

    # Mock successful response
    success_response = Mock(spec=requests.Response)
    success_response.status_code = 200
    success_response.text = mock_device_response
    success_response.content = mock_device_response.encode()

    # Mock error response
    error_response = Mock(spec=requests.Response)
    error_response.status_code = 500
    error_response.text = mock_error_response
    error_response.content = mock_error_response.encode()

    # Configure mock methods - replace with actual methods for your service
    client.get_service_information.return_value = success_response
    client.get_service_capabilities.return_value = success_response
    client.invalid_operation.side_effect = Exception("Invalid operation")

    return client

# Validation fixtures

@pytest.fixture
def compliance_validator(device_client):
    """Provide compliance validator for testing - modify for your profile."""
    from utils.onvif_validator import ONVIFProfileSValidator  # Or ProfileTValidator

    validator = ONVIFProfileSValidator(device_client)
    return validator

# Cleanup fixtures

@pytest.fixture
def cleanup_created_items(device_client):
    """Track and cleanup items created during tests."""
    created_items = []

    yield created_items

    # Cleanup any created test items
    for item_id in created_items:
        try:
            device_client.delete_item(item_id)  # Replace with actual cleanup method
            print(f"Cleaned up test item: {item_id}")
        except Exception as e:
            print(f"Cleanup warning: Could not delete {item_id}: {e}")

# Environment-specific fixtures

@pytest.fixture
def skip_if_not_supported(device_client, feature_name):
    """Skip test if feature is not supported by device."""
    try:
        capabilities = device_client.get_capabilities()
        parsed_caps = device_client.parse_capabilities(capabilities)

        # Check if feature is supported - modify logic for your feature
        if feature_name not in parsed_caps:
            pytest.skip(f"Feature {feature_name} not supported by device")

    except Exception as e:
        pytest.skip(f"Cannot determine feature support: {e}")

# Usage Instructions for Agents:
#
# 1. Copy this template file to the fixtures directory
# 2. Rename to match your service (e.g., media_fixtures.py)
# 3. Update ServiceConfigTemplate with service-specific fields
# 4. Replace placeholder method calls with actual client methods
# 5. Modify mock responses to match your service's SOAP responses
# 6. Update service_operations fixture with your service's operations
# 7. Add service-specific fixtures as needed
# 8. Import and use fixtures in your test files
# 9. Test fixtures work correctly before using in tests
# 10. Add any additional fixtures specific to your service requirements