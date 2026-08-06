//! Frame pacing: encoder cadence (RTP timestamp deltas) and arrival cadence
//! (wall clock), summarised as gap percentiles and a delay-event rate.

use serde::Serialize;

use super::rows::RtpTsharkRow;

/// Video RTP media clock (Hz). H.264 uses 90 kHz per RFC 6184.
const VIDEO_RTP_CLOCK_HZ: f64 = 90_000.0;

/// Gap statistics for one cadence (encoder RTP timestamps or arrival wall-clock).
#[derive(Debug, Default, Serialize)]
pub(crate) struct GapStats {
    count: u32,
    min_ms: f64,
    median_ms: f64,
    p90_ms: f64,
    p99_ms: f64,
    pub(crate) max_ms: f64,
    delay_count: u32,
    pub(crate) delay_percent: f64,
}

/// Frame pacing measurement for the primary video stream.
#[derive(Debug, Serialize)]
pub(crate) struct FramePacing {
    expected_fps: f64,
    nominal_ms: f64,
    delay_multiple: f64,
    delay_floor_ms: f64,
    pub(crate) encoder: GapStats,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) arrival: Option<GapStats>,
}

/// Gap (in ms) at or above which a frame gap counts as a delay event.
fn delay_threshold_ms(nominal_ms: f64, delay_multiple: f64, delay_floor_ms: f64) -> f64 {
    (nominal_ms * delay_multiple).max(delay_floor_ms)
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = ((sorted.len() - 1) as f64 * p).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

fn gap_stats(
    deltas_ms: &[f64],
    nominal_ms: f64,
    delay_multiple: f64,
    delay_floor_ms: f64,
) -> GapStats {
    let count = deltas_ms.len() as u32;
    if count == 0 {
        return GapStats::default();
    }
    let mut sorted: Vec<f64> = deltas_ms.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let threshold = delay_threshold_ms(nominal_ms, delay_multiple, delay_floor_ms);
    let delay_count = sorted.iter().filter(|&&d| d >= threshold).count() as u32;
    GapStats {
        count,
        min_ms: sorted[0],
        median_ms: percentile(&sorted, 0.50),
        p90_ms: percentile(&sorted, 0.90),
        p99_ms: percentile(&sorted, 0.99),
        max_ms: sorted[sorted.len() - 1],
        delay_count,
        delay_percent: delay_count as f64 / count as f64 * 100.0,
    }
}

/// Encoder cadence (A): consecutive-frame RTP timestamp deltas.
///
/// Rows arrive in pcap (capture) order; media time is monotonic in arrival
/// order for a live stream, and that keeps 32-bit wrap-around arithmetic
/// well-defined (the same wrap-around assumption used for sequence-number loss).
fn encoder_deltas_ms(rows: &[RtpTsharkRow]) -> Vec<f64> {
    // Half the 32-bit media-clock space: a larger forward delta means the
    // timestamp went backwards (reordering), not a real gap. Same rule the
    // arrival and loss calculations use for sequence numbers.
    const MAX_FORWARD_TICKS: u32 = u32::MAX / 2;
    let mut deltas = Vec::new();
    let mut prev_ts: Option<u32> = None;
    for row in rows {
        if let Some(prev) = prev_ts
            && row.timestamp != prev
        {
            let delta = row.timestamp.wrapping_sub(prev);
            if delta <= MAX_FORWARD_TICKS {
                deltas.push(delta as f64 / VIDEO_RTP_CLOCK_HZ * 1000.0);
            }
            // Out-of-order timestamp; not a gap. Keep the newer reference.
        }
        prev_ts = Some(row.timestamp);
    }
    deltas
}

/// Arrival cadence (B): wall-clock deltas between consecutive frame completions.
fn arrival_deltas_ms(rows: &[RtpTsharkRow]) -> Vec<f64> {
    // Frame completion = wall-clock of the last packet of the frame (in pcap order).
    let mut completions: Vec<f64> = Vec::new();
    let mut cur_ts: Option<u32> = None;
    let mut cur_epoch: Option<f64> = None;
    for row in rows {
        if cur_ts != Some(row.timestamp) {
            if let Some(e) = cur_epoch {
                completions.push(e);
            }
            cur_ts = Some(row.timestamp);
            cur_epoch = None;
        }
        if let Some(e) = row.time_epoch_sec {
            cur_epoch = Some(e);
        }
    }
    if let Some(e) = cur_epoch {
        completions.push(e);
    }
    completions
        .windows(2)
        .filter_map(|w| {
            // ponytail: skip negative deltas (out-of-order completion); not a gap.
            let delta_ms = (w[1] - w[0]) * 1000.0;
            if delta_ms > 0.0 { Some(delta_ms) } else { None }
        })
        .collect()
}

/// Compute frame pacing (A + B) for the primary video stream rows.
///
/// The encoder-cadence math assumes the primary video stream uses the fixed
/// 90 kHz RTP media clock ([`VIDEO_RTP_CLOCK_HZ`]), which holds for the current
/// H.264 validation pipeline. A codec with another clock rate would scale
/// every gap incorrectly, so that assumption must be revisited if such a
/// stream is ever validated.
///
/// Returns `None` when there is no usable data (no rows, no expected fps, or
/// fewer than two frames). Arrival cadence is skipped when the pcap lacks
/// wall-clock times (`time_epoch_sec`), e.g. a legacy capture.
pub(crate) fn compute_pacing(
    rows: &[RtpTsharkRow],
    expected_fps: f64,
    delay_multiple: f64,
    delay_floor_ms: f64,
) -> Option<FramePacing> {
    if rows.is_empty() || expected_fps <= 0.0 {
        return None;
    }
    let nominal_ms = 1000.0 / expected_fps;
    let encoder = gap_stats(
        &encoder_deltas_ms(rows),
        nominal_ms,
        delay_multiple,
        delay_floor_ms,
    );
    let arrival = if rows.iter().all(|r| r.time_epoch_sec.is_some()) {
        Some(gap_stats(
            &arrival_deltas_ms(rows),
            nominal_ms,
            delay_multiple,
            delay_floor_ms,
        ))
    } else {
        None
    };
    if encoder.count == 0 && arrival.as_ref().is_none_or(|a| a.count == 0) {
        return None;
    }
    Some(FramePacing {
        expected_fps,
        nominal_ms,
        delay_multiple,
        delay_floor_ms,
        encoder,
        arrival,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_row_epoch(timestamp: u32, seq: u16, epoch: Option<f64>) -> RtpTsharkRow {
        RtpTsharkRow {
            payload_type: 96,
            marker: true,
            timestamp,
            seq,
            ssrc: Some(0x11223344),
            ip_src: "192.168.2.198".to_string(),
            ip_dst: "192.168.2.10".to_string(),
            udp_src_port: Some(5004),
            udp_dst_port: Some(6000),
            payload: vec![0x65, 0x88, 0x99],
            time_epoch_sec: epoch,
        }
    }

    #[test]
    fn test_delay_threshold_ms_fps_scaling() {
        // 15 fps: 2x = 133.3ms < 150ms floor -> floor governs.
        assert_eq!(delay_threshold_ms(1000.0 / 15.0, 2.0, 150.0), 150.0);
        // 30 fps: 2x = 66.7ms < 150ms floor -> floor governs.
        assert_eq!(delay_threshold_ms(1000.0 / 30.0, 2.0, 150.0), 150.0);
        // 5 fps: 2x = 400ms > 150ms floor -> multiple governs.
        assert_eq!(delay_threshold_ms(1000.0 / 5.0, 2.0, 150.0), 400.0);
    }

    #[test]
    fn test_encoder_deltas_ms_basic() {
        let rows = vec![
            mk_row_epoch(90_000, 1, None),
            mk_row_epoch(180_000, 2, None),
            mk_row_epoch(270_000, 3, None),
        ];
        let deltas = encoder_deltas_ms(&rows);
        assert_eq!(deltas.len(), 2);
        assert!((deltas[0] - 1000.0).abs() < 0.001);
        assert!((deltas[1] - 1000.0).abs() < 0.001);
    }

    #[test]
    fn test_encoder_deltas_ms_frames_with_multiple_packets_collapse() {
        // One frame = two FU-A packets sharing a timestamp.
        let rows = vec![
            mk_row_epoch(90_000, 1, None),
            mk_row_epoch(90_000, 2, None),
            mk_row_epoch(180_000, 3, None),
            mk_row_epoch(180_000, 4, None),
            mk_row_epoch(270_000, 5, None),
        ];
        let deltas = encoder_deltas_ms(&rows);
        assert_eq!(deltas.len(), 2);
        assert!((deltas[0] - 1000.0).abs() < 0.001);
        assert!((deltas[1] - 1000.0).abs() < 0.001);
    }

    #[test]
    fn test_encoder_deltas_ms_wrap_around() {
        let rows = vec![
            mk_row_epoch(0xFFFF_FF00, 1, None),
            mk_row_epoch(0x0000_0100, 2, None),
        ];
        let deltas = encoder_deltas_ms(&rows);
        assert_eq!(deltas.len(), 1);
        // 512 ticks at 90 kHz = 5.689ms
        assert!((deltas[0] - 512.0 / 90_000.0 * 1000.0).abs() < 0.001);
    }

    #[test]
    fn test_arrival_deltas_ms() {
        let rows = vec![
            mk_row_epoch(90_000, 1, Some(1000.000)),
            mk_row_epoch(90_000, 2, Some(1000.010)), // frame 1 completes at 1000.010
            mk_row_epoch(180_000, 3, Some(1000.050)), // frame 2 starts
            mk_row_epoch(180_000, 4, Some(1000.060)), // frame 2 completes
            mk_row_epoch(270_000, 5, Some(1000.090)), // frame 3 starts
            mk_row_epoch(270_000, 6, Some(1000.100)), // frame 3 completes
        ];
        let deltas = arrival_deltas_ms(&rows);
        assert_eq!(deltas.len(), 2);
        assert!((deltas[0] - 50.0).abs() < 0.001); // 1000.060 - 1000.010
        assert!((deltas[1] - 40.0).abs() < 0.001); // 1000.100 - 1000.060
    }

    #[test]
    fn test_compute_pacing_skips_arrival_when_epochs_missing() {
        let rows = vec![
            mk_row_epoch(90_000, 1, None),
            mk_row_epoch(180_000, 2, Some(1000.0)),
        ];
        let pacing = compute_pacing(&rows, 25.0, 2.0, 150.0).unwrap();
        assert_eq!(pacing.encoder.count, 1);
        assert!(pacing.arrival.is_none());
    }

    #[test]
    fn test_compute_pacing_no_data_returns_none() {
        assert!(compute_pacing(&[], 25.0, 2.0, 150.0).is_none());
        let single = vec![mk_row_epoch(90_000, 1, Some(1000.0))];
        assert!(compute_pacing(&single, 25.0, 2.0, 150.0).is_none());
        assert!(compute_pacing(&single, 0.0, 2.0, 150.0).is_none());
    }

    #[test]
    fn test_gap_stats_delay_rule_boundary() {
        // Floor of 150ms; a gap exactly at the floor counts as a delay (>=).
        let stats = gap_stats(&[40.0, 150.0, 200.0], 40.0, 2.0, 150.0);
        assert_eq!(stats.delay_count, 2);
        assert_eq!(stats.delay_percent, 2.0 / 3.0 * 100.0);
        assert_eq!(stats.min_ms, 40.0);
        assert_eq!(stats.max_ms, 200.0);
        assert_eq!(stats.median_ms, 150.0);
    }

    #[test]
    fn test_gap_stats_percentiles() {
        let stats = gap_stats(&[10.0, 20.0, 30.0, 40.0, 50.0], 100.0, 2.0, 150.0);
        assert_eq!(stats.count, 5);
        assert_eq!(stats.delay_count, 0);
        assert_eq!(stats.delay_percent, 0.0);
        assert_eq!(stats.p90_ms, 50.0);
        assert_eq!(stats.p99_ms, 50.0);
    }

    #[test]
    fn test_compute_pacing_encoder_and_arrival_stats() {
        let rows = vec![
            mk_row_epoch(90_000, 1, Some(1000.000)),
            mk_row_epoch(90_000, 2, Some(1000.010)),
            mk_row_epoch(180_000, 3, Some(1000.050)),
            mk_row_epoch(270_000, 4, Some(1000.090)),
            mk_row_epoch(360_000, 5, Some(1000.130)),
        ];
        let pacing = compute_pacing(&rows, 15.0, 2.0, 150.0).unwrap();
        assert_eq!(pacing.encoder.count, 3); // 4 frames -> 3 gaps
        assert_eq!(pacing.arrival.as_ref().unwrap().count, 3);
        // At 15fps nominal 66.7ms; encoder deltas are 1000ms -> all delays.
        assert_eq!(pacing.encoder.delay_count, 3);
        // Arrival gaps ~40-60ms -> no delays.
        assert_eq!(pacing.arrival.as_ref().unwrap().delay_count, 0);
    }
}
