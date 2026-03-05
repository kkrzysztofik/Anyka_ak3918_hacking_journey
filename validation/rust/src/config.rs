//! Configuration (TOML + CLI) and effective merged config.

use anyhow::{Context, Result};
use clap::parser::ValueSource;
use clap::{CommandFactory, FromArgMatches, Parser, ValueEnum};
use serde::Deserialize;
use std::env;
use std::ffi::OsString;
use std::path::PathBuf;
use tracing::{info, warn};

pub const DEFAULT_VIDEO_STARTUP_TARGET_MS: u64 = 1500;
pub const DEFAULT_RTSP_HOST: &str = "127.0.0.1";
pub const DEFAULT_RTSP_PORT: u16 = 554;
pub const DEFAULT_RTSP_STREAM: &str = "/stream1";
pub const DEFAULT_DURATION_SEC: u64 = 30;
pub const DEFAULT_LONG_DURATION_SEC: u64 = 600;
pub const DEFAULT_CONCURRENT_CLIENTS: u32 = 4;
pub const DEFAULT_BASELINE_DIR: &str = "rtsp_results/baselines";
pub const DEFAULT_ARTIFACTS_ROOT_DIR: &str = "rtsp_results/runs";
pub const DEFAULT_HTTPFLV_PORT: u16 = 8080;
pub const DEFAULT_HTTPFLV_PATH: &str = "/live/stream1.flv";
pub const DEFAULT_CONFIG_FILE_NAME: &str = "rtsp_validation.toml";

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
    pub artifacts: ArtifactsSection,
    #[serde(default)]
    pub device: DeviceSection,
    #[serde(default)]
    pub run: RunSection,
    /// Optional HTTP-FLV section for future tests.
    #[serde(default)]
    pub httpflv: Option<HttpFlvSection>,
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct HttpFlvSection {
    #[serde(default = "default_httpflv_port")]
    pub port: u16,
    #[serde(default = "default_httpflv_path")]
    pub path: String,
    #[serde(default)]
    pub timeout_sec: u64,
}

fn default_httpflv_port() -> u16 {
    DEFAULT_HTTPFLV_PORT
}
fn default_httpflv_path() -> String {
    DEFAULT_HTTPFLV_PATH.to_string()
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct RunSection {
    #[serde(default)]
    pub no_launch: bool,
    #[serde(default)]
    pub launch_on_device: bool,
    #[serde(default)]
    pub h264_file: Option<String>,
    #[serde(default)]
    pub output: Option<String>,
    #[serde(default)]
    pub update_baseline: bool,
    #[serde(default)]
    pub compare_baseline: bool,
}

#[derive(Debug, Clone, Deserialize)]
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
    #[serde(default = "default_initial_timestamp_policy")]
    pub initial_timestamp_policy: InitialTimestampPolicyArg,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
}

