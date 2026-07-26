//! Device persistent state management.
//!
//! This module provides the DeviceStore struct which wraps persistent state
//! that is saved to disk. This includes configuration and user storage.

#![cfg_attr(not(test), allow(dead_code))]

use crate::config::{ConfigRuntime, UserStorage};
use std::sync::Arc;

/// Persistent state for the Device Service.
///
/// This struct holds state that is persisted to disk:
/// - Configuration runtime
/// - User storage
pub struct DeviceStore {
    /// Configuration runtime.
    pub(crate) config: Arc<ConfigRuntime>,
    /// User storage.
    pub(crate) users: Arc<UserStorage>,
}

impl DeviceStore {
    /// Create a new DeviceStore with default configuration.
    pub fn new(users: Arc<UserStorage>) -> Self {
        Self {
            config: Arc::new(ConfigRuntime::new(Default::default())),
            users,
        }
    }

    /// Create a new DeviceStore with custom configuration.
    pub fn with_config(users: Arc<UserStorage>, config: Arc<ConfigRuntime>) -> Self {
        Self { config, users }
    }

    /// Get a clone of the config reference.
    pub fn config(&self) -> Arc<ConfigRuntime> {
        Arc::clone(&self.config)
    }
}

/// Type alias for shared reference to DeviceStore.
pub type DeviceStoreRef = Arc<DeviceStore>;

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_store() -> DeviceStore {
        let users = Arc::new(UserStorage::new());
        DeviceStore::new(users)
    }

    #[test]
    fn test_device_store_new() {
        let store = create_test_store();
        assert!(Arc::clone(&store.users).as_ref().list_users().is_empty());
    }

    #[test]
    fn test_device_store_with_config() {
        let users = Arc::new(UserStorage::new());
        let config = Arc::new(ConfigRuntime::new(Default::default()));

        let store = DeviceStore::with_config(users, Arc::clone(&config));

        // Verify we can access config
        let _network = store.config.read().network.clone();
    }

    #[test]
    fn test_device_store_clone_references() {
        let store = create_test_store();

        let config1 = store.config();
        let config2 = store.config();

        // Both should point to the same underlying data
        let port1 = config1.read().server.port;
        let port2 = config2.read().server.port;
        assert_eq!(port1, port2);
    }
}
