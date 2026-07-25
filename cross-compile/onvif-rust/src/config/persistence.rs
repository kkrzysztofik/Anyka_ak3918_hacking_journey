//! Async configuration persistence service.
//!
//! This module provides a generic background service that handles debounced,
//! off-executor persistence for any shared state that can be snapshotted under
//! a lock and serialized to a file. Changes are queued via a non-blocking
//! [`PersistenceHandle::request_save`] and flushed after a configurable delay,
//! allowing multiple rapid changes to be batched into a single write.
//!
//! # Features
//!
//! - Configurable debounce delay
//! - Graceful shutdown with pending save flush
//! - Non-blocking save requests from async handlers
//! - Snapshots state under the caller's lock, then performs the blocking file
//!   write (temp + `fsync` + rename) on a `spawn_blocking` thread so the async
//!   executor is never blocked on SD-card fsync latency
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

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{broadcast, mpsc};
use tokio::time::{Instant, sleep};

use super::file_ops::atomic_write;
use super::runtime::ConfigRuntime;
use super::storage::ConfigStorage;

/// Default debounce delay in milliseconds.
pub const DEFAULT_SAVE_DELAY_MS: u64 = 500;

/// Capacity of the save request channel.
const CHANNEL_CAPACITY: usize = 16;

/// A serialized snapshot ready to be flushed to disk atomically.
///
/// Produced by a [`SnapshotFn`] while holding the source state's lock, then
/// written on a blocking thread without holding any lock.
pub struct PendingWrite {
    /// Target file path.
    pub path: PathBuf,
    /// Serialized bytes to write.
    pub bytes: Vec<u8>,
    /// Optional Unix file permissions (e.g. `Some(0o600)` for secrets).
    pub mode: Option<u32>,
}

/// Snapshot closure: acquires the source lock, serializes state, and returns
/// the bytes to persist. Returns `None` when there is nothing to persist or
/// serialization fails (the closure is responsible for logging in that case).
pub type SnapshotFn = Box<dyn Fn() -> Option<PendingWrite> + Send + Sync>;

/// Handle for requesting persistence flushes.
///
/// This handle can be cloned and shared across multiple tasks/handlers.
/// Calling `request_save()` is non-blocking.
#[derive(Clone)]
pub struct PersistenceHandle {
    /// Sender for save requests.
    save_tx: mpsc::Sender<()>,
}

impl PersistenceHandle {
    /// Request a save.
    ///
    /// This is a non-blocking operation. If the channel is full, the request
    /// is dropped (a save is already pending anyway).
    pub fn request_save(&self) {
        // Use try_send to avoid blocking - if channel is full, a save is pending
        let _ = self.save_tx.try_send(());
    }
}

/// Generic debounced, off-executor persistence service.
///
/// Runs in the background and handles save requests with debouncing. When the
/// debounce timer fires (or on shutdown), it invokes the snapshot closure to
/// serialize state under its lock, then performs the atomic file write on a
/// `spawn_blocking` thread.
pub struct PersistenceService {
    /// Human-readable name for logging (e.g. "config", "profiles", "users").
    name: &'static str,
    /// Debounce delay.
    save_delay: Duration,
    /// Receiver for save requests.
    save_rx: mpsc::Receiver<()>,
    /// Snapshot + serialize closure.
    snapshot: SnapshotFn,
}

impl PersistenceService {
    /// Create a new persistence service and handle.
    pub fn new(
        name: &'static str,
        save_delay_ms: u64,
        snapshot: SnapshotFn,
    ) -> (Self, PersistenceHandle) {
        let (save_tx, save_rx) = mpsc::channel(CHANNEL_CAPACITY);
        let service = Self {
            name,
            save_delay: Duration::from_millis(save_delay_ms),
            save_rx,
            snapshot,
        };
        (service, PersistenceHandle { save_tx })
    }

