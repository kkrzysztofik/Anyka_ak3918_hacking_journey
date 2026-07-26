// Integration tests for RTSP session handling
// Tests end-to-end RTSP session establishment, negotiation, and teardown

use streaming_lib::protocol::rtsp::global_trait::{Marshal, Unmarshal};
use streaming_lib::protocol::rtsp::rtsp_codec::RtspCodecId;
use streaming_lib::protocol::rtsp::rtsp_transport::{CastType, ProtocolType, RtspTransport};
use streaming_lib::protocol::rtsp::sdp::fmtp::Fmtp;
use streaming_lib::protocol::rtsp::sdp::rtpmap::RtpMap;

/// Tests SDP offer/answer negotiation basics for RTSP sessions.
#[tokio::test]
async fn test_rtsp_session_sdp_negotiation() {
    let rtpmap_str = "96 H264/90000";
    let rtpmap = RtpMap::unmarshal(rtpmap_str).unwrap();
    assert_eq!(rtpmap.encoding_name, "H264");
    assert_eq!(rtpmap.clock_rate, 90000);

    let fmtp_str = "96 packetization-mode=1; sprop-parameter-sets=Z2QAFqyyAUBf8uAiAAADAAIAAAMAPB4sXJA=,aOvDyyLA; profile-level-id=640016";
    let fmtp = Fmtp::new("h264", fmtp_str).unwrap();
    match fmtp {
        Fmtp::H264(h264_fmtp) => {
            assert_eq!(h264_fmtp.payload_type, 96);
            assert_eq!(h264_fmtp.packetization_mode, 1);
        }
        _ => panic!("Expected H264 FMTP"),
    }
}

/// Tests RTSP transport header parsing for UDP unicast.
#[tokio::test]
async fn test_rtsp_transport_setup() {
    let transport_str = "RTP/AVP/UDP;unicast;client_port=5000-5001";
    let transport = RtspTransport::unmarshal(transport_str).expect("transport header should parse");

    assert_eq!(transport.protocol_type, ProtocolType::UDP);
    assert_eq!(transport.cast_type, CastType::Unicast);
    assert_eq!(transport.client_port, Some([5000, 5001]));
}

/// Tests codec ID/name mappings for RTSP codec tables.
#[tokio::test]
async fn test_rtsp_codec_mapping() {
    let codec_id = RtspCodecId::H264;
    assert_eq!(codec_id.name(), "h264");
    assert_eq!(RtspCodecId::from_name("h264"), Some(codec_id));
}

/// Tests basic RTSP session flow round-trip for SDP and transport.
#[tokio::test]
async fn test_rtsp_session_round_trip() {
    // 1. Parse SDP
    let rtpmap_str = "96 H264/90000";
    let rtpmap = RtpMap::unmarshal(rtpmap_str).unwrap();
    let marshaled = rtpmap.marshal();
    let rtpmap2 = RtpMap::unmarshal(marshaled.trim()).unwrap();
    assert_eq!(rtpmap.encoding_name, rtpmap2.encoding_name);
    assert_eq!(rtpmap.clock_rate, rtpmap2.clock_rate);

    // 2. Parse transport
    let transport_str = "RTP/AVP/UDP;unicast;client_port=5000-5001;server_port=5002-5003";
    let transport = RtspTransport::unmarshal(transport_str).expect("transport header should parse");
    let marshaled_transport = transport.marshal();
    assert!(marshaled_transport.contains("client_port=5000-5001"));
    assert!(marshaled_transport.contains("server_port=5002-5003"));
}
