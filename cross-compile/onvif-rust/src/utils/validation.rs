//! Security-focused input validation and sanitization.
//!
//! This module provides security validation for XML/SOAP inputs to prevent:
//! - XXE (XML External Entity) attacks
//! - XSS (Cross-Site Scripting) patterns
//! - XML bomb attacks
//!
//! Note: HTTP-level validation (method, content-type, body size) is handled
//! by axum middleware in `onvif/server.rs`. This module focuses on
//! content-level security checks.
//!
//! For struct field validation, use the `validator` crate with derive macros.
//!
//! # Example
//!
//! ```
//! use onvif_rust::utils::validation::{SecurityValidator, SecurityError};
//!
//! let validator = SecurityValidator::default();
//!
//! // Check for XXE attacks
//! let malicious = "<!ENTITY xxe SYSTEM 'file:///etc/passwd'>";
//! assert!(validator.check_xml_security(malicious).is_err());
//!
//! // Safe input passes
//! let safe = "<GetDeviceInformation/>";
//! assert!(validator.check_xml_security(safe).is_ok());
//! ```

use std::path::{Path, PathBuf};
use thiserror::Error;

/// Default maximum XML payload size (1MB).
pub const DEFAULT_MAX_PAYLOAD_SIZE: usize = 1024 * 1024;

/// Maximum entity expansion depth to prevent XML bombs.
pub const MAX_ENTITY_EXPANSIONS: usize = 10;

/// Security validation error types.
#[derive(Debug, Clone, Error, PartialEq)]
pub enum SecurityError {
    /// XXE (XML External Entity) attack detected.
    #[error("XXE attack detected: {0}")]
    XxeDetected(String),

    /// XSS (Cross-Site Scripting) pattern detected.
    #[error("XSS pattern detected: {0}")]
    XssDetected(String),

    /// XML bomb (billion laughs) attack detected.
    #[error("XML bomb detected: too many entity declarations")]
    XmlBombDetected,

    /// Payload exceeds maximum size.
    #[error("Payload too large: {0} bytes (max: {1})")]
    PayloadTooLarge(usize, usize),

    /// Invalid characters in input.
    #[error("Invalid characters: {0}")]
    InvalidCharacters(String),

    /// Path traversal attack detected.
    #[error("Path traversal detected: {0}")]
    PathTraversal(String),
}

/// Security validator for XML/SOAP content.
///
/// Detects and prevents common XML-based attacks.
#[derive(Debug, Clone)]
pub struct SecurityValidator {
    /// Maximum payload size in bytes.
    max_payload_size: usize,
}

