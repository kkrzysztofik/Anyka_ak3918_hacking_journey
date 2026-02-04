//! Host-side RTSP validation tool.
//!
//! Launches `onvif-rust` in validation-mode (or connects to an existing server),
//! then performs **protocol-level** RTSP/SDP/RTP conformance checks using an
//! external RTSP client library (Retina) and writes a structured JSON report.
//!
//! Performance/bitrate/FPS measurements are primarily handled by
//! `scripts/rtsp_validation_tool.sh` via ffmpeg/ffprobe/tshark.

use anyhow::{Context, Result, anyhow, bail};
use chrono::Utc;
use clap::{Parser, ValueEnum};
use futures_util::StreamExt;
use retina::client::{Credentials, PlayOptions, Session, SessionOptions, SetupOptions, Transport};
use retina::codec::{CodecItem, ParametersRef};
use serde::Serialize;
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use std::env;
use tokio::net::TcpStream;
use tokio::time::{Instant, sleep, timeout};
use tracing::{debug, info, warn};
use tracing_subscriber::EnvFilter;
use url::Url;

const DEFAULT_VIDEO_STARTUP_TARGET_MS: u64 = 1500;

#[derive(Parser, Debug)]
#[command(name = "rtsp_validation_tool")]
#[command(about = "RTSP protocol conformance validation (host-side)")]
struct Args {
    /// Path to H.264 test file (used when launching server)
    #[arg(long)]
    h264_file: Option<String>,

    /// RTSP host (default: 127.0.0.1)
    #[arg(long, default_value = "127.0.0.1")]
    rtsp_host: String,

    /// RTSP port
    #[arg(long, default_value = "554")]
    rtsp_port: u16,

    /// RTSP stream path (example: /vs0 or /stream1)
    #[arg(long, default_value = "/stream1")]
    rtsp_stream: String,

    /// Username for RTSP authentication (digest; used when server challenges)
    #[arg(long)]
    username: Option<String>,

    /// Password for RTSP authentication (digest; used when server challenges)
    #[arg(long)]
    password: Option<String>,

    /// Transport to request for SETUP
    #[arg(long, value_enum, default_value_t = TransportArg::Tcp)]
    transport: TransportArg,

    /// HTTP-FLV port (only used when launching server)
    #[arg(long, default_value = "8080")]
    httpflv_port: u16,

    /// Probe duration in seconds (how long to wait for frames/packets)
    #[arg(long, default_value = "60")]
    duration: u64,

    /// Maximum acceptable video startup latency (ms) before failing the metric
    #[arg(long, default_value_t = DEFAULT_VIDEO_STARTUP_TARGET_MS)]
    max_video_startup_latency_ms: u64,

    /// Output JSON report path
    #[arg(long, default_value = "rtsp_validation.json")]
    output: String,

    /// Path to onvif-rust binary (default: cross-compile/onvif-rust/target/debug/onvif-rust)
    #[arg(long)]
    onvif_binary: Option<String>,

    /// Do not launch server; connect to existing server
    #[arg(long)]
    no_launch: bool,

    /// Loop playback for long-duration tests (only used when launching server)
    #[arg(long)]
    loop_playback: bool,

    /// Require that at least one audio frame is observed during the probe window.
    #[arg(long)]
    require_audio: bool,
}

#[derive(ValueEnum, Clone, Debug)]
enum TransportArg {
    Tcp,
    Udp,
}

