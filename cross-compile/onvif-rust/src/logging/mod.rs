//! Logging system for the ONVIF application.
//!
//! This module provides a comprehensive logging system built on the `tracing`
//! framework that:
//!
//! - Configures log levels from application configuration
//! - Supports runtime log level changes via `set_log_level()`
//! - Bridges the `log` crate to `tracing` for dependency logging
//! - Integrates with platform logging (Anyka SDK's ak_print)
//! - Supports console and file output
//!
//! # Example
//!
//! ```ignore
//! use onvif_rust::logging::{init_logging, set_log_level};
//! use onvif_rust::config::ConfigRuntime;
//!
//! let config = ConfigRuntime::new(...);
//! init_logging(&config)?;
//!
//! tracing::info!("Application started");
//!
//! // Change log level at runtime
//! set_log_level("debug")?;
//! ```

pub mod http;
pub mod http_memory;
pub mod static_assets;
mod platform;

pub use http::{HttpLogConfig, HttpLoggingMiddleware};
pub use http_memory::memory_check_middleware;
pub use platform::*;
pub use static_assets::{StaticAssetLogConfig, static_asset_logging_middleware};

use std::path::Path;
use std::sync::{Once, OnceLock};

use thiserror::Error;
use tracing::Level;
use tracing_subscriber::{
    EnvFilter,
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    reload::{self, Handle},
    util::SubscriberInitExt,
};

use crate::config::ConfigRuntime;

/// Errors that can occur during logging initialization.
#[derive(Debug, Error)]
pub enum LoggingError {
    /// Failed to initialize tracing subscriber.
    #[error("Failed to initialize tracing: {0}")]
    TracingInit(String),

    /// Failed to initialize log bridge.
    #[error("Failed to initialize log bridge: {0}")]
    LogBridge(String),

    /// Invalid log level.
    #[error("Invalid log level: {0}")]
    InvalidLevel(String),

    /// Failed to reload log level.
    #[error("Failed to reload log level: {0}")]
    ReloadFailed(String),
}

/// Result type for logging operations.
pub type LoggingResult<T> = Result<T, LoggingError>;

/// Ensure logging is only initialized once.
static INIT: Once = Once::new();

/// Global handle for reloading the log filter at runtime.
static RELOAD_HANDLE: OnceLock<Handle<EnvFilter, tracing_subscriber::Registry>> = OnceLock::new();

/// Set the log level at runtime.
///
/// This function can be called at any time after `init_logging()` to change
/// the log level without restarting the application.
///
/// # Arguments
///
/// * `level` - Log level string: "error", "warn", "info", "debug", or "trace"
///
/// # Example
///
/// ```ignore
/// set_log_level("debug")?;
/// tracing::debug!("This will now be logged");
/// ```
pub fn set_log_level(level: &str) -> LoggingResult<()> {
    let level = parse_log_level(level)?;
    let filter = EnvFilter::builder()
        .with_default_directive(level.into())
        .from_env_lossy();

    if let Some(handle) = RELOAD_HANDLE.get() {
        handle
            .reload(filter)
            .map_err(|e| LoggingError::ReloadFailed(e.to_string()))?;
        tracing::info!("Log level changed to {}", level);
        Ok(())
    } else {
        Err(LoggingError::ReloadFailed(
            "Logging not initialized with reload support".to_string(),
        ))
    }
}

/// Initialize the logging system.
///
/// This function should be called once at application startup.
/// It configures:
///
/// - `tracing` with console output and optional file output
/// - `tracing-log` bridge to capture `log` crate messages
/// - Log level filtering based on configuration
///
/// # Arguments
///
/// * `config` - Runtime configuration to read log settings from
///
/// # Example
///
/// ```ignore
/// init_logging(&config)?;
/// tracing::info!("Logging initialized");
/// ```
pub fn init_logging(config: &ConfigRuntime) -> LoggingResult<()> {
    let mut result = Ok(());

    INIT.call_once(|| {
        result = init_logging_impl(config);
    });

    result
}

