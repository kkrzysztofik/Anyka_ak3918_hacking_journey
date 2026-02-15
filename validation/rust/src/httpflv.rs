//! HTTP-FLV validation and harness scenarios.
//!
//! Validates FLV container format correctness by parsing the binary stream (header + tags)
//! and performs harness analysis (bitrate, FPS) using ffmpeg.

use std::time::Duration;

use anyhow::{Context, Result};
use ffmpeg_sidecar::command::FfmpegCommand;
use ffmpeg_sidecar::event::FfmpegEvent;
use futures_util::StreamExt;
use tokio::time::timeout;
use tracing::{debug, info};
use url::Url;

use crate::config::EffectiveConfig;
use crate::report::TestResult;

fn httpflv_base_url(effective: &EffectiveConfig) -> String {
    format!(
        "http://{}:{}{}",
        effective.rtsp_host, effective.httpflv_port, effective.httpflv_path
    )
}

fn url_with_credentials(
    url: &str,
    username: Option<&str>,
    password: Option<&str>,
) -> Result<String> {
    match (username, password) {
        (None, None) => Ok(url.to_string()),
        (Some(username), Some(password)) => {
            let mut parsed =
                Url::parse(url).with_context(|| format!("invalid HTTP-FLV URL: {}", url))?;
            parsed
                .set_username(username)
                .map_err(|_| anyhow::anyhow!("failed to set HTTP-FLV URL username"))?;
            parsed
                .set_password(Some(password))
                .map_err(|_| anyhow::anyhow!("failed to set HTTP-FLV URL password"))?;
            Ok(parsed.to_string())
        }
        _ => Ok(url.to_string()),
    }
}

fn httpflv_url_with_credentials(effective: &EffectiveConfig) -> Result<String> {
    let url = httpflv_base_url(effective);
    url_with_credentials(
        &url,
        effective.stream_username.as_deref(),
        effective.stream_password.as_deref(),
    )
}

fn apply_basic_auth(
    request: reqwest::RequestBuilder,
    username: Option<&str>,
    password: Option<&str>,
) -> reqwest::RequestBuilder {
    match (username, password) {
        (Some(username), Some(password)) => request.basic_auth(username, Some(password)),
        _ => request,
    }
}

fn apply_stream_basic_auth(
    request: reqwest::RequestBuilder,
    effective: &EffectiveConfig,
) -> reqwest::RequestBuilder {
    apply_basic_auth(
        request,
        effective.stream_username.as_deref(),
        effective.stream_password.as_deref(),
    )
}

fn parse_ffmpeg_kbytes_token(raw: &str) -> Option<f32> {
    let trimmed = raw.trim().trim_end_matches(',');
    if let Some(value) = trimmed.strip_suffix("kB") {
        return value.trim().parse::<f32>().ok();
    }
    if let Some(value) = trimmed.strip_suffix("MB") {
        return value.trim().parse::<f32>().ok().map(|v| v * 1000.0);
    }
    if let Some(value) = trimmed.strip_suffix('B') {
        return value.trim().parse::<f32>().ok().map(|v| v / 1000.0);
    }
    None
}

fn parse_ffmpeg_summary_bitrate_kbps(log_line: &str, duration_sec: u64) -> Option<f32> {
    if duration_sec == 0 {
        return None;
    }
    let mut total_kbytes = 0.0_f32;
    let mut found = false;
    for field in log_line.split_whitespace() {
        let maybe_value = field
            .strip_prefix("video:")
            .or_else(|| field.strip_prefix("audio:"));
        if let Some(value) = maybe_value
            && let Some(kbytes) = parse_ffmpeg_kbytes_token(value)
        {
            total_kbytes += kbytes;
            found = true;
        }
    }
    if !found || total_kbytes <= 0.0 {
        return None;
    }
    Some((total_kbytes * 8.0) / duration_sec as f32)
}

