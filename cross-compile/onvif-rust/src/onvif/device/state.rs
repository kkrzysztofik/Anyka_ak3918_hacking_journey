//! Device runtime state management.
//!
//! This module provides the DeviceState struct which holds runtime state
//! that changes during operation but is not persisted to disk.
//! This includes scopes, discovery mode, and other transient state.

#![cfg_attr(not(test), allow(dead_code))]

use crate::onvif::types::common::DiscoveryMode;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Runtime state for the Device Service.
///
/// This struct holds state that changes during operation but is not
/// persisted to persistent storage (config, users). This includes:
/// - Device scopes for discovery
/// - Discovery mode (Discoverable/NonDiscoverable)
pub struct DeviceState {
    /// Device scopes for discovery.
    pub scopes: RwLock<Vec<crate::onvif::types::common::Scope>>,
    /// Current discovery mode.
    pub discovery_mode: RwLock<DiscoveryMode>,
}

impl DeviceState {
    /// Create a new DeviceState with default values.
    ///
    /// PTZ scope advertising defaults to enabled; call [`DeviceState::with_ptz`]
    /// to honor the `[ptz] enabled` config flag.
    pub fn new() -> Self {
        Self::with_ptz(true)
    }

    /// Create a new DeviceState whose fixed scopes reflect the given PTZ support.
    pub fn with_ptz(ptz_enabled: bool) -> Self {
        Self {
            scopes: RwLock::new(crate::onvif::device::ops::discovery::default_scopes(
                ptz_enabled,
            )),
            discovery_mode: RwLock::new(DiscoveryMode::Discoverable),
        }
    }

    /// Create a new DeviceState with custom initial scopes and discovery mode.
    pub fn with_values(
        scopes: Vec<crate::onvif::types::common::Scope>,
        discovery_mode: DiscoveryMode,
    ) -> Self {
        Self {
            scopes: RwLock::new(scopes),
            discovery_mode: RwLock::new(discovery_mode),
        }
    }

    /// Get the current scopes.
    pub async fn get_scopes(&self) -> Vec<crate::onvif::types::common::Scope> {
        // Note: clone required to release the RwLock before returning
        self.scopes.read().await.clone()
    }

    /// Replace all scopes.
    pub async fn set_scopes(&self, scopes: Vec<crate::onvif::types::common::Scope>) {
        *self.scopes.write().await = scopes;
    }

    /// Get the current discovery mode.
    pub async fn get_discovery_mode(&self) -> DiscoveryMode {
        // Note: clone required to release the RwLock before returning
        self.discovery_mode.read().await.clone()
    }

    /// Set the discovery mode.
    pub async fn set_discovery_mode(&self, mode: DiscoveryMode) {
        *self.discovery_mode.write().await = mode;
    }
}

impl Default for DeviceState {
    fn default() -> Self {
        Self::new()
    }
}

/// Type alias for shared reference to DeviceState.
pub type DeviceStateRef = Arc<DeviceState>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::onvif::types::common::{Scope, ScopeDefinition};

    #[tokio::test]
    async fn test_device_state_default() {
        let state = DeviceState::new();
        let scopes = state.get_scopes().await;
        assert!(!scopes.is_empty());

        let mode = state.get_discovery_mode().await;
        assert_eq!(mode, DiscoveryMode::Discoverable);
    }

    #[tokio::test]
    async fn test_device_state_set_scopes() {
        let state = DeviceState::new();

        let new_scopes = vec![Scope {
            scope_def: ScopeDefinition::Configurable,
            scope_item: "onvif://www.onvif.org/name/TestCamera".to_string(),
        }];

        state.set_scopes(new_scopes.clone()).await;

        let scopes = state.get_scopes().await;
        assert_eq!(scopes, new_scopes);
    }

    #[tokio::test]
    async fn test_device_state_set_discovery_mode() {
        let state = DeviceState::new();

        state
            .set_discovery_mode(DiscoveryMode::NonDiscoverable)
            .await;

        let mode = state.get_discovery_mode().await;
        assert_eq!(mode, DiscoveryMode::NonDiscoverable);
    }

    #[tokio::test]
    async fn test_device_state_with_values() {
        let custom_scopes = vec![Scope {
            scope_def: ScopeDefinition::Configurable,
            scope_item: "onvif://www.onvif.org/name/Custom".to_string(),
        }];

        let state = DeviceState::with_values(custom_scopes.clone(), DiscoveryMode::NonDiscoverable);

        assert_eq!(state.get_scopes().await, custom_scopes);
        assert_eq!(
            state.get_discovery_mode().await,
            DiscoveryMode::NonDiscoverable
        );
    }

    #[test]
    fn test_device_state_default_trait() {
        let _state: DeviceState = Default::default();
        // Just ensure it compiles and creates a valid instance
    }
}
