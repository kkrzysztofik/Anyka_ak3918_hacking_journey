#!/usr/bin/env python3
"""
WS-Discovery Validation Script for ONVIF Devices

This script sends WS-Discovery Probe messages to discover ONVIF devices on the local
network and validates their responses. It can also passively listen for Hello messages
when devices join the network.

Usage:
    python ws_discovery_validator.py [--timeout SECONDS] [--listen-hello] [--verbose] [--debug] [--interface IP]

Examples:
    # Discover all devices with default 5 second timeout
    python ws_discovery_validator.py

    # Discover with 10 second timeout and verbose output
    python ws_discovery_validator.py --timeout 10 --verbose

    # Discover with full debug output (shows raw XML)
    python ws_discovery_validator.py --debug

    # Also listen for Hello announcements
    python ws_discovery_validator.py --listen-hello

    # Bind to specific network interface
    python ws_discovery_validator.py --interface 192.168.1.100
"""

import argparse
import json
import logging
import socket
import struct
import sys
import time
import uuid
from dataclasses import dataclass, field, asdict
from typing import List, Optional, Dict, Any
from urllib.parse import urlparse

try:
    from lxml import etree  # type: ignore[import-untyped]
except ImportError:
    print("Error: lxml is required. Install with: pip install lxml", file=sys.stderr)
    sys.exit(1)

# Set up module logger
logger = logging.getLogger(__name__)

# WS-Discovery Constants (per OASIS WS-Discovery spec and ONVIF)
WS_DISCOVERY_MULTICAST = "239.255.255.250"
WS_DISCOVERY_PORT = 3702
# Using 2005/04 namespace for broader ONVIF compatibility
WS_DISCOVERY_MULTICAST_TO = "urn:schemas-xmlsoap-org:ws:2005:04:discovery"

# XML Namespaces - Using 2005/04 WS-Discovery for ONVIF compatibility
NAMESPACES = {
    "s": "http://www.w3.org/2003/05/soap-envelope",
    "a": "http://schemas.xmlsoap.org/ws/2004/08/addressing",
    "d": "http://schemas.xmlsoap.org/ws/2005/04/discovery",
    "tdn": "http://www.onvif.org/ver10/network/wsdl",
}

# Also support 2009/01 namespace in responses (some devices use newer spec)
NAMESPACES_2009 = {
    "s": "http://www.w3.org/2003/05/soap-envelope",
    "a": "http://www.w3.org/2005/08/addressing",
    "d": "http://docs.oasis-open.org/ws-dd/ns/discovery/2009/01",
    "tdn": "http://www.onvif.org/ver10/network/wsdl",
}

# WS-Discovery Actions (2005/04 version)
ACTION_PROBE = "http://schemas.xmlsoap.org/ws/2005/04/discovery/Probe"
ACTION_PROBE_MATCHES = "http://schemas.xmlsoap.org/ws/2005/04/discovery/ProbeMatches"
ACTION_HELLO = "http://schemas.xmlsoap.org/ws/2005/04/discovery/Hello"

# ONVIF Device Type
ONVIF_NVT_TYPE = "tdn:NetworkVideoTransmitter"


def setup_logging(verbose: bool = False, debug: bool = False) -> None:
    """Configure logging based on verbosity level."""
    if debug:
        level = logging.DEBUG
    elif verbose:
        level = logging.INFO
    else:
        level = logging.WARNING

    # Configure root logger for this script
    handler = logging.StreamHandler(sys.stderr)
    handler.setFormatter(logging.Formatter(
        '%(asctime)s [%(levelname)8s] %(name)s: %(message)s',
        datefmt='%H:%M:%S'
    ))
    logger.addHandler(handler)
    logger.setLevel(level)


@dataclass
class DiscoveredDevice:
    """Represents a discovered ONVIF device."""
    endpoint_uuid: str
    xaddrs: List[str]
    types: List[str]
    scopes: List[str]
    metadata_version: Optional[int]
    source_ip: str
    source_port: int
    response_time_ms: float
    message_type: str  # "ProbeMatch" or "Hello"
    relates_to: Optional[str] = None


