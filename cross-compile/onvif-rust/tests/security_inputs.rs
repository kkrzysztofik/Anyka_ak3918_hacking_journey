//! Integration tests for security input validation.
//!
//! Tests XSS, path traversal, and XML attack prevention.

use onvif_rust::security::{XmlSecurityError, XmlSecurityValidator};
use onvif_rust::utils::validation::{PathValidator, SecurityError, SecurityValidator};

/// Test that XXE attacks in SOAP bodies are detected.
#[test]
fn test_xxe_in_soap_request() {
    let validator = XmlSecurityValidator::default();

    let soap_xxe = r#"<?xml version="1.0"?>
        <!DOCTYPE foo [
            <!ENTITY xxe SYSTEM "file:///etc/passwd">
        ]>
        <s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
            <s:Body>
                <GetDeviceInformation>
                    <Username>&xxe;</Username>
                </GetDeviceInformation>
            </s:Body>
        </s:Envelope>"#;

    let result = validator.validate(soap_xxe);
    assert!(
        matches!(result, Err(XmlSecurityError::XxeDetected(_))),
        "XXE attack should be detected"
    );
}

/// Test that XML bombs are detected in SOAP requests.
#[test]
fn test_xml_bomb_in_soap_request() {
    let validator = SecurityValidator::default();

    // Simplified billion laughs pattern
    let bomb = r#"<?xml version="1.0"?>
        <!DOCTYPE lolz [
            <!ENTITY lol "lol">
            <!ENTITY lol2 "&lol;&lol;&lol;&lol;&lol;&lol;&lol;&lol;&lol;&lol;">
            <!ENTITY lol3 "&lol2;&lol2;&lol2;&lol2;&lol2;&lol2;&lol2;&lol2;&lol2;&lol2;">
        ]>
        <s:Envelope><s:Body>&lol3;</s:Body></s:Envelope>"#;

    let result = validator.check_xml_security(bomb);
    assert!(
        result.is_err(),
        "XML bomb should be detected (XXE or bomb detection)"
    );
}

/// Test that XSS patterns in SOAP headers are rejected.
#[test]
fn test_xss_in_soap_header() {
    let validator = SecurityValidator::default();

    let xss_soap = r#"<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
        <s:Header>
            <Security>
                <Username><script>alert('xss')</script></Username>
            </Security>
        </s:Header>
        <s:Body/>
    </s:Envelope>"#;

    let result = validator.check_xml_security(xss_soap);
    assert!(
        matches!(result, Err(SecurityError::XssDetected(_))),
        "XSS in SOAP header should be detected"
    );
}

/// Test that JavaScript protocol XSS is rejected.
#[test]
fn test_xss_javascript_protocol() {
    let validator = SecurityValidator::default();

    let xss_soap = r#"<SetHostname>
        <Name>javascript:alert(document.cookie)</Name>
    </SetHostname>"#;

    let result = validator.check_xml_security(xss_soap);
    assert!(
        matches!(result, Err(SecurityError::XssDetected(_))),
        "JavaScript protocol XSS should be detected"
    );
}

