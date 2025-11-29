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
//!
//! let config = OnvifServerConfig {
//!     bind_address: "0.0.0.0".to_string(),
//!     port: 8080,
//!     request_timeout_secs: 30,
//!     max_body_size: 1024 * 1024, // 1MB
//!     enable_cors: false,
//!     http_verbose: false,        // Enable for detailed HTTP logging
//! };
//!
//! let server = OnvifServer::new(config)?;
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

use super::dispatcher::ServiceDispatcher;
use super::error::OnvifError;
use crate::logging::{HttpLogConfig, HttpLoggingMiddleware};

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
}

impl OnvifServer {
    /// Create a new ONVIF server with the given configuration.
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
        let (shutdown_tx, _) = broadcast::channel(1);

        Ok(Self {
            config,
            dispatcher,
            shutdown_tx,
        })
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

        let state = OnvifServerState {
            dispatcher: Arc::clone(&self.dispatcher),
            shutdown_tx: self.shutdown_tx.clone(),
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

        // Build the main router with middleware
        let app = Router::new()
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
            .with_state(state);

        app
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
        [(header::CONTENT_TYPE, "text/xml; charset=utf-8")],
        fault.to_soap_fault(),
    )
        .into_response()
}

/// Handler for Device Service requests.
async fn handle_device_service(
    State(state): State<OnvifServerState>,
    request: Request<Body>,
) -> Response {
    state
        .dispatcher
        .dispatch("device", request)
        .await
        .into_response()
}

/// Handler for Media Service requests.
async fn handle_media_service(
    State(state): State<OnvifServerState>,
    request: Request<Body>,
) -> Response {
    state
        .dispatcher
        .dispatch("media", request)
        .await
        .into_response()
}

/// Handler for PTZ Service requests.
async fn handle_ptz_service(
    State(state): State<OnvifServerState>,
    request: Request<Body>,
) -> Response {
    state
        .dispatcher
        .dispatch("ptz", request)
        .await
        .into_response()
}

/// Handler for Imaging Service requests.
async fn handle_imaging_service(
    State(state): State<OnvifServerState>,
    request: Request<Body>,
) -> Response {
    state
        .dispatcher
        .dispatch("imaging", request)
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
        };

        let cloned = state.clone();

        // Should be able to clone state
        assert!(Arc::ptr_eq(&state.dispatcher, &cloned.dispatcher));
    }
}
