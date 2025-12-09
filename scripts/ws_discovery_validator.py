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
    from lxml import etree
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


def parse_response(data: bytes, source_addr: tuple, start_time: float,
                   probe_message_id: str, verbose: bool = False) -> Optional[DiscoveredDevice]:
    """
    Parse a WS-Discovery response (ProbeMatch or Hello).

    Args:
        data: Raw XML response bytes
        source_addr: (ip, port) tuple of the sender
        start_time: Time when probe was sent (for response time calculation)
        probe_message_id: The MessageID of our Probe (for validation)
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
        # Parse XML
        logger.debug("Parsing XML response")
        root = etree.fromstring(data)
        logger.debug("XML parsed successfully")

        # Try both 2005/04 and 2009/01 namespaces
        ns_to_try = [NAMESPACES, NAMESPACES_2009]
        action_elem = None
        active_ns = NAMESPACES

        logger.debug("Searching for Action element in both namespace versions")
        for ns in ns_to_try:
            action_elem = root.find(".//a:Action", namespaces=ns)
            if action_elem is not None:
                active_ns = ns
                ns_name = "2005/04" if ns == NAMESPACES else "2009/01"
                logger.debug("Found Action element using %s namespace", ns_name)
                break

        if action_elem is None:
            logger.debug("No Action header found in any namespace")
            if verbose:
                print(f"  [WARN] No Action header in response from {source_ip}", file=sys.stderr)
            return None

        action = action_elem.text.strip() if action_elem.text else ""
        logger.debug("Action header value: %s", action)

        # Determine message type - check both namespace versions
        if "ProbeMatches" in action:
            message_type = "ProbeMatch"
            logger.debug("Message type: ProbeMatch, searching for ProbeMatch element")
            match_elem = root.find(".//d:ProbeMatch", namespaces=active_ns)
            # Also try without namespace prefix
            if match_elem is None:
                logger.debug("Trying 2005/04 namespace directly")
                match_elem = root.find(".//{http://schemas.xmlsoap.org/ws/2005/04/discovery}ProbeMatch")
            if match_elem is None:
                logger.debug("Trying 2009/01 namespace directly")
                match_elem = root.find(".//{http://docs.oasis-open.org/ws-dd/ns/discovery/2009/01}ProbeMatch")
        elif "Hello" in action:
            message_type = "Hello"
            logger.debug("Message type: Hello, searching for Hello element")
            match_elem = root.find(".//d:Hello", namespaces=active_ns)
            if match_elem is None:
                match_elem = root.find(".//{http://schemas.xmlsoap.org/ws/2005/04/discovery}Hello")
            if match_elem is None:
                match_elem = root.find(".//{http://docs.oasis-open.org/ws-dd/ns/discovery/2009/01}Hello")
        else:
            logger.debug("Unknown action type: %s", action)
            if verbose:
                print(f"  [WARN] Unknown action: {action} from {source_ip}", file=sys.stderr)
            return None

        if match_elem is None:
            if verbose:
                print(f"  [WARN] No {message_type} element in response from {source_ip}", file=sys.stderr)
            return None

        # Extract RelatesTo for ProbeMatch validation
        relates_to = None
        if message_type == "ProbeMatch":
            relates_to_elem = root.find(".//a:RelatesTo", namespaces=active_ns)
            if relates_to_elem is not None and relates_to_elem.text:
                relates_to = relates_to_elem.text.strip()

        # Extract EndpointReference/Address (UUID) - try multiple patterns
        endpoint_elem = match_elem.find(".//a:EndpointReference/a:Address", namespaces=active_ns)
        if endpoint_elem is None:
            endpoint_elem = match_elem.find(".//{http://schemas.xmlsoap.org/ws/2004/08/addressing}Address")
        if endpoint_elem is None:
            endpoint_elem = match_elem.find(".//{http://www.w3.org/2005/08/addressing}Address")
        endpoint_uuid = endpoint_elem.text.strip() if endpoint_elem is not None and endpoint_elem.text else ""

        # Extract XAddrs (service URLs) - try multiple patterns
        xaddrs_elem = match_elem.find("d:XAddrs", namespaces=active_ns)
        if xaddrs_elem is None:
            xaddrs_elem = match_elem.find("{http://schemas.xmlsoap.org/ws/2005/04/discovery}XAddrs")
        if xaddrs_elem is None:
            xaddrs_elem = match_elem.find("{http://docs.oasis-open.org/ws-dd/ns/discovery/2009/01}XAddrs")
        xaddrs = []
        if xaddrs_elem is not None and xaddrs_elem.text:
            xaddrs = xaddrs_elem.text.strip().split()

        # Extract Types - try multiple patterns
        types_elem = match_elem.find("d:Types", namespaces=active_ns)
        if types_elem is None:
            types_elem = match_elem.find("{http://schemas.xmlsoap.org/ws/2005/04/discovery}Types")
        if types_elem is None:
            types_elem = match_elem.find("{http://docs.oasis-open.org/ws-dd/ns/discovery/2009/01}Types")
        types = []
        if types_elem is not None and types_elem.text:
            types = types_elem.text.strip().split()

        # Extract Scopes - try multiple patterns
        scopes_elem = match_elem.find("d:Scopes", namespaces=active_ns)
        if scopes_elem is None:
            scopes_elem = match_elem.find("{http://schemas.xmlsoap.org/ws/2005/04/discovery}Scopes")
        if scopes_elem is None:
            scopes_elem = match_elem.find("{http://docs.oasis-open.org/ws-dd/ns/discovery/2009/01}Scopes")
        scopes = []
        if scopes_elem is not None and scopes_elem.text:
            scopes = scopes_elem.text.strip().split()

        # Extract MetadataVersion - try multiple patterns
        metadata_elem = match_elem.find("d:MetadataVersion", namespaces=active_ns)
        if metadata_elem is None:
            metadata_elem = match_elem.find("{http://schemas.xmlsoap.org/ws/2005/04/discovery}MetadataVersion")
        if metadata_elem is None:
            metadata_elem = match_elem.find("{http://docs.oasis-open.org/ws-dd/ns/discovery/2009/01}MetadataVersion")
        metadata_version = None
        if metadata_elem is not None and metadata_elem.text:
            try:
                metadata_version = int(metadata_elem.text.strip())
            except ValueError:
                pass

        return DiscoveredDevice(
            endpoint_uuid=endpoint_uuid,
            xaddrs=xaddrs,
            types=types,
            scopes=scopes,
            metadata_version=metadata_version,
            source_ip=source_ip,
            source_port=source_port,
            response_time_ms=round(response_time_ms, 2),
            message_type=message_type,
            relates_to=relates_to,
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
        # Create socket
        logger.debug("Creating multicast socket")
        sock = create_multicast_socket(interface)
        sock.settimeout(0.5)  # Short timeout for non-blocking receive loop
        logger.debug("Socket created and configured, recv_timeout=0.5")

        # Build and send Probe
        message_id, probe_xml = build_probe_message()
        result.probe_message_id = message_id

        if verbose:
            print(f"[INFO] Sending WS-Discovery Probe", file=sys.stderr)
            print(f"  MessageID: {message_id}", file=sys.stderr)
            print(f"  Multicast: {WS_DISCOVERY_MULTICAST}:{WS_DISCOVERY_PORT}", file=sys.stderr)
            if interface:
                print(f"  Interface: {interface}", file=sys.stderr)

        # Send Probe to multicast address
        logger.debug("Sending Probe to multicast address: dest=%s:%d, bytes=%d",
                     WS_DISCOVERY_MULTICAST, WS_DISCOVERY_PORT, len(probe_xml))
        sock.sendto(probe_xml.encode("utf-8"), (WS_DISCOVERY_MULTICAST, WS_DISCOVERY_PORT))
        logger.debug("Probe sent successfully")

        if verbose:
            print(f"[INFO] Listening for responses (timeout: {timeout}s)...", file=sys.stderr)
            if listen_hello:
                print(f"[INFO] Also listening for Hello announcements", file=sys.stderr)

        logger.debug("Starting response receive loop, timeout=%.1f", timeout)

        # Collect responses
        devices: Dict[str, DiscoveredDevice] = {}  # Deduplicate by endpoint UUID
        recv_count = 0

        while (time.time() - start_time) < timeout:
            try:
                data, addr = sock.recvfrom(65535)
                recv_count += 1

                logger.debug("Received packet #%d from %s:%d, bytes=%d",
                             recv_count, addr[0], addr[1], len(data))

                if verbose:
                    print(f"  [RECV] Response from {addr[0]}:{addr[1]} ({len(data)} bytes)", file=sys.stderr)

                device = parse_response(data, addr, start_time, message_id, verbose)

                if device:
                    logger.debug("Parsed device response: type=%s, endpoint=%s",
                                 device.message_type,
                                 device.endpoint_uuid[:40] if device.endpoint_uuid else "none")

                    # Filter: for ProbeMatch, only accept responses to our Probe
                    if device.message_type == "ProbeMatch":
                        if device.relates_to != message_id:
                            logger.debug("ProbeMatch RelatesTo mismatch: expected=%s, got=%s",
                                         message_id, device.relates_to)
                            if verbose:
                                print(f"  [SKIP] ProbeMatch RelatesTo mismatch: expected {message_id}, got {device.relates_to}", file=sys.stderr)
                            continue
                        logger.debug("ProbeMatch RelatesTo validated successfully")
                    elif device.message_type == "Hello" and not listen_hello:
                        logger.debug("Ignoring Hello message (--listen-hello not enabled)")
                        if verbose:
                            print(f"  [SKIP] Hello message (--listen-hello not enabled)", file=sys.stderr)
                        continue

                    # Deduplicate by endpoint UUID
                    key = device.endpoint_uuid or f"{device.source_ip}:{device.source_port}"
                    if key not in devices:
                        devices[key] = device
                        if verbose:
                            print(f"  [FOUND] {device.message_type}: {device.endpoint_uuid}", file=sys.stderr)
                            for xaddr in device.xaddrs:
                                print(f"          XAddr: {xaddr}", file=sys.stderr)
                    elif verbose:
                        print(f"  [DUP] Already discovered: {device.endpoint_uuid}", file=sys.stderr)

            except socket.timeout:
                continue
            except Exception as e:
                if verbose:
                    print(f"  [ERROR] Receive error: {e}", file=sys.stderr)
                result.errors.append(str(e))

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
