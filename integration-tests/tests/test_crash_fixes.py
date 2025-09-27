"""
Phase 4 Integration Tests for ONVIF Crash Fixes

This module contains comprehensive tests to validate the crash fixes implemented
in Phase 1, 2, and 3 of the ONVIF service stability improvements.

Tests cover:
- NULL pointer crash prevention
- XML builder error handling
- Buffer overflow protection
- Error response validation
- Service stability under stress
"""

import pytest
import logging
import time
import threading
import concurrent.futures
from typing import Dict, Any, List
import xml.etree.ElementTree as ET
from dataclasses import dataclass

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
test_logger = logging.getLogger('onvif.tests.crash_fixes')
request_logger = logging.getLogger('onvif.requests')
response_logger = logging.getLogger('onvif.responses')
performance_logger = logging.getLogger('onvif.performance')


@dataclass
class StressTestResult:
    """Results from stress testing"""
    total_requests: int
    successful_requests: int
    failed_requests: int
    error_responses: int
    crash_count: int
    average_response_time: float
    max_response_time: float
    min_response_time: float


class TestONVIFCrashFixes:
    """Integration tests for ONVIF crash fixes and stability improvements"""

    @pytest.mark.crash_fixes
    @pytest.mark.integration
    @pytest.mark.critical
    @log_test_execution
    def test_get_capabilities_no_crash(self, device_client, device_config: 'DeviceConfig'):
        """Test that GetCapabilities no longer crashes with NULL pointer dereference"""
        test_logger.info("Testing GetCapabilities crash fix - Phase 1")

        # Start performance tracking
        performance_tracker.start_timer("get_capabilities_crash_fix")

        try:
            # Create GetCapabilities request
            test_logger.info("Creating GetCapabilities SOAP request")
            soap_request = device_client.create_get_capabilities_request()

            test_logger.info(f"Sending request to: {device_client.device_url}")
            response = make_soap_request(
                device_client.device_url,
                soap_request,
                timeout=device_config.timeout
            )

            # Validate response
            test_logger.info("Validating GetCapabilities response")
            assert response is not None, "Response should not be None"
            assert response.status_code == 200, f"Expected status 200, got {response.status_code}"

            # Parse and validate XML content
            root = ET.fromstring(response.text)
            assert root is not None, "XML response should be parseable"

            # Check for proper SOAP structure
            soap_envelope = root.find('.//{http://schemas.xmlsoap.org/soap/envelope/}Envelope')
            assert soap_envelope is not None, "Response should contain SOAP envelope"

            # Check for capabilities response
            capabilities = root.find('.//{http://www.onvif.org/ver10/device/wsdl}Capabilities')
            assert capabilities is not None, "Response should contain Capabilities element"

            # Verify no crash occurred (service should still be responsive)
            test_logger.info("Verifying service is still responsive after GetCapabilities")
            health_response = make_soap_request(
                device_client.device_url,
                device_client.create_get_device_information_request(),
                timeout=5
            )
            assert health_response is not None, "Service should still be responsive"
            assert health_response.status_code == 200, "Service should return 200 after GetCapabilities"

            performance_tracker.end_timer("get_capabilities_crash_fix")
            test_logger.info("✅ GetCapabilities crash fix test passed")

        except Exception as e:
            performance_tracker.end_timer("get_capabilities_crash_fix")
            test_logger.error(f"❌ GetCapabilities crash fix test failed: {e}")
            raise

    @pytest.mark.crash_fixes
    @pytest.mark.integration
    @pytest.mark.critical
    @log_test_execution
    def test_xml_builder_error_handling(self, device_client, device_config: 'DeviceConfig'):
        """Test XML builder error handling improvements - Phase 2"""
        test_logger.info("Testing XML builder error handling - Phase 2")

        # Test all device service functions that use XML builder
        test_functions = [
            ("GetCapabilities", device_client.create_get_capabilities_request),
            ("GetDeviceInformation", device_client.create_get_device_information_request),
            ("GetSystemDateAndTime", device_client.create_get_system_date_time_request),
            ("GetServices", device_client.create_get_services_request),
        ]

        for function_name, request_creator in test_functions:
            test_logger.info(f"Testing {function_name} error handling")

            try:
                # Create and send request
                soap_request = request_creator()
                response = make_soap_request(
                    device_client.device_url,
                    soap_request,
                    timeout=device_config.timeout
                )

                # Validate response structure
                assert response is not None, f"{function_name} response should not be None"
                assert response.status_code in [200, 500], f"{function_name} should return 200 or 500, got {response.status_code}"

                # If it's an error response, validate it's a proper SOAP fault
                if response.status_code == 500:
                    test_logger.info(f"{function_name} returned error response - validating SOAP fault structure")
                    root = ET.fromstring(response.text)
                    fault = root.find('.//{http://schemas.xmlsoap.org/soap/envelope/}Fault')
                    assert fault is not None, f"{function_name} error response should contain SOAP fault"
                else:
                    # Success response should be valid XML
                    root = ET.fromstring(response.text)
                    assert root is not None, f"{function_name} success response should be valid XML"

                test_logger.info(f"✅ {function_name} error handling test passed")

            except Exception as e:
                test_logger.error(f"❌ {function_name} error handling test failed: {e}")
                raise

    @pytest.mark.crash_fixes
    @pytest.mark.integration
    @pytest.mark.critical
    @log_test_execution
    def test_buffer_overflow_protection(self, device_client, device_config: 'DeviceConfig'):
        """Test buffer overflow protection - Phase 3"""
        test_logger.info("Testing buffer overflow protection - Phase 3")

        # Test with multiple rapid requests to stress the buffer
        test_logger.info("Sending multiple rapid GetCapabilities requests")

        responses = []
        for i in range(10):  # Send 10 rapid requests
            try:
                soap_request = device_client.create_get_capabilities_request()
                response = make_soap_request(
                    device_client.device_url,
                    soap_request,
                    timeout=device_config.timeout
                )
                responses.append(response)
                test_logger.debug(f"Request {i+1}/10 completed with status {response.status_code}")

                # Small delay to avoid overwhelming the service
                time.sleep(0.1)

            except Exception as e:
                test_logger.error(f"Request {i+1}/10 failed: {e}")
                raise

        # Validate all responses
        test_logger.info("Validating all responses")
        for i, response in enumerate(responses):
            assert response is not None, f"Response {i+1} should not be None"
            assert response.status_code == 200, f"Response {i+1} should return 200, got {response.status_code}"

            # Validate XML structure
            root = ET.fromstring(response.text)
            assert root is not None, f"Response {i+1} should be valid XML"

        test_logger.info("✅ Buffer overflow protection test passed")

    @pytest.mark.crash_fixes
    @pytest.mark.integration
    @pytest.mark.stress
    @log_test_execution
    def test_concurrent_requests_stability(self, device_client, device_config: 'DeviceConfig'):
        """Test service stability under concurrent requests"""
        test_logger.info("Testing concurrent requests stability")

        def make_request(request_id: int) -> Dict[str, Any]:
            """Make a single request and return results"""
            try:
                soap_request = device_client.create_get_capabilities_request()
                start_time = time.time()
                response = make_soap_request(
                    device_client.device_url,
                    soap_request,
                    timeout=device_config.timeout
                )
                end_time = time.time()

                return {
                    'request_id': request_id,
                    'success': True,
                    'status_code': response.status_code,
                    'response_time': end_time - start_time,
                    'error': None
                }
            except Exception as e:
                return {
                    'request_id': request_id,
                    'success': False,
                    'status_code': None,
                    'response_time': None,
                    'error': str(e)
                }

        # Send 20 concurrent requests
        test_logger.info("Sending 20 concurrent GetCapabilities requests")
        with concurrent.futures.ThreadPoolExecutor(max_workers=10) as executor:
            futures = [executor.submit(make_request, i) for i in range(20)]
            results = [future.result() for future in concurrent.futures.as_completed(futures)]

        # Analyze results
        successful_requests = [r for r in results if r['success']]
        failed_requests = [r for r in results if not r['success']]

        test_logger.info(f"Concurrent test results: {len(successful_requests)} successful, {len(failed_requests)} failed")

        # Validate results
        assert len(successful_requests) >= 18, f"Expected at least 18 successful requests, got {len(successful_requests)}"
        assert len(failed_requests) <= 2, f"Expected at most 2 failed requests, got {len(failed_requests)}"

        # Check that no crashes occurred (all failures should be network/timeout related)
        for failed_request in failed_requests:
            assert "crash" not in failed_request['error'].lower(), f"Request {failed_request['request_id']} failed due to crash: {failed_request['error']}"

        test_logger.info("✅ Concurrent requests stability test passed")

    @pytest.mark.crash_fixes
    @pytest.mark.integration
    @pytest.mark.stress
    @log_test_execution
    def test_memory_pressure_stability(self, device_client, device_config: 'DeviceConfig'):
        """Test service stability under memory pressure"""
        test_logger.info("Testing memory pressure stability")

        # Send many requests to create memory pressure
        test_logger.info("Sending 100 requests to create memory pressure")

        responses = []
        for i in range(100):
            try:
                soap_request = device_client.create_get_capabilities_request()
                response = make_soap_request(
                    device_client.device_url,
                    soap_request,
                    timeout=device_config.timeout
                )
                responses.append(response)

                if i % 20 == 0:
                    test_logger.info(f"Completed {i+1}/100 requests")

            except Exception as e:
                test_logger.error(f"Request {i+1}/100 failed: {e}")
                # Allow some failures due to memory pressure, but not crashes
                if "crash" in str(e).lower() or "segmentation" in str(e).lower():
                    raise
                continue

        # Validate that we got reasonable number of successful responses
        successful_responses = [r for r in responses if r and r.status_code == 200]
        test_logger.info(f"Memory pressure test: {len(successful_responses)}/{len(responses)} successful responses")

        assert len(successful_responses) >= 80, f"Expected at least 80 successful responses, got {len(successful_responses)}"

        test_logger.info("✅ Memory pressure stability test passed")

    @pytest.mark.crash_fixes
    @pytest.mark.integration
    @pytest.mark.critical
    @log_test_execution
    def test_error_response_validation(self, device_client, device_config: 'DeviceConfig'):
        """Test that error responses are properly formatted SOAP faults"""
        test_logger.info("Testing error response validation")

        # Test with malformed requests to trigger error responses
        malformed_requests = [
            "<soap:Envelope><soap:Body><tds:GetCapabilities/></soap:Body></soap:Envelope>",  # Missing namespace
            "<soap:Envelope><soap:Body><tds:GetCapabilities><tds:Category>InvalidCategory</tds:Category></tds:GetCapabilities></soap:Body></soap:Envelope>",  # Invalid category
            "<soap:Envelope><soap:Body><tds:NonExistentOperation/></soap:Body></soap:Envelope>",  # Non-existent operation
        ]

        for i, malformed_request in enumerate(malformed_requests):
            test_logger.info(f"Testing malformed request {i+1}/3")

            try:
                response = make_soap_request(
                    device_client.device_url,
                    malformed_request,
                    timeout=device_config.timeout
                )

                # Should get an error response
                assert response is not None, f"Malformed request {i+1} should get a response"
                assert response.status_code in [400, 500], f"Malformed request {i+1} should return 400 or 500, got {response.status_code}"

                # Validate SOAP fault structure
                root = ET.fromstring(response.text)
                fault = root.find('.//{http://schemas.xmlsoap.org/soap/envelope/}Fault')
                assert fault is not None, f"Malformed request {i+1} should return SOAP fault"

                # Check fault code
                fault_code = fault.find('.//{http://schemas.xmlsoap.org/soap/envelope/}faultcode')
                assert fault_code is not None, f"SOAP fault should contain faultcode"

                test_logger.info(f"✅ Malformed request {i+1} handled correctly")

            except Exception as e:
                test_logger.error(f"❌ Malformed request {i+1} test failed: {e}")
                raise

        test_logger.info("✅ Error response validation test passed")

    @pytest.mark.crash_fixes
    @pytest.mark.integration
    @pytest.mark.performance
    @log_test_execution
    def test_performance_impact(self, device_client, device_config: 'DeviceConfig'):
        """Test that crash fixes don't significantly impact performance"""
        test_logger.info("Testing performance impact of crash fixes")

        # Measure response times for multiple requests
        response_times = []

        for i in range(20):
            start_time = time.time()

            soap_request = device_client.create_get_capabilities_request()
            response = make_soap_request(
                device_client.device_url,
                soap_request,
                timeout=device_config.timeout
            )

            end_time = time.time()
            response_time = end_time - start_time
            response_times.append(response_time)

            assert response.status_code == 200, f"Request {i+1} should succeed"
            test_logger.debug(f"Request {i+1}/20: {response_time:.3f}s")

        # Calculate performance metrics
        avg_response_time = sum(response_times) / len(response_times)
        max_response_time = max(response_times)
        min_response_time = min(response_times)

        test_logger.info(f"Performance metrics: avg={avg_response_time:.3f}s, max={max_response_time:.3f}s, min={min_response_time:.3f}s")

        # Performance should be reasonable (less than 5 seconds average)
        assert avg_response_time < 5.0, f"Average response time {avg_response_time:.3f}s is too high"
        assert max_response_time < 10.0, f"Max response time {max_response_time:.3f}s is too high"

        test_logger.info("✅ Performance impact test passed")

    @pytest.mark.crash_fixes
    @pytest.mark.integration
    @pytest.mark.critical
    @log_test_execution
    def test_service_recovery_after_errors(self, device_client, device_config: 'DeviceConfig'):
        """Test that service recovers properly after errors"""
        test_logger.info("Testing service recovery after errors")

        # First, verify service is working
        test_logger.info("Verifying initial service health")
        soap_request = device_client.create_get_device_information_request()
        response = make_soap_request(
            device_client.device_url,
            soap_request,
            timeout=device_config.timeout
        )
        assert response.status_code == 200, "Initial service health check should pass"

        # Send some requests that might cause errors
        test_logger.info("Sending potentially problematic requests")
        for i in range(5):
            try:
                # Send malformed request
                malformed_request = "<soap:Envelope><soap:Body><tds:GetCapabilities><tds:Category>Invalid</tds:Category></tds:GetCapabilities></soap:Body></soap:Envelope>"
                error_response = make_soap_request(
                    device_client.device_url,
                    malformed_request,
                    timeout=device_config.timeout
                )
                test_logger.debug(f"Error request {i+1}/5 returned status {error_response.status_code}")
            except Exception as e:
                test_logger.debug(f"Error request {i+1}/5 failed: {e}")

        # Verify service is still working after errors
        test_logger.info("Verifying service recovery")
        recovery_response = make_soap_request(
            device_client.device_url,
            device_client.create_get_device_information_request(),
            timeout=device_config.timeout
        )

        assert recovery_response is not None, "Service should still be responsive after errors"
        assert recovery_response.status_code == 200, f"Service should return 200 after errors, got {recovery_response.status_code}"

        # Verify we can still get capabilities
        capabilities_response = make_soap_request(
            device_client.device_url,
            device_client.create_get_capabilities_request(),
            timeout=device_config.timeout
        )
        assert capabilities_response.status_code == 200, "GetCapabilities should still work after errors"

        test_logger.info("✅ Service recovery test passed")


