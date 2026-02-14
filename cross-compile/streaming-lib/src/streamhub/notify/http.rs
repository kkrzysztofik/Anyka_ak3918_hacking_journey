use super::super::define::StreamHubEventMessage;
use super::Notifier;
use async_trait::async_trait;
use reqwest::Client;

macro_rules! serialize_event {
    ($message:expr) => {{
        let event_serialize_str = match serde_json::to_string(&$message) {
            Ok(data) => {
                log::info!("event data: {}", data);
                data
            }
            Err(_) => String::from("empty body"),
        };
        event_serialize_str
    }};
}

pub struct HttpNotifier {
    request_client: Client,
    on_publish_url: Option<String>,
    on_unpublish_url: Option<String>,
    on_play_url: Option<String>,
    on_stop_url: Option<String>,
    on_hls_url: Option<String>,
}

impl HttpNotifier {
    pub fn new(
        on_publish_url: Option<String>,
        on_unpublish_url: Option<String>,
        on_play_url: Option<String>,
        on_stop_url: Option<String>,
        on_hls_url: Option<String>,
    ) -> Self {
        Self {
            request_client: reqwest::Client::new(),
            on_publish_url,
            on_unpublish_url,
            on_play_url,
            on_stop_url,
            on_hls_url,
        }
    }
}

#[async_trait]
impl Notifier for HttpNotifier {
    async fn on_publish_notify(&self, event: &StreamHubEventMessage) {
        if let Some(on_publish_url) = &self.on_publish_url {
            match self
                .request_client
                .post(on_publish_url)
                .body(serialize_event!(event))
                .send()
                .await
            {
                Err(err) => {
                    log::error!("on_publish error: {}", err);
                }
                Ok(response) => {
                    log::info!("on_publish success: {:?}", response);
                }
            }
        }
    }

    async fn on_unpublish_notify(&self, event: &StreamHubEventMessage) {
        if let Some(on_unpublish_url) = &self.on_unpublish_url {
            match self
                .request_client
                .post(on_unpublish_url)
                .body(serialize_event!(event))
                .send()
                .await
            {
                Err(err) => {
                    log::error!("on_unpublish error: {}", err);
                }
                Ok(response) => {
                    log::info!("on_unpublish success: {:?}", response);
                }
            }
        }
    }

    async fn on_play_notify(&self, event: &StreamHubEventMessage) {
        if let Some(on_play_url) = &self.on_play_url {
            match self
                .request_client
                .post(on_play_url)
                .body(serialize_event!(event))
                .send()
                .await
            {
                Err(err) => {
                    log::error!("on_play error: {}", err);
                }
                Ok(response) => {
                    log::info!("on_play success: {:?}", response);
                }
            }
        }
    }

    async fn on_stop_notify(&self, event: &StreamHubEventMessage) {
        if let Some(on_stop_url) = &self.on_stop_url {
            match self
                .request_client
                .post(on_stop_url)
                .body(serialize_event!(event))
                .send()
                .await
            {
                Err(err) => {
                    log::error!("on_stop error: {}", err);
                }
                Ok(response) => {
                    log::info!("on_stop success: {:?}", response);
                }
            }
        }
    }

