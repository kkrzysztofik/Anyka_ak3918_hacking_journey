"""
Test fixtures and utilities for ONVIF E2E tests
"""
import asyncio
import pytest
import requests
import time
import logging
import json
from datetime import datetime
from typing import Generator, Optional, Dict, Any, Callable
from requests.auth import HTTPDigestAuth
from zeep import Client
from zeep.transports import Transport
from functools import wraps

from config import config, DeviceConfig
from typing import TYPE_CHECKING
import xml.etree.ElementTree as ET

if TYPE_CHECKING:
    from typing import Dict, Any

# Configure detailed logging for E2E tests
logging.basicConfig(
    level=logging.DEBUG,
    format='%(asctime)s [%(levelname)8s] %(name)s: %(message)s',
    datefmt='%Y-%m-%d %H:%M:%S'
)

# Create specialized loggers for different components
request_logger = logging.getLogger('onvif.requests')
response_logger = logging.getLogger('onvif.responses')
test_logger = logging.getLogger('onvif.tests')
performance_logger = logging.getLogger('onvif.performance')


def log_request_details(func: Callable) -> Callable:
    """
    Decorator to log detailed request information for SOAP requests

    Args:
        func: Function to decorate

    Returns:
        Decorated function with logging
    """
    @wraps(func)
    def wrapper(*args, **kwargs):
        # Extract request details from function arguments
        url = kwargs.get('url', args[0] if args else 'Unknown')
        soap_body = kwargs.get('soap_body', args[1] if len(args) > 1 else 'Unknown')
        headers = kwargs.get('headers', {})
        auth = kwargs.get('auth')
        timeout = kwargs.get('timeout', 30)

        # Log request details
        request_logger.info(f"=== SOAP REQUEST START ===")
        request_logger.info(f"URL: {url}")
        request_logger.info(f"Timeout: {timeout}s")
        request_logger.info(f"Auth: {'Yes' if auth else 'No'}")
        request_logger.info(f"Headers: {json.dumps(headers, indent=2)}")
        request_logger.debug(f"SOAP Body:\n{soap_body}")

        start_time = time.time()

        try:
            result = func(*args, **kwargs)
            end_time = time.time()
            duration = end_time - start_time

            # Log response details
            if hasattr(result, 'status_code'):
                response_logger.info(f"=== SOAP RESPONSE ===")
                response_logger.info(f"Status Code: {result.status_code}")
                response_logger.info(f"Duration: {duration:.3f}s")
                response_logger.info(f"Headers: {dict(result.headers)}")
                response_logger.debug(f"Response Content:\n{result.text}")

                # Log performance metrics
                performance_logger.info(f"Request completed in {duration:.3f}s")

                # Log warning for slow requests
                if duration > 5.0:
                    performance_logger.warning(f"Slow request detected: {duration:.3f}s")
                elif duration > 2.0:
                    performance_logger.info(f"Moderate request time: {duration:.3f}s")
                else:
                    performance_logger.debug(f"Fast request: {duration:.3f}s")

            return result

        except Exception as e:
            end_time = time.time()
            duration = end_time - start_time

            # Log error details
            request_logger.error(f"=== SOAP REQUEST FAILED ===")
            request_logger.error(f"URL: {url}")
            request_logger.error(f"Duration: {duration:.3f}s")
            request_logger.error(f"Error: {str(e)}")
            request_logger.error(f"Error Type: {type(e).__name__}")

            raise

    return wrapper


def log_test_execution(func: Callable) -> Callable:
    """
    Decorator to log test execution details

    Args:
        func: Test function to decorate

    Returns:
        Decorated function with logging
    """
    @wraps(func)
    def wrapper(*args, **kwargs):
        test_logger.info(f"=== TEST START: {func.__name__} ===")
        test_logger.info(f"Test Description: {func.__doc__ or 'No description'}")

        start_time = time.time()

        try:
            result = func(*args, **kwargs)
            end_time = time.time()
            duration = end_time - start_time

            test_logger.info(f"=== TEST PASSED: {func.__name__} ===")
            test_logger.info(f"Duration: {duration:.3f}s")

            return result

        except Exception as e:
            end_time = time.time()
            duration = end_time - start_time

            test_logger.error(f"=== TEST FAILED: {func.__name__} ===")
            test_logger.error(f"Duration: {duration:.3f}s")
            test_logger.error(f"Error: {str(e)}")
            test_logger.error(f"Error Type: {type(e).__name__}")

            raise

    return wrapper


