use super::*;
use crate::bytesio::bytes_reader::BytesReader;
use crate::common::http::HttpRequest as RtspRequest;
use crate::config::StreamingConfig;
use bytes::BytesMut;
use http::StatusCode;

// ========================================================================
// InterleavedBinaryData Tests
// ========================================================================

#[test]
fn test_interleaved_binary_data_parse_valid() {
    // Dollar sign (0x24) + channel (0x00) + length (0x0004)
    let data: &[u8] = &[0x24, 0x00, 0x00, 0x04, 0xDE, 0xAD, 0xBE, 0xEF];
    let mut reader = BytesReader::new(BytesMut::from(data));

    let result = InterleavedBinaryData::new(&mut reader).unwrap();
    assert!(result.is_some());
    let interleaved = result.unwrap();
    assert_eq!(interleaved.channel_identifier, 0x00);
    assert_eq!(interleaved.length, 4);
}

#[test]
fn test_interleaved_binary_data_parse_channel_1() {
    // Dollar sign + channel 1 + length 10
    let data: &[u8] = &[0x24, 0x01, 0x00, 0x0A];
    let mut reader = BytesReader::new(BytesMut::from(data));

    let result = InterleavedBinaryData::new(&mut reader).unwrap();
    assert!(result.is_some());
    let interleaved = result.unwrap();
    assert_eq!(interleaved.channel_identifier, 0x01);
    assert_eq!(interleaved.length, 10);
}

#[test]
fn test_interleaved_binary_data_parse_large_length() {
    // Dollar sign + channel 2 + length 0xFFFF (65535)
    let data: &[u8] = &[0x24, 0x02, 0xFF, 0xFF];
    let mut reader = BytesReader::new(BytesMut::from(data));

    let result = InterleavedBinaryData::new(&mut reader).unwrap();
    assert!(result.is_some());
    let interleaved = result.unwrap();
    assert_eq!(interleaved.channel_identifier, 0x02);
    assert_eq!(interleaved.length, 65535);
}

#[test]
fn test_interleaved_binary_data_no_dollar_sign() {
    // Not starting with dollar sign - should return None
    let data: &[u8] = &[0x52, 0x54, 0x53, 0x50]; // "RTSP"
    let mut reader = BytesReader::new(BytesMut::from(data));

    let result = InterleavedBinaryData::new(&mut reader).unwrap();
    assert!(result.is_none());
}

#[test]
fn test_interleaved_binary_data_insufficient_data() {
    // Only dollar sign, not enough for full header
    let data: &[u8] = &[0x24];
    let mut reader = BytesReader::new(BytesMut::from(data));

    let result = InterleavedBinaryData::new(&mut reader);
    // Should return an error due to insufficient bytes
    assert!(result.is_err());
}

