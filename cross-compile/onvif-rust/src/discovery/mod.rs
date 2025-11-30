//! WS-Discovery Protocol Implementation (OASIS WS-Discovery 1.1)
//!
//! This module implements the WS-Discovery 1.1 protocol for ONVIF device discovery
//! as specified in:
//! - OASIS WS-Discovery 1.1 Specification (July 2009)
//! - ONVIF Core Specification Section 7 (Discovery)
//!
//! ## Protocol Overview
//!
//! WS-Discovery defines four message types:
//! - **Hello**: Sent when a device joins the network (multicast announcement)
//! - **Bye**: Sent when a device leaves the network (multicast announcement)
//! - **Probe**: Sent by clients to discover devices (multicast query)
//! - **ProbeMatch**: Response to Probe with matching device information (unicast)
//!
//! ## ONVIF Compliance
//!
//! ONVIF devices MUST:
//! - Send Hello on startup after random delay (0-500ms per APP_MAX_DELAY)
//! - Send Bye on controlled shutdown
//! - Respond to Probe requests matching their Types and Scopes
//! - Use `tdn:NetworkVideoTransmitter` as the device Type
//! - Include XAddrs pointing to the device_service endpoint
//!
//! ## Discovery Modes (ONVIF EC-014)
//!
//! - **Discoverable**: Device responds to Probe and sends Hello/Bye
//! - **NonDiscoverable**: Device silently ignores Probe and does not announce

mod ws_discovery;

pub use ws_discovery::{
    APP_MAX_DELAY_MS, DiscoveryConfig, DiscoveryError, DiscoveryMode, HELLO_INTERVAL_SECONDS,
    WS_DISCOVERY_MULTICAST, WS_DISCOVERY_PORT, WsDiscovery, WsDiscoveryHandle, WsDiscoveryMessage,
};
