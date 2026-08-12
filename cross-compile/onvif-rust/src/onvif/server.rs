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
    extract::{ConnectInfo, Request, State},
    http::{Method, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post, put},
};
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tower::ServiceBuilder;
use tower_http::timeout::TimeoutLayer;

use super::dispatcher::{AuthContext, ServiceDispatcher};
use super::error::OnvifError;
use super::ws_security::{WsSecurityConfig, WsSecurityValidator};
use crate::app::AppState;
use crate::config::{PasswordManager, UserStorage};
use crate::logging::{
    HttpLogConfig, HttpLoggingMiddleware, memory_check_middleware, static_asset_logging_middleware,
};
use crate::security::RateLimiter;
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
    /// Path to static files root directory (e.g. "www").
    pub static_root: Option<String>,
    /// Enable verbose HTTP request/response logging.
    pub http_verbose: bool,
    /// Enable TLS/HTTPS for encrypted transport.
    pub tls_enabled: bool,
    /// Path to TLS certificate file (PEM format).
    pub tls_cert_path: Option<std::path::PathBuf>,
    /// Path to TLS private key file (PEM format).
    pub tls_key_path: Option<std::path::PathBuf>,
    /// Rate limit: maximum requests per minute per IP address.
    pub rate_limit_per_minute: u32,
}

