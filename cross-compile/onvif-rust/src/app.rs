//! Main Application struct with lifecycle management.
//!
//! This module provides the `Application` struct which is the central entry point
//! for the ONVIF Rust application. It manages the complete lifecycle:
//! - `start()` - Ordered async initialization
//! - `run()` - Main event loop with signal handling
//! - `shutdown()` - Coordinated async cleanup
//!
//! # Design Principles
//!
//! - **No global state**: All state is owned by the `Application` struct
//! - **Explicit lifecycle**: No reliance on `Drop` for async cleanup
//! - **Dependency injection**: Components receive dependencies via constructors
//! - **Graceful degradation**: Optional components can fail without stopping the app

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::broadcast;
use tokio::task::JoinHandle;

use crate::config::ConfigRuntime;
use crate::lifecycle::health::{ComponentHealth, HealthStatus};
use crate::lifecycle::shutdown::{DEFAULT_SHUTDOWN_TIMEOUT, ShutdownCoordinator};
use crate::lifecycle::startup::{StartupPhase, StartupProgress};
use crate::lifecycle::{RuntimeError, ShutdownReport, StartupError};
use crate::onvif::ptz::PTZStateManager;
use crate::onvif::server::{OnvifServer, OnvifServerConfig};
use crate::platform::Platform;
use crate::users::password::PasswordManager;
use crate::users::storage::UserStorage;

// ============================================================================
// AppState - Shared application state for dependency injection
// ============================================================================

/// Shared application state for dependency injection.
///
/// This struct holds all shared dependencies that services need. It is designed
/// to be passed to the ONVIF server and used to construct service instances.
///
/// # Design
///
/// - All fields are `Arc`-wrapped for cheap cloning and shared ownership
/// - Optional `platform` field allows running without hardware access (testing)
/// - Builder pattern available via [`AppStateBuilder`] for flexible construction
///
/// # Example
///
/// ```ignore
/// use onvif_rust::app::AppState;
/// use std::sync::Arc;
///
/// let state = AppState::builder()
///     .user_storage(Arc::new(UserStorage::new("/tmp/users.json")?))
///     .password_manager(Arc::new(PasswordManager::new(10)?))
///     .ptz_state(Arc::new(PTZStateManager::new()))
///     .config(Arc::new(ConfigRuntime::new(Default::default())))
///     .build()?;
/// ```
#[derive(Clone)]
pub struct AppState {
    /// User storage for authentication.
    user_storage: Arc<UserStorage>,
    /// Password manager for credential handling.
    password_manager: Arc<PasswordManager>,
    /// PTZ state manager for PTZ operations.
    ptz_state: Arc<PTZStateManager>,
    /// Configuration runtime.
    config: Arc<ConfigRuntime>,
    /// Platform abstraction (optional for testing without hardware).
    platform: Option<Arc<dyn Platform>>,
}

impl AppState {
    /// Create a new builder for constructing `AppState`.
    pub fn builder() -> AppStateBuilder {
        AppStateBuilder::default()
    }

    /// Get a reference to the user storage.
    pub fn user_storage(&self) -> &Arc<UserStorage> {
        &self.user_storage
    }

    /// Get a reference to the password manager.
    pub fn password_manager(&self) -> &Arc<PasswordManager> {
        &self.password_manager
    }

    /// Get a reference to the PTZ state manager.
    pub fn ptz_state(&self) -> &Arc<PTZStateManager> {
        &self.ptz_state
    }

    /// Get a reference to the configuration runtime.
    pub fn config(&self) -> &Arc<ConfigRuntime> {
        &self.config
    }

    /// Get a reference to the platform abstraction, if available.
    pub fn platform(&self) -> Option<&Arc<dyn Platform>> {
        self.platform.as_ref()
    }
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState")
            .field("user_storage", &"Arc<UserStorage>")
            .field("password_manager", &"Arc<PasswordManager>")
            .field("ptz_state", &"Arc<PTZStateManager>")
            .field("config", &"Arc<ConfigRuntime>")
            .field(
                "platform",
                &self.platform.as_ref().map(|_| "Some(Arc<dyn Platform>)"),
            )
            .finish()
    }
}

// ============================================================================
// AppStateBuilder - Builder pattern for AppState construction
// ============================================================================

/// Builder for constructing [`AppState`] with optional components.
///
/// This builder allows flexible construction of application state, supporting
/// both full production configurations and minimal test configurations.
#[derive(Default)]
pub struct AppStateBuilder {
    user_storage: Option<Arc<UserStorage>>,
    password_manager: Option<Arc<PasswordManager>>,
    ptz_state: Option<Arc<PTZStateManager>>,
    config: Option<Arc<ConfigRuntime>>,
    platform: Option<Arc<dyn Platform>>,
}

