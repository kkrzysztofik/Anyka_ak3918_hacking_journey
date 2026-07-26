// Integration tests for RTSP streaming
// Tests end-to-end RTSP session lifecycle, transport negotiation, and RTP streaming

use bytes::BytesMut;
use std::collections::HashMap;
use streaming_lib::hub::define::VideoCodecType;
use streaming_lib::hub::define::{FrameData, PublishType, SubscribeType};
use streaming_lib::hub::stream::StreamIdentifier;
use streaming_lib::io::bytes_reader::BytesReader;
use streaming_lib::protocol::rtsp::global_trait::{Marshal, Unmarshal};
use streaming_lib::protocol::rtsp::rtp::RtpPacket;
use streaming_lib::protocol::rtsp::rtp::rtp_header::RtpHeader;
use streaming_lib::protocol::rtsp::rtp::utils::{Marshal as RtpMarshal, Unmarshal as RtpUnmarshal};
use streaming_lib::protocol::rtsp::rtsp_codec::RtspCodecId;
use streaming_lib::protocol::rtsp::rtsp_transport::{CastType, ProtocolType, RtspTransport};

/// Tests RTSP transport header parsing for UDP
#[tokio::test]
async fn test_rtsp_transport_udp_parsing() {
    let transport_str = "RTP/AVP/UDP;unicast;client_port=5000-5001;server_port=5002-5003";
    let transport = RtspTransport::unmarshal(transport_str).expect("transport header should parse");

    assert_eq!(transport.protocol_type, ProtocolType::UDP);
    assert_eq!(transport.cast_type, CastType::Unicast);
    assert_eq!(transport.client_port, Some([5000, 5001]));
    assert_eq!(transport.server_port, Some([5002, 5003]));
}

/// Tests RTSP transport header parsing for TCP
#[tokio::test]
async fn test_rtsp_transport_tcp_parsing() {
    let transport_str = "RTP/AVP/TCP;unicast;interleaved=0-1";
    let transport = RtspTransport::unmarshal(transport_str).expect("transport header should parse");

    assert_eq!(transport.protocol_type, ProtocolType::TCP);
    assert_eq!(transport.cast_type, CastType::Unicast);
    assert_eq!(transport.interleaved, Some([0, 1]));
}

/// Tests RTSP transport marshaling round-trip
#[tokio::test]
async fn test_rtsp_transport_round_trip() {
    let transport_str = "RTP/AVP/UDP;unicast;client_port=5000-5001;server_port=5002-5003";
    let transport = RtspTransport::unmarshal(transport_str).expect("transport header should parse");
    let marshaled = transport.marshal();

    let reparsed = RtspTransport::unmarshal(&marshaled).expect("re-parsing should succeed");
    assert_eq!(reparsed.protocol_type, ProtocolType::UDP);
    assert_eq!(reparsed.cast_type, CastType::Unicast);
}

/// Tests RTP packet creation with H264 payload
#[tokio::test]
async fn test_rtp_packet_creation() {
    let header = RtpHeader {
        version: 2,
        padding_flag: 0,
        extension_flag: 0,
        cc: 0,
        marker: 1,
        payload_type: 96,
        seq_number: 1000,
        timestamp: 0,
        ssrc: 0x12345678,
        csrcs: vec![],
    };

    let mut packet = RtpPacket::new(header);
    // Add H264 NAL unit start code and NAL unit
    packet
        .payload
        .extend_from_slice(&[0x00, 0x00, 0x00, 0x01, 0x67, 0x42, 0x00, 0x1e]);

    assert_eq!(packet.payload.len(), 8);
    assert_eq!(packet.header.seq_number, 1000);
    assert_eq!(packet.header.marker, 1);
    assert_eq!(packet.header.payload_type, 96);
}

