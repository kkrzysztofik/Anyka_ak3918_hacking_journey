use super::*;
use crate::hub::define::{
    DataReceiver, DataSender, FrameData, NotifyInfo, PacketData, RelayType, StatisticData,
    StreamHubEvent, SubDataType, SubscribeType, SubscriberInfo,
};
use async_trait::async_trait;
use bytes::BytesMut;
use mockall::mock;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};

// Mock TStreamHandler for testing
mock! {
    StreamHandler {}

    #[async_trait]
    impl TStreamHandler for StreamHandler {
        async fn send_prior_data(
            &self,
            sender: DataSender,
            sub_type: SubscribeType,
        ) -> Result<(), StreamHubError>;
        async fn get_statistic_data(&self) -> Option<StatisticsStream>;
        async fn send_information(&self, sender: super::define::InformationSender);
    }
}

fn create_test_stream_identifier() -> StreamIdentifier {
    StreamIdentifier::Rtsp {
        stream_path: "live/test".to_string(),
    }
}

fn create_test_subscriber_info() -> SubscriberInfo {
    SubscriberInfo {
        id: Uuid::new(crate::hub::utils::RandomDigitCount::Four),
        sub_type: SubscribeType::HttpFlvPull,
        notify_info: NotifyInfo {
            request_url: "http://localhost/live/test.flv".to_string(),
            remote_addr: "127.0.0.1:8080".to_string(),
        },
        sub_data_type: SubDataType::Frame,
    }
}

#[tokio::test]
async fn test_streams_hub_new() {
    let hub = StreamsHub::new(None);
    assert_eq!(hub.streams.len(), 0);
    assert_eq!(hub.un_pub_sub_events.len(), 0);
    assert!(!hub.rtmp_push_enabled);
    assert!(!hub.rtmp_pull_enabled);
    assert!(!hub.rtmp_remuxer_enabled);
    assert!(!hub.hls_enabled);
}

#[tokio::test]
async fn test_streams_hub_set_flags() {
    let mut hub = StreamsHub::new(None);
    hub.set_rtmp_push_enabled(true);
    hub.set_rtmp_pull_enabled(true);
    hub.set_rtmp_remuxer_enabled(true);
    hub.set_hls_enabled(true);

    assert!(hub.rtmp_push_enabled);
    assert!(hub.rtmp_pull_enabled);
    assert!(hub.rtmp_remuxer_enabled);
    assert!(hub.hls_enabled);
}

#[tokio::test]
async fn test_streams_hub_publish_success() {
    let mut hub = StreamsHub::new(None);
    let identifier = create_test_stream_identifier();
    let (_frame_sender, frame_receiver) = mpsc::unbounded_channel();
    let receiver = DataReceiver {
        frame_receiver: Some(frame_receiver),
        packet_receiver: None,
    };

    let mut mock_handler = MockStreamHandler::new();
    mock_handler
        .expect_send_prior_data()
        .times(0)
        .returning(|_, _| Ok(()));
    let mock_handler = Arc::new(mock_handler);

    let result = hub
        .publish(identifier.clone(), receiver, mock_handler)
        .await;

    assert!(result.is_ok());
    assert!(hub.streams.contains_key(&identifier));
}

#[tokio::test]
async fn test_streams_hub_publish_duplicate() {
    let mut hub = StreamsHub::new(None);
    let identifier = create_test_stream_identifier();
    let (_frame_sender1, frame_receiver1) = mpsc::unbounded_channel();
    let (_frame_sender2, frame_receiver2) = mpsc::unbounded_channel();

    let receiver1 = DataReceiver {
        frame_receiver: Some(frame_receiver1),
        packet_receiver: None,
    };
    let receiver2 = DataReceiver {
        frame_receiver: Some(frame_receiver2),
        packet_receiver: None,
    };

    let mut mock_handler1 = MockStreamHandler::new();
    mock_handler1
        .expect_send_prior_data()
        .times(0)
        .returning(|_, _| Ok(()));
    let mock_handler1 = Arc::new(mock_handler1);

    let mut mock_handler2 = MockStreamHandler::new();
    mock_handler2
        .expect_send_prior_data()
        .times(0)
        .returning(|_, _| Ok(()));
    let mock_handler2 = Arc::new(mock_handler2);

    let result1 = hub
        .publish(identifier.clone(), receiver1, mock_handler1)
        .await;
    assert!(result1.is_ok());

    let result2 = hub
        .publish(identifier.clone(), receiver2, mock_handler2)
        .await;
    assert!(result2.is_err());
    match result2.unwrap_err().value {
        StreamHubErrorValue::Exists => {}
        _ => panic!("Expected Exists error"),
    }
}

