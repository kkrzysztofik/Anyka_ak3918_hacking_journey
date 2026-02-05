//! Configuration (TOML + CLI) and effective merged config.

use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use serde::Deserialize;
use std::env;
use std::path::PathBuf;
use tracing::info;

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
pub const DEFAULT_HTTPFLV_PATH: &str = "/live/stream.flv";

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

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct DeviceSection {
    #[serde(default = "default_device_host")]
    pub host: String,
    #[serde(default = "default_device_telnet_port")]
    pub telnet_port: u16,
    #[serde(default = "default_telemetry_enabled")]
    pub telemetry: bool,
    #[serde(default)]
    pub h264_file: Option<String>,
    #[serde(default)]
    pub aac_file: Option<String>,
    #[serde(default)]
    pub loop_playback: bool,
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
    pub artifacts_root_dir: PathBuf,
    pub artifacts_dir: PathBuf,
    pub capture_tool_output: bool,
    pub keep_pcaps: bool,
    pub launch_on_device: bool,
    pub device_host: String,
    pub device_telnet_port: u16,
    pub collect_telemetry: bool,
    pub device_h264_file: Option<String>,
    pub device_aac_file: Option<String>,
    pub device_loop_playback: bool,
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
}

impl EffectiveConfig {
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

    /// Build effective config: config file first, then CLI overrides.
    pub fn from_config_and_args(config: Option<&RtspValidationConfig>, args: &Args) -> Self {
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
        let device_telnet_port = args
            .device_telnet_port
            .or(Some(c.device.telnet_port))
            .filter(|&p| p != 0)
            .unwrap_or(24);
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
        let ffmpeg_log_level =
            Self::ffmpeg_log_level_from_config(Some(c), args.ffmpeg_log_level.as_deref());
        let rtsp_host = if launch_on_device {
            device_host.clone()
        } else if !c.rtsp.host.is_empty() {
            c.rtsp.host.clone()
        } else {
            args.rtsp_host.clone()
        };
        let rtsp_port = if c.rtsp.port != 0 {
            c.rtsp.port
        } else {
            args.rtsp_port
        };
        let rtsp_stream = if !c.rtsp.stream.is_empty() {
            c.rtsp.stream.clone()
        } else {
            args.rtsp_stream.clone()
        };
        let h264_file = args
            .h264_file
            .clone()
            .or_else(|| c.run.h264_file.clone())
            .filter(|s| !s.is_empty());
        let output = c
            .run
            .output
            .clone()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| args.output.clone());
        let update_baseline = c.run.update_baseline || args.update_baseline;
        let compare_baseline = c.run.compare_baseline || args.compare_baseline;

        let httpflv_path = c
            .httpflv
            .as_ref()
            .map(|h| h.path.clone())
            .unwrap_or_else(|| DEFAULT_HTTPFLV_PATH.to_string());
        let httpflv_port = args.httpflv_port;

