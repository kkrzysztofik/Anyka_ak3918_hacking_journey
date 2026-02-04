//! Host-side RTSP validation tool.
//!
//! Launches `onvif-rust` in validation-mode (or connects to an existing server),
//! then performs protocol-level RTSP/SDP/RTP checks (Retina) and harness
//! scenarios (ffmpeg, ffprobe, tshark) with a single JSON report.
//! Configuration is loaded from TOML (rtsp_validation.toml) with CLI overrides.

use anyhow::{Context, Result, anyhow, bail};
use chrono::Utc;
use clap::{Parser, ValueEnum};
use ffmpeg_sidecar::command::FfmpegCommand;
use ffmpeg_sidecar::event::{FfmpegEvent, FfmpegProgress};
use futures_util::StreamExt;
use retina::client::{Credentials, PlayOptions, Session, SessionOptions, SetupOptions, Transport};
use retina::codec::{CodecItem, ParametersRef};
use rtshark::RTSharkBuilder;
use serde::{Deserialize, Serialize};
use std::env;
use std::net::ToSocketAddrs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use telnet::{Event, Telnet};
use tokio::net::TcpStream;
use tokio::time::{Instant, sleep, timeout};
use tracing::{debug, info, warn};
use tracing_subscriber::EnvFilter;
use url::Url;

const DEFAULT_VIDEO_STARTUP_TARGET_MS: u64 = 1500;
const DEFAULT_RTSP_HOST: &str = "127.0.0.1";
const DEFAULT_RTSP_PORT: u16 = 554;
const DEFAULT_RTSP_STREAM: &str = "/stream1";
const DEFAULT_DURATION_SEC: u64 = 30;
const DEFAULT_LONG_DURATION_SEC: u64 = 600;
const DEFAULT_CONCURRENT_CLIENTS: u32 = 4;
const DEFAULT_BASELINE_DIR: &str = "rtsp_results/baselines";

/// TOML config file schema (rtsp_validation.toml).
#[derive(Debug, Default, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct RtspValidationConfig {
    #[serde(default)]
    pub rtsp: RtspSection,
    #[serde(default)]
    pub test: TestSection,
    #[serde(default)]
    pub thresholds: ThresholdsSection,
    #[serde(default)]
    pub baseline: BaselineSection,
    #[serde(default)]
    pub capture: CaptureSection,
    #[serde(default)]
    pub logging: LoggingSection,
    #[serde(default)]
    pub device: DeviceSection,
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct RtspSection {
    #[serde(default = "default_rtsp_host")]
    pub host: String,
    #[serde(default = "default_rtsp_port")]
    pub port: u16,
    #[serde(default = "default_rtsp_stream")]
    pub stream: String,
    #[serde(default = "default_rtsp_timeout")]
    pub timeout_sec: u64,
}

