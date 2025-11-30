//! Async configuration persistence service.
//!
//! This module provides a background service that handles configuration saves
//! with debouncing to avoid excessive disk writes. Changes are queued and
//! saved after a configurable delay, allowing multiple rapid changes to be
//! batched into a single write.
//!
//! # Features
//!
//! - Configurable debounce delay via `[server] config_save_delay_ms`
//! - Graceful shutdown with pending save flush
//! - Non-blocking save requests from handlers
//! - Integration with application shutdown coordinator
//!
//! # Example
//!
//! ```ignore
//! use onvif_rust::config::{ConfigPersistenceService, ConfigRuntime, ConfigStorage};
//!
//! let config = ConfigStorage::load_or_default("/etc/onvif/config.toml")?;
//! let runtime = Arc::new(ConfigRuntime::new(config));
//! let storage = ConfigStorage::new("/etc/onvif/config.toml");
//!
//! let (service, handle) = ConfigPersistenceService::new(
//!     runtime.clone(),
//!     storage,
//!     500, // 500ms debounce
//! );
//!
//! // Spawn the service
//! tokio::spawn(service.run(shutdown_rx));
//!
//! // Request a save (non-blocking)
//! handle.request_save();
//! ```

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{broadcast, mpsc};
use tokio::time::{Instant, sleep};

use super::runtime::ConfigRuntime;
use super::storage::ConfigStorage;

/// Default debounce delay in milliseconds.
pub const DEFAULT_SAVE_DELAY_MS: u64 = 500;

/// Capacity of the save request channel.
const CHANNEL_CAPACITY: usize = 16;

/// Handle for requesting configuration saves.
///
/// This handle can be cloned and shared across multiple tasks/handlers.
/// Calling `request_save()` is non-blocking.
#[derive(Clone)]
pub struct ConfigPersistenceHandle {
    /// Sender for save requests.
    save_tx: mpsc::Sender<()>,
}

impl ConfigPersistenceHandle {
    /// Request a configuration save.
    ///
    /// This is a non-blocking operation. If the channel is full, the request
    /// is dropped (a save is already pending anyway).
    pub fn request_save(&self) {
        // Use try_send to avoid blocking - if channel is full, a save is pending
        let _ = self.save_tx.try_send(());
    }
}

/// Configuration persistence service.
///
/// Runs in the background and handles save requests with debouncing.
pub struct ConfigPersistenceService {
    /// Runtime configuration to save.
    runtime: Arc<ConfigRuntime>,
    /// Storage backend.
    storage: ConfigStorage,
    /// Debounce delay.
    save_delay: Duration,
    /// Receiver for save requests.
    save_rx: mpsc::Receiver<()>,
}

impl ConfigPersistenceService {
    /// Create a new persistence service and handle.
    ///
    /// # Arguments
    ///
    /// * `runtime` - The configuration runtime to persist
    /// * `storage` - The storage backend for saving
    /// * `save_delay_ms` - Debounce delay in milliseconds
    ///
    /// # Returns
    ///
    /// A tuple of (service, handle). The service should be spawned as a task,
    /// and the handle is used to request saves.
    pub fn new(
        runtime: Arc<ConfigRuntime>,
        storage: ConfigStorage,
        save_delay_ms: u64,
    ) -> (Self, ConfigPersistenceHandle) {
        let (save_tx, save_rx) = mpsc::channel(CHANNEL_CAPACITY);

        let service = Self {
            runtime,
            storage,
            save_delay: Duration::from_millis(save_delay_ms),
            save_rx,
        };

        let handle = ConfigPersistenceHandle { save_tx };

        (service, handle)
    }

