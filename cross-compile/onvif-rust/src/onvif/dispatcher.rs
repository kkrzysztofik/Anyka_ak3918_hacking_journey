//! Service dispatcher for routing SOAP requests to handlers.
//!
//! The dispatcher is responsible for:
//!
//! - Extracting the SOAP action from request headers or body
//! - Routing requests to the appropriate service handler
//! - Managing service registration
//! - Authentication and authorization (when auth is enabled)
//!
//! # Architecture
//!
//! ```text
//! Request → Extract Action → Auth Check → Find Handler → Execute → Response
//!              ↓                 ↓              ↓
//!         SOAPAction header  WS-Security   HashMap<service, Handler>
//!         or Body first elem
//! ```
//!
//! # Example
//!
//! ```ignore
//! use onvif_rust::onvif::dispatcher::{ServiceDispatcher, ServiceHandler, AuthContext};
//!
//! let mut dispatcher = ServiceDispatcher::new();
//! dispatcher.register_service("device", DeviceServiceHandler::new());
//!
//! // Without auth
//! let response = dispatcher.dispatch("device", request).await;
//!
//! // With auth
//! let auth_ctx = AuthContext::new(ws_security, user_storage, password_manager, true);
//! let response = dispatcher.dispatch_with_auth("device", request, &auth_ctx).await;
//! ```

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use axum::{
    body::Body,
    extract::Request,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use parking_lot::RwLock;

use super::auth_requirements::{AuthLevel, get_required_level};
use super::error::OnvifError;
use super::soap::{UsernameToken, parse_soap_request};
use super::ws_security::{WsSecurityError, WsSecurityValidator};
use crate::users::{PasswordManager, UserAccount, UserStorage};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};

/// Authentication context for dispatch operations.
///
/// This struct holds references to authentication-related components
/// and configuration needed to perform WS-Security authentication.
#[derive(Clone)]
pub struct AuthContext {
    /// WS-Security validator for digest verification and nonce checking.
    pub ws_security: Arc<WsSecurityValidator>,
    /// User storage for credential lookup.
    pub user_storage: Arc<UserStorage>,
    /// Password manager for password retrieval.
    pub password_manager: Arc<PasswordManager>,
    /// Whether authentication is enabled.
    pub auth_enabled: bool,
}

impl AuthContext {
    /// Create a new authentication context.
    pub fn new(
        ws_security: Arc<WsSecurityValidator>,
        user_storage: Arc<UserStorage>,
        password_manager: Arc<PasswordManager>,
        auth_enabled: bool,
    ) -> Self {
        Self {
            ws_security,
            user_storage,
            password_manager,
            auth_enabled,
        }
    }

    /// Create a disabled authentication context (for testing).
    pub fn disabled() -> Self {
        Self {
            ws_security: Arc::new(WsSecurityValidator::with_defaults()),
            user_storage: Arc::new(UserStorage::new()),
            password_manager: Arc::new(PasswordManager::new()),
            auth_enabled: false,
        }
    }
}

/// Trait for service handlers that process SOAP operations.
///
/// Each ONVIF service (Device, Media, PTZ, Imaging) implements this trait
/// to handle incoming SOAP requests.
///
/// # Example
///
/// ```ignore
/// use onvif_rust::onvif::dispatcher::ServiceHandler;
///
/// struct DeviceServiceHandler;
///
/// #[async_trait]
/// impl ServiceHandler for DeviceServiceHandler {
///     async fn handle_operation(&self, action: &str, body: &str) -> Result<String, OnvifError> {
///         match action {
///             "GetDeviceInformation" => Ok(self.get_device_information()),
///             _ => Err(OnvifError::ActionNotSupported(action.to_string())),
///         }
///     }
///
///     fn service_name(&self) -> &str {
///         "Device"
///     }
///
///     fn supported_actions(&self) -> Vec<&str> {
///         vec!["GetDeviceInformation", "GetCapabilities", "GetScopes"]
///     }
/// }
/// ```
#[async_trait]
pub trait ServiceHandler: Send + Sync {
    /// Handle a SOAP operation.
    ///
    /// # Arguments
    ///
    /// * `action` - The SOAP action/operation name (e.g., "GetDeviceInformation")
    /// * `body_xml` - The raw SOAP body XML content
    ///
    /// # Returns
    ///
    /// The response body XML on success, or an `OnvifError` on failure.
    async fn handle_operation(&self, action: &str, body_xml: &str) -> Result<String, OnvifError>;

