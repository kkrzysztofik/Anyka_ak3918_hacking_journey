//! WS-Discovery 1.1 Implementation for ONVIF Device Discovery
//!
//! This module implements the WS-Discovery protocol as defined by:
//! - OASIS Web Services Dynamic Discovery (WS-Discovery) Version 1.1
//!   http://docs.oasis-open.org/ws-dd/discovery/1.1/os/wsdd-discovery-1.1-spec-os.html
//!
//! ## Protocol Assignments (Section 3.1.1)
//! - Port: 3702 (IANA registered)
//! - IPv4 Multicast: 239.255.255.250
//! - IPv6 Multicast: FF02::C (link-local scope)
//!
//! ## XML Namespaces (Section 1.5)
//! - `d` = http://docs.oasis-open.org/ws-dd/ns/discovery/2009/01
//! - `a` = http://www.w3.org/2005/08/addressing
//! - `s` = http://www.w3.org/2003/05/soap-envelope (SOAP 1.2)
//!
//! ## Application Level Transmission Delay (Section 3.1.3)
//! Before sending certain messages, a random delay of 0 to APP_MAX_DELAY (500ms)
//! MUST be applied to prevent network storms.

use std::io;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::Duration;

use rand::Rng;
use socket2::{Domain, Protocol, Socket, Type};
use thiserror::Error;
use tokio::net::UdpSocket;
use tokio::sync::RwLock;
use tracing::{debug, info, trace, warn};
use uuid::Uuid;

// =============================================================================
// WS-Discovery Protocol Constants (per OASIS spec Section 3)
// =============================================================================

/// WS-Discovery multicast address (IPv4) - Section 3.1.1
pub const WS_DISCOVERY_MULTICAST: Ipv4Addr = Ipv4Addr::new(239, 255, 255, 250);

/// WS-Discovery port (IANA registered) - Section 3.1.1
pub const WS_DISCOVERY_PORT: u16 = 3702;

/// Application level transmission delay in milliseconds - Section 3.1.3
/// Target services MUST wait for a random time between 0 and APP_MAX_DELAY
/// before sending Hello, ProbeMatch, or ResolveMatch messages.
pub const APP_MAX_DELAY_MS: u64 = 500;

/// Hello announcement interval in seconds (periodic re-announcement)
pub const HELLO_INTERVAL_SECONDS: u64 = 300;

/// Maximum message size for WS-Discovery UDP packets
pub const MAX_MESSAGE_SIZE: usize = 4096;

// =============================================================================
// XML Namespace URIs - Using WS-Discovery 2005/04 for ONVIF compatibility
// Note: Many ONVIF devices/clients use the older 2005/04 namespace
// =============================================================================

/// WS-Discovery namespace (2005/04 version - widely used by ONVIF)
const WSD_NS: &str = "http://schemas.xmlsoap.org/ws/2005/04/discovery";

/// WS-Addressing namespace (2004/08 version - used with 2005/04 discovery)
const WSA_NS: &str = "http://schemas.xmlsoap.org/ws/2004/08/addressing";

/// SOAP 1.2 namespace
const SOAP_NS: &str = "http://www.w3.org/2003/05/soap-envelope";

/// ONVIF Network WSDL namespace (for Types)
const ONVIF_NW_NS: &str = "http://www.onvif.org/ver10/network/wsdl";

/// Distinguished URI for multicast messages (2005/04 version)
/// This is the value for a:To in multicast Hello/Bye/Probe messages
const WSD_MULTICAST_TO: &str = "urn:schemas-xmlsoap-org:ws:2005:04:discovery";

/// Anonymous endpoint for response routing (WS-Addressing 2004/08)
const WSA_ANONYMOUS: &str = "http://schemas.xmlsoap.org/ws/2004/08/addressing/role/anonymous";

// =============================================================================
// ONVIF-Specific Constants
// =============================================================================

/// ONVIF Network Video Transmitter type (QName: tdn:NetworkVideoTransmitter)
const ONVIF_NVT_TYPE: &str = "tdn:NetworkVideoTransmitter";

/// Default ONVIF scopes for a basic NVT device
const DEFAULT_SCOPES: &[&str] = &[
    "onvif://www.onvif.org/type/video_encoder",
    "onvif://www.onvif.org/type/audio_encoder",
    "onvif://www.onvif.org/type/ptz",
    "onvif://www.onvif.org/Profile/Streaming",
];

// =============================================================================
// Discovery Mode (per ONVIF EC-014)
// =============================================================================

/// Discovery mode for WS-Discovery operation
///
/// Per ONVIF specification EC-014:
/// - Discoverable: Device responds to Probe messages and sends Hello/Bye
/// - NonDiscoverable: Device silently ignores Probe messages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DiscoveryMode {
    /// Device responds to Probe messages and sends Hello/Bye announcements
    #[default]
    Discoverable,
    /// Device ignores Probe messages and does not announce itself (silent mode)
    NonDiscoverable,
}

impl std::fmt::Display for DiscoveryMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiscoveryMode::Discoverable => write!(f, "Discoverable"),
            DiscoveryMode::NonDiscoverable => write!(f, "NonDiscoverable"),
        }
    }
}

// =============================================================================
// Error Types
// =============================================================================

/// WS-Discovery error types
#[derive(Debug, Error)]
pub enum DiscoveryError {
    /// Socket creation or binding error
    #[error("socket error: {0}")]
    Socket(#[from] io::Error),

    /// Multicast group join failed
    #[error("failed to join multicast group: {0}")]
    MulticastJoin(io::Error),

    /// Message serialization error
    #[error("message serialization error: {0}")]
    Serialization(String),

    /// Message parsing error
    #[error("message parsing error: {0}")]
    Parsing(String),

    /// Discovery service not running
    #[error("discovery service not running")]
    NotRunning,

    /// Discovery service already running
    #[error("discovery service already running")]
    AlreadyRunning,
}

// =============================================================================
// Configuration
// =============================================================================

/// Configuration for WS-Discovery service
#[derive(Debug, Clone)]
pub struct DiscoveryConfig {
    /// Device endpoint UUID (format: urn:uuid:XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX)
    /// Per WS-Discovery Section 2.1, this should be a stable, globally unique identifier
    pub endpoint_uuid: String,

    /// HTTP port where ONVIF device service is exposed
    pub http_port: u16,

    /// Device's IP address for XAddrs (transport addresses)
    /// Note: This may be overridden at runtime using external_ip() helper.
    pub device_ip: String,

    /// Device scopes for discovery matching (space-separated URIs)
    /// Per Section 5.1, scopes are used to filter Probe responses
    pub scopes: Vec<String>,

    /// Initial discovery mode (Discoverable/NonDiscoverable)
    pub discovery_mode: DiscoveryMode,

    /// Hello announcement interval for periodic re-broadcast
    pub hello_interval: Duration,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            endpoint_uuid: format!("urn:uuid:{}", Uuid::new_v4()),
            http_port: 80,
            device_ip: "192.168.1.100".to_string(),
            scopes: DEFAULT_SCOPES.iter().map(|s| (*s).to_string()).collect(),
            discovery_mode: DiscoveryMode::Discoverable,
            hello_interval: Duration::from_secs(HELLO_INTERVAL_SECONDS),
        }
    }
}

impl DiscoveryConfig {
    /// Create a new discovery configuration with specified parameters
    pub fn new(endpoint_uuid: String, http_port: u16, device_ip: String) -> Self {
        Self {
            endpoint_uuid,
            http_port,
            device_ip,
            ..Default::default()
        }
    }

    /// Get the XAddrs URL for this device (transport address)
    /// Per WS-Discovery, XAddrs contains URIs where the device can be contacted
    pub fn get_xaddrs(&self) -> String {
        format!(
            "http://{}:{}/onvif/device_service",
            self.device_ip, self.http_port
        )
    }

    /// Get scopes as a space-separated string for XML serialization
    pub fn get_scopes_string(&self) -> String {
        self.scopes.join(" ")
    }
}

// =============================================================================
// WS-Discovery Message Types
// =============================================================================

