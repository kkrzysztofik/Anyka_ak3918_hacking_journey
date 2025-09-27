"""
RTSP streaming integration tests
"""
import pytest
import cv2
import numpy as np
import time
from typing import Optional, Tuple, Dict, Any
from urllib.parse import urlparse

from .fixtures import (
    device_config,
    retry_on_failure,
    ONVIF_NAMESPACES
)


class RTSPStreamTester:
    """RTSP stream testing utilities"""

    def __init__(self, rtsp_url: str):
        self.rtsp_url = rtsp_url
        self.cap: Optional[cv2.VideoCapture] = None

    def connect(self, timeout: int = 30) -> bool:
        """Connect to RTSP stream"""
        try:
            self.cap = cv2.VideoCapture(self.rtsp_url)

            if not self.cap.isOpened():
                return False

            # Set buffer size to minimize latency
            self.cap.set(cv2.CAP_PROP_BUFFERSIZE, 1)

            # Wait for connection to establish
            start_time = time.time()
            while time.time() - start_time < timeout:
                if self.cap.isOpened() and self.cap.grab():
                    return True
                time.sleep(0.1)

            return False
        except Exception:
            return False

    def get_frame(self, timeout: float = 5.0) -> Optional[np.ndarray]:
        """Get a single frame from the stream"""
        if self.cap is None:
            return None

        start_time = time.time()
        while time.time() - start_time < timeout:
            ret, frame = self.cap.read()
            if ret and frame is not None:
                return frame
            time.sleep(0.01)

        return None

    def get_frame_info(self, frame: np.ndarray) -> Dict[str, Any]:
        """Get information about a frame"""
        height, width = frame.shape[:2]
        channels = frame.shape[2] if len(frame.shape) > 2 else 1

        return {
            'width': width,
            'height': height,
            'channels': channels,
            'shape': frame.shape,
            'dtype': frame.dtype,
            'size_bytes': frame.nbytes
        }

    def disconnect(self):
        """Disconnect from RTSP stream"""
        if self.cap is not None:
            self.cap.release()
            self.cap = None

    def test_stream_properties(self) -> Dict[str, Any]:
        """Test various stream properties"""
        if self.cap is None:
            return {}

        properties = {}

        # Test common properties
        prop_map = {
            'width': cv2.CAP_PROP_FRAME_WIDTH,
            'height': cv2.CAP_PROP_FRAME_HEIGHT,
            'fps': cv2.CAP_PROP_FPS,
            'format': cv2.CAP_PROP_FORMAT,
            'brightness': cv2.CAP_PROP_BRIGHTNESS,
            'contrast': cv2.CAP_PROP_CONTRAST,
            'saturation': cv2.CAP_PROP_SATURATION,
            'hue': cv2.CAP_PROP_HUE,
            'gain': cv2.CAP_PROP_GAIN,
            'exposure': cv2.CAP_PROP_EXPOSURE
        }

        for prop_name, prop_id in prop_map.items():
            try:
                value = self.cap.get(prop_id)
                properties[prop_name] = value
            except Exception:
                properties[prop_name] = None

        return properties


@pytest.fixture
def rtsp_tester(device_config):
    """Provide RTSP stream tester"""
    return RTSPStreamTester(device_config.get_rtsp_main_url())