    /// Get the service name for logging and debugging.
    fn service_name(&self) -> &str;

    /// Get the list of actions supported by this service.
    fn supported_actions(&self) -> Vec<&str>;

    /// Get the required authentication level for an operation.
    ///
    /// The default implementation looks up the requirement from the global
    /// `auth_requirements` map. Services can override this method to provide
    /// custom auth requirements for specific operations.
    ///
    /// # Arguments
    ///
    /// * `action` - The SOAP action/operation name
    ///
    /// # Returns
    ///
    /// The required `AuthLevel` for this operation.
    fn required_auth_level(&self, action: &str) -> AuthLevel {
        get_required_level(self.service_name(), action)
    }
}

/// Service dispatcher for routing SOAP requests to handlers.
pub struct ServiceDispatcher {
    /// Registered service handlers.
    handlers: RwLock<HashMap<String, Arc<dyn ServiceHandler>>>,
}

impl ServiceDispatcher {
    /// Create a new empty dispatcher.
    pub fn new() -> Self {
        Self {
            handlers: RwLock::new(HashMap::new()),
        }
    }

    /// Register a service handler.
    ///
    /// # Arguments
    ///
    /// * `service_name` - The service identifier (e.g., "device", "media")
    /// * `handler` - The handler implementation
    pub fn register_service(&self, service_name: &str, handler: Arc<dyn ServiceHandler>) {
        let mut handlers = self.handlers.write();
        handlers.insert(service_name.to_lowercase(), handler);
        tracing::debug!("Registered service handler: {}", service_name);
    }