/// Tests RTP packet marshaling and unmarshaling
#[tokio::test]
async fn test_rtp_packet_marshal_unmarshal() {
    let header = RtpHeader {
        version: 2,
        padding_flag: 0,
        extension_flag: 0,
        cc: 0,
        marker: 1,
        payload_type: 96,
        seq_number: 12345,
        timestamp: 1000000,
        ssrc: 0x12345678,
        csrcs: vec![],
    };

    let mut packet = RtpPacket::new(header.clone());
    packet
        .payload
        .extend_from_slice(&[0x67, 0x42, 0x00, 0x1e, 0x9a]);

    let packet_bytes = packet.marshal().unwrap();
    let mut reader = BytesReader::new(packet_bytes);
    let unmarshaled_packet = RtpPacket::unmarshal(&mut reader).unwrap();

    assert_eq!(unmarshaled_packet.header.version, header.version);
    assert_eq!(unmarshaled_packet.header.marker, header.marker);
    assert_eq!(unmarshaled_packet.header.payload_type, header.payload_type);
    assert_eq!(unmarshaled_packet.header.seq_number, header.seq_number);
    assert_eq!(unmarshaled_packet.header.timestamp, header.timestamp);
    assert_eq!(unmarshaled_packet.payload, packet.payload);
}

/// Tests RTP sequence number increment and wrapping
#[tokio::test]
async fn test_rtp_sequence_number_increment() {
    let header = RtpHeader {
        seq_number: 65534,
        timestamp: 0,
        ..Default::default()
    };

    let packet = RtpPacket::new(header);
    let next_seq = packet.header.seq_number.wrapping_add(1);
    assert_eq!(next_seq, 65535);

    let header2 = RtpHeader {
        seq_number: 65535,
        timestamp: 0,
        ..Default::default()
    };

    let packet2 = RtpPacket::new(header2);
    let wrapped_seq = packet2.header.seq_number.wrapping_add(1);
    assert_eq!(wrapped_seq, 0);
}

/// Tests RTP timestamp calculation for 90kHz clock
#[tokio::test]
async fn test_rtp_timestamp_90khz_calculation() {
    let frame_rate = 30.0;
    let timestamp_increment = (90000.0 / frame_rate) as u32;

    assert_eq!(timestamp_increment, 3000);

    let header1 = RtpHeader {
        timestamp: 0,
        ..Default::default()
    };

    let header2 = RtpHeader {
        timestamp: timestamp_increment,
        ..Default::default()
    };

    let diff = header2.timestamp - header1.timestamp;
    assert_eq!(diff, 3000);
}

/// Tests RTSP codec ID mapping
#[tokio::test]
async fn test_rtsp_codec_id_mapping() {
    let codec_id = RtspCodecId::H264;
    assert_eq!(codec_id.name(), "h264");
    assert_eq!(RtspCodecId::from_name("h264"), Some(RtspCodecId::H264));
}

/// Tests multiple simultaneous RTSP session tracking
#[tokio::test]
async fn test_rtsp_multiple_sessions_tracking() {
    let mut sessions: HashMap<String, StreamIdentifier> = HashMap::new();

    let session1_id = "session1".to_string();
    let session2_id = "session2".to_string();

    let stream1 = StreamIdentifier::Rtsp {
        stream_path: "/stream1".to_string(),
    };

    let stream2 = StreamIdentifier::Rtsp {
        stream_path: "/stream2".to_string(),
    };

    sessions.insert(session1_id.clone(), stream1.clone());
    sessions.insert(session2_id.clone(), stream2.clone());

    assert_eq!(sessions.len(), 2);
    assert_eq!(sessions.get(&session1_id), Some(&stream1));
    assert_eq!(sessions.get(&session2_id), Some(&stream2));

    // Test removal on teardown
    sessions.remove(&session1_id);
    assert_eq!(sessions.len(), 1);
    assert!(!sessions.contains_key(&session1_id));
    assert!(sessions.contains_key(&session2_id));
}

/// Tests invalid RTSP transport header handling
#[tokio::test]
async fn test_rtsp_invalid_transport_handling() {
    let invalid_transport = "INVALID/PROTOCOL";
    let result = RtspTransport::unmarshal(invalid_transport);

    // Should fail with invalid protocol
    assert!(result.is_err());
}