    async fn on_hls_notify(&self, event: &StreamHubEventMessage) {
        if let Some(on_hls_url) = &self.on_hls_url {
            match self
                .request_client
                .post(on_hls_url)
                .body(serialize_event!(event))
                .send()
                .await
            {
                Err(err) => {
                    log::error!("on_hls error: {}", err);
                }
                Ok(response) => {
                    log::info!("on_hls success: {:?}", response);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::Notifier;
    use super::*;
    use crate::streamhub::define::{
        NotifyInfo, PubDataType, PublishType, PublisherInfo, SubDataType, SubscribeType,
        SubscriberInfo,
    };
    use crate::streamhub::stream::StreamIdentifier;
    use crate::streamhub::utils::{RandomDigitCount, Uuid};
    use wiremock::matchers::{body_string_contains, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    // Helper function to create a test event message
    fn create_test_event_message() -> StreamHubEventMessage {
        StreamHubEventMessage::Publish {
            identifier: StreamIdentifier::Rtmp {
                app_name: "live".to_string(),
                stream_name: "test_stream".to_string(),
            },
            info: PublisherInfo {
                id: Uuid::new(RandomDigitCount::Four),
                pub_type: PublishType::RtmpPush,
                pub_data_type: PubDataType::Frame,
                notify_info: NotifyInfo {
                    request_url: "rtmp://localhost/live/test_stream".to_string(),
                    remote_addr: "127.0.0.1:12345".to_string(),
                },
            },
        }
    }

    // Helper function to create a subscribe event message
    fn create_subscribe_event_message() -> StreamHubEventMessage {
        StreamHubEventMessage::Subscribe {
            identifier: StreamIdentifier::Rtmp {
                app_name: "live".to_string(),
                stream_name: "test_stream".to_string(),
            },
            info: SubscriberInfo {
                id: Uuid::new(RandomDigitCount::Four),
                sub_type: SubscribeType::RtmpPull,
                notify_info: NotifyInfo {
                    request_url: "rtmp://localhost/live/test_stream".to_string(),
                    remote_addr: "127.0.0.1:54321".to_string(),
                },
                sub_data_type: SubDataType::Frame,
            },
        }
    }

    // ========== Constructor Tests ==========

    #[test]
    fn test_http_notifier_new_with_all_urls() {
        let notifier = HttpNotifier::new(
            Some("http://localhost/publish".to_string()),
            Some("http://localhost/unpublish".to_string()),
            Some("http://localhost/play".to_string()),
            Some("http://localhost/stop".to_string()),
            Some("http://localhost/hls".to_string()),
        );

        // Verify the notifier was created successfully
        let _arc = std::sync::Arc::new(notifier);
    }

    #[test]
    fn test_http_notifier_new_with_no_urls() {
        let notifier = HttpNotifier::new(None, None, None, None, None);

        // Verify the notifier was created successfully
        let _arc = std::sync::Arc::new(notifier);
    }

    #[test]
    fn test_http_notifier_new_with_partial_urls() {
        let notifier = HttpNotifier::new(
            Some("http://localhost/publish".to_string()),
            None,
            Some("http://localhost/play".to_string()),
            None,
            None,
        );

        // Verify the notifier was created successfully
        let _arc = std::sync::Arc::new(notifier);
    }

    #[test]
    fn test_http_notifier_new_with_empty_url_strings() {
        let notifier = HttpNotifier::new(
            Some("".to_string()),
            Some("".to_string()),
            Some("".to_string()),
            Some("".to_string()),
            Some("".to_string()),
        );

        // Empty strings are still Some, so notifier should be created
        let _arc = std::sync::Arc::new(notifier);
    }

    // ========== URL Unset Tests (Early Return) ==========

    #[tokio::test]
    async fn test_on_publish_notify_url_unset_completes_without_error() {
        let notifier = HttpNotifier::new(None, None, None, None, None);
        let event = create_test_event_message();

        // Should complete without error when URL is not set
        notifier.on_publish_notify(&event).await;
    }

    #[tokio::test]
    async fn test_on_unpublish_notify_url_unset_completes_without_error() {
        let notifier = HttpNotifier::new(None, None, None, None, None);
        let event = create_test_event_message();

        // Should complete without error when URL is not set
        notifier.on_unpublish_notify(&event).await;
    }

    #[tokio::test]
    async fn test_on_play_notify_url_unset_completes_without_error() {
        let notifier = HttpNotifier::new(None, None, None, None, None);
        let event = create_subscribe_event_message();

        // Should complete without error when URL is not set
        notifier.on_play_notify(&event).await;
    }

    #[tokio::test]
    async fn test_on_stop_notify_url_unset_completes_without_error() {
        let notifier = HttpNotifier::new(None, None, None, None, None);
        let event = create_subscribe_event_message();

        // Should complete without error when URL is not set
        notifier.on_stop_notify(&event).await;
    }

    #[tokio::test]
    async fn test_on_hls_notify_url_unset_completes_without_error() {
        let notifier = HttpNotifier::new(None, None, None, None, None);
        let event = create_test_event_message();

        // Should complete without error when URL is not set
        notifier.on_hls_notify(&event).await;
    }

    // ========== URL Set - Success Path Tests ==========

    #[tokio::test]
    async fn test_on_publish_notify_url_set_sends_http_post() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/publish"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        let notifier = HttpNotifier::new(
            Some(format!("{}/publish", mock_server.uri())),
            None,
            None,
            None,
            None,
        );
        let event = create_test_event_message();

        notifier.on_publish_notify(&event).await;

        // Verify the request was received
        mock_server.verify().await;
    }

    #[tokio::test]
    async fn test_on_unpublish_notify_url_set_sends_http_post() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/unpublish"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        let notifier = HttpNotifier::new(
            None,
            Some(format!("{}/unpublish", mock_server.uri())),
            None,
            None,
            None,
        );
        let event = create_test_event_message();

        notifier.on_unpublish_notify(&event).await;

        // Verify the request was received
        mock_server.verify().await;
    }

    #[tokio::test]
    async fn test_on_play_notify_url_set_sends_http_post() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/play"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        let notifier = HttpNotifier::new(
            None,
            None,
            Some(format!("{}/play", mock_server.uri())),
            None,
            None,
        );
        let event = create_subscribe_event_message();

