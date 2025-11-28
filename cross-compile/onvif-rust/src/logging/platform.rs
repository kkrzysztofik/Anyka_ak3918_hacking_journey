//! Platform logging integration.
//!
//! This module provides integration with the Anyka SDK's logging system
//! (ak_print) to capture platform-level log messages into the unified
//! tracing system.

use std::ffi::CStr;
use std::os::raw::{c_char, c_int};

/// Log level mapping from Anyka SDK to tracing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum PlatformLogLevel {
    /// Reserved (not used).
    Reserved = 0,
    /// Error level.
    Error = 1,
    /// Warning level.
    Warning = 2,
    /// Notice level (maps to info).
    Notice = 3,
    /// Normal level (maps to info).
    Normal = 4,
    /// Info level (maps to debug for less noise).
    Info = 5,
    /// Debug level (maps to trace).
    Debug = 6,
}

impl From<i32> for PlatformLogLevel {
    fn from(level: i32) -> Self {
        match level {
            0 => PlatformLogLevel::Reserved,
            1 => PlatformLogLevel::Error,
            2 => PlatformLogLevel::Warning,
            3 => PlatformLogLevel::Notice,
            4 => PlatformLogLevel::Normal,
            5 => PlatformLogLevel::Info,
            6 => PlatformLogLevel::Debug,
            _ => PlatformLogLevel::Debug,
        }
    }
}

impl PlatformLogLevel {
    /// Convert to tracing log level.
    pub fn to_tracing_level(self) -> tracing::Level {
        match self {
            PlatformLogLevel::Error => tracing::Level::ERROR,
            PlatformLogLevel::Warning => tracing::Level::WARN,
            PlatformLogLevel::Notice => tracing::Level::INFO,
            PlatformLogLevel::Normal => tracing::Level::INFO,
            PlatformLogLevel::Info => tracing::Level::DEBUG,
            PlatformLogLevel::Debug => tracing::Level::TRACE,
            PlatformLogLevel::Reserved => tracing::Level::TRACE,
        }
    }
}

/// Log a message from the platform (Anyka SDK).
///
/// This function is called from FFI code to bridge platform logs
/// into the Rust tracing system.
///
/// # Safety
///
/// This function is safe as long as the `message` pointer is valid
/// and null-terminated.
pub fn log_platform_message(level: PlatformLogLevel, message: &str) {
    match level {
        PlatformLogLevel::Error => {
            tracing::error!(target: "platform", "{}", message);
        }
        PlatformLogLevel::Warning => {
            tracing::warn!(target: "platform", "{}", message);
        }
        PlatformLogLevel::Notice | PlatformLogLevel::Normal => {
            tracing::info!(target: "platform", "{}", message);
        }
        PlatformLogLevel::Info => {
            tracing::debug!(target: "platform", "{}", message);
        }
        PlatformLogLevel::Debug | PlatformLogLevel::Reserved => {
            tracing::trace!(target: "platform", "{}", message);
        }
    }
}

/// FFI wrapper for ak_print that redirects to Rust logging.
///
/// This function can be used as a replacement for the Anyka SDK's ak_print
/// when we want to capture all platform logs in our unified logging system.
///
/// # Safety
///
/// This function is unsafe because it deals with raw C pointers.
/// The caller must ensure:
/// - `message` is a valid, null-terminated C string
/// - `message` remains valid for the duration of the call
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ak_print_wrapper(level: c_int, message: *const c_char) -> c_int {
    if message.is_null() {
        return -1;
    }

    // SAFETY: We've checked that message is not null.
    // Caller guarantees it's a valid C string.
    let c_str = unsafe { CStr::from_ptr(message) };

    match c_str.to_str() {
        Ok(msg) => {
            // Trim trailing newlines
            let msg = msg.trim_end_matches('\n');
            let log_level = PlatformLogLevel::from(level);
            log_platform_message(log_level, msg);
            0
        }
        Err(_) => {
            // Invalid UTF-8, log as lossy string
            let msg = c_str.to_string_lossy();
            let msg = msg.trim_end_matches('\n');
            let log_level = PlatformLogLevel::from(level);
            log_platform_message(log_level, msg);
            0
        }
    }
}

/// Platform log context for structured platform logging.
#[derive(Debug, Clone)]
pub struct PlatformLogContext {
    /// Module name (e.g., "video", "audio", "ptz").
    pub module: String,
    /// Optional device identifier.
    pub device_id: Option<String>,
}

impl PlatformLogContext {
    /// Create a new platform log context.
    pub fn new(module: &str) -> Self {
        Self {
            module: module.to_string(),
            device_id: None,
        }
    }

    /// Add device ID to context.
    pub fn with_device(mut self, device_id: &str) -> Self {
        self.device_id = Some(device_id.to_string());
        self
    }

    /// Log an error message.
    pub fn error(&self, message: &str) {
        tracing::error!(
            target: "platform",
            module = %self.module,
            device_id = ?self.device_id,
            "{}", message
        );
    }

