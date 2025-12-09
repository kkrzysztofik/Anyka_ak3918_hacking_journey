//! ONVIF HTTP Server implementation using axum.
//!
//! This module provides the main HTTP server that hosts all ONVIF services.
//! It handles:
//!
//! - HTTP server setup with configurable bind address and port
//! - Request routing to service endpoints
//! - Middleware configuration (timeouts, body limits, CORS)
//! - Graceful shutdown coordination
//!
//! # Service Endpoints
//!
//! The server exposes the following ONVIF service endpoints:
//!
//! | Path                      | Service        |
//! |---------------------------|----------------|
//! | `/onvif/device_service`   | Device Service |
//! | `/onvif/media_service`    | Media Service  |
//! | `/onvif/ptz_service`      | PTZ Service    |
//! | `/onvif/imaging_service`  | Imaging Service|
//!
//! # Example
//!
//! ```ignore
//! use onvif_rust::onvif::server::{OnvifServer, OnvifServerConfig};
//! use onvif_rust::app::AppState;
//!
//! let config = OnvifServerConfig {
//!     bind_address: "0.0.0.0".to_string(),
//!     port: 8080,
//!     request_timeout_secs: 30,
//!     max_body_size: 1024 * 1024, // 1MB
//!     enable_cors: false,
//!     http_verbose: false,
//! };
//!
//! let app_state = AppState::builder()
//!     .user_storage(Arc::new(UserStorage::new()))
//!     .password_manager(Arc::new(PasswordManager::new()))
//!     .ptz_state(Arc::new(PTZStateManager::new()))
//!     .config(Arc::new(ConfigRuntime::new(Default::default())))
//!     .build()?;
//!
//! let server = OnvifServer::with_app_state(config, app_state)?;
//! server.start().await?;
//! ```

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::{
    Router,
    body::Body,
    extract::{Request, State},
    http::{Method, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::post,
};
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tower::ServiceBuilder;
use tower_http::timeout::TimeoutLayer;

use super::dispatcher::{AuthContext, ServiceDispatcher};
use super::error::OnvifError;
use super::ws_security::{WsSecurityConfig, WsSecurityValidator};
use crate::app::AppState;
use crate::logging::{HttpLogConfig, HttpLoggingMiddleware, memory_check_middleware};
use crate::users::{PasswordManager, UserStorage};
use crate::utils::MemoryMonitor;

/// Configuration for the ONVIF HTTP server.
#[derive(Debug, Clone)]
pub struct OnvifServerConfig {
    /// Address to bind the server to (e.g., "0.0.0.0", "127.0.0.1").
    pub bind_address: String,
    /// Port to listen on.
    pub port: u16,
    /// Request timeout in seconds.
    pub request_timeout_secs: u64,
    /// Maximum request body size in bytes.
    pub max_body_size: usize,
    /// Enable CORS for browser-based clients.
    pub enable_cors: bool,
    /// Enable verbose HTTP request/response logging.
    pub http_verbose: bool,
}

impl Default for OnvifServerConfig {
    fn default() -> Self {
        Self {
            bind_address: "0.0.0.0".to_string(),
            port: 8080,
            request_timeout_secs: 30,
            max_body_size: 1024 * 1024, // 1MB
            enable_cors: false,
            http_verbose: false,
        }
    }
}

/// Shared state for the ONVIF server.
#[derive(Clone)]
pub struct OnvifServerState {
    /// Service dispatcher for routing requests to handlers.
    pub dispatcher: Arc<ServiceDispatcher>,
    /// Shutdown signal sender.
    pub shutdown_tx: broadcast::Sender<()>,
    /// WS-Security validator for authentication.
    pub ws_security: Arc<WsSecurityValidator>,
    /// User storage for looking up credentials.
    pub user_storage: Arc<UserStorage>,
    /// Password manager for credential verification.
    pub password_manager: Arc<PasswordManager>,
    /// Whether authentication is enabled.
    pub auth_enabled: bool,
    /// Memory monitor for resource enforcement.
    pub memory_monitor: Arc<MemoryMonitor>,
}

impl OnvifServerState {
    /// Create an AuthContext from the server state.
    ///
    /// This is used by service handlers to pass authentication
    /// context to the dispatcher.
    pub fn auth_context(&self) -> AuthContext {
        AuthContext {
            auth_enabled: self.auth_enabled,
            ws_security: Arc::clone(&self.ws_security),
            user_storage: Arc::clone(&self.user_storage),
            password_manager: Arc::clone(&self.password_manager),
        }
    }
}

/// ONVIF HTTP Server.
///
/// The main server struct that manages the HTTP server lifecycle and routes
/// requests to the appropriate ONVIF service handlers.
pub struct OnvifServer {
    /// Server configuration.
    config: OnvifServerConfig,
    /// Service dispatcher.
    dispatcher: Arc<ServiceDispatcher>,
    /// Shutdown signal sender.
    shutdown_tx: broadcast::Sender<()>,
    /// WS-Security validator.
    ws_security: Arc<WsSecurityValidator>,
    /// User storage for authentication.
    user_storage: Arc<UserStorage>,
    /// Password manager.
    password_manager: Arc<PasswordManager>,
    /// Whether authentication is enabled.
    auth_enabled: bool,
    /// Memory monitor for resource enforcement.
    memory_monitor: Arc<MemoryMonitor>,
}

impl OnvifServer {
    /// Create a new ONVIF server with the given configuration.
    ///
    /// This constructor creates a server with Media and Imaging services only,
    /// with authentication disabled. For full service registration including
    /// Device and PTZ services with authentication, use
    /// [`OnvifServer::with_app_state`] instead.
    ///
    /// # Arguments
    ///
    /// * `config` - Server configuration
    ///
    /// # Returns
    ///
    /// A new `OnvifServer` instance, or an error if configuration is invalid.
    pub fn new(config: OnvifServerConfig) -> Result<Self, OnvifError> {
        // Validate configuration
        if config.port == 0 {
            return Err(OnvifError::InvalidArgVal {
                subcode: "InvalidPort".to_string(),
                reason: "Port cannot be 0".to_string(),
            });
        }

        if config.max_body_size == 0 {
            return Err(OnvifError::InvalidArgVal {
                subcode: "InvalidBodySize".to_string(),
                reason: "Max body size cannot be 0".to_string(),
            });
        }

        let dispatcher = Arc::new(ServiceDispatcher::new());

        // Register built-in services (minimal set for backward compatibility)
        Self::register_minimal_services(&dispatcher);

        let (shutdown_tx, _) = broadcast::channel(1);

        // Create default user storage and password manager (auth disabled in this mode)
        let user_storage = Arc::new(UserStorage::new());
        let password_manager = Arc::new(PasswordManager::new());

        // WS-Security validator with default config
        let ws_security = Arc::new(WsSecurityValidator::new(WsSecurityConfig::default()));

        // Default memory monitor
        let memory_monitor = Arc::new(MemoryMonitor::new());

        Ok(Self {
            config,
            dispatcher,
            shutdown_tx,
            ws_security,
            user_storage,
            password_manager,
            auth_enabled: false, // Authentication disabled in minimal mode
            memory_monitor,
        })
    }

    /// Create a new ONVIF server with full dependency injection via AppState.
    ///
    /// This constructor creates a server with all four ONVIF services:
    /// - Device Service (requires UserStorage, PasswordManager, ConfigRuntime, Platform)
    /// - Media Service
    /// - PTZ Service (requires PTZStateManager, ConfigRuntime, Platform)
    /// - Imaging Service
    ///
    /// Authentication is enabled based on the configuration in AppState.
    ///
    /// # Arguments
    ///
    /// * `config` - Server configuration
    /// * `app_state` - Shared application state containing all service dependencies
    ///
    /// # Returns
    ///
    /// A new `OnvifServer` instance, or an error if configuration is invalid.
    pub fn with_app_state(
        config: OnvifServerConfig,
        app_state: AppState,
    ) -> Result<Self, OnvifError> {
        // Validate configuration
        if config.port == 0 {
            return Err(OnvifError::InvalidArgVal {
                subcode: "InvalidPort".to_string(),
                reason: "Port cannot be 0".to_string(),
            });
        }

        if config.max_body_size == 0 {
            return Err(OnvifError::InvalidArgVal {
                subcode: "InvalidBodySize".to_string(),
                reason: "Max body size cannot be 0".to_string(),
            });
        }

        let dispatcher = Arc::new(ServiceDispatcher::new());

        // Register all services with dependencies from AppState
        Self::register_all_services(&dispatcher, &app_state);

        let (shutdown_tx, _) = broadcast::channel(1);

        // Extract auth configuration from ConfigRuntime
        let auth_enabled = app_state
            .config()
            .get_bool("server.auth_enabled")
            .unwrap_or(true);

        // Configure WS-Security based on app config
        let ws_config = WsSecurityConfig {
            clock_skew_seconds: 300, // 5 minutes
            nonce_ttl_seconds: 300,  // 5 minutes
            max_nonce_cache_size: 10000,
            require_digest: true, // Require digest auth in production
        };
        let ws_security = Arc::new(WsSecurityValidator::new(ws_config));

        Ok(Self {
            config,
            dispatcher,
            shutdown_tx,
            ws_security,
            user_storage: Arc::clone(app_state.user_storage()),
            password_manager: Arc::clone(app_state.password_manager()),
            auth_enabled,
            memory_monitor: Arc::clone(app_state.memory_monitor()),
        })
    }

    /// Register minimal ONVIF services (Media and Imaging only).
    ///
    /// This is used by the default `new()` constructor for backward compatibility.
    fn register_minimal_services(dispatcher: &ServiceDispatcher) {
        use super::imaging::ImagingService;
        use super::media::MediaService;

        // Register Imaging Service
        tracing::debug!("Registering Imaging Service");
        dispatcher.register_service("imaging", Arc::new(ImagingService::new()));

        // Register Media Service
        tracing::debug!("Registering Media Service");
        let media_service = MediaService::new();
        dispatcher.register_service("media", Arc::new(media_service));

        tracing::info!(
            "Registered {} ONVIF service(s) (minimal mode)",
            dispatcher.services().len()
        );
    }

    /// Register all built-in ONVIF services with the dispatcher.
    ///
    /// This method registers all four services using dependencies from AppState:
    /// - Device Service
    /// - Media Service
    /// - PTZ Service
    /// - Imaging Service
    fn register_all_services(dispatcher: &ServiceDispatcher, app_state: &AppState) {
        use super::device::DeviceService;
        use super::imaging::ImagingService;
        use super::media::MediaService;
        use super::ptz::PTZService;

        // Register Device Service
        tracing::debug!("Registering Device Service");
        let device_service = if let Some(platform) = app_state.platform() {
            DeviceService::with_config_and_platform(
                Arc::clone(app_state.user_storage()),
                Arc::clone(app_state.password_manager()),
                Arc::clone(app_state.config()),
                Arc::clone(platform),
            )
        } else {
            DeviceService::new(
                Arc::clone(app_state.user_storage()),
                Arc::clone(app_state.password_manager()),
            )
        };
        dispatcher.register_service("device", Arc::new(device_service));

        // Register Media Service
        tracing::debug!("Registering Media Service");
        dispatcher.register_service("media", Arc::new(MediaService::new()));

        // Register PTZ Service
        tracing::debug!("Registering PTZ Service");
        let ptz_service = if let Some(platform) = app_state.platform() {
            PTZService::with_platform(
                Arc::clone(app_state.ptz_state()),
                Arc::clone(app_state.config()),
                Arc::clone(platform),
            )
        } else {
            PTZService::new(
                Arc::clone(app_state.ptz_state()),
                Arc::clone(app_state.config()),
            )
        };
        dispatcher.register_service("ptz", Arc::new(ptz_service));

        // Register Imaging Service
        tracing::debug!("Registering Imaging Service");
        dispatcher.register_service("imaging", Arc::new(ImagingService::new()));

        tracing::info!(
            "Registered {} ONVIF service(s) (full mode)",
            dispatcher.services().len()
        );
    }

    /// Start the HTTP server and begin accepting connections.
    ///
    /// This method binds to the configured address and port, then enters
    /// the main accept loop. It will run until a shutdown signal is received.
    ///
    /// # Returns
    ///
    /// `Ok(())` when the server shuts down gracefully, or an error if
    /// binding or serving fails.
    pub async fn start(&self) -> Result<(), OnvifError> {
        let addr: SocketAddr = format!("{}:{}", self.config.bind_address, self.config.port)
            .parse()
            .map_err(|e| OnvifError::InvalidArgVal {
                subcode: "InvalidAddress".to_string(),
                reason: format!("Invalid bind address: {}", e),
            })?;

        tracing::info!("Starting ONVIF server on {}", addr);
        tracing::info!(
            "Authentication: {}",
            if self.auth_enabled {
                "enabled"
            } else {
                "disabled"
            }
        );

        let state = OnvifServerState {
            dispatcher: Arc::clone(&self.dispatcher),
            shutdown_tx: self.shutdown_tx.clone(),
            ws_security: Arc::clone(&self.ws_security),
            user_storage: Arc::clone(&self.user_storage),
            password_manager: Arc::clone(&self.password_manager),
            auth_enabled: self.auth_enabled,
            memory_monitor: Arc::clone(&self.memory_monitor),
        };

        let app = self.build_router(state);

        let listener = TcpListener::bind(addr).await.map_err(|e| {
            tracing::error!("Failed to bind to {}: {}", addr, e);
            OnvifError::HardwareFailure(format!("Failed to bind to {}: {}", addr, e))
        })?;

        tracing::info!("ONVIF server listening on {}", addr);

        let mut shutdown_rx = self.shutdown_tx.subscribe();

        axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                let _ = shutdown_rx.recv().await;
                tracing::info!("Shutdown signal received, stopping server...");
            })
            .await
            .map_err(|e| {
                tracing::error!("Server error: {}", e);
                OnvifError::HardwareFailure(format!("Server error: {}", e))
            })?;

        tracing::info!("ONVIF server stopped");
        Ok(())
    }

    /// Build the axum router with all service endpoints and middleware.
    fn build_router(&self, state: OnvifServerState) -> Router {
        // Build service routes
        let service_routes = Router::new()
            .route("/device_service", post(handle_device_service))
            .route("/media_service", post(handle_media_service))
            .route("/ptz_service", post(handle_ptz_service))
            .route("/imaging_service", post(handle_imaging_service));

        // Configure HTTP logging middleware
        let http_log_config = HttpLogConfig {
            verbose: self.config.http_verbose,
            max_body_log_size: 4096,
            sanitize_passwords: true,
        };
        let http_logging = HttpLoggingMiddleware::new(http_log_config);

        // Clone memory monitor for the memory check middleware
        let memory_monitor = Arc::clone(&state.memory_monitor);

        // Build the main router with middleware
        // Layers are applied in reverse order: last added = first executed
        Router::new()
            .nest("/onvif", service_routes)
            .layer(
                ServiceBuilder::new()
                    // Add request timeout (returns 408 Request Timeout on timeout)
                    .layer(TimeoutLayer::with_status_code(
                        StatusCode::REQUEST_TIMEOUT,
                        Duration::from_secs(self.config.request_timeout_secs),
                    ))
                    // Add middleware for logging and validation
                    .layer(middleware::from_fn(validate_content_type))
                    // Add HTTP logging middleware (replaces basic log_request)
                    .layer(http_logging.layer()),
            )
            // Add body limit extractor configuration
            .layer(axum::extract::DefaultBodyLimit::max(
                self.config.max_body_size,
            ))
            .with_state(state)
            // Memory check middleware - runs FIRST (outermost layer)
            // Order matters: first add the middleware, then the Extension it uses
            .layer(middleware::from_fn(memory_check_middleware))
            .layer(axum::Extension(memory_monitor))
    }

    /// Signal the server to shut down gracefully.
    pub fn shutdown(&self) {
        tracing::info!("Initiating ONVIF server shutdown...");
        let _ = self.shutdown_tx.send(());
    }

    /// Get a receiver for shutdown signals.
    ///
    /// Other components can use this to be notified when the server
    /// is shutting down.
    pub fn subscribe_shutdown(&self) -> broadcast::Receiver<()> {
        self.shutdown_tx.subscribe()
    }

    /// Get the server's bind address and port.
    pub fn address(&self) -> String {
        format!("{}:{}", self.config.bind_address, self.config.port)
    }
}