/// Error type for AppState construction failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppStateError {
    /// Missing required component.
    MissingComponent(String),
}

impl std::fmt::Display for AppStateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppStateError::MissingComponent(name) => {
                write!(f, "Missing required component: {}", name)
            }
        }
    }
}

impl std::error::Error for AppStateError {}

impl AppStateBuilder {
    /// Set the user storage.
    pub fn user_storage(mut self, storage: Arc<UserStorage>) -> Self {
        self.user_storage = Some(storage);
        self
    }

    /// Set the password manager.
    pub fn password_manager(mut self, manager: Arc<PasswordManager>) -> Self {
        self.password_manager = Some(manager);
        self
    }

    /// Set the PTZ state manager.
    pub fn ptz_state(mut self, state: Arc<PTZStateManager>) -> Self {
        self.ptz_state = Some(state);
        self
    }

    /// Set the configuration runtime.
    pub fn config(mut self, config: Arc<ConfigRuntime>) -> Self {
        self.config = Some(config);
        self
    }

    /// Set the platform abstraction.
    pub fn platform(mut self, platform: Arc<dyn Platform>) -> Self {
        self.platform = Some(platform);
        self
    }

    /// Build the `AppState`, returning an error if required components are missing.
    pub fn build(self) -> Result<AppState, AppStateError> {
        Ok(AppState {
            user_storage: self
                .user_storage
                .ok_or_else(|| AppStateError::MissingComponent("user_storage".to_string()))?,
            password_manager: self
                .password_manager
                .ok_or_else(|| AppStateError::MissingComponent("password_manager".to_string()))?,
            ptz_state: self
                .ptz_state
                .ok_or_else(|| AppStateError::MissingComponent("ptz_state".to_string()))?,
            config: self
                .config
                .ok_or_else(|| AppStateError::MissingComponent("config".to_string()))?,
            platform: self.platform,
        })
    }
}

// ============================================================================
// Unit Tests for AppState
// ============================================================================

#[cfg(test)]
mod app_state_tests {
    use super::*;

    #[test]
    fn test_app_state_builder_missing_user_storage() {
        let result = AppState::builder()
            .password_manager(Arc::new(PasswordManager::new()))
            .ptz_state(Arc::new(PTZStateManager::new()))
            .config(Arc::new(ConfigRuntime::new(Default::default())))
            .build();

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            AppStateError::MissingComponent("user_storage".to_string())
        );
    }

    #[test]
    fn test_app_state_builder_missing_password_manager() {
        let storage = UserStorage::new();
        let result = AppState::builder()
            .user_storage(Arc::new(storage))
            .ptz_state(Arc::new(PTZStateManager::new()))
            .config(Arc::new(ConfigRuntime::new(Default::default())))
            .build();

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            AppStateError::MissingComponent("password_manager".to_string())
        );
    }

    #[test]
    fn test_app_state_builder_success_without_platform() {
        let storage = UserStorage::new();
        let result = AppState::builder()
            .user_storage(Arc::new(storage))
            .password_manager(Arc::new(PasswordManager::new()))
            .ptz_state(Arc::new(PTZStateManager::new()))
            .config(Arc::new(ConfigRuntime::new(Default::default())))
            .build();

        assert!(result.is_ok());
        let state = result.unwrap();
        assert!(state.platform().is_none());
    }

    #[test]
    fn test_app_state_clone() {
        let storage = UserStorage::new();
        let state = AppState::builder()
            .user_storage(Arc::new(storage))
            .password_manager(Arc::new(PasswordManager::new()))
            .ptz_state(Arc::new(PTZStateManager::new()))
            .config(Arc::new(ConfigRuntime::new(Default::default())))
            .build()
            .unwrap();

        let cloned = state.clone();
        // Arc::ptr_eq checks they point to the same allocation
        assert!(Arc::ptr_eq(state.user_storage(), cloned.user_storage()));
        assert!(Arc::ptr_eq(
            state.password_manager(),
            cloned.password_manager()
        ));
        assert!(Arc::ptr_eq(state.ptz_state(), cloned.ptz_state()));
        assert!(Arc::ptr_eq(state.config(), cloned.config()));
    }

    #[test]
    fn test_app_state_debug() {
        let storage = UserStorage::new();
        let state = AppState::builder()
            .user_storage(Arc::new(storage))
            .password_manager(Arc::new(PasswordManager::new()))
            .ptz_state(Arc::new(PTZStateManager::new()))
            .config(Arc::new(ConfigRuntime::new(Default::default())))
            .build()
            .unwrap();

        let debug_str = format!("{:?}", state);
        assert!(debug_str.contains("AppState"));
        assert!(debug_str.contains("user_storage"));
    }
}