impl Default for RtspSection {
    fn default() -> Self {
        Self {
            host: default_rtsp_host(),
            port: default_rtsp_port(),
            stream: default_rtsp_stream(),
            timeout_sec: default_rtsp_timeout(),
            initial_timestamp_policy: default_initial_timestamp_policy(),
            username: None,
            password: None,
        }
    }
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

fn default_initial_timestamp_policy() -> InitialTimestampPolicyArg {
    InitialTimestampPolicyArg::Permissive
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct TestSection {
    #[serde(default = "default_short_duration")]
    pub short_duration_sec: u64,
    #[serde(default = "default_long_duration")]
    pub long_duration_sec: u64,
    #[serde(default = "default_concurrent_clients")]
    pub concurrent_clients: u32,
}

impl Default for TestSection {
    fn default() -> Self {
        Self {
            short_duration_sec: default_short_duration(),
            long_duration_sec: default_long_duration(),
            concurrent_clients: default_concurrent_clients(),
        }
    }
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

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ThresholdsSection {
    #[serde(default = "default_video_startup_latency_ms")]
    pub video_startup_latency_ms: u64,
    #[serde(default = "default_harness_startup_latency_ms")]
    pub harness_startup_latency_ms: u64,
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

impl Default for ThresholdsSection {
    fn default() -> Self {
        Self {
            video_startup_latency_ms: default_video_startup_latency_ms(),
            harness_startup_latency_ms: default_harness_startup_latency_ms(),
            audio_startup_latency_ms: 0,
            bitrate_tolerance_percent: default_bitrate_tolerance(),
            fps_tolerance_percent: default_fps_tolerance(),
            packet_loss_tolerance_percent: default_packet_loss_tolerance(),
            expected: None,
        }
    }
}

fn default_video_startup_latency_ms() -> u64 {
    1500
}
fn default_harness_startup_latency_ms() -> u64 {
    3000
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

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct BaselineSection {
    #[serde(default = "default_baseline_dir")]
    pub dir: String,
}

impl Default for BaselineSection {
    fn default() -> Self {
        Self {
            dir: default_baseline_dir(),
        }
    }
}

fn default_baseline_dir() -> String {
    DEFAULT_BASELINE_DIR.to_string()
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct CaptureSection {
    pub interface: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ArtifactsSection {
    #[serde(default = "default_artifacts_root_dir")]
    pub dir: String,
    #[serde(default = "default_true")]
    pub capture_tool_output: bool,
    #[serde(default = "default_true")]
    pub keep_pcaps: bool,
}

impl Default for ArtifactsSection {
    fn default() -> Self {
        Self {
            dir: default_artifacts_root_dir(),
            capture_tool_output: true,
            keep_pcaps: true,
        }
    }
}

fn default_artifacts_root_dir() -> String {
    DEFAULT_ARTIFACTS_ROOT_DIR.to_string()
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct LoggingSection {
    #[serde(default)]
    pub level: String,
    #[serde(default)]
    pub retina_level: String,
    #[serde(default)]
    pub file: String,
    #[serde(default)]
    pub ffmpeg_level: String,
}

/// A stream to validate (RTSP + HTTP-FLV paths).
#[derive(Debug, Clone)]
pub struct StreamConfig {
    /// Human-readable label (e.g. "main", "sub").
    pub label: String,
    /// RTSP stream path (e.g. "/main").
    pub rtsp_stream: String,
    /// HTTP-FLV path (e.g. "/live/main.flv").
    pub httpflv_path: String,
}

/// TOML `[[device.streams]]` array entry.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct DeviceStreamSection {
    pub label: String,
    pub rtsp_stream: String,
    pub httpflv_path: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct DeviceSection {
    #[serde(default = "default_device_host")]
    pub host: String,
    #[serde(default = "default_device_ssh_port")]
    pub ssh_port: u16,
    #[serde(default = "default_device_user")]
    pub user: String,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default = "default_telemetry_enabled")]
    pub telemetry: bool,
    #[serde(default)]
    pub h264_file: Option<String>,
    #[serde(default)]
    pub aac_file: Option<String>,
    #[serde(default)]
    pub loop_playback: bool,
    /// Launch the real camera pipeline (no validation-mode, no H.264/AAC files).
    #[serde(default)]
    pub real_mode: bool,
    /// Streams to validate in real mode.
    #[serde(default)]
    pub streams: Vec<DeviceStreamSection>,
}

impl Default for DeviceSection {
    fn default() -> Self {
        Self {
            host: default_device_host(),
            ssh_port: default_device_ssh_port(),
            user: default_device_user(),
            password: None,
            telemetry: default_telemetry_enabled(),
            h264_file: None,
            aac_file: None,
            loop_playback: false,
            real_mode: false,
            streams: Vec::new(),
        }
    }
}

fn default_device_host() -> String {
    "192.168.2.198".to_string()
}
fn default_device_ssh_port() -> u16 {
    22
}
fn default_device_user() -> String {
    "root".to_string()
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
    pub stream_username: Option<String>,
    pub stream_password: Option<String>,
    pub rtsp_timeout_sec: u64,
    pub initial_timestamp_policy: InitialTimestampPolicyArg,
    pub short_duration_sec: u64,
    pub long_duration_sec: u64,
    pub concurrent_clients: u32,
    pub video_startup_latency_ms: u64,
    pub harness_startup_latency_ms: u64,
    pub audio_startup_latency_ms: u64,
    pub bitrate_tolerance_percent: u32,
    pub fps_tolerance_percent: u32,
    pub packet_loss_tolerance_percent: f64,
    pub expected_bitrate_kbps: Option<f64>,
    pub expected_fps: Option<f64>,
    pub baseline_dir: PathBuf,
    pub capture_interface: String,
    pub artifacts_root_dir: PathBuf,
    pub artifacts_dir: PathBuf,
    pub capture_tool_output: bool,
    pub keep_pcaps: bool,
    pub launch_on_device: bool,
    pub device_host: String,
    pub device_ssh_port: u16,
    pub device_user: String,
    pub device_password: Option<String>,
    pub collect_telemetry: bool,
    pub device_h264_file: Option<String>,
    pub device_aac_file: Option<String>,
    pub device_loop_playback: bool,
    /// True when running the real camera pipeline (no validation-mode).
    pub device_real_mode: bool,
    /// Streams to validate (1 in normal mode, 2+ in real mode).
    pub streams: Vec<StreamConfig>,
    pub no_launch: bool,
    pub h264_file: Option<String>,
    pub output: String,
    pub update_baseline: bool,
    pub compare_baseline: bool,
    pub ffmpeg_log_level: String,
    /// HTTP-FLV port (for future tests).
    pub httpflv_port: u16,
    /// HTTP-FLV path (for future tests).
    pub httpflv_path: String,
    pub httpflv_timeout_sec: u64,
}

impl EffectiveConfig {
    fn resolve_device_auth(
        args: &Args,
        c: &RtspValidationConfig,
        env_user: Option<String>,
        env_password: Option<String>,
    ) -> (String, Option<String>) {
        let device_user = args
            .device_user
            .clone()
            .or(env_user)
            .or_else(|| Some(c.device.user.clone()))
            .filter(|u| !u.is_empty())
            .unwrap_or_else(default_device_user);
        let device_password = args
            .device_password
            .clone()
            .or(env_password)
            .or_else(|| c.device.password.clone())
            .filter(|p| !p.is_empty());
        (device_user, device_password)
    }

    fn resolve_stream_auth(
        args: &Args,
        c: &RtspValidationConfig,
    ) -> (Option<String>, Option<String>) {
        let username = args
            .username
            .clone()
            .or_else(|| c.rtsp.username.clone())
            .filter(|u| !u.is_empty());
        let password = args
            .password
            .clone()
            .or_else(|| c.rtsp.password.clone())
            .filter(|p| !p.is_empty());

        if username.is_some() != password.is_some() {
            warn!(
                "RTSP credentials are incomplete (username/password must both be set); ignoring partial credentials"
            );
            return (None, None);
        }

        (username, password)
    }

    /// Resolve effective FFmpeg log level from CLI and config.
    pub fn ffmpeg_log_level_from_config(
        config: Option<&RtspValidationConfig>,
        cli_level: Option<&str>,
    ) -> String {
        if let Some(cli) = cli_level {
            let trimmed = cli.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
        if let Some(c) = config {
            let trimmed = c.logging.ffmpeg_level.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
        "verbose".to_string()
    }

    /// Build effective config from config file and CLI.
    ///
    /// Precedence is:
    /// - For options: CLI values explicitly provided by the user win over config.
    /// - For flags: `true` on either side wins (cannot be forced to `false` from CLI).
    pub fn from_config_and_args(
        config: Option<&RtspValidationConfig>,
        args: &Args,
        sources: &ArgSources,
    ) -> Self {
        let default_config = RtspValidationConfig::default();
        let c = config.unwrap_or(&default_config);
        let baseline_dir = if c.baseline.dir.is_empty() {
            PathBuf::from(DEFAULT_BASELINE_DIR)
        } else {
            PathBuf::from(&c.baseline.dir)
        };
        let capture_interface = c.capture.interface.clone().unwrap_or_default();
        let artifacts_root_dir = args
            .artifacts_dir
            .clone()
            .or_else(|| Some(c.artifacts.dir.clone()))
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| DEFAULT_ARTIFACTS_ROOT_DIR.to_string());
        let capture_tool_output = c.artifacts.capture_tool_output;
        let keep_pcaps = c.artifacts.keep_pcaps;
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
        let device_ssh_port = args
            .device_ssh_port
            .or(Some(c.device.ssh_port))
            .filter(|&p| p != 0)
            .unwrap_or(22);
        let (device_user, device_password) = Self::resolve_device_auth(
            args,
            c,
            env::var("RTSP_VALIDATION_DEVICE_USER").ok(),
            env::var("RTSP_VALIDATION_DEVICE_PASSWORD").ok(),
        );
        let launch_on_device = args.launch_on_device || c.run.launch_on_device;
        let no_launch = args.no_launch || c.run.no_launch || launch_on_device;
        let collect_telemetry = launch_on_device && !args.no_telemetry && c.device.telemetry;
        let device_h264_file = args
            .device_h264_file
            .clone()
            .or_else(|| c.device.h264_file.clone())
            .filter(|s| !s.is_empty());
        let device_aac_file = args
            .device_aac_file
            .clone()
            .or_else(|| c.device.aac_file.clone())
            .filter(|s| !s.is_empty());
        let device_loop_playback = args.device_loop_playback || c.device.loop_playback;
        let device_real_mode = args.device_real_mode || c.device.real_mode;
        // In real mode, force h264/aac files to None (uses camera sensor).
        let (device_h264_file, device_aac_file) = if device_real_mode {
            (None, None)
        } else {
            (device_h264_file, device_aac_file)
        };
        let ffmpeg_log_level =
            Self::ffmpeg_log_level_from_config(Some(c), args.ffmpeg_log_level.as_deref());
        let rtsp_host = if launch_on_device {
            device_host.clone()
        } else if sources.rtsp_host {
            args.rtsp_host.clone()
        } else if !c.rtsp.host.is_empty() {
            c.rtsp.host.clone()
        } else {
            args.rtsp_host.clone()
        };
        let rtsp_port = if sources.rtsp_port {
            args.rtsp_port
        } else if c.rtsp.port != 0 {
            c.rtsp.port
        } else {
            args.rtsp_port
        };
        let rtsp_stream = if sources.rtsp_stream {
            args.rtsp_stream.clone()
        } else if !c.rtsp.stream.is_empty() {
            c.rtsp.stream.clone()
        } else {
            args.rtsp_stream.clone()
        };
        let initial_timestamp_policy = if sources.initial_timestamp_policy {
            args.initial_timestamp_policy
        } else {
            c.rtsp.initial_timestamp_policy
        };
        let (stream_username, stream_password) = Self::resolve_stream_auth(args, c);
        let h264_file = args
            .h264_file
            .clone()
            .or_else(|| c.run.h264_file.clone())
            .filter(|s| !s.is_empty());
        let output = if sources.output {
            args.output.clone()
        } else {
            c.run
                .output
                .clone()
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| args.output.clone())
        };
        let update_baseline = c.run.update_baseline || args.update_baseline;
        let compare_baseline = c.run.compare_baseline || args.compare_baseline;

        let (httpflv_port, httpflv_path, httpflv_timeout_sec) = if let Some(hf) = &c.httpflv {
            (
                if sources.httpflv_port {
                    args.httpflv_port
                } else {
                    hf.port
                },
                args.httpflv_stream
                    .clone()
                    .unwrap_or_else(|| hf.path.clone()),
                hf.timeout_sec,
            )
        } else {
            (
                if sources.httpflv_port {
                    args.httpflv_port
                } else {
                    DEFAULT_HTTPFLV_PORT
                },
                args.httpflv_stream
                    .clone()
                    .unwrap_or_else(|| DEFAULT_HTTPFLV_PATH.to_string()),
                10, // default timeout
            )
        };

        // Build streams list.
        let streams = if !c.device.streams.is_empty() {
            // Explicit [[device.streams]] from TOML config.
            c.device
                .streams
                .iter()
                .map(|s| StreamConfig {
                    label: s.label.clone(),
                    rtsp_stream: s.rtsp_stream.clone(),
                    httpflv_path: s.httpflv_path.clone(),
                })
                .collect()
        } else if device_real_mode && !sources.rtsp_stream {
            // Real mode with no explicit --rtsp-stream: default to main + sub.
            vec![
                StreamConfig {
                    label: "main".to_string(),
                    rtsp_stream: "/main".to_string(),
                    httpflv_path: "/live/main.flv".to_string(),
                },
                StreamConfig {
                    label: "sub".to_string(),
                    rtsp_stream: "/sub".to_string(),
                    httpflv_path: "/live/sub.flv".to_string(),
                },
            ]
        } else {
            // Single stream (backward compatible).
            vec![StreamConfig {
                label: String::new(),
                rtsp_stream: rtsp_stream.clone(),
                httpflv_path: httpflv_path.clone(),
            }]
        };

        // First stream becomes the primary for backward compatibility.
        let rtsp_stream = streams
            .first()
            .map(|s| s.rtsp_stream.clone())
            .unwrap_or(rtsp_stream);
        let httpflv_path = streams
            .first()
            .map(|s| s.httpflv_path.clone())
            .unwrap_or(httpflv_path);

        Self {
            rtsp_host,
            rtsp_port,
            rtsp_stream,
            stream_username,
            stream_password,
            rtsp_timeout_sec: c.rtsp.timeout_sec,
            initial_timestamp_policy,
            short_duration_sec: if sources.duration {
                args.duration.max(1)
            } else {
                c.test.short_duration_sec.max(1)
            },
            long_duration_sec: c.test.long_duration_sec,
            concurrent_clients: args.concurrent.unwrap_or(c.test.concurrent_clients),
            video_startup_latency_ms: if sources.max_video_startup_latency_ms {
                args.max_video_startup_latency_ms.max(1)
            } else {
                c.thresholds.video_startup_latency_ms.max(1)
            },
            harness_startup_latency_ms: c.thresholds.harness_startup_latency_ms.max(1),
            audio_startup_latency_ms: c.thresholds.audio_startup_latency_ms,
            bitrate_tolerance_percent: c.thresholds.bitrate_tolerance_percent,
            fps_tolerance_percent: c.thresholds.fps_tolerance_percent,
            packet_loss_tolerance_percent: c.thresholds.packet_loss_tolerance_percent,
            expected_bitrate_kbps: expected_bitrate,
            expected_fps,
            baseline_dir,
            capture_interface,
            artifacts_root_dir: PathBuf::from(artifacts_root_dir),
            artifacts_dir: PathBuf::new(),
            capture_tool_output,
            keep_pcaps,
            launch_on_device,
            device_host,
            device_ssh_port,
            device_user,
            device_password,
            collect_telemetry,
            device_h264_file,
            device_aac_file,
            device_loop_playback,
            device_real_mode,
            streams,
            no_launch,
            h264_file,
            output,
            update_baseline,
            compare_baseline,
            ffmpeg_log_level,
            httpflv_port,
            httpflv_path,
            httpflv_timeout_sec,
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

/// Load config from path.
///
/// Search order (when `path_override` is `None`):
/// - env `RTSP_VALIDATION_CONFIG`
/// - `./rtsp_validation.toml`
/// - `./validation/rtsp_validation.toml`
pub fn load_config(path_override: Option<&str>) -> Result<Option<RtspValidationConfig>> {
    let path = path_override
        .map(PathBuf::from)
        .or_else(|| env::var_os("RTSP_VALIDATION_CONFIG").map(PathBuf::from))
        .or_else(|| {
            let cwd = env::current_dir().ok()?;
            let p = cwd.join(DEFAULT_CONFIG_FILE_NAME);
            if p.is_file() { Some(p) } else { None }
        })
        .or_else(|| {
            let cwd = env::current_dir().ok()?;
            let p = cwd.join("validation").join(DEFAULT_CONFIG_FILE_NAME);
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
#[command(
    name = "rtsp_validation_tool",
    about = "Host-side RTSP validation (Retina + FFmpeg + PCAP)",
    long_about = "Host-side RTSP protocol validation tool for Anyka/onvif-rust.\n\nRuns a mix of:\n- Protocol sequencing checks (DESCRIBE/SETUP/PLAY) via Retina\n- Media sanity checks via ffprobe\n- Startup latency / bitrate / FPS checks via ffmpeg progress\n- PCAP-based RTP/RTSP analysis (tshark + rtshark)\n\nArtifacts are written under the run directory (by default: rtsp_results/runs/<timestamp>_pid<id>/).",
    after_help = "Config search order:\n  1) --config <PATH>\n  2) env RTSP_VALIDATION_CONFIG\n  3) ./rtsp_validation.toml\n  4) ./validation/rtsp_validation.toml\n\nExamples:\n  # Use repo default config (auto-discovered)\n  rtsp_validation_tool\n\n  # Use an explicit config file\n  rtsp_validation_tool --config validation/rtsp_validation.toml\n\n  # Override a config value on the command line\n  rtsp_validation_tool --rtsp-host 192.168.2.198 --rtsp-stream /vs0\n\n  # Launch local onvif-rust in validation-mode with a test H.264 file\n  rtsp_validation_tool --h264-file validation/rtsp_results/test.h264\n",
    next_line_help = true
)]
pub struct Args {
    #[arg(
        long,
        value_name = "PATH",
        help_heading = "Run / Output",
        help = "Path to a test H.264 file used when launching local onvif-rust in --validation-mode."
    )]
    pub h264_file: Option<String>,

    #[arg(
        long,
        default_value = DEFAULT_RTSP_HOST,
        value_name = "HOST",
        help_heading = "RTSP",
        help = "RTSP server host/IP to validate (overrides config when explicitly set)."
    )]
    pub rtsp_host: String,

    #[arg(
        long,
        default_value_t = DEFAULT_RTSP_PORT,
        value_name = "PORT",
        help_heading = "RTSP",
        help = "RTSP server port to validate (overrides config when explicitly set)."
    )]
    pub rtsp_port: u16,

    #[arg(
        long,
        default_value = DEFAULT_RTSP_STREAM,
        value_name = "PATH",
        help_heading = "RTSP",
        help = "RTSP stream path (must start with '/'; overrides config when explicitly set)."
    )]
    pub rtsp_stream: String,

    #[arg(
        long,
        value_name = "USER",
        help_heading = "RTSP",
        help = "RTSP username (use with --password)."
    )]
    pub username: Option<String>,

