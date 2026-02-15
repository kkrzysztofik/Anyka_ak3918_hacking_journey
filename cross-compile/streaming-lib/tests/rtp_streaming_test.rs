// Integration tests for RTP streaming
// Tests end-to-end RTP packetization, transmission, and depacketization

use streaming_lib::bytesio::bytes_reader::BytesReader;
use streaming_lib::rtsp::rtp::RtpPacket;
use streaming_lib::rtsp::rtp::rtp_header::RtpHeader;
use streaming_lib::rtsp::rtp::utils::Marshal;
use streaming_lib::rtsp::rtp::utils::Unmarshal;

/// Tests RTP packet marshaling/unmarshaling round-trip integrity.
#[tokio::test]
async fn test_rtp_packet_round_trip() {
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
    assert_eq!(unmarshaled_packet.header.ssrc, header.ssrc);
    assert_eq!(unmarshaled_packet.payload, packet.payload);
}

/// Tests sequence number wrapping behavior for RTP headers.
#[tokio::test]
async fn test_rtp_sequence_number_wrapping() {
    let header = RtpHeader {
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

/// Tests timestamp increments for 90kHz video clock.
#[tokio::test]
async fn test_rtp_timestamp_increment() {
    let base_timestamp = 1000000u32;
    let frame_duration_90khz = 3000u32; // ~33ms at 90kHz

    let header1 = RtpHeader {
        timestamp: base_timestamp,
        ..Default::default()
    };

    let header2 = RtpHeader {
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

/// Tests marker-bit round-trip after marshaling/unmarshaling.
#[tokio::test]
async fn test_rtp_marker_bit_round_trip() {
    let header = RtpHeader {
        marker: 1,
        ..Default::default()
    };

    let packet = RtpPacket::new(header.clone());
    let packet_bytes = packet.marshal().unwrap();
    let mut reader = BytesReader::new(packet_bytes);
    let unmarshaled_packet = RtpPacket::unmarshal(&mut reader).unwrap();

    assert_eq!(unmarshaled_packet.header.marker, header.marker);
}

/// Tests CSRC list handling and count derivation.
#[tokio::test]
async fn test_rtp_csrc_handling() {
    let header = RtpHeader {
        cc: 0,
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
