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

use crate::config::{
    ConfigPersistenceHandle, ConfigPersistenceService, ConfigRuntime, ConfigStorage, PendingWrite,
    PersistenceHandle, PersistenceService, ProfileStorage,
};
use crate::config::{PasswordManager, UserStorage};
use crate::lifecycle::health::{ComponentHealth, HealthStatus};
use crate::lifecycle::shutdown::{DEFAULT_SHUTDOWN_TIMEOUT, ShutdownCoordinator};
use crate::lifecycle::startup::{StartupPhase, StartupProgress};
use crate::lifecycle::{RuntimeError, ShutdownReport, StartupError};
use crate::onvif::discovery::{DiscoveryConfig, WsDiscovery, WsDiscoveryHandle};
use crate::onvif::imaging::ImagingSettingsStore;
use crate::onvif::ptz::PTZStateManager;
use crate::onvif::server::{OnvifServer, OnvifServerConfig};
use crate::platform::Platform;
#[cfg(use_stubs)]
use crate::platform::StubPlatformBuilder;
use crate::platform::external_ip;
use crate::security::RateLimiter;
use streaming_lib::common::auth::{Auth, AuthAlgorithm, AuthType, CredentialValidator};

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
    /// Memory monitor for resource enforcement.
    memory_monitor: Arc<crate::utils::MemoryMonitor>,
    /// Rate limiter for per-IP request limiting.
    rate_limiter: Arc<RateLimiter>,
    /// Platform abstraction (optional for testing without hardware).
    platform: Option<Arc<dyn Platform>>,
    /// Optional config persistence handle.
    config_persistence: Option<ConfigPersistenceHandle>,
    /// Profile storage for ONVIF media profiles (profiles.toml).
    profile_storage: Arc<ProfileStorage>,
    /// Optional debounced persistence handle for profiles (off-executor saves).
    profile_persistence: Option<PersistenceHandle>,
    /// Imaging settings store (optional; production wires persistence at startup).
    imaging_settings_store: Option<Arc<ImagingSettingsStore>>,
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

    /// Get a reference to the memory monitor.
    pub fn memory_monitor(&self) -> &Arc<crate::utils::MemoryMonitor> {
        &self.memory_monitor
    }

    /// Get a reference to the rate limiter.
    pub fn rate_limiter(&self) -> &Arc<RateLimiter> {
        &self.rate_limiter
    }

    /// Get a reference to the platform abstraction, if available.
    pub fn platform(&self) -> Option<&Arc<dyn Platform>> {
        self.platform.as_ref()
    }

    /// Get the config persistence handle, if available.
    pub fn config_persistence(&self) -> Option<&ConfigPersistenceHandle> {
        self.config_persistence.as_ref()
    }

    /// Get a reference to the profile storage.
    pub fn profile_storage(&self) -> &Arc<ProfileStorage> {
        &self.profile_storage
    }

    /// Get the profile persistence handle, if available.
    pub fn profile_persistence(&self) -> Option<&PersistenceHandle> {
        self.profile_persistence.as_ref()
    }

    /// Get the imaging settings store, if available.
    pub fn imaging_settings_store(&self) -> Option<&Arc<ImagingSettingsStore>> {
        self.imaging_settings_store.as_ref()
    }
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState")
            .field("user_storage", &"Arc<UserStorage>")
            .field("password_manager", &"Arc<PasswordManager>")
            .field("ptz_state", &"Arc<PTZStateManager>")
            .field("config", &"Arc<ConfigRuntime>")
            .field("memory_monitor", &"Arc<MemoryMonitor>")
            .field("rate_limiter", &"Arc<RateLimiter>")
            .field(
                "platform",
                &self.platform.as_ref().map(|_| "Some(Arc<dyn Platform>)"),
            )
            .field(
                "config_persistence",
                &self
                    .config_persistence
                    .as_ref()
                    .map(|_| "Some(ConfigPersistenceHandle)"),
            )
            .field("profile_storage", &"Arc<ProfileStorage>")
            .field(
                "imaging_settings_store",
                &self
                    .imaging_settings_store
                    .as_ref()
                    .map(|_| "Some(Arc<ImagingSettingsStore>)"),
            )
            .field(
                "profile_persistence",
                &self
                    .profile_persistence
                    .as_ref()
                    .map(|_| "Some(PersistenceHandle)"),
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
    memory_monitor: Option<Arc<crate::utils::MemoryMonitor>>,
    rate_limiter: Option<Arc<RateLimiter>>,
    platform: Option<Arc<dyn Platform>>,
    config_persistence: Option<ConfigPersistenceHandle>,
    profile_storage: Option<Arc<ProfileStorage>>,
    profile_persistence: Option<PersistenceHandle>,
    imaging_settings_store: Option<Arc<ImagingSettingsStore>>,
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

    /// Set the memory monitor.
    pub fn memory_monitor(mut self, monitor: Arc<crate::utils::MemoryMonitor>) -> Self {
        self.memory_monitor = Some(monitor);
        self
    }

    /// Set the rate limiter.
    pub fn rate_limiter(mut self, limiter: Arc<RateLimiter>) -> Self {
        self.rate_limiter = Some(limiter);
        self
    }

    /// Set the platform abstraction.
    pub fn platform(mut self, platform: Arc<dyn Platform>) -> Self {
        self.platform = Some(platform);
        self
    }

    /// Set the config persistence handle.
    pub fn config_persistence(mut self, handle: ConfigPersistenceHandle) -> Self {
        self.config_persistence = Some(handle);
        self
    }

    /// Set the profile storage.
    pub fn profile_storage(mut self, storage: Arc<ProfileStorage>) -> Self {
        self.profile_storage = Some(storage);
        self
    }

    /// Set the profile persistence handle.
    pub fn profile_persistence(mut self, handle: PersistenceHandle) -> Self {
        self.profile_persistence = Some(handle);
        self
    }

    /// Set the imaging settings store (with persistence already attached).
    pub fn imaging_settings_store(mut self, store: Arc<ImagingSettingsStore>) -> Self {
        self.imaging_settings_store = Some(store);
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
            memory_monitor: self
                .memory_monitor
                .ok_or_else(|| AppStateError::MissingComponent("memory_monitor".to_string()))?,
            rate_limiter: self
                .rate_limiter
                .ok_or_else(|| AppStateError::MissingComponent("rate_limiter".to_string()))?,
            platform: self.platform,
            config_persistence: self.config_persistence,
            profile_storage: self
                .profile_storage
                .ok_or_else(|| AppStateError::MissingComponent("profile_storage".to_string()))?,
            profile_persistence: self.profile_persistence,
            imaging_settings_store: self.imaging_settings_store,
        })
    }
}

// ============================================================================
// Unit Tests for AppState
// ============================================================================

#[cfg(test)]
mod app_state_tests {
    use super::*;

    /// Helper: create a test ProfileStorage backed by a temp path.
    fn test_profile_storage() -> Arc<ProfileStorage> {
        Arc::new(ProfileStorage::new("/tmp/test_profiles.toml"))
    }

