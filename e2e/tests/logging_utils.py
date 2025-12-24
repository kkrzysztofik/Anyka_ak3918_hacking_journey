"""
Enhanced logging utilities for ONVIF E2E tests
"""
import logging
import time
import json
from datetime import datetime
from typing import Dict, Any, Optional, List
from functools import wraps
import xml.etree.ElementTree as ET


class TestLoggingUtils:
    """Utility class for enhanced test logging"""

    def __init__(self):
        self.test_logger = logging.getLogger('onvif.tests')
        self.request_logger = logging.getLogger('onvif.requests')
        self.response_logger = logging.getLogger('onvif.responses')
        self.performance_logger = logging.getLogger('onvif.performance')
        self.test_metrics = {}

    def log_test_start(self, test_name: str, test_description: str = None) -> None:
        """Log the start of a test with detailed information"""
        self.test_logger.info("=" * 60)
        self.test_logger.info(f"TEST START: {test_name}")
        self.test_logger.info(f"Description: {test_description or 'No description'}")
        self.test_logger.info(f"Timestamp: {datetime.now().isoformat()}")
        self.test_logger.info("=" * 60)

    def log_test_end(self, test_name: str, success: bool, duration: float, error: str = None) -> None:
        """Log the end of a test with results"""
        status = "PASSED" if success else "FAILED"
        self.test_logger.info("=" * 60)
        self.test_logger.info(f"TEST END: {test_name}")
        self.test_logger.info(f"Status: {status}")
        self.test_logger.info(f"Duration: {duration:.3f}s")
        if error:
            self.test_logger.error(f"Error: {error}")
        self.test_logger.info("=" * 60)

    def log_soap_request(self, url: str, soap_body: str, headers: Dict[str, str] = None,
                        auth: bool = False, timeout: int = 30) -> None:
        """Log detailed SOAP request information"""
        self.request_logger.info("=== SOAP REQUEST ===")
        self.request_logger.info(f"URL: {url}")
        self.request_logger.info("Method: POST")
        self.request_logger.info(f"Timeout: {timeout}s")
        self.request_logger.info(f"Authentication: {'Yes' if auth else 'No'}")
        self.request_logger.info(f"Content-Length: {len(soap_body.encode('utf-8'))}")

        if headers:
            self.request_logger.info(f"Headers: {json.dumps(headers, indent=2)}")

        # Log SOAP body with formatting
        formatted_soap = self._format_xml_for_logging(soap_body)
        self.request_logger.debug(f"SOAP Body:\n{formatted_soap}")

    def log_soap_response(self, response, duration: float) -> None:
        """Log detailed SOAP response information"""
        self.response_logger.info("=== SOAP RESPONSE ===")
        self.response_logger.info(f"Status Code: {response.status_code}")
        self.response_logger.info(f"Response Time: {duration:.3f}s")
        self.response_logger.info(f"Content-Length: {len(response.content)}")
        self.response_logger.info(f"Content-Type: {response.headers.get('Content-Type', 'Unknown')}")

        # Log response headers
        self.response_logger.debug(f"Response Headers: {dict(response.headers)}")

        # Log response content with formatting
        formatted_response = self._format_xml_for_logging(response.text)
        self.response_logger.debug(f"Response Content:\n{formatted_response}")

        # Log performance analysis
        self._log_performance_analysis(duration)

    def log_performance_metric(self, operation: str, duration: float,
                              additional_info: Dict[str, Any] = None) -> None:
        """Log performance metrics for an operation"""
        self.performance_logger.info(f"Performance: {operation} completed in {duration:.3f}s")

        if additional_info:
            for key, value in additional_info.items():
                self.performance_logger.info(f"  {key}: {value}")

        # Store metrics for summary
        if operation not in self.test_metrics:
            self.test_metrics[operation] = []
        self.test_metrics[operation].append(duration)

    def log_error(self, operation: str, error: Exception, context: str = None) -> None:
        """Log detailed error information"""
        self.test_logger.error(f"ERROR in {operation}: {str(error)}")
        self.test_logger.error(f"Error Type: {type(error).__name__}")
        if context:
            self.test_logger.error(f"Context: {context}")

        # Log stack trace for debugging
        import traceback
        self.test_logger.debug(f"Stack trace:\n{traceback.format_exc()}")

    def log_assertion(self, assertion_name: str, success: bool, expected: Any = None,
                     actual: Any = None) -> None:
        """Log assertion results with details"""
        status = "PASSED" if success else "FAILED"
        self.test_logger.info(f"Assertion {status}: {assertion_name}")

        if not success and expected is not None and actual is not None:
            self.test_logger.error(f"  Expected: {expected}")
            self.test_logger.error(f"  Actual: {actual}")

    def log_test_data(self, data_name: str, data: Any, max_length: int = 500) -> None:
        """Log test data with length limits"""
        data_str = str(data)
        if len(data_str) > max_length:
            data_str = data_str[:max_length] + "... [TRUNCATED]"

        self.test_logger.debug(f"Test Data - {data_name}: {data_str}")

    def _format_xml_for_logging(self, xml_content: str, max_length: int = 1000) -> str:
        """Format XML content for logging with proper indentation"""
        try:
            root = ET.fromstring(xml_content)
            ET.indent(root, space="  ", level=0)
            formatted = ET.tostring(root, encoding='unicode')

            if len(formatted) > max_length:
                formatted = formatted[:max_length] + "\n... [TRUNCATED]"

            return formatted
        except ET.ParseError:
            if len(xml_content) > max_length:
                return xml_content[:max_length] + "\n... [TRUNCATED - INVALID XML]"
            return xml_content

    def _log_performance_analysis(self, duration: float) -> None:
        """Log performance analysis based on duration"""
        if duration > 10.0:
            self.performance_logger.warning(f"Very slow response: {duration:.3f}s")
        elif duration > 5.0:
            self.performance_logger.warning(f"Slow response: {duration:.3f}s")
        elif duration > 2.0:
            self.performance_logger.info(f"Moderate response time: {duration:.3f}s")
        else:
            self.performance_logger.debug(f"Fast response: {duration:.3f}s")

    def get_performance_summary(self) -> Dict[str, Dict[str, float]]:
        """Get performance summary for all tracked operations"""
        summary = {}
        for operation, times in self.test_metrics.items():
            if times:
                summary[operation] = {
                    'count': len(times),
                    'min': min(times),
                    'max': max(times),
                    'avg': sum(times) / len(times),
                    'total': sum(times)
                }
        return summary

    def log_performance_summary(self) -> None:
        """Log performance summary for all tracked operations"""
        summary = self.get_performance_summary()
        self.performance_logger.info("=== PERFORMANCE SUMMARY ===")
        for operation, stats in summary.items():
            self.performance_logger.info(
                f"{operation}: {stats['count']} calls, "
                f"avg: {stats['avg']:.3f}s, "
                f"min: {stats['min']:.3f}s, "
                f"max: {stats['max']:.3f}s, "
                f"total: {stats['total']:.3f}s"
            )


