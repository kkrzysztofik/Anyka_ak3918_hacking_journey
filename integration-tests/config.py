"""
Configuration for ONVIF integration tests
"""
import os
from dataclasses import dataclass
from typing import Optional, List, Dict, Any
from pathlib import Path
from dotenv import load_dotenv

# Load environment variables from .env file
load_dotenv()


@dataclass
class DeviceConfig:
    """Configuration for a single camera device under test"""
    name: str
    ip_address: str
    http_port: int = 8080
    rtsp_port: int = 554
    ws_discovery_port: int = 3702
    username: str = "admin"
    password: str = "admin"
    device_type: str = "camera"
    expected_manufacturer: str = "Anyka"
    expected_model: str = "AK3918 Camera"
    enable_auth: bool = False
    timeout: int = 30

    def get_base_url(self) -> str:
        """Get the base HTTP URL for the device"""
        protocol = "https" if self.http_port == 443 else "http"
        return f"{protocol}://{self.ip_address}:{self.http_port}"

    def get_onvif_device_url(self) -> str:
        """Get the ONVIF Device service URL"""
        return f"{self.get_base_url()}/onvif/device_service"

    def get_onvif_media_url(self) -> str:
        """Get the ONVIF Media service URL"""
        return f"{self.get_base_url()}/onvif/media_service"

    def get_onvif_ptz_url(self) -> str:
        """Get the ONVIF PTZ service URL"""
        return f"{self.get_base_url()}/onvif/ptz_service"

    def get_onvif_imaging_url(self) -> str:
        """Get the ONVIF Imaging service URL"""
        return f"{self.get_base_url()}/onvif/imaging_service"

    def get_rtsp_main_url(self) -> str:
        """Get the main RTSP stream URL"""
        return f"rtsp://{self.ip_address}:{self.rtsp_port}/vs0"

    def get_rtsp_sub_url(self) -> str:
        """Get the sub RTSP stream URL"""
        return f"rtsp://{self.ip_address}:{self.rtsp_port}/vs1"

    def get_snapshot_url(self) -> str:
        """Get the snapshot URL"""
        return f"{self.get_base_url()}/snapshot.bmp"


@dataclass
class TestConfig:
    """Global test configuration"""
    devices: List[DeviceConfig]
    test_data_dir: Path
    reports_dir: Path
    logs_dir: Path
    network_interface: str = "auto"
    multicast_address: str = "239.255.255.250"
    discovery_timeout: int = 10
    service_timeout: int = 30
    stream_timeout: int = 60
    enable_video_analysis: bool = False
    enable_performance_tests: bool = False
    max_concurrent_tests: int = 3
    retry_attempts: int = 3
    retry_delay: float = 1.0

    def get_device(self, name: str) -> Optional[DeviceConfig]:
        """Get a device configuration by name"""
        for device in self.devices:
            if device.name == name:
                return device
        return None

    def get_default_device(self) -> Optional[DeviceConfig]:
        """Get the first device (default)"""
        return self.devices[0] if self.devices else None


# Default test devices
DEFAULT_DEVICES = [
    DeviceConfig(
        name="camera1",
        ip_address="192.168.1.100",
        http_port=8080,
        rtsp_port=554,
        username="admin",
        password="admin"
    ),
    DeviceConfig(
        name="camera2",
        ip_address="192.168.1.101",
        http_port=8080,
        rtsp_port=554,
        username="admin",
        password="admin"
    )
]