impl Default for OnvifServerConfig {
    fn default() -> Self {
        Self {
            bind_address: "0.0.0.0".to_string(),
            port: 8080,
            request_timeout_secs: 30,
            max_body_size: 1024 * 1024, // 1MB
            enable_cors: false,
            static_root: Some("www".to_string()),
            http_verbose: false,
            tls_enabled: false,
            tls_cert_path: None,
            tls_key_path: None,
            rate_limit_per_minute: 60,
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
    /// Rate limiter for per-IP request limiting.
    pub rate_limiter: Arc<RateLimiter>,
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

/// Rate limiting middleware for axum.
///
/// Checks if the client IP has exceeded the rate limit and returns
/// HTTP 429 (Too Many Requests) if the limit is exceeded.
pub async fn rate_limit_middleware(
    State(state): State<OnvifServerState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request: Request,
    next: Next,
) -> Response {
    let ip = addr.ip();

    if !state.rate_limiter.check_rate_limit(&ip) {
        let count = state.rate_limiter.get_count(&ip).unwrap_or(0);
        crate::security::log_rate_limit_exceeded(&ip, count);
        return (StatusCode::TOO_MANY_REQUESTS, "Rate limit exceeded").into_response();
    }

    next.run(request).await
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
    /// Rate limiter for per-IP request limiting.
    rate_limiter: Arc<RateLimiter>,
    /// Optional diagnostics state. `None` disables the /api routes entirely.
    diagnostics: Option<Arc<crate::diagnostics::state::DiagnosticsState>>,
}

/// Validate security configuration for TLS and authentication.
///
/// This function validates that:
/// - TLS certificate and key paths are provided when TLS is enabled
/// - Warns when authentication is enabled without TLS (acceptable for air-gapped environments)
///
/// # Arguments
///
/// * `config` - Server configuration to validate
/// * `auth_enabled` - Whether authentication is enabled
///
/// # Returns
///
/// `Ok(())` if configuration is valid, or an error describing the issue.
fn validate_security_config(
    config: &OnvifServerConfig,
    auth_enabled: bool,
) -> Result<(), OnvifError> {
    // Validate TLS certificate paths if TLS is enabled
    if config.tls_enabled && (config.tls_cert_path.is_none() || config.tls_key_path.is_none()) {
        return Err(OnvifError::InvalidArgVal {
            subcode: "InvalidTLSConfig".to_string(),
            reason: "TLS certificate and key paths must be provided when TLS is enabled"
                .to_string(),
        });
    }

    // Warn about weak crypto without TLS (but don't block)
    if auth_enabled && !config.tls_enabled {
        tracing::warn!(
            "⚠️  SECURITY WARNING: Authentication enabled without TLS! \
             SHA-1/MD5 credentials (required by ONVIF 24.12) will be transmitted \
             without encryption. This is acceptable ONLY in air-gapped environments. \
             For production deployments, enable TLS to protect credentials in transit."
        );
    }

    Ok(())
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

        // Validate security configuration (auth disabled in minimal mode)
        validate_security_config(&config, false)?;

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

        // Default rate limiter (use config value or default to 60 requests per minute)
        let rate_limiter = Arc::new(RateLimiter::new(config.rate_limit_per_minute));

        Ok(Self {
            config,
            dispatcher,
            shutdown_tx,
            ws_security,
            user_storage,
            password_manager,
            auth_enabled: false, // Authentication disabled in minimal mode
            memory_monitor,
            rate_limiter,
            diagnostics: None,
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

        // Extract auth configuration from ConfigRuntime
        let auth_enabled = app_state.config().read().server.auth_enabled;

        // Validate security configuration
        validate_security_config(&config, auth_enabled)?;

        let dispatcher = Arc::new(ServiceDispatcher::new());

        // Register all services with dependencies from AppState
        Self::register_all_services(&dispatcher, &app_state);

        let (shutdown_tx, _) = broadcast::channel(1);

        // Extract auth configuration from ConfigRuntime
        let auth_enabled = app_state.config().read().server.auth_enabled;

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
            rate_limiter: Arc::clone(app_state.rate_limiter()),
            diagnostics: None,
        })
    }

    /// Attach a diagnostics state, enabling the `GET /api/diagnostics` and
    /// `GET /api/logs` routes on the router.
    ///
    /// When not called the `/api` sub-router is not mounted at all.
    #[must_use]
    pub fn with_diagnostics(
        mut self,
        diagnostics: Arc<crate::diagnostics::state::DiagnosticsState>,
    ) -> Self {
        self.diagnostics = Some(diagnostics);
        self
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
                Arc::clone(app_state.config()),
                Arc::clone(platform),
            )
        } else {
            DeviceService::new(Arc::clone(app_state.user_storage()))
        };
        dispatcher.register_service("device", Arc::new(device_service));

        // Register Media Service
        tracing::debug!("Registering Media Service");
        let mut media_service = MediaService::with_storage_and_persistence(
            Arc::clone(app_state.config()),
            Arc::clone(app_state.profile_storage()),
            app_state.platform().map(Arc::clone),
            app_state.profile_persistence().cloned(),
        );
        if let Some(rx) = app_state.availability() {
            media_service = media_service.with_availability(rx.clone());
        }
        dispatcher.register_service("media", Arc::new(media_service));

        // Register PTZ Service
        tracing::debug!("Registering PTZ Service");
        let ptz_service = if let Some(platform) = app_state.platform() {
            PTZService::with_platform(Arc::clone(app_state.ptz_state()), Arc::clone(platform))
        } else {
            PTZService::new(Arc::clone(app_state.ptz_state()))
        };
        dispatcher.register_service("ptz", Arc::new(ptz_service));

        // Register Imaging Service (use AppState store when persistence is wired)
        tracing::debug!("Registering Imaging Service");
        let imaging_service = if let Some(store) = app_state.imaging_settings_store() {
            ImagingService::with_store(
                Arc::clone(store),
                app_state.platform().map(Arc::clone),
                Some(Arc::clone(app_state.config())),
            )
        } else if let Some(platform) = app_state.platform() {
            ImagingService::with_config_and_platform(
                Arc::clone(app_state.config()),
                Arc::clone(platform),
            )
        } else {
            ImagingService::new()
        };
        dispatcher.register_service("imaging", Arc::new(imaging_service));

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
            rate_limiter: Arc::clone(&self.rate_limiter),
        };

        let app = self.build_router(state);

