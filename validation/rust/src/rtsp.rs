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
use crate::report::{StreamInfo, TestResult, TestRun, ValidationReport, result_ok};
use crate::util::{
    MAX_TOOL_LOG_BYTES, parse_ffmpeg_summary_bitrate_kbps, tail_lossy, write_bytes_tail,
};

const PROBE_DEMUX_ERROR_TOLERANCE: u32 = 3;
const PROTOCOL_SEQUENCE_CAPTURE_MAX_DURATION_SEC: u64 = 12;
const MIN_HARNESS_FPS: f64 = 5.0;
const MIN_HARNESS_BITRATE_KBPS: f64 = 1.0;

#[derive(Debug, Clone, Copy, Default)]
struct RtspProtocolSequenceStats {
    describe: u32,
    setup: u32,
    play: u32,
    teardown: u32,
    status_200: u32,
    status_4xx: u32,
    status_401: u32,
    status_5xx: u32,
}

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

fn parse_status_code(value: &str) -> Option<u32> {
    value
        .split_whitespace()
        .find_map(|token| token.parse::<u32>().ok())
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
    pacing: Option<FramePacing>,
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

/// Video RTP media clock (Hz). H.264 uses 90 kHz per RFC 6184.
const VIDEO_RTP_CLOCK_HZ: f64 = 90_000.0;

/// Gap statistics for one cadence (encoder RTP timestamps or arrival wall-clock).
#[derive(Debug, Clone, Default, Serialize)]
struct GapStats {
    count: u32,
    min_ms: f64,
    median_ms: f64,
    p90_ms: f64,
    p99_ms: f64,
    max_ms: f64,
    delay_count: u32,
    delay_percent: f64,
}

/// Frame pacing measurement for the primary video stream.
#[derive(Debug, Clone, Default, Serialize)]
struct FramePacing {
    expected_fps: f64,
    nominal_ms: f64,
    delay_multiple: f64,
    delay_floor_ms: f64,
    encoder: GapStats,
    #[serde(skip_serializing_if = "Option::is_none")]
    arrival: Option<GapStats>,
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
/// well-defined (same assumption as `compute_packet_loss_from_seqs`).
fn encoder_deltas_ms(rows: &[RtpTsharkRow]) -> Vec<f64> {
    let mut deltas = Vec::new();
    let mut prev_ts: Option<u32> = None;
    for row in rows {
        if let Some(prev) = prev_ts
            && row.timestamp != prev
        {
            let delta = row.timestamp.wrapping_sub(prev);
            if delta > 0 {
                deltas.push(delta as f64 / VIDEO_RTP_CLOCK_HZ * 1000.0);
            }
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
/// Returns `None` when there is no usable data (no rows, no expected fps, or
/// fewer than two frames). Arrival cadence is skipped when the pcap lacks
/// wall-clock times (`time_epoch_sec`), e.g. a legacy capture.
fn compute_pacing(
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
    /// Wall-clock arrival time (seconds since epoch) of the packet, if captured.
    time_epoch_sec: Option<f64>,
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

pub fn critical_proto_failed(tests: &[TestResult]) -> bool {
    tests.iter().any(|t| {
        if let TestResult::Fail { name, .. } = t {
            // Strip optional stream prefix (e.g. "main:describe_ok" → "describe_ok").
            let base = name.rsplit(':').next().unwrap_or(name);
            base == "describe_ok" || base == "play_ok" || base.starts_with("setup_stream_")
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

fn harness_bitrate_pass(
    measured_kbps: f64,
    expected_kbps: Option<f64>,
    tolerance_percent: u32,
) -> bool {
    if measured_kbps < MIN_HARNESS_BITRATE_KBPS {
        return false;
    }
    expected_kbps
        .map(|e| bitrate_within_tolerance(measured_kbps, e, tolerance_percent))
        .unwrap_or(true)
}

fn harness_fps_pass(measured_fps: f64, expected_fps: Option<f64>, tolerance_percent: u32) -> bool {
    if measured_fps < MIN_HARNESS_FPS {
        return false;
    }
    expected_fps
        .map(|e| fps_within_tolerance(measured_fps, e, tolerance_percent))
        .unwrap_or(true)
}

fn harness_protocol_sequence_pass(stats: &RtspProtocolSequenceStats) -> bool {
    let non_auth_4xx = stats.status_4xx.saturating_sub(stats.status_401);
    stats.describe > 0
        && stats.setup > 0
        && stats.play > 0
        && stats.status_200 > 0
        && non_auth_4xx == 0
        && stats.status_5xx == 0
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

/// Open a per-step ffmpeg log (when capturing) and write its standard header.
fn ffmpeg_log(
    artifacts_dir: &Path,
    file_name: &str,
    capture: bool,
    header: &str,
    url: &str,
    duration_sec: Option<u64>,
) -> Result<Option<BoundedLogWriter>> {
    if !capture {
        return Ok(None);
    }
    let mut log = BoundedLogWriter::create(&artifacts_dir.join(file_name))?;
    log.write_line(&format!("=== {} ===", header))?;
    log.write_line(&format!("url={}", redact_url_credentials(url)))?;
    if let Some(sec) = duration_sec {
        log.write_line(&format!("duration_sec={}", sec))?;
    }
    Ok(Some(log))
}

/// Spawn ffmpeg, mirror every event into `log`, and hand each event to `on_event`.
///
/// `on_event` returns `false` to stop reading events early (the child is dropped).
fn run_ffmpeg(
    mut log: Option<&mut BoundedLogWriter>,
    configure: impl FnOnce(&mut FfmpegCommand),
    mut on_event: impl FnMut(&FfmpegEvent) -> bool,
) -> Result<()> {
    let mut cmd = FfmpegCommand::new();
    cmd.hide_banner();
    configure(&mut cmd);
    let mut child = cmd.spawn().context("spawn ffmpeg")?;
    for event in child.iter().context("ffmpeg iter")? {
        if let Some(l) = log.as_deref_mut() {
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
        if !on_event(&event) {
            break;
        }
    }
    Ok(())
}

/// Pull a stream for `duration_sec` purely to generate traffic for a tshark capture.
#[allow(clippy::too_many_arguments)]
fn drain_ffmpeg(
    artifacts_dir: &Path,
    file_name: &str,
    capture: bool,
    header: &str,
    url: &str,
    duration_sec: u64,
    transport: &str,
) -> Result<()> {
    let mut log = ffmpeg_log(
        artifacts_dir,
        file_name,
        capture,
        header,
        url,
        Some(duration_sec),
    )?;
    run_ffmpeg(
        log.as_mut(),
        |cmd| {
            cmd.arg("-rtsp_transport")
                .arg(transport)
                .input(url)
                .duration(duration_sec.to_string())
                .format("null")
                .output("-");
        },
        |_| true,
    )
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
            latency_ms <= effective.video_startup_latency_ms,
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

#[derive(Debug)]
enum HarnessProcessOutcome<T> {
    Success(T),
    Error(String),
    Timeout,
}

impl<T> HarnessProcessOutcome<T> {
    fn is_timeout(&self) -> bool {
        matches!(self, Self::Timeout)
    }
}

fn to_harness_process_outcome<T>(
    result: std::result::Result<Result<T>, tokio::time::error::Elapsed>,
) -> HarnessProcessOutcome<T> {
    match result {
        Ok(Ok(value)) => HarnessProcessOutcome::Success(value),
        Ok(Err(e)) => HarnessProcessOutcome::Error(e.to_string()),
        Err(_) => HarnessProcessOutcome::Timeout,
    }
}

fn harness_timeout_reason(step_cap: Duration) -> String {
    format!("harness step timed out after {}s", step_cap.as_secs())
}

/// Unwrap a harness step outcome, pushing a `fail(name, …)` for error/timeout.
///
/// Every harness step reports failure the same way; only the success arm differs.
fn harness_value<T>(
    tests: &mut Vec<TestResult>,
    name: &str,
    outcome: HarnessProcessOutcome<T>,
    step_cap: Duration,
) -> Option<T> {
    let reason = match outcome {
        HarnessProcessOutcome::Success(value) => return Some(value),
        HarnessProcessOutcome::Error(error) => error,
        HarnessProcessOutcome::Timeout => harness_timeout_reason(step_cap),
    };
    tests.push(TestResult::fail(name, reason));
    None
}

fn append_harness_basic_connectivity_result(
    tests: &mut Vec<TestResult>,
    outcome: HarnessProcessOutcome<bool>,
    step_cap: Duration,
) {
    const NAME: &str = "harness_basic_connectivity";
    let Some(ok) = harness_value(tests, NAME, outcome, step_cap) else {
        return;
    };
    tests.push(if ok {
        TestResult::pass(NAME)
    } else {
        TestResult::fail(NAME, "no stream in output")
    });
}

fn append_harness_startup_latency_result(
    tests: &mut Vec<TestResult>,
    outcome: HarnessProcessOutcome<Option<u64>>,
    target_ms: u64,
    step_cap: Duration,
) {
    const NAME: &str = "harness_startup_latency_ms";
    let Some(ms) = harness_value(tests, NAME, outcome, step_cap) else {
        return;
    };
    tests.push(match ms {
        Some(ms) => TestResult::metric(NAME, serde_json::json!(ms), ms <= target_ms),
        None => TestResult::fail(NAME, "no frame decoded"),
    });
}

fn append_harness_bitrate_fps_result(
    tests: &mut Vec<TestResult>,
    outcome: HarnessProcessOutcome<(f64, f64)>,
    expected_bitrate_kbps: Option<f64>,
    bitrate_tolerance_percent: u32,
    expected_fps: Option<f64>,
    fps_tolerance_percent: u32,
    step_cap: Duration,
) {
    let Some((bitrate, fps)) = harness_value(tests, "harness_bitrate_fps", outcome, step_cap)
    else {
        return;
    };
    tests.push(TestResult::metric(
        "harness_bitrate_kbps",
        serde_json::json!(bitrate),
        harness_bitrate_pass(bitrate, expected_bitrate_kbps, bitrate_tolerance_percent),
    ));
    tests.push(TestResult::metric(
        "harness_fps",
        serde_json::json!(fps),
        harness_fps_pass(fps, expected_fps, fps_tolerance_percent),
    ));
}

fn append_harness_sdp_validation_result(
    tests: &mut Vec<TestResult>,
    outcome: HarnessProcessOutcome<(usize, usize, bool, bool)>,
    step_cap: Duration,
) -> (bool, bool) {
    let Some((video_count, audio_count, has_h264, has_aac)) =
        harness_value(tests, "harness_sdp_validation", outcome, step_cap)
    else {
        return (false, false);
    };
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
    (has_h264, has_aac)
}

fn append_harness_protocol_sequence_result(
    tests: &mut Vec<TestResult>,
    outcome: HarnessProcessOutcome<RtspProtocolSequenceStats>,
    step_cap: Duration,
) {
    const NAME: &str = "harness_protocol_sequence";
    let Some(stats) = harness_value(tests, NAME, outcome, step_cap) else {
        return;
    };
    tests.push(TestResult::metric(
        NAME,
        serde_json::json!({
            "describe": stats.describe,
            "setup": stats.setup,
            "play": stats.play,
            "teardown": stats.teardown,
            "status_200": stats.status_200,
            "status_4xx": stats.status_4xx,
            "status_401_auth_challenge": stats.status_401,
            "status_non_auth_4xx": stats.status_4xx.saturating_sub(stats.status_401),
            "status_5xx": stats.status_5xx,
        }),
        harness_protocol_sequence_pass(&stats),
    ));
}

fn append_harness_packet_loss_result(
    tests: &mut Vec<TestResult>,
    outcome: HarnessProcessOutcome<HarnessPacketLossResult>,
    packet_loss_tolerance_percent: f64,
    pacing_tolerance_percent: f64,
    expect_h264: bool,
    expect_aac: bool,
    step_cap: Duration,
) {
    let Some(res) = harness_value(tests, "harness_packet_loss", outcome, step_cap) else {
        return;
    };

    let within = |m: &HarnessRtpLossMetric| {
        packet_loss_within_tolerance(m.loss_percent, packet_loss_tolerance_percent)
    };
    let video_metric = res.video.clone().unwrap_or_default();
    tests.push(TestResult::metric(
        "harness_packet_loss_percent",
        serde_json::json!({
            "rtp_packets": video_metric.rtp_packets,
            "packet_loss": video_metric.packet_loss,
            "loss_percent": video_metric.loss_percent,
            "payload_type": video_metric.payload_type,
            "ssrc": video_metric.ssrc,
        }),
        res.video.as_ref().is_some_and(within),
    ));

    for (name, metric) in [
        ("harness_packet_loss_percent_video", &res.video),
        ("harness_packet_loss_percent_audio", &res.audio),
    ] {
        if let Some(m) = metric {
            tests.push(TestResult::metric(name, serde_json::json!(m), within(m)));
        }
    }

    append_pcap_rfc_result(
        tests,
        "harness_pcap_rfc6184_h264",
        expect_h264,
        res.h264_rfc6184
            .map(|r| r.map(|s| (serde_json::json!(s), s.packets_analyzed, s.invalid_packets))),
    );
    append_pcap_rfc_result(
        tests,
        "harness_pcap_rfc3640_aac",
        expect_aac,
        res.aac_rfc3640
            .map(|r| r.map(|s| (serde_json::json!(s), s.packets_analyzed, s.invalid_packets))),
    );

    if let Some(pacing) = &res.pacing {
        let ok = |g: &GapStats| g.delay_percent <= pacing_tolerance_percent;
        tests.push(TestResult::metric(
            "frame_pacing_encoder_delay_percent",
            serde_json::json!(pacing.encoder.delay_percent),
            ok(&pacing.encoder),
        ));
        tests.push(TestResult::metric(
            "frame_pacing_encoder_max_gap_ms",
            serde_json::json!({ "max_ms": pacing.encoder.max_ms }),
            true,
        ));
        if let Some(arrival) = &pacing.arrival {
            tests.push(TestResult::metric(
                "frame_pacing_arrival_delay_percent",
                serde_json::json!(arrival.delay_percent),
                ok(arrival),
            ));
            tests.push(TestResult::metric(
                "frame_pacing_arrival_max_gap_ms",
                serde_json::json!({ "max_ms": arrival.max_ms }),
                true,
            ));
        }
        tests.push(TestResult::metric(
            "frame_pacing",
            serde_json::json!(pacing),
            ok(&pacing.encoder) && pacing.arrival.as_ref().is_none_or(ok),
        ));
    }
}

/// Report one pcap RFC-conformance check. Passes vacuously when the codec isn't expected.
fn append_pcap_rfc_result(
    tests: &mut Vec<TestResult>,
    name: &str,
    expected: bool,
    stats: Option<std::result::Result<(serde_json::Value, u32, u32), String>>,
) {
    if !expected {
        tests.push(TestResult::pass(name));
        return;
    }
    tests.push(match stats {
        Some(Ok((value, analyzed, invalid))) => {
            TestResult::metric(name, value, analyzed > 0 && invalid == 0)
        }
        Some(Err(e)) => TestResult::fail(name, e),
        None => TestResult::fail(name, "pcap validation skipped unexpectedly"),
    });
}

fn append_harness_concurrent_clients_result(
    tests: &mut Vec<TestResult>,
    outcome: HarnessProcessOutcome<u32>,
    requested: u32,
    step_cap: Duration,
) {
    const NAME: &str = "harness_concurrent_clients";
    let Some(failed) = harness_value(tests, NAME, outcome, step_cap) else {
        return;
    };
    tests.push(TestResult::metric(
        NAME,
        serde_json::json!({ "requested": requested, "failed": failed }),
        failed == 0,
    ));
}

fn append_harness_long_duration_result(
    tests: &mut Vec<TestResult>,
    outcome: HarnessProcessOutcome<u32>,
    step_cap: Duration,
) {
    let Some(degradation_pct) = harness_value(tests, "harness_long_duration", outcome, step_cap)
    else {
        return;
    };
    tests.push(TestResult::metric(
        "harness_long_duration_degradation_pct",
        serde_json::json!(degradation_pct),
        degradation_pct < 20,
    ));
}

fn append_harness_error_handling_result(
    tests: &mut Vec<TestResult>,
    outcome: HarnessProcessOutcome<(bool, bool)>,
    step_cap: Duration,
) {
    let Some((invalid_creds_ok, bogus_url_ok)) =
        harness_value(tests, "harness_error_handling", outcome, step_cap)
    else {
        return;
    };
    for (name, ok) in [
        ("harness_error_invalid_creds", invalid_creds_ok),
        ("harness_error_bogus_url", bogus_url_ok),
    ] {
        tests.push(TestResult::metric(name, serde_json::json!(ok), ok));
    }
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
    debug!("harness: basic connectivity");
    let basic_connectivity = to_harness_process_outcome(
        timeout(
            step_cap_short,
            harness_basic_connectivity(
                &auth_url,
                &artifacts_dir,
                capture_tool_output,
                &effective.ffmpeg_log_level,
            ),
        )
        .await,
    );
    if basic_connectivity.is_timeout() {
        cleanup_timed_out_media_processes(&auth_url, "harness_basic_connectivity");
    }
    append_harness_basic_connectivity_result(tests, basic_connectivity, step_cap_short);

    debug!(
        url = %url,
        target_ms = effective.harness_startup_latency_ms,
        "harness: startup latency"
    );
    let startup_latency = to_harness_process_outcome(
        timeout(
            step_cap_short,
            harness_startup_latency(
                &auth_url,
                &artifacts_dir,
                capture_tool_output,
                &effective.ffmpeg_log_level,
            ),
        )
        .await,
    );
    if startup_latency.is_timeout() {
        cleanup_timed_out_media_processes(&auth_url, "harness_startup_latency_ms");
    }
    append_harness_startup_latency_result(
        tests,
        startup_latency,
        effective.harness_startup_latency_ms,
        step_cap_short,
    );

    debug!(url = %url, duration_sec = effective.short_duration_sec, "harness: bitrate/fps");
    let bitrate_fps = to_harness_process_outcome(
        timeout(
            step_cap_long,
            harness_bitrate_fps(
                &auth_url,
                effective.short_duration_sec,
                &artifacts_dir,
                capture_tool_output,
                &effective.ffmpeg_log_level,
            ),
        )
        .await,
    );
    if bitrate_fps.is_timeout() {
        cleanup_timed_out_media_processes(&auth_url, "harness_bitrate_fps");
    }
    append_harness_bitrate_fps_result(
        tests,
        bitrate_fps,
        effective.expected_bitrate_kbps,
        effective.bitrate_tolerance_percent,
        effective.expected_fps,
        effective.fps_tolerance_percent,
        step_cap_long,
    );

    debug!(url = %url, "harness: SDP validation");
    let sdp_validation = to_harness_process_outcome(
        timeout(
            step_cap_short,
            harness_sdp_validation(&auth_url, timeout_sec, &artifacts_dir, capture_tool_output),
        )
        .await,
    );
    if sdp_validation.is_timeout() {
        cleanup_timed_out_media_processes(&auth_url, "harness_sdp_validation");
    }
    let (expect_h264, expect_aac) =
        append_harness_sdp_validation_result(tests, sdp_validation, step_cap_short);

    debug!(url = %url, "harness: RTSP protocol sequence");
    let protocol_sequence = to_harness_process_outcome(
        timeout(
            step_cap_protocol_sequence,
            harness_rtsp_protocol_sequence(&auth_url, effective, args),
        )
        .await,
    );
    if protocol_sequence.is_timeout() {
        cleanup_timed_out_media_processes(&auth_url, "harness_protocol_sequence");
    }
    append_harness_protocol_sequence_result(tests, protocol_sequence, step_cap_protocol_sequence);

    debug!(url = %url, "harness: packet loss + pcap RFC checks");
    let packet_loss = to_harness_process_outcome(
        timeout(
            step_cap_long,
            harness_packet_loss(&auth_url, effective, args, expect_h264, expect_aac),
        )
        .await,
    );
    if packet_loss.is_timeout() {
        cleanup_timed_out_media_processes(&auth_url, "harness_packet_loss");
    }
    append_harness_packet_loss_result(
        tests,
        packet_loss,
        effective.packet_loss_tolerance_percent,
        effective.pacing_delay_tolerance_percent,
        expect_h264,
        expect_aac,
        step_cap_long,
    );

    if effective.concurrent_clients > 0 {
        debug!(url = %url, concurrent = effective.concurrent_clients, "harness: concurrent clients");
        let concurrent_clients = to_harness_process_outcome(
            timeout(
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
            .await,
        );
        if concurrent_clients.is_timeout() {
            cleanup_timed_out_media_processes(&auth_url, "harness_concurrent_clients");
        }
        append_harness_concurrent_clients_result(
            tests,
            concurrent_clients,
            effective.concurrent_clients,
            step_cap_long,
        );
    }

    if args.long_duration {
        let step_cap_long_duration =
            Duration::from_secs(effective.long_duration_sec.saturating_add(30));
        debug!(url = %url, duration_sec = effective.long_duration_sec, "harness: long duration");
        let long_duration = to_harness_process_outcome(
            timeout(
                step_cap_long_duration,
                harness_long_duration(
                    &auth_url,
                    effective.long_duration_sec,
                    &artifacts_dir,
                    capture_tool_output,
                ),
            )
            .await,
        );
        if long_duration.is_timeout() {
            cleanup_timed_out_media_processes(&auth_url, "harness_long_duration");
        }
        append_harness_long_duration_result(tests, long_duration, step_cap_long_duration);
    }

    if !args.skip_error_handling {
        debug!(host = %effective.rtsp_host, port = effective.rtsp_port, "harness: error handling");
        let error_handling = to_harness_process_outcome(
            timeout(
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
            .await,
        );
        if error_handling.is_timeout() {
            cleanup_timed_out_media_processes(&auth_url, "harness_error_handling");
        }
        append_harness_error_handling_result(tests, error_handling, step_cap_short);
    }

    Ok(())
}

async fn harness_basic_connectivity(
    url: &str,
    artifacts_dir: &Path,
    capture_tool_output: bool,
    ffmpeg_log_level: &str,
) -> Result<bool> {
    let url = url.to_string();
    let level = ffmpeg_log_level.to_string();
    let dir = artifacts_dir.to_path_buf();
    tokio::task::spawn_blocking(move || {
        let mut log = ffmpeg_log(
            &dir,
            "ffmpeg_basic_connectivity.log",
            capture_tool_output,
            "ffmpeg basic connectivity",
            &url,
            None,
        )?;
        let mut saw_stream = false;
        run_ffmpeg(
            log.as_mut(),
            |cmd| {
                cmd.arg("-loglevel")
                    .arg(&level)
                    .arg("-rtsp_transport")
                    .arg("tcp")
                    .input(&url)
                    .duration("0.1")
                    .format("null")
                    .output("-");
            },
            |event| {
                let found = match event {
                    FfmpegEvent::Log(_, msg) => msg.contains("Stream #"),
                    FfmpegEvent::Progress(_) | FfmpegEvent::Done => true,
                    _ => false,
                };
                if found {
                    saw_stream = true;
                }
                !found // stop as soon as we have an answer
            },
        )?;
        Ok::<_, anyhow::Error>(saw_stream)
    })
    .await
    .context("spawn_blocking")?
}

async fn harness_startup_latency(
    url: &str,
    artifacts_dir: &Path,
    capture_tool_output: bool,
    ffmpeg_log_level: &str,
) -> Result<Option<u64>> {
    let url = url.to_string();
    let level = ffmpeg_log_level.to_string();
    let dir = artifacts_dir.to_path_buf();
    tokio::task::spawn_blocking(move || {
        let mut log = ffmpeg_log(
            &dir,
            "ffmpeg_startup_latency.log",
            capture_tool_output,
            "ffmpeg startup latency",
            &url,
            None,
        )?;
        let start = std::time::Instant::now();
        let mut first_frame_ms = None;
        run_ffmpeg(
            log.as_mut(),
            |cmd| {
                cmd.arg("-loglevel")
                    .arg(&level)
                    .arg("-rtsp_transport")
                    .arg("tcp")
                    .input(&url)
                    .frames(1)
                    .format("null")
                    .output("-");
            },
            |event| match event {
                FfmpegEvent::Progress(FfmpegProgress { frame, .. }) if *frame > 0 => {
                    first_frame_ms = Some(start.elapsed().as_millis() as u64);
                    false
                }
                FfmpegEvent::Done => false,
                _ => true,
            },
        )?;
        Ok::<_, anyhow::Error>(first_frame_ms)
    })
    .await
    .context("spawn_blocking")?
}

async fn harness_bitrate_fps(
    url: &str,
    duration_sec: u64,
    artifacts_dir: &Path,
    capture_tool_output: bool,
    ffmpeg_log_level: &str,
) -> Result<(f64, f64)> {
    let url = url.to_string();
    let level = ffmpeg_log_level.to_string();
    let dir = artifacts_dir.to_path_buf();
    tokio::task::spawn_blocking(move || {
        let mut log = ffmpeg_log(
            &dir,
            "ffmpeg_bitrate_fps.log",
            capture_tool_output,
            "ffmpeg bitrate/fps",
            &url,
            Some(duration_sec),
        )?;
        let mut last_bitrate = 0.0_f64;
        let mut last_fps = 0.0_f64;
        let mut summary_bitrate_kbps = None;
        let mut progress_count: u32 = 0;
        run_ffmpeg(
            log.as_mut(),
            |cmd| {
                cmd.arg("-loglevel")
                    .arg(&level)
                    .arg("-rtsp_transport")
                    .arg("tcp")
                    .input(&url)
                    .duration(duration_sec.to_string())
                    .format("null")
                    .output("-");
            },
            |event| {
                match event {
                    FfmpegEvent::Log(_, msg) => {
                        if let Some(estimated) =
                            parse_ffmpeg_summary_bitrate_kbps(msg, duration_sec)
                        {
                            summary_bitrate_kbps = Some(estimated);
                        }
                    }
                    FfmpegEvent::Progress(p) => {
                        last_bitrate = p.bitrate_kbps as f64;
                        last_fps = p.fps as f64;
                        progress_count += 1;
                        if progress_count == 1 || progress_count.is_multiple_of(100) {
                            debug!(frame = p.frame, fps = p.fps, bitrate_kbps = p.bitrate_kbps, time = %p.time, speed = p.speed, "ffmpeg: bitrate/fps progress");
                        }
                    }
                    _ => {}
                }
                true
            },
        )?;
        let resolved_bitrate = if last_bitrate > 0.0 {
            last_bitrate
        } else {
            summary_bitrate_kbps.unwrap_or(0.0)
        };
        Ok::<_, anyhow::Error>((resolved_bitrate, last_fps))
    })
    .await
    .context("spawn_blocking")?
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
) -> Result<RtspProtocolSequenceStats> {
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
    let capture_dir = artifacts_dir.clone();
    tokio::task::spawn_blocking(move || {
        drain_ffmpeg(
            &capture_dir,
            "ffmpeg_protocol_sequence_capture.log",
            capture_tool_output,
            "ffmpeg protocol sequence capture",
            &url2,
            short_dur,
            "tcp",
        )
    })
    .await
    .context("ffmpeg join")??;

    tokio::time::sleep(Duration::from_secs(2)).await;
    let _ = tshark_handle.kill();
    let _ = tshark_handle.wait();

    let stats = tokio::task::spawn_blocking(move || {
        let stats = tshark_rtsp_sequence_stats(&pcap_path)?;
        if !keep_pcaps {
            let _ = std::fs::remove_file(&pcap_path);
        }
        Ok::<_, anyhow::Error>(stats)
    })
    .await
    .context("spawn_blocking")??;

    Ok(stats)
}

/// Count RTSP methods and response classes in `pcap` via `tshark -T fields`.
///
/// One field row per packet: `rtsp.method<TAB>rtsp.status`.
fn tshark_rtsp_sequence_stats(pcap_path: &Path) -> Result<RtspProtocolSequenceStats> {
    let out = Command::new("tshark")
        .arg("-r")
        .arg(pcap_path)
        .args([
            "-Y",
            "rtsp",
            "-T",
            "fields",
            "-E",
            "separator=\t",
            "-E",
            "occurrence=f",
            "-e",
            "rtsp.method",
            "-e",
            "rtsp.status",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .context("tshark -T fields (rtsp)")?;

    if !out.status.success() {
        bail!(
            "tshark rtsp parse failed (code={:?}): {}",
            out.status.code(),
            tail_lossy(String::from_utf8_lossy(&out.stderr).trim(), 1200)
        );
    }

    Ok(rtsp_sequence_stats_from_field_rows(
        &String::from_utf8_lossy(&out.stdout),
    ))
}

/// Tally `rtsp.method<TAB>rtsp.status` rows. Split out so it is testable without tshark.
fn rtsp_sequence_stats_from_field_rows(rows: &str) -> RtspProtocolSequenceStats {
    let mut stats = RtspProtocolSequenceStats::default();
    for line in rows.lines() {
        let mut cols = line.split('\t');
        let method = cols.next().unwrap_or("").trim();
        let status = cols.next().unwrap_or("").trim();

        match method {
            "DESCRIBE" => stats.describe += 1,
            "SETUP" => stats.setup += 1,
            "PLAY" => stats.play += 1,
            "TEARDOWN" => stats.teardown += 1,
            _ => {}
        }
        if let Some(status) = parse_status_code(status) {
            if (200..300).contains(&status) {
                stats.status_200 += 1;
            } else if (400..500).contains(&status) {
                stats.status_4xx += 1;
                if status == 401 {
                    stats.status_401 += 1;
                }
            } else if status >= 500 {
                stats.status_5xx += 1;
            }
        }
    }
    stats
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
    let pacing_expected_fps = effective.pacing_expected_fps;
    let pacing_delay_multiple = effective.pacing_delay_multiple;
    let pacing_delay_floor_ms = effective.pacing_delay_floor_ms;
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
    let capture_dir = artifacts_dir.clone();
    tokio::task::spawn_blocking(move || {
        drain_ffmpeg(
            &capture_dir,
            "ffmpeg_packet_loss_capture.log",
            capture_tool_output,
            "ffmpeg packet loss capture",
            &url2,
            short_dur_rtp,
            "udp",
        )
    })
    .await
    .context("ffmpeg join")??;

    tokio::time::sleep(Duration::from_secs(2)).await;
    let _ = tshark_handle_rtp.kill();
    let _ = tshark_handle_rtp.wait();

    let pcap_path_for_parse = pcap_path.clone();
    let (video, audio, h264_rfc6184, aac_rfc3640, pacing) =
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
            let pacing = video_stream.and_then(|s| {
                compute_pacing(
                    &s.rows,
                    pacing_expected_fps,
                    pacing_delay_multiple,
                    pacing_delay_floor_ms,
                )
            });

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

            Ok::<_, anyhow::Error>((
                video_metric,
                audio_metric,
                h264_rfc6184,
                aac_rfc3640,
                pacing,
            ))
    })
    .await
    .context("spawn_blocking")??;

    Ok(HarnessPacketLossResult {
        video,
        audio,
        h264_rfc6184,
        aac_rfc3640,
        pacing,
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
        let timeout_micros = timeout_micros.clone();
        let dir = artifacts_dir.to_path_buf();
        handles.push(tokio::task::spawn_blocking(move || {
            let mut log = ffmpeg_log(
                &dir,
                &format!("ffmpeg_concurrent_client_{}.log", i),
                capture_tool_output,
                &format!("ffmpeg concurrent client {}", i),
                &url,
                Some(duration_sec),
            )?;
            run_ffmpeg(
                log.as_mut(),
                |cmd| {
                    cmd.arg("-nostdin")
                        .arg("-rw_timeout")
                        .arg(&timeout_micros)
                        .arg("-rtsp_transport")
                        .arg("tcp")
                        .input(&url)
                        .duration(duration_sec.to_string())
                        .format("null")
                        .output("-");
                },
                |_| true,
            )
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
    let dir = artifacts_dir.to_path_buf();
    tokio::task::spawn_blocking(move || {
        let mut log = ffmpeg_log(
            &dir,
            "ffmpeg_long_duration.log",
            capture_tool_output,
            "ffmpeg long duration",
            &url,
            Some(long_duration_sec),
        )?;
        let mut first_bitrate = None::<f32>;
        let mut last_bitrate = None::<f32>;
        let mut progress_count: u32 = 0;
        run_ffmpeg(
            log.as_mut(),
            |cmd| {
                cmd.arg("-rtsp_transport")
                    .arg("tcp")
                    .input(&url)
                    .duration(long_duration_sec.to_string())
                    .format("null")
                    .output("-");
            },
            |event| {
                if let FfmpegEvent::Progress(p) = event {
                    first_bitrate.get_or_insert(p.bitrate_kbps);
                    last_bitrate = Some(p.bitrate_kbps);
                    progress_count += 1;
                    if progress_count.is_multiple_of(500) {
                        debug!(
                            frame = p.frame,
                            bitrate_kbps = p.bitrate_kbps,
                            "ffmpeg: long duration progress"
                        );
                    }
                }
                true
            },
        )?;
        let degradation = match (first_bitrate, last_bitrate) {
            (Some(f), Some(l)) if f > 0.0 => (100.0_f64 * (1.0 - l as f64 / f as f64)) as u32,
            _ => 0,
        };
        Ok::<_, anyhow::Error>(degradation)
    })
    .await
    .context("spawn_blocking")?
}

/// Probe `url` briefly and report whether ffmpeg logged any of `needles`.
///
/// Used for the negative checks: the server is expected to reject the request.
#[allow(clippy::too_many_arguments)]
async fn expect_ffmpeg_log_match(
    url: String,
    timeout_sec: u64,
    artifacts_dir: &Path,
    capture_tool_output: bool,
    file_name: &'static str,
    header: &'static str,
    needles: &'static [&'static str],
) -> Result<bool> {
    let dir = artifacts_dir.to_path_buf();
    let timeout_micros = timeout_sec.saturating_mul(1_000_000).to_string();
    tokio::task::spawn_blocking(move || {
        let mut log = ffmpeg_log(&dir, file_name, capture_tool_output, header, &url, None)?;
        let mut matched = false;
        run_ffmpeg(
            log.as_mut(),
            |cmd| {
                cmd.arg("-nostdin")
                    .arg("-rw_timeout")
                    .arg(&timeout_micros)
                    .arg("-rtsp_transport")
                    .arg("tcp")
                    .input(&url)
                    .duration("0.1")
                    .format("null")
                    .output("-");
            },
            |event| {
                if let FfmpegEvent::Log(_, msg) = event
                    && needles.iter().any(|n| msg.contains(n))
                {
                    matched = true;
                    return false;
                }
                true
            },
        )?;
        Ok::<_, anyhow::Error>(matched)
    })
    .await
    .context("spawn_blocking")?
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

    let invalid_ok = expect_ffmpeg_log_match(
        invalid_url,
        timeout_sec,
        artifacts_dir,
        capture_tool_output,
        "ffmpeg_error_invalid_creds.log",
        "ffmpeg error handling: invalid creds",
        &["401", "Unauthorized"],
    )
    .await?;
    let bogus_ok = expect_ffmpeg_log_match(
        bogus_url,
        timeout_sec,
        artifacts_dir,
        capture_tool_output,
        "ffmpeg_error_bogus_url.log",
        "ffmpeg error handling: bogus url",
        &["404", "Not Found"],
    )
    .await?;

    Ok((invalid_ok, bogus_ok))
}

#[cfg(test)]
mod tests {
    use super::{
        BoundedLogWriter, HarnessPacketLossResult, HarnessProcessOutcome, HarnessRtpLossMetric,
        RtpPcapRfc3640Stats, RtpPcapRfc6184Stats, RtpTsharkRow, RtspProtocolSequenceStats,
        analyze_aac_rfc3640_from_rows, analyze_h264_rfc6184_from_rows,
        append_harness_basic_connectivity_result, append_harness_bitrate_fps_result,
        append_harness_concurrent_clients_result, append_harness_error_handling_result,
        append_harness_long_duration_result, append_harness_packet_loss_result,
        append_harness_protocol_sequence_result, append_harness_sdp_validation_result,
        append_harness_startup_latency_result, arrival_deltas_ms, bitrate_within_tolerance,
        build_sdp_test_results, compute_pacing, compute_packet_loss_from_seqs,
        critical_proto_failed, delay_threshold_ms, empty_report, encoder_deltas_ms,
        fps_within_tolerance, gap_stats, group_rtp_rows_by_stream, harness_bitrate_pass,
        harness_fps_pass, harness_protocol_sequence_pass, packet_loss_within_tolerance,
        parse_tshark_hex_bytes, parse_tshark_rtp_row_line, parse_tshark_u32,
        pick_primary_audio_stream, pick_primary_video_stream, redact_url_credentials,
        rtsp_sequence_stats_from_field_rows, rtsp_url, rtsp_url_with_credentials,
        to_retina_initial_timestamp_policy, to_retina_transport, validate_aac_rtp_payload_rfc3640,
        validate_h264_length_prefixed_nals, validate_h264_rtp_payload_rfc6184,
    };
    use crate::config::{InitialTimestampPolicyArg, TransportArg};
    use crate::report::{StreamInfo, TestResult, TestRun, result_ok};
    use crate::util::MAX_TOOL_LOG_BYTES;
    use std::time::Duration;

    fn fail_reason(test: &TestResult) -> Option<&str> {
        match test {
            TestResult::Fail { reason, .. } => Some(reason),
            _ => None,
        }
    }

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
    fn test_harness_fps_pass_enforces_minimum_without_expected() {
        assert!(!harness_fps_pass(0.3, None, 10));
        assert!(harness_fps_pass(25.0, None, 10));
    }

    #[test]
    fn test_harness_bitrate_pass_enforces_minimum_without_expected() {
        assert!(!harness_bitrate_pass(0.0, None, 15));
        assert!(harness_bitrate_pass(800.0, None, 15));
    }

    #[test]
    fn test_harness_protocol_sequence_pass_allows_auth_challenge_401() {
        let stats = RtspProtocolSequenceStats {
            describe: 2,
            setup: 2,
            play: 1,
            teardown: 1,
            status_200: 5,
            status_4xx: 1,
            status_401: 1,
            status_5xx: 0,
        };
        assert!(harness_protocol_sequence_pass(&stats));
    }

    #[test]
    fn test_harness_protocol_sequence_pass_rejects_non_auth_4xx() {
        let stats = RtspProtocolSequenceStats {
            describe: 1,
            setup: 2,
            play: 1,
            teardown: 1,
            status_200: 4,
            status_4xx: 1,
            status_401: 0,
            status_5xx: 0,
        };
        assert!(!harness_protocol_sequence_pass(&stats));
    }

    #[test]
    fn test_rtsp_sequence_stats_from_field_rows() {
        // One row per RTSP packet: requests carry a method, responses a status.
        let rows = "DESCRIBE\t\nSETUP\t\nPLAY\t\nTEARDOWN\t\n\t200 OK\n\t200 OK\n\t401 Unauthorized\n\t404 Not Found\n\t503 Busy\n";
        let stats = rtsp_sequence_stats_from_field_rows(rows);
        assert_eq!(stats.describe, 1);
        assert_eq!(stats.setup, 1);
        assert_eq!(stats.play, 1);
        assert_eq!(stats.teardown, 1);
        assert_eq!(stats.status_200, 2);
        assert_eq!(stats.status_4xx, 2);
        assert_eq!(stats.status_401, 1);
        assert_eq!(stats.status_5xx, 1);
        // 401 is an auth challenge, not a real 4xx failure.
        assert!(!harness_protocol_sequence_pass(&stats));
    }

    #[test]
    fn test_rtsp_sequence_stats_from_field_rows_healthy_run_passes() {
        let rows = "DESCRIBE\t\n\t200 OK\nSETUP\t\n\t200 OK\nPLAY\t\n\t200 OK\n";
        let stats = rtsp_sequence_stats_from_field_rows(rows);
        assert!(harness_protocol_sequence_pass(&stats));
    }

    #[test]
    fn test_packet_loss_within_tolerance() {
        assert!(packet_loss_within_tolerance(0.5, 1.0));
        assert!(packet_loss_within_tolerance(1.0, 1.0));
        assert!(!packet_loss_within_tolerance(1.5, 1.0));
    }

    #[test]
    fn test_harness_basic_connectivity_outcomes() {
        let mut tests = Vec::new();
        append_harness_basic_connectivity_result(
            &mut tests,
            HarnessProcessOutcome::Success(true),
            Duration::from_secs(10),
        );
        assert!(result_ok(tests.last().unwrap()));

        append_harness_basic_connectivity_result(
            &mut tests,
            HarnessProcessOutcome::Error("ffmpeg failed".to_string()),
            Duration::from_secs(10),
        );
        assert_eq!(tests.last().unwrap().name(), "harness_basic_connectivity");
        assert_eq!(fail_reason(tests.last().unwrap()), Some("ffmpeg failed"));

        append_harness_basic_connectivity_result(
            &mut tests,
            HarnessProcessOutcome::Timeout,
            Duration::from_secs(7),
        );
        assert!(
            fail_reason(tests.last().unwrap())
                .unwrap()
                .contains("timed out after 7s")
        );
    }

    #[test]
    fn test_harness_startup_latency_outcomes() {
        let mut tests = Vec::new();
        append_harness_startup_latency_result(
            &mut tests,
            HarnessProcessOutcome::Success(Some(80)),
            100,
            Duration::from_secs(9),
        );
        assert!(result_ok(tests.last().unwrap()));

        append_harness_startup_latency_result(
            &mut tests,
            HarnessProcessOutcome::Error("decode failed".to_string()),
            100,
            Duration::from_secs(9),
        );
        assert_eq!(tests.last().unwrap().name(), "harness_startup_latency_ms");
        assert_eq!(fail_reason(tests.last().unwrap()), Some("decode failed"));

        append_harness_startup_latency_result(
            &mut tests,
            HarnessProcessOutcome::Timeout,
            100,
            Duration::from_secs(9),
        );
        assert!(
            fail_reason(tests.last().unwrap())
                .unwrap()
                .contains("timed out after 9s")
        );
    }

    #[test]
    fn test_harness_bitrate_fps_outcomes() {
        let mut tests = Vec::new();
        append_harness_bitrate_fps_result(
            &mut tests,
            HarnessProcessOutcome::Success((800.0, 25.0)),
            Some(820.0),
            10,
            Some(24.0),
            10,
            Duration::from_secs(20),
        );
        assert_eq!(tests.len(), 2);
        assert!(result_ok(&tests[0]));
        assert!(result_ok(&tests[1]));

        append_harness_bitrate_fps_result(
            &mut tests,
            HarnessProcessOutcome::Error("ffmpeg exited".to_string()),
            None,
            10,
            None,
            10,
            Duration::from_secs(20),
        );
        assert_eq!(fail_reason(tests.last().unwrap()), Some("ffmpeg exited"));

        append_harness_bitrate_fps_result(
            &mut tests,
            HarnessProcessOutcome::Timeout,
            None,
            10,
            None,
            10,
            Duration::from_secs(20),
        );
        assert!(
            fail_reason(tests.last().unwrap())
                .unwrap()
                .contains("timed out after 20s")
        );
    }

    #[test]
    fn test_harness_sdp_validation_outcomes() {
        let mut tests = Vec::new();
        let flags = append_harness_sdp_validation_result(
            &mut tests,
            HarnessProcessOutcome::Success((1, 1, true, true)),
            Duration::from_secs(8),
        );
        assert_eq!(flags, (true, true));
        assert_eq!(tests.len(), 4);

        let flags = append_harness_sdp_validation_result(
            &mut tests,
            HarnessProcessOutcome::Error("ffprobe failed".to_string()),
            Duration::from_secs(8),
        );
        assert_eq!(flags, (false, false));
        assert_eq!(fail_reason(tests.last().unwrap()), Some("ffprobe failed"));

        let flags = append_harness_sdp_validation_result(
            &mut tests,
            HarnessProcessOutcome::Timeout,
            Duration::from_secs(8),
        );
        assert_eq!(flags, (false, false));
        assert!(
            fail_reason(tests.last().unwrap())
                .unwrap()
                .contains("timed out after 8s")
        );
    }

    #[test]
    fn test_harness_protocol_sequence_outcomes() {
        let mut tests = Vec::new();
        append_harness_protocol_sequence_result(
            &mut tests,
            HarnessProcessOutcome::Success(RtspProtocolSequenceStats {
                describe: 1,
                setup: 1,
                play: 1,
                teardown: 1,
                status_200: 3,
                status_4xx: 0,
                status_401: 0,
                status_5xx: 0,
            }),
            Duration::from_secs(30),
        );
        assert!(result_ok(tests.last().unwrap()));

        append_harness_protocol_sequence_result(
            &mut tests,
            HarnessProcessOutcome::Error("tshark parse failed".to_string()),
            Duration::from_secs(30),
        );
        assert_eq!(
            fail_reason(tests.last().unwrap()),
            Some("tshark parse failed")
        );

        append_harness_protocol_sequence_result(
            &mut tests,
            HarnessProcessOutcome::Timeout,
            Duration::from_secs(30),
        );
        assert!(
            fail_reason(tests.last().unwrap())
                .unwrap()
                .contains("timed out after 30s")
        );
    }

    #[test]
    fn test_harness_packet_loss_outcomes() {
        let mut tests = Vec::new();
        append_harness_packet_loss_result(
            &mut tests,
            HarnessProcessOutcome::Success(HarnessPacketLossResult {
                video: Some(HarnessRtpLossMetric {
                    rtp_packets: 100,
                    packet_loss: 0,
                    loss_percent: 0.0,
                    payload_type: 96,
                    ssrc: Some(1),
                }),
                audio: Some(HarnessRtpLossMetric {
                    rtp_packets: 80,
                    packet_loss: 1,
                    loss_percent: 1.25,
                    payload_type: 97,
                    ssrc: Some(2),
                }),
                h264_rfc6184: Some(Ok(RtpPcapRfc6184Stats {
                    payload_type: 96,
                    packets_analyzed: 100,
                    invalid_packets: 0,
                    marker_violations: 0,
                    fu_a_invalid: 0,
                    stap_a_invalid: 0,
                })),
                aac_rfc3640: Some(Ok(RtpPcapRfc3640Stats {
                    payload_type: 97,
                    packets_analyzed: 80,
                    invalid_packets: 0,
                    au_header_invalid: 0,
                    au_size_invalid: 0,
                    timestamp_anomalies: 0,
                })),
                pacing: None,
            }),
            2.0,
            5.0,
            true,
            true,
            Duration::from_secs(25),
        );
        assert!(
            tests
                .iter()
                .any(|t| t.name() == "harness_packet_loss_percent")
        );
        assert!(
            tests
                .iter()
                .any(|t| t.name() == "harness_pcap_rfc6184_h264")
        );
        assert!(tests.iter().any(|t| t.name() == "harness_pcap_rfc3640_aac"));

        append_harness_packet_loss_result(
            &mut tests,
            HarnessProcessOutcome::Error("pcap missing".to_string()),
            2.0,
            5.0,
            true,
            true,
            Duration::from_secs(25),
        );
        assert_eq!(fail_reason(tests.last().unwrap()), Some("pcap missing"));

        append_harness_packet_loss_result(
            &mut tests,
            HarnessProcessOutcome::Timeout,
            2.0,
            5.0,
            true,
            true,
            Duration::from_secs(25),
        );
        assert!(
            fail_reason(tests.last().unwrap())
                .unwrap()
                .contains("timed out after 25s")
        );
    }

    #[test]
    fn test_harness_concurrent_clients_outcomes() {
        let mut tests = Vec::new();
        append_harness_concurrent_clients_result(
            &mut tests,
            HarnessProcessOutcome::Success(0),
            4,
            Duration::from_secs(15),
        );
        assert!(result_ok(tests.last().unwrap()));

        append_harness_concurrent_clients_result(
            &mut tests,
            HarnessProcessOutcome::Error("worker panic".to_string()),
            4,
            Duration::from_secs(15),
        );
        assert_eq!(fail_reason(tests.last().unwrap()), Some("worker panic"));

        append_harness_concurrent_clients_result(
            &mut tests,
            HarnessProcessOutcome::Timeout,
            4,
            Duration::from_secs(15),
        );
        assert!(
            fail_reason(tests.last().unwrap())
                .unwrap()
                .contains("timed out after 15s")
        );
    }

    #[test]
    fn test_harness_long_duration_outcomes() {
        let mut tests = Vec::new();
        append_harness_long_duration_result(
            &mut tests,
            HarnessProcessOutcome::Success(5),
            Duration::from_secs(60),
        );
        assert!(result_ok(tests.last().unwrap()));

        append_harness_long_duration_result(
            &mut tests,
            HarnessProcessOutcome::Error("throughput probe failed".to_string()),
            Duration::from_secs(60),
        );
        assert_eq!(
            fail_reason(tests.last().unwrap()),
            Some("throughput probe failed")
        );

        append_harness_long_duration_result(
            &mut tests,
            HarnessProcessOutcome::Timeout,
            Duration::from_secs(60),
        );
        assert!(
            fail_reason(tests.last().unwrap())
                .unwrap()
                .contains("timed out after 60s")
        );
    }

    #[test]
    fn test_harness_error_handling_outcomes() {
        let mut tests = Vec::new();
        append_harness_error_handling_result(
            &mut tests,
            HarnessProcessOutcome::Success((true, true)),
            Duration::from_secs(11),
        );
        assert_eq!(tests.len(), 2);
        assert!(result_ok(&tests[0]));
        assert!(result_ok(&tests[1]));

        append_harness_error_handling_result(
            &mut tests,
            HarnessProcessOutcome::Error("ffmpeg not found".to_string()),
            Duration::from_secs(11),
        );
        assert_eq!(fail_reason(tests.last().unwrap()), Some("ffmpeg not found"));

        append_harness_error_handling_result(
            &mut tests,
            HarnessProcessOutcome::Timeout,
            Duration::from_secs(11),
        );
        assert!(
            fail_reason(tests.last().unwrap())
                .unwrap()
                .contains("timed out after 11s")
        );
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