impl SecurityValidator {
    /// Create a new security validator with custom payload size limit.
    pub fn new(max_payload_size: usize) -> Self {
        Self { max_payload_size }
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
    /// `Ok(())` if safe, or `SecurityError` if attack detected.
    pub fn check_xml_security(&self, xml: &str) -> Result<(), SecurityError> {
        // Check payload size
        if xml.len() > self.max_payload_size {
            return Err(SecurityError::PayloadTooLarge(
                xml.len(),
                self.max_payload_size,
            ));
        }

        let xml_lower = xml.to_lowercase();

        // Check for XXE patterns
        self.check_xxe_patterns(&xml_lower)?;

        // Check for XML bomb patterns
        self.check_xml_bomb(&xml_lower)?;

        // Check for XSS patterns (relevant for SOAP responses displayed in browsers)
        self.check_xss_patterns(&xml_lower)?;

        Ok(())
    }

    /// Check for XXE (XML External Entity) attack patterns.
    fn check_xxe_patterns(&self, xml: &str) -> Result<(), SecurityError> {
        // DOCTYPE with ENTITY is the primary XXE vector
        if xml.contains("<!entity") {
            return Err(SecurityError::XxeDetected(
                "ENTITY declaration detected".to_string(),
            ));
        }

        // DOCTYPE with SYSTEM or PUBLIC can reference external resources
        if xml.contains("<!doctype") {
            if xml.contains("system") || xml.contains("public") {
                return Err(SecurityError::XxeDetected(
                    "DOCTYPE with external reference detected".to_string(),
                ));
            }
            // Even internal DOCTYPE can be used for attacks
            return Err(SecurityError::XxeDetected(
                "DOCTYPE declaration detected".to_string(),
            ));
        }

        // Parameter entities
        if xml.contains("%") && xml.contains("entity") {
            return Err(SecurityError::XxeDetected(
                "Parameter entity detected".to_string(),
            ));
        }

        Ok(())
    }

    /// Check for XML bomb (billion laughs) patterns.
    fn check_xml_bomb(&self, xml: &str) -> Result<(), SecurityError> {
        // Count entity-like patterns
        let entity_count = xml.matches("<!entity").count() + xml.matches("&").count();

        // Excessive entity references indicate potential XML bomb
        if entity_count > MAX_ENTITY_EXPANSIONS {
            return Err(SecurityError::XmlBombDetected);
        }

        Ok(())
    }

    /// Check for XSS (Cross-Site Scripting) patterns.
    fn check_xss_patterns(&self, xml: &str) -> Result<(), SecurityError> {
        let xss_patterns = [
            ("javascript:", "JavaScript protocol"),
            ("vbscript:", "VBScript protocol"),
            ("<script", "Script tag"),
            ("onclick", "Event handler: onclick"),
            ("onerror", "Event handler: onerror"),
            ("onload", "Event handler: onload"),
            ("onmouseover", "Event handler: onmouseover"),
            ("onfocus", "Event handler: onfocus"),
            ("eval(", "eval() function"),
            ("expression(", "CSS expression"),
        ];

        for (pattern, description) in xss_patterns {
            if xml.contains(pattern) {
                return Err(SecurityError::XssDetected(description.to_string()));
            }
        }

        Ok(())
    }

    /// Sanitize a string by removing/escaping dangerous content.
    ///
    /// Use this for user-provided strings that will be included in XML responses.
    ///
    /// # Arguments
    ///
    /// * `input` - The string to sanitize
    ///
    /// # Returns
    ///
    /// A sanitized copy with dangerous patterns escaped or removed.
    pub fn sanitize_for_xml(&self, input: &str) -> String {
        input
            // XML entity encoding for special characters
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&apos;")
            // Remove null bytes and other control characters
            .chars()
            .filter(|c| !c.is_control() || *c == '\n' || *c == '\r' || *c == '\t')
            .collect()
    }

    /// Check a string for control characters (except whitespace).
    ///
    /// # Arguments
    ///
    /// * `input` - The string to check
    ///
    /// # Returns
    ///
    /// `Ok(())` if no invalid characters, or `SecurityError` if found.
    pub fn check_control_chars(&self, input: &str) -> Result<(), SecurityError> {
        for (idx, ch) in input.char_indices() {
            if ch.is_control() && ch != '\n' && ch != '\r' && ch != '\t' {
                return Err(SecurityError::InvalidCharacters(format!(
                    "Control character at position {}",
                    idx
                )));
            }
        }
        Ok(())
    }

    /// Get the maximum payload size.
    pub fn max_payload_size(&self) -> usize {
        self.max_payload_size
    }
}

impl Default for SecurityValidator {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_PAYLOAD_SIZE)
    }
}

/// Path traversal validator for HTTP request paths.
///
/// Validates that request paths do not contain path traversal sequences
/// that could escape the configured root directory.
///
/// # Example
///
/// ```
/// use onvif_rust::utils::validation::{PathValidator, SecurityError};
///
/// let validator = PathValidator::new("/onvif");
///
/// // Safe paths pass
/// assert!(validator.validate_path("/onvif/device_service").is_ok());
///
/// // Path traversal is rejected
/// assert!(validator.validate_path("/onvif/../../../etc/passwd").is_err());
/// ```
#[derive(Debug, Clone)]
pub struct PathValidator {
    /// The root path that requests must be confined to.
    root: PathBuf,
}

