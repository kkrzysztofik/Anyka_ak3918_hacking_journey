use crate::hub::define::StreamHubEventSender;

use super::session::server_session::RtspServerSession;
use crate::common::auth::Auth;
use crate::config::StreamingConfig;
use async_trait::async_trait;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::io::Error;
use tokio::net::TcpListener;
use tokio::sync::Semaphore;
use tokio::sync::broadcast;

/// Trait for RTSP server implementations
///
/// This trait provides the abstraction for RTSP server functionality,
/// allowing platform-specific implementations (e.g., Anyka hardware integration).
#[async_trait]
pub trait RtspServer: Send + Sync {
    /// Create a new RTSP server instance
    fn new(
        address: String,
        event_producer: StreamHubEventSender,
        auth: Option<Auth>,
        config: StreamingConfig,
    ) -> Self
    where
        Self: Sized;

    /// Run the RTSP server
    /// The `shutdown_rx` parameter is optional - when received, signals server to stop
    async fn run(&mut self, shutdown_rx: Option<broadcast::Receiver<()>>) -> Result<(), Error>;

    /// Get shutdown flag for graceful shutdown
    fn get_shutdown_flag(&self) -> Option<Arc<AtomicBool>>;
}

/// Maximum number of concurrent RTSP sessions.
/// Sized for embedded targets (Anyka AK3918) with limited RAM.
const MAX_CONCURRENT_SESSIONS: usize = 16;

/// Default implementation of the RTSP server trait
pub struct DefaultRtspServer {
    address: String,
    event_producer: StreamHubEventSender,
    auth: Option<Auth>,
    config: StreamingConfig,
    shutdown_flag: Option<Arc<AtomicBool>>,
    session_semaphore: Arc<Semaphore>,
}

#[async_trait]
impl RtspServer for DefaultRtspServer {
    fn new(
        address: String,
        event_producer: StreamHubEventSender,
        auth: Option<Auth>,
        config: StreamingConfig,
    ) -> Self {
        Self {
            address,
            event_producer,
            auth,
            config,
            shutdown_flag: None,
            session_semaphore: Arc::new(Semaphore::new(MAX_CONCURRENT_SESSIONS)),
        }
    }

    fn get_shutdown_flag(&self) -> Option<Arc<AtomicBool>> {
        self.shutdown_flag.clone()
    }