def format_xml_for_logging(xml_content: str, max_length: int = 1000) -> str:
    """
    Format XML content for logging with proper indentation and length limits

    Args:
        xml_content: XML content to format
        max_length: Maximum length before truncation

    Returns:
        Formatted XML string
    """
    try:
        # Parse and pretty-print XML
        root = ET.fromstring(xml_content)
        ET.indent(root, space="  ", level=0)
        formatted = ET.tostring(root, encoding='unicode')

        # Truncate if too long
        if len(formatted) > max_length:
            formatted = formatted[:max_length] + "\n... [TRUNCATED]"

        return formatted
    except ET.ParseError:
        # If XML parsing fails, return truncated original
        if len(xml_content) > max_length:
            return xml_content[:max_length] + "\n... [TRUNCATED - INVALID XML]"
        return xml_content


def log_soap_envelope(envelope: str, direction: str = "REQUEST") -> None:
    """
    Log SOAP envelope with proper formatting

    Args:
        envelope: SOAP envelope content
        direction: Direction of the envelope (REQUEST/RESPONSE)
    """
    logger = request_logger if direction == "REQUEST" else response_logger

    logger.info(f"=== SOAP {direction} ENVELOPE ===")
    formatted_xml = format_xml_for_logging(envelope)
    logger.debug(f"SOAP Content:\n{formatted_xml}")


@pytest.fixture(scope="session")
def test_config():
    """Provide test configuration"""
    return config


@pytest.fixture(scope="session")
def device_config(test_config) -> DeviceConfig:
    """Provide device configuration for tests"""
    device = test_config.get_default_device()
    if not device:
        pytest.skip("No device configuration available")
    return device


class ONVIFDeviceClient:
    """ONVIF Device service client"""

    def __init__(self, device_url: str):
        self.device_url = device_url

    def create_get_device_information_request(self) -> str:
        """Create GetDeviceInformation SOAP request"""
        body = '''<wsdl:GetDeviceInformation xmlns:wsdl="http://www.onvif.org/ver10/device/wsdl"/>'''
        return SOAP_ENVELOPE_TEMPLATE.format(body=body)

    def create_get_capabilities_request(self) -> str:
        """Create GetCapabilities SOAP request"""
        body = '''<wsdl:GetCapabilities xmlns:wsdl="http://www.onvif.org/ver10/device/wsdl">
            <wsdl:Category>All</wsdl:Category>
        </wsdl:GetCapabilities>'''
        return SOAP_ENVELOPE_TEMPLATE.format(body=body)

    def create_get_system_date_and_time_request(self) -> str:
        """Create GetSystemDateAndTime SOAP request"""
        body = '''<wsdl:GetSystemDateAndTime xmlns:wsdl="http://www.onvif.org/ver10/device/wsdl"/>'''
        return SOAP_ENVELOPE_TEMPLATE.format(body=body)

    def create_get_services_request(self) -> str:
        """Create GetServices SOAP request"""
        body = '''<wsdl:GetServices xmlns:wsdl="http://www.onvif.org/ver10/device/wsdl">
            <wsdl:IncludeCapability>true</wsdl:IncludeCapability>
        </wsdl:GetServices>'''
        return SOAP_ENVELOPE_TEMPLATE.format(body=body)

    def create_get_network_interfaces_request(self) -> str:
        """Create GetNetworkInterfaces SOAP request"""
        body = '''<wsdl:GetNetworkInterfaces xmlns:wsdl="http://www.onvif.org/ver10/device/wsdl"/>'''
        return SOAP_ENVELOPE_TEMPLATE.format(body=body)

    def parse_device_information(self, response_xml: str) -> Dict[str, str]:
        """Parse GetDeviceInformation response"""
        try:
            root = ET.fromstring(response_xml)

            # Remove namespace prefixes for easier parsing
            for elem in root.iter():
                if '}' in elem.tag:
                    elem.tag = elem.tag.split('}', 1)[1]

            # Find the Body element
            body = root.find('Body')
            if body is None:
                raise ValueError("No Body element found in response")

            # Find GetDeviceInformationResponse
            response_elem = body.find('GetDeviceInformationResponse')
            if response_elem is None:
                raise ValueError("No GetDeviceInformationResponse found")

            # Extract device information
            device_info = {}
            for child in response_elem:
                device_info[child.tag] = child.text or ""

            return device_info

        except ET.ParseError as e:
            raise ValueError(f"Failed to parse XML response: {e}")

    def parse_capabilities(self, response_xml: str) -> Dict[str, bool]:
        """Parse GetCapabilities response"""
        try:
            root = ET.fromstring(response_xml)

            # Remove namespace prefixes for easier parsing
            for elem in root.iter():
                if '}' in elem.tag:
                    elem.tag = elem.tag.split('}', 1)[1]

            # Find the Body element
            body = root.find('Body')
            if body is None:
                raise ValueError("No Body element found in response")

            # Find GetCapabilitiesResponse
            response_elem = body.find('GetCapabilitiesResponse')
            if response_elem is None:
                raise ValueError("No GetCapabilitiesResponse found")

            # Find Capabilities element
            capabilities_elem = response_elem.find('Capabilities')
            if capabilities_elem is None:
                raise ValueError("No Capabilities element found")

            # Extract capabilities
            capabilities = {}
            for child in capabilities_elem:
                if child.tag in ['Device', 'Media', 'PTZ', 'Imaging', 'Events', 'Analytics']:
                    capabilities[child.tag.lower()] = True

            return capabilities

        except ET.ParseError as e:
            raise ValueError(f"Failed to parse XML response: {e}")


