"""
ONVIF Imaging service E2E tests
"""
import pytest
from typing import Dict, Any, List
import xml.etree.ElementTree as ET
import time

from .fixtures import (
    device_config,
    soap_client,
    make_soap_request,
    validate_xml_response,
    retry_on_failure,
    SOAP_ENVELOPE_TEMPLATE,
    ONVIF_NAMESPACES
)


class ONVIFImagingClient:
    """ONVIF Imaging service client"""

    def __init__(self, imaging_url: str):
        self.imaging_url = imaging_url

    def create_get_imaging_settings_request(self, video_source_token: str) -> str:
        """Create GetImagingSettings SOAP request"""
        body = f'''<timg:GetImagingSettings xmlns:timg="http://www.onvif.org/ver20/imaging/wsdl">
            <timg:VideoSourceToken>{video_source_token}</timg:VideoSourceToken>
        </timg:GetImagingSettings>'''
        return SOAP_ENVELOPE_TEMPLATE.format(body=body)

    def create_set_imaging_settings_request(self, video_source_token: str,
                                          brightness: int = 50,
                                          contrast: int = 50,
                                          saturation: int = 50,
                                          sharpness: int = 50) -> str:
        """Create SetImagingSettings SOAP request"""
        body = f'''<timg:SetImagingSettings xmlns:timg="http://www.onvif.org/ver20/imaging/wsdl">
            <timg:VideoSourceToken>{video_source_token}</timg:VideoSourceToken>
            <timg:ImagingSettings>
                <tt:Brightness xmlns:tt="http://www.onvif.org/ver10/schema">{brightness}</tt:Brightness>
                <tt:Contrast xmlns:tt="http://www.onvif.org/ver10/schema">{contrast}</tt:Contrast>
                <tt:Saturation xmlns:tt="http://www.onvif.org/ver10/schema">{saturation}</tt:Saturation>
                <tt:Sharpness xmlns:tt="http://www.onvif.org/ver10/schema">{sharpness}</tt:Sharpness>
            </timg:ImagingSettings>
        </timg:SetImagingSettings>'''
        return SOAP_ENVELOPE_TEMPLATE.format(body=body)

    def create_get_options_request(self, video_source_token: str) -> str:
        """Create GetOptions SOAP request"""
        body = f'''<timg:GetOptions xmlns:timg="http://www.onvif.org/ver20/imaging/wsdl">
            <timg:VideoSourceToken>{video_source_token}</timg:VideoSourceToken>
        </timg:GetOptions>'''
        return SOAP_ENVELOPE_TEMPLATE.format(body=body)

    def create_get_move_options_request(self, video_source_token: str) -> str:
        """Create GetMoveOptions SOAP request"""
        body = f'''<timg:GetMoveOptions xmlns:timg="http://www.onvif.org/ver20/imaging/wsdl">
            <timg:VideoSourceToken>{video_source_token}</timg:VideoSourceToken>
        </timg:GetMoveOptions>'''
        return SOAP_ENVELOPE_TEMPLATE.format(body=body)

    def create_move_request(self, video_source_token: str,
                           focus_move: str = "Near",
                           continuous_duration: int = 1) -> str:
        """Create Move SOAP request for focus"""
        body = f'''<timg:Move xmlns:timg="http://www.onvif.org/ver20/imaging/wsdl">
            <timg:VideoSourceToken>{video_source_token}</timg:VideoSourceToken>
            <timg:Focus>
                <tt:Continuous xmlns:tt="http://www.onvif.org/ver10/schema">
                    <tt:Move>{focus_move}</tt:Move>
                    <tt:Timeout>PT{continuous_duration}S</tt:Timeout>
                </tt:Continuous>
            </timg:Focus>
        </timg:Move>'''
        return SOAP_ENVELOPE_TEMPLATE.format(body=body)

    def create_stop_request(self, video_source_token: str) -> str:
        """Create Stop SOAP request for imaging"""
        body = f'''<timg:Stop xmlns:timg="http://www.onvif.org/ver20/imaging/wsdl">
            <timg:VideoSourceToken>{video_source_token}</timg:VideoSourceToken>
        </timg:Stop>'''
        return SOAP_ENVELOPE_TEMPLATE.format(body=body)

    def parse_imaging_settings(self, response_xml: str) -> Dict[str, Any]:
        """Parse GetImagingSettings response"""
        try:
            root = ET.fromstring(response_xml)

            for elem in root.iter():
                if '}' in elem.tag:
                    elem.tag = elem.tag.split('}', 1)[1]

            body = root.find('Body')
            if body is None:
                raise ValueError("No Body element found")

            response = body.find('GetImagingSettingsResponse')
            if response is None:
                raise ValueError("No GetImagingSettingsResponse found")

            settings = response.find('ImagingSettings')
            if settings is None:
                raise ValueError("No ImagingSettings found")

            imaging_settings = {
                'brightness': 50,
                'contrast': 50,
                'saturation': 50,
                'sharpness': 50
            }

            # Extract settings values
            for setting_name in imaging_settings.keys():
                setting_elem = settings.find(setting_name.capitalize())
                if setting_elem is not None and setting_elem.text:
                    try:
                        imaging_settings[setting_name] = int(setting_elem.text)
                    except ValueError:
                        pass  # Keep default value

            return imaging_settings
        except Exception as e:
            pytest.fail(f"Failed to parse imaging settings response: {e}")

    def parse_imaging_options(self, response_xml: str) -> Dict[str, Any]:
        """Parse GetOptions response"""
        try:
            root = ET.fromstring(response_xml)

            for elem in root.iter():
                if '}' in elem.tag:
                    elem.tag = elem.tag.split('}', 1)[1]

            body = root.find('Body')
            if body is None:
                raise ValueError("No Body element found")

            response = body.find('GetOptionsResponse')
            if response is None:
                raise ValueError("No GetOptionsResponse found")

            options = response.find('ImagingOptions')
            if options is None:
                raise ValueError("No ImagingOptions found")

            imaging_options = {
                'brightness_min': 0,
                'brightness_max': 100,
                'contrast_min': 0,
                'contrast_max': 100,
                'saturation_min': 0,
                'saturation_max': 100,
                'sharpness_min': 0,
                'sharpness_max': 100
            }

            # Extract option ranges
            for option_name in ['Brightness', 'Contrast', 'Saturation', 'Sharpness']:
                option_elem = options.find(option_name)
                if option_elem is not None:
                    min_elem = option_elem.find('Min')
                    max_elem = option_elem.find('Max')
                    if min_elem is not None and min_elem.text:
                        imaging_options[f'{option_name.lower()}_min'] = int(min_elem.text)
                    if max_elem is not None and max_elem.text:
                        imaging_options[f'{option_name.lower()}_max'] = int(max_elem.text)

            return imaging_options
        except Exception as e:
            pytest.fail(f"Failed to parse imaging options response: {e}")


