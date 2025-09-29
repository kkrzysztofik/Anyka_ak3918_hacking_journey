"""
ONVIF Media service E2E tests
"""
import pytest
from typing import Dict, Any, List
import xml.etree.ElementTree as ET

from .fixtures import (
    device_config,
    soap_client,
    make_soap_request,
    validate_xml_response,
    retry_on_failure,
    SOAP_ENVELOPE_TEMPLATE,
    ONVIF_NAMESPACES
)


class ONVIFMediaClient:
    """ONVIF Media service client"""

    def __init__(self, media_url: str):
        self.media_url = media_url

    def create_get_profiles_request(self) -> str:
        """Create GetProfiles SOAP request"""
        body = '''<trt:GetProfiles xmlns:trt="http://www.onvif.org/ver10/media/wsdl"/>'''
        return SOAP_ENVELOPE_TEMPLATE.format(body=body)

    def create_get_stream_uri_request(self, profile_token: str) -> str:
        """Create GetStreamUri SOAP request"""
        body = f'''<trt:GetStreamUri xmlns:trt="http://www.onvif.org/ver10/media/wsdl">
            <trt:StreamSetup>
                <tt:Stream xmlns:tt="http://www.onvif.org/ver10/schema">RTP-Unicast</tt:Stream>
                <tt:Transport xmlns:tt="http://www.onvif.org/ver10/schema">
                    <tt:Protocol>RTSP</tt:Protocol>
                </tt:Transport>
            </trt:StreamSetup>
            <trt:ProfileToken>{profile_token}</trt:ProfileToken>
        </trt:GetStreamUri>'''
        return SOAP_ENVELOPE_TEMPLATE.format(body=body)

    def create_get_video_sources_request(self) -> str:
        """Create GetVideoSources SOAP request"""
        body = '''<trt:GetVideoSources xmlns:trt="http://www.onvif.org/ver10/media/wsdl"/>'''
        return SOAP_ENVELOPE_TEMPLATE.format(body=body)

    def create_get_audio_sources_request(self) -> str:
        """Create GetAudioSources SOAP request"""
        body = '''<trt:GetAudioSources xmlns:trt="http://www.onvif.org/ver10/media/wsdl"/>'''
        return SOAP_ENVELOPE_TEMPLATE.format(body=body)

    def create_get_video_source_configurations_request(self) -> str:
        """Create GetVideoSourceConfigurations SOAP request"""
        body = '''<trt:GetVideoSourceConfigurations xmlns:trt="http://www.onvif.org/ver10/media/wsdl"/>'''
        return SOAP_ENVELOPE_TEMPLATE.format(body=body)

    def parse_profiles(self, response_xml: str) -> List[Dict[str, Any]]:
        """Parse GetProfiles response"""
        try:
            root = ET.fromstring(response_xml)

            for elem in root.iter():
                if '}' in elem.tag:
                    elem.tag = elem.tag.split('}', 1)[1]

            body = root.find('Body')
            if body is None:
                raise ValueError("No Body element found")

            response = body.find('GetProfilesResponse')
            if response is None:
                raise ValueError("No GetProfilesResponse found")

            profiles = []
            for profile_elem in response.findall('Profiles'):
                profile = {
                    'token': profile_elem.find('token').text if profile_elem.find('token') is not None else '',
                    'name': profile_elem.find('Name').text if profile_elem.find('Name') is not None else '',
                    'fixed': profile_elem.find('fixed').text == 'true' if profile_elem.find('fixed') is not None else False,
                    'video_source_token': '',
                    'video_encoder_token': '',
                    'audio_source_token': '',
                    'audio_encoder_token': ''
                }

                # Extract configuration tokens
                video_source = profile_elem.find('VideoSourceConfiguration')
                if video_source is not None:
                    profile['video_source_token'] = video_source.find('SourceToken').text if video_source.find('SourceToken') is not None else ''

                video_encoder = profile_elem.find('VideoEncoderConfiguration')
                if video_encoder is not None:
                    profile['video_encoder_token'] = video_encoder.find('token').text if video_encoder.find('token') is not None else ''

                audio_source = profile_elem.find('AudioSourceConfiguration')
                if audio_source is not None:
                    profile['audio_source_token'] = audio_source.find('SourceToken').text if audio_source.find('SourceToken') is not None else ''

                audio_encoder = profile_elem.find('AudioEncoderConfiguration')
                if audio_encoder is not None:
                    profile['audio_encoder_token'] = audio_encoder.find('token').text if audio_encoder.find('token') is not None else ''

                profiles.append(profile)

            return profiles
        except Exception as e:
            pytest.fail(f"Failed to parse profiles response: {e}")

    def parse_stream_uri(self, response_xml: str) -> str:
        """Parse GetStreamUri response"""
        try:
            root = ET.fromstring(response_xml)

            for elem in root.iter():
                if '}' in elem.tag:
                    elem.tag = elem.tag.split('}', 1)[1]

            body = root.find('Body')
            if body is None:
                raise ValueError("No Body element found")

            response = body.find('GetStreamUriResponse')
            if response is None:
                raise ValueError("No GetStreamUriResponse found")

            media_uri = response.find('MediaUri')
            if media_uri is None:
                raise ValueError("No MediaUri found")

            uri = media_uri.find('Uri')
            if uri is None:
                raise ValueError("No Uri found")

            return uri.text if uri.text else ''
        except Exception as e:
            pytest.fail(f"Failed to parse stream URI response: {e}")

    def parse_video_sources(self, response_xml: str) -> List[Dict[str, Any]]:
        """Parse GetVideoSources response"""
        try:
            root = ET.fromstring(response_xml)

            for elem in root.iter():
                if '}' in elem.tag:
                    elem.tag = elem.tag.split('}', 1)[1]

            body = root.find('Body')
            if body is None:
                raise ValueError("No Body element found")

            response = body.find('GetVideoSourcesResponse')
            if response is None:
                raise ValueError("No GetVideoSourcesResponse found")

            sources = []
            for source_elem in response.findall('VideoSources'):
                source = {
                    'token': source_elem.find('token').text if source_elem.find('token') is not None else '',
                    'framerate': source_elem.find('Framerate').text if source_elem.find('Framerate') is not None else '',
                    'resolution_width': source_elem.find('Resolution').find('Width').text if source_elem.find('Resolution') and source_elem.find('Resolution').find('Width') is not None else '',
                    'resolution_height': source_elem.find('Resolution').find('Height').text if source_elem.find('Resolution') and source_elem.find('Resolution').find('Height') is not None else ''
                }
                sources.append(source)

            return sources
        except Exception as e:
            pytest.fail(f"Failed to parse video sources response: {e}")