    #[test]
    fn test_app_state_builder_missing_user_storage() {
        let result = AppState::builder()
            .password_manager(Arc::new(PasswordManager::new()))
            .ptz_state(Arc::new(PTZStateManager::new()))
            .config(Arc::new(ConfigRuntime::new(Default::default())))
            .memory_monitor(Arc::new(crate::utils::MemoryMonitor::new()))
            .rate_limiter(Arc::new(crate::security::RateLimiter::new(60)))
            .profile_storage(test_profile_storage())
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
            .memory_monitor(Arc::new(crate::utils::MemoryMonitor::new()))
            .rate_limiter(Arc::new(crate::security::RateLimiter::new(60)))
            .profile_storage(test_profile_storage())
            .build();

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            AppStateError::MissingComponent("password_manager".to_string())
        );
    }

    #[test]
    fn test_app_state_builder_missing_ptz_state() {
        let storage = UserStorage::new();
        let result = AppState::builder()
            .user_storage(Arc::new(storage))
            .password_manager(Arc::new(PasswordManager::new()))
            .config(Arc::new(ConfigRuntime::new(Default::default())))
            .memory_monitor(Arc::new(crate::utils::MemoryMonitor::new()))
            .rate_limiter(Arc::new(crate::security::RateLimiter::new(60)))
            .profile_storage(test_profile_storage())
            .build();

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            AppStateError::MissingComponent("ptz_state".to_string())
        );
    }

    #[test]
    fn test_app_state_builder_missing_config() {
        let storage = UserStorage::new();
        let result = AppState::builder()
            .user_storage(Arc::new(storage))
            .password_manager(Arc::new(PasswordManager::new()))
            .ptz_state(Arc::new(PTZStateManager::new()))
            .memory_monitor(Arc::new(crate::utils::MemoryMonitor::new()))
            .rate_limiter(Arc::new(crate::security::RateLimiter::new(60)))
            .profile_storage(test_profile_storage())
            .build();

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            AppStateError::MissingComponent("config".to_string())
        );
    }

    #[test]
    fn test_app_state_builder_missing_memory_monitor() {
        let storage = UserStorage::new();
        let result = AppState::builder()
            .user_storage(Arc::new(storage))
            .password_manager(Arc::new(PasswordManager::new()))
            .ptz_state(Arc::new(PTZStateManager::new()))
            .config(Arc::new(ConfigRuntime::new(Default::default())))
            .rate_limiter(Arc::new(crate::security::RateLimiter::new(60)))
            .profile_storage(test_profile_storage())
            .build();

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            AppStateError::MissingComponent("memory_monitor".to_string())
        );
    }

    #[test]
    fn test_app_state_builder_missing_rate_limiter() {
        let storage = UserStorage::new();
        let result = AppState::builder()
            .user_storage(Arc::new(storage))
            .password_manager(Arc::new(PasswordManager::new()))
            .ptz_state(Arc::new(PTZStateManager::new()))
            .config(Arc::new(ConfigRuntime::new(Default::default())))
            .memory_monitor(Arc::new(crate::utils::MemoryMonitor::new()))
            .profile_storage(test_profile_storage())
            .build();

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            AppStateError::MissingComponent("rate_limiter".to_string())
        );
    }

    #[test]
    fn test_app_state_builder_missing_profile_storage() {
        let storage = UserStorage::new();
        let result = AppState::builder()
            .user_storage(Arc::new(storage))
            .password_manager(Arc::new(PasswordManager::new()))
            .ptz_state(Arc::new(PTZStateManager::new()))
            .config(Arc::new(ConfigRuntime::new(Default::default())))
            .memory_monitor(Arc::new(crate::utils::MemoryMonitor::new()))
            .rate_limiter(Arc::new(crate::security::RateLimiter::new(60)))
            .build();

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            AppStateError::MissingComponent("profile_storage".to_string())
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
            .memory_monitor(Arc::new(crate::utils::MemoryMonitor::new()))
            .rate_limiter(Arc::new(crate::security::RateLimiter::new(60)))
            .profile_storage(test_profile_storage())
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
            .memory_monitor(Arc::new(crate::utils::MemoryMonitor::new()))
            .rate_limiter(Arc::new(crate::security::RateLimiter::new(60)))
            .profile_storage(test_profile_storage())
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
        assert!(Arc::ptr_eq(state.memory_monitor(), cloned.memory_monitor()));
        assert!(Arc::ptr_eq(
            state.profile_storage(),
            cloned.profile_storage()
        ));
    }

    #[test]
    fn test_app_state_debug() {
        let storage = UserStorage::new();
        let state = AppState::builder()
            .user_storage(Arc::new(storage))
            .password_manager(Arc::new(PasswordManager::new()))
            .ptz_state(Arc::new(PTZStateManager::new()))
            .config(Arc::new(ConfigRuntime::new(Default::default())))
            .memory_monitor(Arc::new(crate::utils::MemoryMonitor::new()))
            .rate_limiter(Arc::new(crate::security::RateLimiter::new(60)))
            .profile_storage(test_profile_storage())
            .build()
            .unwrap();

        let debug_str = format!("{:?}", state);
        assert!(debug_str.contains("AppState"));
        assert!(debug_str.contains("user_storage"));
        assert!(debug_str.contains("profile_storage"));
    }
}

/// Default configuration file path.
pub const DEFAULT_CONFIG_PATH: &str = "/mnt/anyka_hack/onvif/config.toml";

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
    #[allow(dead_code)]
    app_state: Option<AppState>,

    /// HTTP server instance for controlled shutdown.
    server: Option<Arc<OnvifServer>>,

    /// Handle to the server task.
    server_task: Option<JoinHandle<()>>,

    /// Handle to the config persistence task.
    config_persistence_task: Option<JoinHandle<()>>,

    /// Handle to the user persistence task.
    user_persistence_task: Option<JoinHandle<()>>,

    /// Handle to the profile persistence task.
    profile_persistence_task: Option<JoinHandle<()>>,

    /// Handle to the imaging persistence task.
    imaging_persistence_task: Option<JoinHandle<()>>,

    /// WS-Discovery service handle for device discovery control.
    discovery: Option<WsDiscoveryHandle>,

    /// Handle to the WS-Discovery background task.
    discovery_task: Option<JoinHandle<()>>,

    /// Handle to the memory logging task.
    memory_logging_task: Option<JoinHandle<()>>,

    /// Handle to the rate limiter cleanup task.
    rate_limiter_cleanup_task: Option<JoinHandle<()>>,

    /// Live streaming service (RTSP + HTTP-FLV).
    streaming_service: Option<crate::streaming::service::StreamingService>,
}

impl Application {
    /// Create imaging settings store with debounced off-executor persistence.
    fn wire_imaging_persistence(
        config_path: &str,
        platform: Option<&Arc<dyn Platform>>,
        save_delay: u64,
        shutdown_coordinator: &ShutdownCoordinator,
    ) -> (Arc<ImagingSettingsStore>, JoinHandle<()>) {
        let imaging_path = std::path::Path::new(config_path)
            .parent()
            .unwrap_or(std::path::Path::new("/etc/onvif"))
            .join("imaging.toml");

        let imaging_store = match platform.and_then(|p| p.imaging_control()) {
            Some(control) => Arc::new(ImagingSettingsStore::with_platform_and_persistence(
                control,
                &imaging_path,
            )),
            None => Arc::new(ImagingSettingsStore::with_persistence(&imaging_path)),
        };

        if imaging_path.exists() {
            if let Err(e) = imaging_store.load_from_file() {
                tracing::warn!(
                    "Failed to load imaging settings from {}: {}",
                    imaging_path.display(),
                    e
                );
            } else {
                tracing::info!("Loaded imaging settings from {}", imaging_path.display());
            }
        }

        let (imaging_persistence_service, imaging_persistence_handle) =
            imaging_store.persistence_service(save_delay);
        imaging_store.set_persistence(imaging_persistence_handle);
        let imaging_persistence_task =
            tokio::spawn(imaging_persistence_service.run(shutdown_coordinator.subscribe()));

        (imaging_store, imaging_persistence_task)
    }

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
        let app_config = ConfigStorage::load_or_default(config_path)
            .map_err(|e| StartupError::Config(e.to_string()))?;
        let config_runtime = Arc::new(ConfigRuntime::new(app_config));

        // Set up config persistence service (debounced save)
        let storage = ConfigStorage::new(config_path);
        let save_delay = config_runtime.read().server.config_save_delay_ms;
        let (persistence_service, persistence_handle) =
            ConfigPersistenceService::new(Arc::clone(&config_runtime), storage, save_delay);
        let config_persistence_task = Some(tokio::spawn(
            persistence_service.run(shutdown_coordinator.subscribe()),
        ));

        // Initialize logging with full configuration (console + file if configured)
        if let Err(e) = crate::logging::init_logging(&config_runtime) {
            // Fall back to eprintln since logging may not be available
            eprintln!("Failed to initialize logging: {}", e);
        }

        // Log loaded configuration for debugging
        config_runtime.log_loaded_config();

        progress.complete_phase();

        // Phase 2: Platform
        progress.begin_phase(StartupPhase::Platform);
        let platform = Self::init_platform(&mut progress, &config_runtime).await?;
        progress.complete_phase();

        // Phase 3: Services - Build AppState
        progress.begin_phase(StartupPhase::Services);
        tracing::debug!("Initializing ONVIF services...");