class TestONVIFStressTesting:
    """Stress testing for ONVIF service stability"""

    @pytest.mark.crash_fixes
    @pytest.mark.stress
    @pytest.mark.slow
    @log_test_execution
    def test_extended_stress_test(self, device_client, device_config: 'DeviceConfig'):
        """Extended stress test to validate long-term stability"""
        test_logger.info("Starting extended stress test")

        def stress_worker(worker_id: int, duration_seconds: int = 60) -> StressTestResult:
            """Worker function for stress testing"""
            start_time = time.time()
            results = []

            while time.time() - start_time < duration_seconds:
                try:
                    request_start = time.time()
                    soap_request = device_client.create_get_capabilities_request()
                    response = make_soap_request(
                        device_client.device_url,
                        soap_request,
                        timeout=device_config.timeout
                    )
                    request_end = time.time()

                    results.append({
                        'success': True,
                        'status_code': response.status_code,
                        'response_time': request_end - request_start,
                        'is_error_response': response.status_code != 200
                    })

                except Exception as e:
                    results.append({
                        'success': False,
                        'status_code': None,
                        'response_time': None,
                        'is_error_response': False,
                        'error': str(e)
                    })

                # Small delay to avoid overwhelming
                time.sleep(0.1)

            # Calculate metrics
            total_requests = len(results)
            successful_requests = len([r for r in results if r['success']])
            failed_requests = total_requests - successful_requests
            error_responses = len([r for r in results if r.get('is_error_response', False)])
            crash_count = len([r for r in results if not r['success'] and 'crash' in str(r.get('error', '')).lower()])

            response_times = [r['response_time'] for r in results if r['response_time'] is not None]
            if response_times:
                avg_response_time = sum(response_times) / len(response_times)
                max_response_time = max(response_times)
                min_response_time = min(response_times)
            else:
                avg_response_time = max_response_time = min_response_time = 0.0

            return StressTestResult(
                total_requests=total_requests,
                successful_requests=successful_requests,
                failed_requests=failed_requests,
                error_responses=error_responses,
                crash_count=crash_count,
                average_response_time=avg_response_time,
                max_response_time=max_response_time,
                min_response_time=min_response_time
            )

        # Run stress test with multiple workers
        test_logger.info("Running stress test with 3 workers for 60 seconds each")
        with concurrent.futures.ThreadPoolExecutor(max_workers=3) as executor:
            futures = [executor.submit(stress_worker, i, 60) for i in range(3)]
            results = [future.result() for future in concurrent.futures.as_completed(futures)]

        # Analyze combined results
        total_requests = sum(r.total_requests for r in results)
        total_successful = sum(r.successful_requests for r in results)
        total_failed = sum(r.failed_requests for r in results)
        total_crashes = sum(r.crash_count for r in results)

        test_logger.info(f"Stress test results:")
        test_logger.info(f"  Total requests: {total_requests}")
        test_logger.info(f"  Successful: {total_successful}")
        test_logger.info(f"  Failed: {total_failed}")
        test_logger.info(f"  Crashes: {total_crashes}")

        # Validate results
        success_rate = total_successful / total_requests if total_requests > 0 else 0
        assert success_rate >= 0.95, f"Success rate {success_rate:.2%} is too low (expected >= 95%)"
        assert total_crashes == 0, f"Expected 0 crashes, got {total_crashes}"

        test_logger.info("✅ Extended stress test passed")
