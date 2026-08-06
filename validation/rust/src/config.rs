//! Configuration (TOML + CLI) and effective merged config.

use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use serde::Deserialize;
use std::env;
use std::ffi::OsString;
use std::path::PathBuf;
use tracing::{info, warn};

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
pub const DEFAULT_DEVICE_HOST: &str = "192.168.2.198";
pub const DEFAULT_DEVICE_USER: &str = "root";
pub const DEFAULT_DEVICE_SSH_PORT: u16 = 22;
pub const DEFAULT_OUTPUT_FILE_NAME: &str = "rtsp_validation.json";
pub const DEFAULT_HTTPFLV_TIMEOUT_SEC: u64 = 10;
pub const DEFAULT_PACING_EXPECTED_FPS: f64 = 25.0;
pub const DEFAULT_PACING_DELAY_MULTIPLE: f64 = 2.0;
pub const DEFAULT_PACING_DELAY_FLOOR_MS: f64 = 150.0;
pub const DEFAULT_PACING_DELAY_TOLERANCE_PERCENT: f64 = 5.0;

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
    #[serde(default)]
    pub pacing: PacingSection,
}

// Every section below is `#[serde(default)]` at the container level: any field the
// TOML omits falls back to that section's `Default`, so the defaults live in exactly
// one place instead of being repeated in a per-field `default = "…"` helper.

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields, default)]
pub struct PacingSection {
    pub expected_fps: f64,
    pub delay_multiple: f64,
    pub delay_floor_ms: f64,
    pub delay_tolerance_percent: f64,
}

impl Default for PacingSection {
    fn default() -> Self {
        Self {
            expected_fps: DEFAULT_PACING_EXPECTED_FPS,
            delay_multiple: DEFAULT_PACING_DELAY_MULTIPLE,
            delay_floor_ms: DEFAULT_PACING_DELAY_FLOOR_MS,
            delay_tolerance_percent: DEFAULT_PACING_DELAY_TOLERANCE_PERCENT,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields, default)]
pub struct HttpFlvSection {
    pub port: u16,
    pub path: String,
    pub timeout_sec: u64,
}

impl Default for HttpFlvSection {
    fn default() -> Self {
        Self {
            port: DEFAULT_HTTPFLV_PORT,
            path: DEFAULT_HTTPFLV_PATH.to_string(),
            // 0 means "unset"; the effective config substitutes its own default.
            timeout_sec: 0,
        }
    }
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields, default)]
pub struct RunSection {
    pub no_launch: bool,
    pub launch_on_device: bool,
    pub h264_file: Option<String>,
    pub output: Option<String>,
    pub update_baseline: bool,
    pub compare_baseline: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields, default)]
pub struct RtspSection {
    pub host: String,
    pub port: u16,
    pub stream: String,
    pub timeout_sec: u64,
    pub initial_timestamp_policy: InitialTimestampPolicyArg,
    pub username: Option<String>,
    pub password: Option<String>,
}

impl Default for RtspSection {
    fn default() -> Self {
        Self {
            host: DEFAULT_RTSP_HOST.to_string(),
            port: DEFAULT_RTSP_PORT,
            stream: DEFAULT_RTSP_STREAM.to_string(),
            timeout_sec: 10,
            initial_timestamp_policy: InitialTimestampPolicyArg::Permissive,
            username: None,
            password: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields, default)]
pub struct TestSection {
    pub short_duration_sec: u64,
    pub long_duration_sec: u64,
    pub concurrent_clients: u32,
}

impl Default for TestSection {
    fn default() -> Self {
        Self {
            short_duration_sec: DEFAULT_DURATION_SEC,
            long_duration_sec: DEFAULT_LONG_DURATION_SEC,
            concurrent_clients: DEFAULT_CONCURRENT_CLIENTS,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields, default)]
pub struct ThresholdsSection {
    pub video_startup_latency_ms: u64,
    pub harness_startup_latency_ms: u64,
    pub audio_startup_latency_ms: u64,
    pub bitrate_tolerance_percent: u32,
    pub fps_tolerance_percent: u32,
    pub packet_loss_tolerance_percent: f64,
    pub expected: Option<ThresholdsExpectedSection>,
}

impl Default for ThresholdsSection {
    fn default() -> Self {
        Self {
            video_startup_latency_ms: 1500,
            harness_startup_latency_ms: 3000,
            audio_startup_latency_ms: 0,
            bitrate_tolerance_percent: 15,
            fps_tolerance_percent: 10,
            packet_loss_tolerance_percent: 1.0,
            expected: None,
        }
    }
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields, default)]
pub struct ThresholdsExpectedSection {
    pub bitrate_kbps: Option<f64>,
    pub fps: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields, default)]
