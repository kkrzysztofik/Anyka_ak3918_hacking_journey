"""
ONVIF Device service integration tests with enhanced logging
"""
import pytest
import logging
import time
from typing import Dict, Any
import xml.etree.ElementTree as ET

from .fixtures import (
    device_config,
    device_client,
    soap_client,
    make_soap_request,
    validate_xml_response,
    retry_on_failure,
    SOAP_ENVELOPE_TEMPLATE,
    ONVIF_NAMESPACES,
    performance_tracker,
    log_test_execution
)

# Get specialized loggers
test_logger = logging.getLogger('onvif.tests')
request_logger = logging.getLogger('onvif.requests')
response_logger = logging.getLogger('onvif.responses')
performance_logger = logging.getLogger('onvif.performance')


class TestONVIFDeviceService:
    """ONVIF Device service integration tests"""

    @pytest.mark.onvif_device
    @pytest.mark.integration
    @log_test_execution
    def test_get_device_information(self, device_client, device_config: 'DeviceConfig'):
        """Test GetDeviceInformation operation with detailed logging"""
        test_logger.info("Starting GetDeviceInformation test")

        # Start performance tracking
        performance_tracker.start_timer("get_device_information")

        try:
            # Create and send request
            test_logger.info("Creating GetDeviceInformation SOAP request")
            soap_request = device_client.create_get_device_information_request()

            test_logger.info(f"Sending request to: {device_client.device_url}")
            response = make_soap_request(
                device_client.device_url,
                soap_request,
                timeout=device_config.timeout
            )

            # Validate response
            test_logger.info("Validating XML response")
            content = validate_xml_response(response, 200)

            # Parse device information
            test_logger.info("Parsing device information from response")
            device_info = device_client.parse_device_information(content)

            test_logger.info(f"Parsed device information: {device_info}")

            # Verify required fields (case-insensitive)
            test_logger.info("Verifying required device information fields")
            required_fields = ['manufacturer', 'model', 'firmwareversion', 'serialnumber']

            # Create case-insensitive lookup
            device_info_lower = {k.lower(): v for k, v in device_info.items()}

            for field in required_fields:
                if field not in device_info_lower:
                    test_logger.error(f"Missing required field: {field}")
                    assert field in device_info_lower, f"{field} field missing"
                else:
                    test_logger.info(f"✓ {field}: {device_info_lower[field]}")

            test_logger.info("All required fields present - test passed")

        finally:
            # End performance tracking
            duration = performance_tracker.end_timer("get_device_information")
            performance_logger.info(f"GetDeviceInformation test completed in {duration:.3f}s")

    @pytest.mark.onvif_device
    @pytest.mark.integration
    def test_get_capabilities(self, device_client):
        """Test GetCapabilities operation"""
        # Create and send request
        soap_request = device_client.create_get_capabilities_request()

        response = make_soap_request(
            device_client.device_url,
            soap_request,
            timeout=30
        )

        # Validate response
        content = validate_xml_response(response, 200)

        # Parse capabilities
        capabilities = device_client.parse_capabilities(content)

        # Verify expected capabilities for a camera
        assert 'device' in capabilities, "Device capability missing"
        assert 'media' in capabilities, "Media capability missing"

    @pytest.mark.onvif_device
    @pytest.mark.integration
    def test_get_system_date_and_time(self, device_client):
        """Test GetSystemDateAndTime operation"""
        soap_request = device_client.create_get_system_date_and_time_request()

        response = make_soap_request(
            device_client.device_url,
            soap_request,
            timeout=30
        )

        # Validate response
        content = validate_xml_response(response, 200)

        # Basic validation - response should contain date/time elements
        assert 'UTCDateTime' in content or 'LocalDateTime' in content, "No date/time information found"

    @pytest.mark.onvif_device
    @pytest.mark.integration
    def test_get_services(self, device_client):
        """Test GetServices operation"""
        soap_request = device_client.create_get_services_request()

        response = make_soap_request(
            device_client.device_url,
            soap_request,
            timeout=30
        )

        # Validate response
        content = validate_xml_response(response, 200)

        # Verify service information is present
        assert 'Service' in content, "No service information found"

    @pytest.mark.onvif_device
    @pytest.mark.integration
    def test_get_network_interfaces(self, device_client):
        """Test GetNetworkInterfaces operation"""
        soap_request = device_client.create_get_network_interfaces_request()

        response = make_soap_request(
            device_client.device_url,
            soap_request,
            timeout=30
        )

        # Validate response
        content = validate_xml_response(response, 200)

        # Verify network interface information is present
        assert 'NetworkInterface' in content, "No network interface information found"

    @pytest.mark.onvif_device
    @pytest.mark.integration
    @retry_on_failure(max_attempts=3, delay=2.0)
    def test_device_service_availability(self, device_client):
        """Test that Device service is available and responding"""
        # Simple request to check service availability
        soap_request = device_client.create_get_device_information_request()

        response = make_soap_request(
            device_client.device_url,
            soap_request,
            timeout=30
        )

        # Should get a valid response
        assert response.status_code == 200, f"Service not available: {response.status_code}"

    @pytest.mark.onvif_device
    @pytest.mark.integration
    def test_device_service_error_handling(self, device_client):
        """Test error handling for invalid requests"""
        # Send invalid SOAP request
        invalid_soap = "<invalid>request</invalid>"

        response = make_soap_request(
            device_client.device_url,
            invalid_soap,
            timeout=30
        )

        # Should get an error response
        assert response.status_code in [400, 500], f"Expected error response, got {response.status_code}"

    @pytest.mark.onvif_device
    @pytest.mark.integration
    def test_device_information_consistency(self, device_client):
        """Test that device information is consistent across multiple requests"""
        results = []

        # Make multiple requests
        for i in range(3):
            soap_request = device_client.create_get_device_information_request()

            response = make_soap_request(
                device_client.device_url,
                soap_request,
                timeout=30
            )

            content = validate_xml_response(response, 200)
            device_info = device_client.parse_device_information(content)
            results.append(device_info)

        # All results should be identical
        for i in range(1, len(results)):
            assert results[i] == results[0], f"Device information inconsistent between requests {i} and 0"

    @pytest.mark.onvif_device
    @pytest.mark.integration
    def test_device_capabilities_completeness(self, device_client):
        """Test that device reports all expected capabilities"""
        soap_request = device_client.create_get_capabilities_request()

        response = make_soap_request(
            device_client.device_url,
            soap_request,
            timeout=30
        )

        content = validate_xml_response(response, 200)
        capabilities = device_client.parse_capabilities(content)

        # For a camera, these should typically be true
        assert capabilities.get('device', False), "Device capability should be true"
        assert capabilities.get('media', False), "Media capability should be true"

    @pytest.mark.onvif_device
    @pytest.mark.integration
    @pytest.mark.slow
    def test_device_service_performance(self, device_client):
        """Test device service response time"""
        import time

        response_times = []
        for i in range(5):
            start_time = time.time()

            soap_request = device_client.create_get_device_information_request()
            response = make_soap_request(
                device_client.device_url,
                soap_request,
                timeout=30
            )

            end_time = time.time()
            response_times.append(end_time - start_time)

        # Calculate statistics
        avg_time = sum(response_times) / len(response_times)
        max_time = max(response_times)

        # Assert reasonable performance
        assert avg_time < 2.0, f"Average response time too slow: {avg_time:.2f}s"
        assert max_time < 5.0, f"Maximum response time too slow: {max_time:.2f}s"
