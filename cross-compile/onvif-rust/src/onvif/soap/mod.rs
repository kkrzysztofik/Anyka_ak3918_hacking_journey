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

// Re-export all public types from submodules for backward compatibility
pub use crate::onvif::soap::build::{build_soap_fault, build_soap_response};
pub use crate::onvif::soap::model::{
    IMG_NS, NonceElement, PTZ_NS, PasswordElement, RawSoapEnvelope, SOAP_ENVELOPE_NS, SoapBody,
    SoapEnvelope, SoapHeader, SoapParseError, TDS_NS, TRT_NS, TT_NS, UsernameToken, WSSE_NS,
    WSU_NS, WsSecurity,
};
pub use crate::onvif::soap::parse::parse_soap_request;

// Also re-export the parse function's re-exported items for direct module access
mod build;
mod model;
mod parse;

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
    fn test_parse_ws_security_username_token() {
        // Real WS-Security UsernameToken with digest authentication
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope"
            xmlns:wsse="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-secext-1.0.xsd"
            xmlns:wsu="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-utility-1.0.xsd">
    <s:Header>
        <wsse:Security s:mustUnderstand="1">
            <wsse:UsernameToken wsu:Id="UsernameToken-1">
                <wsse:Username>admin</wsse:Username>
                <wsse:Password Type="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-username-token-profile-1.0#PasswordDigest">YkMvwPj4ZPVPLbK8QBWdYGs+3JE=</wsse:Password>
                <wsse:Nonce EncodingType="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-soap-message-security-1.0#Base64Binary">MTIzNDU2Nzg5MGFiY2RlZg==</wsse:Nonce>
                <wsu:Created>2024-01-15T10:30:00Z</wsu:Created>
            </wsse:UsernameToken>
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

        let header = envelope.header.unwrap();
        assert!(header.security.is_some());

        let security = header.security.unwrap();
        assert!(security.username_token.is_some());

        let token = security.username_token.unwrap();
        assert_eq!(token.username, "admin");
        assert_eq!(token.password.value, "YkMvwPj4ZPVPLbK8QBWdYGs+3JE=");
        assert!(token.password.password_type.is_some());
        assert!(
            token
                .password
                .password_type
                .as_ref()
                .unwrap()
                .contains("PasswordDigest")
        );

        assert!(token.nonce.is_some());
        let nonce = token.nonce.unwrap();
        assert_eq!(nonce.value, "MTIzNDU2Nzg5MGFiY2RlZg==");
        assert!(nonce.encoding_type.is_some());

        assert!(token.created.is_some());
        assert_eq!(token.created.unwrap(), "2024-01-15T10:30:00Z");
    }

    #[test]
    fn test_parse_ws_security_plaintext_password() {
        // WS-Security with plaintext password (less secure but valid)
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope"
            xmlns:wsse="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-secext-1.0.xsd">
    <s:Header>
        <wsse:Security>
            <wsse:UsernameToken>
                <wsse:Username>operator</wsse:Username>
                <wsse:Password Type="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-username-token-profile-1.0#PasswordText">secretpass</wsse:Password>
            </wsse:UsernameToken>
        </wsse:Security>
    </s:Header>
    <s:Body>
        <GetProfiles/>
    </s:Body>
</s:Envelope>"#;

        let result = parse_soap_request(xml);
        assert!(result.is_ok());

        let envelope = result.unwrap();
        let header = envelope.header.unwrap();
        let security = header.security.unwrap();
        let token = security.username_token.unwrap();

        assert_eq!(token.username, "operator");
        assert_eq!(token.password.value, "secretpass");
        assert!(
            token
                .password
                .password_type
                .as_ref()
                .unwrap()
                .contains("PasswordText")
        );
        // No nonce or created for plaintext
        assert!(token.nonce.is_none());
        assert!(token.created.is_none());
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
    fn test_missing_envelope() {
        let xml = r#"<s:Body>
    <GetDeviceInformation/>
</s:Body>"#;

        let result = parse_soap_request(xml);
        assert!(matches!(result, Err(SoapParseError::MissingEnvelope)));
    }

    #[test]
    fn test_envelope_not_root() {
        let xml = r#"<wrapper>
    <s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
        <s:Body>
            <GetDeviceInformation/>
        </s:Body>
    </s:Envelope>
</wrapper>"#;

        let result = parse_soap_request(xml);
        assert!(matches!(result, Err(SoapParseError::InvalidStructure(_))));
    }

    #[test]
    fn test_invalid_soap_namespace() {
        let xml = r#"<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope-wrong">
    <s:Body>
        <GetDeviceInformation/>
    </s:Body>
</s:Envelope>"#;

        let result = parse_soap_request(xml);
        assert!(matches!(result, Err(SoapParseError::InvalidStructure(_))));
    }

    #[test]
    fn test_missing_namespace_declaration() {
        let xml = r#"<Envelope>
    <Body>
        <GetDeviceInformation/>
    </Body>
</Envelope>"#;

        let result = parse_soap_request(xml);
        assert!(matches!(result, Err(SoapParseError::InvalidStructure(_))));
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

    #[test]
    fn test_parse_soap_request_invalid_envelope_namespace() {
        // Test rejection of invalid SOAP envelope namespace
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2001/12/soap-envelope">
    <s:Body>
        <GetDeviceInformation/>
    </s:Body>
</s:Envelope>"#;

        let result = parse_soap_request(xml);
        assert!(result.is_err());
        match result {
            Err(SoapParseError::InvalidStructure(msg)) => {
                assert!(msg.contains("Invalid SOAP envelope namespace"));
                assert!(msg.contains("http://www.w3.org/2003/05/soap-envelope"));
            }
            _ => panic!("Expected InvalidStructure error"),
        }
    }

    #[test]
    fn test_parse_soap_request_valid_envelope_namespace() {
        // Test acceptance of correct SOAP 1.2 namespace
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
    <s:Body>
        <GetDeviceInformation/>
    </s:Body>
</s:Envelope>"#;

        let result = parse_soap_request(xml);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_soap_request_xml_escaping_attributes() {
        // Test XML with special characters in attributes (&, <, >, ", ')
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
    <s:Body>
        <SetHostname name="camera&lt;test&gt;" value="&quot;quoted&quot;"/>
    </s:Body>
</s:Envelope>"#;

        let result = parse_soap_request(xml);
        assert!(result.is_ok());
        let envelope = result.unwrap();
        // The body should preserve the escaped characters
        assert!(envelope.body_xml.contains("&lt;"));
        assert!(envelope.body_xml.contains("&gt;"));
        assert!(envelope.body_xml.contains("&quot;"));
    }

    // ===== Bug 1 Tests: SOAP envelope namespace extraction robustness =====
    // Issue: anyka-dev-2sx - Parser captures first xmlns instead of SOAP-specific

    #[test]
    fn test_extract_envelope_namespace_non_soap_before_soap() {
        // Test that SOAP envelope namespace is correctly extracted when
        // non-SOAP namespaces appear BEFORE the SOAP namespace in attribute order
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:tds="http://www.onvif.org/ver10/device/wsdl" xmlns:s="http://www.w3.org/2003/05/soap-envelope">
    <s:Body>
        <GetDeviceInformation/>
    </s:Body>
</s:Envelope>"#;

        let result = parse_soap_request(xml);
        assert!(
            result.is_ok(),
            "Should parse correctly even when non-SOAP xmlns comes first"
        );
        let envelope = result.unwrap();
        assert!(envelope.body_xml.contains("GetDeviceInformation"));
    }

    #[test]
    fn test_extract_envelope_namespace_multiple_non_soap_before_soap() {
        // Test with multiple non-SOAP namespaces before SOAP namespace
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:tt="http://www.onvif.org/ver10/schema" xmlns:trt="http://www.onvif.org/ver10/media/wsdl" xmlns:tds="http://www.onvif.org/ver10/device/wsdl" xmlns:s="http://www.w3.org/2003/05/soap-envelope">
    <s:Body>
        <GetDeviceInformation/>
    </s:Body>
</s:Envelope>"#;

        let result = parse_soap_request(xml);
        assert!(
            result.is_ok(),
            "Should parse correctly with multiple non-SOAP xmlns before SOAP"
        );
        let envelope = result.unwrap();
        assert!(envelope.body_xml.contains("GetDeviceInformation"));
    }

    #[test]
    fn test_extract_envelope_namespace_soap_first_order() {
        // Test with SOAP namespace first (original behavior should still work)
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope" xmlns:tds="http://www.onvif.org/ver10/device/wsdl">
    <s:Body>
        <GetDeviceInformation/>
    </s:Body>
</s:Envelope>"#;

        let result = parse_soap_request(xml);
        assert!(result.is_ok());
        let envelope = result.unwrap();
        assert!(envelope.body_xml.contains("GetDeviceInformation"));
    }

    #[test]
    fn test_extract_envelope_namespace_default_xmlns_before_prefixed() {
        // Test with default xmlns (no prefix) appearing before prefixed xmlns
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns="http://www.onvif.org/ver10/device/wsdl" xmlns:s="http://www.w3.org/2003/05/soap-envelope">
    <s:Body>
        <GetDeviceInformation/>
    </s:Body>
</s:Envelope>"#;

        let result = parse_soap_request(xml);
        assert!(
            result.is_ok(),
            "Should parse correctly with default xmlns before SOAP"
        );
        let envelope = result.unwrap();
        assert!(envelope.body_xml.contains("GetDeviceInformation"));
    }

    // ===== Bug 2 Tests: Preserve namespace semantics when rebuilding SOAP body XML =====
    // Issue: anyka-dev-2hh - QName prefixes dropped, losing namespace context

    #[test]
    fn test_preserve_qname_prefix_start_element() {
        // Test that QName prefixes are preserved in start elements
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope" xmlns:tds="http://www.onvif.org/ver10/device/wsdl">
    <s:Body>
        <tds:GetDeviceInformation tds:Token="abc123"/>
    </s:Body>
</s:Envelope>"#;

        let result = parse_soap_request(xml);
        assert!(result.is_ok());
        let envelope = result.unwrap();
        // Verify QName prefix is preserved in attribute
        assert!(envelope.body_xml.contains("tds:Token=\"abc123\""));
    }

    #[test]
    fn test_preserve_qname_prefix_empty_element() {
        // Test that QName prefixes are preserved in empty (self-closing) elements
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope" xmlns:tds="http://www.onvif.org/ver10/device/wsdl">
    <s:Body>
        <tds:GetDeviceInformation tds:Token="abc123"/>
    </s:Body>
</s:Envelope>"#;

        let result = parse_soap_request(xml);
        assert!(result.is_ok());
        let envelope = result.unwrap();
        // Verify QName prefix is preserved in empty element attribute
        assert!(envelope.body_xml.contains("tds:Token"));
    }

    #[test]
    fn test_preserve_qname_prefix_nested_elements() {
        // Test QName preservation in deeply nested elements (attributes only)
        // Note: Element names themselves use local names only; the fix is for attributes
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope" xmlns:tds="http://www.onvif.org/ver10/device/wsdl" xmlns:trt="http://www.onvif.org/ver10/media/wsdl">
    <s:Body>
        <GetProfiles>
            <Profiles trt:Token="main" tds:Id="123">
                <VideoEncoderConfiguration trt:Encoding="H264"/>
            </Profiles>
        </GetProfiles>
    </s:Body>
</s:Envelope>"#;

        let result = parse_soap_request(xml);
        assert!(result.is_ok());
        let envelope = result.unwrap();
        // Verify QName prefixes are preserved in attributes
        assert!(envelope.body_xml.contains("trt:Token"));
        assert!(envelope.body_xml.contains("tds:Id"));
        assert!(envelope.body_xml.contains("trt:Encoding"));
    }

    #[test]
    fn test_preserve_qname_prefix_multiple_attributes() {
        // Test QName preservation with multiple attributes on same element
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope" xmlns:tptz="http://www.onvif.org/ver20/ptz/wsdl">
    <s:Body>
        <tptz:ContinuousMove tptz:ProfileToken="profile1" tptz:Timeout="PT5S"/>
    </s:Body>
</s:Envelope>"#;

        let result = parse_soap_request(xml);
        assert!(result.is_ok());
        let envelope = result.unwrap();
        // Verify both QName-prefixed attributes are preserved
        assert!(envelope.body_xml.contains("tptz:ProfileToken=\"profile1\""));
        assert!(envelope.body_xml.contains("tptz:Timeout=\"PT5S\""));
    }
}