class TestRTSPStreaming:
    """RTSP streaming integration tests"""

    @pytest.mark.rtsp
    @pytest.mark.integration
    @pytest.mark.slow
    def test_rtsp_stream_connection(self, rtsp_tester: RTSPStreamTester):
        """Test basic RTSP stream connection"""
        assert rtsp_tester.connect(), f"Failed to connect to RTSP stream: {rtsp_tester.rtsp_url}"

        # Verify stream is actually working by getting a frame
        frame = rtsp_tester.get_frame()
        assert frame is not None, "No frames received from RTSP stream"

        rtsp_tester.disconnect()

    @pytest.mark.rtsp
    @pytest.mark.integration
    @pytest.mark.slow
    def test_rtsp_frame_properties(self, rtsp_tester: RTSPStreamTester):
        """Test RTSP frame properties and format"""
        assert rtsp_tester.connect(), "Failed to connect to RTSP stream"

        frame = rtsp_tester.get_frame()
        assert frame is not None, "No frame received"

        frame_info = rtsp_tester.get_frame_info(frame)

        # Verify basic frame properties
        assert frame_info['width'] > 0, f"Invalid frame width: {frame_info['width']}"
        assert frame_info['height'] > 0, f"Invalid frame height: {frame_info['height']}"
        assert frame_info['channels'] in [1, 3, 4], f"Invalid channel count: {frame_info['channels']}"

        # Verify reasonable frame dimensions for a camera
        assert 100 <= frame_info['width'] <= 4000, f"Frame width out of reasonable range: {frame_info['width']}"
        assert 100 <= frame_info['height'] <= 4000, f"Frame height out of reasonable range: {frame_info['height']}"

        rtsp_tester.disconnect()

    @pytest.mark.rtsp
    @pytest.mark.integration
    @pytest.mark.slow
    def test_rtsp_stream_continuity(self, rtsp_tester: RTSPStreamTester):
        """Test RTSP stream continuity and frame rate"""
        assert rtsp_tester.connect(), "Failed to connect to RTSP stream"

        frames = []
        start_time = time.time()

        # Collect frames for a few seconds
        while time.time() - start_time < 5.0:
            frame = rtsp_tester.get_frame(timeout=1.0)
            if frame is not None:
                frames.append(frame)
            time.sleep(0.1)

        assert len(frames) > 0, "No frames received during continuity test"

        # Calculate approximate frame rate
        duration = time.time() - start_time
        fps = len(frames) / duration

        # Verify reasonable frame rate for a camera stream
        assert 1 <= fps <= 60, f"Unreasonable frame rate: {fps:.2f} fps"

        rtsp_tester.disconnect()

    @pytest.mark.rtsp
    @pytest.mark.integration
    @pytest.mark.slow
    def test_rtsp_stream_stability(self, rtsp_tester: RTSPStreamTester):
        """Test RTSP stream stability over time"""
        assert rtsp_tester.connect(), "Failed to connect to RTSP stream"

        frame_count = 0
        error_count = 0
        start_time = time.time()

        # Test for 10 seconds
        while time.time() - start_time < 10.0:
            frame = rtsp_tester.get_frame(timeout=0.5)
            if frame is not None:
                frame_count += 1
            else:
                error_count += 1

            time.sleep(0.2)

        # Verify we got a reasonable number of frames
        assert frame_count > 0, "No frames received during stability test"

        # Error rate should be low
        total_attempts = frame_count + error_count
        error_rate = error_count / total_attempts if total_attempts > 0 else 0
        assert error_rate < 0.3, f"High error rate during streaming: {error_rate:.2f}"

        rtsp_tester.disconnect()

    @pytest.mark.rtsp
    @pytest.mark.integration
    @pytest.mark.slow
    def test_rtsp_stream_reconnection(self, rtsp_tester: RTSPStreamTester):
        """Test RTSP stream reconnection capability"""
        # First connection
        assert rtsp_tester.connect(), "Failed initial connection"
        frame1 = rtsp_tester.get_frame()
        assert frame1 is not None, "No frame from initial connection"
        rtsp_tester.disconnect()

        # Wait a bit
        time.sleep(2)

        # Second connection
        assert rtsp_tester.connect(), "Failed to reconnect to RTSP stream"
        frame2 = rtsp_tester.get_frame()
        assert frame2 is not None, "No frame from reconnected stream"

        # Frames should have similar properties
        info1 = rtsp_tester.get_frame_info(frame1)
        info2 = rtsp_tester.get_frame_info(frame2)

        assert info1['width'] == info2['width'], "Frame width changed after reconnection"
        assert info1['height'] == info2['height'], "Frame height changed after reconnection"
        assert info1['channels'] == info2['channels'], "Frame channels changed after reconnection"

        rtsp_tester.disconnect()

    @pytest.mark.rtsp
    @pytest.mark.integration
    @pytest.mark.slow
    def test_multiple_rtsp_streams(self, device_config):
        """Test multiple RTSP streams (main and sub)"""
        main_tester = RTSPStreamTester(device_config.get_rtsp_main_url())
        sub_tester = RTSPStreamTester(device_config.get_rtsp_sub_url())

        try:
            # Connect to both streams
            assert main_tester.connect(), "Failed to connect to main stream"
            assert sub_tester.connect(), "Failed to connect to sub stream"

            # Get frames from both
            main_frame = main_tester.get_frame()
            sub_frame = sub_tester.get_frame()

            assert main_frame is not None, "No frame from main stream"
            assert sub_frame is not None, "No frame from sub stream"

            # Compare stream properties
            main_info = main_tester.get_frame_info(main_frame)
            sub_info = sub_tester.get_frame_info(sub_frame)

            # Main stream should typically have higher resolution than sub stream
            if main_info['width'] and sub_info['width']:
                # Allow some tolerance in resolution comparison
                assert main_info['width'] >= sub_info['width'] * 0.8, \
                    f"Main stream width {main_info['width']} should be >= sub stream width {sub_info['width']}"

            if main_info['height'] and sub_info['height']:
                assert main_info['height'] >= sub_info['height'] * 0.8, \
                    f"Main stream height {main_info['height']} should be >= sub stream height {sub_info['height']}"

        finally:
            main_tester.disconnect()
            sub_tester.disconnect()

    @pytest.mark.rtsp
    @pytest.mark.integration
    @pytest.mark.slow
    def test_rtsp_stream_metadata(self, rtsp_tester: RTSPStreamTester):
        """Test RTSP stream metadata and properties"""
        assert rtsp_tester.connect(), "Failed to connect to RTSP stream"

        properties = rtsp_tester.test_stream_properties()

        # Verify we can read properties
        assert isinstance(properties, dict), "Stream properties should be a dictionary"

        # Check for some expected properties
        expected_props = ['width', 'height', 'fps']
        for prop in expected_props:
            assert prop in properties, f"Missing expected property: {prop}"

        # Verify property values are reasonable
        if properties['width'] is not None:
            assert 100 <= properties['width'] <= 4000, f"Invalid width: {properties['width']}"

        if properties['height'] is not None:
            assert 100 <= properties['height'] <= 4000, f"Invalid height: {properties['height']}"

        if properties['fps'] is not None:
            assert 1 <= properties['fps'] <= 60, f"Invalid FPS: {properties['fps']}"

        rtsp_tester.disconnect()

    @pytest.mark.rtsp
    @pytest.mark.integration
    @pytest.mark.slow
    @pytest.mark.camera
    def test_rtsp_video_quality(self, rtsp_tester: RTSPStreamTester):
        """Test RTSP video quality metrics"""
        assert rtsp_tester.connect(), "Failed to connect to RTSP stream"

        frames = []
        for i in range(10):
            frame = rtsp_tester.get_frame()
            if frame is not None:
                frames.append(frame)
            time.sleep(0.2)

        assert len(frames) >= 5, f"Insufficient frames for quality test: got {len(frames)}"

        # Calculate quality metrics
        frame_sizes = [frame.nbytes for frame in frames]
        avg_frame_size = sum(frame_sizes) / len(frame_sizes)

        # Verify reasonable frame sizes
        assert avg_frame_size > 1000, f"Frame size too small: {avg_frame_size} bytes"
        assert avg_frame_size < 10 * 1024 * 1024, f"Frame size too large: {avg_frame_size} bytes"

        # Check frame consistency
        first_frame_info = rtsp_tester.get_frame_info(frames[0])
        for frame in frames[1:]:
            info = rtsp_tester.get_frame_info(frame)
            assert info['width'] == first_frame_info['width'], "Frame width inconsistent"
            assert info['height'] == first_frame_info['height'], "Frame height inconsistent"
            assert info['channels'] == first_frame_info['channels'], "Frame channels inconsistent"

        rtsp_tester.disconnect()

    @pytest.mark.rtsp
    @pytest.mark.integration
    @pytest.mark.network
    def test_rtsp_url_format_validation(self, device_config):
        """Test RTSP URL format and accessibility"""
        rtsp_urls = [
            device_config.get_rtsp_main_url(),
            device_config.get_rtsp_sub_url()
        ]

        for url in rtsp_urls:
            # Parse URL
            parsed = urlparse(url)
            assert parsed.scheme == 'rtsp', f"Invalid scheme in RTSP URL: {url}"
            assert parsed.hostname, f"Missing hostname in RTSP URL: {url}"
            assert parsed.port, f"Missing port in RTSP URL: {url}"
            assert parsed.path, f"Missing path in RTSP URL: {url}"

            # Test URL structure
            assert parsed.path.startswith('/'), f"RTSP path should start with /: {url}"

    @pytest.mark.rtsp
    @pytest.mark.integration
    @pytest.mark.slow
    @retry_on_failure(max_attempts=3, delay=5.0)
    def test_rtsp_stream_performance(self, rtsp_tester: RTSPStreamTester):
        """Test RTSP stream performance metrics"""
        assert rtsp_tester.connect(), "Failed to connect to RTSP stream"

        frame_times = []
        frame_sizes = []

        start_time = time.time()

        # Collect performance data for 10 seconds
        while time.time() - start_time < 10.0:
            frame_start = time.time()
            frame = rtsp_tester.get_frame(timeout=1.0)
            frame_end = time.time()

            if frame is not None:
                frame_times.append(frame_end - frame_start)
                frame_sizes.append(frame.nbytes)

            time.sleep(0.1)

        assert len(frame_times) > 0, "No frames received during performance test"

        # Calculate performance metrics
        avg_frame_time = sum(frame_times) / len(frame_times)
        avg_frame_size = sum(frame_sizes) / len(frame_sizes)
        fps = len(frame_times) / (time.time() - start_time)

        # Verify performance
        assert avg_frame_time < 1.0, f"Average frame time too slow: {avg_frame_time:.3f}s"
        assert fps >= 1.0, f"Frame rate too low: {fps:.2f} fps"
        assert avg_frame_size > 1000, f"Average frame size too small: {avg_frame_size} bytes"

        rtsp_tester.disconnect()

    @pytest.mark.rtsp
    @pytest.mark.integration
    @pytest.mark.slow
    def test_rtsp_stream_error_recovery(self, rtsp_tester: RTSPStreamTester):
        """Test RTSP stream error recovery"""
        assert rtsp_tester.connect(), "Failed to connect to RTSP stream"

        # Disconnect and reconnect multiple times
        for i in range(3):
            # Disconnect
            rtsp_tester.disconnect()
            time.sleep(1)

            # Reconnect
            assert rtsp_tester.connect(), f"Failed to reconnect (attempt {i+1})"

            # Verify stream still works
            frame = rtsp_tester.get_frame()
            assert frame is not None, f"No frame after reconnection (attempt {i+1})"

            time.sleep(0.5)

        rtsp_tester.disconnect()