    /// Dispatch a request to the appropriate service handler.
    ///
    /// # Arguments
    ///
    /// * `service` - The service to dispatch to (e.g., "device", "media")
    /// * `request` - The HTTP request
    ///
    /// # Returns
    ///
    /// An HTTP response with the SOAP response or fault.
    pub async fn dispatch(&self, service: &str, request: Request<Body>) -> Response {
        // Extract SOAP action from header
        let soap_action = extract_soap_action(&request);

        // Read body
        let body_bytes = match axum::body::to_bytes(request.into_body(), 1024 * 1024).await {
            Ok(bytes) => bytes,
            Err(e) => {
                tracing::error!("Failed to read request body: {}", e);
                return error_response(OnvifError::WellFormed(format!(
                    "Failed to read request body: {}",
                    e
                )));
            }
        };

        let body_str = match std::str::from_utf8(&body_bytes) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("Invalid UTF-8 in request body: {}", e);
                return error_response(OnvifError::WellFormed(format!(
                    "Invalid UTF-8 in request body: {}",
                    e
                )));
            }
        };

        // Parse SOAP envelope
        let envelope = match parse_soap_request(body_str) {
            Ok(env) => env,
            Err(e) => {
                tracing::error!("Failed to parse SOAP envelope: {}", e);
                return error_response(OnvifError::WellFormed(format!(
                    "Failed to parse SOAP envelope: {}",
                    e
                )));
            }
        };

        // Determine action (prefer header, fallback to body)
        tracing::debug!(
            "Action extraction: soap_action_header={:?}, envelope_action={:?}",
            soap_action,
            envelope.action
        );
        let action = soap_action
            .clone()
            .or(envelope.action.clone())
            .unwrap_or_default();

        if action.is_empty() {
            tracing::warn!(
                "Missing SOAP action in request (header={:?}, body={:?})",
                soap_action,
                envelope.action
            );
            return error_response(OnvifError::WellFormed(
                "Missing SOAP action in request".to_string(),
            ));
        }

        tracing::debug!("Dispatching {} to service '{}'", action, service);

        // Find handler
        let handler = {
            let handlers = self.handlers.read();
            handlers.get(&service.to_lowercase()).cloned()
        };

        let handler = match handler {
            Some(h) => h,
            None => {
                tracing::warn!("No handler registered for service: {}", service);
                return error_response(OnvifError::ActionNotSupported(format!(
                    "Service '{}' not available",
                    service
                )));
            }
        };

        // Handle the operation
        match handler.handle_operation(&action, &envelope.body_xml).await {
            Ok(response_body) => {
                let response_xml = super::soap::build_soap_response(&response_body);
                (
                    StatusCode::OK,
                    [(header::CONTENT_TYPE, "application/soap+xml; charset=utf-8")],
                    response_xml,
                )
                    .into_response()
            }
            Err(e) => {
                tracing::warn!("Operation {} failed: {:?}", action, e);
                error_response(e)
            }
        }
    }

    /// Dispatch a request with authentication.
    ///
    /// This method performs WS-Security authentication before routing to the handler.
    /// If authentication is disabled in the auth context, this behaves like `dispatch()`.
    ///
    /// # Arguments
    ///
    /// * `service` - The service to dispatch to (e.g., "device", "media")
    /// * `request` - The HTTP request
    /// * `auth_ctx` - Authentication context containing validators and settings
    ///
    /// # Returns
    ///
    /// An HTTP response with the SOAP response or a SOAP fault for auth failures.
    pub async fn dispatch_with_auth(
        &self,
        service: &str,
        request: Request<Body>,
        auth_ctx: &AuthContext,
    ) -> Response {
        // Extract SOAP action from header
        let soap_action = extract_soap_action(&request);

        // Check Basic Auth existence and validity (before consuming body)
        let basic_auth_result = if auth_ctx.auth_enabled {
            self.verify_basic_auth(&request, auth_ctx)
        } else {
            Ok(None)
        };

        // Read and parse request body
        let envelope = match self.read_and_parse_request(request).await {
            Ok(env) => env,
            Err(response) => return *response,
        };

        // Determine action (prefer header, fallback to body)
        let action = match self.extract_action(soap_action, &envelope) {
            Ok(action) => action,
            Err(response) => return *response,
        };

        tracing::debug!(
            "Dispatching {} to service '{}' (auth_enabled: {})",
            action,
            service,
            auth_ctx.auth_enabled
        );

        // Find handler
        let handler = match self.find_handler(service) {
            Ok(handler) => handler,
            Err(response) => return *response,
        };

        // Check authentication if enabled
        if let Err(response) = self
            .check_authentication(&action, &handler, &basic_auth_result, &envelope, auth_ctx)
            .await
        {
            return *response;
        }

        // Handle the operation
        self.handle_operation(&handler, &action, &envelope.body_xml)
            .await
    }

    /// Read request body and parse SOAP envelope.
    async fn read_and_parse_request(
        &self,
        request: Request<Body>,
    ) -> Result<super::soap::RawSoapEnvelope, Box<Response>> {
        // Read body
        let body_bytes = match axum::body::to_bytes(request.into_body(), 1024 * 1024).await {
            Ok(bytes) => bytes,
            Err(e) => {
                tracing::error!("Failed to read request body: {}", e);
                return Err(Box::new(error_response(OnvifError::WellFormed(format!(
                    "Failed to read request body: {}",
                    e
                )))));
            }
        };

        let body_str = match std::str::from_utf8(&body_bytes) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("Invalid UTF-8 in request body: {}", e);
                return Err(Box::new(error_response(OnvifError::WellFormed(format!(
                    "Invalid UTF-8 in request body: {}",
                    e
                )))));
            }
        };

        // Parse SOAP envelope
        match parse_soap_request(body_str) {
            Ok(env) => Ok(env),
            Err(e) => {
                tracing::error!("Failed to parse SOAP envelope: {}", e);
                Err(Box::new(error_response(OnvifError::WellFormed(format!(
                    "Failed to parse SOAP envelope: {}",
                    e
                )))))
            }
        }
    }

    /// Extract SOAP action from header or envelope.
    fn extract_action(
        &self,
        soap_action: Option<String>,
        envelope: &super::soap::RawSoapEnvelope,
    ) -> Result<String, Box<Response>> {
        tracing::debug!(
            "Action extraction (with auth): soap_action_header={:?}, envelope_action={:?}",
            soap_action,
            envelope.action
        );

        let action = soap_action
            .clone()
            .or(envelope.action.clone())
            .unwrap_or_default();

        if action.is_empty() {
            tracing::warn!(
                "Missing SOAP action in request (header={:?}, body={:?})",
                soap_action,
                envelope.action
            );
            return Err(Box::new(error_response(OnvifError::WellFormed(
                "Missing SOAP action in request".to_string(),
            ))));
        }

        Ok(action)
    }

    /// Find handler for the given service.
    fn find_handler(&self, service: &str) -> Result<Arc<dyn ServiceHandler>, Box<Response>> {
        let handler = {
            let handlers = self.handlers.read();
            handlers.get(&service.to_lowercase()).cloned()
        };

        match handler {
            Some(h) => Ok(h),
            None => {
                tracing::warn!("No handler registered for service: {}", service);
                Err(Box::new(error_response(OnvifError::ActionNotSupported(
                    format!("Service '{}' not available", service),
                ))))
            }
        }
    }

    /// Check authentication for the request.
    async fn check_authentication(
        &self,
        action: &str,
        handler: &Arc<dyn ServiceHandler>,
        basic_auth_result: &Result<Option<UserAccount>, OnvifError>,
        envelope: &super::soap::RawSoapEnvelope,
        auth_ctx: &AuthContext,
    ) -> Result<(), Box<Response>> {
        if !auth_ctx.auth_enabled {
            return Ok(());
        }

        let required_level = handler.required_auth_level(action);

        // Skip authentication for anonymous operations
        if required_level == AuthLevel::Anonymous {
            return Ok(());
        }

        // Try Basic Auth first
        match basic_auth_result {
            Ok(Some(user)) => {
                if required_level.is_satisfied_by(Some(user.level)) {
                    tracing::debug!("Basic Auth successful for action {}", action);
                    return Ok(());
                }
                return Err(Box::new(error_response(OnvifError::NotAuthorized(
                    "Insufficient privileges".to_string(),
                ))));
            }
            Ok(None) => {
                // No Basic Auth header, proceed to WS-Security
            }
            Err(e) => {
                // Basic Auth header present but invalid
                return Err(Box::new(error_response(e.clone())));
            }
        }

        // Fallback to WS-Security (UsernameToken)
        let ws_security = envelope
            .header
            .as_ref()
            .and_then(|h| h.security.as_ref())
            .and_then(|s| s.username_token.as_ref());

        match self
            .authenticate(ws_security, auth_ctx, required_level)
            .await
        {
            Ok(()) => {
                tracing::debug!("WS-Security Auth successful for action {}", action);
                Ok(())
            }
            Err(e) => {
                tracing::warn!("Authentication failed for action {}: {:?}", action, e);
                Err(Box::new(error_response(e)))
            }
        }
    }

    /// Handle the operation and return response.
    async fn handle_operation(
        &self,
        handler: &Arc<dyn ServiceHandler>,
        action: &str,
        body_xml: &str,
    ) -> Response {
        match handler.handle_operation(action, body_xml).await {
            Ok(response_body) => {
                let response_xml = super::soap::build_soap_response(&response_body);
                (
                    StatusCode::OK,
                    [(header::CONTENT_TYPE, "application/soap+xml; charset=utf-8")],
                    response_xml,
                )
                    .into_response()
            }
            Err(e) => {
                tracing::warn!("Operation {} failed: {:?}", action, e);
                error_response(e)
            }
        }
    }

    /// Authenticate a request using WS-Security credentials.
    async fn authenticate(
        &self,
        token: Option<&UsernameToken>,
        auth_ctx: &AuthContext,
        required_level: AuthLevel,
    ) -> Result<(), OnvifError> {
        // Get the UsernameToken or fail
        let token = token.ok_or_else(|| {
            OnvifError::NotAuthorized("Missing WS-Security credentials".to_string())
        })?;

        let username = &token.username;
        if username.is_empty() {
            return Err(OnvifError::NotAuthorized("Empty username".to_string()));
        }

        // Look up user
        let user = auth_ctx
            .user_storage
            .get_user(username)
            .ok_or_else(|| OnvifError::NotAuthorized(format!("User '{}' not found", username)))?;

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
            self.authenticate_digest(token, username, auth_ctx, &user)?;
        } else if password_type.contains("PasswordText") || password_type.is_empty() {
            self.authenticate_plaintext(token, auth_ctx, &user)?;
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
        &self,
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
            OnvifError::NotAuthorized(
                "Missing created timestamp for digest authentication".to_string(),
            )
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
        &self,
        token: &UsernameToken,
        auth_ctx: &AuthContext,
        user: &UserAccount,
    ) -> Result<(), OnvifError> {
        // Check if digest is required
        if auth_ctx.ws_security.requires_digest() {
            return Err(OnvifError::NotAuthorized(
                "Digest authentication required".to_string(),
            ));
        }

        // Verify plaintext password
        if !auth_ctx
            .password_manager
            .verify_password(&token.password.value, &user.password)
        {
            return Err(OnvifError::NotAuthorized("Invalid credentials".to_string()));
        }

        Ok(())
    }

    /// Verify HTTP Basic Authentication credentials.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(UserAccount))` - Valid Basic Auth credentials found
    /// * `Ok(None)` - No Basic Auth header found
    /// * `Err(OnvifError)` - Basic Auth header found but invalid
    fn verify_basic_auth(
        &self,
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
        let user = auth_ctx
            .user_storage
            .get_user(username)
            .ok_or_else(|| OnvifError::NotAuthorized(format!("User '{}' not found", username)))?;

        // Validate password
        // Use password_manager to handle verification (constant time comparison inside)
        if !auth_ctx
            .password_manager
            .verify_password(password, &user.password)
        {
            return Err(OnvifError::NotAuthorized("Invalid credentials".to_string()));
        }

        Ok(Some(user))
    }

    /// Check if a service is registered.
    pub fn has_service(&self, service: &str) -> bool {
        let handlers = self.handlers.read();
        handlers.contains_key(&service.to_lowercase())
    }

    /// Get the list of registered services.
    pub fn services(&self) -> Vec<String> {
        let handlers = self.handlers.read();
        handlers.keys().cloned().collect()
    }
}