    #[arg(
        long,
        value_name = "PASS",
        help_heading = "RTSP",
        help = "RTSP password (use with --username)."
    )]
    pub password: Option<String>,

    #[arg(
        long,
        value_enum,
        default_value_t = TransportArg::Tcp,
        help_heading = "RTSP",
        help = "RTSP transport for protocol validation (tcp or udp)."
    )]
    pub transport: TransportArg,

    #[arg(
        long,
        value_enum,
        default_value_t = InitialTimestampPolicyArg::Permissive,
        help_heading = "RTSP",
        help = "Policy for missing RTP-Info rtptime on PLAY (default, require, ignore, permissive)."
    )]
    pub initial_timestamp_policy: InitialTimestampPolicyArg,

    #[arg(
        long,
        default_value_t = DEFAULT_HTTPFLV_PORT,
        value_name = "PORT",
        help_heading = "Run / Output",
        help = "HTTP-FLV port passed to onvif-rust when launching locally (overrides config when explicitly set)."
    )]
    pub httpflv_port: u16,

    #[arg(
        long,
        default_value_t = DEFAULT_DURATION_SEC,
        value_name = "SEC",
        help_heading = "Run / Output",
        help = "Short test duration in seconds (overrides config when explicitly set)."
    )]
    pub duration: u64,

    #[arg(
        long,
        default_value_t = DEFAULT_VIDEO_STARTUP_TARGET_MS,
        value_name = "MS",
        help_heading = "Thresholds",
        help = "Maximum allowed protocol first-frame latency in ms (`first_video_frame_latency_ms`; overrides config when explicitly set)."
    )]
    pub max_video_startup_latency_ms: u64,

    #[arg(
        long,
        default_value = "rtsp_validation.json",
        value_name = "FILE",
        help_heading = "Run / Output",
        help = "Report filename written inside the run artifacts directory."
    )]
    pub output: String,

    #[arg(
        long,
        value_name = "PATH",
        help_heading = "Run / Output",
        help = "Path to the onvif-rust binary to launch locally (default: auto-detected under cross-compile/onvif-rust/target/...)."
    )]
    pub onvif_binary: Option<String>,

    #[arg(
        long,
        help_heading = "Run / Output",
        help = "Do not launch onvif-rust locally (validate an already-running RTSP server)."
    )]
    pub no_launch: bool,

    #[arg(
        long,
        help_heading = "Run / Output",
        help = "Enable loop playback when launching local onvif-rust with --h264-file."
    )]
    pub loop_playback: bool,

    #[arg(
        long,
        help_heading = "Thresholds",
        help = "Fail if audio is missing from SDP/streams (useful when validating A/V pipelines)."
    )]
    pub require_audio: bool,

    #[arg(
        short = 'c',
        long,
        value_name = "PATH",
        help_heading = "Config",
        help = "Use this TOML config file instead of the default search paths."
    )]
    pub config: Option<String>,

    #[arg(
        long,
        value_name = "DIR",
        help_heading = "Run / Output",
        help = "Artifacts root directory (default from config: rtsp_results/runs)."
    )]
    pub artifacts_dir: Option<String>,

    #[arg(
        long,
        value_name = "LEVEL",
        help_heading = "Debug",
        help = "FFmpeg log level (e.g. quiet, error, warning, info, verbose, debug, trace)."
    )]
    pub ffmpeg_log_level: Option<String>,

    #[arg(
        long,
        help_heading = "Baselines",
        help = "Update baseline JSON under rtsp_results/baselines (for regression tracking)."
    )]
    pub update_baseline: bool,

    #[arg(
        long,
        help_heading = "Baselines",
        help = "Compare against baseline JSON and mark deviations as failures."
    )]
    pub compare_baseline: bool,

    #[arg(
        long,
        value_name = "N",
        help_heading = "Run / Output",
        help = "Number of concurrent ffmpeg clients (overrides config)."
    )]
    pub concurrent: Option<u32>,

    #[arg(
        long,
        help_heading = "Run / Output",
        help = "Run the long-duration scenario (uses config test.long_duration_sec)."
    )]
    pub long_duration: bool,

    #[arg(
        long,
        help_heading = "Debug",
        help = "Skip negative/error-handling checks (invalid creds, bogus URLs)."
    )]
    pub skip_error_handling: bool,

    #[arg(
        long,
        help_heading = "Device",
        help = "Launch onvif-rust on the device via SSH (implies --no-launch)."
    )]
    pub launch_on_device: bool,

    #[arg(
        long,
        value_name = "HOST",
        help_heading = "Device",
        help = "Device host/IP for SSH control (default from config)."
    )]
    pub device_host: Option<String>,

    #[arg(
        long,
        value_name = "PORT",
        help_heading = "Device",
        help = "Device SSH port (default from config)."
    )]
    pub device_ssh_port: Option<u16>,

    #[arg(
        long,
        value_name = "USER",
        help_heading = "Device",
        help = "Device SSH username (CLI > env RTSP_VALIDATION_DEVICE_USER > config > root)."
    )]
    pub device_user: Option<String>,

    #[arg(
        long,
        value_name = "PASSWORD",
        help_heading = "Device",
        help = "Device SSH password (CLI > env RTSP_VALIDATION_DEVICE_PASSWORD > config)."
    )]
    pub device_password: Option<String>,

    /// Skip HTTP-FLV validation
    #[arg(long)]
    pub skip_httpflv: bool,

    /// Override HTTP-FLV stream path (e.g. /live/stream1)
    #[arg(long)]
    pub httpflv_stream: Option<String>,

    #[arg(
        long,
        help_heading = "Device",
        help = "Disable device telemetry collection (when --launch-on-device)."
    )]
    pub no_telemetry: bool,

    #[arg(
        long,
        value_name = "PATH",
        help_heading = "Device",
        help = "Device-side H.264 file to play in validation mode (path is on the device)."
    )]
    pub device_h264_file: Option<String>,

    #[arg(
        long,
        value_name = "PATH",
        help_heading = "Device",
        help = "Device-side AAC file to play in validation mode (path is on the device)."
    )]
    pub device_aac_file: Option<String>,

    #[arg(
        long,
        help_heading = "Device",
        help = "Loop device-side playback when using --device-h264-file / --device-aac-file."
    )]
    pub device_loop_playback: bool,

    #[arg(
        long,
        help_heading = "Device",
        help = "Launch onvif-rust on device in real mode (camera sensor, no --validation-mode). Validates both main and sub streams by default."
    )]
    pub device_real_mode: bool,
}