@pytest.fixture
def device_client(device_config: DeviceConfig) -> ONVIFDeviceClient:
    """Provide ONVIF Device service client"""
    return ONVIFDeviceClient(device_config.get_onvif_device_url())


@pytest.fixture(scope="session")
def event_loop():
    """Create an instance of the default event loop for the test session"""
    loop = asyncio.get_event_loop_policy().new_event_loop()
    yield loop
    loop.close()


@pytest.fixture
def http_session():
    """Provide an HTTP session for making requests"""
    session = requests.Session()
    yield session
    session.close()


@pytest.fixture
def authenticated_session(device_config: DeviceConfig):
    """Provide an authenticated HTTP session"""
    session = requests.Session()

    if device_config.enable_auth:
        session.auth = HTTPDigestAuth(device_config.username, device_config.password)

    yield session
    session.close()


@pytest.fixture
def soap_client(device_config: DeviceConfig):
    """Provide a SOAP client for ONVIF services"""
    # Create transport with authentication if needed
    transport = Transport(
        timeout=device_config.timeout,
        operation_timeout=device_config.timeout
    )

    # For now, we'll use basic HTTP client
    # In production, you might want to use zeep with proper WSDL
    yield {
        'transport': transport,
        'device_url': device_config.get_onvif_device_url(),
        'media_url': device_config.get_onvif_media_url(),
        'ptz_url': device_config.get_onvif_ptz_url(),
        'imaging_url': device_config.get_onvif_imaging_url()
    }


@pytest.fixture
def ws_discovery_config(device_config: DeviceConfig):
    """Provide WS-Discovery configuration"""
    return {
        'multicast_address': config.multicast_address,
        'multicast_port': device_config.ws_discovery_port,
        'timeout': config.discovery_timeout,
        'device_ip': device_config.ip_address
    }


def wait_for_service(url: str, timeout: int = 30) -> bool:
    """
    Wait for a service to become available

    Args:
        url: Service URL to check
        timeout: Maximum time to wait in seconds

    Returns:
        True if service is available, False otherwise
    """
    start_time = time.time()

    while time.time() - start_time < timeout:
        try:
            response = requests.get(url, timeout=5)
            if response.status_code == 200:
                return True
        except (requests.RequestException, ConnectionError):
            pass

        time.sleep(1)

    return False