impl Default for ServiceDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

/// Extract the SOAPAction from request headers.
///
/// The SOAPAction can be in the `SOAPAction` header or in the
/// `action` parameter of the `Content-Type` header.
fn extract_soap_action(request: &Request<Body>) -> Option<String> {
    // Try SOAPAction header first
    if let Some(action) = extract_action_from_soap_header(request) {
        return Some(action);
    }

    // Try Content-Type header with action parameter
    extract_action_from_content_type(request)
}

/// Extract action from SOAPAction header.
fn extract_action_from_soap_header(request: &Request<Body>) -> Option<String> {
    let action_header = request.headers().get("SOAPAction")?;
    let action_str = action_header.to_str().ok()?;
    normalize_action(action_str)
}

/// Extract action from Content-Type header's action parameter.
fn extract_action_from_content_type(request: &Request<Body>) -> Option<String> {
    let content_type = request.headers().get(header::CONTENT_TYPE)?;
    let ct_str = content_type.to_str().ok()?;

    // Look for action= parameter
    for part in ct_str.split(';') {
        let part = part.trim();
        if let Some(action) = part.strip_prefix("action=") {
            return normalize_action(action);
        }
    }

    None
}

/// Normalize action string by trimming quotes and extracting from URI if needed.
fn normalize_action(action: &str) -> Option<String> {
    let action = action.trim_matches('"');
    let action = extract_action_from_uri(action).unwrap_or_else(|| action.to_string());

    if action.is_empty() {
        None
    } else {
        Some(action)
    }
}

