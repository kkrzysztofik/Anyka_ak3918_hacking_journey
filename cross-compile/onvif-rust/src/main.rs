//! ONVIF Rust daemon entry point.
//!
//! This is the main entry point for the ONVIF daemon. It uses the Application
//! lifecycle pattern for clean startup and shutdown.

use anyhow::Result;
use onvif_rust::app::{Application, DEFAULT_CONFIG_PATH};

#[tokio::main]
async fn main() -> Result<()> {
    // Logging is initialized by Application::start() after loading config
    // This allows proper file logging setup based on configuration

    // Get config path from command line or use default
    let config_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| DEFAULT_CONFIG_PATH.to_string());

    // Start the application with ordered initialization
    let app = match Application::start(&config_path).await {
        Ok(app) => app,
        Err(e) => {
            tracing::error!("Failed to start application: {}", e);
            return Err(e.into());
        }
    };

    // Log health status
    let health = app.health();
    if app.is_degraded() {
        tracing::warn!(
            "Application running in DEGRADED mode. Unavailable services: {:?}",
            app.degraded_services()
        );
    } else {
        tracing::info!("Application health: {}", health.status);
    }

    // Run until shutdown signal (SIGINT/SIGTERM)
    if let Err(e) = app.run().await {
        tracing::error!("Runtime error: {}", e);
    }

    // Perform graceful shutdown
    let report = app.shutdown().await;

    // Log shutdown report
    match report.status {
        onvif_rust::ShutdownStatus::Success => {
            tracing::info!("Shutdown completed successfully in {:?}", report.duration);
        }
        onvif_rust::ShutdownStatus::Timeout => {
            tracing::warn!(
                "Shutdown timed out after {:?}. Some components may not have stopped cleanly.",
                report.duration
            );
        }
        onvif_rust::ShutdownStatus::Error => {
            tracing::error!("Shutdown encountered errors: {:?}", report.errors);
        }
    }

    Ok(())
}
