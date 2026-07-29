use super::*;
use crate::common::http::HttpRequest as RtspRequest;
use crate::config::StreamingConfig;
use crate::io::bytes_reader::BytesReader;
use bytes::BytesMut;
use http::StatusCode;

// ========================================================================
// gen_response Tests
// ========================================================================

/// Create a test RtspRequest with the given method and CSeq
fn create_test_request(method: &str, cseq: Option<&str>) -> RtspRequest {
    let mut request = RtspRequest {
        method: method.to_string(),
        version: "RTSP/1.0".to_string(),
        ..Default::default()
    };
    if let Some(seq) = cseq {
        request.headers.insert("CSeq".to_string(), seq.to_string());
    }
    request
}

fn add_default_video_track(session: &mut RtspServerSession) {
    let codec_info = RtspCodecInfo {
        codec_id: crate::protocol::rtsp::rtsp_codec::RtspCodecId::H264,
        payload_type: 96,
        sample_rate: 90000,
        channel_count: 0,
    };
    let track = RtspTrack::new(TrackType::Video, codec_info, "trackID=0".to_string());
    session.tracks.insert(TrackType::Video, track);
}

#[test]
fn test_gen_response_ok_status() {
    let request = create_test_request("OPTIONS", Some("1"));

    let response = RtspServerSession::gen_response(StatusCode::OK, &request);
    assert_eq!(response.version, "RTSP/1.0");
    assert_eq!(response.status_code, 200);
    assert_eq!(response.reason_phrase, "OK");
    assert_eq!(response.headers.get("CSeq"), Some(&"1".to_string()));
}

#[test]
fn test_gen_response_not_found_status() {
    let request = create_test_request("DESCRIBE", None);

    let response = RtspServerSession::gen_response(StatusCode::NOT_FOUND, &request);
    assert_eq!(response.status_code, 404);
    assert_eq!(response.reason_phrase, "Not Found");
}

#[test]
fn test_gen_response_unauthorized_status() {
    let request = create_test_request("PLAY", None);

    let response = RtspServerSession::gen_response(StatusCode::UNAUTHORIZED, &request);
    assert_eq!(response.status_code, 401);
    assert_eq!(response.reason_phrase, "Unauthorized");
}

#[test]
fn test_gen_response_with_cseq() {
    let request = create_test_request("SETUP", Some("42"));

    let response = RtspServerSession::gen_response(StatusCode::OK, &request);
    assert_eq!(response.headers.get("CSeq"), Some(&"42".to_string()));
}

#[test]
fn test_gen_response_without_cseq() {
    let request = create_test_request("OPTIONS", None);

    let response = RtspServerSession::gen_response(StatusCode::OK, &request);
    assert!(response.headers.get("CSeq").is_none());
}

#[test]
fn test_gen_response_bad_request() {
    let request = create_test_request("INVALID", None);

    let response = RtspServerSession::gen_response(StatusCode::BAD_REQUEST, &request);
    assert_eq!(response.status_code, 400);
    assert_eq!(response.reason_phrase, "Bad Request");
}

#[test]
fn test_gen_response_internal_error() {
    let request = create_test_request("PLAY", None);

    let response = RtspServerSession::gen_response(StatusCode::INTERNAL_SERVER_ERROR, &request);
    assert_eq!(response.status_code, 500);
    assert_eq!(response.reason_phrase, "Internal Server Error");
}

// ========================================================================
// parse_session_header Tests
// ========================================================================

#[test]
fn test_parse_session_header_extracts_id_before_semicolon() {
    let id = RtspServerSession::parse_session_header("1234567890;timeout=60");
    assert_eq!(id, "1234567890");
}

#[test]
fn test_parse_session_header_trimmed() {
    let id = RtspServerSession::parse_session_header("  abc123  ");
    assert_eq!(id, "abc123");
}

#[test]
fn test_parse_session_header_empty_returns_empty() {
    let id = RtspServerSession::parse_session_header("");
    assert_eq!(id, "");
}

#[test]
fn test_parse_session_header_semicolon_only_returns_empty() {
    let id = RtspServerSession::parse_session_header(";timeout=60");
    assert_eq!(id, "");
}

// ========================================================================
// validate_session_id Tests
// ========================================================================

#[test]
fn test_validate_session_id_mismatch_returns_454_response() {
    let (event_sender, _) = tokio::sync::mpsc::unbounded_channel();
    let mock_io = MockNetIO::new();
    let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
    let remote_addr = "127.0.0.1:0".parse().unwrap();
    let mut session = RtspServerSession::new_with_io(
        session_io,
        event_sender,
        None,
        remote_addr,
        StreamingConfig::default(),
    );
    session.session_id = Some(Uuid::new(SESSION_ID_RANDOM_DIGITS));

    let mut request = create_test_request("TEARDOWN", Some("1"));
    request
        .headers
        .insert("Session".to_string(), "wrong-session-id".to_string());

    let response = session.validate_session_id(&request);
    let resp = response.expect("expected Some(454)");
    assert_eq!(resp.status_code, 454);
    assert_eq!(resp.reason_phrase, "Session Not Found");
}

#[test]
fn test_validate_session_id_match_returns_none() {
    let (event_sender, _) = tokio::sync::mpsc::unbounded_channel();
    let mock_io = MockNetIO::new();
    let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
    let remote_addr = "127.0.0.1:0".parse().unwrap();
    let mut session = RtspServerSession::new_with_io(
        session_io,
        event_sender,
        None,
        remote_addr,
        StreamingConfig::default(),
    );
    let session_id = Uuid::new(SESSION_ID_RANDOM_DIGITS);
    session.session_id = Some(session_id);

    let mut request = create_test_request("TEARDOWN", Some("1"));
    request
        .headers
        .insert("Session".to_string(), session_id.to_string());

    let response = session.validate_session_id(&request);
    assert!(response.is_none());
}

#[test]
fn test_validate_session_id_no_session_header_returns_none() {
    let (event_sender, _) = tokio::sync::mpsc::unbounded_channel();
    let mock_io = MockNetIO::new();
    let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
    let remote_addr = "127.0.0.1:0".parse().unwrap();
    let mut session = RtspServerSession::new_with_io(
        session_io,
        event_sender,
        None,
        remote_addr,
        StreamingConfig::default(),
    );
    session.session_id = Some(Uuid::new(SESSION_ID_RANDOM_DIGITS));

    let request = create_test_request("TEARDOWN", Some("1"));

    let response = session.validate_session_id(&request);
    assert!(response.is_none());
}

#[test]
fn test_validate_session_id_no_current_session_returns_none() {
    let (event_sender, _) = tokio::sync::mpsc::unbounded_channel();
    let mock_io = MockNetIO::new();
    let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
    let remote_addr = "127.0.0.1:0".parse().unwrap();
    let session = RtspServerSession::new_with_io(
        session_io,
        event_sender,
        None,
        remote_addr,
        StreamingConfig::default(),
    );
    assert!(session.session_id.is_none());

    let mut request = create_test_request("TEARDOWN", Some("1"));
    request
        .headers
        .insert("Session".to_string(), "any-id".to_string());

    let response = session.validate_session_id(&request);
    assert!(response.is_none());
}

#[test]
fn test_validate_session_id_empty_session_value_returns_none() {
    let (event_sender, _) = tokio::sync::mpsc::unbounded_channel();
    let mock_io = MockNetIO::new();
    let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
    let remote_addr = "127.0.0.1:0".parse().unwrap();
    let mut session = RtspServerSession::new_with_io(
        session_io,
        event_sender,
        None,
        remote_addr,
        StreamingConfig::default(),
    );
    session.session_id = Some(Uuid::new(SESSION_ID_RANDOM_DIGITS));

    let mut request = create_test_request("TEARDOWN", Some("1"));
    request
        .headers
        .insert("Session".to_string(), ";timeout=60".to_string());

    let response = session.validate_session_id(&request);
    assert!(
        response.is_none(),
        "empty parsed session id should not trigger 454"
    );
}

// ========================================================================
// build_content_base Tests
// ========================================================================

#[test]
fn test_build_content_base_with_host_and_path_returns_trailing_slash() {
    let (event_sender, _) = tokio::sync::mpsc::unbounded_channel();
    let mock_io = MockNetIO::new();
    let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
    let remote_addr = "127.0.0.1:0".parse().unwrap();
    let session = RtspServerSession::new_with_io(
        session_io,
        event_sender,
        None,
        remote_addr,
        StreamingConfig::default(),
    );

    let mut request = create_test_request("DESCRIBE", Some("1"));
    request.uri.host = "127.0.0.1".to_string();
    request.uri.port = Some(8554);
    request.uri.path = "live/test".to_string();

    let base = session.build_content_base(&request);
    assert_eq!(base.as_deref(), Some("rtsp://127.0.0.1:8554/live/test/"));
}

#[test]
fn test_build_content_base_host_only_no_path_returns_base() {
    let (event_sender, _) = tokio::sync::mpsc::unbounded_channel();
    let mock_io = MockNetIO::new();
    let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
    let remote_addr = "127.0.0.1:0".parse().unwrap();
    let session = RtspServerSession::new_with_io(
        session_io,
        event_sender,
        None,
        remote_addr,
        StreamingConfig::default(),
    );

    let mut request = create_test_request("DESCRIBE", Some("1"));
    request.uri.host = "127.0.0.1".to_string();
    request.uri.port = Some(8554);
    request.uri.path = String::new();

    let base = session.build_content_base(&request);
    assert_eq!(base.as_deref(), Some("rtsp://127.0.0.1:8554/"));
}

#[test]
fn test_build_content_base_empty_host_uses_host_header() {
    let (event_sender, _) = tokio::sync::mpsc::unbounded_channel();
    let mock_io = MockNetIO::new();
    let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
    let remote_addr = "127.0.0.1:0".parse().unwrap();
    let session = RtspServerSession::new_with_io(
        session_io,
        event_sender,
        None,
        remote_addr,
        StreamingConfig::default(),
    );

    let mut request = create_test_request("DESCRIBE", Some("1"));
    request.uri.host = String::new();
    request.uri.path = "live/stream".to_string();
    request
        .headers
        .insert("Host".to_string(), "example.com:554".to_string());

    let base = session.build_content_base(&request);
    assert_eq!(base.as_deref(), Some("rtsp://example.com:554/live/stream/"));
}

#[test]
fn test_build_content_base_empty_host_no_host_header_empty_path_returns_none() {
    let (event_sender, _) = tokio::sync::mpsc::unbounded_channel();
    let mock_io = MockNetIO::new();
    let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
    let remote_addr = "127.0.0.1:0".parse().unwrap();
    let session = RtspServerSession::new_with_io(
        session_io,
        event_sender,
        None,
        remote_addr,
        StreamingConfig::default(),
    );

    let mut request = create_test_request("DESCRIBE", Some("1"));
    request.uri.host = String::new();
    request.uri.path = String::new();

    let base = session.build_content_base(&request);
    assert!(base.is_none());
}

#[test]
fn test_build_content_base_host_header_without_colon_uses_full_header() {
    let (event_sender, _) = tokio::sync::mpsc::unbounded_channel();
    let mock_io = MockNetIO::new();
    let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
    let remote_addr = "127.0.0.1:0".parse().unwrap();
    let session = RtspServerSession::new_with_io(
        session_io,
        event_sender,
        None,
        remote_addr,
        StreamingConfig::default(),
    );

    let mut request = create_test_request("DESCRIBE", Some("1"));
    request.uri.host = String::new();
    request.uri.path = "live/stream".to_string();
    request
        .headers
        .insert("Host".to_string(), "example.com".to_string());

    let base = session.build_content_base(&request);
    assert_eq!(base.as_deref(), Some("rtsp://example.com/live/stream/"));
}

#[test]
fn test_build_content_base_host_header_invalid_port_omits_port() {
    let (event_sender, _) = tokio::sync::mpsc::unbounded_channel();
    let mock_io = MockNetIO::new();
    let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
    let remote_addr = "127.0.0.1:0".parse().unwrap();
    let session = RtspServerSession::new_with_io(
        session_io,
        event_sender,
        None,
        remote_addr,
        StreamingConfig::default(),
    );

    let mut request = create_test_request("DESCRIBE", Some("1"));
    request.uri.host = String::new();
    request.uri.port = None;
    request.uri.path = "live/stream".to_string();
    request
        .headers
        .insert("Host".to_string(), "host:99999".to_string());

    let base = session.build_content_base(&request);
    assert_eq!(base.as_deref(), Some("rtsp://host/live/stream/"));
}