fn validate_video_tag_payload_header(tag_body: &[u8]) -> Result<(), String> {
    if tag_body.len() < 5 {
        return Err(format!(
            "video tag body too short for AVC header: {} bytes",
            tag_body.len()
        ));
    }

    let codec_id = tag_body[0] & 0x0F;
    if codec_id != 7 {
        return Err(format!(
            "video tag codec id is {}, expected 7 (AVC)",
            codec_id
        ));
    }

    let avc_packet_type = tag_body[1];
    if !matches!(avc_packet_type, 0 | 1) {
        return Err(format!(
            "invalid AVC packet type {}, expected 0 or 1",
            avc_packet_type
        ));
    }

    Ok(())
}

fn validate_audio_tag_payload_header(tag_body: &[u8]) -> Result<(), String> {
    if tag_body.len() < 2 {
        return Err(format!(
            "audio tag body too short for AAC header: {} bytes",
            tag_body.len()
        ));
    }

    let sound_format = tag_body[0] >> 4;
    if sound_format != 10 {
        return Err(format!(
            "audio tag sound format is {}, expected 10 (AAC)",
            sound_format
        ));
    }

    let aac_packet_type = tag_body[1];
    if !matches!(aac_packet_type, 0 | 1) {
        return Err(format!(
            "invalid AAC packet type {}, expected 0 or 1",
            aac_packet_type
        ));
    }

    Ok(())
}

#[derive(Debug, Default)]
struct FlvTimestampTracker {
    audio_last_ts: Option<u32>,
    video_last_ts: Option<u32>,
    metadata_last_ts: Option<u32>,
}

impl FlvTimestampTracker {
    fn last_timestamp_for_tag_type_mut(&mut self, tag_type: u8) -> Option<&mut Option<u32>> {
        match tag_type {
            8 => Some(&mut self.audio_last_ts),
            9 => Some(&mut self.video_last_ts),
            18 => Some(&mut self.metadata_last_ts),
            _ => None,
        }
    }

    fn observe(&mut self, tag_type: u8, timestamp: u32) -> Option<u32> {
        let last_ts = self.last_timestamp_for_tag_type_mut(tag_type)?;
        let previous = *last_ts;
        *last_ts = Some(timestamp);

        match previous {
            Some(prev) if timestamp != 0 && timestamp < prev => Some(prev),
            _ => None,
        }
    }
}