    /// Run the persistence service until shutdown is signaled.
    ///
    /// On shutdown, any pending save is flushed.
    pub async fn run(mut self, mut shutdown_rx: broadcast::Receiver<()>) {
        tracing::info!("{} persistence service started", self.name);

        let mut pending_save = false;
        let mut last_request: Option<Instant> = None;

        loop {
            tokio::select! {
                result = self.save_rx.recv() => {
                    match result {
                        Some(()) => {
                            pending_save = true;
                            last_request = Some(Instant::now());
                            tracing::debug!(
                                "{} save requested, debouncing for {:?}",
                                self.name,
                                self.save_delay
                            );
                        }
                        None => {
                            tracing::debug!("All {} save handles dropped, shutting down", self.name);
                            break;
                        }
                    }
                }

                _ = shutdown_rx.recv() => {
                    tracing::info!("{} persistence service received shutdown signal", self.name);
                    if pending_save {
                        tracing::info!("Flushing pending {} save before shutdown", self.name);
                        self.do_save().await;
                    }
                    break;
                }

                _ = sleep(self.save_delay), if pending_save => {
                    if let Some(last) = last_request
                        && last.elapsed() >= self.save_delay
                    {
                        self.do_save().await;
                        pending_save = false;
                        last_request = None;
                    }
                }
            }
        }

        tracing::info!("{} persistence service stopped", self.name);
    }

    /// Snapshot state (under lock) and perform the atomic write on a blocking thread.
    async fn do_save(&self) {
        let Some(pending) = (self.snapshot)() else {
            return;
        };

        let name = self.name;
        let path_for_log = pending.path.clone();
        let result = tokio::task::spawn_blocking(move || {
            atomic_write(&pending.path, &pending.bytes, pending.mode)
        })
        .await;

        match result {
            Ok(Ok(())) => {
                tracing::info!("{} saved successfully to {}", name, path_for_log.display());
            }
            Ok(Err(e)) => {
                tracing::error!(
                    "Failed to save {} to {}: {}",
                    name,
                    path_for_log.display(),
                    e
                );
            }
            Err(e) => {
                tracing::error!("{} save task panicked: {}", name, e);
            }
        }
    }
}

/// Handle for requesting configuration saves.
///
/// Alias of the generic [`PersistenceHandle`], kept as a distinct name for the
/// configuration wiring in [`crate::app`].
pub type ConfigPersistenceHandle = PersistenceHandle;

/// Configuration persistence service.
///
/// Thin wrapper around [`PersistenceService`] that snapshots [`ConfigRuntime`]
/// and serializes it to the [`ConfigStorage`] path.
pub struct ConfigPersistenceService {
    inner: PersistenceService,
}

impl ConfigPersistenceService {
    /// Create a new configuration persistence service and handle.
    ///
    /// # Arguments
    ///
    /// * `runtime` - The configuration runtime to persist
    /// * `storage` - The storage backend (used for its file path)
    /// * `save_delay_ms` - Debounce delay in milliseconds
    pub fn new(
        runtime: Arc<ConfigRuntime>,
        storage: ConfigStorage,
        save_delay_ms: u64,
    ) -> (Self, ConfigPersistenceHandle) {
        let path = PathBuf::from(storage.path());
        let snapshot: SnapshotFn = Box::new(move || {
            let config = runtime.snapshot();
            match toml::to_string_pretty(&config) {
                Ok(content) => Some(PendingWrite {
                    path: path.clone(),
                    bytes: content.into_bytes(),
                    mode: None,
                }),
                Err(e) => {
                    tracing::error!("Failed to serialize configuration: {}", e);
                    None
                }
            }
        });

        let (inner, handle) = PersistenceService::new("config", save_delay_ms, snapshot);
        (Self { inner }, handle)
    }

    /// Run the persistence service until shutdown is signaled.
    pub async fn run(self, shutdown_rx: broadcast::Receiver<()>) {
        self.inner.run(shutdown_rx).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    use crate::config::AppConfig;

    #[tokio::test]
    async fn test_persistence_handle_request_save() {
        let config = AppConfig::default();
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
        let mut config = AppConfig::default();
        config.device.manufacturer = "DebounceTest".to_string();
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
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Shutdown
        let _ = shutdown_tx.send(());
        service_handle.await.unwrap();

        // Verify file was saved with our custom manufacturer
        let content = std::fs::read_to_string(temp_file.path()).unwrap();
        assert!(content.contains("DebounceTest"));
    }

    #[tokio::test]
    async fn test_persistence_service_shutdown_flushes_pending() {
        let mut config = AppConfig::default();
        config.device.manufacturer = "FlushTest".to_string();
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
        assert!(content.contains("FlushTest"));
    }
}
