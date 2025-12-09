//! Startup sequence for ordered application initialization.
//!
//! This module provides the startup sequence that initializes all application
//! components in the correct order, handling both required and optional components.

use super::StartupError;

/// Startup phase identifiers for logging and error reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupPhase {
    /// Phase 1: Loading and validating configuration.
    Configuration,
    /// Phase 2: Initializing platform abstraction.
    Platform,
    /// Phase 3: Initializing ONVIF services.
    Services,
    /// Phase 4: Initializing network (HTTP server, etc.).
    Network,
    /// Phase 5: Sending discovery announcements.
    Discovery,
}

impl std::fmt::Display for StartupPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StartupPhase::Configuration => write!(f, "Configuration"),
            StartupPhase::Platform => write!(f, "Platform"),
            StartupPhase::Services => write!(f, "Services"),
            StartupPhase::Network => write!(f, "Network"),
            StartupPhase::Discovery => write!(f, "Discovery"),
        }
    }
}

/// Result of an optional component initialization.
#[derive(Debug)]
pub enum OptionalInitResult<T> {
    /// Component initialized successfully.
    Success(T),
    /// Component initialization failed but application can continue.
    Failed { component: String, error: String },
    /// Component is disabled in configuration.
    Disabled,
}

impl<T> OptionalInitResult<T> {
    /// Convert to Option, logging any failures.
    pub fn into_option(self) -> Option<T> {
        match self {
            OptionalInitResult::Success(value) => Some(value),
            OptionalInitResult::Failed { component, error } => {
                tracing::warn!("Optional component '{}' unavailable: {}", component, error);
                None
            }
            OptionalInitResult::Disabled => None,
        }
    }

    /// Check if initialization was successful.
    pub fn is_success(&self) -> bool {
        matches!(self, OptionalInitResult::Success(_))
    }

    /// Get the error message if initialization failed.
    pub fn error_message(&self) -> Option<String> {
        match self {
            OptionalInitResult::Failed { error, .. } => Some(error.clone()),
            _ => None,
        }
    }
}

/// Builder for tracking startup progress and collecting degraded services.
#[derive(Debug, Default)]
pub struct StartupProgress {
    /// Current phase of startup.
    current_phase: Option<StartupPhase>,
    /// Services that are running in degraded mode.
    degraded_services: Vec<String>,
    /// Warnings collected during startup.
    warnings: Vec<String>,
}

impl StartupProgress {
    /// Create a new startup progress tracker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Begin a new startup phase.
    pub fn begin_phase(&mut self, phase: StartupPhase) {
        tracing::info!("Phase {}: {}...", phase as u8 + 1, phase);
        self.current_phase = Some(phase);
    }

    /// Complete the current phase successfully.
    pub fn complete_phase(&mut self) {
        if let Some(phase) = self.current_phase.take() {
            tracing::info!("Phase {} ({}) completed", phase as u8 + 1, phase);
        }
    }

    /// Record a degraded service.
    pub fn record_degraded(&mut self, service: impl Into<String>, reason: impl Into<String>) {
        let service = service.into();
        let reason = reason.into();
        tracing::warn!("Service '{}' unavailable: {}", service, reason);
        self.degraded_services.push(service);
        self.warnings.push(reason);
    }

    /// Record a warning during startup.
    pub fn record_warning(&mut self, warning: impl Into<String>) {
        let warning = warning.into();
        tracing::warn!("{}", warning);
        self.warnings.push(warning);
    }

    /// Get the list of degraded services.
    pub fn degraded_services(&self) -> &[String] {
        &self.degraded_services
    }

    /// Get the list of warnings.
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    /// Check if any services are degraded.
    pub fn has_degraded_services(&self) -> bool {
        !self.degraded_services.is_empty()
    }
}

/// Validate that a required component initialized successfully.
///
/// Returns an error if the result is Err, with context about the startup phase.
pub fn require<T, E: std::fmt::Display>(
    result: Result<T, E>,
    phase: StartupPhase,
    component: &str,
) -> Result<T, StartupError> {
    result.map_err(|e| {
        let msg = format!("{} initialization failed: {}", component, e);
        tracing::error!("{}", msg);
        match phase {
            StartupPhase::Configuration => StartupError::Config(msg),
            StartupPhase::Platform => StartupError::Platform(msg),
            StartupPhase::Services => StartupError::Services(msg),
            StartupPhase::Network => StartupError::Network(msg),
            StartupPhase::Discovery => StartupError::Network(msg),
        }
    })
}