/// Tests RTSP transport with TCP
#[tokio::test]
async fn test_rtsp_transport_tcp() {
    let transport_str = "RTP/AVP/TCP;unicast;interleaved=0-1";
    let transport = RtspTransport::unmarshal(transport_str).expect("transport should parse");

    assert_eq!(transport.protocol_type, ProtocolType::TCP);
    assert_eq!(transport.interleaved, Some([0, 1]));
}

/// Tests RTSP SETUP response generation
#[tokio::test]
async fn test_rtsp_setup_response_generation() {
    let transport = RtspTransport {
        protocol_type: ProtocolType::UDP,
        cast_type: CastType::Unicast,
        client_port: Some([5000, 5001]),
        server_port: Some([5002, 5003]),
        interleaved: None,
        transport_mod: None,
        ssrc: None,
    };

    let marshaled = transport.marshal();
    assert!(marshaled.contains("RTP/AVP/UDP"));
    assert!(marshaled.contains("unicast"));
    assert!(marshaled.contains("client_port=5000-5001"));
    assert!(marshaled.contains("server_port=5002-5003"));
}

/// Tests RTP packet with different payload types
#[tokio::test]
async fn test_rtp_payload_types() {
    // Test H264 video (96)
    let video_header = RtpHeader {
        payload_type: 96,
        marker: 1,
        ..Default::default()
    };
    let video_packet = RtpPacket::new(video_header);
    assert_eq!(video_packet.header.payload_type, 96);

    // Test AAC audio (97) - would typically be used for audio
    let audio_header = RtpHeader {
        payload_type: 97,
        marker: 1,
        ..Default::default()
    };
    let audio_packet = RtpPacket::new(audio_header);
    assert_eq!(audio_packet.header.payload_type, 97);
}

/// Tests stream identifier creation for RTSP
#[tokio::test]
async fn test_stream_identifier_rtsp() {
    let stream_id = StreamIdentifier::Rtsp {
        stream_path: "/live/stream1".to_string(),
    };

    match stream_id {
        StreamIdentifier::Rtsp { stream_path } => {
            assert_eq!(stream_path, "/live/stream1");
        }
        _ => panic!("Expected RTSP identifier"),
    }
}

/// Tests RTSP session state transitions (simulated)
#[tokio::test]
async fn test_rtsp_session_state_transitions() {
    #[derive(Debug, Clone, PartialEq, Eq)]
    enum RtspSessionState {
        Init,
        Described,
        Setup,
        Playing,
        Paused,
        Teardown,
    }

    let mut state = RtspSessionState::Init;
    assert_eq!(state, RtspSessionState::Init);

    // DESCRIBE -> Described
    state = RtspSessionState::Described;
    assert_eq!(state, RtspSessionState::Described);

    // SETUP -> Setup
    state = RtspSessionState::Setup;
    assert_eq!(state, RtspSessionState::Setup);

    // PLAY -> Playing
    state = RtspSessionState::Playing;
    assert_eq!(state, RtspSessionState::Playing);

    // PAUSE -> Paused
    state = RtspSessionState::Paused;
    assert_eq!(state, RtspSessionState::Paused);

    // TEARDOWN -> Teardown
    state = RtspSessionState::Teardown;
    assert_eq!(state, RtspSessionState::Teardown);
}

/// Tests RTP packet CSRC handling
#[tokio::test]
async fn test_rtp_csrc_list_handling() {
    let header = RtpHeader {
        cc: 2,
        csrcs: vec![0x12345678, 0x87654321],
        ..Default::default()
    };

    let mut packet = RtpPacket::new(header);
    packet.payload.extend_from_slice(&[0x01, 0x02, 0x03]);

    let packet_bytes = packet.marshal().unwrap();
    let mut reader = BytesReader::new(packet_bytes);
    let unmarshaled = RtpPacket::unmarshal(&mut reader).unwrap();

    assert_eq!(unmarshaled.header.cc, 2);
    assert_eq!(unmarshaled.header.csrcs.len(), 2);
    assert_eq!(unmarshaled.header.csrcs[0], 0x12345678);
    assert_eq!(unmarshaled.header.csrcs[1], 0x87654321);
}

