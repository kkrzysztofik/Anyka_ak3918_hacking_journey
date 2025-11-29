//! HTTP Digest Authentication (RFC 2617).
//!
//! Implements HTTP Digest authentication for non-SOAP HTTP endpoints
//! like snapshot requests.
//!
//! # Algorithm
//!
//! The digest is computed as:
//! ```text
//! HA1 = MD5(username:realm:password)
//! HA2 = MD5(method:uri)
//! response = MD5(HA1:nonce:nc:cnonce:qop:HA2)
//! ```
//!
//! # Example
//!
//! ```
//! use onvif_rust::auth::http_digest::HttpDigestAuth;
//!
//! let auth = HttpDigestAuth::new("ONVIF", 300);
//!
//! // Generate a challenge
//! let challenge = auth.generate_challenge();
//!
//! // Later, validate the response
//! // let result = auth.validate_response(...);
//! ```

use axum::{
    body::Body,
    extract::Request,
    http::{StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use constant_time_eq::constant_time_eq;
use dashmap::DashMap;
use md5::{Digest, Md5};
use rand::Rng;
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;

use crate::users::{PasswordManager, UserStorage};

/// Default nonce validity period in seconds (5 minutes).
#[allow(dead_code)]
pub const DEFAULT_NONCE_VALIDITY: u64 = 300;

/// HTTP Digest authentication errors.
#[derive(Debug, Clone, Error, PartialEq)]
pub enum HttpDigestError {
    /// Authorization header is missing.
    #[error("Authorization header required")]
    MissingHeader,

    /// Authorization header is malformed.
    #[error("Malformed Authorization header: {0}")]
    MalformedHeader(String),

    /// Nonce is invalid or expired.
    #[error("Invalid or expired nonce")]
    InvalidNonce,

    /// Nonce count is invalid (replay attack).
    #[error("Invalid nonce count")]
    InvalidNonceCount,

    /// Digest response does not match.
    #[error("Invalid credentials")]
    InvalidDigest,

    /// Required parameter is missing.
    #[error("Missing required parameter: {0}")]
    MissingParameter(String),

    /// Unsupported qop value.
    #[error("Unsupported qop: {0}")]
    UnsupportedQop(String),

    /// Unsupported algorithm.
    #[error("Unsupported algorithm: {0}")]
    UnsupportedAlgorithm(String),
}

/// Parsed HTTP Digest authorization parameters.
#[derive(Debug, Clone)]
pub struct DigestParams {
    /// Username.
    pub username: String,
    /// Realm (must match server's realm).
    pub realm: String,
    /// Server-provided nonce.
    pub nonce: String,
    /// Request URI.
    pub uri: String,
    /// Client nonce.
    pub cnonce: Option<String>,
    /// Nonce count (hex string).
    pub nc: Option<String>,
    /// Quality of protection.
    pub qop: Option<String>,
    /// Computed digest response.
    pub response: String,
    /// Algorithm (MD5 or MD5-sess).
    pub algorithm: Option<String>,
    /// Opaque value from server.
    pub opaque: Option<String>,
}

impl DigestParams {
    /// Parse from Authorization header value.
    ///
    /// Expected format: `Digest username="admin", realm="ONVIF", ...`
    pub fn parse(header: &str) -> Result<Self, HttpDigestError> {
        // Remove "Digest " prefix
        let params_str = header
            .strip_prefix("Digest ")
            .or_else(|| header.strip_prefix("digest "))
            .ok_or_else(|| HttpDigestError::MalformedHeader("Missing Digest prefix".to_string()))?;

        let mut params = std::collections::HashMap::new();

        // Parse key="value" pairs
        for part in params_str.split(',') {
            let part = part.trim();
            if let Some((key, value)) = part.split_once('=') {
                let key = key.trim().to_lowercase();
                // Remove quotes from value
                let value = value.trim().trim_matches('"').to_string();
                params.insert(key, value);
            }
        }

        // Extract required fields
        let username = params
            .remove("username")
            .ok_or_else(|| HttpDigestError::MissingParameter("username".to_string()))?;
        let realm = params
            .remove("realm")
            .ok_or_else(|| HttpDigestError::MissingParameter("realm".to_string()))?;
        let nonce = params
            .remove("nonce")
            .ok_or_else(|| HttpDigestError::MissingParameter("nonce".to_string()))?;
        let uri = params
            .remove("uri")
            .ok_or_else(|| HttpDigestError::MissingParameter("uri".to_string()))?;
        let response = params
            .remove("response")
            .ok_or_else(|| HttpDigestError::MissingParameter("response".to_string()))?;

        // Optional fields
        let cnonce = params.remove("cnonce");
        let nc = params.remove("nc");
        let qop = params.remove("qop");
        let algorithm = params.remove("algorithm");
        let opaque = params.remove("opaque");

        Ok(DigestParams {
            username,
            realm,
            nonce,
            uri,
            cnonce,
            nc,
            qop,
            response,
            algorithm,
            opaque,
        })
    }
}

/// Stored nonce information for validation.
struct NonceEntry {
    /// When the nonce was created.
    created: Instant,
    /// Expected next nonce count (for replay protection).
    expected_nc: u32,
}

/// HTTP Digest authentication handler.
#[derive(Clone)]
pub struct HttpDigestAuth {
    /// Authentication realm.
    realm: String,
    /// Nonce validity period.
    nonce_validity: Duration,
    /// Active nonces with their creation time and nc counter.
    nonces: Arc<DashMap<String, NonceEntry>>,
    /// Opaque value (optional, for session binding).
    opaque: Option<String>,
}

impl HttpDigestAuth {
    /// Create a new HTTP Digest authenticator.
    ///
    /// # Arguments
    ///
    /// * `realm` - The authentication realm (displayed to user).
    /// * `nonce_validity_seconds` - How long nonces remain valid.
    pub fn new(realm: impl Into<String>, nonce_validity_seconds: u64) -> Self {
        Self {
            realm: realm.into(),
            nonce_validity: Duration::from_secs(nonce_validity_seconds),
            nonces: Arc::new(DashMap::new()),
            opaque: None,
        }
    }

    /// Set an opaque value for session binding.
    pub fn with_opaque(mut self, opaque: impl Into<String>) -> Self {
        self.opaque = Some(opaque.into());
        self
    }

    /// Generate a new nonce using cryptographically secure random bytes.
    ///
    /// # Returns
    ///
    /// A Base64-encoded random nonce.
    pub fn generate_nonce(&self) -> String {
        let mut rng = rand::rng();
        let bytes: [u8; 32] = rng.random();
        let nonce = BASE64.encode(bytes);

        // Store the nonce
        self.nonces.insert(
            nonce.clone(),
            NonceEntry {
                created: Instant::now(),
                expected_nc: 1,
            },
        );

        nonce
    }

    /// Generate the WWW-Authenticate challenge header value.
    ///
    /// # Returns
    ///
    /// The header value to send with a 401 response.
    pub fn generate_challenge(&self) -> String {
        let nonce = self.generate_nonce();

        let mut challenge = format!(
            r#"Digest realm="{}", nonce="{}", qop="auth", algorithm=MD5"#,
            self.realm, nonce
        );

        if let Some(ref opaque) = self.opaque {
            challenge.push_str(&format!(r#", opaque="{}""#, opaque));
        }

        challenge
    }

    /// Validate an HTTP Digest response.
    ///
    /// # Arguments
    ///
    /// * `params` - Parsed digest parameters from Authorization header
    /// * `method` - HTTP method (GET, POST, etc.)
    /// * `password` - The expected password for the user
    ///
    /// # Returns
    ///
    /// `Ok(())` if valid, or `HttpDigestError` describing the failure.
    pub fn validate_response(
        &self,
        params: &DigestParams,
        method: &str,
        password: &str,
    ) -> Result<(), HttpDigestError> {
        // Validate realm matches
        if params.realm != self.realm {
            return Err(HttpDigestError::InvalidDigest);
        }

        // Validate algorithm (only MD5 supported)
        if let Some(ref alg) = params.algorithm {
            let alg_lower = alg.to_lowercase();
            if alg_lower != "md5" && alg_lower != "md-5" {
                return Err(HttpDigestError::UnsupportedAlgorithm(alg.clone()));
            }
        }

        // Validate nonce
        self.validate_nonce(&params.nonce, params.nc.as_deref())?;

        // Compute expected response
        let expected = self.compute_response(params, method, password)?;

        // Timing-safe comparison
        if !constant_time_eq(expected.as_bytes(), params.response.as_bytes()) {
            return Err(HttpDigestError::InvalidDigest);
        }

        Ok(())
    }

    /// Validate the nonce and optionally check the nonce count.
    fn validate_nonce(&self, nonce: &str, nc: Option<&str>) -> Result<(), HttpDigestError> {
        let mut entry = self
            .nonces
            .get_mut(nonce)
            .ok_or(HttpDigestError::InvalidNonce)?;

        // Check if nonce has expired
        if entry.created.elapsed() > self.nonce_validity {
            drop(entry);
            self.nonces.remove(nonce);
            return Err(HttpDigestError::InvalidNonce);
        }

        // Check nonce count if provided
        if let Some(nc_str) = nc {
            let nc_value =
                u32::from_str_radix(nc_str, 16).map_err(|_| HttpDigestError::InvalidNonceCount)?;

            if nc_value != entry.expected_nc {
                return Err(HttpDigestError::InvalidNonceCount);
            }

            // Increment expected nc for next request
            entry.expected_nc = nc_value + 1;
        }

        Ok(())
    }

    /// Compute the expected digest response.
    fn compute_response(
        &self,
        params: &DigestParams,
        method: &str,
        password: &str,
    ) -> Result<String, HttpDigestError> {
        // HA1 = MD5(username:realm:password)
        let ha1 = md5_hex(&format!(
            "{}:{}:{}",
            params.username, params.realm, password
        ));

        // HA2 = MD5(method:uri)
        let ha2 = md5_hex(&format!("{}:{}", method, params.uri));

        // Response depends on qop
        let response = match params.qop.as_deref() {
            Some("auth") | Some("auth-int") => {
                // response = MD5(HA1:nonce:nc:cnonce:qop:HA2)
                let cnonce = params.cnonce.as_deref().ok_or_else(|| {
                    HttpDigestError::MissingParameter("cnonce (required with qop)".to_string())
                })?;
                let nc = params.nc.as_deref().ok_or_else(|| {
                    HttpDigestError::MissingParameter("nc (required with qop)".to_string())
                })?;
                let qop = params.qop.as_deref().unwrap();

                md5_hex(&format!(
                    "{}:{}:{}:{}:{}:{}",
                    ha1, params.nonce, nc, cnonce, qop, ha2
                ))
            }
            None => {
                // response = MD5(HA1:nonce:HA2)
                md5_hex(&format!("{}:{}:{}", ha1, params.nonce, ha2))
            }
            Some(qop) => {
                return Err(HttpDigestError::UnsupportedQop(qop.to_string()));
            }
        };

        Ok(response)
    }

    /// Check if a nonce is still valid (not expired).
    pub fn is_nonce_valid(&self, nonce: &str) -> bool {
        self.nonces
            .get(nonce)
            .is_some_and(|entry| entry.created.elapsed() <= self.nonce_validity)
    }

    /// Clean up expired nonces.
    ///
    /// Call this periodically to prevent memory growth.
    pub fn cleanup_expired_nonces(&self) {
        self.nonces
            .retain(|_, entry| entry.created.elapsed() <= self.nonce_validity);
    }

    /// Get the authentication realm.
    pub fn realm(&self) -> &str {
        &self.realm
    }

    /// Get the nonce validity period.
    pub fn nonce_validity(&self) -> Duration {
        self.nonce_validity
    }

    /// Parse an Authorization header and validate it.
    ///
    /// This is a convenience method combining parsing and validation.
    pub fn authenticate(
        &self,
        auth_header: &str,
        method: &str,
        password: &str,
    ) -> Result<String, HttpDigestError> {
        let params = DigestParams::parse(auth_header)?;
        self.validate_response(&params, method, password)?;
        Ok(params.username)
    }
}

impl std::fmt::Debug for HttpDigestAuth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpDigestAuth")
            .field("realm", &self.realm)
            .field("nonce_validity", &self.nonce_validity)
            .field("opaque", &self.opaque)
            .field("active_nonces", &self.nonces.len())
            .finish()
    }
}

/// Compute MD5 hash and return as lowercase hex string.
fn md5_hex(input: &str) -> String {
    let mut hasher = Md5::new();
    hasher.update(input.as_bytes());
    let result = hasher.finalize();
    hex_encode(&result)
}

/// Encode bytes as lowercase hex string.
fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// State for HTTP Digest authentication middleware.
///
/// This struct holds the dependencies needed for HTTP Digest authentication
/// and can be passed to the middleware.
#[derive(Clone)]
pub struct HttpDigestState {
    /// HTTP Digest authenticator.
    pub digest_auth: Arc<HttpDigestAuth>,
    /// User storage for looking up users.
    pub user_storage: Arc<UserStorage>,
    /// Password manager for retrieving passwords.
    pub password_manager: Arc<PasswordManager>,
    /// Whether authentication is enabled.
    pub auth_enabled: bool,
}

impl HttpDigestState {
    /// Create a new HTTP Digest state.
    pub fn new(
        digest_auth: Arc<HttpDigestAuth>,
        user_storage: Arc<UserStorage>,
        password_manager: Arc<PasswordManager>,
        auth_enabled: bool,
    ) -> Self {
        Self {
            digest_auth,
            user_storage,
            password_manager,
            auth_enabled,
        }
    }
}

/// HTTP Digest authentication middleware for axum.
///
/// This middleware checks for HTTP Digest authentication on incoming requests.
/// If authentication is enabled and the request lacks valid credentials,
/// it returns a 401 Unauthorized response with a WWW-Authenticate challenge.
///
/// # Usage
///
/// ```ignore
/// use axum::{Router, routing::get, middleware};
/// use onvif_rust::auth::http_digest::{http_digest_middleware, HttpDigestState};
///
/// let state = HttpDigestState::new(
///     Arc::new(HttpDigestAuth::new("ONVIF", 300)),
///     user_storage,
///     password_manager,
///     true,
/// );
///
/// let app = Router::new()
///     .route("/snapshot", get(handle_snapshot))
///     .layer(middleware::from_fn_with_state(state, http_digest_middleware));
/// ```
pub async fn http_digest_middleware(
    axum::extract::State(state): axum::extract::State<HttpDigestState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    // If auth is disabled, pass through
    if !state.auth_enabled {
        return next.run(request).await;
    }

    // Extract Authorization header
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok());

    match auth_header {
        Some(header) if header.starts_with("Digest ") || header.starts_with("digest ") => {
            // Parse and validate digest auth
            match validate_digest_request(&state, header, request.method().as_str()) {
                Ok(_username) => {
                    // Authentication successful, continue to handler
                    next.run(request).await
                }
                Err(e) => {
                    // Authentication failed
                    tracing::warn!("HTTP Digest auth failed: {}", e);
                    unauthorized_response(&state.digest_auth)
                }
            }
        }
        _ => {
            // No auth header or not Digest - send challenge
            tracing::debug!("No HTTP Digest credentials provided, sending challenge");
            unauthorized_response(&state.digest_auth)
        }
    }
}

/// Validate an HTTP Digest request.
fn validate_digest_request(
    state: &HttpDigestState,
    auth_header: &str,
    method: &str,
) -> Result<String, HttpDigestError> {
    // Parse the digest parameters
    let params = DigestParams::parse(auth_header)?;

    // Look up the user
    let user = state
        .user_storage
        .get_user(&params.username)
        .ok_or_else(|| HttpDigestError::InvalidDigest)?;

    // Get the password for verification
    let password = state
        .password_manager
        .get_password_for_digest(&user.password);

    // Validate the digest response
    state
        .digest_auth
        .validate_response(&params, method, password)?;

    Ok(params.username)
}

/// Create a 401 Unauthorized response with WWW-Authenticate challenge.
fn unauthorized_response(digest_auth: &HttpDigestAuth) -> Response {
    let challenge = digest_auth.generate_challenge();
    (
        StatusCode::UNAUTHORIZED,
        [(header::WWW_AUTHENTICATE, challenge)],
        "Unauthorized",
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_challenge() {
        let auth = HttpDigestAuth::new("ONVIF", 300);
        let challenge = auth.generate_challenge();

        assert!(challenge.contains("Digest"));
        assert!(challenge.contains("realm=\"ONVIF\""));
        assert!(challenge.contains("nonce=\""));
        assert!(challenge.contains("qop=\"auth\""));
        assert!(challenge.contains("algorithm=MD5"));
    }

    #[test]
    fn test_generate_challenge_with_opaque() {
        let auth = HttpDigestAuth::new("ONVIF", 300).with_opaque("session123");
        let challenge = auth.generate_challenge();

        assert!(challenge.contains("opaque=\"session123\""));
    }

    #[test]
    fn test_nonce_validity() {
        let auth = HttpDigestAuth::new("ONVIF", 300);
        let nonce = auth.generate_nonce();

        assert!(auth.is_nonce_valid(&nonce));
        assert!(!auth.is_nonce_valid("invalid_nonce"));
    }

    #[test]
    fn test_parse_digest_params() {
        let header = r#"Digest username="admin", realm="ONVIF", nonce="abc123", uri="/onvif/snapshot", response="deadbeef", qop=auth, nc=00000001, cnonce="xyz789""#;

        let params = DigestParams::parse(header).unwrap();

        assert_eq!(params.username, "admin");
        assert_eq!(params.realm, "ONVIF");
        assert_eq!(params.nonce, "abc123");
        assert_eq!(params.uri, "/onvif/snapshot");
        assert_eq!(params.response, "deadbeef");
        assert_eq!(params.qop, Some("auth".to_string()));
        assert_eq!(params.nc, Some("00000001".to_string()));
        assert_eq!(params.cnonce, Some("xyz789".to_string()));
    }

    #[test]
    fn test_parse_minimal_digest_params() {
        let header =
            r#"Digest username="admin", realm="ONVIF", nonce="abc", uri="/", response="123""#;

        let params = DigestParams::parse(header).unwrap();

        assert_eq!(params.username, "admin");
        assert!(params.qop.is_none());
        assert!(params.nc.is_none());
    }

    #[test]
    fn test_parse_missing_username() {
        let header = r#"Digest realm="ONVIF", nonce="abc", uri="/", response="123""#;

        let result = DigestParams::parse(header);
        assert!(matches!(result, Err(HttpDigestError::MissingParameter(_))));
    }

    #[test]
    fn test_validate_response_basic() {
        let auth = HttpDigestAuth::new("ONVIF", 300);
        let nonce = auth.generate_nonce();

        // Manually compute expected response for basic auth (no qop)
        let username = "admin";
        let password = "secret";
        let uri = "/onvif/snapshot";
        let method = "GET";

        let ha1 = md5_hex(&format!("{}:{}:{}", username, "ONVIF", password));
        let ha2 = md5_hex(&format!("{}:{}", method, uri));
        let expected_response = md5_hex(&format!("{}:{}:{}", ha1, nonce, ha2));

        let params = DigestParams {
            username: username.to_string(),
            realm: "ONVIF".to_string(),
            nonce,
            uri: uri.to_string(),
            cnonce: None,
            nc: None,
            qop: None,
            response: expected_response,
            algorithm: None,
            opaque: None,
        };

        let result = auth.validate_response(&params, method, password);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_response_with_qop() {
        let auth = HttpDigestAuth::new("ONVIF", 300);
        let nonce = auth.generate_nonce();

        let username = "admin";
        let password = "secret";
        let uri = "/onvif/snapshot";
        let method = "GET";
        let cnonce = "client_nonce";
        let nc = "00000001";
        let qop = "auth";

        let ha1 = md5_hex(&format!("{}:{}:{}", username, "ONVIF", password));
        let ha2 = md5_hex(&format!("{}:{}", method, uri));
        let expected_response = md5_hex(&format!(
            "{}:{}:{}:{}:{}:{}",
            ha1, nonce, nc, cnonce, qop, ha2
        ));

        let params = DigestParams {
            username: username.to_string(),
            realm: "ONVIF".to_string(),
            nonce,
            uri: uri.to_string(),
            cnonce: Some(cnonce.to_string()),
            nc: Some(nc.to_string()),
            qop: Some(qop.to_string()),
            response: expected_response,
            algorithm: None,
            opaque: None,
        };

        let result = auth.validate_response(&params, method, password);
        assert!(result.is_ok());
    }

    #[test]
    fn test_invalid_nonce() {
        let auth = HttpDigestAuth::new("ONVIF", 300);

        let params = DigestParams {
            username: "admin".to_string(),
            realm: "ONVIF".to_string(),
            nonce: "invalid_nonce".to_string(),
            uri: "/".to_string(),
            cnonce: None,
            nc: None,
            qop: None,
            response: "any".to_string(),
            algorithm: None,
            opaque: None,
        };

        let result = auth.validate_response(&params, "GET", "password");
        assert_eq!(result, Err(HttpDigestError::InvalidNonce));
    }

    #[test]
    fn test_wrong_password() {
        let auth = HttpDigestAuth::new("ONVIF", 300);
        let nonce = auth.generate_nonce();

        let ha1 = md5_hex(&format!("admin:ONVIF:correct_password"));
        let ha2 = md5_hex("GET:/");
        let response = md5_hex(&format!("{}:{}:{}", ha1, nonce, ha2));

        let params = DigestParams {
            username: "admin".to_string(),
            realm: "ONVIF".to_string(),
            nonce,
            uri: "/".to_string(),
            cnonce: None,
            nc: None,
            qop: None,
            response,
            algorithm: None,
            opaque: None,
        };

        let result = auth.validate_response(&params, "GET", "wrong_password");
        assert_eq!(result, Err(HttpDigestError::InvalidDigest));
    }

    #[test]
    fn test_nonce_count_replay_protection() {
        let auth = HttpDigestAuth::new("ONVIF", 300);
        let nonce = auth.generate_nonce();

        // First request with nc=00000001 should succeed
        let username = "admin";
        let password = "secret";
        let cnonce = "client";
        let nc1 = "00000001";

        let ha1 = md5_hex(&format!("{}:{}:{}", username, "ONVIF", password));
        let ha2 = md5_hex("GET:/");
        let response1 = md5_hex(&format!(
            "{}:{}:{}:{}:auth:{}",
            ha1, nonce, nc1, cnonce, ha2
        ));

        let params1 = DigestParams {
            username: username.to_string(),
            realm: "ONVIF".to_string(),
            nonce: nonce.clone(),
            uri: "/".to_string(),
            cnonce: Some(cnonce.to_string()),
            nc: Some(nc1.to_string()),
            qop: Some("auth".to_string()),
            response: response1,
            algorithm: None,
            opaque: None,
        };

        assert!(auth.validate_response(&params1, "GET", password).is_ok());

        // Replay with same nc should fail
        let response1_again = md5_hex(&format!(
            "{}:{}:{}:{}:auth:{}",
            ha1, nonce, nc1, cnonce, ha2
        ));
        let params_replay = DigestParams {
            username: username.to_string(),
            realm: "ONVIF".to_string(),
            nonce: nonce.clone(),
            uri: "/".to_string(),
            cnonce: Some(cnonce.to_string()),
            nc: Some(nc1.to_string()),
            qop: Some("auth".to_string()),
            response: response1_again,
            algorithm: None,
            opaque: None,
        };

        assert_eq!(
            auth.validate_response(&params_replay, "GET", password),
            Err(HttpDigestError::InvalidNonceCount)
        );
    }

    #[test]
    fn test_cleanup_expired_nonces() {
        let auth = HttpDigestAuth::new("ONVIF", 0); // Immediate expiration
        let _nonce = auth.generate_nonce();

        // Small delay to ensure expiration
        std::thread::sleep(std::time::Duration::from_millis(10));

        auth.cleanup_expired_nonces();
        assert_eq!(auth.nonces.len(), 0);
    }

    #[test]
    fn test_authenticate_convenience_method() {
        let auth = HttpDigestAuth::new("ONVIF", 300);
        let nonce = auth.generate_nonce();

        let ha1 = md5_hex("admin:ONVIF:secret");
        let ha2 = md5_hex("GET:/test");
        let response = md5_hex(&format!("{}:{}:{}", ha1, nonce, ha2));

        let header = format!(
            r#"Digest username="admin", realm="ONVIF", nonce="{}", uri="/test", response="{}""#,
            nonce, response
        );

        let result = auth.authenticate(&header, "GET", "secret");
        assert_eq!(result, Ok("admin".to_string()));
    }

    #[test]
    fn test_realm_mismatch_rejected() {
        let auth = HttpDigestAuth::new("ONVIF", 300);
        let nonce = auth.generate_nonce();

        // Compute digest with wrong realm
        let ha1 = md5_hex("admin:WrongRealm:secret");
        let ha2 = md5_hex("GET:/");
        let response = md5_hex(&format!("{}:{}:{}", ha1, nonce, ha2));

        let params = DigestParams {
            username: "admin".to_string(),
            realm: "WrongRealm".to_string(), // Different from auth's realm
            nonce,
            uri: "/".to_string(),
            cnonce: None,
            nc: None,
            qop: None,
            response,
            algorithm: None,
            opaque: None,
        };

        let result = auth.validate_response(&params, "GET", "secret");
        assert_eq!(result, Err(HttpDigestError::InvalidDigest));
    }

    #[test]
    fn test_unsupported_algorithm_rejected() {
        let auth = HttpDigestAuth::new("ONVIF", 300);
        let nonce = auth.generate_nonce();

        let params = DigestParams {
            username: "admin".to_string(),
            realm: "ONVIF".to_string(),
            nonce,
            uri: "/".to_string(),
            cnonce: None,
            nc: None,
            qop: None,
            response: "dummy".to_string(),
            algorithm: Some("SHA-256".to_string()), // Unsupported
            opaque: None,
        };

        let result = auth.validate_response(&params, "GET", "secret");
        assert!(matches!(
            result,
            Err(HttpDigestError::UnsupportedAlgorithm(_))
        ));
    }

    #[test]
    fn test_md5_algorithm_variations_accepted() {
        let auth = HttpDigestAuth::new("ONVIF", 300);
        let nonce = auth.generate_nonce();

        let ha1 = md5_hex("admin:ONVIF:secret");
        let ha2 = md5_hex("GET:/");
        let response = md5_hex(&format!("{}:{}:{}", ha1, nonce, ha2));

        // Test "MD5" (uppercase)
        let params = DigestParams {
            username: "admin".to_string(),
            realm: "ONVIF".to_string(),
            nonce: nonce.clone(),
            uri: "/".to_string(),
            cnonce: None,
            nc: None,
            qop: None,
            response: response.clone(),
            algorithm: Some("MD5".to_string()),
            opaque: None,
        };
        assert!(auth.validate_response(&params, "GET", "secret").is_ok());
    }

    #[test]
    fn test_unsupported_qop_rejected() {
        let auth = HttpDigestAuth::new("ONVIF", 300);
        let nonce = auth.generate_nonce();

        let params = DigestParams {
            username: "admin".to_string(),
            realm: "ONVIF".to_string(),
            nonce,
            uri: "/".to_string(),
            cnonce: Some("client".to_string()),
            nc: Some("00000001".to_string()),
            qop: Some("auth-custom".to_string()), // Unsupported qop
            response: "dummy".to_string(),
            algorithm: None,
            opaque: None,
        };

        let result = auth.validate_response(&params, "GET", "secret");
        assert!(matches!(result, Err(HttpDigestError::UnsupportedQop(_))));
    }

    #[test]
    fn test_qop_auth_missing_cnonce() {
        let auth = HttpDigestAuth::new("ONVIF", 300);
        let nonce = auth.generate_nonce();

        let params = DigestParams {
            username: "admin".to_string(),
            realm: "ONVIF".to_string(),
            nonce,
            uri: "/".to_string(),
            cnonce: None, // Missing cnonce with qop=auth
            nc: Some("00000001".to_string()),
            qop: Some("auth".to_string()),
            response: "dummy".to_string(),
            algorithm: None,
            opaque: None,
        };

        let result = auth.validate_response(&params, "GET", "secret");
        assert!(matches!(result, Err(HttpDigestError::MissingParameter(_))));
    }

    #[test]
    fn test_qop_auth_missing_nc() {
        let auth = HttpDigestAuth::new("ONVIF", 300);
        let nonce = auth.generate_nonce();

        let params = DigestParams {
            username: "admin".to_string(),
            realm: "ONVIF".to_string(),
            nonce,
            uri: "/".to_string(),
            cnonce: Some("client".to_string()),
            nc: None, // Missing nc with qop=auth
            qop: Some("auth".to_string()),
            response: "dummy".to_string(),
            algorithm: None,
            opaque: None,
        };

        let result = auth.validate_response(&params, "GET", "secret");
        assert!(matches!(result, Err(HttpDigestError::MissingParameter(_))));
    }

    #[test]
    fn test_concurrent_nonce_access() {
        use std::sync::Arc;
        use std::thread;

        let auth = Arc::new(HttpDigestAuth::new("ONVIF", 300));
        let mut handles = vec![];

        // Spawn multiple threads generating and validating nonces
        for _ in 0..10 {
            let auth_clone = Arc::clone(&auth);
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    let nonce = auth_clone.generate_nonce();
                    assert!(auth_clone.is_nonce_valid(&nonce));
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // Should have ~1000 nonces stored
        assert!(auth.nonces.len() >= 100);
    }

    #[test]
    fn test_getters() {
        let auth = HttpDigestAuth::new("TestRealm", 600);
        assert_eq!(auth.realm(), "TestRealm");
        assert_eq!(auth.nonce_validity(), Duration::from_secs(600));
    }
}