@dataclass
class DiscoveryResult:
    """Result of the WS-Discovery validation."""
    success: bool
    message: str
    probe_message_id: str
    devices: List[Dict[str, Any]] = field(default_factory=list)
    total_devices: int = 0
    elapsed_time_seconds: float = 0.0
    errors: List[str] = field(default_factory=list)


def build_probe_message() -> tuple[str, str]:
    """
    Build a WS-Discovery Probe SOAP message.

    Returns:
        Tuple of (message_id, xml_string)
    """
    message_id = f"uuid:{uuid.uuid4()}"

    logger.debug("Building Probe message: message_id=%s", message_id)
    logger.debug("Using namespace: action=%s, to=%s", ACTION_PROBE, WS_DISCOVERY_MULTICAST_TO)

    # Using 2005/04 format matching working ONVIF clients
    probe_xml = f"""<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope" xmlns:a="http://schemas.xmlsoap.org/ws/2004/08/addressing">
  <s:Header>
    <a:Action s:mustUnderstand="1">{ACTION_PROBE}</a:Action>
    <a:MessageID>{message_id}</a:MessageID>
    <a:ReplyTo>
      <a:Address>http://schemas.xmlsoap.org/ws/2004/08/addressing/role/anonymous</a:Address>
    </a:ReplyTo>
    <a:To s:mustUnderstand="1">{WS_DISCOVERY_MULTICAST_TO}</a:To>
  </s:Header>
  <s:Body>
    <Probe xmlns="http://schemas.xmlsoap.org/ws/2005/04/discovery">
      <d:Types xmlns:d="http://schemas.xmlsoap.org/ws/2005/04/discovery" xmlns:dp0="http://www.onvif.org/ver10/network/wsdl">dp0:NetworkVideoTransmitter</d:Types>
    </Probe>
  </s:Body>
</s:Envelope>"""

    logger.debug("Built Probe message: bytes=%d", len(probe_xml))
    logger.debug("Probe message content:\n%s", probe_xml)

    return message_id, probe_xml


def _find_action_element(root: etree.Element) -> tuple[Optional[etree.Element], Dict[str, str]]:
    """Find Action element in XML root, trying both namespace versions."""
    ns_to_try = [NAMESPACES, NAMESPACES_2009]
    for ns in ns_to_try:
        action_elem = root.find(".//a:Action", namespaces=ns)
        if action_elem is not None:
            return action_elem, ns
    return None, NAMESPACES


def _find_match_element(root: etree.Element, message_type: str, active_ns: Dict[str, str]) -> Optional[etree.Element]:
    """Find ProbeMatch or Hello element in XML root."""
    if message_type == "ProbeMatch":
        match_elem = root.find(".//d:ProbeMatch", namespaces=active_ns)
        if match_elem is None:
            match_elem = root.find(".//{http://schemas.xmlsoap.org/ws/2005/04/discovery}ProbeMatch")
        if match_elem is None:
            match_elem = root.find(".//{http://docs.oasis-open.org/ws-dd/ns/discovery/2009/01}ProbeMatch")
    else:  # Hello
        match_elem = root.find(".//d:Hello", namespaces=active_ns)
        if match_elem is None:
            match_elem = root.find(".//{http://schemas.xmlsoap.org/ws/2005/04/discovery}Hello")
        if match_elem is None:
            match_elem = root.find(".//{http://docs.oasis-open.org/ws-dd/ns/discovery/2009/01}Hello")
    return match_elem


def _extract_element_text(match_elem: etree.Element, elem_name: str, active_ns: Dict[str, str]) -> Optional[etree.Element]:
    """Extract element by name, trying multiple namespace patterns."""
    elem = match_elem.find(f"d:{elem_name}", namespaces=active_ns)
    if elem is None:
        elem = match_elem.find(f"{{http://schemas.xmlsoap.org/ws/2005/04/discovery}}{elem_name}")
    if elem is None:
        elem = match_elem.find(f"{{http://docs.oasis-open.org/ws-dd/ns/discovery/2009/01}}{elem_name}")
    return elem


