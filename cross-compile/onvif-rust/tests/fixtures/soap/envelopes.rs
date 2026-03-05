//! SOAP envelope fixtures for ONVIF testing.
//!
//! This module contains `pub const` strings representing valid and invalid
//! SOAP envelopes used for testing the ONVIF SOAP dispatcher.
//!
//! ## Namespaces Used
//!
//! - SOAP Envelope: `http://www.w3.org/2003/05/soap-envelope`
//! - ONVIF Device: `http://www.onvif.org/ver10/device/wsdl`
//! - ONVIF Schema: `http://www.onvif.org/ver10/schema`
//! - WS-Security: `http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-secext-1.0.xsd`
//! - WS-Security Utility: `http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-utility-1.0.xsd`

// =============================================================================
// VALID ENVELOPES (Happy Path)
// =============================================================================

/// Simplest valid SOAP envelope for GetDeviceInformation request.
/// Tests the minimal case for SOAP envelope parsing.
///
/// **Bug Reference**: Baseline test - no specific bug
pub const MINIMAL_GET_DEVICE_INFO: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
  <s:Body>
    <GetDeviceInformation xmlns="http://www.onvif.org/ver10/device/wsdl"/>
  </s:Body>
</s:Envelope>"#;

/// Body element with `tds:` prefix (tests anyka-dev-2hh prefix handling).
/// The request body uses a namespace prefix which some parsers may mishandle.
///
/// **Bug Reference**: anyka-dev-2hh - prefix handling in body elements
pub const PREFIXED_BODY_GET_DEVICE_INFO: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
  <s:Body>
    <tds:GetDeviceInformation xmlns:tds="http://www.onvif.org/ver10/device/wsdl"/>
  </s:Body>
</s:Envelope>"#;

/// Body with multiple prefixes: `tds:` for device namespace and `tt:` for type namespace.
/// Tests proper namespace resolution with multiple prefixes.
///
/// **Bug Reference**: anyka-dev-2hh - multiple prefix handling
pub const NESTED_BODY_WITH_PREFIXES: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope" xmlns:tds="http://www.onvif.org/ver10/device/wsdl" xmlns:tt="http://www.onvif.org/ver10/schema">
  <s:Body>
    <tds:GetDeviceInformation>
      <tds:Info>
        <tt:Manufacturer>Anyka</tt:Manufacturer>
      </tds:Info>
    </tds:GetDeviceInformation>
  </s:Body>
</s:Envelope>"#;

/// Complete envelope with WS-Security digest authentication.
/// This tests the full security header processing path.
///
/// **Bug Reference**: Baseline test - WS-Security digest
pub const FULL_WS_SECURITY_DIGEST: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope" xmlns:a="http://schemas.xmlsoap.org/ws/2004/08/addressing" xmlns:u="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-utility-1.0.xsd">
  <s:Header>
    <a:Action s:mustUnderstand="1">http://www.onvif.org/ver10/device/wsdl/GetDeviceInformation</a:Action>
    <a:MessageID>uuid:84ede29d-7f01-4a1c-9d3c-123456789012</a:MessageID>
    <a:To s:mustUnderstand="1">http://192.168.1.100:8080/onvif/device_service</a:To>
    <o:Security xmlns:o="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-secext-1.0.xsd" s:mustUnderstand="1">
      <o:UsernameToken>
        <o:Username>admin</o:Username>
        <o:Nonce>abc123nonce</o:Nonce>
        <o:Created>2024-01-15T10:30:00Z</o:Created>
        <o:Password Type="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-username-token-profile-1.0#PasswordDigest">passworddigestvalue</o:Password>
      </o:UsernameToken>
    </o:Security>
  </s:Header>
  <s:Body>
    <GetDeviceInformation xmlns="http://www.onvif.org/ver10/device/wsdl"/>
  </s:Body>
</s:Envelope>"#;

/// WS-Security with PasswordText (plain password authentication).
/// Tests the alternative password type in WS-Security.
///
/// **Bug Reference**: Baseline test - WS-Security plaintext
pub const FULL_WS_SECURITY_PLAINTEXT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope" xmlns:a="http://schemas.xmlsoap.org/ws/2004/08/addressing">
  <s:Header>
    <a:Action s:mustUnderstand="1">http://www.onvif.org/ver10/device/wsdl/GetDeviceInformation</a:Action>
    <a:MessageID>uuid:84ede29d-7f01-4a1c-9d3c-123456789013</a:MessageID>
    <a:To s:mustUnderstand="1">http://192.168.1.100:8080/onvif/device_service</a:To>
    <o:Security xmlns:o="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-secext-1.0.xsd" s:mustUnderstand="1">
      <o:UsernameToken>
        <o:Username>admin</o:Username>
        <o:Password Type="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-username-token-profile-1.0#PasswordText">admin123</o:Password>
      </o:UsernameToken>
    </o:Security>
  </s:Header>
  <s:Body>
    <GetDeviceInformation xmlns="http://www.onvif.org/ver10/device/wsdl"/>
  </s:Body>
