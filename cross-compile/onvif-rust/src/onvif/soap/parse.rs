//! SOAP request parsing logic.
//!
//! This module provides the functions for parsing incoming SOAP requests from XML,
//! including envelope validation, header extraction, and body content capture.

use quick_xml::Reader;
use quick_xml::events::Event;

use crate::onvif::soap::model::{
    NonceElement, PasswordElement, RawSoapEnvelope, SOAP_ENVELOPE_NS, SoapHeader, SoapParseError,
    UsernameToken, WSSE_NS, WsSecurity,
};

/// Helper struct for collecting WS-Security data during parsing.
#[derive(Default)]
pub(crate) struct WsSecurityParseData {
    pub(crate) username: Option<String>,
    pub(crate) password: Option<String>,
    pub(crate) password_type: Option<String>,
    pub(crate) nonce: Option<String>,
    pub(crate) nonce_encoding: Option<String>,
    pub(crate) created: Option<String>,
}

/// Parsing state for SOAP request parsing.
pub(crate) struct SoapParseState {
    pub(crate) header: Option<SoapHeader>,
    pub(crate) body_xml: String,
    pub(crate) in_body: bool,
    pub(crate) body_depth: u32,
    pub(crate) action: Option<String>,
    pub(crate) in_header: bool,
    pub(crate) in_security: bool,
    pub(crate) in_username_token: bool,
    pub(crate) current_element: Option<String>,
    pub(crate) security_data: WsSecurityParseData,
    pub(crate) envelope_namespace: Option<String>,
    pub(crate) security_namespace: Option<String>,
    pub(crate) envelope_seen: bool,
    pub(crate) body_seen: bool,
    pub(crate) elements_before_envelope: u32,
}

impl SoapParseState {
    pub(crate) fn new() -> Self {
        Self {
            header: None,
            body_xml: String::new(),
            in_body: false,
            body_depth: 0,
            action: None,
            in_header: false,
            in_security: false,
            in_username_token: false,
            current_element: None,
            security_data: WsSecurityParseData::default(),
            envelope_namespace: None,
            security_namespace: None,
            envelope_seen: false,
            body_seen: false,
            elements_before_envelope: 0,
        }
    }
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
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut state = SoapParseState::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let name = String::from_utf8_lossy(e.local_name().as_ref()).to_string();
                handle_start_event(&mut state, &name, &e);
            }
            Ok(Event::End(e)) => {
                let name = String::from_utf8_lossy(e.local_name().as_ref()).to_string();
                handle_end_event(&mut state, &name);
            }
            Ok(Event::Empty(e)) => {
                let name = String::from_utf8_lossy(e.local_name().as_ref()).to_string();
                handle_empty_event(&mut state, &name, &e);
            }
            Ok(Event::Text(e)) => {
                handle_text_event(&mut state, &e);
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

    // Validate that Envelope exists (check this first for clearer error messages)
    if !state.envelope_seen {
        return Err(SoapParseError::MissingEnvelope);
    }

    // Validate that Envelope is the root element
    if state.elements_before_envelope > 0 {
        return Err(SoapParseError::InvalidStructure(
            "SOAP Envelope must be the root element".to_string(),
        ));
    }

    // Validate SOAP envelope namespace
    if let Some(ref ns) = state.envelope_namespace {
        if ns != SOAP_ENVELOPE_NS {
            return Err(SoapParseError::InvalidStructure(format!(
                "Invalid SOAP envelope namespace: expected '{}', got '{}'",
                SOAP_ENVELOPE_NS, ns
            )));
        }
    } else {
        // Envelope was seen but no namespace was found - this is also invalid
        return Err(SoapParseError::InvalidStructure(
            "SOAP envelope missing namespace declaration".to_string(),
        ));
    }

    // Validate that Body exists within Envelope
    if !state.body_seen || state.body_xml.is_empty() {
        return Err(SoapParseError::MissingBody);
    }

    // Validate WS-Security namespace if Security header is present
    let has_security = state.in_security
        || state
            .header
            .as_ref()
            .and_then(|h| h.security.as_ref())
            .is_some();

    if has_security
        && let Some(ref ns) = state.security_namespace
        && ns != WSSE_NS
    {
        return Err(SoapParseError::InvalidStructure(format!(
            "Invalid WS-Security namespace: expected '{}', got '{}'",
            WSSE_NS, ns
        )));
    }

    Ok(RawSoapEnvelope {
        header: state.header,
        body_xml: state.body_xml,
        action: state.action,
    })
}