# Load configuration from environment variables
def load_config_from_env() -> TestConfig:
    """Load test configuration from environment variables"""
    devices = []

    # Load devices from environment
    device_count = int(os.getenv("ONVIF_TEST_DEVICE_COUNT", "1"))
    for i in range(device_count):
        device_name = os.getenv(f"ONVIF_TEST_DEVICE_{i}_NAME", f"camera{i+1}")
        device_ip = os.getenv(f"ONVIF_TEST_DEVICE_{i}_IP", f"192.168.1.{100+i}")
        device_port = int(os.getenv(f"ONVIF_TEST_DEVICE_{i}_PORT", "8080"))
        device_rtsp_port = int(os.getenv(f"ONVIF_TEST_DEVICE_{i}_RTSP_PORT", "554"))
        device_ws_port = int(os.getenv(f"ONVIF_TEST_DEVICE_{i}_WS_DISCOVERY_PORT", "3702"))
        device_username = os.getenv(f"ONVIF_TEST_DEVICE_{i}_USERNAME", "admin")
        device_password = os.getenv(f"ONVIF_TEST_DEVICE_{i}_PASSWORD", "admin")
        device_type = os.getenv(f"ONVIF_TEST_DEVICE_{i}_DEVICE_TYPE", "camera")
        expected_manufacturer = os.getenv(f"ONVIF_TEST_DEVICE_{i}_EXPECTED_MANUFACTURER", "Anyka")
        expected_model = os.getenv(f"ONVIF_TEST_DEVICE_{i}_EXPECTED_MODEL", "AK3918 Camera")
        enable_auth = os.getenv(f"ONVIF_TEST_DEVICE_{i}_ENABLE_AUTH", "false").lower() == "true"
        device_timeout = int(os.getenv(f"ONVIF_TEST_DEVICE_{i}_TIMEOUT", "30"))

        devices.append(DeviceConfig(
            name=device_name,
            ip_address=device_ip,
            http_port=device_port,
            rtsp_port=device_rtsp_port,
            ws_discovery_port=device_ws_port,
            username=device_username,
            password=device_password,
            device_type=device_type,
            expected_manufacturer=expected_manufacturer,
            expected_model=expected_model,
            enable_auth=enable_auth,
            timeout=device_timeout
        ))

    # Create base directories
    base_dir = Path(__file__).parent
    test_data_dir = Path(os.getenv("ONVIF_TEST_DATA_DIR", "test_data"))
    reports_dir = Path(os.getenv("ONVIF_TEST_REPORTS_DIR", "reports"))
    logs_dir = Path(os.getenv("ONVIF_TEST_LOGS_DIR", "logs"))

    # Make paths absolute if they're relative
    if not test_data_dir.is_absolute():
        test_data_dir = base_dir / test_data_dir
    if not reports_dir.is_absolute():
        reports_dir = base_dir / reports_dir
    if not logs_dir.is_absolute():
        logs_dir = base_dir / logs_dir

    # Create directories if they don't exist
    for dir_path in [test_data_dir, reports_dir, logs_dir]:
        dir_path.mkdir(exist_ok=True)

    return TestConfig(
        devices=devices if devices else DEFAULT_DEVICES,
        test_data_dir=test_data_dir,
        reports_dir=reports_dir,
        logs_dir=logs_dir,
        network_interface=os.getenv("ONVIF_TEST_NETWORK_INTERFACE", "auto"),
        multicast_address=os.getenv("ONVIF_TEST_MULTICAST_ADDRESS", "239.255.255.250"),
        discovery_timeout=int(os.getenv("ONVIF_TEST_DISCOVERY_TIMEOUT", "10")),
        service_timeout=int(os.getenv("ONVIF_TEST_SERVICE_TIMEOUT", "30")),
        stream_timeout=int(os.getenv("ONVIF_TEST_STREAM_TIMEOUT", "60")),
        enable_video_analysis=os.getenv("ONVIF_TEST_VIDEO_ANALYSIS", "false").lower() == "true",
        enable_performance_tests=os.getenv("ONVIF_TEST_PERFORMANCE", "false").lower() == "true",
        max_concurrent_tests=int(os.getenv("ONVIF_TEST_MAX_CONCURRENT", "3")),
        retry_attempts=int(os.getenv("ONVIF_TEST_RETRY_ATTEMPTS", "3")),
        retry_delay=float(os.getenv("ONVIF_TEST_RETRY_DELAY", "1.0"))
    )


# Global configuration instance
config = load_config_from_env()
