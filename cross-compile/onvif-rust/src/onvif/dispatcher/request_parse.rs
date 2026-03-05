//! Request parsing module for the dispatcher.
//!
//! This module provides functions for extracting SOAP actions from requests
//! and parsing SOAP request bodies.

use axum::{body::Body, extract::Request, response::Response};
use serde::de::DeserializeOwned;

use super::response::error_response;
use crate::utils::validation::{SecurityError, SecurityValidator};

/// Parse XML body with security validation and deserialization.
pub fn parse_body<T: DeserializeOwned>(
    body_xml: &str,
) -> Result<T, crate::onvif::error::OnvifError> {
    // Validate XML security before deserialization
    let security_validator = SecurityValidator::default();
    security_validator
        .check_xml_security(body_xml)
        .map_err(|e| match e {
            SecurityError::XxeDetected(msg) => crate::onvif::error::OnvifError::WellFormed(
                format!("XML security validation failed: {}", msg),
            ),
            SecurityError::XssDetected(msg) => crate::onvif::error::OnvifError::WellFormed(
                format!("XML security validation failed: {}", msg),
            ),
            SecurityError::XmlBombDetected => crate::onvif::error::OnvifError::WellFormed(
                "XML bomb detected: excessive entity declarations".to_string(),
            ),
            SecurityError::PayloadTooLarge(actual, max) => {
                crate::onvif::error::OnvifError::WellFormed(format!(
                    "Payload too large: {} bytes (max: {})",
                    actual, max
                ))
            }
            SecurityError::InvalidCharacters(msg) => crate::onvif::error::OnvifError::WellFormed(
                format!("Invalid characters in XML: {}", msg),
            ),
            SecurityError::PathTraversal(msg) => crate::onvif::error::OnvifError::WellFormed(
                format!("Path traversal detected: {}", msg),
            ),
        })?;

    // Deserialize XML
    quick_xml::de::from_str(body_xml).map_err(|e| {
        crate::onvif::error::OnvifError::WellFormed(format!("Invalid request XML: {}", e))
    })
}

/// Extract the SOAPAction from request headers.
///
/// The SOAPAction can be in the `SOAPAction` header or in the
/// `action` parameter of the `Content-Type` header.
pub(super) fn extract_soap_action(request: &Request<Body>) -> Option<String> {
    // Try SOAPAction header first
    if let Some(action) = extract_action_from_soap_header(request) {
        return Some(action);
    }

    // Try Content-Type header with action parameter
    extract_action_from_content_type(request)
}

/// Extract action from SOAPAction header.
fn extract_action_from_soap_header(request: &Request<Body>) -> Option<String> {
    let action_header = request.headers().get("SOAPAction")?;
    let action_str = action_header.to_str().ok()?;
    normalize_action(action_str)
}

/// Extract action from Content-Type header's action parameter.
fn extract_action_from_content_type(request: &Request<Body>) -> Option<String> {
    let content_type = request.headers().get(axum::http::header::CONTENT_TYPE)?;
    let ct_str = content_type.to_str().ok()?;

    // Look for action= parameter
    for part in ct_str.split(';') {
        let part = part.trim();
        if let Some(action) = part.strip_prefix("action=") {
            return normalize_action(action);
        }
    }

    None
}

/// Normalize action string by trimming quotes and extracting from URI if needed.
fn normalize_action(action: &str) -> Option<String> {
    let action = action.trim_matches('"');
    let action = extract_action_from_uri(action).unwrap_or_else(|| action.to_string());

    if action.is_empty() {
        None
    } else {
        Some(action)
    }
}

/// Extract action name from URI (last segment after '/').
fn extract_action_from_uri(uri: &str) -> Option<String> {
    uri.rfind('/').map(|pos| uri[pos + 1..].to_string())
}