/// Default configuration file path.
pub const DEFAULT_CONFIG_PATH: &str = "/etc/onvif/config.toml";

/// Capacity of the shutdown broadcast channel.
const SHUTDOWN_CHANNEL_CAPACITY: usize = 1;

/// Main application struct that owns all components and manages lifecycle.
///
/// # Example
///
/// ```ignore
/// use onvif_rust::app::Application;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     // Start the application
///     let app = Application::start("/etc/onvif/config.toml").await?;
///
///     // Run until shutdown signal
///     app.run().await?;
///
///     // Graceful shutdown
///     let report = app.shutdown().await;
///     println!("Shutdown completed: {:?}", report);
///
///     Ok(())
/// }
/// ```
pub struct Application {
    /// Timestamp when the application started.
    started_at: Instant,

    /// Shutdown coordinator for graceful termination.
    shutdown_coordinator: ShutdownCoordinator,

    /// Broadcast sender for shutdown signals.
    shutdown_tx: broadcast::Sender<()>,

    /// Services that are running in degraded mode.
    degraded_services: Vec<String>,

    /// Configuration path used to start the application.
    config_path: String,

    /// Application state with shared dependencies.
    app_state: Option<AppState>,

    /// HTTP server instance for controlled shutdown.
    server: Option<Arc<OnvifServer>>,

    /// Handle to the server task.
    server_task: Option<JoinHandle<()>>,
}

impl Application {
    /// Start the application with ordered initialization.
    ///
    /// This is the **only** way to create an `Application` instance. It performs
    /// initialization in the following order:
    ///
    /// 1. Load and validate configuration
    /// 2. Initialize platform abstraction
    /// 3. Initialize required services (Device, Media)
    /// 4. Initialize optional services (PTZ, Imaging) - continues on failure
    /// 5. Initialize network (HTTP server, WS-Discovery)
    ///
    /// # Arguments
    ///
    /// * `config_path` - Path to the TOML configuration file
    ///
    /// # Errors
    ///
    /// Returns `StartupError` if any required component fails to initialize.
    pub async fn start(config_path: &str) -> Result<Self, StartupError> {
        let started_at = Instant::now();
        let mut progress = StartupProgress::new();

        tracing::info!("Starting ONVIF application...");
        tracing::info!("Configuration path: {}", config_path);

        // Create shutdown channel
        let (shutdown_tx, _) = broadcast::channel(SHUTDOWN_CHANNEL_CAPACITY);
        let shutdown_coordinator =
            ShutdownCoordinator::new(shutdown_tx.clone(), DEFAULT_SHUTDOWN_TIMEOUT);

        // Phase 1: Configuration
        progress.begin_phase(StartupPhase::Configuration);
        // Load configuration from file or use defaults
        let app_config = crate::config::ConfigStorage::load_or_default(config_path)
            .map_err(|e| StartupError::Config(e.to_string()))?;
        let config_runtime = Arc::new(ConfigRuntime::new(app_config));
        progress.complete_phase();

        // Phase 2: Platform
        progress.begin_phase(StartupPhase::Platform);
        // Platform initialization is optional - we continue without it for testing
        tracing::debug!("Platform initialization placeholder - will be implemented in T034-T050");
        progress.complete_phase();

        // Phase 3: Services - Build AppState
        progress.begin_phase(StartupPhase::Services);
        tracing::debug!("Initializing ONVIF services...");

        // Create user storage with optional persistence
        let user_storage = UserStorage::new();
        // Try to load existing users from TOML file
        let users_path = std::path::Path::new(config_path)
            .parent()
            .unwrap_or(std::path::Path::new("/etc/onvif"))
            .join("users.toml");
        if users_path.exists() {
            if let Err(e) =
                user_storage.load_from_toml(users_path.to_str().unwrap_or("/etc/onvif/users.toml"))
            {
                tracing::warn!("Failed to load users from {}: {}", users_path.display(), e);
            } else {
                tracing::info!("Loaded users from {}", users_path.display());
            }
        }

        let app_state = AppState::builder()
            .user_storage(Arc::new(user_storage))
            .password_manager(Arc::new(PasswordManager::new()))
            .ptz_state(Arc::new(PTZStateManager::new()))
            .config(Arc::clone(&config_runtime))
            .build()
            .map_err(|e| StartupError::Services(e.to_string()))?;

        progress.complete_phase();

        // Phase 4: Network - Start HTTP Server
        progress.begin_phase(StartupPhase::Network);
        tracing::debug!("Starting HTTP server...");

        // Get HTTP settings from config or use defaults
        let bind_address = config_runtime
            .get_string("server.bind_address")
            .unwrap_or_else(|_| "0.0.0.0".to_string());
        let port = config_runtime.get_int("server.port").unwrap_or(8080) as u16;
        let request_timeout = config_runtime
            .get_int("server.request_timeout")
            .unwrap_or(30) as u64;
        let max_body_size = config_runtime
            .get_int("server.max_body_size")
            .unwrap_or(1024 * 1024) as usize;
        let http_verbose = config_runtime.get_bool("server.verbose").unwrap_or(false);

        let server_config = OnvifServerConfig {
            bind_address,
            port,
            request_timeout_secs: request_timeout,
            max_body_size,
            enable_cors: false,
            http_verbose,
        };

        let server = Arc::new(
            OnvifServer::with_app_state(server_config, app_state.clone())
                .map_err(|e| StartupError::Network(e.to_string()))?,
        );

        // Start the server in a background task
        let server_clone: Arc<OnvifServer> = Arc::clone(&server);
        let server_task = tokio::spawn(async move {
            if let Err(e) = server_clone.start().await {
                tracing::error!("HTTP server error: {}", e);
            }
        });

        progress.complete_phase();

        // Phase 5: Discovery
        progress.begin_phase(StartupPhase::Discovery);
        // TODO: Send WS-Discovery Hello when discovery module is implemented
        tracing::debug!("Discovery placeholder - will be implemented in T190-T203");
        progress.complete_phase();

        let startup_duration = started_at.elapsed();
        if progress.has_degraded_services() {
            tracing::warn!(
                "Application started in DEGRADED mode in {:?}. Unavailable services: {:?}",
                startup_duration,
                progress.degraded_services()
            );
        } else {
            tracing::info!("Application started successfully in {:?}", startup_duration);
        }

        Ok(Self {
            started_at,
            shutdown_coordinator,
            shutdown_tx,
            degraded_services: progress.degraded_services().to_vec(),
            config_path: config_path.to_string(),
            app_state: Some(app_state),
            server: Some(server),
            server_task: Some(server_task),
        })
    }

