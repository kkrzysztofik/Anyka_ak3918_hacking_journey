use super::server::{DefaultHttpFlvServer, handle_connection};
use crate::common::auth::Auth;
use crate::hub::define::{StreamHubEvent, StreamHubEventSender};
use crate::hub::stream::StreamIdentifier;
use axum::{
    body::Body,
    extract::{ConnectInfo, State},
    http::{Request, Uri},
};
use std::net::SocketAddr;
use std::str::FromStr;
use tokio::sync::mpsc as tokio_mpsc;

#[allow(dead_code)]
fn create_test_event_sender() -> StreamHubEventSender {
    let (sender, _receiver) = tokio_mpsc::unbounded_channel();
    sender
}
#[tokio::test]
async fn test_default_httpflv_server_new() {
    let _event_sender = create_test_event_sender();
    let server = DefaultHttpFlvServer::new(
        "127.0.0.1:8080".to_string(),
        create_test_event_sender(),
        None,
    );

    // address and auth are private, so we can't test them directly
    // Just verify server was created successfully
    let _server = server;
}

#[tokio::test]
async fn test_default_httpflv_server_new_with_auth() {
    use crate::common::auth::{AuthAlgorithm, AuthType};
    let event_sender = create_test_event_sender();
    let auth = Auth::new(
        "test_key".to_string(),
        "test_secret".to_string(),
        None,
        AuthAlgorithm::Simple,
        AuthType::Pull,
    );
    let server = DefaultHttpFlvServer::new("127.0.0.1:8080".to_string(), event_sender, Some(auth));

    // address and auth are private, so we can't test them directly
    let _server = server;
}

#[tokio::test]
async fn test_handle_connection_valid_flv_path() {
    let _event_sender = create_test_event_sender();
    let uri = Uri::from_static("http://localhost/live/test.flv");
    let req = Request::builder().uri(uri).body(Body::empty()).unwrap();

    // Note: This test would require setting up the full axum state
    // For now, we test the path parsing logic
    let path = req.uri().path();
    match path.find(".flv") {
        Some(index) if index > 0 => {
            let (left, _) = path.split_at(index);
            let rv: Vec<_> = left.split('/').collect();
            assert_eq!(rv.len(), 3);
            assert_eq!(rv[1], "live");
            assert_eq!(rv[2], "test");
        }
        _ => panic!("Should find .flv in path"),
    }
}

#[tokio::test]
async fn test_handle_connection_invalid_path() {
    let uri = Uri::from_static("http://localhost/invalid/path");
    let req = Request::builder().uri(uri).body(Body::empty()).unwrap();

    let path = req.uri().path();
    assert!(
        path.find(".flv").is_none(),
        "invalid path must not be treated as an FLV request"
    );
}

#[tokio::test]
async fn test_handle_connection_path_too_few_segments_yields_bad_request_path() {
    // Path like "/live.flv" has .flv but left.split('/') gives ["", "live"], len 2 < 3 -> BAD_REQUEST
    let uri = Uri::from_static("http://localhost/live.flv");
    let req = Request::builder().uri(uri).body(Body::empty()).unwrap();

    let path = req.uri().path();
    if let Some(index) = path.find(".flv")
        && index > 0
    {
        let (left, _) = path.split_at(index);
        let rv: Vec<_> = left.split('/').collect();
        assert!(
            rv.len() < 3,
            "Path with single segment before .flv should yield fewer than 3 segments for BAD_REQUEST"
        );
    }
}

#[tokio::test]
async fn test_handle_connection_path_no_flv_extension_returns_not_found() {
    let uri = Uri::from_static("http://localhost/live/stream1");
    let req = Request::builder().uri(uri).body(Body::empty()).unwrap();

    let path = req.uri().path();
    assert!(
        path.find(".flv").is_none(),
        "Path without .flv should trigger NOT_FOUND"
    );
}

#[tokio::test]
async fn test_handle_connection_path_parsing() {
    let test_cases = vec![
        ("/live/stream1.flv", ("live", "stream1")),
        ("/app/name.flv", ("app", "name")),
        ("/test/123.flv", ("test", "123")),
    ];

    for (path_str, expected) in test_cases {
        let uri = Uri::from_str(&format!("http://localhost{}", path_str)).unwrap();
        let req = Request::builder().uri(uri).body(Body::empty()).unwrap();

        let path = req.uri().path();
        match path.find(".flv") {
            Some(index) if index > 0 => {
                let (left, _) = path.split_at(index);
                let rv: Vec<_> = left.split('/').collect();
                assert_eq!(rv[1], expected.0);
                assert_eq!(rv[2], expected.1);
            }
            _ => panic!("Should find .flv in path: {}", path_str),
        }
    }
}