/// Test that event handler XSS is rejected.
#[test]
fn test_xss_event_handlers() {
    let validator = SecurityValidator::default();

    for handler in &["onclick", "onerror", "onload", "onmouseover", "onfocus"] {
        let xss_soap = format!(r#"<Config {}="alert(1)">test</Config>"#, handler);

        let result = validator.check_xml_security(&xss_soap);
        assert!(
            matches!(result, Err(SecurityError::XssDetected(_))),
            "Event handler '{}' XSS should be detected",
            handler
        );
    }
}

/// Test path traversal in request paths.
#[test]
fn test_path_traversal_basic() {
    let validator = PathValidator::new("/onvif");

    // Various traversal patterns
    let attack_paths = [
        "/onvif/../../../etc/passwd",
        "/onvif/..\\..\\..\\etc\\passwd",
        "/onvif/device_service/../../../etc/shadow",
        "/onvif/..",
    ];

    for path in attack_paths {
        let result = validator.validate_path(path);
        assert!(
            matches!(result, Err(SecurityError::PathTraversal(_))),
            "Path traversal should be detected for: {}",
            path
        );
    }
}

/// Test URL-encoded path traversal.
#[test]
fn test_path_traversal_url_encoded() {
    let validator = PathValidator::new("/onvif");

    // URL-encoded traversal attempts
    let encoded_attacks = [
        "/onvif/%2e%2e/%2e%2e/etc/passwd",
        "/onvif/%252e%252e/etc/passwd", // Double-encoded
    ];

    for path in encoded_attacks {
        let result = validator.validate_path(path);
        assert!(
            matches!(result, Err(SecurityError::PathTraversal(_))),
            "URL-encoded path traversal should be detected for: {}",
            path
        );
    }
}

/// Test null byte injection in paths.
#[test]
fn test_path_null_byte_injection() {
    let validator = PathValidator::new("/onvif");

    let result = validator.validate_path("/onvif/device_service\0.php");
    assert!(
        matches!(result, Err(SecurityError::PathTraversal(_))),
        "Null byte in path should be detected"
    );
}

/// Test that safe paths are allowed.
#[test]
fn test_safe_paths_allowed() {
    let validator = PathValidator::new("/onvif");

    let safe_paths = [
        "/onvif/device_service",
        "/onvif/media_service",
        "/onvif/ptz_service",
        "/onvif/imaging_service",
        "/onvif/snapshot",
    ];

    for path in safe_paths {
        let result = validator.validate_path(path);
        assert!(result.is_ok(), "Safe path should be allowed: {}", path);
    }
}

/// Test that safe SOAP requests pass validation.
#[test]
fn test_safe_soap_request() {
    let validator = SecurityValidator::default();

    let safe_soap = r#"<?xml version="1.0" encoding="UTF-8"?>
        <s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope"
                    xmlns:tds="http://www.onvif.org/ver10/device/wsdl">
            <s:Body>
                <tds:GetDeviceInformation/>
            </s:Body>
        </s:Envelope>"#;

    let result = validator.check_xml_security(safe_soap);
    assert!(result.is_ok(), "Safe SOAP request should pass validation");
}

/// Test that CDATA sections are allowed.
#[test]
fn test_cdata_sections_allowed() {
    let validator = SecurityValidator::default();

    let cdata_soap = r#"<s:Envelope>
        <s:Body>
            <Data><![CDATA[<raw> & "unescaped" content]]></Data>
        </s:Body>
    </s:Envelope>"#;

    let result = validator.check_xml_security(cdata_soap);
    assert!(result.is_ok(), "CDATA sections should be allowed");
}

/// Test large payload rejection.
#[test]
fn test_large_payload_rejected() {
    // Create validator with 1KB limit
    let validator = XmlSecurityValidator::new(10, 1024);

    let large_payload = "x".repeat(2048);

    let result = validator.validate(&large_payload);
    assert!(
        matches!(result, Err(XmlSecurityError::PayloadTooLarge { .. })),
        "Large payload should be rejected"
    );
}

/// Test that standard XML entities are allowed.
#[test]
fn test_standard_entities_allowed() {
    let validator = SecurityValidator::default();

    let with_entities = r#"<Message>Tom &amp; Jerry &lt;together&gt;</Message>"#;

    let result = validator.check_xml_security(with_entities);
    assert!(result.is_ok(), "Standard XML entities should be allowed");
}

/// Test Unicode content is allowed.
#[test]
fn test_unicode_content_allowed() {
    let validator = SecurityValidator::default();

    let unicode_soap = r#"<s:Envelope>
        <s:Body>
            <Name>カメラ名前 🎥</Name>
            <Description>日本語テスト</Description>
        </s:Body>
    </s:Envelope>"#;

    let result = validator.check_xml_security(unicode_soap);
    assert!(result.is_ok(), "Unicode content should be allowed");
}

/// Test file:// protocol detection (XXE vector).
#[test]
fn test_file_protocol_detected() {
    let validator = XmlSecurityValidator::default();

    let xxe = r#"<Config>file:///etc/passwd</Config>"#;

    let result = validator.validate(xxe);
    assert!(
        matches!(result, Err(XmlSecurityError::XxeDetected(_))),
        "file:// protocol should be detected as XXE vector"
    );
}

/// Test expect:// and other dangerous protocols.
#[test]
fn test_dangerous_protocols_detected() {
    let validator = XmlSecurityValidator::default();

    for protocol in &["expect://", "php://", "data://", "gopher://"] {
        let payload = format!("<url>{}</url>", protocol);
        let result = validator.validate(&payload);
        assert!(
            matches!(result, Err(XmlSecurityError::XxeDetected(_))),
            "Protocol {} should be detected as dangerous",
            protocol
        );
    }
}
