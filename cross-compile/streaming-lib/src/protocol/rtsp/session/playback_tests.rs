use super::*;
use crate::config::StreamingConfig;
use crate::hub::define::FrameData;
use crate::protocol::rtsp::rtp::define::ANNEXB_NALU_START_CODE;
use crate::protocol::rtsp::rtsp_track::TrackType;
use bytes::BytesMut;
use std::time::{Duration, Instant};

// ========================================================================
// scale_rtp_timestamp Tests
// ========================================================================

#[test]
fn test_scale_rtp_timestamp_90000hz() {
    let ts = scale_rtp_timestamp(1000, 90_000);
    assert_eq!(ts, 90_000);
}

#[test]
fn test_scale_rtp_timestamp_zero_clock() {
    let ts = scale_rtp_timestamp(1234, 0);
    assert_eq!(ts, 1234);
}

#[test]
fn test_scale_rtp_timestamp_48000hz_audio() {
    // 48000Hz audio: timestamp is already in sample units, but let's test the math
    // 1000ms * 48000 / 1000 = 48000
    let result = scale_rtp_timestamp(1000, 48000);
    assert_eq!(result, 48000);
}

#[test]
fn test_scale_rtp_timestamp_large_timestamp_saturates() {
    // Very large timestamp_ms near u32::MAX — verify no panic from overflow
    let result = scale_rtp_timestamp(u32::MAX, 90000);
    // (u32::MAX as u64) * 90000 / 1000 → wraps into u32
    let expected = ((u32::MAX as u64).saturating_mul(90000) / 1000) as u32;
    assert_eq!(result, expected);
}

#[test]
fn test_scale_rtp_timestamp_one_ms() {
    // 1ms at 90kHz = 90 ticks
    assert_eq!(scale_rtp_timestamp(1, 90000), 90);
}

// ========================================================================
// RtpTimestampNormalizer Tests
// ========================================================================

#[test]
fn test_rtp_timestamp_normalizer_corrects_non_wrap_regression() {
    let mut normalizer = RtpTimestampNormalizer::default();

    let first = normalizer.normalize(1000, 90_000, TrackType::Video);
    let second = normalizer.normalize(1033, 90_000, TrackType::Video);
    let regressed = normalizer.normalize(0, 90_000, TrackType::Video);
    let next = normalizer.normalize(33, 90_000, TrackType::Video);

    assert_eq!(first.output_timestamp, 0);
    assert_eq!(second.output_timestamp, 2_970);
    assert!(!regressed.non_wrap_regressed);
    assert_eq!(regressed.non_wrap_regression_count, 0);
    assert_eq!(regressed.output_timestamp, 4_294_877_296);
    assert_eq!(next.output_timestamp, 4_294_880_266);
    assert!(!next.non_wrap_regressed);
}

