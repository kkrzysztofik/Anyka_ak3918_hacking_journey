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
use parking_lot::RwLock;

use super::auth_requirements::{AuthLevel, get_required_level};
use super::error::OnvifError;
use super::ws_security::WsSecurityValidator;
use crate::config::{PasswordManager, UserStorage};

// Module declarations - each contains part of ServiceDispatcher impl
pub mod auth;
pub mod request_parse;
pub mod response;
pub mod routing;

// Re-export parse_body for convenience
pub use request_parse::parse_body;
// Re-export for diagnostics HTTP auth middleware (single credential-check path)
pub(crate) use auth::verify_basic_auth_self;

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

impl Default for ServiceDispatcher {
    fn default() -> Self {
        Self::new()
    }
}