/// Initialize an optional component, recording degradation if it fails.
///
/// This function attempts to initialize a component and returns an `OptionalInitResult`
/// that can be converted to an `Option<T>` while logging any failures.
pub fn optional<T, E: std::fmt::Display>(
    result: Result<T, E>,
    component: &str,
) -> OptionalInitResult<T> {
    match result {
        Ok(value) => OptionalInitResult::Success(value),
        Err(e) => OptionalInitResult::Failed {
            component: component.to_string(),
            error: e.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_startup_phase_display() {
        assert_eq!(format!("{}", StartupPhase::Configuration), "Configuration");
        assert_eq!(format!("{}", StartupPhase::Platform), "Platform");
        assert_eq!(format!("{}", StartupPhase::Services), "Services");
        assert_eq!(format!("{}", StartupPhase::Network), "Network");
        assert_eq!(format!("{}", StartupPhase::Discovery), "Discovery");
    }

    #[test]
    fn test_optional_init_result_success() {
        let result: OptionalInitResult<i32> = OptionalInitResult::Success(42);
        assert!(result.is_success());
        assert!(result.error_message().is_none());
    }

    #[test]
    fn test_optional_init_result_failed() {
        let result: OptionalInitResult<i32> = OptionalInitResult::Failed {
            component: "PTZ".to_string(),
            error: "hardware not found".to_string(),
        };
        assert!(!result.is_success());
        assert_eq!(
            result.error_message(),
            Some("hardware not found".to_string())
        );
    }

    #[test]
    fn test_optional_init_result_into_option() {
        let success: OptionalInitResult<i32> = OptionalInitResult::Success(42);
        assert_eq!(success.into_option(), Some(42));

        let failed: OptionalInitResult<i32> = OptionalInitResult::Failed {
            component: "test".to_string(),
            error: "error".to_string(),
        };
        assert_eq!(failed.into_option(), None);

        let disabled: OptionalInitResult<i32> = OptionalInitResult::Disabled;
        assert_eq!(disabled.into_option(), None);
    }

    #[test]
    fn test_startup_progress_new() {
        let progress = StartupProgress::new();
        assert!(progress.current_phase.is_none());
        assert!(progress.degraded_services.is_empty());
        assert!(progress.warnings.is_empty());
    }

    #[test]
    fn test_startup_progress_phases() {
        let mut progress = StartupProgress::new();

        progress.begin_phase(StartupPhase::Configuration);
        assert_eq!(progress.current_phase, Some(StartupPhase::Configuration));

        progress.complete_phase();
        assert!(progress.current_phase.is_none());
    }

    #[test]
    fn test_startup_progress_degraded() {
        let mut progress = StartupProgress::new();

        progress.record_degraded("PTZ", "hardware not found");
        progress.record_degraded("Imaging", "driver error");

        assert!(progress.has_degraded_services());
        assert_eq!(progress.degraded_services(), &["PTZ", "Imaging"]);
    }

    #[test]
    fn test_require_success() {
        let result: Result<i32, &str> = Ok(42);
        let value = require(result, StartupPhase::Configuration, "test");
        assert_eq!(value.unwrap(), 42);
    }

    #[test]
    fn test_require_failure() {
        let result: Result<i32, &str> = Err("test error");
        let value = require(result, StartupPhase::Configuration, "test");
        assert!(matches!(value, Err(StartupError::Config(_))));
    }

    #[test]
    fn test_require_failure_phases() {
        let err: Result<(), &str> = Err("error");

        assert!(matches!(
            require(err, StartupPhase::Platform, "test"),
            Err(StartupError::Platform(_))
        ));
        assert!(matches!(
            require(err, StartupPhase::Services, "test"),
            Err(StartupError::Services(_))
        ));
        assert!(matches!(
            require(err, StartupPhase::Network, "test"),
            Err(StartupError::Network(_))
        ));
    }

    #[test]
    fn test_optional_success() {
        let result: Result<i32, &str> = Ok(42);
        let opt = optional(result, "test");
        assert!(matches!(opt, OptionalInitResult::Success(42)));
    }

    #[test]
    fn test_optional_failure() {
        let result: Result<i32, &str> = Err("test error");
        let opt = optional(result, "test");
        assert!(matches!(opt, OptionalInitResult::Failed { .. }));
    }
}