/// Handle a Start event during SOAP parsing.
pub(crate) fn handle_start_event(
    state: &mut SoapParseState,
    name: &str,
    e: &quick_xml::events::BytesStart,
) {
    track_elements_before_envelope(state, name);

    if handle_envelope_start(state, name, e) {
        return;
    }
    if handle_body_start(state, name) {
        return;
    }
    if handle_header_start(state, name) {
        return;
    }
    if handle_security_start(state, name, e) {
        return;
    }
    if handle_username_token_start(state, name) {
        return;
    }
    if handle_security_element_start(state, name, e) {
        return;
    }
    if state.in_body {
        append_body_start_tag(state, name, e);
    }
}

/// Track elements before Envelope to ensure Envelope is the root.
pub(crate) fn track_elements_before_envelope(state: &mut SoapParseState, name: &str) {
    if state.envelope_seen || name == "Envelope" {
        return;
    }
    // Ignore XML declaration and processing instructions
    if name != "?xml" && !name.starts_with('?') {
        state.elements_before_envelope += 1;
    }
}

/// Handle Envelope element start. Returns true if handled.
pub(crate) fn handle_envelope_start(
    state: &mut SoapParseState,
    name: &str,
    e: &quick_xml::events::BytesStart,
) -> bool {
    if name != "Envelope" || state.envelope_seen {
        return false;
    }

    state.envelope_seen = true;
    extract_envelope_namespace(state, e);
    true
}

/// Extract namespace from Envelope attributes.
/// Matches any xmlns prefix whose value equals the SOAP envelope namespace URI,
/// making the parser spec-compliant (XML namespaces are identified by URI, not prefix).
pub(crate) fn extract_envelope_namespace(
    state: &mut SoapParseState,
    e: &quick_xml::events::BytesStart,
) {
    for attr in e.attributes().flatten() {
        let key = String::from_utf8_lossy(attr.key.as_ref());
        // Check key before allocating value to avoid unnecessary String allocation
        if !(key.starts_with("xmlns:") || key == "xmlns") {
            continue;
        }
        let value = String::from_utf8_lossy(&attr.value).to_string();
        // Only capture namespace values that match SOAP envelope namespace pattern.
        // This prevents capturing non-SOAP namespaces that appear first in attribute order.
        if value.contains("soap-envelope") {
            state.envelope_namespace = Some(value);
            break;
        }
    }
}

/// Handle Body element start. Returns true if handled.
pub(crate) fn handle_body_start(state: &mut SoapParseState, name: &str) -> bool {
    if name != "Body" || state.in_body {
        return false;
    }

    if !state.envelope_seen {
        // Body found before Envelope - invalid structure
        return true;
    }

    state.in_body = true;
    state.body_seen = true;
    state.in_header = false;
    state.body_depth = 0;
    true
}

/// Handle Header element start. Returns true if handled.
pub(crate) fn handle_header_start(state: &mut SoapParseState, name: &str) -> bool {
    if name != "Header" || state.in_body {
        return false;
    }

    state.in_header = true;
    state.header = Some(SoapHeader::default());
    true
}

/// Handle Security element start. Returns true if handled.
pub(crate) fn handle_security_start(
    state: &mut SoapParseState,
    name: &str,
    e: &quick_xml::events::BytesStart,
) -> bool {
    if name != "Security" || !state.in_header {
        return false;
    }

    state.in_security = true;
    extract_security_namespace(state, e);
    true
}

/// Extract WS-Security namespace from Security element attributes.
pub(crate) fn extract_security_namespace(
    state: &mut SoapParseState,
    e: &quick_xml::events::BytesStart,
) {
    for attr in e.attributes().flatten() {
        let key = String::from_utf8_lossy(attr.key.as_ref());
        if key.starts_with("xmlns") {
            let value = String::from_utf8_lossy(&attr.value).to_string();
            if value.contains("wss-wssecurity-secext") {
                state.security_namespace = Some(value);
                break;
            }
        }
    }
}

/// Handle UsernameToken element start. Returns true if handled.
pub(crate) fn handle_username_token_start(state: &mut SoapParseState, name: &str) -> bool {
    if name != "UsernameToken" || !state.in_security {
        return false;
    }

    state.in_username_token = true;
    true
}

/// Handle security element start (within UsernameToken). Returns true if handled.
pub(crate) fn handle_security_element_start(
    state: &mut SoapParseState,
    name: &str,
    e: &quick_xml::events::BytesStart,
) -> bool {
    if !state.in_username_token {
        return false;
    }

    state.current_element = Some(name.to_string());
    handle_security_attributes(state, name, e);
    true
}