    async fn run(&mut self, shutdown_rx: Option<broadcast::Receiver<()>>) -> Result<(), Error> {
        // Create shutdown flag for graceful shutdown
        let shutdown_flag = Arc::new(AtomicBool::new(false));
        self.shutdown_flag = Some(shutdown_flag.clone());

        let socket_addr: SocketAddr =
            self.address
                .parse()
                .map_err(|e: std::net::AddrParseError| {
                    Error::new(std::io::ErrorKind::InvalidInput, e.to_string())
                })?;
        let listener = TcpListener::bind(socket_addr).await?;

        tracing::info!(address = %socket_addr, "rtsp_server_start");

        // Clone flag for the accept loop
        let shutdown_flag_clone = shutdown_flag.clone();

        // Clone shutdown receiver for the select loop
        let mut shutdown_rx = shutdown_rx;

        loop {
            tokio::select! {
                result = listener.accept() => {
                    match result {
                        Ok((tcp_stream, peer_addr)) => {
                            // Acquire a session permit before spawning.
                            // try_acquire_owned is non-blocking: if all slots are taken
                            // we reject immediately (DoS protection).
                            let permit = match self.session_semaphore.clone().try_acquire_owned() {
                                Ok(p) => p,
                                Err(_) => {
                                    tracing::warn!(
                                        remote_addr = %peer_addr,
                                        limit = %MAX_CONCURRENT_SESSIONS,
                                        "rtsp_session_rejected_max_sessions"
                                    );
                                    // Drop the TcpStream (closes connection)
                                    drop(tcp_stream);
                                    continue;
                                }
                            };

                            tracing::info!(
                                remote_addr = %peer_addr,
                                local_addr = %socket_addr,
                                "rtsp_connection_start"
                            );
                            // Create session with shared shutdown flag
                            let mut session = RtspServerSession::new(
                                tcp_stream,
                                self.event_producer.clone(),
                                self.auth.clone(),
                                self.config.clone(),
                            );
                            // Set the shared shutdown flag for graceful shutdown
                            session.set_shutdown_flag(shutdown_flag_clone.clone());

                            tokio::spawn(async move {
                                // Hold the permit for the lifetime of this session.
                                // When this task completes, `_permit` is dropped, releasing
                                // the semaphore slot for a new connection.
                                let _permit = permit;
                                if let Err(err) = session.run().await {
                                    let session_id = if let Some(id) = session.session_id {
                                        id.to_string()
                                    } else {
                                        "none".to_string()
                                    };
                                    tracing::info!(
                                        session_id = %session_id,
                                        session_type = %session.session_type,
                                        error = %err,
                                        "session_run_exit"
                                    );

                                    if !session.is_normal_exit
                                        && let Some(identifier) = session.stream_identifier.clone()
                                    {
                                        match session.exit(identifier) {
                                            Err(err) => {
                                                tracing::error!(
                                                    session_id = %session_id,
                                                    session_type = %session.session_type,
                                                    error = %err,
                                                    "session_exit_error"
                                                );
                                            }
                                            Ok(()) => {
                                                tracing::info!(
                                                    session_id = %session_id,
                                                    session_type = %session.session_type,
                                                    "session_exit_success"
                                                );
                                            }
                                        }
                                    }
                                }
                            });
                        }
                        Err(e) => {
                            // Check if we should stop accepting
                            if shutdown_flag_clone.load(Ordering::Acquire) {
                                break;
                            }
                            tracing::error!(error = %e, "failed_accept_connection");
                        }
                    }
                }
                _ = async {
                    // Wait for shutdown signal if provided
                    if let Some(ref mut rx) = shutdown_rx {
                        let _ = rx.recv().await;
                    } else {
                        // No shutdown receiver provided - block forever
                        // This ensures the select! doesn't complete immediately
                        futures::future::pending::<()>().await;
                    }
                } => {
                    // Shutdown requested via broadcast
                    break;
                }
            }
        }

        // Signal all sessions to shut down
        shutdown_flag.store(true, Ordering::Release);

        tracing::info!("rtsp_server_shutdown_complete");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::StreamingConfig;
    use crate::hub::define::StreamHubEventSender;
    use tokio::sync::mpsc;

    fn create_test_event_sender() -> StreamHubEventSender {
        let (tx, _) = mpsc::unbounded_channel();
        tx
    }

    fn create_test_config() -> StreamingConfig {
        StreamingConfig::default()
    }

    #[test]
    fn test_default_rtsp_server_new() {
        let event_sender = create_test_event_sender();
        let config = create_test_config();
        let server = DefaultRtspServer::new("127.0.0.1:0".to_string(), event_sender, None, config);
        assert_eq!(server.address, "127.0.0.1:0");
        assert!(server.auth.is_none());
    }

    #[test]
    fn test_default_rtsp_server_new_with_auth() {
        use crate::common::auth::{Auth, AuthAlgorithm, AuthType};
        let event_sender = create_test_event_sender();
        let config = create_test_config();
        let auth = Auth::new(
            "user".to_string(),
            "pass".to_string(),
            None,
            AuthAlgorithm::Simple,
            AuthType::None,
        );
        let server =
            DefaultRtspServer::new("0.0.0.0:554".to_string(), event_sender, Some(auth), config);
        assert_eq!(server.address, "0.0.0.0:554");
        assert!(server.auth.is_some());
    }

    #[tokio::test]
    async fn test_default_rtsp_server_run_invalid_address_returns_err() {
        let event_sender = create_test_event_sender();
        let config = create_test_config();
        let mut server =
            DefaultRtspServer::new("not-an-address".to_string(), event_sender, None, config);
        let result = server.run(None).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    }

    #[tokio::test]
    async fn test_default_rtsp_server_run_empty_address_returns_err() {
        let event_sender = create_test_event_sender();
        let config = create_test_config();
        let mut server = DefaultRtspServer::new("".to_string(), event_sender, None, config);
        let result = server.run(None).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    }

    #[tokio::test]
    async fn test_default_rtsp_server_run_invalid_port_returns_err() {
        let event_sender = create_test_event_sender();
        let config = create_test_config();
        let mut server =
            DefaultRtspServer::new("127.0.0.1:99999".to_string(), event_sender, None, config);
        let result = server.run(None).await;
        assert!(result.is_err());
    }
}