impl PathValidator {
    /// Create a new path validator with the given root.
    ///
    /// # Arguments
    ///
    /// * `root` - The root path that all requests must be within
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Validate that a request path is safe and within the root.
    ///
    /// Checks for:
    /// - `..` path traversal sequences
    /// - Null bytes (used in some bypass techniques)
    /// - Path normalization escapes
    ///
    /// # Arguments
    ///
    /// * `path` - The request path to validate
    ///
    /// # Returns
    ///
    /// `Ok(())` if safe, or `SecurityError::PathTraversal` if attack detected.
    pub fn validate_path(&self, path: &str) -> Result<(), SecurityError> {
        // Check for null bytes
        if path.contains('\0') {
            return Err(SecurityError::PathTraversal(
                "Null byte in path".to_string(),
            ));
        }

        // Check for explicit traversal sequences
        if self.contains_traversal_sequence(path) {
            return Err(SecurityError::PathTraversal(
                "Path traversal sequence detected".to_string(),
            ));
        }

        // Normalize and verify path stays within root
        let normalized = self.normalize_path(path);
        if !self.is_within_root(&normalized) {
            return Err(SecurityError::PathTraversal(
                "Path escapes root directory".to_string(),
            ));
        }

        Ok(())
    }

    /// Check for common path traversal sequences.
    fn contains_traversal_sequence(&self, path: &str) -> bool {
        // Check for ../ and ..\
        if path.contains("../") || path.contains("..\\") {
            return true;
        }

        // Check for URL-encoded variants
        let decoded = path.to_lowercase();
        if decoded.contains("%2e%2e") {
            return true;
        }

        // Check for double URL-encoding
        if decoded.contains("%252e") {
            return true;
        }

        // Check for trailing ..
        if path.ends_with("..") {
            return true;
        }

        false
    }

    /// Normalize a path by resolving . and .. segments.
    fn normalize_path(&self, path: &str) -> PathBuf {
        let mut normalized = PathBuf::new();

        for segment in path.split('/').filter(|s| !s.is_empty() && *s != ".") {
            if segment == ".." {
                normalized.pop();
            } else {
                normalized.push(segment);
            }
        }

        // Prepend root
        self.root.join(normalized)
    }

    /// Check if a normalized path is within the root directory.
    fn is_within_root(&self, path: &Path) -> bool {
        // Use starts_with to check containment
        path.starts_with(&self.root)
    }

    /// Get the configured root path.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Canonicalize and validate a path.
    ///
    /// This method normalizes the path and returns the canonical form
    /// if it's within the root. Use this to get a safe path for further
    /// processing.
    ///
    /// # Arguments
    ///
    /// * `path` - The request path to validate and normalize
    ///
    /// # Returns
    ///
    /// The normalized path if safe, or `SecurityError` if attack detected.
    pub fn canonicalize(&self, path: &str) -> Result<PathBuf, SecurityError> {
        self.validate_path(path)?;
        Ok(self.normalize_path(path))
    }
}