/// Handle security-related attributes during parsing.
pub(crate) fn handle_security_attributes(
    state: &mut SoapParseState,
    name: &str,
    e: &quick_xml::events::BytesStart,
) {
    if name == "Password" {
        for attr in e.attributes().flatten() {
            let key = String::from_utf8_lossy(attr.key.local_name().as_ref()).to_string();
            if key == "Type" {
                state.security_data.password_type =
                    Some(String::from_utf8_lossy(&attr.value).to_string());
            }
        }
    } else if name == "Nonce" {
        for attr in e.attributes().flatten() {
            let key = String::from_utf8_lossy(attr.key.local_name().as_ref()).to_string();
            if key == "EncodingType" {
                state.security_data.nonce_encoding =
                    Some(String::from_utf8_lossy(&attr.value).to_string());
            }
        }
    }
}

/// Append a body start tag to the body XML.
pub(crate) fn append_body_start_tag(
    state: &mut SoapParseState,
    name: &str,
    e: &quick_xml::events::BytesStart,
) {
    state.body_depth += 1;
    state.body_xml.push('<');
    state.body_xml.push_str(name);

    for attr in e.attributes().flatten() {
        // Preserve the full qualified name (prefix:local) for attributes
        let key = String::from_utf8_lossy(attr.key.as_ref());
        let value = String::from_utf8_lossy(&attr.value);
        state.body_xml.push(' ');
        state.body_xml.push_str(&key);
        state.body_xml.push_str("=\"");
        state.body_xml.push_str(&value);
        state.body_xml.push('"');
    }
    state.body_xml.push('>');

    if state.body_depth == 1 && state.action.is_none() {
        state.action = Some(name.to_string());
    }
}

/// Handle an End event during SOAP parsing.
pub(crate) fn handle_end_event(state: &mut SoapParseState, name: &str) {
    if name == "Body" && state.in_body && state.body_depth == 0 {
        state.in_body = false;
    } else if name == "Header" && state.in_header {
        state.in_header = false;
    } else if name == "Security" && state.in_security {
        state.in_security = false;
    } else if name == "UsernameToken" && state.in_username_token {
        state.in_username_token = false;
        build_username_token(state);
    } else if state.in_username_token {
        state.current_element = None;
    } else if state.in_body {
        append_body_end_tag(state, name);
    }
}

/// Build the UsernameToken from collected security data.
pub(crate) fn build_username_token(state: &mut SoapParseState) {
    if let Some(ref mut h) = state.header {
        h.security = Some(WsSecurity {
            username_token: Some(UsernameToken {
                username: state.security_data.username.take().unwrap_or_default(),
                password: PasswordElement {
                    password_type: state.security_data.password_type.take(),
                    value: state.security_data.password.take().unwrap_or_default(),
                },
                nonce: state.security_data.nonce.take().map(|v| NonceElement {
                    encoding_type: state.security_data.nonce_encoding.take(),
                    value: v,
                }),
                created: state.security_data.created.take(),
            }),
        });
    }
}

/// Append a body end tag to the body XML.
pub(crate) fn append_body_end_tag(state: &mut SoapParseState, name: &str) {
    state.body_xml.push_str("</");
    state.body_xml.push_str(name);
    state.body_xml.push('>');
    state.body_depth = state.body_depth.saturating_sub(1);
}

/// Handle an Empty event during SOAP parsing.
pub(crate) fn handle_empty_event(
    state: &mut SoapParseState,
    name: &str,
    e: &quick_xml::events::BytesStart,
) {
    if !state.in_body {
        return;
    }

    state.body_xml.push('<');
    state.body_xml.push_str(name);

    for attr in e.attributes().flatten() {
        // Preserve the full qualified name (prefix:local) for attributes
        let key = String::from_utf8_lossy(attr.key.as_ref());
        let value = String::from_utf8_lossy(&attr.value);
        state.body_xml.push(' ');
        state.body_xml.push_str(&key);
        state.body_xml.push_str("=\"");
        state.body_xml.push_str(&value);
        state.body_xml.push('"');
    }
    state.body_xml.push_str("/>");

    if state.body_depth == 0 && state.action.is_none() {
        state.action = Some(name.to_string());
    }
}

/// Handle a Text event during SOAP parsing.
pub(crate) fn handle_text_event(state: &mut SoapParseState, e: &quick_xml::events::BytesText) {
    if state.in_body {
        let text = e.xml_content().unwrap_or_default();
        state.body_xml.push_str(&text);
    } else if state.in_username_token {
        let text = e.xml_content().unwrap_or_default().to_string();
        if let Some(ref elem) = state.current_element {
            match elem.as_str() {
                "Username" => state.security_data.username = Some(text),
                "Password" => state.security_data.password = Some(text),
                "Nonce" => state.security_data.nonce = Some(text),
                "Created" => state.security_data.created = Some(text),
                _ => {}
            }
        }
    }
}