#[derive(ValueEnum, Clone, Copy, Debug)]
pub enum TransportArg {
    Tcp,
    Udp,
}

#[derive(ValueEnum, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum InitialTimestampPolicyArg {
    Default,
    Require,
    Ignore,
    Permissive,
}

#[derive(Debug, Clone, Copy)]
pub struct ArgSources {
    pub rtsp_host: bool,
    pub rtsp_port: bool,
    pub rtsp_stream: bool,
    pub initial_timestamp_policy: bool,
    pub duration: bool,
    pub max_video_startup_latency_ms: bool,
    pub output: bool,
    pub httpflv_port: bool,
}

impl ArgSources {
    pub fn from_matches(matches: &clap::ArgMatches) -> Self {
        fn is_cli(matches: &clap::ArgMatches, id: &str) -> bool {
            matches.value_source(id) == Some(ValueSource::CommandLine)
        }

        Self {
            rtsp_host: is_cli(matches, "rtsp_host"),
            rtsp_port: is_cli(matches, "rtsp_port"),
            rtsp_stream: is_cli(matches, "rtsp_stream"),
            initial_timestamp_policy: is_cli(matches, "initial_timestamp_policy"),
            duration: is_cli(matches, "duration"),
            max_video_startup_latency_ms: is_cli(matches, "max_video_startup_latency_ms"),
            output: is_cli(matches, "output"),
            httpflv_port: is_cli(matches, "httpflv_port"),
        }
    }
}

