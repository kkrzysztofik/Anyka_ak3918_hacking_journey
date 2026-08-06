//! RTP payload conformance: RFC 6184 (H.264) and RFC 3640 (AAC).

use anyhow::{Result, bail};
use serde::Serialize;
use std::collections::HashMap;

use super::rows::RtpTsharkRow;

#[derive(Debug, Default, Serialize)]
pub(crate) struct RtpPcapRfc6184Stats {
    pub(crate) payload_type: u8,
    pub(crate) packets_analyzed: u32,
    pub(crate) invalid_packets: u32,
    pub(crate) marker_violations: u32,
    pub(crate) fu_a_invalid: u32,
    pub(crate) stap_a_invalid: u32,
}

#[derive(Debug, Default, Serialize)]
pub(crate) struct RtpPcapRfc3640Stats {
    pub(crate) payload_type: u8,
    pub(crate) packets_analyzed: u32,
    pub(crate) invalid_packets: u32,
    pub(crate) au_header_invalid: u32,
    pub(crate) au_size_invalid: u32,
    pub(crate) timestamp_anomalies: u32,
}

fn is_h264_vcl_nal_type(nal_type: u8) -> bool {
    matches!(nal_type, 1..=5)
}

pub(super) fn validate_h264_rtp_payload_rfc6184(
    payload: &[u8],
    marker: bool,
) -> (bool, bool, bool, bool) {
    // Returns: (valid, marker_violation, fu_a_invalid, stap_a_invalid)
    if payload.is_empty() {
        return (false, false, false, false);
    }
    let nal_unit_type = payload[0] & 0x1F;
    if nal_unit_type == 0 || nal_unit_type == 31 {
        return (false, false, false, false);
    }

    let mut marker_violation = false;
    let mut fu_a_invalid = false;
    let mut stap_a_invalid = false;

    match nal_unit_type {
        1..=23 => {
            if marker && !is_h264_vcl_nal_type(nal_unit_type) {
                marker_violation = true;
            }
            (true, marker_violation, false, false)
        }
        24 => {
            // STAP-A: [STAP-A header][2-byte size][NALU]...
            let mut i = 1usize;
            let mut units = 0u32;
            while i + 2 <= payload.len() {
                let size = u16::from_be_bytes([payload[i], payload[i + 1]]) as usize;
                i += 2;
                if size == 0 || i + size > payload.len() {
                    stap_a_invalid = true;
                    break;
                }
                let nalu_type = payload[i] & 0x1F;
                if nalu_type == 0 || nalu_type == 31 {
                    stap_a_invalid = true;
                    break;
                }
                i += size;
                units += 1;
            }
            // RFC 6184 requires at least one aggregation unit in a STAP-A.
            if i != payload.len() || units == 0 {
                stap_a_invalid = true;
            }
            (!stap_a_invalid, marker_violation, false, stap_a_invalid)
        }
        28 => {
            // FU-A: [FU indicator][FU header][fragment]
            if payload.len() < 2 {
                return (false, false, true, false);
            }
            let fu_header = payload[1];
            let start = (fu_header & 0x80) != 0;
            let end = (fu_header & 0x40) != 0;
            let reserved = (fu_header & 0x20) != 0;
            let original_type = fu_header & 0x1F;
            if reserved || (start && end) || original_type == 0 || original_type == 31 {
                fu_a_invalid = true;
            }
            if payload.len() < 3 {
                // Must contain at least one byte of fragment data.
                fu_a_invalid = true;
            }

            // Marker is advisory (access unit boundary). Count unexpected patterns but do not fail.
            if marker && !end {
                marker_violation = true;
            }

            (!fu_a_invalid, marker_violation, fu_a_invalid, false)
        }
        _ => {
            // For RFC 6184 completeness: allow other packet types but don't attempt deep validation.
            // If a server starts emitting these unexpectedly, fail so we can inspect.
            (false, false, false, false)
        }
    }
}