        let listener = TcpListener::bind(addr).await.map_err(|e| {
            tracing::error!("Failed to bind to {}: {}", addr, e);
            OnvifError::HardwareFailure(format!("Failed to bind to {}: {}", addr, e))
        })?;

        tracing::info!("ONVIF server listening on {}", addr);

        let mut shutdown_rx = self.shutdown_tx.subscribe();

        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
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
        // Request timeout, applied per route rather than over the whole app.
        //
        // PUT /api/update must be exempt: a bundle is ~19 MB and the upload is
        // one request, so a 30 s ceiling would abort every real update over
        // camera wifi while the SD card is also taking video writes. Every
        // other route keeps the timeout, which is what bounds a slow-loris on
        // the SOAP endpoints.
        let timeout = || {
            TimeoutLayer::with_status_code(
                StatusCode::REQUEST_TIMEOUT,
                Duration::from_secs(self.config.request_timeout_secs),
            )
        };

        // Build service routes
        let service_routes = Router::new()
            .route("/device_service", post(handle_device_service))
            .route("/media_service", post(handle_media_service))
            .route("/ptz_service", post(handle_ptz_service))
            .route("/imaging_service", post(handle_imaging_service))
            .layer(timeout());

        // Configure HTTP logging middleware
        let http_log_config = HttpLogConfig {
            verbose: self.config.http_verbose,
            max_body_log_size: 4096,
            sanitize_passwords: true,
        };
        let http_logging = HttpLoggingMiddleware::new(http_log_config);

        // Clone memory monitor for the memory check middleware
        let memory_monitor = Arc::clone(&state.memory_monitor);

        // Build the main router with middleware.
        // IMPORTANT: /api must be nested BEFORE the rate-limit and memory-check
        // layers so those protections also cover the diagnostics endpoints.
        // Layers are applied in reverse order: last added = first executed.
        let mut app = Router::new().nest("/onvif", service_routes);

        // Mount /api routes when diagnostics state is available.
        if let Some(diagnostics) = &self.diagnostics {
            let api = Router::new()
                .route(
                    "/diagnostics",
                    get(crate::diagnostics::http::handle_diagnostics).layer(timeout()),
                )
                .route(
                    "/logs",
                    get(crate::diagnostics::http::handle_logs).layer(timeout()),
                )
                // No timeout: see the comment on `timeout` above.
                .route("/update", put(crate::diagnostics::update::handle_update))
                .fallback(|| async { StatusCode::NOT_FOUND })
                .layer(middleware::from_fn_with_state(
                    state.clone(),
                    crate::diagnostics::http::diagnostics_auth_middleware,
                ))
                .layer(axum::Extension(Arc::clone(diagnostics)))
                .layer(axum::Extension(Arc::new(
                    crate::diagnostics::update::UpdateState::default(),
                )));
            app = app.nest("/api", api);
        }

        let app = app
            .layer(
                ServiceBuilder::new()
                    // Add middleware for logging and validation
                    .layer(middleware::from_fn(validate_content_type))
                    // Add HTTP logging middleware (replaces basic log_request)
                    .layer(http_logging.layer()),
            )
            // Add body limit extractor configuration
            .layer(axum::extract::DefaultBodyLimit::max(
                self.config.max_body_size,
            ))
            .with_state(state.clone())
            // Rate limiting middleware - runs BEFORE memory check
            .layer(middleware::from_fn_with_state(state, rate_limit_middleware))
            // Memory check middleware - runs FIRST (outermost layer)
            // Order matters: first add the middleware, then the Extension it uses
            .layer(middleware::from_fn(memory_check_middleware))
            .layer(axum::Extension(memory_monitor));