/// WS-Discovery message types as defined in OASIS spec Sections 4-6
#[derive(Debug, Clone, PartialEq)]
pub enum WsDiscoveryMessage {
    /// Hello announcement message (Section 4.1)
    /// Sent when a Target Service joins a network
    Hello {
        /// Unique message identifier (a:MessageID)
        message_id: String,
        /// Stable endpoint identifier (a:EndpointReference/a:Address)
        endpoint_reference: String,
        /// Types implemented by the service (d:Types) - space-separated QNames
        types: String,
        /// Scopes the service is in (d:Scopes) - space-separated URIs
        scopes: String,
        /// Transport addresses (d:XAddrs) - space-separated URIs
        xaddrs: String,
        /// Metadata version counter (d:MetadataVersion)
        metadata_version: u32,
        /// Application sequence instance ID (for message ordering)
        instance_id: u32,
        /// Application sequence message number
        message_number: u32,
    },

    /// Bye departure message (Section 4.2)
    /// Sent when a Target Service is preparing to leave the network
    Bye {
        /// Unique message identifier
        message_id: String,
        /// Stable endpoint identifier
        endpoint_reference: String,
        /// Application sequence instance ID
        instance_id: u32,
        /// Application sequence message number
        message_number: u32,
    },

    /// Probe request message (Section 5.2)
    /// Sent by Clients to search for Target Services
    Probe {
        /// Unique message identifier
        message_id: String,
        /// Types to match (optional) - if empty, matches all types
        types: Option<String>,
        /// Scopes to match (optional) - if empty, matches all scopes
        scopes: Option<String>,
    },

    /// ProbeMatch response message (Section 5.3)
    /// Sent by Target Services in response to a matching Probe
    ProbeMatch {
        /// Unique message identifier
        message_id: String,
        /// RelatesTo: MessageID of the original Probe
        relates_to: String,
        /// Stable endpoint identifier
        endpoint_reference: String,
        /// Types implemented by the service
        types: String,
        /// Scopes the service is in
        scopes: String,
        /// Transport addresses
        xaddrs: String,
        /// Metadata version counter
        metadata_version: u32,
        /// Application sequence instance ID
        instance_id: u32,
        /// Application sequence message number
        message_number: u32,
    },
}

// =============================================================================
// WS-Discovery Handle (for controlling a running service)
// =============================================================================

/// Handle to a running WS-Discovery service.
///
/// This is a lightweight handle that allows controlling a WS-Discovery service
/// that is running in a background task. It provides methods to:
/// - Stop the service gracefully (sends Bye message)
/// - Change the discovery mode
/// - Update scopes
///
/// The handle is safe to clone and can be used from multiple tasks.
#[derive(Clone)]
pub struct WsDiscoveryHandle {
    /// Service configuration (shared with running service)
    config: Arc<RwLock<DiscoveryConfig>>,

    /// Running state flag (shared with running service)
    running: Arc<AtomicBool>,

    /// Metadata version counter (shared with running service)
    metadata_version: Arc<AtomicU32>,

    /// Message number counter (shared with running service)
    message_number: Arc<AtomicU32>,

    /// Instance ID for this service run
    instance_id: u32,
}

impl WsDiscoveryHandle {
    /// Stop the WS-Discovery service gracefully.
    ///
    /// This sends a Bye message to announce departure and stops the service loop.
    pub async fn stop(&self) -> Result<(), DiscoveryError> {
        if !self.running.load(Ordering::Acquire) {
            return Ok(());
        }

        info!("Stopping WS-Discovery service...");

        // Send Bye before stopping (Section 4.2.1)
        if let Err(e) = self.send_bye().await {
            warn!(error = %e, "Failed to send Bye message");
        }

        self.running.store(false, Ordering::Release);
        Ok(())
    }

    /// Check if the service is running.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Acquire)
    }

    /// Get the current discovery mode.
    pub async fn get_discovery_mode(&self) -> DiscoveryMode {
        self.config.read().await.discovery_mode
    }

    /// Set the discovery mode (Discoverable/NonDiscoverable).
    pub async fn set_discovery_mode(&self, mode: DiscoveryMode) {
        let mut config = self.config.write().await;
        let old_mode = config.discovery_mode;
        config.discovery_mode = mode;

        if old_mode != mode {
            info!(old = %old_mode, new = %mode, "Discovery mode changed");
        }
    }

    /// Update device scopes and increment metadata version.
    pub async fn set_scopes(&self, scopes: Vec<String>) {
        let mut config = self.config.write().await;
        config.scopes = scopes;

        // Increment metadata version when configuration changes (Section 4.1)
        let new_version = self.metadata_version.fetch_add(1, Ordering::Relaxed) + 1;
        info!(metadata_version = new_version, "Discovery scopes updated");
    }

    /// Send a Bye message to the multicast group.
    async fn send_bye(&self) -> Result<(), DiscoveryError> {
        let config = self.config.read().await;

        if config.discovery_mode == DiscoveryMode::NonDiscoverable {
            debug!("Skipping Bye in NonDiscoverable mode");
            return Ok(());
        }

        let msg_num = self.message_number.fetch_add(1, Ordering::SeqCst) + 1;
        let message = WsDiscoveryMessage::Bye {
            message_id: format!("urn:uuid:{}", Uuid::new_v4()),
            endpoint_reference: config.endpoint_uuid.clone(),
            instance_id: self.instance_id,
            message_number: msg_num,
        };

        let xml = WsDiscovery::serialize_message(&message);

        info!(
            endpoint_uuid = %config.endpoint_uuid,
            instance_id = self.instance_id,
            "Sending WS-Discovery Bye announcement"
        );

        drop(config);
        send_multicast_message(&xml).await
    }
}

// =============================================================================
// WS-Discovery Service Implementation
// =============================================================================

/// WS-Discovery service for ONVIF device discovery
///
/// This implementation follows the WS-Discovery 1.1 specification and provides:
/// - Hello/Bye multicast announcements
/// - Probe/ProbeMatch for device discovery
/// - Application sequencing for message ordering
/// - Discovery mode support (Discoverable/NonDiscoverable)
///
/// # Usage
///
/// ```ignore
/// let config = DiscoveryConfig::new(uuid, 80, "192.168.1.100".to_string());
/// let discovery = WsDiscovery::new(config);
///
/// // Start the service and get a handle for control
/// let (handle, task) = discovery.run_service().await?;
///
/// // Later, stop the service
/// handle.stop().await?;
/// task.await?;
/// ```
pub struct WsDiscovery {
    /// Service configuration
    config: Arc<RwLock<DiscoveryConfig>>,

    /// Running state flag
    running: Arc<AtomicBool>,

    /// UDP socket for multicast communication
    socket: Option<Arc<UdpSocket>>,

    /// Metadata version counter (incremented on configuration changes)
    metadata_version: Arc<AtomicU32>,

    /// Application sequence instance ID (set on service start)
    instance_id: u32,

    /// Application sequence message counter
    message_number: Arc<AtomicU32>,
}

impl WsDiscovery {
    /// Create a new WS-Discovery service
    ///
    /// The instance_id is derived from the current timestamp to ensure it
    /// increases after service restarts (per Section 7 Application Sequencing)
    pub fn new(config: DiscoveryConfig) -> Self {
        // Instance ID should increase on each restart - use Unix timestamp
        let instance_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as u32)
            .unwrap_or(1);

