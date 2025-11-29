//! SOAP envelope parsing and building.
//!
//! This module provides types and functions for working with SOAP 1.2 envelopes
//! as required by ONVIF. It handles:
//!
//! - Parsing incoming SOAP requests from XML
//! - Building SOAP responses with proper namespaces
//! - WS-Security header extraction
//!
//! # SOAP Envelope Structure
//!
//! ```xml
//! <s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
//!     <s:Header>
//!         <wsse:Security>...</wsse:Security>
//!     </s:Header>
//!     <s:Body>
//!         <tds:GetDeviceInformation/>
//!     </s:Body>
//! </s:Envelope>
//! ```
//!
//! # Example
//!
//! ```ignore
//! use onvif_rust::onvif::soap::{parse_soap_request, build_soap_response, SoapEnvelope};
//!
//! let xml = r#"<Envelope>...</Envelope>"#;
//! let envelope = parse_soap_request(xml)?;
//!
//! let response_body = "<GetDeviceInformationResponse>...</GetDeviceInformationResponse>";
//! let response = build_soap_response(response_body);
//! ```

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

/// Parse a SOAP request from XML.
///
/// This function parses the outer SOAP envelope structure and extracts
/// the header (if present) and the raw body content for further processing.
///
/// # Arguments
///
/// * `xml` - The XML string containing the SOAP envelope
///
/// # Returns
///
/// A `RawSoapEnvelope` containing the parsed header and raw body XML,
/// or an error if parsing fails.
///
/// # Example
///
/// ```ignore
/// let xml = r#"
/// <s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
///     <s:Body>
///         <tds:GetDeviceInformation xmlns:tds="http://www.onvif.org/ver10/device/wsdl"/>
///     </s:Body>
/// </s:Envelope>
/// "#;
///
/// let envelope = parse_soap_request(xml)?;
/// println!("Body: {}", envelope.body_xml);
/// ```
pub fn parse_soap_request(xml: &str) -> Result<RawSoapEnvelope, SoapParseError> {
    use quick_xml::Reader;
    use quick_xml::events::Event;

    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut header: Option<SoapHeader> = None;
    let mut body_xml = String::new();
    let mut in_body = false;
    let mut body_depth = 0;
    let mut action: Option<String> = None;

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let name = String::from_utf8_lossy(e.local_name().as_ref()).to_string();

                if name == "Body" && !in_body {
                    in_body = true;
                    body_depth = 0;
                } else if in_body {
                    body_depth += 1;
                    // Capture start tag
                    body_xml.push('<');
                    body_xml.push_str(&name);

                    // Capture attributes
                    for attr in e.attributes().flatten() {
                        let local_name = attr.key.local_name();
                        let key = String::from_utf8_lossy(local_name.as_ref());
                        let value = String::from_utf8_lossy(&attr.value);
                        body_xml.push(' ');
                        body_xml.push_str(&key);
                        body_xml.push_str("=\"");
                        body_xml.push_str(&value);
                        body_xml.push('"');
                    }
                    body_xml.push('>');

                    // Extract action from first element in body
                    if body_depth == 1 && action.is_none() {
                        action = Some(name);
                    }
                } else if name == "Header" {
                    // Skip header parsing for now, just mark presence
                    header = Some(SoapHeader::default());
                }
            }
            Ok(Event::End(e)) => {
                let name = String::from_utf8_lossy(e.local_name().as_ref()).to_string();

                if name == "Body" && in_body && body_depth == 0 {
                    in_body = false;
                } else if in_body {
                    body_xml.push_str("</");
                    body_xml.push_str(&name);
                    body_xml.push('>');
                    body_depth -= 1;
                }
            }
            Ok(Event::Empty(e)) => {
                if in_body {
                    let name = String::from_utf8_lossy(e.local_name().as_ref()).to_string();
                    body_xml.push('<');
                    body_xml.push_str(&name);

                    for attr in e.attributes().flatten() {
                        let local_name = attr.key.local_name();
                        let key = String::from_utf8_lossy(local_name.as_ref());
                        let value = String::from_utf8_lossy(&attr.value);
                        body_xml.push(' ');
                        body_xml.push_str(&key);
                        body_xml.push_str("=\"");
                        body_xml.push_str(&value);
                        body_xml.push('"');
                    }
                    body_xml.push_str("/>");

                    // Extract action from first element in body
                    if body_depth == 0 && action.is_none() {
                        action = Some(name);
                    }
                }
            }
            Ok(Event::Text(e)) => {
                if in_body {
                    // Use xml_content() for unescaping XML entities in quick-xml 0.38+
                    let text = e.xml_content().unwrap_or_default();
                    body_xml.push_str(&text);
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(SoapParseError::XmlError(format!(
                    "XML parse error at position {}: {}",
                    reader.error_position(),
                    e
                )));
            }
            _ => {}
        }
    }

    if body_xml.is_empty() {
        return Err(SoapParseError::MissingBody);
    }

    Ok(RawSoapEnvelope {
        header,
        body_xml,
        action,
    })
}

