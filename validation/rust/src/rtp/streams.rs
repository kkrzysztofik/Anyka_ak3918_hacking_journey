//! Grouping RTP rows into streams, selecting the primary video/audio stream,
//! and computing packet loss.

use serde::Serialize;
use std::collections::HashMap;

use super::payload::{validate_aac_rtp_payload_rfc3640, validate_h264_rtp_payload_rfc6184};
use super::rows::RtpTsharkRow;

#[derive(Debug, Clone, Default, Serialize)]
pub(crate) struct HarnessRtpLossMetric {
    pub(crate) rtp_packets: u32,
    pub(crate) packet_loss: u32,
    pub(crate) loss_percent: f64,
    pub(crate) payload_type: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) ssrc: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct RtpStreamKey {
    payload_type: u8,
    ssrc: Option<u32>,
    dst_port: Option<u16>,
}

#[derive(Debug, Clone)]
pub(crate) struct RtpStreamStats {
    pub(crate) key: RtpStreamKey,
    pub(crate) rows: Vec<RtpTsharkRow>,
    valid_h264: u32,
    valid_aac: u32,
}

fn compute_packet_loss_from_seqs(seqs: &[u16]) -> (u32, u32, f64) {
    // Compute loss on capture order while ignoring likely reordering.
    // This is good enough for the short-duration harness capture.
    let mut total = 0u32;
    let mut loss = 0u32;
    let mut prev: Option<u16> = None;
    for &seq in seqs {
        total = total.saturating_add(1);
        let Some(p) = prev else {
            prev = Some(seq);
            continue;
        };
        let delta = seq.wrapping_sub(p) as u32;
        if delta == 0 {
            continue;
        }
        if delta < 32768 {
            if delta > 1 {
                loss = loss.saturating_add(delta - 1);
            }
            prev = Some(seq);
        } else {
            // Likely out-of-order delivery. Do not count as loss.
            continue;
        }
    }
    let loss_percent = if total > 0 {
        100.0 * (loss as f64) / (total as f64)
    } else {
        0.0
    };
    (total, loss, loss_percent)
}

pub(crate) fn compute_stream_loss_metric(stats: &RtpStreamStats) -> HarnessRtpLossMetric {
    let seqs: Vec<u16> = stats.rows.iter().map(|r| r.seq).collect();
    let (total, loss, pct) = compute_packet_loss_from_seqs(&seqs);
    HarnessRtpLossMetric {
        rtp_packets: total,
        packet_loss: loss,
        loss_percent: pct,
        payload_type: stats.key.payload_type,
        ssrc: stats.key.ssrc,
    }
}

fn is_reasonably_h264(stats: &RtpStreamStats) -> bool {
    const MIN_PACKETS: u32 = 10;
    const MIN_VALID_RATIO: f64 = 0.80;
    let total = stats.rows.len() as u32;
    total >= MIN_PACKETS && (stats.valid_h264 as f64 / total as f64) >= MIN_VALID_RATIO
}

fn is_reasonably_aac(stats: &RtpStreamStats) -> bool {
    const MIN_PACKETS: u32 = 10;
    const MIN_VALID_RATIO: f64 = 0.80;
    let total = stats.rows.len() as u32;
    total >= MIN_PACKETS && (stats.valid_aac as f64 / total as f64) >= MIN_VALID_RATIO
}

pub(crate) fn pick_primary_video_stream(streams: &[RtpStreamStats]) -> Option<&RtpStreamStats> {
    streams
        .iter()
        .filter(|s| is_reasonably_h264(s))
        .max_by_key(|s| (s.valid_h264, s.rows.len()))
        .or_else(|| streams.iter().max_by_key(|s| s.rows.len()))
}

pub(crate) fn pick_primary_audio_stream(
    streams: &[RtpStreamStats],
    video_key: Option<RtpStreamKey>,
) -> Option<&RtpStreamStats> {
    let candidates: Vec<&RtpStreamStats> = streams
        .iter()
        .filter(|s| Some(s.key) != video_key)
        .collect();

    candidates
        .iter()
        .copied()
        .filter(|s| is_reasonably_aac(s))
        .max_by_key(|s| (s.valid_aac, s.rows.len()))
        .or_else(|| candidates.into_iter().max_by_key(|s| s.rows.len()))
}

pub(crate) fn group_rtp_rows_by_stream(rows: Vec<RtpTsharkRow>) -> Vec<RtpStreamStats> {
    let mut streams: HashMap<RtpStreamKey, RtpStreamStats> = HashMap::new();
    for row in rows {
        let key = RtpStreamKey {
            payload_type: row.payload_type,
            ssrc: row.ssrc,
            dst_port: row.udp_dst_port,
        };
        let entry = streams.entry(key).or_insert_with(|| RtpStreamStats {
            key,
            rows: Vec::new(),
            valid_h264: 0,
            valid_aac: 0,
        });
        if validate_h264_rtp_payload_rfc6184(&row.payload, row.marker).0 {
            entry.valid_h264 = entry.valid_h264.saturating_add(1);
        }
        if validate_aac_rtp_payload_rfc3640(&row.payload).0 {
            entry.valid_aac = entry.valid_aac.saturating_add(1);
        }
        entry.rows.push(row);
    }
    streams.into_values().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packet_loss_computation_does_not_mix_streams() {
        fn mk_row(pt: u8, ssrc: u32, dst_port: u16, seq: u16, payload: &[u8]) -> RtpTsharkRow {
            RtpTsharkRow {
                payload_type: pt,
                marker: true,
                timestamp: 0,
                seq,
                ssrc: Some(ssrc),
                ip_src: "192.168.2.198".to_string(),
                ip_dst: "192.168.2.10".to_string(),
                udp_src_port: Some(5004),
                udp_dst_port: Some(dst_port),
                payload: payload.to_vec(),
                time_epoch_sec: None,
            }
        }

        let h264_payload = [0x65, 0x88, 0x99]; // Single NAL, IDR.
        let aac_payload = [0x00, 0x10, 0x00, 0x10, 0x11, 0x22]; // RFC 3640 AU header + 2 bytes.

        let rows = vec![
            mk_row(96, 1, 6000, 100, &h264_payload),
            mk_row(97, 2, 6002, 200, &aac_payload),
            mk_row(96, 1, 6000, 101, &h264_payload),
            mk_row(97, 2, 6002, 201, &aac_payload),
            mk_row(96, 1, 6000, 102, &h264_payload),
            mk_row(97, 2, 6002, 202, &aac_payload),
        ];

        let streams = group_rtp_rows_by_stream(rows);
        assert_eq!(streams.len(), 2);

        let video = pick_primary_video_stream(&streams).expect("video stream");
        let audio = pick_primary_audio_stream(&streams, Some(video.key)).expect("audio stream");

        let (video_total, video_loss, _) =
            compute_packet_loss_from_seqs(&video.rows.iter().map(|r| r.seq).collect::<Vec<_>>());
        let (audio_total, audio_loss, _) =
            compute_packet_loss_from_seqs(&audio.rows.iter().map(|r| r.seq).collect::<Vec<_>>());

        assert_eq!(video_total, 3);
        assert_eq!(video_loss, 0);
        assert_eq!(audio_total, 3);
        assert_eq!(audio_loss, 0);
    }
}