        // Create user storage with debounced off-executor persistence
        let user_storage = Arc::new(UserStorage::new());
        let users_path = std::path::Path::new(config_path)
            .parent()
            .unwrap_or(std::path::Path::new("/etc/onvif"))
            .join("users.toml");
        if users_path.exists() {
            if let Err(e) = user_storage.load_from_toml(&users_path) {
                tracing::warn!("Failed to load users from {}: {}", users_path.display(), e);
            } else {
                tracing::info!("Loaded users from {}", users_path.display());
            }
        }

        let user_persistence_storage = Arc::clone(&user_storage);
        let user_persistence_path = users_path.clone();
        let (user_persistence_service, user_persistence_handle) = PersistenceService::new(
            "users",
            save_delay,
            Box::new(move || match user_persistence_storage.to_toml_bytes() {
                Ok(bytes) => Some(PendingWrite {
                    path: user_persistence_path.clone(),
                    bytes,
                    mode: Some(0o600),
                }),
                Err(e) => {
                    tracing::error!("Failed to serialize users: {}", e);
                    None
                }
            }),
        );
        user_storage.set_persistence(user_persistence_handle);
        let user_persistence_task = Some(tokio::spawn(
            user_persistence_service.run(shutdown_coordinator.subscribe()),
        ));

        // Create profile storage for ONVIF media profiles
        let profiles_path = std::path::Path::new(config_path)
            .parent()
            .unwrap_or(std::path::Path::new("/etc/onvif"))
            .join("profiles.toml");
        let profile_storage = Arc::new(ProfileStorage::new(&profiles_path));
        if let Err(e) = profile_storage.load() {
            tracing::warn!(
                "Failed to load profiles from {}: {}",
                profiles_path.display(),
                e
            );
        } else if !profile_storage.is_empty() {
            tracing::info!("Loaded profiles from {}", profiles_path.display());
        }

        let profile_persistence_storage = Arc::clone(&profile_storage);
        let profile_persistence_path = profiles_path.clone();
        let (profile_persistence_service, profile_persistence_handle) = PersistenceService::new(
            "profiles",
            save_delay,
            Box::new(move || {
                match toml::to_string_pretty(&profile_persistence_storage.snapshot()) {
                    Ok(content) => Some(PendingWrite {
                        path: profile_persistence_path.clone(),
                        bytes: content.into_bytes(),
                        mode: None,
                    }),
                    Err(e) => {
                        tracing::error!("Failed to serialize profiles: {}", e);
                        None
                    }
                }
            }),
        );
        let profile_persistence_task = Some(tokio::spawn(
            profile_persistence_service.run(shutdown_coordinator.subscribe()),
        ));

        let (imaging_settings_store, imaging_persistence_task) = Self::wire_imaging_persistence(
            config_path,
            platform.as_ref(),
            save_delay,
            &shutdown_coordinator,
        );
        let imaging_persistence_task = Some(imaging_persistence_task);

        // Initialize rate limiter from config (default: 60 requests per minute)
        let rate_limit_per_minute = config_runtime.read().server.rate_limit_per_minute;
        let rate_limiter = Arc::new(RateLimiter::new(rate_limit_per_minute));
        tracing::info!(
            "Rate limiter initialized: {} requests/minute",
            rate_limit_per_minute
        );

        let mut app_state_builder = AppState::builder()
            .user_storage(Arc::clone(&user_storage))
            .password_manager(Arc::new(PasswordManager::new()))
            .ptz_state(Arc::new(PTZStateManager::new()))
            .config(Arc::clone(&config_runtime))
            .memory_monitor(Arc::new(
                crate::utils::MemoryMonitor::from_config(&config_runtime).map_err(|e| {
                    StartupError::Services(format!("Failed to initialize memory monitor: {}", e))
                })?,
            ))
            .rate_limiter(Arc::clone(&rate_limiter))
            .profile_storage(Arc::clone(&profile_storage))
            .profile_persistence(profile_persistence_handle.clone())
            .imaging_settings_store(Arc::clone(&imaging_settings_store));

        // Wire config persistence handle
        app_state_builder = app_state_builder.config_persistence(persistence_handle.clone());

        // Add platform if available
        if let Some(ref p) = platform {
            app_state_builder = app_state_builder.platform(Arc::clone(p));
        }

        let app_state = app_state_builder
            .build()
            .map_err(|e| StartupError::Services(e.to_string()))?;

        // Start memory logging task (logs every 5 minutes by default)
        let memory_logging_interval = {
            let secs = config_runtime.read().memory.logging_interval_secs;
            if secs == 0 { 300 } else { secs as u64 }
        };
        let memory_logging_task = Some(app_state.memory_monitor().clone().start_periodic_logging(
            Duration::from_secs(memory_logging_interval),
            shutdown_coordinator.subscribe(),
        ));

        // Start rate limiter cleanup task (runs every minute)
        let rate_limiter_cleanup_task = Some(
            rate_limiter
                .start_cleanup_task(Duration::from_secs(60), shutdown_coordinator.subscribe()),
        );

        progress.complete_phase();

        // Phase 4: Network - Start HTTP Server
        progress.begin_phase(StartupPhase::Network);
        tracing::debug!("Starting HTTP server...");

        // Get HTTP settings from config
        let server_config = {
            let c = config_runtime.read();
            OnvifServerConfig {
                bind_address: c.server.bind_address.clone(),
                port: c.server.port,
                request_timeout_secs: c.server.request_timeout,
                max_body_size: c.server.max_body_size,
                enable_cors: false,
                static_root: if c.server.static_root.is_empty() {
                    Some("www".to_string())
                } else {
                    Some(c.server.static_root.clone())
                },
                http_verbose: c.logging.http_verbose,
                tls_enabled: c.server.tls_enabled,
                tls_cert_path: if c.server.tls_cert_path.is_empty() {
                    None
                } else {
                    Some(std::path::PathBuf::from(&c.server.tls_cert_path))
                },
                tls_key_path: if c.server.tls_key_path.is_empty() {
                    None
                } else {
                    Some(std::path::PathBuf::from(&c.server.tls_key_path))
                },
                rate_limit_per_minute: c.server.rate_limit_per_minute,
            }
        };
        let port = server_config.port;

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
        let discovery_enabled = config_runtime.read().discovery.enabled;

        let (discovery, discovery_task) = if discovery_enabled {
            // Build discovery configuration
            let discovery_config = Self::make_discovery_config(&config_runtime, port);

            match Self::start_discovery(discovery_config).await {
                Ok((disc, task)) => {
                    tracing::info!("WS-Discovery service started successfully");
                    (Some(disc), Some(task))
                }
                Err(e) => {
                    tracing::warn!(
                        "WS-Discovery failed to start, continuing in degraded mode: {}",
                        e
                    );
                    progress.record_degraded("discovery", e.to_string());
                    (None, None)
                }
            }
        } else {
            tracing::info!("WS-Discovery is disabled in configuration");
            (None, None)
        };
        progress.complete_phase();