    /// Run the application until a shutdown signal is received.
    ///
    /// This method blocks until one of the following occurs:
    /// - SIGINT (Ctrl+C) is received
    /// - SIGTERM is received
    /// - An unrecoverable error occurs
    ///
    /// # Errors
    ///
    /// Returns `RuntimeError` if an unrecoverable error occurs during operation.
    pub async fn run(&self) -> Result<(), RuntimeError> {
        tracing::info!("Application running. Press Ctrl+C to stop.");

        // Wait for shutdown signal
        let ctrl_c = async {
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to install Ctrl+C handler");
        };

        #[cfg(unix)]
        let terminate = async {
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("Failed to install SIGTERM handler")
                .recv()
                .await;
        };

        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

        tokio::select! {
            _ = ctrl_c => {
                tracing::info!("Received SIGINT (Ctrl+C)");
            }
            _ = terminate => {
                tracing::info!("Received SIGTERM");
            }
        }

        Ok(())
    }

    /// Perform graceful shutdown of all components.
    ///
    /// This method shuts down components in reverse initialization order:
    ///
    /// 1. Send WS-Discovery Bye message
    /// 2. Stop accepting new HTTP connections
    /// 3. Broadcast shutdown signal to all tasks
    /// 4. Wait for in-flight requests (with timeout)
    /// 5. Shutdown services in reverse order
    /// 6. Shutdown platform
    ///
    /// # Returns
    ///
    /// A `ShutdownReport` containing details about the shutdown process.
    pub async fn shutdown(mut self) -> ShutdownReport {
        tracing::info!("Beginning graceful shutdown...");

        let mut report = self.shutdown_coordinator.initiate_shutdown().await;

        // Record component shutdown status
        // In a full implementation, each component would report its shutdown status

        // Phase 1: Discovery Bye (non-fatal)
        tracing::debug!("Sending WS-Discovery Bye...");
        report.record_success("discovery");

        // Phase 2: Network shutdown - Stop HTTP server
        tracing::debug!("Shutting down network services...");
        if let Some(server) = self.server.take() {
            server.shutdown();
            // Wait for server task to complete
            if let Some(task) = self.server_task.take() {
                let _ = tokio::time::timeout(Duration::from_secs(5), task).await;
            }
        }
        report.record_success("network");

        // Phase 3: Services shutdown (reverse order)
        tracing::debug!("Shutting down ONVIF services...");
        report.record_success("imaging");
        report.record_success("ptz");
        report.record_success("media");
        report.record_success("device");

        // Phase 4: Platform shutdown
        tracing::debug!("Shutting down platform...");
        report.record_success("platform");

        // Phase 5: Configuration cleanup
        tracing::debug!("Cleaning up configuration...");
        report.record_success("config");

        let total_duration = self.started_at.elapsed();
        tracing::info!(
            "Shutdown complete. Application ran for {:?}. Shutdown took {:?}",
            total_duration,
            report.duration
        );

        report
    }

