//! Password storage and verification.
//!
//! This module provides password management for ONVIF authentication.
//! Passwords are stored in plaintext because WS-Security UsernameToken
//! digest authentication requires the server to compute:
//!
//! ```text
//! SHA1(Nonce + Created + Password)
//! ```
//!
//! This computation requires access to the original password, making
//! one-way hashes (like Argon2) incompatible with ONVIF's authentication model.
//!
//! # Security Notes
//!
//! - Passwords are stored in plaintext (required for WS-Security)
//! - File permissions should be restricted (`chmod 600`)
//! - Consider encryption-at-rest for production deployments
//! - Timing-safe comparison is used to prevent timing attacks

use constant_time_eq::constant_time_eq;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use zeroize::Zeroizing;

// ============================================================================
// PasswordError
// ============================================================================

/// Errors that can occur during password operations.
#[derive(Debug, Clone, Error, PartialEq)]
pub enum PasswordError {
    /// Password is empty.
    #[error("Password cannot be empty")]
    EmptyPassword,

    /// Password is too long.
    #[error("Password too long (max {0} characters)")]
    TooLong(usize),

    /// Password contains invalid characters.
    #[error("Password contains invalid characters")]
    InvalidCharacters,
}

/// Maximum password length.
pub const MAX_PASSWORD_LENGTH: usize = 128;

// ============================================================================
// SecurePassword
// ============================================================================

/// Secure password wrapper that zeros memory on drop.
///
/// This type wraps a password string and ensures that the password is
/// securely zeroed from memory when the value is dropped. This prevents
/// passwords from lingering in memory after use.
///
/// # Security
///
/// - Memory is automatically zeroed on drop using `zeroize`
/// - Passwords are still stored in plaintext (required for WS-Security)
/// - File permissions should be restricted (`chmod 600`)
/// - Consider encryption-at-rest for production deployments
///
/// # Example
///
/// ```ignore
/// use onvif_rust::config::users::password::SecurePassword;
///
/// let password = SecurePassword::new("my_secret".to_string());
/// let bytes = password.as_bytes(); // For digest computation
/// // Password memory is automatically zeroed when `password` goes out of scope
/// ```
#[derive(Clone, PartialEq, Eq)]
pub struct SecurePassword(Zeroizing<String>);

impl SecurePassword {
    /// Create a new secure password.
    ///
    /// # Arguments
    ///
    /// * `password` - The password string to protect
    ///
    /// # Returns
    ///
    /// A new `SecurePassword` that will zero memory on drop.
    pub fn new(password: String) -> Self {
        Self(Zeroizing::new(password))
    }

    /// Get the password as a byte slice for digest computation.
    ///
    /// # Returns
    ///
    /// The password bytes for use in WS-Security digest authentication.
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    /// Get the password as a string slice.
    ///
    /// # Returns
    ///
    /// The password string for use in authentication.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Get the length of the password in bytes.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Check if the password is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl std::fmt::Debug for SecurePassword {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SecurePassword([REDACTED])")
    }
}

impl std::fmt::Display for SecurePassword {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("[REDACTED]")
    }
}

impl From<String> for SecurePassword {
    fn from(password: String) -> Self {
        Self::new(password)
    }
}

impl From<&str> for SecurePassword {
    fn from(password: &str) -> Self {
        Self::new(password.to_string())
    }
}

// Custom Serialize/Deserialize to work with TOML
impl Serialize for SecurePassword {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for SecurePassword {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(SecurePassword::new(s))
    }
}

// ============================================================================
// PasswordManager
// ============================================================================

/// Password storage and verification manager.
///
/// Stores passwords in plaintext to support WS-Security UsernameToken
/// digest authentication, which requires computing SHA1(Nonce + Created + Password).
///
/// # Example
///
/// ```ignore
/// let manager = PasswordManager::new();
///
/// // Validate password format
/// manager.validate_password("secret123")?;
///
/// // Verify password (timing-safe)
/// assert!(manager.verify_password("secret123", "secret123"));
/// assert!(!manager.verify_password("wrong", "secret123"));
/// ```
#[derive(Debug, Clone, Default)]
pub struct PasswordManager {
    // No internal state needed for plaintext storage
}