#[derive(Debug)]
pub struct ParsedArgs {
    pub args: Args,
    pub sources: ArgSources,
}

pub fn parse_args() -> Result<ParsedArgs> {
    let matches = Args::command().get_matches();
    let sources = ArgSources::from_matches(&matches);
    let args = Args::from_arg_matches(&matches).context("decode CLI arguments")?;
    Ok(ParsedArgs { args, sources })
}

pub fn parse_args_from<I, T>(iter: I) -> Result<ParsedArgs>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let cmd = Args::command();
    let matches = cmd
        .try_get_matches_from(iter)
        .context("parse CLI arguments")?;
    let sources = ArgSources::from_matches(&matches);
    let args = Args::from_arg_matches(&matches).context("decode CLI arguments")?;
    Ok(ParsedArgs { args, sources })
}

#[cfg(test)]
mod tests {
    use super::{
        DEFAULT_ARTIFACTS_ROOT_DIR, DEFAULT_BASELINE_DIR, DEFAULT_HTTPFLV_PATH, DEFAULT_RTSP_HOST,
        DEFAULT_RTSP_PORT, DEFAULT_RTSP_STREAM, EffectiveConfig, HttpFlvSection,
        InitialTimestampPolicyArg, LoggingSection, RtspValidationConfig, load_config,
        parse_args_from,
    };
    use clap::CommandFactory;
    use std::path::PathBuf;