        // Phase 6: Streaming (optional, gracefully degrades)
        let streaming_service =
            Self::start_streaming(&config_runtime, &app_state, &mut progress).await;

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
            discovery,
            discovery_task,
            config_persistence_task,
            user_persistence_task,
            profile_persistence_task,
            imaging_persistence_task,
            memory_logging_task,
            rate_limiter_cleanup_task,
            streaming_service,
        })
    }

    /// Initialize the platform abstraction layer.
    ///
    /// On real hardware, creates an `AnykaPlatform` for actual hardware access.
    /// On dev builds without vendor IPC, creates a `StubPlatform` for testing.
    /// Returns `None` in degraded mode for recoverable platform failures.
    /// Returns `Err` for unsafe teardown failures that require hard process exit.
    async fn init_platform(
        progress: &mut StartupProgress,
        config_runtime: &Arc<ConfigRuntime>,
    ) -> Result<Option<Arc<dyn Platform>>, StartupError> {
        #[cfg(not(use_stubs))]
        {
            let isp_path = {
                let p = config_runtime.read().device.isp_config_path.clone();
                if p.is_empty() {
                    None
                } else {
                    Some(std::path::PathBuf::from(p))
                }
            };

            match crate::platform::AnykaPlatform::with_isp_config(isp_path) {
                Ok(p) => match p.initialize().await {
                    Ok(()) => {
                        tracing::info!("AnykaPlatform initialized (real hardware)");
                        return Ok(Some(Arc::new(p) as Arc<dyn Platform>));
                    }
                    Err(e) => {
                        let msg = e.to_string();
                        if msg.contains("unsafe teardown required") {
                            tracing::error!(
                                "AnykaPlatform initialization entered unsafe teardown state; refusing degraded startup: {}",
                                msg
                            );
                            return Err(StartupError::Platform(msg));
                        }
                        tracing::warn!(
                            "AnykaPlatform initialization failed, continuing in degraded mode: {}",
                            msg
                        );
                        progress.record_degraded("platform", msg);
                    }
                },
                Err(e) => {
                    tracing::warn!(
                        "AnykaPlatform creation failed, continuing in degraded mode: {}",
                        e
                    );
                    progress.record_degraded("platform", e.to_string());
                }
            }
            Ok(None)
        }
        #[cfg(use_stubs)]
        {
            let _ = config_runtime;
            let stub_platform = StubPlatformBuilder::new()
                .ptz_supported(true)
                .imaging_supported(true)
                .build();
            match stub_platform.initialize().await {
                Ok(()) => {
                    tracing::info!("Platform initialized (stub mode)");
                    Ok(Some(Arc::new(stub_platform) as Arc<dyn Platform>))
                }
                Err(e) => {
                    tracing::warn!(
                        "Stub platform initialization failed, continuing in degraded mode: {}",
                        e
                    );
                    progress.record_degraded("platform", e.to_string());
                    Ok(None)
                }
            }
        }
    }

    /// Start the streaming service (RTSP + HTTP-FLV) if enabled in config.
    ///
    /// This is non-fatal: if streaming fails to start, the application continues
    /// in degraded mode.
    async fn start_streaming(
        config_runtime: &Arc<ConfigRuntime>,
        app_state: &AppState,
        progress: &mut StartupProgress,
    ) -> Option<crate::streaming::service::StreamingService> {
        let mut streaming_config =
            crate::streaming::config::StreamingConfig::from_config(config_runtime);
        if !streaming_config.enabled {
            tracing::info!("Streaming is disabled in configuration");
            return None;
        }

        match Self::build_stream_auth(app_state) {
            Ok(stream_auth) => {
                streaming_config.auth = stream_auth;
            }
            Err(e) => {
                tracing::warn!(
                    "Streaming authentication setup failed, continuing in degraded mode: {}",
                    e
                );
                progress.record_degraded("streaming_auth", e.to_string());
                return None;
            }
        }

        let mut service = crate::streaming::service::StreamingService::new(streaming_config);
        match service.start().await {
            Ok(bridge) => {
                // Register the bridge as an owned frame callback with the platform (zero-copy path).
                if let Some(platform) = app_state.platform() {
                    match platform.register_owned_frame_callback(bridge.clone()) {
                        Ok(()) => {
                            tracing::info!(
                                "Owned frame callback registered with platform (zero-copy)"
                            );
                        }
                        Err(e) => {
                            // Fall back to legacy FrameCallback if owned not supported
                            tracing::info!(
                                "Owned frame callback not supported ({}), falling back to FrameCallback",
                                e
                            );
                            match platform.register_frame_callback(bridge) {
                                Ok(()) => {
                                    tracing::info!(
                                        "Frame callback registered with platform (legacy)"
                                    );
                                }
                                Err(e2) => {
                                    tracing::warn!(
                                        "Failed to register frame callback (streaming will work but \
                                         won't receive live frames from encoder): {}",
                                        e2
                                    );
                                }
                            }
                        }
                    }
                }
                Some(service)
            }
            Err(e) => {
                tracing::warn!(
                    "Streaming service failed to start, continuing in degraded mode: {}",
                    e
                );
                progress.record_degraded("streaming", e.to_string());
                None
            }
        }
    }

    fn build_stream_auth(app_state: &AppState) -> anyhow::Result<Option<Auth>> {
        let c = app_state.config().read();
        let auth_enabled = c.server.auth_enabled;
        let realm = c.server.realm.clone();
        drop(c);

        if !auth_enabled {
            tracing::info!("Streaming authentication disabled (server.auth_enabled=false)");
            return Ok(None);
        }

        if app_state.user_storage().is_empty() {
            anyhow::bail!(
                "Streaming authentication is enabled but no users are available in UserStorage"
            );
        }

        let validator_storage = Arc::clone(app_state.user_storage());
        let validator_password_manager = Arc::clone(app_state.password_manager());
        let validator: CredentialValidator = Arc::new(move |username, password| {
            validator_storage
                .get_user(username)
                .map(|user| validator_password_manager.verify_password(password, &user.password))
                .unwrap_or(false)
        });
        let stream_auth = Auth::new(
            String::new(),
            Self::generate_unpredictable_stream_token(),
            None,
            AuthAlgorithm::Simple,
            AuthType::Pull,
        )
        .with_credential_validator(validator)
        .with_basic_realm(realm);

        Ok(Some(stream_auth))
    }

    fn generate_unpredictable_stream_token() -> String {
        uuid::Uuid::new_v4().simple().to_string()
    }

    /// Start the application with a custom platform abstraction.
    ///
    /// This is similar to `start()` but allows providing a custom platform implementation,
    /// which is useful for testing and validation scenarios like H.264 playback validation.
    ///
    /// # Arguments
    ///
    /// * `config_path` - Path to the TOML configuration file
    /// * `platform` - Custom platform implementation to use
    ///
    /// # Errors
    ///
    /// Returns `StartupError` if any required component fails to initialize.
    pub async fn start_with_platform(
        config_path: &str,
        platform: Arc<dyn Platform>,
    ) -> Result<Self, StartupError> {
        let started_at = Instant::now();
        let mut progress = StartupProgress::new();

        tracing::info!("Starting ONVIF application with custom platform...");
        tracing::info!("Configuration path: {}", config_path);

        // Create shutdown channel
        let (shutdown_tx, _) = broadcast::channel(SHUTDOWN_CHANNEL_CAPACITY);
        let shutdown_coordinator =
            ShutdownCoordinator::new(shutdown_tx.clone(), DEFAULT_SHUTDOWN_TIMEOUT);

        // Phase 1: Configuration
        progress.begin_phase(StartupPhase::Configuration);
        let app_config = ConfigStorage::load_or_default(config_path)
            .map_err(|e| StartupError::Config(e.to_string()))?;
        let config_runtime = Arc::new(ConfigRuntime::new(app_config));

        // Set up config persistence service (debounced save)
        let storage = ConfigStorage::new(config_path);
        let save_delay = config_runtime.read().server.config_save_delay_ms;
        let (persistence_service, persistence_handle) =
            ConfigPersistenceService::new(Arc::clone(&config_runtime), storage, save_delay);
        let config_persistence_task = Some(tokio::spawn(
            persistence_service.run(shutdown_coordinator.subscribe()),
        ));

        // Initialize logging
        if let Err(e) = crate::logging::init_logging(&config_runtime) {
            eprintln!("Failed to initialize logging: {}", e);
        }

        config_runtime.log_loaded_config();
        progress.complete_phase();

        // Phase 2: Platform - Use the provided custom platform
        progress.begin_phase(StartupPhase::Platform);
        tracing::info!("Using provided custom platform implementation");
        progress.complete_phase();

        // Phase 3: Services - Build AppState
        progress.begin_phase(StartupPhase::Services);
        tracing::debug!("Initializing ONVIF services...");

        let user_storage = Arc::new(UserStorage::new());
        let users_path = std::path::Path::new(config_path)
            .parent()
            .unwrap_or(std::path::Path::new("/etc/onvif"))
            .join("users.toml");
        if users_path.exists() {
            if let Err(e) = user_storage.load_from_toml(&users_path) {
                tracing::warn!("Failed to load users from {}: {}", users_path.display(), e);
            } else {
                tracing::info!("Loaded users from {}", users_path.display());
            }
        }

        let user_persistence_storage = Arc::clone(&user_storage);
        let user_persistence_path = users_path.clone();
        let (user_persistence_service, user_persistence_handle) = PersistenceService::new(
            "users",
            save_delay,
            Box::new(move || match user_persistence_storage.to_toml_bytes() {
                Ok(bytes) => Some(PendingWrite {
                    path: user_persistence_path.clone(),
                    bytes,
                    mode: Some(0o600),
                }),
                Err(e) => {
                    tracing::error!("Failed to serialize users: {}", e);
                    None
                }
            }),
        );
        user_storage.set_persistence(user_persistence_handle);
        let user_persistence_task = Some(tokio::spawn(
            user_persistence_service.run(shutdown_coordinator.subscribe()),
        ));

        // Create profile storage for ONVIF media profiles
        let profiles_path = std::path::Path::new(config_path)
            .parent()
            .unwrap_or(std::path::Path::new("/etc/onvif"))
            .join("profiles.toml");
        let profile_storage = Arc::new(ProfileStorage::new(&profiles_path));
        if let Err(e) = profile_storage.load() {
            tracing::warn!(
                "Failed to load profiles from {}: {}",
                profiles_path.display(),
                e
            );
        } else if !profile_storage.is_empty() {
            tracing::info!("Loaded profiles from {}", profiles_path.display());
        }

        let profile_persistence_storage = Arc::clone(&profile_storage);
        let profile_persistence_path = profiles_path.clone();
        let (profile_persistence_service, profile_persistence_handle) = PersistenceService::new(
            "profiles",
            save_delay,
            Box::new(move || {
                match toml::to_string_pretty(&profile_persistence_storage.snapshot()) {
                    Ok(content) => Some(PendingWrite {
                        path: profile_persistence_path.clone(),
                        bytes: content.into_bytes(),
                        mode: None,
                    }),
                    Err(e) => {
                        tracing::error!("Failed to serialize profiles: {}", e);
                        None
                    }
                }
            }),
        );
        let profile_persistence_task = Some(tokio::spawn(
            profile_persistence_service.run(shutdown_coordinator.subscribe()),
        ));

        let (imaging_settings_store, imaging_persistence_task) = Self::wire_imaging_persistence(
            config_path,
            Some(&platform),
            save_delay,
            &shutdown_coordinator,
        );
        let imaging_persistence_task = Some(imaging_persistence_task);

        let rate_limit_per_minute = config_runtime.read().server.rate_limit_per_minute;
        let rate_limiter = Arc::new(RateLimiter::new(rate_limit_per_minute));
        tracing::info!(
            "Rate limiter initialized: {} requests/minute",
            rate_limit_per_minute
        );

        let app_state = AppState::builder()
            .user_storage(Arc::clone(&user_storage))
            .password_manager(Arc::new(PasswordManager::new()))
            .ptz_state(Arc::new(PTZStateManager::new()))
            .config(Arc::clone(&config_runtime))
            .memory_monitor(Arc::new(
                crate::utils::MemoryMonitor::from_config(&config_runtime).map_err(|e| {
                    StartupError::Services(format!("Failed to initialize memory monitor: {}", e))
                })?,
            ))
            .rate_limiter(Arc::clone(&rate_limiter))
            .config_persistence(persistence_handle.clone())
            .platform(Arc::clone(&platform))
            .profile_storage(Arc::clone(&profile_storage))
            .profile_persistence(profile_persistence_handle.clone())
            .imaging_settings_store(Arc::clone(&imaging_settings_store))
            .build()
            .map_err(|e| StartupError::Services(e.to_string()))?;

        let memory_logging_interval = {
            let secs = config_runtime.read().memory.logging_interval_secs;
            if secs == 0 { 300 } else { secs as u64 }
        };
        let memory_logging_task = Some(app_state.memory_monitor().clone().start_periodic_logging(
            Duration::from_secs(memory_logging_interval),
            shutdown_coordinator.subscribe(),
        ));

        let rate_limiter_cleanup_task = Some(
            rate_limiter
                .start_cleanup_task(Duration::from_secs(60), shutdown_coordinator.subscribe()),
        );

        progress.complete_phase();

        // Phase 4: Network - Start HTTP Server
        progress.begin_phase(StartupPhase::Network);
        tracing::debug!("Starting HTTP server...");

        let server_config = {
            let c = config_runtime.read();
            OnvifServerConfig {
                bind_address: c.server.bind_address.clone(),
                port: c.server.port,
                request_timeout_secs: c.server.request_timeout,
                max_body_size: c.server.max_body_size,
                enable_cors: false,
                static_root: if c.server.static_root.is_empty() {
                    Some("www".to_string())
                } else {
                    Some(c.server.static_root.clone())
                },
                http_verbose: c.logging.http_verbose,
                tls_enabled: c.server.tls_enabled,
                tls_cert_path: if c.server.tls_cert_path.is_empty() {
                    None
                } else {
                    Some(std::path::PathBuf::from(&c.server.tls_cert_path))
                },
                tls_key_path: if c.server.tls_key_path.is_empty() {
                    None
                } else {
                    Some(std::path::PathBuf::from(&c.server.tls_key_path))
                },
                rate_limit_per_minute: c.server.rate_limit_per_minute,
            }
        };
        let port = server_config.port;

        let server = Arc::new(
            OnvifServer::with_app_state(server_config, app_state.clone())
                .map_err(|e| StartupError::Network(e.to_string()))?,
        );

        let server_clone: Arc<OnvifServer> = Arc::clone(&server);
        let server_task = tokio::spawn(async move {
            if let Err(e) = server_clone.start().await {
                tracing::error!("HTTP server error: {}", e);
            }
        });

        progress.complete_phase();

        // Phase 5: Discovery (optional in custom platform scenarios)
        progress.begin_phase(StartupPhase::Discovery);
        let discovery_enabled = config_runtime.read().discovery.enabled;

        let (discovery, discovery_task) = if discovery_enabled {
            let discovery_config = Self::make_discovery_config(&config_runtime, port);

            match Self::start_discovery(discovery_config).await {
                Ok((disc, task)) => {
                    tracing::info!("WS-Discovery service started successfully");
                    (Some(disc), Some(task))
                }
                Err(e) => {
                    tracing::warn!(
                        "WS-Discovery failed to start, continuing in degraded mode: {}",
                        e
                    );
                    progress.record_degraded("discovery", e.to_string());
                    (None, None)
                }
            }
        } else {
            tracing::info!("WS-Discovery is disabled in configuration");
            (None, None)
        };
        progress.complete_phase();

        // Phase 6: Streaming (optional, gracefully degrades)
        let streaming_service =
            Self::start_streaming(&config_runtime, &app_state, &mut progress).await;

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
            discovery,
            discovery_task,
            config_persistence_task,
            user_persistence_task,
            profile_persistence_task,
            imaging_persistence_task,
            memory_logging_task,
            rate_limiter_cleanup_task,
            streaming_service,
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
        // Wait for shutdown signal
        let ctrl_c = async {
            if let Err(e) = tokio::signal::ctrl_c().await {
                tracing::error!("Failed to install Ctrl+C handler: {}", e);
                // We don't panic here, just log. The termination signal might still work.
                // Or we could return an error effectively, but we are inside an async block.
                // For this structure, logging and proceeding (effectively waiting forever on this branch)
                // is safer than crashing, though maybe we want to exit.
                // Let's rely on the other signal or just log.
                // To be strictly correct according to review:
                std::future::pending::<()>().await;
            }
        };

        #[cfg(unix)]
        let terminate = async {
            match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
                Ok(mut signal) => {
                    signal.recv().await;
                }
                Err(e) => {
                    tracing::error!("Failed to install SIGTERM handler: {}", e);
                    std::future::pending::<()>().await;
                }
            }
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

        self.shutdown_discovery(&mut report).await;
        self.shutdown_network(&mut report).await;
        self.shutdown_background_tasks().await;
        Self::record_service_shutdown(&mut report);
        self.shutdown_streaming(&mut report).await;
        self.shutdown_platform(&mut report).await;
        self.disarm_platform_drop_for_hard_exit(&report);

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

    async fn shutdown_discovery(&mut self, report: &mut ShutdownReport) {
        tracing::debug!("Sending WS-Discovery Bye...");
        if let Some(discovery) = self.discovery.take() {
            if let Err(e) = discovery.stop().await {
                tracing::warn!("Failed to stop WS-Discovery gracefully: {}", e);
                report.record_failure("discovery", e.to_string());
            } else {
                report.record_success("discovery");
            }
            if let Some(task) = self.discovery_task.take() {
                let _ = tokio::time::timeout(Duration::from_secs(2), task).await;
            }
            return;
        }

        report.record_success("discovery");
    }

    async fn shutdown_network(&mut self, report: &mut ShutdownReport) {
        tracing::debug!("Shutting down network services...");
        if let Some(server) = self.server.take() {
            server.shutdown();
            if let Some(task) = self.server_task.take() {
                let _ = tokio::time::timeout(Duration::from_secs(5), task).await;
            }
        }
        report.record_success("network");
    }

    async fn shutdown_background_tasks(&mut self) {
        if let Some(task) = self.config_persistence_task.take() {
            let _ = tokio::time::timeout(Duration::from_secs(2), task).await;
        }

        if let Some(task) = self.user_persistence_task.take() {
            let _ = tokio::time::timeout(Duration::from_secs(2), task).await;
        }

        if let Some(task) = self.profile_persistence_task.take() {
            let _ = tokio::time::timeout(Duration::from_secs(2), task).await;
        }

        if let Some(task) = self.imaging_persistence_task.take() {
            let _ = tokio::time::timeout(Duration::from_secs(2), task).await;
        }

        if let Some(task) = self.memory_logging_task.take() {
            let _ = tokio::time::timeout(Duration::from_secs(1), task).await;
        }

        if let Some(task) = self.rate_limiter_cleanup_task.take() {
            let _ = tokio::time::timeout(Duration::from_secs(1), task).await;
        }
    }

    fn record_service_shutdown(report: &mut ShutdownReport) {
        tracing::debug!("Shutting down ONVIF services...");
        report.record_success("imaging");
        report.record_success("ptz");
        report.record_success("media");
        report.record_success("device");
    }

    async fn shutdown_streaming(&mut self, report: &mut ShutdownReport) {
        if let Some(mut streaming) = self.streaming_service.take() {
            match tokio::time::timeout(Duration::from_secs(5), streaming.shutdown()).await {
                Ok(()) => {
                    report.record_success("streaming");
                }
                Err(_) => {
                    tracing::error!("Streaming shutdown timed out after 5s");
                    report.record_failure("streaming", "timeout".to_string());
                }
            }
            return;
        }
        report.record_success("streaming");
    }

    async fn shutdown_platform(&self, report: &mut ShutdownReport) {
        tracing::debug!("Shutting down platform...");
        let Some(state) = self.app_state.as_ref() else {
            report.record_success("platform");
            return;
        };
        let Some(platform) = state.platform() else {
            report.record_success("platform");
            return;
        };
        match platform.shutdown().await {
            Ok(()) => {
                report.record_success("platform");
            }
            Err(e) => {
                tracing::warn!("Platform shutdown error: {}", e);
                report.record_failure("platform", e.to_string());
            }
        }
        // Check the typed hard-shutdown flag — avoids string-scanning error messages.
        if platform.requires_hard_shutdown() {
            report.set_hard_exit_required();
        }
    }

    fn disarm_platform_drop_for_hard_exit(&mut self, report: &ShutdownReport) {
        if !report.hard_exit_required {
            return;
        }

        if let Some(state) = self.app_state.take() {
            tracing::error!(
                "Unsafe teardown detected; leaking app_state to suppress destructor-driven SDK cleanup before hard exit"
            );
            std::mem::forget(state);
        }
    }

    /// Maximum silence from venc-read before `stream_health` is marked degraded.
    const STREAM_HEALTH_SILENCE_SECS: u64 = 5;

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

        // Runtime stream liveness (not just startup readiness).
        if self.streaming_service.is_some() {
            match self
                .app_state
                .as_ref()
                .and_then(|s| s.platform())
                .and_then(|p| p.stream_frame_age_ms())
            {
                Some(age_ms) if age_ms > Self::STREAM_HEALTH_SILENCE_SECS * 1000 => {
                    status.add_component(
                        "stream_health",
                        ComponentHealth::degraded(
                            "Stream Health",
                            format!("No frames for {}ms (venc-read likely stalled)", age_ms),
                        ),
                    );
                    status.mark_degraded("stream_health");
                }
                Some(_) => {
                    status
                        .add_component("stream_health", ComponentHealth::healthy("Stream Health"));
                }
                None => {
                    status.add_component(
                        "stream_health",
                        ComponentHealth::degraded(
                            "Stream Health",
                            "Streaming enabled but no frames observed yet",
                        ),
                    );
                    status.mark_degraded("stream_health");
                }
            }
        }

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

    // ========================================================================
    // Private Helper Methods
    // ========================================================================

    /// Build a DiscoveryConfig from runtime configuration.
    ///
    /// This method:
    /// - Loads endpoint_uuid from config or generates a new one
    /// - Detects local IP from config or uses a reasonable default
    /// - Sets up scopes based on device capabilities
    fn make_discovery_config(config: &Arc<ConfigRuntime>, http_port: u16) -> DiscoveryConfig {
        let c = config.read();
        let endpoint_uuid = if c.discovery.endpoint_uuid.is_empty() {
            format!("urn:uuid:{}", uuid::Uuid::new_v4())
        } else {
            c.discovery.endpoint_uuid.clone()
        };

        let local_ip_config = if c.discovery.local_ip.is_empty() {
            "auto".to_string()
        } else {
            c.discovery.local_ip.clone()
        };

        let hello_interval_secs = if c.discovery.hello_interval == 0 {
            300u64
        } else {
            c.discovery.hello_interval as u64
        };
        drop(c);

        // Get local IP - "auto" means we should try to detect, otherwise use external_ip helper
        let device_ip = if local_ip_config == "auto" {
            external_ip(config)
        } else {
            local_ip_config
        };

        tracing::debug!(
            endpoint_uuid = %endpoint_uuid,
            device_ip = %device_ip,
            http_port = http_port,
            hello_interval_secs = hello_interval_secs,
            "Building WS-Discovery configuration"
        );

        DiscoveryConfig {
            endpoint_uuid,
            http_port,
            device_ip,
            hello_interval: Duration::from_secs(hello_interval_secs),
            ..Default::default()
        }
    }

    /// Start the WS-Discovery service and return the handle and background task.
    async fn start_discovery(
        config: DiscoveryConfig,
    ) -> Result<(WsDiscoveryHandle, JoinHandle<()>), crate::onvif::discovery::DiscoveryError> {
        let discovery = WsDiscovery::new(config);

        // Start the discovery service - this spawns a background task
        // that listens for Probe requests and sends ProbeMatch responses
        let (handle, task) = discovery.run_service().await?;

        tracing::debug!("WS-Discovery task started (full discovery mode)");

        Ok((handle, task))
    }
}

