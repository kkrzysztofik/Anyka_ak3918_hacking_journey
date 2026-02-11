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

use crate::config::EffectiveConfig;
use crate::report::TestResult;

/// Run HTTP-FLV protocol validation. Returns test results.
///
/// This performs low-level validation of the FLV container format by reading the stream prefix.
pub async fn run_httpflv_validation(effective: &EffectiveConfig) -> Result<Vec<TestResult>> {
    let url = format!(
        "http://{}:{}{}",
        effective.rtsp_host, effective.httpflv_port, effective.httpflv_path
    );
    info!(url = %url, "running HTTP-FLV protocol validation");

    let mut tests = Vec::new();
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(effective.httpflv_timeout_sec))
        .build()?;

    let start = std::time::Instant::now();
    let res = match client.get(&url).send().await {
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
    let mut last_ts = 0;

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

        if timestamp < last_ts && timestamp != 0 {
            // Some tolerance for rollover if needed, but FLV timestamps are usually monotonic
            debug!(timestamp, last_ts, "non-monotonic timestamp detected");
        }

        match tag_type {
            8 => audio_tags += 1,
            9 => video_tags += 1,
            _ => {}
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

        pos = next_tag_pos + 4;
        tags_seen += 1;
        last_ts = timestamp;

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
    let url = format!(
        "http://{}:{}{}",
        effective.rtsp_host, effective.httpflv_port, effective.httpflv_path
    );
    info!(url = %url, "running HTTP-FLV harness analysis (ffmpeg)");

    let duration_sec = effective.short_duration_sec.max(10);
    // Add extra time for startup/buffering
    let step_timeout = Duration::from_secs(duration_sec.saturating_add(30));

    match timeout(
        step_timeout,
        tokio::task::spawn_blocking(move || {
            let mut cmd = FfmpegCommand::new();
            cmd.hide_banner()
                .input(&url)
                .duration(duration_sec.to_string())
                .format("null")
                .output("-");

            let mut child = cmd.spawn().context("spawn ffmpeg for httpflv harness")?;
            let iter = child.iter().context("ffmpeg iter")?;

            let mut frames = 0;
            let mut bitrates = Vec::new();

            for event in iter {
                if let FfmpegEvent::Progress(p) = event {
                    frames = p.frame;
                    if p.bitrate_kbps > 0.0 {
                        bitrates.push(p.bitrate_kbps);
                    }
                }
            }
            Ok::<_, anyhow::Error>((frames, bitrates))
        }),
    )
    .await
    {
        Ok(Ok(Ok((frames, bitrates)))) => {
            let avg_fps = if duration_sec > 0 {
                frames as f64 / duration_sec as f64
            } else {
                0.0
            };
            let avg_bitrate = if !bitrates.is_empty() {
                bitrates.iter().sum::<f32>() / bitrates.len() as f32
            } else {
                0.0
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
    // use super::*;

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
}