    /// Run the persistence service.
    ///
    /// This method runs until shutdown is signaled. On shutdown, any pending
    /// save is flushed with a short timeout.
    ///
    /// # Arguments
    ///
    /// * `shutdown_rx` - Receiver for shutdown signal
    pub async fn run(mut self, mut shutdown_rx: broadcast::Receiver<()>) {
        tracing::info!("Config persistence service started");

        let mut pending_save = false;
        let mut last_request: Option<Instant> = None;

        loop {
            tokio::select! {
                // Handle save requests
                result = self.save_rx.recv() => {
                    match result {
                        Some(()) => {
                            pending_save = true;
                            last_request = Some(Instant::now());
                            tracing::debug!("Config save requested, debouncing for {:?}", self.save_delay);
                        }
                        None => {
                            // All senders dropped, exit
                            tracing::debug!("All save handles dropped, shutting down");
                            break;
                        }
                    }
                }

                // Handle shutdown signal
                _ = shutdown_rx.recv() => {
                    tracing::info!("Config persistence service received shutdown signal");
                    if pending_save {
                        tracing::info!("Flushing pending config save before shutdown");
                        self.do_save();
                    }
                    break;
                }

                // Handle debounce timer
                _ = sleep(self.save_delay), if pending_save => {
                    if let Some(last) = last_request {
                        if last.elapsed() >= self.save_delay {
                            self.do_save();
                            pending_save = false;
                            last_request = None;
                        }
                    }
                }
            }
        }

        tracing::info!("Config persistence service stopped");
    }

    /// Perform the actual save operation.
    fn do_save(&self) {
        tracing::debug!("Saving configuration to {}", self.storage.path());

        // Get a snapshot of the current config
        let config = self.runtime.snapshot();

        match self.storage.save(&config) {
            Ok(()) => {
                tracing::info!(
                    "Configuration saved successfully to {}",
                    self.storage.path()
                );
            }
            Err(e) => {
                tracing::error!("Failed to save configuration: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use tempfile::NamedTempFile;

    use crate::config::ApplicationConfig;

    #[tokio::test]
    async fn test_persistence_handle_request_save() {
        let config = ApplicationConfig::new();
        let runtime = Arc::new(ConfigRuntime::new(config));
        let temp_file = NamedTempFile::new().unwrap();
        let storage = ConfigStorage::new(temp_file.path().to_str().unwrap());

        let (_service, handle) = ConfigPersistenceService::new(runtime, storage, 100);

        // Should not block or panic
        handle.request_save();
        handle.request_save();
        handle.request_save();
    }

    #[tokio::test]
    async fn test_persistence_service_debounce() {
        let mut config = ApplicationConfig::new();
        config.set("test.key", "value1".to_string());
        let runtime = Arc::new(ConfigRuntime::new(config));
        let temp_file = NamedTempFile::new().unwrap();
        let storage = ConfigStorage::new(temp_file.path().to_str().unwrap());

        let (service, handle) = ConfigPersistenceService::new(runtime.clone(), storage, 50);

        let (shutdown_tx, shutdown_rx) = broadcast::channel(1);

        // Spawn the service
        let service_handle = tokio::spawn(service.run(shutdown_rx));

        // Request multiple saves rapidly
        handle.request_save();
        tokio::time::sleep(Duration::from_millis(10)).await;
        handle.request_save();
        tokio::time::sleep(Duration::from_millis(10)).await;
        handle.request_save();

        // Wait for debounce
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Shutdown
        let _ = shutdown_tx.send(());
        service_handle.await.unwrap();

        // Verify file was saved
        let content = std::fs::read_to_string(temp_file.path()).unwrap();
        assert!(content.contains("test"));
    }

    #[tokio::test]
    async fn test_persistence_service_shutdown_flushes_pending() {
        let mut config = ApplicationConfig::new();
        config.set("flush.test", "pending".to_string());
        let runtime = Arc::new(ConfigRuntime::new(config));
        let temp_file = NamedTempFile::new().unwrap();
        let storage = ConfigStorage::new(temp_file.path().to_str().unwrap());

        let (service, handle) = ConfigPersistenceService::new(runtime, storage, 5000); // Long delay

        let (shutdown_tx, shutdown_rx) = broadcast::channel(1);

        // Spawn the service
        let service_handle = tokio::spawn(service.run(shutdown_rx));

        // Request save
        handle.request_save();

        // Immediately shutdown (before debounce would fire)
        tokio::time::sleep(Duration::from_millis(10)).await;
        let _ = shutdown_tx.send(());
        service_handle.await.unwrap();

        // Verify file was saved despite not waiting for debounce
        let content = std::fs::read_to_string(temp_file.path()).unwrap();
        assert!(content.contains("flush"));
    }
}