pub(super) fn validate_aac_rtp_payload_rfc3640(payload: &[u8]) -> (bool, bool, bool) {
    // Returns: (valid, au_header_invalid, au_size_invalid)
    if payload.len() < 4 {
        return (false, true, false);
    }

    let au_headers_len_bits = u16::from_be_bytes([payload[0], payload[1]]) as usize;
    if au_headers_len_bits == 0
        || !au_headers_len_bits.is_multiple_of(8)
        || !au_headers_len_bits.is_multiple_of(16)
    {
        return (false, true, false);
    }
    let au_headers_len_bytes = au_headers_len_bits.div_ceil(8);
    if payload.len() < 2 + au_headers_len_bytes {
        return (false, true, false);
    }
    let au_count = au_headers_len_bits / 16;
    if au_count == 0 {
        return (false, true, false);
    }

    let mut total_au_bytes = 0usize;
    for i in 0..au_count {
        let off = 2 + i * 2;
        if off + 2 > payload.len() {
            return (false, true, false);
        }
        let b0 = payload[off] as usize;
        let b1 = payload[off + 1] as usize;
        let au_size = (b0 << 5) | (b1 >> 3);
        if au_size == 0 {
            return (false, false, true);
        }
        total_au_bytes = total_au_bytes.saturating_add(au_size);
    }

    let data_len = payload.len() - (2 + au_headers_len_bytes);
    if total_au_bytes > data_len {
        return (false, false, true);
    }

    (true, false, false)
}

fn pick_best_payload_type<F>(rows: &[RtpTsharkRow], validator: F) -> Option<(u8, u32, u32)>
where
    F: Fn(&RtpTsharkRow) -> bool,
{
    let mut per_pt: HashMap<u8, (u32, u32)> = HashMap::new(); // (valid, total)
    for row in rows {
        let entry = per_pt.entry(row.payload_type).or_insert((0, 0));
        entry.1 = entry.1.saturating_add(1);
        if validator(row) {
            entry.0 = entry.0.saturating_add(1);
        }
    }

    per_pt
        .into_iter()
        .max_by_key(|(_pt, (valid, total))| (*valid, *total))
        .map(|(pt, (valid, total))| (pt, valid, total))
}

pub(crate) fn analyze_h264_rfc6184_from_rows(rows: &[RtpTsharkRow]) -> Result<RtpPcapRfc6184Stats> {
    let Some((pt, valid, total)) = pick_best_payload_type(rows, |row| {
        validate_h264_rtp_payload_rfc6184(&row.payload, row.marker).0
    }) else {
        bail!("no RTP packets found in pcap");
    };

    if total < super::MIN_PACKETS || valid == 0 {
        bail!(
            "insufficient H.264-like RTP packets (pt={}, valid={}, total={})",
            pt,
            valid,
            total
        );
    }
    let ratio = valid as f64 / total as f64;
    if ratio < super::MIN_VALID_RATIO {
        bail!(
            "could not classify H.264 RTP payload type (pt={}, valid_ratio={:.2}, valid={}, total={})",
            pt,
            ratio,
            valid,
            total
        );
    }

    let mut stats = RtpPcapRfc6184Stats {
        payload_type: pt,
        ..Default::default()
    };
    let mut last_marker_timestamp: Option<u32> = None;
    for row in rows.iter().filter(|r| r.payload_type == pt) {
        let (ok, marker_violation, fu_a_invalid, stap_a_invalid) =
            validate_h264_rtp_payload_rfc6184(&row.payload, row.marker);
        stats.packets_analyzed = stats.packets_analyzed.saturating_add(1);
        if !ok {
            stats.invalid_packets = stats.invalid_packets.saturating_add(1);
        }
        if marker_violation {
            stats.marker_violations = stats.marker_violations.saturating_add(1);
        }
        if row.marker {
            if last_marker_timestamp == Some(row.timestamp) {
                stats.marker_violations = stats.marker_violations.saturating_add(1);
            }
            last_marker_timestamp = Some(row.timestamp);
        }
        if fu_a_invalid {
            stats.fu_a_invalid = stats.fu_a_invalid.saturating_add(1);
        }
        if stap_a_invalid {
            stats.stap_a_invalid = stats.stap_a_invalid.saturating_add(1);
        }
    }
    Ok(stats)
}