@log_request_details
def make_soap_request(
    url: str,
    soap_body: str,
    headers: Optional[Dict[str, str]] = None,
    auth: Optional[HTTPDigestAuth] = None,
    timeout: int = 30
) -> requests.Response:
    """
    Make a SOAP request to an ONVIF service with detailed logging

    Args:
        url: Service endpoint URL
        soap_body: SOAP XML body
        headers: Optional HTTP headers
        auth: Optional authentication
        timeout: Request timeout in seconds

    Returns:
        Response object
    """
    default_headers = {
        'Content-Type': 'application/soap+xml; charset=utf-8',
        'SOAPAction': '""'
    }

    if headers:
        default_headers.update(headers)

    # Log SOAP envelope details
    log_soap_envelope(soap_body, "REQUEST")

    # Log additional request metadata
    request_logger.info(f"Request Method: POST")
    request_logger.info(f"Content-Length: {len(soap_body.encode('utf-8'))}")

    # Log authentication details (without credentials)
    if auth:
        request_logger.info(f"Authentication: HTTP Digest (username: {auth.username})")
    else:
        request_logger.info("Authentication: None")

    try:
        start_time = time.time()

        response = requests.post(
            url,
            data=soap_body,
            headers=default_headers,
            auth=auth,
            timeout=timeout
        )

        end_time = time.time()
        duration = end_time - start_time

        # Log response details
        response_logger.info(f"=== SOAP RESPONSE RECEIVED ===")
        response_logger.info(f"Status Code: {response.status_code}")
        response_logger.info(f"Response Time: {duration:.3f}s")
        response_logger.info(f"Content-Length: {len(response.content)}")
        response_logger.info(f"Content-Type: {response.headers.get('Content-Type', 'Unknown')}")

        # Log response headers
        response_logger.debug(f"Response Headers: {dict(response.headers)}")

        # Log response content (formatted XML)
        log_soap_envelope(response.text, "RESPONSE")

        # Log performance metrics
        performance_logger.info(f"SOAP request completed in {duration:.3f}s")

        # Performance warnings
        if duration > 10.0:
            performance_logger.warning(f"Very slow SOAP request: {duration:.3f}s")
        elif duration > 5.0:
            performance_logger.warning(f"Slow SOAP request: {duration:.3f}s")
        elif duration > 2.0:
            performance_logger.info(f"Moderate SOAP request time: {duration:.3f}s")
        else:
            performance_logger.debug(f"Fast SOAP request: {duration:.3f}s")

        # Log HTTP status code analysis
        if response.status_code == 200:
            response_logger.info("Request successful (200 OK)")
        elif response.status_code == 400:
            response_logger.warning("Bad Request (400) - Check SOAP format")
        elif response.status_code == 401:
            response_logger.warning("Unauthorized (401) - Check authentication")
        elif response.status_code == 404:
            response_logger.warning("Not Found (404) - Check service endpoint")
        elif response.status_code == 500:
            response_logger.error("Internal Server Error (500) - Server issue")
        else:
            response_logger.warning(f"Unexpected status code: {response.status_code}")

        return response

    except requests.Timeout as e:
        request_logger.error(f"SOAP request timeout after {timeout}s: {e}")
        pytest.fail(f"SOAP request timeout to {url} after {timeout}s")
    except requests.ConnectionError as e:
        request_logger.error(f"SOAP request connection error: {e}")
        pytest.fail(f"Failed to connect to {url}: {e}")
    except requests.RequestException as e:
        request_logger.error(f"SOAP request failed: {e}")
        pytest.fail(f"Failed to make SOAP request to {url}: {e}")
    except Exception as e:
        request_logger.error(f"Unexpected error during SOAP request: {e}")
        pytest.fail(f"Unexpected error during SOAP request to {url}: {e}")


def validate_xml_response(response: requests.Response, expected_status: int = 200) -> str:
    """
    Validate and extract content from XML response with detailed logging

    Args:
        response: HTTP response object
        expected_status: Expected HTTP status code

    Returns:
        Response content as string

    Raises:
        pytest.fail if validation fails
    """
    response_logger.info(f"=== VALIDATING XML RESPONSE ===")
    response_logger.info(f"Expected Status: {expected_status}")
    response_logger.info(f"Actual Status: {response.status_code}")

    if response.status_code != expected_status:
        response_logger.error(f"Status code mismatch: expected {expected_status}, got {response.status_code}")
        response_logger.error(f"Response content: {response.text[:500]}...")
        pytest.fail(
            f"Expected status {expected_status}, got {response.status_code}. "
            f"Response: {response.text}"
        )

    content = response.text
    response_logger.info(f"Content length: {len(content)} characters")

    if not content.strip():
        response_logger.error("Empty response content detected")
        pytest.fail("Empty response content")

    # Validate XML structure
    try:
        root = ET.fromstring(content)
        response_logger.info("XML structure validation: PASSED")
        response_logger.debug(f"Root element: {root.tag}")
    except ET.ParseError as e:
        response_logger.error(f"XML parsing failed: {e}")
        response_logger.error(f"Invalid XML content: {content[:200]}...")
        pytest.fail(f"Invalid XML response: {e}")

    response_logger.info("XML response validation: PASSED")
    return content