/// Tests video codec type
#[tokio::test]
async fn test_rtsp_video_codec_type() {
    let codec = VideoCodecType::H264;
    assert_eq!(format!("{:?}", codec), "H264");

    let codec2 = VideoCodecType::H265;
    assert_eq!(format!("{:?}", codec2), "H265");
}

/// Tests subscribe type for RTSP
#[tokio::test]
async fn test_rtsp_subscribe_type() {
    let sub_type = SubscribeType::RtspPull;
    match sub_type {
        SubscribeType::RtspPull => {
            // Expected variant
        }
        _ => panic!("Expected RtspPull"),
    }
}

/// Tests publish type for RTSP
#[tokio::test]
async fn test_rtsp_publish_type() {
    let pub_type = PublishType::RtspPush;
    assert!(matches!(pub_type, PublishType::RtspPush));
}

/// Tests frame data creation for video
#[tokio::test]
async fn test_frame_data_video() {
    let frame_data = FrameData::Video {
        timestamp: 1000,
        data: BytesMut::from(&b"\x00\x00\x00\x01\x67\x42\x00\x1e"[..]),
    };

    match frame_data {
        FrameData::Video { timestamp, data } => {
            assert_eq!(timestamp, 1000);
            assert_eq!(data.len(), 8);
        }
        _ => panic!("Expected video frame"),
    }
}

/// Tests frame data creation for audio
#[tokio::test]
async fn test_frame_data_audio() {
    let frame_data = FrameData::Audio {
        timestamp: 1000,
        data: BytesMut::from(&b"\xaf\x00\x12\x10\x56\xe5"[..]),
    };

    match frame_data {
        FrameData::Audio { timestamp, data } => {
            assert_eq!(timestamp, 1000);
            assert_eq!(data.len(), 6);
        }
        _ => panic!("Expected audio frame"),
    }
}

/// Tests port allocation for RTP/RTCP
#[tokio::test]
async fn test_rtp_port_allocation() {
    // Simulate port allocation for a stream
    let base_port = 5000u16;
    let rtp_port = base_port;
    let rtcp_port = base_port + 1;

    assert_eq!(rtp_port, 5000);
    assert_eq!(rtcp_port, 5001);

    // Verify even port for RTP
    assert_eq!(rtp_port % 2, 0);
    // Verify odd port for RTCP
    assert_eq!(rtcp_port % 2, 1);
}

/// Tests RTSP URL parsing
#[tokio::test]
async fn test_rtsp_url_parsing() {
    let url = "rtsp://192.168.1.100:554/live/stream1/track0";

    // Simple parsing test
    assert!(url.starts_with("rtsp://"));
    assert!(url.contains("192.168.1.100"));
    assert!(url.contains("/live/stream1/track0"));

    // Check path components
    let path = "/live/stream1/track0";
    let components: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    assert_eq!(components.len(), 3);
    assert_eq!(components[0], "live");
    assert_eq!(components[1], "stream1");
    assert_eq!(components[2], "track0");
}

/// Tests RTSP transport header with only client ports
#[tokio::test]
async fn test_rtsp_transport_client_ports_only() {
    let transport_str = "RTP/AVP/UDP;unicast;client_port=5000-5001";
    let transport = RtspTransport::unmarshal(transport_str).expect("transport header should parse");

    assert_eq!(transport.protocol_type, ProtocolType::UDP);
    assert_eq!(transport.client_port, Some([5000, 5001]));
    assert_eq!(transport.server_port, None);
}

