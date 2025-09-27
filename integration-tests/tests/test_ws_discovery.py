"""
WS-Discovery integration tests for ONVIF devices
"""
import socket
import struct
import time
import pytest
from typing import List, Dict, Any
from scapy.all import IP, UDP, Raw, send, sniff

from .fixtures import (
    ws_discovery_config,
    retry_on_failure,
    SOAP_ENVELOPE_TEMPLATE,
    ONVIF_NAMESPACES
)


class WSDiscoveryClient:
    """WS-Discovery client for testing device discovery"""

    def __init__(self, config: Dict[str, Any]):
        self.multicast_address = config['multicast_address']
        self.multicast_port = config['multicast_port']
        self.timeout = config['timeout']
        self.device_ip = config['device_ip']

    def create_hello_message(self) -> str:
        """Create a WS-Discovery Hello message"""
        message_id = "urn:uuid:" + socket.gethostname() + "-hello"

        body = f'''<wsdd:Hello xmlns:wsdd="http://schemas.xmlsoap.org/ws/2005/04/discovery">
            <wsa:EndpointReference xmlns:wsa="http://schemas.xmlsoap.org/ws/2004/08/addressing">
                <wsa:Address>{message_id}</wsa:Address>
            </wsa:EndpointReference>
            <wsdd:Types>dn:NetworkVideoTransmitter</wsdd:Types>
            <wsdd:XAddrs>http://{self.device_ip}:8080/onvif/device_service</wsdd:XAddrs>
            <wsdd:MetadataVersion>1</wsdd:MetadataVersion>
        </wsdd:Hello>'''

        return SOAP_ENVELOPE_TEMPLATE.format(body=body)

    def create_probe_message(self) -> str:
        """Create a WS-Discovery Probe message"""
        message_id = "urn:uuid:" + socket.gethostname() + "-probe"

        body = f'''<wsdd:Probe xmlns:wsdd="http://schemas.xmlsoap.org/ws/2005/04/discovery">
            <wsa:EndpointReference xmlns:wsa="http://schemas.xmlsoap.org/ws/2004/08/addressing">
                <wsa:Address>{message_id}</wsa:Address>
            </wsa:EndpointReference>
            <wsdd:Types>dn:NetworkVideoTransmitter</wsdd:Types>
        </wsdd:Probe>'''

        return SOAP_ENVELOPE_TEMPLATE.format(body=body)

    def send_multicast(self, message: str) -> None:
        """Send a multicast message"""
        sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM, socket.IPPROTO_UDP)
        sock.setsockopt(socket.IPPROTO_IP, socket.IP_MULTICAST_TTL, 1)

        try:
            sock.sendto(message.encode(), (self.multicast_address, self.multicast_port))
        finally:
            sock.close()

    def listen_for_responses(self, duration: float = 5.0) -> List[bytes]:
        """Listen for multicast responses"""
        responses = []

        # Create socket for listening
        sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM, socket.IPPROTO_UDP)
        sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)

        # Bind to all interfaces
        sock.bind(('', self.multicast_port))

        # Join multicast group
        mreq = struct.pack("4sl", socket.inet_aton(self.multicast_address), socket.INADDR_ANY)
        sock.setsockopt(socket.IPPROTO_IP, socket.IP_ADD_MEMBERSHIP, mreq)

        # Set timeout
        sock.settimeout(1.0)

        start_time = time.time()
        while time.time() - start_time < duration:
            try:
                data, addr = sock.recvfrom(4096)
                responses.append(data)
            except socket.timeout:
                continue

        sock.close()
        return responses


@pytest.fixture
def ws_client(ws_discovery_config):
    """Provide WS-Discovery client"""
    return WSDiscoveryClient(ws_discovery_config)


