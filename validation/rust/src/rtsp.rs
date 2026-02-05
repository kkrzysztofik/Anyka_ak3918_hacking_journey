//! RTSP protocol validation and harness (Retina, ffmpeg, tshark).

use anyhow::{Context, Result, bail};
use chrono::Utc;
use ffmpeg_sidecar::command::FfmpegCommand;
use ffmpeg_sidecar::event::{FfmpegEvent, FfmpegProgress};
use futures_util::StreamExt;
use retina::client::{Credentials, PlayOptions, Session, SessionOptions, SetupOptions, Transport};
use retina::codec::{CodecItem, ParametersRef};
use rtshark::RTSharkBuilder;
use serde::Deserialize;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::time::{Instant, timeout};
use tracing::{debug, info, trace, warn};
use url::Url;

use crate::config::{Args, EffectiveConfig, TransportArg};
use crate::report::{StreamInfo, TestResult, TestRun, ValidationReport};
use crate::util::{MAX_TOOL_LOG_BYTES, tail_lossy, write_bytes_tail};

pub(crate) fn rtsp_url(host: &str, port: u16, stream: &str) -> String {
    format!("rtsp://{}:{}{}", host, port, stream)
}

pub(crate) fn to_retina_transport(arg: TransportArg) -> Transport {
    match arg {
        TransportArg::Tcp => Transport::Tcp(Default::default()),
        TransportArg::Udp => Transport::Udp(Default::default()),
    }
}

