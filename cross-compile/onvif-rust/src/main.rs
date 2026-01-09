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
            "Application health: {} (DEGRADED). Unavailable services: {:?}",
            health.status,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_path_extraction() {
        // Test default config path
        let args: Vec<String> = vec![];
        let config_path = args
            .get(1)
            .map(|s| s.clone())
            .unwrap_or_else(|| DEFAULT_CONFIG_PATH.to_string());
        assert_eq!(config_path, DEFAULT_CONFIG_PATH);

        // Test custom config path
        let args: Vec<String> = vec!["program".to_string(), "/custom/path.toml".to_string()];
        let config_path = args
            .get(1)
            .map(|s| s.clone())
            .unwrap_or_else(|| DEFAULT_CONFIG_PATH.to_string());
        assert_eq!(config_path, "/custom/path.toml");
    }

    #[test]
    fn test_config_path_default() {
        // Simulate no command line args
        let config_path = std::env::args()
            .nth(1)
            .unwrap_or_else(|| DEFAULT_CONFIG_PATH.to_string());
        assert_eq!(config_path, DEFAULT_CONFIG_PATH);
    }
}