pub struct BaselineSection {
    pub dir: String,
}

impl Default for BaselineSection {
    fn default() -> Self {
        Self {
            dir: DEFAULT_BASELINE_DIR.to_string(),
        }
    }
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields, default)]
pub struct CaptureSection {
    pub interface: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields, default)]
pub struct ArtifactsSection {
    pub dir: String,
    pub capture_tool_output: bool,
    pub keep_pcaps: bool,
}

impl Default for ArtifactsSection {
    fn default() -> Self {
        Self {
            dir: DEFAULT_ARTIFACTS_ROOT_DIR.to_string(),
            capture_tool_output: true,
            keep_pcaps: true,
        }
    }
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields, default)]
pub struct LoggingSection {
    pub level: String,
    pub retina_level: String,
    pub file: String,
    pub ffmpeg_level: String,
}

/// A stream to validate (RTSP + HTTP-FLV paths).
///
/// Also the schema for a TOML `[[device.streams]]` array entry.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct StreamConfig {
    /// Human-readable label (e.g. "main", "sub").
    pub label: String,
    /// RTSP stream path (e.g. "/main").
    pub rtsp_stream: String,
    /// HTTP-FLV path (e.g. "/live/main.flv").
    pub httpflv_path: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields, default)]
pub struct DeviceSection {
    pub host: String,
    pub ssh_port: u16,
    pub user: String,
    pub password: Option<String>,
    pub telemetry: bool,
    pub h264_file: Option<String>,
    pub aac_file: Option<String>,
    pub loop_playback: bool,
    /// Launch the real camera pipeline (no validation-mode, no H.264/AAC files).
    pub real_mode: bool,
    /// Streams to validate in real mode.
    pub streams: Vec<StreamConfig>,
}

impl Default for DeviceSection {
    fn default() -> Self {
        Self {
            host: DEFAULT_DEVICE_HOST.to_string(),
            ssh_port: DEFAULT_DEVICE_SSH_PORT,
            user: DEFAULT_DEVICE_USER.to_string(),
            password: None,
            telemetry: true,
            h264_file: None,
            aac_file: None,
            loop_playback: false,
            real_mode: false,
            streams: Vec::new(),
        }
    }
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
    pub pacing_expected_fps: f64,
    pub pacing_delay_multiple: f64,
    pub pacing_delay_floor_ms: f64,
    pub pacing_delay_tolerance_percent: f64,
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
            .unwrap_or_else(|| DEFAULT_DEVICE_USER.to_string());
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
            .unwrap_or_else(|| DEFAULT_DEVICE_HOST.to_string());
        let device_ssh_port = args
            .device_ssh_port
            .or(Some(c.device.ssh_port))
            .filter(|&p| p != 0)
            .unwrap_or(DEFAULT_DEVICE_SSH_PORT);
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
        // CLI wins when present, then a non-empty config value, then the built-in default.
        let rtsp_host = if launch_on_device {
            device_host.clone()
        } else {
            args.rtsp_host
                .clone()
                .or_else(|| Some(c.rtsp.host.clone()).filter(|h| !h.is_empty()))
                .unwrap_or_else(|| DEFAULT_RTSP_HOST.to_string())
        };
        let rtsp_port = args
            .rtsp_port
            .or(Some(c.rtsp.port).filter(|&p| p != 0))
            .unwrap_or(DEFAULT_RTSP_PORT);
        let rtsp_stream = args
            .rtsp_stream
            .clone()
            .or_else(|| Some(c.rtsp.stream.clone()).filter(|s| !s.is_empty()))
            .unwrap_or_else(|| DEFAULT_RTSP_STREAM.to_string());
        let initial_timestamp_policy = args
            .initial_timestamp_policy
            .unwrap_or(c.rtsp.initial_timestamp_policy);
        let (stream_username, stream_password) = Self::resolve_stream_auth(args, c);
        let h264_file = args
            .h264_file
            .clone()
            .or_else(|| c.run.h264_file.clone())
            .filter(|s| !s.is_empty());
        let output = args
            .output
            .clone()
            .or_else(|| c.run.output.clone().filter(|s| !s.is_empty()))
            .unwrap_or_else(|| DEFAULT_OUTPUT_FILE_NAME.to_string());
        let update_baseline = c.run.update_baseline || args.update_baseline;
        let compare_baseline = c.run.compare_baseline || args.compare_baseline;