    /// Get the current health status of the application.
    ///
    /// This can be used for health check endpoints (e.g., `/health`, `/ready`).
    pub fn health(&self) -> HealthStatus {
        let mut status = HealthStatus::new(self.started_at.elapsed());

        // Add component health
        status.add_component("config", ComponentHealth::healthy("Configuration"));
        status.add_component("platform", ComponentHealth::healthy("Platform"));
        status.add_component("device", ComponentHealth::healthy("Device Service"));
        status.add_component("media", ComponentHealth::healthy("Media Service"));

        // Mark degraded services
        for service in &self.degraded_services {
            status.mark_degraded(service);
            let component_key = service.to_lowercase();
            status.add_component(
                &component_key,
                ComponentHealth::degraded(service, "Initialization failed"),
            );
        }

        status
    }

    /// Get the application uptime.
    pub fn uptime(&self) -> Duration {
        self.started_at.elapsed()
    }

    /// Check if the application is running in degraded mode.
    pub fn is_degraded(&self) -> bool {
        !self.degraded_services.is_empty()
    }

    /// Get the list of degraded services.
    pub fn degraded_services(&self) -> &[String] {
        &self.degraded_services
    }

    /// Get a receiver for shutdown signals.
    ///
    /// Components can use this to be notified when shutdown is initiated.
    pub fn subscribe_shutdown(&self) -> broadcast::Receiver<()> {
        self.shutdown_tx.subscribe()
    }

    /// Get the configuration path used to start the application.
    pub fn config_path(&self) -> &str {
        &self.config_path
    }
}

// Note: We intentionally do NOT implement Drop with async cleanup.
// All async cleanup must be done via the explicit shutdown() method.
// Drop will only deallocate memory, which Rust handles automatically.

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_application_start() {
        let app = Application::start("/nonexistent/config.toml").await;
        assert!(app.is_ok());
    }

    #[tokio::test]
    async fn test_application_health() {
        let app = Application::start("/test/config.toml").await.unwrap();
        let health = app.health();

        assert!(health.is_ready());
        assert!(health.uptime > Duration::ZERO);
    }

    #[tokio::test]
    async fn test_application_uptime() {
        let app = Application::start("/test/config.toml").await.unwrap();

        // Wait a bit
        tokio::time::sleep(Duration::from_millis(10)).await;

        let uptime = app.uptime();
        assert!(uptime >= Duration::from_millis(10));
    }

    #[tokio::test]
    async fn test_application_shutdown() {
        let app = Application::start("/test/config.toml").await.unwrap();
        let report = app.shutdown().await;

        assert_eq!(report.status, crate::lifecycle::ShutdownStatus::Success);
        assert!(!report.successful_components.is_empty());
    }

    #[tokio::test]
    async fn test_application_subscribe_shutdown() {
        let app = Application::start("/test/config.toml").await.unwrap();
        let mut rx = app.subscribe_shutdown();

        // Spawn a task that will receive the shutdown signal
        let handle = tokio::spawn(async move {
            rx.recv().await.ok();
            true
        });

        // Give the task time to start
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Shutdown should send the signal
        let _report = app.shutdown().await;

        // Task should complete
        let result = tokio::time::timeout(Duration::from_millis(100), handle).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_application_config_path() {
        let app = Application::start("/custom/path/config.toml")
            .await
            .unwrap();
        assert_eq!(app.config_path(), "/custom/path/config.toml");
    }

    #[tokio::test]
    async fn test_application_not_degraded_by_default() {
        let app = Application::start("/test/config.toml").await.unwrap();
        assert!(!app.is_degraded());
        assert!(app.degraded_services().is_empty());
    }
}