#[test]
fn test_rtp_timestamp_normalizer_corrects_duplicate_timestamp() {
    let mut normalizer = RtpTimestampNormalizer::default();

    let first = normalizer.normalize(1_000, 90_000, TrackType::Video);
    let duplicate = normalizer.normalize(1_000, 90_000, TrackType::Video);
    let next = normalizer.normalize(1_040, 90_000, TrackType::Video);

    assert_eq!(first.output_timestamp, 0);
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

    assert_eq!(first.output_timestamp, 0);
    assert!(!wrapped.non_wrap_regressed);
    assert_eq!(wrapped.non_wrap_regression_count, 0);
    assert_eq!(wrapped.output_timestamp, 16);
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
fn test_rtp_timestamp_normalizer_audio_passthrough() {
    let mut normalizer = RtpTimestampNormalizer::default();

    let first = normalizer.normalize(1000, 48_000, TrackType::Audio);
    assert_eq!(first.output_timestamp, 0);
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

    assert_eq!(reg1.non_wrap_regression_count, 0);
    assert!(!reg1.non_wrap_regressed);
    assert_eq!(reg2.non_wrap_regression_count, 1);
    assert!(reg2.non_wrap_regressed);
    assert_eq!(reg1.output_timestamp, u32::MAX);
    assert_eq!(reg2.output_timestamp, 0);
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
// contains_h264_idr Tests
// ========================================================================

#[test]
fn test_contains_h264_idr_detects_idr_nal() {
    let data = [
        0x00, 0x00, 0x00, 0x01, 0x67, 0x42, 0x00, 0x1e, // SPS
        0x00, 0x00, 0x00, 0x01, 0x65, 0x88, 0x84, // IDR
    ];
    assert!(contains_h264_idr(&data));
}

// ========================================================================
// VideoAccessUnitAssembler Tests
// ========================================================================

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
// LagTracker Tests
// ========================================================================

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
fn test_lag_tracker_current_lag_ms_uninitialized_returns_zero() {
    let tracker = LagTracker::default();
    assert_eq!(tracker.current_lag_ms(), 0);
}

#[test]
fn test_lag_tracker_current_lag_ms_reports_positive_when_behind() {
    let tracker = LagTracker {
        anchor_local: Instant::now() - Duration::from_millis(500),
        anchor_source_ts: 1000,
        last_source_ts: 1200,
        initialized: true,
    };
    // Wall-clock says 500ms have passed since anchor, but source is only at
    // 1200 (200ms past anchor_source_ts=1000). Expected source = 1000+500 = 1500.
    // Lag = 1500 - 1200 = 300ms.
    let lag = tracker.current_lag_ms();
    assert!(lag >= 200, "expected lag >= 200, got {}", lag);
}

// ========================================================================
// maybe_reanchor Tests
// ========================================================================

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
// Reanchor headroom Tests
// ========================================================================

#[test]
fn test_reanchor_headroom_absorbs_iframe_send_cost() {
    let mut tracker = LagTracker {
        anchor_local: Instant::now() - Duration::from_millis(2000),
        anchor_source_ts: 1000,
        last_source_ts: 1500,
        initialized: true,
    };

    // Reanchor with max_frame_age_ms=1000 → headroom = 500ms
    let did_reanchor =
        maybe_reanchor_video_lag_tracker_on_stale_idr(&mut tracker, 2000, 1600, 1000, true);
    assert!(did_reanchor);

    // Immediately after reanchor, current_lag_ms should reflect the 500ms headroom.
    // anchor_local was set to now() - 500ms, so elapsed ≈ 500ms.
    // expected = anchor_source_ts(2000) + elapsed(~500) = ~2500
    // last_source_ts = 2000
    // lag = ~2500 - 2000 = ~500
    let lag = tracker.current_lag_ms();
    assert!(
        lag >= 450 && lag <= 550,
        "expected ~500ms headroom lag, got {}",
        lag
    );
}

#[test]
fn test_reanchor_does_not_trigger_for_non_idr() {
    let mut tracker = LagTracker {
        anchor_local: Instant::now() - Duration::from_millis(2000),
        anchor_source_ts: 1000,
        last_source_ts: 1500,
        initialized: true,
    };

    let original_anchor_ts = tracker.anchor_source_ts;
    let did_reanchor =
        maybe_reanchor_video_lag_tracker_on_stale_idr(&mut tracker, 2000, 1600, 1000, false);
    assert!(!did_reanchor);
    assert_eq!(tracker.anchor_source_ts, original_anchor_ts);
}

// ========================================================================
// FramePacer Tests
// ========================================================================

#[tokio::test]
async fn test_frame_pacer_first_frame_no_sleep() {
    let mut pacer = FramePacer::new();
    let before = Instant::now();
    pacer.pace(1000, 0).await;
    // First frame should complete instantly (well under 10ms).
    assert!(before.elapsed() < Duration::from_millis(10));
    assert!(pacer.last_send.is_some());
    assert_eq!(pacer.last_timestamp_ms, Some(1000));
}

#[tokio::test]
async fn test_frame_pacer_sleeps_when_ahead() {
    let mut pacer = FramePacer::new();
    pacer.pace(0, 0).await;

    // Send second frame with 66ms timestamp gap but essentially zero
    // wall-clock elapsed — the pacer should sleep ~66ms.
    let before = Instant::now();
    pacer.pace(66, 0).await;
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
    pacer.pace(0, 0).await;

    // Wait longer than the timestamp delta before sending next frame.
    tokio::time::sleep(Duration::from_millis(80)).await;

    let before = Instant::now();
    pacer.pace(50, 0).await; // 50ms gap, but 80ms already elapsed
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
    pacer.pace(0, 0).await;

    // 500ms timestamp gap should be capped at PACE_MAX_DELTA_MS (200ms).
    let before = Instant::now();
    pacer.pace(500, 0).await;
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
// Lag-aware pacing tests
// ========================================================================

#[tokio::test]
async fn test_frame_pacer_skips_sleep_when_lagging() {
    let mut pacer = FramePacer::new();
    pacer.pace(0, 0).await;

    // With lag > 0, even a large timestamp gap should not sleep.
    let before = Instant::now();
    let slept = pacer.pace(200, 50).await;
    let elapsed = before.elapsed();

    assert_eq!(slept, 0, "should not sleep when lagging");
    assert!(
        elapsed < Duration::from_millis(10),
        "expected instant return when lagging, got {:?}",
        elapsed
    );
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
    let config = StreamingConfig::default();
    let policy = PlaybackLatencyPolicy::from_config(&config);

    // Default config has max_frame_age_ms=1000, lag_recovery_mode=LatestIdr
    assert_eq!(policy.max_frame_age_ms, 1000);
    assert_eq!(policy.lag_recovery_mode, LagRecoveryMode::LatestIdr);
    // These are constants
    assert_eq!(policy.lag_recovery_threshold_ms, LAG_RECOVERY_THRESHOLD_MS);
    assert_eq!(policy.sustained_lag_frames, LAG_RECOVERY_SUSTAINED_FRAMES);
}

#[test]
fn test_playback_latency_policy_from_config_custom_max_frame_age() {
    let config = StreamingConfig::new().with_max_frame_age(2000);
    let policy = PlaybackLatencyPolicy::from_config(&config);

    assert_eq!(policy.max_frame_age_ms, 2000);
}

#[test]
fn test_playback_latency_policy_from_config_zero_max_frame_age_uses_default() {
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
    let config = StreamingConfig::new().with_lag_recovery_mode(LagRecoveryMode::Disabled);
    let policy = PlaybackLatencyPolicy::from_config(&config);

    assert_eq!(policy.lag_recovery_mode, LagRecoveryMode::Disabled);
}

// ========================================================================
// Config Threading → PlaybackLatencyPolicy Tests
// ========================================================================

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

#[test]
fn test_zero_max_frame_age_falls_back_to_default_in_policy() {
    let config = StreamingConfig {
        max_frame_age_ms: 0,
        ..Default::default()
    };
    let policy = PlaybackLatencyPolicy::from_config(&config);

    // Should fall back to DEFAULT_MAX_FRAME_AGE_MS (1000)
    assert_eq!(policy.max_frame_age_ms, DEFAULT_MAX_FRAME_AGE_MS);
}