        // Add static file serving if configured
        if let Some(static_root) = &self.config.static_root {
            use tower_http::services::{ServeDir, ServeFile};

            tracing::info!("Serving static files from: {}", static_root);

            let index_path = std::path::Path::new(static_root).join("index.html");

            // Serve pre-compressed files if available (brotli/gzip) to save CPU on embedded device
            let serve_dir = ServeDir::new(static_root)
                .precompressed_br()
                .precompressed_gzip()
                .append_index_html_on_directories(true)
                .fallback(ServeFile::new(index_path));

            // Add static asset logging middleware
            // Note: ConnectInfo is extracted from the TCP connection by the server
            // It's available as an extractor in the middleware
            app.layer(axum::middleware::from_fn(static_asset_logging_middleware))
                .fallback_service(serve_dir)
        } else {
            app
        }
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
            static_root: Some("/tmp".to_string()),
            http_verbose: true,
            tls_enabled: false,
            tls_cert_path: None,
            tls_key_path: None,
            rate_limit_per_minute: 60,
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
        use axum::extract::ConnectInfo;
        use axum::http::{Request, StatusCode};
        use std::net::SocketAddr;
        use tower::ServiceExt;

        let config = OnvifServerConfig {
            static_root: None,
            ..Default::default()
        };
        let server = OnvifServer::new(config).unwrap();

        let state = OnvifServerState {
            dispatcher: Arc::clone(&server.dispatcher),
            shutdown_tx: server.shutdown_tx.clone(),
            ws_security: Arc::clone(&server.ws_security),
            user_storage: Arc::clone(&server.user_storage),
            password_manager: Arc::clone(&server.password_manager),
            auth_enabled: server.auth_enabled,
            memory_monitor: Arc::clone(&server.memory_monitor),
            rate_limiter: Arc::clone(&server.rate_limiter),
        };

        let app = server.build_router(state);

        // Test that router responds to device service endpoint
        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        let request = Request::builder()
            .method("POST")
            .uri("/onvif/device_service")
            .header("Content-Type", "text/xml")
            .extension(ConnectInfo(addr))
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
        use axum::extract::ConnectInfo;
        use axum::http::{Request, StatusCode};
        use std::net::SocketAddr;
        use tower::ServiceExt;

        let config = OnvifServerConfig {
            static_root: None,
            ..Default::default()
        };
        let server = OnvifServer::new(config).unwrap();

        let state = OnvifServerState {
            dispatcher: Arc::clone(&server.dispatcher),
            shutdown_tx: server.shutdown_tx.clone(),
            ws_security: Arc::clone(&server.ws_security),
            user_storage: Arc::clone(&server.user_storage),
            password_manager: Arc::clone(&server.password_manager),
            auth_enabled: server.auth_enabled,
            memory_monitor: Arc::clone(&server.memory_monitor),
            rate_limiter: Arc::clone(&server.rate_limiter),
        };

        let app = server.build_router(state);

        // Test that invalid content-type is rejected
        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        let request = Request::builder()
            .method("POST")
            .uri("/onvif/device_service")
            .header("Content-Type", "application/json")
            .extension(ConnectInfo(addr))
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
            rate_limiter: Arc::clone(&server.rate_limiter),
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
        use crate::config::{PasswordManager, ProfileStorage, UserStorage};
        use crate::onvif::ptz::PTZStateManager;
        use crate::utils::MemoryMonitor;