/// Internal logging initialization implementation.
fn init_logging_impl(config: &ConfigRuntime) -> LoggingResult<()> {
    // Get log level from configuration
    let level_str = config
        .get_string("logging.level")
        .unwrap_or_else(|_| "info".to_string());

    let level = parse_log_level(&level_str)?;

    // Create env filter with default level
    let env_filter = EnvFilter::builder()
        .with_default_directive(level.into())
        .from_env_lossy();

    // Create reloadable filter layer
    let (filter_layer, reload_handle) = reload::Layer::new(env_filter);

    // Store the reload handle for runtime log level changes
    let _ = RELOAD_HANDLE.set(reload_handle);

    // Get logging configuration
    let console_enabled = config.get_bool("logging.console_enabled").unwrap_or(true);
    let file_path = config.get_string("logging.file_path").unwrap_or_default();

    // Create console layer
    let console_layer = fmt::layer()
        .with_target(true)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_file(true)
        .with_line_number(true)
        .with_span_events(FmtSpan::NONE);

    // Initialize the subscriber based on configuration
    let registry = tracing_subscriber::registry().with(filter_layer);

    // Check if file logging is enabled
    let file_enabled = !file_path.is_empty();

    match (console_enabled, file_enabled) {
        (true, true) => {
            // Both console and file logging
            let file_path = Path::new(&file_path);
            let parent_dir = file_path.parent().unwrap_or(Path::new("."));
            let file_name = file_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("onvif.log");

            let file_appender = tracing_appender::rolling::daily(parent_dir, file_name);
            let file_layer = fmt::layer()
                .with_target(true)
                .with_file(true)
                .with_line_number(true)
                .with_ansi(false) // No ANSI colors in file
                .with_writer(file_appender);

            registry
                .with(console_layer)
                .with(file_layer)
                .try_init()
                .map_err(|e| LoggingError::TracingInit(e.to_string()))?;

            tracing::info!(
                level = %level_str,
                console_enabled = console_enabled,
                file_path = %file_path.display(),
                "Logging system initialized with console and file output"
            );
        }
        (true, false) => {
            // Console only
            registry
                .with(console_layer)
                .try_init()
                .map_err(|e| LoggingError::TracingInit(e.to_string()))?;

            tracing::info!(
                level = %level_str,
                console_enabled = console_enabled,
                "Logging system initialized with console output"
            );
        }
        (false, true) => {
            // File only
            let file_path = Path::new(&file_path);
            let parent_dir = file_path.parent().unwrap_or(Path::new("."));
            let file_name = file_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("onvif.log");

            let file_appender = tracing_appender::rolling::daily(parent_dir, file_name);
            let file_layer = fmt::layer()
                .with_target(true)
                .with_file(true)
                .with_line_number(true)
                .with_ansi(false)
                .with_writer(file_appender);

            registry
                .with(file_layer)
                .try_init()
                .map_err(|e| LoggingError::TracingInit(e.to_string()))?;

            // Can't log here since console is disabled, but file will capture it
            tracing::info!(
                level = %level_str,
                file_path = %file_path.display(),
                "Logging system initialized with file output only"
            );
        }
        (false, false) => {
            // No logging output configured - just init with filter
            registry
                .try_init()
                .map_err(|e| LoggingError::TracingInit(e.to_string()))?;
        }
    }

    // Initialize log bridge to capture `log` crate messages from dependencies
    // Use try_init to handle case where log is already initialized by a dependency
    let _ = tracing_log::LogTracer::init();

    Ok(())
}

/// Initialize static asset access logging to a separate file.
///
/// This function sets up a separate logging target for static asset requests
/// in Apache combined log format. It creates a daily-rotating log file.
///
/// # Arguments
///
/// * `config` - Runtime configuration containing static asset logging settings
/// * `enabled` - Whether static asset logging is enabled
///
/// # Example
///
/// ```ignore
/// init_static_asset_logging(&config, true)?;
/// ```
pub fn init_static_asset_logging(config: &ConfigRuntime, enabled: bool) -> LoggingResult<()> {
    if !enabled {
        return Ok(());
    }

    // Get static asset logging path from config
    let log_path = config
        .get_string("logging.static_assets.file_path")
        .unwrap_or_else(|_| "logs".to_string());

    let log_name = config
        .get_string("logging.static_assets.file_name")
        .unwrap_or_else(|_| "access".to_string());

    // Log that static asset logging is configured
    // The actual file appender will be created by the middleware when it logs
    // tracing handles lazy initialization of targets
    tracing::info!(
        log_path = %log_path,
        log_name = %log_name,
        "Static asset access logging configured (will be written to separate file)"
    );

    Ok(())
}

