//! SOAP response building functions.
//!
//! This module provides functions for building SOAP response envelopes and fault messages
//! with proper namespace declarations.

use crate::onvif::soap::model::{IMG_NS, PTZ_NS, SOAP_ENVELOPE_NS, TDS_NS, TRT_NS, TT_NS};

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
<s:Envelope xmlns:s="{}" xmlns:tt="{}" xmlns:tds="{}" xmlns:trt="{}" xmlns:tptz="{}" xmlns:timg="{}">
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