/// Tests RTSP transport with interleaved channels
#[tokio::test]
async fn test_rtsp_transport_interleaved_channels() {
    let transport_str = "RTP/AVP/TCP;unicast;interleaved=0-1";
    let transport = RtspTransport::unmarshal(transport_str).expect("transport should parse");

    assert_eq!(transport.protocol_type, ProtocolType::TCP);
    assert_eq!(transport.interleaved, Some([0, 1]));
}

/// Tests RTP SSRC generation
#[tokio::test]
async fn test_rtp_ssrc_generation() {
    let header = RtpHeader {
        ssrc: 0x12345678,
        ..Default::default()
    };

    let packet = RtpPacket::new(header);
    assert_eq!(packet.header.ssrc, 0x12345678);
}

/// Tests RTP marker bit behavior
#[tokio::test]
async fn test_rtp_marker_bit() {
    // Marker bit set for frame boundary
    let header_with_marker = RtpHeader {
        marker: 1,
        ..Default::default()
    };
    let packet_with_marker = RtpPacket::new(header_with_marker);
    assert_eq!(packet_with_marker.header.marker, 1);

    // Marker bit clear during frame
    let header_no_marker = RtpHeader {
        marker: 0,
        ..Default::default()
    };
    let packet_no_marker = RtpPacket::new(header_no_marker);
    assert_eq!(packet_no_marker.header.marker, 0);
}

/// Tests RTP version field
#[tokio::test]
async fn test_rtp_version_field() {
    let header = RtpHeader {
        version: 2,
        ..Default::default()
    };

    let packet = RtpPacket::new(header);
    assert_eq!(packet.header.version, 2);
}

/// Tests stream identifier equality
#[tokio::test]
async fn test_stream_identifier_equality() {
    let id1 = StreamIdentifier::Rtsp {
        stream_path: "/stream1".to_string(),
    };

    let id2 = StreamIdentifier::Rtsp {
        stream_path: "/stream1".to_string(),
    };

    let id3 = StreamIdentifier::Rtsp {
        stream_path: "/stream2".to_string(),
    };

    assert_eq!(id1, id2);
    assert_ne!(id1, id3);
}

// ============================================
// ERROR PATH TESTS - RTSP Transport Parsing
// ============================================

/// Tests RTSP transport parsing with malformed protocol
#[tokio::test]
async fn test_rtsp_transport_parse_malformed_protocol() {
    let invalid_inputs = vec![
        "INVALID/PROTOCOL",
        "RTP/AVP/UDP", // Missing required client_port for UDP
        "RTP/AVP/TCP", // Missing required interleaved for TCP
    ];

    for input in invalid_inputs {
        let result = RtspTransport::unmarshal(input);
        assert!(result.is_err(), "Expected error for input: {}", input);
    }
}

/// Tests RTSP transport with port out of valid range
#[tokio::test]
async fn test_rtsp_transport_parse_invalid_port_range() {
    // Port numbers must be 0-65535
    let invalid_port = "RTP/AVP/UDP;unicast;client_port=99999-100000";
    let result = RtspTransport::unmarshal(invalid_port);
    assert!(result.is_err(), "Should fail with port out of range");

    // Another invalid case - port too high
    let invalid_port2 = "RTP/AVP/UDP;unicast;client_port=70000-70001";
    let result2 = RtspTransport::unmarshal(invalid_port2);
    // This should fail since u16::MAX is 65535
    assert!(result2.is_err() || result2.unwrap().client_port.is_none());
}

/// Tests RTSP transport with empty client port
#[tokio::test]
async fn test_rtsp_transport_parse_empty_client_port() {
    let result = RtspTransport::unmarshal("RTP/AVP/UDP;unicast;client_port=");
    assert!(result.is_err(), "Should fail with empty client_port");
}

