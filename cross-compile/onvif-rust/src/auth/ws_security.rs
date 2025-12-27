//! WS-Security UsernameToken authentication.
//!
//! Implements ONVIF-compliant WS-Security UsernameToken digest authentication.
//! The digest is computed as: Base64(SHA1(Nonce + Created + Password))
//!
//! # Security Notes
//!
//! - **WARNING:** SHA-1 is cryptographically broken and should not be relied upon for security.
//!   ONVIF mandates SHA-1 for UsernameToken digest (legacy protocol requirement).
//! - **This implementation MUST only be used with HTTPS/TLS transport to mitigate SHA-1 vulnerabilities,
//!   as the protocol layer provides the actual security.**
//! - All comparisons use timing-safe operations to prevent timing attacks
//! - Timestamps are validated to prevent replay attacks
//!
//! # Example
//!
//! ```
//! use onvif_rust::auth::ws_security::{WsSecurityValidator, UsernameToken};
//!
//! let validator = WsSecurityValidator::new(300); // 5 minute window
//!
//! // Parse and validate a token (from SOAP header)
//! let token = UsernameToken {
//!     username: "admin".to_string(),
//!     nonce: "base64_encoded_nonce".to_string(),
//!     created: "2025-01-27T10:00:00Z".to_string(),
//!     password_digest: "base64_encoded_digest".to_string(),
//! };
//!
//! // Validate against known password
//! let result = validator.validate(&token, "secret_password");
//! ```

use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use chrono::{DateTime, Duration, Utc};
use constant_time_eq::constant_time_eq;
use sha1::{Digest, Sha1};
use thiserror::Error;

/// Default maximum timestamp age in seconds (5 minutes).
pub const DEFAULT_MAX_TIMESTAMP_AGE: i64 = 300;

/// Authentication errors for WS-Security.
#[derive(Debug, Clone, Error, PartialEq)]
pub enum AuthError {
    /// Nonce is missing or invalid format.
    #[error("Invalid nonce: {0}")]
    InvalidNonce(String),

    /// Timestamp is missing or invalid format.
    #[error("Invalid timestamp: {0}")]
    InvalidTimestamp(String),

    /// Timestamp is outside the acceptable window (potential replay attack).
    #[error("Timestamp expired or in the future")]
    ExpiredTimestamp,

    /// Password digest does not match.
    #[error("Invalid credentials")]
    InvalidDigest,

    /// Username is missing.
    #[error("Username is required")]
    MissingUsername,

    /// Token structure is invalid.
    #[error("Malformed UsernameToken: {0}")]
    MalformedToken(String),
}

/// Parsed WS-Security UsernameToken from SOAP header.
#[derive(Debug, Clone, PartialEq)]
pub struct UsernameToken {
    /// The username.
    pub username: String,

    /// Base64-encoded nonce value.
    pub nonce: String,

    /// ISO 8601 timestamp when the token was created.
    pub created: String,

    /// Base64-encoded SHA-1 digest: Base64(SHA1(Nonce + Created + Password))
    pub password_digest: String,
}

impl UsernameToken {
    /// Parse a UsernameToken from XML string.
    ///
    /// Expected format:
    /// ```xml
    /// <wsse:UsernameToken>
    ///   <wsse:Username>admin</wsse:Username>
    ///   <wsse:Password Type="...#PasswordDigest">digest</wsse:Password>
    ///   <wsse:Nonce EncodingType="...#Base64Binary">nonce</wsse:Nonce>
    ///   <wsu:Created>2025-01-27T10:00:00Z</wsu:Created>
    /// </wsse:UsernameToken>
    /// ```
    pub fn parse(xml: &str) -> Result<Self, AuthError> {
        // Extract username
        let username = extract_element(xml, "Username").ok_or(AuthError::MissingUsername)?;

        // Extract password digest
        let password_digest = extract_element(xml, "Password")
            .ok_or_else(|| AuthError::MalformedToken("Missing Password element".to_string()))?;

        // Extract nonce
        let nonce = extract_element(xml, "Nonce")
            .ok_or_else(|| AuthError::MalformedToken("Missing Nonce element".to_string()))?;

        // Extract created timestamp
        let created = extract_element(xml, "Created")
            .ok_or_else(|| AuthError::MalformedToken("Missing Created element".to_string()))?;

        Ok(UsernameToken {
            username,
            nonce,
            created,
            password_digest,
        })
    }
}

