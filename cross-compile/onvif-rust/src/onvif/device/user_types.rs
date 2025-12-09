//! User management type mappings and conversions.
//!
//! This module provides conversions between:
//! - Internal user storage types (`crate::users::UserLevel`, `crate::users::UserAccount`)
//! - ONVIF WSDL types (`crate::onvif::types::common::User`, `crate::onvif::types::common::UserLevel`)
//!
//! The WSDL types are defined in `devicemgmt.wsdl` and match the ONVIF specification.

use crate::onvif::types::common::{User as OnvifUser, UserLevel as OnvifUserLevel};
use crate::users::{UserAccount, UserLevel as InternalUserLevel};

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

    /// Username is too long.
    #[error("Username too long (max 64 characters)")]
    UsernameTooLong,

    /// Password is required but not provided.
    #[error("Password is required")]
    PasswordRequired,

    /// Password is too short.
    #[error("Password too short (minimum 4 characters)")]
    PasswordTooShort,

    /// Password is too long.
    #[error("Password too long (maximum 64 characters)")]
    PasswordTooLong,

    /// Invalid characters in username.
    #[error("Invalid characters in username")]
    InvalidUsernameChars,
}

/// Validate a username.
pub fn validate_username(username: &str) -> Result<(), UserValidationError> {
    if username.is_empty() {
        return Err(UserValidationError::EmptyUsername);
    }
    if username.len() > 64 {
        return Err(UserValidationError::UsernameTooLong);
    }
    // Allow alphanumeric, underscore, hyphen, dot
    if !username
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.')
    {
        return Err(UserValidationError::InvalidUsernameChars);
    }
    Ok(())
}

/// Validate a password.
pub fn validate_password(
    password: Option<&str>,
    required: bool,
) -> Result<(), UserValidationError> {
    match password {
        Some(pwd) => {
            if pwd.len() < 4 {
                return Err(UserValidationError::PasswordTooShort);
            }
            if pwd.len() > 64 {
                return Err(UserValidationError::PasswordTooLong);
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
        assert!(validate_username("john.doe").is_ok());
        assert!(validate_username("user-123").is_ok());
    }

    #[test]
    fn test_validate_username_invalid() {
        assert!(validate_username("").is_err());
        assert!(validate_username(&"a".repeat(65)).is_err());
        assert!(validate_username("user@domain").is_err());
        assert!(validate_username("user name").is_err());
    }

    #[test]
    fn test_validate_password_valid() {
        assert!(validate_password(Some("secret"), true).is_ok());
        assert!(validate_password(Some("test"), false).is_ok());
        assert!(validate_password(None, false).is_ok());
    }

    #[test]
    fn test_validate_password_invalid() {
        assert!(validate_password(None, true).is_err());
        assert!(validate_password(Some("abc"), true).is_err()); // Too short
        assert!(validate_password(Some(&"a".repeat(65)), true).is_err()); // Too long
    }
}