/// Extract action name from URI (last segment after '/').
fn extract_action_from_uri(uri: &str) -> Option<String> {
    uri.rfind('/').map(|pos| uri[pos + 1..].to_string())
}

/// Build an error response from an OnvifError.
fn error_response(error: OnvifError) -> Response {
    let status = error.http_status();
    let fault_xml = error.to_soap_fault();

    (
        status,
        [(header::CONTENT_TYPE, "application/soap+xml; charset=utf-8")],
        fault_xml,
    )
        .into_response()
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
        WsSecurityError::UserNotFound(user) => {
            OnvifError::NotAuthorized(format!("User '{}' not found", user))
        }
        WsSecurityError::InsufficientPrivileges => {
            OnvifError::NotAuthorized("Insufficient privileges for this operation".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Request as HttpRequest;

    #[test]
    fn test_dispatcher_new() {
        let dispatcher = ServiceDispatcher::new();
        assert!(dispatcher.services().is_empty());
    }

    struct TestHandler;

    #[async_trait]
    impl ServiceHandler for TestHandler {
        async fn handle_operation(&self, action: &str, _body: &str) -> Result<String, OnvifError> {
            if action == "GetTest" {
                Ok("<TestResponse/>".to_string())
            } else {
                Err(OnvifError::ActionNotSupported(action.to_string()))
            }
        }

        fn service_name(&self) -> &str {
            "Test"
        }

        fn supported_actions(&self) -> Vec<&str> {
            vec!["GetTest"]
        }
    }

    #[test]
    fn test_register_service() {
        let dispatcher = ServiceDispatcher::new();
        dispatcher.register_service("test", Arc::new(TestHandler));

        assert!(dispatcher.has_service("test"));
        assert!(dispatcher.has_service("TEST")); // Case insensitive
        assert!(!dispatcher.has_service("unknown"));
    }

    #[test]
    fn test_services_list() {
        let dispatcher = ServiceDispatcher::new();
        dispatcher.register_service("test", Arc::new(TestHandler));
        dispatcher.register_service("device", Arc::new(TestHandler));

        let services = dispatcher.services();
        assert_eq!(services.len(), 2);
        assert!(services.contains(&"test".to_string()));
        assert!(services.contains(&"device".to_string()));
    }

    #[test]
    fn test_extract_soap_action_from_header() {
        let request = HttpRequest::builder()
            .method("POST")
            .header(
                "SOAPAction",
                "\"http://www.onvif.org/ver10/device/wsdl/GetDeviceInformation\"",
            )
            .body(Body::empty())
            .unwrap();

        let action = extract_soap_action(&request);
        assert_eq!(action, Some("GetDeviceInformation".to_string()));
    }

    #[test]
    fn test_extract_soap_action_from_content_type() {
        let request = HttpRequest::builder()
            .method("POST")
            .header(
                "Content-Type",
                "application/soap+xml; charset=utf-8; action=\"GetDeviceInformation\"",
            )
            .body(Body::empty())
            .unwrap();

        let action = extract_soap_action(&request);
        assert_eq!(action, Some("GetDeviceInformation".to_string()));
    }

    #[test]
    fn test_extract_soap_action_none() {
        let request = HttpRequest::builder()
            .method("POST")
            .header("Content-Type", "text/xml")
            .body(Body::empty())
            .unwrap();

        let action = extract_soap_action(&request);
        assert!(action.is_none());
    }

    #[test]
    fn test_extract_soap_action_strips_quotes() {
        // Test with double quotes
        let request = HttpRequest::builder()
            .method("POST")
            .header("SOAPAction", "\"GetDeviceInformation\"")
            .body(Body::empty())
            .unwrap();

        let action = extract_soap_action(&request);
        assert_eq!(action, Some("GetDeviceInformation".to_string()));
    }

    #[test]
    fn test_extract_soap_action_extracts_last_segment() {
        // Full URI should extract just the action name
        let request = HttpRequest::builder()
            .method("POST")
            .header(
                "SOAPAction",
                "http://www.onvif.org/ver10/device/wsdl/GetCapabilities",
            )
            .body(Body::empty())
            .unwrap();

        let action = extract_soap_action(&request);
        assert_eq!(action, Some("GetCapabilities".to_string()));
    }

    #[tokio::test]
    async fn test_dispatch_no_handler() {
        let dispatcher = ServiceDispatcher::new();

        // Create a simple SOAP request
        let soap_body = r#"<?xml version="1.0"?>
            <s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
                <s:Body><GetTest/></s:Body>
            </s:Envelope>"#;

        let request = HttpRequest::builder()
            .method("POST")
            .header("Content-Type", "application/soap+xml")
            .header("SOAPAction", "GetTest")
            .body(Body::from(soap_body))
            .unwrap();

        let response = dispatcher.dispatch("nonexistent", request).await;

        // Should return error for missing service
        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_dispatch_with_handler() {
        let dispatcher = ServiceDispatcher::new();
        dispatcher.register_service("test", Arc::new(TestHandler));

        let soap_body = r#"<?xml version="1.0"?>
            <s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
                <s:Body><GetTest/></s:Body>
            </s:Envelope>"#;

        let request = HttpRequest::builder()
            .method("POST")
            .header("Content-Type", "application/soap+xml")
            .header("SOAPAction", "GetTest")
            .body(Body::from(soap_body))
            .unwrap();

        let response = dispatcher.dispatch("test", request).await;

        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }

    #[tokio::test]
    async fn test_dispatch_action_not_supported() {
        let dispatcher = ServiceDispatcher::new();
        dispatcher.register_service("test", Arc::new(TestHandler));

        let soap_body = r#"<?xml version="1.0"?>
            <s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
                <s:Body><UnsupportedAction/></s:Body>
            </s:Envelope>"#;

        let request = HttpRequest::builder()
            .method("POST")
            .header("Content-Type", "application/soap+xml")
            .header("SOAPAction", "UnsupportedAction")
            .body(Body::from(soap_body))
            .unwrap();

        let response = dispatcher.dispatch("test", request).await;

        // Should return error for unsupported action
        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_dispatch_invalid_utf8() {
        let dispatcher = ServiceDispatcher::new();
        dispatcher.register_service("test", Arc::new(TestHandler));

        // Invalid UTF-8 bytes
        let invalid_bytes = vec![0xff, 0xfe, 0x00, 0x01];

        let request = HttpRequest::builder()
            .method("POST")
            .header("Content-Type", "application/soap+xml")
            .header("SOAPAction", "GetTest")
            .body(Body::from(invalid_bytes))
            .unwrap();

        let response = dispatcher.dispatch("test", request).await;

        // Should return error for invalid UTF-8
        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_dispatch_invalid_soap() {
        let dispatcher = ServiceDispatcher::new();
        dispatcher.register_service("test", Arc::new(TestHandler));

        let invalid_soap = "not valid xml at all";

        let request = HttpRequest::builder()
            .method("POST")
            .header("Content-Type", "application/soap+xml")
            .header("SOAPAction", "GetTest")
            .body(Body::from(invalid_soap))
            .unwrap();

        let response = dispatcher.dispatch("test", request).await;

        // Should return error for invalid SOAP
        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_dispatch_missing_action() {
        let dispatcher = ServiceDispatcher::new();
        dispatcher.register_service("test", Arc::new(TestHandler));

        // SOAP without action in body and no header
        let soap_body = r#"<?xml version="1.0"?>
            <s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
                <s:Body></s:Body>
            </s:Envelope>"#;

        let request = HttpRequest::builder()
            .method("POST")
            .header("Content-Type", "application/soap+xml")
            .body(Body::from(soap_body))
            .unwrap();

        let response = dispatcher.dispatch("test", request).await;

        // Should return error for missing action
        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_required_auth_level_default() {
        // TestHandler uses the default implementation which looks up from auth_requirements
        let handler = TestHandler;

        // Since TestHandler's service_name is "Test" and there's no entry for it,
        // the default should be Administrator (fail-secure)
        assert_eq!(
            handler.required_auth_level("GetTest"),
            AuthLevel::Administrator
        );
    }

    // Custom handler that overrides required_auth_level
    struct CustomAuthHandler;

    #[async_trait]
    impl ServiceHandler for CustomAuthHandler {
        async fn handle_operation(&self, _action: &str, _body: &str) -> Result<String, OnvifError> {
            Ok("<Response/>".to_string())
        }

        fn service_name(&self) -> &str {
            "custom"
        }

        fn supported_actions(&self) -> Vec<&str> {
            vec!["PublicOp", "AdminOp"]
        }

        fn required_auth_level(&self, action: &str) -> AuthLevel {
            match action {
                "PublicOp" => AuthLevel::Anonymous,
                "AdminOp" => AuthLevel::Administrator,
                _ => AuthLevel::User,
            }
        }
    }

    #[test]
    fn test_required_auth_level_custom_override() {
        let handler = CustomAuthHandler;

        assert_eq!(
            handler.required_auth_level("PublicOp"),
            AuthLevel::Anonymous
        );
        assert_eq!(
            handler.required_auth_level("AdminOp"),
            AuthLevel::Administrator
        );
        assert_eq!(handler.required_auth_level("UnknownOp"), AuthLevel::User);
    }

    #[tokio::test]
    async fn test_dispatch_basic_auth_success() {
        // Setup auth context with enabled auth
        let user_storage = Arc::new(UserStorage::new());
        user_storage
            .create_user(
                "admin",
                "password123",
                crate::users::UserLevel::Administrator,
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

        // Create Basic Auth header (admin:password123)
        let credentials = base64::engine::general_purpose::STANDARD.encode("admin:password123");

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
                "password123",
                crate::users::UserLevel::Administrator,
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

        // Should NOT be OK. Expecting 401 Unauthorized or 400 Bad Request (depending on error mapping)
        assert_ne!(response.status(), axum::http::StatusCode::OK);
    }
}
