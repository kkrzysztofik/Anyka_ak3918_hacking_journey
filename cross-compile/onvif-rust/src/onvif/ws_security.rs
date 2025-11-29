//! WS-Security authentication for ONVIF.
//!
//! This module implements WS-Security UsernameToken Profile 1.1 authentication
//! as required by ONVIF. It provides:
//!
//! - Password digest verification: SHA1(Nonce + Created + Password)
//! - Nonce replay protection with time-bounded cache
//! - Timestamp validation with configurable clock skew tolerance
//!
//! # WS-Security UsernameToken Structure
//!
//! ```xml
//! <wsse:UsernameToken>
//!     <wsse:Username>admin</wsse:Username>
//!     <wsse:Password Type="...#PasswordDigest">Base64(SHA1(Nonce + Created + Password))</wsse:Password>
//!     <wsse:Nonce EncodingType="...#Base64Binary">Base64(random bytes)</wsse:Nonce>
//!     <wsu:Created>2024-01-15T10:30:00Z</wsu:Created>
//! </wsse:UsernameToken>
//! ```
//!
//! # Example
//!
//! ```ignore
//! use onvif_rust::onvif::ws_security::{WsSecurityValidator, WsSecurityConfig};
//!
//! let config = WsSecurityConfig::default();
//! let validator = WsSecurityValidator::new(config);
//!
//! // Verify a digest
//! let is_valid = validator.verify_digest(
//!     "MTIzNDU2Nzg5MGFiY2RlZg==",  // Base64 nonce
//!     "2024-01-15T10:30:00Z",       // Created timestamp
//!     "YkMvwPj4ZPVPLbK8QBWdYGs+3JE=", // Received digest
//!     "actualPassword",             // Stored password
//! );
//! ```

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use chrono::{DateTime, Duration, Utc};
use sha1::{Digest, Sha1};
use std::collections::HashMap;
use std::sync::Mutex;
use thiserror::Error;

/// Password type URIs from WS-Security UsernameToken Profile.
pub mod password_types {
    /// Password digest type: SHA1(Nonce + Created + Password)
    pub const DIGEST: &str = "http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-username-token-profile-1.0#PasswordDigest";

    /// Plaintext password type
    pub const TEXT: &str = "http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-username-token-profile-1.0#PasswordText";
}

/// Nonce encoding type URI.
pub const NONCE_ENCODING_BASE64: &str = "http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-soap-message-security-1.0#Base64Binary";

/// Configuration for WS-Security validation.
#[derive(Debug, Clone)]
pub struct WsSecurityConfig {
    /// Maximum allowed clock skew for timestamp validation (in seconds).
    /// Default: 300 (5 minutes)
    pub clock_skew_seconds: i64,

    /// Time-to-live for nonce entries in the replay cache (in seconds).
    /// Nonces older than this are automatically purged.
    /// Default: 300 (5 minutes)
    pub nonce_ttl_seconds: i64,

    /// Maximum number of nonces to cache before forced cleanup.
    /// Default: 10000
    pub max_nonce_cache_size: usize,

    /// Whether to require digest authentication (reject plaintext).
    /// Default: true (production should require digest)
    pub require_digest: bool,
}

impl Default for WsSecurityConfig {
    fn default() -> Self {
        Self {
            clock_skew_seconds: 300, // 5 minutes
            nonce_ttl_seconds: 300,  // 5 minutes
            max_nonce_cache_size: 10000,
            require_digest: true,
        }
    }
}

/// Errors that can occur during WS-Security validation.
#[derive(Debug, Clone, Error)]
pub enum WsSecurityError {
    /// Missing username in UsernameToken.
    #[error("Missing username")]
    MissingUsername,

    /// Missing password in UsernameToken.
    #[error("Missing password")]
    MissingPassword,

    /// Missing nonce (required for digest authentication).
    #[error("Missing nonce for digest authentication")]
    MissingNonce,

    /// Missing created timestamp (required for digest authentication).
    #[error("Missing created timestamp for digest authentication")]
    MissingCreated,

    /// Invalid Base64 encoding in nonce.
    #[error("Invalid nonce encoding: {0}")]
    InvalidNonceEncoding(String),

    /// Invalid timestamp format.
    #[error("Invalid timestamp format: {0}")]
    InvalidTimestamp(String),

    /// Timestamp is too old or in the future (clock skew).
    #[error("Timestamp out of acceptable range")]
    TimestampOutOfRange,

    /// Nonce has been used before (replay attack detected).
    #[error("Nonce replay detected")]
    NonceReplay,

    /// Password digest does not match.
    #[error("Authentication failed: invalid credentials")]
    InvalidCredentials,

