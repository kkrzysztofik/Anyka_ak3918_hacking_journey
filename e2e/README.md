# ONVIF E2E Tests

This directory contains comprehensive end-to-end (E2E) tests for the Anyka AK3918 ONVIF implementation. The test suite validates WS-Discovery functionality and all ONVIF services (Device, Media, PTZ, Imaging) as well as RTSP streaming.

## Features

### Test Coverage
- **WS-Discovery**: Device discovery via multicast
- **ONVIF Device Service**: Device information and capabilities
- **ONVIF Media Service**: Video profiles and stream URIs
- **ONVIF PTZ Service**: Pan/Tilt/Zoom controls and presets
- **ONVIF Imaging Service**: Image settings and focus control
- **RTSP Streaming**: Video stream connectivity and quality
- **Crash Fix Tests**: Service stability and crash prevention validation

### Test Types
- **E2E Tests**: Full end-to-end testing with real camera
- **Performance Tests**: Response time and throughput validation
- **Security Tests**: Input validation and error handling
- **Smoke Tests**: Basic functionality verification

## Quick Start

### Prerequisites
1. Python 3.9+
2. Camera device running ONVIF daemon
3. Dependencies installed

### Installation
```bash
cd e2e
pip install -r requirements.txt
```

### Configuration
Edit `config.py` to specify your camera device:

```python
# Example configuration
devices = [
    DeviceConfig(
        name="camera1",
        ip_address="192.168.1.100",
        http_port=8080,
        rtsp_port=554,
        username="admin",
        password="admin"
    )
]
```

Or set environment variables:
```bash
export ONVIF_TEST_DEVICE_0_IP="192.168.1.100"
export ONVIF_TEST_DEVICE_0_PORT="8080"
export ONVIF_TEST_DEVICE_0_USERNAME="admin"
export ONVIF_TEST_DEVICE_0_PASSWORD="admin"
```

### Running Tests

#### Run All Tests
```bash
cd e2e
python -m pytest
```

#### Run Specific Test Categories
```bash
# WS-Discovery tests only
python -m pytest -m "ws_discovery"

# ONVIF Device service tests
python -m pytest -m "onvif_device"

# Media service tests
python -m pytest -m "onvif_media"

# PTZ service tests
python -m pytest -m "onvif_ptz"

# Imaging service tests
python -m pytest -m "onvif_imaging"

# RTSP streaming tests
python -m pytest -m "rtsp"

# Crash fix tests
python -m pytest -m "crash_fixes"

# Critical tests only
python -m pytest -m "critical"

# Phase-specific crash fix tests
python -m pytest -m "phase1_fixes"  # Phase 1: NULL pointer prevention
python -m pytest -m "phase2_fixes"  # Phase 2: Error handling
python -m pytest -m "phase3_fixes"  # Phase 3: Buffer overflow protection

# Stress tests
python -m pytest -m "stress"

# E2E tests (all)
python -m pytest -m "integration"
```

#### Run with Custom Device
```bash
python -m pytest --device-ip=192.168.1.100
```

#### Performance Testing
```bash
python -m pytest -m "not slow" --tb=short
```

#### Generate HTML Report
```bash
python -m pytest --html=reports/report.html --self-contained-html
```

#### Using the Integrated Test Runner
```bash
# Run all tests
python run_tests.py --all

# Run crash fix tests only
python run_tests.py --crash-fixes

# Run critical tests only
python run_tests.py --critical

# Run specific phase tests
python run_tests.py --phase 1  # Phase 1: NULL pointer prevention
python run_tests.py --phase 2  # Phase 2: Error handling
python run_tests.py --phase 3  # Phase 3: Buffer overflow protection

# Run stress tests
python run_tests.py --stress

# Run with custom device
python run_tests.py --crash-fixes --device-ip 192.168.1.100 --verbose

# Run with HTML report
python run_tests.py --crash-fixes --html-report
```