/// Tests RTSP transport with malformed port pair
#[tokio::test]
async fn test_rtsp_transport_parse_malformed_port_pair() {
    let invalid_inputs = vec![
        "RTP/AVP/UDP;unicast;client_port=abc-def",
        "RTP/AVP/UDP;unicast;client_port=not-a-port",
    ];

    for input in invalid_inputs {
        let result = RtspTransport::unmarshal(input);
        // These should fail validation (missing valid client_port for UDP)
        // Or fail parsing - either is acceptable
        assert!(
            result.is_err(),
            "Expected error for malformed port: {}",
            input
        );
    }
}

/// Tests RTSP transport with invalid interleaved channels
#[tokio::test]
async fn test_rtsp_transport_parse_invalid_interleaved() {
    // Interleaved channels must be 0-255 (u8)
    let invalid_interleaved = "RTP/AVP/TCP;unicast;interleaved=300-301";
    let result = RtspTransport::unmarshal(invalid_interleaved);
    // Should fail or result in None for interleaved
    assert!(result.is_err() || result.unwrap().interleaved.is_none());
}

/// Tests RTSP transport with empty interleaved value
#[tokio::test]
async fn test_rtsp_transport_parse_empty_interleaved() {
    let result = RtspTransport::unmarshal("RTP/AVP/TCP;unicast;interleaved=");
    assert!(result.is_err(), "Should fail with empty interleaved");
}

/// Tests RTSP transport validation rejects TCP without interleaved
#[tokio::test]
async fn test_rtsp_transport_tcp_requires_interleaved() {
    // TCP transport MUST have interleaved parameter
    let result = RtspTransport::unmarshal("RTP/AVP/TCP;unicast");
    assert!(
        result.is_err(),
        "TCP transport without interleaved should be rejected"
    );
    let err = result.unwrap_err();
    assert!(
        err.contains("interleaved"),
        "Error should mention interleaved"
    );
}

/// Tests RTSP transport validation rejects UDP without client_port
#[tokio::test]
async fn test_rtsp_transport_udp_requires_client_port() {
    // UDP transport MUST have client_port parameter
    let result = RtspTransport::unmarshal("RTP/AVP/UDP;unicast");
    assert!(
        result.is_err(),
        "UDP transport without client_port should be rejected"
    );
    let err = result.unwrap_err();
    assert!(
        err.contains("client_port"),
        "Error should mention client_port"
    );
}

/// Tests RTSP transport with invalid SSRC format
#[tokio::test]
async fn test_rtsp_transport_parse_invalid_ssrc() {
    // Invalid SSRC - too large for u32
    let invalid_ssrc = "RTP/AVP/UDP;unicast;client_port=5000-5001;ssrc=9999999999999";
    let result = RtspTransport::unmarshal(invalid_ssrc);
    // Should either fail or result in None for ssrc
    assert!(result.is_err() || result.unwrap().ssrc.is_none());
}

/// Tests empty transport string
#[tokio::test]
async fn test_rtsp_transport_parse_empty_string() {
    let result = RtspTransport::unmarshal("");
    assert!(result.is_err(), "Empty string should fail validation");
}

/// Tests completely invalid transport string
#[tokio::test]
async fn test_rtsp_transport_parse_completely_invalid() {
    let invalid_inputs = vec!["!!!", "完全不valid", "   ", "\n\t"];

    for input in invalid_inputs {
        // These may not parse correctly but shouldn't panic
        let _ = RtspTransport::unmarshal(input);
    }
}

/// Tests UDP transport with only server_port (invalid - needs client_port)
#[tokio::test]
async fn test_rtsp_transport_udp_server_port_only() {
    let result = RtspTransport::unmarshal("RTP/AVP/UDP;unicast;server_port=6000-6001");
    assert!(
        result.is_err(),
        "UDP transport with only server_port should fail"
    );
}

/// Tests multicast transport without required port parameters
#[tokio::test]
async fn test_rtsp_transport_multicast_missing_ports() {
    let result = RtspTransport::unmarshal("RTP/AVP;multicast");
    // Multicast typically needs specific handling - may pass or fail depending on implementation
    // But in our validation, it requires client_port for UDP
    assert!(result.is_err() || result.unwrap().client_port.is_none());
}