impl TransportArg {
    fn to_retina_transport(&self) -> Transport {
        match self {
            TransportArg::Tcp => Transport::Tcp(Default::default()),
            TransportArg::Udp => Transport::Udp(Default::default()),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
struct StreamInfo {
    media: String,
    encoding_name: String,
    control_present: bool,
}

impl From<&retina::client::Stream> for StreamInfo {
    fn from(s: &retina::client::Stream) -> Self {
        Self {
            media: s.media().to_string(),
            encoding_name: s.encoding_name().to_string(),
            control_present: s.control().is_some(),
        }
    }
}

#[derive(Serialize)]
struct TestRun {
    timestamp: String,
    rtsp_host: String,
    rtsp_port: u16,
    rtsp_stream: String,
    test_duration_seconds: u64,
}

#[derive(Serialize)]
struct ValidationReport {
    test_run: TestRun,
    tests: Vec<TestResult>,
    summary: Summary,
}

#[derive(Serialize)]
#[serde(tag = "type", content = "value")]
enum TestResult {
    Pass {
        name: String,
    },
    Fail {
        name: String,
        reason: String,
    },
    Metric {
        name: String,
        value: serde_json::Value,
        pass: bool,
    },
}

#[derive(Serialize)]
struct Summary {
    total_tests: usize,
    passed: usize,
    failed: usize,
    overall_pass: bool,
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let args = Args::parse();
    validate_args(&args)?;

    let child = maybe_launch_server(&args).context("failed to launch onvif-rust")?;
    if child.is_some() {
        wait_for_server(&args.rtsp_host, args.rtsp_port)
            .await
            .context("server did not become ready")?;
    }

    let mut report = run_validation(&args)
        .await
        .context("RTSP validation run failed")?;

    if let Some(mut c) = child {
        let _ = c.kill();
        let _ = c.wait();
    }

    let passed = report.tests.iter().filter(|t| result_ok(t)).count();
    let failed = report.tests.len().saturating_sub(passed);
    report.summary = Summary {
        total_tests: report.tests.len(),
        passed,
        failed,
        overall_pass: failed == 0,
    };

    let json = serde_json::to_string_pretty(&report).context("failed to serialize JSON report")?;
    std::fs::write(&args.output, json)
        .with_context(|| format!("failed to write {}", args.output))?;
    info!(path = %args.output, "RTSP validation report written");
    Ok(())
}

fn result_ok(r: &TestResult) -> bool {
    match r {
        TestResult::Pass { .. } => true,
        TestResult::Fail { .. } => false,
        TestResult::Metric { pass, .. } => *pass,
    }
}

fn validate_args(args: &Args) -> Result<()> {
    if !args.no_launch && args.h264_file.is_none() {
        bail!("when not using --no-launch, --h264-file <path> is required");
    }

    match (&args.username, &args.password) {
        (Some(_), None) | (None, Some(_)) => {
            bail!("--username and --password must be provided together");
        }
        _ => {}
    }

    if !args.rtsp_stream.starts_with('/') {
        bail!(
            "--rtsp-stream must start with '/' (got {})",
            args.rtsp_stream
        );
    }

    Ok(())
}

fn maybe_launch_server(args: &Args) -> Result<Option<Child>> {
    if args.no_launch {
        return Ok(None);
    }

    let h264_file = args.h264_file.as_ref().context("missing --h264-file")?;

    let bin = args.onvif_binary.clone().unwrap_or_else(|| {
        // Keep the same heuristic as before, but avoid panicking if cwd is unavailable.
        let cwd = env::current_dir().unwrap_or_else(|_| ".".into());
        let cwd_str = cwd.to_string_lossy();
        if cwd_str.ends_with("onvif-rust") {
            format!("{}/target/debug/onvif-rust", cwd.display())
        } else {
            format!(
                "{}/cross-compile/onvif-rust/target/debug/onvif-rust",
                cwd.display()
            )
        }
    });

    info!(%bin, "starting onvif-rust in validation mode");

    let mut cmd = Command::new(&bin);
    cmd.arg("--validation-mode")
        .arg("--h264-file")
        .arg(h264_file)
        .arg("--rtsp-port")
        .arg(args.rtsp_port.to_string())
        .arg("--httpflv-port")
        .arg(args.httpflv_port.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if args.loop_playback {
        cmd.arg("--loop-playback");
    }

    let child = cmd
        .spawn()
        .with_context(|| format!("failed to spawn {}", bin))?;
    Ok(Some(child))
}

async fn wait_for_server(host: &str, port: u16) -> Result<()> {
    let addr = format!("{}:{}", host, port);
    for attempt in 1..=30u32 {
        match TcpStream::connect(&addr).await {
            Ok(_) => {
                sleep(Duration::from_millis(200)).await;
                return Ok(());
            }
            Err(e) => {
                debug!(attempt, error = %e, "RTSP port not ready yet");
                sleep(Duration::from_secs(1)).await;
            }
        }
    }
    Err(anyhow!("server did not become ready on {}", addr))
}

async fn run_validation(args: &Args) -> Result<ValidationReport> {
    let timestamp = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let test_run = TestRun {
        timestamp,
        rtsp_host: args.rtsp_host.clone(),
        rtsp_port: args.rtsp_port,
        rtsp_stream: args.rtsp_stream.clone(),
        test_duration_seconds: args.duration,
    };

    let mut tests: Vec<TestResult> = Vec::new();

    let url_str = format!(
        "rtsp://{}:{}{}",
        args.rtsp_host, args.rtsp_port, args.rtsp_stream
    );
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

    let describe_start = Instant::now();
    let mut session = match Session::describe(url, options).await {
        Ok(s) => {
            tests.push(TestResult::Pass {
                name: "describe_ok".to_string(),
            });
            s
        }
        Err(e) => {
            tests.push(TestResult::Fail {
                name: "describe_ok".to_string(),
                reason: e.to_string(),
            });
            return Ok(empty_report(test_run, tests));
        }
    };
    let describe_ms = describe_start.elapsed().as_millis() as u64;
    tests.push(TestResult::Metric {
        name: "describe_latency_ms".to_string(),
        value: serde_json::json!(describe_ms),
        pass: true,
    });

    let stream_infos: Vec<StreamInfo> = session.streams().iter().map(StreamInfo::from).collect();
    tests.push(TestResult::Metric {
        name: "stream_count".to_string(),
        value: serde_json::json!(stream_infos.len()),
        pass: !stream_infos.is_empty(),
    });
    tests.push(TestResult::Metric {
        name: "sdp_streams".to_string(),
        value: serde_json::json!(stream_infos),
        pass: true,
    });

    let has_video = stream_infos.iter().any(|s| s.media == "video");
    tests.push(if has_video {
        TestResult::Pass {
            name: "sdp_has_video".to_string(),
        }
    } else {
        TestResult::Fail {
            name: "sdp_has_video".to_string(),
            reason: "no SDP stream with media=video".to_string(),
        }
    });

    let video_is_h264 = stream_infos
        .iter()
        .any(|s| s.media == "video" && s.encoding_name == "h264");
    tests.push(if !has_video {
        TestResult::Pass {
            name: "video_encoding_h264".to_string(),
        }
    } else if video_is_h264 {
        TestResult::Pass {
            name: "video_encoding_h264".to_string(),
        }
    } else {
        TestResult::Fail {
            name: "video_encoding_h264".to_string(),
            reason: "no video stream advertised encoding_name=h264".to_string(),
        }
    });

    let has_audio = stream_infos.iter().any(|s| s.media == "audio");
    tests.push(TestResult::Metric {
        name: "sdp_has_audio".to_string(),
        value: serde_json::json!(has_audio),
        pass: true,
    });

    if stream_infos.len() > 1 {
        let all_have_control = stream_infos.iter().all(|s| s.control_present);
        tests.push(if all_have_control {
            TestResult::Pass {
                name: "multitrack_controls_present".to_string(),
            }
        } else {
            TestResult::Fail {
                name: "multitrack_controls_present".to_string(),
                reason: "multiple streams advertised but at least one lacks a=control".to_string(),
            }
        });
    } else {
        tests.push(TestResult::Pass {
            name: "multitrack_controls_present".to_string(),
        });
    }

    let setup_transport = args.transport.to_retina_transport();
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
                tests.push(TestResult::Metric {
                    name: format!("setup_stream_{}_latency_ms", i),
                    value: serde_json::json!(elapsed_ms),
                    pass: true,
                });
            }
            Err(e) => {
                setup_ok = false;
                tests.push(TestResult::Fail {
                    name: format!("setup_stream_{}", i),
                    reason: format!(
                        "SETUP failed for stream {} (media={}, encoding={}): {}",
                        i,
                        s.media,
                        s.encoding_name,
                        e
                    ),
                });
            }
        }
    }
    tests.push(TestResult::Metric {
        name: "setup_all_streams_ok".to_string(),
        value: serde_json::json!(setup_ok),
        pass: setup_ok,
    });
    if !setup_ok {
        return Ok(empty_report(test_run, tests));
    }

    let play_opts = PlayOptions::default();
    let play_start = Instant::now();
    let playing = match session.play(play_opts).await {
        Ok(s) => {
            tests.push(TestResult::Pass {
                name: "play_ok".to_string(),
            });
            s
        }
        Err(e) => {
            tests.push(TestResult::Fail {
                name: "play_ok".to_string(),
                reason: e.to_string(),
            });
            return Ok(empty_report(test_run, tests));
        }
    };
    let play_rtt_ms = play_start.elapsed().as_millis() as u64;
    tests.push(TestResult::Metric {
        name: "play_rtt_ms".to_string(),
        value: serde_json::json!(play_rtt_ms),
        pass: true,
    });

    let mut demuxed = playing.demuxed().context("failed to demux/depacketize")?;

    let mut first_video_latency_ms: Option<u64> = None;
    let mut first_audio_latency_ms: Option<u64> = None;
    let mut video_frames: u64 = 0;
    let mut audio_frames: u64 = 0;
    let mut total_loss_packets: u64 = 0;
    let mut saw_rap: bool = false;
    let mut h264_length_prefix_ok: bool = true;
    let mut h264_length_prefix_error: Option<String> = None;

    let probe_duration = Duration::from_secs(args.duration);
    let probe_res: Result<()> = timeout(probe_duration, async {
        while let Some(item) = demuxed.next().await {
            let item = item.context("demuxed stream error")?;
            match item {
                CodecItem::VideoFrame(frame) => {
                    video_frames = video_frames.saturating_add(1);
                    total_loss_packets = total_loss_packets.saturating_add(frame.loss() as u64);
                    if first_video_latency_ms.is_none() {
                        first_video_latency_ms = Some(play_start.elapsed().as_millis() as u64);
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
        tests.push(TestResult::Fail {
            name: "probe_loop".to_string(),
            reason: e.to_string(),
        });
    } else {
        tests.push(TestResult::Pass {
            name: "probe_loop".to_string(),
        });
    }

    tests.push(TestResult::Metric {
        name: "video_frames_observed".to_string(),
        value: serde_json::json!(video_frames),
        pass: video_frames > 0,
    });
    tests.push(TestResult::Metric {
        name: "audio_frames_observed".to_string(),
        value: serde_json::json!(audio_frames),
        pass: !args.require_audio || !has_audio || audio_frames > 0,
    });

    if let Some(latency_ms) = first_video_latency_ms {
        tests.push(TestResult::Metric {
            name: "first_video_frame_latency_ms".to_string(),
            value: serde_json::json!(latency_ms),
            pass: latency_ms <= args.max_video_startup_latency_ms,
        });
    } else {
        tests.push(TestResult::Fail {
            name: "first_video_frame_latency_ms".to_string(),
            reason: "no video frames observed during probe window".to_string(),
        });
    }

    if let Some(latency_ms) = first_audio_latency_ms {
        tests.push(TestResult::Metric {
            name: "first_audio_frame_latency_ms".to_string(),
            value: serde_json::json!(latency_ms),
            pass: true,
        });
    }

    tests.push(TestResult::Metric {
        name: "rtp_loss_packets_total".to_string(),
        value: serde_json::json!(total_loss_packets),
        pass: total_loss_packets == 0,
    });

    tests.push(TestResult::Metric {
        name: "random_access_point_seen".to_string(),
        value: serde_json::json!(saw_rap),
        pass: saw_rap,
    });

    tests.push(if h264_length_prefix_ok {
        TestResult::Pass {
            name: "h264_length_prefix_ok".to_string(),
        }
    } else {
        TestResult::Fail {
            name: "h264_length_prefix_ok".to_string(),
            reason: h264_length_prefix_error
                .unwrap_or_else(|| "invalid H.264 length-prefixed framing".to_string()),
        }
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
                _ => {
                    video_params_ok = false;
                }
            },
            "audio" => match s.parameters() {
                Some(ParametersRef::Audio(_)) => {}
                _ => {
                    audio_params_ok = false;
                }
            },
            _ => {}
        }
    }
    tests.push(TestResult::Metric {
        name: "video_parameters_available".to_string(),
        value: serde_json::json!(video_params_ok),
        pass: !has_video || video_params_ok,
    });
    tests.push(TestResult::Metric {
        name: "audio_parameters_available".to_string(),
        value: serde_json::json!(audio_params_ok),
        pass: !has_audio || audio_params_ok,
    });

    Ok(ValidationReport {
        test_run,
        tests,
        summary: Summary {
            total_tests: 0,
            passed: 0,
            failed: 0,
            overall_pass: false,
        },
    })
}

fn empty_report(test_run: TestRun, tests: Vec<TestResult>) -> ValidationReport {
    ValidationReport {
        test_run,
        tests,
        summary: Summary {
            total_tests: 0,
            passed: 0,
            failed: 0,
            overall_pass: false,
        },
    }
}

fn validate_h264_length_prefixed_nals(data: &[u8]) -> Result<()> {
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

#[cfg(test)]
mod tests {
    use super::validate_h264_length_prefixed_nals;

    #[test]
    fn test_validate_h264_length_prefixed_nals_ok_single() {
        // One NAL: length=1, header=0x65 (IDR slice).
        let data = [0, 0, 0, 1, 0x65];
        validate_h264_length_prefixed_nals(&data).unwrap();
    }

    #[test]
    fn test_validate_h264_length_prefixed_nals_rejects_truncated() {
        let data = [0, 0, 0, 2, 0x65];
        assert!(validate_h264_length_prefixed_nals(&data).is_err());
    }
}