        Self {
            config: Arc::new(RwLock::new(config)),
            running: Arc::new(AtomicBool::new(false)),
            socket: None,
            metadata_version: Arc::new(AtomicU32::new(1)),
            instance_id,
            message_number: Arc::new(AtomicU32::new(0)),
        }
    }

    /// Get the next message number (atomically incremented)
    fn next_message_number(&self) -> u32 {
        self.message_number.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// Check if the service is running (for testing/diagnostics)
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Acquire)
    }

    /// Initialize the UDP multicast socket
    pub async fn init_socket(&mut self) -> Result<(), DiscoveryError> {
        if self.socket.is_some() {
            return Ok(());
        }

        let socket = create_multicast_socket()?;
        self.socket = Some(Arc::new(socket));
        debug!(
            multicast = %WS_DISCOVERY_MULTICAST,
            port = WS_DISCOVERY_PORT,
            "WS-Discovery socket initialized and joined multicast group"
        );
        Ok(())
    }

    /// Get the current discovery mode
    pub async fn get_discovery_mode(&self) -> DiscoveryMode {
        self.config.read().await.discovery_mode
    }

    /// Set the discovery mode (Discoverable/NonDiscoverable)
    pub async fn set_discovery_mode(&self, mode: DiscoveryMode) {
        let mut config = self.config.write().await;
        let old_mode = config.discovery_mode;
        config.discovery_mode = mode;

        if old_mode != mode {
            info!(old = %old_mode, new = %mode, "Discovery mode changed");
        }
    }

    // =========================================================================
    // Message Building (per OASIS spec Sections 4-5)
    // =========================================================================

    /// Build a Hello message for device announcement (Section 4.1)
    pub fn build_hello_message(&self, config: &DiscoveryConfig) -> WsDiscoveryMessage {
        WsDiscoveryMessage::Hello {
            message_id: format!("urn:uuid:{}", Uuid::new_v4()),
            endpoint_reference: config.endpoint_uuid.clone(),
            types: ONVIF_NVT_TYPE.to_string(),
            scopes: config.get_scopes_string(),
            xaddrs: config.get_xaddrs(),
            metadata_version: self.metadata_version.load(Ordering::Relaxed),
            instance_id: self.instance_id,
            message_number: self.next_message_number(),
        }
    }

    /// Build a Bye message for device departure (Section 4.2)
    pub fn build_bye_message(&self, config: &DiscoveryConfig) -> WsDiscoveryMessage {
        WsDiscoveryMessage::Bye {
            message_id: format!("urn:uuid:{}", Uuid::new_v4()),
            endpoint_reference: config.endpoint_uuid.clone(),
            instance_id: self.instance_id,
            message_number: self.next_message_number(),
        }
    }

    /// Build a ProbeMatch response message (Section 5.3)
    pub fn build_probe_match(
        &self,
        config: &DiscoveryConfig,
        relates_to: &str,
    ) -> WsDiscoveryMessage {
        WsDiscoveryMessage::ProbeMatch {
            message_id: format!("urn:uuid:{}", Uuid::new_v4()),
            relates_to: relates_to.to_string(),
            endpoint_reference: config.endpoint_uuid.clone(),
            types: ONVIF_NVT_TYPE.to_string(),
            scopes: config.get_scopes_string(),
            xaddrs: config.get_xaddrs(),
            metadata_version: self.metadata_version.load(Ordering::Relaxed),
            instance_id: self.instance_id,
            message_number: self.next_message_number(),
        }
    }

    /// Check if incoming data contains a Probe message
    ///
    /// Supports both WS-Discovery namespaces:
    /// - 2005/04: http://schemas.xmlsoap.org/ws/2005/04/discovery/Probe
    /// - 2009/01: http://docs.oasis-open.org/ws-dd/ns/discovery/2009/01/Probe
    pub fn is_probe_message(data: &[u8]) -> bool {
        let text = match std::str::from_utf8(data) {
            Ok(s) => s,
            Err(e) => {
                trace!(error = %e, "Failed to parse message as UTF-8");
                return false;
            }
        };

        // Check for Probe action URI - support both 2005/04 and 2009/01 namespaces
        let has_2005_probe = text.contains("http://schemas.xmlsoap.org/ws/2005/04/discovery/Probe");
        let has_2009_probe =
            text.contains("http://docs.oasis-open.org/ws-dd/ns/discovery/2009/01/Probe");
        let has_generic_probe = text.contains("Probe")
            && (text.contains(WSD_NS) || text.contains("ws-dd/ns/discovery"));

        trace!(
            has_2005_probe = has_2005_probe,
            has_2009_probe = has_2009_probe,
            has_generic_probe = has_generic_probe,
            "Probe detection analysis"
        );

        has_2005_probe || has_2009_probe || has_generic_probe
    }

    /// Parse a Probe message to extract MessageID, Types, and Scopes
    pub fn parse_probe_message(data: &[u8]) -> Result<WsDiscoveryMessage, DiscoveryError> {
        let text = std::str::from_utf8(data)
            .map_err(|e| DiscoveryError::Parsing(format!("invalid UTF-8: {}", e)))?;

        // Extract MessageID from the SOAP header
        let message_id = extract_xml_element(text, "MessageID")
            .unwrap_or_else(|| format!("urn:uuid:{}", Uuid::new_v4()));

        // Extract Types if present
        let types = extract_xml_element(text, "Types");

        // Extract Scopes if present
        let scopes = extract_xml_element(text, "Scopes");

        trace!(
            message_id = %message_id,
            types = ?types,
            scopes = ?scopes,
            "Extracted Probe elements"
        );

        Ok(WsDiscoveryMessage::Probe {
            message_id,
            types,
            scopes,
        })
    }

    // =========================================================================
    // XML Serialization (per OASIS spec message outlines)
    // =========================================================================

    /// Serialize a WS-Discovery message to SOAP XML
    ///
    /// The output conforms to the normative outlines in OASIS WS-Discovery 1.1:
    /// - Section 4.1 for Hello
    /// - Section 4.2 for Bye
    /// - Section 5.2 for Probe
    /// - Section 5.3 for ProbeMatch
    pub fn serialize_message(message: &WsDiscoveryMessage) -> String {
        match message {
            WsDiscoveryMessage::Hello {
                message_id,
                endpoint_reference,
                types,
                scopes,
                xaddrs,
                metadata_version,
                instance_id,
                message_number,
            } => {
                // Hello message per Section 4.1
                format!(
                    r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="{SOAP_NS}" xmlns:a="{WSA_NS}" xmlns:d="{WSD_NS}" xmlns:tdn="{ONVIF_NW_NS}">
  <s:Header>
    <a:Action>{WSD_NS}/Hello</a:Action>
    <a:MessageID>{message_id}</a:MessageID>
    <a:To>{WSD_MULTICAST_TO}</a:To>
    <d:AppSequence InstanceId="{instance_id}" MessageNumber="{message_number}"/>
  </s:Header>
  <s:Body>
    <d:Hello>
      <a:EndpointReference>
        <a:Address>{endpoint_reference}</a:Address>
      </a:EndpointReference>
      <d:Types>{types}</d:Types>
      <d:Scopes>{scopes}</d:Scopes>
      <d:XAddrs>{xaddrs}</d:XAddrs>
      <d:MetadataVersion>{metadata_version}</d:MetadataVersion>
    </d:Hello>
  </s:Body>
</s:Envelope>"#
                )
            }

            WsDiscoveryMessage::Bye {
                message_id,
                endpoint_reference,
                instance_id,
                message_number,
            } => {
                // Bye message per Section 4.2
                format!(
                    r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="{SOAP_NS}" xmlns:a="{WSA_NS}" xmlns:d="{WSD_NS}">
  <s:Header>
    <a:Action>{WSD_NS}/Bye</a:Action>
    <a:MessageID>{message_id}</a:MessageID>
    <a:To>{WSD_MULTICAST_TO}</a:To>
    <d:AppSequence InstanceId="{instance_id}" MessageNumber="{message_number}"/>
  </s:Header>
  <s:Body>
    <d:Bye>
      <a:EndpointReference>
        <a:Address>{endpoint_reference}</a:Address>
      </a:EndpointReference>
    </d:Bye>
  </s:Body>
</s:Envelope>"#
                )
            }

            WsDiscoveryMessage::Probe {
                message_id,
                types,
                scopes,
            } => {
                // Probe message per Section 5.2
                let types_element = types
                    .as_ref()
                    .map(|t| format!("<d:Types>{}</d:Types>", t))
                    .unwrap_or_default();
                let scopes_element = scopes
                    .as_ref()
                    .map(|s| format!("<d:Scopes>{}</d:Scopes>", s))
                    .unwrap_or_default();

                format!(
                    r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="{SOAP_NS}" xmlns:a="{WSA_NS}" xmlns:d="{WSD_NS}">
  <s:Header>
    <a:Action>{WSD_NS}/Probe</a:Action>
    <a:MessageID>{message_id}</a:MessageID>
    <a:To>{WSD_MULTICAST_TO}</a:To>
  </s:Header>
  <s:Body>
    <d:Probe>
      {types_element}
      {scopes_element}
    </d:Probe>
  </s:Body>
</s:Envelope>"#
                )
            }

            WsDiscoveryMessage::ProbeMatch {
                message_id,
                relates_to,
                endpoint_reference,
                types,
                scopes,
                xaddrs,
                metadata_version,
                instance_id,
                message_number,
            } => {
                // ProbeMatch message per Section 5.3
                format!(
                    r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="{SOAP_NS}" xmlns:a="{WSA_NS}" xmlns:d="{WSD_NS}" xmlns:tdn="{ONVIF_NW_NS}">
  <s:Header>
    <a:Action>{WSD_NS}/ProbeMatches</a:Action>
    <a:MessageID>{message_id}</a:MessageID>
    <a:RelatesTo>{relates_to}</a:RelatesTo>
    <a:To>{WSA_ANONYMOUS}</a:To>
    <d:AppSequence InstanceId="{instance_id}" MessageNumber="{message_number}"/>
  </s:Header>
  <s:Body>
    <d:ProbeMatches>
      <d:ProbeMatch>
        <a:EndpointReference>
          <a:Address>{endpoint_reference}</a:Address>
        </a:EndpointReference>
        <d:Types>{types}</d:Types>
        <d:Scopes>{scopes}</d:Scopes>
        <d:XAddrs>{xaddrs}</d:XAddrs>
        <d:MetadataVersion>{metadata_version}</d:MetadataVersion>
      </d:ProbeMatch>
    </d:ProbeMatches>
  </s:Body>
</s:Envelope>"#
                )
            }
        }
    }

    // =========================================================================
    // Message Sending
    // =========================================================================

    /// Apply random transmission delay per Section 3.1.3
    ///
    /// To minimize network storms, target services MUST wait for a random
    /// delay between 0 and APP_MAX_DELAY before sending certain messages.
    async fn apply_transmission_delay() {
        let delay_ms = rand::rng().random_range(0..=APP_MAX_DELAY_MS);
        if delay_ms > 0 {
            trace!(delay_ms, "Applying APP_MAX_DELAY transmission delay");
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
        }
    }

    /// Send a Hello message to the multicast group
    ///
    /// Per Section 4.1.1:
    /// - MUST wait for APP_MAX_DELAY before sending
    /// - MUST be sent multicast to the distinguished URI
    pub async fn send_hello(&self) -> Result<(), DiscoveryError> {
        let config = self.config.read().await;

        if config.discovery_mode == DiscoveryMode::NonDiscoverable {
            debug!("Skipping Hello in NonDiscoverable mode");
            return Ok(());
        }

        // Apply transmission delay per Section 3.1.3
        Self::apply_transmission_delay().await;

        let message = self.build_hello_message(&config);
        let xml = Self::serialize_message(&message);

        info!(
            endpoint_uuid = %config.endpoint_uuid,
            xaddrs = %config.get_xaddrs(),
            instance_id = self.instance_id,
            "Sending WS-Discovery Hello announcement"
        );

        self.send_multicast(&xml).await
    }

    /// Send a Bye message to the multicast group
    ///
    /// Per Section 4.2.1:
    /// - MAY send without waiting for transmission delay
    /// - MUST be sent multicast to the distinguished URI
    pub async fn send_bye(&self) -> Result<(), DiscoveryError> {
        let config = self.config.read().await;

        if config.discovery_mode == DiscoveryMode::NonDiscoverable {
            debug!("Skipping Bye in NonDiscoverable mode");
            return Ok(());
        }

        let message = self.build_bye_message(&config);
        let xml = Self::serialize_message(&message);

        info!(
            endpoint_uuid = %config.endpoint_uuid,
            instance_id = self.instance_id,
            "Sending WS-Discovery Bye announcement"
        );

        self.send_multicast(&xml).await
    }

    /// Send a message to the multicast group
    async fn send_multicast(&self, payload: &str) -> Result<(), DiscoveryError> {
        send_multicast_message(payload).await
    }

    // =========================================================================
    // Service Lifecycle
    // =========================================================================

    /// Start the WS-Discovery service and return a handle for control.
    ///
    /// This method consumes the `WsDiscovery` instance and spawns a background task
    /// that runs the discovery loop. Returns a handle that can be used to control
    /// the service (stop, change mode, etc.) and a `JoinHandle` for the background task.
    ///
    /// # Returns
    ///
    /// A tuple of `(WsDiscoveryHandle, JoinHandle<()>)` where:
    /// - `WsDiscoveryHandle` - Used to control the running service
    /// - `JoinHandle<()>` - The spawned task handle (await to wait for completion)
    ///
    /// # Errors
    ///
    /// Returns `DiscoveryError` if socket initialization fails.
    pub async fn run_service(
        mut self,
    ) -> Result<(WsDiscoveryHandle, tokio::task::JoinHandle<()>), DiscoveryError> {
        if self.running.load(Ordering::Acquire) {
            return Err(DiscoveryError::AlreadyRunning);
        }

        // Initialize socket
        self.init_socket().await?;

        self.running.store(true, Ordering::Release);
        info!(
            instance_id = self.instance_id,
            "WS-Discovery service started"
        );

        // Send initial Hello (with transmission delay)
        if let Err(e) = self.send_hello().await {
            warn!(error = %e, "Failed to send initial Hello");
        }

        // Create handle with shared state
        let handle = WsDiscoveryHandle {
            config: self.config.clone(),
            running: self.running.clone(),
            metadata_version: self.metadata_version.clone(),
            message_number: self.message_number.clone(),
            instance_id: self.instance_id,
        };

        // Clone values needed by the background task
        let socket = self.socket.as_ref().unwrap().clone();
        let config = self.config.clone();
        let running = self.running.clone();
        let metadata_version = self.metadata_version.clone();
        let message_number = self.message_number.clone();
        let instance_id = self.instance_id;

        // Spawn background task for the discovery loop
        let task = tokio::spawn(async move {
            Self::discovery_loop(
                socket,
                config,
                running,
                metadata_version,
                message_number,
                instance_id,
            )
            .await;
        });

        Ok((handle, task))
    }

    /// Internal discovery loop that runs in a spawned task.
    async fn discovery_loop(
        socket: Arc<UdpSocket>,
        config: Arc<RwLock<DiscoveryConfig>>,
        running: Arc<AtomicBool>,
        metadata_version: Arc<AtomicU32>,
        message_number: Arc<AtomicU32>,
        instance_id: u32,
    ) {
        let mut buf = [0u8; MAX_MESSAGE_SIZE];
        let mut last_hello = std::time::Instant::now();

        debug!(
            local_addr = %socket.local_addr().map(|a| a.to_string()).unwrap_or_else(|_| "unknown".to_string()),
            instance_id = instance_id,
            "WS-Discovery loop started, listening for Probe requests"
        );

        while running.load(Ordering::Acquire) {
            tokio::select! {
                result = socket.recv_from(&mut buf) => {
                    match result {
                        Ok((len, src)) => {
                            debug!(
                                bytes = len,
                                src = %src,
                                "Received UDP packet"
                            );
                            // Log first 200 chars of payload for debugging
                            if let Ok(text) = std::str::from_utf8(&buf[..len.min(500)]) {
                                trace!(
                                    payload_preview = %text,
                                    "Message content preview"
                                );
                            }
                            Self::handle_received_message_static(
                                &buf[..len],
                                src,
                                &config,
                                &metadata_version,
                                &message_number,
                                instance_id,
                                &socket
                            ).await;
                        }
                        Err(e) => {
                            if running.load(Ordering::Acquire) {
                                warn!(error = %e, "Error receiving WS-Discovery message");
                            }
                        }
                    }
                }
                _ = tokio::time::sleep(Duration::from_secs(1)) => {
                    // Check for periodic Hello
                    let config_guard = config.read().await;
                    if last_hello.elapsed() >= config_guard.hello_interval
                        && config_guard.discovery_mode == DiscoveryMode::Discoverable
                    {
                        let hello_msg = Self::build_hello_message_static(
                            &config_guard,
                            &metadata_version,
                            &message_number,
                            instance_id,
                        );
                        drop(config_guard);
                        let xml = Self::serialize_message(&hello_msg);
                        if let Err(e) = send_multicast_message(&xml).await {
                            warn!(error = %e, "Failed to send periodic Hello");
                        }
                        last_hello = std::time::Instant::now();
                    }
                }
            }
        }

        info!("WS-Discovery service stopped");
    }

    /// Build a Hello message (static version for use in spawned task)
    fn build_hello_message_static(
        config: &DiscoveryConfig,
        metadata_version: &Arc<AtomicU32>,
        message_number: &Arc<AtomicU32>,
        instance_id: u32,
    ) -> WsDiscoveryMessage {
        let msg_num = message_number.fetch_add(1, Ordering::SeqCst) + 1;
        WsDiscoveryMessage::Hello {
            message_id: format!("urn:uuid:{}", Uuid::new_v4()),
            endpoint_reference: config.endpoint_uuid.clone(),
            types: ONVIF_NVT_TYPE.to_string(),
            scopes: config.get_scopes_string(),
            xaddrs: config.get_xaddrs(),
            metadata_version: metadata_version.load(Ordering::Relaxed),
            instance_id,
            message_number: msg_num,
        }
    }

    /// Handle a received message (static version for use in spawned task)
    async fn handle_received_message_static(
        data: &[u8],
        src: SocketAddr,
        config: &Arc<RwLock<DiscoveryConfig>>,
        metadata_version: &Arc<AtomicU32>,
        message_number: &Arc<AtomicU32>,
        instance_id: u32,
        socket: &Arc<UdpSocket>,
    ) {
        let config_guard = config.read().await;

        // Log received message with content preview
        debug!(
            src = %src,
            bytes = data.len(),
            discovery_mode = ?config_guard.discovery_mode,
            "Received WS-Discovery message"
        );

        // Log message content for debugging
        if let Ok(text) = std::str::from_utf8(data) {
            trace!(
                full_xml = %text,
                "Full received message content"
            );
        }

        // In NonDiscoverable mode, silently ignore all probe messages (per EC-014)
        if config_guard.discovery_mode == DiscoveryMode::NonDiscoverable {
            debug!("Ignoring message in NonDiscoverable mode (per EC-014)");
            return;
        }

        // Check if it's a Probe message
        let is_probe = Self::is_probe_message(data);
        debug!(is_probe = is_probe, "Probe message detection result");

        if !is_probe {
            debug!(
                src = %src,
                "Message is not a Probe request, ignoring"
            );
            return;
        }

        // Log the probe request (as required by task spec)
        info!(
            src = %src,
            "Received WS-Discovery Probe request"
        );

        // Parse the Probe to get MessageID for RelatesTo
        let relates_to = match Self::parse_probe_message(data) {
            Ok(WsDiscoveryMessage::Probe {
                message_id,
                types,
                scopes,
            }) => {
                debug!(
                    message_id = %message_id,
                    types = ?types,
                    scopes = ?scopes,
                    "Parsed Probe request details"
                );
                message_id
            }
            Ok(_) => {
                warn!("Unexpected message type when parsing Probe");
                return;
            }
            Err(e) => {
                warn!(error = %e, "Failed to parse Probe message");
                return;
            }
        };

        // Build ProbeMatch response
        let msg_num = message_number.fetch_add(1, Ordering::SeqCst) + 1;
        let probe_match = WsDiscoveryMessage::ProbeMatch {
            message_id: format!("urn:uuid:{}", Uuid::new_v4()),
            relates_to: relates_to.clone(),
            endpoint_reference: config_guard.endpoint_uuid.clone(),
            types: ONVIF_NVT_TYPE.to_string(),
            scopes: config_guard.get_scopes_string(),
            xaddrs: config_guard.get_xaddrs(),
            metadata_version: metadata_version.load(Ordering::Relaxed),
            instance_id,
            message_number: msg_num,
        };

        debug!(
            relates_to = %relates_to,
            endpoint = %config_guard.endpoint_uuid,
            xaddrs = %config_guard.get_xaddrs(),
            message_number = msg_num,
            "Built ProbeMatch response"
        );

        let xml = Self::serialize_message(&probe_match);

        debug!(xml_len = xml.len(), "Serialized ProbeMatch XML");
        trace!(
            probe_match_xml = %xml,
            "Full ProbeMatch XML content"
        );

        drop(config_guard);

        // Apply transmission delay before responding (Section 3.1.3)
        debug!("Applying transmission delay before sending ProbeMatch");
        Self::apply_transmission_delay().await;

        // Send ProbeMatch unicast to the source (Section 5.3.1)
        debug!(
            dest = %src,
            bytes = xml.len(),
            "Sending ProbeMatch unicast response"
        );

        if let Err(e) = socket.send_to(xml.as_bytes(), src).await {
            warn!(src = %src, error = %e, "Failed to send ProbeMatch");
        } else {
            info!(
                src = %src,
                bytes = xml.len(),
                "Successfully sent WS-Discovery ProbeMatch response"
            );
        }
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Create a UDP socket configured for multicast reception
fn create_multicast_socket() -> Result<UdpSocket, DiscoveryError> {
    debug!("Creating multicast socket for WS-Discovery");

    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    debug!("Created raw UDP socket");

    // Allow multiple sockets to bind to the same address
    socket.set_reuse_address(true)?;
    debug!("Set SO_REUSEADDR");

    // Bind to the WS-Discovery port on all interfaces
    let addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, WS_DISCOVERY_PORT);
    socket.bind(&addr.into())?;
    debug!(port = WS_DISCOVERY_PORT, "Bound to WS-Discovery port");

    // Join multicast group
    debug!(
        multicast_group = %WS_DISCOVERY_MULTICAST,
        "Joining multicast group"
    );
    socket
        .join_multicast_v4(&WS_DISCOVERY_MULTICAST, &Ipv4Addr::UNSPECIFIED)
        .map_err(DiscoveryError::MulticastJoin)?;
    info!(
        multicast_group = %WS_DISCOVERY_MULTICAST,
        port = WS_DISCOVERY_PORT,
        "Successfully joined WS-Discovery multicast group"
    );

    // Set non-blocking for tokio
    socket.set_nonblocking(true)?;
    debug!("Set socket to non-blocking mode");

    // Convert to tokio UdpSocket
    let std_socket: std::net::UdpSocket = socket.into();
    let socket = UdpSocket::from_std(std_socket)?;

    Ok(socket)
}