impl PasswordManager {
    /// Create a new password manager.
    pub fn new() -> Self {
        Self {}
    }

    /// Validate a password meets requirements.
    ///
    /// # Requirements
    ///
    /// - Not empty
    /// - Maximum 128 characters
    /// - Valid UTF-8 (implicit in Rust strings)
    ///
    /// # Arguments
    ///
    /// * `password` - The password to validate
    ///
    /// # Returns
    ///
    /// `Ok(())` if valid, or `PasswordError` describing the issue.
    pub fn validate_password(&self, password: &str) -> Result<(), PasswordError> {
        if password.is_empty() {
            return Err(PasswordError::EmptyPassword);
        }

        if password.len() > MAX_PASSWORD_LENGTH {
            return Err(PasswordError::TooLong(MAX_PASSWORD_LENGTH));
        }

        // Check for null bytes or other problematic characters
        if password.contains('\0') {
            return Err(PasswordError::InvalidCharacters);
        }

        Ok(())
    }

    /// Verify a password against a stored password using timing-safe comparison.
    ///
    /// # Arguments
    ///
    /// * `provided` - The password provided by the user
    /// * `stored` - The stored secure password to compare against
    ///
    /// # Returns
    ///
    /// `true` if the passwords match, `false` otherwise.
    pub fn verify_password(&self, provided: &str, stored: &SecurePassword) -> bool {
        constant_time_eq(provided.as_bytes(), stored.as_bytes())
    }