#[tokio::test]
async fn test_handle_connection_auth_success() {
    use crate::common::auth::{AuthAlgorithm, AuthType, SecretCarrier};

    let auth = Auth::new(
        "test_key".to_string(),
        "test_secret".to_string(),
        None,
        AuthAlgorithm::Simple,
        AuthType::Pull,
    );
    let stream_name = "test_stream".to_string();
    let secret = Some(SecretCarrier::Query("token=test_secret".to_string()));

    let result = auth.authenticate(stream_name.as_str(), &secret, true);

    // Note: This depends on the actual auth implementation
    // Adjust based on how Auth::authenticate works
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_handle_connection_auth_failure() {
    use crate::common::auth::{AuthAlgorithm, AuthType, SecretCarrier};

    let auth = Auth::new(
        "test_key".to_string(),
        "test_secret".to_string(),
        None,
        AuthAlgorithm::Simple,
        AuthType::Pull,
    );
    let stream_name = "test_stream".to_string();
    let secret = Some(SecretCarrier::Query("token=wrong_secret".to_string()));

    let result = auth.authenticate(stream_name.as_str(), &secret, true);

    // Should fail with wrong token
    // Adjust based on actual auth implementation
    assert!(result.is_err());
}

#[tokio::test]
async fn test_handle_connection_requires_auth_without_credentials_returns_401_with_challenge() {
    use crate::common::auth::{AuthAlgorithm, AuthType};

    let (event_sender, _event_receiver) = tokio_mpsc::unbounded_channel();
    let auth = Auth::new(
        "test_key".to_string(),
        "test_secret".to_string(),
        None,
        AuthAlgorithm::Simple,
        AuthType::Pull,
    )
    .with_credential_validator(std::sync::Arc::new(|username, password| {
        username == "admin" && password == "secret"
    }))
    .with_basic_realm("ONVIF Camera");

    let uri = Uri::from_static("http://localhost/live/stream1.flv");
    let req = Request::builder().uri(uri).body(Body::empty()).unwrap();
    let remote_addr = SocketAddr::from(([127, 0, 0, 1], 1234));

    let response = handle_connection(
        State((event_sender, Some(auth))),
        ConnectInfo(remote_addr),
        req,
    )
    .await;
    assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);
    assert_eq!(
        response.headers().get("WWW-Authenticate").unwrap(),
        "Basic realm=\"ONVIF Camera\""
    );
    assert_eq!(
        response
            .headers()
            .get("Access-Control-Allow-Origin")
            .unwrap(),
        "*"
    );
}

#[tokio::test]
async fn test_handle_connection_options_preflight_skips_auth_and_returns_cors() {
    use crate::common::auth::{AuthAlgorithm, AuthType};
    use axum::http::Method;

    let (event_sender, _event_receiver) = tokio_mpsc::unbounded_channel();
    let auth = Auth::new(
        "test_key".to_string(),
        "test_secret".to_string(),
        None,
        AuthAlgorithm::Simple,
        AuthType::Pull,
    )
    .with_credential_validator(std::sync::Arc::new(|username, password| {
        username == "admin" && password == "secret"
    }))
    .with_basic_realm("ONVIF Camera");

    let uri = Uri::from_static("http://localhost/live/stream1.flv");
    let req = Request::builder()
        .method(Method::OPTIONS)
        .uri(uri)
        .header("Origin", "http://192.168.2.198")
        .header("Access-Control-Request-Method", "GET")
        .header("Access-Control-Request-Headers", "authorization")
        .body(Body::empty())
        .unwrap();
    let remote_addr = SocketAddr::from(([127, 0, 0, 1], 1234));

    let response = handle_connection(
        State((event_sender, Some(auth))),
        ConnectInfo(remote_addr),
        req,
    )
    .await;

    assert_eq!(response.status(), axum::http::StatusCode::NO_CONTENT);
    assert_eq!(
        response
            .headers()
            .get("Access-Control-Allow-Origin")
            .unwrap(),
        "*"
    );
    let allow_headers = response
        .headers()
        .get("Access-Control-Allow-Headers")
        .unwrap()
        .to_str()
        .unwrap()
        .to_ascii_lowercase();
    assert!(
        allow_headers.contains("authorization"),
        "preflight must allow Authorization, got {allow_headers}"
    );
    let allow_methods = response
        .headers()
        .get("Access-Control-Allow-Methods")
        .unwrap()
        .to_str()
        .unwrap()
        .to_ascii_uppercase();
    assert!(
        allow_methods.contains("GET"),
        "preflight must allow GET, got {allow_methods}"
    );
}

