//! SOAP data types and constants.
//!
//! This module provides the core data structures for working with SOAP 1.2 envelopes
//! as required by ONVIF, including:
//!
//! - Namespace constants for SOAP, WS-Security, and ONVIF services
//! - Type definitions for SOAP envelope, header, and security elements
//! - Error types for SOAP parsing failures

use serde::{Deserialize, Serialize};

/// SOAP 1.2 namespace URI.
pub const SOAP_ENVELOPE_NS: &str = "http://www.w3.org/2003/05/soap-envelope";

/// WS-Security namespace URI.
pub const WSSE_NS: &str =
    "http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-secext-1.0.xsd";

/// WS-Security Utility namespace URI.
pub const WSU_NS: &str =
    "http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-utility-1.0.xsd";

/// ONVIF Device Service namespace URI.
pub const TDS_NS: &str = "http://www.onvif.org/ver10/device/wsdl";

/// ONVIF Media Service namespace URI.
pub const TRT_NS: &str = "http://www.onvif.org/ver10/media/wsdl";

/// ONVIF PTZ Service namespace URI.
pub const PTZ_NS: &str = "http://www.onvif.org/ver20/ptz/wsdl";

/// ONVIF Imaging Service namespace URI.
pub const IMG_NS: &str = "http://www.onvif.org/ver20/imaging/wsdl";

/// ONVIF Schema namespace URI.
pub const TT_NS: &str = "http://www.onvif.org/ver10/schema";

/// Generic SOAP Envelope structure.
///
/// The envelope wraps the header (optional) and body of a SOAP message.
///
/// # Type Parameter
///
/// * `T` - The type of the body content
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename = "Envelope")]
pub struct SoapEnvelope<T> {
    /// Optional SOAP header containing security and addressing info.
    #[serde(rename = "Header", default, skip_serializing_if = "Option::is_none")]
    pub header: Option<SoapHeader>,

    /// SOAP body containing the actual message.
    #[serde(rename = "Body")]
    pub body: SoapBody<T>,
}

impl<T> SoapEnvelope<T> {
    /// Create a new SOAP envelope with a body and optional header.
    pub fn new(body: T, header: Option<SoapHeader>) -> Self {
        Self {
            header,
            body: SoapBody { content: body },
        }
    }

    /// Create a new SOAP envelope with just a body (no header).
    pub fn with_body(body: T) -> Self {
        Self::new(body, None)
    }
}

/// SOAP Body wrapper.
///
/// The body contains the actual message content, which can be any serializable type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoapBody<T> {
    /// The body content, flattened during serialization.
    #[serde(flatten)]
    pub content: T,
}

/// SOAP Header structure.
///
/// Contains optional WS-Security information and other SOAP headers.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SoapHeader {
    /// WS-Security element for authentication.
    #[serde(rename = "Security", default, skip_serializing_if = "Option::is_none")]
    pub security: Option<WsSecurity>,

    /// Optional message ID for WS-Addressing.
    #[serde(rename = "MessageID", default, skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,

    /// Optional action for WS-Addressing.
    #[serde(rename = "Action", default, skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
}

/// WS-Security element containing UsernameToken.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsSecurity {
    /// Username token for authentication.
    #[serde(
        rename = "UsernameToken",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub username_token: Option<UsernameToken>,
}

/// WS-Security UsernameToken element.
///
/// Contains credentials for ONVIF authentication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsernameToken {
    /// Username.
    #[serde(rename = "Username")]
    pub username: String,

    /// Password (digest or plain text).
    #[serde(rename = "Password")]
    pub password: PasswordElement,

    /// Nonce for digest authentication.
    #[serde(rename = "Nonce", default, skip_serializing_if = "Option::is_none")]
    pub nonce: Option<NonceElement>,

    /// Created timestamp for digest authentication.
    #[serde(rename = "Created", default, skip_serializing_if = "Option::is_none")]
    pub created: Option<String>,
}

/// Password element with type attribute.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordElement {
    /// Password type (Digest or Text).
    #[serde(rename = "@Type", default)]
    pub password_type: Option<String>,

    /// Password value.
    #[serde(rename = "$text")]
    pub value: String,
}

/// Nonce element with encoding attribute.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NonceElement {
    /// Encoding type (usually Base64Binary).
    #[serde(rename = "@EncodingType", default)]
    pub encoding_type: Option<String>,

    /// Nonce value.
    #[serde(rename = "$text")]
    pub value: String,
}

/// Raw SOAP envelope for initial parsing.
///
/// Used to extract the body content as raw XML for further processing.
#[derive(Debug, Clone)]
pub struct RawSoapEnvelope {
    /// Optional SOAP header.
    pub header: Option<SoapHeader>,
    /// Raw body content as XML string.
    pub body_xml: String,
    /// SOAP action extracted from headers or body.
    pub action: Option<String>,
}

/// Error type for SOAP parsing failures.
#[derive(Debug, Clone)]
pub enum SoapParseError {
    /// XML parsing failed.
    XmlError(String),
    /// Missing SOAP envelope element.
    MissingEnvelope,
    /// Missing SOAP body element.
    MissingBody,
    /// Invalid SOAP structure.
    InvalidStructure(String),
}

impl std::fmt::Display for SoapParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SoapParseError::XmlError(e) => write!(f, "XML parse error: {}", e),
            SoapParseError::MissingEnvelope => write!(f, "Missing SOAP Envelope element"),
            SoapParseError::MissingBody => write!(f, "Missing SOAP Body element"),
            SoapParseError::InvalidStructure(e) => write!(f, "Invalid SOAP structure: {}", e),
        }
    }
}

impl std::error::Error for SoapParseError {}