pub(crate) fn analyze_aac_rfc3640_from_rows(rows: &[RtpTsharkRow]) -> Result<RtpPcapRfc3640Stats> {
    let Some((pt, valid, total)) =
        pick_best_payload_type(rows, |row| validate_aac_rtp_payload_rfc3640(&row.payload).0)
    else {
        bail!("no RTP packets found in pcap");
    };

    if total < super::MIN_PACKETS || valid == 0 {
        bail!(
            "insufficient AAC-like RTP packets (pt={}, valid={}, total={})",
            pt,
            valid,
            total
        );
    }
    let ratio = valid as f64 / total as f64;
    if ratio < super::MIN_VALID_RATIO {
        bail!(
            "could not classify AAC RTP payload type (pt={}, valid_ratio={:.2}, valid={}, total={})",
            pt,
            ratio,
            valid,
            total
        );
    }

    let mut stats = RtpPcapRfc3640Stats {
        payload_type: pt,
        ..Default::default()
    };

    let mut last_timestamp: Option<u32> = None;
    for row in rows.iter().filter(|r| r.payload_type == pt) {
        let (ok, au_header_invalid, au_size_invalid) =
            validate_aac_rtp_payload_rfc3640(&row.payload);
        stats.packets_analyzed = stats.packets_analyzed.saturating_add(1);
        if !ok {
            stats.invalid_packets = stats.invalid_packets.saturating_add(1);
        }
        if au_header_invalid {
            stats.au_header_invalid = stats.au_header_invalid.saturating_add(1);
        }
        if au_size_invalid {
            stats.au_size_invalid = stats.au_size_invalid.saturating_add(1);
        }

        if let Some(prev) = last_timestamp {
            let delta = row.timestamp.wrapping_sub(prev);
            if delta != 0 && delta % 1024 != 0 {
                stats.timestamp_anomalies = stats.timestamp_anomalies.saturating_add(1);
            }
        }
        last_timestamp = Some(row.timestamp);
    }
    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_rtp_row(
        payload_type: u8,
        marker: bool,
        timestamp: u32,
        seq: u16,
        payload: &[u8],
    ) -> RtpTsharkRow {
        RtpTsharkRow {
            payload_type,
            marker,
            timestamp,
            seq,
            ssrc: Some(0x11223344),
            ip_src: "192.168.2.198".to_string(),
            ip_dst: "192.168.2.10".to_string(),
            udp_src_port: Some(5004),
            udp_dst_port: Some(6000),
            payload: payload.to_vec(),
            time_epoch_sec: None,
        }
    }

    #[test]
    fn test_validate_h264_rfc6184_single_nal_ok() {
        // NAL type 5 (IDR)
        let payload = [0x65, 0x88, 0x99];
        let (ok, _marker_violation, fu_a_invalid, stap_a_invalid) =
            validate_h264_rtp_payload_rfc6184(&payload, true);
        assert!(ok);
        assert!(!fu_a_invalid);
        assert!(!stap_a_invalid);
    }

    #[test]
    fn test_validate_h264_rfc6184_stap_a_ok() {
        // STAP-A with SPS (type 7) and PPS (type 8)
        let payload = [
            0x78, // F=0 NRI=3 Type=24
            0x00, 0x02, 0x67, 0xAA, // len=2, SPS
            0x00, 0x02, 0x68, 0xBB, // len=2, PPS
        ];
        let (ok, _marker_violation, fu_a_invalid, stap_a_invalid) =
            validate_h264_rtp_payload_rfc6184(&payload, false);
        assert!(ok);
        assert!(!fu_a_invalid);
        assert!(!stap_a_invalid);
    }

    #[test]
    fn test_validate_h264_rfc6184_fu_a_ok() {
        // FU-A start fragment for NAL type 5 (IDR)
        let payload = [0x7C, 0x85, 0x11, 0x22];
        let (ok, _marker_violation, fu_a_invalid, stap_a_invalid) =
            validate_h264_rtp_payload_rfc6184(&payload, false);
        assert!(ok);
        assert!(!fu_a_invalid);
        assert!(!stap_a_invalid);
    }

    #[test]
    fn test_validate_h264_rfc6184_marker_violation_non_vcl_single_nal() {
        let payload = [0x67, 0xAA]; // SPS with marker set.
        let (ok, marker_violation, fu_a_invalid, stap_a_invalid) =
            validate_h264_rtp_payload_rfc6184(&payload, true);
        assert!(ok);
        assert!(marker_violation);
        assert!(!fu_a_invalid);
        assert!(!stap_a_invalid);
    }

    #[test]
    fn test_validate_h264_rfc6184_fu_a_marker_violation_when_not_end() {
        let payload = [0x7C, 0x85, 0x11]; // Start fragment, not end.
        let (ok, marker_violation, fu_a_invalid, stap_a_invalid) =
            validate_h264_rtp_payload_rfc6184(&payload, true);
        assert!(ok);
        assert!(marker_violation);
        assert!(!fu_a_invalid);
        assert!(!stap_a_invalid);
    }

    #[test]
    fn test_validate_h264_rfc6184_fu_a_malformed_short_and_invalid_header() {
        let too_short = [0x7C];
        let (ok, _marker_violation, fu_a_invalid, stap_a_invalid) =
            validate_h264_rtp_payload_rfc6184(&too_short, false);
        assert!(!ok);
        assert!(fu_a_invalid);
        assert!(!stap_a_invalid);

        let invalid_header = [0x7C, 0xE0, 0x11]; // start+end+reserved and orig type 0.
        let (ok, _marker_violation, fu_a_invalid, stap_a_invalid) =
            validate_h264_rtp_payload_rfc6184(&invalid_header, false);
        assert!(!ok);
        assert!(fu_a_invalid);
        assert!(!stap_a_invalid);
    }

    #[test]
    fn test_validate_h264_rfc6184_stap_a_malformed_variants() {
        let zero_size = [0x78, 0x00, 0x00];
        let (ok, _marker_violation, fu_a_invalid, stap_a_invalid) =
            validate_h264_rtp_payload_rfc6184(&zero_size, false);
        assert!(!ok);
        assert!(!fu_a_invalid);
        assert!(stap_a_invalid);

        let overflow_size = [0x78, 0x00, 0x04, 0x67, 0xAA];
        let (ok, _marker_violation, fu_a_invalid, stap_a_invalid) =
            validate_h264_rtp_payload_rfc6184(&overflow_size, false);
        assert!(!ok);
        assert!(!fu_a_invalid);
        assert!(stap_a_invalid);

        let trailing_bytes = [0x78, 0x00, 0x01, 0x67, 0xFF];
        let (ok, _marker_violation, fu_a_invalid, stap_a_invalid) =
            validate_h264_rtp_payload_rfc6184(&trailing_bytes, false);
        assert!(!ok);
        assert!(!fu_a_invalid);
        assert!(stap_a_invalid);
    }

    #[test]
    fn test_validate_aac_rfc3640_single_au_ok() {
        // AU-headers-length = 16 bits, AU-size=4 bytes, index=0, then 4 bytes data
        let payload = [0x00, 0x10, 0x00, 0x20, 0xDE, 0xAD, 0xBE, 0xEF];
        let (ok, au_header_invalid, au_size_invalid) = validate_aac_rtp_payload_rfc3640(&payload);
        assert!(ok);
        assert!(!au_header_invalid);
        assert!(!au_size_invalid);
    }

    #[test]
    fn test_validate_aac_rfc3640_malformed_au_header() {
        let zero_header_len = [0x00, 0x00, 0x00, 0x00];
        let (ok, au_header_invalid, au_size_invalid) =
            validate_aac_rtp_payload_rfc3640(&zero_header_len);
        assert!(!ok);
        assert!(au_header_invalid);
        assert!(!au_size_invalid);

        let non_multiple_of_16 = [0x00, 0x08, 0x00, 0x00];
        let (ok, au_header_invalid, au_size_invalid) =
            validate_aac_rtp_payload_rfc3640(&non_multiple_of_16);
        assert!(!ok);
        assert!(au_header_invalid);
        assert!(!au_size_invalid);

        let headers_too_short = [0x00, 0x20, 0x00, 0x10];
        let (ok, au_header_invalid, au_size_invalid) =
            validate_aac_rtp_payload_rfc3640(&headers_too_short);
        assert!(!ok);
        assert!(au_header_invalid);
        assert!(!au_size_invalid);
    }

    #[test]
    fn test_validate_aac_rfc3640_zero_au_size_is_invalid() {
        let payload = [0x00, 0x10, 0x00, 0x00, 0xAA];
        let (ok, au_header_invalid, au_size_invalid) = validate_aac_rtp_payload_rfc3640(&payload);
        assert!(!ok);
        assert!(!au_header_invalid);
        assert!(au_size_invalid);
    }

    #[test]
    fn test_validate_aac_rfc3640_au_size_too_large() {
        // AU-size=16 bytes but only 1 byte data.
        let payload = [0x00, 0x10, 0x00, 0x80, 0x00];
        let (ok, _au_header_invalid, au_size_invalid) = validate_aac_rtp_payload_rfc3640(&payload);
        assert!(!ok);
        assert!(au_size_invalid);
    }

    #[test]
    fn test_analyze_h264_rfc6184_from_rows_no_rows_returns_error() {
        let err = analyze_h264_rfc6184_from_rows(&[]).unwrap_err();
        assert!(err.to_string().contains("no RTP packets found"));
    }

    #[test]
    fn test_analyze_h264_rfc6184_from_rows_insufficient_packets_returns_error() {
        let rows = vec![
            mk_rtp_row(96, true, 1000, 1, &[0x65, 0x11]),
            mk_rtp_row(96, true, 2000, 2, &[0x65, 0x22]),
            mk_rtp_row(96, true, 3000, 3, &[0x65, 0x33]),
        ];
        let err = analyze_h264_rfc6184_from_rows(&rows).unwrap_err();
        assert!(
            err.to_string()
                .contains("insufficient H.264-like RTP packets")
        );
    }

    #[test]
    fn test_analyze_h264_rfc6184_from_rows_low_valid_ratio_returns_error() {
        let mut rows = Vec::new();
        for i in 0..7u16 {
            rows.push(mk_rtp_row(
                96,
                false,
                1000 + (i as u32) * 3000,
                100 + i,
                &[0x61, 0xAA],
            ));
        }
        for i in 0..3u16 {
            rows.push(mk_rtp_row(
                96,
                false,
                40000 + (i as u32) * 3000,
                200 + i,
                &[0x00],
            ));
        }
        let err = analyze_h264_rfc6184_from_rows(&rows).unwrap_err();
        assert!(
            err.to_string()
                .contains("could not classify H.264 RTP payload type")
        );
    }

    #[test]
    fn test_analyze_h264_rfc6184_from_rows_collects_violation_counters() {
        let rows = vec![
            mk_rtp_row(96, true, 1000, 1, &[0x65, 0x11]),
            mk_rtp_row(96, true, 2000, 2, &[0x67, 0x11]),
            mk_rtp_row(96, true, 2000, 3, &[0x68, 0x11]),
            mk_rtp_row(96, true, 3000, 4, &[0x7C, 0x85, 0x11]),
            mk_rtp_row(96, false, 4000, 5, &[0x7C]),
            mk_rtp_row(96, false, 5000, 6, &[0x78, 0x00, 0x00]),
            mk_rtp_row(96, false, 6000, 7, &[0x61, 0x11]),
            mk_rtp_row(96, false, 7000, 8, &[0x61, 0x11]),
            mk_rtp_row(96, false, 8000, 9, &[0x61, 0x11]),
            mk_rtp_row(96, false, 9000, 10, &[0x61, 0x11]),
        ];

        let stats = analyze_h264_rfc6184_from_rows(&rows).unwrap();
        assert_eq!(stats.payload_type, 96);
        assert_eq!(stats.packets_analyzed, 10);
        assert_eq!(stats.invalid_packets, 2);
        assert_eq!(stats.marker_violations, 4);
        assert_eq!(stats.fu_a_invalid, 1);
        assert_eq!(stats.stap_a_invalid, 1);
    }

    #[test]
    fn test_analyze_aac_rfc3640_from_rows_no_rows_returns_error() {
        let err = analyze_aac_rfc3640_from_rows(&[]).unwrap_err();
        assert!(err.to_string().contains("no RTP packets found"));
    }

    #[test]
    fn test_analyze_aac_rfc3640_from_rows_insufficient_packets_returns_error() {
        let rows = vec![
            mk_rtp_row(97, false, 0, 1, &[0x00, 0x10, 0x00, 0x10, 0xAA, 0xBB]),
            mk_rtp_row(97, false, 1024, 2, &[0x00, 0x10, 0x00, 0x10, 0xAA, 0xBB]),
            mk_rtp_row(97, false, 2048, 3, &[0x00, 0x10, 0x00, 0x10, 0xAA, 0xBB]),
        ];
        let err = analyze_aac_rfc3640_from_rows(&rows).unwrap_err();
        assert!(
            err.to_string()
                .contains("insufficient AAC-like RTP packets")
        );
    }

    #[test]
    fn test_analyze_aac_rfc3640_from_rows_low_valid_ratio_returns_error() {
        let mut rows = Vec::new();
        for i in 0..7u16 {
            rows.push(mk_rtp_row(
                97,
                false,
                (i as u32) * 1024,
                1 + i,
                &[0x00, 0x10, 0x00, 0x10, 0xAA, 0xBB],
            ));
        }
        for i in 0..3u16 {
            rows.push(mk_rtp_row(
                97,
                false,
                20000 + (i as u32) * 1024,
                100 + i,
                &[0x00, 0x00, 0x00, 0x00],
            ));
        }
        let err = analyze_aac_rfc3640_from_rows(&rows).unwrap_err();
        assert!(
            err.to_string()
                .contains("could not classify AAC RTP payload type")
        );
    }

    #[test]
    fn test_analyze_aac_rfc3640_from_rows_collects_error_and_timestamp_counters() {
        let rows = vec![
            mk_rtp_row(97, false, 0, 1, &[0x00, 0x10, 0x00, 0x10, 0xAA, 0xBB]),
            mk_rtp_row(97, false, 1024, 2, &[0x00, 0x10, 0x00, 0x10, 0xAA, 0xBB]),
            mk_rtp_row(97, false, 2048, 3, &[0x00, 0x10, 0x00, 0x10, 0xAA, 0xBB]),
            mk_rtp_row(97, false, 3072, 4, &[0x00, 0x10, 0x00, 0x10, 0xAA, 0xBB]),
            mk_rtp_row(97, false, 4096, 5, &[0x00, 0x10, 0x00, 0x00, 0xAA]),
            mk_rtp_row(97, false, 5120, 6, &[0x00, 0x08, 0x00, 0x00]),
            mk_rtp_row(97, false, 6144, 7, &[0x00, 0x10, 0x00, 0x10, 0xAA, 0xBB]),
            mk_rtp_row(97, false, 7000, 8, &[0x00, 0x10, 0x00, 0x10, 0xAA, 0xBB]),
            mk_rtp_row(97, false, 8024, 9, &[0x00, 0x10, 0x00, 0x10, 0xAA, 0xBB]),
            mk_rtp_row(97, false, 9048, 10, &[0x00, 0x10, 0x00, 0x10, 0xAA, 0xBB]),
        ];

        let stats = analyze_aac_rfc3640_from_rows(&rows).unwrap();
        assert_eq!(stats.payload_type, 97);
        assert_eq!(stats.packets_analyzed, 10);
        assert_eq!(stats.invalid_packets, 2);
        assert_eq!(stats.au_header_invalid, 1);
        assert_eq!(stats.au_size_invalid, 1);
        assert_eq!(stats.timestamp_anomalies, 1);
    }
}