/// Run HTTP-FLV protocol validation. Returns test results.
///
/// This performs low-level validation of the FLV container format by reading the stream prefix.
pub async fn run_httpflv_validation(effective: &EffectiveConfig) -> Result<Vec<TestResult>> {
    let url = httpflv_base_url(effective);
    info!(url = %url, uses_credentials = effective.stream_username.is_some(), "running HTTP-FLV protocol validation");

    let mut tests = Vec::new();
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(effective.httpflv_timeout_sec))
        .build()?;

    let start = std::time::Instant::now();
    let request = apply_stream_basic_auth(client.get(&url), effective);
    let res = match request.send().await {
        Ok(r) => {
            tests.push(TestResult::pass_proto("httpflv_connect_ok", "httpflv"));
            r
        }
        Err(e) => {
            tests.push(TestResult::fail_proto(
                "httpflv_connect_ok",
                e.to_string(),
                "httpflv",
            ));
            return Ok(tests);
        }
    };

    let latency_ms = start.elapsed().as_millis() as u64;
    tests.push(TestResult::metric_proto(
        "httpflv_connect_latency_ms",
        serde_json::json!(latency_ms),
        true,
        "httpflv",
    ));

    if !res.status().is_success() {
        tests.push(TestResult::fail_proto(
            "httpflv_status_200_ok",
            format!("HTTP status {}", res.status()),
            "httpflv",
        ));
        return Ok(tests);
    }
    tests.push(TestResult::pass_proto("httpflv_status_200_ok", "httpflv"));

    let mut body = res.bytes_stream();
    let mut buffer = Vec::with_capacity(4096);

    // 1. Validate FLV Header (9 bytes)
    if let Some(chunk) = body.next().await {
        buffer.extend_from_slice(&chunk?);
    }

    if buffer.len() < 9 {
        tests.push(TestResult::fail_proto(
            "httpflv_header_valid",
            "response too short for FLV header",
            "httpflv",
        ));
        return Ok(tests);
    }

    let header = &buffer[0..9];
    if &header[0..3] != b"FLV" {
        tests.push(TestResult::fail_proto(
            "httpflv_header_valid",
            "missing 'FLV' signature",
            "httpflv",
        ));
    } else if header[3] != 1 {
        tests.push(TestResult::fail_proto(
            "httpflv_header_valid",
            format!("invalid FLV version: {}", header[3]),
            "httpflv",
        ));
    } else {
        let flags = header[4];
        let _has_video = (flags & 0x01) != 0;
        let _data_offset_unused = u32::from_be_bytes([0, header[5], header[6], header[7]]); // Actually [5..9] is 4 bytes
        let data_offset = u32::from_be_bytes([header[5], header[6], header[7], header[8]]);

        if data_offset != 9 {
            tests.push(TestResult::fail_proto(
                "httpflv_header_valid",
                format!("invalid data offset: {}", data_offset),
                "httpflv",
            ));
        } else if !_has_video {
            tests.push(TestResult::fail_proto(
                "httpflv_header_valid",
                "video bit not set in FLV flags",
                "httpflv",
            ));
        } else {
            tests.push(TestResult::pass_proto("httpflv_header_valid", "httpflv"));
        }
    }

    // 2. Validate First PreviousTagSize (4 bytes, must be 0)
    #[allow(clippy::collapsible_if)]
    if buffer.len() < 13 {
        if let Some(chunk) = body.next().await {
            buffer.extend_from_slice(&chunk?);
        }
    }
    if buffer.len() >= 13 {
        let prev_tag_size_0 = u32::from_be_bytes([buffer[9], buffer[10], buffer[11], buffer[12]]);
        if prev_tag_size_0 != 0 {
            tests.push(TestResult::fail_proto(
                "httpflv_first_prev_tag_size_zero",
                format!("expected 0, got {}", prev_tag_size_0),
                "httpflv",
            ));
        } else {
            tests.push(TestResult::pass_proto(
                "httpflv_first_prev_tag_size_zero",
                "httpflv",
            ));
        }
    }

    // 3. Validate Tags (at least a few)
    let mut pos = 13;
    let mut tags_seen = 0;
    let mut video_tags = 0;
    let mut audio_tags = 0;
    let mut timestamp_tracker = FlvTimestampTracker::default();
    let mut video_payload_header_validated = false;
    let mut audio_payload_header_validated = false;

    let validation_start = std::time::Instant::now();
    // Analyze up to 50 tags or 2 seconds of the stream prefix
    while tags_seen < 50 && validation_start.elapsed() < Duration::from_secs(2) {
        if buffer.len() < pos + 11 {
            if let Some(chunk) = body.next().await {
                buffer.extend_from_slice(&chunk?);
            } else {
                break;
            }
            continue;
        }

        let tag_type = buffer[pos];
        let data_size = u32::from_be_bytes([0, buffer[pos + 1], buffer[pos + 2], buffer[pos + 3]]);
        let ts_lower = u32::from_be_bytes([0, buffer[pos + 4], buffer[pos + 5], buffer[pos + 6]]);
        let ts_upper = buffer[pos + 7] as u32;
        let timestamp = (ts_upper << 24) | ts_lower;

        if !matches!(tag_type, 8 | 9 | 18) {
            tests.push(TestResult::fail_proto(
                "httpflv_tags_valid",
                format!("invalid tag type {} at pos {}", tag_type, pos),
                "httpflv",
            ));
            break;
        }

        if data_size > 16 * 1024 * 1024 {
            tests.push(TestResult::fail_proto(
                "httpflv_tags_valid",
                format!("tag data size too large: {} at pos {}", data_size, pos),
                "httpflv",
            ));
            break;
        }

        let next_tag_pos = pos + 11 + data_size as usize;
        if buffer.len() < next_tag_pos + 4 {
            if let Some(chunk) = body.next().await {
                buffer.extend_from_slice(&chunk?);
            } else {
                break;
            }
            continue;
        }

        let tag_body = &buffer[pos + 11..next_tag_pos];
        if tag_type == 9 && !video_payload_header_validated {
            match validate_video_tag_payload_header(tag_body) {
                Ok(()) => {
                    video_payload_header_validated = true;
                }
                Err(reason) => {
                    tests.push(TestResult::fail_proto(
                        "httpflv_video_payload_header_valid",
                        reason,
                        "httpflv",
                    ));
                    break;
                }
            }
        }
        if tag_type == 8 && !audio_payload_header_validated {
            match validate_audio_tag_payload_header(tag_body) {
                Ok(()) => {
                    audio_payload_header_validated = true;
                }
                Err(reason) => {
                    tests.push(TestResult::fail_proto(
                        "httpflv_audio_payload_header_valid",
                        reason,
                        "httpflv",
                    ));
                    break;
                }
            }
        }

        let footer_prev_tag_size = u32::from_be_bytes([
            buffer[next_tag_pos],
            buffer[next_tag_pos + 1],
            buffer[next_tag_pos + 2],
            buffer[next_tag_pos + 3],
        ]);
        if footer_prev_tag_size != (11 + data_size) {
            tests.push(TestResult::fail_proto(
                "httpflv_tags_valid",
                format!(
                    "incorrect PreviousTagSize: expected {}, got {} at pos {}",
                    11 + data_size,
                    footer_prev_tag_size,
                    next_tag_pos
                ),
                "httpflv",
            ));
            break;
        }

        match tag_type {
            8 => audio_tags += 1,
            9 => video_tags += 1,
            _ => {}
        }
        if let Some(last_ts) = timestamp_tracker.observe(tag_type, timestamp) {
            debug!(
                tag_type,
                timestamp, last_ts, "non-monotonic timestamp detected within FLV tag type"
            );
        }

        pos = next_tag_pos + 4;
        tags_seen += 1;

        if buffer.len() > pos + 512 * 1024 {
            buffer.drain(0..pos);
            pos = 0;
        }
    }

    if tags_seen > 0 {
        tests.push(TestResult::pass_proto("httpflv_tags_valid", "httpflv"));
        tests.push(TestResult::metric_proto(
            "httpflv_tags_analyzed",
            serde_json::json!(tags_seen),
            true,
            "httpflv",
        ));
        tests.push(TestResult::metric_proto(
            "httpflv_video_tags",
            serde_json::json!(video_tags),
            video_tags > 0,
            "httpflv",
        ));
        tests.push(TestResult::metric_proto(
            "httpflv_audio_tags",
            serde_json::json!(audio_tags),
            true,
            "httpflv",
        ));
        if video_tags > 0 {
            if video_payload_header_validated {
                tests.push(TestResult::pass_proto(
                    "httpflv_video_payload_header_valid",
                    "httpflv",
                ));
            } else {
                tests.push(TestResult::fail_proto(
                    "httpflv_video_payload_header_valid",
                    "video tags found but payload headers not validated",
                    "httpflv",
                ));
            }
        }
        if audio_tags > 0 {
            if audio_payload_header_validated {
                tests.push(TestResult::pass_proto(
                    "httpflv_audio_payload_header_valid",
                    "httpflv",
                ));
            } else {
                tests.push(TestResult::fail_proto(
                    "httpflv_audio_payload_header_valid",
                    "audio tags found but payload headers not validated",
                    "httpflv",
                ));
            }
        }
    } else {
        tests.push(TestResult::fail_proto(
            "httpflv_tags_valid",
            "no FLV tags found in response",
            "httpflv",
        ));
    }

    Ok(tests)
}