#[test]
fn test_build_content_base_empty_host_no_host_header_non_empty_path_uses_path() {
    let (event_sender, _) = tokio::sync::mpsc::unbounded_channel();
    let mock_io = MockNetIO::new();
    let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
    let remote_addr = "127.0.0.1:0".parse().unwrap();
    let session = RtspServerSession::new_with_io(
        session_io,
        event_sender,
        None,
        remote_addr,
        StreamingConfig::default(),
    );

    let mut request = create_test_request("DESCRIBE", Some("1"));
    request.uri.host = String::new();
    request.uri.path = "live/stream".to_string();

    let base = session.build_content_base(&request);
    assert_eq!(base.as_deref(), Some("live/stream/"));
}

#[test]
fn test_build_content_base_with_port_none_omits_port_in_base() {
    let (event_sender, _) = tokio::sync::mpsc::unbounded_channel();
    let mock_io = MockNetIO::new();
    let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
    let remote_addr = "127.0.0.1:0".parse().unwrap();
    let session = RtspServerSession::new_with_io(
        session_io,
        event_sender,
        None,
        remote_addr,
        StreamingConfig::default(),
    );

    let mut request = create_test_request("DESCRIBE", Some("1"));
    request.uri.host = "192.168.1.1".to_string();
    request.uri.port = None;
    request.uri.path = "stream".to_string();

    let base = session.build_content_base(&request);
    assert_eq!(base.as_deref(), Some("rtsp://192.168.1.1/stream/"));
}

// ========================================================================
// normalize_rtsp_stream_path Tests
// ========================================================================

#[test]
fn test_normalize_rtsp_stream_path_trimmed_no_track_returns_trimmed() {
    let (event_sender, _) = tokio::sync::mpsc::unbounded_channel();
    let mock_io = MockNetIO::new();
    let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
    let remote_addr = "127.0.0.1:0".parse().unwrap();
    let session = RtspServerSession::new_with_io(
        session_io,
        event_sender,
        None,
        remote_addr,
        StreamingConfig::default(),
    );

    assert_eq!(
        session.normalize_rtsp_stream_path("/live/stream1/"),
        "live/stream1"
    );
    assert_eq!(
        session.normalize_rtsp_stream_path("live/stream1"),
        "live/stream1"
    );
}

#[test]
fn test_normalize_rtsp_stream_path_with_track_segment_returns_base() {
    let (event_sender, _) = tokio::sync::mpsc::unbounded_channel();
    let mock_io = MockNetIO::new();
    let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
    let remote_addr = "127.0.0.1:0".parse().unwrap();
    let session = RtspServerSession::new_with_io(
        session_io,
        event_sender,
        None,
        remote_addr,
        StreamingConfig::default(),
    );

    assert_eq!(
        session.normalize_rtsp_stream_path("live/stream1/trackID=0"),
        "live/stream1"
    );
    assert_eq!(
        session.normalize_rtsp_stream_path("live/stream1/Track1"),
        "live/stream1"
    );
}

#[test]
fn test_normalize_rtsp_stream_path_with_streamid_returns_base() {
    let (event_sender, _) = tokio::sync::mpsc::unbounded_channel();
    let mock_io = MockNetIO::new();
    let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
    let remote_addr = "127.0.0.1:0".parse().unwrap();
    let session = RtspServerSession::new_with_io(
        session_io,
        event_sender,
        None,
        remote_addr,
        StreamingConfig::default(),
    );

    assert_eq!(
        session.normalize_rtsp_stream_path("app/stream/streamid=0"),
        "app/stream"
    );
}

#[test]
fn test_normalize_rtsp_stream_path_empty_returns_empty() {
    let (event_sender, _) = tokio::sync::mpsc::unbounded_channel();
    let mock_io = MockNetIO::new();
    let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
    let remote_addr = "127.0.0.1:0".parse().unwrap();
    let session = RtspServerSession::new_with_io(
        session_io,
        event_sender,
        None,
        remote_addr,
        StreamingConfig::default(),
    );

    assert_eq!(session.normalize_rtsp_stream_path(""), "");
    assert_eq!(session.normalize_rtsp_stream_path("/"), "");
}

// ========================================================================
// RtspStreamHandler Tests
// ========================================================================

#[test]
fn test_rtsp_stream_handler_new() {
    let handler = RtspStreamHandler::new();
    // Handler should be created successfully
    assert!(std::mem::size_of_val(&handler) > 0);
}

#[test]
fn test_rtsp_stream_handler_default() {
    let handler = RtspStreamHandler::default();
    assert!(std::mem::size_of_val(&handler) > 0);
}

#[tokio::test]
async fn test_rtsp_stream_handler_set_sdp() {
    let handler = RtspStreamHandler::new();
    let sdp = Sdp::default();
    handler.set_sdp(sdp).await;
    // Should not panic
}

#[tokio::test]
async fn test_rtsp_stream_handler_send_information() {
    let handler = RtspStreamHandler::new();
    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();

    handler.send_information(sender).await;

    // Should receive SDP information
    if let Some(info) = receiver.recv().await {
        let Information::Sdp { data: _ } = info;
    } else {
        panic!("Expected to receive information");
    }
}

#[tokio::test]
async fn test_rtsp_stream_handler_get_statistic_data() {
    let handler = RtspStreamHandler::new();
    let stats = handler.get_statistic_data().await;
    assert!(stats.is_none());
}

// ========================================================================
// MockNetIO for Testing
// ========================================================================
use crate::io::NetType;
use crate::io::TNetIO;
use crate::io::bytesio_errors::BytesIOError;
use async_trait::async_trait;
use bytes::Bytes;
use mockall::mock;

mock! {
    pub NetIO {}
    #[async_trait]
    impl TNetIO for NetIO {
        fn get_net_type(&self) -> NetType;
        async fn read(&mut self) -> Result<BytesMut, BytesIOError>;
        async fn write(&mut self, bytes: Bytes) -> Result<(), BytesIOError>;
        async fn read_timeout(&mut self, duration: std::time::Duration) -> Result<BytesMut, BytesIOError>;
    }
}