        let httpflv = c.httpflv.clone().unwrap_or(HttpFlvSection {
            timeout_sec: DEFAULT_HTTPFLV_TIMEOUT_SEC,
            ..Default::default()
        });
        let httpflv_port = args.httpflv_port.unwrap_or(httpflv.port);
        let httpflv_path = args.httpflv_stream.clone().unwrap_or(httpflv.path);
        let httpflv_timeout_sec = httpflv.timeout_sec;

        // Build streams list.
        let streams = if !c.device.streams.is_empty() {
            // Explicit [[device.streams]] from TOML config.
            c.device.streams.clone()
        } else if device_real_mode && args.rtsp_stream.is_none() {
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
            short_duration_sec: args.duration.unwrap_or(c.test.short_duration_sec).max(1),
            long_duration_sec: c.test.long_duration_sec,
            concurrent_clients: args.concurrent.unwrap_or(c.test.concurrent_clients),
            video_startup_latency_ms: args
                .max_video_startup_latency_ms
                .unwrap_or(c.thresholds.video_startup_latency_ms)
                .max(1),
            harness_startup_latency_ms: c.thresholds.harness_startup_latency_ms.max(1),
            audio_startup_latency_ms: c.thresholds.audio_startup_latency_ms,
            bitrate_tolerance_percent: c.thresholds.bitrate_tolerance_percent,
            fps_tolerance_percent: c.thresholds.fps_tolerance_percent,
            packet_loss_tolerance_percent: c.thresholds.packet_loss_tolerance_percent,
            expected_bitrate_kbps: expected_bitrate,
            expected_fps,
            pacing_expected_fps: c.pacing.expected_fps.max(0.1),
            pacing_delay_multiple: c.pacing.delay_multiple.max(0.0),
            pacing_delay_floor_ms: c.pacing.delay_floor_ms.max(0.0),
            pacing_delay_tolerance_percent: c.pacing.delay_tolerance_percent.max(0.0),
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

    // The options below are `Option<T>` on purpose: `None` means "user did not pass
    // this flag", which is what lets config-file values win. Their documented defaults
    // live in `EffectiveConfig::from_config_and_args`.
    #[arg(
        long,
        value_name = "HOST",
        help_heading = "RTSP",
        help = "RTSP server host/IP to validate (overrides config; default 127.0.0.1)."
    )]
    pub rtsp_host: Option<String>,

    #[arg(
        long,
        value_name = "PORT",
        help_heading = "RTSP",
        help = "RTSP server port to validate (overrides config; default 554)."
    )]
    pub rtsp_port: Option<u16>,

    #[arg(
        long,
        value_name = "PATH",
        help_heading = "RTSP",
        help = "RTSP stream path (must start with '/'; overrides config; default /stream1)."
    )]
    pub rtsp_stream: Option<String>,

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
        help_heading = "RTSP",
        help = "Policy for missing RTP-Info rtptime on PLAY (default, require, ignore, permissive)."
    )]
    pub initial_timestamp_policy: Option<InitialTimestampPolicyArg>,

    #[arg(
        long,
        value_name = "PORT",
        help_heading = "Run / Output",
        help = "HTTP-FLV port passed to onvif-rust when launching locally (overrides config; default 8080)."
    )]
    pub httpflv_port: Option<u16>,

    #[arg(
        long,
        value_name = "SEC",
        help_heading = "Run / Output",
        help = "Short test duration in seconds (overrides config; default 30)."
    )]
    pub duration: Option<u64>,

    #[arg(
        long,
        value_name = "MS",
        help_heading = "Thresholds",
        help = "Maximum allowed protocol first-frame latency in ms (`first_video_frame_latency_ms`; overrides config; default 1500)."
    )]
    pub max_video_startup_latency_ms: Option<u64>,

    #[arg(
        long,
        value_name = "FILE",
        help_heading = "Run / Output",
        help = "Report filename written inside the run artifacts directory (default rtsp_validation.json)."
    )]
    pub output: Option<String>,

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

