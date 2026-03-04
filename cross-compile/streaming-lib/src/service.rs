//! Streaming service lifecycle management
//!
//! This module provides the `StreamingService` struct for managing the RTSP and HTTP-FLV
//! servers with proper lifecycle control (startup, graceful shutdown).

use thiserror::Error;

use tokio::sync::broadcast;
use tokio::task::JoinSet;

use crate::common::auth::Auth;
use crate::config::StreamingConfig;
use crate::hub::define::StreamHubEventSender;
use crate::protocol::httpflv::server::{DefaultHttpFlvServer, HttpFlvServer};
use crate::protocol::rtsp::{DefaultRtspServer, RtspServer};

/// Errors that can occur during streaming service operations
#[derive(Debug, Error)]
pub enum StreamingError {
    /// RTSP server error
    #[error("RTSP server error: {0}")]
    RtspServer(String),

    /// HTTP-FLV server error
    #[error("HTTP-FLV server error: {0}")]
    HttpFlv(String),

    /// Stream hub error
    #[error("Stream hub error: {0}")]
    StreamHub(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(String),
}

/// Streaming service that manages RTSP and HTTP-FLV servers
///
/// This struct provides a unified interface for starting and gracefully shutting down
/// both the RTSP and HTTP-FLV servers.
pub struct StreamingService {
    tasks: JoinSet<Result<(), StreamingError>>,
    shutdown_tx: broadcast::Sender<()>,
}

impl StreamingService {
    /// Create a new streaming service
    ///
    /// # Arguments
    ///
    /// * `config` - Streaming configuration
    /// * `event_sender` - StreamHub event sender for routing messages to the hub
    /// * `auth` - Authentication configuration. **MUST be provided for production deployments.**
    ///   Passing `None` allows unauthenticated access and should only be used for
    ///   local development or testing.
    ///
    /// # Returns
    ///
    /// A new `StreamingService` instance with spawned server tasks
    pub async fn new(
        config: StreamingConfig,
        event_sender: StreamHubEventSender,
        auth: Option<Auth>,
    ) -> Result<Self, StreamingError> {
        let mut tasks = JoinSet::new();
        let (shutdown_tx, _) = broadcast::channel(1);

        // Clone the event sender for each server
        let event_sender_rtsp = event_sender.clone();
        let event_sender_httpflv = event_sender.clone();

        // Spawn RTSP server
        let rtsp_shutdown = shutdown_tx.subscribe();
        let rtsp_config = config.clone();
        let auth_clone = auth.clone();
        let config_clone = config.clone();
        tasks.spawn(async move {
            let mut server: DefaultRtspServer = DefaultRtspServer::new(
                rtsp_config.rtsp_listen_addr.clone(),
                event_sender_rtsp,
                auth_clone,
                config_clone,
            );
            tokio::select! {
                result = server.run(Some(rtsp_shutdown)) => {
                    match result {
                        Ok(()) => Ok(()),
                        Err(e) => Err(StreamingError::RtspServer(e.to_string())),
                    }
                }
            }
        });

        // Spawn HTTP-FLV server
        let mut httpflv_shutdown = shutdown_tx.subscribe();
        let httpflv_config = config.clone();
        tasks.spawn(async move {
            let mut server: DefaultHttpFlvServer = DefaultHttpFlvServer::new(
                httpflv_config.httpflv_listen_addr.clone(),
                event_sender_httpflv,
                auth,
            );
            tokio::select! {
                result = server.run() => {
                    match result {
                        Ok(()) => Ok(()),
                        Err(e) => Err(StreamingError::HttpFlv(e.to_string())),
                    }
                }
                _ = httpflv_shutdown.recv() => Ok(()),
            }
        });

        Ok(Self { tasks, shutdown_tx })
    }

    /// Gracefully shutdown the streaming service
    ///
    /// This method sends a shutdown signal to all servers and waits for them
    /// to complete. It returns a `ShutdownReport` with information about
    /// the shutdown status of each server.
    ///
    /// # Returns
    ///
    /// A `ShutdownReport` with the results of the shutdown
    pub async fn shutdown(mut self) -> ShutdownReport {
        // Signal all servers to shut down
        let _ = self.shutdown_tx.send(());

        let mut report = ShutdownReport::default();

        // Wait for all tasks to complete
        while let Some(result) = self.tasks.join_next().await {
            match result {
                Ok(Ok(())) => {
                    report.success_count += 1;
                }
                Ok(Err(e)) => {
                    report.failed_count += 1;
                    report.errors.push(e.to_string());
                }
                Err(e) => {
                    report.failed_count += 1;
                    report.errors.push(format!("Task panicked: {}", e));
                }
            }
        }

        report
    }

    /// Check if the service is still running
    ///
    /// # Returns
    ///
    /// `true` if there are still running tasks, `false` otherwise
    pub fn is_running(&self) -> bool {
        !self.tasks.is_empty()
    }
}

/// Report of a graceful shutdown operation
///
/// This struct contains information about the results of shutting down
/// the streaming service.
#[derive(Debug, Default)]
pub struct ShutdownReport {
    /// Number of servers that shut down successfully
    pub success_count: usize,
    /// Number of servers that failed to shut down
    pub failed_count: usize,
    /// List of error messages from failed shutdowns
    pub errors: Vec<String>,
}

impl ShutdownReport {
    /// Check if the shutdown was completely successful
    ///
    /// # Returns
    ///
    /// `true` if all servers shut down successfully
    pub fn is_success(&self) -> bool {
        self.failed_count == 0
    }

    /// Get the total number of servers that were shut down
    ///
    /// # Returns
    ///
    /// The total count of servers
    pub fn total(&self) -> usize {
        self.success_count + self.failed_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    fn create_test_event_sender() -> StreamHubEventSender {
        let (tx, _) = mpsc::unbounded_channel();
        tx
    }

    #[test]
    fn test_shutdown_report_default() {
        let report = ShutdownReport::default();
        assert_eq!(report.success_count, 0);
        assert_eq!(report.failed_count, 0);
        assert!(report.errors.is_empty());
        assert!(report.is_success());
    }

    #[test]
    fn test_shutdown_report_with_errors() {
        let report = ShutdownReport {
            success_count: 1,
            failed_count: 1,
            errors: vec!["error 1".to_string()],
        };
        assert_eq!(report.total(), 2);
        assert!(!report.is_success());
    }

    #[test]
    fn test_streaming_error_display() {
        let err = StreamingError::RtspServer("test".to_string());
        assert!(format!("{}", err).contains("RTSP"));

        let err = StreamingError::HttpFlv("test".to_string());
        assert!(format!("{}", err).contains("HTTP-FLV"));

        let err = StreamingError::StreamHub("test".to_string());
        assert!(format!("{}", err).contains("Stream hub"));

        let err = StreamingError::Io("test".to_string());
        assert!(format!("{}", err).contains("IO"));
    }

    #[test]
    fn test_streaming_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::Other, "test error");
        let streaming_err = StreamingError::Io(io_err.to_string());
        assert!(format!("{}", streaming_err).contains("IO"));
    }
}
