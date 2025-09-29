"""
Configuration for ONVIF crash fix E2E tests

This module provides specialized configuration for testing the crash fixes
implemented in Phase 1, 2, and 3 of the ONVIF service stability improvements.
"""

import os
from pathlib import Path
from typing import List, Dict, Any
from dataclasses import dataclass
from config import DeviceConfig, TestConfig


@dataclass
class CrashFixTestConfig:
    """Configuration for crash fix testing"""
    # Test device configuration
    device: DeviceConfig

    # Test parameters
    stress_test_duration: int = 60  # seconds
    concurrent_workers: int = 3
    rapid_request_count: int = 100
    memory_pressure_requests: int = 100

    # Performance thresholds
    max_avg_response_time: float = 5.0  # seconds
    max_response_time: float = 10.0  # seconds
    min_success_rate: float = 0.95  # 95%

    # Error handling thresholds
    max_crash_count: int = 0
    max_concurrent_failures: int = 2

    # Test data
    malformed_requests: List[str] = None

    def __post_init__(self):
        """Initialize default malformed requests if not provided"""
        if self.malformed_requests is None:
            self.malformed_requests = [
                # Missing namespace
                "<soap:Envelope><soap:Body><tds:GetCapabilities/></soap:Body></soap:Envelope>",
                # Invalid category
                "<soap:Envelope><soap:Body><tds:GetCapabilities><tds:Category>InvalidCategory</tds:Category></tds:GetCapabilities></soap:Body></soap:Envelope>",
                # Non-existent operation
                "<soap:Envelope><soap:Body><tds:NonExistentOperation/></soap:Body></soap:Envelope>",
                # Malformed XML
                "<soap:Envelope><soap:Body><tds:GetCapabilities><tds:Category></tds:GetCapabilities></soap:Body></soap:Envelope>",
                # Empty body
                "<soap:Envelope><soap:Body></soap:Body></soap:Envelope>",
            ]


def load_crash_fix_test_config() -> CrashFixTestConfig:
    """Load crash fix test configuration from environment variables"""

    # Get device configuration from main config
    from config import config
    device = config.get_default_device()
    if not device:
        raise ValueError("No test device configured")

    # Load test-specific parameters from environment
    stress_test_duration = int(os.getenv("CRASH_FIX_STRESS_DURATION", "60"))
    concurrent_workers = int(os.getenv("CRASH_FIX_CONCURRENT_WORKERS", "3"))
    rapid_request_count = int(os.getenv("CRASH_FIX_RAPID_REQUESTS", "100"))
    memory_pressure_requests = int(os.getenv("CRASH_FIX_MEMORY_PRESSURE", "100"))

    max_avg_response_time = float(os.getenv("CRASH_FIX_MAX_AVG_RESPONSE_TIME", "5.0"))
    max_response_time = float(os.getenv("CRASH_FIX_MAX_RESPONSE_TIME", "10.0"))
    min_success_rate = float(os.getenv("CRASH_FIX_MIN_SUCCESS_RATE", "0.95"))

    max_crash_count = int(os.getenv("CRASH_FIX_MAX_CRASH_COUNT", "0"))
    max_concurrent_failures = int(os.getenv("CRASH_FIX_MAX_CONCURRENT_FAILURES", "2"))

    return CrashFixTestConfig(
        device=device,
        stress_test_duration=stress_test_duration,
        concurrent_workers=concurrent_workers,
        rapid_request_count=rapid_request_count,
        memory_pressure_requests=memory_pressure_requests,
        max_avg_response_time=max_avg_response_time,
        max_response_time=max_response_time,
        min_success_rate=min_success_rate,
        max_crash_count=max_crash_count,
        max_concurrent_failures=max_concurrent_failures
    )


# Global configuration instance
crash_fix_config = load_crash_fix_test_config()


def get_test_markers() -> Dict[str, str]:
    """Get pytest markers for crash fix tests"""
    return {
        "crash_fixes": "Tests for ONVIF crash fixes and stability improvements",
        "critical": "Critical tests that must pass for service stability",
        "stress": "Stress tests that may take longer to run",
        "performance": "Performance-related tests",
        "slow": "Slow tests that may take several minutes",
    }


def get_test_categories() -> Dict[str, List[str]]:
    """Get test categories and their associated test methods"""
    return {
        "phase1_fixes": [
            "test_get_capabilities_no_crash",
        ],
        "phase2_fixes": [
            "test_xml_builder_error_handling",
            "test_error_response_validation",
        ],
        "phase3_fixes": [
            "test_buffer_overflow_protection",
        ],
        "stability_tests": [
            "test_concurrent_requests_stability",
            "test_memory_pressure_stability",
            "test_service_recovery_after_errors",
        ],
        "performance_tests": [
            "test_performance_impact",
            "test_extended_stress_test",
        ],
    }


def get_test_requirements() -> Dict[str, Any]:
    """Get test requirements and dependencies"""
    return {
        "python_version": ">=3.8",
        "dependencies": [
            "pytest>=6.0.0",
            "pytest-html>=2.0.0",
            "pytest-cov>=2.10.0",
            "requests>=2.25.0",
            "lxml>=4.6.0",
        ],
        "system_requirements": {
            "memory": ">=2GB",
            "cpu_cores": ">=2",
            "network": "Stable connection to test device",
        },
        "device_requirements": {
            "firmware": "Updated with crash fixes",
            "services": ["ONVIF Device Service", "ONVIF Media Service"],
            "capabilities": ["GetCapabilities", "GetDeviceInformation"],
        }
    }
