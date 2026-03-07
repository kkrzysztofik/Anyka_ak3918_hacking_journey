//! Authentication module for the dispatcher.
//!
//! This module contains authentication-related methods for the ServiceDispatcher.

#![allow(unused_imports)]

use axum::{body::Body, extract::Request, http::header, response::Response};

use base64::{Engine, engine::general_purpose::STANDARD as BASE64};

use crate::config::{PasswordManager, UserAccount, UserStorage};

use crate::onvif::auth_requirements::AuthLevel;
use crate::onvif::dispatcher::ServiceDispatcher;
use crate::onvif::dispatcher::response::error_response;
use crate::onvif::dispatcher::{AuthContext, ServiceHandler};
use crate::onvif::error::OnvifError;
use crate::onvif::soap::UsernameToken;
use crate::onvif::ws_security::WsSecurityError;
use std::sync::Arc;

/// Verify HTTP Basic Authentication credentials.
pub(super) fn verify_basic_auth_self(
    _dispatcher: &ServiceDispatcher,
    request: &Request<Body>,
    auth_ctx: &AuthContext,
) -> Result<Option<UserAccount>, OnvifError> {
    let auth_header = match request.headers().get(header::AUTHORIZATION) {
        Some(h) => h,
        None => return Ok(None),
    };

    let auth_str = match auth_header.to_str() {
        Ok(s) => s,
        Err(_) => return Ok(None),
    };

    // Check scheme
    if !auth_str.starts_with("Basic ") {
        return Ok(None);
    }

    let token = &auth_str[6..];
    let decoded = match BASE64.decode(token) {
        Ok(d) => d,
        Err(_) => {
            return Err(OnvifError::NotAuthorized(
                "Invalid Base64 in Authorization header".to_string(),
            ));
        }
    };

    let credential_str = match std::str::from_utf8(&decoded) {
        Ok(s) => s,
        Err(_) => {
            return Err(OnvifError::NotAuthorized(
                "Invalid UTF-8 in Basic credentials".to_string(),
            ));
        }
    };

    let (username, password) = match credential_str.split_once(':') {
        Some(pair) => pair,
        None => {
            return Err(OnvifError::NotAuthorized(
                "Invalid Basic credentials format".to_string(),
            ));
        }
    };

    // Validate user existence
    // SECURITY: Generic error message prevents username enumeration (OWASP A07:2021)
    let user = auth_ctx.user_storage.get_user(username).ok_or_else(|| {
        tracing::warn!(
            target: "security",
            "Authentication failed: user '{}' not found (Basic Auth)",
            username
        );
        OnvifError::NotAuthorized("Invalid credentials".to_string())
    })?;

    // Validate password
    if !auth_ctx
        .password_manager
        .verify_password(password, &user.password)
    {
        return Err(OnvifError::NotAuthorized("Invalid credentials".to_string()));
    }

    Ok(Some(user))
}

/// Validate credentials for a request.
pub(super) async fn validate_credentials(
    _dispatcher: &ServiceDispatcher,
    required_level: AuthLevel,
    basic_auth_result: &Result<Option<UserAccount>, OnvifError>,
    ws_security_token: Option<&UsernameToken>,
    auth_ctx: &AuthContext,
) -> Result<(), OnvifError> {
    if !auth_ctx.auth_enabled {
        return Ok(());
    }

    // Skip authentication for anonymous operations
    if required_level == AuthLevel::Anonymous {
        return Ok(());
    }

    // Try Basic Auth first
    match basic_auth_result {
        Ok(Some(user)) => {
            if required_level.is_satisfied_by(Some(user.level)) {
                tracing::debug!("Basic Auth successful");
                return Ok(());
            }
            return Err(OnvifError::NotAuthorized(
                "Insufficient privileges".to_string(),
            ));
        }
        Ok(None) => {
            // No Basic Auth header, proceed to WS-Security
        }
        Err(e) => {
            // Basic Auth header present but invalid
            return Err(e.clone());
        }
    }

    // Fallback to WS-Security (UsernameToken)
    if let Some(token) = ws_security_token {
        authenticate(_dispatcher, Some(token), auth_ctx, required_level).await
    } else {
        Err(OnvifError::NotAuthorized(
            "Missing authentication credentials".to_string(),
        ))
    }
}

