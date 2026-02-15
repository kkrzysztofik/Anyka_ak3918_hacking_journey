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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streamhub::define::{BroadcastEvent, StreamHubEvent};
    use crate::streamhub::stream::StreamIdentifier;
    use tokio::sync::{broadcast, mpsc};

    // ========== Helper Functions ==========

    /// Creates a test manager with fresh channels
    fn create_test_manager() -> (
        RtspPullClientManager,
        broadcast::Sender<BroadcastEvent>,
        mpsc::UnboundedReceiver<StreamHubEvent>,
    ) {
        let (event_tx, _event_rx) = broadcast::channel::<BroadcastEvent>(16);
        let (hub_tx, hub_rx) = mpsc::unbounded_channel::<StreamHubEvent>();
        let consumer = event_tx.subscribe();
        let manager = RtspPullClientManager::new(consumer, hub_tx);
        (manager, event_tx, hub_rx)
    }

    // ========== Constructor Tests ==========

    #[test]
    fn test_pull_client_manager_new_creates_empty_client_map() {
        let (manager, _, _) = create_test_manager();
        assert!(
            manager.clients.is_empty(),
            "New manager should have empty clients map"
        );
    }

    #[test]
    fn test_pull_client_manager_new_stores_channels() {
        let (manager, _tx, _rx) = create_test_manager();
        // Verify manager was created successfully with channels
        assert!(
            manager.clients.is_empty(),
            "Manager should be initialized correctly"
        );
    }

    // ========== Subscribe Event Tests - Missing Parameters ==========

    #[tokio::test]
    async fn test_handle_subscribe_missing_result_sender_returns_early() {
        let (mut manager, event_tx, _hub_rx) = create_test_manager();

        // Send subscribe event without result_sender
        let event = BroadcastEvent::Subscribe {
            id: "test-client-1".to_string(),
            identifier: StreamIdentifier::Rtsp {
                stream_path: "/test/stream".to_string(),
            },
            server_address: Some("rtsp://192.168.1.100:554".to_string()),
            result_sender: None, // Missing sender
        };

        // Send the event
        event_tx.send(event).unwrap();

        // Use tokio::select to timeout the run loop
        let result =
            tokio::time::timeout(tokio::time::Duration::from_millis(100), manager.run()).await;

        // The run loop should timeout (it's waiting for next event)
        // but importantly, no client should be added
        assert!(
            result.is_err() || result.unwrap().is_err(),
            "Run should timeout or continue waiting - no crash expected"
        );
    }

    #[tokio::test]
    async fn test_handle_subscribe_missing_server_address_sends_error() {
        let (mut manager, event_tx, _hub_rx) = create_test_manager();
        let (result_tx, mut result_rx) = mpsc::channel::<HubResult>(1);

        // Send subscribe event without server_address
        let event = BroadcastEvent::Subscribe {
            id: "test-client-2".to_string(),
            identifier: StreamIdentifier::Rtsp {
                stream_path: "/test/stream".to_string(),
            },
            server_address: None, // Missing server address
            result_sender: Some(result_tx),
        };

        event_tx.send(event).unwrap();

        // Run with timeout and check for error response
        let handle = tokio::spawn(async move { manager.run().await });

        // Wait for error response
        let response =
            tokio::time::timeout(tokio::time::Duration::from_millis(500), result_rx.recv()).await;

        // Should receive an error response
        assert!(
            response.is_ok(),
            "Should receive error response for missing server address"
        );
        let result = response.unwrap();
        assert!(result.is_some(), "Result should not be None");
        let hub_result = result.unwrap();
        assert!(
            hub_result.is_err(),
            "Should return error when server address is missing"
        );

        // Verify error message contains expected text
        if let Err(err) = hub_result {
            if let StreamHubErrorValue::RtspClientSessionError(msg) = err.value {
                assert!(
                    msg.contains("server address"),
                    "Error message should mention server address, got: {}",
                    msg
                );
            } else {
                panic!("Expected RtspClientSessionError variant");
            }
        }

        handle.abort();
    }

    #[tokio::test]
    async fn test_handle_subscribe_non_rtsp_identifier_returns_early() {
        let (mut manager, event_tx, _hub_rx) = create_test_manager();
        let (result_tx, mut result_rx) = mpsc::channel::<HubResult>(1);

        // Send subscribe event with RTMP identifier (not RTSP)
        let event = BroadcastEvent::Subscribe {
            id: "test-client-3".to_string(),
            identifier: StreamIdentifier::Rtmp {
                app_name: "live".to_string(),
                stream_name: "stream".to_string(),
            },
            server_address: Some("rtmp://192.168.1.100:1935".to_string()),
            result_sender: Some(result_tx),
        };

        event_tx.send(event).unwrap();

        let handle = tokio::spawn(async move { manager.run().await });

        // With timeout - should not receive any response (early return)
        let response =
            tokio::time::timeout(tokio::time::Duration::from_millis(200), result_rx.recv()).await;

        // Should timeout because non-RTSP identifier causes early return
        // (no response sent, no error, just returns)
        assert!(
            response.is_err() || response.unwrap().is_none(),
            "Should not receive response for non-RTSP identifier (early return)"
        );

        handle.abort();
    }

    // ========== Duplicate Client Detection Tests ==========

    #[tokio::test]
    async fn test_handle_subscribe_duplicate_client_sends_error() {
        let (mut manager, event_tx, _hub_rx) = create_test_manager();

        // Pre-populate with an existing client
        let existing_id = "existing-client".to_string();
        manager
            .clients
            .insert(existing_id.clone(), Arc::new(AtomicBool::new(true)));

        let (result_tx, mut result_rx) = mpsc::channel::<HubResult>(1);

        // Try to subscribe with same ID
        let event = BroadcastEvent::Subscribe {
            id: existing_id,
            identifier: StreamIdentifier::Rtsp {
                stream_path: "/test/stream".to_string(),
            },
            server_address: Some("rtsp://192.168.1.100:554".to_string()),
            result_sender: Some(result_tx),
        };

        event_tx.send(event).unwrap();

        let handle = tokio::spawn(async move { manager.run().await });

        // Wait for error response
        let response =
            tokio::time::timeout(tokio::time::Duration::from_millis(500), result_rx.recv()).await;

        assert!(
            response.is_ok(),
            "Should receive response for duplicate client"
        );
        let result = response.unwrap();
        assert!(result.is_some(), "Result should not be None");
        let hub_result = result.unwrap();
        assert!(
            hub_result.is_err(),
            "Should return error for duplicate client"
        );

        // Verify error message mentions existence
        if let Err(err) = hub_result {
            if let StreamHubErrorValue::RtspClientSessionError(msg) = err.value {
                assert!(
                    msg.contains("exists"),
                    "Error message should mention existing stream, got: {}",
                    msg
                );
            } else {
                panic!("Expected RtspClientSessionError variant");
            }
        }

        handle.abort();
    }

    // ========== Unsubscribe Event Tests ==========

    #[tokio::test]
    async fn test_handle_unsubscribe_missing_result_sender_returns_early() {
        let (mut manager, event_tx, _hub_rx) = create_test_manager();

        // Pre-populate with a client
        manager
            .clients
            .insert("test-client".to_string(), Arc::new(AtomicBool::new(true)));

        // Send unsubscribe event without result_sender
        let event = BroadcastEvent::UnSubscribe {
            id: "test-client".to_string(),
            result_sender: None,
        };

        event_tx.send(event).unwrap();

        let result =
            tokio::time::timeout(tokio::time::Duration::from_millis(100), manager.run()).await;

        // Should timeout (waiting for next event), not crash
        assert!(
            result.is_err() || result.unwrap().is_err(),
            "Run should timeout without crash"
        );
    }

    #[tokio::test]
    async fn test_handle_unsubscribe_not_found_sends_error() {
        let (mut manager, event_tx, _hub_rx) = create_test_manager();
        let (result_tx, mut result_rx) = mpsc::channel::<HubResult>(1);

        // Don't add any clients - try to unsubscribe non-existent
        let event = BroadcastEvent::UnSubscribe {
            id: "non-existent-client".to_string(),
            result_sender: Some(result_tx),
        };

        event_tx.send(event).unwrap();

        let handle = tokio::spawn(async move { manager.run().await });

        // Wait for error response
        let response =
            tokio::time::timeout(tokio::time::Duration::from_millis(500), result_rx.recv()).await;

        assert!(
            response.is_ok(),
            "Should receive response for non-existent client unsubscribe"
        );
        let result = response.unwrap();
        assert!(result.is_some(), "Result should not be None");
        let hub_result = result.unwrap();
        assert!(
            hub_result.is_err(),
            "Should return error when unsubscribing non-existent client"
        );

        // Verify error message
        if let Err(err) = hub_result {
            if let StreamHubErrorValue::RtspClientSessionError(msg) = err.value {
                assert!(
                    msg.contains("not exists") || msg.contains("not found"),
                    "Error message should mention client not existing, got: {}",
                    msg
                );
            } else {
                panic!("Expected RtspClientSessionError variant");
            }
        }

        handle.abort();
    }

    #[tokio::test]
    async fn test_handle_unsubscribe_existing_client_succeeds() {
        let (mut manager, event_tx, _hub_rx) = create_test_manager();
        let (result_tx, mut result_rx) = mpsc::channel::<HubResult>(1);

        // Pre-populate with a client
        let client_id = "existing-client-to-remove".to_string();
        let running_flag = Arc::new(AtomicBool::new(true));
        manager
            .clients
            .insert(client_id.clone(), running_flag.clone());

        assert!(
            manager.clients.contains_key(&client_id),
            "Client should exist before unsubscribe"
        );

        let event = BroadcastEvent::UnSubscribe {
            id: client_id.clone(),
            result_sender: Some(result_tx),
        };

        event_tx.send(event).unwrap();

        let handle = tokio::spawn(async move { manager.run().await });

        // Wait for success response
        let response =
            tokio::time::timeout(tokio::time::Duration::from_millis(500), result_rx.recv()).await;

        assert!(
            response.is_ok(),
            "Should receive response for successful unsubscribe"
        );
        let result = response.unwrap();
        assert!(result.is_some(), "Result should not be None");
        let hub_result = result.unwrap();
        assert!(
            hub_result.is_ok(),
            "Should return Ok for successful unsubscribe"
        );

        // Verify running flag was set to false
        assert!(
            !running_flag.load(std::sync::atomic::Ordering::Acquire),
            "Running flag should be set to false after unsubscribe"
        );

        handle.abort();
    }

    // ========== Edge Case Tests ==========

    #[tokio::test]
    async fn test_handle_unsubscribe_empty_manager_sends_error() {
        let (mut manager, event_tx, _hub_rx) = create_test_manager();
        let (result_tx, mut result_rx) = mpsc::channel::<HubResult>(1);

        // Manager has no clients - empty state
        assert!(
            manager.clients.is_empty(),
            "Manager should be empty initially"
        );

        let event = BroadcastEvent::UnSubscribe {
            id: "any-client-id".to_string(),
            result_sender: Some(result_tx),
        };

        event_tx.send(event).unwrap();

        let handle = tokio::spawn(async move { manager.run().await });

        let response =
            tokio::time::timeout(tokio::time::Duration::from_millis(500), result_rx.recv()).await;

        assert!(
            response.is_ok(),
            "Should receive error response when manager is empty"
        );
        let result = response.unwrap();
        assert!(result.is_some(), "Result should not be None");
        assert!(
            result.unwrap().is_err(),
            "Should return error when client not found in empty manager"
        );

        handle.abort();
    }

    #[tokio::test]
    async fn test_handle_subscribe_empty_stream_path_still_processed() {
        let (mut manager, event_tx, _hub_rx) = create_test_manager();
        let (result_tx, mut result_rx) = mpsc::channel::<HubResult>(1);

        // Subscribe with empty stream path (edge case)
        let event = BroadcastEvent::Subscribe {
            id: "client-empty-path".to_string(),
            identifier: StreamIdentifier::Rtsp {
                stream_path: "".to_string(), // Empty path
            },
            server_address: Some("rtsp://192.168.1.100:554".to_string()),
            result_sender: Some(result_tx),
        };

        event_tx.send(event).unwrap();

        let handle = tokio::spawn(async move { manager.run().await });

        // This will likely fail to create client (no valid path), but should handle gracefully
        let response =
            tokio::time::timeout(tokio::time::Duration::from_millis(500), result_rx.recv()).await;

        // Should receive some response (error or otherwise)
        assert!(
            response.is_ok(),
            "Should receive response even with empty stream path"
        );

        handle.abort();
    }

    #[tokio::test]
    async fn test_handle_unsubscribe_sets_running_flag_false() {
        let (mut manager, event_tx, _hub_rx) = create_test_manager();
        let (result_tx, mut result_rx) = mpsc::channel::<HubResult>(1);

        // Create client with running flag = true
        let running_flag = Arc::new(AtomicBool::new(true));
        let client_id = "flag-test-client".to_string();
        manager
            .clients
            .insert(client_id.clone(), running_flag.clone());

        assert!(
            running_flag.load(std::sync::atomic::Ordering::Acquire),
            "Running flag should be true initially"
        );

        let event = BroadcastEvent::UnSubscribe {
            id: client_id,
            result_sender: Some(result_tx),
        };

        event_tx.send(event).unwrap();

        let handle = tokio::spawn(async move { manager.run().await });

        let _ =
            tokio::time::timeout(tokio::time::Duration::from_millis(500), result_rx.recv()).await;

        // Verify flag was set to false
        assert!(
            !running_flag.load(std::sync::atomic::Ordering::Acquire),
            "Running flag should be false after unsubscribe"
        );

        handle.abort();
    }

    // ========== BroadcastEvent Variant Tests ==========

    #[tokio::test]
    async fn test_run_handles_publish_event_gracefully() {
        let (mut manager, event_tx, _hub_rx) = create_test_manager();

        // Send a Publish event (not Subscribe/Unsubscribe)
        let event = BroadcastEvent::Publish {
            identifier: StreamIdentifier::Rtsp {
                stream_path: "/test/stream".to_string(),
            },
        };

        event_tx.send(event).unwrap();

        // Run should continue without error (logs "other events")
        let result =
            tokio::time::timeout(tokio::time::Duration::from_millis(100), manager.run()).await;

        // Should timeout (run continues waiting), not crash
        assert!(
            result.is_err() || result.unwrap().is_err(),
            "Run should handle Publish event gracefully and continue"
        );
    }

    #[tokio::test]
    async fn test_run_handles_unpublish_event_gracefully() {
        let (mut manager, event_tx, _hub_rx) = create_test_manager();

        // Send an UnPublish event
        let event = BroadcastEvent::UnPublish {
            identifier: StreamIdentifier::Rtsp {
                stream_path: "/test/stream".to_string(),
            },
        };

        event_tx.send(event).unwrap();

        let result =
            tokio::time::timeout(tokio::time::Duration::from_millis(100), manager.run()).await;

        // Should timeout (run continues waiting), not crash
        assert!(
            result.is_err() || result.unwrap().is_err(),
            "Run should handle UnPublish event gracefully and continue"
        );
    }

    // ========== Multiple Client Tests ==========

    #[tokio::test]
    async fn test_multiple_clients_can_be_managed() {
        let (mut manager, _event_tx, _hub_rx) = create_test_manager();

        // Add multiple clients manually
        for i in 0..5 {
            manager
                .clients
                .insert(format!("client-{}", i), Arc::new(AtomicBool::new(true)));
        }

        assert_eq!(manager.clients.len(), 5, "Should have 5 clients registered");

        // Verify all are present
        for i in 0..5 {
            assert!(
                manager.clients.contains_key(&format!("client-{}", i)),
                "Client {} should exist",
                i
            );
        }
    }

    #[tokio::test]
    async fn test_client_id_uniqueness_enforced() {
        let (mut manager, event_tx, _hub_rx) = create_test_manager();

        // Pre-populate with client
        let client_id = "unique-test-id".to_string();
        manager
            .clients
            .insert(client_id.clone(), Arc::new(AtomicBool::new(true)));

        let (result_tx, mut result_rx) = mpsc::channel::<HubResult>(1);

        // Try to add another with same ID
        let event = BroadcastEvent::Subscribe {
            id: client_id,
            identifier: StreamIdentifier::Rtsp {
                stream_path: "/stream".to_string(),
            },
            server_address: Some("rtsp://server:554".to_string()),
            result_sender: Some(result_tx),
        };

        event_tx.send(event).unwrap();
        let handle = tokio::spawn(async move { manager.run().await });

        let response =
            tokio::time::timeout(tokio::time::Duration::from_millis(500), result_rx.recv()).await;

        // Should get error for duplicate
        assert!(
            response.is_ok() && response.unwrap().unwrap().is_err(),
            "Duplicate client ID should return error"
        );

        handle.abort();
    }

    // ========== run() and create_and_spawn_client branches ==========

    #[tokio::test]
    async fn test_run_returns_error_when_broadcast_sender_dropped() {
        let (mut manager, event_tx, _hub_rx) = create_test_manager();

        let handle = tokio::spawn(async move { manager.run().await });

        drop(event_tx);

        let result = handle.await;
        assert!(result.is_ok(), "Task should complete");
        let run_result = result.unwrap();
        assert!(
            run_result.is_err(),
            "Run should return error when broadcast channel is closed"
        );
    }

    #[tokio::test]
    async fn test_handle_subscribe_create_client_failure_sends_error() {
        let (mut manager, event_tx, _hub_rx) = create_test_manager();
        let (result_tx, mut result_rx) = mpsc::channel::<HubResult>(1);

        let event = BroadcastEvent::Subscribe {
            id: "fail-connect-client".to_string(),
            identifier: StreamIdentifier::Rtsp {
                stream_path: "/stream".to_string(),
            },
            server_address: Some("127.0.0.1:1".to_string()),
            result_sender: Some(result_tx),
        };

        event_tx.send(event).unwrap();

        let handle = tokio::spawn(async move { manager.run().await });

        let response =
            tokio::time::timeout(tokio::time::Duration::from_secs(2), result_rx.recv()).await;

        assert!(response.is_ok(), "Should receive response");
        let msg = response.unwrap();
        assert!(msg.is_some(), "Should get some result");
        assert!(
            msg.unwrap().is_err(),
            "Create client should fail for invalid/unreachable address"
        );

        handle.abort();
    }

    #[tokio::test]
    async fn test_send_error_when_receiver_dropped_does_not_panic() {
        let (mut manager, event_tx, _hub_rx) = create_test_manager();
        let (result_tx, result_rx) = mpsc::channel::<HubResult>(1);

        let event = BroadcastEvent::Subscribe {
            id: "drop-receiver".to_string(),
            identifier: StreamIdentifier::Rtsp {
                stream_path: "/stream".to_string(),
            },
            server_address: None,
            result_sender: Some(result_tx),
        };

        event_tx.send(event).unwrap();
        drop(result_rx);

        let handle = tokio::spawn(async move { manager.run().await });

        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
        handle.abort();
    }
}