#### Using the Crash Fix Integration Script
```bash
# Check integration status
python integrate_crash_fix_tests.py status

# Run all crash fix tests
python integrate_crash_fix_tests.py run-all 192.168.1.100

# Run critical tests only
python integrate_crash_fix_tests.py run-critical --verbose

# Run specific phase tests
python integrate_crash_fix_tests.py run-phase1
python integrate_crash_fix_tests.py run-phase2
python integrate_crash_fix_tests.py run-phase3

# Run stress tests
python integrate_crash_fix_tests.py run-stress
```

## Test Structure

```
e2e/
├── config.py                    # Test configuration
├── test_crash_fixes_config.py   # Crash fix test configuration
├── integrate_crash_fix_tests.py # Crash fix test integration script
├── requirements.txt             # Python dependencies
├── pytest.ini                  # Pytest configuration
├── pytest_crash_fixes.ini      # Crash fix specific pytest config
├── README.md                    # This file
├── CRASH_FIX_TESTS.md          # Crash fix tests documentation
├── tests/
│   ├── __init__.py
│   ├── fixtures.py              # Test fixtures and utilities
│   ├── test_ws_discovery.py     # WS-Discovery tests
│   ├── test_onvif_device.py     # Device service tests
│   ├── test_onvif_media.py      # Media service tests
│   ├── test_onvif_ptz.py        # PTZ service tests
│   ├── test_onvif_imaging.py    # Imaging service tests
│   ├── test_rtsp_streaming.py   # RTSP streaming tests
│   └── test_crash_fixes.py      # Crash fix and stability tests
├── test_data/                   # Test data directory
├── reports/                     # Test reports and coverage
└── logs/                        # Test logs
```

## Test Categories and Markers

### Pytest Markers
- `slow`: Tests that take longer to run (deselect with `-m "not slow"`)
- `integration`: Full end-to-end scenarios requiring camera hardware
- `ws_discovery`: WS-Discovery specific tests
- `onvif_device`: ONVIF Device service tests
- `onvif_media`: ONVIF Media service tests
- `onvif_ptz`: ONVIF PTZ service tests
- `onvif_imaging`: ONVIF Imaging service tests
- `rtsp`: RTSP streaming tests
- `network`: Network-related tests
- `camera`: Camera hardware specific tests
- `crash_fixes`: ONVIF crash fix and stability tests
- `critical`: Critical tests that must pass for service stability
- `stress`: Stress tests that may take longer to run
- `performance`: Performance-related tests
- `phase1_fixes`: Phase 1 crash fix tests (NULL pointer prevention)
- `phase2_fixes`: Phase 2 crash fix tests (error handling)
- `phase3_fixes`: Phase 3 crash fix tests (buffer overflow protection)
- `stability`: Service stability tests

### Test Files

#### `test_ws_discovery.py`
- Device discovery via WS-Discovery protocol
- Multicast message handling
- Service announcement verification
- Network reachability tests

#### `test_onvif_device.py`
- GetDeviceInformation operation
- GetCapabilities operation
- GetSystemDateAndTime operation
- GetServices operation
- Device information consistency

#### `test_onvif_media.py`
- GetProfiles operation
- GetStreamUri operation
- GetVideoSources operation
- GetAudioSources operation
- Media profile validation

#### `test_onvif_ptz.py`
- GetNodes operation
- GetConfigurations operation
- ContinuousMove and Stop operations
- GetStatus operation
- PTZ position validation

#### `test_onvif_imaging.py`
- GetImagingSettings operation
- SetImagingSettings operation
- GetOptions operation
- Focus control operations
- Settings validation

#### `test_rtsp_streaming.py`
- Stream connectivity tests
- Frame quality validation
- Stream continuity tests
- Multi-stream testing
- Performance metrics

#### `test_crash_fixes.py`
- **Phase 1 Tests**: NULL pointer crash prevention validation
- **Phase 2 Tests**: XML builder error handling improvements
- **Phase 3 Tests**: Buffer overflow protection mechanisms
- **Stability Tests**: Service stability under concurrent load and memory pressure
- **Performance Tests**: Impact assessment of crash fixes on service performance
- **Stress Tests**: Extended stress testing for long-term stability validation