class TestWSDiscovery:
    """WS-Discovery integration tests"""

    @pytest.mark.ws_discovery
    @pytest.mark.integration
    @pytest.mark.slow
    def test_device_responds_to_hello(self, ws_client: WSDiscoveryClient):
        """Test that device responds to Hello messages"""
        # Send Hello message
        hello_msg = ws_client.create_hello_message()
        ws_client.send_multicast(hello_msg)

        # Listen for responses
        responses = ws_client.listen_for_responses(duration=3.0)

        # Verify we got at least one response
        assert len(responses) > 0, "No responses received to Hello message"

        # Check response content
        response_text = responses[0].decode('utf-8', errors='ignore')

        # Should contain Hello response elements
        assert 'wsdd:Hello' in response_text, "Response does not contain Hello element"
        assert ws_client.device_ip in response_text, f"Response does not contain device IP {ws_client.device_ip}"

    @pytest.mark.ws_discovery
    @pytest.mark.integration
    @pytest.mark.slow
    def test_device_responds_to_probe(self, ws_client: WSDiscoveryClient):
        """Test that device responds to Probe messages"""
        # Send Probe message
        probe_msg = ws_client.create_probe_message()
        ws_client.send_multicast(probe_msg)

        # Listen for responses
        responses = ws_client.listen_for_responses(duration=3.0)

        # Verify we got at least one response
        assert len(responses) > 0, "No responses received to Probe message"

        # Check response content
        response_text = responses[0].decode('utf-8', errors='ignore')

        # Should contain ProbeMatch response elements
        assert 'wsdd:ProbeMatches' in response_text or 'wsdd:ProbeMatch' in response_text, \
            "Response does not contain ProbeMatch element"
        assert ws_client.device_ip in response_text, f"Response does not contain device IP {ws_client.device_ip}"

    @pytest.mark.ws_discovery
    @pytest.mark.integration
    @pytest.mark.slow
    def test_device_discovery_multiple_probes(self, ws_client: WSDiscoveryClient):
        """Test device responds consistently to multiple probes"""
        response_times = []

        for i in range(3):
            # Send Probe message
            probe_msg = ws_client.create_probe_message()
            ws_client.send_multicast(probe_msg)

            # Listen for responses
            start_time = time.time()
            responses = ws_client.listen_for_responses(duration=2.0)
            response_time = time.time() - start_time
            response_times.append(response_time)

            # Verify response
            assert len(responses) > 0, f"No response received to probe {i+1}"

            # Small delay between probes
            time.sleep(0.5)

        # Verify consistent response times
        avg_response_time = sum(response_times) / len(response_times)
        assert avg_response_time < 1.0, f"Average response time too slow: {avg_response_time:.2f}s"

    @pytest.mark.ws_discovery
    @pytest.mark.integration
    @pytest.mark.slow
    def test_discovery_message_format(self, ws_client: WSDiscoveryClient):
        """Test that discovery response has correct format"""
        # Send Probe message
        probe_msg = ws_client.create_probe_message()
        ws_client.send_multicast(probe_msg)

        # Listen for responses
        responses = ws_client.listen_for_responses(duration=3.0)

        assert len(responses) > 0, "No responses received"

        response_text = responses[0].decode('utf-8', errors='ignore')

        # Check for required elements
        required_elements = [
            'wsdd:ProbeMatches',
            'wsdd:ProbeMatch',
            'wsa:EndpointReference',
            'wsa:Address',
            'wsdd:Types',
            'wsdd:XAddrs',
            'wsdd:MetadataVersion'
        ]

        for element in required_elements:
            assert element in response_text, f"Response missing required element: {element}"

        # Check for device type
        assert 'NetworkVideoTransmitter' in response_text, "Response missing device type"

    @pytest.mark.ws_discovery
    @pytest.mark.integration
    @pytest.mark.slow
    @retry_on_failure(max_attempts=3, delay=2.0)
    def test_device_periodic_announcements(self, ws_client: WSDiscoveryClient):
        """Test that device sends periodic Hello announcements"""
        # Listen for periodic announcements
        responses = ws_client.listen_for_responses(duration=10.0)

        # Should receive at least one Hello message (device announcing itself)
        hello_messages = [r for r in responses if b'wsdd:Hello' in r]

        # We expect at least one Hello message from the device itself
        # (device announcing its presence)
        assert len(hello_messages) > 0, "No Hello messages received from device"

    @pytest.mark.ws_discovery
    @pytest.mark.integration
    @pytest.mark.network
    def test_multicast_group_membership(self, ws_client: WSDiscoveryClient):
        """Test multicast group membership and message delivery"""
        # Send multiple messages
        messages = []
        for i in range(3):
            probe_msg = ws_client.create_probe_message()
            messages.append(probe_msg)
            ws_client.send_multicast(probe_msg)
            time.sleep(0.1)  # Small delay

        # Listen for all responses
        responses = ws_client.listen_for_responses(duration=5.0)

        # Should receive responses to our probes
        assert len(responses) > 0, "No responses received to multicast messages"

    @pytest.mark.ws_discovery
    @pytest.mark.integration
    @pytest.mark.network
    def test_discovery_network_reachability(self, ws_client: WSDiscoveryClient):
        """Test that device is reachable via network for discovery"""
        # Create a UDP socket to test basic connectivity
        sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)

        try:
            # Try to send a simple message to the device
            test_message = b"TEST_DISCOVERY"
            sock.sendto(test_message, (ws_client.device_ip, ws_client.multicast_port))

            # Listen for any response
            sock.settimeout(2.0)
            try:
                data, addr = sock.recvfrom(1024)
                # If we get any response, network is working
                assert addr[0] == ws_client.device_ip or '239.255.255.250' in addr[0], \
                    "Unexpected response source"
            except socket.timeout:
                # Timeout is OK - device might not respond to unknown messages
                pass

        finally:
            sock.close()

    @pytest.mark.ws_discovery
    @pytest.mark.integration
    @pytest.mark.slow
    def test_discovery_service_availability(self, ws_client: WSDiscoveryClient, http_session):
        """Test that discovered services are actually available"""
        # First discover the device
        probe_msg = ws_client.create_probe_message()
        ws_client.send_multicast(probe_msg)

        responses = ws_client.listen_for_responses(duration=3.0)

        if len(responses) == 0:
            pytest.skip("No discovery responses received")

        # Extract service URLs from response
        response_text = responses[0].decode('utf-8', errors='ignore')

        # Look for XAddrs in response
        xaddrs_start = response_text.find('wsdd:XAddrs') + len('wsdd:XAddrs>')
        xaddrs_end = response_text.find('</wsdd:XAddrs>', xaddrs_start)
        xaddrs = response_text[xaddrs_start:xaddrs_end].strip()

        # Test that the discovered URL is accessible
        try:
            response = http_session.get(
                xaddrs,
                timeout=10,
                headers={'Content-Type': 'application/soap+xml'}
            )
            assert response.status_code == 200, f"Service at {xaddrs} not accessible"
        except Exception as e:
            pytest.fail(f"Could not access discovered service {xaddrs}: {e}")