@pytest.fixture
def media_client(device_config):
    """Provide ONVIF Media service client"""
    return ONVIFMediaClient(device_config.get_onvif_media_url())


class TestONVIFMediaService:
    """ONVIF Media service E2E tests"""

    @pytest.mark.onvif_media
    @pytest.mark.integration
    def test_get_profiles(self, media_client: ONVIFMediaClient):
        """Test GetProfiles operation"""
        soap_request = media_client.create_get_profiles_request()

        response = make_soap_request(
            media_client.media_url,
            soap_request,
            timeout=30
        )

        content = validate_xml_response(response, 200)

        # Parse profiles
        profiles = media_client.parse_profiles(content)

        # Verify we have at least one profile
        assert len(profiles) > 0, "No media profiles found"

        # Verify profile structure
        for profile in profiles:
            assert profile['token'], f"Profile missing token: {profile}"
            assert profile['name'], f"Profile missing name: {profile}"

    @pytest.mark.onvif_media
    @pytest.mark.integration
    def test_get_stream_uri(self, media_client: ONVIFMediaClient):
        """Test GetStreamUri operation"""
        # First get profiles to find a profile token
        soap_request = media_client.create_get_profiles_request()

        response = make_soap_request(
            media_client.media_url,
            soap_request,
            timeout=30
        )

        content = validate_xml_response(response, 200)
        profiles = media_client.parse_profiles(content)

        assert len(profiles) > 0, "No profiles available for stream URI test"

        # Use the first profile
        profile_token = profiles[0]['token']
        assert profile_token, "First profile missing token"

        # Get stream URI for the profile
        soap_request = media_client.create_get_stream_uri_request(profile_token)

        response = make_soap_request(
            media_client.media_url,
            soap_request,
            timeout=30
        )

        content = validate_xml_response(response, 200)

        # Parse stream URI
        stream_uri = media_client.parse_stream_uri(content)

        # Verify stream URI format
        assert stream_uri, "Stream URI not provided"
        assert stream_uri.startswith('rtsp://'), f"Invalid stream URI format: {stream_uri}"

        # Verify URI contains expected elements
        assert 'rtsp://' in stream_uri, "Stream URI should use RTSP protocol"

    @pytest.mark.onvif_media
    @pytest.mark.integration
    def test_get_video_sources(self, media_client: ONVIFMediaClient):
        """Test GetVideoSources operation"""
        soap_request = media_client.create_get_video_sources_request()

        response = make_soap_request(
            media_client.media_url,
            soap_request,
            timeout=30
        )

        content = validate_xml_response(response, 200)

        # Parse video sources
        video_sources = media_client.parse_video_sources(content)

        # Verify we have at least one video source
        assert len(video_sources) > 0, "No video sources found"

        # Verify video source structure
        for source in video_sources:
            assert source['token'], f"Video source missing token: {source}"
            assert source['framerate'], f"Video source missing framerate: {source}"
            assert source['resolution_width'], f"Video source missing resolution width: {source}"
            assert source['resolution_height'], f"Video source missing resolution height: {source}"

            # Verify reasonable values
            width = int(source['resolution_width'])
            height = int(source['resolution_height'])
            assert 100 <= width <= 4000, f"Invalid video width: {width}"
            assert 100 <= height <= 4000, f"Invalid video height: {height}"

    @pytest.mark.onvif_media
    @pytest.mark.integration
    def test_get_audio_sources(self, media_client: ONVIFMediaClient):
        """Test GetAudioSources operation"""
        soap_request = media_client.create_get_audio_sources_request()

        response = make_soap_request(
            media_client.media_url,
            soap_request,
            timeout=30
        )

        content = validate_xml_response(response, 200)

        # Audio sources might not be present on all devices
        if 'GetAudioSourcesResponse' in content:
            assert 'AudioSources' in content, "Response should contain AudioSources"

    @pytest.mark.onvif_media
    @pytest.mark.integration
    def test_get_video_source_configurations(self, media_client: ONVIFMediaClient):
        """Test GetVideoSourceConfigurations operation"""
        soap_request = media_client.create_get_video_source_configurations_request()

        response = make_soap_request(
            media_client.media_url,
            soap_request,
            timeout=30
        )

        content = validate_xml_response(response, 200)

        # Verify response contains configuration information
        assert 'GetVideoSourceConfigurationsResponse' in content, "Response should contain GetVideoSourceConfigurationsResponse"

    @pytest.mark.onvif_media
    @pytest.mark.integration
    @retry_on_failure(max_attempts=3, delay=2.0)
    def test_media_service_availability(self, media_client: ONVIFMediaClient):
        """Test that Media service is available and responding"""
        soap_request = media_client.create_get_profiles_request()

        response = make_soap_request(
            media_client.media_url,
            soap_request,
            timeout=30
        )

        assert response.status_code == 200, f"Media service not available, status: {response.status_code}"

        content = response.text
        assert 'GetProfilesResponse' in content, "Response should contain profiles"

    @pytest.mark.onvif_media
    @pytest.mark.integration
    def test_multiple_profiles_consistency(self, media_client: ONVIFMediaClient):
        """Test that multiple profile requests return consistent data"""
        results = []

        # Make multiple requests
        for i in range(3):
            soap_request = media_client.create_get_profiles_request()

            response = make_soap_request(
                media_client.media_url,
                soap_request,
                timeout=30
            )

            content = validate_xml_response(response, 200)
            profiles = media_client.parse_profiles(content)
            results.append(profiles)

        # Verify all responses have the same number of profiles
        profile_counts = [len(r) for r in results]
        assert all(count == profile_counts[0] for count in profile_counts), \
            f"Inconsistent profile counts: {profile_counts}"

        # Verify profile tokens are consistent
        first_profile_tokens = [p['token'] for p in results[0]]
        for profiles in results[1:]:
            tokens = [p['token'] for p in profiles]
            assert tokens == first_profile_tokens, "Profile tokens inconsistent across requests"

    @pytest.mark.onvif_media
    @pytest.mark.integration
    def test_stream_uri_format_validation(self, media_client: ONVIFMediaClient):
        """Test that stream URIs have correct format and are accessible"""
        # Get profiles first
        soap_request = media_client.create_get_profiles_request()

        response = make_soap_request(
            media_client.media_url,
            soap_request,
            timeout=30
        )

        content = validate_xml_response(response, 200)
        profiles = media_client.parse_profiles(content)

        if len(profiles) == 0:
            pytest.skip("No profiles available for stream URI format test")

        # Test stream URI for each profile
        for profile in profiles:
            if not profile['token']:
                continue

            soap_request = media_client.create_get_stream_uri_request(profile['token'])

            response = make_soap_request(
                media_client.media_url,
                soap_request,
                timeout=30
            )

            content = validate_xml_response(response, 200)
            stream_uri = media_client.parse_stream_uri(content)

            # Validate RTSP URI format
            assert stream_uri.startswith('rtsp://'), f"Invalid RTSP URI format: {stream_uri}"

            # Check for required RTSP elements
            assert 'rtsp://' in stream_uri, "Stream URI must use RTSP protocol"

            # Parse RTSP URL components
            if '://' in stream_uri:
                protocol, rest = stream_uri.split('://', 1)
                assert protocol == 'rtsp', f"Expected RTSP protocol, got {protocol}"

                if '/' in rest:
                    host_port, path = rest.split('/', 1)
                    assert host_port, "Host/port missing in RTSP URI"
                    assert path.startswith('/'), "RTSP path should start with /"

    @pytest.mark.onvif_media
    @pytest.mark.integration
    @pytest.mark.slow
    def test_media_service_performance(self, media_client: ONVIFMediaClient):
        """Test Media service response time"""
        import time

        # Measure response time for profile requests
        response_times = []

        for i in range(5):
            start_time = time.time()

            soap_request = media_client.create_get_profiles_request()
            response = make_soap_request(
                media_client.media_url,
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

    @pytest.mark.onvif_media
    @pytest.mark.integration
    def test_profile_configuration_completeness(self, media_client: ONVIFMediaClient):
        """Test that profiles have complete configuration information"""
        soap_request = media_client.create_get_profiles_request()

        response = make_soap_request(
            media_client.media_url,
            soap_request,
            timeout=30
        )

        content = validate_xml_response(response, 200)
        profiles = media_client.parse_profiles(content)

        for profile in profiles:
            # Check for essential configuration tokens
            assert profile['video_source_token'], f"Profile {profile['name']} missing video source token"
            assert profile['video_encoder_token'], f"Profile {profile['name']} missing video encoder token"

            # Verify token formats (should be non-empty strings)
            assert isinstance(profile['token'], str) and len(profile['token']) > 0, \
                f"Invalid profile token: {profile['token']}"
            assert isinstance(profile['name'], str) and len(profile['name']) > 0, \
                f"Invalid profile name: {profile['name']}"