@pytest.fixture
def imaging_client(device_config):
    """Provide ONVIF Imaging service client"""
    return ONVIFImagingClient(device_config.get_onvif_imaging_url())


class TestONVIFImagingService:
    """ONVIF Imaging service E2E tests"""

    @pytest.mark.onvif_imaging
    @pytest.mark.integration
    def test_get_imaging_settings(self, imaging_client: ONVIFImagingClient, device_config: 'DeviceConfig'):
        """Test GetImagingSettings operation"""
        # First need to get a video source token from Media service
        media_client = ONVIFMediaClient(device_config.get_onvif_media_url())
        soap_request = media_client.create_get_video_sources_request()

        response = make_soap_request(
            device_config.get_onvif_media_url(),
            soap_request,
            timeout=30
        )

        content = validate_xml_response(response, 200)
        video_sources = media_client.parse_video_sources(content)

        if len(video_sources) == 0:
            pytest.skip("No video sources available for imaging settings test")

        video_source_token = video_sources[0]['token']

        # Get imaging settings
        soap_request = imaging_client.create_get_imaging_settings_request(video_source_token)

        response = make_soap_request(
            imaging_client.imaging_url,
            soap_request,
            timeout=30
        )

        content = validate_xml_response(response, 200)

        # Parse imaging settings
        settings = imaging_client.parse_imaging_settings(content)

        # Verify settings structure
        assert 'brightness' in settings, "Settings missing brightness"
        assert 'contrast' in settings, "Settings missing contrast"
        assert 'saturation' in settings, "Settings missing saturation"
        assert 'sharpness' in settings, "Settings missing sharpness"

        # Verify reasonable value ranges
        for setting_name, value in settings.items():
            assert 0 <= value <= 100, f"Invalid {setting_name} value: {value}"

    @pytest.mark.onvif_imaging
    @pytest.mark.integration
    def test_get_imaging_options(self, imaging_client: ONVIFImagingClient, device_config: 'DeviceConfig'):
        """Test GetOptions operation"""
        # First need to get a video source token from Media service
        media_client = ONVIFMediaClient(device_config.get_onvif_media_url())
        soap_request = media_client.create_get_video_sources_request()

        response = make_soap_request(
            device_config.get_onvif_media_url(),
            soap_request,
            timeout=30
        )

        content = validate_xml_response(response, 200)
        video_sources = media_client.parse_video_sources(content)

        if len(video_sources) == 0:
            pytest.skip("No video sources available for imaging options test")

        video_source_token = video_sources[0]['token']

        # Get imaging options
        soap_request = imaging_client.create_get_options_request(video_source_token)

        response = make_soap_request(
            imaging_client.imaging_url,
            soap_request,
            timeout=30
        )

        content = validate_xml_response(response, 200)

        # Parse imaging options
        options = imaging_client.parse_imaging_options(content)

        # Verify options structure
        expected_options = [
            'brightness_min', 'brightness_max',
            'contrast_min', 'contrast_max',
            'saturation_min', 'saturation_max',
            'sharpness_min', 'sharpness_max'
        ]

        for option in expected_options:
            assert option in options, f"Options missing {option}"

        # Verify reasonable ranges
        assert options['brightness_min'] >= 0, "Invalid brightness min"
        assert options['brightness_max'] <= 100, "Invalid brightness max"
        assert options['contrast_min'] >= 0, "Invalid contrast min"
        assert options['contrast_max'] <= 100, "Invalid contrast max"
        assert options['saturation_min'] >= 0, "Invalid saturation min"
        assert options['saturation_max'] <= 100, "Invalid saturation max"
        assert options['sharpness_min'] >= 0, "Invalid sharpness min"
        assert options['sharpness_max'] <= 100, "Invalid sharpness max"

    @pytest.mark.onvif_imaging
    @pytest.mark.integration
    def test_set_and_get_imaging_settings(self, imaging_client: ONVIFImagingClient, device_config: 'DeviceConfig'):
        """Test SetImagingSettings and GetImagingSettings operations"""
        # First need to get a video source token from Media service
        media_client = ONVIFMediaClient(device_config.get_onvif_media_url())
        soap_request = media_client.create_get_video_sources_request()

        response = make_soap_request(
            device_config.get_onvif_media_url(),
            soap_request,
            timeout=30
        )

        content = validate_xml_response(response, 200)
        video_sources = media_client.parse_video_sources(content)

        if len(video_sources) == 0:
            pytest.skip("No video sources available for imaging settings test")

        video_source_token = video_sources[0]['token']

        # Get original settings
        soap_request = imaging_client.create_get_imaging_settings_request(video_source_token)

        response = make_soap_request(
            imaging_client.imaging_url,
            soap_request,
            timeout=30
        )

        content = validate_xml_response(response, 200)
        original_settings = imaging_client.parse_imaging_settings(content)

        # Set new settings (slightly different values)
        new_brightness = max(0, min(100, original_settings['brightness'] + 10))
        new_contrast = max(0, min(100, original_settings['contrast'] + 5))
        new_saturation = max(0, min(100, original_settings['saturation'] + 5))
        new_sharpness = max(0, min(100, original_settings['sharpness'] + 5))

        soap_request = imaging_client.create_set_imaging_settings_request(
            video_source_token,
            brightness=new_brightness,
            contrast=new_contrast,
            saturation=new_saturation,
            sharpness=new_sharpness
        )

        response = make_soap_request(
            imaging_client.imaging_url,
            soap_request,
            timeout=30
        )

        assert response.status_code == 200, f"SetImagingSettings failed, status: {response.status_code}"

        # Wait a moment for settings to take effect
        time.sleep(1)

        # Get settings again to verify they were applied
        soap_request = imaging_client.create_get_imaging_settings_request(video_source_token)

        response = make_soap_request(
            imaging_client.imaging_url,
            soap_request,
            timeout=30
        )

        content = validate_xml_response(response, 200)
        updated_settings = imaging_client.parse_imaging_settings(content)

        # Verify settings were updated
        assert updated_settings['brightness'] == new_brightness, \
            f"Brightness not updated: expected {new_brightness}, got {updated_settings['brightness']}"
        assert updated_settings['contrast'] == new_contrast, \
            f"Contrast not updated: expected {new_contrast}, got {updated_settings['contrast']}"
        assert updated_settings['saturation'] == new_saturation, \
            f"Saturation not updated: expected {new_saturation}, got {updated_settings['saturation']}"
        assert updated_settings['sharpness'] == new_sharpness, \
            f"Sharpness not updated: expected {new_sharpness}, got {updated_settings['sharpness']}"

    @pytest.mark.onvif_imaging
    @pytest.mark.integration
    def test_get_move_options(self, imaging_client: ONVIFImagingClient, device_config: 'DeviceConfig'):
        """Test GetMoveOptions operation"""
        # First need to get a video source token from Media service
        media_client = ONVIFMediaClient(device_config.get_onvif_media_url())
        soap_request = media_client.create_get_video_sources_request()

        response = make_soap_request(
            device_config.get_onvif_media_url(),
            soap_request,
            timeout=30
        )

        content = validate_xml_response(response, 200)
        video_sources = media_client.parse_video_sources(content)

        if len(video_sources) == 0:
            pytest.skip("No video sources available for move options test")

        video_source_token = video_sources[0]['token']

        # Get move options
        soap_request = imaging_client.create_get_move_options_request(video_source_token)

        response = make_soap_request(
            imaging_client.imaging_url,
            soap_request,
            timeout=30
        )

        content = validate_xml_response(response, 200)

        # Verify response contains move options
        assert 'GetMoveOptionsResponse' in content, "Response should contain GetMoveOptionsResponse"

    @pytest.mark.onvif_imaging
    @pytest.mark.integration
    def test_focus_move_and_stop(self, imaging_client: ONVIFImagingClient, device_config: 'DeviceConfig'):
        """Test Move and Stop operations for focus control"""
        # First need to get a video source token from Media service
        media_client = ONVIFMediaClient(device_config.get_onvif_media_url())
        soap_request = media_client.create_get_video_sources_request()

        response = make_soap_request(
            device_config.get_onvif_media_url(),
            soap_request,
            timeout=30
        )

        content = validate_xml_response(response, 200)
        video_sources = media_client.parse_video_sources(content)

        if len(video_sources) == 0:
            pytest.skip("No video sources available for focus control test")

        video_source_token = video_sources[0]['token']

        # Test focus move to Near position
        soap_request = imaging_client.create_move_request(
            video_source_token,
            focus_move="Near",
            continuous_duration=2
        )

        response = make_soap_request(
            imaging_client.imaging_url,
            soap_request,
            timeout=30
        )

        assert response.status_code == 200, f"Focus move failed, status: {response.status_code}"

        # Wait during movement
        time.sleep(2)

        # Stop focus movement
        soap_request = imaging_client.create_stop_request(video_source_token)

        response = make_soap_request(
            imaging_client.imaging_url,
            soap_request,
            timeout=30
        )

        assert response.status_code == 200, f"Focus stop failed, status: {response.status_code}"

    @pytest.mark.onvif_imaging
    @pytest.mark.integration
    def test_imaging_service_availability(self, imaging_client: ONVIFImagingClient, device_config: 'DeviceConfig'):
        """Test that Imaging service is available and responding"""
        # First need to get a video source token from Media service
        media_client = ONVIFMediaClient(device_config.get_onvif_media_url())
        soap_request = media_client.create_get_video_sources_request()

        response = make_soap_request(
            device_config.get_onvif_media_url(),
            soap_request,
            timeout=30
        )

        content = validate_xml_response(response, 200)
        video_sources = media_client.parse_video_sources(content)

        if len(video_sources) == 0:
            pytest.skip("No video sources available for imaging service test")

        video_source_token = video_sources[0]['token']

        # Test GetImagingSettings to verify service availability
        soap_request = imaging_client.create_get_imaging_settings_request(video_source_token)

        response = make_soap_request(
            imaging_client.imaging_url,
            soap_request,
            timeout=30
        )

        assert response.status_code == 200, f"Imaging service not available, status: {response.status_code}"

        content = response.text
        assert 'GetImagingSettingsResponse' in content, "Response should contain imaging settings"

    @pytest.mark.onvif_imaging
    @pytest.mark.integration
    @pytest.mark.slow
    def test_imaging_service_performance(self, imaging_client: ONVIFImagingClient, device_config: 'DeviceConfig'):
        """Test Imaging service response time"""
        import time

        # First need to get a video source token from Media service
        media_client = ONVIFMediaClient(device_config.get_onvif_media_url())
        soap_request = media_client.create_get_video_sources_request()

        response = make_soap_request(
            device_config.get_onvif_media_url(),
            soap_request,
            timeout=30
        )

        content = validate_xml_response(response, 200)
        video_sources = media_client.parse_video_sources(content)

        if len(video_sources) == 0:
            pytest.skip("No video sources available for performance test")

        video_source_token = video_sources[0]['token']

        # Measure response time for multiple requests
        response_times = []

        for i in range(5):
            start_time = time.time()

            soap_request = imaging_client.create_get_imaging_settings_request(video_source_token)
            response = make_soap_request(
                imaging_client.imaging_url,
                soap_request,
                timeout=30
            )

            end_time = time.time()
            response_times.append(end_time - start_time)

            validate_xml_response(response, 200)

        # Calculate statistics
        avg_time = sum(response_times) / len(response_times)
        max_time = max(response_times)

        # Assert reasonable performance
        assert avg_time < 2.0, f"Average response time too slow: {avg_time:.2f}s"
        assert max_time < 5.0, f"Maximum response time too slow: {max_time:.2f}s"

    @pytest.mark.onvif_imaging
    @pytest.mark.integration
    def test_imaging_settings_validation(self, imaging_client: ONVIFImagingClient, device_config: 'DeviceConfig'):
        """Test that imaging settings are within valid ranges"""
        # First need to get a video source token from Media service
        media_client = ONVIFMediaClient(device_config.get_onvif_media_url())
        soap_request = media_client.create_get_video_sources_request()

        response = make_soap_request(
            device_config.get_onvif_media_url(),
            soap_request,
            timeout=30
        )

        content = validate_xml_response(response, 200)
        video_sources = media_client.parse_video_sources(content)

        if len(video_sources) == 0:
            pytest.skip("No video sources available for settings validation test")

        video_source_token = video_sources[0]['token']

        # Get both settings and options
        soap_request = imaging_client.create_get_imaging_settings_request(video_source_token)
        response = make_soap_request(imaging_client.imaging_url, soap_request, timeout=30)
        content = validate_xml_response(response, 200)
        settings = imaging_client.parse_imaging_settings(content)

        soap_request = imaging_client.create_get_options_request(video_source_token)
        response = make_soap_request(imaging_client.imaging_url, soap_request, timeout=30)
        content = validate_xml_response(response, 200)
        options = imaging_client.parse_imaging_options(content)

        # Verify settings are within option ranges
        assert options['brightness_min'] <= settings['brightness'] <= options['brightness_max'], \
            f"Brightness {settings['brightness']} out of range [{options['brightness_min']}, {options['brightness_max']}]"
        assert options['contrast_min'] <= settings['contrast'] <= options['contrast_max'], \
            f"Contrast {settings['contrast']} out of range [{options['contrast_min']}, {options['contrast_max']}]"
        assert options['saturation_min'] <= settings['saturation'] <= options['saturation_max'], \
            f"Saturation {settings['saturation']} out of range [{options['saturation_min']}, {options['saturation_max']}]"
        assert options['sharpness_min'] <= settings['sharpness'] <= options['sharpness_max'], \
            f"Sharpness {settings['sharpness']} out of range [{options['sharpness_min']}, {options['sharpness_max']}]"


# Import required for type hints
from tests.test_onvif_media import ONVIFMediaClient