def retry_on_failure(max_attempts: int = 3, delay: float = 1.0):
    """
    Decorator to retry a test function on failure with detailed logging

    Args:
        max_attempts: Maximum number of attempts
        delay: Delay between attempts in seconds
    """
    def decorator(func):
        @wraps(func)
        def wrapper(*args, **kwargs):
            last_exception = None
            test_logger.info(f"Starting retry-enabled test: {func.__name__}")
            test_logger.info(f"Max attempts: {max_attempts}, Delay: {delay}s")

            for attempt in range(max_attempts):
                attempt_num = attempt + 1
                test_logger.info(f"Attempt {attempt_num}/{max_attempts} for {func.__name__}")

                try:
                    start_time = time.time()
                    result = func(*args, **kwargs)
                    end_time = time.time()
                    duration = end_time - start_time

                    test_logger.info(f"Attempt {attempt_num} succeeded in {duration:.3f}s")
                    return result

                except Exception as e:
                    last_exception = e
                    end_time = time.time()
                    duration = end_time - start_time

                    test_logger.warning(f"Attempt {attempt_num} failed after {duration:.3f}s: {str(e)}")
                    test_logger.debug(f"Exception type: {type(e).__name__}")

                    if attempt < max_attempts - 1:
                        test_logger.info(f"Retrying in {delay}s...")
                        time.sleep(delay)
                        continue
                    else:
                        test_logger.error(f"All {max_attempts} attempts failed for {func.__name__}")
                        test_logger.error(f"Final error: {str(last_exception)}")
                        pytest.fail(
                            f"Function {func.__name__} failed after {max_attempts} attempts. "
                            f"Last error: {str(last_exception)}"
                        )

            return None  # Should never reach here
        return wrapper
    return decorator


class PerformanceTracker:
    """Track and log performance metrics for test operations"""

    def __init__(self):
        self.metrics = {}
        self.start_times = {}

    def start_timer(self, operation: str) -> None:
        """Start timing an operation"""
        self.start_times[operation] = time.time()
        performance_logger.debug(f"Started timing: {operation}")

    def end_timer(self, operation: str) -> float:
        """End timing an operation and return duration"""
        if operation not in self.start_times:
            performance_logger.warning(f"No start time found for operation: {operation}")
            return 0.0

        duration = time.time() - self.start_times[operation]
        del self.start_times[operation]

        if operation not in self.metrics:
            self.metrics[operation] = []

        self.metrics[operation].append(duration)
        performance_logger.info(f"Operation '{operation}' completed in {duration:.3f}s")

        return duration

    def get_stats(self, operation: str) -> Dict[str, float]:
        """Get performance statistics for an operation"""
        if operation not in self.metrics or not self.metrics[operation]:
            return {}

        times = self.metrics[operation]
        return {
            'count': len(times),
            'min': min(times),
            'max': max(times),
            'avg': sum(times) / len(times),
            'total': sum(times)
        }

    def log_summary(self) -> None:
        """Log performance summary for all tracked operations"""
        performance_logger.info("=== PERFORMANCE SUMMARY ===")
        for operation, times in self.metrics.items():
            stats = self.get_stats(operation)
            performance_logger.info(
                f"{operation}: {stats['count']} calls, "
                f"avg: {stats['avg']:.3f}s, "
                f"min: {stats['min']:.3f}s, "
                f"max: {stats['max']:.3f}s, "
                f"total: {stats['total']:.3f}s"
            )


# Global performance tracker instance
performance_tracker = PerformanceTracker()


# Common SOAP envelope templates
SOAP_ENVELOPE_TEMPLATE = """<?xml version="1.0" encoding="UTF-8"?>
<soapenv:Envelope xmlns:soapenv="http://schemas.xmlsoap.org/soap/envelope/"
                  xmlns:wsdl="http://www.onvif.org/ver10/device/wsdl">
    <soapenv:Header/>
    <soapenv:Body>
        {body}
    </soapenv:Body>
</soapenv:Envelope>"""

# ONVIF namespace prefixes
ONVIF_NAMESPACES = {
    'device': 'http://www.onvif.org/ver10/device/wsdl',
    'media': 'http://www.onvif.org/ver10/media/wsdl',
    'ptz': 'http://www.onvif.org/ver20/ptz/wsdl',
    'imaging': 'http://www.onvif.org/ver20/imaging/wsdl',
    'tt': 'http://www.onvif.org/ver10/schema'
}