/// Middleware to validate Content-Type header.
///
/// ONVIF requires `text/xml` or `application/soap+xml` content types.
async fn validate_content_type(request: Request, next: Next) -> Response {
    // Only validate POST requests
    if request.method() != Method::POST {
        return next.run(request).await;
    }

    let content_type = request
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    // Accept text/xml or application/soap+xml
    if content_type.starts_with("text/xml") || content_type.starts_with("application/soap+xml") {
        return next.run(request).await;
    }

    tracing::warn!(
        "Invalid Content-Type: {}. Expected text/xml or application/soap+xml",
        content_type
    );

    let fault = OnvifError::WellFormed(
        "Invalid Content-Type. Expected text/xml or application/soap+xml".to_string(),
    );

    (
        StatusCode::UNSUPPORTED_MEDIA_TYPE,
        [(header::CONTENT_TYPE, "application/soap+xml; charset=utf-8")],
        fault.to_soap_fault(),
    )
        .into_response()
}

/// Handler for Device Service requests.
///
/// Device service operations have varying auth requirements:
/// - GetDeviceInformation, GetCapabilities: No auth required
/// - GetUsers, GetScopes: User level required
/// - CreateUsers, DeleteUsers, SetUser: Administrator required
async fn handle_device_service(
    State(state): State<OnvifServerState>,
    request: Request<Body>,
) -> Response {
    let auth_ctx = state.auth_context();
    state
        .dispatcher
        .dispatch_with_auth("device", request, &auth_ctx)
        .await
        .into_response()
}