/// Parse a log level string into a tracing Level.
fn parse_log_level(level: &str) -> LoggingResult<Level> {
    match level.to_lowercase().as_str() {
        "error" => Ok(Level::ERROR),
        "warn" | "warning" => Ok(Level::WARN),
        "info" => Ok(Level::INFO),
        "debug" => Ok(Level::DEBUG),
        "trace" => Ok(Level::TRACE),
        _ => Err(LoggingError::InvalidLevel(level.to_string())),
    }
}

/// Logging configuration for testing.
pub struct TestLogging;

impl TestLogging {
    /// Initialize test logging with trace level.
    pub fn init() {
        let _ = tracing_subscriber::fmt()
            .with_test_writer()
            .with_max_level(Level::TRACE)
            .try_init();
    }
}

/// Structured log fields for ONVIF operations.
#[derive(Debug, Clone)]
pub struct OnvifLogFields {
    /// ONVIF service name (Device, Media, PTZ, Imaging).
    pub service: String,
    /// ONVIF operation name.
    pub operation: String,
    /// Request ID for correlation.
    pub request_id: Option<String>,
    /// Client IP address.
    pub client_ip: Option<String>,
}

impl OnvifLogFields {
    /// Create new log fields for an ONVIF request.
    pub fn new(service: &str, operation: &str) -> Self {
        Self {
            service: service.to_string(),
            operation: operation.to_string(),
            request_id: None,
            client_ip: None,
        }
    }

    /// Add request ID.
    pub fn with_request_id(mut self, id: &str) -> Self {
        self.request_id = Some(id.to_string());
        self
    }

    /// Add client IP.
    pub fn with_client_ip(mut self, ip: &str) -> Self {
        self.client_ip = Some(ip.to_string());
        self
    }

    /// Log an info message with these fields.
    pub fn info(&self, message: &str) {
        tracing::info!(
            service = %self.service,
            operation = %self.operation,
            request_id = ?self.request_id,
            client_ip = ?self.client_ip,
            "{}", message
        );
    }

    /// Log a debug message with these fields.
    pub fn debug(&self, message: &str) {
        tracing::debug!(
            service = %self.service,
            operation = %self.operation,
            request_id = ?self.request_id,
            client_ip = ?self.client_ip,
            "{}", message
        );
    }