    /// Log a warning message.
    pub fn warn(&self, message: &str) {
        tracing::warn!(
            target: "platform",
            module = %self.module,
            device_id = ?self.device_id,
            "{}", message
        );
    }

    /// Log an info message.
    pub fn info(&self, message: &str) {
        tracing::info!(
            target: "platform",
            module = %self.module,
            device_id = ?self.device_id,
            "{}", message
        );
    }

    /// Log a debug message.
    pub fn debug(&self, message: &str) {
        tracing::debug!(
            target: "platform",
            module = %self.module,
            device_id = ?self.device_id,
            "{}", message
        );
    }

    /// Log a trace message.
    pub fn trace(&self, message: &str) {
        tracing::trace!(
            target: "platform",
            module = %self.module,
            device_id = ?self.device_id,
            "{}", message
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_log_level_from_i32() {
        assert_eq!(PlatformLogLevel::from(0), PlatformLogLevel::Reserved);
        assert_eq!(PlatformLogLevel::from(1), PlatformLogLevel::Error);
        assert_eq!(PlatformLogLevel::from(2), PlatformLogLevel::Warning);
        assert_eq!(PlatformLogLevel::from(3), PlatformLogLevel::Notice);
        assert_eq!(PlatformLogLevel::from(4), PlatformLogLevel::Normal);
        assert_eq!(PlatformLogLevel::from(5), PlatformLogLevel::Info);
        assert_eq!(PlatformLogLevel::from(6), PlatformLogLevel::Debug);
        assert_eq!(PlatformLogLevel::from(99), PlatformLogLevel::Debug);
    }

    #[test]
    fn test_platform_log_level_to_tracing() {
        assert_eq!(
            PlatformLogLevel::Error.to_tracing_level(),
            tracing::Level::ERROR
        );
        assert_eq!(
            PlatformLogLevel::Warning.to_tracing_level(),
            tracing::Level::WARN
        );
        assert_eq!(
            PlatformLogLevel::Notice.to_tracing_level(),
            tracing::Level::INFO
        );
        assert_eq!(
            PlatformLogLevel::Normal.to_tracing_level(),
            tracing::Level::INFO
        );
        assert_eq!(
            PlatformLogLevel::Info.to_tracing_level(),
            tracing::Level::DEBUG
        );
        assert_eq!(
            PlatformLogLevel::Debug.to_tracing_level(),
            tracing::Level::TRACE
        );
    }

    #[test]
    fn test_platform_log_context() {
        let ctx = PlatformLogContext::new("video").with_device("cam0");

        assert_eq!(ctx.module, "video");
        assert_eq!(ctx.device_id, Some("cam0".to_string()));
    }

    #[test]
    fn test_platform_log_context_without_device() {
        let ctx = PlatformLogContext::new("audio");

        assert_eq!(ctx.module, "audio");
        assert!(ctx.device_id.is_none());
    }

    #[test]
    fn test_platform_log_context_clone() {
        let ctx = PlatformLogContext::new("ptz").with_device("motor0");
        let cloned = ctx.clone();

        assert_eq!(ctx.module, cloned.module);
        assert_eq!(ctx.device_id, cloned.device_id);
    }

    #[test]
    fn test_log_platform_message_all_levels() {
        // These just verify they don't panic - actual logging is hard to test
        log_platform_message(PlatformLogLevel::Error, "test error");
        log_platform_message(PlatformLogLevel::Warning, "test warning");
        log_platform_message(PlatformLogLevel::Notice, "test notice");
        log_platform_message(PlatformLogLevel::Normal, "test normal");
        log_platform_message(PlatformLogLevel::Info, "test info");
        log_platform_message(PlatformLogLevel::Debug, "test debug");
        log_platform_message(PlatformLogLevel::Reserved, "test reserved");
    }

    #[test]
    fn test_platform_log_context_logging_methods() {
        // Initialize test logging to avoid panics
        let _ = tracing_subscriber::fmt()
            .with_test_writer()
            .try_init();

        let ctx = PlatformLogContext::new("test_module").with_device("dev0");

        // These just verify they don't panic
        ctx.error("test error message");
        ctx.warn("test warning message");
        ctx.info("test info message");
        ctx.debug("test debug message");
        ctx.trace("test trace message");
    }

    #[test]
    fn test_platform_log_level_boundary_values() {
        // Test boundary and invalid values
        assert_eq!(PlatformLogLevel::from(-1), PlatformLogLevel::Debug);
        assert_eq!(PlatformLogLevel::from(7), PlatformLogLevel::Debug);
        assert_eq!(PlatformLogLevel::from(100), PlatformLogLevel::Debug);
        assert_eq!(PlatformLogLevel::from(i32::MAX), PlatformLogLevel::Debug);
        assert_eq!(PlatformLogLevel::from(i32::MIN), PlatformLogLevel::Debug);
    }
}
