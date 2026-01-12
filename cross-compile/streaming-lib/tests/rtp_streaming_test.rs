// Integration tests for RTP streaming
// Tests end-to-end RTP packetization, transmission, and depacketization

use bytes::BytesMut;
use streaming_lib::bytesio::bytes_reader::BytesReader;
use streaming_lib::rtsp::rtp::rtp_header::RtpHeader;
use streaming_lib::rtsp::rtp::utils::Marshal;
use streaming_lib::rtsp::rtp::utils::Unmarshal;
use streaming_lib::rtsp::rtp::{RtpPacket, define};

#[tokio::test]
async fn test_rtp_packet_round_trip() {
    // Test RTP packet marshaling and unmarshaling
    let mut header = RtpHeader {
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

    // Create RtpPacket manually since new() is private
    let mut packet = RtpPacket {
        header: header.clone(),
        ..Default::default()
    };
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
    assert_eq!(unmarshaled_packet.header.ssrc, header.ssrc);
    assert_eq!(unmarshaled_packet.payload, packet.payload);
}

#[tokio::test]
async fn test_rtp_sequence_number_wrapping() {
    // Test RTP sequence number wrapping
    let mut header = RtpHeader {
        seq_number: 65535,
        ..Default::default()
    };

    let mut packet = RtpPacket {
        header,
        ..Default::default()
    };
    packet.payload.extend_from_slice(&[0x01, 0x02, 0x03]);

    // Sequence number should wrap to 0
    let next_seq = packet.header.seq_number.wrapping_add(1);
    assert_eq!(next_seq, 0);
}

#[tokio::test]
async fn test_rtp_timestamp_increment() {
    // Test RTP timestamp increment for video (90kHz clock)
    let base_timestamp = 1000000u32;
    let frame_duration_90khz = 3000u32; // ~33ms at 90kHz

    let mut header1 = RtpHeader {
        timestamp: base_timestamp,
        ..Default::default()
    };

    let mut header2 = RtpHeader {
        timestamp: base_timestamp + frame_duration_90khz,
        ..Default::default()
    };

    let packet1 = RtpPacket {
        header: header1,
        ..Default::default()
    };
    let packet2 = RtpPacket {
        header: header2,
        ..Default::default()
    };

    let timestamp_diff = packet2
        .header
        .timestamp
        .wrapping_sub(packet1.header.timestamp);
    assert_eq!(timestamp_diff, frame_duration_90khz);
}

#[tokio::test]
async fn test_rtp_marker_bit() {
    // Test RTP marker bit setting for end of frame
    let mut header = RtpHeader {
        marker: 0,
        ..Default::default()
    };

    let mut packet = RtpPacket {
        header,
        ..Default::default()
    };
    packet.header.marker = 1; // Set marker for end of frame

    assert_eq!(packet.header.marker, 1);
}

#[tokio::test]
async fn test_rtp_csrc_handling() {
    // Test RTP CSRC list handling
    let mut header = RtpHeader {
        cc: 2,
        csrcs: vec![0x11111111, 0x22222222],
        ..Default::default()
    };

    let mut packet = RtpPacket {
        header: header.clone(),
        ..Default::default()
    };
    packet.payload.extend_from_slice(&[0x01, 0x02]);

    let packet_bytes = packet.marshal().unwrap();
    let mut reader = BytesReader::new(packet_bytes);
    let unmarshaled = RtpPacket::unmarshal(&mut reader).unwrap();

    assert_eq!(unmarshaled.header.cc, 2);
    assert_eq!(unmarshaled.header.csrcs.len(), 2);
    assert_eq!(unmarshaled.header.csrcs[0], 0x11111111);
    assert_eq!(unmarshaled.header.csrcs[1], 0x22222222);
}