        let app_state = AppState::builder()
            .user_storage(Arc::new(UserStorage::new()))
            .password_manager(Arc::new(PasswordManager::new()))
            .ptz_state(Arc::new(PTZStateManager::new()))
            .config(Arc::new(ConfigRuntime::new(Default::default())))
            .memory_monitor(Arc::new(MemoryMonitor::new()))
            .rate_limiter(Arc::new(crate::security::RateLimiter::new(60)))
            .profile_storage(Arc::new(ProfileStorage::new("/tmp/test_profiles.toml")))
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
    #[tokio::test]
    async fn test_serve_static_files_with_compression() {
        use axum::extract::ConnectInfo;
        use axum::http::{Request, StatusCode};
        use std::fs::File;
        use std::io::Write;
        use std::net::SocketAddr;
        use tower::ServiceExt;

        // Create a temp directory for static files
        let temp_dir = tempfile::tempdir().unwrap();
        let static_root = temp_dir.path().to_str().unwrap().to_string();

        // Create index.html
        let file_path = temp_dir.path().join("index.html");
        let mut file = File::create(file_path).unwrap();
        file.write_all(b"Hello World").unwrap();

        // Create index.html.gz (simulated compressed content)
        // We use distinct content to verify the server picks the .gz file
        let gz_path = temp_dir.path().join("index.html.gz");
        let mut gz_file = File::create(gz_path).unwrap();
        gz_file.write_all(b"Hello Gzip").unwrap();

        let config = OnvifServerConfig {
            static_root: Some(static_root),
            ..Default::default()
        };

        let server = OnvifServer::new(config).unwrap();
        let state = OnvifServerState {
            dispatcher: Arc::clone(&server.dispatcher),
            shutdown_tx: server.shutdown_tx.clone(),
            ws_security: Arc::clone(&server.ws_security),
            user_storage: Arc::clone(&server.user_storage),
            password_manager: Arc::clone(&server.password_manager),
            auth_enabled: server.auth_enabled,
            memory_monitor: Arc::clone(&server.memory_monitor),
            rate_limiter: Arc::clone(&server.rate_limiter),
        };

        let app = server.build_router(state);
        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();

        // Test 1: Request without compression preference -> serves plain file
        let request = Request::builder()
            .uri("/")
            .extension(ConnectInfo(addr))
            .body(Body::empty())
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert!(response.headers().get("content-encoding").is_none());
        let body = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .unwrap();
        assert_eq!(&body[..], b"Hello World");

        // Test 2: Request with gzip preference -> serves .gz file
        let request = Request::builder()
            .uri("/")
            .header("Accept-Encoding", "gzip")
            .extension(ConnectInfo(addr))
            .body(Body::empty())
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers().get("content-encoding").unwrap(), "gzip");
        let body = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .unwrap();
        assert_eq!(&body[..], b"Hello Gzip");
    }

    #[test]
    fn test_server_config_tls_validation_enabled_without_cert() {
        let config = OnvifServerConfig {
            tls_enabled: true,
            tls_cert_path: None,
            tls_key_path: None,
            ..Default::default()
        };

        // Should fail validation when TLS is enabled without cert/key
        let result = OnvifServer::new(config);
        assert!(result.is_err());
    }

    #[test]
    fn test_server_config_tls_validation_enabled_without_key() {
        let config = OnvifServerConfig {
            tls_enabled: true,
            tls_cert_path: Some(std::path::PathBuf::from("/tmp/cert.pem")),
            tls_key_path: None,
            ..Default::default()
        };

        let result = OnvifServer::new(config);
        assert!(result.is_err());
    }

    #[test]
    fn test_server_config_tls_validation_enabled_without_cert_path() {
        let config = OnvifServerConfig {
            tls_enabled: true,
            tls_cert_path: None,
            tls_key_path: Some(std::path::PathBuf::from("/tmp/key.pem")),
            ..Default::default()
        };

        let result = OnvifServer::new(config);
        assert!(result.is_err());
    }

    #[test]
    fn test_server_state_auth_context() {
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
            rate_limiter: Arc::clone(&server.rate_limiter),
        };

        let auth_ctx = state.auth_context();
        assert_eq!(auth_ctx.auth_enabled, state.auth_enabled);
    }

    #[test]
    fn test_validate_content_type_get_request() {
        use axum::body::Body;
        use axum::http::Request;

        // GET requests should not be validated
        let request = Request::builder()
            .method("GET")
            .header("Content-Type", "application/json")
            .body(Body::empty())
            .unwrap();

        // This is a simple test - in real usage, this would go through the middleware
        // For now, just verify the request can be created
        assert_eq!(request.method(), "GET");
    }

    #[test]
    fn test_validate_content_type_text_xml() {
        use axum::body::Body;
        use axum::http::Request;

        // text/xml should be accepted
        let request = Request::builder()
            .method("POST")
            .header("Content-Type", "text/xml")
            .body(Body::empty())
            .unwrap();

        assert!(request.headers().get("Content-Type").is_some());
    }

    #[test]
    fn test_validate_content_type_application_soap_xml() {
        use axum::body::Body;
        use axum::http::Request;

        // application/soap+xml should be accepted
        let request = Request::builder()
            .method("POST")
            .header("Content-Type", "application/soap+xml")
            .body(Body::empty())
            .unwrap();

        assert!(request.headers().get("Content-Type").is_some());
    }

    #[test]
    fn test_server_with_app_state_tls_validation() {
        use crate::config::ConfigRuntime;
        use crate::config::{PasswordManager, ProfileStorage, UserStorage};
        use crate::onvif::ptz::PTZStateManager;
        use crate::utils::MemoryMonitor;

        let app_state = AppState::builder()
            .user_storage(Arc::new(UserStorage::new()))
            .password_manager(Arc::new(PasswordManager::new()))
            .ptz_state(Arc::new(PTZStateManager::new()))
            .config(Arc::new(ConfigRuntime::new(Default::default())))
            .memory_monitor(Arc::new(MemoryMonitor::new()))
            .rate_limiter(Arc::new(crate::security::RateLimiter::new(60)))
            .profile_storage(Arc::new(ProfileStorage::new("/tmp/test_profiles.toml")))
            .build()
            .unwrap();

        let config = OnvifServerConfig {
            tls_enabled: true,
            tls_cert_path: None,
            tls_key_path: None,
            ..Default::default()
        };

        // Should fail validation
        let result = OnvifServer::with_app_state(config, app_state);
        assert!(result.is_err());
    }

    #[test]
    fn test_rate_limit_middleware_state() {
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
            rate_limiter: Arc::clone(&server.rate_limiter),
        };

        // Verify state can be cloned
        let _cloned = state.clone();
    }

    // Helper: build a server+state+router with diagnostics attached.
    //
    // `auth_on` sets `auth_enabled` on the `OnvifServerState` used in
    // `build_router`; the `OnvifServer` is always constructed with auth off
    // (its constructor sets `auth_enabled: false` in minimal mode).
    fn make_diagnostics_app(auth_on: bool) -> Router {
        make_diagnostics_app_with_timeout(
            auth_on,
            OnvifServerConfig::default().request_timeout_secs,
        )
    }

    fn make_diagnostics_app_with_timeout(auth_on: bool, request_timeout_secs: u64) -> Router {
        use axum::extract::ConnectInfo;
        use std::net::SocketAddr;

        let config = OnvifServerConfig {
            static_root: None,
            request_timeout_secs,
            ..Default::default()
        };
        let server = OnvifServer::new(config).unwrap().with_diagnostics(Arc::new(
            crate::diagnostics::state::DiagnosticsState::new(
                std::time::Instant::now(),
                None,
                vec![],
            ),
        ));

        let state = OnvifServerState {
            dispatcher: Arc::clone(&server.dispatcher),
            shutdown_tx: server.shutdown_tx.clone(),
            ws_security: Arc::clone(&server.ws_security),
            user_storage: Arc::clone(&server.user_storage),
            password_manager: Arc::clone(&server.password_manager),
            auth_enabled: auth_on,
            memory_monitor: Arc::clone(&server.memory_monitor),
            rate_limiter: Arc::clone(&server.rate_limiter),
        };

        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        server
            .build_router(state)
            // ConnectInfo is required by rate_limit_middleware
            .layer(axum::Extension(ConnectInfo(addr)))
    }

    /// A body whose second chunk arrives only after `delay`.
    ///
    /// Stands in for a real bundle upload, which takes far longer than the
    /// SOAP request timeout: ~19 MB over camera wifi while the SD card is also
    /// taking video writes.
    struct SlowBody {
        delay: Duration,
        timer: Option<std::pin::Pin<Box<tokio::time::Sleep>>>,
        finished: bool,
    }

    impl http_body::Body for SlowBody {
        type Data = bytes::Bytes;
        type Error = std::io::Error;

        fn poll_frame(
            mut self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Option<Result<http_body::Frame<bytes::Bytes>, Self::Error>>> {
            use std::task::Poll;
            if self.finished {
                return Poll::Ready(None);
            }
            let delay = self.delay;
            let timer = self
                .timer
                .get_or_insert_with(|| Box::pin(tokio::time::sleep(delay)));
            match timer.as_mut().poll(cx) {
                Poll::Pending => Poll::Pending,
                Poll::Ready(()) => {
                    self.finished = true;
                    Poll::Ready(Some(Ok(http_body::Frame::data(bytes::Bytes::from_static(
                        b"slow bundle",
                    )))))
                }
            }
        }
    }

    /// The request timeout must not apply to bundle uploads.
    ///
    /// A 30 s ceiling — the deployed value — would abort every real update,
    /// since a bundle is one ~19 MB request. Guards against the timeout layer
    /// being moved back onto the whole app.
    #[tokio::test]
    async fn test_update_route_is_exempt_from_the_request_timeout() {
        use axum::body::Body;
        use axum::http::{Request, StatusCode};
        use tower::ServiceExt;

        let app = make_diagnostics_app_with_timeout(false, 1);

        let request = Request::builder()
            .method("PUT")
            .uri("/api/update")
            .body(Body::new(SlowBody {
                delay: Duration::from_secs(2),
                timer: None,
                finished: false,
            }))
            .unwrap();

        // Asserts only that the timeout did not fire. Whether the write itself
        // succeeds depends on the spool path, which `build_router` fixes at
        // /mnt/anyka_hack/spool and does not exist on a host — `receive_bundle`
        // has its own eight tests for that. A 408 here means the timeout layer
        // has been moved back onto the whole app and every real update is dead.
        let response = app.oneshot(request).await.unwrap();
        assert_ne!(
            response.status(),
            StatusCode::REQUEST_TIMEOUT,
            "an upload slower than request_timeout_secs must not be aborted"
        );
    }

    #[tokio::test]
    async fn test_diagnostics_route_requires_auth() {
        use axum::body::Body;
        use axum::http::{Request, StatusCode};
        use tower::ServiceExt;

        let app = make_diagnostics_app(true);

        // No Authorization header → must get 401, not 200, not 404.
        let request = Request::builder()
            .method("GET")
            .uri("/api/diagnostics")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(
            response.status(),
            StatusCode::UNAUTHORIZED,
            "unauthenticated request should be rejected"
        );
    }

    #[tokio::test]
    async fn test_update_route_requires_auth() {
        use axum::body::Body;
        use axum::http::{Request, StatusCode};
        use tower::ServiceExt;

        let app = make_diagnostics_app(true);

        // PUT /api/update without credentials → 401, exactly like /logs.
        let request = Request::builder()
            .method("PUT")
            .uri("/api/update")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(
            response.status(),
            StatusCode::UNAUTHORIZED,
            "unauthenticated upload must be rejected"
        );
    }

    #[tokio::test]
    async fn test_diagnostics_route_returns_json_when_authorized() {
        use axum::body::Body;
        use axum::http::{Request, StatusCode};
        use tower::ServiceExt;

        // auth_enabled=false → middleware passes through without credential check.
        let app = make_diagnostics_app(false);

        let request = Request::builder()
            .method("GET")
            .uri("/api/diagnostics")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), 64 * 1024)
            .await
            .unwrap();
        let text = std::str::from_utf8(&body).unwrap();
        assert!(
            text.contains("uptime"),
            "response body should contain 'uptime'; got: {text}"
        );
    }

    #[tokio::test]
    async fn test_diagnostics_route_allows_user_with_valid_credentials() {
        use axum::body::Body;
        use axum::http::{Request, StatusCode};
        use base64::Engine;
        use tower::ServiceExt;

        let config = OnvifServerConfig {
            static_root: None,
            ..Default::default()
        };
        let server = OnvifServer::new(config).unwrap().with_diagnostics(Arc::new(
            crate::diagnostics::state::DiagnosticsState::new(
                std::time::Instant::now(),
                None,
                vec![],
            ),
        ));
        server
            .user_storage
            .create_user("viewer", "pass", crate::config::UserLevel::User)
            .unwrap();

        let state = OnvifServerState {
            dispatcher: Arc::clone(&server.dispatcher),
            shutdown_tx: server.shutdown_tx.clone(),
            ws_security: Arc::clone(&server.ws_security),
            user_storage: Arc::clone(&server.user_storage),
            password_manager: Arc::clone(&server.password_manager),
            auth_enabled: true,
            memory_monitor: Arc::clone(&server.memory_monitor),
            rate_limiter: Arc::clone(&server.rate_limiter),
        };

        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        let app = server
            .build_router(state)
            .layer(axum::Extension(ConnectInfo(addr)));

        let credentials = base64::engine::general_purpose::STANDARD.encode("viewer:pass");
        let request = Request::builder()
            .method("GET")
            .uri("/api/diagnostics")
            .header("Authorization", format!("Basic {credentials}"))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "User-level credentials must reach /api/diagnostics through the nested router"
        );
    }

    #[tokio::test]
    async fn test_unknown_api_route_is_not_swallowed_by_static_fallback() {
        use axum::body::Body;
        use axum::extract::ConnectInfo;
        use axum::http::{Request, StatusCode};
        use std::io::Write;
        use std::net::SocketAddr;
        use tower::ServiceExt;

        // Create a real static root so ServeDir is active; index.html holds a
        // unique sentinel we can check is NOT returned for /api paths.
        let temp_dir = tempfile::tempdir().unwrap();
        let mut index = std::fs::File::create(temp_dir.path().join("index.html")).unwrap();
        index.write_all(b"STATIC_INDEX").unwrap();

        let config = OnvifServerConfig {
            static_root: Some(temp_dir.path().to_str().unwrap().to_string()),
            ..Default::default()
        };
        let server = OnvifServer::new(config).unwrap().with_diagnostics(Arc::new(
            crate::diagnostics::state::DiagnosticsState::new(
                std::time::Instant::now(),
                None,
                vec![],
            ),
        ));

        // auth_enabled=true: unauthenticated unknown /api path hits auth middleware → 401.
        let state = OnvifServerState {
            dispatcher: Arc::clone(&server.dispatcher),
            shutdown_tx: server.shutdown_tx.clone(),
            ws_security: Arc::clone(&server.ws_security),
            user_storage: Arc::clone(&server.user_storage),
            password_manager: Arc::clone(&server.password_manager),
            auth_enabled: true,
            memory_monitor: Arc::clone(&server.memory_monitor),
            rate_limiter: Arc::clone(&server.rate_limiter),
        };

        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        let app = server
            .build_router(state)
            .layer(axum::Extension(ConnectInfo(addr)));

        let request = Request::builder()
            .method("GET")
            .uri("/api/nonexistent")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        // With the inner /api fallback placed before the auth layer (Option A),
        // an unauthenticated unknown path returns 401. Either way it must not
        // be 200, and the body must never contain the static index sentinel.
        let status = response.status();
        assert_eq!(
            status,
            StatusCode::UNAUTHORIZED,
            "/api/nonexistent must be rejected by the auth layer; got: {status}"
        );

        let body = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .unwrap();
        let body_text = String::from_utf8_lossy(&body);
        assert!(
            !body_text.contains("STATIC_INDEX"),
            "/api/nonexistent must not serve static content; body: {body_text:?}"
        );
    }
}