    #[test]
    fn test_ffmpeg_log_level_resolution_prefers_cli_then_config_then_default() {
        let mut cfg = RtspValidationConfig::default();
        cfg.logging = LoggingSection {
            level: "info".to_string(),
            retina_level: "".to_string(),
            file: "".to_string(),
            ffmpeg_level: "debug".to_string(),
        };

        assert_eq!(
            EffectiveConfig::ffmpeg_log_level_from_config(Some(&cfg), None),
            "debug".to_string()
        );

        assert_eq!(
            EffectiveConfig::ffmpeg_log_level_from_config(Some(&cfg), Some("trace")),
            "trace".to_string()
        );

        let empty_cfg = RtspValidationConfig::default();
        assert_eq!(
            EffectiveConfig::ffmpeg_log_level_from_config(Some(&empty_cfg), None),
            "verbose".to_string()
        );
    }

    #[test]
    fn test_from_config_and_args_defaults() {
        let parsed = parse_args_from(["rtsp_validation_tool"]).unwrap();
        let effective = EffectiveConfig::from_config_and_args(None, &parsed.args, &parsed.sources);
        assert_eq!(effective.rtsp_host, DEFAULT_RTSP_HOST);
        assert_eq!(effective.rtsp_port, DEFAULT_RTSP_PORT);
        assert_eq!(effective.rtsp_stream, DEFAULT_RTSP_STREAM);
        assert_eq!(
            effective.initial_timestamp_policy,
            InitialTimestampPolicyArg::Permissive
        );
        assert_eq!(effective.short_duration_sec, 30); // DEFAULT_DURATION_SEC
        assert_eq!(effective.harness_startup_latency_ms, 3000);
        assert_eq!(effective.baseline_dir, PathBuf::from(DEFAULT_BASELINE_DIR));
        assert_eq!(
            effective.artifacts_root_dir,
            PathBuf::from(DEFAULT_ARTIFACTS_ROOT_DIR)
        );
        assert!(!effective.launch_on_device);
        assert_eq!(effective.device_ssh_port, 22);
        assert_eq!(effective.device_user, "root");
        assert_eq!(effective.device_password, None);
        assert!(!effective.update_baseline);
        assert!(!effective.compare_baseline);
    }

    #[test]
    fn test_from_config_and_args_cli_overrides() {
        let parsed = parse_args_from([
            "rtsp_validation_tool",
            "--rtsp-host",
            "10.0.0.1",
            "--rtsp-port",
            "8554",
            "--duration",
            "5",
            "--initial-timestamp-policy",
            "require",
            "--artifacts-dir",
            "/tmp/artifacts",
            "--ffmpeg-log-level",
            "warning",
            "--update-baseline",
            "--device-host",
            "192.168.1.1",
            "--device-ssh-port",
            "2222",
            "--device-user",
            "root2",
            "--device-password",
            "pw2",
        ])
        .unwrap();
        let effective = EffectiveConfig::from_config_and_args(None, &parsed.args, &parsed.sources);
        assert_eq!(effective.rtsp_host, "10.0.0.1");
        assert_eq!(effective.rtsp_port, 8554);
        assert_eq!(
            effective.initial_timestamp_policy,
            InitialTimestampPolicyArg::Require
        );
        assert_eq!(effective.short_duration_sec, 5);
        assert_eq!(
            effective.artifacts_root_dir,
            PathBuf::from("/tmp/artifacts")
        );
        assert_eq!(effective.ffmpeg_log_level, "warning");
        assert!(effective.update_baseline);
        assert_eq!(effective.device_host, "192.168.1.1");
        assert_eq!(effective.device_ssh_port, 2222);
        assert_eq!(effective.device_user, "root2");
        assert_eq!(effective.device_password.as_deref(), Some("pw2"));
    }

    #[test]
    fn test_from_config_and_args_device_auth_env_overrides_config() {
        let parsed = parse_args_from(["rtsp_validation_tool"]).unwrap();
        let mut cfg = RtspValidationConfig::default();
        cfg.device.user = "cfg_user".to_string();
        cfg.device.password = Some("cfg_pass".to_string());
        let (user, password) = EffectiveConfig::resolve_device_auth(
            &parsed.args,
            &cfg,
            Some("env_user".to_string()),
            Some("env_pass".to_string()),
        );

        assert_eq!(user, "env_user");
        assert_eq!(password.as_deref(), Some("env_pass"));
    }

    #[test]
    fn test_from_config_and_args_device_auth_cli_overrides_env() {
        let parsed = parse_args_from([
            "rtsp_validation_tool",
            "--device-user",
            "cli_user",
            "--device-password",
            "cli_pass",
        ])
        .unwrap();
        let cfg = RtspValidationConfig::default();
        let (user, password) = EffectiveConfig::resolve_device_auth(
            &parsed.args,
            &cfg,
            Some("env_user".to_string()),
            Some("env_pass".to_string()),
        );

        assert_eq!(user, "cli_user");
        assert_eq!(password.as_deref(), Some("cli_pass"));
    }

    #[test]
    fn test_from_config_and_args_launch_on_device_uses_device_host() {
        let parsed = parse_args_from([
            "rtsp_validation_tool",
            "--launch-on-device",
            "--device-host",
            "192.168.2.100",
        ])
        .unwrap();
        let effective = EffectiveConfig::from_config_and_args(None, &parsed.args, &parsed.sources);
        assert!(effective.launch_on_device);
        assert_eq!(effective.rtsp_host, "192.168.2.100");
        assert_eq!(effective.device_host, "192.168.2.100");
    }

