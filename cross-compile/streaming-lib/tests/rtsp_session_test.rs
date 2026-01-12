// Integration tests for RTSP session handling
// Tests end-to-end RTSP session establishment, negotiation, and teardown

use streaming_lib::rtsp::global_trait::{Marshal, Unmarshal};
use streaming_lib::rtsp::rtsp_codec::RtspCodecId;
use streaming_lib::rtsp::rtsp_transport::{CastType, ProtocolType, RtspTransport};
use streaming_lib::rtsp::sdp::fmtp::{Fmtp, H264Fmtp};
use streaming_lib::rtsp::sdp::rtpmap::RtpMap;

#[tokio::test]
async fn test_rtsp_session_sdp_negotiation() {
    // Test SDP offer/answer negotiation
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

#[tokio::test]
async fn test_rtsp_transport_setup() {
    // Test RTSP transport header parsing and setup
    let transport_str = "RTP/AVP/UDP;unicast;client_port=5000-5001";
    let transport = RtspTransport::unmarshal(transport_str).unwrap();

    assert_eq!(transport.protocol_type, ProtocolType::UDP);
    assert_eq!(transport.cast_type, CastType::Unicast);
    assert_eq!(transport.client_port, Some([5000, 5001]));
}

#[tokio::test]
async fn test_rtsp_codec_mapping() {
    // Test codec ID to name mapping
    let codec_id = RtspCodecId::H264;
    let codec_name = streaming_lib::rtsp::rtsp_codec::RTSP_CODEC_ID_2_NAME
        .get(&codec_id)
        .unwrap();
    assert_eq!(*codec_name, "H264");

    let mapped_id = streaming_lib::rtsp::rtsp_codec::RTSP_CODEC_NAME_2_ID
        .get("H264")
        .unwrap();
    assert_eq!(*mapped_id, codec_id);
}

#[tokio::test]
async fn test_rtsp_session_round_trip() {
    // Test complete RTSP session flow
    // 1. Parse SDP
    let rtpmap_str = "96 H264/90000";
    let rtpmap = RtpMap::unmarshal(rtpmap_str).unwrap();
    let marshaled = rtpmap.marshal();
    let rtpmap2 = RtpMap::unmarshal(&marshaled.trim()).unwrap();
    assert_eq!(rtpmap.encoding_name, rtpmap2.encoding_name);
    assert_eq!(rtpmap.clock_rate, rtpmap2.clock_rate);

    // 2. Parse transport
    let transport_str = "RTP/AVP/UDP;unicast;client_port=5000-5001;server_port=5002-5003";
    let transport = RtspTransport::unmarshal(transport_str).unwrap();
    let marshaled_transport = transport.marshal();
    assert!(marshaled_transport.contains("client_port=5000-5001"));
    assert!(marshaled_transport.contains("server_port=5002-5003"));
}
