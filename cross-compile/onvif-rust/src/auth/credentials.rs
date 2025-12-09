//! Authentication configuration and credential validation.
//!
//! This module handles loading authentication settings from the configuration
//! and provides the middleware for authenticating requests.
//!
//! # Configuration
//!
//! Authentication settings are loaded from `[server]` section:
//! ```toml
//! [server]
//! auth_enabled = true
//! ```
//!
//! # Auth Bypass (FR-048a)
//!
//! When `auth_enabled = false`, all authentication checks are skipped.
//! This is useful for development and testing environments.

use std::sync::Arc;

use thiserror::Error;

use crate::config::ConfigRuntime;

/// Authentication configuration errors.
#[derive(Debug, Clone, Error, PartialEq)]
pub enum AuthError {
    /// Configuration error.
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// No credentials provided.
    #[error("Authentication required")]
    AuthRequired,

    /// Invalid credentials.
    #[error("Invalid credentials")]
    InvalidCredentials,
}

/// Authentication configuration loaded from runtime config.
#[derive(Debug, Clone)]
pub struct AuthConfig {
    /// Whether authentication is enabled.
    enabled: bool,
}

impl AuthConfig {
    /// Create a new AuthConfig with explicit settings.
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    /// Load authentication configuration from runtime config.
    ///
    /// Reads `[server] auth_enabled` from the configuration.
    /// Defaults to `true` if not specified.
    pub fn from_config(config: &ConfigRuntime) -> Self {
        let enabled = config.get_bool("server.auth_enabled").unwrap_or(true);

        Self { enabled }
    }

    /// Check if authentication should be performed.
    ///
    /// # Returns
    ///
    /// `true` if authentication is enabled and should be checked,
    /// `false` if authentication is disabled (bypass mode).
    pub fn should_authenticate(&self) -> bool {
        self.enabled
    }

    /// Check if authentication is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

/// Authentication state for axum middleware.
///
/// This can be used with axum's Extension layer to provide
/// authentication state to handlers.
#[derive(Clone)]
pub struct AuthState {
    /// The authentication configuration.
    pub config: AuthConfig,
    /// The user storage (for credential lookup).
    /// Will be populated when user storage is implemented.
    _users: Option<Arc<()>>, // Placeholder for UserStorage
}

impl AuthState {
    /// Create a new authentication state.
    pub fn new(config: AuthConfig) -> Self {
        Self {
            config,
            _users: None,
        }
    }

    /// Check if a request should be authenticated.
    ///
    /// This is the main entry point for the authentication middleware.
    pub fn requires_authentication(&self) -> bool {
        self.config.should_authenticate()
    }
}

/// Represents an authenticated user.
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    /// The username.
    pub username: String,
    /// The user's role/level.
    pub level: UserLevel,
}

/// User privilege levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserLevel {
    /// Full administrative access.
    Administrator,
    /// Can operate but not configure.
    Operator,
    /// Read-only access.
    User,
}

impl AuthenticatedUser {
    /// Create a new authenticated user.
    pub fn new(username: impl Into<String>, level: UserLevel) -> Self {
        Self {
            username: username.into(),
            level,
        }
    }

    /// Check if the user has administrator privileges.
    pub fn is_admin(&self) -> bool {
        matches!(self.level, UserLevel::Administrator)
    }

    /// Check if the user has operator privileges (operator or higher).
    pub fn is_operator(&self) -> bool {
        matches!(self.level, UserLevel::Administrator | UserLevel::Operator)
    }
}

/// Extract authentication credentials from request.
///
/// This is a placeholder that will be extended to support both
/// WS-Security (SOAP) and HTTP Digest (HTTP) authentication.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum AuthCredentials {
    /// WS-Security UsernameToken from SOAP header.
    WsSecurity {
        /// Raw XML containing the UsernameToken.
        token_xml: String,
    },
    /// HTTP Digest from Authorization header.
    HttpDigest {
        /// The Authorization header value.
        header: String,
    },
    /// No credentials provided.
    None,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_config_default_enabled() {
        let config = AuthConfig::default();
        assert!(config.is_enabled());
        assert!(config.should_authenticate());
    }

    #[test]
    fn test_auth_config_disabled() {
        let config = AuthConfig::new(false);
        assert!(!config.is_enabled());
        assert!(!config.should_authenticate());
    }

    #[test]
    fn test_auth_state_requires_auth_when_enabled() {
        let config = AuthConfig::new(true);
        let state = AuthState::new(config);
        assert!(state.requires_authentication());
    }

    #[test]
    fn test_auth_state_bypass_when_disabled() {
        let config = AuthConfig::new(false);
        let state = AuthState::new(config);
        assert!(!state.requires_authentication());
    }

    #[test]
    fn test_authenticated_user_admin() {
        let user = AuthenticatedUser::new("admin", UserLevel::Administrator);
        assert!(user.is_admin());
        assert!(user.is_operator());
    }

    #[test]
    fn test_authenticated_user_operator() {
        let user = AuthenticatedUser::new("operator", UserLevel::Operator);
        assert!(!user.is_admin());
        assert!(user.is_operator());
    }

    #[test]
    fn test_authenticated_user_regular() {
        let user = AuthenticatedUser::new("viewer", UserLevel::User);
        assert!(!user.is_admin());
        assert!(!user.is_operator());
    }
}
