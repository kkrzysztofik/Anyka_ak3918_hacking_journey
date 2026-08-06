//! Parsing `tshark -T fields` output into RTP rows.

use anyhow::{Context, Result, bail};
use std::path::Path;
use std::process::{Command, Stdio};
use tracing::warn;

use crate::util::tail_lossy;

fn parse_tshark_hex_bytes(raw: &str) -> Result<Vec<u8>> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed == "<none>" {
        return Ok(Vec::new());
    }

    let mut out = Vec::with_capacity(trimmed.len().saturating_div(2));
    let mut hi: Option<u8> = None;
    for ch in trimmed.chars() {
        if matches!(ch, ':' | ' ' | '\t' | '\r' | '\n') {
            continue;
        }
        let v = ch
            .to_digit(16)
            .map(|d| d as u8)
            .with_context(|| format!("invalid hex in rtp.payload: {}", trimmed))?;
        if let Some(h) = hi.take() {
            out.push((h << 4) | v);
        } else {
            hi = Some(v);
        }
    }
    if hi.is_some() {
        bail!("odd-length hex in rtp.payload: {}", trimmed);
    }
    Ok(out)
}

#[derive(Debug)]
pub(crate) struct RtpTsharkRow {
    pub(crate) payload_type: u8,
    pub(crate) marker: bool,
    pub(crate) timestamp: u32,
    pub(crate) seq: u16,
    pub(crate) ssrc: Option<u32>,
    pub(crate) ip_src: String,
    #[allow(dead_code)]
    pub(crate) ip_dst: String,
    #[allow(dead_code)]
    pub(crate) udp_src_port: Option<u16>,
    pub(crate) udp_dst_port: Option<u16>,
    pub(crate) payload: Vec<u8>,
    /// Wall-clock arrival time (seconds since epoch) of the packet, if captured.
    pub(crate) time_epoch_sec: Option<f64>,
}

fn parse_tshark_u32(raw: &str) -> Option<u32> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Some(rest) = trimmed.strip_prefix("0x") {
        return u32::from_str_radix(rest, 16).ok();
    }
    if trimmed
        .chars()
        .any(|c| c.is_ascii_hexdigit() && c.is_ascii_alphabetic())
    {
        return u32::from_str_radix(trimmed, 16).ok();
    }
    trimmed.parse::<u32>().ok()
}

fn parse_tshark_u16(raw: &str) -> Option<u16> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    trimmed.parse::<u16>().ok()
}

fn parse_tshark_f64(raw: &str) -> Option<f64> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    trimmed.parse::<f64>().ok()
}

fn parse_tshark_rtp_row_line(line: &str, line_no: usize) -> Result<Option<RtpTsharkRow>> {
    // Field order must match `tshark_extract_rtp_rows` (frame.time_epoch appended
    // after rtp.payload, so the strict payload field stays at index 9).
    // Keep this parser separate so unit tests can validate row parsing without invoking tshark.
    let parts: Vec<&str> = line.split('\t').collect();
    if parts.len() < 10 {
        return Ok(None);
    }

    let Ok(payload_type) = parts[0].trim().parse::<u8>() else {
        return Ok(None);
    };
    let marker = parts[1].trim() == "1";
    let Ok(timestamp) = parts[2].trim().parse::<u32>() else {
        return Ok(None);
    };
    let Ok(seq) = parts[3].trim().parse::<u16>() else {
        return Ok(None);
    };
    let ssrc = parse_tshark_u32(parts[4]);
    let ip_src = parts[5].trim().to_string();
    let ip_dst = parts[6].trim().to_string();
    let udp_src_port = parse_tshark_u16(parts[7]);
    let udp_dst_port = parse_tshark_u16(parts[8]);

    let payload = parse_tshark_hex_bytes(parts[9])
        .with_context(|| format!("parse rtp.payload at line {}", line_no))?;
    if payload.is_empty() {
        return Ok(None);
    }

    let time_epoch_sec = parts.get(10).and_then(|raw| parse_tshark_f64(raw));

    Ok(Some(RtpTsharkRow {
        payload_type,
        marker,
        timestamp,
        seq,
        ssrc,
        ip_src,
        ip_dst,
        udp_src_port,
        udp_dst_port,
        payload,
        time_epoch_sec,
    }))
}