/// Read request body and parse SOAP envelope.
pub(super) async fn read_and_parse_request(
    request: Request<Body>,
) -> Result<crate::onvif::soap::RawSoapEnvelope, Box<Response>> {
    // Read body
    let body_bytes = match axum::body::to_bytes(request.into_body(), 1024 * 1024).await {
        Ok(bytes) => bytes,
        Err(e) => {
            tracing::error!("Failed to read request body: {}", e);
            return Err(Box::new(error_response(
                crate::onvif::error::OnvifError::WellFormed(format!(
                    "Failed to read request body: {}",
                    e
                )),
            )));
        }
    };

    let body_str = match std::str::from_utf8(&body_bytes) {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("Invalid UTF-8 in request body: {}", e);
            return Err(Box::new(error_response(
                crate::onvif::error::OnvifError::WellFormed(format!(
                    "Invalid UTF-8 in request body: {}",
                    e
                )),
            )));
        }
    };

    // CRIT-002: Validate XML security before parsing to prevent XXE, XML bombs, etc.
    let security_validator = SecurityValidator::default();
    if let Err(e) = security_validator.check_xml_security(body_str) {
        tracing::warn!("XML security validation failed: {}", e);
        return Err(Box::new(error_response(
            crate::onvif::error::OnvifError::WellFormed(format!(
                "XML security validation failed: {}",
                e
            )),
        )));
    }

    // Parse SOAP envelope
    match crate::onvif::soap::parse_soap_request(body_str) {
        Ok(env) => Ok(env),
        Err(e) => {
            tracing::error!("Failed to parse SOAP envelope: {}", e);
            Err(Box::new(error_response(
                crate::onvif::error::OnvifError::WellFormed(format!(
                    "Failed to parse SOAP envelope: {}",
                    e
                )),
            )))
        }
    }
}

/// Extract SOAP action from header or envelope.
pub(super) fn extract_action(
    soap_action: Option<String>,
    envelope: &crate::onvif::soap::RawSoapEnvelope,
) -> Result<String, Box<Response>> {
    tracing::debug!(
        "Action extraction (with auth): soap_action_header={:?}, envelope_action={:?}",
        soap_action,
        envelope.action
    );

    let action = soap_action
        .clone()
        .or(envelope.action.clone())
        .unwrap_or_default();

    if action.is_empty() {
        tracing::warn!(
            "Missing SOAP action in request (header={:?}, body={:?})",
            soap_action,
            envelope.action
        );
        return Err(Box::new(error_response(
            crate::onvif::error::OnvifError::WellFormed(
                "Missing SOAP action in request".to_string(),
            ),
        )));
    }

    Ok(action)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Request as HttpRequest;

    #[test]
    fn test_extract_soap_action_from_header() {
        let request = HttpRequest::builder()
            .method("POST")
            .header(
                "SOAPAction",
                "\"http://www.onvif.org/ver10/device/wsdl/GetDeviceInformation\"",
            )
            .body(Body::empty())
            .unwrap();

        let action = extract_soap_action(&request);
        assert_eq!(action, Some("GetDeviceInformation".to_string()));
    }

    #[test]
    fn test_extract_soap_action_from_content_type() {
        let request = HttpRequest::builder()
            .method("POST")
            .header(
                "Content-Type",
                "application/soap+xml; charset=utf-8; action=\"GetDeviceInformation\"",
            )
            .body(Body::empty())
            .unwrap();

        let action = extract_soap_action(&request);
        assert_eq!(action, Some("GetDeviceInformation".to_string()));
    }

    #[test]
    fn test_extract_soap_action_none() {
        let request = HttpRequest::builder()
            .method("POST")
            .header("Content-Type", "text/xml")
            .body(Body::empty())
            .unwrap();

        let action = extract_soap_action(&request);
        assert!(action.is_none());
    }

    #[test]
    fn test_extract_soap_action_strips_quotes() {
        // Test with double quotes
        let request = HttpRequest::builder()
            .method("POST")
            .header("SOAPAction", "\"GetDeviceInformation\"")
            .body(Body::empty())
            .unwrap();

        let action = extract_soap_action(&request);
        assert_eq!(action, Some("GetDeviceInformation".to_string()));
    }

    #[test]
    fn test_extract_soap_action_extracts_last_segment() {
        // Full URI should extract just the action name
        let request = HttpRequest::builder()
            .method("POST")
            .header(
                "SOAPAction",
                "http://www.onvif.org/ver10/device/wsdl/GetCapabilities",
            )
            .body(Body::empty())
            .unwrap();

        let action = extract_soap_action(&request);
        assert_eq!(action, Some("GetCapabilities".to_string()));
    }
}