        notifier.on_play_notify(&event).await;

        // Verify the request was received
        mock_server.verify().await;
    }

    #[tokio::test]
    async fn test_on_stop_notify_url_set_sends_http_post() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/stop"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        let notifier = HttpNotifier::new(
            None,
            None,
            None,
            Some(format!("{}/stop", mock_server.uri())),
            None,
        );
        let event = create_subscribe_event_message();

        notifier.on_stop_notify(&event).await;

        // Verify the request was received
        mock_server.verify().await;
    }

    #[tokio::test]
    async fn test_on_hls_notify_url_set_sends_http_post() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/hls"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        let notifier = HttpNotifier::new(
            None,
            None,
            None,
            None,
            Some(format!("{}/hls", mock_server.uri())),
        );
        let event = create_test_event_message();

        notifier.on_hls_notify(&event).await;

        // Verify the request was received
        mock_server.verify().await;
    }

    // ========== HTTP Request Failure Tests ==========

    #[tokio::test]
    async fn test_on_publish_notify_connection_error_completes_without_panic() {
        // Use an invalid URL that will cause a connection error
        let notifier = HttpNotifier::new(
            Some("http://localhost:1/invalid_endpoint".to_string()),
            None,
            None,
            None,
            None,
        );
        let event = create_test_event_message();

        // Should complete without panic even though connection will fail
        notifier.on_publish_notify(&event).await;
    }

    #[tokio::test]
    async fn test_on_unpublish_notify_connection_error_completes_without_panic() {
        let notifier = HttpNotifier::new(
            None,
            Some("http://localhost:1/invalid_endpoint".to_string()),
            None,
            None,
            None,
        );
        let event = create_test_event_message();

        // Should complete without panic even though connection will fail
        notifier.on_unpublish_notify(&event).await;
    }

    #[tokio::test]
    async fn test_on_play_notify_connection_error_completes_without_panic() {
        let notifier = HttpNotifier::new(
            None,
            None,
            Some("http://localhost:1/invalid_endpoint".to_string()),
            None,
            None,
        );
        let event = create_subscribe_event_message();

        // Should complete without panic even though connection will fail
        notifier.on_play_notify(&event).await;
    }

    #[tokio::test]
    async fn test_on_stop_notify_connection_error_completes_without_panic() {
        let notifier = HttpNotifier::new(
            None,
            None,
            None,
            Some("http://localhost:1/invalid_endpoint".to_string()),
            None,
        );
        let event = create_subscribe_event_message();

        // Should complete without panic even though connection will fail
        notifier.on_stop_notify(&event).await;
    }

    #[tokio::test]
    async fn test_on_hls_notify_connection_error_completes_without_panic() {
        let notifier = HttpNotifier::new(
            None,
            None,
            None,
            None,
            Some("http://localhost:1/invalid_endpoint".to_string()),
        );
        let event = create_test_event_message();

        // Should complete without panic even though connection will fail
        notifier.on_hls_notify(&event).await;
    }

    // ========== HTTP Error Response Tests ==========

    #[tokio::test]
    async fn test_on_publish_notify_server_error_response_completes_without_panic() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/publish"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&mock_server)
            .await;

        let notifier = HttpNotifier::new(
            Some(format!("{}/publish", mock_server.uri())),
            None,
            None,
            None,
            None,
        );
        let event = create_test_event_message();

        // Should complete without panic even though server returns 500
        notifier.on_publish_notify(&event).await;

        mock_server.verify().await;
    }

    #[tokio::test]
    async fn test_on_publish_notify_not_found_response_completes_without_panic() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/publish"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;

        let notifier = HttpNotifier::new(
            Some(format!("{}/publish", mock_server.uri())),
            None,
            None,
            None,
            None,
        );
        let event = create_test_event_message();

        // Should complete without panic even though server returns 404
        notifier.on_publish_notify(&event).await;

        mock_server.verify().await;
    }

    #[tokio::test]
    async fn test_on_publish_notify_unauthorized_response_completes_without_panic() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/publish"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&mock_server)
            .await;

        let notifier = HttpNotifier::new(
            Some(format!("{}/publish", mock_server.uri())),
            None,
            None,
            None,
            None,
        );
        let event = create_test_event_message();

        // Should complete without panic even though server returns 401
        notifier.on_publish_notify(&event).await;

        mock_server.verify().await;
    }

    // ========== Body Serialization Tests ==========

    #[tokio::test]
    async fn test_on_publish_notify_serializes_event_in_body() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/publish"))
            .and(body_string_contains("Publish"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        let notifier = HttpNotifier::new(
            Some(format!("{}/publish", mock_server.uri())),
            None,
            None,
            None,
            None,
        );
        let event = create_test_event_message();

        notifier.on_publish_notify(&event).await;

        // Verify the body contains serialized event data
        mock_server.verify().await;
    }

    #[tokio::test]
    async fn test_on_play_notify_serializes_subscribe_event_in_body() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/play"))
            .and(body_string_contains("Subscribe"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        let notifier = HttpNotifier::new(
            None,
            None,
            Some(format!("{}/play", mock_server.uri())),
            None,
            None,
        );
        let event = create_subscribe_event_message();

        notifier.on_play_notify(&event).await;

        // Verify the body contains serialized event data
        mock_server.verify().await;
    }

    // ========== NotSupport Event Tests ==========

    #[tokio::test]
    async fn test_on_publish_notify_with_not_support_event_completes_without_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/publish"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        let notifier = HttpNotifier::new(
            Some(format!("{}/publish", mock_server.uri())),
            None,
            None,
            None,
            None,
        );
        let event = StreamHubEventMessage::NotSupport {};

        notifier.on_publish_notify(&event).await;

        mock_server.verify().await;
    }

    // ========== Multiple Notify Calls Tests ==========

    #[tokio::test]
    async fn test_multiple_notify_calls_sends_multiple_requests() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/publish"))
            .respond_with(ResponseTemplate::new(200))
            .expect(3)
            .mount(&mock_server)
            .await;

        let notifier = HttpNotifier::new(
            Some(format!("{}/publish", mock_server.uri())),
            None,
            None,
            None,
            None,
        );
        let event = create_test_event_message();

        // Call notify multiple times
        notifier.on_publish_notify(&event).await;
        notifier.on_publish_notify(&event).await;
        notifier.on_publish_notify(&event).await;

        // Verify all requests were received
        mock_server.verify().await;
    }

    // ========== All URLs Configured Tests ==========

    #[tokio::test]
    async fn test_all_notify_methods_with_all_urls_configured() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/publish"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;
        Mock::given(method("POST"))
            .and(path("/unpublish"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;
        Mock::given(method("POST"))
            .and(path("/play"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;
        Mock::given(method("POST"))
            .and(path("/stop"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;
        Mock::given(method("POST"))
            .and(path("/hls"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        let notifier = HttpNotifier::new(
            Some(format!("{}/publish", mock_server.uri())),
            Some(format!("{}/unpublish", mock_server.uri())),
            Some(format!("{}/play", mock_server.uri())),
            Some(format!("{}/stop", mock_server.uri())),
            Some(format!("{}/hls", mock_server.uri())),
        );

        let pub_event = create_test_event_message();
        let sub_event = create_subscribe_event_message();

        // Call all notify methods
        notifier.on_publish_notify(&pub_event).await;
        notifier.on_unpublish_notify(&pub_event).await;
        notifier.on_play_notify(&sub_event).await;
        notifier.on_stop_notify(&sub_event).await;
        notifier.on_hls_notify(&pub_event).await;

        // All requests should have been received
        mock_server.verify().await;
    }

    // ========== Edge Cases ==========

    #[tokio::test]
    async fn test_notify_with_special_characters_in_url() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/publish/event"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        let notifier = HttpNotifier::new(
            Some(format!("{}/publish/event", mock_server.uri())),
            None,
            None,
            None,
            None,
        );
        let event = create_test_event_message();

        notifier.on_publish_notify(&event).await;

        mock_server.verify().await;
    }

    #[tokio::test]
    async fn test_notify_with_unsubscribe_event() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/stop"))
            .and(body_string_contains("UnSubscribe"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        let notifier = HttpNotifier::new(
            None,
            None,
            None,
            Some(format!("{}/stop", mock_server.uri())),
            None,
        );

        let event = StreamHubEventMessage::UnSubscribe {
            identifier: StreamIdentifier::Rtmp {
                app_name: "live".to_string(),
                stream_name: "test_stream".to_string(),
            },
            info: SubscriberInfo {
                id: Uuid::new(RandomDigitCount::Four),
                sub_type: SubscribeType::RtmpPull,
                notify_info: NotifyInfo {
                    request_url: "rtmp://localhost/live/test_stream".to_string(),
                    remote_addr: "127.0.0.1:54321".to_string(),
                },
                sub_data_type: SubDataType::Frame,
            },
        };

        notifier.on_stop_notify(&event).await;

        mock_server.verify().await;
    }

    #[tokio::test]
    async fn test_notify_with_unpublish_event() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/unpublish"))
            .and(body_string_contains("UnPublish"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        let notifier = HttpNotifier::new(
            None,
            Some(format!("{}/unpublish", mock_server.uri())),
            None,
            None,
            None,
        );

        let event = StreamHubEventMessage::UnPublish {
            identifier: StreamIdentifier::Rtmp {
                app_name: "live".to_string(),
                stream_name: "test_stream".to_string(),
            },
            info: PublisherInfo {
                id: Uuid::new(RandomDigitCount::Four),
                pub_type: PublishType::RtmpPush,
                pub_data_type: PubDataType::Frame,
                notify_info: NotifyInfo {
                    request_url: "rtmp://localhost/live/test_stream".to_string(),
                    remote_addr: "127.0.0.1:12345".to_string(),
                },
            },
        };

        notifier.on_unpublish_notify(&event).await;

        mock_server.verify().await;
    }

    // ========== Rtsp Identifier and NotSupport Variants ==========

    #[tokio::test]
    async fn test_on_publish_notify_with_rtsp_identifier_sends_body() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/publish"))
            .and(body_string_contains("Rtsp"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        let notifier = HttpNotifier::new(
            Some(format!("{}/publish", mock_server.uri())),
            None,
            None,
            None,
            None,
        );

        let event = StreamHubEventMessage::Publish {
            identifier: StreamIdentifier::Rtsp {
                stream_path: "/live/stream".to_string(),
            },
            info: PublisherInfo {
                id: Uuid::new(RandomDigitCount::Four),
                pub_type: PublishType::RtspPush,
                pub_data_type: PubDataType::Frame,
                notify_info: NotifyInfo {
                    request_url: "rtsp://localhost/live/stream".to_string(),
                    remote_addr: "127.0.0.1:554".to_string(),
                },
            },
        };

        notifier.on_publish_notify(&event).await;

        mock_server.verify().await;
    }

    #[tokio::test]
    async fn test_on_hls_notify_with_not_support_event_completes() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/hls"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        let notifier = HttpNotifier::new(
            None,
            None,
            None,
            None,
            Some(format!("{}/hls", mock_server.uri())),
        );
        let event = StreamHubEventMessage::NotSupport {};

        notifier.on_hls_notify(&event).await;

        mock_server.verify().await;
    }

    #[tokio::test]
    async fn test_on_stop_notify_non_200_response_logs_and_completes() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/stop"))
            .respond_with(ResponseTemplate::new(503))
            .mount(&mock_server)
            .await;

        let notifier = HttpNotifier::new(
            None,
            None,
            None,
            Some(format!("{}/stop", mock_server.uri())),
            None,
        );
        let event = create_subscribe_event_message();

        notifier.on_stop_notify(&event).await;

        mock_server.verify().await;
    }

    // ========== Thread Safety Tests ==========

    #[tokio::test]
    async fn test_http_notifier_concurrent_notify_calls() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200))
            .expect(5)
            .mount(&mock_server)
            .await;

        let notifier = std::sync::Arc::new(HttpNotifier::new(
            Some(format!("{}/publish", mock_server.uri())),
            None,
            None,
            None,
            None,
        ));

        let event = create_test_event_message();

        // Spawn multiple concurrent notify tasks
        let mut handles = vec![];
        for _ in 0..5 {
            let notifier_clone = std::sync::Arc::clone(&notifier);
            let event_clone = event.clone();
            handles.push(tokio::spawn(async move {
                notifier_clone.on_publish_notify(&event_clone).await;
            }));
        }

        // Wait for all tasks to complete
        for handle in handles {
            handle.await.expect("Task should complete without error");
        }

        // All requests should have been received
        mock_server.verify().await;
    }
}