impl Default for PathValidator {
    fn default() -> Self {
        Self::new("/onvif")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_xml_passes() {
        let validator = SecurityValidator::default();

        let safe_xml = r#"<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
            <s:Body><GetDeviceInformation/></s:Body>
        </s:Envelope>"#;

        assert!(validator.check_xml_security(safe_xml).is_ok());
    }

    #[test]
    fn test_xxe_entity_detected() {
        let validator = SecurityValidator::default();

        let xxe = r#"<?xml version="1.0"?>
            <!DOCTYPE foo [<!ENTITY xxe SYSTEM "file:///etc/passwd">]>
            <foo>&xxe;</foo>"#;

        let result = validator.check_xml_security(xxe);
        assert!(matches!(result, Err(SecurityError::XxeDetected(_))));
    }

    #[test]
    fn test_xxe_doctype_detected() {
        let validator = SecurityValidator::default();

        let xxe = r#"<!DOCTYPE foo SYSTEM "http://evil.com/xxe.dtd"><foo/>"#;

        let result = validator.check_xml_security(xxe);
        assert!(matches!(result, Err(SecurityError::XxeDetected(_))));
    }

    #[test]
    fn test_xss_script_detected() {
        let validator = SecurityValidator::default();

        let xss = r#"<Name><script>alert('xss')</script></Name>"#;

        let result = validator.check_xml_security(xss);
        assert!(matches!(result, Err(SecurityError::XssDetected(_))));
    }

    #[test]
    fn test_xss_javascript_protocol_detected() {
        let validator = SecurityValidator::default();

        let xss = r#"<Link>javascript:alert(1)</Link>"#;

        let result = validator.check_xml_security(xss);
        assert!(matches!(result, Err(SecurityError::XssDetected(_))));
    }

    #[test]
    fn test_xss_event_handler_detected() {
        let validator = SecurityValidator::default();

        let xss = r#"<img onerror="alert(1)"/>"#;

        let result = validator.check_xml_security(xss);
        assert!(matches!(result, Err(SecurityError::XssDetected(_))));
    }

    #[test]
    fn test_payload_too_large() {
        let validator = SecurityValidator::new(100);

        let large = "x".repeat(200);

        let result = validator.check_xml_security(&large);
        assert!(matches!(result, Err(SecurityError::PayloadTooLarge(_, _))));
    }

    #[test]
    fn test_xml_bomb_detected() {
        let validator = SecurityValidator::default();

        // Simulate many entity references
        let bomb = "&a;&b;&c;&d;&e;&f;&g;&h;&i;&j;&k;&l;";

        let result = validator.check_xml_security(bomb);
        assert!(matches!(result, Err(SecurityError::XmlBombDetected)));
    }

    #[test]
    fn test_sanitize_for_xml() {
        let validator = SecurityValidator::default();

        assert_eq!(
            validator.sanitize_for_xml("<script>alert('xss')</script>"),
            "&lt;script&gt;alert(&apos;xss&apos;)&lt;/script&gt;"
        );

        assert_eq!(
            validator.sanitize_for_xml("Hello & Goodbye"),
            "Hello &amp; Goodbye"
        );

        assert_eq!(
            validator.sanitize_for_xml("\"quoted\""),
            "&quot;quoted&quot;"
        );
    }

    #[test]
    fn test_sanitize_removes_control_chars() {
        let validator = SecurityValidator::default();

        let dirty = "Hello\x00\x01World";
        let clean = validator.sanitize_for_xml(dirty);
        assert_eq!(clean, "HelloWorld");
    }

    #[test]
    fn test_sanitize_preserves_whitespace() {
        let validator = SecurityValidator::default();

        let input = "Hello\n\tWorld";
        let output = validator.sanitize_for_xml(input);
        assert_eq!(output, "Hello\n\tWorld");
    }

    #[test]
    fn test_check_control_chars_valid() {
        let validator = SecurityValidator::default();

        assert!(validator.check_control_chars("Hello\nWorld\tTest").is_ok());
    }

    #[test]
    fn test_check_control_chars_invalid() {
        let validator = SecurityValidator::default();

        let result = validator.check_control_chars("Hello\x00World");
        assert!(matches!(result, Err(SecurityError::InvalidCharacters(_))));
    }

    #[test]
    fn test_default_max_payload_size() {
        let validator = SecurityValidator::default();
        assert_eq!(validator.max_payload_size(), DEFAULT_MAX_PAYLOAD_SIZE);
    }

    #[test]
    fn test_custom_max_payload_size() {
        let validator = SecurityValidator::new(500);
        assert_eq!(validator.max_payload_size(), 500);
    }

    // === Additional XXE Tests ===

    #[test]
    fn test_xxe_parameter_entity_detected() {
        let validator = SecurityValidator::default();

        let xxe = r#"<!DOCTYPE foo [<!ENTITY % pe SYSTEM "http://evil.com/xxe.dtd">%pe;]>"#;

        let result = validator.check_xml_security(xxe);
        assert!(matches!(result, Err(SecurityError::XxeDetected(_))));
    }

    #[test]
    fn test_xxe_public_keyword_detected() {
        let validator = SecurityValidator::default();

        let xxe = r#"<!DOCTYPE foo PUBLIC "-//W3C//DTD" "http://evil.com/dtd">"#;

        let result = validator.check_xml_security(xxe);
        assert!(matches!(result, Err(SecurityError::XxeDetected(_))));
    }

    #[test]
    fn test_xxe_internal_doctype_detected() {
        let validator = SecurityValidator::default();

        // Even internal DOCTYPE without SYSTEM/PUBLIC should be flagged
        let xxe = r#"<!DOCTYPE foo [<!ELEMENT foo (#PCDATA)>]><foo/>"#;

        let result = validator.check_xml_security(xxe);
        assert!(matches!(result, Err(SecurityError::XxeDetected(_))));
    }

    // === Additional XSS Tests ===

    #[test]
    fn test_xss_vbscript_detected() {
        let validator = SecurityValidator::default();

        let xss = r#"<a href="vbscript:msgbox('xss')">click</a>"#;

        let result = validator.check_xml_security(xss);
        assert!(matches!(result, Err(SecurityError::XssDetected(_))));
    }

    #[test]
    fn test_xss_eval_detected() {
        let validator = SecurityValidator::default();

        let xss = r#"<Data>eval(user_input)</Data>"#;

        let result = validator.check_xml_security(xss);
        assert!(matches!(result, Err(SecurityError::XssDetected(_))));
    }

    #[test]
    fn test_xss_expression_detected() {
        let validator = SecurityValidator::default();

        let xss = r#"<style>width: expression(alert('xss'))</style>"#;

        let result = validator.check_xml_security(xss);
        assert!(matches!(result, Err(SecurityError::XssDetected(_))));
    }

    #[test]
    fn test_xss_onload_detected() {
        let validator = SecurityValidator::default();

        let xss = r#"<body onload="alert(1)">"#;

        let result = validator.check_xml_security(xss);
        assert!(matches!(result, Err(SecurityError::XssDetected(_))));
    }

    #[test]
    fn test_xss_onfocus_detected() {
        let validator = SecurityValidator::default();

        let xss = r#"<input onfocus="alert(1)">"#;

        let result = validator.check_xml_security(xss);
        assert!(matches!(result, Err(SecurityError::XssDetected(_))));
    }

    #[test]
    fn test_xss_onmouseover_detected() {
        let validator = SecurityValidator::default();

        let xss = r#"<div onmouseover="alert(1)">hover</div>"#;

        let result = validator.check_xml_security(xss);
        assert!(matches!(result, Err(SecurityError::XssDetected(_))));
    }

    #[test]
    fn test_xss_onclick_detected() {
        let validator = SecurityValidator::default();

        let xss = r#"<button onclick="alert(1)">click</button>"#;

        let result = validator.check_xml_security(xss);
        assert!(matches!(result, Err(SecurityError::XssDetected(_))));
    }

    // === Edge Case Tests ===

    #[test]
    fn test_empty_string_passes() {
        let validator = SecurityValidator::default();
        assert!(validator.check_xml_security("").is_ok());
    }

    #[test]
    fn test_unicode_content_passes() {
        let validator = SecurityValidator::default();

        let unicode = r#"<Name>日本語テスト</Name>"#;
        assert!(validator.check_xml_security(unicode).is_ok());
    }

    #[test]
    fn test_emoji_content_passes() {
        let validator = SecurityValidator::default();

        let emoji = r#"<Status>🎥 Camera OK ✅</Status>"#;
        assert!(validator.check_xml_security(emoji).is_ok());
    }

    #[test]
    fn test_cdata_section_passes() {
        let validator = SecurityValidator::default();

        let cdata = r#"<Data><![CDATA[Some <raw> data & content]]></Data>"#;
        assert!(validator.check_xml_security(cdata).is_ok());
    }

    #[test]
    fn test_valid_entity_references_within_limit() {
        let validator = SecurityValidator::default();

        // Up to MAX_ENTITY_EXPANSIONS is allowed
        let valid = "&amp;&lt;&gt;&quot;&apos;";
        assert!(validator.check_xml_security(valid).is_ok());
    }

    #[test]
    fn test_mixed_case_xss_detected() {
        let validator = SecurityValidator::default();

        // Case-insensitive detection
        let xss = r#"<SCRIPT>alert(1)</SCRIPT>"#;
        let result = validator.check_xml_security(xss);
        assert!(matches!(result, Err(SecurityError::XssDetected(_))));
    }

    #[test]
    fn test_mixed_case_xxe_detected() {
        let validator = SecurityValidator::default();

        // Case-insensitive detection
        let xxe = r#"<!DOCTYPE FOO SYSTEM "http://evil.com">"#;
        let result = validator.check_xml_security(xxe);
        assert!(matches!(result, Err(SecurityError::XxeDetected(_))));
    }

    // === Sanitization Edge Cases ===

    #[test]
    fn test_sanitize_empty_string() {
        let validator = SecurityValidator::default();
        assert_eq!(validator.sanitize_for_xml(""), "");
    }

    #[test]
    fn test_sanitize_unicode() {
        let validator = SecurityValidator::default();

        let input = "カメラ名 <test>";
        let output = validator.sanitize_for_xml(input);
        assert_eq!(output, "カメラ名 &lt;test&gt;");
    }

    #[test]
    fn test_sanitize_all_special_chars() {
        let validator = SecurityValidator::default();

        let input = r#"<tag attr="val" & 'other'>"#;
        let output = validator.sanitize_for_xml(input);
        assert_eq!(
            output,
            "&lt;tag attr=&quot;val&quot; &amp; &apos;other&apos;&gt;"
        );
    }

    // === Error Display Tests ===

    #[test]
    fn test_error_display_xxe() {
        let err = SecurityError::XxeDetected("test".to_string());
        assert_eq!(format!("{}", err), "XXE attack detected: test");
    }

    #[test]
    fn test_error_display_xss() {
        let err = SecurityError::XssDetected("script".to_string());
        assert_eq!(format!("{}", err), "XSS pattern detected: script");
    }

    #[test]
    fn test_error_display_xml_bomb() {
        let err = SecurityError::XmlBombDetected;
        assert_eq!(
            format!("{}", err),
            "XML bomb detected: too many entity declarations"
        );
    }

    #[test]
    fn test_error_display_payload() {
        let err = SecurityError::PayloadTooLarge(2000, 1000);
        assert_eq!(
            format!("{}", err),
            "Payload too large: 2000 bytes (max: 1000)"
        );
    }

    #[test]
    fn test_error_display_invalid_chars() {
        let err = SecurityError::InvalidCharacters("null byte".to_string());
        assert_eq!(format!("{}", err), "Invalid characters: null byte");
    }

    // === SecurityError Equality ===

    #[test]
    fn test_security_error_equality() {
        let e1 = SecurityError::XxeDetected("test".to_string());
        let e2 = SecurityError::XxeDetected("test".to_string());
        let e3 = SecurityError::XxeDetected("other".to_string());

        assert_eq!(e1, e2);
        assert_ne!(e1, e3);
    }

    #[test]
    fn test_security_error_clone() {
        let err = SecurityError::XmlBombDetected;
        let cloned = err.clone();
        assert_eq!(err, cloned);
    }

    // === Boundary Tests ===

    #[test]
    fn test_payload_exactly_at_limit() {
        let validator = SecurityValidator::new(100);

        let exact = "x".repeat(100);
        assert!(validator.check_xml_security(&exact).is_ok());
    }

    #[test]
    fn test_payload_one_over_limit() {
        let validator = SecurityValidator::new(100);

        let over = "x".repeat(101);
        let result = validator.check_xml_security(&over);
        assert!(matches!(
            result,
            Err(SecurityError::PayloadTooLarge(101, 100))
        ));
    }

    #[test]
    fn test_entity_count_at_limit() {
        let validator = SecurityValidator::default();

        // Exactly at limit (MAX_ENTITY_EXPANSIONS = 10)
        let at_limit = "&1;&2;&3;&4;&5;&6;&7;&8;&9;&a;";
        assert!(validator.check_xml_security(at_limit).is_ok());
    }

    #[test]
    fn test_entity_count_over_limit() {
        let validator = SecurityValidator::default();

        // One over limit
        let over_limit = "&1;&2;&3;&4;&5;&6;&7;&8;&9;&a;&b;";
        let result = validator.check_xml_security(over_limit);
        assert!(matches!(result, Err(SecurityError::XmlBombDetected)));
    }

    // === Path Traversal Tests ===

    #[test]
    fn test_path_validator_safe_path() {
        let validator = PathValidator::new("/onvif");
        assert!(validator.validate_path("/onvif/device_service").is_ok());
        assert!(validator.validate_path("/onvif/media_service").is_ok());
    }

    #[test]
    fn test_path_validator_traversal_detected() {
        let validator = PathValidator::new("/onvif");

        let result = validator.validate_path("/onvif/../../../etc/passwd");
        assert!(matches!(result, Err(SecurityError::PathTraversal(_))));
    }

    #[test]
    fn test_path_validator_null_byte() {
        let validator = PathValidator::new("/onvif");

        let result = validator.validate_path("/onvif/device\0.txt");
        assert!(matches!(result, Err(SecurityError::PathTraversal(_))));
    }

    #[test]
    fn test_path_validator_url_encoded() {
        let validator = PathValidator::new("/onvif");

        // URL-encoded ..
        let result = validator.validate_path("/onvif/%2e%2e/etc/passwd");
        assert!(matches!(result, Err(SecurityError::PathTraversal(_))));
    }

    #[test]
    fn test_path_validator_double_encoded() {
        let validator = PathValidator::new("/onvif");

        // Double URL-encoded
        let result = validator.validate_path("/onvif/%252e%252e/etc/passwd");
        assert!(matches!(result, Err(SecurityError::PathTraversal(_))));
    }

    #[test]
    fn test_path_validator_windows_style() {
        let validator = PathValidator::new("/onvif");

        let result = validator.validate_path("/onvif/..\\..\\etc\\passwd");
        assert!(matches!(result, Err(SecurityError::PathTraversal(_))));
    }

    #[test]
    fn test_path_validator_trailing_dots() {
        let validator = PathValidator::new("/onvif");

        let result = validator.validate_path("/onvif/..");
        assert!(matches!(result, Err(SecurityError::PathTraversal(_))));
    }

    #[test]
    fn test_path_validator_canonicalize() {
        let validator = PathValidator::new("/onvif");

        let result = validator.canonicalize("/onvif/device_service");
        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path.starts_with("/onvif"));
    }

    #[test]
    fn test_path_validator_root() {
        let validator = PathValidator::new("/api");
        assert_eq!(validator.root().to_str().unwrap(), "/api");
    }

    #[test]
    fn test_path_validator_default() {
        let validator = PathValidator::default();
        assert_eq!(validator.root().to_str().unwrap(), "/onvif");
    }

    #[test]
    fn test_path_traversal_error_display() {
        let err = SecurityError::PathTraversal("test".to_string());
        assert_eq!(format!("{}", err), "Path traversal detected: test");
    }
}