#[tokio::test]
async fn test_rtsp_server_session_options() {
    let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();
    let mut mock_io = MockNetIO::new();

    // Expect get_net_type to be called
    mock_io.expect_get_net_type().returning(|| NetType::TCP);

    // Expect write logic for the response
    mock_io
        .expect_write()
        .withf(|bytes| {
            let s = std::str::from_utf8(bytes).unwrap();
            s.contains("RTSP/1.0 200 OK")
                && s.contains("Public: OPTIONS, DESCRIBE")
                && !s.contains("REDIRECT")
                && s.contains("Date:")
                && s.contains("Server: streaming-lib")
        })
        .times(1)
        .returning(|_| Ok(()));

    let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
    let remote_addr = "127.0.0.1:0".parse().unwrap();

    let mut session = RtspServerSession::new_with_io(
        session_io,
        event_sender,
        None,
        remote_addr,
        StreamingConfig::default(),
    );

    let request = create_test_request("OPTIONS", Some("1"));
    let result = session.handle_options(&request).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_rtsp_server_session_on_rtsp_message_leaves_interleaved_binary_buffered() {
    let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();
    let mut mock_io = MockNetIO::new();

    mock_io.expect_get_net_type().returning(|| NetType::TCP);
    mock_io.expect_read().times(0);

    mock_io
        .expect_write()
        .withf(|bytes| {
            let s = std::str::from_utf8(bytes).unwrap();
            s.contains("RTSP/1.0 200 OK")
                && s.contains("Public: OPTIONS, DESCRIBE")
                && !s.contains("REDIRECT")
                && s.contains("Date:")
                && s.contains("Server: streaming-lib")
        })
        .times(1)
        .returning(|_| Ok(()));

    let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
    let remote_addr = "127.0.0.1:0".parse().unwrap();
    let mut session = RtspServerSession::new_with_io(
        session_io,
        event_sender,
        None,
        remote_addr,
        StreamingConfig::default(),
    );

    let mut data = BytesMut::from("OPTIONS rtsp://localhost/stream1 RTSP/1.0\r\nCSeq: 1\r\n\r\n");
    data.extend_from_slice(&[0x24, 0x00, 0x00, 0x04, 0xff, 0xff, 0xff, 0xff]);
    session.reader.extend_from_slice(&data[..]);

    let result = session.on_rtsp_message().await;
    assert!(result.is_ok());

    let remaining = session.reader.get_remaining_bytes();
    assert_eq!(remaining.len(), 8);
    assert_eq!(remaining[0], 0x24);
}

#[tokio::test]
async fn test_rtsp_server_session_on_rtsp_message_unsupported_version_returns_505() {
    let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();
    let mut mock_io = MockNetIO::new();

    mock_io.expect_get_net_type().returning(|| NetType::TCP);
    mock_io.expect_read().times(0);
    mock_io
        .expect_write()
        .withf(|bytes| {
            let s = std::str::from_utf8(bytes).unwrap();
            s.contains("RTSP/1.0 505") && s.contains("CSeq: 1")
        })
        .times(1)
        .returning(|_| Ok(()));

    let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
    let remote_addr = "127.0.0.1:0".parse().unwrap();
    let mut session = RtspServerSession::new_with_io(
        session_io,
        event_sender,
        None,
        remote_addr,
        StreamingConfig::default(),
    );

    let data = BytesMut::from("OPTIONS rtsp://localhost/stream1 RTSP/2.0\r\nCSeq: 1\r\n\r\n");
    session.reader.extend_from_slice(&data[..]);

    let result = session.on_rtsp_message().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_rtsp_server_session_on_rtsp_message_content_length_body_mismatch_returns_400() {
    let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();
    let mut mock_io = MockNetIO::new();

    mock_io.expect_get_net_type().returning(|| NetType::TCP);
    mock_io.expect_read().times(0);
    mock_io
        .expect_write()
        .withf(|bytes| {
            let s = std::str::from_utf8(bytes).unwrap();
            s.contains("RTSP/1.0 400 Bad Request") && s.contains("CSeq: 1")
        })
        .times(1)
        .returning(|_| Ok(()));

    let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
    let remote_addr = "127.0.0.1:0".parse().unwrap();
    let mut session = RtspServerSession::new_with_io(
        session_io,
        event_sender,
        None,
        remote_addr,
        StreamingConfig::default(),
    );

    // First Content-Length controls framing (5 bytes body), second is used by header lookup (3)
    // so on_rtsp_message hits the mismatch branch deterministically.
    let data = BytesMut::from(
        "OPTIONS rtsp://localhost/stream1 RTSP/1.0\r\ncontent-length: 5\r\nContent-Length: 3\r\nCSeq: 1\r\n\r\nabcde",
    );
    session.reader.extend_from_slice(&data[..]);

    let result = session.on_rtsp_message().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_rtsp_server_session_on_rtsp_message_unknown_method_returns_501() {
    let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();
    let mut mock_io = MockNetIO::new();

    mock_io.expect_get_net_type().returning(|| NetType::TCP);
    mock_io.expect_read().times(0);
    mock_io
        .expect_write()
        .withf(|bytes| {
            let s = std::str::from_utf8(bytes).unwrap();
            s.contains("RTSP/1.0 501 Not Implemented") && s.contains("CSeq: 1")
        })
        .times(1)
        .returning(|_| Ok(()));

    let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
    let remote_addr = "127.0.0.1:0".parse().unwrap();
    let mut session = RtspServerSession::new_with_io(
        session_io,
        event_sender,
        None,
        remote_addr,
        StreamingConfig::default(),
    );

    let data = BytesMut::from("BREW rtsp://localhost/stream1 RTSP/1.0\r\nCSeq: 1\r\n\r\n");
    session.reader.extend_from_slice(&data[..]);

    let result = session.on_rtsp_message().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_rtsp_server_session_get_parameter_keep_alive_ok_includes_session() {
    let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();
    let mut mock_io = MockNetIO::new();

    mock_io.expect_get_net_type().returning(|| NetType::TCP);
    mock_io.expect_read().times(0);

    mock_io
        .expect_write()
        .withf(|bytes| {
            let s = std::str::from_utf8(bytes).unwrap();
            s.contains("RTSP/1.0 200 OK") && s.contains("Session:")
        })
        .times(1)
        .returning(|_| Ok(()));

    let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
    let remote_addr = "127.0.0.1:0".parse().unwrap();
    let mut session = RtspServerSession::new_with_io(
        session_io,
        event_sender,
        None,
        remote_addr,
        StreamingConfig::default(),
    );

    let session_id = Uuid::new(SESSION_ID_RANDOM_DIGITS);
    let session_id_str = session_id.to_string();
    session.session_id = Some(session_id);

    let mut request = create_test_request(rtsp_method_name::GET_PARAMETER, Some("1"));
    request
        .headers
        .insert("Session".to_string(), session_id_str);
    let result = session.handle_get_parameter(&request).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_rtsp_server_session_get_parameter_wrong_session_returns_454() {
    let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();
    let mut mock_io = MockNetIO::new();

    mock_io.expect_get_net_type().returning(|| NetType::TCP);
    mock_io.expect_read().times(0);

    mock_io
        .expect_write()
        .withf(|bytes| {
            let s = std::str::from_utf8(bytes).unwrap();
            s.contains("RTSP/1.0 454 Session Not Found") && s.contains("CSeq: 1")
        })
        .times(1)
        .returning(|_| Ok(()));

    let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
    let remote_addr = "127.0.0.1:0".parse().unwrap();
    let mut session = RtspServerSession::new_with_io(
        session_io,
        event_sender,
        None,
        remote_addr,
        StreamingConfig::default(),
    );

    session.session_id = Some(Uuid::new(SESSION_ID_RANDOM_DIGITS));

    let mut request = create_test_request(rtsp_method_name::GET_PARAMETER, Some("1"));
    request
        .headers
        .insert("Session".to_string(), "does-not-exist".to_string());
    let result = session.handle_get_parameter(&request).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_rtsp_server_session_set_parameter_success_returns_200() {
    let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();
    let mut mock_io = MockNetIO::new();

    mock_io.expect_get_net_type().returning(|| NetType::TCP);
    mock_io.expect_read().times(0);
    mock_io
        .expect_write()
        .withf(|bytes| {
            let s = std::str::from_utf8(bytes).unwrap();
            s.contains("RTSP/1.0 200 OK") && s.contains("CSeq: 2") && s.contains("Session:")
        })
        .times(1)
        .returning(|_| Ok(()));

    let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
    let remote_addr = "127.0.0.1:0".parse().unwrap();
    let mut session = RtspServerSession::new_with_io(
        session_io,
        event_sender,
        None,
        remote_addr,
        StreamingConfig::default(),
    );

    let session_id = Uuid::new(SESSION_ID_RANDOM_DIGITS);
    session.session_id = Some(session_id);

    let mut request = create_test_request(rtsp_method_name::SET_PARAMETER, Some("2"));
    request
        .headers
        .insert("Session".to_string(), session_id.to_string());
    let result = session.handle_set_parameter(&request).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_rtsp_server_session_set_parameter_wrong_session_returns_454() {
    let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();
    let mut mock_io = MockNetIO::new();

    mock_io.expect_get_net_type().returning(|| NetType::TCP);
    mock_io.expect_read().times(0);
    mock_io
        .expect_write()
        .withf(|bytes| {
            let s = std::str::from_utf8(bytes).unwrap();
            s.contains("RTSP/1.0 454 Session Not Found") && s.contains("CSeq: 2")
        })
        .times(1)
        .returning(|_| Ok(()));

    let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
    let remote_addr = "127.0.0.1:0".parse().unwrap();
    let mut session = RtspServerSession::new_with_io(
        session_io,
        event_sender,
        None,
        remote_addr,
        StreamingConfig::default(),
    );

    session.session_id = Some(Uuid::new(SESSION_ID_RANDOM_DIGITS));

    let mut request = create_test_request(rtsp_method_name::SET_PARAMETER, Some("2"));
    request
        .headers
        .insert("Session".to_string(), "wrong-session-id".to_string());
    let result = session.handle_set_parameter(&request).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_rtsp_server_session_set_parameter_no_session_header_returns_200() {
    let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();
    let mut mock_io = MockNetIO::new();

    mock_io.expect_get_net_type().returning(|| NetType::TCP);
    mock_io.expect_read().times(0);
    mock_io
        .expect_write()
        .withf(|bytes| {
            let s = std::str::from_utf8(bytes).unwrap();
            s.contains("RTSP/1.0 200 OK") && s.contains("CSeq: 2")
        })
        .times(1)
        .returning(|_| Ok(()));

    let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
    let remote_addr = "127.0.0.1:0".parse().unwrap();
    let mut session = RtspServerSession::new_with_io(
        session_io,
        event_sender,
        None,
        remote_addr,
        StreamingConfig::default(),
    );

    session.session_id = Some(Uuid::new(SESSION_ID_RANDOM_DIGITS));
    let request = create_test_request(rtsp_method_name::SET_PARAMETER, Some("2"));

    let result = session.handle_set_parameter(&request).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_rtsp_server_session_set_parameter_no_session_id_returns_200() {
    let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();
    let mut mock_io = MockNetIO::new();

    mock_io.expect_get_net_type().returning(|| NetType::TCP);
    mock_io.expect_read().times(0);
    mock_io
        .expect_write()
        .withf(|bytes| {
            let s = std::str::from_utf8(bytes).unwrap();
            s.contains("RTSP/1.0 200 OK") && s.contains("CSeq: 2")
        })
        .times(1)
        .returning(|_| Ok(()));

    let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
    let remote_addr = "127.0.0.1:0".parse().unwrap();
    let mut session = RtspServerSession::new_with_io(
        session_io,
        event_sender,
        None,
        remote_addr,
        StreamingConfig::default(),
    );

    assert!(session.session_id.is_none());
    let request = create_test_request(rtsp_method_name::SET_PARAMETER, Some("2"));

    let result = session.handle_set_parameter(&request).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_rtsp_server_session_pause_unsubscribes_and_responds_ok() {
    let (event_sender, mut event_receiver) = tokio::sync::mpsc::unbounded_channel();
    let mut mock_io = MockNetIO::new();

    mock_io.expect_get_net_type().returning(|| NetType::TCP);
    mock_io.expect_read().times(0);

    mock_io
        .expect_write()
        .withf(|bytes| {
            let s = std::str::from_utf8(bytes).unwrap();
            s.contains("RTSP/1.0 200 OK") && s.contains("CSeq: 1")
        })
        .times(1)
        .returning(|_| Ok(()));

    let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
    let remote_addr = "127.0.0.1:0".parse().unwrap();
    let mut session = RtspServerSession::new_with_io(
        session_io,
        event_sender,
        None,
        remote_addr,
        StreamingConfig::default(),
    );

    session.has_subscribed = true;
    session.session_id = Some(Uuid::new(SESSION_ID_RANDOM_DIGITS));
    session.stream_identifier = Some(StreamIdentifier::Rtsp {
        stream_path: "live/stream1".to_string(),
    });

    let mut request = create_test_request(rtsp_method_name::PAUSE, Some("1"));
    request.uri.path = "live/stream1".to_string();

    let result = session.handle_pause(&request).await;
    assert!(result.is_ok());
    assert!(!session.has_subscribed);

    let event = event_receiver.recv().await.expect("expected unsubscribe");
    match event {
        StreamHubEvent::UnSubscribe { identifier, .. } => match identifier {
            StreamIdentifier::Rtsp { stream_path } => {
                assert_eq!(stream_path, "live/stream1");
            }
            _ => panic!("unexpected identifier"),
        },
        _ => panic!("expected UnSubscribe event"),
    }
}

#[tokio::test]
async fn test_rtsp_server_session_pause_event_send_failure_returns_err() {
    let (event_sender, event_receiver) = tokio::sync::mpsc::unbounded_channel();
    drop(event_receiver);

    let mut mock_io = MockNetIO::new();
    mock_io.expect_get_net_type().returning(|| NetType::TCP);
    mock_io.expect_read().times(0);

    let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
    let remote_addr = "127.0.0.1:0".parse().unwrap();
    let mut session = RtspServerSession::new_with_io(
        session_io,
        event_sender,
        None,
        remote_addr,
        StreamingConfig::default(),
    );

    session.has_subscribed = true;
    session.stream_identifier = Some(StreamIdentifier::Rtsp {
        stream_path: "live/stream1".to_string(),
    });

    let mut request = create_test_request(rtsp_method_name::PAUSE, Some("1"));
    request.uri.path = "live/stream1".to_string();

    let result = session.handle_pause(&request).await;
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(matches!(e.value, SessionErrorValue::StreamHubEventSendErr));
    }
}

#[tokio::test]
async fn test_rtsp_server_session_redirect_returns_405_with_allow() {
    let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();
    let mut mock_io = MockNetIO::new();

    mock_io.expect_get_net_type().returning(|| NetType::TCP);
    mock_io.expect_read().times(0);

    mock_io
        .expect_write()
        .withf(|bytes| {
            let s = std::str::from_utf8(bytes).unwrap();
            s.contains("RTSP/1.0 405 Method Not Allowed") && s.contains("Allow:")
        })
        .times(1)
        .returning(|_| Ok(()));

    let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
    let remote_addr = "127.0.0.1:0".parse().unwrap();
    let mut session = RtspServerSession::new_with_io(
        session_io,
        event_sender,
        None,
        remote_addr,
        StreamingConfig::default(),
    );

    let request = create_test_request(rtsp_method_name::REDIRECT, Some("1"));
    let result = session.handle_redirect(&request).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_rtsp_server_session_describe() {
    let (event_sender, mut event_receiver) = tokio::sync::mpsc::unbounded_channel();
    let mut mock_io = MockNetIO::new();

    mock_io.expect_get_net_type().returning(|| NetType::TCP);
    mock_io
        .expect_write()
        .withf(|bytes| {
            let s = std::str::from_utf8(bytes).unwrap();
            s.contains("RTSP/1.0 200 OK")
                && s.contains("application/sdp")
                && s.contains("Content-Base: rtsp://127.0.0.1:8554/live/test/")
        })
        .times(1)
        .returning(|_| Ok(()));

    // Start a mock StreamHub event loop to handle the Request event
    tokio::spawn(async move {
        if let Some(event) = event_receiver.recv().await
            && let StreamHubEvent::Request {
                identifier: _,
                sender,
            } = event
        {
            // Respond with a minimal valid SDP containing one media block
            let dummy_sdp = "v=0\r\no=- 0 0 IN IP4 127.0.0.1\r\ns=No Name\r\nc=IN IP4 127.0.0.1\r\nt=0 0\r\nm=video 0 RTP/AVP 96\r\na=rtpmap:96 H264/90000\r\n";
            let _ = sender.send(Information::Sdp {
                data: dummy_sdp.to_string(),
            });
        }
    });

    let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
    let remote_addr = "127.0.0.1:0".parse().unwrap();

    let mut session = RtspServerSession::new_with_io(
        session_io,
        event_sender,
        None,
        remote_addr,
        StreamingConfig::default(),
    );

    let mut request = create_test_request("DESCRIBE", Some("2"));
    // Need to set a valid path for StreamIdentifier
    request.uri.schema = crate::common::http::Schema::RTSP;
    request.uri.host = "127.0.0.1".to_string();
    request.uri.port = Some(8554);
    request.uri.path = "live/test".to_string();

    let result = session.handle_describe(&request).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_rtsp_server_session_describe_unacceptable_accept_returns_406() {
    let (event_sender, mut event_receiver) = tokio::sync::mpsc::unbounded_channel();
    let mut mock_io = MockNetIO::new();

    mock_io.expect_get_net_type().returning(|| NetType::TCP);
    mock_io.expect_read().times(0);
    mock_io
        .expect_write()
        .withf(|bytes| {
            let s = std::str::from_utf8(bytes).unwrap();
            s.contains("RTSP/1.0 406 Not Acceptable") && s.contains("CSeq: 2")
        })
        .times(1)
        .returning(|_| Ok(()));

    let event_handle = tokio::spawn(async move {
        if let Some(StreamHubEvent::Request { sender, .. }) = event_receiver.recv().await {
            let dummy_sdp = "v=0\r\no=- 0 0 IN IP4 127.0.0.1\r\ns=No Name\r\nc=IN IP4 127.0.0.1\r\nt=0 0\r\nm=video 0 RTP/AVP 96\r\na=rtpmap:96 H264/90000\r\n";
            let _ = sender.send(Information::Sdp {
                data: dummy_sdp.to_string(),
            });
        }
    });

    let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
    let remote_addr = "127.0.0.1:0".parse().unwrap();

    let mut session = RtspServerSession::new_with_io(
        session_io,
        event_sender,
        None,
        remote_addr,
        StreamingConfig::default(),
    );

    let mut request = create_test_request("DESCRIBE", Some("2"));
    request.uri.path = "live/test".to_string();
    request
        .headers
        .insert("Accept".to_string(), "application/json".to_string());

    let result = session.handle_describe(&request).await;
    assert!(result.is_ok());
    event_handle.await.expect("event task panicked");
}

#[tokio::test]
async fn test_rtsp_server_session_describe_normalizes_path() {
    let (event_sender, mut event_receiver) = tokio::sync::mpsc::unbounded_channel();
    let mut mock_io = MockNetIO::new();

    mock_io.expect_get_net_type().returning(|| NetType::TCP);
    mock_io
        .expect_write()
        .withf(|bytes| {
            let s = std::str::from_utf8(bytes).unwrap();
            s.contains("RTSP/1.0 200 OK")
                && s.contains("application/sdp")
                && s.contains("Content-Base: rtsp://127.0.0.1:8554/live/test/")
        })
        .times(1)
        .returning(|_| Ok(()));

    tokio::spawn(async move {
        if let Some(event) = event_receiver.recv().await
            && let StreamHubEvent::Request { identifier, sender } = event
        {
            match identifier {
                StreamIdentifier::Rtsp { stream_path } => {
                    assert_eq!(stream_path, "live/test");
                }
                _ => panic!("unexpected identifier type"),
            }
            let dummy_sdp = "v=0\r\no=- 0 0 IN IP4 127.0.0.1\r\ns=No Name\r\nc=IN IP4 127.0.0.1\r\nt=0 0\r\nm=video 0 RTP/AVP 96\r\na=rtpmap:96 H264/90000\r\n";
            let _ = sender.send(Information::Sdp {
                data: dummy_sdp.to_string(),
            });
        }
    });

    let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
    let remote_addr = "127.0.0.1:0".parse().unwrap();
    let mut session = RtspServerSession::new_with_io(
        session_io,
        event_sender,
        None,
        remote_addr,
        StreamingConfig::default(),
    );

    let mut request = create_test_request("DESCRIBE", Some("3"));
    request.uri.schema = crate::common::http::Schema::RTSP;
    request.uri.host = "127.0.0.1".to_string();
    request.uri.port = Some(8554);
    request.uri.path = "live/test/trackID=0".to_string();

    let result = session.handle_describe(&request).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_rtsp_server_session_describe_empty_sdp_returns_not_found() {
    let (event_sender, mut event_receiver) = tokio::sync::mpsc::unbounded_channel();
    let mut mock_io = MockNetIO::new();

    mock_io.expect_get_net_type().returning(|| NetType::TCP);
    mock_io
        .expect_write()
        .withf(|bytes| {
            let s = std::str::from_utf8(bytes).unwrap();
            s.contains("RTSP/1.0 404 Not Found")
        })
        .times(1)
        .returning(|_| Ok(()));

    tokio::spawn(async move {
        if let Some(StreamHubEvent::Request { sender, .. }) = event_receiver.recv().await {
            // No media blocks -> server should treat as not-found stream.
            let empty_sdp =
                "v=0\r\no=- 0 0 IN IP4 127.0.0.1\r\ns=No Name\r\nc=IN IP4 127.0.0.1\r\nt=0 0\r\n";
            let _ = sender.send(Information::Sdp {
                data: empty_sdp.to_string(),
            });
        }
    });

    let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
    let remote_addr = "127.0.0.1:0".parse().unwrap();
    let mut session = RtspServerSession::new_with_io(
        session_io,
        event_sender,
        None,
        remote_addr,
        StreamingConfig::default(),
    );

    let mut request = create_test_request("DESCRIBE", Some("4"));
    request.uri.schema = crate::common::http::Schema::RTSP;
    request.uri.host = "127.0.0.1".to_string();
    request.uri.port = Some(8554);
    request.uri.path = "bogus_stream".to_string();

    let result = session.handle_describe(&request).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_rtsp_server_session_describe_auth_header_without_auth_returns_unauthorized() {
    let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();
    let mut mock_io = MockNetIO::new();

    mock_io.expect_get_net_type().returning(|| NetType::TCP);
    mock_io
        .expect_write()
        .withf(|bytes| {
            let s = std::str::from_utf8(bytes).unwrap();
            s.contains("RTSP/1.0 401 Unauthorized")
        })
        .times(1)
        .returning(|_| Ok(()));

    let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
    let remote_addr = "127.0.0.1:0".parse().unwrap();
    let mut session = RtspServerSession::new_with_io(
        session_io,
        event_sender,
        None,
        remote_addr,
        StreamingConfig::default(),
    );

    let mut request = create_test_request("DESCRIBE", Some("5"));
    request.uri.schema = crate::common::http::Schema::RTSP;
    request.uri.host = "127.0.0.1".to_string();
    request.uri.port = Some(8554);
    request.uri.path = "stream1".to_string();
    request.headers.insert(
        "Authorization".to_string(),
        "Basic aW52YWxpZDppbnZhbGlk".to_string(),
    );

    let result = session.handle_describe(&request).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_rtsp_server_session_describe_with_basic_auth_returns_ok() {
    use crate::common::auth::{AuthAlgorithm, AuthType};

    let (event_sender, mut event_receiver) = tokio::sync::mpsc::unbounded_channel();
    let mut mock_io = MockNetIO::new();

    mock_io.expect_get_net_type().returning(|| NetType::TCP);
    mock_io
        .expect_write()
        .withf(|bytes| {
            let s = std::str::from_utf8(bytes).unwrap();
            s.contains("RTSP/1.0 200 OK")
                && s.contains("application/sdp")
                && s.contains("Content-Base: rtsp://127.0.0.1:8554/live/test/")
        })
        .times(1)
        .returning(|_| Ok(()));

    tokio::spawn(async move {
        if let Some(StreamHubEvent::Request { sender, .. }) = event_receiver.recv().await {
            let dummy_sdp = "v=0\r\no=- 0 0 IN IP4 127.0.0.1\r\ns=No Name\r\nc=IN IP4 127.0.0.1\r\nt=0 0\r\nm=video 0 RTP/AVP 96\r\na=rtpmap:96 H264/90000\r\n";
            let _ = sender.send(Information::Sdp {
                data: dummy_sdp.to_string(),
            });
        }
    });

    let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
    let remote_addr = "127.0.0.1:0".parse().unwrap();
    let auth = Auth::new(
        "key".to_string(),
        "unused".to_string(),
        None,
        AuthAlgorithm::Simple,
        AuthType::Pull,
    )
    .with_credential_validator(Arc::new(|username, password| {
        username == "admin" && password == "secret"
    }))
    .with_basic_realm("ONVIF Camera");
    let mut session = RtspServerSession::new_with_io(
        session_io,
        event_sender,
        Some(auth),
        remote_addr,
        StreamingConfig::default(),
    );

    let mut request = create_test_request("DESCRIBE", Some("6"));
    request.uri.schema = crate::common::http::Schema::RTSP;
    request.uri.host = "127.0.0.1".to_string();
    request.uri.port = Some(8554);
    request.uri.path = "live/test/trackID=0".to_string();
    request.headers.insert(
        "Authorization".to_string(),
        "Basic YWRtaW46c2VjcmV0".to_string(),
    );

    let result = session.handle_describe(&request).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_rtsp_server_session_describe_auth_failure_returns_challenge() {
    use crate::common::auth::{AuthAlgorithm, AuthType};

    let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();
    let mut mock_io = MockNetIO::new();

    mock_io.expect_get_net_type().returning(|| NetType::TCP);
    mock_io
        .expect_write()
        .withf(|bytes| {
            let s = std::str::from_utf8(bytes).unwrap();
            s.contains("RTSP/1.0 401 Unauthorized")
                && s.contains("WWW-Authenticate: Basic realm=\"ONVIF Camera\"")
        })
        .times(1)
        .returning(|_| Ok(()));

    let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
    let remote_addr = "127.0.0.1:0".parse().unwrap();
    let auth = Auth::new(
        "key".to_string(),
        "unused".to_string(),
        None,
        AuthAlgorithm::Simple,
        AuthType::Pull,
    )
    .with_credential_validator(Arc::new(|username, password| {
        username == "admin" && password == "secret"
    }))
    .with_basic_realm("ONVIF Camera");
    let mut session = RtspServerSession::new_with_io(
        session_io,
        event_sender,
        Some(auth),
        remote_addr,
        StreamingConfig::default(),
    );

    let mut request = create_test_request("DESCRIBE", Some("7"));
    request.uri.schema = crate::common::http::Schema::RTSP;
    request.uri.host = "127.0.0.1".to_string();
    request.uri.port = Some(8554);
    request.uri.path = "stream1".to_string();

    let result = session.handle_describe(&request).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_rtsp_server_session_play_auth_failure_returns_challenge() {
    use crate::common::auth::{AuthAlgorithm, AuthType};

    let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();
    let mut mock_io = MockNetIO::new();

    mock_io.expect_get_net_type().returning(|| NetType::TCP);
    mock_io
        .expect_write()
        .withf(|bytes| {
            let s = std::str::from_utf8(bytes).unwrap();
            s.contains("RTSP/1.0 401 Unauthorized")
                && s.contains("WWW-Authenticate: Basic realm=\"ONVIF Camera\"")
        })
        .times(1)
        .returning(|_| Ok(()));

    let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
    let remote_addr = "127.0.0.1:0".parse().unwrap();
    let auth = Auth::new(
        "key".to_string(),
        "unused".to_string(),
        None,
        AuthAlgorithm::Simple,
        AuthType::Pull,
    )
    .with_credential_validator(Arc::new(|username, password| {
        username == "admin" && password == "secret"
    }))
    .with_basic_realm("ONVIF Camera");
    let mut session = RtspServerSession::new_with_io(
        session_io,
        event_sender,
        Some(auth),
        remote_addr,
        StreamingConfig::default(),
    );

    let mut request = create_test_request(rtsp_method_name::PLAY, Some("8"));
    request.uri.path = "stream1".to_string();

    let result = session.handle_play(&request).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_rtsp_server_session_setup() {
    let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();
    let mut mock_io = MockNetIO::new();

    mock_io.expect_get_net_type().returning(|| NetType::TCP);
    mock_io
        .expect_write()
        .withf(|bytes| {
            let s = std::str::from_utf8(bytes).unwrap();
            s.contains("RTSP/1.0 200 OK") && s.contains("Transport")
        })
        .times(1)
        .returning(|_| Ok(()));

    let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
    let remote_addr = "127.0.0.1:0".parse().unwrap();

    let mut session = RtspServerSession::new_with_io(
        session_io,
        event_sender,
        None,
        remote_addr,
        StreamingConfig::default(),
    );

    // Pre-populate a track so setup acts on it
    // The track logic requires an existing track in self.tracks map matching the control URI
    let codec_info = RtspCodecInfo {
        codec_id: crate::protocol::rtsp::rtsp_codec::RtspCodecId::H264,
        payload_type: 96,
        sample_rate: 90000,
        channel_count: 0,
    };
    let track = RtspTrack::new(TrackType::Video, codec_info, "trackID=0".to_string());
    session.tracks.insert(TrackType::Video, track);

    let content = "SETUP rtsp://localhost/live/test/trackID=0 RTSP/1.0\r\nCSeq: 3\r\nTransport: RTP/AVP/TCP;unicast;interleaved=0-1\r\n\r\n";
    let request = RtspRequest::unmarshal(content).unwrap();

    let result = session.handle_setup(&request).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_rtsp_server_session_setup_malformed_transport_returns_461() {
    let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();
    let mut mock_io = MockNetIO::new();

    mock_io.expect_get_net_type().returning(|| NetType::TCP);
    mock_io.expect_read().times(0);
    mock_io
        .expect_write()
        .withf(|bytes| {
            let s = std::str::from_utf8(bytes).unwrap();
            s.contains("RTSP/1.0 461") && s.contains("CSeq: 3")
        })
        .times(1)
        .returning(|_| Ok(()));

    let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
    let remote_addr = "127.0.0.1:0".parse().unwrap();

    let mut session = RtspServerSession::new_with_io(
        session_io,
        event_sender,
        None,
        remote_addr,
        StreamingConfig::default(),
    );

    let codec_info = RtspCodecInfo {
        codec_id: crate::protocol::rtsp::rtsp_codec::RtspCodecId::H264,
        payload_type: 96,
        sample_rate: 90000,
        channel_count: 0,
    };
    let track = RtspTrack::new(TrackType::Video, codec_info, "trackID=0".to_string());
    session.tracks.insert(TrackType::Video, track);

    let content = "SETUP rtsp://localhost/live/test/trackID=0 RTSP/1.0\r\nCSeq: 3\r\nTransport: RTP/AVP/TCP;unicast\r\n\r\n";
    let request = RtspRequest::unmarshal(content).unwrap();

    let result = session.handle_setup(&request).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_rtsp_server_session_setup_base_path_selects_video_track() {
    let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();
    let mut mock_io = MockNetIO::new();

    mock_io.expect_get_net_type().returning(|| NetType::TCP);
    mock_io
        .expect_write()
        .withf(|bytes| {
            let s = std::str::from_utf8(bytes).unwrap();
            s.contains("RTSP/1.0 200 OK") && s.contains("Transport") && s.contains("Session")
        })
        .times(1)
        .returning(|_| Ok(()));

    let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
    let remote_addr = "127.0.0.1:0".parse().unwrap();

    let mut session = RtspServerSession::new_with_io(
        session_io,
        event_sender,
        None,
        remote_addr,
        StreamingConfig::default(),
    );

    let codec_info = RtspCodecInfo {
        codec_id: crate::protocol::rtsp::rtsp_codec::RtspCodecId::H264,
        payload_type: 96,
        sample_rate: 90000,
        channel_count: 0,
    };
    let track = RtspTrack::new(TrackType::Video, codec_info, "trackID=0".to_string());
    session.tracks.insert(TrackType::Video, track);

    let content = "SETUP rtsp://localhost/stream1 RTSP/1.0\r\nCSeq: 3\r\nTransport: RTP/AVP/TCP;unicast;interleaved=0-1\r\n\r\n";
    let request = RtspRequest::unmarshal(content).unwrap();

    let result = session.handle_setup(&request).await;
    assert!(result.is_ok());
    assert_eq!(
        session.stream_identifier,
        Some(StreamIdentifier::Rtsp {
            stream_path: "stream1".to_string(),
        })
    );
}

#[tokio::test]
async fn test_rtsp_server_session_teardown() {
    let (event_sender, mut event_receiver) = tokio::sync::mpsc::unbounded_channel();
    let mock_io = MockNetIO::new();

    let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
    let remote_addr = "127.0.0.1:0".parse().unwrap();

    let mut session = RtspServerSession::new_with_io(
        session_io,
        event_sender,
        None,
        remote_addr,
        StreamingConfig::default(),
    );

    // Mock event receiver to handle UnSubscribe/UnPublish
    tokio::spawn(async move {
        if let Some(_event) = event_receiver.recv().await {
            // Just consume the event
        }
    });

    let request = create_test_request("TEARDOWN", Some("4"));
    let result = session.handle_teardown(&request);
    assert!(result.is_ok());
    assert!(session.is_normal_exit);
}

#[tokio::test]
async fn test_rtsp_server_session_teardown_with_rtp_counters_exercises_summary_branch() {
    let (event_sender, mut event_receiver) = tokio::sync::mpsc::unbounded_channel();
    let mock_io = MockNetIO::new();

    let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
    let remote_addr = "127.0.0.1:0".parse().unwrap();

    let mut session = RtspServerSession::new_with_io(
        session_io,
        event_sender,
        None,
        remote_addr,
        StreamingConfig::default(),
    );

    let counter = Arc::new(RtpTrackCounters::new());
    counter.on_packet_sent(100, 1, 1000);
    counter.on_packet_sent(100, 2, 2000);
    session.rtp_counters.insert(TrackType::Video, counter);

    tokio::spawn(async move {
        if let Some(_event) = event_receiver.recv().await {
            // Consume the event
        }
    });

    let mut request = create_test_request("TEARDOWN", Some("5"));
    request.uri.path = "stream1".to_string();
    let result = session.handle_teardown(&request);
    assert!(result.is_ok());
}

/// Drives the TEARDOWN branch in on_rtsp_message via run() to assert the server sends RTSP 200 OK.
#[tokio::test]
async fn test_rtsp_server_session_teardown_sends_response() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    const TEARDOWN_REQ: &str =
        "TEARDOWN rtsp://localhost/stream1 RTSP/1.0\r\nCSeq: 4\r\nSession: 1\r\n\r\n";

    let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();
    let mut mock_io = MockNetIO::new();

    mock_io.expect_get_net_type().returning(|| NetType::TCP);

    let read_count = AtomicUsize::new(0);
    let teardown_bytes = BytesMut::from(TEARDOWN_REQ);
    mock_io.expect_read().times(2).returning(move || {
        if read_count.fetch_add(1, Ordering::SeqCst) == 0 {
            Ok(teardown_bytes.clone())
        } else {
            Err(BytesIOError::from(std::io::Error::new(
                std::io::ErrorKind::ConnectionAborted,
                "eof",
            )))
        }
    });

    mock_io
        .expect_write()
        .withf(|bytes| {
            let s = std::str::from_utf8(bytes).unwrap();
            s.contains("RTSP/1.0 200 OK")
        })
        .times(1)
        .returning(|_| Ok(()));

    let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
    let remote_addr = "127.0.0.1:0".parse().unwrap();

    let mut session = RtspServerSession::new_with_io(
        session_io,
        event_sender,
        None,
        remote_addr,
        StreamingConfig::default(),
    );

    let run_handle = tokio::spawn(async move {
        let _ = session.run().await;
    });

    run_handle.await.expect("run task panicked");
}

#[tokio::test]
async fn test_rtsp_server_session_play_then_teardown_sends_two_responses_and_unsubscribes_once() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    const PLAY_REQ: &str = "PLAY rtsp://localhost/stream1/trackID=0 RTSP/1.0\r\nCSeq: 5\r\nSession: 1\r\nRange: npt=0.000-\r\n\r\n";
    const TEARDOWN_REQ: &str =
        "TEARDOWN rtsp://localhost/stream1 RTSP/1.0\r\nCSeq: 6\r\nSession: 1\r\n\r\n";

    let (event_sender, mut event_receiver) = tokio::sync::mpsc::unbounded_channel();
    let mut mock_io = MockNetIO::new();

    mock_io.expect_get_net_type().returning(|| NetType::TCP);

    let read_count = AtomicUsize::new(0);
    let play_bytes = BytesMut::from(PLAY_REQ);
    let teardown_bytes = BytesMut::from(TEARDOWN_REQ);
    mock_io.expect_read().times(3).returning(move || {
        match read_count.fetch_add(1, Ordering::SeqCst) {
            0 => Ok(play_bytes.clone()),
            1 => Ok(teardown_bytes.clone()),
            _ => Err(BytesIOError::from(std::io::Error::new(
                std::io::ErrorKind::ConnectionAborted,
                "eof",
            ))),
        }
    });

    mock_io
        .expect_write()
        .withf(|bytes| {
            let s = std::str::from_utf8(bytes).unwrap();
            s.contains("RTSP/1.0 200 OK")
        })
        .times(2)
        .returning(|_| Ok(()));

    let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
    let remote_addr = "127.0.0.1:0".parse().unwrap();

    let unsubscribe_count = Arc::new(AtomicUsize::new(0));
    let unsubscribe_count_for_task = unsubscribe_count.clone();
    let event_handle = tokio::spawn(async move {
        use crate::hub::define::DataReceiver;

        let mut held_frame_sender = None;
        while let Some(event) = event_receiver.recv().await {
            match event {
                StreamHubEvent::Subscribe { result_sender, .. } => {
                    let (frame_sender, frame_receiver) = crate::hub::define::frame_data_channel();
                    held_frame_sender = Some(frame_sender);
                    let data_receiver = DataReceiver {
                        frame_receiver: Some(frame_receiver),
                        packet_receiver: None,
                    };
                    let _ = result_sender.send(Ok((data_receiver, None)));
                }
                StreamHubEvent::UnSubscribe { .. } => {
                    unsubscribe_count_for_task.fetch_add(1, Ordering::SeqCst);
                    drop(held_frame_sender);
                    break;
                }
                _ => {}
            }
        }
    });

    let mut session = RtspServerSession::new_with_io(
        session_io,
        event_sender,
        None,
        remote_addr,
        StreamingConfig::default(),
    );
    add_default_video_track(&mut session);

    let run_result = session.run().await;
    assert!(run_result.is_err());
    assert!(session.playback_task.is_none());
    assert!(session.playback_cancel.is_none());

    event_handle.await.expect("event task panicked");
    assert_eq!(unsubscribe_count.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn test_rtsp_server_session_teardown_trailing_slash_unsubscribes_normalized_stream() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    const PLAY_REQ: &str = "PLAY rtsp://localhost/stream1/trackID=0 RTSP/1.0\r\nCSeq: 5\r\nSession: 1\r\nRange: npt=0.000-\r\n\r\n";
    const TEARDOWN_REQ: &str =
        "TEARDOWN rtsp://localhost/stream1/ RTSP/1.0\r\nCSeq: 6\r\nSession: 1\r\n\r\n";

    let (event_sender, mut event_receiver) = tokio::sync::mpsc::unbounded_channel();
    let mut mock_io = MockNetIO::new();

    mock_io.expect_get_net_type().returning(|| NetType::TCP);

    let read_count = AtomicUsize::new(0);
    let play_bytes = BytesMut::from(PLAY_REQ);
    let teardown_bytes = BytesMut::from(TEARDOWN_REQ);
    mock_io.expect_read().times(3).returning(move || {
        match read_count.fetch_add(1, Ordering::SeqCst) {
            0 => Ok(play_bytes.clone()),
            1 => Ok(teardown_bytes.clone()),
            _ => Err(BytesIOError::from(std::io::Error::new(
                std::io::ErrorKind::ConnectionAborted,
                "eof",
            ))),
        }
    });

    mock_io.expect_write().times(2).returning(|_| Ok(()));

    let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
    let remote_addr = "127.0.0.1:0".parse().unwrap();

    let event_handle = tokio::spawn(async move {
        use crate::hub::define::DataReceiver;

        let mut held_frame_sender = None;
        while let Some(event) = event_receiver.recv().await {
            match event {
                StreamHubEvent::Subscribe { result_sender, .. } => {
                    let (frame_sender, frame_receiver) = crate::hub::define::frame_data_channel();
                    held_frame_sender = Some(frame_sender);
                    let data_receiver = DataReceiver {
                        frame_receiver: Some(frame_receiver),
                        packet_receiver: None,
                    };
                    let _ = result_sender.send(Ok((data_receiver, None)));
                }
                StreamHubEvent::UnSubscribe { identifier, .. } => {
                    match identifier {
                        StreamIdentifier::Rtsp { stream_path } => {
                            assert_eq!(stream_path, "stream1");
                        }
                        _ => panic!("Expected RTSP identifier"),
                    }
                    drop(held_frame_sender);
                    break;
                }
                _ => {}
            }
        }
    });

    let mut session = RtspServerSession::new_with_io(
        session_io,
        event_sender,
        None,
        remote_addr,
        StreamingConfig::default(),
    );
    add_default_video_track(&mut session);

    let run_result = session.run().await;
    assert!(run_result.is_err());
    event_handle.await.expect("event task panicked");
}

#[tokio::test]
async fn test_rtsp_server_session_run_eof_stops_playback_task() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    const PLAY_REQ: &str = "PLAY rtsp://localhost/stream1/trackID=0 RTSP/1.0\r\nCSeq: 5\r\nSession: 1\r\nRange: npt=0.000-\r\n\r\n";

    let (event_sender, mut event_receiver) = tokio::sync::mpsc::unbounded_channel();
    let mut mock_io = MockNetIO::new();

    mock_io.expect_get_net_type().returning(|| NetType::TCP);

    let read_count = AtomicUsize::new(0);
    let play_bytes = BytesMut::from(PLAY_REQ);
    mock_io.expect_read().times(2).returning(move || {
        if read_count.fetch_add(1, Ordering::SeqCst) == 0 {
            Ok(play_bytes.clone())
        } else {
            Err(BytesIOError::from(std::io::Error::new(
                std::io::ErrorKind::ConnectionAborted,
                "eof",
            )))
        }
    });

    mock_io
        .expect_write()
        .withf(|bytes| {
            let s = std::str::from_utf8(bytes).unwrap();
            s.contains("RTSP/1.0 200 OK")
        })
        .times(1)
        .returning(|_| Ok(()));

    let event_handle = tokio::spawn(async move {
        use crate::hub::define::DataReceiver;
        if let Some(StreamHubEvent::Subscribe { result_sender, .. }) = event_receiver.recv().await {
            let (frame_sender, frame_receiver) = crate::hub::define::frame_data_channel();
            let _hold_sender = frame_sender;
            let data_receiver = DataReceiver {
                frame_receiver: Some(frame_receiver),
                packet_receiver: None,
            };
            let _ = result_sender.send(Ok((data_receiver, None)));
        }
    });

    let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
    let remote_addr = "127.0.0.1:0".parse().unwrap();
    let mut session = RtspServerSession::new_with_io(
        session_io,
        event_sender,
        None,
        remote_addr,
        StreamingConfig::default(),
    );
    add_default_video_track(&mut session);

    let run_result = session.run().await;
    assert!(run_result.is_err());
    assert!(session.playback_task.is_none());
    assert!(session.playback_cancel.is_none());

    event_handle.await.expect("event task panicked");
}

#[tokio::test]
async fn test_rtsp_server_session_run_shutdown_sends_unsubscribe_cleanup() {
    let (event_sender, mut event_receiver) = tokio::sync::mpsc::unbounded_channel();
    let mut mock_io = MockNetIO::new();

    mock_io.expect_get_net_type().returning(|| NetType::TCP);
    mock_io.expect_read().times(0);
    mock_io.expect_write().times(0);

    let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
    let remote_addr = "127.0.0.1:0".parse().unwrap();
    let mut session = RtspServerSession::new_with_io(
        session_io,
        event_sender,
        None,
        remote_addr,
        StreamingConfig::default(),
    );

    session.has_subscribed = true;
    session.stream_identifier = Some(StreamIdentifier::Rtsp {
        stream_path: "stream1".to_string(),
    });
    session.shutdown();

    let run_result = session.run().await;
    assert!(run_result.is_ok());
    assert!(session.is_normal_exit);

    match event_receiver
        .try_recv()
        .expect("expected UnSubscribe event")
    {
        StreamHubEvent::UnSubscribe { identifier, .. } => match identifier {
            StreamIdentifier::Rtsp { stream_path } => {
                assert_eq!(stream_path, "stream1");
            }
            _ => panic!("Expected RTSP identifier"),
        },
        _ => panic!("Expected UnSubscribe event"),
    }
}

#[tokio::test]
async fn test_rtsp_server_session_play() {
    let (event_sender, mut event_receiver) = tokio::sync::mpsc::unbounded_channel();
    let mut mock_io = MockNetIO::new();

    mock_io.expect_get_net_type().returning(|| NetType::TCP);
    // Expect response write (200 OK)
    mock_io
        .expect_write()
        .withf(|bytes| {
            let s = std::str::from_utf8(bytes).unwrap();
            s.contains("RTSP/1.0 200 OK")
        })
        .times(1)
        .returning(|_| Ok(()));

    let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
    let remote_addr = "127.0.0.1:0".parse().unwrap();

    let mut session = RtspServerSession::new_with_io(
        session_io,
        event_sender,
        None,
        remote_addr,
        StreamingConfig::default(),
    );
    add_default_video_track(&mut session);

    // Ensure we use the aggregate stream identifier for PLAY requests that include track IDs.
    session.stream_identifier = Some(StreamIdentifier::Rtsp {
        stream_path: "live/test".to_string(),
    });

    // Mock StreamHub handling Subscribe
    let subscribe_handle = tokio::spawn(async move {
        use crate::hub::define::DataReceiver;
        if let Some(event) = event_receiver.recv().await
            && let StreamHubEvent::Subscribe {
                identifier,
                result_sender,
                ..
            } = event
        {
            match identifier {
                StreamIdentifier::Rtsp { stream_path } => {
                    assert_eq!(stream_path, "live/test");
                }
                _ => panic!("Expected RTSP identifier"),
            }
            // Create a channel for frame data that we immediately close to simulate end/error
            let (_frame_sender, frame_receiver) = crate::hub::define::frame_data_channel();

            let data_receiver = DataReceiver {
                frame_receiver: Some(frame_receiver),
                packet_receiver: None,
            };

            let _ = result_sender.send(Ok((data_receiver, None)));
        }
    });

    let content = "PLAY rtsp://localhost/live/test/trackID=0 RTSP/1.0\r\nCSeq: 5\r\nRange: npt=0.000-\r\n\r\n";
    let request = RtspRequest::unmarshal(content).unwrap();

    let result = session.handle_play(&request).await;
    assert!(result.is_ok());
    session.stop_playback_task().await;
    assert!(session.playback_task.is_none());

    subscribe_handle
        .await
        .expect("Subscribe handler task panicked");
}

#[tokio::test]
async fn test_rtsp_server_session_play_malformed_range_returns_457_without_subscribe_event() {
    let (event_sender, mut event_receiver) = tokio::sync::mpsc::unbounded_channel();
    let mut mock_io = MockNetIO::new();

    mock_io.expect_get_net_type().returning(|| NetType::TCP);
    mock_io.expect_read().times(0);
    mock_io
        .expect_write()
        .withf(|bytes| {
            let s = std::str::from_utf8(bytes).unwrap();
            s.contains("RTSP/1.0 457 Invalid Range") && s.contains("CSeq: 9")
        })
        .times(1)
        .returning(|_| Ok(()));

    let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
    let remote_addr = "127.0.0.1:0".parse().unwrap();

    let mut session = RtspServerSession::new_with_io(
        session_io,
        event_sender,
        None,
        remote_addr,
        StreamingConfig::default(),
    );

    let content = "PLAY rtsp://localhost/live/test/trackID=0 RTSP/1.0\r\nCSeq: 9\r\nRange: npt=bad-value\r\n\r\n";
    let request = RtspRequest::unmarshal(content).unwrap();

    let result = session.handle_play(&request).await;
    assert!(result.is_ok());
    assert!(matches!(
        event_receiver.try_recv(),
        Err(tokio::sync::mpsc::error::TryRecvError::Empty)
    ));
}

#[tokio::test]
async fn test_rtsp_server_session_play_normalizes_track_path() {
    let (event_sender, mut event_receiver) = tokio::sync::mpsc::unbounded_channel();
    let mut mock_io = MockNetIO::new();

    mock_io.expect_get_net_type().returning(|| NetType::TCP);
    mock_io
        .expect_write()
        .withf(|bytes| {
            let s = std::str::from_utf8(bytes).unwrap();
            s.contains("RTSP/1.0 200 OK")
        })
        .times(1)
        .returning(|_| Ok(()));

    let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
    let remote_addr = "127.0.0.1:0".parse().unwrap();

    let mut session = RtspServerSession::new_with_io(
        session_io,
        event_sender,
        None,
        remote_addr,
        StreamingConfig::default(),
    );
    add_default_video_track(&mut session);

    let subscribe_handle = tokio::spawn(async move {
        use crate::hub::define::DataReceiver;
        if let Some(event) = event_receiver.recv().await
            && let StreamHubEvent::Subscribe {
                identifier,
                result_sender,
                ..
            } = event
        {
            match identifier {
                StreamIdentifier::Rtsp { stream_path } => {
                    assert_eq!(stream_path, "live/test");
                }
                _ => panic!("Expected RTSP identifier"),
            }
            let (_frame_sender, frame_receiver) = crate::hub::define::frame_data_channel();

            let data_receiver = DataReceiver {
                frame_receiver: Some(frame_receiver),
                packet_receiver: None,
            };

            let _ = result_sender.send(Ok((data_receiver, None)));
        }
    });

    let content = "PLAY rtsp://localhost/live/test/trackID=0 RTSP/1.0\r\nCSeq: 5\r\nRange: npt=0.000-\r\n\r\n";
    let request = RtspRequest::unmarshal(content).unwrap();

    let result = session.handle_play(&request).await;
    assert!(result.is_ok());
    session.stop_playback_task().await;
    assert!(session.playback_task.is_none());

    subscribe_handle
        .await
        .expect("Subscribe handler task panicked");
}

#[tokio::test]
async fn test_rtsp_server_session_play_includes_rtp_info() {
    let (event_sender, mut event_receiver) = tokio::sync::mpsc::unbounded_channel();
    let mut mock_io = MockNetIO::new();

    mock_io.expect_get_net_type().returning(|| NetType::TCP);
    mock_io
        .expect_write()
        .withf(|bytes| {
            let s = std::str::from_utf8(bytes).unwrap();
            s.contains("RTSP/1.0 200 OK")
                && s.contains("RTP-Info")
                && s.contains("rtptime=")
                && s.contains("url=")
                && s.contains("seq=")
        })
        .times(1)
        .returning(|_| Ok(()));

    let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
    let remote_addr = "127.0.0.1:0".parse().unwrap();

    let mut session = RtspServerSession::new_with_io(
        session_io,
        event_sender,
        None,
        remote_addr,
        StreamingConfig::default(),
    );

    let codec_info = RtspCodecInfo {
        codec_id: crate::protocol::rtsp::rtsp_codec::RtspCodecId::H264,
        payload_type: 96,
        sample_rate: 90000,
        channel_count: 0,
    };
    let track = RtspTrack::new(TrackType::Video, codec_info, "trackID=0".to_string());
    session.tracks.insert(TrackType::Video, track);
    session.stream_identifier = Some(StreamIdentifier::Rtsp {
        stream_path: "live/test".to_string(),
    });

    let subscribe_handle = tokio::spawn(async move {
        use crate::hub::define::DataReceiver;
        if let Some(event) = event_receiver.recv().await
            && let StreamHubEvent::Subscribe { result_sender, .. } = event
        {
            let (_frame_sender, frame_receiver) = crate::hub::define::frame_data_channel();
            drop(_frame_sender);
            let data_receiver = DataReceiver {
                frame_receiver: Some(frame_receiver),
                packet_receiver: None,
            };
            let _ = result_sender.send(Ok((data_receiver, None)));
        }
    });

    let content = "PLAY rtsp://127.0.0.1:8554/live/test/trackID=0 RTSP/1.0\r\nCSeq: 5\r\nRange: npt=0.000-\r\n\r\n";
    let request = RtspRequest::unmarshal(content).unwrap();

    let result = session.handle_play(&request).await;
    assert!(result.is_ok());
    session.stop_playback_task().await;
    assert!(session.playback_task.is_none());

    subscribe_handle
        .await
        .expect("Subscribe handler task panicked");
}

// ========================================================================
// ServerSessionType Tests
// ========================================================================

#[test]
fn test_server_session_type_push() {
    let session_type = define::ServerSessionType::Push;
    // Should be able to compare
    assert!(matches!(session_type, define::ServerSessionType::Push));
}

#[test]
fn test_server_session_type_pull() {
    let session_type = define::ServerSessionType::Pull;
    assert!(matches!(session_type, define::ServerSessionType::Pull));
}

#[test]
fn test_rtsp_server_session_exit_without_publish_or_subscribe() {
    let (event_sender, mut event_receiver) = tokio::sync::mpsc::unbounded_channel();
    let mock_io = MockNetIO::new();
    let remote_addr = SocketAddr::from(([127, 0, 0, 1], 0));
    let mut session = RtspServerSession::new_with_io(
        Box::new(mock_io),
        event_sender,
        None,
        remote_addr,
        StreamingConfig::default(),
    );
    let identifier = StreamIdentifier::Rtsp {
        stream_path: "stream1".to_string(),
    };

    let result = session.exit(identifier);
    assert!(result.is_ok());
    assert!(session.is_normal_exit);
    assert!(matches!(
        event_receiver.try_recv(),
        Err(tokio::sync::mpsc::error::TryRecvError::Empty)
    ));
}

#[test]
fn test_rtsp_server_session_exit_published_sends_unpublish() {
    let (event_sender, mut event_receiver) = tokio::sync::mpsc::unbounded_channel();
    let mock_io = MockNetIO::new();
    let remote_addr = SocketAddr::from(([127, 0, 0, 1], 0));
    let mut session = RtspServerSession::new_with_io(
        Box::new(mock_io),
        event_sender,
        None,
        remote_addr,
        StreamingConfig::default(),
    );
    session.has_published = true;

    let identifier = StreamIdentifier::Rtsp {
        stream_path: "stream1".to_string(),
    };
    let result = session.exit(identifier);
    assert!(result.is_ok());

    match event_receiver.try_recv().expect("expected UnPublish event") {
        StreamHubEvent::UnPublish { identifier, .. } => match identifier {
            StreamIdentifier::Rtsp { stream_path } => {
                assert_eq!(stream_path, "stream1");
            }
            _ => panic!("Expected RTSP identifier"),
        },
        _ => panic!("Expected UnPublish event"),
    }
}

#[test]
fn test_rtsp_server_session_exit_subscribed_sends_unsubscribe() {
    let (event_sender, mut event_receiver) = tokio::sync::mpsc::unbounded_channel();
    let mock_io = MockNetIO::new();
    let remote_addr = SocketAddr::from(([127, 0, 0, 1], 0));
    let mut session = RtspServerSession::new_with_io(
        Box::new(mock_io),
        event_sender,
        None,
        remote_addr,
        StreamingConfig::default(),
    );
    session.has_subscribed = true;

    let identifier = StreamIdentifier::Rtsp {
        stream_path: "stream1".to_string(),
    };
    let result = session.exit(identifier);
    assert!(result.is_ok());

    match event_receiver
        .try_recv()
        .expect("expected UnSubscribe event")
    {
        StreamHubEvent::UnSubscribe { identifier, .. } => match identifier {
            StreamIdentifier::Rtsp { stream_path } => {
                assert_eq!(stream_path, "stream1");
            }
            _ => panic!("Expected RTSP identifier"),
        },
        _ => panic!("Expected UnSubscribe event"),
    }
}

// ========================================================================
// Integration-style tests for parsing
// ========================================================================

#[test]
fn test_interleaved_binary_data_all_channels() {
    // Test all common channel identifiers (0-3 for RTP/RTCP audio/video)
    for channel in 0..4u8 {
        let data: &[u8] = &[0x24, channel, 0x00, 0x10];
        let mut reader = BytesReader::new(BytesMut::from(data));

        let result = InterleavedBinaryData::new(&mut reader).unwrap();
        assert!(result.is_some());
        let interleaved = result.unwrap();
        assert_eq!(interleaved.channel_identifier, channel);
        assert_eq!(interleaved.length, 16);
    }
}

#[test]
fn test_gen_response_service_unavailable() {
    let request = create_test_request("DESCRIBE", None);

    let response = RtspServerSession::gen_response(StatusCode::SERVICE_UNAVAILABLE, &request);
    assert_eq!(response.status_code, 503);
    assert_eq!(response.reason_phrase, "Service Unavailable");
}

#[test]
fn test_gen_response_method_not_allowed() {
    let request = create_test_request("UNKNOWN", None);

    let response = RtspServerSession::gen_response(StatusCode::METHOD_NOT_ALLOWED, &request);
    assert_eq!(response.status_code, 405);
    assert_eq!(response.reason_phrase, "Method Not Allowed");
}

// ========================================================================
// validate_rtsp_request_headers Tests
// ========================================================================

#[test]
fn test_validate_rtsp_request_headers_valid_request() {
    let request = create_test_request("OPTIONS", Some("1"));
    let addr: SocketAddr = "127.0.0.1:5000".parse().unwrap();
    let result = RtspServerSession::validate_rtsp_request_headers(&request, &addr);
    assert!(result.is_none());
}

#[test]
fn test_validate_rtsp_request_headers_wrong_version_returns_505() {
    let mut request = create_test_request("DESCRIBE", Some("2"));
    request.version = "RTSP/2.0".to_string();
    let addr: SocketAddr = "127.0.0.1:5000".parse().unwrap();
    let result = RtspServerSession::validate_rtsp_request_headers(&request, &addr);
    assert!(result.is_some());
    let response = result.unwrap();
    assert_eq!(
        response.status_code,
        StatusCode::HTTP_VERSION_NOT_SUPPORTED.as_u16()
    );
}

#[test]
fn test_validate_rtsp_request_headers_empty_version_returns_505() {
    let mut request = create_test_request("OPTIONS", Some("1"));
    request.version = String::new();
    let addr: SocketAddr = "192.168.1.1:8554".parse().unwrap();
    let result = RtspServerSession::validate_rtsp_request_headers(&request, &addr);
    assert!(result.is_some());
    assert_eq!(
        result.unwrap().status_code,
        StatusCode::HTTP_VERSION_NOT_SUPPORTED.as_u16()
    );
}

#[test]
fn test_validate_rtsp_request_headers_content_length_matches_body() {
    let mut request = create_test_request("ANNOUNCE", Some("3"));
    request.body = Some("hello".to_string());
    request
        .headers
        .insert("Content-Length".to_string(), "5".to_string());
    let addr: SocketAddr = "127.0.0.1:5000".parse().unwrap();
    let result = RtspServerSession::validate_rtsp_request_headers(&request, &addr);
    assert!(result.is_none());
}

#[test]
fn test_validate_rtsp_request_headers_content_length_mismatch_returns_400() {
    let mut request = create_test_request("ANNOUNCE", Some("4"));
    request.body = Some("hello".to_string());
    request
        .headers
        .insert("Content-Length".to_string(), "100".to_string());
    let addr: SocketAddr = "10.0.0.1:554".parse().unwrap();
    let result = RtspServerSession::validate_rtsp_request_headers(&request, &addr);
    assert!(result.is_some());
    assert_eq!(
        result.unwrap().status_code,
        StatusCode::BAD_REQUEST.as_u16()
    );
}

#[test]
fn test_validate_rtsp_request_headers_content_length_zero_no_body() {
    let mut request = create_test_request("SETUP", Some("5"));
    request
        .headers
        .insert("Content-Length".to_string(), "0".to_string());
    let addr: SocketAddr = "127.0.0.1:5000".parse().unwrap();
    let result = RtspServerSession::validate_rtsp_request_headers(&request, &addr);
    assert!(result.is_none());
}

#[test]
fn test_validate_rtsp_request_headers_no_content_length_is_valid() {
    let request = create_test_request("PLAY", Some("6"));
    let addr: SocketAddr = "127.0.0.1:5000".parse().unwrap();
    let result = RtspServerSession::validate_rtsp_request_headers(&request, &addr);
    assert!(result.is_none());
}

#[test]
fn test_validate_rtsp_request_headers_non_numeric_content_length_with_empty_body() {
    let mut request = create_test_request("ANNOUNCE", Some("7"));
    request
        .headers
        .insert("Content-Length".to_string(), "abc".to_string());
    // Non-numeric parses to 0, no body means actual=0, so 0==0 → valid
    let addr: SocketAddr = "127.0.0.1:5000".parse().unwrap();
    let result = RtspServerSession::validate_rtsp_request_headers(&request, &addr);
    assert!(result.is_none());
}

#[test]
fn test_validate_rtsp_request_headers_non_numeric_content_length_with_body_returns_400() {
    let mut request = create_test_request("ANNOUNCE", Some("8"));
    request.body = Some("data".to_string());
    request
        .headers
        .insert("Content-Length".to_string(), "abc".to_string());
    // Non-numeric parses to 0, body has 4 bytes → 0!=4 → bad request
    let addr: SocketAddr = "127.0.0.1:5000".parse().unwrap();
    let result = RtspServerSession::validate_rtsp_request_headers(&request, &addr);
    assert!(result.is_some());
    assert_eq!(
        result.unwrap().status_code,
        StatusCode::BAD_REQUEST.as_u16()
    );
}

#[test]
fn test_validate_rtsp_request_headers_content_length_with_whitespace() {
    let mut request = create_test_request("ANNOUNCE", Some("9"));
    request.body = Some("ab".to_string());
    request
        .headers
        .insert("Content-Length".to_string(), " 2 ".to_string());
    let addr: SocketAddr = "127.0.0.1:5000".parse().unwrap();
    let result = RtspServerSession::validate_rtsp_request_headers(&request, &addr);
    assert!(result.is_none());
}

// ========================================================================
// Config Threading Integration Tests
// ========================================================================

/// Test that session stores custom rtp_sample_interval from config
#[test]
fn test_session_uses_config_rtp_sample_interval() {
    let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();
    let mock_io = MockNetIO::new();
    let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
    let remote_addr = "127.0.0.1:0".parse().unwrap();

    // Create session with custom rtp_sample_interval of 42
    let config = StreamingConfig::new().with_rtp_sample_interval(42);
    let session =
        RtspServerSession::new_with_io(session_io, event_sender, None, remote_addr, config);

    // Verify the rtp_sample_interval is stored on the session
    assert_eq!(session.config.rtp_sample_interval, 42);
}

/// Test that session has expected default config values
#[test]
fn test_session_default_config_has_expected_defaults() {
    let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();
    let mock_io = MockNetIO::new();
    let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
    let remote_addr = "127.0.0.1:0".parse().unwrap();

    // Create session with default config
    let config = StreamingConfig::default();
    let session =
        RtspServerSession::new_with_io(session_io, event_sender, None, remote_addr, config);

    // Verify expected defaults
    assert_eq!(session.config.rtp_sample_interval, 0);
    assert_eq!(session.config.max_frame_age_ms, 1500);
    assert_eq!(session.config.play_ready_timeout_ms, 1500);
    assert_eq!(session.config.lag_recovery_mode, LagRecoveryMode::LatestIdr);
    assert_eq!(session.config.rtsp_listen_addr, "0.0.0.0:554");
    assert_eq!(session.config.httpflv_listen_addr, "0.0.0.0:8080");
}

/// Test that env vars have no effect on session config (config is explicit)
#[test]
fn test_session_ignores_env_vars_uses_config() {
    // Set a contradictory env var that should NOT affect session behavior
    // SAFETY: This test uses a known environment variable name that is not used
    // by the streaming library itself. The variable is cleaned up after the test.
    unsafe {
        std::env::set_var("ONVIF_MAX_FRAME_AGE_MS", "9999");
    }

    let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();
    let mock_io = MockNetIO::new();
    let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
    let remote_addr = "127.0.0.1:0".parse().unwrap();

    // Create session with explicit config value (500ms), ignoring the env var
    let config = StreamingConfig::new().with_max_frame_age(500);
    let session =
        RtspServerSession::new_with_io(session_io, event_sender, None, remote_addr, config);

    // Verify session uses the explicit config value (500), NOT the env var (9999)
    assert_eq!(session.config.max_frame_age_ms, 500);

    // Also verify PlaybackLatencyPolicy uses the config value
    let policy = PlaybackLatencyPolicy::from_config(&session.config);
    assert_eq!(policy.max_frame_age_ms, 500);

    // Cleanup: remove the env var
    // SAFETY: Cleanup of the test environment variable set above
    unsafe {
        std::env::remove_var("ONVIF_MAX_FRAME_AGE_MS");
    }
}

/// Test that play_ready_timeout_ms is correctly threaded to session
#[test]
fn test_play_ready_timeout_from_config() {
    let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();
    let mock_io = MockNetIO::new();
    let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
    let remote_addr = "127.0.0.1:0".parse().unwrap();

    // Create session with custom play_ready_timeout_ms
    let config = StreamingConfig::new().with_play_ready_timeout(3000);
    let session =
        RtspServerSession::new_with_io(session_io, event_sender, None, remote_addr, config);

    // Verify play_ready_timeout_ms is stored on the session config
    assert_eq!(session.config.play_ready_timeout_ms, 3000);
}

// ========================================================================
// RTP batching tests
// ========================================================================

struct TestNetIO {
    writes: std::sync::Arc<tokio::sync::Mutex<Vec<bytes::Bytes>>>,
    net_type: crate::io::NetType,
}

impl TestNetIO {
    fn new(
        writes: std::sync::Arc<tokio::sync::Mutex<Vec<bytes::Bytes>>>,
        net_type: crate::io::NetType,
    ) -> Self {
        Self { writes, net_type }
    }
}

#[async_trait::async_trait]
impl crate::io::TNetIO for TestNetIO {
    async fn write(
        &mut self,
        bytes: bytes::Bytes,
    ) -> Result<(), crate::io::bytesio_errors::BytesIOError> {
        self.writes.lock().await.push(bytes);
        Ok(())
    }

    async fn read(&mut self) -> Result<BytesMut, crate::io::bytesio_errors::BytesIOError> {
        Err(crate::io::bytesio_errors::BytesIOErrorValue::NoneReturn.into())
    }

    async fn read_timeout(
        &mut self,
        _duration: std::time::Duration,
    ) -> Result<BytesMut, crate::io::bytesio_errors::BytesIOError> {
        Err(crate::io::bytesio_errors::BytesIOErrorValue::NoneReturn.into())
    }

    fn get_net_type(&self) -> crate::io::NetType {
        self.net_type
    }
}

fn make_test_rtp_packet(
    seq_number: u16,
    marker: u8,
    payload_len: usize,
) -> crate::protocol::rtsp::rtp::RtpPacket {
    let mut packet = crate::protocol::rtsp::rtp::RtpPacket::new(
        crate::protocol::rtsp::rtp::rtp_header::RtpHeader {
            marker,
            payload_type: 96,
            seq_number,
            timestamp: 90_000,
            ssrc: 0x1234_5678,
            ..Default::default()
        },
    );
    packet.payload = BytesMut::from(vec![0xAB; payload_len].as_slice());
    packet
}

fn count_interleaved_packets(bytes: &[u8]) -> usize {
    let mut offset = 0usize;
    let mut count = 0usize;

    while offset + 4 <= bytes.len() {
        if bytes[offset] != 0x24 {
            break;
        }

        let packet_len = u16::from_be_bytes([bytes[offset + 2], bytes[offset + 3]]) as usize;
        let next_offset = offset + 4 + packet_len;
        if next_offset > bytes.len() {
            break;
        }

        count += 1;
        offset = next_offset;
    }

    count
}

fn make_socket_addr() -> std::net::SocketAddr {
    std::net::SocketAddr::from(([127, 0, 0, 1], 8554))
}

fn make_counters() -> std::sync::Arc<RtpTrackCounters> {
    std::sync::Arc::new(RtpTrackCounters::new())
}

#[tokio::test]
async fn test_tcp_batching_flushes_once_per_marker_terminated_frame() {
    let writes = Arc::new(tokio::sync::Mutex::new(Vec::<Bytes>::new()));
    let io: std::sync::Arc<tokio::sync::Mutex<Box<dyn crate::io::TNetIO + Send + Sync>>> =
        std::sync::Arc::new(tokio::sync::Mutex::new(Box::new(TestNetIO::new(
            writes.clone(),
            crate::io::NetType::TCP,
        ))));
    let handler = RtspServerSession::setup_tcp_play_packet_handler(
        0,
        make_counters(),
        None,
        "video".to_string(),
        "sess-1".to_string(),
        make_socket_addr(),
        0,
        1024 * 1024,
    );

    for seq in 1..=5u16 {
        handler(io.clone(), make_test_rtp_packet(seq, 0, 100))
            .await
            .expect("non-marker packet should batch");
    }

    assert!(writes.lock().await.is_empty());

    handler(io.clone(), make_test_rtp_packet(6, 1, 100))
        .await
        .expect("marker packet should flush frame");

    let captured = writes.lock().await;
    assert_eq!(captured.len(), 1);
    assert_eq!(count_interleaved_packets(captured[0].as_ref()), 6);
}

#[tokio::test]
async fn test_tcp_batching_keeps_frames_separate_across_markers() {
    let writes = std::sync::Arc::new(tokio::sync::Mutex::new(Vec::<bytes::Bytes>::new()));
    let io: std::sync::Arc<tokio::sync::Mutex<Box<dyn crate::io::TNetIO + Send + Sync>>> =
        std::sync::Arc::new(tokio::sync::Mutex::new(Box::new(TestNetIO::new(
            writes.clone(),
            crate::io::NetType::TCP,
        ))));
    let handler = RtspServerSession::setup_tcp_play_packet_handler(
        2,
        make_counters(),
        None,
        "video".to_string(),
        "sess-2".to_string(),
        make_socket_addr(),
        0,
        1024 * 1024,
    );

    handler(io.clone(), make_test_rtp_packet(1, 0, 64))
        .await
        .expect("first packet should batch");
    handler(io.clone(), make_test_rtp_packet(2, 1, 64))
        .await
        .expect("second packet should flush frame one");
    handler(io.clone(), make_test_rtp_packet(3, 1, 64))
        .await
        .expect("third packet should flush frame two");

    let captured = writes.lock().await;
    assert_eq!(captured.len(), 2);
    assert_eq!(count_interleaved_packets(captured[0].as_ref()), 2);
    assert_eq!(count_interleaved_packets(captured[1].as_ref()), 1);
}

#[tokio::test]
async fn test_tcp_batching_handles_large_iframe_burst() {
    let writes = std::sync::Arc::new(tokio::sync::Mutex::new(Vec::<bytes::Bytes>::new()));
    let io: std::sync::Arc<tokio::sync::Mutex<Box<dyn crate::io::TNetIO + Send + Sync>>> =
        std::sync::Arc::new(tokio::sync::Mutex::new(Box::new(TestNetIO::new(
            writes.clone(),
            crate::io::NetType::TCP,
        ))));
    let handler = RtspServerSession::setup_tcp_play_packet_handler(
        4,
        make_counters(),
        None,
        "video".to_string(),
        "sess-3".to_string(),
        make_socket_addr(),
        0,
        1024 * 1024,
    );

    for seq in 0..71 {
        handler(io.clone(), make_test_rtp_packet(seq, 0, 1300))
            .await
            .expect("large non-marker packet should batch");
    }
    handler(io.clone(), make_test_rtp_packet(71, 1, 1300))
        .await
        .expect("marker packet should flush large frame");

    let captured = writes.lock().await;
    assert_eq!(captured.len(), 1);
    assert_eq!(count_interleaved_packets(captured[0].as_ref()), 72);
}

#[tokio::test]
async fn test_udp_batching_flushes_all_packets_on_marker() {
    let writes = std::sync::Arc::new(tokio::sync::Mutex::new(Vec::<bytes::Bytes>::new()));
    let io: std::sync::Arc<tokio::sync::Mutex<Box<dyn crate::io::TNetIO + Send + Sync>>> =
        std::sync::Arc::new(tokio::sync::Mutex::new(Box::new(TestNetIO::new(
            writes.clone(),
            crate::io::NetType::UDP,
        ))));
    let handler = RtspServerSession::setup_udp_play_packet_handler(
        make_counters(),
        None,
        "video".into(),
        "sess-udp".into(),
        make_socket_addr(),
        0,
        10,
        300,
        1024 * 1024,
    );

    for seq in 10..20u16 {
        handler(io.clone(), make_test_rtp_packet(seq, 0, 80))
            .await
            .expect("udp packet should batch");
    }
    assert!(writes.lock().await.is_empty());

    handler(io.clone(), make_test_rtp_packet(20, 1, 80))
        .await
        .expect("udp marker packet should flush");

    let captured = writes.lock().await;
    assert_eq!(captured.len(), 11);
}

#[tokio::test]
async fn test_async_bytes_writer_flush_moves_buffer_contents() {
    let writes = std::sync::Arc::new(tokio::sync::Mutex::new(Vec::<bytes::Bytes>::new()));
    let io: std::sync::Arc<tokio::sync::Mutex<Box<dyn crate::io::TNetIO + Send + Sync>>> =
        std::sync::Arc::new(tokio::sync::Mutex::new(Box::new(TestNetIO::new(
            writes.clone(),
            crate::io::NetType::TCP,
        ))));
    let mut writer = crate::io::bytes_writer::AsyncBytesWriter::new(io);

    writer.write(&[1, 2, 3, 4]).expect("write should succeed");
    assert_eq!(writer.bytes_writer.len(), 4);

    writer.flush().await.expect("flush should succeed");

    assert!(writer.bytes_writer.is_empty());
    let captured = writes.lock().await;
    assert_eq!(captured.len(), 1);
    assert_eq!(captured[0].as_ref(), &[1, 2, 3, 4]);
}

/// A mismatched Session header must not put either session id in the log: whoever reads the
/// log would hold a credential good for PLAY or TEARDOWN on the live session.
#[test]
fn test_redact_session_id_keeps_only_a_short_suffix() {
    let redacted = RtspServerSession::redact_session_id("1234567890abcdef");
    assert_eq!(redacted, "...cdef");
    assert!(
        !redacted.contains("1234567890"),
        "redaction must not leak the leading bytes of a session id"
    );
}

/// The requested id is taken straight off the wire, so it can be arbitrary UTF-8. Slicing it
/// by byte offset would panic mid-character and drop the session.
#[test]
fn test_redact_session_id_does_not_panic_on_multibyte_input() {
    assert_eq!(RtspServerSession::redact_session_id("aaaa€€€€"), "...€€€€");
    assert_eq!(RtspServerSession::redact_session_id(""), "...");
    assert_eq!(RtspServerSession::redact_session_id("ab"), "...ab");
}

/// Lazy `Display` must format both a known path and the missing-path sentinel without allocating
/// up front — the whole point of deferring the label until a log line actually renders.
#[test]
fn test_stream_path_display_formats_known_and_unknown() {
    let id = StreamIdentifier::Rtsp {
        stream_path: "/live/main".to_string(),
    };
    assert_eq!(
        format!("{}", StreamPath(Some(&id))),
        format!("{id}"),
        "StreamPath must forward to StreamIdentifier's Display"
    );
    assert_eq!(format!("{}", StreamPath(None)), "unknown");
}

// ========================================================================
// UDP intra-frame pacing
// ========================================================================

/// Records the length of every `write_batch` call so a test can see how a frame was split.
struct BatchRecordingIO(std::sync::Arc<std::sync::Mutex<Vec<usize>>>);

#[async_trait]
impl TNetIO for BatchRecordingIO {
    fn get_net_type(&self) -> NetType {
        NetType::UDP
    }

    async fn write(&mut self, _bytes: Bytes) -> Result<(), BytesIOError> {
        unreachable!("write_udp_frame must batch, never write one datagram at a time")
    }

    async fn write_batch(&mut self, messages: &[Bytes]) -> Result<(), BytesIOError> {
        self.0
            .lock()
            .map_err(|e| std::io::Error::other(e.to_string()))?
            .push(messages.len());
        Ok(())
    }

    async fn read(&mut self) -> Result<BytesMut, BytesIOError> {
        unreachable!("send-only test IO")
    }

    async fn read_timeout(&mut self, _d: std::time::Duration) -> Result<BytesMut, BytesIOError> {
        unreachable!("send-only test IO")
    }
}

/// A main-stream I-frame must leave the box as one `sendmmsg`, with no intra-frame sleep.
///
/// Every extra pacing batch costs a `tokio::time::sleep`. On the camera's contended
/// single core that sleep resolves to ~12 ms, not the 300 us it asks for: measured over
/// 1301 logged frames, `send_ms ~= 31.6 + 12.1 * (batches - 1)`. At the old default of 10
/// packets per batch a ~110 KB I-frame is 8 batches, so it spent ~85 ms of its ~116 ms
/// asleep -- against a 66 ms frame budget at 15 fps. The kernel already paces this path:
/// the socket buffer is ~304 KB (larger than any single frame) and the device reported
/// zero `SndbufErrors` across 287k datagrams.
#[tokio::test]
async fn default_udp_pacing_writes_an_iframe_in_a_single_batch()
-> Result<(), Box<dyn std::error::Error>> {
    let config = StreamingConfig::default();
    // ~110 KB I-frame at the 1400-byte RTP MTU used by rtsp_channel.
    let packets: Vec<BytesMut> = (0..80).map(|_| BytesMut::from(&[0u8; 1400][..])).collect();

    let calls = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let recording_io = BatchRecordingIO(calls.clone());
    assert_eq!(recording_io.get_net_type(), NetType::UDP);
    let io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>> =
        Arc::new(Mutex::new(Box::new(recording_io)));
    let throttle = LogThrottle::new(SLOW_WRITE_REPORT_PERIOD);

    write_udp_frame(
        &io,
        packets,
        config.udp_pace_batch,
        config.udp_pace_sleep_micros,
        "Video",
        "test-session",
        "127.0.0.1:5004".parse()?,
        &throttle,
    )
    .await?;

    let recorded = calls.lock().map_err(|_| "calls mutex poisoned")?.clone();
    assert_eq!(
        recorded,
        vec![80],
        "an I-frame must be one batch; each extra batch costs ~12 ms of scheduler latency"
    );
    Ok(())
}

/// A transport that reports a fixed cost per batch and counts how often it is asked for it.
struct StatsAccountingIO {
    batches: std::sync::Arc<std::sync::Mutex<u32>>,
    takes: std::sync::Arc<std::sync::Mutex<u32>>,
}

#[async_trait]
impl TNetIO for StatsAccountingIO {
    fn get_net_type(&self) -> NetType {
        NetType::UDP
    }
    async fn write(&mut self, _bytes: Bytes) -> Result<(), BytesIOError> {
        unreachable!("write_udp_frame must batch, never write one datagram at a time")
    }
    async fn write_batch(&mut self, _messages: &[Bytes]) -> Result<(), BytesIOError> {
        *self
            .batches
            .lock()
            .map_err(|e| std::io::Error::other(e.to_string()))? += 1;
        Ok(())
    }
    fn take_batch_stats(&mut self) -> Option<BatchWriteStats> {
        *self.takes.lock().ok()? += 1;
        Some(BatchWriteStats {
            attempts: 3,
            park_micros: 5_000,
        })
    }
    async fn read(&mut self) -> Result<BytesMut, BytesIOError> {
        unreachable!("send-only test IO")
    }
    async fn read_timeout(&mut self, _d: std::time::Duration) -> Result<BytesMut, BytesIOError> {
        unreachable!("send-only test IO")
    }
}

/// Every `write_batch` must have its cost collected, not just the last one.
///
/// Pacing is off by default, so the common case is one batch per frame and the distinction is
/// invisible. But `udp_pace_batch` is still a supported knob, and reading the stats only after
/// the loop would silently divide a paced frame's reported park time by the number of chunks --
/// which would understate exactly the cost this instrumentation exists to find.
#[tokio::test]
async fn udp_frame_diagnostics_account_for_every_pacing_chunk()
-> Result<(), Box<dyn std::error::Error>> {
    let packets: Vec<BytesMut> = (0..80).map(|_| BytesMut::from(&[0u8; 1400][..])).collect();
    let batches = std::sync::Arc::new(std::sync::Mutex::new(0u32));
    let takes = std::sync::Arc::new(std::sync::Mutex::new(0u32));
    let accounting_io = StatsAccountingIO {
        batches: batches.clone(),
        takes: takes.clone(),
    };
    assert_eq!(accounting_io.get_net_type(), NetType::UDP);
    let io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>> =
        Arc::new(Mutex::new(Box::new(accounting_io)));
    let throttle = LogThrottle::new(SLOW_WRITE_REPORT_PERIOD);

    // 80 packets, 20 per pacing chunk => 4 batches.
    write_udp_frame(
        &io,
        packets,
        20,
        1,
        "Video",
        "test-session",
        "127.0.0.1:5004".parse()?,
        &throttle,
    )
    .await?;

    let batch_count = *batches.lock().map_err(|_| "batches mutex poisoned")?;
    let take_count = *takes.lock().map_err(|_| "takes mutex poisoned")?;
    assert_eq!(batch_count, 4, "80 packets / 20 per chunk");
    assert_eq!(
        take_count, 4,
        "each batch's park cost must be collected, or a paced frame under-reports"
    );
    Ok(())
}