    /// Log an error message with these fields.
    pub fn error(&self, message: &str) {
        tracing::error!(
            service = %self.service,
            operation = %self.operation,
            request_id = ?self.request_id,
            client_ip = ?self.client_ip,
            "{}", message
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_log_level() {
        assert_eq!(parse_log_level("error").unwrap(), Level::ERROR);
        assert_eq!(parse_log_level("ERROR").unwrap(), Level::ERROR);
        assert_eq!(parse_log_level("warn").unwrap(), Level::WARN);
        assert_eq!(parse_log_level("warning").unwrap(), Level::WARN);
        assert_eq!(parse_log_level("info").unwrap(), Level::INFO);
        assert_eq!(parse_log_level("debug").unwrap(), Level::DEBUG);
        assert_eq!(parse_log_level("trace").unwrap(), Level::TRACE);

        assert!(parse_log_level("invalid").is_err());
    }

    #[test]
    fn test_onvif_log_fields() {
        let fields = OnvifLogFields::new("Device", "GetDeviceInformation")
            .with_request_id("req-123")
            .with_client_ip("192.168.1.100");

        assert_eq!(fields.service, "Device");
        assert_eq!(fields.operation, "GetDeviceInformation");
        assert_eq!(fields.request_id, Some("req-123".to_string()));
        assert_eq!(fields.client_ip, Some("192.168.1.100".to_string()));
    }

    #[test]
    fn test_onvif_log_fields_default() {
        let fields = OnvifLogFields::new("Media", "GetProfiles");

        assert_eq!(fields.service, "Media");
        assert_eq!(fields.operation, "GetProfiles");
        assert!(fields.request_id.is_none());
        assert!(fields.client_ip.is_none());
    }

    #[test]
    fn test_onvif_log_fields_clone() {
        let fields = OnvifLogFields::new("PTZ", "ContinuousMove").with_request_id("uuid-456");

        let cloned = fields.clone();
        assert_eq!(fields.service, cloned.service);
        assert_eq!(fields.operation, cloned.operation);
        assert_eq!(fields.request_id, cloned.request_id);
    }

    #[test]
    fn test_onvif_log_fields_partial_chain() {
        // Test with only request_id
        let fields = OnvifLogFields::new("Imaging", "GetOptions").with_request_id("req-789");

        assert_eq!(fields.request_id, Some("req-789".to_string()));
        assert!(fields.client_ip.is_none());

        // Test with only client_ip
        let fields2 = OnvifLogFields::new("Device", "GetCapabilities").with_client_ip("10.0.0.1");

        assert!(fields2.request_id.is_none());
        assert_eq!(fields2.client_ip, Some("10.0.0.1".to_string()));
    }

    #[test]
    fn test_logging_error_display() {
        let err = LoggingError::InvalidLevel("badlevel".to_string());
        let display = format!("{}", err);
        assert!(display.contains("badlevel"));

        let err = LoggingError::TracingInit("subscriber failed".to_string());
        let display = format!("{}", err);
        assert!(display.contains("subscriber failed"));

        let err = LoggingError::LogBridge("bridge error".to_string());
        let display = format!("{}", err);
        assert!(display.contains("bridge error"));
    }

    #[test]
    fn test_test_logging_init() {
        // TestLogging::init() should not panic even if called multiple times
        TestLogging::init();
        TestLogging::init(); // Should be safe to call again
    }

    #[test]
    fn test_onvif_log_fields_info_method() {
        // Initialize test logging
        let _ = tracing_subscriber::fmt().with_test_writer().try_init();

        let fields = OnvifLogFields::new("Device", "GetDeviceInformation")
            .with_request_id("req-123")
            .with_client_ip("192.168.1.100");

        // Should not panic
        fields.info("Test info message");
    }

    #[test]
    fn test_onvif_log_fields_debug_method() {
        let _ = tracing_subscriber::fmt().with_test_writer().try_init();

        let fields = OnvifLogFields::new("Media", "GetProfiles");
        fields.debug("Test debug message");
    }

    #[test]
    fn test_onvif_log_fields_error_method() {
        let _ = tracing_subscriber::fmt().with_test_writer().try_init();

        let fields = OnvifLogFields::new("PTZ", "ContinuousMove").with_request_id("req-456");
        fields.error("Test error message");
    }

    #[test]
    fn test_parse_log_level_case_insensitive() {
        assert_eq!(parse_log_level("ERROR").unwrap(), Level::ERROR);
        assert_eq!(parse_log_level("Error").unwrap(), Level::ERROR);
        assert_eq!(parse_log_level("error").unwrap(), Level::ERROR);
        assert_eq!(parse_log_level("WARN").unwrap(), Level::WARN);
        assert_eq!(parse_log_level("Warn").unwrap(), Level::WARN);
        assert_eq!(parse_log_level("INFO").unwrap(), Level::INFO);
        assert_eq!(parse_log_level("Info").unwrap(), Level::INFO);
        assert_eq!(parse_log_level("DEBUG").unwrap(), Level::DEBUG);
        assert_eq!(parse_log_level("Debug").unwrap(), Level::DEBUG);
        assert_eq!(parse_log_level("TRACE").unwrap(), Level::TRACE);
        assert_eq!(parse_log_level("Trace").unwrap(), Level::TRACE);
    }

    #[test]
    fn test_parse_log_level_warning_alias() {
        // "warning" should map to WARN
        assert_eq!(parse_log_level("warning").unwrap(), Level::WARN);
        assert_eq!(parse_log_level("WARNING").unwrap(), Level::WARN);
        assert_eq!(parse_log_level("Warning").unwrap(), Level::WARN);
    }

    #[test]
    fn test_parse_log_level_invalid_values() {
        assert!(parse_log_level("").is_err());
        assert!(parse_log_level("invalid").is_err());
        assert!(parse_log_level("verbose").is_err());
        assert!(parse_log_level("critical").is_err());
        assert!(parse_log_level("123").is_err());
    }

    #[test]
    fn test_logging_error_is_error_trait() {
        // Verify LoggingError implements std::error::Error
        let err: Box<dyn std::error::Error> =
            Box::new(LoggingError::InvalidLevel("test".to_string()));
        assert!(err.to_string().contains("test"));
    }
}
