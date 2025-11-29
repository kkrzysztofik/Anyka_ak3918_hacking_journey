//! XML security validation.
//!
//! Detects and prevents XML-based attacks:
//! - XXE (XML External Entity) attacks
//! - XML bomb (billion laughs) attacks
//! - Entity expansion attacks
//!
//! # Example
//!
//! ```
//! use onvif_rust::security::xml_security::{XmlSecurityValidator, XmlSecurityError};
//!
//! let validator = XmlSecurityValidator::default();
//!
//! // Check for attacks
//! let malicious = "<!ENTITY xxe SYSTEM 'file:///etc/passwd'>";
//! assert!(matches!(
//!     validator.validate(malicious),
//!     Err(XmlSecurityError::XxeDetected(_))
//! ));
//!
//! // Safe XML passes
//! let safe = "<GetDeviceInformation/>";
//! assert!(validator.validate(safe).is_ok());
//! ```

use thiserror::Error;

/// Default maximum payload size (1MB).
pub const DEFAULT_MAX_PAYLOAD_SIZE: usize = 1024 * 1024;

/// Maximum allowed entity expansions.
pub const DEFAULT_MAX_ENTITY_EXPANSIONS: usize = 10;

/// XML security validation errors.
#[derive(Debug, Clone, Error, PartialEq)]
pub enum XmlSecurityError {
    /// XXE (XML External Entity) attack detected.
    #[error("XXE attack detected: {0}")]
    XxeDetected(String),

    /// XML bomb (billion laughs) attack detected.
    #[error("XML bomb detected: excessive entity references")]
    XmlBombDetected,

    /// Payload exceeds maximum size.
    #[error("Payload too large: {actual} bytes (max: {max})")]
    PayloadTooLarge { actual: usize, max: usize },
}

/// XML security validator for detecting common attacks.
#[derive(Debug, Clone)]
pub struct XmlSecurityValidator {
    /// Maximum payload size in bytes.
    max_payload_size: usize,
    /// Maximum allowed entity expansions.
    max_entity_expansions: usize,
}

impl XmlSecurityValidator {
    /// Create a new XML security validator.
    ///
    /// # Arguments
    ///
    /// * `max_entity_expansions` - Maximum entity references allowed.
    /// * `max_payload_size` - Maximum payload size in bytes.
    pub fn new(max_entity_expansions: usize, max_payload_size: usize) -> Self {
        Self {
            max_payload_size,
            max_entity_expansions,
        }
    }

    /// Validate XML content for security issues.
    ///
    /// Checks for:
    /// - XXE entity declarations
    /// - DOCTYPE declarations (potential XXE vector)
    /// - XML bomb patterns (excessive entity declarations)
    /// - Payload size limits
    ///
    /// # Arguments
    ///
    /// * `xml` - The XML string to validate
    ///
    /// # Returns
    ///
    /// `Ok(())` if safe, or `XmlSecurityError` if attack detected.
    pub fn validate(&self, xml: &str) -> Result<(), XmlSecurityError> {
        // Check payload size first (cheapest check)
        if xml.len() > self.max_payload_size {
            return Err(XmlSecurityError::PayloadTooLarge {
                actual: xml.len(),
                max: self.max_payload_size,
            });
        }

        let xml_lower = xml.to_lowercase();

        // Check for XXE patterns
        self.check_xxe_patterns(&xml_lower)?;

        // Check for XML bomb patterns
        self.check_xml_bomb(&xml_lower)?;

        Ok(())
    }

    /// Check for XXE (XML External Entity) attack patterns.
    fn check_xxe_patterns(&self, xml: &str) -> Result<(), XmlSecurityError> {
        // ENTITY declaration is the primary XXE vector
        if xml.contains("<!entity") {
            return Err(XmlSecurityError::XxeDetected(
                "ENTITY declaration detected".to_string(),
            ));
        }

        // DOCTYPE with SYSTEM or PUBLIC can reference external resources
        if xml.contains("<!doctype") {
            if xml.contains("system") || xml.contains("public") {
                return Err(XmlSecurityError::XxeDetected(
                    "DOCTYPE with external reference detected".to_string(),
                ));
            }
            // Even internal DOCTYPE can be used for attacks
            return Err(XmlSecurityError::XxeDetected(
                "DOCTYPE declaration detected".to_string(),
            ));
        }

        // Parameter entities (used in advanced XXE attacks)
        if xml.contains("%") && xml.contains("entity") {
            return Err(XmlSecurityError::XxeDetected(
                "Parameter entity detected".to_string(),
            ));
        }

        // Check for file:// protocol references
        if xml.contains("file://") {
            return Err(XmlSecurityError::XxeDetected(
                "Local file reference detected".to_string(),
            ));
        }

        // Check for expect:// and other dangerous protocols
        for protocol in &["expect://", "php://", "data://", "gopher://"] {
            if xml.contains(protocol) {
                return Err(XmlSecurityError::XxeDetected(format!(
                    "Dangerous protocol {} detected",
                    protocol
                )));
            }
        }

        Ok(())
    }