    #[test]
    fn test_resolve_capture_interface_non_empty_unchanged() {
        let parsed = parse_args_from(["rtsp_validation_tool"]).unwrap();
        let mut effective =
            EffectiveConfig::from_config_and_args(None, &parsed.args, &parsed.sources);
        effective.capture_interface = "eth0".to_string();
        effective.resolve_capture_interface();
        assert_eq!(effective.capture_interface, "eth0");
    }

    #[test]
    fn test_resolve_capture_interface_empty_localhost_lo() {
        let parsed = parse_args_from(["rtsp_validation_tool", "--rtsp-host", "127.0.0.1"]).unwrap();
        let mut effective =
            EffectiveConfig::from_config_and_args(None, &parsed.args, &parsed.sources);
        effective.capture_interface = String::new();
        effective.resolve_capture_interface();
        assert_eq!(effective.capture_interface, "lo");
    }

    #[test]
    fn test_resolve_capture_interface_empty_localhost_string() {
        let parsed = parse_args_from(["rtsp_validation_tool"]).unwrap();
        let mut cfg = RtspValidationConfig::default();
        cfg.rtsp.host = "localhost".to_string();
        let mut effective =
            EffectiveConfig::from_config_and_args(Some(&cfg), &parsed.args, &parsed.sources);
        effective.capture_interface = String::new();
        effective.resolve_capture_interface();
        assert_eq!(effective.capture_interface, "lo");
    }

    #[test]
    fn test_resolve_capture_interface_empty_ipv6_localhost_lo() {
        let parsed = parse_args_from(["rtsp_validation_tool"]).unwrap();
        let mut cfg = RtspValidationConfig::default();
        cfg.rtsp.host = "::1".to_string();
        let mut effective =
            EffectiveConfig::from_config_and_args(Some(&cfg), &parsed.args, &parsed.sources);
        effective.capture_interface = String::new();
        effective.resolve_capture_interface();
        assert_eq!(effective.capture_interface, "lo");
    }

    #[test]
    fn test_resolve_capture_interface_empty_remote_any() {
        let parsed = parse_args_from(["rtsp_validation_tool"]).unwrap();
        let mut cfg = RtspValidationConfig::default();
        cfg.rtsp.host = "10.0.0.1".to_string();
        let mut effective =
            EffectiveConfig::from_config_and_args(Some(&cfg), &parsed.args, &parsed.sources);
        effective.capture_interface = String::new();
        effective.resolve_capture_interface();
        assert_eq!(effective.capture_interface, "any");
    }

    #[test]
    fn test_from_config_and_args_cli_overrides_config_when_explicit() {
        let parsed = parse_args_from([
            "rtsp_validation_tool",
            "--rtsp-host",
            "10.0.0.2",
            "--rtsp-port",
            "8554",
        ])
        .unwrap();
        let mut cfg = RtspValidationConfig::default();
        cfg.rtsp.host = "192.168.2.198".to_string();
        cfg.rtsp.port = 554;
        let effective =
            EffectiveConfig::from_config_and_args(Some(&cfg), &parsed.args, &parsed.sources);
        assert_eq!(effective.rtsp_host, "10.0.0.2");
        assert_eq!(effective.rtsp_port, 8554);
    }

    #[test]
    fn test_from_config_and_args_stream_credentials_cli_overrides_config() {
        let parsed = parse_args_from([
            "rtsp_validation_tool",
            "--username",
            "cli_user",
            "--password",
            "cli_pass",
        ])
        .unwrap();
        let mut cfg = RtspValidationConfig::default();
        cfg.rtsp.username = Some("cfg_user".to_string());
        cfg.rtsp.password = Some("cfg_pass".to_string());

        let effective =
            EffectiveConfig::from_config_and_args(Some(&cfg), &parsed.args, &parsed.sources);
        assert_eq!(effective.stream_username.as_deref(), Some("cli_user"));
        assert_eq!(effective.stream_password.as_deref(), Some("cli_pass"));
    }

    #[test]
    fn test_from_config_and_args_stream_credentials_from_config_when_cli_omitted() {
        let parsed = parse_args_from(["rtsp_validation_tool"]).unwrap();
        let mut cfg = RtspValidationConfig::default();
        cfg.rtsp.username = Some("cfg_user".to_string());
        cfg.rtsp.password = Some("cfg_pass".to_string());

        let effective =
            EffectiveConfig::from_config_and_args(Some(&cfg), &parsed.args, &parsed.sources);
        assert_eq!(effective.stream_username.as_deref(), Some("cfg_user"));
        assert_eq!(effective.stream_password.as_deref(), Some("cfg_pass"));
    }

    #[test]
    fn test_from_config_and_args_stream_credentials_incomplete_pair_ignored() {
        let parsed = parse_args_from(["rtsp_validation_tool", "--username", "cli_user"]).unwrap();
        let effective = EffectiveConfig::from_config_and_args(None, &parsed.args, &parsed.sources);
        assert!(effective.stream_username.is_none());
        assert!(effective.stream_password.is_none());
    }

    #[test]
    fn test_from_config_and_args_initial_timestamp_policy_from_config_when_cli_omitted() {
        let parsed = parse_args_from(["rtsp_validation_tool"]).unwrap();
        let mut cfg = RtspValidationConfig::default();
        cfg.rtsp.initial_timestamp_policy = InitialTimestampPolicyArg::Ignore;
        let effective =
            EffectiveConfig::from_config_and_args(Some(&cfg), &parsed.args, &parsed.sources);
        assert_eq!(
            effective.initial_timestamp_policy,
            InitialTimestampPolicyArg::Ignore
        );
    }

    #[test]
    fn test_config_flag_short_c_parses() {
        let parsed = parse_args_from(["rtsp_validation_tool", "-c", "foo.toml"]).unwrap();
        assert_eq!(parsed.args.config.as_deref(), Some("foo.toml"));
    }

    #[test]
    fn test_help_mentions_config_search_order_and_env_var() {
        let help = super::Args::command().render_help().to_string();
        assert!(help.contains("--config"));
        assert!(help.contains("RTSP_VALIDATION_CONFIG"));
        assert!(help.contains("rtsp_validation.toml"));
    }

