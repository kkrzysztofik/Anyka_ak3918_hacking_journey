//! External-tool RTSP harness: drives ffmpeg, ffprobe and tshark against a
//! running server and turns their output into test results.
//!
//! The in-process Retina probe lives in [`crate::probe`]; pcap/RTP analysis in
//! [`crate::rtp`].

use anyhow::{Context, Result, anyhow, bail};
use ffmpeg_sidecar::command::FfmpegCommand;
use ffmpeg_sidecar::event::{FfmpegEvent, FfmpegProgress};
use serde::Deserialize;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::time::timeout;
use tracing::{debug, info, warn};
use url::Url;

use crate::config::{Args, EffectiveConfig};
use crate::report::TestResult;
use crate::rtp::{
    FramePacing, GapStats, RtpLossMetric, RtpPcapRfc3640Stats, RtpPcapRfc6184Stats,
    analyze_aac_rfc3640_from_rows, analyze_h264_rfc6184_from_rows, compute_pacing,
    compute_stream_loss_metric, group_rtp_rows_by_stream, pick_primary_audio_stream,
    pick_primary_video_stream, tshark_extract_rtp_rows,
};
use crate::util::{
    MAX_TOOL_LOG_BYTES, parse_ffmpeg_summary_bitrate_kbps, rtsp_url, tail_lossy, write_bytes_tail,
};

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

fn parse_status_code(value: &str) -> Option<u32> {
    value
        .split_whitespace()
        .find_map(|token| token.parse::<u32>().ok())
}

#[derive(Debug)]
struct HarnessPacketLossResult {
    video: Option<RtpLossMetric>,
    audio: Option<RtpLossMetric>,
    h264_rfc6184: Option<std::result::Result<RtpPcapRfc6184Stats, String>>,
    aac_rfc3640: Option<std::result::Result<RtpPcapRfc3640Stats, String>>,
    pacing: Option<FramePacing>,
}

/// Returns true if measured bitrate is within tolerance_percent of expected.
fn bitrate_within_tolerance(
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
fn fps_within_tolerance(measured: f64, expected: f64, tolerance_percent: u32) -> bool {
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
fn packet_loss_within_tolerance(loss_percent: f64, max_percent: f64) -> bool {
    loss_percent <= max_percent
}

struct BoundedLogWriter {
    file: File,
    written: usize,
    truncated: bool,
}

impl BoundedLogWriter {
    fn create(path: &Path) -> Result<Self> {
        let file = File::create(path).with_context(|| format!("create {}", path.display()))?;
        Ok(Self {
            file,
            written: 0,
            truncated: false,
        })
    }

    fn write_line(&mut self, line: &str) -> Result<()> {
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

/// A running `tshark -w` capture writing to `pcap_path`.
///
/// Both harness captures follow the same shape: start tshark, let it settle, drive
/// traffic through ffmpeg, let it drain, then kill it and parse the pcap.
struct TsharkCapture {
    child: std::process::Child,
    pcap_path: std::path::PathBuf,
}

impl TsharkCapture {
    /// Spawn tshark on `iface` with capture filter `filter`, then wait for it to settle.
    async fn start(
        artifacts_dir: &Path,
        log_stem: &str,
        capture_tool_output: bool,
        iface: &str,
        filter: &str,
        pcap_path: std::path::PathBuf,
    ) -> Result<Self> {
        let sink = |suffix: &str| -> Result<Stdio> {
            if !capture_tool_output {
                return Ok(Stdio::null());
            }
            let path = artifacts_dir.join(format!("{}.{}.log", log_stem, suffix));
            Ok(Stdio::from(
                File::create(&path).with_context(|| format!("create {}", path.display()))?,
            ))
        };

        let child = Command::new("tshark")
            .args([
                "-i",
                iface,
                "-f",
                filter,
                "-w",
                &pcap_path.to_string_lossy(),
            ])
            .stdout(sink("stdout")?)
            .stderr(sink("stderr")?)
            .spawn()
            .context("spawn tshark")?;
        info!(pcap = %pcap_path.display(), filter, "tshark capture started");

        // Give tshark a moment to bind the interface before traffic starts.
        tokio::time::sleep(Duration::from_secs(1)).await;
        Ok(Self { child, pcap_path })
    }

    /// Let in-flight packets land, stop tshark, and return the pcap path.
    async fn stop(mut self) -> std::path::PathBuf {
        tokio::time::sleep(Duration::from_secs(2)).await;
        let _ = self.child.kill();
        let _ = self.child.wait();
        self.pcap_path
    }
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

    let within = |m: &RtpLossMetric| {
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
            harness_sdp_validation(&auth_url, &artifacts_dir, capture_tool_output),
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
            harness_rtsp_protocol_sequence(&auth_url, effective),
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
            harness_packet_loss(&auth_url, effective, expect_h264, expect_aac),
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
) -> Result<RtspProtocolSequenceStats> {
    let iface = effective.capture_interface.clone();
    let port = effective.rtsp_port;
    let url = url.to_string();
    let artifacts_dir = effective.artifacts_dir.clone();
    let capture_tool_output = effective.capture_tool_output;
    let keep_pcaps = effective.keep_pcaps;
    let pcap_path = artifacts_dir.join(format!("rtsp_protocol_sequence_tcp_port{}.pcap", port));

    let capture = TsharkCapture::start(
        &artifacts_dir,
        "tshark_rtsp_protocol_sequence",
        capture_tool_output,
        &iface,
        &format!("tcp port {}", port),
        pcap_path,
    )
    .await?;

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

    let pcap_path = capture.stop().await;

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

    let capture = TsharkCapture::start(
        &artifacts_dir,
        "tshark_packet_loss",
        capture_tool_output,
        &iface,
        &filter,
        pcap_path,
    )
    .await?;

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

    let pcap_path = capture.stop().await;

    let (video, audio, h264_rfc6184, aac_rfc3640, pacing) =
        tokio::task::spawn_blocking(move || {
            let mut rows =
                tshark_extract_rtp_rows(&pcap_path).context("tshark extract rtp rows")?;

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
                let _ = std::fs::remove_file(&pcap_path);
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
        BoundedLogWriter, HarnessPacketLossResult, HarnessProcessOutcome, RtpLossMetric,
        RtpPcapRfc3640Stats, RtpPcapRfc6184Stats, RtspProtocolSequenceStats,
        append_harness_basic_connectivity_result, append_harness_bitrate_fps_result,
        append_harness_concurrent_clients_result, append_harness_error_handling_result,
        append_harness_long_duration_result, append_harness_packet_loss_result,
        append_harness_protocol_sequence_result, append_harness_sdp_validation_result,
        append_harness_startup_latency_result, bitrate_within_tolerance, fps_within_tolerance,
        harness_bitrate_pass, harness_fps_pass, harness_protocol_sequence_pass,
        packet_loss_within_tolerance, redact_url_credentials, rtsp_sequence_stats_from_field_rows,
        rtsp_url_with_credentials,
    };
    use crate::report::{TestResult, result_ok};
    use crate::util::MAX_TOOL_LOG_BYTES;
    use std::time::Duration;

    fn fail_reason(test: &TestResult) -> Option<&str> {
        match test {
            TestResult::Fail { reason, .. } => Some(reason),
            _ => None,
        }
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
                video: Some(RtpLossMetric {
                    rtp_packets: 100,
                    packet_loss: 0,
                    loss_percent: 0.0,
                    payload_type: 96,
                    ssrc: Some(1),
                }),
                audio: Some(RtpLossMetric {
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