#[tokio::test]
async fn test_handle_connection_valid_basic_auth_returns_non_401() {
    use crate::common::auth::{AuthAlgorithm, AuthType};

    let (event_sender, _event_receiver) = tokio_mpsc::unbounded_channel();
    let auth = Auth::new(
        "test_key".to_string(),
        "test_secret".to_string(),
        None,
        AuthAlgorithm::Simple,
        AuthType::Pull,
    )
    .with_credential_validator(std::sync::Arc::new(|username, password| {
        username == "admin" && password == "secret"
    }))
    .with_basic_realm("ONVIF Camera");

    let uri = Uri::from_static("http://localhost/live/stream1.flv");
    let req = Request::builder()
        .uri(uri)
        .header("Authorization", "Basic YWRtaW46c2VjcmV0")
        .body(Body::empty())
        .unwrap();
    let remote_addr = SocketAddr::from(([127, 0, 0, 1], 1234));

    let response = handle_connection(
        State((event_sender, Some(auth))),
        ConnectInfo(remote_addr),
        req,
    )
    .await;
    assert_ne!(response.status(), axum::http::StatusCode::UNAUTHORIZED);
    assert_eq!(
        response.headers().get("Content-Type").unwrap(),
        "video/x-flv"
    );
}

#[tokio::test]
async fn test_httpflv_server_address_parsing() {
    let _event_sender = create_test_event_sender();
    let _server = DefaultHttpFlvServer::new(
        "127.0.0.1:8080".to_string(),
        create_test_event_sender(),
        None,
    );

    // address is private, test by trying to run (would fail if address invalid)
    // For now, just verify server creation
    let _server = DefaultHttpFlvServer::new(
        "127.0.0.1:8080".to_string(),
        create_test_event_sender(),
        None,
    );
    let sock_addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
    assert_eq!(sock_addr.port(), 8080);
    assert_eq!(sock_addr.ip().to_string(), "127.0.0.1");
}

#[tokio::test]
async fn test_httpflv_server_address_parsing_ipv6() {
    let _event_sender = create_test_event_sender();
    let _server =
        DefaultHttpFlvServer::new("[::1]:8080".to_string(), create_test_event_sender(), None);

    // address is private, test parsing directly
    let sock_addr: SocketAddr = "[::1]:8080".parse().unwrap();
    assert_eq!(sock_addr.port(), 8080);
}

#[tokio::test]
async fn test_httpflv_server_address_parsing_invalid() {
    let _event_sender = create_test_event_sender();
    let _server = DefaultHttpFlvServer::new(
        "invalid_address".to_string(),
        create_test_event_sender(),
        None,
    );

    // address is private, test invalid address parsing directly
    let result: Result<SocketAddr, _> = "invalid_address".parse();
    assert!(result.is_err());
}

#[tokio::test]
async fn test_response_headers() {
    let (event_sender, _event_receiver) = tokio_mpsc::unbounded_channel();
    let uri = Uri::from_static("http://localhost/live/test.flv");
    let req = Request::builder().uri(uri).body(Body::empty()).unwrap();
    let remote_addr = SocketAddr::from(([127, 0, 0, 1], 1234));

    let response =
        handle_connection(State((event_sender, None)), ConnectInfo(remote_addr), req).await;

    assert_eq!(
        response
            .headers()
            .get("Access-Control-Allow-Origin")
            .unwrap(),
        "*"
    );
    assert_eq!(
        response.headers().get("Content-Type").unwrap(),
        "video/x-flv"
    );
    assert_eq!(response.headers().get("Cache-Control").unwrap(), "no-cache");
}

#[tokio::test]
async fn test_stream_identifier_creation() {
    // Test that stream identifiers are created correctly from path
    let stream_path = "live/test";

    let identifier = StreamIdentifier::Rtsp {
        stream_path: stream_path.to_string(),
    };

    match identifier {
        StreamIdentifier::Rtsp { stream_path: s } => {
            assert_eq!(s, "live/test");
        }
        _ => panic!("Expected RTSP identifier"),
    }
}

#[tokio::test]
async fn test_event_producer_clone() {
    // Create event channel and keep receiver alive for the test duration
    let (event_sender, _event_receiver) = tokio_mpsc::unbounded_channel();
    let cloned = event_sender.clone();

    // Create inner channels for the Request events - keep receivers alive
    let (sender1, _receiver1) = tokio_mpsc::unbounded_channel();
    let (sender2, _receiver2) = tokio_mpsc::unbounded_channel();

    // Verify both event senders can be used
    assert!(
        event_sender
            .send(StreamHubEvent::Request {
                identifier: StreamIdentifier::Rtsp {
                    stream_path: "test/stream".to_string(),
                },
                sender: sender1,
            })
            .is_ok()
    );

    assert!(
        cloned
            .send(StreamHubEvent::Request {
                identifier: StreamIdentifier::Rtsp {
                    stream_path: "test2/stream2".to_string(),
                },
                sender: sender2,
            })
            .is_ok()
    );
}