/// Check authentication for the request.
pub(super) async fn check_authentication_self(
    dispatcher: &ServiceDispatcher,
    action: &str,
    handler: &Arc<dyn ServiceHandler>,
    basic_auth_result: &Result<Option<UserAccount>, OnvifError>,
    envelope: &crate::onvif::soap::RawSoapEnvelope,
    auth_ctx: &AuthContext,
) -> Result<(), Box<Response>> {
    let required_level = handler.required_auth_level(action);

    // Extract WS-Security token
    let ws_security_token = envelope
        .header
        .as_ref()
        .and_then(|h| h.security.as_ref())
        .and_then(|s| s.username_token.as_ref());

    match validate_credentials(
        dispatcher,
        required_level,
        basic_auth_result,
        ws_security_token,
        auth_ctx,
    )
    .await
    {
        Ok(()) => {
            tracing::debug!("Authentication successful for action {}", action);
            Ok(())
        }
        Err(e) => {
            tracing::warn!("Authentication failed for action {}: {:?}", action, e);
            Err(Box::new(error_response(e)))
        }
    }
}

/// Authenticate a request using WS-Security credentials.
async fn authenticate(
    _dispatcher: &ServiceDispatcher,
    token: Option<&UsernameToken>,
    auth_ctx: &AuthContext,
    required_level: AuthLevel,
) -> Result<(), OnvifError> {
    // Get the UsernameToken or fail
    let token = token
        .ok_or_else(|| OnvifError::NotAuthorized("Missing WS-Security credentials".to_string()))?;

    let username = &token.username;
    if username.is_empty() {
        return Err(OnvifError::NotAuthorized("Empty username".to_string()));
    }

    // Look up user
    // SECURITY: Generic error message prevents username enumeration (OWASP A07:2021)
    let user = auth_ctx.user_storage.get_user(username).ok_or_else(|| {
        tracing::warn!(
            target: "security",
            "Authentication failed: user '{}' not found (WS-Security)",
            username
        );
        OnvifError::NotAuthorized("Invalid credentials".to_string())
    })?;

    // Check if user has sufficient privileges
    if !required_level.is_satisfied_by(Some(user.level)) {
        return Err(OnvifError::NotAuthorized(
            "Insufficient privileges".to_string(),
        ));
    }

    // Get password type and validate
    let password_type = token.password.password_type.as_deref().unwrap_or("");

    // Route to appropriate authentication method
    if password_type.contains("PasswordDigest") {
        authenticate_digest(_dispatcher, token, username, auth_ctx, &user)?;
    } else if password_type.contains("PasswordText") || password_type.is_empty() {
        authenticate_plaintext(_dispatcher, token, auth_ctx, &user)?;
    } else {
        return Err(OnvifError::NotAuthorized(format!(
            "Unsupported password type: {}",
            password_type
        )));
    }

    Ok(())
}

/// Authenticate using digest (PasswordDigest) method.
fn authenticate_digest(
    _dispatcher: &ServiceDispatcher,
    token: &UsernameToken,
    username: &str,
    auth_ctx: &AuthContext,
    user: &UserAccount,
) -> Result<(), OnvifError> {
    // Extract nonce
    let nonce = token
        .nonce
        .as_ref()
        .map(|n| n.value.as_str())
        .ok_or_else(|| {
            OnvifError::NotAuthorized("Missing nonce for digest authentication".to_string())
        })?;

    // Extract created timestamp
    let created = token.created.as_deref().ok_or_else(|| {
        OnvifError::NotAuthorized("Missing created timestamp for digest authentication".to_string())
    })?;

    // Validate timestamp
    auth_ctx
        .ws_security
        .validate_timestamp(created)
        .map_err(ws_error_to_onvif)?;

    // Check nonce for replay
    auth_ctx
        .ws_security
        .check_nonce(nonce, username)
        .map_err(ws_error_to_onvif)?;

    // Get stored password for digest verification
    let stored_password = auth_ctx
        .password_manager
        .get_password_for_digest(&user.password);

    // Verify digest
    auth_ctx
        .ws_security
        .verify_digest(nonce, created, &token.password.value, stored_password)
        .map_err(ws_error_to_onvif)?;

    Ok(())
}