/// WS-Security UsernameToken validator.
///
/// Validates ONVIF UsernameToken digest authentication.
#[derive(Debug, Clone)]
pub struct WsSecurityValidator {
    /// Maximum allowed timestamp age in seconds.
    max_timestamp_age: i64,
}

impl WsSecurityValidator {
    /// Create a new validator with a custom timestamp window.
    ///
    /// # Arguments
    ///
    /// * `max_timestamp_age_seconds` - Maximum allowed age of the Created timestamp.
    ///   Both past and future drift are limited to this value.
    pub fn new(max_timestamp_age_seconds: i64) -> Self {
        Self {
            max_timestamp_age: max_timestamp_age_seconds,
        }
    }

    /// Validate a UsernameToken against a known password.
    ///
    /// # Arguments
    ///
    /// * `token` - The parsed UsernameToken from the SOAP header
    /// * `password` - The expected password for the user
    ///
    /// # Returns
    ///
    /// `Ok(())` if valid, or `AuthError` describing the failure.
    pub fn validate(&self, token: &UsernameToken, password: &str) -> Result<(), AuthError> {
        // Validate timestamp first (cheap check)
        self.validate_timestamp(&token.created)?;

        // Validate the digest
        self.validate_digest(token, password)?;

        Ok(())
    }

    /// Validate that the timestamp is within the acceptable window.
    ///
    /// # Arguments
    ///
    /// * `created` - ISO 8601 timestamp string
    ///
    /// # Returns
    ///
    /// `Ok(())` if valid, or `AuthError::ExpiredTimestamp` if outside window.
    pub fn validate_timestamp(&self, created: &str) -> Result<(), AuthError> {
        let created_time = DateTime::parse_from_rfc3339(created)
            .map_err(|e| AuthError::InvalidTimestamp(e.to_string()))?
            .with_timezone(&Utc);

        let now = Utc::now();
        let max_age = Duration::seconds(self.max_timestamp_age);

        // Check if timestamp is too old
        if now - created_time > max_age {
            return Err(AuthError::ExpiredTimestamp);
        }

        // Check if timestamp is in the future (with some tolerance)
        if created_time - now > max_age {
            return Err(AuthError::ExpiredTimestamp);
        }

        Ok(())
    }

    /// Validate the password digest.
    ///
    /// Computes SHA1(Nonce + Created + Password) and compares with the provided digest.
    ///
    /// # Arguments
    ///
    /// * `token` - The UsernameToken with nonce, created, and digest
    /// * `password` - The expected password
    ///
    /// # Returns
    ///
    /// `Ok(())` if digest matches, `AuthError::InvalidDigest` otherwise.
    pub fn validate_digest(&self, token: &UsernameToken, password: &str) -> Result<(), AuthError> {
        // Decode the nonce from Base64
        let nonce_bytes = BASE64
            .decode(&token.nonce)
            .map_err(|e| AuthError::InvalidNonce(e.to_string()))?;

        // Compute expected digest: SHA1(Nonce + Created + Password)
        let mut hasher = Sha1::new();
        hasher.update(&nonce_bytes);
        hasher.update(token.created.as_bytes());
        hasher.update(password.as_bytes());
        let expected_digest = hasher.finalize();

        // Decode the provided digest from Base64
        let provided_digest = BASE64
            .decode(&token.password_digest)
            .map_err(|_| AuthError::InvalidDigest)?;

        // Timing-safe comparison
        if !constant_time_eq(&expected_digest, &provided_digest) {
            return Err(AuthError::InvalidDigest);
        }

        Ok(())
    }

    /// Parse a UsernameToken from a SOAP header XML string.
    ///
    /// This is a convenience method that combines parsing and validation.
    ///
    /// # Arguments
    ///
    /// * `xml` - The SOAP header containing the Security element
    ///
    /// # Returns
    ///
    /// The parsed `UsernameToken` or an `AuthError`.
    pub fn parse_username_token(&self, xml: &str) -> Result<UsernameToken, AuthError> {
        UsernameToken::parse(xml)
    }

    /// Get the maximum timestamp age in seconds.
    pub fn max_timestamp_age(&self) -> i64 {
        self.max_timestamp_age
    }
}

impl Default for WsSecurityValidator {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_TIMESTAMP_AGE)
    }
}

/// Extract element content from XML using simple string matching.
///
/// This is a lightweight extraction that doesn't require a full XML parser.
/// It looks for patterns like `<tag>content</tag>` or `<ns:tag>content</ns:tag>`.
fn extract_element(xml: &str, tag: &str) -> Option<String> {
    for prefix in ["wsse:", "wsu:", ""] {
        if let Some(content) = extract_element_with_prefix(xml, tag, prefix) {
            return Some(content);
        }
    }
    None
}

