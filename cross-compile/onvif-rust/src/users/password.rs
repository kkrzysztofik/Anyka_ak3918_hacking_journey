//! Password hashing and verification using Argon2id.
//!
//! This module provides secure password hashing following OWASP recommendations:
//! - Argon2id algorithm (memory-hard, resistant to GPU attacks)
//! - Per-password random salt
//! - Timing-safe comparison
//!
//! # Security Notes
//!
//! - Passwords are NEVER stored in plaintext
//! - Each password gets a unique random salt
//! - Verification uses constant-time comparison
//! - Hash format is PHC string format (portable)

use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
};
use rand::Rng;
use thiserror::Error;

// ============================================================================
// PasswordError
// ============================================================================

/// Errors that can occur during password operations.
#[derive(Debug, Clone, Error, PartialEq)]
pub enum PasswordError {
    /// Password hashing failed.
    #[error("Password hashing failed: {0}")]
    HashingFailed(String),

    /// Password verification failed.
    #[error("Password verification failed: {0}")]
    VerificationFailed(String),

    /// Invalid hash format.
    #[error("Invalid password hash format: {0}")]
    InvalidHash(String),
}

// ============================================================================
// PasswordManager
// ============================================================================

/// Secure password hashing and verification manager.
///
/// Uses Argon2id with the following parameters:
/// - Memory cost: 19456 KiB (19 MiB) - reduced for embedded systems
/// - Time cost: 2 iterations
/// - Parallelism: 1 thread
///
/// # Example
///
/// ```ignore
/// let manager = PasswordManager::new();
///
/// // Hash a password
/// let hash = manager.hash_password("secret123")?;
///
/// // Verify the password
/// assert!(manager.verify_password("secret123", &hash)?);
/// assert!(!manager.verify_password("wrong", &hash)?);
/// ```
#[derive(Debug, Clone)]
pub struct PasswordManager {
    /// Argon2 hasher instance.
    argon2: Argon2<'static>,
}

impl PasswordManager {
    /// Create a new password manager with default settings.
    ///
    /// Uses Argon2id with parameters suitable for embedded systems:
    /// - Memory: 19 MiB (reduced from 64 MiB default)
    /// - Iterations: 2
    /// - Parallelism: 1
    pub fn new() -> Self {
        // Use default Argon2id parameters (suitable for most cases)
        // For embedded systems, we could customize with:
        // Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        Self {
            argon2: Argon2::default(),
        }
    }

    /// Hash a password using Argon2id.
    ///
    /// Generates a random salt and returns the hash in PHC string format.
    /// The returned string contains all information needed for verification.
    ///
    /// # Arguments
    ///
    /// * `password` - The plaintext password to hash
    ///
    /// # Returns
    ///
    /// The password hash in PHC format: `$argon2id$v=19$m=19456,t=2,p=1$salt$hash`
    ///
    /// # Errors
    ///
    /// Returns an error if hashing fails (rare, usually system issues).
    pub fn hash_password(&self, password: &str) -> Result<String, PasswordError> {
        // Generate a random salt (16 bytes = 128 bits, standard for Argon2)
        let mut salt_bytes = [0u8; 16];
        rand::rng().fill(&mut salt_bytes);

        let salt = SaltString::encode_b64(&salt_bytes)
            .map_err(|e| PasswordError::HashingFailed(e.to_string()))?;

        // Hash the password
        let hash = self
            .argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| PasswordError::HashingFailed(e.to_string()))?;

        Ok(hash.to_string())
    }

    /// Verify a password against a stored hash.
    ///
    /// Uses timing-safe comparison to prevent timing attacks.
    ///
    /// # Arguments
    ///
    /// * `password` - The plaintext password to verify
    /// * `hash` - The stored password hash in PHC format
    ///
    /// # Returns
    ///
    /// `true` if the password matches, `false` otherwise.
    ///
    /// # Errors
    ///
    /// Returns an error if the hash format is invalid.
    pub fn verify_password(&self, password: &str, hash: &str) -> Result<bool, PasswordError> {
        // Parse the stored hash
        let parsed_hash =
            PasswordHash::new(hash).map_err(|e| PasswordError::InvalidHash(e.to_string()))?;

        // Verify using timing-safe comparison
        match self
            .argon2
            .verify_password(password.as_bytes(), &parsed_hash)
        {
            Ok(()) => Ok(true),
            Err(argon2::password_hash::Error::Password) => Ok(false),
            Err(e) => Err(PasswordError::VerificationFailed(e.to_string())),
        }
    }

    /// Check if a string looks like a valid Argon2 hash.
    ///
    /// This doesn't verify the hash is correct, just that it's properly formatted.
    pub fn is_valid_hash_format(hash: &str) -> bool {
        // Argon2 hashes start with $argon2
        if !hash.starts_with("$argon2") {
            return false;
        }

        // Try to parse it
        PasswordHash::new(hash).is_ok()
    }
}

