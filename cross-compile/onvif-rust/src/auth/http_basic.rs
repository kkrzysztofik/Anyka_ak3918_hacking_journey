//! HTTP Basic Authentication using axum-extra's TypedHeader.
//!
//! This module provides HTTP Basic authentication support for ONVIF services
//! using the standard `Authorization: Basic <base64>` header.
//!
//! # Usage
//!
//! ```ignore
//! use axum::{routing::get, Router};
//! use onvif_rust::auth::BasicAuth;
//!
//! async fn protected_handler(auth: BasicAuth) -> String {
//!     format!("Hello, {}!", auth.username())
//! }
//!
//! let app = Router::new()
//!     .route("/protected", get(protected_handler));
//! ```
//!
//! # Security Note
//!
//! HTTP Basic auth transmits credentials in base64 (NOT encrypted).
//! Always use HTTPS in production to protect credentials in transit.

use axum::{
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
    response::{IntoResponse, Response},
};
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Basic},
};

/// HTTP Basic authentication extractor.
///
/// Extracts and decodes the `Authorization: Basic <credentials>` header.
/// Returns a 401 Unauthorized response if the header is missing or invalid.
///
/// # Example
///
/// ```ignore
/// async fn handler(BasicAuth(credentials): BasicAuth) -> impl IntoResponse {
///     // credentials.username() and credentials.password()
///     format!("Welcome, {}", credentials.username())
/// }
/// ```
#[derive(Debug, Clone)]
pub struct BasicAuth(pub Basic);

impl BasicAuth {
    /// Get the username from the Basic auth credentials.
    pub fn username(&self) -> &str {
        self.0.username()
    }

    /// Get the password from the Basic auth credentials.
    pub fn password(&self) -> &str {
        self.0.password()
    }

    /// Validate credentials against a username/password pair.
    ///
    /// Uses constant-time comparison to prevent timing attacks.
    pub fn validate(&self, expected_username: &str, expected_password: &str) -> bool {
        use constant_time_eq::constant_time_eq;

        let username_match =
            constant_time_eq(self.username().as_bytes(), expected_username.as_bytes());
        let password_match =
            constant_time_eq(self.password().as_bytes(), expected_password.as_bytes());

        username_match && password_match
    }
}

/// Error returned when Basic auth extraction fails.
#[derive(Debug)]
pub enum BasicAuthError {
    /// Missing Authorization header.
    MissingHeader,
    /// Invalid Authorization header format.
    InvalidHeader,
}

impl IntoResponse for BasicAuthError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            BasicAuthError::MissingHeader => {
                (StatusCode::UNAUTHORIZED, "Missing Authorization header")
            }
            BasicAuthError::InvalidHeader => {
                (StatusCode::BAD_REQUEST, "Invalid Authorization header")
            }
        };

        // Include WWW-Authenticate header to prompt for credentials
        (
            status,
            [("WWW-Authenticate", "Basic realm=\"ONVIF\"")],
            message,
        )
            .into_response()
    }
}

impl<S> FromRequestParts<S> for BasicAuth
where
    S: Send + Sync,
{
    type Rejection = BasicAuthError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let TypedHeader(Authorization(basic)) =
            TypedHeader::<Authorization<Basic>>::from_request_parts(parts, state)
                .await
                .map_err(|_| BasicAuthError::MissingHeader)?;

        Ok(BasicAuth(basic))
    }
}

/// Optional Basic authentication extractor.
///
/// Similar to `BasicAuth` but returns `None` instead of an error
/// when credentials are not provided.
#[derive(Debug, Clone)]
pub struct OptionalBasicAuth(pub Option<Basic>);

impl OptionalBasicAuth {
    /// Check if credentials were provided.
    pub fn is_some(&self) -> bool {
        self.0.is_some()
    }

    /// Get the credentials if provided.
    pub fn credentials(&self) -> Option<&Basic> {
        self.0.as_ref()
    }

    /// Get the username if credentials were provided.
    pub fn username(&self) -> Option<&str> {
        self.0.as_ref().map(|b| b.username())
    }

    /// Get the password if credentials were provided.
    pub fn password(&self) -> Option<&str> {
        self.0.as_ref().map(|b| b.password())
    }
}