def _extract_relates_to(root: etree.Element, message_type: str, active_ns: Dict[str, str]) -> Optional[str]:
    """Extract RelatesTo for ProbeMatch validation."""
    if message_type != "ProbeMatch":
        return None
    relates_to_elem = root.find(".//a:RelatesTo", namespaces=active_ns)
    if relates_to_elem is not None and relates_to_elem.text:
        return relates_to_elem.text.strip()
    return None


def _extract_endpoint_uuid(match_elem: etree.Element, active_ns: Dict[str, str]) -> str:
    """Extract EndpointReference/Address (UUID)."""
    endpoint_elem = match_elem.find(".//a:EndpointReference/a:Address", namespaces=active_ns)
    if endpoint_elem is None:
        endpoint_elem = match_elem.find(".//{http://schemas.xmlsoap.org/ws/2004/08/addressing}Address")
    if endpoint_elem is None:
        endpoint_elem = match_elem.find(".//{http://www.w3.org/2005/08/addressing}Address")
    if endpoint_elem is not None and endpoint_elem.text:
        return endpoint_elem.text.strip()
    return ""


def _extract_text_list(match_elem: etree.Element, elem_name: str, active_ns: Dict[str, str]) -> List[str]:
    """Extract a text-based list element (XAddrs, Types, Scopes)."""
    elem = _extract_element_text(match_elem, elem_name, active_ns)
    if elem is not None and elem.text:
        return elem.text.strip().split()
    return []


def _extract_metadata_version(match_elem: etree.Element, active_ns: Dict[str, str]) -> Optional[int]:
    """Extract MetadataVersion as integer."""
    metadata_elem = _extract_element_text(match_elem, "MetadataVersion", active_ns)
    if metadata_elem is not None and metadata_elem.text:
        try:
            return int(metadata_elem.text.strip())
        except ValueError:
            pass
    return None


def _extract_device_data(match_elem: etree.Element, root: etree.Element, message_type: str,
                        active_ns: Dict[str, str]) -> Dict[str, Any]:
    """Extract all device data from match element."""
    relates_to = _extract_relates_to(root, message_type, active_ns)
    endpoint_uuid = _extract_endpoint_uuid(match_elem, active_ns)
    xaddrs = _extract_text_list(match_elem, "XAddrs", active_ns)
    types = _extract_text_list(match_elem, "Types", active_ns)
    scopes = _extract_text_list(match_elem, "Scopes", active_ns)
    metadata_version = _extract_metadata_version(match_elem, active_ns)

    return {
        "endpoint_uuid": endpoint_uuid,
        "xaddrs": xaddrs,
        "types": types,
        "scopes": scopes,
        "metadata_version": metadata_version,
        "relates_to": relates_to,
    }