/// Extract element content with a specific namespace prefix.
fn extract_element_with_prefix(xml: &str, tag: &str, prefix: &str) -> Option<String> {
    let open_tag_start = format!("<{}{}", prefix, tag);
    let close_tag = format!("</{}{}>", prefix, tag);

    let mut search_start = 0;
    while let Some(relative_pos) = xml[search_start..].find(&open_tag_start) {
        let tag_start_pos = search_start + relative_pos;
        let remaining_after_tag_name = &xml[tag_start_pos + open_tag_start.len()..];

        if !is_valid_tag_boundary(remaining_after_tag_name) {
            search_start = tag_start_pos + open_tag_start.len();
            continue;
        }

        if let Some(content) = extract_content_from_tag(
            xml,
            tag_start_pos,
            &open_tag_start,
            &close_tag,
            remaining_after_tag_name,
        ) {
            return Some(content);
        }

        search_start = tag_start_pos + open_tag_start.len();
    }
    None
}

/// Check if the next character is a valid tag boundary ('>' or whitespace).
fn is_valid_tag_boundary(remaining: &str) -> bool {
    match remaining.chars().next() {
        Some(c) => c == '>' || c.is_whitespace(),
        None => false,
    }
}

/// Extract content from a tag if found.
fn extract_content_from_tag(
    xml: &str,
    tag_start_pos: usize,
    open_tag_start: &str,
    close_tag: &str,
    remaining_after_tag_name: &str,
) -> Option<String> {
    let close_bracket_offset = remaining_after_tag_name.find('>')?;
    let content_start = tag_start_pos + open_tag_start.len() + close_bracket_offset + 1;
    let close_offset = xml[content_start..].find(close_tag)?;
    let content_end = content_start + close_offset;
    let content = &xml[content_start..content_end];
    Some(content.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Generate a valid UsernameToken for testing.
    fn generate_token(password: &str, timestamp: DateTime<Utc>) -> UsernameToken {
        // Generate a random nonce
        let nonce_bytes: [u8; 16] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
        let nonce = BASE64.encode(nonce_bytes);

        // Format timestamp
        let created = timestamp.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);

        // Compute digest
        let mut hasher = Sha1::new();
        hasher.update(nonce_bytes);
        hasher.update(created.as_bytes());
        hasher.update(password.as_bytes());
        let digest = BASE64.encode(hasher.finalize());

        UsernameToken {
            username: "admin".to_string(),
            nonce,
            created,
            password_digest: digest,
        }
    }

    #[test]
    fn test_valid_token_validates() {
        let validator = WsSecurityValidator::new(300);
        let password = "secret123";
        let token = generate_token(password, Utc::now());

        let result = validator.validate(&token, password);
        assert!(result.is_ok());
    }

    #[test]
    fn test_wrong_password_fails() {
        let validator = WsSecurityValidator::new(300);
        let token = generate_token("correct_password", Utc::now());

        let result = validator.validate(&token, "wrong_password");
        assert_eq!(result, Err(AuthError::InvalidDigest));
    }

    #[test]
    fn test_expired_timestamp_fails() {
        let validator = WsSecurityValidator::new(300);
        let password = "secret123";

        // Create token with timestamp 10 minutes ago
        let old_time = Utc::now() - Duration::minutes(10);
        let token = generate_token(password, old_time);

        let result = validator.validate(&token, password);
        assert_eq!(result, Err(AuthError::ExpiredTimestamp));
    }

    #[test]
    fn test_future_timestamp_fails() {
        let validator = WsSecurityValidator::new(300);
        let password = "secret123";

        // Create token with timestamp 10 minutes in the future
        let future_time = Utc::now() + Duration::minutes(10);
        let token = generate_token(password, future_time);

        let result = validator.validate(&token, password);
        assert_eq!(result, Err(AuthError::ExpiredTimestamp));
    }

    #[test]
    fn test_timestamp_at_edge_of_window() {
        let validator = WsSecurityValidator::new(300);
        let password = "secret123";

        // Create token with timestamp just within window (4 minutes ago)
        let edge_time = Utc::now() - Duration::minutes(4);
        let token = generate_token(password, edge_time);

        let result = validator.validate(&token, password);
        assert!(result.is_ok());
    }

    #[test]
    fn test_invalid_nonce_base64() {
        let validator = WsSecurityValidator::new(300);

        let token = UsernameToken {
            username: "admin".to_string(),
            nonce: "not_valid_base64!!!".to_string(),
            created: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            password_digest: BASE64.encode([0u8; 20]),
        };

        let result = validator.validate(&token, "password");
        assert!(matches!(result, Err(AuthError::InvalidNonce(_))));
    }

    #[test]
    fn test_invalid_timestamp_format() {
        let validator = WsSecurityValidator::new(300);

        let result = validator.validate_timestamp("not-a-timestamp");
        assert!(matches!(result, Err(AuthError::InvalidTimestamp(_))));
    }

    #[test]
    fn test_parse_username_token() {
        let xml = r#"
            <wsse:Security>
                <wsse:UsernameToken>
                    <wsse:Username>admin</wsse:Username>
                    <wsse:Password Type="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-username-token-profile-1.0#PasswordDigest">YWJjZGVm</wsse:Password>
                    <wsse:Nonce EncodingType="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-soap-message-security-1.0#Base64Binary">MTIzNDU2</wsse:Nonce>
                    <wsu:Created>2025-01-27T10:00:00Z</wsu:Created>
                </wsse:UsernameToken>
            </wsse:Security>
        "#;

        let token = UsernameToken::parse(xml).unwrap();
        assert_eq!(token.username, "admin");
        assert_eq!(token.password_digest, "YWJjZGVm");
        assert_eq!(token.nonce, "MTIzNDU2");
        assert_eq!(token.created, "2025-01-27T10:00:00Z");
    }

    #[test]
    fn test_parse_token_missing_username() {
        let xml = r#"
            <wsse:UsernameToken>
                <wsse:Password>digest</wsse:Password>
                <wsse:Nonce>nonce</wsse:Nonce>
                <wsu:Created>2025-01-27T10:00:00Z</wsu:Created>
            </wsse:UsernameToken>
        "#;

        let result = UsernameToken::parse(xml);
        assert_eq!(result, Err(AuthError::MissingUsername));
    }

    #[test]
    fn test_parse_token_missing_password() {
        let xml = r#"
            <wsse:UsernameToken>
                <wsse:Username>admin</wsse:Username>
                <wsse:Nonce>nonce</wsse:Nonce>
                <wsu:Created>2025-01-27T10:00:00Z</wsu:Created>
            </wsse:UsernameToken>
        "#;

        let result = UsernameToken::parse(xml);
        assert!(matches!(result, Err(AuthError::MalformedToken(_))));
    }

    #[test]
    fn test_parse_token_missing_nonce() {
        let xml = r#"
            <wsse:UsernameToken>
                <wsse:Username>admin</wsse:Username>
                <wsse:Password>digest</wsse:Password>
                <wsu:Created>2025-01-27T10:00:00Z</wsu:Created>
            </wsse:UsernameToken>
        "#;

        let result = UsernameToken::parse(xml);
        assert!(matches!(result, Err(AuthError::MalformedToken(_))));
    }

    #[test]
    fn test_parse_token_missing_created() {
        let xml = r#"
            <wsse:UsernameToken>
                <wsse:Username>admin</wsse:Username>
                <wsse:Password>digest</wsse:Password>
                <wsse:Nonce>nonce</wsse:Nonce>
            </wsse:UsernameToken>
        "#;

        let result = UsernameToken::parse(xml);
        assert!(matches!(result, Err(AuthError::MalformedToken(_))));
    }

    #[test]
    fn test_parse_token_empty_username() {
        let xml = r#"
            <wsse:UsernameToken>
                <wsse:Username></wsse:Username>
                <wsse:Password>digest</wsse:Password>
                <wsse:Nonce>nonce</wsse:Nonce>
                <wsu:Created>2025-01-27T10:00:00Z</wsu:Created>
            </wsse:UsernameToken>
        "#;

        let token = UsernameToken::parse(xml).unwrap();
        // Empty username is technically parsed (validation is separate)
        assert_eq!(token.username, "");
    }

    #[test]
    fn test_extract_element_with_namespace() {
        let xml = r#"<wsse:Username>admin</wsse:Username>"#;
        assert_eq!(extract_element(xml, "Username"), Some("admin".to_string()));
    }

    #[test]
    fn test_extract_element_without_namespace() {
        let xml = r#"<Username>admin</Username>"#;
        assert_eq!(extract_element(xml, "Username"), Some("admin".to_string()));
    }

    #[test]
    fn test_default_validator() {
        let validator = WsSecurityValidator::default();
        assert_eq!(validator.max_timestamp_age(), DEFAULT_MAX_TIMESTAMP_AGE);
    }
}