    /// Check for XML bomb (billion laughs) patterns.
    fn check_xml_bomb(&self, xml: &str) -> Result<(), XmlSecurityError> {
        // Count entity-like patterns (both declarations and references)
        let entity_decl_count = xml.matches("<!entity").count();
        let entity_ref_count = xml.matches('&').count().saturating_sub(
            // Subtract common safe entity references
            xml.matches("&amp;").count()
                + xml.matches("&lt;").count()
                + xml.matches("&gt;").count()
                + xml.matches("&quot;").count()
                + xml.matches("&apos;").count(),
        );

        // Total suspicious entity activity
        let total_entity_activity = entity_decl_count + entity_ref_count;

        // Excessive entity activity indicates potential XML bomb
        if total_entity_activity > self.max_entity_expansions {
            return Err(XmlSecurityError::XmlBombDetected);
        }

        // Check for deeply nested entity patterns (common in billion laughs)
        // Pattern: &lol1; &lol2; ... usually indicates escalating entities
        let mut consecutive_entity_refs = 0;
        let mut max_consecutive = 0;
        let mut in_entity_ref = false;

        for ch in xml.chars() {
            if ch == '&' {
                in_entity_ref = true;
            } else if ch == ';' && in_entity_ref {
                consecutive_entity_refs += 1;
                max_consecutive = max_consecutive.max(consecutive_entity_refs);
                in_entity_ref = false;
            } else if ch.is_whitespace() || ch == '<' {
                consecutive_entity_refs = 0;
                in_entity_ref = false;
            }
        }

        // Many consecutive entity references is suspicious
        if max_consecutive > self.max_entity_expansions / 2 {
            return Err(XmlSecurityError::XmlBombDetected);
        }

        Ok(())
    }

    /// Get the maximum payload size.
    pub fn max_payload_size(&self) -> usize {
        self.max_payload_size
    }

    /// Get the maximum entity expansions.
    pub fn max_entity_expansions(&self) -> usize {
        self.max_entity_expansions
    }
}

impl Default for XmlSecurityValidator {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_ENTITY_EXPANSIONS, DEFAULT_MAX_PAYLOAD_SIZE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_xml_passes() {
        let validator = XmlSecurityValidator::default();

        let safe_xml = r#"<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
            <s:Body>
                <GetDeviceInformation/>
            </s:Body>
        </s:Envelope>"#;

        assert!(validator.validate(safe_xml).is_ok());
    }

    #[test]
    fn test_xxe_entity_declaration() {
        let validator = XmlSecurityValidator::default();

        let xxe = r#"<?xml version="1.0"?>
            <!DOCTYPE foo [
                <!ENTITY xxe SYSTEM "file:///etc/passwd">
            ]>
            <foo>&xxe;</foo>"#;

        let result = validator.validate(xxe);
        assert!(matches!(result, Err(XmlSecurityError::XxeDetected(_))));
    }

    #[test]
    fn test_xxe_doctype_system() {
        let validator = XmlSecurityValidator::default();

        let xxe = r#"<!DOCTYPE foo SYSTEM "http://evil.com/xxe.dtd">
            <foo>bar</foo>"#;

        let result = validator.validate(xxe);
        assert!(matches!(result, Err(XmlSecurityError::XxeDetected(_))));
    }

    #[test]
    fn test_xxe_doctype_public() {
        let validator = XmlSecurityValidator::default();

        let xxe = r#"<!DOCTYPE foo PUBLIC "-//EVIL//DTD" "http://evil.com/xxe.dtd">
            <foo>bar</foo>"#;

        let result = validator.validate(xxe);
        assert!(matches!(result, Err(XmlSecurityError::XxeDetected(_))));
    }