pub(crate) fn tshark_extract_rtp_rows(pcap_path: &Path) -> Result<Vec<RtpTsharkRow>> {
    let out = Command::new("tshark")
        .args(["-o", "rtp.heuristic_rtp:TRUE"])
        .arg("-r")
        .arg(pcap_path)
        .args([
            "-Y",
            "rtp",
            "-T",
            "fields",
            "-E",
            "separator=\t",
            "-E",
            "occurrence=f",
            "-e",
            "rtp.p_type",
            "-e",
            "rtp.marker",
            "-e",
            "rtp.timestamp",
            "-e",
            "rtp.seq",
            "-e",
            "rtp.ssrc",
            "-e",
            "ip.src",
            "-e",
            "ip.dst",
            "-e",
            "udp.srcport",
            "-e",
            "udp.dstport",
            "-e",
            "rtp.payload",
            "-e",
            "frame.time_epoch",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .context("tshark -T fields")?;

    if !out.status.success() {
        bail!(
            "tshark parse failed (code={:?}): {}",
            out.status.code(),
            tail_lossy(String::from_utf8_lossy(&out.stderr).trim(), 1200)
        );
    }

    let stdout = String::from_utf8_lossy(&out.stdout);
    let mut rows = Vec::new();
    for (idx, line) in stdout.lines().enumerate() {
        match parse_tshark_rtp_row_line(line, idx + 1) {
            Ok(Some(row)) => rows.push(row),
            Ok(None) => {}
            Err(e) => warn!(error = %e, "skipping unparsable tshark rtp row"),
        }
    }
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tshark_hex_bytes_empty_ok() {
        let out = parse_tshark_hex_bytes("  ").unwrap();
        assert!(out.is_empty());
    }

    #[test]
    fn test_parse_tshark_hex_bytes_colon_separated() {
        let out = parse_tshark_hex_bytes("7c:85:01:ff").unwrap();
        assert_eq!(out, vec![0x7c, 0x85, 0x01, 0xff]);
    }

    #[test]
    fn test_parse_tshark_hex_bytes_contiguous() {
        let out = parse_tshark_hex_bytes("7c8501ff").unwrap();
        assert_eq!(out, vec![0x7c, 0x85, 0x01, 0xff]);
    }

    #[test]
    fn test_parse_tshark_rtp_row_line_contiguous_payload_and_hex_ssrc() {
        let line =
            "96\t1\t9000\t123\t0x11223344\t192.168.2.198\t192.168.2.10\t5004\t6000\t7c8501ff";
        let row = parse_tshark_rtp_row_line(line, 1).unwrap().unwrap();
        assert_eq!(row.payload_type, 96);
        assert!(row.marker);
        assert_eq!(row.timestamp, 9000);
        assert_eq!(row.seq, 123);
        assert_eq!(row.ssrc, Some(0x11223344));
        assert_eq!(row.ip_src, "192.168.2.198");
        assert_eq!(row.ip_dst, "192.168.2.10");
        assert_eq!(row.udp_src_port, Some(5004));
        assert_eq!(row.udp_dst_port, Some(6000));
        assert_eq!(row.payload, vec![0x7c, 0x85, 0x01, 0xff]);
    }

    #[test]
    fn test_parse_tshark_u32_accepts_hex_and_decimal() {
        assert_eq!(parse_tshark_u32("0x10"), Some(16));
        assert_eq!(parse_tshark_u32("DEADBEEF"), Some(0xDEADBEEF));
        assert_eq!(parse_tshark_u32("1234"), Some(1234));
    }

    #[test]
    fn test_parse_tshark_u32_invalid_and_empty_return_none() {
        assert_eq!(parse_tshark_u32(""), None);
        assert_eq!(parse_tshark_u32("  \t"), None);
        assert_eq!(parse_tshark_u32("xyz123"), None);
        assert_eq!(parse_tshark_u32("0xZZ"), None);
    }

    #[test]
    fn test_parse_tshark_rtp_row_line_insufficient_fields_returns_none() {
        let row = parse_tshark_rtp_row_line("96\t1\t9000", 42).unwrap();
        assert!(row.is_none());
    }

    #[test]
    fn test_parse_tshark_rtp_row_line_invalid_numeric_fields_return_none() {
        let bad_payload_type = "x\t1\t9000\t123\t1\t1.1.1.1\t2.2.2.2\t5004\t6000\t65";
        assert!(
            parse_tshark_rtp_row_line(bad_payload_type, 1)
                .unwrap()
                .is_none()
        );

        let bad_timestamp = "96\t1\tnot_a_number\t123\t1\t1.1.1.1\t2.2.2.2\t5004\t6000\t65";
        assert!(
            parse_tshark_rtp_row_line(bad_timestamp, 2)
                .unwrap()
                .is_none()
        );

        let bad_seq = "96\t1\t9000\tNaN\t1\t1.1.1.1\t2.2.2.2\t5004\t6000\t65";
        assert!(parse_tshark_rtp_row_line(bad_seq, 3).unwrap().is_none());
    }

    #[test]
    fn test_parse_tshark_rtp_row_line_invalid_payload_hex_returns_err() {
        let line = "96\t1\t9000\t123\t1\t1.1.1.1\t2.2.2.2\t5004\t6000\t7c:zz";
        let err = parse_tshark_rtp_row_line(line, 7).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("parse rtp.payload at line 7"));
    }

    #[test]
    fn test_parse_tshark_rtp_row_line_empty_payload_returns_none() {
        let line = "96\t1\t9000\t123\t1\t1.1.1.1\t2.2.2.2\t5004\t6000\t<none>";
        let row = parse_tshark_rtp_row_line(line, 8).unwrap();
        assert!(row.is_none());
    }

    #[test]
    fn test_parse_tshark_rtp_row_line_with_epoch() {
        let line = "96\t1\t9000\t123\t0x11223344\t192.168.2.198\t192.168.2.10\t5004\t6000\t7c8501ff\t1700000000.123456";
        let row = parse_tshark_rtp_row_line(line, 1).unwrap().unwrap();
        assert_eq!(row.time_epoch_sec, Some(1700000000.123456));
    }

    #[test]
    fn test_parse_tshark_rtp_row_line_missing_epoch_is_none() {
        let line =
            "96\t1\t9000\t123\t0x11223344\t192.168.2.198\t192.168.2.10\t5004\t6000\t7c8501ff";
        let row = parse_tshark_rtp_row_line(line, 1).unwrap().unwrap();
        assert_eq!(row.time_epoch_sec, None);
    }
}