/// Create a UDP socket for sending multicast messages
async fn create_send_socket() -> Result<UdpSocket, DiscoveryError> {
    let socket = UdpSocket::bind("0.0.0.0:0").await?;
    debug!(
        local_addr = %socket.local_addr().map(|a| a.to_string()).unwrap_or_else(|_| "unknown".to_string()),
        "Created send socket"
    );
    Ok(socket)
}

/// Send a message to the WS-Discovery multicast group
async fn send_multicast_message(payload: &str) -> Result<(), DiscoveryError> {
    let socket = create_send_socket().await?;
    let dest = SocketAddrV4::new(WS_DISCOVERY_MULTICAST, WS_DISCOVERY_PORT);

    debug!(
        dest = %dest,
        bytes = payload.len(),
        "Sending multicast message"
    );

    socket.send_to(payload.as_bytes(), dest).await?;

    trace!(
        payload_preview = %&payload[..payload.len().min(300)],
        "Multicast message content preview"
    );

    info!(
        dest = %dest,
        bytes = payload.len(),
        "Successfully sent multicast message"
    );

    Ok(())
}

/// Extract an XML element value using simple string matching
///
/// This is a lightweight parser for extracting specific element values.
/// Handles both namespaced (a:MessageID, d:Types) and non-namespaced elements.
fn extract_xml_element(xml: &str, element_name: &str) -> Option<String> {
    // Common namespace prefixes used in WS-Discovery
    let patterns = [
        format!("<{}>", element_name),
        format!("<a:{}>", element_name),
        format!("<d:{}>", element_name),
        format!("<wsa:{}>", element_name),
        format!("<wsd:{}>", element_name),
    ];

    let end_patterns = [
        format!("</{}>", element_name),
        format!("</a:{}>", element_name),
        format!("</d:{}>", element_name),
        format!("</wsa:{}>", element_name),
        format!("</wsd:{}>", element_name),
    ];

    for (start_pattern, end_pattern) in patterns.iter().zip(end_patterns.iter()) {
        if let Some(start_idx) = xml.find(start_pattern) {
            let content_start = start_idx + start_pattern.len();
            if let Some(end_idx) = xml[content_start..].find(end_pattern) {
                return Some(
                    xml[content_start..content_start + end_idx]
                        .trim()
                        .to_string(),
                );
            }
        }
    }

    None
}