</s:Envelope>"#;

/// Self-closing body element (minimal valid body).
/// Some XML parsers may handle this differently.
///
/// **Bug Reference**: anyka-dev-2hh - self-closing element parsing
pub const EMPTY_BODY_ELEMENT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
  <s:Body/>
</s:Envelope>"#;

/// Body element with XML attributes.
/// Tests attribute preservation during parsing.
///
/// **Bug Reference**: Baseline test - body with attributes
pub const BODY_WITH_ATTRIBUTES: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
  <s:Body xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
    <GetDeviceInformation xmlns="http://www.onvif.org/ver10/device/wsdl" xsi:nil="false"/>
  </s:Body>
</s:Envelope>"#;

/// Multiple xmlns declarations on envelope.
/// Tests proper handling of multiple namespace declarations.
///
/// **Bug Reference**: anyka-dev-2sx - multiple xmlns declarations
pub const MULTIPLE_XMLNS_ON_ENVELOPE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope" xmlns:tds="http://www.onvif.org/ver10/device/wsdl" xmlns:tt="http://www.onvif.org/ver10/schema" xmlns:trt="http://www.onvif.org/ver10/media/wsdl" xmlns:tev="http://www.onvif.org/ver10/events/wsdl">
  <s:Body>
    <GetDeviceInformation xmlns="http://www.onvif.org/ver10/device/wsdl"/>
  </s:Body>
</s:Envelope>"#;

/// **CRITICAL**: xmlns:tds appears BEFORE xmlns:s (tests anyka-dev-2sx bug).
/// This specific ordering triggered a namespace resolution bug where
/// the parser fails to find the SOAP namespace when tds is declared first.
///
/// **Bug Reference**: anyka-dev-2sx - xmlns ordering causes parser failure
pub const NON_SOAP_XMLNS_FIRST: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:tds="http://www.onvif.org/ver10/device/wsdl" xmlns:s="http://www.w3.org/2003/05/soap-envelope">
  <s:Body>
    <GetDeviceInformation xmlns="http://www.onvif.org/ver10/device/wsdl"/>
  </s:Body>
</s:Envelope>"#;

// =============================================================================
// INVALID ENVELOPES (Rejection Cases)
// =============================================================================

/// Just the Body element without Envelope wrapper.
/// Should be rejected as invalid SOAP.
///
/// **Bug Reference**: Baseline test - missing envelope
pub const MISSING_ENVELOPE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Body xmlns="http://www.onvif.org/ver10/device/wsdl">
  <GetDeviceInformation/>
</Body>"#;

/// Envelope with no Body element.
/// Should be rejected as invalid SOAP.
///
/// **Bug Reference**: Baseline test - missing body
pub const MISSING_BODY: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
</s:Envelope>"#;

/// SOAP 1.1 namespace instead of SOAP 1.2.
/// ONVIF requires SOAP 1.2, so this should be rejected.
///
/// **Bug Reference**: Baseline test - wrong SOAP version
pub const WRONG_SOAP_NAMESPACE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/">
  <s:Body>
    <GetDeviceInformation xmlns="http://www.onvif.org/ver10/device/wsdl"/>
  </s:Body>
</s:Envelope>"#;

/// Envelope not at root level (wrapped envelope).
/// Tests that we reject non-root envelopes.
///
/// **Bug Reference**: Baseline test - nested envelope
pub const ENVELOPE_NOT_ROOT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Wrapper>
  <s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
    <s:Body>
      <GetDeviceInformation xmlns="http://www.onvif.org/ver10/device/wsdl"/>
    </s:Body>
  </s:Envelope>
</Wrapper>"#;

/// Elements without namespace declarations.
/// Should be rejected as invalid ONVIF.
///
/// **Bug Reference**: Baseline test - missing namespace
pub const NO_NAMESPACE_DECLARATION: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
  <s:Body>
    <GetDeviceInformation>
      <Manufacturer>Anyka</Manufacturer>
    </GetDeviceInformation>
  </s:Body>
</s:Envelope>"#;

/// Empty string input.
/// Tests rejection of empty/non-existent input.
///
/// **Bug Reference**: Baseline test - empty input
pub const EMPTY_STRING: &str = "";