def parse_response(data: bytes, source_addr: tuple, start_time: float,
                   verbose: bool = False) -> Optional[DiscoveredDevice]:
    """
    Parse a WS-Discovery response (ProbeMatch or Hello).

    Args:
        data: Raw XML response bytes
        source_addr: (ip, port) tuple of the sender
        start_time: Time when probe was sent (for response time calculation)
        verbose: Enable verbose output

    Returns:
        DiscoveredDevice if successfully parsed, None otherwise
    """
    response_time_ms = (time.time() - start_time) * 1000
    source_ip, source_port = source_addr

    logger.debug("Parsing response from %s:%d, bytes=%d, response_time_ms=%.2f",
                 source_ip, source_port, len(data), response_time_ms)
    logger.debug("Raw response data:\n%s", data.decode('utf-8', errors='replace'))

    try:
        root = etree.fromstring(data)
        logger.debug("XML parsed successfully")

        action_elem, active_ns = _find_action_element(root)
        if action_elem is None:
            logger.debug("No Action header found in any namespace")
            if verbose:
                print(f"  [WARN] No Action header in response from {source_ip}", file=sys.stderr)
            return None

        action = action_elem.text.strip() if action_elem.text else ""
        logger.debug("Action header value: %s", action)

        # Determine message type
        if "ProbeMatches" in action:
            message_type = "ProbeMatch"
        elif "Hello" in action:
            message_type = "Hello"
        else:
            logger.debug("Unknown action type: %s", action)
            if verbose:
                print(f"  [WARN] Unknown action: {action} from {source_ip}", file=sys.stderr)
            return None

        match_elem = _find_match_element(root, message_type, active_ns)
        if match_elem is None:
            if verbose:
                print(f"  [WARN] No {message_type} element in response from {source_ip}", file=sys.stderr)
            return None

        device_data = _extract_device_data(match_elem, root, message_type, active_ns)

        return DiscoveredDevice(
            endpoint_uuid=device_data["endpoint_uuid"],
            xaddrs=device_data["xaddrs"],
            types=device_data["types"],
            scopes=device_data["scopes"],
            metadata_version=device_data["metadata_version"],
            source_ip=source_ip,
            source_port=source_port,
            response_time_ms=round(response_time_ms, 2),
            message_type=message_type,
            relates_to=device_data["relates_to"],
        )

    except etree.XMLSyntaxError as e:
        if verbose:
            print(f"  [WARN] XML parse error from {source_ip}: {e}", file=sys.stderr)
        return None


def create_multicast_socket(interface: Optional[str] = None) -> socket.socket:
    """
    Create and configure a UDP socket for WS-Discovery multicast.

    Args:
        interface: Optional IP address of the interface to bind to

    Returns:
        Configured socket
    """
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM, socket.IPPROTO_UDP)

    # Allow multiple sockets to use the same port
    sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    logger.debug("Set SO_REUSEADDR on socket")

    # Set TTL for multicast (1 = local network only)
    sock.setsockopt(socket.IPPROTO_IP, socket.IP_MULTICAST_TTL, 1)
    logger.debug("Set IP_MULTICAST_TTL=1")

    # Bind to receive responses
    bind_addr = interface if interface else ""
    sock.bind((bind_addr, 0))  # Bind to any available port
    local_port = sock.getsockname()[1]
    logger.debug("Bound socket: bind_addr=%s, local_port=%d", bind_addr or "127.0.0.1", local_port)

    # Join multicast group to receive Hello messages
    logger.debug("Joining multicast group: %s", WS_DISCOVERY_MULTICAST)
    mreq = struct.pack("4s4s",
                       socket.inet_aton(WS_DISCOVERY_MULTICAST),
                       socket.inet_aton(interface if interface else "127.0.0.1"))
    sock.setsockopt(socket.IPPROTO_IP, socket.IP_ADD_MEMBERSHIP, mreq)
    logger.debug("Successfully joined multicast group: %s", WS_DISCOVERY_MULTICAST)

    return sock


def _send_probe(sock: socket.socket, interface: Optional[str], verbose: bool) -> tuple[str, str]:
    """Build and send Probe message, returning message_id and probe_xml."""
    message_id, probe_xml = build_probe_message()

    if verbose:
        print("[INFO] Sending WS-Discovery Probe", file=sys.stderr)
        print(f"  MessageID: {message_id}", file=sys.stderr)
        print(f"  Multicast: {WS_DISCOVERY_MULTICAST}:{WS_DISCOVERY_PORT}", file=sys.stderr)
        if interface:
            print(f"  Interface: {interface}", file=sys.stderr)

    logger.debug("Sending Probe to multicast address: dest=%s:%d, bytes=%d",
                 WS_DISCOVERY_MULTICAST, WS_DISCOVERY_PORT, len(probe_xml))
    sock.sendto(probe_xml.encode("utf-8"), (WS_DISCOVERY_MULTICAST, WS_DISCOVERY_PORT))
    logger.debug("Probe sent successfully")

    return message_id, probe_xml