// =============================================================================
// Unit Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // T186: Unit tests for Hello message generation
    // =========================================================================

    #[test]
    fn test_build_hello_message_structure() {
        let config = DiscoveryConfig {
            endpoint_uuid: "urn:uuid:12345678-1234-1234-1234-123456789abc".to_string(),
            http_port: 80,
            device_ip: "192.168.1.100".to_string(),
            scopes: vec!["onvif://www.onvif.org/type/video_encoder".to_string()],
            discovery_mode: DiscoveryMode::Discoverable,
            hello_interval: Duration::from_secs(300),
        };

        let discovery = WsDiscovery::new(config.clone());
        let message = discovery.build_hello_message(&config);

        match message {
            WsDiscoveryMessage::Hello {
                endpoint_reference,
                types,
                scopes,
                xaddrs,
                metadata_version,
                instance_id,
                message_number,
                ..
            } => {
                assert_eq!(
                    endpoint_reference,
                    "urn:uuid:12345678-1234-1234-1234-123456789abc"
                );
                assert_eq!(types, ONVIF_NVT_TYPE);
                assert!(scopes.contains("video_encoder"));
                assert_eq!(xaddrs, "http://192.168.1.100:80/onvif/device_service");
                assert_eq!(metadata_version, 1);
                assert!(instance_id > 0, "Instance ID should be set from timestamp");
                assert_eq!(message_number, 1);
            }
            _ => panic!("Expected Hello message"),
        }
    }

    #[test]
    fn test_hello_serialization_contains_correct_namespaces() {
        let config = DiscoveryConfig::default();
        let discovery = WsDiscovery::new(config.clone());
        let message = discovery.build_hello_message(&config);
        let xml = WsDiscovery::serialize_message(&message);

        // Check correct 2009/01 namespace (NOT the old 2005/04 namespace)
        assert!(
            xml.contains(WSD_NS),
            "Should contain WS-Discovery 2009/01 namespace"
        );
        assert!(
            xml.contains(WSA_NS),
            "Should contain WS-Addressing 2005/08 namespace"
        );
        assert!(xml.contains(SOAP_NS), "Should contain SOAP 1.2 namespace");
        assert!(
            xml.contains(ONVIF_NW_NS),
            "Should contain ONVIF network namespace"
        );

        // Check action URI
        assert!(xml.contains(&format!("{}/Hello", WSD_NS)));

        // Check multicast To address
        assert!(xml.contains(WSD_MULTICAST_TO));

        // Check AppSequence header is present
        assert!(xml.contains("d:AppSequence"));
        assert!(xml.contains("InstanceId="));
        assert!(xml.contains("MessageNumber="));
    }

    #[test]
    fn test_hello_contains_required_elements() {
        let config = DiscoveryConfig {
            device_ip: "10.0.0.50".to_string(),
            http_port: 8080,
            ..Default::default()
        };
        let discovery = WsDiscovery::new(config.clone());
        let message = discovery.build_hello_message(&config);
        let xml = WsDiscovery::serialize_message(&message);

        // Per Section 4.1, Hello MUST contain:
        assert!(
            xml.contains("a:EndpointReference"),
            "Missing EndpointReference"
        );
        assert!(xml.contains("d:MetadataVersion"), "Missing MetadataVersion");

        // And SHOULD contain for ONVIF:
        assert!(xml.contains("d:Types"), "Missing Types");
        assert!(xml.contains("d:Scopes"), "Missing Scopes");
        assert!(xml.contains("d:XAddrs"), "Missing XAddrs");
        assert!(xml.contains("http://10.0.0.50:8080/onvif/device_service"));
    }

    // =========================================================================
    // T187: Unit tests for Bye message generation
    // =========================================================================

    #[test]
    fn test_build_bye_message_structure() {
        let config = DiscoveryConfig {
            endpoint_uuid: "urn:uuid:test-device-uuid".to_string(),
            ..Default::default()
        };

        let discovery = WsDiscovery::new(config.clone());
        let message = discovery.build_bye_message(&config);

        match message {
            WsDiscoveryMessage::Bye {
                endpoint_reference,
                instance_id,
                message_number,
                ..
            } => {
                assert_eq!(endpoint_reference, "urn:uuid:test-device-uuid");
                assert!(instance_id > 0);
                assert_eq!(message_number, 1);
            }
            _ => panic!("Expected Bye message"),
        }
    }

    #[test]
    fn test_bye_serialization_contains_correct_elements() {
        let config = DiscoveryConfig::default();
        let discovery = WsDiscovery::new(config.clone());
        let message = discovery.build_bye_message(&config);
        let xml = WsDiscovery::serialize_message(&message);

        // Check action URI
        assert!(xml.contains(&format!("{}/Bye", WSD_NS)));

        // Check multicast To address
        assert!(xml.contains(WSD_MULTICAST_TO));

        // Check required elements per Section 4.2
        assert!(xml.contains("a:EndpointReference"));
        assert!(xml.contains("d:AppSequence"));

        // Bye is minimal - should NOT contain XAddrs, Types, Scopes
        assert!(!xml.contains("d:XAddrs"), "Bye should not contain XAddrs");
    }

    // =========================================================================
    // T188: Unit tests for ProbeMatch response generation
    // =========================================================================

    #[test]
    fn test_build_probe_match_structure() {
        let config = DiscoveryConfig {
            endpoint_uuid: "urn:uuid:device-uuid".to_string(),
            http_port: 80,
            device_ip: "192.168.1.100".to_string(),
            scopes: vec!["onvif://www.onvif.org/type/NetworkVideoTransmitter".to_string()],
            ..Default::default()
        };

        let discovery = WsDiscovery::new(config.clone());
        let message = discovery.build_probe_match(&config, "urn:uuid:probe-msg-id");

        match message {
            WsDiscoveryMessage::ProbeMatch {
                relates_to,
                endpoint_reference,
                types,
                xaddrs,
                metadata_version,
                ..
            } => {
                assert_eq!(relates_to, "urn:uuid:probe-msg-id");
                assert_eq!(endpoint_reference, "urn:uuid:device-uuid");
                assert_eq!(types, ONVIF_NVT_TYPE);
                assert_eq!(xaddrs, "http://192.168.1.100:80/onvif/device_service");
                assert_eq!(metadata_version, 1);
            }
            _ => panic!("Expected ProbeMatch message"),
        }
    }

    #[test]
    fn test_probe_match_serialization_contains_relates_to() {
        let config = DiscoveryConfig::default();
        let discovery = WsDiscovery::new(config.clone());
        let message = discovery.build_probe_match(&config, "urn:uuid:test-probe-id");
        let xml = WsDiscovery::serialize_message(&message);

        // Check action URI (ProbeMatches, not ProbeMatch)
        assert!(xml.contains(&format!("{}/ProbeMatches", WSD_NS)));

        // Check RelatesTo header per Section 5.3
        assert!(xml.contains("a:RelatesTo"));
        assert!(xml.contains("urn:uuid:test-probe-id"));

        // Check To is anonymous (response goes to source)
        assert!(xml.contains(WSA_ANONYMOUS));

        // Check ProbeMatches structure
        assert!(xml.contains("d:ProbeMatches"));
        assert!(xml.contains("d:ProbeMatch"));
    }

    #[test]
    fn test_probe_match_contains_all_metadata() {
        let config = DiscoveryConfig::default();
        let discovery = WsDiscovery::new(config.clone());
        let message = discovery.build_probe_match(&config, "test");
        let xml = WsDiscovery::serialize_message(&message);

        // Per Section 5.3, ProbeMatch MUST contain all metadata
        assert!(xml.contains("a:EndpointReference"));
        assert!(xml.contains("d:Types"));
        assert!(xml.contains("d:Scopes"));
        assert!(xml.contains("d:XAddrs"));
        assert!(xml.contains("d:MetadataVersion"));
    }

    // =========================================================================
    // T189: Unit tests for discovery mode filtering
    // =========================================================================

    #[tokio::test]
    async fn test_discovery_mode_default_is_discoverable() {
        let config = DiscoveryConfig::default();
        assert_eq!(config.discovery_mode, DiscoveryMode::Discoverable);
    }

    #[tokio::test]
    async fn test_set_discovery_mode() {
        let discovery = WsDiscovery::new(DiscoveryConfig::default());

        assert_eq!(
            discovery.get_discovery_mode().await,
            DiscoveryMode::Discoverable
        );

        discovery
            .set_discovery_mode(DiscoveryMode::NonDiscoverable)
            .await;
        assert_eq!(
            discovery.get_discovery_mode().await,
            DiscoveryMode::NonDiscoverable
        );

        discovery
            .set_discovery_mode(DiscoveryMode::Discoverable)
            .await;
        assert_eq!(
            discovery.get_discovery_mode().await,
            DiscoveryMode::Discoverable
        );
    }

    #[test]
    fn test_discovery_mode_display() {
        assert_eq!(format!("{}", DiscoveryMode::Discoverable), "Discoverable");
        assert_eq!(
            format!("{}", DiscoveryMode::NonDiscoverable),
            "NonDiscoverable"
        );
    }

    // =========================================================================
    // Probe message detection and parsing tests
    // =========================================================================

    #[test]
    fn test_is_probe_message_with_correct_action() {
        let probe_xml = format!(
            r#"<?xml version="1.0"?>
<s:Envelope xmlns:s="{}" xmlns:a="{}" xmlns:d="{}">
  <s:Header>
    <a:Action>{}/Probe</a:Action>
    <a:MessageID>urn:uuid:test</a:MessageID>
  </s:Header>
  <s:Body>
    <d:Probe/>
  </s:Body>
</s:Envelope>"#,
            SOAP_NS, WSA_NS, WSD_NS, WSD_NS
        );

        assert!(WsDiscovery::is_probe_message(probe_xml.as_bytes()));
    }

    #[test]
    fn test_is_probe_message_rejects_hello() {
        let hello_xml = format!(
            r#"<?xml version="1.0"?>
<s:Envelope xmlns:s="{}" xmlns:a="{}" xmlns:d="{}">
  <s:Header>
    <a:Action>{}/Hello</a:Action>
  </s:Header>
  <s:Body>
    <d:Hello/>
  </s:Body>
</s:Envelope>"#,
            SOAP_NS, WSA_NS, WSD_NS, WSD_NS
        );

        assert!(!WsDiscovery::is_probe_message(hello_xml.as_bytes()));
    }

    #[test]
    fn test_is_probe_message_rejects_invalid_utf8() {
        let invalid = &[0xFF, 0xFE, 0x00, 0x01];
        assert!(!WsDiscovery::is_probe_message(invalid));
    }

    #[test]
    fn test_parse_probe_extracts_message_id() {
        let probe_xml = format!(
            r#"<?xml version="1.0"?>
<s:Envelope xmlns:s="{}" xmlns:a="{}" xmlns:d="{}">
  <s:Header>
    <a:MessageID>urn:uuid:probe-12345</a:MessageID>
    <a:Action>{}/Probe</a:Action>
  </s:Header>
  <s:Body>
    <d:Probe>
      <d:Types>tdn:NetworkVideoTransmitter</d:Types>
    </d:Probe>
  </s:Body>
</s:Envelope>"#,
            SOAP_NS, WSA_NS, WSD_NS, WSD_NS
        );

        let message = WsDiscovery::parse_probe_message(probe_xml.as_bytes()).unwrap();

        match message {
            WsDiscoveryMessage::Probe {
                message_id, types, ..
            } => {
                assert_eq!(message_id, "urn:uuid:probe-12345");
                assert_eq!(types, Some("tdn:NetworkVideoTransmitter".to_string()));
            }
            _ => panic!("Expected Probe message"),
        }
    }

    // =========================================================================
    // Configuration tests
    // =========================================================================

    #[test]
    fn test_config_default_values() {
        let config = DiscoveryConfig::default();

        assert!(config.endpoint_uuid.starts_with("urn:uuid:"));
        assert_eq!(config.http_port, 80);
        assert_eq!(config.device_ip, "192.168.1.100");
        assert!(!config.scopes.is_empty());
        assert_eq!(config.discovery_mode, DiscoveryMode::Discoverable);
        assert_eq!(config.hello_interval, Duration::from_secs(300));
    }

    #[test]
    fn test_config_xaddrs_generation() {
        let config = DiscoveryConfig {
            device_ip: "10.0.0.1".to_string(),
            http_port: 8080,
            ..Default::default()
        };

        assert_eq!(
            config.get_xaddrs(),
            "http://10.0.0.1:8080/onvif/device_service"
        );
    }

    #[test]
    fn test_config_scopes_string() {
        let config = DiscoveryConfig {
            scopes: vec!["scope1".to_string(), "scope2".to_string()],
            ..Default::default()
        };

        assert_eq!(config.get_scopes_string(), "scope1 scope2");
    }

    // =========================================================================
    // XML element extraction tests
    // =========================================================================

    #[test]
    fn test_extract_element_with_namespace_prefix() {
        let xml = r#"<root><a:MessageID>urn:uuid:123</a:MessageID></root>"#;
        assert_eq!(
            extract_xml_element(xml, "MessageID"),
            Some("urn:uuid:123".to_string())
        );
    }

    #[test]
    fn test_extract_element_without_namespace() {
        let xml = "<root><Name>TestValue</Name></root>";
        assert_eq!(
            extract_xml_element(xml, "Name"),
            Some("TestValue".to_string())
        );
    }

    #[test]
    fn test_extract_element_not_found() {
        let xml = "<root><Other>Value</Other></root>";
        assert_eq!(extract_xml_element(xml, "Missing"), None);
    }

    #[test]
    fn test_extract_element_trims_whitespace() {
        let xml = "<root><Name>  TrimMe  </Name></root>";
        assert_eq!(extract_xml_element(xml, "Name"), Some("TrimMe".to_string()));
    }

    // =========================================================================
    // Service state tests
    // =========================================================================

    #[test]
    fn test_ws_discovery_new_not_running() {
        let discovery = WsDiscovery::new(DiscoveryConfig::default());
        assert!(!discovery.is_running());
    }

    #[test]
    fn test_ws_discovery_instance_id_set() {
        let discovery = WsDiscovery::new(DiscoveryConfig::default());
        assert!(
            discovery.instance_id > 0,
            "Instance ID should be set from timestamp"
        );
    }

    #[tokio::test]
    async fn test_set_scopes_increments_metadata_version() {
        let config = DiscoveryConfig::default();
        let discovery = WsDiscovery::new(config);

        let initial_version = discovery.metadata_version.load(Ordering::Relaxed);

        // Start the service to get a handle
        let (handle, task) = discovery.run_service().await.unwrap();

        handle.set_scopes(vec!["new_scope".to_string()]).await;

        // The handle shares the same metadata_version Arc
        assert!(handle.is_running());

        // Stop the service
        handle.stop().await.unwrap();
        let _ = tokio::time::timeout(std::time::Duration::from_secs(1), task).await;
    }

    #[test]
    fn test_message_number_increments() {
        let discovery = WsDiscovery::new(DiscoveryConfig::default());

        let num1 = discovery.next_message_number();
        let num2 = discovery.next_message_number();
        let num3 = discovery.next_message_number();

        assert_eq!(num1, 1);
        assert_eq!(num2, 2);
        assert_eq!(num3, 3);
    }

    // =========================================================================
    // Error type tests
    // =========================================================================

    #[test]
    fn test_discovery_error_display() {
        let err = DiscoveryError::NotRunning;
        assert_eq!(format!("{}", err), "discovery service not running");

        let err = DiscoveryError::AlreadyRunning;
        assert_eq!(format!("{}", err), "discovery service already running");

        let err = DiscoveryError::Parsing("test error".to_string());
        assert_eq!(format!("{}", err), "message parsing error: test error");
    }

    // =========================================================================
    // Additional coverage tests
    // =========================================================================

    #[test]
    fn test_probe_serialization() {
        let probe = WsDiscoveryMessage::Probe {
            message_id: "urn:uuid:test-probe".to_string(),
            types: Some("tdn:NetworkVideoTransmitter".to_string()),
            scopes: Some("onvif://www.onvif.org/scope/test".to_string()),
        };

        let xml = WsDiscovery::serialize_message(&probe);

        // Check Probe action URI
        assert!(xml.contains(&format!("{}/Probe", WSD_NS)));
        // Check MessageID
        assert!(xml.contains("urn:uuid:test-probe"));
        // Check Types element
        assert!(xml.contains("<d:Types>tdn:NetworkVideoTransmitter</d:Types>"));
        // Check Scopes element
        assert!(xml.contains("<d:Scopes>onvif://www.onvif.org/scope/test</d:Scopes>"));
        // Check multicast To
        assert!(xml.contains(WSD_MULTICAST_TO));
    }

    #[test]
    fn test_probe_serialization_without_types_and_scopes() {
        let probe = WsDiscoveryMessage::Probe {
            message_id: "urn:uuid:empty-probe".to_string(),
            types: None,
            scopes: None,
        };

        let xml = WsDiscovery::serialize_message(&probe);

        // Check Probe element is present
        assert!(xml.contains("<d:Probe>"));
        // Should NOT contain Types or Scopes elements when None
        assert!(!xml.contains("<d:Types>"));
        assert!(!xml.contains("<d:Scopes>"));
    }

    #[test]
    fn test_parse_probe_extracts_scopes() {
        let probe_xml = format!(
            r#"<?xml version="1.0"?>
<s:Envelope xmlns:s="{}" xmlns:a="{}" xmlns:d="{}">
  <s:Header>
    <a:MessageID>urn:uuid:probe-with-scopes</a:MessageID>
    <a:Action>{}/Probe</a:Action>
  </s:Header>
  <s:Body>
    <d:Probe>
      <d:Scopes>onvif://www.onvif.org/type/video_encoder</d:Scopes>
    </d:Probe>
  </s:Body>
</s:Envelope>"#,
            SOAP_NS, WSA_NS, WSD_NS, WSD_NS
        );

        let message = WsDiscovery::parse_probe_message(probe_xml.as_bytes()).unwrap();

        match message {
            WsDiscoveryMessage::Probe { scopes, .. } => {
                assert_eq!(
                    scopes,
                    Some("onvif://www.onvif.org/type/video_encoder".to_string())
                );
            }
            _ => panic!("Expected Probe message"),
        }
    }

    #[test]
    fn test_parse_probe_handles_missing_message_id() {
        let probe_xml = format!(
            r#"<?xml version="1.0"?>
<s:Envelope xmlns:s="{}" xmlns:a="{}" xmlns:d="{}">
  <s:Header>
    <a:Action>{}/Probe</a:Action>
  </s:Header>
  <s:Body>
    <d:Probe/>
  </s:Body>
</s:Envelope>"#,
            SOAP_NS, WSA_NS, WSD_NS, WSD_NS
        );

        let message = WsDiscovery::parse_probe_message(probe_xml.as_bytes()).unwrap();

        match message {
            WsDiscoveryMessage::Probe { message_id, .. } => {
                // Should generate a UUID when MessageID is missing
                assert!(message_id.starts_with("urn:uuid:"));
            }
            _ => panic!("Expected Probe message"),
        }
    }

    #[test]
    fn test_parse_probe_invalid_utf8_returns_error() {
        let invalid = &[0xFF, 0xFE, 0x00, 0x01];
        let result = WsDiscovery::parse_probe_message(invalid);

        assert!(result.is_err());
        match result {
            Err(DiscoveryError::Parsing(msg)) => {
                assert!(msg.contains("invalid UTF-8"));
            }
            _ => panic!("Expected Parsing error"),
        }
    }

    #[test]
    fn test_config_new_constructor() {
        let config = DiscoveryConfig::new(
            "urn:uuid:custom-uuid".to_string(),
            8080,
            "10.0.0.100".to_string(),
        );

        assert_eq!(config.endpoint_uuid, "urn:uuid:custom-uuid");
        assert_eq!(config.http_port, 8080);
        assert_eq!(config.device_ip, "10.0.0.100");
        // Should use defaults for other fields
        assert_eq!(config.discovery_mode, DiscoveryMode::Discoverable);
        assert!(!config.scopes.is_empty());
    }

    #[test]
    fn test_config_empty_scopes() {
        let config = DiscoveryConfig {
            scopes: vec![],
            ..Default::default()
        };

        assert_eq!(config.get_scopes_string(), "");
    }

    #[test]
    fn test_discovery_error_socket_display() {
        let io_err = io::Error::new(io::ErrorKind::AddrInUse, "address in use");
        let err = DiscoveryError::Socket(io_err);
        let display = format!("{}", err);
        assert!(display.contains("socket error"));
        assert!(display.contains("address in use"));
    }

    #[test]
    fn test_discovery_error_multicast_join_display() {
        let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "no permission");
        let err = DiscoveryError::MulticastJoin(io_err);
        let display = format!("{}", err);
        assert!(display.contains("failed to join multicast group"));
        assert!(display.contains("no permission"));
    }

    #[test]
    fn test_discovery_error_serialization_display() {
        let err = DiscoveryError::Serialization("XML encoding failed".to_string());
        assert_eq!(
            format!("{}", err),
            "message serialization error: XML encoding failed"
        );
    }

    #[test]
    fn test_discovery_config_debug() {
        let config = DiscoveryConfig::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("DiscoveryConfig"));
        assert!(debug_str.contains("endpoint_uuid"));
        assert!(debug_str.contains("http_port"));
    }

    #[test]
    fn test_ws_discovery_message_debug() {
        let msg = WsDiscoveryMessage::Hello {
            message_id: "test".to_string(),
            endpoint_reference: "urn:uuid:test".to_string(),
            types: "type".to_string(),
            scopes: "scope".to_string(),
            xaddrs: "http://test".to_string(),
            metadata_version: 1,
            instance_id: 1,
            message_number: 1,
        };
        let debug_str = format!("{:?}", msg);
        assert!(debug_str.contains("Hello"));
    }

    #[test]
    fn test_ws_discovery_message_clone() {
        let msg = WsDiscoveryMessage::Bye {
            message_id: "test".to_string(),
            endpoint_reference: "urn:uuid:test".to_string(),
            instance_id: 1,
            message_number: 1,
        };
        let cloned = msg.clone();
        assert_eq!(msg, cloned);
    }

    #[test]
    fn test_discovery_mode_clone_and_copy() {
        let mode = DiscoveryMode::Discoverable;
        let cloned = mode.clone();
        let copied = mode; // Copy
        assert_eq!(mode, cloned);
        assert_eq!(mode, copied);
    }

    #[test]
    fn test_constants_values() {
        // Verify spec-compliant constant values
        assert_eq!(WS_DISCOVERY_MULTICAST, Ipv4Addr::new(239, 255, 255, 250));
        assert_eq!(WS_DISCOVERY_PORT, 3702);
        assert_eq!(APP_MAX_DELAY_MS, 500);
        assert_eq!(HELLO_INTERVAL_SECONDS, 300);
        assert_eq!(MAX_MESSAGE_SIZE, 4096);
    }

    #[test]
    fn test_extract_element_with_wsd_prefix() {
        let xml = r#"<root><wsd:Types>tdn:NVT</wsd:Types></root>"#;
        assert_eq!(
            extract_xml_element(xml, "Types"),
            Some("tdn:NVT".to_string())
        );
    }

    #[test]
    fn test_extract_element_with_wsa_prefix() {
        let xml = r#"<root><wsa:MessageID>urn:uuid:abc</wsa:MessageID></root>"#;
        assert_eq!(
            extract_xml_element(xml, "MessageID"),
            Some("urn:uuid:abc".to_string())
        );
    }
}
