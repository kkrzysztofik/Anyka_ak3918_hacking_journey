// Tests for RTP send counters and TCP interleaved header parsing (`InterleavedBinaryData`).
//
// Covers anomaly detection (sequence gaps/regressions, timestamp wrap), snapshot timing, and
// framing edge cases used by the RTSP session layer.

use super::*;
use crate::io::bytes_reader::BytesReader;
use crate::protocol::rtsp::session::errors::SessionErrorValue;
use bytes::BytesMut;
// ========================================================================
// InterleavedBinaryData Tests
// ========================================================================

#[test]
fn test_interleaved_binary_data_parse_valid() {
    // Dollar sign (0x24) + channel (0x00) + length (0x0004)
    let data: &[u8] = &[0x24, 0x00, 0x00, 0x04, 0xDE, 0xAD, 0xBE, 0xEF];
    let mut reader = BytesReader::new(BytesMut::from(data));

    let parsed = InterleavedBinaryData::new(&mut reader).unwrap();
    let Some(interleaved) = parsed else {
        panic!("expected Some for valid interleaved header");
    };
    assert_eq!(
        interleaved.channel_identifier, 0x00,
        "channel id: got {}",
        interleaved.channel_identifier
    );
    assert_eq!(interleaved.length, 4, "length: got {}", interleaved.length);
}

#[test]
fn test_interleaved_binary_data_parse_channel_1() {
    // Dollar sign + channel 1 + length 10
    let data: &[u8] = &[0x24, 0x01, 0x00, 0x0A];
    let mut reader = BytesReader::new(BytesMut::from(data));

    let result = InterleavedBinaryData::new(&mut reader).unwrap();
    let Some(interleaved) = result else {
        panic!("expected Some for channel 1");
    };
    assert_eq!(interleaved.channel_identifier, 0x01);
    assert_eq!(interleaved.length, 10);
}

#[test]
fn test_interleaved_binary_data_parse_large_length() {
    // Dollar sign + channel 2 + length 0xFFFF (65535)
    let data: &[u8] = &[0x24, 0x02, 0xFF, 0xFF];
    let mut reader = BytesReader::new(BytesMut::from(data));

    let result = InterleavedBinaryData::new(&mut reader).unwrap();
    let Some(interleaved) = result else {
        panic!("expected Some for 0xFFFF length");
    };
    assert_eq!(interleaved.channel_identifier, 0x02);
    assert_eq!(interleaved.length, 65535);
}

#[test]
fn test_interleaved_binary_data_no_dollar_sign() {
    // Not starting with dollar sign - should return None
    let data: &[u8] = &[0x52, 0x54, 0x53, 0x50]; // "RTSP"
    let mut reader = BytesReader::new(BytesMut::from(data));

    let result = InterleavedBinaryData::new(&mut reader).unwrap();
    assert!(result.is_none(), "expected None when not starting with $");
}

#[test]
fn test_interleaved_binary_data_insufficient_data() {
    // Only dollar sign, not enough for full header
    let data: &[u8] = &[0x24];
    let mut reader = BytesReader::new(BytesMut::from(data));

    let result = InterleavedBinaryData::new(&mut reader);
    assert!(result.is_err(), "expected Err for truncated header after $");
}

#[test]
fn test_interleaved_binary_data_empty() {
    let data: &[u8] = &[];
    let mut reader = BytesReader::new(BytesMut::from(data));

    let result = InterleavedBinaryData::new(&mut reader);
    assert!(result.is_err(), "expected Err for empty reader");
}

#[test]
fn test_interleaved_binary_data_zero_length_errors() {
    let data: &[u8] = &[0x24, 0x01, 0x00, 0x00];
    let mut reader = BytesReader::new(BytesMut::from(data));
    let err = InterleavedBinaryData::new(&mut reader).expect_err("zero length must error");
    assert!(
        matches!(err.value, SessionErrorValue::ZeroLengthInterleavedPayload),
        "expected ZeroLengthInterleavedPayload, got {:?}",
        err.value
    );
}

// ========================================================================
// RtpTrackCounters Tests
// ========================================================================

#[test]
fn test_rtp_track_counters_new_initial_state() {
    let counters = RtpTrackCounters::new();
    assert_eq!(counters.packet_count(), 0);
    assert_eq!(counters.byte_count(), 0);
    assert_eq!(counters.first_send_ms(), 0);
    assert_eq!(counters.last_send_ms(), 0);
    assert_eq!(counters.last_seq_raw(), u32::MAX);
    assert_eq!(counters.last_timestamp_raw(), u32::MAX);
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

    let seq_delta = obs
        .seq_delta
        .expect("expected seq_delta to be Some for backwards sequence step");
    assert!(seq_delta >= RTP_SEQUENCE_WRAP_THRESHOLD);
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
    std::thread::sleep(std::time::Duration::from_millis(100));
    counters.on_packet_sent(150, 1001, 48000);

    let (packets, bytes, duration) = counters.snapshot();
    assert_eq!(packets, 2);
    assert_eq!(bytes, 250);
    assert!(duration.is_some());
    // 100ms wall sleep: allow CI jitter; still require a clearly non-zero gap.
    assert!(duration.unwrap() >= 50, "duration_ms={:?}", duration);
}

// ========================================================================
// Helper Function Tests
// ========================================================================

#[test]
fn test_now_millis_positive() {
    let now = now_millis();
    assert!(now > 0);
}
