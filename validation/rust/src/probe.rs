//! Retina-based live RTSP probe: DESCRIBE/SETUP/PLAY sequencing, SDP structure,
//! and first-frame timing measured in-process.

use anyhow::{Context, Result, bail};
use chrono::Utc;
use futures_util::StreamExt;
use retina::client::{
    Credentials, InitialTimestampPolicy, PlayOptions, Session, SessionOptions, SetupOptions,
    Transport,
};
use retina::codec::{CodecItem, ParametersRef};
use std::time::Duration;
use tokio::time::{Instant, timeout};
use tracing::{debug, info, trace, warn};
use url::Url;

use crate::config::{Args, EffectiveConfig, InitialTimestampPolicyArg, TransportArg};
use crate::report::{StreamInfo, TestResult, TestRun, ValidationReport, result_ok};

const PROBE_DEMUX_ERROR_TOLERANCE: u32 = 3;

fn to_retina_transport(arg: TransportArg) -> Transport {
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

fn stream_info_from_retina(s: &retina::client::Stream) -> StreamInfo {
    StreamInfo {
        media: s.media().to_string(),
        encoding_name: s.encoding_name().to_string(),
        control_present: s.control().is_some(),
    }
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

/// Build a report whose summary is not filled in yet.
///
/// The caller (`main`) always recomputes `summary` with `compute_summary` once every
/// stream has run, so there is nothing useful to put here.
pub fn empty_report(test_run: TestRun, tests: Vec<TestResult>) -> ValidationReport {
    ValidationReport {
        test_run,
        tests,
        summary: crate::report::Summary::default(),
        artifacts_dir: None,
        telemetry: None,
        telemetry_before_shutdown: None,
    }
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
    let url_str = crate::harness::rtsp_url(
        &effective.rtsp_host,
        effective.rtsp_port,
        &effective.rtsp_stream,
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

    Ok(empty_report(test_run, tests))
}

#[cfg(test)]
mod tests {
    use super::{
        build_sdp_test_results, critical_proto_failed, empty_report,
        to_retina_initial_timestamp_policy, to_retina_transport,
        validate_h264_length_prefixed_nals,
    };
    use crate::config::{InitialTimestampPolicyArg, TransportArg};
    use crate::report::{StreamInfo, TestResult, TestRun, result_ok};

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
}
