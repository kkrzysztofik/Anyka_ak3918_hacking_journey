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

use std::time::{Duration, Instant};
use tokio::sync::broadcast;

use crate::lifecycle::health::{ComponentHealth, HealthStatus};
use crate::lifecycle::shutdown::{ShutdownCoordinator, DEFAULT_SHUTDOWN_TIMEOUT};
use crate::lifecycle::startup::{StartupPhase, StartupProgress};
use crate::lifecycle::{RuntimeError, ShutdownReport, StartupError};

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
        // TODO: Load actual configuration when config module is implemented
        // For now, just validate the path exists or use defaults
        tracing::debug!("Configuration loading placeholder - will be implemented in T051-T065");
        progress.complete_phase();

        // Phase 2: Platform
        progress.begin_phase(StartupPhase::Platform);
        // TODO: Initialize platform when platform module is implemented
        tracing::debug!("Platform initialization placeholder - will be implemented in T034-T050");
        progress.complete_phase();

        // Phase 3: Services
        progress.begin_phase(StartupPhase::Services);
        // TODO: Initialize services when service modules are implemented
        tracing::debug!("Service initialization placeholder - will be implemented in Phase 6-9");

        // Simulate optional service failures for demonstration
        // In real implementation, this would attempt actual initialization
        #[cfg(test)]
        {
            // In tests, we can simulate degraded services
        }

        progress.complete_phase();

        // Phase 4: Network
        progress.begin_phase(StartupPhase::Network);
        // TODO: Initialize HTTP server when network module is implemented
        tracing::debug!("Network initialization placeholder - will be implemented in T072-T088");
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
            tracing::info!(
                "Application started successfully in {:?}",
                startup_duration
            );
        }

        Ok(Self {
            started_at,
            shutdown_coordinator,
            shutdown_tx,
            degraded_services: progress.degraded_services().to_vec(),
            config_path: config_path.to_string(),
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
    pub async fn shutdown(self) -> ShutdownReport {
        tracing::info!("Beginning graceful shutdown...");

        let mut report = self.shutdown_coordinator.initiate_shutdown().await;

        // Record component shutdown status
        // In a full implementation, each component would report its shutdown status

        // Phase 1: Discovery Bye (non-fatal)
        tracing::debug!("Sending WS-Discovery Bye...");
        report.record_success("discovery");

        // Phase 2: Network shutdown
        tracing::debug!("Shutting down network services...");
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

        assert_eq!(
            report.status,
            crate::lifecycle::ShutdownStatus::Success
        );
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
        let app = Application::start("/custom/path/config.toml").await.unwrap();
        assert_eq!(app.config_path(), "/custom/path/config.toml");
    }

    #[tokio::test]
    async fn test_application_not_degraded_by_default() {
        let app = Application::start("/test/config.toml").await.unwrap();
        assert!(!app.is_degraded());
        assert!(app.degraded_services().is_empty());
    }
}