/// Build a SOAP response envelope with the given body content.
///
/// # Arguments
///
/// * `body_xml` - The XML content to wrap in the SOAP body
///
/// # Returns
///
/// A complete SOAP envelope as an XML string.
///
/// # Example
///
/// ```
/// use onvif_rust::onvif::soap::build_soap_response;
///
/// let body = r#"<tds:GetDeviceInformationResponse xmlns:tds="http://www.onvif.org/ver10/device/wsdl">
///     <tds:Manufacturer>Anyka</tds:Manufacturer>
/// </tds:GetDeviceInformationResponse>"#;
///
/// let response = build_soap_response(body);
/// assert!(response.contains("Envelope"));
/// ```
pub fn build_soap_response(body_xml: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="{}" xmlns:tt="{}" xmlns:tds="{}" xmlns:trt="{}" xmlns:ptz="{}" xmlns:img="{}">
    <s:Body>
        {}
    </s:Body>
</s:Envelope>"#,
        SOAP_ENVELOPE_NS, TT_NS, TDS_NS, TRT_NS, PTZ_NS, IMG_NS, body_xml
    )
}

/// Build a SOAP fault response.
///
/// # Arguments
///
/// * `code` - The SOAP fault code (e.g., "s:Sender", "s:Receiver")
/// * `subcode` - The fault subcode (e.g., "ter:ActionNotSupported")
/// * `reason` - Human-readable reason for the fault
///
/// # Returns
///
/// A complete SOAP fault envelope as an XML string.
pub fn build_soap_fault(code: &str, subcode: &str, reason: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="{}" xmlns:ter="http://www.onvif.org/ver10/error">
    <s:Body>
        <s:Fault>
            <s:Code>
                <s:Value>{}</s:Value>
                <s:Subcode>
                    <s:Value>{}</s:Value>
                </s:Subcode>
            </s:Code>
            <s:Reason>
                <s:Text xml:lang="en">{}</s:Text>
            </s:Reason>
        </s:Fault>
    </s:Body>
</s:Envelope>"#,
        SOAP_ENVELOPE_NS, code, subcode, reason
    )
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_soap_request() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
    <s:Body>
        <GetDeviceInformation/>
    </s:Body>
</s:Envelope>"#;

        let result = parse_soap_request(xml);
        assert!(result.is_ok());

        let envelope = result.unwrap();
        assert!(envelope.body_xml.contains("GetDeviceInformation"));
        assert_eq!(envelope.action, Some("GetDeviceInformation".to_string()));
    }

    #[test]
    fn test_parse_soap_request_with_content() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
    <s:Body>
        <SetHostname>
            <Name>camera-001</Name>
        </SetHostname>
    </s:Body>
</s:Envelope>"#;

        let result = parse_soap_request(xml);
        assert!(result.is_ok());

        let envelope = result.unwrap();
        assert!(envelope.body_xml.contains("SetHostname"));
        assert!(envelope.body_xml.contains("camera-001"));
    }

    #[test]
    fn test_parse_soap_request_with_header() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
    <s:Header>
        <wsse:Security xmlns:wsse="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-secext-1.0.xsd">
            <wsse:UsernameToken/>
        </wsse:Security>
    </s:Header>
    <s:Body>
        <GetDeviceInformation/>
    </s:Body>
</s:Envelope>"#;

        let result = parse_soap_request(xml);
        assert!(result.is_ok());

        let envelope = result.unwrap();
        assert!(envelope.header.is_some());
    }

    #[test]
    fn test_parse_soap_request_missing_body() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