impl Default for PasswordManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_password() {
        let manager = PasswordManager::new();

        let hash = manager.hash_password("test_password").unwrap();

        // Hash should be in PHC format
        assert!(hash.starts_with("$argon2"));
        assert!(hash.contains("$v="));
        assert!(hash.contains("$m="));
    }

    #[test]
    fn test_different_passwords_different_hashes() {
        let manager = PasswordManager::new();

        let hash1 = manager.hash_password("password1").unwrap();
        let hash2 = manager.hash_password("password2").unwrap();

        // Different passwords should produce different hashes
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_same_password_different_salts() {
        let manager = PasswordManager::new();

        let hash1 = manager.hash_password("same_password").unwrap();
        let hash2 = manager.hash_password("same_password").unwrap();

        // Same password should produce different hashes (different salts)
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_verify_correct_password() {
        let manager = PasswordManager::new();

        let password = "correct_password";
        let hash = manager.hash_password(password).unwrap();

        assert!(manager.verify_password(password, &hash).unwrap());
    }

    #[test]
    fn test_verify_wrong_password() {
        let manager = PasswordManager::new();

        let hash = manager.hash_password("correct").unwrap();

        assert!(!manager.verify_password("wrong", &hash).unwrap());
    }

    #[test]
    fn test_verify_empty_password() {
        let manager = PasswordManager::new();

        let hash = manager.hash_password("").unwrap();

        assert!(manager.verify_password("", &hash).unwrap());
        assert!(!manager.verify_password("not_empty", &hash).unwrap());
    }

    #[test]
    fn test_verify_invalid_hash_format() {
        let manager = PasswordManager::new();

        let result = manager.verify_password("password", "not_a_valid_hash");

        assert!(matches!(result, Err(PasswordError::InvalidHash(_))));
    }

    #[test]
    fn test_is_valid_hash_format() {
        let manager = PasswordManager::new();
        let hash = manager.hash_password("test").unwrap();

        assert!(PasswordManager::is_valid_hash_format(&hash));
        assert!(!PasswordManager::is_valid_hash_format("not_a_hash"));
        assert!(!PasswordManager::is_valid_hash_format("plaintext_password"));
        assert!(!PasswordManager::is_valid_hash_format(""));
    }

    #[test]
    fn test_hash_special_characters() {
        let manager = PasswordManager::new();

        let password = "p@ssw0rd!#$%^&*()_+-=[]{}|;':\",./<>?";
        let hash = manager.hash_password(password).unwrap();

        assert!(manager.verify_password(password, &hash).unwrap());
    }

    #[test]
    fn test_hash_unicode_password() {
        let manager = PasswordManager::new();

        let password = "пароль密码🔐";
        let hash = manager.hash_password(password).unwrap();

        assert!(manager.verify_password(password, &hash).unwrap());
    }

    #[test]
    fn test_hash_long_password() {
        let manager = PasswordManager::new();

        // 256 character password
        let password = "a".repeat(256);
        let hash = manager.hash_password(&password).unwrap();

        assert!(manager.verify_password(&password, &hash).unwrap());
    }

    #[test]
    fn test_default_creates_same_as_new() {
        let manager1 = PasswordManager::new();
        let manager2 = PasswordManager::default();

        // Both should be able to verify each other's hashes
        let hash1 = manager1.hash_password("test").unwrap();
        assert!(manager2.verify_password("test", &hash1).unwrap());
    }
}