impl<S> FromRequestParts<S> for OptionalBasicAuth
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let result = TypedHeader::<Authorization<Basic>>::from_request_parts(parts, state).await;

        Ok(OptionalBasicAuth(
            result.ok().map(|TypedHeader(Authorization(basic))| basic),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use base64::Engine;

    #[tokio::test]
    async fn test_basic_auth_extraction() {
        // Base64 of "admin:password123"
        let credentials = base64::engine::general_purpose::STANDARD.encode("admin:password123");

        let request = Request::builder()
            .header("Authorization", format!("Basic {}", credentials))
            .body(Body::empty())
            .unwrap();

        let (mut parts, _body) = request.into_parts();

        let auth = BasicAuth::from_request_parts(&mut parts, &())
            .await
            .unwrap();

        assert_eq!(auth.username(), "admin");
        assert_eq!(auth.password(), "password123");
    }

    #[tokio::test]
    async fn test_basic_auth_missing_header() {
        let request = Request::builder().body(Body::empty()).unwrap();

        let (mut parts, _body) = request.into_parts();

        let result = BasicAuth::from_request_parts(&mut parts, &()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_basic_auth_validate() {
        // Create BasicAuth by extracting from a request with known credentials
        let credentials = base64::engine::general_purpose::STANDARD.encode("admin:secret");

        let request = Request::builder()
            .header("Authorization", format!("Basic {}", credentials))
            .body(Body::empty())
            .unwrap();

        let (mut parts, _body) = request.into_parts();

        let auth = BasicAuth::from_request_parts(&mut parts, &())
            .await
            .unwrap();

        assert!(auth.validate("admin", "secret"));
        assert!(!auth.validate("admin", "wrong"));
        assert!(!auth.validate("other", "secret"));
    }

    #[tokio::test]
    async fn test_optional_basic_auth_with_credentials() {
        let credentials = base64::engine::general_purpose::STANDARD.encode("user:pass");

        let request = Request::builder()
            .header("Authorization", format!("Basic {}", credentials))
            .body(Body::empty())
            .unwrap();

        let (mut parts, _body) = request.into_parts();

        let auth = OptionalBasicAuth::from_request_parts(&mut parts, &())
            .await
            .unwrap();

        assert!(auth.is_some());
        assert_eq!(auth.username(), Some("user"));
        assert_eq!(auth.password(), Some("pass"));
    }

    #[tokio::test]
    async fn test_optional_basic_auth_without_credentials() {
        let request = Request::builder().body(Body::empty()).unwrap();

        let (mut parts, _body) = request.into_parts();

        let auth = OptionalBasicAuth::from_request_parts(&mut parts, &())
            .await
            .unwrap();

        assert!(!auth.is_some());
        assert_eq!(auth.username(), None);
    }

    #[tokio::test]
    async fn test_invalid_base64_credentials() {
        let request = Request::builder()
            .header("Authorization", "Basic not_valid_base64!!!")
            .body(Body::empty())
            .unwrap();

        let (mut parts, _body) = request.into_parts();

        // Should fail to parse invalid base64
        let result = BasicAuth::from_request_parts(&mut parts, &()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_missing_password_in_credentials() {
        // Base64 of "usernameonly" (no colon separator)
        let credentials = base64::engine::general_purpose::STANDARD.encode("usernameonly");

        let request = Request::builder()
            .header("Authorization", format!("Basic {}", credentials))
            .body(Body::empty())
            .unwrap();

        let (mut parts, _body) = request.into_parts();

        // The headers crate handles this - password will be empty string
        let auth = BasicAuth::from_request_parts(&mut parts, &()).await;
        // This may or may not succeed depending on how headers crate handles it
        // The important thing is it doesn't panic
        let _ = auth;
    }

    #[tokio::test]
    async fn test_empty_password() {
        // Base64 of "admin:" (empty password)
        let credentials = base64::engine::general_purpose::STANDARD.encode("admin:");

        let request = Request::builder()
            .header("Authorization", format!("Basic {}", credentials))
            .body(Body::empty())
            .unwrap();

        let (mut parts, _body) = request.into_parts();

        let auth = BasicAuth::from_request_parts(&mut parts, &())
            .await
            .unwrap();

        assert_eq!(auth.username(), "admin");
        assert_eq!(auth.password(), "");
    }

    #[tokio::test]
    async fn test_unicode_credentials() {
        // Base64 of "用户:密码" (Chinese for "user:password")
        let credentials = base64::engine::general_purpose::STANDARD.encode("用户:密码");

        let request = Request::builder()
            .header("Authorization", format!("Basic {}", credentials))
            .body(Body::empty())
            .unwrap();

        let (mut parts, _body) = request.into_parts();

        let auth = BasicAuth::from_request_parts(&mut parts, &())
            .await
            .unwrap();

        assert_eq!(auth.username(), "用户");
        assert_eq!(auth.password(), "密码");
    }

    #[tokio::test]
    async fn test_password_with_colon() {
        // Password containing colons: "admin:pass:word:123"
        let credentials = base64::engine::general_purpose::STANDARD.encode("admin:pass:word:123");

        let request = Request::builder()
            .header("Authorization", format!("Basic {}", credentials))
            .body(Body::empty())
            .unwrap();

        let (mut parts, _body) = request.into_parts();

        let auth = BasicAuth::from_request_parts(&mut parts, &())
            .await
            .unwrap();

        assert_eq!(auth.username(), "admin");
        // Password should be everything after the first colon
        assert_eq!(auth.password(), "pass:word:123");
    }

    #[tokio::test]
    async fn test_wrong_auth_scheme() {
        let request = Request::builder()
            .header("Authorization", "Bearer some_token")
            .body(Body::empty())
            .unwrap();

        let (mut parts, _body) = request.into_parts();

        // Should fail - wrong scheme
        let result = BasicAuth::from_request_parts(&mut parts, &()).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_basic_auth_error_into_response() {
        let error = BasicAuthError::MissingHeader;
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        let error = BasicAuthError::InvalidHeader;
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_optional_basic_auth_credentials_getter() {
        let credentials = base64::engine::general_purpose::STANDARD.encode("user:pass");

        let request = Request::builder()
            .header("Authorization", format!("Basic {}", credentials))
            .body(Body::empty())
            .unwrap();

        let (mut parts, _body) = request.into_parts();

        let auth = OptionalBasicAuth::from_request_parts(&mut parts, &())
            .await
            .unwrap();

        // Test the credentials() getter
        assert!(auth.credentials().is_some());
        let creds = auth.credentials().unwrap();
        assert_eq!(creds.username(), "user");
        assert_eq!(creds.password(), "pass");
    }
}