# Global instance for easy access
test_logging_utils = TestLoggingUtils()


def log_test_execution(func):
    """Decorator to automatically log test execution"""
    @wraps(func)
    def wrapper(*args, **kwargs):
        test_name = func.__name__
        test_description = func.__doc__

        test_logging_utils.log_test_start(test_name, test_description)
        start_time = time.time()

        try:
            result = func(*args, **kwargs)
            end_time = time.time()
            duration = end_time - start_time

            test_logging_utils.log_test_end(test_name, True, duration)
            return result

        except Exception as e:
            end_time = time.time()
            duration = end_time - start_time

            test_logging_utils.log_error(test_name, e)
            test_logging_utils.log_test_end(test_name, False, duration, str(e))
            raise

    return wrapper


def log_soap_operation(operation_name: str):
    """Decorator to log SOAP operations with timing"""
    def decorator(func):
        @wraps(func)
        def wrapper(*args, **kwargs):
            test_logging_utils.test_logger.info(f"Starting SOAP operation: {operation_name}")

            start_time = time.time()
            try:
                result = func(*args, **kwargs)
                end_time = time.time()
                duration = end_time - start_time

                test_logging_utils.log_performance_metric(operation_name, duration)
                return result

            except Exception as e:
                end_time = time.time()
                duration = end_time - start_time

                test_logging_utils.log_error(operation_name, e)
                test_logging_utils.log_performance_metric(operation_name, duration, {"error": str(e)})
                raise

        return wrapper
    return decorator