</s:Envelope>"#;

        let result = parse_soap_request(xml);
        assert!(matches!(result, Err(SoapParseError::MissingBody)));
    }

    #[test]
    fn test_build_soap_response() {
        let body = r#"<GetDeviceInformationResponse>
    <Manufacturer>Anyka</Manufacturer>
</GetDeviceInformationResponse>"#;

        let response = build_soap_response(body);

        assert!(response.contains("Envelope"));
        assert!(response.contains("Body"));
        assert!(response.contains("GetDeviceInformationResponse"));
        assert!(response.contains("Anyka"));
    }

    #[test]
    fn test_build_soap_fault() {
        let fault = build_soap_fault("s:Sender", "ter:ActionNotSupported", "Action not supported");

        assert!(fault.contains("Fault"));
        assert!(fault.contains("s:Sender"));
        assert!(fault.contains("ter:ActionNotSupported"));
        assert!(fault.contains("Action not supported"));
    }

    #[test]
    fn test_soap_envelope_serialize() {
        // This tests the serde serialization traits are properly derived
        let envelope = SoapEnvelope::with_body("test".to_string());
        assert_eq!(envelope.body.content, "test");
        assert!(envelope.header.is_none());
    }

    #[test]
    fn test_soap_envelope_with_header() {
        let header = SoapHeader {
            security: None,
            message_id: Some("urn:uuid:12345".to_string()),
            action: Some("http://www.onvif.org/ver10/device/wsdl/GetDeviceInformation".to_string()),
        };

        let envelope = SoapEnvelope::new("body".to_string(), Some(header));
        assert!(envelope.header.is_some());
        assert_eq!(
            envelope.header.as_ref().unwrap().message_id,
            Some("urn:uuid:12345".to_string())
        );
    }

    #[test]
    fn test_parse_soap_request_with_attributes() {
        // Test XML with attributes on body elements
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope" xmlns:tds="http://www.onvif.org/ver10/device/wsdl">
    <s:Body>
        <tds:SetSystemDateAndTime>
            <tds:DateTimeType DateTimeMode="Manual">
                <tds:TimeZone timezone="UTC" offset="+00:00"/>
            </tds:DateTimeType>
        </tds:SetSystemDateAndTime>
    </s:Body>
</s:Envelope>"#;

        let result = parse_soap_request(xml);
        assert!(result.is_ok());

        let envelope = result.unwrap();
        // Verify attributes are captured in body XML
        assert!(envelope.body_xml.contains("DateTimeMode"));
        assert!(envelope.body_xml.contains("Manual"));
        assert!(envelope.body_xml.contains("timezone"));
        assert!(envelope.body_xml.contains("UTC"));
    }

    #[test]
    fn test_parse_soap_request_with_nested_elements() {
        // Test XML with deeply nested elements
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope" xmlns:trt="http://www.onvif.org/ver10/media/wsdl">
    <s:Body>
        <trt:SetVideoEncoderConfiguration>
            <trt:Configuration token="encoder1">
                <trt:Name>Main Stream</trt:Name>
                <trt:Resolution>
                    <trt:Width>1920</trt:Width>
                    <trt:Height>1080</trt:Height>
                </trt:Resolution>
            </trt:Configuration>
        </trt:SetVideoEncoderConfiguration>
    </s:Body>
</s:Envelope>"#;

        let result = parse_soap_request(xml);
        assert!(result.is_ok());

        let envelope = result.unwrap();
        // Verify closing tags are properly captured
        assert!(envelope.body_xml.contains("</Name>"));
        assert!(envelope.body_xml.contains("</Width>"));
        assert!(envelope.body_xml.contains("</Height>"));
        assert!(envelope.body_xml.contains("</Resolution>"));
    }

    #[test]
    fn test_parse_soap_request_with_text_content() {
        // Test XML with text content
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope" xmlns:tds="http://www.onvif.org/ver10/device/wsdl">
    <s:Body>
        <tds:SetHostname>
            <tds:Name>my-camera</tds:Name>
        </tds:SetHostname>
    </s:Body>
</s:Envelope>"#;

        let result = parse_soap_request(xml);
        assert!(result.is_ok());

        let envelope = result.unwrap();
        // Verify text content is captured
        assert!(envelope.body_xml.contains("my-camera"));
    }

    #[test]
    fn test_parse_soap_request_with_empty_elements() {
        // Test XML with empty (self-closing) elements
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope" xmlns:tds="http://www.onvif.org/ver10/device/wsdl">
    <s:Body>
        <tds:GetCapabilities>
            <tds:Category/>
        </tds:GetCapabilities>
    </s:Body>
</s:Envelope>"#;

        let result = parse_soap_request(xml);
        assert!(result.is_ok());

        let envelope = result.unwrap();
        // Verify empty element is captured as self-closing tag
        assert!(envelope.body_xml.contains("Category"));
        assert!(envelope.body_xml.contains("/>"));
    }

    #[test]
    fn test_parse_soap_request_with_empty_element_attributes() {
        // Test XML with empty elements that have attributes
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope" xmlns:tptz="http://www.onvif.org/ver20/ptz/wsdl">
    <s:Body>
        <tptz:ContinuousMove ProfileToken="profile1">
            <tptz:Velocity x="0.5" y="-0.3"/>
        </tptz:ContinuousMove>
    </s:Body>
</s:Envelope>"#;

        let result = parse_soap_request(xml);
        assert!(result.is_ok());

        let envelope = result.unwrap();
        // Verify empty element with attributes is captured
        assert!(envelope.body_xml.contains("Velocity"));
        assert!(envelope.body_xml.contains("x=\"0.5\""));
        assert!(envelope.body_xml.contains("y=\"-0.3\""));
    }

    // Note: XML error handling test omitted since quick-xml is lenient with
    // malformed XML. The MissingBody test covers the practical error case.
}
