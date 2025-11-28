//! Application lifecycle management module.
//!
//! This module provides structured lifecycle management for the ONVIF application
//! using a "kinda-hybrid" approach:
//! - Explicit async `start()`/`shutdown()` methods for ordered initialization and graceful shutdown
//! - Rust ownership handles resource deallocation (no async cleanup in Drop)
//! - No global state - all state owned by Application struct
//! - Dependency injection - components receive dependencies, not global lookups

pub mod health;
pub mod shutdown;
pub mod startup;

use std::time::Duration;
use thiserror::Error;

/// Errors that can occur during application startup.
#[derive(Error, Debug)]
pub enum StartupError {
    /// Configuration loading or validation failed.
    #[error("Configuration error: {0}")]
    Config(String),

    /// Platform initialization failed.
    #[error("Platform initialization error: {0}")]
    Platform(String),

    /// Required service initialization failed.
    #[error("Service initialization error: {0}")]
    Services(String),

    /// Network initialization failed (HTTP server, etc.).
    #[error("Network initialization error: {0}")]
    Network(String),

    /// I/O error during startup.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Errors that can occur during application runtime.
#[derive(Error, Debug)]
pub enum RuntimeError {
    /// HTTP server error.
    #[error("HTTP server error: {0}")]
    HttpServer(String),

    /// Service error during request handling.
    #[error("Service error: {0}")]
    Service(String),

    /// Signal handling error.
    #[error("Signal handling error: {0}")]
    Signal(String),
}

/// Status of the shutdown process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownStatus {
    /// All components shut down successfully within timeout.
    Success,
    /// Shutdown completed but some components timed out.
    Timeout,
    /// Shutdown encountered errors.
    Error,
}

/// Report of the shutdown process.
#[derive(Debug, Clone)]
pub struct ShutdownReport {
    /// Overall shutdown status.
    pub status: ShutdownStatus,
    /// Total duration of the shutdown process.
    pub duration: Duration,
    /// Components that successfully shut down.
    pub successful_components: Vec<String>,
    /// Components that failed or timed out.
    pub failed_components: Vec<String>,
    /// Error messages from failed components.
    pub errors: Vec<String>,
}

impl ShutdownReport {
    /// Create a new shutdown report with default values.
    pub fn new() -> Self {
        Self {
            status: ShutdownStatus::Success,
            duration: Duration::ZERO,
            successful_components: Vec::new(),
            failed_components: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// Record a successful component shutdown.
    pub fn record_success(&mut self, component: impl Into<String>) {
        self.successful_components.push(component.into());
    }

    /// Record a failed component shutdown.
    pub fn record_failure(&mut self, component: impl Into<String>, error: impl Into<String>) {
        let component = component.into();
        self.failed_components.push(component.clone());
        self.errors.push(format!("{}: {}", component, error.into()));
        self.status = ShutdownStatus::Error;
    }

    /// Mark the shutdown as timed out.
    pub fn mark_timeout(&mut self) {
        if self.status != ShutdownStatus::Error {
            self.status = ShutdownStatus::Timeout;
        }
    }
}

impl Default for ShutdownReport {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shutdown_report_new() {
        let report = ShutdownReport::new();
        assert_eq!(report.status, ShutdownStatus::Success);
        assert!(report.successful_components.is_empty());
        assert!(report.failed_components.is_empty());
        assert!(report.errors.is_empty());
    }

    #[test]
    fn test_shutdown_report_record_success() {
        let mut report = ShutdownReport::new();
        report.record_success("config");
        report.record_success("platform");

        assert_eq!(report.status, ShutdownStatus::Success);
        assert_eq!(report.successful_components, vec!["config", "platform"]);
    }

    #[test]
    fn test_shutdown_report_record_failure() {
        let mut report = ShutdownReport::new();
        report.record_success("config");
        report.record_failure("platform", "hardware error");

        assert_eq!(report.status, ShutdownStatus::Error);
        assert_eq!(report.successful_components, vec!["config"]);
        assert_eq!(report.failed_components, vec!["platform"]);
        assert_eq!(report.errors, vec!["platform: hardware error"]);
    }

    #[test]
    fn test_shutdown_report_mark_timeout() {
        let mut report = ShutdownReport::new();
        report.mark_timeout();
        assert_eq!(report.status, ShutdownStatus::Timeout);

        // Error status should not be overwritten by timeout
        let mut report2 = ShutdownReport::new();
        report2.record_failure("test", "error");
        report2.mark_timeout();
        assert_eq!(report2.status, ShutdownStatus::Error);
    }
}
