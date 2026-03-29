//! User management type mappings and conversions.
//!
//! This module provides conversions between:
//! - Internal user storage types (`crate::config::UserLevel`, `crate::config::UserAccount`)
//! - ONVIF WSDL types (`crate::onvif::types::common::User`, `crate::onvif::types::common::UserLevel`)
//!
//! The WSDL types are defined in `devicemgmt.wsdl` and match the ONVIF specification.

use crate::config::{UserAccount, UserLevel as InternalUserLevel};
use crate::onvif::common::{MAX_PASSWORD_CHARS, MAX_USERNAME_CHARS};
use crate::onvif::types::common::{User as OnvifUser, UserLevel as OnvifUserLevel};
use crate::utils::validation::normalize_unicode;
use unicode_segmentation::UnicodeSegmentation;

// ============================================================================
// UserLevel Conversions
// ============================================================================

impl From<InternalUserLevel> for OnvifUserLevel {
    fn from(level: InternalUserLevel) -> Self {
        match level {
            InternalUserLevel::Administrator => OnvifUserLevel::Administrator,
            InternalUserLevel::Operator => OnvifUserLevel::Operator,
            InternalUserLevel::User => OnvifUserLevel::User,
        }
    }
}

impl From<OnvifUserLevel> for InternalUserLevel {
    fn from(level: OnvifUserLevel) -> Self {
        match level {
            OnvifUserLevel::Administrator => InternalUserLevel::Administrator,
            OnvifUserLevel::Operator => InternalUserLevel::Operator,
            OnvifUserLevel::User => InternalUserLevel::User,
            // Map Anonymous and Extended to User level
            OnvifUserLevel::Anonymous | OnvifUserLevel::Extended => InternalUserLevel::User,
        }
    }
}

// ============================================================================
// User Account Conversions
// ============================================================================

/// Convert internal UserAccount to ONVIF User (for GetUsers response).
///
/// Note: Password is never returned in GetUsers response per ONVIF spec.
impl From<&UserAccount> for OnvifUser {
    fn from(account: &UserAccount) -> Self {
        OnvifUser {
            username: account.username.clone(),
            password: None, // Never return password
            user_level: account.level.into(),
            extension: None,
        }
    }
}

impl From<UserAccount> for OnvifUser {
    fn from(account: UserAccount) -> Self {
        OnvifUser::from(&account)
    }
}

// ============================================================================
// Validation
// ============================================================================

/// Validation error for user management requests.
#[derive(Debug, Clone, thiserror::Error)]
pub enum UserValidationError {
    /// Username is empty.
    #[error("Username cannot be empty")]
    EmptyUsername,

    /// Username is too short.
    #[error("Username too short (minimum 3 characters)")]
    UsernameTooShort,

    /// Username is too long.
    #[error("Username too long (max 64 characters)")]
    UsernameTooLong,

    /// Password is required but not provided.
    #[error("Password is required")]
    PasswordRequired,

    /// Password is too short.
    #[error("Password too short (minimum 8 characters)")]
    PasswordTooShort,

    /// Password does not meet complexity requirements.
    #[error("Password must contain at least one letter, one number, or one special character")]
    PasswordTooWeak,

    /// Password is too long.
    #[error("Password too long (maximum 64 characters)")]
    PasswordTooLong,

    /// Invalid characters in username.
    #[error("Invalid characters in username")]
    InvalidUsernameChars,
}

/// Validate a username.
///
/// Requirements:
/// - 3 to 64 user-perceived grapheme clusters ([`MAX_USERNAME_CHARS`](crate::onvif::common::MAX_USERNAME_CHARS))
/// - Alphanumeric and underscore only (no hyphens or dots)
pub fn validate_username(username: &str) -> Result<(), UserValidationError> {
    // Normalize Unicode to prevent variant-based bypasses
    let normalized = normalize_unicode(username);

    if normalized.is_empty() {
        return Err(UserValidationError::EmptyUsername);
    }
    let char_count = normalized.graphemes(true).count();
    if char_count < 3 {
        return Err(UserValidationError::UsernameTooShort);
    }
    if char_count > MAX_USERNAME_CHARS {
        return Err(UserValidationError::UsernameTooLong);
    }
    // Allow alphanumeric and underscore only (removed hyphen and dot)
    if !normalized.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err(UserValidationError::InvalidUsernameChars);
    }
    Ok(())
}