def _should_accept_device(device: DiscoveredDevice, message_id: str, listen_hello: bool, verbose: bool) -> bool:
    """Check if device should be accepted based on message type and filters."""
    if device.message_type == "ProbeMatch":
        if device.relates_to != message_id:
            logger.debug("ProbeMatch RelatesTo mismatch: expected=%s, got=%s",
                         message_id, device.relates_to)
            if verbose:
                print(f"  [SKIP] ProbeMatch RelatesTo mismatch: expected {message_id}, got {device.relates_to}", file=sys.stderr)
            return False
        logger.debug("ProbeMatch RelatesTo validated successfully")
        return True
    elif device.message_type == "Hello" and not listen_hello:
        logger.debug("Ignoring Hello message (--listen-hello not enabled)")
        if verbose:
            print("  [SKIP] Hello message (--listen-hello not enabled)", file=sys.stderr)
        return False
    return True


def _process_received_device(device: DiscoveredDevice, devices: Dict[str, DiscoveredDevice], verbose: bool) -> None:
    """Process a received device, adding it to the devices dict if new."""
    key = device.endpoint_uuid or f"{device.source_ip}:{device.source_port}"
    if key not in devices:
        devices[key] = device
        if verbose:
            print(f"  [FOUND] {device.message_type}: {device.endpoint_uuid}", file=sys.stderr)
            for xaddr in device.xaddrs:
                print(f"          XAddr: {xaddr}", file=sys.stderr)
    elif verbose:
        print(f"  [DUP] Already discovered: {device.endpoint_uuid}", file=sys.stderr)


def _process_packet(data: bytes, addr: tuple, start_time: float, message_id: str,
                    listen_hello: bool, verbose: bool, devices: Dict[str, DiscoveredDevice],
                    recv_count: int) -> None:
    """Process a single received packet."""
    logger.debug("Received packet #%d from %s:%d, bytes=%d",
                 recv_count, addr[0], addr[1], len(data))

    if verbose:
        print(f"  [RECV] Response from {addr[0]}:{addr[1]} ({len(data)} bytes)", file=sys.stderr)

    device = parse_response(data, addr, start_time, verbose)

    if device:
        logger.debug("Parsed device response: type=%s, endpoint=%s",
                     device.message_type,
                     device.endpoint_uuid[:40] if device.endpoint_uuid else "none")

        if _should_accept_device(device, message_id, listen_hello, verbose):
            _process_received_device(device, devices, verbose)


def _receive_responses(sock: socket.socket, start_time: float, timeout: float, message_id: str,
                       listen_hello: bool, verbose: bool, result: DiscoveryResult) -> Dict[str, DiscoveredDevice]:
    """Receive and process responses until timeout."""
    devices: Dict[str, DiscoveredDevice] = {}
    recv_count = 0

    if verbose:
        print(f"[INFO] Listening for responses (timeout: {timeout}s)...", file=sys.stderr)
        if listen_hello:
            print("[INFO] Also listening for Hello announcements", file=sys.stderr)

    logger.debug("Starting response receive loop, timeout=%.1f", timeout)

    while (time.time() - start_time) < timeout:
        try:
            data, addr = sock.recvfrom(65535)
            recv_count += 1
            _process_packet(data, addr, start_time, message_id, listen_hello, verbose, devices, recv_count)

        except socket.timeout:
            continue
        except Exception as e:
            if verbose:
                print(f"  [ERROR] Receive error: {e}", file=sys.stderr)
            result.errors.append(str(e))

    return devices