#[test]
fn test_interleaved_binary_data_empty() {
    let data: &[u8] = &[];
    let mut reader = BytesReader::new(BytesMut::from(data));

    let result = InterleavedBinaryData::new(&mut reader);
    assert!(result.is_err());
}

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
        codec_id: crate::rtsp::rtsp_codec::RtspCodecId::H264,
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
use crate::bytesio::NetType;
use crate::bytesio::TNetIO;
use crate::bytesio::bytesio_errors::BytesIOError;
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
        if let Some(event) = event_receiver.recv().await {
            if let StreamHubEvent::Request {
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
        if let Some(event) = event_receiver.recv().await {
            if let StreamHubEvent::Request { identifier, sender } = event {
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
        codec_id: crate::rtsp::rtsp_codec::RtspCodecId::H264,
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
        codec_id: crate::rtsp::rtsp_codec::RtspCodecId::H264,
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
        codec_id: crate::rtsp::rtsp_codec::RtspCodecId::H264,
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
        use crate::streamhub::define::DataReceiver;

        let mut held_frame_sender = None;
        while let Some(event) = event_receiver.recv().await {
            match event {
                StreamHubEvent::Subscribe { result_sender, .. } => {
                    let (frame_sender, frame_receiver) = tokio::sync::mpsc::unbounded_channel();
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
        use crate::streamhub::define::DataReceiver;

        let mut held_frame_sender = None;
        while let Some(event) = event_receiver.recv().await {
            match event {
                StreamHubEvent::Subscribe { result_sender, .. } => {
                    let (frame_sender, frame_receiver) = tokio::sync::mpsc::unbounded_channel();
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
        use crate::streamhub::define::DataReceiver;
        if let Some(StreamHubEvent::Subscribe { result_sender, .. }) = event_receiver.recv().await {
            let (frame_sender, frame_receiver) = tokio::sync::mpsc::unbounded_channel();
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
        use crate::streamhub::define::DataReceiver;
        if let Some(event) = event_receiver.recv().await {
            if let StreamHubEvent::Subscribe {
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
                let (_frame_sender, frame_receiver) = tokio::sync::mpsc::unbounded_channel();

                let data_receiver = DataReceiver {
                    frame_receiver: Some(frame_receiver),
                    packet_receiver: None,
                };

                let _ = result_sender.send(Ok((data_receiver, None)));
            }
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
        use crate::streamhub::define::DataReceiver;
        if let Some(event) = event_receiver.recv().await {
            if let StreamHubEvent::Subscribe {
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
                let (_frame_sender, frame_receiver) = tokio::sync::mpsc::unbounded_channel();

                let data_receiver = DataReceiver {
                    frame_receiver: Some(frame_receiver),
                    packet_receiver: None,
                };

                let _ = result_sender.send(Ok((data_receiver, None)));
            }
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
        codec_id: crate::rtsp::rtsp_codec::RtspCodecId::H264,
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
        use crate::streamhub::define::DataReceiver;
        if let Some(event) = event_receiver.recv().await {
            if let StreamHubEvent::Subscribe { result_sender, .. } = event {
                let (_frame_sender, frame_receiver) = tokio::sync::mpsc::unbounded_channel();
                drop(_frame_sender);
                let data_receiver = DataReceiver {
                    frame_receiver: Some(frame_receiver),
                    packet_receiver: None,
                };
                let _ = result_sender.send(Ok((data_receiver, None)));
            }
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

#[test]
fn test_scale_rtp_timestamp_90000hz() {
    let ts = RtspServerSession::scale_rtp_timestamp(1000, 90_000);
    assert_eq!(ts, 90_000);
}

#[test]
fn test_scale_rtp_timestamp_zero_clock() {
    let ts = RtspServerSession::scale_rtp_timestamp(1234, 0);
    assert_eq!(ts, 1234);
}

#[test]
fn test_rtp_timestamp_normalizer_corrects_non_wrap_regression() {
    let mut normalizer = RtpTimestampNormalizer::default();

    let first = normalizer.normalize(1000, 90_000, TrackType::Video);
    let second = normalizer.normalize(1033, 90_000, TrackType::Video);
    let regressed = normalizer.normalize(0, 90_000, TrackType::Video);
    let next = normalizer.normalize(33, 90_000, TrackType::Video);

    assert_eq!(first.output_timestamp, 90_000);
    assert_eq!(second.output_timestamp, 92_970);
    assert!(regressed.non_wrap_regressed);
    assert_eq!(regressed.non_wrap_regression_count, 1);
    assert_eq!(
        regressed.output_timestamp,
        second.output_timestamp.wrapping_add(1)
    );
    assert!(next.output_timestamp > regressed.output_timestamp);
}

#[test]
fn test_rtp_timestamp_normalizer_corrects_duplicate_timestamp() {
    let mut normalizer = RtpTimestampNormalizer::default();

    let first = normalizer.normalize(1_000, 90_000, TrackType::Video);
    let duplicate = normalizer.normalize(1_000, 90_000, TrackType::Video);
    let next = normalizer.normalize(1_040, 90_000, TrackType::Video);

    assert_eq!(first.output_timestamp, 90_000);
    assert!(duplicate.non_wrap_regressed);
    assert_eq!(duplicate.non_wrap_regression_count, 1);
    assert_eq!(
        duplicate.output_timestamp,
        first.output_timestamp.wrapping_add(1)
    );
    assert!(next.output_timestamp > duplicate.output_timestamp);
}

#[test]
fn test_rtp_timestamp_normalizer_preserves_true_wrap() {
    let mut normalizer = RtpTimestampNormalizer::default();

    let first = normalizer.normalize(u32::MAX - 10, 0, TrackType::Video);
    let wrapped = normalizer.normalize(5, 0, TrackType::Video);

    assert_eq!(first.output_timestamp, u32::MAX - 10);
    assert!(!wrapped.non_wrap_regressed);
    assert_eq!(wrapped.non_wrap_regression_count, 0);
    assert_eq!(wrapped.output_timestamp, 5);
}

#[test]
fn test_rtp_timestamp_normalizer_audio_no_scaling() {
    // Audio timestamps are already in sample units, should not be scaled
    let mut normalizer = RtpTimestampNormalizer::default();

    // AAC @ 48kHz: First frame at 0 samples
    let first = normalizer.normalize(0, 48_000, TrackType::Audio);
    assert_eq!(first.output_timestamp, 0);

    // Second frame at 1024 samples
    let second = normalizer.normalize(1024, 48_000, TrackType::Audio);
    assert_eq!(second.output_timestamp, 1024);

    // Third frame at 2048 samples
    let third = normalizer.normalize(2048, 48_000, TrackType::Audio);
    assert_eq!(third.output_timestamp, 2048);

    // No scaling should occur
    assert_eq!(second.scaled_timestamp, 1024);
    assert_eq!(third.scaled_timestamp, 2048);
}

#[test]
fn test_rtp_timestamp_normalizer_video_scaling() {
    // Video timestamps are in milliseconds, should be scaled to 90kHz
    let mut normalizer = RtpTimestampNormalizer::default();

    // First frame at 0ms
    let first = normalizer.normalize(0, 90_000, TrackType::Video);
    assert_eq!(first.output_timestamp, 0);

    // Second frame at 33ms (typical for 30fps)
    let second = normalizer.normalize(33, 90_000, TrackType::Video);
    assert_eq!(second.output_timestamp, 2970); // 33 * 90000 / 1000

    // Third frame at 66ms
    let third = normalizer.normalize(66, 90_000, TrackType::Video);
    assert_eq!(third.output_timestamp, 5940); // 66 * 90000 / 1000
}

#[test]
fn test_rtp_timestamp_audio_sequence_monotonic() {
    // Verify audio timestamps produce monotonic sequence without precision loss
    let mut normalizer = RtpTimestampNormalizer::default();

    // Simulate 100 AAC frames @ 48kHz (1024 samples/frame)
    for i in 0..100 {
        let timestamp = i * 1024;
        let result = normalizer.normalize(timestamp, 48_000, TrackType::Audio);

        // Timestamp should exactly match input (no scaling)
        assert_eq!(result.output_timestamp, timestamp);
        assert_eq!(result.scaled_timestamp, timestamp);
        assert!(!result.non_wrap_regressed);
    }
}

#[test]
fn test_video_access_unit_assembler_coalesces_same_timestamp() {
    let mut assembler = VideoAccessUnitAssembler::default();

    let ts1 = 100u32;
    let ts2 = 200u32;

    // First chunk is a raw NAL (no Annex-B prefix).
    assert!(
        assembler
            .push(ts1, BytesMut::from(&b"\x67\x11\x22"[..]))
            .is_none()
    );

    // Second chunk already has a 3-byte Annex-B start code.
    assert!(
        assembler
            .push(ts1, BytesMut::from(&b"\x00\x00\x01\x68\x33"[..]))
            .is_none()
    );

    // Timestamp change flushes the previous access unit.
    let flushed = assembler
        .push(ts2, BytesMut::from(&b"\x65\x44"[..]))
        .expect("expected flush on timestamp change");

    assert_eq!(flushed.0, ts1);
    let mut expected = BytesMut::new();
    expected.extend_from_slice(&ANNEXB_NALU_START_CODE[..]);
    expected.extend_from_slice(&b"\x67\x11\x22"[..]);
    expected.extend_from_slice(&b"\x00\x00\x01\x68\x33"[..]);
    assert_eq!(flushed.1, expected);

    let (ts, bytes) = assembler.flush().expect("expected pending access unit");
    assert_eq!(ts, ts2);
    let mut expected2 = BytesMut::new();
    expected2.extend_from_slice(&ANNEXB_NALU_START_CODE[..]);
    expected2.extend_from_slice(&b"\x65\x44"[..]);
    assert_eq!(bytes, expected2);
}

#[test]
fn test_video_access_unit_assembler_flush_empty_returns_none() {
    let mut assembler = VideoAccessUnitAssembler::default();
    assert!(assembler.flush().is_none());
    assert!(assembler.push(1, BytesMut::new()).is_none());
    assert!(assembler.flush().is_none());
}

// ========================================================================
// RtpTrackCounters Tests
// ========================================================================

#[test]
fn test_rtp_track_counters_new_initial_state() {
    let counters = RtpTrackCounters::new();
    assert_eq!(counters.packet_count.load(Ordering::Relaxed), 0);
    assert_eq!(counters.byte_count.load(Ordering::Relaxed), 0);
    assert_eq!(counters.first_send_ms.load(Ordering::Relaxed), 0);
    assert_eq!(counters.last_send_ms.load(Ordering::Relaxed), 0);
    assert_eq!(counters.last_seq.load(Ordering::Relaxed), u32::MAX);
    assert_eq!(counters.last_timestamp.load(Ordering::Relaxed), u32::MAX);
}

#[test]
fn test_rtp_track_counters_first_packet() {
    let counters = RtpTrackCounters::new();
    let obs = counters.on_packet_sent(100, 1000, 45000);

    assert_eq!(obs.packets_sent, 1);
    assert_eq!(obs.bytes_sent, 100);
    assert!(obs.prev_seq.is_none());
    assert!(obs.prev_timestamp.is_none());
    assert!(obs.seq_delta.is_none());
    assert!(obs.timestamp_delta.is_none());
    assert!(!obs.seq_gap);
    assert!(!obs.seq_regressed);
    assert!(!obs.timestamp_regressed);
}

#[test]
fn test_rtp_track_counters_sequential_packets() {
    let counters = RtpTrackCounters::new();
    counters.on_packet_sent(100, 1000, 45000);
    let obs = counters.on_packet_sent(150, 1001, 48000);

    assert_eq!(obs.packets_sent, 2);
    assert_eq!(obs.bytes_sent, 250);
    assert_eq!(obs.prev_seq, Some(1000));
    assert_eq!(obs.seq_delta, Some(1));
    assert_eq!(obs.prev_timestamp, Some(45000));
    assert_eq!(obs.timestamp_delta, Some(3000));
    assert!(!obs.seq_gap);
    assert!(!obs.seq_regressed);
    assert!(!obs.timestamp_regressed);
}

#[test]
fn test_rtp_track_counters_sequence_gap_detected() {
    let counters = RtpTrackCounters::new();
    counters.on_packet_sent(100, 1000, 45000);
    let obs = counters.on_packet_sent(150, 1005, 48000);

    assert_eq!(obs.seq_delta, Some(5));
    assert!(obs.seq_gap);
    assert!(!obs.seq_regressed);
}

#[test]
fn test_rtp_track_counters_sequence_wraparound() {
    let counters = RtpTrackCounters::new();
    counters.on_packet_sent(100, 65535, 45000);
    let obs = counters.on_packet_sent(150, 0, 48000);

    assert_eq!(obs.prev_seq, Some(65535));
    assert_eq!(obs.seq_delta, Some(1));
    assert!(!obs.seq_gap);
    assert!(!obs.seq_regressed);
}

#[test]
fn test_rtp_track_counters_sequence_regression_detected() {
    let counters = RtpTrackCounters::new();
    counters.on_packet_sent(100, 1005, 45000);
    let obs = counters.on_packet_sent(150, 1000, 48000);

    assert!(obs.seq_delta.unwrap() >= 0x8000);
    assert!(obs.seq_regressed);
    assert!(!obs.seq_gap);
}

#[test]
fn test_rtp_track_counters_timestamp_regression_detected() {
    let counters = RtpTrackCounters::new();
    // Use values where current < previous with small gap -> wrapping delta > threshold
    counters.on_packet_sent(100, 1000, 0x1000);
    let obs = counters.on_packet_sent(150, 1001, 0x0500);

    let delta = obs.timestamp_delta.unwrap();
    // 0x0500 - 0x1000 wraps to 0xFFFFF500, which exceeds the threshold
    assert!(delta > RTP_TIMESTAMP_WRAP_THRESHOLD);
    assert!(obs.timestamp_regressed);
}

#[test]
fn test_rtp_track_counters_snapshot_initial() {
    let counters = RtpTrackCounters::new();
    let (packets, bytes, duration) = counters.snapshot();

    assert_eq!(packets, 0);
    assert_eq!(bytes, 0);
    assert!(duration.is_none());
}

#[test]
fn test_rtp_track_counters_snapshot_after_sends() {
    let counters = RtpTrackCounters::new();
    counters.on_packet_sent(100, 1000, 45000);
    std::thread::sleep(std::time::Duration::from_millis(10));
    counters.on_packet_sent(150, 1001, 48000);

    let (packets, bytes, duration) = counters.snapshot();
    assert_eq!(packets, 2);
    assert_eq!(bytes, 250);
    assert!(duration.is_some());
    assert!(duration.unwrap() >= 10);
}

// ========================================================================
// RtpTimestampNormalizer Tests
// ========================================================================

#[test]
fn test_rtp_timestamp_normalizer_audio_passthrough() {
    let mut normalizer = RtpTimestampNormalizer::default();

    let first = normalizer.normalize(1000, 48_000, TrackType::Audio);
    assert_eq!(first.output_timestamp, 1000);
    assert_eq!(first.scaled_timestamp, 1000);
    assert!(!first.non_wrap_regressed);
}

#[test]
fn test_rtp_timestamp_normalizer_regression_correction() {
    let mut normalizer = RtpTimestampNormalizer::default();

    normalizer.normalize(0, 90_000, TrackType::Video);
    let second = normalizer.normalize(33, 90_000, TrackType::Video);

    // Third frame has lower timestamp -> regression
    let third = normalizer.normalize(20, 90_000, TrackType::Video);
    assert!(third.output_timestamp > second.output_timestamp);
    assert!(third.non_wrap_regressed);
    assert_eq!(third.non_wrap_regression_count, 1);
}

#[test]
fn test_rtp_timestamp_normalizer_equal_timestamp_correction() {
    let mut normalizer = RtpTimestampNormalizer::default();

    let first = normalizer.normalize(1000, 48_000, TrackType::Audio);
    let second = normalizer.normalize(1000, 48_000, TrackType::Audio);

    assert!(second.output_timestamp > first.output_timestamp);
    assert!(second.non_wrap_regressed);
}

#[test]
fn test_rtp_timestamp_normalizer_true_wrap_not_corrected() {
    let mut normalizer = RtpTimestampNormalizer::default();

    normalizer.normalize(0xFFFF_0000, 90_000, TrackType::Video);
    let second = normalizer.normalize(0x0000_1000, 90_000, TrackType::Video);

    // True wrap (large gap > threshold) should NOT be flagged as regression
    assert!(!second.non_wrap_regressed);
}

#[test]
fn test_rtp_timestamp_normalizer_multiple_regressions() {
    let mut normalizer = RtpTimestampNormalizer::default();

    normalizer.normalize(1000, 48_000, TrackType::Audio);
    let reg1 = normalizer.normalize(999, 48_000, TrackType::Audio);
    let reg2 = normalizer.normalize(998, 48_000, TrackType::Audio);

    assert_eq!(reg1.non_wrap_regression_count, 1);
    assert_eq!(reg2.non_wrap_regression_count, 2);
}

// ========================================================================
// has_annexb_start_code Tests
// ========================================================================

#[test]
fn test_has_annexb_start_code_3byte() {
    assert!(has_annexb_start_code(&[0x00, 0x00, 0x01]));
    assert!(has_annexb_start_code(&[0x00, 0x00, 0x01, 0x67]));
}

#[test]
fn test_has_annexb_start_code_4byte() {
    assert!(has_annexb_start_code(&[0x00, 0x00, 0x00, 0x01]));
    assert!(has_annexb_start_code(&[0x00, 0x00, 0x00, 0x01, 0x68]));
}

#[test]
fn test_has_annexb_start_code_false() {
    assert!(!has_annexb_start_code(&[0x00, 0x00, 0x02]));
    assert!(!has_annexb_start_code(&[0x67, 0x00, 0x01]));
    assert!(!has_annexb_start_code(&[0x00, 0x01, 0x00]));
    assert!(!has_annexb_start_code(&[]));
    assert!(!has_annexb_start_code(&[0x00]));
}

// ========================================================================
// Helper Function Tests
// ========================================================================

#[test]
fn test_now_millis_positive() {
    let now = now_millis();
    assert!(now > 0);
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
// VideoAccessUnitAssembler Additional Edge Case Tests
// ========================================================================

#[test]
fn test_video_access_unit_assembler_single_chunk_flush() {
    let mut assembler = VideoAccessUnitAssembler::default();
    // Push one chunk, then flush it
    assert!(
        assembler
            .push(42, BytesMut::from(&b"\x65\xAA\xBB"[..]))
            .is_none()
    );
    let (ts, bytes) = assembler.flush().expect("expected pending data");
    assert_eq!(ts, 42);
    // Should have Annex-B prefix prepended (raw NAL, no start code)
    assert!(bytes.starts_with(&ANNEXB_NALU_START_CODE[..]));
}

#[test]
fn test_video_access_unit_assembler_preserves_existing_annexb_prefix() {
    let mut assembler = VideoAccessUnitAssembler::default();
    let chunk_with_start_code = BytesMut::from(&b"\x00\x00\x00\x01\x67\x11"[..]);
    assembler.push(10, chunk_with_start_code.clone());
    let (ts, bytes) = assembler.flush().unwrap();
    assert_eq!(ts, 10);
    // Should NOT double-prepend start code
    assert_eq!(bytes, chunk_with_start_code);
}

#[test]
fn test_video_access_unit_assembler_three_byte_start_code_preserved() {
    let mut assembler = VideoAccessUnitAssembler::default();
    let chunk = BytesMut::from(&b"\x00\x00\x01\x68\x22"[..]);
    assembler.push(20, chunk.clone());
    let (_, bytes) = assembler.flush().unwrap();
    // 3-byte start code is also recognized
    assert_eq!(bytes, chunk);
}

#[test]
fn test_video_access_unit_assembler_multiple_timestamp_transitions() {
    let mut assembler = VideoAccessUnitAssembler::default();

    // ts=100, two chunks
    assert!(
        assembler
            .push(100, BytesMut::from(&b"\x67\x01"[..]))
            .is_none()
    );
    assert!(
        assembler
            .push(100, BytesMut::from(&b"\x68\x02"[..]))
            .is_none()
    );

    // ts=200 flushes ts=100 AU
    let flushed = assembler.push(200, BytesMut::from(&b"\x65\x03"[..]));
    assert!(flushed.is_some());
    let (ts, _) = flushed.unwrap();
    assert_eq!(ts, 100);

    // ts=300 flushes ts=200 AU
    let flushed2 = assembler.push(300, BytesMut::from(&b"\x65\x04"[..]));
    assert!(flushed2.is_some());
    let (ts2, _) = flushed2.unwrap();
    assert_eq!(ts2, 200);

    // Final flush for ts=300
    let (ts3, _) = assembler.flush().unwrap();
    assert_eq!(ts3, 300);
}

#[test]
fn test_video_access_unit_assembler_empty_chunk_ignored() {
    let mut assembler = VideoAccessUnitAssembler::default();
    assert!(assembler.push(50, BytesMut::new()).is_none());
    // No pending data
    assert!(assembler.flush().is_none());
}

#[test]
fn test_video_access_unit_assembler_empty_after_data_then_empty() {
    let mut assembler = VideoAccessUnitAssembler::default();
    assembler.push(10, BytesMut::from(&b"\x65\x01"[..]));
    // Empty chunk is ignored, does not change pending timestamp
    assert!(assembler.push(10, BytesMut::new()).is_none());
    let (ts, _) = assembler.flush().unwrap();
    assert_eq!(ts, 10);
}

// ========================================================================
// scale_rtp_timestamp Additional Edge Case Tests
// ========================================================================

#[test]
fn test_scale_rtp_timestamp_48000hz_audio() {
    // 48000Hz audio: timestamp is already in sample units, but let's test the math
    // 1000ms * 48000 / 1000 = 48000
    let result = RtspServerSession::scale_rtp_timestamp(1000, 48000);
    assert_eq!(result, 48000);
}

#[test]
fn test_scale_rtp_timestamp_large_timestamp_saturates() {
    // Very large timestamp_ms near u32::MAX — verify no panic from overflow
    let result = RtspServerSession::scale_rtp_timestamp(u32::MAX, 90000);
    // (u32::MAX as u64) * 90000 / 1000 → wraps into u32
    let expected = ((u32::MAX as u64).saturating_mul(90000) / 1000) as u32;
    assert_eq!(result, expected);
}

#[test]
fn test_scale_rtp_timestamp_one_ms() {
    // 1ms at 90kHz = 90 ticks
    assert_eq!(RtspServerSession::scale_rtp_timestamp(1, 90000), 90);
}

#[test]
fn test_contains_h264_idr_detects_idr_nal() {
    let data = [
        0x00, 0x00, 0x00, 0x01, 0x67, 0x42, 0x00, 0x1e, // SPS
        0x00, 0x00, 0x00, 0x01, 0x65, 0x88, 0x84, // IDR
    ];
    assert!(contains_h264_idr(&data));
}

#[test]
fn test_lag_tracker_reports_positive_lag_for_old_frame() {
    let mut tracker = LagTracker {
        anchor_local: Instant::now() - Duration::from_millis(500),
        anchor_source_ts: 1000,
        last_source_ts: 1200,
        initialized: true,
    };
    let lag_ms = tracker.lag_ms(1200);
    assert!(lag_ms >= 200);
}

#[test]
fn test_lag_tracker_resets_on_large_timestamp_regression() {
    let mut tracker = LagTracker {
        anchor_local: Instant::now() - Duration::from_millis(500),
        anchor_source_ts: 10_000,
        last_source_ts: 20_000,
        initialized: true,
    };
    let lag_ms = tracker.lag_ms(100);
    assert_eq!(lag_ms, 0);
    assert_eq!(tracker.anchor_source_ts, 100);
}

#[test]
fn test_maybe_reanchor_video_lag_tracker_on_stale_idr_reanchors() {
    let mut tracker = LagTracker {
        anchor_local: Instant::now() - Duration::from_millis(750),
        anchor_source_ts: 5_000,
        last_source_ts: 5_400,
        initialized: true,
    };

    let did_reanchor =
        maybe_reanchor_video_lag_tracker_on_stale_idr(&mut tracker, 5_420, 650, 600, true);

    assert!(did_reanchor);
    assert_eq!(tracker.anchor_source_ts, 5_420);
    assert_eq!(tracker.last_source_ts, 5_420);
}

#[test]
fn test_maybe_reanchor_video_lag_tracker_on_stale_non_idr_does_not_reanchor() {
    let mut tracker = LagTracker {
        anchor_local: Instant::now() - Duration::from_millis(750),
        anchor_source_ts: 5_000,
        last_source_ts: 5_400,
        initialized: true,
    };

    let did_reanchor =
        maybe_reanchor_video_lag_tracker_on_stale_idr(&mut tracker, 5_420, 650, 600, false);

    assert!(!did_reanchor);
    assert_eq!(tracker.anchor_source_ts, 5_000);
    assert_eq!(tracker.last_source_ts, 5_400);
}

// ========================================================================
// FramePacer Tests
// ========================================================================

#[tokio::test]
async fn test_frame_pacer_first_frame_no_sleep() {
    let mut pacer = FramePacer::new();
    let before = Instant::now();
    pacer.pace(1000).await;
    // First frame should complete instantly (well under 10ms).
    assert!(before.elapsed() < Duration::from_millis(10));
    assert!(pacer.last_send.is_some());
    assert_eq!(pacer.last_timestamp_ms, Some(1000));
}

#[tokio::test]
async fn test_frame_pacer_sleeps_when_ahead() {
    let mut pacer = FramePacer::new();
    pacer.pace(0).await;

    // Send second frame with 66ms timestamp gap but essentially zero
    // wall-clock elapsed — the pacer should sleep ~66ms.
    let before = Instant::now();
    pacer.pace(66).await;
    let elapsed = before.elapsed();

    // Allow generous tolerance for CI/tokio timer granularity.
    assert!(
        elapsed >= Duration::from_millis(40),
        "expected sleep of ~66ms, got {:?}",
        elapsed
    );
    assert!(
        elapsed < Duration::from_millis(200),
        "sleep took too long: {:?}",
        elapsed
    );
}

#[tokio::test]
async fn test_frame_pacer_no_sleep_when_behind() {
    let mut pacer = FramePacer::new();
    pacer.pace(0).await;

    // Wait longer than the timestamp delta before sending next frame.
    tokio::time::sleep(Duration::from_millis(80)).await;

    let before = Instant::now();
    pacer.pace(50).await; // 50ms gap, but 80ms already elapsed
    let elapsed = before.elapsed();

    // Should complete nearly instantly (no sleep needed).
    assert!(
        elapsed < Duration::from_millis(10),
        "expected no sleep, got {:?}",
        elapsed
    );
}

#[tokio::test]
async fn test_frame_pacer_caps_at_max_delta() {
    let mut pacer = FramePacer::new();
    pacer.pace(0).await;

    // 500ms timestamp gap should be capped at PACE_MAX_DELTA_MS (200ms).
    let before = Instant::now();
    pacer.pace(500).await;
    let elapsed = before.elapsed();

    // Should sleep ~200ms (capped), not 500ms.
    assert!(
        elapsed < Duration::from_millis(300),
        "expected ~200ms (capped), got {:?}",
        elapsed
    );
    assert!(
        elapsed >= Duration::from_millis(150),
        "expected ~200ms (capped), got {:?}",
        elapsed
    );
}

#[test]
fn test_pacing_timestamp_ms_video() {
    let frame = FrameData::Video {
        timestamp: 1234,
        data: BytesMut::new(),
    };
    assert_eq!(pacing_timestamp_ms(&frame), Some(1234));
}

#[test]
fn test_pacing_timestamp_ms_audio_none() {
    let frame = FrameData::Audio {
        timestamp: 48_000,
        data: BytesMut::new(),
    };
    assert_eq!(pacing_timestamp_ms(&frame), None);
}

// ========================================================================
// LagRecoveryMode::from_str_value Tests
// ========================================================================

#[test]
fn test_lag_recovery_mode_from_str_value_off_returns_disabled() {
    let mode = LagRecoveryMode::from_str_value("off");
    assert_eq!(mode, LagRecoveryMode::Disabled);
}

#[test]
fn test_lag_recovery_mode_from_str_value_none_returns_disabled() {
    let mode = LagRecoveryMode::from_str_value("none");
    assert_eq!(mode, LagRecoveryMode::Disabled);
}

#[test]
fn test_lag_recovery_mode_from_str_value_disabled_returns_disabled() {
    let mode = LagRecoveryMode::from_str_value("disabled");
    assert_eq!(mode, LagRecoveryMode::Disabled);
}

#[test]
fn test_lag_recovery_mode_from_str_value_latest_idr_returns_latest_idr() {
    let mode = LagRecoveryMode::from_str_value("latest_idr");
    assert_eq!(mode, LagRecoveryMode::LatestIdr);
}

#[test]
fn test_lag_recovery_mode_from_str_value_anything_else_returns_latest_idr() {
    let mode = LagRecoveryMode::from_str_value("anything_else");
    assert_eq!(mode, LagRecoveryMode::LatestIdr);
}

#[test]
fn test_lag_recovery_mode_from_str_value_case_insensitive() {
    assert_eq!(
        LagRecoveryMode::from_str_value("OFF"),
        LagRecoveryMode::Disabled
    );
    assert_eq!(
        LagRecoveryMode::from_str_value("None"),
        LagRecoveryMode::Disabled
    );
    assert_eq!(
        LagRecoveryMode::from_str_value("DISABLED"),
        LagRecoveryMode::Disabled
    );
    assert_eq!(
        LagRecoveryMode::from_str_value("Latest_Idr"),
        LagRecoveryMode::LatestIdr
    );
}

#[test]
fn test_lag_recovery_mode_from_str_value_trims_whitespace() {
    assert_eq!(
        LagRecoveryMode::from_str_value("  off  "),
        LagRecoveryMode::Disabled
    );
    assert_eq!(
        LagRecoveryMode::from_str_value("  latest_idr  "),
        LagRecoveryMode::LatestIdr
    );
}

// ========================================================================
// PlaybackLatencyPolicy::from_config Tests
// ========================================================================

#[test]
fn test_playback_latency_policy_from_config_default() {
    use crate::config::StreamingConfig;

    let config = StreamingConfig::default();
    let policy = PlaybackLatencyPolicy::from_config(&config);

    // Default config has max_frame_age_ms=1500, lag_recovery_mode=LatestIdr
    assert_eq!(policy.max_frame_age_ms, 1500);
    assert_eq!(policy.lag_recovery_mode, LagRecoveryMode::LatestIdr);
    // These are constants
    assert_eq!(policy.lag_recovery_threshold_ms, LAG_RECOVERY_THRESHOLD_MS);
    assert_eq!(policy.sustained_lag_frames, LAG_RECOVERY_SUSTAINED_FRAMES);
}

#[test]
fn test_playback_latency_policy_from_config_custom_max_frame_age() {
    use crate::config::StreamingConfig;

    let config = StreamingConfig::new().with_max_frame_age(2000);
    let policy = PlaybackLatencyPolicy::from_config(&config);

    assert_eq!(policy.max_frame_age_ms, 2000);
}

#[test]
fn test_playback_latency_policy_from_config_zero_max_frame_age_uses_default() {
    use crate::config::StreamingConfig;

    // Config with max_frame_age_ms = 0 should fall back to DEFAULT_MAX_FRAME_AGE_MS
    let config = StreamingConfig {
        max_frame_age_ms: 0,
        ..Default::default()
    };
    let policy = PlaybackLatencyPolicy::from_config(&config);

    assert_eq!(policy.max_frame_age_ms, DEFAULT_MAX_FRAME_AGE_MS);
}

#[test]
fn test_playback_latency_policy_from_config_disabled_lag_recovery() {
    use crate::config::StreamingConfig;

    let config = StreamingConfig::new().with_lag_recovery_mode(LagRecoveryMode::Disabled);
    let policy = PlaybackLatencyPolicy::from_config(&config);

    assert_eq!(policy.lag_recovery_mode, LagRecoveryMode::Disabled);
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

/// Test that max_frame_age config is correctly threaded to PlaybackLatencyPolicy
#[test]
fn test_max_frame_age_config_threaded_to_policy() {
    // Test various custom values
    for custom_max_age in [100, 500, 2000, 5000] {
        let config = StreamingConfig::new().with_max_frame_age(custom_max_age);
        let policy = PlaybackLatencyPolicy::from_config(&config);

        // Verify the policy uses the config value directly
        assert_eq!(
            policy.max_frame_age_ms, custom_max_age,
            "Policy should use config max_frame_age_ms={}, got {}",
            custom_max_age, policy.max_frame_age_ms
        );
    }
}

/// Test that lag_recovery_mode config is correctly threaded to PlaybackLatencyPolicy
#[test]
fn test_lag_recovery_mode_config_threaded_to_policy() {
    // Test Disabled mode
    let config_disabled = StreamingConfig::new().with_lag_recovery_mode(LagRecoveryMode::Disabled);
    let policy_disabled = PlaybackLatencyPolicy::from_config(&config_disabled);
    assert_eq!(policy_disabled.lag_recovery_mode, LagRecoveryMode::Disabled);

    // Test LatestIdr mode
    let config_latest = StreamingConfig::new().with_lag_recovery_mode(LagRecoveryMode::LatestIdr);
    let policy_latest = PlaybackLatencyPolicy::from_config(&config_latest);
    assert_eq!(policy_latest.lag_recovery_mode, LagRecoveryMode::LatestIdr);
}

/// Test that zero max_frame_age falls back to default in policy
#[test]
fn test_zero_max_frame_age_falls_back_to_default_in_policy() {
    let config = StreamingConfig {
        max_frame_age_ms: 0,
        ..Default::default()
    };
    let policy = PlaybackLatencyPolicy::from_config(&config);

    // Should fall back to DEFAULT_MAX_FRAME_AGE_MS (1500)
    assert_eq!(policy.max_frame_age_ms, DEFAULT_MAX_FRAME_AGE_MS);
}