/// Validate a password.
///
/// Requirements:
/// - Minimum 8 Unicode scalar values (if required)
/// - Maximum [`MAX_PASSWORD_CHARS`](crate::onvif::common::MAX_PASSWORD_CHARS) scalar values (distinct from username grapheme limit)
/// - Must contain at least one letter, one number, or one special character
pub fn validate_password(
    password: Option<&str>,
    required: bool,
) -> Result<(), UserValidationError> {
    match password {
        Some(pwd) => {
            let char_count = pwd.chars().count();
            if char_count < 8 {
                return Err(UserValidationError::PasswordTooShort);
            }
            if char_count > MAX_PASSWORD_CHARS {
                return Err(UserValidationError::PasswordTooLong);
            }
            // Complexity requirement: at least one letter, one number, or one special char
            let has_letter = pwd.chars().any(|c| c.is_alphabetic());
            let has_number = pwd.chars().any(|c| c.is_ascii_digit());
            let has_special = pwd.chars().any(|c| {
                c.is_ascii_punctuation()
                    || c.is_ascii_graphic() && !c.is_alphanumeric() && !c.is_whitespace()
            });

            if !has_letter && !has_number && !has_special {
                return Err(UserValidationError::PasswordTooWeak);
            }
            Ok(())
        }
        None if required => Err(UserValidationError::PasswordRequired),
        None => Ok(()),
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::onvif::common::{MAX_PASSWORD_CHARS, MAX_USERNAME_CHARS};
    use unicode_segmentation::UnicodeSegmentation;

    #[test]
    fn test_user_level_conversion_to_onvif() {
        assert_eq!(
            OnvifUserLevel::from(InternalUserLevel::Administrator),
            OnvifUserLevel::Administrator
        );
        assert_eq!(
            OnvifUserLevel::from(InternalUserLevel::Operator),
            OnvifUserLevel::Operator
        );
        assert_eq!(
            OnvifUserLevel::from(InternalUserLevel::User),
            OnvifUserLevel::User
        );
    }

    #[test]
    fn test_user_level_conversion_from_onvif() {
        assert_eq!(
            InternalUserLevel::from(OnvifUserLevel::Administrator),
            InternalUserLevel::Administrator
        );
        assert_eq!(
            InternalUserLevel::from(OnvifUserLevel::Operator),
            InternalUserLevel::Operator
        );
        assert_eq!(
            InternalUserLevel::from(OnvifUserLevel::User),
            InternalUserLevel::User
        );
        assert_eq!(
            InternalUserLevel::from(OnvifUserLevel::Anonymous),
            InternalUserLevel::User
        );
        assert_eq!(
            InternalUserLevel::from(OnvifUserLevel::Extended),
            InternalUserLevel::User
        );
    }

    #[test]
    fn test_user_account_to_onvif() {
        let account = UserAccount::new("admin", "secret_hash", InternalUserLevel::Administrator);
        let onvif_user = OnvifUser::from(&account);

        assert_eq!(onvif_user.username, "admin");
        assert!(onvif_user.password.is_none()); // Password not returned
        assert_eq!(onvif_user.user_level, OnvifUserLevel::Administrator);
    }

    #[test]
    fn test_validate_username_valid() {
        assert!(validate_username("admin").is_ok());
        assert!(validate_username("user_1").is_ok());
        assert!(validate_username("usr").is_ok()); // Minimum 3 chars
        assert!(validate_username("user123").is_ok());
    }

    #[test]
    fn test_validate_username_invalid() {
        assert!(validate_username("").is_err());
        assert!(validate_username("ab").is_err()); // Too short (less than 3)
        assert!(validate_username(&"a".repeat(MAX_USERNAME_CHARS + 1)).is_err()); // Too long
        assert!(validate_username("user@domain").is_err()); // Invalid chars
        assert!(validate_username("user name").is_err()); // Invalid chars
        assert!(validate_username("user-name").is_err()); // Hyphen not allowed
        assert!(validate_username("user.name").is_err()); // Dot not allowed
    }

    #[test]
    fn test_validate_username_at_max_length_ok() {
        let u = format!("ab{}", "c".repeat(MAX_USERNAME_CHARS - 2));
        assert_eq!(u.graphemes(true).count(), MAX_USERNAME_CHARS);
        assert!(validate_username(&u).is_ok());
    }

    #[test]
    fn test_validate_username_one_over_max_rejected() {
        let u = format!("abc{}", "d".repeat(MAX_USERNAME_CHARS - 2));
        assert_eq!(u.graphemes(true).count(), MAX_USERNAME_CHARS + 1);
        assert!(validate_username(&u).is_err());
    }

    #[test]
    fn test_validate_password_valid() {
        assert!(validate_password(Some("password123"), true).is_ok()); // Has letter and number
        assert!(validate_password(Some("secret!@#"), true).is_ok()); // Has letter and special
        assert!(validate_password(Some("12345678"), true).is_ok()); // Has numbers (8+ chars)
        assert!(validate_password(Some("test1234"), false).is_ok()); // Not required but valid if provided
        assert!(validate_password(None, false).is_ok()); // Not required, None is ok
    }

    #[test]
    fn test_validate_password_invalid() {
        assert!(validate_password(None, true).is_err());
        assert!(validate_password(Some("abc"), true).is_err()); // Too short (< 8)
        assert!(validate_password(Some("short"), true).is_err()); // Too short (< 8)
        assert!(validate_password(Some(&"a".repeat(MAX_PASSWORD_CHARS + 1)), true).is_err()); // Too long
        assert!(validate_password(Some("        "), true).is_err()); // Only spaces, no complexity
    }

    #[test]
    fn test_validate_username_unicode_normalization() {
        // Unicode normalization should prevent variant-based bypasses
        // Composed and decomposed forms should normalize consistently
        let composed = "café"; // é as single character
        let decomposed = "cafe\u{0301}"; // e + combining acute accent

        // Both should validate the same way after normalization
        let result1 = validate_username(composed);
        let result2 = validate_username(decomposed);
        assert_eq!(result1.is_ok(), result2.is_ok());
    }

    #[test]
    fn test_validate_username_grapheme_vs_chars() {
        let composed = "café";
        let decomposed = "cafe\u{0301}";
        assert_eq!(composed.graphemes(true).count(), 4);
        assert_eq!(decomposed.graphemes(true).count(), 4);
        assert!(decomposed.chars().count() > composed.chars().count());
        assert!(validate_username(composed).is_ok());
        assert!(validate_username(decomposed).is_ok());

        // Multi-scalar grapheme: one user-perceived character, multiple Unicode scalars.
        let family = "👨‍👩‍👧";
        assert_eq!(family.graphemes(true).count(), 1);
        assert!(family.chars().count() > 1);

        let u = format!("ab{}", "c".repeat(MAX_USERNAME_CHARS - 2));
        assert_eq!(u.graphemes(true).count(), MAX_USERNAME_CHARS);
        assert!(validate_username(&u).is_ok());
    }
}