    /// Get the password for WS-Security digest computation.
    ///
    /// Since passwords are stored in plaintext, this simply returns
    /// the stored password. This method exists to make the intent clear
    /// when used in WS-Security validation.
    ///
    /// # Arguments
    ///
    /// * `stored_password` - The stored secure password
    ///
    /// # Returns
    ///
    /// The password string for digest computation.
    pub fn get_password_for_digest<'a>(&self, stored_password: &'a SecurePassword) -> &'a str {
        stored_password.as_str()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_password_valid() {
        let manager = PasswordManager::new();

        assert!(manager.validate_password("test_password").is_ok());
        assert!(manager.validate_password("a").is_ok());
        assert!(manager.validate_password(&"a".repeat(128)).is_ok());
    }

    #[test]
    fn test_validate_password_empty() {
        let manager = PasswordManager::new();

        let result = manager.validate_password("");
        assert!(matches!(result, Err(PasswordError::EmptyPassword)));
    }

    #[test]
    fn test_validate_password_too_long() {
        let manager = PasswordManager::new();

        let long_password = "a".repeat(129);
        let result = manager.validate_password(&long_password);
        assert!(matches!(result, Err(PasswordError::TooLong(_))));
    }

    #[test]
    fn test_validate_password_null_byte() {
        let manager = PasswordManager::new();

        let result = manager.validate_password("pass\0word");
        assert!(matches!(result, Err(PasswordError::InvalidCharacters)));
    }

    #[test]
    fn test_verify_password_correct() {
        let manager = PasswordManager::new();
        let stored = SecurePassword::new("correct".to_string());

        assert!(manager.verify_password("correct", &stored));
    }

    #[test]
    fn test_verify_password_wrong() {
        let manager = PasswordManager::new();
        let stored = SecurePassword::new("correct".to_string());

        assert!(!manager.verify_password("wrong", &stored));
    }

    /// Exercises [`PasswordManager::verify_password`] when the candidate string diverges from the
    /// stored value (suffix mismatch). Does **not** measure constant-time properties.
    #[test]
    fn test_verify_password_suffix_mismatch_returns_false() {
        let manager = PasswordManager::new();
        let stored = SecurePassword::new("secret".to_string());

        assert!(
            !manager.verify_password("secret!", &stored),
            "verify_password should return false for candidate {:?} vs stored prefix match",
            "secret!"
        );
    }

    #[test]
    fn test_verify_password_empty() {
        let manager = PasswordManager::new();
        let empty = SecurePassword::new("".to_string());
        let not_empty = SecurePassword::new("not_empty".to_string());

        assert!(manager.verify_password("", &empty));
        assert!(!manager.verify_password("", &not_empty));
        assert!(!manager.verify_password("not_empty", &empty));
    }

    #[test]
    fn test_verify_password_special_characters() {
        let manager = PasswordManager::new();

        let password_str = "p@ssw0rd!#$%^&*()_+-=[]{}|;':\",./<>?";
        let password = SecurePassword::new(password_str.to_string());
        assert!(manager.verify_password(password_str, &password));
    }

    #[test]
    fn test_verify_password_unicode() {
        let manager = PasswordManager::new();

        let password_str = "пароль密码🔐";
        let password = SecurePassword::new(password_str.to_string());
        assert!(manager.verify_password(password_str, &password));
    }

    #[test]
    fn test_verify_password_long() {
        let manager = PasswordManager::new();

        let password_str = "a".repeat(128);
        let password = SecurePassword::new(password_str.clone());
        assert!(manager.verify_password(&password_str, &password));
    }

    #[test]
    fn test_get_password_for_digest() {
        let manager = PasswordManager::new();

        let stored = SecurePassword::new("my_secret_password".to_string());
        assert_eq!(
            manager.get_password_for_digest(&stored),
            "my_secret_password"
        );
    }

    #[test]
    fn test_default() {
        let manager1 = PasswordManager::new();
        let manager2 = PasswordManager::default();
        let password = SecurePassword::new("test".to_string());

        // Both should work identically
        assert!(manager1.verify_password("test", &password));
        assert!(manager2.verify_password("test", &password));
    }

    // SecurePassword tests
    #[test]
    fn test_secure_password_new() {
        let password = SecurePassword::new("test123".to_string());
        assert_eq!(password.as_str(), "test123");
        assert_eq!(password.len(), 7);
        assert!(!password.is_empty());
    }

    #[test]
    fn test_secure_password_empty() {
        let password = SecurePassword::new("".to_string());
        assert!(password.is_empty());
        assert_eq!(password.len(), 0);
    }

    #[test]
    fn test_secure_password_from_string() {
        let password: SecurePassword = "test".to_string().into();
        assert_eq!(password.as_str(), "test");
    }

    #[test]
    fn test_secure_password_from_str() {
        let password: SecurePassword = "test".into();
        assert_eq!(password.as_str(), "test");
    }

    #[test]
    fn test_secure_password_debug_redacted() {
        let password = SecurePassword::new("secret123".to_string());
        let debug_str = format!("{:?}", password);
        assert!(debug_str.contains("REDACTED"));
        assert!(!debug_str.contains("secret123"));
    }

    #[test]
    fn test_secure_password_display_redacted() {
        let password = SecurePassword::new("secret123".to_string());
        let display_str = format!("{}", password);
        assert_eq!(display_str, "[REDACTED]");
        assert!(!display_str.contains("secret123"));
    }

    #[test]
    fn test_secure_password_clone() {
        let password1 = SecurePassword::new("test".to_string());
        let password2 = password1.clone();
        assert_eq!(password1, password2);
        assert_eq!(password1.as_str(), password2.as_str());
    }

    #[test]
    fn test_secure_password_serde() {
        use serde_json;

        let password = SecurePassword::new("test123".to_string());

        // Serialize
        let json = serde_json::to_string(&password).unwrap();
        assert_eq!(json, "\"test123\"");

        // Deserialize
        let deserialized: SecurePassword = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.as_str(), "test123");
    }
}
