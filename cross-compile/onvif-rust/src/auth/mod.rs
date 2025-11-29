//! Authentication and authorization for ONVIF services.
//!
//! This module implements multiple authentication methods:
//!
//! - **WS-Security UsernameToken** - For SOAP/ONVIF requests (required by ONVIF spec)
//! - **HTTP Digest (RFC 2617)** - For HTTP endpoints like snapshots
//! - **HTTP Basic** - Simple username/password (use with HTTPS only!)
//!
//! # Authentication Methods
//!
//! ## WS-Security UsernameToken
//!
//! Used for SOAP/ONVIF requests. Implements the digest mode with:
//! - SHA-1 hash of (Nonce + Created + Password)
//! - Timestamp validation to prevent replay attacks
//! - Timing-safe comparison to prevent timing attacks
//!
//! ## HTTP Digest (RFC 2617)
//!
//! Used for HTTP endpoints. Implements:
//! - MD5-based digest authentication
//! - Nonce generation and validation
//! - Replay protection via nonce counting
//!
//! ## HTTP Basic
//!
//! Simple authentication using `Authorization: Basic <base64>` header.
//! Uses `axum-extra::TypedHeader<Authorization<Basic>>` for extraction.
//!
//! **Security Warning**: Basic auth transmits credentials in base64 (NOT encrypted).
//! Always use HTTPS in production!
//!
//! # Example
//!
//! ```ignore
//! use onvif_rust::auth::{WsSecurityValidator, HttpDigestAuth, BasicAuth, AuthConfig};
//!
//! // Create validators
//! let ws_validator = WsSecurityValidator::new(300); // 5 minute timestamp window
//! let http_auth = HttpDigestAuth::new("ONVIF", 300);
//!
//! // Use Basic auth extractor in axum handler
//! async fn handler(auth: BasicAuth) -> String {
//!     format!("Hello, {}!", auth.username())
//! }
//!
//! // Load auth config
//! let config = AuthConfig::from_config(&runtime_config);
//!
//! // Check if auth is enabled
//! if config.should_authenticate() {
//!     // Validate credentials...
//! }
//! ```

mod credentials;
mod http_basic;
mod http_digest;
mod ws_security;

pub use credentials::{
    AuthConfig, AuthError as CredentialError, AuthState, AuthenticatedUser, UserLevel,
};
pub use http_basic::{BasicAuth, BasicAuthError, OptionalBasicAuth};
pub use http_digest::{HttpDigestAuth, HttpDigestError};
pub use ws_security::{AuthError, UsernameToken, WsSecurityValidator};