        Self {
            rtsp_host,
            rtsp_port,
            rtsp_stream,
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
            artifacts_root_dir: PathBuf::from(artifacts_root_dir),
            artifacts_dir: PathBuf::new(),
            capture_tool_output,
            keep_pcaps,
            launch_on_device,
            device_host,
            device_telnet_port,
            collect_telemetry,
            device_h264_file,
            device_aac_file,
            device_loop_playback,
            no_launch,
            h264_file,
            output,
            update_baseline,
            compare_baseline,
            ffmpeg_log_level,
            httpflv_port,
            httpflv_path,
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
pub struct Args {
    #[arg(long)]
    pub h264_file: Option<String>,

    #[arg(long, default_value = "127.0.0.1")]
    pub rtsp_host: String,

    #[arg(long, default_value = "554")]
    pub rtsp_port: u16,

    #[arg(long, default_value = "/stream1")]
    pub rtsp_stream: String,

    #[arg(long)]
    pub username: Option<String>,

    #[arg(long)]
    pub password: Option<String>,

    #[arg(long, value_enum, default_value_t = TransportArg::Tcp)]
    pub transport: TransportArg,

    #[arg(long, default_value = "8080")]
    pub httpflv_port: u16,

    #[arg(long, default_value = "60")]
    pub duration: u64,

    #[arg(long, default_value_t = DEFAULT_VIDEO_STARTUP_TARGET_MS)]
    pub max_video_startup_latency_ms: u64,

    #[arg(long, default_value = "rtsp_validation.json")]
    pub output: String,

    #[arg(long)]
    pub onvif_binary: Option<String>,

    #[arg(long)]
    pub no_launch: bool,

    #[arg(long)]
    pub loop_playback: bool,

    #[arg(long)]
    pub require_audio: bool,

    #[arg(long)]
    pub config: Option<String>,

    #[arg(long)]
    pub artifacts_dir: Option<String>,

    #[arg(long, value_name = "LEVEL")]
    pub ffmpeg_log_level: Option<String>,

    #[arg(long)]
    pub update_baseline: bool,

    #[arg(long)]
    pub compare_baseline: bool,

    #[arg(long)]
    pub concurrent: Option<u32>,

    #[arg(long)]
    pub long_duration: bool,

    #[arg(long)]
    pub skip_error_handling: bool,

    #[arg(long)]
    pub launch_on_device: bool,

    #[arg(long)]
    pub device_host: Option<String>,

    #[arg(long)]
    pub device_telnet_port: Option<u16>,

    #[arg(long)]
    pub no_telemetry: bool,

    #[arg(long)]
    pub device_h264_file: Option<String>,

    #[arg(long)]
    pub device_aac_file: Option<String>,

    #[arg(long)]
    pub device_loop_playback: bool,
}

#[derive(ValueEnum, Clone, Copy, Debug)]
pub enum TransportArg {
    Tcp,
    Udp,
}

#[cfg(test)]
mod tests {
    use super::{
        DEFAULT_ARTIFACTS_ROOT_DIR, DEFAULT_BASELINE_DIR, DEFAULT_RTSP_HOST, DEFAULT_RTSP_PORT,
        DEFAULT_RTSP_STREAM, EffectiveConfig, LoggingSection, RtspValidationConfig, load_config,
    };
    use clap::Parser;
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
        let args = super::Args::parse_from(["rtsp_validation_tool"]);
        let effective = EffectiveConfig::from_config_and_args(None, &args);
        assert_eq!(effective.rtsp_host, DEFAULT_RTSP_HOST);
        assert_eq!(effective.rtsp_port, DEFAULT_RTSP_PORT);
        assert_eq!(effective.rtsp_stream, DEFAULT_RTSP_STREAM);
        assert_eq!(effective.short_duration_sec, 60); // args.duration default 60
        assert_eq!(effective.baseline_dir, PathBuf::from(DEFAULT_BASELINE_DIR));
        assert_eq!(
            effective.artifacts_root_dir,
            PathBuf::from(DEFAULT_ARTIFACTS_ROOT_DIR)
        );
        assert!(!effective.launch_on_device);
        assert!(!effective.update_baseline);
        assert!(!effective.compare_baseline);
    }

    #[test]
    fn test_from_config_and_args_cli_overrides() {
        let args = super::Args::parse_from([
            "rtsp_validation_tool",
            "--rtsp-host",
            "10.0.0.1",
            "--rtsp-port",
            "8554",
            "--duration",
            "5",
            "--artifacts-dir",
            "/tmp/artifacts",
            "--ffmpeg-log-level",
            "warning",
            "--update-baseline",
            "--device-host",
            "192.168.1.1",
        ]);
        let effective = EffectiveConfig::from_config_and_args(None, &args);
        assert_eq!(effective.rtsp_host, "10.0.0.1");
        assert_eq!(effective.rtsp_port, 8554);
        assert_eq!(effective.short_duration_sec, 5);
        assert_eq!(
            effective.artifacts_root_dir,
            PathBuf::from("/tmp/artifacts")
        );
        assert_eq!(effective.ffmpeg_log_level, "warning");
        assert!(effective.update_baseline);
        assert_eq!(effective.device_host, "192.168.1.1");
    }

    #[test]
    fn test_from_config_and_args_launch_on_device_uses_device_host() {
        let args = super::Args::parse_from([
            "rtsp_validation_tool",
            "--launch-on-device",
            "--device-host",
            "192.168.2.100",
        ]);
        let effective = EffectiveConfig::from_config_and_args(None, &args);
        assert!(effective.launch_on_device);
        assert_eq!(effective.rtsp_host, "192.168.2.100");
        assert_eq!(effective.device_host, "192.168.2.100");
    }

    #[test]
    fn test_resolve_capture_interface_non_empty_unchanged() {
        let args = super::Args::parse_from(["rtsp_validation_tool"]);
        let mut effective = EffectiveConfig::from_config_and_args(None, &args);
        effective.capture_interface = "eth0".to_string();
        effective.resolve_capture_interface();
        assert_eq!(effective.capture_interface, "eth0");
    }

    #[test]
    fn test_resolve_capture_interface_empty_localhost_lo() {
        let args = super::Args::parse_from(["rtsp_validation_tool", "--rtsp-host", "127.0.0.1"]);
        let mut effective = EffectiveConfig::from_config_and_args(None, &args);
        effective.capture_interface = String::new();
        effective.resolve_capture_interface();
        assert_eq!(effective.capture_interface, "lo");
    }

    #[test]
    fn test_resolve_capture_interface_empty_localhost_string() {
        let args = super::Args::parse_from(["rtsp_validation_tool"]);
        let mut cfg = RtspValidationConfig::default();
        cfg.rtsp.host = "localhost".to_string();
        let mut effective = EffectiveConfig::from_config_and_args(Some(&cfg), &args);
        effective.capture_interface = String::new();
        effective.resolve_capture_interface();
        assert_eq!(effective.capture_interface, "lo");
    }

    #[test]
    fn test_resolve_capture_interface_empty_ipv6_localhost_lo() {
        let args = super::Args::parse_from(["rtsp_validation_tool"]);
        let mut cfg = RtspValidationConfig::default();
        cfg.rtsp.host = "::1".to_string();
        let mut effective = EffectiveConfig::from_config_and_args(Some(&cfg), &args);
        effective.capture_interface = String::new();
        effective.resolve_capture_interface();
        assert_eq!(effective.capture_interface, "lo");
    }

    #[test]
    fn test_resolve_capture_interface_empty_remote_any() {
        let args = super::Args::parse_from(["rtsp_validation_tool"]);
        let mut cfg = RtspValidationConfig::default();
        cfg.rtsp.host = "10.0.0.1".to_string();
        let mut effective = EffectiveConfig::from_config_and_args(Some(&cfg), &args);
        effective.capture_interface = String::new();
        effective.resolve_capture_interface();
        assert_eq!(effective.capture_interface, "any");
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
"#,
        )
        .unwrap();
        let result = load_config(Some(path.to_str().unwrap())).unwrap();
        let config = result.expect("some config");
        assert_eq!(config.rtsp.host, "192.168.1.100");
        assert_eq!(config.rtsp.port, 8554);
        assert_eq!(config.thresholds.video_startup_latency_ms, 2000);
    }

    #[test]
    fn test_load_config_path_override_missing_file_error() {
        let result = load_config(Some("/nonexistent/path/rtsp_validation.toml"));
        assert!(result.is_err());
    }
}