pub fn parse_args() -> Args {
    Args::parse()
}

pub fn parse_args_from<I, T>(iter: I) -> Result<Args>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    Args::try_parse_from(iter).context("parse CLI arguments")
}

#[cfg(test)]
// Tests build a config then poke one or two fields. Struct-update syntax with
// `..Default::default()` is ~45 lines longer here for no gain in the assertions.
#[allow(clippy::field_reassign_with_default)]
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
        let effective = EffectiveConfig::from_config_and_args(None, &parsed);
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
    fn test_from_config_and_args_pacing_defaults() {
        let parsed = parse_args_from(["rtsp_validation_tool"]).unwrap();
        let effective = EffectiveConfig::from_config_and_args(None, &parsed);
        assert_eq!(effective.pacing_expected_fps, 25.0);
        assert_eq!(effective.pacing_delay_multiple, 2.0);
        assert_eq!(effective.pacing_delay_floor_ms, 150.0);
        assert_eq!(effective.pacing_delay_tolerance_percent, 5.0);
    }

    #[test]
    fn test_pacing_section_parses_from_toml() {
        let cfg: RtspValidationConfig = toml::from_str(
            r#"
            [pacing]
            expected-fps = 15
            delay-multiple = 3.0
            delay-floor-ms = 200
            delay-tolerance-percent = 10
            "#,
        )
        .unwrap();
        assert_eq!(cfg.pacing.expected_fps, 15.0);
        assert_eq!(cfg.pacing.delay_multiple, 3.0);
        assert_eq!(cfg.pacing.delay_floor_ms, 200.0);
        assert_eq!(cfg.pacing.delay_tolerance_percent, 10.0);
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
        let effective = EffectiveConfig::from_config_and_args(None, &parsed);
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
            &parsed,
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
            &parsed,
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
        let effective = EffectiveConfig::from_config_and_args(None, &parsed);
        assert!(effective.launch_on_device);
        assert_eq!(effective.rtsp_host, "192.168.2.100");
        assert_eq!(effective.device_host, "192.168.2.100");
    }

    #[test]
    fn test_resolve_capture_interface_non_empty_unchanged() {
        let parsed = parse_args_from(["rtsp_validation_tool"]).unwrap();
        let mut effective = EffectiveConfig::from_config_and_args(None, &parsed);
        effective.capture_interface = "eth0".to_string();
        effective.resolve_capture_interface();
        assert_eq!(effective.capture_interface, "eth0");
    }

    #[test]
    fn test_resolve_capture_interface_empty_localhost_lo() {
        let parsed = parse_args_from(["rtsp_validation_tool", "--rtsp-host", "127.0.0.1"]).unwrap();
        let mut effective = EffectiveConfig::from_config_and_args(None, &parsed);
        effective.capture_interface = String::new();
        effective.resolve_capture_interface();
        assert_eq!(effective.capture_interface, "lo");
    }

    #[test]
    fn test_resolve_capture_interface_empty_localhost_string() {
        let parsed = parse_args_from(["rtsp_validation_tool"]).unwrap();
        let mut cfg = RtspValidationConfig::default();
        cfg.rtsp.host = "localhost".to_string();
        let mut effective = EffectiveConfig::from_config_and_args(Some(&cfg), &parsed);
        effective.capture_interface = String::new();
        effective.resolve_capture_interface();
        assert_eq!(effective.capture_interface, "lo");
    }

    #[test]
    fn test_resolve_capture_interface_empty_ipv6_localhost_lo() {
        let parsed = parse_args_from(["rtsp_validation_tool"]).unwrap();
        let mut cfg = RtspValidationConfig::default();
        cfg.rtsp.host = "::1".to_string();
        let mut effective = EffectiveConfig::from_config_and_args(Some(&cfg), &parsed);
        effective.capture_interface = String::new();
        effective.resolve_capture_interface();
        assert_eq!(effective.capture_interface, "lo");
    }

    #[test]
    fn test_resolve_capture_interface_empty_remote_any() {
        let parsed = parse_args_from(["rtsp_validation_tool"]).unwrap();
        let mut cfg = RtspValidationConfig::default();
        cfg.rtsp.host = "10.0.0.1".to_string();
        let mut effective = EffectiveConfig::from_config_and_args(Some(&cfg), &parsed);
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
        let effective = EffectiveConfig::from_config_and_args(Some(&cfg), &parsed);
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

        let effective = EffectiveConfig::from_config_and_args(Some(&cfg), &parsed);
        assert_eq!(effective.stream_username.as_deref(), Some("cli_user"));
        assert_eq!(effective.stream_password.as_deref(), Some("cli_pass"));
    }

    #[test]
    fn test_from_config_and_args_stream_credentials_from_config_when_cli_omitted() {
        let parsed = parse_args_from(["rtsp_validation_tool"]).unwrap();
        let mut cfg = RtspValidationConfig::default();
        cfg.rtsp.username = Some("cfg_user".to_string());
        cfg.rtsp.password = Some("cfg_pass".to_string());

        let effective = EffectiveConfig::from_config_and_args(Some(&cfg), &parsed);
        assert_eq!(effective.stream_username.as_deref(), Some("cfg_user"));
        assert_eq!(effective.stream_password.as_deref(), Some("cfg_pass"));
    }

    #[test]
    fn test_from_config_and_args_stream_credentials_incomplete_pair_ignored() {
        let parsed = parse_args_from(["rtsp_validation_tool", "--username", "cli_user"]).unwrap();
        let effective = EffectiveConfig::from_config_and_args(None, &parsed);
        assert!(effective.stream_username.is_none());
        assert!(effective.stream_password.is_none());
    }

    #[test]
    fn test_from_config_and_args_initial_timestamp_policy_from_config_when_cli_omitted() {
        let parsed = parse_args_from(["rtsp_validation_tool"]).unwrap();
        let mut cfg = RtspValidationConfig::default();
        cfg.rtsp.initial_timestamp_policy = InitialTimestampPolicyArg::Ignore;
        let effective = EffectiveConfig::from_config_and_args(Some(&cfg), &parsed);
        assert_eq!(
            effective.initial_timestamp_policy,
            InitialTimestampPolicyArg::Ignore
        );
    }

    #[test]
    fn test_config_flag_short_c_parses() {
        let parsed = parse_args_from(["rtsp_validation_tool", "-c", "foo.toml"]).unwrap();
        assert_eq!(parsed.config.as_deref(), Some("foo.toml"));
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
        let effective = EffectiveConfig::from_config_and_args(None, &parsed);
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

        let effective = EffectiveConfig::from_config_and_args(Some(&cfg), &parsed);
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

        let effective = EffectiveConfig::from_config_and_args(Some(&cfg), &parsed);
        assert!(effective.device_real_mode);
        assert!(effective.device_h264_file.is_none());
        assert!(effective.device_aac_file.is_none());
    }

    #[test]
    fn test_device_real_mode_defaults_two_streams() {
        let parsed = parse_args_from(["rtsp_validation_tool", "--device-real-mode"]).unwrap();
        let effective = EffectiveConfig::from_config_and_args(None, &parsed);
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
        let effective = EffectiveConfig::from_config_and_args(None, &parsed);
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
        let effective = EffectiveConfig::from_config_and_args(Some(&config), &parsed);
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
        let effective = EffectiveConfig::from_config_and_args(Some(&config), &parsed);
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
        let effective = EffectiveConfig::from_config_and_args(None, &parsed);
        assert!(!effective.device_real_mode);
        assert_eq!(effective.streams.len(), 1);
        assert_eq!(effective.streams[0].label, "");
        assert_eq!(effective.streams[0].rtsp_stream, DEFAULT_RTSP_STREAM);
    }
}
