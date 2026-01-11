use crate::streamhub::define::StreamHubEventSender;

use super::session::server_session::RtspServerSession;
use crate::common::auth::Auth;
use async_trait::async_trait;
use std::net::SocketAddr;
use tokio::io::Error;
use tokio::net::TcpListener;

/// Trait for RTSP server implementations
///
/// This trait provides the abstraction for RTSP server functionality,
/// allowing platform-specific implementations (e.g., Anyka hardware integration).
#[async_trait]
pub trait RtspServer: Send + Sync {
    /// Create a new RTSP server instance
    fn new(address: String, event_producer: StreamHubEventSender, auth: Option<Auth>) -> Self
    where
        Self: Sized;

    /// Run the RTSP server
    async fn run(&mut self) -> Result<(), Error>;
}

/// Default implementation of the RTSP server trait
pub struct DefaultRtspServer {
    address: String,
    event_producer: StreamHubEventSender,
    auth: Option<Auth>,
}

#[async_trait]
impl RtspServer for DefaultRtspServer {
    fn new(address: String, event_producer: StreamHubEventSender, auth: Option<Auth>) -> Self {
        Self {
            address,
            event_producer,
            auth,
        }
    }

    async fn run(&mut self) -> Result<(), Error> {
        let socket_addr: &SocketAddr = &self.address.parse().unwrap();
        let listener = TcpListener::bind(socket_addr).await?;

        log::info!("Rtsp server listening on tcp://{}", socket_addr);
        loop {
            let (tcp_stream, _) = listener.accept().await?;
            let mut session =
                RtspServerSession::new(tcp_stream, self.event_producer.clone(), self.auth.clone());
            tokio::spawn(async move {
                if let Err(err) = session.run().await {
                    let session_id = if let Some(id) = session.session_id {
                        id.to_string()
                    } else {
                        "none".to_string()
                    };
                    log::info!(
                        "session run exit: session id: {} session type: {} , err: {}",
                        session_id,
                        session.session_type,
                        err
                    );

                    if !session.is_normal_exit
                        && let Some(identifier) = session.stream_identifier.clone()
                    {
                        match session.exit(identifier) {
                            Err(err) => {
                                log::error!(
                                    "session exit error: session id: {} session type: {}, error info: {}",
                                    session_id,
                                    session.session_type,
                                    err
                                );
                            }
                            Ok(()) => {
                                log::info!(
                                    "session exit successfully: session id: {} session type: {} ",
                                    session_id,
                                    session.session_type,
                                );
                            }
                        }
                    }
                }
            });
        }
    }
}
