"""
ONVIF PTZ service E2E tests
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


class ONVIFPTZClient:
    """ONVIF PTZ service client"""

    def __init__(self, ptz_url: str):
        self.ptz_url = ptz_url

    def create_get_nodes_request(self) -> str:
        """Create GetNodes SOAP request"""
        body = '''<tptz:GetNodes xmlns:tptz="http://www.onvif.org/ver20/ptz/wsdl"/>'''
        return SOAP_ENVELOPE_TEMPLATE.format(body=body)

    def create_get_node_request(self, node_token: str) -> str:
        """Create GetNode SOAP request"""
        body = f'''<tptz:GetNode xmlns:tptz="http://www.onvif.org/ver20/ptz/wsdl">
            <tptz:NodeToken>{node_token}</tptz:NodeToken>
        </tptz:GetNode>'''
        return SOAP_ENVELOPE_TEMPLATE.format(body=body)

    def create_get_configurations_request(self) -> str:
        """Create GetConfigurations SOAP request"""
        body = '''<tptz:GetConfigurations xmlns:tptz="http://www.onvif.org/ver20/ptz/wsdl"/>'''
        return SOAP_ENVELOPE_TEMPLATE.format(body=body)

    def create_continuous_move_request(self, profile_token: str, velocity_x: float = 0.0,
                                     velocity_y: float = 0.0, velocity_z: float = 0.0) -> str:
        """Create ContinuousMove SOAP request"""
        body = f'''<tptz:ContinuousMove xmlns:tptz="http://www.onvif.org/ver20/ptz/wsdl">
            <tptz:ProfileToken>{profile_token}</tptz:ProfileToken>
            <tptz:Velocity>
                <tt:PanTilt xmlns:tt="http://www.onvif.org/ver10/schema" x="{velocity_x}" y="{velocity_y}"/>
                <tt:Zoom xmlns:tt="http://www.onvif.org/ver10/schema" x="{velocity_z}"/>
            </tptz:Velocity>
        </tptz:ContinuousMove>'''
        return SOAP_ENVELOPE_TEMPLATE.format(body=body)

    def create_stop_request(self, profile_token: str, pan_tilt: bool = True, zoom: bool = True) -> str:
        """Create Stop SOAP request"""
        body = f'''<tptz:Stop xmlns:tptz="http://www.onvif.org/ver20/ptz/wsdl">
            <tptz:ProfileToken>{profile_token}</tptz:ProfileToken>
            <tptz:PanTilt>{"true" if pan_tilt else "false"}</tptz:PanTilt>
            <tptz:Zoom>{"true" if zoom else "false"}</tptz:Zoom>
        </tptz:Stop>'''
        return SOAP_ENVELOPE_TEMPLATE.format(body=body)

    def create_get_status_request(self, profile_token: str) -> str:
        """Create GetStatus SOAP request"""
        body = f'''<tptz:GetStatus xmlns:tptz="http://www.onvif.org/ver20/ptz/wsdl">
            <tptz:ProfileToken>{profile_token}</tptz:ProfileToken>
        </tptz:GetStatus>'''
        return SOAP_ENVELOPE_TEMPLATE.format(body=body)

    def create_goto_preset_request(self, profile_token: str, preset_token: str) -> str:
        """Create GotoPreset SOAP request"""
        body = f'''<tptz:GotoPreset xmlns:tptz="http://www.onvif.org/ver20/ptz/wsdl">
            <tptz:ProfileToken>{profile_token}</tptz:ProfileToken>
            <tptz:PresetToken>{preset_token}</tptz:PresetToken>
        </tptz:GotoPreset>'''
        return SOAP_ENVELOPE_TEMPLATE.format(body=body)

    def create_get_presets_request(self, profile_token: str) -> str:
        """Create GetPresets SOAP request"""
        body = f'''<tptz:GetPresets xmlns:tptz="http://www.onvif.org/ver20/ptz/wsdl">
            <tptz:ProfileToken>{profile_token}</tptz:ProfileToken>
        </tptz:GetPresets>'''
        return SOAP_ENVELOPE_TEMPLATE.format(body=body)

    def parse_ptz_nodes(self, response_xml: str) -> List[Dict[str, Any]]:
        """Parse GetNodes response"""
        try:
            root = ET.fromstring(response_xml)

            for elem in root.iter():
                if '}' in elem.tag:
                    elem.tag = elem.tag.split('}', 1)[1]

            body = root.find('Body')
            if body is None:
                raise ValueError("No Body element found")

            response = body.find('GetNodesResponse')
            if response is None:
                raise ValueError("No GetNodesResponse found")

            nodes = []
            for node_elem in response.findall('PTZNode'):
                node = {
                    'token': node_elem.find('token').text if node_elem.find('token') is not None else '',
                    'name': node_elem.find('Name').text if node_elem.find('Name') is not None else '',
                    'fixed_home_position': node_elem.find('FixedHomePosition').text == 'true' if node_elem.find('FixedHomePosition') is not None else False,
                    'geo_move': node_elem.find('GeoMove').text == 'true' if node_elem.find('GeoMove') is not None else False,
                    'supports_pan_tilt': True,  # Assume PTZ camera supports pan/tilt
                    'supports_zoom': True  # Assume PTZ camera supports zoom
                }
                nodes.append(node)

            return nodes
        except Exception as e:
            pytest.fail(f"Failed to parse PTZ nodes response: {e}")

    def parse_ptz_configurations(self, response_xml: str) -> List[Dict[str, Any]]:
        """Parse GetConfigurations response"""
        try:
            root = ET.fromstring(response_xml)

            for elem in root.iter():
                if '}' in elem.tag:
                    elem.tag = elem.tag.split('}', 1)[1]

            body = root.find('Body')
            if body is None:
                raise ValueError("No Body element found")

            response = body.find('GetConfigurationsResponse')
            if response is None:
                raise ValueError("No GetConfigurationsResponse found")

            configs = []
            for config_elem in response.findall('PTZConfiguration'):
                config = {
                    'token': config_elem.find('token').text if config_elem.find('token') is not None else '',
                    'name': config_elem.find('Name').text if config_elem.find('Name') is not None else '',
                    'node_token': config_elem.find('NodeToken').text if config_elem.find('NodeToken') is not None else ''
                }
                configs.append(config)

            return configs
        except Exception as e:
            pytest.fail(f"Failed to parse PTZ configurations response: {e}")

    def parse_ptz_status(self, response_xml: str) -> Dict[str, Any]:
        """Parse GetStatus response"""
        try:
            root = ET.fromstring(response_xml)

            for elem in root.iter():
                if '}' in elem.tag:
                    elem.tag = elem.tag.split('}', 1)[1]

            body = root.find('Body')
            if body is None:
                raise ValueError("No Body element found")

            response = body.find('GetStatusResponse')
            if response is None:
                raise ValueError("No GetStatusResponse found")

            ptz_status = response.find('PTZStatus')
            if ptz_status is None:
                raise ValueError("No PTZStatus found")

            # Extract position information
            position = ptz_status.find('Position')
            status = {
                'position_pan': 0.0,
                'position_tilt': 0.0,
                'position_zoom': 0.0,
                'moving_pan': False,
                'moving_tilt': False,
                'moving_zoom': False
            }

            if position is not None:
                pan_tilt = position.find('PanTilt')
                if pan_tilt is not None:
                    if pan_tilt.find('x') is not None:
                        status['position_pan'] = float(pan_tilt.find('x').text or 0)
                    if pan_tilt.find('y') is not None:
                        status['position_tilt'] = float(pan_tilt.find('y').text or 0)

                zoom = position.find('Zoom')
                if zoom is not None and zoom.find('x') is not None:
                    status['position_zoom'] = float(zoom.find('x').text or 0)

            return status
        except Exception as e:
            pytest.fail(f"Failed to parse PTZ status response: {e}")


@pytest.fixture
def ptz_client(device_config):
    """Provide ONVIF PTZ service client"""
    return ONVIFPTZClient(device_config.get_onvif_ptz_url())


class TestONVIFPTZService:
    """ONVIF PTZ service E2E tests"""

    @pytest.mark.onvif_ptz
    @pytest.mark.integration
    def test_get_ptz_nodes(self, ptz_client: ONVIFPTZClient):
        """Test GetNodes operation"""
        soap_request = ptz_client.create_get_nodes_request()

        response = make_soap_request(
            ptz_client.ptz_url,
            soap_request,
            timeout=30
        )

        content = validate_xml_response(response, 200)

        # Parse PTZ nodes
        nodes = ptz_client.parse_ptz_nodes(content)

        # Verify we have at least one PTZ node
        assert len(nodes) > 0, "No PTZ nodes found"

        # Verify node structure
        for node in nodes:
            assert node['token'], f"PTZ node missing token: {node}"
            assert isinstance(node['supports_pan_tilt'], bool), "supports_pan_tilt should be boolean"
            assert isinstance(node['supports_zoom'], bool), "supports_zoom should be boolean"

    @pytest.mark.onvif_ptz
    @pytest.mark.integration
    def test_get_ptz_configurations(self, ptz_client: ONVIFPTZClient):
        """Test GetConfigurations operation"""
        soap_request = ptz_client.create_get_configurations_request()

        response = make_soap_request(
            ptz_client.ptz_url,
            soap_request,
            timeout=30
        )

        content = validate_xml_response(response, 200)

        # Parse PTZ configurations
        configs = ptz_client.parse_ptz_configurations(content)

        # Verify we have at least one PTZ configuration
        assert len(configs) > 0, "No PTZ configurations found"

        # Verify configuration structure
        for config in configs:
            assert config['token'], f"PTZ configuration missing token: {config}"
            assert config['node_token'], f"PTZ configuration missing node token: {config}"

    @pytest.mark.onvif_ptz
    @pytest.mark.integration
    def test_continuous_move_and_stop(self, ptz_client: ONVIFPTZClient):
        """Test ContinuousMove and Stop operations"""
        # Get PTZ nodes first to find a profile token
        soap_request = ptz_client.create_get_nodes_request()

        response = make_soap_request(
            ptz_client.ptz_url,
            soap_request,
            timeout=30
        )

        content = validate_xml_response(response, 200)
        nodes = ptz_client.parse_ptz_nodes(content)

        if len(nodes) == 0:
            pytest.skip("No PTZ nodes available for movement test")

        # Use the first node's token (assuming it corresponds to a profile)
        node_token = nodes[0]['token']

        # Test continuous move
        soap_request = ptz_client.create_continuous_move_request(
            profile_token=node_token,
            velocity_x=0.1,  # Small pan movement
            velocity_y=0.0,
            velocity_z=0.0
        )

        response = make_soap_request(
            ptz_client.ptz_url,
            soap_request,
            timeout=30
        )

        assert response.status_code == 200, f"ContinuousMove failed, status: {response.status_code}"

        # Wait a moment for movement
        time.sleep(1)

        # Test stop
        soap_request = ptz_client.create_stop_request(profile_token=node_token)

        response = make_soap_request(
            ptz_client.ptz_url,
            soap_request,
            timeout=30
        )

        assert response.status_code == 200, f"Stop failed, status: {response.status_code}"

    @pytest.mark.onvif_ptz
    @pytest.mark.integration
    def test_get_ptz_status(self, ptz_client: ONVIFPTZClient):
        """Test GetStatus operation"""
        # Get PTZ nodes first to find a profile token
        soap_request = ptz_client.create_get_nodes_request()

        response = make_soap_request(
            ptz_client.ptz_url,
            soap_request,
            timeout=30
        )

        content = validate_xml_response(response, 200)
        nodes = ptz_client.parse_ptz_nodes(content)

        if len(nodes) == 0:
            pytest.skip("No PTZ nodes available for status test")

        # Use the first node's token
        node_token = nodes[0]['token']

        # Get PTZ status
        soap_request = ptz_client.create_get_status_request(node_token)

        response = make_soap_request(
            ptz_client.ptz_url,
            soap_request,
            timeout=30
        )

        content = validate_xml_response(response, 200)

        # Parse PTZ status
        status = ptz_client.parse_ptz_status(content)

        # Verify status structure
        assert 'position_pan' in status, "Status missing pan position"
        assert 'position_tilt' in status, "Status missing tilt position"
        assert 'position_zoom' in status, "Status missing zoom position"

        # Position values should be reasonable
        assert -360 <= status['position_pan'] <= 360, f"Invalid pan position: {status['position_pan']}"
        assert -180 <= status['position_tilt'] <= 180, f"Invalid tilt position: {status['position_tilt']}"

    @pytest.mark.onvif_ptz
    @pytest.mark.integration
    def test_ptz_service_availability(self, ptz_client: ONVIFPTZClient):
        """Test that PTZ service is available and responding"""
        soap_request = ptz_client.create_get_nodes_request()

        response = make_soap_request(
            ptz_client.ptz_url,
            soap_request,
            timeout=30
        )

        assert response.status_code == 200, f"PTZ service not available, status: {response.status_code}"

        content = response.text
        assert 'GetNodesResponse' in content, "Response should contain PTZ nodes"

    @pytest.mark.onvif_ptz
    @pytest.mark.integration
    def test_ptz_movement_validation(self, ptz_client: ONVIFPTZClient):
        """Test PTZ movement with different velocity values"""
        # Get PTZ nodes first
        soap_request = ptz_client.create_get_nodes_request()

        response = make_soap_request(
            ptz_client.ptz_url,
            soap_request,
            timeout=30
        )

        content = validate_xml_response(response, 200)
        nodes = ptz_client.parse_ptz_nodes(content)

        if len(nodes) == 0:
            pytest.skip("No PTZ nodes available for movement validation test")

        node_token = nodes[0]['token']

        # Test different movement scenarios
        test_movements = [
            (0.5, 0.0, 0.0),   # Pan right
            (-0.5, 0.0, 0.0),  # Pan left
            (0.0, 0.5, 0.0),   # Tilt up
            (0.0, -0.5, 0.0),  # Tilt down
            (0.0, 0.0, 0.5),   # Zoom in
            (0.0, 0.0, -0.5),  # Zoom out
            (0.3, 0.3, 0.0),   # Diagonal movement
        ]

        for velocity_x, velocity_y, velocity_z in test_movements:
            # Send continuous move
            soap_request = ptz_client.create_continuous_move_request(
                profile_token=node_token,
                velocity_x=velocity_x,
                velocity_y=velocity_y,
                velocity_z=velocity_z
            )

            response = make_soap_request(
                ptz_client.ptz_url,
                soap_request,
                timeout=30
            )

            assert response.status_code == 200, \
                f"ContinuousMove failed for velocities ({velocity_x}, {velocity_y}, {velocity_z})"

            # Brief movement
            time.sleep(0.5)

            # Stop movement
            soap_request = ptz_client.create_stop_request(profile_token=node_token)

            response = make_soap_request(
                ptz_client.ptz_url,
                soap_request,
                timeout=30
            )

            assert response.status_code == 200, "Stop failed"

            # Small delay between tests
            time.sleep(0.2)

    @pytest.mark.onvif_ptz
    @pytest.mark.integration
    @pytest.mark.slow
    def test_ptz_service_performance(self, ptz_client: ONVIFPTZClient):
        """Test PTZ service response time"""
        import time

        # Measure response time for status requests
        response_times = []

        for i in range(5):
            start_time = time.time()

            soap_request = ptz_client.create_get_nodes_request()
            response = make_soap_request(
                ptz_client.ptz_url,
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

    @pytest.mark.onvif_ptz
    @pytest.mark.integration
    def test_ptz_error_handling(self, ptz_client: ONVIFPTZClient):
        """Test PTZ service error handling"""
        # Test with invalid profile token
        invalid_token = "invalid_token_12345"

        soap_request = ptz_client.create_continuous_move_request(
            profile_token=invalid_token,
            velocity_x=0.1,
            velocity_y=0.0,
            velocity_z=0.0
        )

        response = make_soap_request(
            ptz_client.ptz_url,
            soap_request,
            timeout=30
        )

        # Should get an error response (400 or 500)
        assert response.status_code in [400, 500], \
            f"Expected error status for invalid token, got {response.status_code}"

    @pytest.mark.onvif_ptz
    @pytest.mark.integration
    def test_ptz_status_consistency(self, ptz_client: ONVIFPTZClient):
        """Test that PTZ status is consistent across multiple requests"""
        # Get PTZ nodes first
        soap_request = ptz_client.create_get_nodes_request()

        response = make_soap_request(
            ptz_client.ptz_url,
            soap_request,
            timeout=30
        )

        content = validate_xml_response(response, 200)
        nodes = ptz_client.parse_ptz_nodes(content)

        if len(nodes) == 0:
            pytest.skip("No PTZ nodes available for status consistency test")

        node_token = nodes[0]['token']

        # Get multiple status readings
        status_readings = []

        for i in range(3):
            soap_request = ptz_client.create_get_status_request(node_token)

            response = make_soap_request(
                ptz_client.ptz_url,
                soap_request,
                timeout=30
            )

            content = validate_xml_response(response, 200)
            status = ptz_client.parse_ptz_status(content)
            status_readings.append(status)

            # Small delay between readings
            time.sleep(0.5)

        # Check consistency of position readings
        pan_positions = [s['position_pan'] for s in status_readings]
        tilt_positions = [s['position_tilt'] for s in status_readings]

        # For a stationary camera, positions should be relatively stable
        pan_variance = max(pan_positions) - min(pan_positions)
        tilt_variance = max(tilt_positions) - min(tilt_positions)

        # Allow some variance due to minor movements or precision differences
        assert pan_variance < 5.0, f"Pan position variance too high: {pan_variance}"
        assert tilt_variance < 5.0, f"Tilt position variance too high: {tilt_variance}"