fn stream_info_from_retina(s: &retina::client::Stream) -> StreamInfo {
    StreamInfo {
        media: s.media().to_string(),
        encoding_name: s.encoding_name().to_string(),
        control_present: s.control().is_some(),
    }
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
    };

    let mut tests: Vec<TestResult> = Vec::new();

    let url = Url::parse(&url_str).with_context(|| format!("invalid RTSP URL: {}", url_str))?;

    let creds = match (&args.username, &args.password) {
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

    let play_opts = PlayOptions::default();
    debug!("PLAY request");
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

    let probe_duration = Duration::from_secs(effective.short_duration_sec);
    let probe_res: Result<()> = timeout(probe_duration, async {
        while let Some(item) = demuxed.next().await {
            let item = item.context("demuxed stream error")?;
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
    let timeout_sec = effective.rtsp_timeout_sec;
    let step_cap_short = Duration::from_secs(timeout_sec.saturating_add(15));
    let step_cap_long = Duration::from_secs(effective.short_duration_sec.saturating_add(30));
    let artifacts_dir = effective.artifacts_dir.clone();
    let capture_tool_output = effective.capture_tool_output;
    info!(url = %url, "running harness scenarios");

    debug!("harness: basic connectivity");
    match timeout(
        step_cap_short,
        harness_basic_connectivity(
            &url,
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
        Err(_) => tests.push(TestResult::fail(
            "harness_basic_connectivity",
            format!("harness step timed out after {}s", step_cap_short.as_secs()),
        )),
    }

    debug!(url = %url, target_ms = effective.video_startup_latency_ms, "harness: startup latency");
    match timeout(
        step_cap_short,
        harness_startup_latency(
            &url,
            timeout_sec,
            effective.video_startup_latency_ms,
            &artifacts_dir,
            capture_tool_output,
            &effective.ffmpeg_log_level,
        ),
    )
    .await
    {
        Ok(Ok(Some(ms))) => {
            let pass = ms <= effective.video_startup_latency_ms;
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
        Err(_) => tests.push(TestResult::fail(
            "harness_startup_latency_ms",
            format!("harness step timed out after {}s", step_cap_short.as_secs()),
        )),
    }

    debug!(url = %url, duration_sec = effective.short_duration_sec, "harness: bitrate/fps");
    match timeout(
        step_cap_long,
        harness_bitrate_fps(
            &url,
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
        Err(_) => tests.push(TestResult::fail(
            "harness_bitrate_fps",
            format!("harness step timed out after {}s", step_cap_long.as_secs()),
        )),
    }

    debug!(url = %url, "harness: SDP validation");
    match timeout(
        step_cap_short,
        harness_sdp_validation(&url, timeout_sec, &artifacts_dir, capture_tool_output),
    )
    .await
    {
        Ok(Ok((video_count, audio_count, has_h264))) => {
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
        }
        Ok(Err(e)) => tests.push(TestResult::fail("harness_sdp_validation", e.to_string())),
        Err(_) => tests.push(TestResult::fail(
            "harness_sdp_validation",
            format!("harness step timed out after {}s", step_cap_short.as_secs()),
        )),
    }

    debug!(url = %url, "harness: RTSP protocol sequence");
    match timeout(
        step_cap_long,
        harness_rtsp_protocol_sequence(&url, effective, args),
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
        Err(_) => tests.push(TestResult::fail(
            "harness_protocol_sequence",
            format!("harness step timed out after {}s", step_cap_long.as_secs()),
        )),
    }

    debug!(url = %url, "harness: packet loss");
    match timeout(step_cap_long, harness_packet_loss(&url, effective, args)).await {
        Ok(Ok((rtp_packets, packet_loss, loss_percent))) => {
            let pass =
                packet_loss_within_tolerance(loss_percent, effective.packet_loss_tolerance_percent);
            tests.push(TestResult::metric(
                "harness_packet_loss_percent",
                serde_json::json!({ "rtp_packets": rtp_packets, "packet_loss": packet_loss, "loss_percent": loss_percent }),
                pass,
            ));
        }
        Ok(Err(e)) => tests.push(TestResult::fail("harness_packet_loss", e.to_string())),
        Err(_) => tests.push(TestResult::fail(
            "harness_packet_loss",
            format!("harness step timed out after {}s", step_cap_long.as_secs()),
        )),
    }

    if effective.concurrent_clients > 0 {
        debug!(url = %url, concurrent = effective.concurrent_clients, "harness: concurrent clients");
        match timeout(
            step_cap_long,
            harness_concurrent_clients(
                &url,
                effective.short_duration_sec,
                effective.concurrent_clients,
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
            Err(_) => tests.push(TestResult::fail(
                "harness_concurrent_clients",
                format!("harness step timed out after {}s", step_cap_long.as_secs()),
            )),
        }
    }

    if args.long_duration {
        let step_cap_long_duration =
            Duration::from_secs(effective.long_duration_sec.saturating_add(30));
        debug!(url = %url, duration_sec = effective.long_duration_sec, "harness: long duration");
        match timeout(
            step_cap_long_duration,
            harness_long_duration(
                &url,
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
            Err(_) => tests.push(TestResult::fail(
                "harness_long_duration",
                format!(
                    "harness step timed out after {}s",
                    step_cap_long_duration.as_secs()
                ),
            )),
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
            Err(_) => tests.push(TestResult::fail(
                "harness_error_handling",
                format!("harness step timed out after {}s", step_cap_short.as_secs()),
            )),
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
            l.write_line(&format!("url={}", url))?;
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
    _threshold_ms: u64,
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
            l.write_line(&format!("url={}", url))?;
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
            l.write_line(&format!("url={}", url))?;
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
) -> Result<(usize, usize, bool)> {
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
        Ok::<_, anyhow::Error>((video.len(), audio.len(), has_h264))
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
    let short_dur = effective.short_duration_sec;
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
            l.write_line(&format!("url={}", url2))?;
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
                    for meta in layer {
                        if name == "rtsp" {
                            if meta.name() == "rtsp.method" {
                                match meta.value() {
                                    "DESCRIBE" => describe += 1,
                                    "SETUP" => setup += 1,
                                    "PLAY" => play += 1,
                                    "TEARDOWN" => teardown += 1,
                                    _ => {}
                                }
                            }
                            if meta.name() == "rtsp.status_code"
                                && let Ok(n) = meta.value().parse::<u32>()
                            {
                                if n == 200 {
                                    status_200 += 1;
                                } else if n >= 400 {
                                    status_err += 1;
                                }
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
) -> Result<(u32, u32, f64)> {
    let iface = effective.capture_interface.clone();
    let url = url.to_string();
    let artifacts_dir = effective.artifacts_dir.clone();
    let capture_tool_output = effective.capture_tool_output;
    let keep_pcaps = effective.keep_pcaps;
    let pcap_path = artifacts_dir.join("rtp_packet_loss_capture.pcap");

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
            l.write_line(&format!("url={}", url2))?;
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

    let pcap_path_str = pcap_path.to_string_lossy().to_string();
    let (rtp_packets, packet_loss, loss_percent) = tokio::task::spawn_blocking(move || {
        let mut builder = RTSharkBuilder::builder();
        let mut rtshark = builder
            .input_path(&pcap_path_str)
            .spawn()
            .context("rtshark spawn")?;
        let mut seqs: Vec<u16> = Vec::new();
        while let Some(packet) = rtshark.read().context("rtshark read")? {
            for layer in packet {
                if layer.name() == "rtp" {
                    for meta in layer {
                        if meta.name() == "rtp.seq" {
                            if let Ok(n) = meta.value().parse::<u16>() {
                                seqs.push(n);
                            }
                            break;
                        }
                    }
                }
            }
        }
        if !keep_pcaps {
            let _ = std::fs::remove_file(&pcap_path_str);
        }
        seqs.sort();
        let mut loss = 0u32;
        for w in seqs.windows(2) {
            let diff = (w[1] as i32 - w[0] as i32).unsigned_abs();
            if diff > 1 && diff != 65535 {
                loss += diff - 1;
            }
        }
        let total = seqs.len() as u32;
        let loss_pct = if total > 0 {
            100.0 * (loss as f64) / (total as f64)
        } else {
            0.0
        };
        Ok::<_, anyhow::Error>((total, loss, loss_pct))
    })
    .await
    .context("spawn_blocking")??;

    Ok((rtp_packets, packet_loss, loss_percent))
}

async fn harness_concurrent_clients(
    url: &str,
    duration_sec: u64,
    count: u32,
    artifacts_dir: &Path,
    capture_tool_output: bool,
) -> Result<u32> {
    let mut handles = Vec::new();
    for i in 0..count {
        let url = url.to_string();
        let dur = duration_sec;
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
                l.write_line(&format!("url={}", url))?;
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
            l.write_line(&format!("url={}", url))?;
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
    _timeout_sec: u64,
    artifacts_dir: &Path,
    capture_tool_output: bool,
) -> Result<(bool, bool)> {
    let invalid_url = format!("rtsp://invalid:invalid@{}:{}{}", host, port, stream);
    let bogus_url = format!("rtsp://{}:{}/bogus_stream", host, port);

    let invalid_log_path = artifacts_dir.join("ffmpeg_error_invalid_creds.log");
    let invalid_ok = tokio::task::spawn_blocking(move || {
        let mut log = if capture_tool_output {
            Some(BoundedLogWriter::create(&invalid_log_path)?)
        } else {
            None
        };
        if let Some(l) = log.as_mut() {
            l.write_line("=== ffmpeg error handling: invalid creds ===")?;
            l.write_line(&format!("url={}", invalid_url))?;
        }
        let mut cmd = FfmpegCommand::new();
        cmd.hide_banner()
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
    let bogus_ok = tokio::task::spawn_blocking(move || {
        let mut log = if capture_tool_output {
            Some(BoundedLogWriter::create(&bogus_log_path)?)
        } else {
            None
        };
        if let Some(l) = log.as_mut() {
            l.write_line("=== ffmpeg error handling: bogus url ===")?;
            l.write_line(&format!("url={}", bogus_url))?;
        }
        let mut cmd = FfmpegCommand::new();
        cmd.hide_banner()
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
        BoundedLogWriter, bitrate_within_tolerance, build_sdp_test_results, critical_proto_failed,
        empty_report, fps_within_tolerance, packet_loss_within_tolerance, result_ok, rtsp_url,
        to_retina_transport, validate_h264_length_prefixed_nals,
    };
    use crate::config::TransportArg;
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
    fn test_to_retina_transport() {
        let tcp = to_retina_transport(TransportArg::Tcp);
        assert!(matches!(tcp, retina::client::Transport::Tcp(_)));
        let udp = to_retina_transport(TransportArg::Udp);
        assert!(matches!(udp, retina::client::Transport::Udp(_)));
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
