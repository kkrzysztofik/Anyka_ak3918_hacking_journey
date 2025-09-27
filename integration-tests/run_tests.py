#!/usr/bin/env python3
"""
Test runner script for ONVIF integration tests
"""
import argparse
import sys
import os
from pathlib import Path

# Add current directory to Python path
sys.path.insert(0, str(Path(__file__).parent))

from config import load_config_from_env, config


def parse_arguments():
    """Parse command line arguments"""
    parser = argparse.ArgumentParser(
        description="Run ONVIF integration tests",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  python run_tests.py --device-ip 192.168.1.100
  python run_tests.py -m ws_discovery --verbose
  python run_tests.py --all --html-report
  python run_tests.py --performance --timeout 60
  python run_tests.py --crash-fixes --verbose
  python run_tests.py --critical --device-ip 192.168.1.100
  python run_tests.py --phase 1 --html-report
  python run_tests.py --stress --timeout 120
        """
    )

    parser.add_argument(
        '--device-ip',
        help='Camera device IP address'
    )

    parser.add_argument(
        '--device-port',
        type=int,
        default=8080,
        help='Camera HTTP port (default: 8080)'
    )

    parser.add_argument(
        '--username',
        default='admin',
        help='Camera username (default: admin)'
    )

    parser.add_argument(
        '--password',
        default='admin',
        help='Camera password (default: admin)'
    )

    parser.add_argument(
        '-m', '--markers',
        help='Test markers to run (e.g., "ws_discovery", "onvif_device", "all")'
    )

    parser.add_argument(
        '--all',
        action='store_true',
        help='Run all integration tests'
    )

    parser.add_argument(
        '--performance',
        action='store_true',
        help='Run performance tests'
    )

    parser.add_argument(
        '--smoke',
        action='store_true',
        help='Run smoke tests only'
    )

    parser.add_argument(
        '--crash-fixes',
        action='store_true',
        help='Run crash fix tests only'
    )

    parser.add_argument(
        '--critical',
        action='store_true',
        help='Run critical tests only'
    )

    parser.add_argument(
        '--phase',
        choices=['1', '2', '3', 'all'],
        help='Run tests for specific crash fix phase'
    )

    parser.add_argument(
        '--stress',
        action='store_true',
        help='Run stress tests'
    )

    parser.add_argument(
        '--html-report',
        action='store_true',
        help='Generate HTML report'
    )

    parser.add_argument(
        '--coverage',
        action='store_true',
        help='Generate coverage report'
    )

    parser.add_argument(
        '--verbose', '-v',
        action='store_true',
        help='Verbose output'
    )

    parser.add_argument(
        '--debug',
        action='store_true',
        help='Debug output'
    )

    parser.add_argument(
        '--timeout',
        type=int,
        default=30,
        help='Test timeout in seconds (default: 30)'
    )

    parser.add_argument(
        '--retry-attempts',
        type=int,
        default=3,
        help='Number of retry attempts (default: 3)'
    )

    return parser.parse_args()


def configure_environment(args):
    """Configure environment variables from arguments"""
    if args.device_ip:
        os.environ['ONVIF_TEST_DEVICE_0_IP'] = args.device_ip

    if args.device_port:
        os.environ['ONVIF_TEST_DEVICE_0_PORT'] = str(args.device_port)

    if args.username:
        os.environ['ONVIF_TEST_DEVICE_0_USERNAME'] = args.username

    if args.password:
        os.environ['ONVIF_TEST_DEVICE_0_PASSWORD'] = args.password

    if args.timeout:
        os.environ['ONVIF_TEST_SERVICE_TIMEOUT'] = str(args.timeout)

    if args.retry_attempts:
        os.environ['ONVIF_TEST_RETRY_ATTEMPTS'] = str(args.retry_attempts)

    # Set log level
    if args.debug:
        os.environ['ONVIF_TEST_LOG_LEVEL'] = 'DEBUG'
    elif args.verbose:
        os.environ['ONVIF_TEST_LOG_LEVEL'] = 'INFO'


def build_pytest_args(args):
    """Build pytest command arguments"""
    pytest_args = ['-m', 'integration']

    # Add test markers
    if args.markers:
        pytest_args = ['-m', args.markers]
    elif args.all:
        pytest_args = ['-m', 'integration']
    elif args.performance:
        pytest_args = ['-m', 'not slow']
    elif args.crash_fixes:
        pytest_args = ['-m', 'crash_fixes']
    elif args.critical:
        pytest_args = ['-m', 'critical']
    elif args.phase:
        if args.phase == 'all':
            pytest_args = ['-m', 'crash_fixes']
        else:
            pytest_args = ['-m', f'phase{args.phase}_fixes']
    elif args.stress:
        pytest_args = ['-m', 'stress']
    elif args.smoke:
        pytest_args = [
            'tests/test_ws_discovery.py::TestWSDiscovery::test_device_responds_to_probe',
            'tests/test_onvif_device.py::TestONVIFDeviceService::test_get_device_information',
            'tests/test_onvif_device.py::TestONVIFDeviceService::test_get_capabilities'
        ]

    # Add output options
    if args.verbose:
        pytest_args.append('-v')

    if args.debug:
        pytest_args.append('-v')
        pytest_args.append('-s')

    # Add reporting options
    if args.html_report:
        pytest_args.extend(['--html=reports/report.html', '--self-contained-html'])

    if args.coverage:
        pytest_args.extend(['--cov=tests', '--cov-report=html:reports/coverage'])

    # Add common options
    pytest_args.extend([
        '--tb=short',
        '--durations=10',
        '--maxfail=5'
    ])

    return pytest_args


def main():
    """Main entry point"""
    args = parse_arguments()

    # Configure environment
    configure_environment(args)

    # Reload configuration with environment variables
    global config
    config = load_config_from_env()

    # Display configuration
    print("ONVIF Integration Test Configuration:")
    print(f"  Device: {config.get_default_device().name} at {config.get_default_device().ip_address}:{config.get_default_device().http_port}")
    print(f"  Test timeout: {config.service_timeout}s")
    print(f"  Retry attempts: {config.retry_attempts}")
    print()

    # Build pytest command
    pytest_args = build_pytest_args(args)

    print("Running tests with arguments:")
    print(f"  python -m pytest {' '.join(pytest_args)}")
    print()

    # Import pytest and run tests
    try:
        import pytest
        exit_code = pytest.main(pytest_args)
        sys.exit(exit_code)
    except ImportError as e:
        print(f"Error: {e}")
        print("Please install requirements:")
        print("  pip install -r requirements.txt")
        sys.exit(1)


if __name__ == '__main__':
    main()