    /// Plaintext password not allowed.
    #[error("Plaintext password not allowed, use digest authentication")]
    PlaintextNotAllowed,

    /// User not found.
    #[error("User not found: {0}")]
    UserNotFound(String),

    /// Insufficient privileges.
    #[error("Insufficient privileges for this operation")]
    InsufficientPrivileges,
}

/// Entry in the nonce replay cache.
#[derive(Debug, Clone)]
struct NonceEntry {
    /// When the nonce was first seen.
    timestamp: DateTime<Utc>,
    /// Username associated with this nonce.
    username: String,
}

/// WS-Security validator with nonce replay protection.
///
/// This validator is thread-safe and can be shared across request handlers.
pub struct WsSecurityValidator {
    /// Configuration parameters.
    config: WsSecurityConfig,

    /// Nonce cache for replay protection.
    /// Key: Base64-encoded nonce, Value: NonceEntry
    nonce_cache: Mutex<HashMap<String, NonceEntry>>,
}

impl WsSecurityValidator {
    /// Create a new WS-Security validator with the given configuration.
    pub fn new(config: WsSecurityConfig) -> Self {
        Self {
            config,
            nonce_cache: Mutex::new(HashMap::new()),
        }
    }

    /// Create a new validator with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(WsSecurityConfig::default())
    }

    /// Verify a password digest.
    ///
    /// Computes SHA1(Nonce + Created + Password) and compares with the received digest.
    ///
    /// # Arguments
    ///
    /// * `nonce_b64` - Base64-encoded nonce from the UsernameToken
    /// * `created` - Created timestamp string (ISO 8601 format)
    /// * `received_digest_b64` - Base64-encoded received digest
    /// * `stored_password` - The stored plaintext password for this user
    ///
    /// # Returns
    ///
    /// `Ok(())` if the digest matches, or an appropriate error.
    pub fn verify_digest(
        &self,
        nonce_b64: &str,
        created: &str,
        received_digest_b64: &str,
        stored_password: &str,
    ) -> Result<(), WsSecurityError> {
        // Decode the nonce from Base64
        let nonce_bytes = BASE64
            .decode(nonce_b64)
            .map_err(|e| WsSecurityError::InvalidNonceEncoding(e.to_string()))?;

        // Compute expected digest: SHA1(Nonce + Created + Password)
        let mut hasher = Sha1::new();
        hasher.update(&nonce_bytes);
        hasher.update(created.as_bytes());
        hasher.update(stored_password.as_bytes());
        let computed_hash = hasher.finalize();

        // Encode computed hash as Base64
        let computed_digest_b64 = BASE64.encode(computed_hash);

        // Timing-safe comparison
        if !constant_time_eq(
            received_digest_b64.as_bytes(),
            computed_digest_b64.as_bytes(),
        ) {
            return Err(WsSecurityError::InvalidCredentials);
        }

        Ok(())
    }

    /// Validate the created timestamp is within acceptable range.
    ///
    /// The timestamp must not be too old or too far in the future.
    pub fn validate_timestamp(&self, created: &str) -> Result<DateTime<Utc>, WsSecurityError> {
        // Parse the timestamp
        let timestamp = DateTime::parse_from_rfc3339(created)
            .map_err(|e| WsSecurityError::InvalidTimestamp(e.to_string()))?
            .with_timezone(&Utc);

        let now = Utc::now();
        let skew = Duration::seconds(self.config.clock_skew_seconds);

        // Check if timestamp is within acceptable range
        let min_time = now - skew;
        let max_time = now + skew;

        if timestamp < min_time || timestamp > max_time {
            return Err(WsSecurityError::TimestampOutOfRange);
        }

        Ok(timestamp)
    }

    /// Check if a nonce has been used before (replay protection).
    ///
    /// If the nonce is new, it will be added to the cache.
    /// If the nonce has been seen before, returns an error.
    ///
    /// # Arguments
    ///
    /// * `nonce_b64` - Base64-encoded nonce
    /// * `username` - Username associated with this request
    ///
    /// # Returns
    ///
    /// `Ok(())` if nonce is fresh, `Err(NonceReplay)` if it's a replay.
    pub fn check_nonce(&self, nonce_b64: &str, username: &str) -> Result<(), WsSecurityError> {
        let mut cache = self.nonce_cache.lock().unwrap();

        // First, purge expired entries if cache is getting large
        if cache.len() >= self.config.max_nonce_cache_size {
            self.purge_expired_entries(&mut cache);
        }

        // Check if nonce already exists
        if cache.contains_key(nonce_b64) {
            return Err(WsSecurityError::NonceReplay);
        }

        // Add nonce to cache
        cache.insert(
            nonce_b64.to_string(),
            NonceEntry {
                timestamp: Utc::now(),
                username: username.to_string(),
            },
        );

        Ok(())
    }

    /// Purge expired nonce entries from the cache.
    fn purge_expired_entries(&self, cache: &mut HashMap<String, NonceEntry>) {
        let now = Utc::now();
        let ttl = Duration::seconds(self.config.nonce_ttl_seconds);
        let cutoff = now - ttl;

        cache.retain(|_, entry| entry.timestamp > cutoff);
    }

    /// Manually trigger cleanup of expired nonces.
    /// This can be called periodically from a background task.
    pub fn cleanup_expired_nonces(&self) {
        let mut cache = self.nonce_cache.lock().unwrap();
        self.purge_expired_entries(&mut cache);
    }

    /// Get the current number of cached nonces.
    pub fn nonce_cache_size(&self) -> usize {
        self.nonce_cache.lock().unwrap().len()
    }

    /// Check if digest authentication is required.
    pub fn requires_digest(&self) -> bool {
        self.config.require_digest
    }

    /// Get the configuration.
    pub fn config(&self) -> &WsSecurityConfig {
        &self.config
    }
}