    #[test]
    fn test_xxe_internal_doctype() {
        let validator = XmlSecurityValidator::default();

        let xxe = r#"<!DOCTYPE foo [
            <!ELEMENT foo (#PCDATA)>
        ]>
        <foo>bar</foo>"#;

        let result = validator.validate(xxe);
        assert!(matches!(result, Err(XmlSecurityError::XxeDetected(_))));
    }

    #[test]
    fn test_xxe_parameter_entity() {
        let validator = XmlSecurityValidator::default();

        let xxe = r#"<!DOCTYPE foo [
            <!ENTITY % xxe "malicious">
        ]>"#;

        let result = validator.validate(xxe);
        assert!(matches!(result, Err(XmlSecurityError::XxeDetected(_))));
    }

    #[test]
    fn test_xxe_file_protocol() {
        let validator = XmlSecurityValidator::default();

        let xxe = r#"<foo>file:///etc/passwd</foo>"#;

        let result = validator.validate(xxe);
        assert!(matches!(result, Err(XmlSecurityError::XxeDetected(_))));
    }

    #[test]
    fn test_xml_bomb_excessive_entities() {
        let validator = XmlSecurityValidator::new(5, 1024 * 1024);

        // More than 5 entity references
        let bomb = "&lol1;&lol2;&lol3;&lol4;&lol5;&lol6;";

        let result = validator.validate(bomb);
        assert!(matches!(result, Err(XmlSecurityError::XmlBombDetected)));
    }

    #[test]
    fn test_xml_bomb_billion_laughs_pattern() {
        let validator = XmlSecurityValidator::default();

        let bomb = r#"<!DOCTYPE lolz [
            <!ENTITY lol "lol">
            <!ENTITY lol2 "&lol;&lol;&lol;&lol;">
            <!ENTITY lol3 "&lol2;&lol2;&lol2;&lol2;">
        ]>"#;

        let result = validator.validate(bomb);
        assert!(matches!(
            result,
            Err(XmlSecurityError::XxeDetected(_)) | Err(XmlSecurityError::XmlBombDetected)
        ));
    }

    #[test]
    fn test_payload_too_large() {
        let validator = XmlSecurityValidator::new(10, 100);

        let large = "x".repeat(101);

        let result = validator.validate(&large);
        assert!(matches!(
            result,
            Err(XmlSecurityError::PayloadTooLarge {
                actual: 101,
                max: 100
            })
        ));
    }

    #[test]
    fn test_safe_entity_references() {
        let validator = XmlSecurityValidator::default();

        // Standard XML entities are safe
        let safe = "<message>Tom &amp; Jerry &lt;together&gt; &quot;forever&quot;</message>";

        assert!(validator.validate(safe).is_ok());
    }

    #[test]
    fn test_dangerous_protocols() {
        let validator = XmlSecurityValidator::default();

        for protocol in &["expect://", "php://", "data://", "gopher://"] {
            let malicious = format!("<foo>{}</foo>", protocol);
            let result = validator.validate(&malicious);
            assert!(
                matches!(result, Err(XmlSecurityError::XxeDetected(_))),
                "Protocol {} should be detected",
                protocol
            );
        }
    }

    #[test]
    fn test_default_settings() {
        let validator = XmlSecurityValidator::default();
        assert_eq!(validator.max_payload_size(), DEFAULT_MAX_PAYLOAD_SIZE);
        assert_eq!(
            validator.max_entity_expansions(),
            DEFAULT_MAX_ENTITY_EXPANSIONS
        );
    }

    #[test]
    fn test_case_insensitive_detection() {
        let validator = XmlSecurityValidator::default();

        // Mixed case should still be detected
        let xxe = "<!DOCTYPE Foo SYSTEM 'http://evil.com'>";
        assert!(matches!(
            validator.validate(xxe),
            Err(XmlSecurityError::XxeDetected(_))
        ));

        let xxe2 = "<!ENTITY Xxe SYSTEM 'file:///etc/passwd'>";
        assert!(matches!(
            validator.validate(xxe2),
            Err(XmlSecurityError::XxeDetected(_))
        ));
    }
}
