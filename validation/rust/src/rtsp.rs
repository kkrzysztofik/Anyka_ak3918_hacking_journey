//! RTSP protocol validation and harness (Retina, ffmpeg, tshark).

use anyhow::{Context, Result, anyhow, bail};
use chrono::Utc;
use ffmpeg_sidecar::command::FfmpegCommand;
use ffmpeg_sidecar::event::{FfmpegEvent, FfmpegProgress};
use futures_util::StreamExt;
use retina::client::{
    Credentials, InitialTimestampPolicy, PlayOptions, Session, SessionOptions, SetupOptions,
    Transport,
};
use retina::codec::{CodecItem, ParametersRef};
use rtshark::RTSharkBuilder;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::time::{Instant, timeout};
use tracing::{debug, info, trace, warn};
use url::Url;

use crate::config::{Args, EffectiveConfig, InitialTimestampPolicyArg, TransportArg};
use crate::report::{StreamInfo, TestResult, TestRun, ValidationReport};
use crate::util::{MAX_TOOL_LOG_BYTES, tail_lossy, write_bytes_tail};

const PROBE_DEMUX_ERROR_TOLERANCE: u32 = 3;
const PROTOCOL_SEQUENCE_CAPTURE_MAX_DURATION_SEC: u64 = 12;

pub(crate) fn rtsp_url(host: &str, port: u16, stream: &str) -> String {
    format!("rtsp://{}:{}{}", host, port, stream)
}

fn redact_url_credentials(raw_url: &str) -> String {
    let Ok(mut parsed) = Url::parse(raw_url) else {
        return raw_url.to_string();
    };
    if parsed.username().is_empty() && parsed.password().is_none() {
        return raw_url.to_string();
    }
    if parsed.set_username("REDACTED").is_err() {
        return raw_url.to_string();
    }
    if parsed.set_password(Some("REDACTED")).is_err() {
        return raw_url.to_string();
    }
    parsed.to_string()
}

fn rtsp_url_with_credentials(
    host: &str,
    port: u16,
    stream: &str,
    username: Option<&str>,
    password: Option<&str>,
) -> Result<String> {
    let url = rtsp_url(host, port, stream);
    match (username, password) {
        (None, None) => Ok(url),
        (Some(username), Some(password)) => {
            let mut parsed =
                Url::parse(&url).with_context(|| format!("invalid RTSP URL: {}", url))?;
            parsed
                .set_username(username)
                .map_err(|_| anyhow!("failed to set RTSP URL username"))?;
            parsed
                .set_password(Some(password))
                .map_err(|_| anyhow!("failed to set RTSP URL password"))?;
            Ok(parsed.to_string())
        }
        _ => bail!("RTSP credentials must include both username and password"),
    }
}

fn regex_escape_for_pgrep(raw: &str) -> String {
    let mut escaped = String::with_capacity(raw.len());
    for ch in raw.chars() {
        match ch {
            '.' | '\\' | '+' | '*' | '?' | '[' | ']' | '^' | '$' | '(' | ')' | '{' | '}' | '|' => {
                escaped.push('\\');
                escaped.push(ch);
            }
            _ => escaped.push(ch),
        }
    }
    escaped
}

fn process_match_needle(rtsp_url: &str) -> String {
    if let Ok(parsed) = Url::parse(rtsp_url) {
        let host = parsed.host_str().unwrap_or_default();
        let port = parsed.port_or_known_default().unwrap_or(554);
        return format!("{}:{}{}", host, port, parsed.path());
    }
    rtsp_url.to_string()
}

fn terminate_processes_by_pattern(pattern: &str) -> Result<u32> {
    let output = Command::new("pgrep")
        .arg("-f")
        .arg(pattern)
        .output()
        .with_context(|| format!("pgrep -f {}", pattern))?;

    if !output.status.success() {
        if output.status.code() == Some(1) {
            return Ok(0);
        }
        bail!(
            "pgrep failed for pattern={} status={:?}",
            pattern,
            output.status.code()
        );
    }

    let mut killed = 0u32;
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        let pid = line.trim();
        if pid.is_empty() {
            continue;
        }
        match Command::new("kill").arg("-TERM").arg(pid).status() {
            Ok(status) if status.success() => {
                killed = killed.saturating_add(1);
            }
            Ok(status) => {
                warn!(pid = %pid, status = ?status.code(), "failed to terminate stale process");
            }
            Err(e) => {
                warn!(pid = %pid, error = %e, "failed to execute kill for stale process");
            }
        }
    }

    Ok(killed)
}

fn cleanup_timed_out_media_processes(rtsp_url: &str, step_name: &str) {
    let needle = process_match_needle(rtsp_url);
    for process_name in ["ffmpeg", "ffprobe"] {
        let pattern = format!("{}.*{}", process_name, regex_escape_for_pgrep(&needle));
        match terminate_processes_by_pattern(&pattern) {
            Ok(killed) if killed > 0 => {
                warn!(
                    step = %step_name,
                    process = %process_name,
                    killed,
                    needle = %needle,
                    "terminated stale media processes after harness timeout"
                );
            }
            Ok(_) => {
                debug!(
                    step = %step_name,
                    process = %process_name,
                    needle = %needle,
                    "no stale media processes found after harness timeout"
                );
            }
            Err(e) => {
                warn!(
                    step = %step_name,
                    process = %process_name,
                    needle = %needle,
                    error = %e,
                    "failed stale process cleanup after harness timeout"
                );
            }
        }
    }
}

pub(crate) fn to_retina_transport(arg: TransportArg) -> Transport {
    match arg {
        TransportArg::Tcp => Transport::Tcp(Default::default()),
        TransportArg::Udp => Transport::Udp(Default::default()),
    }
}

fn to_retina_initial_timestamp_policy(arg: InitialTimestampPolicyArg) -> InitialTimestampPolicy {
    match arg {
        InitialTimestampPolicyArg::Default => InitialTimestampPolicy::Default,
        InitialTimestampPolicyArg::Require => InitialTimestampPolicy::Require,
        InitialTimestampPolicyArg::Ignore => InitialTimestampPolicy::Ignore,
        InitialTimestampPolicyArg::Permissive => InitialTimestampPolicy::Permissive,
    }
}

fn parse_rtsp_method_from_meta<'a>(meta_name: &str, meta_value: &'a str) -> Option<&'a str> {
    if meta_name == "rtsp.method" {
        return Some(meta_value);
    }
    if meta_name == "rtsp.request" {
        return meta_value.split_whitespace().next();
    }
    None
}

fn parse_status_code(value: &str) -> Option<u32> {
    value
        .split_whitespace()
        .find_map(|token| token.parse::<u32>().ok())
}

fn parse_rtsp_status_code_from_meta(meta_name: &str, meta_value: &str) -> Option<u32> {
    if meta_name == "rtsp.status_code" || meta_name == "rtsp.status" {
        return parse_status_code(meta_value);
    }
    if meta_name == "rtsp.response" {
        return parse_status_code(meta_value);
    }
    None
}

#[derive(Debug, Clone, Default, Serialize)]
struct RtpPcapRfc6184Stats {
    payload_type: u8,
    packets_analyzed: u32,
    invalid_packets: u32,
    marker_violations: u32,
    fu_a_invalid: u32,
    stap_a_invalid: u32,
}

#[derive(Debug, Clone, Default, Serialize)]
struct RtpPcapRfc3640Stats {
    payload_type: u8,
    packets_analyzed: u32,
    invalid_packets: u32,
    au_header_invalid: u32,
    au_size_invalid: u32,
    timestamp_anomalies: u32,
}

#[derive(Debug)]
struct HarnessPacketLossResult {
    video: Option<HarnessRtpLossMetric>,
    audio: Option<HarnessRtpLossMetric>,
    h264_rfc6184: Option<std::result::Result<RtpPcapRfc6184Stats, String>>,
    aac_rfc3640: Option<std::result::Result<RtpPcapRfc3640Stats, String>>,
}