/// Authenticate using plaintext (PasswordText) method.
fn authenticate_plaintext(
    _dispatcher: &ServiceDispatcher,
    token: &UsernameToken,
    auth_ctx: &AuthContext,
    user: &UserAccount,
) -> Result<(), OnvifError> {
    // SECURITY: Verify credentials BEFORE checking policy requirements.
    // Checking digest-required first would leak information about auth policy
    // when the user exists but the password is wrong, enabling username
    // enumeration (OWASP A07:2021). By verifying the password first, both
    // "user not found" (caught in authenticate()) and "wrong password" return
    // identical "Invalid credentials" messages.

    // Verify plaintext password
    if !auth_ctx
        .password_manager
        .verify_password(&token.password.value, &user.password)
    {
        return Err(OnvifError::NotAuthorized("Invalid credentials".to_string()));
    }

    // Check if digest is required (only after credentials are verified)
    if auth_ctx.ws_security.requires_digest() {
        return Err(OnvifError::NotAuthorized(
            "Digest authentication required".to_string(),
        ));
    }

    Ok(())
}

/// Convert a WS-Security error to an ONVIF error.
fn ws_error_to_onvif(error: WsSecurityError) -> OnvifError {
    match error {
        WsSecurityError::MissingUsername
        | WsSecurityError::MissingPassword
        | WsSecurityError::MissingNonce
        | WsSecurityError::MissingCreated => OnvifError::NotAuthorized(error.to_string()),
        WsSecurityError::InvalidNonceEncoding(msg) => {
            OnvifError::NotAuthorized(format!("Invalid nonce encoding: {}", msg))
        }
        WsSecurityError::InvalidTimestamp(msg) => {
            OnvifError::NotAuthorized(format!("Invalid timestamp: {}", msg))
        }
        WsSecurityError::TimestampOutOfRange => OnvifError::NotAuthorized(
            "Timestamp out of acceptable range (possible clock skew)".to_string(),
        ),
        WsSecurityError::NonceReplay => {
            OnvifError::NotAuthorized("Nonce replay detected".to_string())
        }
        WsSecurityError::InvalidCredentials => {
            OnvifError::NotAuthorized("Invalid credentials".to_string())
        }
        WsSecurityError::PlaintextNotAllowed => OnvifError::NotAuthorized(
            "Plaintext password not allowed, use digest authentication".to_string(),
        ),
        WsSecurityError::UserNotFound(ref user) => {
            // SECURITY: Generic error message prevents username enumeration (OWASP A07:2021)
            tracing::warn!(
                target: "security",
                "Authentication failed: user '{}' not found (WS-Security validator)",
                user
            );
            OnvifError::NotAuthorized("Invalid credentials".to_string())
        }
        WsSecurityError::InsufficientPrivileges => {
            OnvifError::NotAuthorized("Insufficient privileges for this operation".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::onvif::ws_security::WsSecurityValidator;
    use async_trait::async_trait;
    use axum::http::Request as HttpRequest;

    // ── Test fixture constants ──────────────────────────────────────────
    // These are intentionally hardcoded values used exclusively in unit
    // tests.  They are NOT production secrets and carry no security risk.
    // CodeQL rule: rust/hard-coded-cryptographic-value
    const TEST_PASSWORD: &str = "password123";
    // ────────────────────────────────────────────────────────────────────

    // Custom handler that overrides required_auth_level
    pub struct CustomAuthHandler;

    #[async_trait]
    impl ServiceHandler for CustomAuthHandler {
        async fn handle_operation(&self, _action: &str, _body: &str) -> Result<String, OnvifError> {
            Ok("<Response/>".to_string())
        }

        fn service_name(&self) -> &str {
            "custom"
        }

        fn required_auth_level(&self, action: &str) -> AuthLevel {
            match action {
                "PublicOp" => AuthLevel::Anonymous,
                "AdminOp" => AuthLevel::Administrator,
                _ => AuthLevel::User,
            }
        }
    }

    #[tokio::test]
    async fn test_dispatch_basic_auth_success() {
        // Setup auth context with enabled auth
        let user_storage = Arc::new(UserStorage::new());
        user_storage
            .create_user(
                "admin",
                TEST_PASSWORD,
                crate::config::UserLevel::Administrator,
            )
            .unwrap();

        let password_manager = Arc::new(PasswordManager::new());
        let ws_security = Arc::new(WsSecurityValidator::with_defaults());

        let auth_ctx = AuthContext::new(
            ws_security,
            user_storage,
            password_manager,
            true, // Enable auth
        );

        let dispatcher = ServiceDispatcher::new();
        dispatcher.register_service("custom", Arc::new(CustomAuthHandler));

        // Create Basic Auth header (admin:<TEST_PASSWORD>)
        let credentials =
            base64::engine::general_purpose::STANDARD.encode(format!("admin:{}", TEST_PASSWORD));

        let soap_body = r#"<?xml version="1.0"?>
            <s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
                <s:Body><AdminOp xmlns="http://www.onvif.org/ver10/device/wsdl"/></s:Body>
            </s:Envelope>"#;

        let request = HttpRequest::builder()
            .method("POST")
            .header("Content-Type", "application/soap+xml")
            .header("Authorization", format!("Basic {}", credentials))
            .body(Body::from(soap_body))
            .unwrap();

        // Dispatch to "custom" service (AdminOp requires Administrator)
        let response = dispatcher
            .dispatch_with_auth("custom", request, &auth_ctx)
            .await;

        // Should succeed
        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }

    #[tokio::test]
    async fn test_dispatch_basic_auth_invalid_password() {
        // Setup auth context
        let user_storage = Arc::new(UserStorage::new());
        user_storage
            .create_user(
                "admin",
                TEST_PASSWORD,
                crate::config::UserLevel::Administrator,
            )
            .unwrap();

        let password_manager = Arc::new(PasswordManager::new());
        let ws_security = Arc::new(WsSecurityValidator::with_defaults());
        let auth_ctx = AuthContext::new(ws_security, user_storage, password_manager, true);

        let dispatcher = ServiceDispatcher::new();
        dispatcher.register_service("custom", Arc::new(CustomAuthHandler));

        // Wrong password
        let credentials = base64::engine::general_purpose::STANDARD.encode("admin:wrongpass");

        let soap_body = r#"<?xml version="1.0"?>
            <s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
                <s:Body><AdminOp xmlns="http://www.onvif.org/ver10/device/wsdl"/></s:Body>
            </s:Envelope>"#;

        let request = HttpRequest::builder()
            .method("POST")
            .header("Content-Type", "application/soap+xml")
            .header("Authorization", format!("Basic {}", credentials))
            .body(Body::from(soap_body))
            .unwrap();

        let response = dispatcher
            .dispatch_with_auth("custom", request, &auth_ctx)
            .await;

        // Should NOT be OK
        assert_ne!(response.status(), axum::http::StatusCode::OK);
    }

    #[tokio::test]
    async fn test_dispatch_auth_disabled() {
        // Setup auth context with disabled auth
        let user_storage = Arc::new(UserStorage::new());
        let password_manager = Arc::new(PasswordManager::new());
        let ws_security = Arc::new(WsSecurityValidator::with_defaults());
        let auth_ctx = AuthContext::new(ws_security, user_storage, password_manager, false);

        let dispatcher = ServiceDispatcher::new();
        dispatcher.register_service("custom", Arc::new(CustomAuthHandler));

        // No auth header - should still work when auth is disabled
        let soap_body = r#"<?xml version="1.0"?>
            <s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
                <s:Body><AdminOp xmlns="http://www.onvif.org/ver10/device/wsdl"/></s:Body>
            </s:Envelope>"#;

        let request = HttpRequest::builder()
            .method("POST")
            .header("Content-Type", "application/soap+xml")
            .header("SOAPAction", "AdminOp")
            .body(Body::from(soap_body))
            .unwrap();

        let response = dispatcher
            .dispatch_with_auth("custom", request, &auth_ctx)
            .await;

        // Should succeed when auth is disabled
        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }

    #[tokio::test]
    async fn test_dispatch_anonymous_operation() {
        // Setup auth context with enabled auth
        let user_storage = Arc::new(UserStorage::new());
        let password_manager = Arc::new(PasswordManager::new());
        let ws_security = Arc::new(WsSecurityValidator::with_defaults());
        let auth_ctx = AuthContext::new(ws_security, user_storage, password_manager, true);

        let dispatcher = ServiceDispatcher::new();
        dispatcher.register_service("custom", Arc::new(CustomAuthHandler));

        // PublicOp requires Anonymous level - should work without auth
        let soap_body = r#"<?xml version="1.0"?>
            <s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
                <s:Body><PublicOp xmlns="http://www.onvif.org/ver10/device/wsdl"/></s:Body>
            </s:Envelope>"#;

        let request = HttpRequest::builder()
            .method("POST")
            .header("Content-Type", "application/soap+xml")
            .header("SOAPAction", "PublicOp")
            .body(Body::from(soap_body))
            .unwrap();

        let response = dispatcher
            .dispatch_with_auth("custom", request, &auth_ctx)
            .await;

        // Should succeed for anonymous operations
        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }

    #[tokio::test]
    async fn test_dispatch_basic_auth_invalid_base64() {
        let user_storage = Arc::new(UserStorage::new());
        let password_manager = Arc::new(PasswordManager::new());
        let ws_security = Arc::new(WsSecurityValidator::with_defaults());
        let auth_ctx = AuthContext::new(ws_security, user_storage, password_manager, true);

        let dispatcher = ServiceDispatcher::new();
        dispatcher.register_service("custom", Arc::new(CustomAuthHandler));

        // Invalid base64
        let request = HttpRequest::builder()
            .method("POST")
            .header("Content-Type", "application/soap+xml")
            .header("Authorization", "Basic !!!invalid!!!")
            .body(Body::from(r#"<?xml version="1.0"?><s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope"><s:Body><AdminOp/></s:Body></s:Envelope>"#))
            .unwrap();

        let response = dispatcher
            .dispatch_with_auth("custom", request, &auth_ctx)
            .await;

        assert_ne!(response.status(), axum::http::StatusCode::OK);
    }

    #[tokio::test]
    async fn test_dispatch_basic_auth_missing_colon() {
        let user_storage = Arc::new(UserStorage::new());
        let password_manager = Arc::new(PasswordManager::new());
        let ws_security = Arc::new(WsSecurityValidator::with_defaults());
        let auth_ctx = AuthContext::new(ws_security, user_storage, password_manager, true);

        let dispatcher = ServiceDispatcher::new();
        dispatcher.register_service("custom", Arc::new(CustomAuthHandler));

        // Missing colon separator
        let credentials = base64::engine::general_purpose::STANDARD.encode("adminpassword123");
        let request = HttpRequest::builder()
            .method("POST")
            .header("Content-Type", "application/soap+xml")
            .header("Authorization", format!("Basic {}", credentials))
            .body(Body::from(r#"<?xml version="1.0"?><s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope"><s:Body><AdminOp/></s:Body></s:Envelope>"#))
            .unwrap();

        let response = dispatcher
            .dispatch_with_auth("custom", request, &auth_ctx)
            .await;

        assert_ne!(response.status(), axum::http::StatusCode::OK);
    }

    #[tokio::test]
    async fn test_dispatch_basic_auth_user_not_found() {
        let user_storage = Arc::new(UserStorage::new());
        let password_manager = Arc::new(PasswordManager::new());
        let ws_security = Arc::new(WsSecurityValidator::with_defaults());
        let auth_ctx = AuthContext::new(ws_security, user_storage, password_manager, true);

        let dispatcher = ServiceDispatcher::new();
        dispatcher.register_service("custom", Arc::new(CustomAuthHandler));

        // User doesn't exist
        let credentials = base64::engine::general_purpose::STANDARD
            .encode(format!("nonexistent:{}", TEST_PASSWORD));
        let request = HttpRequest::builder()
            .method("POST")
            .header("Content-Type", "application/soap+xml")
            .header("Authorization", format!("Basic {}", credentials))
            .body(Body::from(r#"<?xml version="1.0"?><s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope"><s:Body><AdminOp/></s:Body></s:Envelope>"#))
            .unwrap();

        let response = dispatcher
            .dispatch_with_auth("custom", request, &auth_ctx)
            .await;

        assert_ne!(response.status(), axum::http::StatusCode::OK);
    }

    #[tokio::test]
    async fn test_dispatch_insufficient_privileges() {
        // Setup auth context
        let user_storage = Arc::new(UserStorage::new());
        user_storage
            .create_user(
                "operator",
                TEST_PASSWORD,
                crate::config::UserLevel::Operator,
            )
            .unwrap();

        let password_manager = Arc::new(PasswordManager::new());
        let ws_security = Arc::new(WsSecurityValidator::with_defaults());
        let auth_ctx = AuthContext::new(ws_security, user_storage, password_manager, true);

        let dispatcher = ServiceDispatcher::new();
        dispatcher.register_service("custom", Arc::new(CustomAuthHandler));

        // Operator trying to access AdminOp (requires Administrator)
        let credentials =
            base64::engine::general_purpose::STANDARD.encode(format!("operator:{}", TEST_PASSWORD));
        let request = HttpRequest::builder()
            .method("POST")
            .header("Content-Type", "application/soap+xml")
            .header("Authorization", format!("Basic {}", credentials))
            .body(Body::from(r#"<?xml version="1.0"?><s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope"><s:Body><AdminOp xmlns="http://www.onvif.org/ver10/device/wsdl"/></s:Body></s:Envelope>"#))
            .unwrap();

        let response = dispatcher
            .dispatch_with_auth("custom", request, &auth_ctx)
            .await;

        // Should fail with insufficient privileges
        assert_ne!(response.status(), axum::http::StatusCode::OK);
    }

    #[test]
    fn test_auth_context_disabled() {
        let ctx = AuthContext::disabled();
        assert!(!ctx.auth_enabled);
    }

    #[test]
    fn test_auth_context_new() {
        let user_storage = Arc::new(UserStorage::new());
        let password_manager = Arc::new(PasswordManager::new());
        let ws_security = Arc::new(WsSecurityValidator::with_defaults());
        let ctx = AuthContext::new(ws_security, user_storage, password_manager, true);
        assert!(ctx.auth_enabled);
    }

    // ==========================================
    // Security: Username enumeration prevention
    // ==========================================

    /// SECURITY TEST: verify_basic_auth_self returns identical error messages
    /// for "user not found" and "wrong password" to prevent username enumeration.
    #[test]
    fn test_security_basic_auth_no_username_enumeration() {
        let user_storage = Arc::new(UserStorage::new());
        user_storage
            .create_user(
                "admin",
                TEST_PASSWORD,
                crate::config::UserLevel::Administrator,
            )
            .unwrap();

        let password_manager = Arc::new(PasswordManager::new());
        let ws_security = Arc::new(WsSecurityValidator::with_defaults());
        let auth_ctx = AuthContext::new(ws_security, user_storage, password_manager, true);
        let dispatcher = ServiceDispatcher::new();

        // Case 1: nonexistent user
        let creds_bad_user = base64::engine::general_purpose::STANDARD
            .encode(format!("nonexistent:{}", TEST_PASSWORD));
        let req_bad_user = HttpRequest::builder()
            .method("POST")
            .header("Authorization", format!("Basic {}", creds_bad_user))
            .body(Body::empty())
            .unwrap();

        let err_bad_user =
            verify_basic_auth_self(&dispatcher, &req_bad_user, &auth_ctx).unwrap_err();

        // Case 2: valid user, wrong password
        let creds_bad_pass =
            base64::engine::general_purpose::STANDARD.encode("admin:wrongpassword");
        let req_bad_pass = HttpRequest::builder()
            .method("POST")
            .header("Authorization", format!("Basic {}", creds_bad_pass))
            .body(Body::empty())
            .unwrap();

        let err_bad_pass =
            verify_basic_auth_self(&dispatcher, &req_bad_pass, &auth_ctx).unwrap_err();

        // Both errors MUST produce identical messages (prevents username enumeration)
        let msg_bad_user = format!("{}", err_bad_user);
        let msg_bad_pass = format!("{}", err_bad_pass);
        assert_eq!(
            msg_bad_user, msg_bad_pass,
            "User-not-found and wrong-password must produce identical error messages"
        );

        // Verify neither message contains the username
        assert!(
            !msg_bad_user.contains("nonexistent"),
            "Error message must not contain the attempted username"
        );
        assert!(
            !msg_bad_pass.contains("admin"),
            "Error message must not contain the valid username"
        );

        // Verify both use the generic "Invalid credentials" message
        assert!(
            msg_bad_user.contains("Invalid credentials"),
            "Error should use generic 'Invalid credentials' message"
        );
    }

    /// SECURITY TEST: ws_error_to_onvif for UserNotFound must not leak usernames.
    #[test]
    fn test_security_ws_error_user_not_found_no_leak() {
        let error = WsSecurityError::UserNotFound("secret_admin".to_string());
        let onvif_error = ws_error_to_onvif(error);

        let msg = format!("{}", onvif_error);

        // Must not contain the username
        assert!(
            !msg.contains("secret_admin"),
            "ONVIF error must not contain the username from WsSecurityError::UserNotFound"
        );

        // Must use generic credentials message
        assert!(
            msg.contains("Invalid credentials"),
            "ONVIF error should use generic 'Invalid credentials' message"
        );
    }

    /// SECURITY TEST: SOAP fault output for auth errors must not contain usernames.
    #[test]
    fn test_security_soap_fault_no_username_leak() {
        // Simulate the error that would be generated for a nonexistent user
        let error = OnvifError::NotAuthorized("Invalid credentials".to_string());
        let fault_xml = error.to_soap_fault();

        // The SOAP fault must not contain any user-identifying information
        assert!(
            !fault_xml.contains("not found"),
            "SOAP fault must not contain 'not found' phrase"
        );
        assert!(
            fault_xml.contains("Invalid credentials"),
            "SOAP fault should contain generic 'Invalid credentials' message"
        );
        assert!(
            fault_xml.contains("NotAuthorized"),
            "SOAP fault should contain the NotAuthorized subcode"
        );
    }

    /// SECURITY TEST: WS-Security authenticate() returns identical error for
    /// nonexistent user as for wrong password (via validate_credentials path).
    #[tokio::test]
    async fn test_security_ws_auth_no_username_enumeration() {
        use crate::onvif::soap::{PasswordElement, UsernameToken};

        let user_storage = Arc::new(UserStorage::new());
        user_storage
            .create_user(
                "admin",
                TEST_PASSWORD,
                crate::config::UserLevel::Administrator,
            )
            .unwrap();

        let password_manager = Arc::new(PasswordManager::new());
        let ws_security = Arc::new(WsSecurityValidator::with_defaults());
        let auth_ctx = AuthContext::new(ws_security, user_storage, password_manager, true);
        let dispatcher = ServiceDispatcher::new();

        // Case 1: nonexistent user via WS-Security
        let token_bad_user = UsernameToken {
            username: "nonexistent_user".to_string(),
            password: PasswordElement {
                value: "anypassword".to_string(),
                password_type: Some(
                    "http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-username-token-profile-1.0#PasswordText".to_string(),
                ),
            },
            nonce: None,
            created: None,
        };

        let err_bad_user = authenticate(
            &dispatcher,
            Some(&token_bad_user),
            &auth_ctx,
            AuthLevel::User,
        )
        .await
        .unwrap_err();

        // Case 2: valid user, wrong password via WS-Security
        let token_bad_pass = UsernameToken {
            username: "admin".to_string(),
            password: PasswordElement {
                value: "wrongpassword".to_string(),
                password_type: Some(
                    "http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-username-token-profile-1.0#PasswordText".to_string(),
                ),
            },
            nonce: None,
            created: None,
        };

        let err_bad_pass = authenticate(
            &dispatcher,
            Some(&token_bad_pass),
            &auth_ctx,
            AuthLevel::User,
        )
        .await
        .unwrap_err();

        // Both must produce identical error messages
        let msg_bad_user = format!("{}", err_bad_user);
        let msg_bad_pass = format!("{}", err_bad_pass);
        assert_eq!(
            msg_bad_user, msg_bad_pass,
            "WS-Security user-not-found and wrong-password must produce identical error messages"
        );

        // Neither must contain the username
        assert!(
            !msg_bad_user.contains("nonexistent_user"),
            "Error must not contain the attempted username"
        );
        assert!(
            !msg_bad_pass.contains("admin"),
            "Error must not contain the valid username"
        );
    }

    /// SECURITY TEST: All WsSecurityError variants mapped in ws_error_to_onvif
    /// must not leak usernames in their ONVIF error output.
    #[test]
    fn test_security_all_ws_errors_no_username_leak() {
        let test_username = "probe_target_user";
        let error = WsSecurityError::UserNotFound(test_username.to_string());
        let onvif_err = ws_error_to_onvif(error);
        let soap_fault = onvif_err.to_soap_fault();

        assert!(
            !soap_fault.contains(test_username),
            "SOAP fault must not leak the probed username"
        );
    }
}
