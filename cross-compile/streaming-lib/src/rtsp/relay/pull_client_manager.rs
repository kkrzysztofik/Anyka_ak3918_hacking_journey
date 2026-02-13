use {
    super::errors::RelayError,
    crate::streamhub::{
        define::{BroadcastEvent, BroadcastEventReceiver, StreamHubEventSender},
        errors::{StreamHubError, StreamHubErrorValue},
        stream::StreamIdentifier,
    },
    crate::{
        rtsp::rtsp_transport::ProtocolType,
        rtsp::session::{client_session::RtspClientSession, define::ClientSessionType},
    },
    std::{
        collections::HashMap,
        sync::{Arc, atomic::AtomicBool},
    },
    tokio::sync::{Mutex, mpsc},
};

type HubResult = Result<(), StreamHubError>;
type HubResultSender = mpsc::Sender<HubResult>;

pub struct RtspPullClientManager {
    clients: HashMap<String, Arc<AtomicBool>>,
    client_event_consumer: BroadcastEventReceiver,
    channel_event_producer: StreamHubEventSender,
}

impl RtspPullClientManager {
    pub fn new(consumer: BroadcastEventReceiver, producer: StreamHubEventSender) -> Self {
        Self {
            clients: HashMap::new(),
            client_event_consumer: consumer,
            channel_event_producer: producer,
        }
    }

    pub async fn run(&mut self) -> Result<(), RelayError> {
        log::info!("pull client run...");

        loop {
            let event = self.client_event_consumer.recv().await?;

            match event {
                BroadcastEvent::Subscribe {
                    id,
                    identifier,
                    server_address,
                    result_sender,
                } => {
                    self.handle_subscribe(id, identifier, server_address, result_sender)
                        .await;
                }
                BroadcastEvent::UnSubscribe { id, result_sender } => {
                    self.handle_unsubscribe(id, result_sender).await;
                }
                _ => {
                    log::info!("pull client receive other events");
                }
            }
        }
    }

    async fn handle_subscribe(
        &mut self,
        id: String,
        identifier: StreamIdentifier,
        server_address: Option<String>,
        result_sender: Option<HubResultSender>,
    ) {
        let Some(sender) = result_sender else {
            log::error!("missing result sender for subscribe event");
            return;
        };

        let StreamIdentifier::Rtsp { stream_path } = identifier else {
            return;
        };

        let Some(server_address) = server_address else {
            self.send_error(
                &sender,
                "The Rtsp subscribe parameters does not contain server address",
            )
            .await;
            log::error!(
                "The Rtsp subscribe parameters does not contain server address: {}",
                stream_path
            );
            return;
        };

        log::info!("publish stream_path: {}", stream_path);

        if self.clients.contains_key(&id) {
            log::warn!("the client session with id:{} exists", id);
            self.send_error(&sender, &format!("stream {} exists.", stream_path))
                .await;
            return;
        }

        match self
            .create_and_spawn_client(&id, server_address, stream_path, sender.clone())
            .await
        {
            Ok(()) => {
                if let Err(send_err) = sender.send(Ok(())).await {
                    log::error!("sender error: {}", send_err);
                }
            }
            Err(err) => {
                self.send_error(&sender, &err.to_string()).await;
            }
        }
    }

    async fn create_and_spawn_client(
        &mut self,
        id: &str,
        server_address: String,
        stream_path: String,
        error_sender: HubResultSender,
    ) -> Result<(), RelayError> {
        let client_session = RtspClientSession::new(
            server_address,
            stream_path,
            ProtocolType::TCP,
            self.channel_event_producer.clone(),
            ClientSessionType::Pull,
        )
        .await
        .map_err(|_| RelayError {
            value: super::errors::PushClientErrorValue::SendError,
        })?;

        self.clients
            .insert(id.to_string(), client_session.is_running.clone());
        let arc_client_session = Arc::new(Mutex::new(client_session));

        tokio::spawn(async move {
            if let Err(err) = arc_client_session.lock().await.run().await {
                log::error!("client_session as pull client run error: {}", err);
                let hub_err = Err(StreamHubError {
                    value: StreamHubErrorValue::RtspClientSessionError(err.to_string()),
                });
                if let Err(send_err) = error_sender.send(hub_err).await {
                    log::error!("sender error: {}", send_err);
                }
            }
        });

        Ok(())
    }

    async fn handle_unsubscribe(&mut self, id: String, result_sender: Option<HubResultSender>) {
        let Some(sender) = result_sender else {
            log::error!("missing result sender for unsubscribe event");
            return;
        };

        if let Some(client) = self.clients.remove(&id) {
            client.store(false, std::sync::atomic::Ordering::Release);
            if let Err(send_err) = sender.send(Ok(())).await {
                log::error!("sender error: {}", send_err);
            }
        } else {
            log::warn!("the client session with id:{} not exists", id);
            self.send_error(&sender, "the client session not exists")
                .await;
        }
    }

    async fn send_error(&self, sender: &HubResultSender, message: &str) {
        let err = Err(StreamHubError {
            value: StreamHubErrorValue::RtspClientSessionError(message.to_string()),
        });
        if let Err(send_err) = sender.send(err).await {
            log::error!("sender error: {}", send_err);
        }
    }
}