def discover_devices(timeout: float = 5.0, listen_hello: bool = False,
                     interface: Optional[str] = None, verbose: bool = False) -> DiscoveryResult:
    """
    Perform WS-Discovery to find ONVIF devices.

    Args:
        timeout: How long to wait for responses (seconds)
        listen_hello: Also listen for Hello announcements
        interface: Optional interface IP to bind to
        verbose: Enable verbose output

    Returns:
        DiscoveryResult with discovered devices
    """
    logger.debug("Starting WS-Discovery: timeout=%.1f, listen_hello=%s, interface=%s",
                 timeout, listen_hello, interface or "all")

    result = DiscoveryResult(
        success=False,
        message="",
        probe_message_id="",
        devices=[],
        total_devices=0,
        elapsed_time_seconds=0.0,
        errors=[],
    )

    start_time = time.time()

    try:
        logger.debug("Creating multicast socket")
        sock = create_multicast_socket(interface)
        sock.settimeout(0.5)  # Short timeout for non-blocking receive loop
        logger.debug("Socket created and configured, recv_timeout=0.5")

        message_id, _ = _send_probe(sock, interface, verbose)
        result.probe_message_id = message_id

        devices = _receive_responses(sock, start_time, timeout, message_id, listen_hello, verbose, result)

        sock.close()

        # Build result
        result.elapsed_time_seconds = round(time.time() - start_time, 3)
        result.devices = [asdict(d) for d in devices.values()]
        result.total_devices = len(devices)

        if devices:
            result.success = True
            result.message = f"Discovered {len(devices)} ONVIF device(s)"
        else:
            result.success = False
            result.message = "No ONVIF devices discovered"

    except Exception as e:
        result.success = False
        result.message = f"Discovery failed: {e}"
        result.errors.append(str(e))
        result.elapsed_time_seconds = round(time.time() - start_time, 3)

    return result


def main():
    """Main entry point."""
    parser = argparse.ArgumentParser(
        description="WS-Discovery validation script for ONVIF devices",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  %(prog)s                          # Discover with 5s timeout
  %(prog)s --timeout 10             # Discover with 10s timeout
  %(prog)s --listen-hello           # Also listen for Hello announcements
  %(prog)s --verbose                # Show detailed progress
  %(prog)s --debug                  # Show full debug output (raw XML)
  %(prog)s --interface 192.168.1.10 # Bind to specific interface
        """,
    )

    parser.add_argument(
        "--timeout", "-t",
        type=float,
        default=5.0,
        help="Discovery timeout in seconds (default: 5.0)",
    )

    parser.add_argument(
        "--listen-hello",
        action="store_true",
        help="Also listen for Hello announcements (passive discovery)",
    )

    parser.add_argument(
        "--interface", "-i",
        type=str,
        default=None,
        help="IP address of network interface to use",
    )

    parser.add_argument(
        "--verbose", "-v",
        action="store_true",
        help="Enable verbose output to stderr (INFO level)",
    )

    parser.add_argument(
        "--debug", "-d",
        action="store_true",
        help="Enable debug output with raw XML (DEBUG level, implies --verbose)",
    )

    args = parser.parse_args()

    # Setup logging based on verbosity
    setup_logging(verbose=args.verbose, debug=args.debug)

    # Debug implies verbose
    if args.debug:
        args.verbose = True

    if args.verbose:
        logger.info("=" * 60)
        logger.info("WS-Discovery Validator for ONVIF Devices")
        logger.info("=" * 60)
        if args.debug:
            logger.info("DEBUG MODE ENABLED - Full XML output")
            logger.info("Using namespace: %s", NAMESPACES['d'])
            logger.info("Multicast target: %s:%d", WS_DISCOVERY_MULTICAST, WS_DISCOVERY_PORT)
            logger.info("=" * 60)

    # Run discovery
    result = discover_devices(
        timeout=args.timeout,
        listen_hello=args.listen_hello,
        interface=args.interface,
        verbose=args.verbose,
    )

    # Output JSON result to stdout
    print(json.dumps(asdict(result), indent=2))

    # Exit code: 0 if devices found, 1 if none
    sys.exit(0 if result.success else 1)


if __name__ == "__main__":
    main()