#[derive(Debug, Clone, Default, Serialize)]
struct HarnessRtpLossMetric {
    rtp_packets: u32,
    packet_loss: u32,
    loss_percent: f64,
    payload_type: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    ssrc: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct RtpStreamKey {
    payload_type: u8,
    ssrc: Option<u32>,
    dst_port: Option<u16>,
}

#[derive(Debug, Clone)]
struct RtpStreamStats {
    key: RtpStreamKey,
    rows: Vec<RtpTsharkRow>,
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

fn compute_stream_loss_metric(stats: &RtpStreamStats) -> HarnessRtpLossMetric {
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

fn pick_primary_video_stream(streams: &[RtpStreamStats]) -> Option<&RtpStreamStats> {
    streams
        .iter()
        .filter(|s| is_reasonably_h264(s))
        .max_by_key(|s| (s.valid_h264, s.rows.len()))
        .or_else(|| streams.iter().max_by_key(|s| s.rows.len()))
}

fn pick_primary_audio_stream(
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

fn group_rtp_rows_by_stream(rows: Vec<RtpTsharkRow>) -> Vec<RtpStreamStats> {
    use std::collections::HashMap;
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

fn stream_info_from_retina(s: &retina::client::Stream) -> StreamInfo {
    StreamInfo {
        media: s.media().to_string(),
        encoding_name: s.encoding_name().to_string(),
        control_present: s.control().is_some(),
    }
}

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

fn is_h264_vcl_nal_type(nal_type: u8) -> bool {
    matches!(nal_type, 1..=5)
}

fn validate_h264_rtp_payload_rfc6184(payload: &[u8], marker: bool) -> (bool, bool, bool, bool) {
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
            }
            if i != payload.len() {
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

fn validate_aac_rtp_payload_rfc3640(payload: &[u8]) -> (bool, bool, bool) {
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

#[derive(Debug, Clone)]
struct RtpTsharkRow {
    payload_type: u8,
    marker: bool,
    timestamp: u32,
    seq: u16,
    ssrc: Option<u32>,
    ip_src: String,
    #[allow(dead_code)]
    ip_dst: String,
    #[allow(dead_code)]
    udp_src_port: Option<u16>,
    udp_dst_port: Option<u16>,
    payload: Vec<u8>,
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

fn parse_tshark_rtp_row_line(line: &str, line_no: usize) -> Result<Option<RtpTsharkRow>> {
    // Field order must match `tshark_extract_rtp_rows`.
    // Keep this parser separate so unit tests can validate row parsing without invoking tshark.
    let parts: Vec<&str> = line.split('\t').collect();
    if parts.len() < 9 {
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

    let payload = if parts.len() >= 10 {
        parse_tshark_hex_bytes(parts[9])
            .with_context(|| format!("parse rtp.payload at line {}", line_no))?
    } else {
        Vec::new()
    };
    if payload.is_empty() {
        return Ok(None);
    }

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
    }))
}

fn tshark_extract_rtp_rows(pcap_path: &Path) -> Result<Vec<RtpTsharkRow>> {
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
        if let Some(row) = parse_tshark_rtp_row_line(line, idx + 1)? {
            rows.push(row);
        }
    }
    Ok(rows)
}

fn pick_best_payload_type<F>(rows: &[RtpTsharkRow], validator: F) -> Option<(u8, u32, u32)>
where
    F: Fn(&RtpTsharkRow) -> bool,
{
    use std::collections::HashMap;
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

fn analyze_h264_rfc6184_from_rows(rows: &[RtpTsharkRow]) -> Result<RtpPcapRfc6184Stats> {
    const MIN_PACKETS: u32 = 10;
    const MIN_VALID_RATIO: f64 = 0.80;

    let Some((pt, valid, total)) = pick_best_payload_type(rows, |row| {
        validate_h264_rtp_payload_rfc6184(&row.payload, row.marker).0
    }) else {
        bail!("no RTP packets found in pcap");
    };

    if total < MIN_PACKETS || valid == 0 {
        bail!(
            "insufficient H.264-like RTP packets (pt={}, valid={}, total={})",
            pt,
            valid,
            total
        );
    }
    let ratio = valid as f64 / total as f64;
    if ratio < MIN_VALID_RATIO {
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

fn analyze_aac_rfc3640_from_rows(rows: &[RtpTsharkRow]) -> Result<RtpPcapRfc3640Stats> {
    const MIN_PACKETS: u32 = 10;
    const MIN_VALID_RATIO: f64 = 0.80;

    let Some((pt, valid, total)) =
        pick_best_payload_type(rows, |row| validate_aac_rtp_payload_rfc3640(&row.payload).0)
    else {
        bail!("no RTP packets found in pcap");
    };

    if total < MIN_PACKETS || valid == 0 {
        bail!(
            "insufficient AAC-like RTP packets (pt={}, valid={}, total={})",
            pt,
            valid,
            total
        );
    }
    let ratio = valid as f64 / total as f64;
    if ratio < MIN_VALID_RATIO {
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

pub fn result_ok(r: &TestResult) -> bool {
    match r {
        TestResult::Pass { .. } => true,
        TestResult::Fail { .. } => false,
        TestResult::Metric { pass, .. } => *pass,
    }
}

pub fn critical_proto_failed(tests: &[TestResult]) -> bool {
    tests.iter().any(|t| {
        if let TestResult::Fail { name, .. } = t {
            name == "describe_ok" || name == "play_ok" || name.starts_with("setup_stream_")
        } else {
            false
        }
    })
}

pub fn empty_report(test_run: TestRun, tests: Vec<TestResult>) -> ValidationReport {
    ValidationReport {
        test_run,
        tests,
        summary: crate::report::Summary {
            total_tests: 0,
            passed: 0,
            failed: 0,
            overall_pass: false,
        },
        artifacts_dir: None,
        telemetry: None,
        telemetry_before_shutdown: None,
    }
}

/// Returns true if measured bitrate is within tolerance_percent of expected.
pub fn bitrate_within_tolerance(
    measured_kbps: f64,
    expected_kbps: f64,
    tolerance_percent: u32,
) -> bool {
    if expected_kbps <= 0.0 {
        return true;
    }
    let tol = tolerance_percent as f64 / 100.0;
    (measured_kbps - expected_kbps).abs() / expected_kbps <= tol
}

/// Returns true if measured fps is within tolerance_percent of expected.
pub fn fps_within_tolerance(measured: f64, expected: f64, tolerance_percent: u32) -> bool {
    if expected <= 0.0 {
        return true;
    }
    let tol = tolerance_percent as f64 / 100.0;
    (measured - expected).abs() / expected <= tol
}

/// Returns true if loss_percent is within max_percent (i.e. loss_percent <= max_percent).
pub fn packet_loss_within_tolerance(loss_percent: f64, max_percent: f64) -> bool {
    loss_percent <= max_percent
}

/// Build SDP/stream structural test results from stream info (unit-testable without a live RTSP server).
pub fn build_sdp_test_results(stream_infos: &[StreamInfo]) -> Vec<TestResult> {
    let mut tests = Vec::new();
    tests.push(TestResult::metric(
        "stream_count",
        serde_json::json!(stream_infos.len()),
        !stream_infos.is_empty(),
    ));
    tests.push(TestResult::metric(
        "sdp_streams",
        serde_json::json!(stream_infos),
        true,
    ));

    let has_video = stream_infos.iter().any(|s| s.media == "video");
    tests.push(if has_video {
        TestResult::pass("sdp_has_video")
    } else {
        TestResult::fail("sdp_has_video", "no SDP stream with media=video")
    });

    let video_is_h264 = stream_infos
        .iter()
        .any(|s| s.media == "video" && s.encoding_name == "h264");
    tests.push(if !has_video || video_is_h264 {
        TestResult::pass("video_encoding_h264")
    } else {
        TestResult::fail(
            "video_encoding_h264",
            "no video stream advertised encoding_name=h264",
        )
    });

    let has_audio = stream_infos.iter().any(|s| s.media == "audio");
    tests.push(TestResult::metric(
        "sdp_has_audio",
        serde_json::json!(has_audio),
        true,
    ));

    if stream_infos.len() > 1 {
        let all_have_control = stream_infos.iter().all(|s| s.control_present);
        tests.push(if all_have_control {
            TestResult::pass("multitrack_controls_present")
        } else {
            TestResult::fail(
                "multitrack_controls_present",
                "multiple streams advertised but at least one lacks a=control",
            )
        });
    } else {
        tests.push(TestResult::pass("multitrack_controls_present"));
    }

    tests
}

struct BoundedLogWriter {
    file: File,
    written: usize,
    truncated: bool,
}

impl BoundedLogWriter {
    pub(crate) fn create(path: &Path) -> Result<Self> {
        let file = File::create(path).with_context(|| format!("create {}", path.display()))?;
        Ok(Self {
            file,
            written: 0,
            truncated: false,
        })
    }

    pub(crate) fn write_line(&mut self, line: &str) -> Result<()> {
        if self.truncated {
            return Ok(());
        }
        let bytes = line.as_bytes();
        let remaining = MAX_TOOL_LOG_BYTES.saturating_sub(self.written);
        if remaining == 0 {
            self.truncated = true;
            return Ok(());
        }
        if bytes.len() < remaining {
            self.file.write_all(bytes)?;
            self.file.write_all(b"\n")?;
            self.written += bytes.len() + 1;
            return Ok(());
        }
        let take = remaining.saturating_sub(1);
        if take > 0 {
            self.file.write_all(&bytes[..take])?;
            self.file.write_all(b"\n")?;
            self.written += take + 1;
        }
        self.truncated = true;
        Ok(())
    }
}

pub fn validate_h264_length_prefixed_nals(data: &[u8]) -> Result<()> {
    let mut i: usize = 0;
    let mut nals_seen: u32 = 0;
    while i < data.len() {
        let remaining = data.len().saturating_sub(i);
        if remaining < 4 {
            bail!("trailing {} bytes after last NAL length", remaining);
        }
        let len = u32::from_be_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]) as usize;
        i += 4;
        if len == 0 {
            bail!("zero-length NAL unit");
        }
        let remaining_after_len = data.len().saturating_sub(i);
        if len > remaining_after_len {
            bail!(
                "NAL length {} exceeds remaining bytes {}",
                len,
                remaining_after_len
            );
        }
        let nal_header = data[i];
        let nal_type = nal_header & 0x1f;
        if nal_type == 0 {
            bail!("invalid NAL type 0");
        }
        if nal_type > 31 {
            bail!("invalid NAL type {}", nal_type);
        }
        i += len;
        nals_seen = nals_seen.saturating_add(1);
        if nals_seen > 1024 {
            bail!("too many NAL units in a single frame (>{})", nals_seen);
        }
    }
    Ok(())
}

pub async fn run_validation(args: &Args, effective: &EffectiveConfig) -> Result<ValidationReport> {
    let url_str = format!(
        "rtsp://{}:{}{}",
        effective.rtsp_host, effective.rtsp_port, effective.rtsp_stream
    );
    info!(
        url = %url_str,
        duration_sec = effective.short_duration_sec,
        "running RTSP validation"
    );

    let timestamp = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let test_run = TestRun {
        timestamp,
        rtsp_host: effective.rtsp_host.clone(),
        rtsp_port: effective.rtsp_port,
        rtsp_stream: effective.rtsp_stream.clone(),
        test_duration_seconds: effective.short_duration_sec,
        httpflv_port: Some(effective.httpflv_port),
        httpflv_path: Some(effective.httpflv_path.clone()),
    };

    let mut tests: Vec<TestResult> = Vec::new();

    let url = Url::parse(&url_str).with_context(|| format!("invalid RTSP URL: {}", url_str))?;

    let creds = match (&effective.stream_username, &effective.stream_password) {
        (Some(username), Some(password)) => Some(Credentials {
            username: username.clone(),
            password: password.clone(),
        }),
        _ => None,
    };
    let options = SessionOptions::default()
        .user_agent("anyka-rtsp-validation-tool".to_string())
        .creds(creds);

    debug!("DESCRIBE request");
    let describe_start = Instant::now();
    let mut session = match Session::describe(url, options).await {
        Ok(s) => {
            tests.push(TestResult::pass("describe_ok"));
            s
        }
        Err(e) => {
            warn!(error = %e, "DESCRIBE failed");
            tests.push(TestResult::fail("describe_ok", e.to_string()));
            return Ok(empty_report(test_run, tests));
        }
    };
    let describe_ms = describe_start.elapsed().as_millis() as u64;
    debug!(describe_ms, "DESCRIBE ok");
    tests.push(TestResult::metric(
        "describe_latency_ms",
        serde_json::json!(describe_ms),
        true,
    ));

    let stream_infos: Vec<StreamInfo> = session
        .streams()
        .iter()
        .map(stream_info_from_retina)
        .collect();
    tests.extend(build_sdp_test_results(&stream_infos));

    let has_video = stream_infos.iter().any(|s| s.media == "video");
    let has_audio = stream_infos.iter().any(|s| s.media == "audio");
    let setup_transport = to_retina_transport(args.transport);
    debug!(stream_count = stream_infos.len(), "SETUP streams");
    let mut setup_ok = true;
    for (i, s) in stream_infos.iter().enumerate() {
        let setup_start = Instant::now();
        match session
            .setup(
                i,
                SetupOptions::default().transport(setup_transport.clone()),
            )
            .await
        {
            Ok(()) => {
                let elapsed_ms = setup_start.elapsed().as_millis() as u64;
                tests.push(TestResult::metric(
                    format!("setup_stream_{}_latency_ms", i),
                    serde_json::json!(elapsed_ms),
                    true,
                ));
            }
            Err(e) => {
                setup_ok = false;
                tests.push(TestResult::fail(
                    format!("setup_stream_{}", i),
                    format!(
                        "SETUP failed for stream {} (media={}, encoding={}): {}",
                        i, s.media, s.encoding_name, e
                    ),
                ));
            }
        }
    }
    tests.push(TestResult::metric(
        "setup_all_streams_ok",
        serde_json::json!(setup_ok),
        setup_ok,
    ));
    if !setup_ok {
        tokio::spawn(async move {
            drop(session);
        });
        return Ok(empty_report(test_run, tests));
    }
    debug!("SETUP ok");

    let play_opts = PlayOptions::default().initial_timestamp(to_retina_initial_timestamp_policy(
        effective.initial_timestamp_policy,
    ));
    debug!(
        initial_timestamp_policy = ?effective.initial_timestamp_policy,
        "PLAY request"
    );
    let play_start = Instant::now();
    let playing = match session.play(play_opts).await {
        Ok(s) => {
            tests.push(TestResult::pass("play_ok"));
            s
        }
        Err(e) => {
            warn!(error = %e, "PLAY failed");
            tests.push(TestResult::fail("play_ok", e.to_string()));
            return Ok(empty_report(test_run, tests));
        }
    };
    let play_rtt_ms = play_start.elapsed().as_millis() as u64;
    debug!(play_rtt_ms, "PLAY ok");
    tests.push(TestResult::metric(
        "play_rtt_ms",
        serde_json::json!(play_rtt_ms),
        true,
    ));

    let mut demuxed = playing.demuxed().context("failed to demux/depacketize")?;

    let mut first_video_latency_ms: Option<u64> = None;
    let mut first_audio_latency_ms: Option<u64> = None;
    let mut video_frames: u64 = 0;
    let mut audio_frames: u64 = 0;
    let mut total_loss_packets: u64 = 0;
    let mut saw_rap: bool = false;
    let mut h264_length_prefix_ok: bool = true;
    let mut h264_length_prefix_error: Option<String> = None;
    let mut demux_error_count: u32 = 0;
    let mut last_demux_error: Option<String> = None;

    let probe_duration = Duration::from_secs(effective.short_duration_sec);
    let probe_res: Result<()> = timeout(probe_duration, async {
        while let Some(item) = demuxed.next().await {
            let item = match item {
                Ok(item) => item,
                Err(err) => {
                    demux_error_count = demux_error_count.saturating_add(1);
                    last_demux_error = Some(err.to_string());
                    if demux_error_count <= PROBE_DEMUX_ERROR_TOLERANCE {
                        warn!(
                            demux_error_count,
                            tolerance = PROBE_DEMUX_ERROR_TOLERANCE,
                            error = %err,
                            "probe loop demux error"
                        );
                    } else {
                        debug!(
                            demux_error_count,
                            error = %err,
                            "probe loop demux error (continuing)"
                        );
                    }
                    continue;
                }
            };
            match item {
                CodecItem::VideoFrame(frame) => {
                    video_frames = video_frames.saturating_add(1);
                    total_loss_packets = total_loss_packets.saturating_add(frame.loss() as u64);
                    if first_video_latency_ms.is_none() {
                        let latency_ms = play_start.elapsed().as_millis() as u64;
                        first_video_latency_ms = Some(latency_ms);
                        trace!(first_video_latency_ms = latency_ms, "first video frame");
                        if let Err(e) = validate_h264_length_prefixed_nals(frame.data()) {
                            h264_length_prefix_ok = false;
                            h264_length_prefix_error = Some(e.to_string());
                        }
                    }
                    if frame.is_random_access_point() {
                        saw_rap = true;
                    }
                    if args.require_audio && has_audio && first_audio_latency_ms.is_some() {
                        break;
                    }
                }
                CodecItem::AudioFrame(frame) => {
                    audio_frames = audio_frames.saturating_add(1);
                    total_loss_packets = total_loss_packets.saturating_add(frame.loss() as u64);
                    first_audio_latency_ms
                        .get_or_insert_with(|| play_start.elapsed().as_millis() as u64);
                    if args.require_audio && has_video && first_video_latency_ms.is_some() {
                        break;
                    }
                }
                CodecItem::MessageFrame(_) => {}
                CodecItem::Rtcp(_) => {}
                _ => {}
            }
        }
        Ok(())
    })
    .await
    .unwrap_or_else(|_| Ok(()));

    if let Err(e) = probe_res {
        warn!(error = %e, "probe loop ended with error");
        tests.push(TestResult::fail("probe_loop", e.to_string()));
    } else if demux_error_count > PROBE_DEMUX_ERROR_TOLERANCE
        && video_frames == 0
        && (!args.require_audio || !has_audio || audio_frames == 0)
    {
        let reason = match last_demux_error {
            Some(err) => format!(
                "demuxed stream error ({} errors, tolerance {}): {}",
                demux_error_count, PROBE_DEMUX_ERROR_TOLERANCE, err
            ),
            None => format!(
                "demuxed stream error ({} errors, tolerance {})",
                demux_error_count, PROBE_DEMUX_ERROR_TOLERANCE
            ),
        };
        warn!(reason = %reason, "probe loop ended without decodable frames");
        tests.push(TestResult::fail("probe_loop", reason));
    } else {
        tests.push(TestResult::pass("probe_loop"));
    }

    tests.push(TestResult::metric(
        "video_frames_observed",
        serde_json::json!(video_frames),
        video_frames > 0,
    ));
    tests.push(TestResult::metric(
        "audio_frames_observed",
        serde_json::json!(audio_frames),
        !args.require_audio || !has_audio || audio_frames > 0,
    ));

    if let Some(latency_ms) = first_video_latency_ms {
        tests.push(TestResult::metric(
            "first_video_frame_latency_ms",
            serde_json::json!(latency_ms),
            latency_ms <= args.max_video_startup_latency_ms,
        ));
    } else {
        tests.push(TestResult::fail(
            "first_video_frame_latency_ms",
            "no video frames observed during probe window",
        ));
    }

    if let Some(latency_ms) = first_audio_latency_ms {
        tests.push(TestResult::metric(
            "first_audio_frame_latency_ms",
            serde_json::json!(latency_ms),
            true,
        ));
    }

    tests.push(TestResult::metric(
        "rtp_loss_packets_total",
        serde_json::json!(total_loss_packets),
        total_loss_packets == 0,
    ));

    tests.push(TestResult::metric(
        "random_access_point_seen",
        serde_json::json!(saw_rap),
        saw_rap,
    ));

    tests.push(if h264_length_prefix_ok {
        TestResult::pass("h264_length_prefix_ok")
    } else {
        TestResult::fail(
            "h264_length_prefix_ok",
            h264_length_prefix_error
                .unwrap_or_else(|| "invalid H.264 length-prefixed framing".to_string()),
        )
    });

    let mut video_params_ok = true;
    let mut audio_params_ok = true;
    for s in demuxed.streams() {
        match s.media() {
            "video" => match s.parameters() {
                Some(ParametersRef::Video(p)) => {
                    if p.extra_data().is_empty() {
                        video_params_ok = false;
                    }
                }
                _ => video_params_ok = false,
            },
            "audio" => match s.parameters() {
                Some(ParametersRef::Audio(_)) => {}
                _ => audio_params_ok = false,
            },
            _ => {}
        }
    }
    tests.push(TestResult::metric(
        "video_parameters_available",
        serde_json::json!(video_params_ok),
        !has_video || video_params_ok,
    ));
    tests.push(TestResult::metric(
        "audio_parameters_available",
        serde_json::json!(audio_params_ok),
        !has_audio || audio_params_ok,
    ));

    let passed = tests.iter().filter(|t| result_ok(t)).count();
    info!(
        total_tests = tests.len(),
        passed, video_frames, audio_frames, "RTSP validation complete"
    );

    Ok(ValidationReport {
        test_run,
        tests,
        summary: crate::report::Summary {
            total_tests: 0,
            passed: 0,
            failed: 0,
            overall_pass: false,
        },
        artifacts_dir: None,
        telemetry: None,
        telemetry_before_shutdown: None,
    })
}

#[derive(Debug, Deserialize)]
struct FfprobeStreams {
    streams: Option<Vec<FfprobeStream>>,
}

#[derive(Debug, Deserialize)]
struct FfprobeStream {
    codec_type: Option<String>,
    codec_name: Option<String>,
}

pub async fn run_harness(
    args: &Args,
    effective: &EffectiveConfig,
    tests: &mut Vec<TestResult>,
) -> Result<()> {
    let url = rtsp_url(
        &effective.rtsp_host,
        effective.rtsp_port,
        &effective.rtsp_stream,
    );
    let auth_url = rtsp_url_with_credentials(
        &effective.rtsp_host,
        effective.rtsp_port,
        &effective.rtsp_stream,
        effective.stream_username.as_deref(),
        effective.stream_password.as_deref(),
    )?;
    let timeout_sec = effective.rtsp_timeout_sec;
    let step_cap_short = Duration::from_secs(timeout_sec.saturating_add(15));
    let step_cap_long = Duration::from_secs(effective.short_duration_sec.saturating_add(30));
    let step_cap_protocol_sequence = Duration::from_secs(
        effective
            .short_duration_sec
            .saturating_mul(2)
            .saturating_add(30),
    );
    let artifacts_dir = effective.artifacts_dir.clone();
    let capture_tool_output = effective.capture_tool_output;
    info!(url = %url, uses_credentials = effective.stream_username.is_some(), "running harness scenarios");
    let mut expect_h264 = false;
    let mut expect_aac = false;

    debug!("harness: basic connectivity");
    match timeout(
        step_cap_short,
        harness_basic_connectivity(
            &auth_url,
            timeout_sec,
            &artifacts_dir,
            capture_tool_output,
            &effective.ffmpeg_log_level,
        ),
    )
    .await
    {
        Ok(Ok(ok)) => {
            tests.push(if ok {
                TestResult::pass("harness_basic_connectivity")
            } else {
                TestResult::fail("harness_basic_connectivity", "no stream in output")
            });
        }
        Ok(Err(e)) => tests.push(TestResult::fail(
            "harness_basic_connectivity",
            e.to_string(),
        )),
        Err(_) => {
            cleanup_timed_out_media_processes(&auth_url, "harness_basic_connectivity");
            tests.push(TestResult::fail(
                "harness_basic_connectivity",
                format!("harness step timed out after {}s", step_cap_short.as_secs()),
            ));
        }
    }

    debug!(
        url = %url,
        target_ms = effective.harness_startup_latency_ms,
        "harness: startup latency"
    );
    match timeout(
        step_cap_short,
        harness_startup_latency(
            &auth_url,
            timeout_sec,
            &artifacts_dir,
            capture_tool_output,
            &effective.ffmpeg_log_level,
        ),
    )
    .await
    {
        Ok(Ok(Some(ms))) => {
            let pass = ms <= effective.harness_startup_latency_ms;
            tests.push(TestResult::metric(
                "harness_startup_latency_ms",
                serde_json::json!(ms),
                pass,
            ));
        }
        Ok(Ok(None)) => tests.push(TestResult::fail(
            "harness_startup_latency_ms",
            "no frame decoded",
        )),
        Ok(Err(e)) => tests.push(TestResult::fail(
            "harness_startup_latency_ms",
            e.to_string(),
        )),
        Err(_) => {
            cleanup_timed_out_media_processes(&auth_url, "harness_startup_latency_ms");
            tests.push(TestResult::fail(
                "harness_startup_latency_ms",
                format!("harness step timed out after {}s", step_cap_short.as_secs()),
            ));
        }
    }

    debug!(url = %url, duration_sec = effective.short_duration_sec, "harness: bitrate/fps");
    match timeout(
        step_cap_long,
        harness_bitrate_fps(
            &auth_url,
            effective.short_duration_sec,
            &artifacts_dir,
            capture_tool_output,
            effective,
            &effective.ffmpeg_log_level,
        ),
    )
    .await
    {
        Ok(Ok((bitrate, fps))) => {
            let bitrate_pass = effective
                .expected_bitrate_kbps
                .map(|e| bitrate_within_tolerance(bitrate, e, effective.bitrate_tolerance_percent))
                .unwrap_or(true);
            let fps_pass = effective
                .expected_fps
                .map(|e| fps_within_tolerance(fps, e, effective.fps_tolerance_percent))
                .unwrap_or(true);
            tests.push(TestResult::metric(
                "harness_bitrate_kbps",
                serde_json::json!(bitrate),
                bitrate_pass,
            ));
            tests.push(TestResult::metric(
                "harness_fps",
                serde_json::json!(fps),
                fps_pass,
            ));
        }
        Ok(Err(e)) => tests.push(TestResult::fail("harness_bitrate_fps", e.to_string())),
        Err(_) => {
            cleanup_timed_out_media_processes(&auth_url, "harness_bitrate_fps");
            tests.push(TestResult::fail(
                "harness_bitrate_fps",
                format!("harness step timed out after {}s", step_cap_long.as_secs()),
            ));
        }
    }

    debug!(url = %url, "harness: SDP validation");
    match timeout(
        step_cap_short,
        harness_sdp_validation(&auth_url, timeout_sec, &artifacts_dir, capture_tool_output),
    )
    .await
    {
        Ok(Ok((video_count, audio_count, has_h264, has_aac))) => {
            expect_h264 = has_h264;
            expect_aac = has_aac;
            tests.push(TestResult::metric(
                "harness_sdp_video_streams",
                serde_json::json!(video_count),
                video_count > 0,
            ));
            tests.push(TestResult::metric(
                "harness_sdp_audio_streams",
                serde_json::json!(audio_count),
                true,
            ));
            tests.push(if has_h264 {
                TestResult::pass("harness_sdp_video_h264")
            } else {
                TestResult::fail("harness_sdp_video_h264", "no H.264 video stream")
            });
            tests.push(if !has_aac && audio_count > 0 {
                TestResult::fail("harness_sdp_audio_aac", "no AAC audio stream")
            } else {
                TestResult::pass("harness_sdp_audio_aac")
            });
        }
        Ok(Err(e)) => tests.push(TestResult::fail("harness_sdp_validation", e.to_string())),
        Err(_) => {
            cleanup_timed_out_media_processes(&auth_url, "harness_sdp_validation");
            tests.push(TestResult::fail(
                "harness_sdp_validation",
                format!("harness step timed out after {}s", step_cap_short.as_secs()),
            ));
        }
    }

    debug!(url = %url, "harness: RTSP protocol sequence");
    match timeout(
        step_cap_protocol_sequence,
        harness_rtsp_protocol_sequence(&auth_url, effective, args),
    )
    .await
    {
        Ok(Ok((describe, setup, play, teardown, status_200, status_err))) => {
            let pass = describe > 0 && setup > 0 && play > 0 && status_err == 0 && status_200 > 0;
            tests.push(TestResult::metric(
                "harness_protocol_sequence",
                serde_json::json!({
                    "describe": describe,
                    "setup": setup,
                    "play": play,
                    "teardown": teardown,
                    "status_200": status_200,
                    "status_4xx": status_err,
                }),
                pass,
            ));
        }
        Ok(Err(e)) => tests.push(TestResult::fail("harness_protocol_sequence", e.to_string())),
        Err(_) => {
            cleanup_timed_out_media_processes(&auth_url, "harness_protocol_sequence");
            tests.push(TestResult::fail(
                "harness_protocol_sequence",
                format!(
                    "harness step timed out after {}s",
                    step_cap_protocol_sequence.as_secs()
                ),
            ));
        }
    }

    debug!(url = %url, "harness: packet loss + pcap RFC checks");
    match timeout(
        step_cap_long,
        harness_packet_loss(&auth_url, effective, args, expect_h264, expect_aac),
    )
    .await
    {
        Ok(Ok(res)) => {
            let (video_metric, pass) = match res.video.clone() {
                Some(metric) => {
                    let pass = packet_loss_within_tolerance(
                        metric.loss_percent,
                        effective.packet_loss_tolerance_percent,
                    );
                    (metric, pass)
                }
                None => (HarnessRtpLossMetric::default(), false),
            };
            tests.push(TestResult::metric(
                "harness_packet_loss_percent",
                serde_json::json!({
                    "rtp_packets": video_metric.rtp_packets,
                    "packet_loss": video_metric.packet_loss,
                    "loss_percent": video_metric.loss_percent,
                    "payload_type": video_metric.payload_type,
                    "ssrc": video_metric.ssrc,
                }),
                pass,
            ));

            if let Some(video) = res.video.clone() {
                let pass = packet_loss_within_tolerance(
                    video.loss_percent,
                    effective.packet_loss_tolerance_percent,
                );
                tests.push(TestResult::metric(
                    "harness_packet_loss_percent_video",
                    serde_json::json!(video),
                    pass,
                ));
            }

            if let Some(audio) = res.audio.clone() {
                let pass = packet_loss_within_tolerance(
                    audio.loss_percent,
                    effective.packet_loss_tolerance_percent,
                );
                tests.push(TestResult::metric(
                    "harness_packet_loss_percent_audio",
                    serde_json::json!(audio),
                    pass,
                ));
            }

            if expect_h264 {
                match res.h264_rfc6184 {
                    Some(Ok(stats)) => {
                        let pass = stats.packets_analyzed > 0 && stats.invalid_packets == 0;
                        tests.push(TestResult::metric(
                            "harness_pcap_rfc6184_h264",
                            serde_json::json!(stats),
                            pass,
                        ));
                    }
                    Some(Err(e)) => {
                        tests.push(TestResult::fail("harness_pcap_rfc6184_h264", e));
                    }
                    None => tests.push(TestResult::fail(
                        "harness_pcap_rfc6184_h264",
                        "pcap validation skipped unexpectedly",
                    )),
                }
            } else {
                tests.push(TestResult::pass("harness_pcap_rfc6184_h264"));
            }

            if expect_aac {
                match res.aac_rfc3640 {
                    Some(Ok(stats)) => {
                        let pass = stats.packets_analyzed > 0 && stats.invalid_packets == 0;
                        tests.push(TestResult::metric(
                            "harness_pcap_rfc3640_aac",
                            serde_json::json!(stats),
                            pass,
                        ));
                    }
                    Some(Err(e)) => {
                        tests.push(TestResult::fail("harness_pcap_rfc3640_aac", e));
                    }
                    None => tests.push(TestResult::fail(
                        "harness_pcap_rfc3640_aac",
                        "pcap validation skipped unexpectedly",
                    )),
                }
            } else {
                tests.push(TestResult::pass("harness_pcap_rfc3640_aac"));
            }
        }
        Ok(Err(e)) => tests.push(TestResult::fail("harness_packet_loss", e.to_string())),
        Err(_) => {
            cleanup_timed_out_media_processes(&auth_url, "harness_packet_loss");
            tests.push(TestResult::fail(
                "harness_packet_loss",
                format!("harness step timed out after {}s", step_cap_long.as_secs()),
            ));
        }
    }

    if effective.concurrent_clients > 0 {
        debug!(url = %url, concurrent = effective.concurrent_clients, "harness: concurrent clients");
        match timeout(
            step_cap_long,
            harness_concurrent_clients(
                &auth_url,
                effective.short_duration_sec,
                effective.concurrent_clients,
                timeout_sec,
                &artifacts_dir,
                capture_tool_output,
            ),
        )
        .await
        {
            Ok(Ok(failed)) => {
                tests.push(TestResult::metric(
                    "harness_concurrent_clients",
                    serde_json::json!({ "requested": effective.concurrent_clients, "failed": failed }),
                    failed == 0,
                ));
            }
            Ok(Err(e)) => tests.push(TestResult::fail(
                "harness_concurrent_clients",
                e.to_string(),
            )),
            Err(_) => {
                cleanup_timed_out_media_processes(&auth_url, "harness_concurrent_clients");
                tests.push(TestResult::fail(
                    "harness_concurrent_clients",
                    format!("harness step timed out after {}s", step_cap_long.as_secs()),
                ));
            }
        }
    }

    if args.long_duration {
        let step_cap_long_duration =
            Duration::from_secs(effective.long_duration_sec.saturating_add(30));
        debug!(url = %url, duration_sec = effective.long_duration_sec, "harness: long duration");
        match timeout(
            step_cap_long_duration,
            harness_long_duration(
                &auth_url,
                effective.long_duration_sec,
                &artifacts_dir,
                capture_tool_output,
            ),
        )
        .await
        {
            Ok(Ok(degradation_pct)) => {
                tests.push(TestResult::metric(
                    "harness_long_duration_degradation_pct",
                    serde_json::json!(degradation_pct),
                    degradation_pct < 20,
                ));
            }
            Ok(Err(e)) => tests.push(TestResult::fail("harness_long_duration", e.to_string())),
            Err(_) => {
                cleanup_timed_out_media_processes(&auth_url, "harness_long_duration");
                tests.push(TestResult::fail(
                    "harness_long_duration",
                    format!(
                        "harness step timed out after {}s",
                        step_cap_long_duration.as_secs()
                    ),
                ));
            }
        }
    }

    if !args.skip_error_handling {
        debug!(host = %effective.rtsp_host, port = effective.rtsp_port, "harness: error handling");
        match timeout(
            step_cap_short,
            harness_error_handling(
                &effective.rtsp_host,
                effective.rtsp_port,
                &effective.rtsp_stream,
                effective
                    .stream_username
                    .as_deref()
                    .zip(effective.stream_password.as_deref()),
                timeout_sec,
                &artifacts_dir,
                capture_tool_output,
            ),
        )
        .await
        {
            Ok(Ok((invalid_creds_ok, bogus_url_ok))) => {
                tests.push(TestResult::metric(
                    "harness_error_invalid_creds",
                    serde_json::json!(invalid_creds_ok),
                    invalid_creds_ok,
                ));
                tests.push(TestResult::metric(
                    "harness_error_bogus_url",
                    serde_json::json!(bogus_url_ok),
                    bogus_url_ok,
                ));
            }
            Ok(Err(e)) => tests.push(TestResult::fail("harness_error_handling", e.to_string())),
            Err(_) => {
                cleanup_timed_out_media_processes(&auth_url, "harness_error_handling");
                tests.push(TestResult::fail(
                    "harness_error_handling",
                    format!("harness step timed out after {}s", step_cap_short.as_secs()),
                ));
            }
        }
    }

    Ok(())
}

async fn harness_basic_connectivity(
    url: &str,
    _timeout_sec: u64,
    artifacts_dir: &Path,
    capture_tool_output: bool,
    ffmpeg_log_level: &str,
) -> Result<bool> {
    let url = url.to_string();
    let ffmpeg_level = ffmpeg_log_level.to_string();
    let log_path = artifacts_dir.join("ffmpeg_basic_connectivity.log");
    let ok = tokio::task::spawn_blocking(move || {
        let mut log = if capture_tool_output {
            Some(BoundedLogWriter::create(&log_path)?)
        } else {
            None
        };
        if let Some(l) = log.as_mut() {
            l.write_line("=== ffmpeg basic connectivity ===")?;
            l.write_line(&format!("url={}", redact_url_credentials(&url)))?;
        }
        let mut cmd = FfmpegCommand::new();
        cmd.hide_banner()
            .arg("-loglevel")
            .arg(&ffmpeg_level)
            .arg("-rtsp_transport")
            .arg("tcp")
            .input(&url)
            .duration("0.1")
            .format("null")
            .output("-");
        let mut child = cmd.spawn().context("spawn ffmpeg")?;
        let iter = child.iter().context("ffmpeg iter")?;
        let mut saw_stream = false;
        for event in iter {
            if let Some(l) = log.as_mut() {
                match &event {
                    FfmpegEvent::Log(_, msg) => l.write_line(msg)?,
                    FfmpegEvent::Progress(p) => l.write_line(&format!(
                        "progress frame={} fps={} bitrate_kbps={} time={} speed={}",
                        p.frame, p.fps, p.bitrate_kbps, p.time, p.speed
                    ))?,
                    FfmpegEvent::Done => l.write_line("done")?,
                    other => l.write_line(&format!("event={:?}", other))?,
                }
            }
            if let FfmpegEvent::Log(_, msg) = &event
                && msg.contains("Stream #")
            {
                saw_stream = true;
                break;
            }
            if let FfmpegEvent::Progress(_) = &event {
                saw_stream = true;
                break;
            }
            if matches!(event, FfmpegEvent::Done) {
                saw_stream = true;
                break;
            }
        }
        Ok::<_, anyhow::Error>(saw_stream)
    })
    .await
    .context("spawn_blocking")??;
    Ok(ok)
}

async fn harness_startup_latency(
    url: &str,
    _timeout_sec: u64,
    artifacts_dir: &Path,
    capture_tool_output: bool,
    ffmpeg_log_level: &str,
) -> Result<Option<u64>> {
    let url = url.to_string();
    let ffmpeg_level = ffmpeg_log_level.to_string();
    let log_path = artifacts_dir.join("ffmpeg_startup_latency.log");
    let ms = tokio::task::spawn_blocking(move || {
        let mut log = if capture_tool_output {
            Some(BoundedLogWriter::create(&log_path)?)
        } else {
            None
        };
        if let Some(l) = log.as_mut() {
            l.write_line("=== ffmpeg startup latency ===")?;
            l.write_line(&format!("url={}", redact_url_credentials(&url)))?;
        }
        let start = std::time::Instant::now();
        let mut cmd = FfmpegCommand::new();
        cmd.hide_banner()
            .arg("-loglevel")
            .arg(&ffmpeg_level)
            .arg("-rtsp_transport")
            .arg("tcp")
            .input(&url)
            .frames(1)
            .format("null")
            .output("-");
        let mut child = cmd.spawn().context("spawn ffmpeg")?;
        let iter = child.iter().context("ffmpeg iter")?;
        let mut first_frame_ms = None;
        for event in iter {
            if let Some(l) = log.as_mut() {
                match &event {
                    FfmpegEvent::Log(_, msg) => l.write_line(msg)?,
                    FfmpegEvent::Progress(p) => l.write_line(&format!(
                        "progress frame={} fps={} bitrate_kbps={} time={} speed={}",
                        p.frame, p.fps, p.bitrate_kbps, p.time, p.speed
                    ))?,
                    FfmpegEvent::Done => l.write_line("done")?,
                    other => l.write_line(&format!("event={:?}", other))?,
                }
            }
            if let FfmpegEvent::Progress(FfmpegProgress { frame: f, .. }) = &event
                && *f > 0
            {
                first_frame_ms = Some(start.elapsed().as_millis() as u64);
                break;
            }
            if matches!(event, FfmpegEvent::Done) {
                break;
            }
        }
        Ok::<_, anyhow::Error>(first_frame_ms)
    })
    .await
    .context("spawn_blocking")??;
    Ok(ms)
}

async fn harness_bitrate_fps(
    url: &str,
    duration_sec: u64,
    artifacts_dir: &Path,
    capture_tool_output: bool,
    _effective: &EffectiveConfig,
    ffmpeg_log_level: &str,
) -> Result<(f64, f64)> {
    let url = url.to_string();
    let dur = duration_sec;
    let ffmpeg_level = ffmpeg_log_level.to_string();
    let log_path = artifacts_dir.join("ffmpeg_bitrate_fps.log");
    let (bitrate, fps) = tokio::task::spawn_blocking(move || {
        let mut log = if capture_tool_output {
            Some(BoundedLogWriter::create(&log_path)?)
        } else {
            None
        };
        if let Some(l) = log.as_mut() {
            l.write_line("=== ffmpeg bitrate/fps ===")?;
            l.write_line(&format!("url={}", redact_url_credentials(&url)))?;
            l.write_line(&format!("duration_sec={}", dur))?;
        }
        let mut cmd = FfmpegCommand::new();
        cmd.hide_banner()
            .arg("-loglevel")
            .arg(&ffmpeg_level)
            .arg("-rtsp_transport")
            .arg("tcp")
            .input(&url)
            .duration(dur.to_string())
            .format("null")
            .output("-");
        let mut child = cmd.spawn().context("spawn ffmpeg")?;
        let iter = child.iter().context("ffmpeg iter")?;
        let mut last_bitrate = 0.0_f64;
        let mut last_fps = 0.0_f64;
        let mut progress_count: u32 = 0;
        for event in iter {
            if let Some(l) = log.as_mut() {
                match &event {
                    FfmpegEvent::Log(_, msg) => l.write_line(msg)?,
                    FfmpegEvent::Progress(p) => l.write_line(&format!(
                        "progress frame={} fps={} bitrate_kbps={} time={} speed={}",
                        p.frame, p.fps, p.bitrate_kbps, p.time, p.speed
                    ))?,
                    FfmpegEvent::Done => l.write_line("done")?,
                    other => l.write_line(&format!("event={:?}", other))?,
                }
            }
            if let FfmpegEvent::Progress(FfmpegProgress {
                frame,
                fps: f,
                bitrate_kbps: b,
                time,
                speed,
                ..
            }) = &event
            {
                last_bitrate = *b as f64;
                last_fps = *f as f64;
                progress_count += 1;
                if progress_count == 1 || progress_count.is_multiple_of(100) {
                    debug!(frame = *frame, fps = *f, bitrate_kbps = *b, time = %time, speed = *speed, "ffmpeg: bitrate/fps progress");
                }
            }
        }
        Ok::<_, anyhow::Error>((last_bitrate, last_fps))
    })
    .await
    .context("spawn_blocking")??;
    Ok((bitrate, fps))
}

async fn harness_sdp_validation(
    url: &str,
    _timeout_sec: u64,
    artifacts_dir: &Path,
    capture_tool_output: bool,
) -> Result<(usize, usize, bool, bool)> {
    let url = url.to_string();
    let stdout_path = artifacts_dir.join("ffprobe_sdp_validation.stdout.log");
    let stderr_path = artifacts_dir.join("ffprobe_sdp_validation.stderr.log");
    let result = tokio::task::spawn_blocking(move || {
        let out = Command::new("ffprobe")
            .args([
                "-v",
                "error",
                "-rtsp_transport",
                "tcp",
                "-show_streams",
                "-of",
                "json",
                &url,
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .context("ffprobe spawn")?;
        if capture_tool_output {
            write_bytes_tail(&stdout_path, &out.stdout).context("write ffprobe stdout")?;
            write_bytes_tail(&stderr_path, &out.stderr).context("write ffprobe stderr")?;
        }
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr);
            bail!(
                "ffprobe failed (code={:?}): {}",
                out.status.code(),
                tail_lossy(stderr.trim(), 1200)
            );
        }
        let v: FfprobeStreams =
            serde_json::from_slice(&out.stdout).context("parse ffprobe json")?;
        let streams = v.streams.unwrap_or_default();
        let video: Vec<_> = streams
            .iter()
            .filter(|s| s.codec_type.as_deref() == Some("video"))
            .collect();
        let audio: Vec<_> = streams
            .iter()
            .filter(|s| s.codec_type.as_deref() == Some("audio"))
            .collect();
        let has_h264 = video
            .iter()
            .any(|s| s.codec_name.as_deref() == Some("h264"));
        let has_aac = audio.iter().any(|s| s.codec_name.as_deref() == Some("aac"));
        Ok::<_, anyhow::Error>((video.len(), audio.len(), has_h264, has_aac))
    })
    .await
    .context("spawn_blocking")??;
    Ok(result)
}

async fn harness_rtsp_protocol_sequence(
    url: &str,
    effective: &EffectiveConfig,
    _args: &Args,
) -> Result<(u32, u32, u32, u32, u32, u32)> {
    let iface = effective.capture_interface.clone();
    let port = effective.rtsp_port;
    let url = url.to_string();
    let artifacts_dir = effective.artifacts_dir.clone();
    let capture_tool_output = effective.capture_tool_output;
    let keep_pcaps = effective.keep_pcaps;
    let pcap_path = artifacts_dir.join(format!("rtsp_protocol_sequence_tcp_port{}.pcap", port));

    let pcap_str = pcap_path.to_string_lossy().to_string();
    let tshark_stdout_path = artifacts_dir.join("tshark_rtsp_protocol_sequence.stdout.log");
    let tshark_stderr_path = artifacts_dir.join("tshark_rtsp_protocol_sequence.stderr.log");
    let tshark_stdout = if capture_tool_output {
        Stdio::from(
            File::create(&tshark_stdout_path)
                .with_context(|| format!("create {}", tshark_stdout_path.display()))?,
        )
    } else {
        Stdio::null()
    };
    let tshark_stderr = if capture_tool_output {
        Stdio::from(
            File::create(&tshark_stderr_path)
                .with_context(|| format!("create {}", tshark_stderr_path.display()))?,
        )
    } else {
        Stdio::null()
    };
    let mut tshark_handle = Command::new("tshark")
        .args([
            "-i",
            &iface,
            "-f",
            &format!("tcp port {}", port),
            "-w",
            &pcap_str,
        ])
        .stdout(tshark_stdout)
        .stderr(tshark_stderr)
        .spawn()
        .context("spawn tshark")?;
    info!(pcap = %pcap_path.display(), "tshark capture started (rtsp protocol sequence)");

    tokio::time::sleep(Duration::from_secs(1)).await;

    let url2 = url.clone();
    let short_dur = effective
        .short_duration_sec
        .clamp(1, PROTOCOL_SEQUENCE_CAPTURE_MAX_DURATION_SEC);
    let ffmpeg_log_path = artifacts_dir.join("ffmpeg_protocol_sequence_capture.log");
    let capture_tool = capture_tool_output;
    tokio::task::spawn_blocking(move || {
        let mut log = if capture_tool {
            Some(BoundedLogWriter::create(&ffmpeg_log_path)?)
        } else {
            None
        };
        if let Some(l) = log.as_mut() {
            l.write_line("=== ffmpeg protocol sequence capture ===")?;
            l.write_line(&format!("url={}", redact_url_credentials(&url2)))?;
            l.write_line(&format!("duration_sec={}", short_dur))?;
        }
        let mut cmd = FfmpegCommand::new();
        cmd.hide_banner()
            .arg("-rtsp_transport")
            .arg("tcp")
            .input(&url2)
            .duration(short_dur.to_string())
            .format("null")
            .output("-");
        let mut child = cmd.spawn().context("spawn ffmpeg")?;
        let iter = child.iter().context("ffmpeg iter")?;
        for event in iter {
            if let Some(l) = log.as_mut() {
                match &event {
                    FfmpegEvent::Log(_, msg) => l.write_line(msg)?,
                    FfmpegEvent::Progress(p) => l.write_line(&format!(
                        "progress frame={} fps={} bitrate_kbps={} time={} speed={}",
                        p.frame, p.fps, p.bitrate_kbps, p.time, p.speed
                    ))?,
                    FfmpegEvent::Done => l.write_line("done")?,
                    other => l.write_line(&format!("event={:?}", other))?,
                }
            }
        }
        Ok::<_, anyhow::Error>(())
    })
    .await
    .context("ffmpeg join")??;

    tokio::time::sleep(Duration::from_secs(2)).await;
    let _ = tshark_handle.kill();
    let _ = tshark_handle.wait();

    let pcap_path_str = pcap_path.to_string_lossy().to_string();
    let (describe, setup, play, teardown, status_200, status_err) =
        tokio::task::spawn_blocking(move || {
            let mut builder = RTSharkBuilder::builder();
            let mut rtshark = builder
                .input_path(&pcap_path_str)
                .spawn()
                .context("rtshark spawn")?;
            let mut describe = 0u32;
            let mut setup = 0u32;
            let mut play = 0u32;
            let mut teardown = 0u32;
            let mut status_200 = 0u32;
            let mut status_err = 0u32;
            while let Some(packet) = rtshark.read().context("rtshark read")? {
                for layer in packet {
                    let name = layer.name().to_string();
                    if name == "rtsp" {
                        let mut counted_method = false;
                        let mut counted_status = false;
                        for meta in layer {
                            if !counted_method
                                && let Some(method) =
                                    parse_rtsp_method_from_meta(meta.name(), meta.value())
                            {
                                match method {
                                    "DESCRIBE" => describe += 1,
                                    "SETUP" => setup += 1,
                                    "PLAY" => play += 1,
                                    "TEARDOWN" => teardown += 1,
                                    _ => {}
                                }
                                counted_method = true;
                            }
                            if !counted_status
                                && let Some(status) =
                                    parse_rtsp_status_code_from_meta(meta.name(), meta.value())
                            {
                                if status == 200 {
                                    status_200 += 1;
                                } else if status >= 400 {
                                    status_err += 1;
                                }
                                counted_status = true;
                            }
                            if counted_method && counted_status {
                                break;
                            }
                        }
                    }
                }
            }
            if !keep_pcaps {
                let _ = std::fs::remove_file(&pcap_path_str);
            }
            Ok::<_, anyhow::Error>((describe, setup, play, teardown, status_200, status_err))
        })
        .await
        .context("spawn_blocking")??;

    Ok((describe, setup, play, teardown, status_200, status_err))
}

async fn harness_packet_loss(
    url: &str,
    effective: &EffectiveConfig,
    _args: &Args,
    expect_h264: bool,
    expect_aac: bool,
) -> Result<HarnessPacketLossResult> {
    let iface = effective.capture_interface.clone();
    let url = url.to_string();
    let artifacts_dir = effective.artifacts_dir.clone();
    let capture_tool_output = effective.capture_tool_output;
    let keep_pcaps = effective.keep_pcaps;
    let pcap_path = artifacts_dir.join("rtp_packet_loss_capture.pcap");
    let server_ip = effective
        .rtsp_host
        .parse::<std::net::IpAddr>()
        .ok()
        .map(|ip| ip.to_string());

    let filter = if effective.rtsp_host.parse::<std::net::IpAddr>().is_ok() {
        format!("udp and host {}", effective.rtsp_host)
    } else {
        "udp".to_string()
    };

    let pcap_str_rtp = pcap_path.to_string_lossy().to_string();
    let tshark_stdout_path = artifacts_dir.join("tshark_packet_loss.stdout.log");
    let tshark_stderr_path = artifacts_dir.join("tshark_packet_loss.stderr.log");
    let tshark_stdout = if capture_tool_output {
        Stdio::from(
            File::create(&tshark_stdout_path)
                .with_context(|| format!("create {}", tshark_stdout_path.display()))?,
        )
    } else {
        Stdio::null()
    };
    let tshark_stderr = if capture_tool_output {
        Stdio::from(
            File::create(&tshark_stderr_path)
                .with_context(|| format!("create {}", tshark_stderr_path.display()))?,
        )
    } else {
        Stdio::null()
    };
    let mut tshark_handle_rtp = Command::new("tshark")
        .args(["-i", &iface, "-f", &filter, "-w", &pcap_str_rtp])
        .stdout(tshark_stdout)
        .stderr(tshark_stderr)
        .spawn()
        .context("spawn tshark")?;

    tokio::time::sleep(Duration::from_secs(1)).await;

    let url2 = url.clone();
    let short_dur_rtp = effective.short_duration_sec;
    let ffmpeg_log_path = artifacts_dir.join("ffmpeg_packet_loss_capture.log");
    tokio::task::spawn_blocking(move || {
        let mut log = if capture_tool_output {
            Some(BoundedLogWriter::create(&ffmpeg_log_path)?)
        } else {
            None
        };
        if let Some(l) = log.as_mut() {
            l.write_line("=== ffmpeg packet loss capture ===")?;
            l.write_line(&format!("url={}", redact_url_credentials(&url2)))?;
            l.write_line(&format!("duration_sec={}", short_dur_rtp))?;
        }
        let mut cmd = FfmpegCommand::new();
        cmd.hide_banner()
            .arg("-rtsp_transport")
            .arg("udp")
            .input(&url2)
            .duration(short_dur_rtp.to_string())
            .format("null")
            .output("-");
        let mut child = cmd.spawn().context("spawn ffmpeg")?;
        let iter = child.iter().context("ffmpeg iter")?;
        for event in iter {
            if let Some(l) = log.as_mut() {
                match &event {
                    FfmpegEvent::Log(_, msg) => l.write_line(msg)?,
                    FfmpegEvent::Progress(p) => l.write_line(&format!(
                        "progress frame={} fps={} bitrate_kbps={} time={} speed={}",
                        p.frame, p.fps, p.bitrate_kbps, p.time, p.speed
                    ))?,
                    FfmpegEvent::Done => l.write_line("done")?,
                    other => l.write_line(&format!("event={:?}", other))?,
                }
            }
        }
        Ok::<_, anyhow::Error>(())
    })
    .await
    .context("ffmpeg join")??;

    tokio::time::sleep(Duration::from_secs(2)).await;
    let _ = tshark_handle_rtp.kill();
    let _ = tshark_handle_rtp.wait();

    let pcap_path_for_parse = pcap_path.clone();
    let (video, audio, h264_rfc6184, aac_rfc3640) =
        tokio::task::spawn_blocking(move || {
            let pcap_path_str = pcap_path_for_parse.to_string_lossy().to_string();
            let mut rows = tshark_extract_rtp_rows(&pcap_path_for_parse)
                .context("tshark extract rtp rows")?;

            if let Some(server) = server_ip.as_ref() {
                rows.retain(|r| r.ip_src == *server);
            }

            if rows.is_empty() {
                bail!(
                    "No decodable RTP payload packets in UDP pcap; tshark RTP heuristic may be disabled or capture filter/interface is wrong."
                );
            }

            let stream_list = group_rtp_rows_by_stream(rows);

            let video_stream = pick_primary_video_stream(&stream_list);
            let video_key = video_stream.map(|s| s.key);
            let audio_stream = pick_primary_audio_stream(&stream_list, video_key);

            let video_metric = video_stream.map(compute_stream_loss_metric);
            let audio_metric = audio_stream.map(compute_stream_loss_metric);

            let h264_rfc6184 = if expect_h264 {
                match video_stream {
                    Some(s) => Some(analyze_h264_rfc6184_from_rows(&s.rows).map_err(|e| e.to_string())),
                    None => Some(Err("no RTP stream available for H.264 pcap validation".to_string())),
                }
            } else {
                None
            };

            let aac_rfc3640 = if expect_aac {
                match audio_stream {
                    Some(s) => Some(analyze_aac_rfc3640_from_rows(&s.rows).map_err(|e| e.to_string())),
                    None => Some(Err("no RTP stream available for AAC pcap validation".to_string())),
                }
            } else {
                None
            };

            if !keep_pcaps {
                let _ = std::fs::remove_file(&pcap_path_str);
            }

            Ok::<_, anyhow::Error>((video_metric, audio_metric, h264_rfc6184, aac_rfc3640))
    })
    .await
    .context("spawn_blocking")??;

    Ok(HarnessPacketLossResult {
        video,
        audio,
        h264_rfc6184,
        aac_rfc3640,
    })
}

async fn harness_concurrent_clients(
    url: &str,
    duration_sec: u64,
    count: u32,
    rtsp_timeout_sec: u64,
    artifacts_dir: &Path,
    capture_tool_output: bool,
) -> Result<u32> {
    let mut handles = Vec::new();
    let timeout_micros = rtsp_timeout_sec.saturating_mul(1_000_000).to_string();
    for i in 0..count {
        let url = url.to_string();
        let dur = duration_sec;
        let timeout_micros = timeout_micros.clone();
        let log_path = artifacts_dir.join(format!("ffmpeg_concurrent_client_{}.log", i));
        handles.push(tokio::task::spawn_blocking(move || {
            let mut log = if capture_tool_output {
                Some(BoundedLogWriter::create(&log_path)?)
            } else {
                None
            };
            if let Some(l) = log.as_mut() {
                l.write_line("=== ffmpeg concurrent client ===")?;
                l.write_line(&format!("client_index={}", i))?;
                l.write_line(&format!("url={}", redact_url_credentials(&url)))?;
                l.write_line(&format!("duration_sec={}", dur))?;
            }
            let mut cmd = FfmpegCommand::new();
            cmd.hide_banner()
                .arg("-nostdin")
                .arg("-rw_timeout")
                .arg(&timeout_micros)
                .arg("-rtsp_transport")
                .arg("tcp")
                .input(&url)
                .duration(dur.to_string())
                .format("null")
                .output("-");
            let mut child = cmd.spawn().context("spawn ffmpeg")?;
            let iter = child.iter().context("ffmpeg iter")?;
            for event in iter {
                if let Some(l) = log.as_mut() {
                    match &event {
                        FfmpegEvent::Log(_, msg) => l.write_line(msg)?,
                        FfmpegEvent::Progress(p) => l.write_line(&format!(
                            "progress frame={} fps={} bitrate_kbps={} time={} speed={}",
                            p.frame, p.fps, p.bitrate_kbps, p.time, p.speed
                        ))?,
                        FfmpegEvent::Done => l.write_line("done")?,
                        other => l.write_line(&format!("event={:?}", other))?,
                    }
                }
            }
            Ok::<_, anyhow::Error>(())
        }));
    }
    let mut failed = 0u32;
    for (i, h) in handles.into_iter().enumerate() {
        if let Err(e) = h.await.context("join")? {
            failed += 1;
            debug!(client_index = i, error = ?e, "ffmpeg: concurrent client failed");
        }
    }
    Ok(failed)
}

async fn harness_long_duration(
    url: &str,
    long_duration_sec: u64,
    artifacts_dir: &Path,
    capture_tool_output: bool,
) -> Result<u32> {
    let url = url.to_string();
    let dur = long_duration_sec;
    let log_path = artifacts_dir.join("ffmpeg_long_duration.log");
    let degradation = tokio::task::spawn_blocking(move || {
        let mut log = if capture_tool_output {
            Some(BoundedLogWriter::create(&log_path)?)
        } else {
            None
        };
        if let Some(l) = log.as_mut() {
            l.write_line("=== ffmpeg long duration ===")?;
            l.write_line(&format!("url={}", redact_url_credentials(&url)))?;
            l.write_line(&format!("duration_sec={}", dur))?;
        }
        let mut cmd = FfmpegCommand::new();
        cmd.hide_banner()
            .arg("-rtsp_transport")
            .arg("tcp")
            .input(&url)
            .duration(dur.to_string())
            .format("null")
            .output("-");
        let mut child = cmd.spawn().context("spawn ffmpeg")?;
        let iter = child.iter().context("ffmpeg iter")?;
        let mut first_bitrate = None::<f32>;
        let mut last_bitrate = None::<f32>;
        let mut progress_count: u32 = 0;
        for event in iter {
            if let Some(l) = log.as_mut() {
                match &event {
                    FfmpegEvent::Log(_, msg) => l.write_line(msg)?,
                    FfmpegEvent::Progress(p) => l.write_line(&format!(
                        "progress frame={} fps={} bitrate_kbps={} time={} speed={}",
                        p.frame, p.fps, p.bitrate_kbps, p.time, p.speed
                    ))?,
                    FfmpegEvent::Done => l.write_line("done")?,
                    other => l.write_line(&format!("event={:?}", other))?,
                }
            }
            if let FfmpegEvent::Progress(FfmpegProgress {
                frame,
                fps: _fps,
                bitrate_kbps: b,
                ..
            }) = &event
            {
                if first_bitrate.is_none() {
                    first_bitrate = Some(*b);
                }
                last_bitrate = Some(*b);
                progress_count += 1;
                if progress_count.is_multiple_of(500) {
                    debug!(
                        frame = *frame,
                        bitrate_kbps = *b,
                        "ffmpeg: long duration progress"
                    );
                }
            }
        }
        let (f, l) = match (first_bitrate, last_bitrate) {
            (Some(f), Some(l)) if f > 0.0 => (f as f64, l as f64),
            _ => return Ok::<_, anyhow::Error>(0u32),
        };
        let deg = (100.0_f64 * (1.0 - l / f)) as u32;
        Ok(deg)
    })
    .await
    .context("spawn_blocking")??;
    Ok(degradation)
}

async fn harness_error_handling(
    host: &str,
    port: u16,
    stream: &str,
    credentials: Option<(&str, &str)>,
    timeout_sec: u64,
    artifacts_dir: &Path,
    capture_tool_output: bool,
) -> Result<(bool, bool)> {
    let (username, password) = match credentials {
        Some((username, password)) => (Some(username), Some(password)),
        None => (None, None),
    };
    let invalid_url =
        rtsp_url_with_credentials(host, port, stream, Some("invalid"), Some("invalid"))?;
    let bogus_url = rtsp_url_with_credentials(host, port, "/bogus_stream", username, password)?;
    let timeout_micros = timeout_sec.saturating_mul(1_000_000).to_string();
    let invalid_timeout_micros = timeout_micros.clone();

    let invalid_log_path = artifacts_dir.join("ffmpeg_error_invalid_creds.log");
    let invalid_ok = tokio::task::spawn_blocking(move || {
        let mut log = if capture_tool_output {
            Some(BoundedLogWriter::create(&invalid_log_path)?)
        } else {
            None
        };
        if let Some(l) = log.as_mut() {
            l.write_line("=== ffmpeg error handling: invalid creds ===")?;
            l.write_line(&format!("url={}", redact_url_credentials(&invalid_url)))?;
        }
        let mut cmd = FfmpegCommand::new();
        cmd.hide_banner()
            .arg("-nostdin")
            .arg("-rw_timeout")
            .arg(&invalid_timeout_micros)
            .arg("-rtsp_transport")
            .arg("tcp")
            .input(&invalid_url)
            .duration("0.1")
            .format("null")
            .output("-");
        let mut child = cmd.spawn().context("spawn ffmpeg")?;
        let iter = child.iter().context("ffmpeg iter")?;
        let mut saw_401 = false;
        for event in iter {
            if let Some(l) = log.as_mut() {
                match &event {
                    FfmpegEvent::Log(_, msg) => l.write_line(msg)?,
                    FfmpegEvent::Progress(p) => l.write_line(&format!(
                        "progress frame={} fps={} bitrate_kbps={} time={} speed={}",
                        p.frame, p.fps, p.bitrate_kbps, p.time, p.speed
                    ))?,
                    FfmpegEvent::Done => l.write_line("done")?,
                    other => l.write_line(&format!("event={:?}", other))?,
                }
            }
            if let FfmpegEvent::Log(_, msg) = &event
                && (msg.contains("401") || msg.contains("Unauthorized"))
            {
                saw_401 = true;
                break;
            }
        }
        Ok::<_, anyhow::Error>(saw_401)
    })
    .await
    .context("spawn_blocking")??;

    let bogus_log_path = artifacts_dir.join("ffmpeg_error_bogus_url.log");
    let bogus_timeout_micros = timeout_micros.clone();
    let bogus_ok = tokio::task::spawn_blocking(move || {
        let mut log = if capture_tool_output {
            Some(BoundedLogWriter::create(&bogus_log_path)?)
        } else {
            None
        };
        if let Some(l) = log.as_mut() {
            l.write_line("=== ffmpeg error handling: bogus url ===")?;
            l.write_line(&format!("url={}", redact_url_credentials(&bogus_url)))?;
        }
        let mut cmd = FfmpegCommand::new();
        cmd.hide_banner()
            .arg("-nostdin")
            .arg("-rw_timeout")
            .arg(&bogus_timeout_micros)
            .arg("-rtsp_transport")
            .arg("tcp")
            .input(&bogus_url)
            .duration("0.1")
            .format("null")
            .output("-");
        let mut child = cmd.spawn().context("spawn ffmpeg")?;
        let iter = child.iter().context("ffmpeg iter")?;
        let mut saw_404 = false;
        for event in iter {
            if let Some(l) = log.as_mut() {
                match &event {
                    FfmpegEvent::Log(_, msg) => l.write_line(msg)?,
                    FfmpegEvent::Progress(p) => l.write_line(&format!(
                        "progress frame={} fps={} bitrate_kbps={} time={} speed={}",
                        p.frame, p.fps, p.bitrate_kbps, p.time, p.speed
                    ))?,
                    FfmpegEvent::Done => l.write_line("done")?,
                    other => l.write_line(&format!("event={:?}", other))?,
                }
            }
            if let FfmpegEvent::Log(_, msg) = &event
                && (msg.contains("404") || msg.contains("Not Found"))
            {
                saw_404 = true;
                break;
            }
        }
        Ok::<_, anyhow::Error>(saw_404)
    })
    .await
    .context("spawn_blocking")??;

    Ok((invalid_ok, bogus_ok))
}

#[cfg(test)]
mod tests {
    use super::{
        BoundedLogWriter, RtpTsharkRow, bitrate_within_tolerance, build_sdp_test_results,
        compute_packet_loss_from_seqs, critical_proto_failed, empty_report, fps_within_tolerance,
        group_rtp_rows_by_stream, packet_loss_within_tolerance, parse_rtsp_method_from_meta,
        parse_rtsp_status_code_from_meta, parse_tshark_hex_bytes, parse_tshark_rtp_row_line,
        pick_primary_audio_stream, pick_primary_video_stream, redact_url_credentials, result_ok,
        rtsp_url, rtsp_url_with_credentials, to_retina_initial_timestamp_policy,
        to_retina_transport, validate_aac_rtp_payload_rfc3640, validate_h264_length_prefixed_nals,
        validate_h264_rtp_payload_rfc6184,
    };
    use crate::config::{InitialTimestampPolicyArg, TransportArg};
    use crate::report::{StreamInfo, TestResult, TestRun};
    use crate::util::MAX_TOOL_LOG_BYTES;

    #[test]
    fn test_result_ok_pass() {
        assert!(result_ok(&TestResult::pass("x")));
    }

    #[test]
    fn test_result_ok_fail() {
        assert!(!result_ok(&TestResult::fail("x", "reason")));
    }

    #[test]
    fn test_result_ok_metric_pass() {
        assert!(result_ok(&TestResult::metric(
            "m",
            serde_json::json!(1),
            true
        )));
    }

    #[test]
    fn test_result_ok_metric_fail() {
        assert!(!result_ok(&TestResult::metric(
            "m",
            serde_json::json!(1),
            false
        )));
    }

    #[test]
    fn test_critical_proto_failed_no_fail() {
        let tests = vec![
            TestResult::pass("a"),
            TestResult::metric("b", serde_json::json!(1), true),
        ];
        assert!(!critical_proto_failed(&tests));
    }

    #[test]
    fn test_critical_proto_failed_describe_ok() {
        let tests = vec![TestResult::fail("describe_ok", "err")];
        assert!(critical_proto_failed(&tests));
    }

    #[test]
    fn test_critical_proto_failed_play_ok() {
        let tests = vec![TestResult::fail("play_ok", "err")];
        assert!(critical_proto_failed(&tests));
    }

    #[test]
    fn test_critical_proto_failed_setup_stream() {
        let tests = vec![TestResult::fail("setup_stream_0", "err")];
        assert!(critical_proto_failed(&tests));
    }

    #[test]
    fn test_critical_proto_failed_other_fail_ignored() {
        let tests = vec![TestResult::fail("other_test", "err")];
        assert!(!critical_proto_failed(&tests));
    }

    #[test]
    fn test_empty_report() {
        let test_run = TestRun {
            timestamp: "2025-01-01T00:00:00Z".to_string(),
            rtsp_host: "127.0.0.1".to_string(),
            rtsp_port: 554,
            rtsp_stream: "/stream1".to_string(),
            test_duration_seconds: 30,
            httpflv_port: None,
            httpflv_path: None,
        };
        let tests = vec![TestResult::pass("a")];
        let report = empty_report(test_run, tests);
        assert_eq!(report.test_run.rtsp_host, "127.0.0.1");
        assert_eq!(report.tests.len(), 1);
        assert_eq!(report.summary.total_tests, 0);
        assert_eq!(report.summary.passed, 0);
        assert_eq!(report.summary.failed, 0);
        assert!(!report.summary.overall_pass);
    }

    #[test]
    fn test_rtsp_url() {
        assert_eq!(
            rtsp_url("192.168.1.1", 8554, "/live"),
            "rtsp://192.168.1.1:8554/live"
        );
    }

    #[test]
    fn test_rtsp_url_with_credentials_encodes_and_roundtrips() {
        let url = rtsp_url_with_credentials(
            "192.168.1.1",
            8554,
            "/live",
            Some("user@name"),
            Some("p@ss:wo/rd"),
        )
        .unwrap();
        let parsed = url::Url::parse(&url).unwrap();
        assert!(parsed.as_str().contains("user%40name"));
        assert!(parsed.as_str().contains("p%40ss%3Awo%2Frd"));
        assert_eq!(parsed.path(), "/live");
    }

    #[test]
    fn test_rtsp_url_with_credentials_requires_complete_pair() {
        let result = rtsp_url_with_credentials("127.0.0.1", 554, "/stream1", Some("user"), None);
        assert!(result.is_err());
    }

    #[test]
    fn test_redact_url_credentials() {
        let redacted = redact_url_credentials("rtsp://user:pass@127.0.0.1:554/stream1");
        assert_eq!(redacted, "rtsp://REDACTED:REDACTED@127.0.0.1:554/stream1");
        assert_eq!(
            redact_url_credentials("rtsp://127.0.0.1:554/stream1"),
            "rtsp://127.0.0.1:554/stream1"
        );
    }

    #[test]
    fn test_to_retina_transport() {
        let tcp = to_retina_transport(TransportArg::Tcp);
        assert!(matches!(tcp, retina::client::Transport::Tcp(_)));
        let udp = to_retina_transport(TransportArg::Udp);
        assert!(matches!(udp, retina::client::Transport::Udp(_)));
    }

    #[test]
    fn test_to_retina_initial_timestamp_policy() {
        assert!(matches!(
            to_retina_initial_timestamp_policy(InitialTimestampPolicyArg::Default),
            retina::client::InitialTimestampPolicy::Default
        ));
        assert!(matches!(
            to_retina_initial_timestamp_policy(InitialTimestampPolicyArg::Require),
            retina::client::InitialTimestampPolicy::Require
        ));
        assert!(matches!(
            to_retina_initial_timestamp_policy(InitialTimestampPolicyArg::Ignore),
            retina::client::InitialTimestampPolicy::Ignore
        ));
        assert!(matches!(
            to_retina_initial_timestamp_policy(InitialTimestampPolicyArg::Permissive),
            retina::client::InitialTimestampPolicy::Permissive
        ));
    }

    #[test]
    fn test_parse_rtsp_method_from_meta_primary_field() {
        assert_eq!(
            parse_rtsp_method_from_meta("rtsp.method", "DESCRIBE"),
            Some("DESCRIBE")
        );
    }

    #[test]
    fn test_parse_rtsp_method_from_meta_request_line_fallback() {
        assert_eq!(
            parse_rtsp_method_from_meta("rtsp.request", "PLAY rtsp://x RTSP/1.0\r\n"),
            Some("PLAY")
        );
    }

    #[test]
    fn test_parse_rtsp_status_code_from_meta_legacy_and_current_fields() {
        assert_eq!(
            parse_rtsp_status_code_from_meta("rtsp.status_code", "200"),
            Some(200)
        );
        assert_eq!(
            parse_rtsp_status_code_from_meta("rtsp.status", "404"),
            Some(404)
        );
        assert_eq!(
            parse_rtsp_status_code_from_meta("rtsp.status", "404 Not Found"),
            Some(404)
        );
    }

    #[test]
    fn test_parse_rtsp_status_code_from_meta_response_line_fallback() {
        assert_eq!(
            parse_rtsp_status_code_from_meta("rtsp.response", "RTSP/1.0 200 OK\r\n"),
            Some(200)
        );
        assert_eq!(
            parse_rtsp_status_code_from_meta("rtsp.response", "RTSP/1.0 503 Busy\r\n"),
            Some(503)
        );
    }

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
    fn test_validate_aac_rfc3640_single_au_ok() {
        // AU-headers-length = 16 bits, AU-size=4 bytes, index=0, then 4 bytes data
        let payload = [0x00, 0x10, 0x00, 0x20, 0xDE, 0xAD, 0xBE, 0xEF];
        let (ok, au_header_invalid, au_size_invalid) = validate_aac_rtp_payload_rfc3640(&payload);
        assert!(ok);
        assert!(!au_header_invalid);
        assert!(!au_size_invalid);
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
    fn test_validate_h264_length_prefixed_nals_ok_single() {
        let data = [0, 0, 0, 1, 0x65];
        validate_h264_length_prefixed_nals(&data).unwrap();
    }

    #[test]
    fn test_validate_h264_length_prefixed_nals_rejects_truncated() {
        let data = [0, 0, 0, 2, 0x65];
        assert!(validate_h264_length_prefixed_nals(&data).is_err());
    }

    #[test]
    fn test_validate_h264_length_prefixed_nals_empty_ok() {
        validate_h264_length_prefixed_nals(&[]).unwrap();
    }

    #[test]
    fn test_validate_h264_length_prefixed_nals_trailing_bytes_rejected() {
        let data = [0, 0, 0, 1, 0x65, 0x00]; // 1 trailing byte
        assert!(validate_h264_length_prefixed_nals(&data).is_err());
    }

    #[test]
    fn test_validate_h264_length_prefixed_nals_zero_length_nal_rejected() {
        let data = [0, 0, 0, 0, 0x65]; // length 0
        assert!(validate_h264_length_prefixed_nals(&data).is_err());
    }

    #[test]
    fn test_validate_h264_length_prefixed_nals_nal_type_zero_rejected() {
        let data = [0, 0, 0, 1, 0x00]; // NAL type 0
        assert!(validate_h264_length_prefixed_nals(&data).is_err());
    }

    #[test]
    fn test_validate_h264_length_prefixed_nals_multiple_nals_ok() {
        let data = [
            0, 0, 0, 1, 0x67, // NAL 1
            0, 0, 0, 1, 0x68, // NAL 2
        ];
        validate_h264_length_prefixed_nals(&data).unwrap();
    }

    #[test]
    fn test_bounded_log_writer_writes_lines() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("log.txt");
        let mut w = BoundedLogWriter::create(&path).unwrap();
        w.write_line("line1").unwrap();
        w.write_line("line2").unwrap();
        drop(w);
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, "line1\nline2\n");
    }

    fn stream_info(media: &str, encoding: &str, control: bool) -> StreamInfo {
        StreamInfo {
            media: media.to_string(),
            encoding_name: encoding.to_string(),
            control_present: control,
        }
    }

    #[test]
    fn test_build_sdp_test_results_empty() {
        let results = build_sdp_test_results(&[]);
        assert_eq!(results.len(), 6);
        // stream_count 0 -> fail, sdp_streams, sdp_has_video fail, video_encoding_h264 pass (no video), sdp_has_audio, multitrack pass
        let names: Vec<_> = results.iter().map(|r| r.name()).collect();
        assert!(names.contains(&"stream_count"));
        assert!(names.contains(&"sdp_has_video"));
        assert!(names.contains(&"multitrack_controls_present"));
    }

    #[test]
    fn test_build_sdp_test_results_video_h264() {
        let infos = vec![stream_info("video", "h264", true)];
        let results = build_sdp_test_results(&infos);
        assert!(
            results
                .iter()
                .any(|r| r.name() == "sdp_has_video" && result_ok(r))
        );
        assert!(
            results
                .iter()
                .any(|r| r.name() == "video_encoding_h264" && result_ok(r))
        );
        assert!(results.iter().any(|r| r.name() == "sdp_has_audio")); // metric false
    }

    #[test]
    fn test_build_sdp_test_results_video_not_h264() {
        let infos = vec![stream_info("video", "mpeg4", true)];
        let results = build_sdp_test_results(&infos);
        assert!(
            results
                .iter()
                .any(|r| r.name() == "video_encoding_h264" && !result_ok(r))
        );
    }

    #[test]
    fn test_build_sdp_test_results_multitrack_control_missing() {
        let infos = vec![
            stream_info("video", "h264", true),
            stream_info("audio", "aac", false),
        ];
        let results = build_sdp_test_results(&infos);
        assert!(
            results
                .iter()
                .any(|r| r.name() == "multitrack_controls_present" && !result_ok(r))
        );
    }

    #[test]
    fn test_build_sdp_test_results_multitrack_all_control() {
        let infos = vec![
            stream_info("video", "h264", true),
            stream_info("audio", "aac", true),
        ];
        let results = build_sdp_test_results(&infos);
        assert!(
            results
                .iter()
                .any(|r| r.name() == "multitrack_controls_present" && result_ok(r))
        );
    }

    #[test]
    fn test_bitrate_within_tolerance() {
        assert!(bitrate_within_tolerance(1000.0, 1000.0, 15));
        assert!(bitrate_within_tolerance(1150.0, 1000.0, 15));
        assert!(bitrate_within_tolerance(850.0, 1000.0, 15));
        assert!(!bitrate_within_tolerance(1200.0, 1000.0, 15));
        assert!(bitrate_within_tolerance(100.0, 0.0, 15));
    }

    #[test]
    fn test_fps_within_tolerance() {
        assert!(fps_within_tolerance(30.0, 30.0, 10));
        assert!(fps_within_tolerance(33.0, 30.0, 10));
        assert!(fps_within_tolerance(27.0, 30.0, 10));
        assert!(!fps_within_tolerance(25.0, 30.0, 10));
    }

    #[test]
    fn test_packet_loss_within_tolerance() {
        assert!(packet_loss_within_tolerance(0.5, 1.0));
        assert!(packet_loss_within_tolerance(1.0, 1.0));
        assert!(!packet_loss_within_tolerance(1.5, 1.0));
    }

    #[test]
    fn test_bounded_log_writer_truncates_at_limit() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("log.txt");
        let mut w = BoundedLogWriter::create(&path).unwrap();
        let line = "x".repeat(1000);
        let mut written = 0;
        while written <= MAX_TOOL_LOG_BYTES {
            w.write_line(&line).unwrap();
            written += line.len() + 1;
        }
        drop(w);
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.len() <= MAX_TOOL_LOG_BYTES + 1024);
        assert!(content.trim_end().ends_with('x'));
    }
}
