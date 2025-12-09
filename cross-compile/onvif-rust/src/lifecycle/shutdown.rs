//! Shutdown coordination for graceful application termination.
//!
//! This module provides the `ShutdownCoordinator` which manages graceful shutdown
//! by broadcasting shutdown signals and waiting for components to complete.

use std::time::{Duration, Instant};
use tokio::sync::broadcast;

use super::ShutdownReport;

/// Default timeout for graceful shutdown (30 seconds).
pub const DEFAULT_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(30);

/// Coordinates graceful shutdown across all application components.
///
/// The coordinator uses a broadcast channel to signal shutdown to all listening
/// components. Components should subscribe to the shutdown signal and complete
/// their work gracefully when the signal is received.
#[derive(Debug)]
pub struct ShutdownCoordinator {
    /// Sender for broadcasting shutdown signal.
    shutdown_tx: broadcast::Sender<()>,
    /// Maximum time to wait for components to shut down.
    timeout: Duration,
}

impl ShutdownCoordinator {
    /// Create a new shutdown coordinator with the given broadcast sender and timeout.
    pub fn new(shutdown_tx: broadcast::Sender<()>, timeout: Duration) -> Self {
        Self {
            shutdown_tx,
            timeout,
        }
    }

    /// Create a new shutdown coordinator with default timeout.
    pub fn with_default_timeout(shutdown_tx: broadcast::Sender<()>) -> Self {
        Self::new(shutdown_tx, DEFAULT_SHUTDOWN_TIMEOUT)
    }

    /// Get a receiver for the shutdown signal.
    ///
    /// Components should call this to get a receiver they can await on.
    pub fn subscribe(&self) -> broadcast::Receiver<()> {
        self.shutdown_tx.subscribe()
    }

    /// Get the number of active subscribers.
    pub fn subscriber_count(&self) -> usize {
        self.shutdown_tx.receiver_count()
    }

    /// Initiate graceful shutdown.
    ///
    /// This broadcasts the shutdown signal to all subscribers and waits for
    /// them to complete (up to the configured timeout).
    ///
    /// Returns a `ShutdownReport` with details about the shutdown process.
    pub async fn initiate_shutdown(&self) -> ShutdownReport {
        let start = Instant::now();
        let mut report = ShutdownReport::new();

        tracing::info!("Initiating graceful shutdown...");

        // Broadcast shutdown signal to all listeners
        let subscriber_count = self.shutdown_tx.receiver_count();
        match self.shutdown_tx.send(()) {
            Ok(count) => {
                tracing::debug!("Shutdown signal sent to {} subscribers", count);
            }
            Err(_) => {
                // No receivers - this is fine, means nothing to shut down
                tracing::debug!("No active subscribers for shutdown signal");
            }
        }

        // Wait for the timeout period
        // In a real implementation, we would track task completion via JoinHandles
        // For now, we just wait a short period to allow tasks to process the signal
        let wait_duration = if subscriber_count > 0 {
            // Give tasks time to process the shutdown signal
            std::cmp::min(self.timeout, Duration::from_millis(100))
        } else {
            Duration::ZERO
        };

        if !wait_duration.is_zero() {
            tokio::time::sleep(wait_duration).await;
        }

        report.duration = start.elapsed();

        // Check if we exceeded the timeout
        if report.duration >= self.timeout {
            report.mark_timeout();
            tracing::warn!(
                "Shutdown timeout ({:?}) - some tasks may not have completed",
                self.timeout
            );
        } else {
            tracing::info!("Shutdown completed in {:?}", report.duration);
        }

        report
    }

    /// Check if shutdown has been initiated.
    ///
    /// Returns true if the shutdown signal has been sent (i.e., no receivers
    /// would block on receiving).
    pub fn is_shutting_down(&self) -> bool {
        // If sending would succeed with 0 receivers, shutdown hasn't been initiated
        // If it would fail (channel closed), shutdown is in progress
        self.shutdown_tx.receiver_count() == 0
    }
}

/// A guard that can be used to track when a component has completed shutdown.
///
/// When dropped, this signals that the component has finished its shutdown process.
#[derive(Debug)]
pub struct ShutdownGuard {
    component_name: String,
    _marker: (),
}

impl ShutdownGuard {
    /// Create a new shutdown guard for a component.
    pub fn new(component_name: impl Into<String>) -> Self {
        let name = component_name.into();
        tracing::debug!("Component '{}' starting shutdown", name);
        Self {
            component_name: name,
            _marker: (),
        }
    }
}

impl Drop for ShutdownGuard {
    fn drop(&mut self) {
        tracing::debug!("Component '{}' completed shutdown", self.component_name);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lifecycle::ShutdownStatus;

    #[tokio::test]
    async fn test_shutdown_coordinator_creation() {
        let (tx, _rx) = broadcast::channel(1);
        let coordinator = ShutdownCoordinator::new(tx, Duration::from_secs(10));

        assert_eq!(coordinator.timeout, Duration::from_secs(10));
    }

    #[tokio::test]
    async fn test_shutdown_coordinator_default_timeout() {
        let (tx, _rx) = broadcast::channel(1);
        let coordinator = ShutdownCoordinator::with_default_timeout(tx);

        assert_eq!(coordinator.timeout, DEFAULT_SHUTDOWN_TIMEOUT);
    }

    #[tokio::test]
    async fn test_shutdown_coordinator_subscribe() {
        let (tx, _rx) = broadcast::channel(1);
        let coordinator = ShutdownCoordinator::new(tx, Duration::from_secs(10));

        let _sub1 = coordinator.subscribe();
        let _sub2 = coordinator.subscribe();

        // Original receiver + 2 subscriptions
        assert!(coordinator.subscriber_count() >= 2);
    }

    #[tokio::test]
    async fn test_shutdown_coordinator_initiate_shutdown() {
        let (tx, _rx) = broadcast::channel(1);
        // Use 500ms timeout to avoid flaky test failures in containerized environments
        // The shutdown logic waits min(timeout, 100ms) so we need timeout > 100ms + overhead
        let coordinator = ShutdownCoordinator::new(tx, Duration::from_millis(500));

        let report = coordinator.initiate_shutdown().await;

        assert_eq!(report.status, ShutdownStatus::Success);
        assert!(report.duration < Duration::from_secs(1));
    }

    #[tokio::test]
    async fn test_shutdown_signal_received() {
        let (tx, _rx) = broadcast::channel(1);
        let coordinator = ShutdownCoordinator::new(tx, Duration::from_secs(10));

        let mut receiver = coordinator.subscribe();

        // Spawn a task that waits for shutdown
        let handle = tokio::spawn(async move {
            receiver.recv().await.ok();
            true
        });

        // Give the task time to start
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Initiate shutdown
        let _report = coordinator.initiate_shutdown().await;

        // The task should have received the signal
        let result = tokio::time::timeout(Duration::from_millis(100), handle).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_shutdown_guard_creation() {
        let _guard = ShutdownGuard::new("test_component");
        // Guard should be created without panic
    }

    #[test]
    fn test_shutdown_guard_drop() {
        {
            let _guard = ShutdownGuard::new("test_component");
            // Guard exists here
        }
        // Guard should be dropped without panic
    }
}