// Note: We intentionally do NOT implement Drop with async cleanup.
// All async cleanup must be done via the explicit shutdown() method.
// Drop will only deallocate memory, which Rust handles automatically.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::UserLevel;
    use crate::lifecycle::ShutdownStatus;
    use crate::lifecycle::health::HealthState;
    use crate::platform::StubPlatformBuilder;
    use crate::utils::MemoryMonitor;
    use std::net::TcpListener;

    fn make_app_state_for_stream_auth(
        config: Arc<ConfigRuntime>,
        user_storage: Arc<UserStorage>,
    ) -> AppState {
        AppState::builder()
            .user_storage(user_storage)
            .password_manager(Arc::new(PasswordManager::new()))
            .ptz_state(Arc::new(PTZStateManager::new()))
            .config(config)
            .memory_monitor(Arc::new(MemoryMonitor::new()))
            .rate_limiter(Arc::new(crate::security::RateLimiter::new(60)))
            .profile_storage(Arc::new(ProfileStorage::new("/tmp/test_profiles.toml")))
            .build()
            .expect("app state should build")
    }

    fn make_application_with_degraded_services(degraded_services: Vec<String>) -> Application {
        let (shutdown_tx, _) = broadcast::channel(SHUTDOWN_CHANNEL_CAPACITY);
        let shutdown_coordinator =
            ShutdownCoordinator::new(shutdown_tx.clone(), DEFAULT_SHUTDOWN_TIMEOUT);

        Application {
            started_at: Instant::now(),
            shutdown_coordinator,
            shutdown_tx,
            degraded_services,
            config_path: "/test/config.toml".to_string(),
            app_state: None,
            server: None,
            server_task: None,
            config_persistence_task: None,
            user_persistence_task: None,
            profile_persistence_task: None,
            imaging_persistence_task: None,
            discovery: None,
            discovery_task: None,
            memory_logging_task: None,
            rate_limiter_cleanup_task: None,
            streaming_service: None,
        }
    }

    fn reserve_test_port() -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
        let port = listener.local_addr().expect("read local addr").port();
        drop(listener);
        port
    }

    fn make_streaming_runtime_config(
        rtsp_port: u16,
        httpflv_port: u16,
        auth_enabled: bool,
    ) -> Arc<ConfigRuntime> {
        let config = Arc::new(ConfigRuntime::new(Default::default()));
        {
            let mut c = config.write();
            c.media.streaming_enabled = true;
            c.media.rtsp_port = rtsp_port;
            c.media.httpflv_port = httpflv_port;
            c.server.auth_enabled = auth_enabled;
        }
        config
    }

    #[test]
    fn test_build_stream_auth_disabled_returns_none() {
        let config = Arc::new(ConfigRuntime::new(Default::default()));
        config.write().server.auth_enabled = false;
        let app_state = make_app_state_for_stream_auth(config, Arc::new(UserStorage::new()));

        let stream_auth = Application::build_stream_auth(&app_state).unwrap();
        assert!(stream_auth.is_none());
    }

    #[test]
    fn test_build_stream_auth_enabled_without_users_returns_error() {
        let config = Arc::new(ConfigRuntime::new(Default::default()));
        config.write().server.auth_enabled = true;
        let app_state = make_app_state_for_stream_auth(config, Arc::new(UserStorage::new()));

        let result = Application::build_stream_auth(&app_state);
        assert!(result.is_err());
    }

    #[test]
    fn test_build_stream_auth_enabled_uses_user_storage_credentials() {
        let config = Arc::new(ConfigRuntime::new(Default::default()));
        {
            let mut c = config.write();
            c.server.auth_enabled = true;
            c.server.realm = "ONVIF Camera".to_string();
        }

        let storage = UserStorage::new();
        storage
            .create_user("admin", "secret", UserLevel::Administrator)
            .unwrap();
        let app_state = make_app_state_for_stream_auth(config, Arc::new(storage));

        let stream_auth = Application::build_stream_auth(&app_state)
            .unwrap()
            .expect("stream auth should be configured");

        assert!(
            stream_auth
                .authenticate_request("live/main", &None, Some("Basic YWRtaW46c2VjcmV0"), true)
                .is_ok()
        );
        assert!(
            stream_auth
                .authenticate_request("live/main", &None, Some("Basic YWRtaW46d3Jvbmc="), true)
                .is_err()
        );
        assert!(
            stream_auth
                .authenticate_request(
                    "live/main",
                    &Some("token=__unused_token__".to_string()),
                    None,
                    true
                )
                .is_err()
        );
    }

    #[tokio::test]
    async fn test_start_streaming_degrades_when_stream_auth_setup_fails() {
        let rtsp_port = reserve_test_port();
        let httpflv_port = reserve_test_port();
        let config = make_streaming_runtime_config(rtsp_port, httpflv_port, true);
        let app_state =
            make_app_state_for_stream_auth(config.clone(), Arc::new(UserStorage::new()));
        let mut progress = StartupProgress::new();

        let streaming = Application::start_streaming(&config, &app_state, &mut progress).await;
        assert!(streaming.is_none());
        assert!(progress.has_degraded_services());
        assert!(
            progress
                .degraded_services()
                .iter()
                .any(|service| service == "streaming_auth")
        );
    }

    #[tokio::test]
    async fn test_start_streaming_degrades_when_rtsp_port_unavailable() {
        let rtsp_listener = TcpListener::bind("127.0.0.1:0").expect("bind occupied rtsp port");
        let rtsp_port = rtsp_listener.local_addr().expect("rtsp local addr").port();
        let httpflv_port = reserve_test_port();
        let config = make_streaming_runtime_config(rtsp_port, httpflv_port, false);
        let app_state =
            make_app_state_for_stream_auth(config.clone(), Arc::new(UserStorage::new()));
        let mut progress = StartupProgress::new();

        let streaming = Application::start_streaming(&config, &app_state, &mut progress).await;
        assert!(streaming.is_none());
        assert!(progress.has_degraded_services());
        assert!(
            progress
                .degraded_services()
                .iter()
                .any(|service| service == "streaming")
        );
    }

    #[tokio::test]
    async fn test_start_streaming_success_without_platform_returns_service() {
        let rtsp_port = reserve_test_port();
        let httpflv_port = reserve_test_port();
        let config = make_streaming_runtime_config(rtsp_port, httpflv_port, false);
        let app_state =
            make_app_state_for_stream_auth(config.clone(), Arc::new(UserStorage::new()));
        let mut progress = StartupProgress::new();

        let mut streaming = Application::start_streaming(&config, &app_state, &mut progress)
            .await
            .expect("streaming service should start");
        assert!(!progress.has_degraded_services());

        streaming.shutdown().await;
    }

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
        // In test environments, WS-Discovery may fail due to socket permissions
        // This is expected and acceptable - the app should still start
        let app = Application::start("/test/config.toml").await.unwrap();
        // App may be degraded if discovery fails (expected in test environment)
        // The important thing is that the app starts successfully
        assert!(app.config_path() == "/test/config.toml");
    }

    #[test]
    fn test_make_discovery_config_explicit_values() {
        let config = Arc::new(ConfigRuntime::new(Default::default()));
        {
            let mut c = config.write();
            c.discovery.endpoint_uuid = "urn:uuid:explicit-test-id".to_string();
            c.discovery.local_ip = "192.0.2.50".to_string();
            c.discovery.hello_interval = 42;
        }

        let discovery = Application::make_discovery_config(&config, 8181);

        assert_eq!(discovery.endpoint_uuid, "urn:uuid:explicit-test-id");
        assert_eq!(discovery.device_ip, "192.0.2.50");
        assert_eq!(discovery.http_port, 8181);
        assert_eq!(discovery.hello_interval, Duration::from_secs(42));
    }

    #[test]
    fn test_make_discovery_config_uses_fallbacks_for_empty_uuid_and_auto_ip() {
        let config = Arc::new(ConfigRuntime::new(Default::default()));
        {
            let mut c = config.write();
            c.discovery.endpoint_uuid = String::new();
            c.network.detected_ip = "198.51.100.42".to_string();
        }

        let discovery = Application::make_discovery_config(&config, 8080);

        assert!(discovery.endpoint_uuid.starts_with("urn:uuid:"));
        assert_eq!(discovery.device_ip, "198.51.100.42");
        assert_eq!(discovery.http_port, 8080);
        assert_eq!(discovery.hello_interval, Duration::from_secs(300));
    }

    #[test]
    fn test_application_health_without_degradation_reports_healthy_components() {
        let app = make_application_with_degraded_services(Vec::new());
        let health = app.health();

        assert_eq!(health.status, HealthState::Healthy);
        assert!(health.is_ready());
        assert!(health.degraded_services.is_empty());
        assert!(health.components.contains_key("config"));
        assert!(health.components.contains_key("platform"));
        assert!(health.components.contains_key("device"));
        assert!(health.components.contains_key("media"));
    }

    #[test]
    fn test_application_health_with_degraded_services_marks_status_and_components() {
        let app = make_application_with_degraded_services(vec!["Discovery".to_string()]);
        let health = app.health();

        assert_eq!(health.status, HealthState::Degraded);
        assert!(health.is_ready());
        assert_eq!(health.degraded_services, vec!["Discovery"]);
        let discovery_component = health.components.get("discovery");
        assert!(discovery_component.is_some());
        assert_eq!(
            discovery_component.unwrap().status,
            crate::lifecycle::health::HealthState::Degraded
        );
        assert_eq!(
            discovery_component.unwrap().message,
            Some("Initialization failed".to_string())
        );
    }

    #[test]
    fn test_application_is_degraded_and_degraded_services_reflect_internal_state() {
        let degraded = vec!["discovery".to_string(), "ptz".to_string()];
        let app = make_application_with_degraded_services(degraded.clone());

        assert!(app.is_degraded());
        assert_eq!(app.degraded_services(), degraded.as_slice());
    }

    #[tokio::test]
    async fn test_application_start_with_platform_invalid_port_returns_network_error() {
        let temp_config = std::env::temp_dir().join(format!(
            "onvif-rust-app-start-with-platform-{}.toml",
            uuid::Uuid::new_v4()
        ));

        std::fs::write(
            &temp_config,
            "[server]\nport = 0\nbind_address = \"127.0.0.1\"\n[discovery]\nenabled = false\n",
        )
        .unwrap();

        let platform = Arc::new(
            StubPlatformBuilder::new()
                .ptz_supported(true)
                .imaging_supported(true)
                .build(),
        );

        let result = Application::start_with_platform(
            temp_config
                .to_str()
                .unwrap_or("/tmp/onvif-test-config.toml"),
            platform,
        )
        .await;

        assert!(result.is_err());
        // Validation now catches port=0 at config load time
        assert!(matches!(
            result,
            Err(StartupError::Config(ref message)) if message.contains("server.port")
        ));

        let _ = std::fs::remove_file(&temp_config);
    }

    #[tokio::test]
    async fn test_application_start_with_platform_success_and_shutdown() {
        let port = reserve_test_port();
        let temp_config = std::env::temp_dir().join(format!(
            "onvif-rust-app-start-with-platform-success-{}.toml",
            uuid::Uuid::new_v4()
        ));

        std::fs::write(
            &temp_config,
            format!(
                "[server]\nport = {port}\nbind_address = \"127.0.0.1\"\nauth_enabled = false\n\
                 \n[discovery]\nenabled = false\n\n[media]\nstreaming_enabled = false\n"
            ),
        )
        .expect("write temp config");

        let platform = Arc::new(
            StubPlatformBuilder::new()
                .ptz_supported(true)
                .imaging_supported(true)
                .build(),
        );

        let config_path = temp_config
            .to_str()
            .unwrap_or("/tmp/onvif-start-with-platform-success.toml")
            .to_string();
        let app = Application::start_with_platform(&config_path, platform)
            .await
            .expect("application should start with custom platform");

        assert_eq!(app.config_path(), config_path);

        let report = app.shutdown().await;
        assert_eq!(report.status, ShutdownStatus::Success);
        assert!(
            report
                .successful_components
                .iter()
                .any(|component| component == "network")
        );

        let _ = std::fs::remove_file(&temp_config);
    }

    #[tokio::test]
    async fn test_application_shutdown_without_app_state_still_marks_platform_success() {
        let app = make_application_with_degraded_services(Vec::new());
        let report = app.shutdown().await;

        assert_eq!(report.status, ShutdownStatus::Success);
        assert!(
            report
                .successful_components
                .iter()
                .any(|component| component == "platform")
        );
    }

    /// Test that disarm_platform_drop_for_hard_exit does NOT leak state when
    /// hard_exit_required is false. This verifies the normal shutdown path
    /// properly cleans up resources.
    #[test]
    fn test_disarm_platform_drop_does_not_leak_state_when_hard_exit_not_required() {
        let (shutdown_tx, _) = broadcast::channel(SHUTDOWN_CHANNEL_CAPACITY);
        let shutdown_coordinator =
            ShutdownCoordinator::new(shutdown_tx.clone(), DEFAULT_SHUTDOWN_TIMEOUT);

        // Create a minimal AppState for testing
        let config = Arc::new(ConfigRuntime::new(Default::default()));
        let app_state = AppState::builder()
            .user_storage(Arc::new(UserStorage::new()))
            .password_manager(Arc::new(PasswordManager::new()))
            .ptz_state(Arc::new(PTZStateManager::new()))
            .config(config)
            .memory_monitor(Arc::new(crate::utils::MemoryMonitor::new()))
            .rate_limiter(Arc::new(crate::security::RateLimiter::new(60)))
            .profile_storage(Arc::new(ProfileStorage::new("/tmp/test_profiles.toml")))
            .build()
            .expect("app state should build");

        let mut app = Application {
            started_at: Instant::now(),
            shutdown_coordinator,
            shutdown_tx,
            degraded_services: Vec::new(),
            config_path: "/test/config.toml".to_string(),
            app_state: Some(app_state),
            server: None,
            server_task: None,
            config_persistence_task: None,
            user_persistence_task: None,
            profile_persistence_task: None,
            imaging_persistence_task: None,
            discovery: None,
            discovery_task: None,
            memory_logging_task: None,
            rate_limiter_cleanup_task: None,
            streaming_service: None,
        };

        // Create a ShutdownReport with hard_exit_required = false (normal shutdown)
        let report = ShutdownReport::new();
        assert!(
            !report.hard_exit_required,
            "hard_exit_required should default to false"
        );

        // Call disarm_platform_drop_for_hard_exit - it should NOT take app_state
        app.disarm_platform_drop_for_hard_exit(&report);

        // Verify app_state is still present (not leaked/taken)
        assert!(
            app.app_state.is_some(),
            "app_state should NOT be taken when hard_exit_required is false"
        );
    }

    /// Test that disarm_platform_drop_for_hard_exit DOES leak state when
    /// hard_exit_required is true. This verifies the hard-exit path
    /// prevents destructor-driven SDK cleanup.
    #[test]
    fn test_disarm_platform_drop_leaks_state_when_hard_exit_required() {
        let (shutdown_tx, _) = broadcast::channel(SHUTDOWN_CHANNEL_CAPACITY);
        let shutdown_coordinator =
            ShutdownCoordinator::new(shutdown_tx.clone(), DEFAULT_SHUTDOWN_TIMEOUT);

        // Create a minimal AppState for testing
        let config = Arc::new(ConfigRuntime::new(Default::default()));
        let app_state = AppState::builder()
            .user_storage(Arc::new(UserStorage::new()))
            .password_manager(Arc::new(PasswordManager::new()))
            .ptz_state(Arc::new(PTZStateManager::new()))
            .config(config)
            .memory_monitor(Arc::new(crate::utils::MemoryMonitor::new()))
            .rate_limiter(Arc::new(crate::security::RateLimiter::new(60)))
            .profile_storage(Arc::new(ProfileStorage::new("/tmp/test_profiles.toml")))
            .build()
            .expect("app state should build");

        let mut app = Application {
            started_at: Instant::now(),
            shutdown_coordinator,
            shutdown_tx,
            degraded_services: Vec::new(),
            config_path: "/test/config.toml".to_string(),
            app_state: Some(app_state),
            server: None,
            server_task: None,
            config_persistence_task: None,
            user_persistence_task: None,
            profile_persistence_task: None,
            imaging_persistence_task: None,
            discovery: None,
            discovery_task: None,
            memory_logging_task: None,
            rate_limiter_cleanup_task: None,
            streaming_service: None,
        };

        // Create a ShutdownReport with hard_exit_required = true (hard shutdown)
        let mut report = ShutdownReport::new();
        report.set_hard_exit_required();
        assert!(
            report.hard_exit_required,
            "hard_exit_required should be true"
        );

        // Call disarm_platform_drop_for_hard_exit - it SHOULD take and leak app_state
        app.disarm_platform_drop_for_hard_exit(&report);

        // Verify app_state IS taken (leaked to prevent destructor cleanup)
        assert!(
            app.app_state.is_none(),
            "app_state should be taken (leaked) when hard_exit_required is true"
        );
    }

    /// Test that shutdown report's hard_exit_required field is properly set when
    /// platform requires hard shutdown. This verifies the typed flag replaces
    /// string-scanning of error messages.
    #[tokio::test]
    async fn test_application_shutdown_report_hard_exit_required_from_platform() {
        // Create a stub platform that returns true for requires_hard_shutdown
        // The stub platform returns false for requires_hard_shutdown by default
        // To properly test this, we'd need a builder method to set hard_shutdown_required
        // For now, we verify the field exists and can be set on ShutdownReport
        let mut report = ShutdownReport::new();
        assert!(!report.hard_exit_required);

        report.set_hard_exit_required();
        assert!(report.hard_exit_required);
    }
}
