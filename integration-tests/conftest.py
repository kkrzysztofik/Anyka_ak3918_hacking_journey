"""
Pytest configuration and shared fixtures for ONVIF integration tests
"""
import pytest
import logging
from datetime import datetime
from config import config, DeviceConfig
from tests.fixtures import (
    ONVIFDeviceClient,
    make_soap_request,
    validate_xml_response,
    retry_on_failure,
    SOAP_ENVELOPE_TEMPLATE,
    ONVIF_NAMESPACES,
    performance_tracker
)
from logging_config import setup_test_logging, log_test_session_start, log_test_session_end


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


@pytest.fixture
def device_client(device_config: DeviceConfig) -> ONVIFDeviceClient:
    """Provide ONVIF Device service client"""
    return ONVIFDeviceClient(device_config.get_onvif_device_url())


@pytest.fixture(scope="session", autouse=True)
def setup_logging():
    """Set up logging for the test session"""
    setup_test_logging(log_level="DEBUG", log_dir="reports")
    yield
    # Log session end
    performance_tracker.log_summary()


@pytest.fixture(scope="session", autouse=True)
def test_session_tracking():
    """Track test session execution"""
    session_name = f"ONVIF_Integration_Tests_{datetime.now().strftime('%Y%m%d_%H%M%S')}"
    log_test_session_start(session_name)

    yield session_name

    # Log session end with basic results
    results = {
        "session_name": session_name,
        "end_time": datetime.now().isoformat()
    }
    log_test_session_end(session_name, results)


@pytest.fixture(autouse=True)
def test_execution_logging(request):
    """Log each test execution with detailed information"""
    test_logger = logging.getLogger('onvif.tests')

    # Log test start
    test_logger.info(f"=== TEST START: {request.node.name} ===")
    test_logger.info(f"Test file: {request.node.fspath}")
    test_logger.info(f"Test class: {request.cls.__name__ if request.cls else 'N/A'}")
    test_logger.info(f"Test function: {request.function.__name__ if request.function else 'N/A'}")
    test_logger.info(f"Test docstring: {request.function.__doc__ if request.function and request.function.__doc__ else 'No description'}")

    start_time = datetime.now()

    yield

    # Log test end
    end_time = datetime.now()
    duration = (end_time - start_time).total_seconds()

    test_logger.info(f"=== TEST END: {request.node.name} ===")
    test_logger.info(f"Duration: {duration:.3f}s")
    test_logger.info(f"Status: {'PASSED' if request.node.rep_call.passed else 'FAILED'}")

    if not request.node.rep_call.passed:
        test_logger.error(f"Test failed: {request.node.rep_call.longrepr}")


@pytest.hookimpl(tryfirst=True, hookwrapper=True)
def pytest_runtest_makereport(item, call):
    """Hook to capture test results for logging"""
    outcome = yield
    rep = outcome.get_result()
    setattr(item, "rep_" + rep.when, rep)