## Configuration Options

### Device Configuration
Configure your camera device in `config.py` or via environment variables:

```python
DeviceConfig(
    name="camera1",
    ip_address="192.168.1.100",
    http_port=8080,
    rtsp_port=554,
    username="admin",
    password="admin",
    enable_auth=False,
    timeout=30
)
```

### Environment Variables
- `ONVIF_TEST_DEVICE_COUNT`: Number of devices to test
- `ONVIF_TEST_DEVICE_{N}_NAME`: Device name
- `ONVIF_TEST_DEVICE_{N}_IP`: Device IP address
- `ONVIF_TEST_DEVICE_{N}_PORT`: HTTP port
- `ONVIF_TEST_DEVICE_{N}_USERNAME`: Authentication username
- `ONVIF_TEST_DEVICE_{N}_PASSWORD`: Authentication password

### Test Configuration
- `ONVIF_TEST_DISCOVERY_TIMEOUT`: WS-Discovery timeout (seconds)
- `ONVIF_TEST_SERVICE_TIMEOUT`: Service operation timeout (seconds)
- `ONVIF_TEST_STREAM_TIMEOUT`: RTSP stream timeout (seconds)
- `ONVIF_TEST_VIDEO_ANALYSIS`: Enable video analysis features
- `ONVIF_TEST_PERFORMANCE`: Enable performance testing
- `ONVIF_TEST_MAX_CONCURRENT`: Maximum concurrent tests

## Continuous Integration

The test suite includes GitHub Actions workflows for automated testing:

- **E2E Tests**: Full test suite on code changes
- **Smoke Tests**: Basic functionality verification
- **Performance Tests**: Performance validation
- **Security Tests**: Security analysis

### Running Tests in CI
```bash
# Build ONVIF daemon
docker run --rm -v ${PWD}:/workspace anyka-cross-compile make -C /workspace/onvif

# Run specific test category
python -m pytest -m "onvif_device" --tb=short
```

## Troubleshooting

### Common Issues

#### No Device Found
- Verify camera IP address and network connectivity
- Check ONVIF daemon is running on camera
- Verify firewall settings allow required ports

#### Connection Timeouts
- Increase timeout values in configuration
- Check network latency to camera
- Verify camera is not overloaded

#### Authentication Errors
- Verify username/password in configuration
- Check if camera requires authentication
- Try disabling authentication temporarily

#### RTSP Stream Issues
- Verify RTSP port is accessible
- Check camera video encoding settings
- Ensure sufficient network bandwidth

### Debug Mode
Enable debug logging for detailed troubleshooting:

```bash
export ONVIF_TEST_LOG_LEVEL=DEBUG
python -m pytest -v -s
```

### Manual Testing
Test individual components manually:

```python
from config import config
from tests.fixtures import wait_for_service

# Check if services are available
device = config.get_default_device()
print(f"Device URL: {device.get_onvif_device_url()}")

# Wait for service availability
if wait_for_service(device.get_onvif_device_url()):
    print("Device service is available")
```

## Contributing

### Adding New Tests
1. Create test file in `tests/` directory
2. Use appropriate pytest markers
3. Follow existing naming conventions
4. Include comprehensive docstrings
5. Add configuration options as needed

### Test Guidelines
- Use descriptive test names
- Include assertion messages
- Handle expected failures gracefully
- Test both positive and negative scenarios
- Verify error conditions and edge cases

## Dependencies

### Required Packages
- `pytest`: Test framework
- `requests`: HTTP client for ONVIF services
- `zeep`: SOAP client for ONVIF
- `opencv-python`: Video stream analysis
- `numpy`: Numerical computations
- `scapy`: Network packet analysis

### Optional Packages
- `pytest-html`: HTML test reports
- `pytest-cov`: Coverage reporting
- `pytest-xdist`: Parallel test execution
- `pytest-mock`: Test mocking utilities

Install all dependencies:
```bash
pip install -r requirements.txt
```

## License

This test suite is part of the Anyka AK3918 hacking journey project.