    #[test]
    fn test_load_config_path_override_valid_toml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rtsp_validation.toml");
        std::fs::write(
            &path,
            r#"
[rtsp]
host = "192.168.1.100"
port = 8554

[thresholds]
video-startup-latency-ms = 2000
harness-startup-latency-ms = 3500
"#,
        )
        .unwrap();
        let result = load_config(Some(path.to_str().unwrap())).unwrap();
        let config = result.expect("some config");
        assert_eq!(config.rtsp.host, "192.168.1.100");
        assert_eq!(config.rtsp.port, 8554);
        assert_eq!(config.thresholds.video_startup_latency_ms, 2000);
        assert_eq!(config.thresholds.harness_startup_latency_ms, 3500);
    }

    #[test]
    fn test_default_httpflv_path_is_canonical_live_stream1_flv() {
        assert_eq!(DEFAULT_HTTPFLV_PATH, "/live/stream1.flv");
    }

    #[test]
    fn test_from_config_and_args_httpflv_path_uses_default_when_not_overridden() {
        let parsed = parse_args_from(["rtsp_validation_tool"]).expect("parse args");
        let effective = EffectiveConfig::from_config_and_args(None, &parsed.args, &parsed.sources);
        assert_eq!(effective.httpflv_path, "/live/stream1.flv");
    }

    #[test]
    fn test_from_config_and_args_httpflv_path_cli_overrides_config() {
        let parsed = parse_args_from([
            "rtsp_validation_tool",
            "--httpflv-stream",
            "/live/custom.flv",
        ])
        .expect("parse args");
        let mut cfg = RtspValidationConfig::default();
        cfg.httpflv = Some(HttpFlvSection {
            port: 8080,
            path: "/live/stream1.flv".to_string(),
            timeout_sec: 10,
        });

        let effective =
            EffectiveConfig::from_config_and_args(Some(&cfg), &parsed.args, &parsed.sources);
        assert_eq!(effective.httpflv_path, "/live/custom.flv");
    }

    #[test]
    fn test_load_config_path_override_missing_file_error() {
        let result = load_config(Some("/nonexistent/path/rtsp_validation.toml"));
        assert!(result.is_err());
    }

    #[test]
    fn test_device_real_mode_clears_h264_and_aac() {
        let parsed = parse_args_from(["rtsp_validation_tool", "--device-real-mode"]).unwrap();
        let mut cfg = RtspValidationConfig::default();
        cfg.device.h264_file = Some("/mnt/test.h264".to_string());
        cfg.device.aac_file = Some("/mnt/test.aac".to_string());

        let effective =
            EffectiveConfig::from_config_and_args(Some(&cfg), &parsed.args, &parsed.sources);
        assert!(effective.device_real_mode);
        assert!(effective.device_h264_file.is_none());
        assert!(effective.device_aac_file.is_none());
    }

    #[test]
    fn test_device_real_mode_defaults_two_streams() {
        let parsed = parse_args_from(["rtsp_validation_tool", "--device-real-mode"]).unwrap();
        let effective = EffectiveConfig::from_config_and_args(None, &parsed.args, &parsed.sources);
        assert_eq!(effective.streams.len(), 2);
        assert_eq!(effective.streams[0].label, "main");
        assert_eq!(effective.streams[0].rtsp_stream, "/main");
        assert_eq!(effective.streams[0].httpflv_path, "/live/main.flv");
        assert_eq!(effective.streams[1].label, "sub");
        assert_eq!(effective.streams[1].rtsp_stream, "/sub");
        assert_eq!(effective.streams[1].httpflv_path, "/live/sub.flv");
    }

    #[test]
    fn test_device_real_mode_explicit_stream_override() {
        let parsed = parse_args_from([
            "rtsp_validation_tool",
            "--device-real-mode",
            "--rtsp-stream",
            "/sub",
        ])
        .unwrap();
        let effective = EffectiveConfig::from_config_and_args(None, &parsed.args, &parsed.sources);
        assert_eq!(effective.streams.len(), 1);
        assert_eq!(effective.streams[0].rtsp_stream, "/sub");
    }

    #[test]
    fn test_device_real_mode_from_config_toml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.toml");
        std::fs::write(
            &path,
            r#"
[device]
real-mode = true
"#,
        )
        .unwrap();
        let result = load_config(Some(path.to_str().unwrap())).unwrap();
        let config = result.expect("some config");
        assert!(config.device.real_mode);

        let parsed = parse_args_from(["rtsp_validation_tool"]).unwrap();
        let effective =
            EffectiveConfig::from_config_and_args(Some(&config), &parsed.args, &parsed.sources);
        assert!(effective.device_real_mode);
        assert_eq!(effective.streams.len(), 2);
    }

    #[test]
    fn test_device_real_mode_streams_from_config() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.toml");
        std::fs::write(
            &path,
            r#"
[device]
real-mode = true

[[device.streams]]
label = "cam1"
rtsp-stream = "/cam1"
httpflv-path = "/live/cam1.flv"

[[device.streams]]
label = "cam2"
rtsp-stream = "/cam2"
httpflv-path = "/live/cam2.flv"
"#,
        )
        .unwrap();
        let result = load_config(Some(path.to_str().unwrap())).unwrap();
        let config = result.expect("some config");

        let parsed = parse_args_from(["rtsp_validation_tool"]).unwrap();
        let effective =
            EffectiveConfig::from_config_and_args(Some(&config), &parsed.args, &parsed.sources);
        assert_eq!(effective.streams.len(), 2);
        assert_eq!(effective.streams[0].label, "cam1");
        assert_eq!(effective.streams[0].rtsp_stream, "/cam1");
        assert_eq!(effective.streams[1].label, "cam2");
        assert_eq!(effective.streams[1].rtsp_stream, "/cam2");
        // First stream becomes primary
        assert_eq!(effective.rtsp_stream, "/cam1");
        assert_eq!(effective.httpflv_path, "/live/cam1.flv");
    }

    #[test]
    fn test_non_real_mode_single_stream() {
        let parsed = parse_args_from(["rtsp_validation_tool"]).unwrap();
        let effective = EffectiveConfig::from_config_and_args(None, &parsed.args, &parsed.sources);
        assert!(!effective.device_real_mode);
        assert_eq!(effective.streams.len(), 1);
        assert_eq!(effective.streams[0].label, "");
        assert_eq!(effective.streams[0].rtsp_stream, DEFAULT_RTSP_STREAM);
    }
}