fn default_rtsp_host() -> String {
    DEFAULT_RTSP_HOST.to_string()
}
fn default_rtsp_port() -> u16 {
    DEFAULT_RTSP_PORT
}
fn default_rtsp_stream() -> String {
    DEFAULT_RTSP_STREAM.to_string()
}
fn default_rtsp_timeout() -> u64 {
    10
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct TestSection {
    #[serde(default = "default_short_duration")]
    pub short_duration_sec: u64,
    #[serde(default = "default_long_duration")]
    pub long_duration_sec: u64,
    #[serde(default = "default_concurrent_clients")]
    pub concurrent_clients: u32,
}

fn default_short_duration() -> u64 {
    DEFAULT_DURATION_SEC
}
fn default_long_duration() -> u64 {
    DEFAULT_LONG_DURATION_SEC
}
fn default_concurrent_clients() -> u32 {
    DEFAULT_CONCURRENT_CLIENTS
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ThresholdsSection {
    #[serde(default = "default_video_startup_latency_ms")]
    pub video_startup_latency_ms: u64,
    #[serde(default)]
    pub audio_startup_latency_ms: u64,
    #[serde(default = "default_bitrate_tolerance")]
    pub bitrate_tolerance_percent: u32,
    #[serde(default = "default_fps_tolerance")]
    pub fps_tolerance_percent: u32,
    #[serde(default = "default_packet_loss_tolerance")]
    pub packet_loss_tolerance_percent: f64,
    #[serde(default)]
    pub expected: Option<ThresholdsExpectedSection>,
}

fn default_video_startup_latency_ms() -> u64 {
    1500
}
fn default_bitrate_tolerance() -> u32 {
    15
}
fn default_fps_tolerance() -> u32 {
    10
}
fn default_packet_loss_tolerance() -> f64 {
    1.0
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ThresholdsExpectedSection {
    pub bitrate_kbps: Option<f64>,
    pub fps: Option<f64>,
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct BaselineSection {
    #[serde(default = "default_baseline_dir")]
    pub dir: String,
}

fn default_baseline_dir() -> String {
    DEFAULT_BASELINE_DIR.to_string()
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct CaptureSection {
    pub interface: Option<String>,
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct LoggingSection {
    #[serde(default)]
    pub level: String,
    #[serde(default)]
    pub file: String,
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct DeviceSection {
    #[serde(default = "default_device_host")]
    pub host: String,
    #[serde(default = "default_device_telnet_port")]
    pub telnet_port: u16,
    #[serde(default = "default_telemetry_enabled")]
    pub telemetry: bool,
}

fn default_device_host() -> String {
    "192.168.2.198".to_string()
}
fn default_device_telnet_port() -> u16 {
    24
}
fn default_telemetry_enabled() -> bool {
    true
}

/// Effective settings after merging config file and CLI (CLI overrides config).
#[derive(Debug, Clone)]
pub struct EffectiveConfig {
    pub rtsp_host: String,
    pub rtsp_port: u16,
    pub rtsp_stream: String,
    pub rtsp_timeout_sec: u64,
    pub short_duration_sec: u64,
    pub long_duration_sec: u64,
    pub concurrent_clients: u32,
    pub video_startup_latency_ms: u64,
    pub audio_startup_latency_ms: u64,
    pub bitrate_tolerance_percent: u32,
    pub fps_tolerance_percent: u32,
    pub packet_loss_tolerance_percent: f64,
    pub expected_bitrate_kbps: Option<f64>,
    pub expected_fps: Option<f64>,
    pub baseline_dir: PathBuf,
    pub capture_interface: String,
    pub launch_on_device: bool,
    pub device_host: String,
    pub device_telnet_port: u16,
    pub collect_telemetry: bool,
}

impl EffectiveConfig {
    /// Build effective config: config file first, then CLI overrides.
    fn from_config_and_args(config: Option<&RtspValidationConfig>, args: &Args) -> Self {
        let default_config = RtspValidationConfig::default();
        let c = config.unwrap_or(&default_config);
        let baseline_dir = if c.baseline.dir.is_empty() {
            PathBuf::from(DEFAULT_BASELINE_DIR)
        } else {
            PathBuf::from(&c.baseline.dir)
        };
        let capture_interface = c.capture.interface.clone().unwrap_or_default();
        let (expected_bitrate, expected_fps) = c
            .thresholds
            .expected
            .as_ref()
            .map(|e| (e.bitrate_kbps, e.fps))
            .unwrap_or((None, None));
        let device_host = args
            .device_host
            .clone()
            .or_else(|| Some(c.device.host.clone()))
            .filter(|h| !h.is_empty())
            .unwrap_or_else(|| "192.168.2.198".to_string());
        let device_telnet_port = args
            .device_telnet_port
            .or(Some(c.device.telnet_port))
            .filter(|&p| p != 0)
            .unwrap_or(24);
        let launch_on_device = args.launch_on_device;
        let collect_telemetry = launch_on_device && !args.no_telemetry && c.device.telemetry;
        let rtsp_host = if launch_on_device {
            device_host.clone()
        } else {
            args.rtsp_host.clone()
        };
        Self {
            rtsp_host,
            rtsp_port: args.rtsp_port,
            rtsp_stream: args.rtsp_stream.clone(),
            rtsp_timeout_sec: c.rtsp.timeout_sec,
            short_duration_sec: if args.duration > 0 {
                args.duration
            } else {
                c.test.short_duration_sec.max(1)
            },
            long_duration_sec: c.test.long_duration_sec,
            concurrent_clients: args.concurrent.unwrap_or(c.test.concurrent_clients),
            video_startup_latency_ms: if args.max_video_startup_latency_ms > 0 {
                args.max_video_startup_latency_ms
            } else {
                c.thresholds.video_startup_latency_ms.max(1)
            },
            audio_startup_latency_ms: c.thresholds.audio_startup_latency_ms,
            bitrate_tolerance_percent: c.thresholds.bitrate_tolerance_percent,
            fps_tolerance_percent: c.thresholds.fps_tolerance_percent,
            packet_loss_tolerance_percent: c.thresholds.packet_loss_tolerance_percent,
            expected_bitrate_kbps: expected_bitrate,
            expected_fps,
            baseline_dir,
            capture_interface,
            launch_on_device,
            device_host,
            device_telnet_port,
            collect_telemetry,
        }
    }

    /// Resolve capture interface: config value or "lo" for localhost, "any" otherwise.
    pub fn resolve_capture_interface(&mut self) {
        if !self.capture_interface.is_empty() {
            return;
        }
        let is_local = matches!(self.rtsp_host.as_str(), "127.0.0.1" | "localhost" | "::1");
        self.capture_interface = if is_local {
            "lo".to_string()
        } else {
            "any".to_string()
        };
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct BaselineFile {
    test: String,
    created: String,
    baseline_value: f64,
    tolerance_percent: u32,
    direction: String,
}

fn baseline_direction_for(test_name: &str) -> &'static str {
    match test_name {
        "startup_latency_ms"
        | "harness_startup_latency_ms"
        | "packet_loss_percent"
        | "harness_packet_loss_percent" => "lower",
        "bitrate_kbps" | "harness_bitrate_kbps" | "fps" | "harness_fps" => "higher",
        // Device telemetry: free/available/total RAM higher is better; load and process memory lower is better
        "telemetry_mem_free_kib"
        | "telemetry_mem_available_kib"
        | "telemetry_mem_total_kib" => "higher",
        "telemetry_load_avg_1m"
        | "telemetry_load_avg_5m"
        | "telemetry_load_avg_15m"
        | "telemetry_onvif_rss_kib"
        | "telemetry_onvif_vmsize_kib" => "lower",
        _ => "lower",
    }
}

fn update_baseline(
    baseline_dir: &Path,
    test_name: &str,
    value: f64,
    direction: &str,
) -> Result<()> {
    std::fs::create_dir_all(baseline_dir).context("create baseline dir")?;
    let path = baseline_dir.join(format!("{}_baseline.json", test_name));
    let created = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let file = BaselineFile {
        test: test_name.to_string(),
        created,
        baseline_value: value,
        tolerance_percent: 20,
        direction: direction.to_string(),
    };
    let json = serde_json::to_string_pretty(&file).context("serialize baseline")?;
    std::fs::write(&path, json).with_context(|| format!("write {}", path.display()))?;
    info!(test = %test_name, value = %value, "baseline updated");
    Ok(())
}

fn compare_against_baseline(
    baseline_dir: &Path,
    test_name: &str,
    current_value: f64,
    direction_override: Option<&str>,
) -> Result<bool> {
    let path = baseline_dir.join(format!("{}_baseline.json", test_name));
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => {
            debug!(test = %test_name, "no baseline file, skipping comparison");
            return Ok(true);
        }
    };
    let file: BaselineFile = serde_json::from_str(&content).context("parse baseline json")?;
    let baseline_value = file.baseline_value;
    let tolerance = file.tolerance_percent as f64;
    let direction = direction_override.unwrap_or(&file.direction);

    if baseline_value == 0.0 {
        return Ok(true);
    }

    let regression_pct = match direction {
        "lower" => 100.0 * (current_value - baseline_value) / baseline_value,
        "higher" => {
            let d = 100.0 * (baseline_value - current_value) / baseline_value;
            if d < 0.0 { 0.0 } else { d }
        }
        _ => 100.0 * (current_value - baseline_value) / baseline_value,
    };

    if regression_pct > tolerance {
        warn!(
            test = %test_name,
            regression_pct = %regression_pct,
            tolerance = %tolerance,
            "baseline regression"
        );
        Ok(false)
    } else {
        Ok(true)
    }
}

/// Load config from path. Tries env RTSP_VALIDATION_CONFIG, then ./rtsp_validation.toml, then validation/rtsp_validation.toml.
pub fn load_config(path_override: Option<&str>) -> Result<Option<RtspValidationConfig>> {
    let path = path_override
        .map(PathBuf::from)
        .or_else(|| env::var_os("RTSP_VALIDATION_CONFIG").map(PathBuf::from))
        .or_else(|| {
            let cwd = env::current_dir().ok()?;
            let p = cwd.join("rtsp_validation.toml");
            if p.is_file() { Some(p) } else { None }
        })
        .or_else(|| {
            let cwd = env::current_dir().ok()?;
            let p = cwd.join("validation").join("rtsp_validation.toml");
            if p.is_file() { Some(p) } else { None }
        });
    let path = match path {
        Some(p) => p,
        None => return Ok(None),
    };
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("read config {}", path.display()))?;
    let config: RtspValidationConfig =
        toml::from_str(&content).with_context(|| format!("parse config {}", path.display()))?;
    info!(path = %path.display(), "loaded RTSP validation config");
    Ok(Some(config))
}

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

    /// Path to TOML config file; else env RTSP_VALIDATION_CONFIG or rtsp_validation.toml in CWD/validation.
    #[arg(long)]
    config: Option<String>,

    /// Update baseline files for metrics (e.g. startup_latency_ms, bitrate_kbps).
    #[arg(long)]
    update_baseline: bool,

    /// Compare current metrics against baseline and fail on regression.
    #[arg(long)]
    compare_baseline: bool,

    /// Number of concurrent clients for harness scenario (overrides config).
    #[arg(long)]
    concurrent: Option<u32>,

    /// Run long-duration stability test (config long_duration_sec).
    #[arg(long)]
    long_duration: bool,

    /// Skip error-handling scenario (invalid creds, bogus URL).
    #[arg(long)]
    skip_error_handling: bool,

    /// Start onvif-rust on the device via telnet (do not launch locally).
    #[arg(long)]
    launch_on_device: bool,

    /// Device IP for telnet and RTSP when using --launch-on-device.
    #[arg(long)]
    device_host: Option<String>,

    /// Telnet port on device (default 24).
    #[arg(long)]
    device_telnet_port: Option<u16>,

    /// Disable telemetry collection when using --launch-on-device.
    #[arg(long)]
    no_telemetry: bool,
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

/// Device telemetry snapshot (RAM, CPU, onvif-rust memory) when using --launch-on-device.
#[derive(Debug, Default, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct DeviceTelemetry {
    pub mem_total_kib: Option<u64>,
    pub mem_free_kib: Option<u64>,
    pub mem_available_kib: Option<u64>,
    pub load_avg_1m: Option<f64>,
    pub load_avg_5m: Option<f64>,
    pub load_avg_15m: Option<f64>,
    pub onvif_rss_kib: Option<u64>,
    pub onvif_vmsize_kib: Option<u64>,
    pub onvif_pid: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Serialize)]
struct ValidationReport {
    test_run: TestRun,
    tests: Vec<TestResult>,
    summary: Summary,
    #[serde(skip_serializing_if = "Option::is_none")]
    telemetry: Option<DeviceTelemetry>,
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

    let config_path = args
        .config
        .clone()
        .or_else(|| env::var("RTSP_VALIDATION_CONFIG").ok());
    let config = load_config(config_path.as_deref())?;
    let mut effective = EffectiveConfig::from_config_and_args(config.as_ref(), &args);
    effective.resolve_capture_interface();

    let child = maybe_launch_server(&args).context("failed to launch onvif-rust")?;
    if child.is_some() {
        wait_for_server(&effective.rtsp_host, effective.rtsp_port)
            .await
            .context("server did not become ready")?;
    }

    if effective.launch_on_device {
        let host = effective.device_host.clone();
        let port = effective.device_telnet_port;
        tokio::task::spawn_blocking(move || device_start_onvif_blocking(&host, port))
            .await
            .context("spawn_blocking device start")?
            .context("device start onvif-rust")?;
        sleep(Duration::from_secs(2)).await;
        wait_for_server(&effective.rtsp_host, effective.rtsp_port)
            .await
            .context("device RTSP server did not become ready")?;
    }

    let mut report = run_validation(&args, &effective)
        .await
        .context("RTSP validation run failed")?;

    run_harness(&args, &effective, &mut report.tests).await?;

    if effective.launch_on_device && effective.collect_telemetry {
        let host = effective.device_host.clone();
        let port = effective.device_telnet_port;
        let telemetry =
            tokio::task::spawn_blocking(move || device_collect_telemetry_blocking(&host, port))
                .await
                .context("spawn_blocking telemetry")?;
        report.telemetry = Some(telemetry);
    }

    if effective.launch_on_device {
        let host = effective.device_host.clone();
        let port = effective.device_telnet_port;
        let stop_result =
            tokio::task::spawn_blocking(move || device_stop_onvif_blocking(&host, port))
                .await
                .context("spawn_blocking device stop");
        if let Err(e) = stop_result.and_then(|r| r) {
            warn!(error = %e, "device stop onvif-rust failed");
        }
    }

    if args.update_baseline || args.compare_baseline {
        apply_baseline_ops(&args, &effective, &mut report)?;
    }

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

fn rtsp_url(host: &str, port: u16, stream: &str) -> String {
    format!("rtsp://{}:{}{}", host, port, stream)
}

/// Run harness scenarios (ffmpeg/ffprobe/tshark) and append results to tests.
async fn run_harness(
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

    // 1. Basic connectivity
    match harness_basic_connectivity(&url, timeout_sec).await {
        Ok(ok) => {
            tests.push(if ok {
                TestResult::Pass {
                    name: "harness_basic_connectivity".to_string(),
                }
            } else {
                TestResult::Fail {
                    name: "harness_basic_connectivity".to_string(),
                    reason: "no stream in output".to_string(),
                }
            });
        }
        Err(e) => {
            tests.push(TestResult::Fail {
                name: "harness_basic_connectivity".to_string(),
                reason: e.to_string(),
            });
        }
    }

    // 2. Stream startup latency
    match harness_startup_latency(&url, timeout_sec, effective.video_startup_latency_ms).await {
        Ok(Some(ms)) => {
            let pass = ms <= effective.video_startup_latency_ms;
            tests.push(TestResult::Metric {
                name: "harness_startup_latency_ms".to_string(),
                value: serde_json::json!(ms),
                pass,
            });
        }
        Ok(None) => {
            tests.push(TestResult::Fail {
                name: "harness_startup_latency_ms".to_string(),
                reason: "no frame decoded".to_string(),
            });
        }
        Err(e) => {
            tests.push(TestResult::Fail {
                name: "harness_startup_latency_ms".to_string(),
                reason: e.to_string(),
            });
        }
    }

    // 3. Bitrate / FPS stability
    match harness_bitrate_fps(&url, effective.short_duration_sec, effective).await {
        Ok((bitrate, fps)) => {
            let bitrate_pass = effective
                .expected_bitrate_kbps
                .map(|e| {
                    let tol = effective.bitrate_tolerance_percent as f64 / 100.0;
                    (bitrate - e).abs() / e <= tol
                })
                .unwrap_or(true);
            let fps_pass = effective
                .expected_fps
                .map(|e| {
                    let tol = effective.fps_tolerance_percent as f64 / 100.0;
                    (fps - e).abs() / e <= tol
                })
                .unwrap_or(true);
            tests.push(TestResult::Metric {
                name: "harness_bitrate_kbps".to_string(),
                value: serde_json::json!(bitrate),
                pass: bitrate_pass,
            });
            tests.push(TestResult::Metric {
                name: "harness_fps".to_string(),
                value: serde_json::json!(fps),
                pass: fps_pass,
            });
        }
        Err(e) => {
            tests.push(TestResult::Fail {
                name: "harness_bitrate_fps".to_string(),
                reason: e.to_string(),
            });
        }
    }

    // 4. SDP validation (ffprobe)
    match harness_sdp_validation(&url, timeout_sec).await {
        Ok((video_count, audio_count, has_h264)) => {
            tests.push(TestResult::Metric {
                name: "harness_sdp_video_streams".to_string(),
                value: serde_json::json!(video_count),
                pass: video_count > 0,
            });
            tests.push(TestResult::Metric {
                name: "harness_sdp_audio_streams".to_string(),
                value: serde_json::json!(audio_count),
                pass: true,
            });
            tests.push(if has_h264 {
                TestResult::Pass {
                    name: "harness_sdp_video_h264".to_string(),
                }
            } else {
                TestResult::Fail {
                    name: "harness_sdp_video_h264".to_string(),
                    reason: "no H.264 video stream".to_string(),
                }
            });
        }
        Err(e) => {
            tests.push(TestResult::Fail {
                name: "harness_sdp_validation".to_string(),
                reason: e.to_string(),
            });
        }
    }

    // 5. RTSP protocol sequence (tshark + rtshark)
    match harness_rtsp_protocol_sequence(&url, effective, args).await {
        Ok((describe, setup, play, teardown, status_200, status_err)) => {
            let pass = describe > 0 && setup > 0 && play > 0 && status_err == 0 && status_200 > 0;
            tests.push(TestResult::Metric {
                name: "harness_protocol_sequence".to_string(),
                value: serde_json::json!({
                    "describe": describe,
                    "setup": setup,
                    "play": play,
                    "teardown": teardown,
                    "status_200": status_200,
                    "status_4xx": status_err,
                }),
                pass,
            });
        }
        Err(e) => {
            tests.push(TestResult::Fail {
                name: "harness_protocol_sequence".to_string(),
                reason: e.to_string(),
            });
        }
    }

    // 6. Packet loss (UDP + rtshark)
    match harness_packet_loss(&url, effective, args).await {
        Ok((rtp_packets, packet_loss, loss_percent)) => {
            let pass = loss_percent <= effective.packet_loss_tolerance_percent;
            tests.push(TestResult::Metric {
                name: "harness_packet_loss_percent".to_string(),
                value: serde_json::json!({ "rtp_packets": rtp_packets, "packet_loss": packet_loss, "loss_percent": loss_percent }),
                pass,
            });
        }
        Err(e) => {
            tests.push(TestResult::Fail {
                name: "harness_packet_loss".to_string(),
                reason: e.to_string(),
            });
        }
    }

    // 7. Concurrent clients
    if effective.concurrent_clients > 0 {
        match harness_concurrent_clients(
            &url,
            effective.short_duration_sec,
            effective.concurrent_clients,
        )
        .await
        {
            Ok(failed) => {
                let pass = failed == 0;
                tests.push(TestResult::Metric {
                    name: "harness_concurrent_clients".to_string(),
                    value: serde_json::json!({ "requested": effective.concurrent_clients, "failed": failed }),
                    pass,
                });
            }
            Err(e) => {
                tests.push(TestResult::Fail {
                    name: "harness_concurrent_clients".to_string(),
                    reason: e.to_string(),
                });
            }
        }
    }

    // 8. Long duration (optional)
    if args.long_duration {
        match harness_long_duration(&url, effective.long_duration_sec).await {
            Ok(degradation_pct) => {
                let pass = degradation_pct < 20;
                tests.push(TestResult::Metric {
                    name: "harness_long_duration_degradation_pct".to_string(),
                    value: serde_json::json!(degradation_pct),
                    pass,
                });
            }
            Err(e) => {
                tests.push(TestResult::Fail {
                    name: "harness_long_duration".to_string(),
                    reason: e.to_string(),
                });
            }
        }
    }

    // 9. Error handling (optional)
    if !args.skip_error_handling {
        match harness_error_handling(
            &effective.rtsp_host,
            effective.rtsp_port,
            &effective.rtsp_stream,
            timeout_sec,
        )
        .await
        {
            Ok((invalid_creds_ok, bogus_url_ok)) => {
                tests.push(TestResult::Metric {
                    name: "harness_error_invalid_creds".to_string(),
                    value: serde_json::json!(invalid_creds_ok),
                    pass: invalid_creds_ok,
                });
                tests.push(TestResult::Metric {
                    name: "harness_error_bogus_url".to_string(),
                    value: serde_json::json!(bogus_url_ok),
                    pass: bogus_url_ok,
                });
            }
            Err(e) => {
                tests.push(TestResult::Fail {
                    name: "harness_error_handling".to_string(),
                    reason: e.to_string(),
                });
            }
        }
    }

    Ok(())
}

async fn harness_basic_connectivity(url: &str, _timeout_sec: u64) -> Result<bool> {
    let url = url.to_string();
    let ok = tokio::task::spawn_blocking(move || {
        let mut cmd = FfmpegCommand::new();
        cmd.hide_banner()
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
            if let FfmpegEvent::Log(_, msg) = &event
                && msg.contains("Stream #")
            {
                saw_stream = true;
                break;
            }
            if matches!(event, FfmpegEvent::Progress(_) | FfmpegEvent::Done) {
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
) -> Result<Option<u64>> {
    let url = url.to_string();
    let ms = tokio::task::spawn_blocking(move || {
        let start = std::time::Instant::now();
        let mut cmd = FfmpegCommand::new();
        cmd.hide_banner()
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
    _effective: &EffectiveConfig,
) -> Result<(f64, f64)> {
    let url = url.to_string();
    let dur = duration_sec;
    let (bitrate, fps) = tokio::task::spawn_blocking(move || {
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
        let mut last_bitrate = 0.0_f64;
        let mut last_fps = 0.0_f64;
        for event in iter {
            if let FfmpegEvent::Progress(FfmpegProgress {
                bitrate_kbps: b,
                fps: f,
                ..
            }) = &event
            {
                last_bitrate = *b as f64;
                last_fps = *f as f64;
            }
        }
        Ok::<_, anyhow::Error>((last_bitrate, last_fps))
    })
    .await
    .context("spawn_blocking")??;
    Ok((bitrate, fps))
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

async fn harness_sdp_validation(url: &str, timeout_sec: u64) -> Result<(usize, usize, bool)> {
    let url = url.to_string();
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
            .stderr(Stdio::null())
            .output()
            .context("ffprobe spawn")?;
        if !out.status.success() {
            bail!("ffprobe failed: {}", String::from_utf8_lossy(&out.stderr));
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
    let _ = timeout_sec;
    Ok(result)
}

async fn harness_rtsp_protocol_sequence(
    url: &str,
    effective: &EffectiveConfig,
    _args: &Args,
) -> Result<(u32, u32, u32, u32, u32, u32)> {
    let pcap_path = std::env::temp_dir().join(format!("rtsp_capture_{}.pcap", std::process::id()));
    let iface = effective.capture_interface.clone();
    let port = effective.rtsp_port;
    let url = url.to_string();

    let pcap_str = pcap_path.to_str().unwrap_or("/tmp/rtsp.pcap").to_string();
    let mut tshark_handle = Command::new("tshark")
        .args([
            "-i",
            &iface,
            "-f",
            &format!("tcp port {}", port),
            "-w",
            &pcap_str,
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("spawn tshark")?;

    tokio::time::sleep(Duration::from_secs(1)).await;

    let url2 = url.clone();
    let short_dur = effective.short_duration_sec;
    let ffmpeg_handle = tokio::task::spawn_blocking(move || {
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
        for _ in iter {}
        Ok::<_, anyhow::Error>(())
    });
    ffmpeg_handle.await.context("ffmpeg join")??;

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
            let _ = std::fs::remove_file(&pcap_path_str);
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
    let pcap_path = std::env::temp_dir().join(format!("rtp_capture_{}.pcap", std::process::id()));
    let iface = effective.capture_interface.clone();
    let url = url.to_string();
    let _duration = effective.short_duration_sec + 5;

    let filter = if effective.rtsp_host.parse::<std::net::IpAddr>().is_ok() {
        format!("udp and host {}", effective.rtsp_host)
    } else {
        "udp".to_string()
    };

    let pcap_str_rtp = pcap_path.to_str().unwrap_or("/tmp/rtp.pcap").to_string();
    let mut tshark_handle_rtp = Command::new("tshark")
        .args(["-i", &iface, "-f", &filter, "-w", &pcap_str_rtp])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("spawn tshark")?;

    tokio::time::sleep(Duration::from_secs(1)).await;

    let url2 = url.clone();
    let short_dur_rtp = effective.short_duration_sec;
    tokio::task::spawn_blocking(move || {
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
        for _ in iter {}
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
        let _ = std::fs::remove_file(&pcap_path_str);
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

async fn harness_concurrent_clients(url: &str, duration_sec: u64, count: u32) -> Result<u32> {
    let mut handles = Vec::new();
    for _ in 0..count {
        let url = url.to_string();
        let dur = duration_sec;
        handles.push(tokio::task::spawn_blocking(move || {
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
            for _ in iter {}
            Ok::<_, anyhow::Error>(())
        }));
    }
    let mut failed = 0u32;
    for h in handles {
        if h.await.context("join")?.is_err() {
            failed += 1;
        }
    }
    Ok(failed)
}

async fn harness_long_duration(url: &str, long_duration_sec: u64) -> Result<u32> {
    let url = url.to_string();
    let dur = long_duration_sec;
    let degradation = tokio::task::spawn_blocking(move || {
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
        for event in iter {
            if let FfmpegEvent::Progress(FfmpegProgress {
                bitrate_kbps: b, ..
            }) = &event
            {
                if first_bitrate.is_none() {
                    first_bitrate = Some(*b);
                }
                last_bitrate = Some(*b);
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
) -> Result<(bool, bool)> {
    let invalid_url = format!("rtsp://invalid:invalid@{}:{}{}", host, port, stream);
    let bogus_url = format!("rtsp://{}:{}/bogus_stream", host, port);

    let invalid_ok = tokio::task::spawn_blocking(move || {
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

    let bogus_ok = tokio::task::spawn_blocking(move || {
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

/// Collect (name, value) for baseline from device telemetry.
fn telemetry_baseline_metrics(t: &DeviceTelemetry) -> Vec<(String, f64)> {
    let mut out = Vec::new();
    if let Some(v) = t.mem_total_kib {
        out.push(("telemetry_mem_total_kib".to_string(), v as f64));
    }
    if let Some(v) = t.mem_free_kib {
        out.push(("telemetry_mem_free_kib".to_string(), v as f64));
    }
    if let Some(v) = t.mem_available_kib {
        out.push(("telemetry_mem_available_kib".to_string(), v as f64));
    }
    if let Some(v) = t.load_avg_1m {
        out.push(("telemetry_load_avg_1m".to_string(), v));
    }
    if let Some(v) = t.load_avg_5m {
        out.push(("telemetry_load_avg_5m".to_string(), v));
    }
    if let Some(v) = t.load_avg_15m {
        out.push(("telemetry_load_avg_15m".to_string(), v));
    }
    if let Some(v) = t.onvif_rss_kib {
        out.push(("telemetry_onvif_rss_kib".to_string(), v as f64));
    }
    if let Some(v) = t.onvif_vmsize_kib {
        out.push(("telemetry_onvif_vmsize_kib".to_string(), v as f64));
    }
    out
}

fn apply_baseline_ops(
    args: &Args,
    effective: &EffectiveConfig,
    report: &mut ValidationReport,
) -> Result<()> {
    let baseline_dir = &effective.baseline_dir;
    let tests = &mut report.tests;
    let mut metrics: Vec<(String, f64)> = Vec::new();
    for t in tests.iter() {
        if let TestResult::Metric { name, value, .. } = t {
            let v = match name.as_str() {
                "harness_startup_latency_ms" => value.as_f64(),
                "harness_bitrate_kbps" => value.as_f64(),
                "harness_fps" => value.as_f64(),
                "harness_packet_loss_percent" => value
                    .get("loss_percent")
                    .and_then(serde_json::Value::as_f64),
                _ => None,
            };
            if let Some(f) = v {
                metrics.push((name.clone(), f));
            }
        }
    }
    if let Some(ref telemetry) = report.telemetry {
        metrics.extend(telemetry_baseline_metrics(telemetry));
    }
    for (name, value) in metrics {
        if args.update_baseline {
            let dir = baseline_direction_for(&name);
            update_baseline(baseline_dir, &name, value, dir)?;
        }
        if args.compare_baseline {
            let dir = baseline_direction_for(&name);
            let within = compare_against_baseline(baseline_dir, &name, value, Some(dir))?;
            if !within {
                tests.push(TestResult::Fail {
                    name: format!("baseline_regression_{}", name),
                    reason: format!("{} value {} exceeds baseline tolerance", name, value),
                });
            }
        }
    }
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
    if args.launch_on_device && !args.no_launch {
        bail!(
            "when using --launch-on-device, also pass --no-launch (server is started on device, not locally)"
        );
    }
    if !args.no_launch && !args.launch_on_device && args.h264_file.is_none() {
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
    if args.no_launch || args.launch_on_device {
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

// -----------------------------------------------------------------------------
// Device control via telnet (--launch-on-device)
// -----------------------------------------------------------------------------

const DEVICE_ONVIF_DIR: &str = "/mnt/anyka_hack/onvif";
const DEVICE_TELNET_CONNECT_TIMEOUT_SEC: u64 = 15;
const DEVICE_TELNET_READ_TIMEOUT_SEC: u64 = 8;

/// Run a single command on the device via telnet (blocking). Returns accumulated output.
fn run_telnet_command_blocking(
    host: &str,
    port: u16,
    command: &str,
    read_timeout_sec: u64,
) -> Result<String> {
    let addr = (host, port)
        .to_socket_addrs()
        .context("resolve device address")?
        .next()
        .ok_or_else(|| anyhow!("no address for {}:{}", host, port))?;

    let mut telnet = Telnet::connect_timeout(
        &addr,
        4096,
        Duration::from_secs(DEVICE_TELNET_CONNECT_TIMEOUT_SEC),
    )
    .with_context(|| format!("telnet connect to {}:{}", host, port))?;

    let cmd_line = format!("{}\n", command);
    telnet
        .write(cmd_line.as_bytes())
        .context("telnet write command")?;

    let timeout_dur = Duration::from_secs(read_timeout_sec);
    let mut out = Vec::new();
    loop {
        let event = telnet.read_timeout(timeout_dur).context("telnet read")?;
        match event {
            Event::Data(buf) => out.extend_from_slice(&buf),
            Event::TimedOut => break,
            _ => {}
        }
    }
    let s = String::from_utf8_lossy(&out).into_owned();
    Ok(s)
}

/// Start onvif-rust on the device (blocking).
fn device_start_onvif_blocking(host: &str, port: u16) -> Result<()> {
    let cmd = format!(
        "cd {} && nohup ./onvif-rust {}/config.toml &",
        DEVICE_ONVIF_DIR, DEVICE_ONVIF_DIR
    );
    run_telnet_command_blocking(host, port, &cmd, DEVICE_TELNET_READ_TIMEOUT_SEC)?;
    Ok(())
}

/// Stop onvif-rust on the device (blocking).
fn device_stop_onvif_blocking(host: &str, port: u16) -> Result<()> {
    run_telnet_command_blocking(
        host,
        port,
        "pkill -f onvif-rust",
        DEVICE_TELNET_READ_TIMEOUT_SEC,
    )?;
    Ok(())
}

/// Collect device telemetry (blocking). Parses /proc/meminfo, /proc/loadavg, pgrep, /proc/PID/status.
fn device_collect_telemetry_blocking(host: &str, port: u16) -> DeviceTelemetry {
    let mut t = DeviceTelemetry::default();

    let meminfo = match run_telnet_command_blocking(
        host,
        port,
        "cat /proc/meminfo",
        DEVICE_TELNET_READ_TIMEOUT_SEC,
    ) {
        Ok(s) => s,
        Err(e) => {
            t.error = Some(format!("meminfo: {}", e));
            return t;
        }
    };
    for line in meminfo.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let key = parts[0].trim_end_matches(':');
            let value: u64 = match parts[1].parse() {
                Ok(v) => v,
                Err(_) => continue,
            };
            match key {
                "MemTotal" => t.mem_total_kib = Some(value),
                "MemFree" => t.mem_free_kib = Some(value),
                "MemAvailable" => t.mem_available_kib = Some(value),
                _ => {}
            }
        }
    }

    let loadavg = match run_telnet_command_blocking(
        host,
        port,
        "cat /proc/loadavg",
        DEVICE_TELNET_READ_TIMEOUT_SEC,
    ) {
        Ok(s) => s,
        Err(e) => {
            t.error = Some(format!("loadavg: {}", e));
            return t;
        }
    };
    let load_parts: Vec<&str> = loadavg.split_whitespace().take(3).collect();
    if load_parts.len() >= 3 {
        t.load_avg_1m = load_parts[0].parse().ok();
        t.load_avg_5m = load_parts[1].parse().ok();
        t.load_avg_15m = load_parts[2].parse().ok();
    }

    let pgrep_out = match run_telnet_command_blocking(
        host,
        port,
        "pgrep -f onvif-rust",
        DEVICE_TELNET_READ_TIMEOUT_SEC,
    ) {
        Ok(s) => s,
        Err(e) => {
            t.error = t.error.or_else(|| Some(format!("pgrep: {}", e)));
            return t;
        }
    };
    let pid_str = pgrep_out.lines().next().and_then(|l| l.trim().parse().ok());
    let pid = match pid_str {
        Some(p) => p,
        None => return t,
    };
    t.onvif_pid = Some(pid);

    let status_cmd = format!("cat /proc/{}/status 2>/dev/null", pid);
    let status = match run_telnet_command_blocking(
        host,
        port,
        &status_cmd,
        DEVICE_TELNET_READ_TIMEOUT_SEC,
    ) {
        Ok(s) => s,
        Err(e) => {
            t.error = t.error.or_else(|| Some(format!("status: {}", e)));
            return t;
        }
    };
    for line in status.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let key = parts[0].trim_end_matches(':');
            let value: u64 = match parts[1].parse() {
                Ok(v) => v,
                Err(_) => continue,
            };
            match key {
                "VmRSS" => t.onvif_rss_kib = Some(value),
                "VmSize" => t.onvif_vmsize_kib = Some(value),
                _ => {}
            }
        }
    }
    t
}

async fn run_validation(args: &Args, effective: &EffectiveConfig) -> Result<ValidationReport> {
    let timestamp = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let test_run = TestRun {
        timestamp,
        rtsp_host: effective.rtsp_host.clone(),
        rtsp_port: effective.rtsp_port,
        rtsp_stream: effective.rtsp_stream.clone(),
        test_duration_seconds: args.duration,
    };

    let mut tests: Vec<TestResult> = Vec::new();

    let url_str = format!(
        "rtsp://{}:{}{}",
        effective.rtsp_host, effective.rtsp_port, effective.rtsp_stream
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
    tests.push(if !has_video || video_is_h264 {
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
                        i, s.media, s.encoding_name, e
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
        telemetry: None,
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
        telemetry: None,
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
