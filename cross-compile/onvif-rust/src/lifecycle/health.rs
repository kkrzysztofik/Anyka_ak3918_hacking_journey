//! Health status and readiness checks for the ONVIF application.
//!
//! This module provides types and utilities for reporting application health,
//! component status, and degraded service information for observability.

use std::collections::HashMap;
use std::time::Duration;

/// Overall health state of the application or a component.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthState {
    /// All systems operational.
    Healthy,
    /// Some optional components are unavailable but core functionality works.
    Degraded,
    /// Critical components are failing.
    Unhealthy,
}

impl std::fmt::Display for HealthState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthState::Healthy => write!(f, "healthy"),
            HealthState::Degraded => write!(f, "degraded"),
            HealthState::Unhealthy => write!(f, "unhealthy"),
        }
    }
}

/// Health status of an individual component.
#[derive(Debug, Clone)]
pub struct ComponentHealth {
    /// Human-readable name of the component.
    pub name: String,
    /// Current health state.
    pub status: HealthState,
    /// Optional message with additional details.
    pub message: Option<String>,
}

impl ComponentHealth {
    /// Create a healthy component status.
    pub fn healthy(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: HealthState::Healthy,
            message: None,
        }
    }

    /// Create a degraded component status with a message.
    pub fn degraded(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: HealthState::Degraded,
            message: Some(message.into()),
        }
    }

    /// Create an unhealthy component status with a message.
    pub fn unhealthy(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: HealthState::Unhealthy,
            message: Some(message.into()),
        }
    }
}

/// Overall health status of the application.
#[derive(Debug, Clone)]
pub struct HealthStatus {
    /// Overall application health state.
    pub status: HealthState,
    /// Time since application started.
    pub uptime: Duration,
    /// Health status of individual components.
    pub components: HashMap<String, ComponentHealth>,
    /// List of services running in degraded mode.
    pub degraded_services: Vec<String>,
}

impl HealthStatus {
    /// Create a new healthy status with the given uptime.
    pub fn new(uptime: Duration) -> Self {
        Self {
            status: HealthState::Healthy,
            uptime,
            components: HashMap::new(),
            degraded_services: Vec::new(),
        }
    }

    /// Add a component's health status.
    pub fn add_component(&mut self, key: impl Into<String>, health: ComponentHealth) {
        let key = key.into();

        // Update overall status based on component health
        match health.status {
            HealthState::Unhealthy => {
                self.status = HealthState::Unhealthy;
            }
            HealthState::Degraded if self.status == HealthState::Healthy => {
                self.status = HealthState::Degraded;
            }
            _ => {}
        }

        self.components.insert(key, health);
    }

    /// Mark a service as degraded.
    pub fn mark_degraded(&mut self, service: impl Into<String>) {
        self.degraded_services.push(service.into());
        if self.status == HealthState::Healthy {
            self.status = HealthState::Degraded;
        }
    }

    /// Check if the application is healthy enough to serve requests.
    pub fn is_ready(&self) -> bool {
        matches!(self.status, HealthState::Healthy | HealthState::Degraded)
    }

    /// Check if the application is fully healthy (no degradation).
    pub fn is_healthy(&self) -> bool {
        self.status == HealthState::Healthy
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_state_display() {
        assert_eq!(format!("{}", HealthState::Healthy), "healthy");
        assert_eq!(format!("{}", HealthState::Degraded), "degraded");
        assert_eq!(format!("{}", HealthState::Unhealthy), "unhealthy");
    }

    #[test]
    fn test_component_health_constructors() {
        let healthy = ComponentHealth::healthy("test");
        assert_eq!(healthy.status, HealthState::Healthy);
        assert!(healthy.message.is_none());

        let degraded = ComponentHealth::degraded("test", "partially working");
        assert_eq!(degraded.status, HealthState::Degraded);
        assert_eq!(degraded.message, Some("partially working".to_string()));

        let unhealthy = ComponentHealth::unhealthy("test", "failed");
        assert_eq!(unhealthy.status, HealthState::Unhealthy);
        assert_eq!(unhealthy.message, Some("failed".to_string()));
    }

    #[test]
    fn test_health_status_new() {
        let status = HealthStatus::new(Duration::from_secs(60));
        assert_eq!(status.status, HealthState::Healthy);
        assert_eq!(status.uptime, Duration::from_secs(60));
        assert!(status.components.is_empty());
        assert!(status.degraded_services.is_empty());
    }

    #[test]
    fn test_health_status_add_healthy_component() {
        let mut status = HealthStatus::new(Duration::from_secs(60));
        status.add_component("config", ComponentHealth::healthy("Configuration"));

        assert_eq!(status.status, HealthState::Healthy);
        assert!(status.components.contains_key("config"));
    }

    #[test]
    fn test_health_status_add_degraded_component() {
        let mut status = HealthStatus::new(Duration::from_secs(60));
        status.add_component("config", ComponentHealth::healthy("Configuration"));
        status.add_component(
            "ptz",
            ComponentHealth::degraded("PTZ", "hardware unavailable"),
        );

        assert_eq!(status.status, HealthState::Degraded);
    }

    #[test]
    fn test_health_status_add_unhealthy_component() {
        let mut status = HealthStatus::new(Duration::from_secs(60));
        status.add_component("config", ComponentHealth::healthy("Configuration"));
        status.add_component(
            "platform",
            ComponentHealth::unhealthy("Platform", "init failed"),
        );

        assert_eq!(status.status, HealthState::Unhealthy);
    }

    #[test]
    fn test_health_status_mark_degraded() {
        let mut status = HealthStatus::new(Duration::from_secs(60));
        status.mark_degraded("PTZ");
        status.mark_degraded("Imaging");

        assert_eq!(status.status, HealthState::Degraded);
        assert_eq!(status.degraded_services, vec!["PTZ", "Imaging"]);
    }

    #[test]
    fn test_health_status_is_ready() {
        let healthy = HealthStatus::new(Duration::from_secs(60));
        assert!(healthy.is_ready());

        let mut degraded = HealthStatus::new(Duration::from_secs(60));
        degraded.mark_degraded("PTZ");
        assert!(degraded.is_ready());

        let mut unhealthy = HealthStatus::new(Duration::from_secs(60));
        unhealthy.add_component("platform", ComponentHealth::unhealthy("Platform", "failed"));
        assert!(!unhealthy.is_ready());
    }

    #[test]
    fn test_health_status_is_healthy() {
        let healthy = HealthStatus::new(Duration::from_secs(60));
        assert!(healthy.is_healthy());

        let mut degraded = HealthStatus::new(Duration::from_secs(60));
        degraded.mark_degraded("PTZ");
        assert!(!degraded.is_healthy());
    }
}