/// Run HTTP-FLV harness scenarios.
///
/// Uses ffmpeg to analyze the stream for bitrate, FPS, and consistency over a period of time.
pub async fn run_httpflv_harness(
    _args: &crate::config::Args,
    effective: &EffectiveConfig,
    tests: &mut Vec<TestResult>,
) -> Result<()> {
    let url = httpflv_base_url(effective);
    let harness_url = httpflv_url_with_credentials(effective)?;
    info!(url = %url, uses_credentials = effective.stream_username.is_some(), "running HTTP-FLV harness analysis (ffmpeg)");

    let duration_sec = effective.short_duration_sec.max(10);
    // Add extra time for startup/buffering
    let step_timeout = Duration::from_secs(duration_sec.saturating_add(30));

    match timeout(
        step_timeout,
        tokio::task::spawn_blocking(move || {
            let mut cmd = FfmpegCommand::new();
            cmd.hide_banner()
                .input(&harness_url)
                .duration(duration_sec.to_string())
                .format("null")
                .output("-");

            let mut child = cmd.spawn().context("spawn ffmpeg for httpflv harness")?;
            let iter = child.iter().context("ffmpeg iter")?;

            let mut frames = 0;
            let mut bitrates = Vec::new();
            let mut summary_bitrate_kbps = None;

            for event in iter {
                match event {
                    FfmpegEvent::Progress(p) => {
                        frames = p.frame;
                        if p.bitrate_kbps > 0.0 {
                            bitrates.push(p.bitrate_kbps);
                        }
                    }
                    FfmpegEvent::Log(_, msg) => {
                        if let Some(estimated) =
                            parse_ffmpeg_summary_bitrate_kbps(&msg, duration_sec)
                        {
                            summary_bitrate_kbps = Some(estimated);
                        }
                    }
                    _ => {}
                }
            }
            Ok::<_, anyhow::Error>((frames, bitrates, summary_bitrate_kbps))
        }),
    )
    .await
    {
        Ok(Ok(Ok((frames, bitrates, summary_bitrate_kbps)))) => {
            let avg_fps = if duration_sec > 0 {
                frames as f64 / duration_sec as f64
            } else {
                0.0
            };
            let avg_bitrate = if !bitrates.is_empty() {
                bitrates.iter().sum::<f32>() / bitrates.len() as f32
            } else {
                summary_bitrate_kbps.unwrap_or(0.0)
            };

            tests.push(TestResult::metric_proto(
                "httpflv_harness_avg_fps",
                serde_json::json!(avg_fps),
                avg_fps > 5.0,
                "httpflv",
            ));
            tests.push(TestResult::metric_proto(
                "httpflv_harness_avg_bitrate_kbps",
                serde_json::json!(avg_bitrate),
                avg_bitrate > 0.0,
                "httpflv",
            ));
        }
        Ok(Ok(Err(e))) => {
            tests.push(TestResult::fail_proto(
                "httpflv_harness_run",
                e.to_string(),
                "httpflv",
            ));
        }
        Ok(Err(e)) => {
            tests.push(TestResult::fail_proto(
                "httpflv_harness_run",
                e.to_string(),
                "httpflv",
            ));
        }
        Err(_) => {
            tests.push(TestResult::fail_proto(
                "httpflv_harness_run",
                "ffmpeg timeout during stream analysis",
                "httpflv",
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        FlvTimestampTracker, apply_basic_auth, parse_ffmpeg_kbytes_token,
        parse_ffmpeg_summary_bitrate_kbps, url_with_credentials, validate_audio_tag_payload_header,
        validate_video_tag_payload_header,
    };

    #[test]
    fn test_parse_flv_header() {
        let buf = vec![
            0x46, 0x4C, 0x56, // FLV
            0x01, // Version 1
            0x05, // Flags: Audio + Video
            0x00, 0x00, 0x00, 0x09, // DataOffset 9
        ];
        assert_eq!(&buf[0..3], b"FLV");
        assert_eq!(buf[3], 1);
        assert_eq!(buf[4], 5);
        let offset = u32::from_be_bytes([buf[5], buf[6], buf[7], buf[8]]);
        assert_eq!(offset, 9);
    }

    #[test]
    fn test_validate_video_tag_payload_header_success() {
        let tag_body = [0x17, 0x01, 0x00, 0x00, 0x00, 0x00];
        let result = validate_video_tag_payload_header(&tag_body);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_video_tag_payload_header_invalid_codec_id() {
        let tag_body = [0x12, 0x01, 0x00, 0x00, 0x00];
        let result = validate_video_tag_payload_header(&tag_body);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_video_tag_payload_header_invalid_avc_packet_type() {
        let tag_body = [0x17, 0x02, 0x00, 0x00, 0x00];
        let result = validate_video_tag_payload_header(&tag_body);
        assert!(result.is_err());
    }

    #[test]
    fn test_url_with_credentials_roundtrip() {
        let url = url_with_credentials(
            "http://127.0.0.1:8080/live/stream1.flv",
            Some("user@name"),
            Some("p@ss:wo/rd"),
        )
        .unwrap();
        let parsed = url::Url::parse(&url).unwrap();
        assert!(parsed.as_str().contains("user%40name"));
        assert!(parsed.as_str().contains("p%40ss%3Awo%2Frd"));
    }

    #[test]
    fn test_apply_basic_auth_sets_authorization_header() {
        let client = reqwest::Client::new();
        let request = apply_basic_auth(
            client.get("http://127.0.0.1:8080/live/stream1.flv"),
            Some("user"),
            Some("pass"),
        )
        .build()
        .unwrap();
        let header = request
            .headers()
            .get(reqwest::header::AUTHORIZATION)
            .unwrap()
            .to_str()
            .unwrap();
        assert_eq!(header, "Basic dXNlcjpwYXNz");
    }

    #[test]
    fn test_apply_basic_auth_ignores_partial_credentials() {
        let client = reqwest::Client::new();
        let request = apply_basic_auth(
            client.get("http://127.0.0.1:8080/live/stream1.flv"),
            Some("user"),
            None,
        )
        .build()
        .unwrap();
        assert!(
            request
                .headers()
                .get(reqwest::header::AUTHORIZATION)
                .is_none()
        );
    }

    #[test]
    fn test_validate_audio_tag_payload_header_success() {
        let tag_body = [0xAF, 0x01, 0x11, 0x90];
        let result = validate_audio_tag_payload_header(&tag_body);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_audio_tag_payload_header_invalid_sound_format() {
        let tag_body = [0x2F, 0x01, 0x11, 0x90];
        let result = validate_audio_tag_payload_header(&tag_body);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_audio_tag_payload_header_invalid_aac_packet_type() {
        let tag_body = [0xAF, 0x05, 0x11, 0x90];
        let result = validate_audio_tag_payload_header(&tag_body);
        assert!(result.is_err());
    }

    #[test]
    fn test_flv_timestamp_tracker_tracks_per_tag_type() {
        let mut tracker = FlvTimestampTracker::default();
        assert!(tracker.observe(9, 23_040).is_none());
        assert!(tracker.observe(8, 40).is_none());
        assert_eq!(tracker.observe(9, 120), Some(23_040));
    }

    #[test]
    fn test_flv_timestamp_tracker_allows_zero_timestamp_regression() {
        let mut tracker = FlvTimestampTracker::default();
        assert!(tracker.observe(9, 1_000).is_none());
        assert!(tracker.observe(9, 0).is_none());
    }

    #[test]
    fn test_parse_ffmpeg_kbytes_token() {
        assert_eq!(parse_ffmpeg_kbytes_token("52kB"), Some(52.0));
        assert_eq!(parse_ffmpeg_kbytes_token("1MB"), Some(1000.0));
        assert_eq!(parse_ffmpeg_kbytes_token("500B"), Some(0.5));
        assert!(parse_ffmpeg_kbytes_token("n/a").is_none());
    }

    #[test]
    fn test_parse_ffmpeg_summary_bitrate_kbps() {
        let line = "[info] video:52kB audio:816kB subtitle:0kB";
        let bitrate = parse_ffmpeg_summary_bitrate_kbps(line, 4).unwrap();
        assert!((bitrate - 1736.0).abs() < 0.01);
    }

    #[test]
    fn test_parse_ffmpeg_summary_bitrate_kbps_missing_fields() {
        let line = "[info] Output #0, null, to 'pipe:'";
        assert!(parse_ffmpeg_summary_bitrate_kbps(line, 10).is_none());
    }

    #[test]
    fn test_parse_ffmpeg_summary_bitrate_kbps_duration_zero() {
        let line = "[info] video:52kB audio:816kB subtitle:0kB";
        assert!(parse_ffmpeg_summary_bitrate_kbps(line, 0).is_none());
    }

    #[test]
    fn test_validate_video_tag_payload_header_short_payload() {
        let tag_body = [0x17, 0x01, 0x00, 0x00];
        let result = validate_video_tag_payload_header(&tag_body);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_audio_tag_payload_header_short_payload() {
        let tag_body = [0xAF];
        let result = validate_audio_tag_payload_header(&tag_body);
        assert!(result.is_err());
    }
}