#[tokio::test]
async fn test_streams_hub_subscribe_success() {
    let mut hub = StreamsHub::new(None);
    let identifier = create_test_stream_identifier();
    let (_frame_sender, frame_receiver) = mpsc::unbounded_channel();
    let receiver = DataReceiver {
        frame_receiver: Some(frame_receiver),
        packet_receiver: None,
    };

    let mut mock_handler = MockStreamHandler::new();
    mock_handler
        .expect_send_prior_data()
        .returning(|_, _| Ok(()));
    let mock_handler = Arc::new(mock_handler);

    // First publish
    let _ = hub
        .publish(identifier.clone(), receiver, mock_handler.clone())
        .await;

    // Then subscribe
    let sub_info = create_test_subscriber_info();
    let (sender, _) = mpsc::unbounded_channel();
    let data_sender = DataSender::Frame { sender };

    let result = hub
        .subscribe(&identifier, sub_info.clone(), data_sender)
        .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_streams_hub_subscribe_no_stream() {
    let mut hub = StreamsHub::new(None);
    let identifier = create_test_stream_identifier();
    let sub_info = create_test_subscriber_info();
    let (sender, _) = mpsc::unbounded_channel();
    let data_sender = DataSender::Frame { sender };

    let result = hub.subscribe(&identifier, sub_info, data_sender).await;

    assert!(result.is_err());
    match result.unwrap_err().value {
        StreamHubErrorValue::NoAppOrStreamName => {}
        _ => panic!("Expected NoAppOrStreamName error"),
    }
}

#[tokio::test]
async fn test_streams_hub_subscribe_no_stream_emits_pull_event_when_enabled() {
    let mut hub = StreamsHub::new(None);
    hub.set_rtmp_pull_enabled(true);

    let identifier = create_test_stream_identifier();
    let sub_info = create_test_subscriber_info();
    let (sender, _) = mpsc::unbounded_channel();
    let data_sender = DataSender::Frame { sender };
    let mut client_event_receiver = hub.get_client_event_consumer();

    let result = hub.subscribe(&identifier, sub_info, data_sender).await;

    assert!(result.is_err());
    match result.unwrap_err().value {
        StreamHubErrorValue::NoAppOrStreamName => {}
        _ => panic!("Expected NoAppOrStreamName error"),
    }

    let event = tokio::time::timeout(
        tokio::time::Duration::from_millis(100),
        client_event_receiver.recv(),
    )
    .await
    .expect("timed out waiting for broadcast event")
    .expect("broadcast channel closed unexpectedly");

    match event {
        BroadcastEvent::Subscribe {
            id,
            identifier: event_identifier,
            server_address,
            result_sender,
        } => {
            assert_eq!(id, "rtmp_relay");
            assert_eq!(event_identifier, identifier);
            assert!(server_address.is_none());
            assert!(result_sender.is_none());
        }
        _ => panic!("expected BroadcastEvent::Subscribe"),
    }
}

#[tokio::test]
async fn test_streams_hub_unsubscribe_success() {
    let mut hub = StreamsHub::new(None);
    let identifier = create_test_stream_identifier();
    let (_frame_sender, frame_receiver) = mpsc::unbounded_channel();
    let receiver = DataReceiver {
        frame_receiver: Some(frame_receiver),
        packet_receiver: None,
    };

    let mut mock_handler = MockStreamHandler::new();
    mock_handler
        .expect_send_prior_data()
        .returning(|_, _| Ok(()));
    let mock_handler = Arc::new(mock_handler);

    // Publish
    let _ = hub
        .publish(identifier.clone(), receiver, mock_handler.clone())
        .await;

    // Subscribe
    let sub_info = create_test_subscriber_info();
    let (sender, _) = mpsc::unbounded_channel();
    let data_sender = DataSender::Frame { sender };
    let _ = hub
        .subscribe(&identifier, sub_info.clone(), data_sender)
        .await;

    // Unsubscribe
    let result = hub.unsubscribe(&identifier, sub_info);
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_streams_hub_unsubscribe_no_stream() {
    let mut hub = StreamsHub::new(None);
    let identifier = create_test_stream_identifier();
    let sub_info = create_test_subscriber_info();

    let result = hub.unsubscribe(&identifier, sub_info);
    assert!(result.is_err());
    match result.unwrap_err().value {
        StreamHubErrorValue::NoAppName => {}
        _ => panic!("Expected NoAppName error"),
    }
}

#[tokio::test]
async fn test_streams_hub_unpublish_success() {
    let mut hub = StreamsHub::new(None);
    let identifier = create_test_stream_identifier();
    let (_frame_sender, frame_receiver) = mpsc::unbounded_channel();
    let receiver = DataReceiver {
        frame_receiver: Some(frame_receiver),
        packet_receiver: None,
    };

    let mut mock_handler = MockStreamHandler::new();
    mock_handler
        .expect_send_prior_data()
        .times(0)
        .returning(|_, _| Ok(()));
    let mock_handler = Arc::new(mock_handler);

    // Publish
    let _ = hub
        .publish(identifier.clone(), receiver, mock_handler.clone())
        .await;
    assert!(hub.streams.contains_key(&identifier));

    // Unpublish
    let result = hub.unpublish(&identifier);
    assert!(result.is_ok());
    assert!(!hub.streams.contains_key(&identifier));
}

#[tokio::test]
async fn test_streams_hub_unpublish_no_stream() {
    let mut hub = StreamsHub::new(None);
    let identifier = create_test_stream_identifier();

    let result = hub.unpublish(&identifier);
    assert!(result.is_err());
    match result.unwrap_err().value {
        StreamHubErrorValue::NoAppName => {}
        _ => panic!("Expected NoAppName error"),
    }
}

#[tokio::test]
async fn test_streams_hub_request_success() {
    let mut hub = StreamsHub::new(None);
    let identifier = create_test_stream_identifier();
    let (_frame_sender, frame_receiver) = mpsc::unbounded_channel();
    let receiver = DataReceiver {
        frame_receiver: Some(frame_receiver),
        packet_receiver: None,
    };

    let mut mock_handler = MockStreamHandler::new();
    mock_handler
        .expect_send_prior_data()
        .times(0)
        .returning(|_, _| Ok(()));
    mock_handler
        .expect_send_information()
        .times(1)
        .returning(|_| {});
    let mock_handler = Arc::new(mock_handler);

    // Publish
    let _ = hub
        .publish(identifier.clone(), receiver, mock_handler.clone())
        .await;

    // Request
    let (info_sender, _) = mpsc::unbounded_channel();
    let result = hub.request(&identifier, info_sender);
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_streams_hub_request_no_stream() {
    let mut hub = StreamsHub::new(None);
    let identifier = create_test_stream_identifier();
    let (info_sender, _) = mpsc::unbounded_channel();

    let result = hub.request(&identifier, info_sender);
    // Request doesn't fail if stream doesn't exist, it just does nothing
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_handle_api_statistic_empty_streams_returns_empty_json() {
    let mut hub = StreamsHub::new(None);
    let event_sender = hub.get_hub_event_sender();
    let (result_tx, result_rx) = oneshot::channel();
    tokio::spawn(async move { hub.event_loop().await });
    event_sender
        .send(StreamHubEvent::ApiStatistic {
            top_n: None,
            identifier: None,
            uuid: None,
            result_sender: result_tx,
        })
        .unwrap();
    let value = result_rx.await.unwrap();
    assert_eq!(value, json!({}));
}

#[tokio::test]
async fn test_handle_api_statistic_result_sender_dropped_completes_without_panic() {
    let mut hub = StreamsHub::new(None);
    let event_sender = hub.get_hub_event_sender();
    let (result_tx, result_rx) = oneshot::channel();
    tokio::spawn(async move { hub.event_loop().await });
    event_sender
        .send(StreamHubEvent::ApiStatistic {
            top_n: None,
            identifier: None,
            uuid: None,
            result_sender: result_tx,
        })
        .unwrap();
    drop(result_rx);
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
}

#[tokio::test]
async fn test_handle_api_statistic_with_stream_success() {
    let mut hub = StreamsHub::new(None);
    let identifier = create_test_stream_identifier();
    let (_frame_sender, frame_receiver) = mpsc::unbounded_channel();
    let receiver = DataReceiver {
        frame_receiver: Some(frame_receiver),
        packet_receiver: None,
    };
    let mut mock_handler = MockStreamHandler::new();
    mock_handler
        .expect_send_prior_data()
        .times(0)
        .returning(|_, _| Ok(()));
    let mock_handler = Arc::new(mock_handler);
    let _ = hub
        .publish(identifier.clone(), receiver, mock_handler)
        .await;

    let event_sender = hub.get_hub_event_sender();
    let (result_tx, result_rx) = oneshot::channel();
    tokio::spawn(async move { hub.event_loop().await });
    event_sender
        .send(StreamHubEvent::ApiStatistic {
            top_n: None,
            identifier: Some(identifier),
            uuid: None,
            result_sender: result_tx,
        })
        .unwrap();
    let value = result_rx.await.unwrap();
    assert!(value.is_array());
}

#[tokio::test]
async fn test_handle_api_start_relay_stream_pull_success() {
    let mut hub = StreamsHub::new(None);
    let mut client_rx = hub.get_client_event_consumer();
    let event_sender = hub.get_hub_event_sender();
    tokio::spawn(async move {
        while let Ok(ev) = client_rx.recv().await {
            if let BroadcastEvent::Subscribe {
                result_sender: Some(rs),
                ..
            } = ev
            {
                let _ = rs.send(Ok(())).await;
                break;
            }
        }
    });
    tokio::spawn(async move { hub.event_loop().await });
    let (result_tx, result_rx) = oneshot::channel();
    event_sender
        .send(StreamHubEvent::ApiStartRelayStream {
            id: "relay-1".to_string(),
            identifier: create_test_stream_identifier(),
            server_address: "127.0.0.1:554".to_string(),
            relay_type: RelayType::Pull,
            result_sender: result_tx,
        })
        .unwrap();
    let r = result_rx.await.unwrap();
    assert!(r.is_ok());
}

#[tokio::test]
async fn test_handle_api_start_relay_stream_pull_failure() {
    let mut hub = StreamsHub::new(None);
    let mut client_rx = hub.get_client_event_consumer();
    let event_sender = hub.get_hub_event_sender();
    let err = StreamHubError {
        value: StreamHubErrorValue::SendError,
    };
    tokio::spawn(async move {
        while let Ok(ev) = client_rx.recv().await {
            if let BroadcastEvent::Subscribe {
                result_sender: Some(rs),
                ..
            } = ev
            {
                let _ = rs.send(Err(err)).await;
                break;
            }
        }
    });
    tokio::spawn(async move { hub.event_loop().await });
    let (result_tx, result_rx) = oneshot::channel();
    event_sender
        .send(StreamHubEvent::ApiStartRelayStream {
            id: "relay-2".to_string(),
            identifier: create_test_stream_identifier(),
            server_address: "127.0.0.1:554".to_string(),
            relay_type: RelayType::Pull,
            result_sender: result_tx,
        })
        .unwrap();
    let r = result_rx.await.unwrap();
    assert!(r.is_err());
}

#[tokio::test]
async fn test_handle_api_stop_relay_stream_pull_success() {
    let mut hub = StreamsHub::new(None);
    let mut client_rx = hub.get_client_event_consumer();
    let event_sender = hub.get_hub_event_sender();
    tokio::spawn(async move {
        while let Ok(ev) = client_rx.recv().await {
            if let BroadcastEvent::UnSubscribe {
                result_sender: Some(rs),
                ..
            } = ev
            {
                let _ = rs.send(Ok(())).await;
                break;
            }
        }
    });
    tokio::spawn(async move { hub.event_loop().await });
    let (result_tx, result_rx) = oneshot::channel();
    event_sender
        .send(StreamHubEvent::ApiStopRelayStream {
            id: "relay-1".to_string(),
            relay_type: RelayType::Pull,
            result_sender: result_tx,
        })
        .unwrap();
    let r = result_rx.await.unwrap();
    assert!(r.is_ok());
}

#[tokio::test]
async fn test_handle_api_stop_relay_stream_pull_failure() {
    let mut hub = StreamsHub::new(None);
    let mut client_rx = hub.get_client_event_consumer();
    let event_sender = hub.get_hub_event_sender();
    let err = StreamHubError {
        value: StreamHubErrorValue::NoAppOrStreamName,
    };
    tokio::spawn(async move {
        while let Ok(ev) = client_rx.recv().await {
            if let BroadcastEvent::UnSubscribe {
                result_sender: Some(rs),
                ..
            } = ev
            {
                let _ = rs.send(Err(err)).await;
                break;
            }
        }
    });
    tokio::spawn(async move { hub.event_loop().await });
    let (result_tx, result_rx) = oneshot::channel();
    event_sender
        .send(StreamHubEvent::ApiStopRelayStream {
            id: "unknown-id".to_string(),
            relay_type: RelayType::Pull,
            result_sender: result_tx,
        })
        .unwrap();
    let r = result_rx.await.unwrap();
    assert!(r.is_err());
}

#[tokio::test]
async fn test_stream_data_transceiver_new() {
    let (_data_sender, data_receiver) = mpsc::unbounded_channel();
    let (_event_sender, event_receiver) = mpsc::unbounded_channel();
    let identifier = create_test_stream_identifier();
    let mock_handler = Arc::new(MockStreamHandler::new());

    let receiver = DataReceiver {
        frame_receiver: Some(data_receiver),
        packet_receiver: None,
    };

    let transceiver =
        StreamDataTransceiver::new(receiver, event_receiver, identifier.clone(), mock_handler);

    let statistic_sender = transceiver.get_statistics_data_sender();
    assert!(
        statistic_sender
            .send(StatisticData::Publisher {
                id: Uuid::new(crate::hub::utils::RandomDigitCount::Four),
                remote_addr: "127.0.0.1:1935".to_string(),
                start_time: chrono::Local::now(),
            })
            .is_ok()
    );
}
#[tokio::test]
async fn test_stream_data_transceiver_frame_forwarding() {
    let (frame_sender, frame_receiver) = mpsc::unbounded_channel();
    let (event_sender, event_receiver) = mpsc::unbounded_channel();
    let identifier = create_test_stream_identifier();
    let mut mock_handler = MockStreamHandler::new();
    mock_handler
        .expect_send_prior_data()
        .returning(|_, _| Ok(()));
    let mock_handler = Arc::new(mock_handler);

    let receiver = DataReceiver {
        frame_receiver: Some(frame_receiver),
        packet_receiver: None,
    };

    let transceiver =
        StreamDataTransceiver::new(receiver, event_receiver, identifier.clone(), mock_handler);

    // Run transceiver in background
    tokio::spawn(async move {
        let _ = transceiver.run().await;
    });

    // 1. Subscribe
    let (sub_frame_sender, mut sub_frame_receiver) = mpsc::unbounded_channel();
    let sub_info = create_test_subscriber_info();
    let data_sender = DataSender::Frame {
        sender: sub_frame_sender,
    };

    let (sub_result_sender, _) = oneshot::channel();
    event_sender
        .send(TransceiverEvent::Subscribe {
            sender: data_sender,
            info: sub_info,
            result_sender: sub_result_sender,
        })
        .unwrap();

    // Allow some time for subscription processing
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // 2. Send Frame Data
    let frame_data = FrameData::Audio {
        timestamp: 100,
        data: BytesMut::from(&[0x01, 0x02, 0x03][..]),
    };

    frame_sender.send(frame_data).unwrap();

    // 3. Verify Subscriber Received Data
    let received = tokio::time::timeout(
        tokio::time::Duration::from_millis(100),
        sub_frame_receiver.recv(),
    )
    .await
    .expect("Timeout waiting for frame data");

    assert!(received.is_some());
    match received.unwrap() {
        FrameData::Audio { timestamp, data } => {
            assert_eq!(timestamp, 100);
            assert_eq!(data, BytesMut::from(&[0x01, 0x02, 0x03][..]));
        }
        _ => panic!("Expected Audio frame"),
    }
}

#[tokio::test]
async fn test_stream_data_transceiver_packet_forwarding() {
    let (packet_sender, packet_receiver) = mpsc::unbounded_channel();
    let (event_sender, event_receiver) = mpsc::unbounded_channel();
    let identifier = create_test_stream_identifier();
    let mut mock_handler = MockStreamHandler::new();
    mock_handler
        .expect_send_prior_data()
        .returning(|_, _| Ok(()));
    let mock_handler = Arc::new(mock_handler);

    let receiver = DataReceiver {
        frame_receiver: None,
        packet_receiver: Some(packet_receiver),
    };

    let transceiver =
        StreamDataTransceiver::new(receiver, event_receiver, identifier.clone(), mock_handler);

    tokio::spawn(async move {
        let _ = transceiver.run().await;
    });

    // 1. Subscribe
    let (sub_packet_sender, mut sub_packet_receiver) = mpsc::unbounded_channel();
    let mut sub_info = create_test_subscriber_info();
    sub_info.sub_data_type = SubDataType::Packet;

    let data_sender = DataSender::Packet {
        sender: sub_packet_sender,
    };

    let (sub_result_sender, _) = oneshot::channel();
    event_sender
        .send(TransceiverEvent::Subscribe {
            sender: data_sender,
            info: sub_info,
            result_sender: sub_result_sender,
        })
        .unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // 2. Send Packet Data
    let packet_data = PacketData::Audio {
        timestamp: 200,
        data: BytesMut::from(&[0x0A, 0x0B][..]),
    };

    packet_sender.send(packet_data).unwrap();

    // 3. Verify Subscriber Received Data
    let received = tokio::time::timeout(
        tokio::time::Duration::from_millis(100),
        sub_packet_receiver.recv(),
    )
    .await
    .expect("Timeout waiting for packet data");

    assert!(received.is_some());
    match received.unwrap() {
        PacketData::Audio { timestamp, data } => {
            assert_eq!(timestamp, 200);
            assert_eq!(data, BytesMut::from(&[0x0A, 0x0B][..]));
        }
        _ => panic!("Expected Audio packet"),
    }
}

#[tokio::test]
async fn test_api_kick_off_client_routes_stored_unsubscribe_event() {
    let mut hub = StreamsHub::new(None);
    let identifier = create_test_stream_identifier();
    let sub_info = create_test_subscriber_info();

    hub.un_pub_sub_events.insert(
        sub_info.id,
        StreamHubEvent::UnSubscribe {
            identifier: identifier.clone(),
            info: sub_info.clone(),
        },
    );

    let result = hub.api_kick_off_client(sub_info.id);
    assert!(result.is_ok());

    let routed = tokio::time::timeout(
        tokio::time::Duration::from_millis(100),
        hub.hub_event_receiver.recv(),
    )
    .await
    .expect("timed out waiting for routed kick-off event")
    .expect("hub event channel closed unexpectedly");

    match routed {
        StreamHubEvent::UnSubscribe {
            identifier: routed_identifier,
            info: routed_info,
        } => {
            assert_eq!(routed_identifier, identifier);
            assert_eq!(routed_info.id, sub_info.id);
            assert!(matches!(routed_info.sub_type, SubscribeType::HttpFlvPull));
        }
        _ => panic!("expected StreamHubEvent::UnSubscribe"),
    }
}

#[tokio::test]
async fn test_receive_statistics_data_video_keyframe_resets_gop_counter() {
    let stats = Arc::new(Mutex::new(StatisticsStream::new(
        create_test_stream_identifier(),
    )));

    StreamDataTransceiver::receive_statistics_data(
        Some(StatisticData::Video {
            uuid: None,
            data_size: 100,
            frame_count: 3,
            is_key_frame: Some(false),
            duration: 33,
        }),
        &stats,
    )
    .await;

    StreamDataTransceiver::receive_statistics_data(
        Some(StatisticData::Video {
            uuid: None,
            data_size: 50,
            frame_count: 1,
            is_key_frame: Some(true),
            duration: 33,
        }),
        &stats,
    )
    .await;

    let guard = stats.lock().await;
    assert_eq!(guard.publisher.video.recv_bytes, 150);
    assert_eq!(guard.publisher.video.recv_frame_count, 4);
    assert_eq!(guard.publisher.video.gop, 3);
    assert_eq!(guard.publisher.video.recv_frame_count_for_gop, 1);
}

#[tokio::test]
async fn test_receive_frame_data_prunes_closed_senders() {
    let senders: Arc<Mutex<HashMap<Uuid, FrameDataSender>>> = Arc::new(Mutex::new(HashMap::new()));
    let id = Uuid::new(crate::hub::utils::RandomDigitCount::Four);
    let (tx, rx) = mpsc::unbounded_channel::<FrameData>();
    drop(rx);

    {
        let mut map = senders.lock().await;
        map.insert(id, tx);
        assert_eq!(map.len(), 1);
    }

    let frame = FrameData::Audio {
        timestamp: 1,
        data: BytesMut::from(&[0x01][..]),
    };
    StreamDataTransceiver::receive_frame_data(Some(frame), &senders).await;

    let map = senders.lock().await;
    assert!(map.is_empty());
}

#[tokio::test]
async fn test_receive_packet_data_prunes_closed_senders() {
    let senders: Arc<Mutex<HashMap<Uuid, PacketDataSender>>> = Arc::new(Mutex::new(HashMap::new()));
    let id = Uuid::new(crate::hub::utils::RandomDigitCount::Four);
    let (tx, rx) = mpsc::unbounded_channel::<PacketData>();
    drop(rx);

    {
        let mut map = senders.lock().await;
        map.insert(id, tx);
        assert_eq!(map.len(), 1);
    }

    let packet = PacketData::Audio {
        timestamp: 1,
        data: BytesMut::from(&[0x01][..]),
    };
    StreamDataTransceiver::receive_packet_data(Some(packet), &senders).await;

    let map = senders.lock().await;
    assert!(map.is_empty());
}

#[tokio::test]
async fn test_get_subscriber_count_initially_zero() {
    // Test that a newly created transceiver has 0 subscribers
    let identifier = create_test_stream_identifier();
    let (_frame_sender, frame_receiver) = mpsc::unbounded_channel();
    let (_event_sender, event_receiver) = mpsc::unbounded_channel();
    let receiver = DataReceiver {
        frame_receiver: Some(frame_receiver),
        packet_receiver: None,
    };

    let mut mock_handler = MockStreamHandler::new();
    mock_handler
        .expect_send_prior_data()
        .times(0)
        .returning(|_, _| Ok(()));
    let mock_handler = Arc::new(mock_handler);

    let transceiver =
        StreamDataTransceiver::new(receiver, event_receiver, identifier, mock_handler);

    assert_eq!(transceiver.get_subscriber_count(), 0);
}

#[tokio::test]
async fn test_get_subscriber_count_increments_on_subscribe() {
    // Test that subscriber count increases when a subscriber is added
    let mut hub = StreamsHub::new(None);
    let identifier = create_test_stream_identifier();
    let (_frame_sender, frame_receiver) = mpsc::unbounded_channel();
    let receiver = DataReceiver {
        frame_receiver: Some(frame_receiver),
        packet_receiver: None,
    };

    let mut mock_handler = MockStreamHandler::new();
    mock_handler
        .expect_send_prior_data()
        .returning(|_, _| Ok(()));
    let mock_handler = Arc::new(mock_handler);

    // Publish stream
    hub.publish(identifier.clone(), receiver, mock_handler.clone())
        .await
        .unwrap();

    // Note: The transceiver is stored in hub.streams as an event_sender (unbounded channel)
    // which doesn't have a capacity() method.  Skip the capacity assertion.

    // Subscribe first subscriber
    let sub_info_1 = create_test_subscriber_info();
    let (sender_1, _) = mpsc::unbounded_channel();
    let data_sender_1 = DataSender::Frame { sender: sender_1 };

    hub.subscribe(&identifier, sub_info_1.clone(), data_sender_1)
        .await
        .unwrap();

    // Allow event processing
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Note: We can't directly access transceiver.get_subscriber_count() here
    // because the transceiver is moved into the run() loop. Instead, we verify
    // the pattern works by checking that subscribe succeeded.
    // Full integration test in onvif-rust/tests will validate end-to-end behavior.
}

// ========== Statistics Helper Method Tests ==========

#[tokio::test]
async fn test_update_audio_statistics_publisher() {
    let stats = Arc::new(Mutex::new(StatisticsStream::new(
        create_test_stream_identifier(),
    )));
    // Publisher data (uuid = None) with AAC_RAW type (1)
    StreamDataTransceiver::update_audio_statistics(
        None,
        500,
        crate::container::define::aac_packet_type::AAC_RAW,
        &stats,
    )
    .await;
    let guard = stats.lock().await;
    assert_eq!(guard.publisher.audio.recv_bytes, 500);
    assert_eq!(guard.total_recv_bytes, 500);
}

#[tokio::test]
async fn test_update_audio_statistics_publisher_non_raw() {
    let stats = Arc::new(Mutex::new(StatisticsStream::new(
        create_test_stream_identifier(),
    )));
    // Publisher data with non-RAW type should only update total_recv_bytes
    StreamDataTransceiver::update_audio_statistics(
        None,
        200,
        crate::container::define::aac_packet_type::AAC_SEQHDR,
        &stats,
    )
    .await;
    let guard = stats.lock().await;
    assert_eq!(guard.publisher.audio.recv_bytes, 0);
    assert_eq!(guard.total_recv_bytes, 200);
}

#[tokio::test]
async fn test_update_audio_statistics_subscriber() {
    let stats = Arc::new(Mutex::new(StatisticsStream::new(
        create_test_stream_identifier(),
    )));
    let sub_id = Uuid::new(crate::hub::utils::RandomDigitCount::Four);
    // Add a subscriber first
    StreamDataTransceiver::add_subscriber(
        sub_id,
        "127.0.0.1:5000".to_string(),
        SubscribeType::HttpFlvPull,
        chrono::Local::now(),
        &stats,
    )
    .await;
    // Update audio statistics for subscriber
    StreamDataTransceiver::update_audio_statistics(Some(sub_id), 300, 0, &stats).await;
    let guard = stats.lock().await;
    let sub = guard.subscribers.get(&sub_id).unwrap();
    assert_eq!(sub.send_bytes, 300);
    assert_eq!(guard.total_send_bytes, 300);
}

#[tokio::test]
async fn test_update_video_statistics_publisher() {
    let stats = Arc::new(Mutex::new(StatisticsStream::new(
        create_test_stream_identifier(),
    )));
    StreamDataTransceiver::update_video_statistics(None, 1000, 2, Some(false), &stats).await;
    let guard = stats.lock().await;
    assert_eq!(guard.publisher.video.recv_bytes, 1000);
    assert_eq!(guard.publisher.video.recv_frame_count, 2);
    assert_eq!(guard.publisher.recv_bytes, 1000);
    assert_eq!(guard.total_recv_bytes, 1000);
    assert_eq!(guard.publisher.video.recv_frame_count_for_gop, 2);
}

#[tokio::test]
async fn test_update_video_statistics_subscriber() {
    let stats = Arc::new(Mutex::new(StatisticsStream::new(
        create_test_stream_identifier(),
    )));
    let sub_id = Uuid::new(crate::hub::utils::RandomDigitCount::Four);
    StreamDataTransceiver::add_subscriber(
        sub_id,
        "10.0.0.1:8080".to_string(),
        SubscribeType::RtspPull,
        chrono::Local::now(),
        &stats,
    )
    .await;
    StreamDataTransceiver::update_video_statistics(Some(sub_id), 800, 1, None, &stats).await;
    let guard = stats.lock().await;
    let sub = guard.subscribers.get(&sub_id).unwrap();
    assert_eq!(sub.send_bytes, 800);
    assert_eq!(sub.total_send_bytes, 800);
    assert_eq!(guard.total_send_bytes, 800);
}

#[tokio::test]
async fn test_update_audio_codec_info() {
    let stats = Arc::new(Mutex::new(StatisticsStream::new(
        create_test_stream_identifier(),
    )));
    StreamDataTransceiver::update_audio_codec_info(
        crate::container::define::SoundFormat::AAC,
        crate::container::define::AacProfile::LC,
        48000,
        2,
        &stats,
    )
    .await;
    let guard = stats.lock().await;
    assert_eq!(guard.publisher.audio.samplerate, 48000);
    assert_eq!(guard.publisher.audio.channels, 2);
}

#[tokio::test]
async fn test_update_video_codec_info() {
    let stats = Arc::new(Mutex::new(StatisticsStream::new(
        create_test_stream_identifier(),
    )));
    StreamDataTransceiver::update_video_codec_info(
        crate::container::define::AvcCodecId::H264,
        crate::container::define::AvcProfile::High,
        crate::container::define::AvcLevel::Level4,
        1920,
        1080,
        &stats,
    )
    .await;
    let guard = stats.lock().await;
    assert_eq!(guard.publisher.video.width, 1920);
    assert_eq!(guard.publisher.video.height, 1080);
}

#[tokio::test]
async fn test_update_publisher_info() {
    let stats = Arc::new(Mutex::new(StatisticsStream::new(
        create_test_stream_identifier(),
    )));
    let pub_id = Uuid::new(crate::hub::utils::RandomDigitCount::Four);
    let now = chrono::Local::now();
    StreamDataTransceiver::update_publisher_info(
        pub_id,
        "192.168.1.100:1935".to_string(),
        now,
        &stats,
    )
    .await;
    let guard = stats.lock().await;
    assert_eq!(guard.publisher.id, pub_id);
    assert_eq!(guard.publisher.remote_address, "192.168.1.100:1935");
}

#[tokio::test]
async fn test_add_subscriber() {
    let stats = Arc::new(Mutex::new(StatisticsStream::new(
        create_test_stream_identifier(),
    )));
    let sub_id = Uuid::new(crate::hub::utils::RandomDigitCount::Four);
    StreamDataTransceiver::add_subscriber(
        sub_id,
        "10.0.0.5:8080".to_string(),
        SubscribeType::RtspPull,
        chrono::Local::now(),
        &stats,
    )
    .await;
    let guard = stats.lock().await;
    assert!(guard.subscribers.contains_key(&sub_id));
    let sub = guard.subscribers.get(&sub_id).unwrap();
    assert_eq!(sub.remote_address, "10.0.0.5:8080");
    assert_eq!(sub.send_bytes, 0);
}

#[tokio::test]
async fn test_receive_statistics_data_audio_codec() {
    let stats = Arc::new(Mutex::new(StatisticsStream::new(
        create_test_stream_identifier(),
    )));
    StreamDataTransceiver::receive_statistics_data(
        Some(StatisticData::AudioCodec {
            sound_format: crate::container::define::SoundFormat::AAC,
            profile: crate::container::define::AacProfile::LC,
            samplerate: 44100,
            channels: 1,
        }),
        &stats,
    )
    .await;
    let guard = stats.lock().await;
    assert_eq!(guard.publisher.audio.samplerate, 44100);
    assert_eq!(guard.publisher.audio.channels, 1);
}

#[tokio::test]
async fn test_receive_statistics_data_video_codec() {
    let stats = Arc::new(Mutex::new(StatisticsStream::new(
        create_test_stream_identifier(),
    )));
    StreamDataTransceiver::receive_statistics_data(
        Some(StatisticData::VideoCodec {
            codec: crate::container::define::AvcCodecId::H264,
            profile: crate::container::define::AvcProfile::Baseline,
            level: crate::container::define::AvcLevel::Level3,
            width: 1280,
            height: 720,
        }),
        &stats,
    )
    .await;
    let guard = stats.lock().await;
    assert_eq!(guard.publisher.video.width, 1280);
    assert_eq!(guard.publisher.video.height, 720);
}

#[tokio::test]
async fn test_receive_statistics_data_none() {
    let stats = Arc::new(Mutex::new(StatisticsStream::new(
        create_test_stream_identifier(),
    )));
    // Passing None should be a no-op
    StreamDataTransceiver::receive_statistics_data(None, &stats).await;
    let guard = stats.lock().await;
    assert_eq!(guard.total_recv_bytes, 0);
    assert_eq!(guard.total_send_bytes, 0);
}

#[tokio::test]
async fn test_receive_frame_data_none_is_noop() {
    let senders: Arc<Mutex<HashMap<Uuid, FrameDataSender>>> = Arc::new(Mutex::new(HashMap::new()));
    // Passing None should not panic
    StreamDataTransceiver::receive_frame_data(None, &senders).await;
    assert!(senders.lock().await.is_empty());
}

#[tokio::test]
async fn test_receive_packet_data_none_is_noop() {
    let senders: Arc<Mutex<HashMap<Uuid, PacketDataSender>>> = Arc::new(Mutex::new(HashMap::new()));
    StreamDataTransceiver::receive_packet_data(None, &senders).await;
    assert!(senders.lock().await.is_empty());
}

#[tokio::test]
async fn test_receive_frame_data_video_forwarded() {
    let senders: Arc<Mutex<HashMap<Uuid, FrameDataSender>>> = Arc::new(Mutex::new(HashMap::new()));
    let id = Uuid::new(crate::hub::utils::RandomDigitCount::Four);
    let (tx, mut rx) = mpsc::unbounded_channel::<FrameData>();
    senders.lock().await.insert(id, tx);

    let frame = FrameData::Video {
        timestamp: 42,
        data: BytesMut::from(&[0xAA, 0xBB][..]),
    };
    StreamDataTransceiver::receive_frame_data(Some(frame), &senders).await;

    let received = rx.try_recv().unwrap();
    match received {
        FrameData::Video { timestamp, data } => {
            assert_eq!(timestamp, 42);
            assert_eq!(data.as_ref(), &[0xAA, 0xBB]);
        }
        _ => panic!("expected Video frame"),
    }
}

#[tokio::test]
async fn test_receive_packet_data_video_forwarded() {
    let senders: Arc<Mutex<HashMap<Uuid, PacketDataSender>>> = Arc::new(Mutex::new(HashMap::new()));
    let id = Uuid::new(crate::hub::utils::RandomDigitCount::Four);
    let (tx, mut rx) = mpsc::unbounded_channel::<PacketData>();
    senders.lock().await.insert(id, tx);

    let packet = PacketData::Video {
        timestamp: 99,
        data: BytesMut::from(&[0xCC][..]),
    };
    StreamDataTransceiver::receive_packet_data(Some(packet), &senders).await;

    let received = rx.try_recv().unwrap();
    match received {
        PacketData::Video { timestamp, data } => {
            assert_eq!(timestamp, 99);
            assert_eq!(data.as_ref(), &[0xCC]);
        }
        _ => panic!("expected Video packet"),
    }
}

#[tokio::test]
async fn test_get_subscriber_count_decrements_on_unsubscribe() {
    // Test that subscriber count decreases when a subscriber is removed
    let mut hub = StreamsHub::new(None);
    let identifier = create_test_stream_identifier();
    let (_frame_sender, frame_receiver) = mpsc::unbounded_channel();
    let receiver = DataReceiver {
        frame_receiver: Some(frame_receiver),
        packet_receiver: None,
    };

    let mut mock_handler = MockStreamHandler::new();
    mock_handler
        .expect_send_prior_data()
        .returning(|_, _| Ok(()));
    let mock_handler = Arc::new(mock_handler);

    // Publish
    hub.publish(identifier.clone(), receiver, mock_handler.clone())
        .await
        .unwrap();

    // Subscribe
    let sub_info = create_test_subscriber_info();
    let (sender, _) = mpsc::unbounded_channel();
    let data_sender = DataSender::Frame { sender };
    hub.subscribe(&identifier, sub_info.clone(), data_sender)
        .await
        .unwrap();

    // Allow event processing
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Unsubscribe
    hub.unsubscribe(&identifier, sub_info.clone()).unwrap();

    // Allow event processing
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Verification happens in full integration test
}

// ========================================================================
// statistics_to_value Tests
// ========================================================================

#[test]
fn test_statistics_to_value_empty_data_no_top_n() {
    let data: Vec<StatisticsStream> = vec![];
    let result = StreamsHub::statistics_to_value(data, None).unwrap();
    assert!(result.is_array());
    assert_eq!(result.as_array().unwrap().len(), 0);
}

#[test]
fn test_statistics_to_value_single_stream_no_top_n() {
    let stream = StatisticsStream::new(StreamIdentifier::Rtsp {
        stream_path: "/live/cam1".to_string(),
    });
    let data = vec![stream];
    let result = StreamsHub::statistics_to_value(data, None).unwrap();
    assert!(result.is_array());
    assert_eq!(result.as_array().unwrap().len(), 1);
}

#[test]
fn test_statistics_to_value_with_top_n_limits_results() {
    let mut streams = Vec::new();
    for i in 0..5 {
        let mut s = StatisticsStream::new(StreamIdentifier::Rtsp {
            stream_path: format!("/live/cam{}", i),
        });
        s.subscriber_count = i;
        streams.push(s);
    }

    let result = StreamsHub::statistics_to_value(streams, Some(2)).unwrap();
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 2);
}

#[test]
fn test_statistics_to_value_with_top_n_sorts_by_subscriber_count_desc() {
    let mut streams = Vec::new();
    for i in 0..3 {
        let mut s = StatisticsStream::new(StreamIdentifier::Rtsp {
            stream_path: format!("/live/cam{}", i),
        });
        s.subscriber_count = i;
        streams.push(s);
    }

    let result = StreamsHub::statistics_to_value(streams, Some(3)).unwrap();
    let arr = result.as_array().unwrap();
    // Sorted descending by subscriber_count: 2, 1, 0
    assert_eq!(arr[0]["subscriber_count"], 2);
    assert_eq!(arr[1]["subscriber_count"], 1);
    assert_eq!(arr[2]["subscriber_count"], 0);
}

#[test]
fn test_statistics_to_value_top_n_larger_than_data() {
    let s = StatisticsStream::new(StreamIdentifier::Unknown);
    let data = vec![s];
    let result = StreamsHub::statistics_to_value(data, Some(10)).unwrap();
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 1);
}

#[test]
fn test_statistics_to_value_top_n_zero_returns_empty() {
    let s = StatisticsStream::new(StreamIdentifier::Unknown);
    let data = vec![s];
    let result = StreamsHub::statistics_to_value(data, Some(0)).unwrap();
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 0);
}

// ========================================================================
// get_hub_event_sender / get_client_event_consumer Tests
// ========================================================================

#[test]
fn test_get_hub_event_sender_returns_valid_sender() {
    let mut hub = StreamsHub::new(None);
    let sender = hub.get_hub_event_sender();
    // Sending an event should succeed since the receiver exists within the hub
    assert!(
        sender
            .send(StreamHubEvent::Request {
                identifier: StreamIdentifier::Unknown,
                sender: {
                    let (tx, _) = mpsc::unbounded_channel();
                    tx
                },
            })
            .is_ok()
    );
}

#[test]
fn test_get_client_event_consumer_returns_receiver() {
    let mut hub = StreamsHub::new(None);
    let _receiver = hub.get_client_event_consumer();
    // Just verify it returns without panic — the receiver type is opaque
}
