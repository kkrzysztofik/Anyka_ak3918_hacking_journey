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
use thiserror::Error;

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
    /// * `stored` - The stored password to compare against
    ///
    /// # Returns
    ///
    /// `true` if the passwords match, `false` otherwise.
    pub fn verify_password(&self, provided: &str, stored: &str) -> bool {
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
    /// * `stored_password` - The stored password
    ///
    /// # Returns
    ///
    /// The password for digest computation.
    pub fn get_password_for_digest<'a>(&self, stored_password: &'a str) -> &'a str {
        stored_password
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

        assert!(manager.verify_password("correct", "correct"));
    }

    #[test]
    fn test_verify_password_wrong() {
        let manager = PasswordManager::new();

        assert!(!manager.verify_password("wrong", "correct"));
    }

    #[test]
    fn test_verify_password_empty() {
        let manager = PasswordManager::new();

        assert!(manager.verify_password("", ""));
        assert!(!manager.verify_password("", "not_empty"));
        assert!(!manager.verify_password("not_empty", ""));
    }

    #[test]
    fn test_verify_password_special_characters() {
        let manager = PasswordManager::new();

        let password = "p@ssw0rd!#$%^&*()_+-=[]{}|;':\",./<>?";
        assert!(manager.verify_password(password, password));
    }

    #[test]
    fn test_verify_password_unicode() {
        let manager = PasswordManager::new();

        let password = "пароль密码🔐";
        assert!(manager.verify_password(password, password));
    }

    #[test]
    fn test_verify_password_long() {
        let manager = PasswordManager::new();

        let password = "a".repeat(128);
        assert!(manager.verify_password(&password, &password));
    }

    #[test]
    fn test_get_password_for_digest() {
        let manager = PasswordManager::new();

        let stored = "my_secret_password";
        assert_eq!(manager.get_password_for_digest(stored), stored);
    }

    #[test]
    fn test_default() {
        let manager1 = PasswordManager::new();
        let manager2 = PasswordManager::default();

        // Both should work identically
        assert!(manager1.verify_password("test", "test"));
        assert!(manager2.verify_password("test", "test"));
    }
}