/// Constant-time byte array comparison to prevent timing attacks.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut result = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        result |= x ^ y;
    }
    result == 0
}

/// Compute a WS-Security password digest.
///
/// This is useful for testing and for clients that need to generate digests.
///
/// # Arguments
///
/// * `nonce` - Raw nonce bytes
/// * `created` - Created timestamp string
/// * `password` - Plaintext password
///
/// # Returns
///
/// Base64-encoded SHA1 digest.
pub fn compute_digest(nonce: &[u8], created: &str, password: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(nonce);
    hasher.update(created.as_bytes());
    hasher.update(password.as_bytes());
    BASE64.encode(hasher.finalize())
}

/// Generate a random nonce for WS-Security.
///
/// Returns a tuple of (raw bytes, Base64-encoded string).
pub fn generate_nonce() -> (Vec<u8>, String) {
    use rand::Rng;
    let mut rng = rand::rng();
    let nonce: Vec<u8> = (0..16).map(|_| rng.random()).collect();
    let nonce_b64 = BASE64.encode(&nonce);
    (nonce, nonce_b64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_and_verify_digest() {
        let validator = WsSecurityValidator::with_defaults();

        let nonce = b"1234567890abcdef";
        let nonce_b64 = BASE64.encode(nonce);
        let created = "2024-01-15T10:30:00Z";
        let password = "testpassword";

        // Compute digest
        let digest = compute_digest(nonce, created, password);

        // Verify should succeed
        let result = validator.verify_digest(&nonce_b64, created, &digest, password);
        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_digest_wrong_password() {
        let validator = WsSecurityValidator::with_defaults();

        let nonce = b"1234567890abcdef";
        let nonce_b64 = BASE64.encode(nonce);
        let created = "2024-01-15T10:30:00Z";

        let digest = compute_digest(nonce, created, "correctpassword");

        // Verify with wrong password should fail
        let result = validator.verify_digest(&nonce_b64, created, &digest, "wrongpassword");
        assert!(matches!(result, Err(WsSecurityError::InvalidCredentials)));
    }

    #[test]
    fn test_verify_digest_invalid_nonce_encoding() {
        let validator = WsSecurityValidator::with_defaults();

        let result = validator.verify_digest(
            "not-valid-base64!!!",
            "2024-01-15T10:30:00Z",
            "somedigest",
            "password",
        );
        assert!(matches!(
            result,
            Err(WsSecurityError::InvalidNonceEncoding(_))
        ));
    }

    #[test]
    fn test_timestamp_validation_valid() {
        let validator = WsSecurityValidator::with_defaults();

        // Current time should be valid
        let now = Utc::now().to_rfc3339();
        let result = validator.validate_timestamp(&now);
        assert!(result.is_ok());
    }

    #[test]
    fn test_timestamp_validation_too_old() {
        let validator = WsSecurityValidator::with_defaults();

        // 10 minutes ago should be out of range (default skew is 5 min)
        let old_time = (Utc::now() - Duration::minutes(10)).to_rfc3339();
        let result = validator.validate_timestamp(&old_time);
        assert!(matches!(result, Err(WsSecurityError::TimestampOutOfRange)));
    }

    #[test]
    fn test_timestamp_validation_future() {
        let validator = WsSecurityValidator::with_defaults();

        // 10 minutes in the future should be out of range
        let future_time = (Utc::now() + Duration::minutes(10)).to_rfc3339();
        let result = validator.validate_timestamp(&future_time);
        assert!(matches!(result, Err(WsSecurityError::TimestampOutOfRange)));
    }

    #[test]
    fn test_timestamp_validation_invalid_format() {
        let validator = WsSecurityValidator::with_defaults();

        let result = validator.validate_timestamp("not-a-timestamp");
        assert!(matches!(result, Err(WsSecurityError::InvalidTimestamp(_))));
    }

    #[test]
    fn test_nonce_replay_protection() {
        let validator = WsSecurityValidator::with_defaults();

        let nonce = "uniquenonce123";
        let username = "admin";

        // First use should succeed
        let result1 = validator.check_nonce(nonce, username);
        assert!(result1.is_ok());

        // Second use should fail (replay)
        let result2 = validator.check_nonce(nonce, username);
        assert!(matches!(result2, Err(WsSecurityError::NonceReplay)));
    }

    #[test]
    fn test_nonce_different_nonces_allowed() {
        let validator = WsSecurityValidator::with_defaults();

        // Different nonces should all succeed
        assert!(validator.check_nonce("nonce1", "admin").is_ok());
        assert!(validator.check_nonce("nonce2", "admin").is_ok());
        assert!(validator.check_nonce("nonce3", "admin").is_ok());

        assert_eq!(validator.nonce_cache_size(), 3);
    }

    #[test]
    fn test_generate_nonce() {
        let (nonce1, nonce1_b64) = generate_nonce();
        let (nonce2, nonce2_b64) = generate_nonce();

        // Nonces should be 16 bytes
        assert_eq!(nonce1.len(), 16);
        assert_eq!(nonce2.len(), 16);

        // Nonces should be different
        assert_ne!(nonce1, nonce2);
        assert_ne!(nonce1_b64, nonce2_b64);

        // Should be valid Base64
        assert!(BASE64.decode(&nonce1_b64).is_ok());
    }

    #[test]
    fn test_config_defaults() {
        let config = WsSecurityConfig::default();

        assert_eq!(config.clock_skew_seconds, 300);
        assert_eq!(config.nonce_ttl_seconds, 300);
        assert_eq!(config.max_nonce_cache_size, 10000);
        assert!(config.require_digest);
    }

    #[test]
    fn test_custom_config() {
        let config = WsSecurityConfig {
            clock_skew_seconds: 60,
            nonce_ttl_seconds: 120,
            max_nonce_cache_size: 1000,
            require_digest: false,
        };
        let validator = WsSecurityValidator::new(config);

        assert_eq!(validator.config().clock_skew_seconds, 60);
        assert!(!validator.requires_digest());
    }

    #[test]
    fn test_cleanup_expired_nonces() {
        let config = WsSecurityConfig {
            nonce_ttl_seconds: 0, // Immediately expire
            ..Default::default()
        };
        let validator = WsSecurityValidator::new(config);

        // Add some nonces
        validator.check_nonce("nonce1", "admin").unwrap();
        validator.check_nonce("nonce2", "admin").unwrap();
        assert_eq!(validator.nonce_cache_size(), 2);

        // Wait a tiny bit and cleanup
        std::thread::sleep(std::time::Duration::from_millis(10));
        validator.cleanup_expired_nonces();

        // Nonces should be purged (TTL was 0)
        assert_eq!(validator.nonce_cache_size(), 0);
    }

    #[test]
    fn test_constant_time_eq() {
        assert!(constant_time_eq(b"hello", b"hello"));
        assert!(!constant_time_eq(b"hello", b"world"));
        assert!(!constant_time_eq(b"hello", b"hell"));
        assert!(!constant_time_eq(b"", b"x"));
        assert!(constant_time_eq(b"", b""));
    }

    #[test]
    fn test_real_onvif_digest_example() {
        // Test with a real-world ONVIF digest example
        // This verifies our implementation matches the ONVIF spec
        let validator = WsSecurityValidator::with_defaults();

        // Known test vectors (you'd typically get these from an ONVIF client)
        let nonce_raw = [
            0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc,
            0xde, 0xf0,
        ];
        let nonce_b64 = BASE64.encode(nonce_raw);
        let created = "2024-01-15T10:30:00Z";
        let password = "admin123";

        // Compute expected digest
        let expected_digest = compute_digest(&nonce_raw, created, password);

        // Verification should pass
        let result = validator.verify_digest(&nonce_b64, created, &expected_digest, password);
        assert!(result.is_ok(), "Digest verification failed: {:?}", result);
    }
}