/// Handler for Media Service requests.
///
/// Most Media operations require User level authentication.
/// Configuration changes require Operator or Administrator level.
async fn handle_media_service(
    State(state): State<OnvifServerState>,
    request: Request<Body>,
) -> Response {
    let auth_ctx = state.auth_context();
    state
        .dispatcher
        .dispatch_with_auth("media", request, &auth_ctx)
        .await
        .into_response()
}

/// Handler for PTZ Service requests.
///
/// PTZ operations generally require Operator level authentication.
/// Preset management may require Administrator level.
async fn handle_ptz_service(
    State(state): State<OnvifServerState>,
    request: Request<Body>,
) -> Response {
    let auth_ctx = state.auth_context();
    state
        .dispatcher
        .dispatch_with_auth("ptz", request, &auth_ctx)
        .await
        .into_response()
}

/// Handler for Imaging Service requests.
///
/// Imaging operations require Operator level authentication
/// as they affect camera image settings.
async fn handle_imaging_service(
    State(state): State<OnvifServerState>,
    request: Request<Body>,
) -> Response {
    let auth_ctx = state.auth_context();
    state
        .dispatcher
        .dispatch_with_auth("imaging", request, &auth_ctx)
        .await
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_config_default() {
        let config = OnvifServerConfig::default();
        assert_eq!(config.bind_address, "0.0.0.0");
        assert_eq!(config.port, 8080);
        assert_eq!(config.request_timeout_secs, 30);
        assert_eq!(config.max_body_size, 1024 * 1024);
        assert!(!config.enable_cors);
    }

    #[test]
    fn test_server_new_valid_config() {
        let config = OnvifServerConfig::default();
        let server = OnvifServer::new(config);
        assert!(server.is_ok());
    }

    #[test]
    fn test_server_new_invalid_port() {
        let config = OnvifServerConfig {
            port: 0,
            ..Default::default()
        };
        let server = OnvifServer::new(config);
        assert!(server.is_err());
    }

    #[test]
    fn test_server_new_invalid_body_size() {
        let config = OnvifServerConfig {
            max_body_size: 0,
            ..Default::default()
        };
        let server = OnvifServer::new(config);
        assert!(server.is_err());
    }

    #[test]
    fn test_server_address() {
        let config = OnvifServerConfig {
            bind_address: "127.0.0.1".to_string(),
            port: 9000,
            ..Default::default()
        };
        let server = OnvifServer::new(config).unwrap();
        assert_eq!(server.address(), "127.0.0.1:9000");
    }

    #[tokio::test]
    async fn test_server_shutdown_signal() {
        let config = OnvifServerConfig::default();
        let server = OnvifServer::new(config).unwrap();

        let mut rx = server.subscribe_shutdown();

        // Shutdown should send a signal
        server.shutdown();

        // Should receive the signal
        let result = tokio::time::timeout(Duration::from_millis(100), rx.recv()).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_server_config_custom() {
        let config = OnvifServerConfig {
            bind_address: "127.0.0.1".to_string(),
            port: 9090,
            request_timeout_secs: 60,
            max_body_size: 2 * 1024 * 1024,
            enable_cors: true,
            http_verbose: true,
        };

        assert_eq!(config.bind_address, "127.0.0.1");
        assert_eq!(config.port, 9090);
        assert_eq!(config.request_timeout_secs, 60);
        assert_eq!(config.max_body_size, 2 * 1024 * 1024);
        assert!(config.enable_cors);
        assert!(config.http_verbose);
    }

    #[test]
    fn test_server_config_clone() {
        let config = OnvifServerConfig::default();
        let cloned = config.clone();

        assert_eq!(config.bind_address, cloned.bind_address);
        assert_eq!(config.port, cloned.port);
        assert_eq!(config.request_timeout_secs, cloned.request_timeout_secs);
        assert_eq!(config.max_body_size, cloned.max_body_size);
        assert_eq!(config.enable_cors, cloned.enable_cors);
        assert_eq!(config.http_verbose, cloned.http_verbose);
    }

    #[test]
    fn test_server_multiple_shutdown_subscribers() {
        let config = OnvifServerConfig::default();
        let server = OnvifServer::new(config).unwrap();

        let _rx1 = server.subscribe_shutdown();
        let _rx2 = server.subscribe_shutdown();
        let _rx3 = server.subscribe_shutdown();

        // All subscribers should be created without error
        server.shutdown();
    }

    #[test]
    fn test_server_config_debug() {
        let config = OnvifServerConfig::default();
        let debug_str = format!("{:?}", config);

        assert!(debug_str.contains("bind_address"));
        assert!(debug_str.contains("port"));
        assert!(debug_str.contains("8080"));
    }

    #[tokio::test]
    async fn test_server_build_router() {
        use axum::body::Body;
        use axum::http::{Request, StatusCode};
        use tower::ServiceExt;

        let config = OnvifServerConfig::default();
        let server = OnvifServer::new(config).unwrap();

        let state = OnvifServerState {
            dispatcher: Arc::clone(&server.dispatcher),
            shutdown_tx: server.shutdown_tx.clone(),
            ws_security: Arc::clone(&server.ws_security),
            user_storage: Arc::clone(&server.user_storage),
            password_manager: Arc::clone(&server.password_manager),
            auth_enabled: server.auth_enabled,
            memory_monitor: Arc::clone(&server.memory_monitor),
        };

        let app = server.build_router(state);

        // Test that router responds to device service endpoint
        let request = Request::builder()
            .method("POST")
            .uri("/onvif/device_service")
            .header("Content-Type", "text/xml")
            .body(Body::from(r#"<?xml version="1.0"?><s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope"><s:Body><GetDeviceInformation/></s:Body></s:Envelope>"#))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        // Will return 400 because no handler registered, but route exists
        assert!(
            response.status() == StatusCode::BAD_REQUEST || response.status() == StatusCode::OK
        );
    }

    #[tokio::test]
    async fn test_server_invalid_content_type_rejected() {
        use axum::body::Body;
        use axum::http::{Request, StatusCode};
        use tower::ServiceExt;

        let config = OnvifServerConfig::default();
        let server = OnvifServer::new(config).unwrap();

        let state = OnvifServerState {
            dispatcher: Arc::clone(&server.dispatcher),
            shutdown_tx: server.shutdown_tx.clone(),
            ws_security: Arc::clone(&server.ws_security),
            user_storage: Arc::clone(&server.user_storage),
            password_manager: Arc::clone(&server.password_manager),
            auth_enabled: server.auth_enabled,
            memory_monitor: Arc::clone(&server.memory_monitor),
        };

        let app = server.build_router(state);

        // Test that invalid content-type is rejected
        let request = Request::builder()
            .method("POST")
            .uri("/onvif/device_service")
            .header("Content-Type", "application/json")
            .body(Body::from("{}"))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
    }

    #[tokio::test]
    async fn test_server_state_clone() {
        let config = OnvifServerConfig::default();
        let server = OnvifServer::new(config).unwrap();

        let state = OnvifServerState {
            dispatcher: Arc::clone(&server.dispatcher),
            shutdown_tx: server.shutdown_tx.clone(),
            ws_security: Arc::clone(&server.ws_security),
            user_storage: Arc::clone(&server.user_storage),
            password_manager: Arc::clone(&server.password_manager),
            auth_enabled: server.auth_enabled,
            memory_monitor: Arc::clone(&server.memory_monitor),
        };

        let cloned = state.clone();

        // Should be able to clone state
        assert!(Arc::ptr_eq(&state.dispatcher, &cloned.dispatcher));
        assert!(Arc::ptr_eq(&state.ws_security, &cloned.ws_security));
        assert!(Arc::ptr_eq(&state.user_storage, &cloned.user_storage));
        assert_eq!(state.auth_enabled, cloned.auth_enabled);
    }

    #[test]
    fn test_server_with_app_state_registers_all_services() {
        use crate::config::ConfigRuntime;
        use crate::onvif::ptz::PTZStateManager;
        use crate::users::password::PasswordManager;
        use crate::users::storage::UserStorage;
        use crate::utils::MemoryMonitor;

        let app_state = AppState::builder()
            .user_storage(Arc::new(UserStorage::new()))
            .password_manager(Arc::new(PasswordManager::new()))
            .ptz_state(Arc::new(PTZStateManager::new()))
            .config(Arc::new(ConfigRuntime::new(Default::default())))
            .memory_monitor(Arc::new(MemoryMonitor::new()))
            .build()
            .unwrap();

        let config = OnvifServerConfig::default();
        let server = OnvifServer::with_app_state(config, app_state).unwrap();

        // Should have all 4 services registered
        let services = server.dispatcher.services();
        assert_eq!(
            services.len(),
            4,
            "Expected 4 services, got: {:?}",
            services
        );

        // Verify each service is registered
        assert!(
            services.contains(&"device".to_string()),
            "Device service not registered"
        );
        assert!(
            services.contains(&"media".to_string()),
            "Media service not registered"
        );
        assert!(
            services.contains(&"ptz".to_string()),
            "PTZ service not registered"
        );
        assert!(
            services.contains(&"imaging".to_string()),
            "Imaging service not registered"
        );
    }

    #[test]
    fn test_server_new_registers_minimal_services() {
        let config = OnvifServerConfig::default();
        let server = OnvifServer::new(config).unwrap();

        // Should have only 2 services registered (Media and Imaging)
        let services = server.dispatcher.services();
        assert_eq!(
            services.len(),
            2,
            "Expected 2 services, got: {:?}",
            services
        );

        // Verify minimal services are registered
        assert!(
            services.contains(&"media".to_string()),
            "Media service not registered"
        );
        assert!(
            services.contains(&"imaging".to_string()),
            "Imaging service not registered"
        );
    }
}
