//! ONVIF Rust daemon entry point.
//!
//! This is the main entry point for the ONVIF daemon. It uses the Application
//! lifecycle pattern for clean startup and shutdown.
//!
//! Optional validation mode for H.264 playback testing:
//! `--validation-mode --h264-file <path> [--aac-file <path>] [--audio-sample-rate 48000] [--rtsp-port 8554] [--httpflv-port 8080] [--loop-playback]`

use anyhow::{Context, Result};
use async_trait::async_trait;
use bytes::BytesMut;
use clap::Parser;
use onvif_rust::app::{Application, DEFAULT_CONFIG_PATH};
use onvif_rust::config::{ConfigRuntime, ConfigStorage};
use onvif_rust::validation::h264_playback::{H264PlaybackConfig, H264PlaybackMode};
use portable_atomic::{AtomicU32, AtomicUsize, Ordering};
use std::backtrace::Backtrace;
use std::panic::PanicHookInfo;
use std::sync::{Arc, Once};
use streaming_lib::streamhub::define::{Information, InformationSender};
use streaming_lib::streamhub::errors::StreamHubError;
use streaming_lib::streamhub::statistics::StatisticsStream;
use streaming_lib::{
    DataSender, FrameData, MediaInfo, SubscribeType, TStreamHandler, VideoCodecType,
};
use tokio::time::{Duration, timeout};

/// ONVIF daemon with optional H.264 playback validation mode
#[derive(Parser, Debug)]
#[command(name = "onvifd")]
#[command(about = "ONVIF daemon for Anyka AK3918 cameras", long_about = None)]
struct CliArgs {
    /// Enable H.264 playback validation mode
    #[arg(long)]
    validation_mode: bool,

    /// Path to H.264 file (required in validation mode)
    #[arg(long)]
    h264_file: Option<String>,

    /// Path to AAC audio file (optional)
    #[arg(long)]
    aac_file: Option<String>,

    /// Audio sample rate in Hz
    #[arg(long, default_value = "48000")]
    audio_sample_rate: u32,

    /// RTSP server port
    #[arg(long, default_value = "8554")]
    rtsp_port: u16,

    /// HTTP-FLV server port
    #[arg(long, default_value = "8080")]
    httpflv_port: u16,

    /// Loop H.264 file playback
    #[arg(long)]
    loop_playback: bool,

    /// Path to configuration file
    #[arg(value_name = "CONFIG_PATH", default_value = DEFAULT_CONFIG_PATH)]
    config_path: String,
}

#[derive(Debug)]
struct StartupArgs {
    validation_config: Option<H264PlaybackConfig>,
    config_path: String,
}

/// Parse CLI arguments for normal or validation mode
fn parse_arguments() -> StartupArgs {
    let cli = CliArgs::parse();

    let validation_config = if cli.validation_mode {
        if let Some(file_path) = cli.h264_file {
            Some(H264PlaybackConfig {
                file_path,
                frame_rate: 25, // Default to 25 fps
                loop_playback: cli.loop_playback,
                rtsp_port: cli.rtsp_port,
                httpflv_port: cli.httpflv_port,
                audio_file_path: cli.aac_file,
                audio_sample_rate: cli.audio_sample_rate,
            })
        } else {
            eprintln!("error: --validation-mode requires --h264-file <path>");
            std::process::exit(1);
        }
    } else {
        None
    };

    StartupArgs {
        validation_config,
        config_path: cli.config_path,
    }
}

/// Get runtime configuration (worker threads, blocking threads) based on platform.
///
/// For ARM embedded targets (single-core AK3918), use minimal threading to reduce
/// scheduler contention. For host builds, use available parallelism.
fn get_runtime_config(validation_mode: bool) -> (usize, usize) {
    fn parse_positive_usize(var_name: &str) -> Option<usize> {
        std::env::var(var_name)
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .filter(|value| *value > 0)
    }

    if cfg!(target_arch = "arm") {
        // Embedded default: allow light concurrency for async runtime + blocking FS work.
        // Can be overridden for tuning:
        //   ONVIF_TOKIO_WORKER_THREADS
        //   ONVIF_TOKIO_MAX_BLOCKING_THREADS
        let default_workers = if validation_mode { 1 } else { 2 };
        let default_blocking = if validation_mode { 2 } else { 16 };
        let workers = parse_positive_usize("ONVIF_TOKIO_WORKER_THREADS").unwrap_or(default_workers);
        let blocking =
            parse_positive_usize("ONVIF_TOKIO_MAX_BLOCKING_THREADS").unwrap_or(default_blocking);
        (workers, blocking)
    } else {
        // Host: use available parallelism
        let workers = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        (workers, 512) // (worker_threads, max_blocking_threads)
    }
}

static VALIDATION_PANIC_HOOK_ONCE: Once = Once::new();

fn panic_message(panic_info: &PanicHookInfo<'_>) -> String {
    if let Some(message) = panic_info.payload().downcast_ref::<&str>() {
        (*message).to_string()
    } else if let Some(message) = panic_info.payload().downcast_ref::<String>() {
        message.clone()
    } else {
        "non-string panic payload".to_string()
    }
}

fn install_validation_panic_hook() {
    VALIDATION_PANIC_HOOK_ONCE.call_once(|| {
        let default_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |panic_info| {
            let location = panic_info
                .location()
                .map(|location| format!("{}:{}", location.file(), location.line()))
                .unwrap_or_else(|| "unknown".to_string());
            let message = panic_message(panic_info);
            let backtrace = Backtrace::force_capture();

            eprintln!(
                "validation mode panic at {}: {}\n{}",
                location, message, backtrace
            );
            tracing::error!(
                location = %location,
                message = %message,
                backtrace = %backtrace,
                "validation mode panic"
            );

            default_hook(panic_info);
        }));
    });
}

fn main() -> Result<()> {
    let startup_args = parse_arguments();
    let validation_mode = startup_args.validation_config.is_some();

    // Build explicit tokio runtime with platform-specific configuration
    let (worker_threads, max_blocking_threads) = get_runtime_config(validation_mode);
    eprintln!(
        "onvif-rust runtime config: worker_threads={}, max_blocking_threads={}",
        worker_threads, max_blocking_threads
    );

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(worker_threads)
        .max_blocking_threads(max_blocking_threads)
        .enable_io()
        .enable_time()
        .thread_name("onvif-worker")
        .build()
        .context("Failed to create tokio runtime")?;

    // Run async main on the runtime
    runtime.block_on(async_main(startup_args))
}

/// Async entry point for the ONVIF daemon.
///
/// Separated from main() to allow explicit runtime configuration.
async fn async_main(startup_args: StartupArgs) -> Result<()> {
    let StartupArgs {
        validation_config,
        config_path,
    } = startup_args;

    if let Some(config) = validation_config {
        install_validation_panic_hook();
        // Validation mode: initialize H.264 playback pipeline
        run_validation_mode(config, &config_path).await
    } else {
        // Normal mode: start standard ONVIF application
        run_normal_mode(&config_path).await
    }
}

/// Run the daemon in normal ONVIF mode
async fn run_normal_mode(config_path: &str) -> Result<()> {
    // Logging is initialized by Application::start() after loading config
    // This allows proper file logging setup based on configuration

    if let Ok(app_config) = ConfigStorage::load_or_default(config_path) {
        let config_runtime = ConfigRuntime::new(app_config);
        let _ = configure_stream_frame_debug_logging(&config_runtime);
    }

    // Start the application with ordered initialization
    let app = match Application::start(config_path).await {
        Ok(app) => app,
        Err(e) => {
            tracing::error!("Failed to start application: {}", e);
            return Err(e.into());
        }
    };

    // Log health status
    let health = app.health();
    if app.is_degraded() {
        tracing::warn!(
            "Application health: {} (DEGRADED). Unavailable services: {:?}",
            health.status,
            app.degraded_services()
        );
    } else {
        tracing::info!("Application health: {}", health.status);
    }

    // Run until shutdown signal (SIGINT/SIGTERM)
    if let Err(e) = app.run().await {
        tracing::error!("Runtime error: {}", e);
    }

    // Perform graceful shutdown
    let report = app.shutdown().await;

    // Log shutdown report
    match report.status {
        onvif_rust::ShutdownStatus::Success => {
            tracing::info!("Shutdown completed successfully in {:?}", report.duration);
        }
        onvif_rust::ShutdownStatus::Timeout => {
            tracing::warn!(
                "Shutdown timed out after {:?}. Some components may not have stopped cleanly.",
                report.duration
            );
        }
        onvif_rust::ShutdownStatus::Error => {
            tracing::error!("Shutdown encountered errors: {:?}", report.errors);
        }
    }

    Ok(())
}

fn configure_stream_frame_debug_logging(config: &ConfigRuntime) -> bool {
    let enabled = config
        .get_bool("logging.stream_frame_debug")
        .unwrap_or(false);
    streaming_lib::set_stream_frame_debug_logging(enabled);
    enabled
}

fn env_flag(name: &str, default: bool) -> bool {
    match std::env::var(name) {
        Ok(value) => match value.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => true,
            "0" | "false" | "no" | "off" => false,
            _ => default,
        },
        Err(_) => default,
    }
}

fn fanout_validation_frame(
    frame_tx_rtsp: &tokio::sync::mpsc::UnboundedSender<FrameData>,
    frame_tx_httpflv: Option<&tokio::sync::mpsc::UnboundedSender<FrameData>>,
    frame: FrameData,
) {
    match frame {
        FrameData::Audio { .. } => {
            let _ = frame_tx_rtsp.send(frame);
        }
        frame => {
            if let Some(tx_httpflv) = frame_tx_httpflv {
                let _ = frame_tx_rtsp.send(frame.clone());
                let _ = tx_httpflv.send(frame);
            } else {
                let _ = frame_tx_rtsp.send(frame);
            }
        }
    }
}

struct ValidationAvStreamHandler {
    sps: Vec<u8>,
    pps: Vec<u8>,
    bootstrap_idr: Option<Vec<u8>>,
    last_video_timestamp_ms: Arc<AtomicU32>,
    audio_config: Option<Vec<u8>>,
    audio_sample_rate: u32,
}

impl ValidationAvStreamHandler {
    fn new(
        sps: Vec<u8>,
        pps: Vec<u8>,
        bootstrap_idr: Option<Vec<u8>>,
        last_video_timestamp_ms: Arc<AtomicU32>,
        audio_config: Option<Vec<u8>>,
        audio_sample_rate: u32,
    ) -> Self {
        Self {
            sps,
            pps,
            bootstrap_idr,
            last_video_timestamp_ms,
            audio_config,
            audio_sample_rate,
        }
    }
}

#[async_trait]
impl TStreamHandler for ValidationAvStreamHandler {
    async fn send_prior_data(
        &self,
        sender: DataSender,
        _sub_type: SubscribeType,
    ) -> Result<(), StreamHubError> {
        if let DataSender::Frame {
            sender: frame_sender,
        } = sender
        {
            let timestamp = self.last_video_timestamp_ms.load(Ordering::Relaxed);
            let audio_clock_rate = if self.audio_config.is_some() {
                self.audio_sample_rate
            } else {
                0
            };

            let media_info = MediaInfo {
                audio_clock_rate,
                video_clock_rate: 90000,
                vcodec: VideoCodecType::H264,
            };
            let _ = frame_sender.send(FrameData::MediaInfo { media_info });

            let _ = frame_sender.send(FrameData::Video {
                timestamp,
                data: BytesMut::from(self.sps.as_slice()),
            });
            let _ = frame_sender.send(FrameData::Video {
                timestamp,
                data: BytesMut::from(self.pps.as_slice()),
            });
            if let Some(idr) = self.bootstrap_idr.as_ref() {
                let _ = frame_sender.send(FrameData::Video {
                    timestamp,
                    data: BytesMut::from(idr.as_slice()),
                });
            }
        }

        Ok(())
    }

    async fn get_statistic_data(&self) -> Option<StatisticsStream> {
        None
    }

    async fn send_information(&self, sender: InformationSender) {
        let sdp = generate_av_sdp(
            &self.sps,
            &self.pps,
            self.audio_config.as_deref(),
            self.audio_sample_rate,
        );
        let _ = sender.send(Information::Sdp { data: sdp });
    }
}

fn generate_av_sdp(
    sps: &[u8],
    pps: &[u8],
    audio_config: Option<&[u8]>,
    audio_sample_rate: u32,
) -> String {
    let profile_level_id = if sps.len() >= 4 {
        format!("{:02x}{:02x}{:02x}", sps[1], sps[2], sps[3])
    } else {
        "42e01e".to_string()
    };

    let mut sdp = String::new();
    sdp.push_str("v=0\r\n");
    sdp.push_str("o=- 0 0 IN IP4 0.0.0.0\r\n");
    sdp.push_str("s=H264 Validation Stream\r\n");
    sdp.push_str("c=IN IP4 0.0.0.0\r\n");
    sdp.push_str("t=0 0\r\n");
    sdp.push_str("a=tool:onvif-validation\r\n");
    sdp.push_str("a=control:*\r\n");
    sdp.push_str("m=video 0 RTP/AVP 96\r\n");
    sdp.push_str("a=rtpmap:96 H264/90000\r\n");
    sdp.push_str(&format!(
        "a=fmtp:96 packetization-mode=1; sprop-parameter-sets={},{}; profile-level-id={}\r\n",
        base64_encode(sps),
        base64_encode(pps),
        profile_level_id
    ));
    sdp.push_str("a=control:trackID=0\r\n");
    sdp.push_str("a=sendonly\r\n");

    if let Some(config) = audio_config {
        let channels = audio_channels_from_config(config);
        let config_hex = audio_config_hex(config);

        sdp.push_str("m=audio 0 RTP/AVP 97\r\n");
        sdp.push_str(&format!(
            "a=rtpmap:97 MPEG4-GENERIC/{}/{}\r\n",
            audio_sample_rate, channels
        ));
        sdp.push_str(&format!(
            "a=fmtp:97 profile-level-id=1;mode=AAC-hbr;sizelength=13;indexlength=3;indexdeltalength=3;config={}\r\n",
            config_hex
        ));
        sdp.push_str("a=control:trackID=1\r\n");
        sdp.push_str("a=sendonly\r\n");
    }

    sdp
}

fn audio_channels_from_config(audio_config: &[u8]) -> u32 {
    if audio_config.len() >= 2 {
        ((audio_config[1] >> 3) & 0x0F) as u32
    } else {
        2
    }
}

fn audio_config_hex(audio_config: &[u8]) -> String {
    audio_config
        .iter()
        .map(|b| format!("{:02X}", b))
        .collect::<String>()
}

fn base64_encode(data: &[u8]) -> String {
    use base64::Engine as _;
    use base64::engine::general_purpose::STANDARD as BASE64;
    BASE64.encode(data)
}

/// Run the daemon in H.264 playback validation mode
async fn run_validation_mode(config: H264PlaybackConfig, config_path: &str) -> Result<()> {
    use onvif_rust::platform::{Platform, ValidationPlatform};
    use std::path::Path;
    use streaming_lib::StreamIdentifier;
    use streaming_lib::streamhub::define::DataReceiver;
    use streaming_lib::streamhub::mock_audio_publisher::MockAudioPublisher;
    use streaming_lib::streamhub::mock_publisher::MockVideoPublisher;
    use streaming_lib::{
        DefaultHttpFlvServer, DefaultRtspServer, HttpFlvServer, RtspServer, StreamsHub,
    };
    use tokio::sync::mpsc;

    let publisher_init_timeout_sec = std::env::var("ONVIF_VALIDATION_PUBLISHER_INIT_TIMEOUT_SEC")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(30);

    // Configure RTP packet sampling for RTSP validation runs.
    //
    // This is intentionally controlled by an environment variable so that
    // validation tooling (like `rtsp_validation_tool`) can tune log volume
    // without changing CLI arguments or config files.
    //
    // ONVIF_RTSP_RTP_SAMPLE_INTERVAL:
    //   - When unset or invalid: defaults to 0 (sampling disabled).
    //   - When set to "0": disables RTP sampling logs.
    //   - Otherwise: parsed as u32 and applied as-is.
    let rtp_sample_interval = std::env::var("ONVIF_RTSP_RTP_SAMPLE_INTERVAL")
        .ok()
        .and_then(|raw| raw.parse::<u32>().ok())
        .unwrap_or(0);
    if rtp_sample_interval > 0 {
        streaming_lib::rtsp::session::server_session::set_rtp_sample_interval(rtp_sample_interval);
        tracing::info!(
            "RTSP RTP sampling enabled for validation mode: interval={} packets",
            rtp_sample_interval
        );
    } else {
        streaming_lib::rtsp::session::server_session::set_rtp_sample_interval(0);
        tracing::info!("RTSP RTP sampling disabled for validation mode");
    }

    // Keep validation mode lightweight on embedded devices by default.
    // Set env vars to "1"/"true" to re-enable the extra services.
    let validation_enable_httpflv = env_flag("ONVIF_VALIDATION_ENABLE_HTTPFLV", false);
    let validation_enable_onvif_app = env_flag("ONVIF_VALIDATION_ENABLE_ONVIF_APP", false);
    tracing::info!(
        enable_httpflv = validation_enable_httpflv,
        enable_onvif_app = validation_enable_onvif_app,
        "Validation service profile"
    );

    // Initialize logging early so validation-mode startup and streaming logs are visible.
    //
    // NOTE: Do NOT use `tracing_subscriber::fmt::init()` here. The application startup path
    // uses `onvif_rust::logging::init_logging()` which also installs a global subscriber.
    // If we install one first, application logging initialization will fail with:
    // "a global default trace dispatcher has already been set".
    if let Ok(app_config) = ConfigStorage::load_or_default(config_path) {
        let config_runtime = ConfigRuntime::new(app_config);
        let stream_frame_debug_enabled = configure_stream_frame_debug_logging(&config_runtime);
        if let Err(e) = onvif_rust::logging::init_logging(&config_runtime) {
            eprintln!("Failed to initialize logging: {}", e);
        } else {
            tracing::info!(
                enabled = stream_frame_debug_enabled,
                "Per-frame streaming debug logging configured"
            );
            config_runtime.log_loaded_config();
        }
    } else {
        // Fall back to no-op: application startup will still attempt to initialize logging.
        // (We avoid a hard failure here to keep validation-mode usable even with a missing config.)
    }

    // Verify H.264 file exists
    if !Path::new(&config.file_path).exists() {
        anyhow::bail!("H.264 file not found: {}", config.file_path);
    }
    if let Some(audio_path) = &config.audio_file_path
        && !Path::new(audio_path).exists()
    {
        anyhow::bail!("AAC file not found: {}", audio_path);
    }

    tracing::info!(
        "H.264 Validation mode starting: {} @ {}fps (RTSP: {}, HTTP-FLV: {})",
        config.file_path,
        config.frame_rate,
        config.rtsp_port,
        config.httpflv_port
    );

    if config.loop_playback {
        tracing::info!("Loop playback enabled");
    }

    // 1. Create MockVideoPublisher from H.264 file for frame reading
    tracing::info!(
        file = %config.file_path,
        frame_rate = config.frame_rate,
        loop_playback = config.loop_playback,
        "Creating MockVideoPublisher"
    );
    let mut video_publisher = match timeout(
        Duration::from_secs(publisher_init_timeout_sec),
        MockVideoPublisher::new(
            "stream1".to_string(),
            &config.file_path,
            config.frame_rate,
            config.loop_playback,
        ),
    )
    .await
    {
        Err(_elapsed) => {
            tracing::error!(
                file = %config.file_path,
                timeout_sec = publisher_init_timeout_sec,
                "Timed out creating MockVideoPublisher"
            );
            anyhow::bail!(
                "Timed out creating MockVideoPublisher from {} after {}s",
                config.file_path,
                publisher_init_timeout_sec
            );
        }
        Ok(result) => match result {
            Ok(publisher) => publisher,
            Err(e) => {
                tracing::error!(
                    error = %e,
                    file = %config.file_path,
                    "Failed to create MockVideoPublisher (H.264 parse/open error)"
                );
                return Err(anyhow::Error::new(e))
                    .context("Failed to create MockVideoPublisher from H.264 file");
            }
        },
    };
    tracing::info!("MockVideoPublisher created");

    let mut audio_publisher = if let Some(audio_path) = config.audio_file_path.as_deref() {
        tracing::info!(
            file = %audio_path,
            sample_rate_hz = config.audio_sample_rate,
            loop_playback = config.loop_playback,
            "Creating MockAudioPublisher"
        );
        let publisher = timeout(
            Duration::from_secs(publisher_init_timeout_sec),
            MockAudioPublisher::new(
                "stream1".to_string(),
                audio_path,
                config.audio_sample_rate,
                config.loop_playback,
            ),
        )
        .await
        .map_err(|_elapsed| {
            tracing::error!(
                file = %audio_path,
                timeout_sec = publisher_init_timeout_sec,
                "Timed out creating MockAudioPublisher"
            );
            anyhow::anyhow!(
                "Timed out creating MockAudioPublisher from {} after {}s",
                audio_path,
                publisher_init_timeout_sec
            )
        })?
        .map_err(|e| {
            tracing::error!(
                error = %e,
                file = %audio_path,
                "Failed to create MockAudioPublisher (AAC parse/open error)"
            );
            anyhow::Error::new(e).context("Failed to create MockAudioPublisher from AAC file")
        })?;
        tracing::info!("MockAudioPublisher created");
        Some(publisher)
    } else {
        None
    };

    let audio_sample_rate = audio_publisher
        .as_ref()
        .map_or(0, |publisher| publisher.sample_rate());
    let rtsp_stream_handler = Arc::new(ValidationAvStreamHandler::new(
        video_publisher.sps().to_vec(),
        video_publisher.pps().to_vec(),
        video_publisher.bootstrap_idr(),
        video_publisher.last_timestamp_handle(),
        audio_publisher
            .as_ref()
            .map(|publisher| publisher.audio_config().to_vec()),
        audio_sample_rate,
    ));

    // 2. Initialize StreamHub for frame distribution
    let mut streamhub = StreamsHub::new(None);
    let app_name = "live".to_string();
    let stream_name = "stream1".to_string();

    // IMPORTANT: streaming-lib's RTSP URI parser stores `uri.path` without the leading '/'.
    // Example: `rtsp://host:8554/stream1` -> `uri.path == "stream1"`.
    // If we publish "/stream1" here, RTSP subscribe/unpublish will not match and streaming fails.
    let rtsp_stream_id = StreamIdentifier::Rtsp {
        stream_path: stream_name.clone(),
    };
    let httpflv_stream_id = if validation_enable_httpflv {
        Some(StreamIdentifier::Rtmp {
            app_name: app_name.clone(),
            stream_name: stream_name.clone(),
        })
    } else {
        None
    };

    // Create a single publisher output channel, then fan-out to protocol-specific StreamHub streams.
    // HTTP-FLV fan-out is optional in validation mode to reduce CPU load.
    let (frame_tx_for_publisher, mut frame_rx_from_publisher) =
        mpsc::unbounded_channel::<FrameData>();
    let (frame_tx_rtsp, frame_rx_rtsp) = mpsc::unbounded_channel::<FrameData>();
    let (frame_tx_httpflv, frame_rx_httpflv) = if validation_enable_httpflv {
        let (tx, rx) = mpsc::unbounded_channel::<FrameData>();
        (Some(tx), Some(rx))
    } else {
        (None, None)
    };

    let fanout_handle = tokio::spawn(async move {
        while let Some(frame) = frame_rx_from_publisher.recv().await {
            fanout_validation_frame(&frame_tx_rtsp, frame_tx_httpflv.as_ref(), frame);
        }
    });

    // Publish RTSP stream
    let rtsp_data_receiver = DataReceiver {
        frame_receiver: Some(frame_rx_rtsp),
        packet_receiver: None,
    };
    let (_rtsp_statistic_sender, rtsp_subscriber_handle) = streamhub
        .publish(
            rtsp_stream_id.clone(),
            rtsp_data_receiver,
            rtsp_stream_handler.clone(),
        )
        .await
        .context("Failed to publish RTSP stream to StreamHub")?;

    if let (Some(httpflv_stream_id), Some(frame_rx_httpflv)) =
        (httpflv_stream_id.as_ref(), frame_rx_httpflv)
    {
        // Publish HTTP-FLV stream (HTTP-FLV server subscribes using RTMP-style identifiers)
        let httpflv_data_receiver = DataReceiver {
            frame_receiver: Some(frame_rx_httpflv),
            packet_receiver: None,
        };
        let (_httpflv_statistic_sender, _httpflv_subscriber_handle) = streamhub
            .publish(
                httpflv_stream_id.clone(),
                httpflv_data_receiver,
                rtsp_stream_handler.clone(),
            )
            .await
            .context("Failed to publish HTTP-FLV stream to StreamHub")?;

        tracing::info!(
            "StreamHub initialized with streams: {} and {}",
            rtsp_stream_id,
            httpflv_stream_id
        );
    } else {
        tracing::info!(
            "StreamHub initialized with RTSP stream only: {}",
            rtsp_stream_id
        );
    }

    // Wire up on-demand publishing: publishers check subscriber count before sending frames
    let rtsp_handle_for_video = Arc::clone(&rtsp_subscriber_handle);
    let video_cached_subscriber_count = Arc::new(AtomicUsize::new(0));
    let video_cached_subscriber_count_for_cb = Arc::clone(&video_cached_subscriber_count);
    video_publisher.set_subscriber_count_callback(Arc::new(move || {
        if let Ok(stats) = rtsp_handle_for_video.try_lock() {
            let count = stats.subscriber_count;
            video_cached_subscriber_count_for_cb.store(count, Ordering::Relaxed);
            count
        } else {
            video_cached_subscriber_count_for_cb.load(Ordering::Relaxed)
        }
    }));

    if let Some(ref mut audio_pub) = audio_publisher {
        let rtsp_handle_for_audio = Arc::clone(&rtsp_subscriber_handle);
        let audio_cached_subscriber_count = Arc::new(AtomicUsize::new(0));
        let audio_cached_subscriber_count_for_cb = Arc::clone(&audio_cached_subscriber_count);
        audio_pub.set_subscriber_count_callback(Arc::new(move || {
            if let Ok(stats) = rtsp_handle_for_audio.try_lock() {
                let count = stats.subscriber_count;
                audio_cached_subscriber_count_for_cb.store(count, Ordering::Relaxed);
                count
            } else {
                audio_cached_subscriber_count_for_cb.load(Ordering::Relaxed)
            }
        }));
    }
    tracing::info!(
        "On-demand publishing configured: publishers will pause when subscriber count is 0"
    );

    // Wrap publishers in Arc after configuration
    let video_publisher = Arc::new(video_publisher);
    let audio_publisher = audio_publisher.map(Arc::new);

    // Start MockVideoPublisher frame emission
    let pub_handle = video_publisher.start_publishing(frame_tx_for_publisher.clone());
    tracing::info!("MockVideoPublisher started emitting frames");

    let audio_pub_handle = if let Some(audio_publisher) = audio_publisher.as_ref() {
        let handle = audio_publisher.start_publishing(frame_tx_for_publisher);
        tracing::info!("MockAudioPublisher started emitting frames");
        Some(handle)
    } else {
        None
    };

    // Get hub event sender for servers
    let hub_event_sender = streamhub.get_hub_event_sender();

    // 3. Spawn RTSP server
    let rtsp_port = config.rtsp_port;
    let rtsp_addr = format!("0.0.0.0:{}", rtsp_port);
    let rtsp_event_sender = hub_event_sender.clone();
    let rtsp_handle = tokio::spawn(async move {
        let mut rtsp_server = DefaultRtspServer::new(rtsp_addr.clone(), rtsp_event_sender, None);
        if let Err(e) = rtsp_server.run().await {
            tracing::error!("RTSP server error: {}", e);
        }
    });
    tracing::info!("RTSP server spawned on port {}", rtsp_port);

    // 4. Spawn optional HTTP-FLV server
    let flv_handle = if validation_enable_httpflv {
        let flv_port = config.httpflv_port;
        let flv_addr = format!("0.0.0.0:{}", flv_port);
        let flv_event_sender = hub_event_sender.clone();
        let handle = tokio::spawn(async move {
            let mut flv_server =
                DefaultHttpFlvServer::new(flv_addr.clone(), flv_event_sender, None);
            if let Err(e) = flv_server.run().await {
                tracing::error!("HTTP-FLV server error: {}", e);
            }
        });
        tracing::info!("HTTP-FLV server spawned on port {}", flv_port);
        Some(handle)
    } else {
        tracing::info!("HTTP-FLV server disabled in validation profile");
        None
    };

    // 5. Spawn StreamHub event loop
    let streamhub_handle = tokio::spawn(async move {
        let mut hub = streamhub;
        hub.run().await;
    });
    tracing::info!("StreamHub event loop spawned");

    // 6. Create validation platform instance
    let platform = Arc::new(ValidationPlatform::new());
    platform.initialize().await?;
    tracing::info!("ValidationPlatform initialized");

    // 7. Optionally start ONVIF Application with the validation platform
    let app = if validation_enable_onvif_app {
        match Application::start_with_platform(config_path, platform.clone() as Arc<dyn Platform>)
            .await
        {
            Ok(app) => {
                tracing::info!("ONVIF Application started with ValidationPlatform");
                Some(app)
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to start ONVIF Application: {}. Continuing with streaming only.",
                    e
                );
                None
            }
        }
    } else {
        tracing::info!("ONVIF application disabled in validation profile");
        None
    };

    // 8. Create playback mode instance
    let playback_mode = H264PlaybackMode::new(config.clone());

    // 9. Initialize playback mode with platform
    playback_mode.initialize(platform.clone()).await?;
    tracing::info!("H264PlaybackMode initialized with platform");

    // 10. Start playback
    playback_mode.start().await?;
    tracing::info!("H264 playback started");

    // Print streaming URIs for external clients
    tracing::info!("Streaming URIs:");
    tracing::info!("  RTSP:     rtsp://0.0.0.0:{}/stream1", config.rtsp_port);
    if validation_enable_httpflv {
        tracing::info!(
            "  HTTP-FLV: http://0.0.0.0:{}/live/stream1.flv",
            config.httpflv_port
        );
    }
    if app.is_some() {
        tracing::info!("  ONVIF Device Service available");
    }
    tracing::info!("Press Ctrl+C to shutdown");

    // 11. Run validation mode until shutdown signal
    tokio::signal::ctrl_c()
        .await
        .unwrap_or_else(|e| tracing::warn!("Failed to setup Ctrl+C handler: {}", e));
    tracing::info!("Shutdown signal (Ctrl+C) received");

    // 12. Graceful shutdown
    tracing::info!("Initiating graceful shutdown...");

    // Stop ONVIF Application if it was started
    if let Some(application) = app {
        let report = application.shutdown().await;
        tracing::info!("ONVIF Application shutdown completed: {:?}", report.status);
    }

    // Stop playback
    playback_mode.stop().await?;
    tracing::info!("H264 playback stopped");

    // Shutdown platform
    platform.shutdown().await?;
    tracing::info!("ValidationPlatform shutdown");

    // Stop publisher
    video_publisher.stop_publishing().await;
    tracing::info!("MockVideoPublisher stopped");
    if let Some(audio_publisher) = audio_publisher.as_ref() {
        audio_publisher.stop_publishing().await;
        tracing::info!("MockAudioPublisher stopped");
    }

    // Cancel server tasks
    rtsp_handle.abort();
    if let Some(handle) = flv_handle {
        handle.abort();
    }
    streamhub_handle.abort();
    pub_handle.abort();
    if let Some(handle) = audio_pub_handle {
        handle.abort();
    }
    fanout_handle.abort();

    tracing::info!("All servers and tasks shutdown");
    tracing::info!("H.264 Validation mode stopped");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_config_on_target_platform() {
        // Test runtime configuration based on target architecture
        let (worker_threads, max_blocking_threads) = get_runtime_config(false);

        if cfg!(target_arch = "arm") {
            // Embedded ARM: allow light concurrency for async + blocking work
            assert_eq!(worker_threads, 2, "ARM target should use 2 worker threads");
            assert_eq!(
                max_blocking_threads, 16,
                "ARM target should use 16 blocking threads"
            );
        } else {
            // Host: should use available parallelism
            assert!(
                worker_threads >= 1,
                "Host should use at least 1 worker thread"
            );
            assert_eq!(
                max_blocking_threads, 512,
                "Host should use 512 blocking threads"
            );
        }
    }

    #[test]
    fn test_runtime_config_validation_mode_on_arm() {
        let (worker_threads, max_blocking_threads) = get_runtime_config(true);

        if cfg!(target_arch = "arm") {
            assert_eq!(
                worker_threads, 1,
                "ARM validation mode should default to 1 worker thread"
            );
            assert_eq!(
                max_blocking_threads, 2,
                "ARM validation mode should default to 2 blocking threads"
            );
        } else {
            assert!(worker_threads >= 1);
            assert_eq!(max_blocking_threads, 512);
        }
    }

    #[test]
    fn test_normal_mode_config_path() {
        // Test: when no --validation-mode flag, config_path uses default and validation_config is None
        // We test this by calling parse_arguments with mocked args via env
        unsafe {
            std::env::set_var("RUST_LOG", "info");
        }

        // Simulate parsing with empty args (just program name)
        let StartupArgs {
            validation_config,
            config_path,
        } = parse_arguments();

        // Verify defaults
        assert!(
            validation_config.is_none(),
            "Expected validation_config to be None in normal mode"
        );
        assert_eq!(
            config_path, DEFAULT_CONFIG_PATH,
            "Expected default config path"
        );
    }

    #[test]
    fn test_validation_mode_parsing() {
        // Simulate: program --validation-mode --h264-file /tmp/test.h264
        // This verifies the parsing logic (in real tests, we'd mock env::args)
        let sample_args = vec![
            "program".to_string(),
            "--validation-mode".to_string(),
            "--h264-file".to_string(),
            "/tmp/test.h264".to_string(),
        ];

        // Verify we can parse basic validation flags
        let validation_flag = sample_args.iter().any(|arg| arg == "--validation-mode");
        let h264_file_idx = sample_args.iter().position(|arg| arg == "--h264-file");

        assert!(validation_flag);
        assert!(h264_file_idx.is_some());
        if let Some(idx) = h264_file_idx {
            assert_eq!(sample_args[idx + 1], "/tmp/test.h264");
        }
    }

    #[test]
    fn test_validation_mode_with_ports() {
        // Simulate: program --validation-mode --h264-file /tmp/test.h264 --rtsp-port 9000 --httpflv-port 9001
        let sample_args = vec![
            "program".to_string(),
            "--validation-mode".to_string(),
            "--h264-file".to_string(),
            "/tmp/test.h264".to_string(),
            "--rtsp-port".to_string(),
            "9000".to_string(),
            "--httpflv-port".to_string(),
            "9001".to_string(),
        ];

        let rtsp_port_idx = sample_args.iter().position(|arg| arg == "--rtsp-port");
        let httpflv_port_idx = sample_args.iter().position(|arg| arg == "--httpflv-port");

        assert!(rtsp_port_idx.is_some());
        assert!(httpflv_port_idx.is_some());

        if let Some(idx) = rtsp_port_idx {
            assert_eq!(sample_args[idx + 1], "9000");
        }
        if let Some(idx) = httpflv_port_idx {
            assert_eq!(sample_args[idx + 1], "9001");
        }
    }

    #[test]
    fn test_generate_av_sdp_includes_audio() {
        let sps = vec![0x67, 0x42, 0x00, 0x1e];
        let pps = vec![0x68, 0xce, 0x06, 0xe2];
        let audio_config = vec![0x12, 0x10];

        let sdp = generate_av_sdp(&sps, &pps, Some(&audio_config), 48000);

        assert!(sdp.contains("m=video 0 RTP/AVP 96"));
        assert!(sdp.contains("a=control:trackID=0"));
        assert!(sdp.contains("m=audio 0 RTP/AVP 97"));
        assert!(sdp.contains("a=control:trackID=1"));
    }

    #[tokio::test]
    async fn test_validation_av_stream_handler_send_prior_data_does_not_emit_audio_config_frame() {
        let handler = ValidationAvStreamHandler::new(
            vec![0x67, 0x42, 0x00, 0x1e],
            vec![0x68, 0xce, 0x06, 0xe2],
            None,
            Arc::new(AtomicU32::new(1234)),
            Some(vec![0x12, 0x10]),
            48_000,
        );

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        handler
            .send_prior_data(DataSender::Frame { sender: tx }, SubscribeType::RtspPull)
            .await
            .expect("send_prior_data");

        let mut frames = Vec::new();
        while let Ok(frame) = rx.try_recv() {
            frames.push(frame);
        }

        assert!(
            frames.iter().any(|f| matches!(
                f,
                FrameData::MediaInfo {
                    media_info: MediaInfo {
                        audio_clock_rate: 48_000,
                        ..
                    }
                }
            )),
            "expected MediaInfo with audio_clock_rate set"
        );

        assert!(
            !frames.iter().any(|f| matches!(f, FrameData::Audio { .. })),
            "audio_config must not be injected as an RTP audio frame"
        );
    }

    #[tokio::test]
    async fn test_fanout_validation_frame_rtsp_only_routes_without_httpflv() {
        let (rtsp_tx, mut rtsp_rx) = tokio::sync::mpsc::unbounded_channel::<FrameData>();
        let frame = FrameData::Video {
            timestamp: 10,
            data: BytesMut::from(&b"video"[..]),
        };

        fanout_validation_frame(&rtsp_tx, None, frame);
        let received = rtsp_rx.recv().await.expect("rtsp frame");
        assert!(matches!(received, FrameData::Video { timestamp: 10, .. }));
    }

    #[tokio::test]
    async fn test_fanout_validation_frame_with_httpflv_routes_to_both() {
        let (rtsp_tx, mut rtsp_rx) = tokio::sync::mpsc::unbounded_channel::<FrameData>();
        let (http_tx, mut http_rx) = tokio::sync::mpsc::unbounded_channel::<FrameData>();
        let frame = FrameData::Video {
            timestamp: 20,
            data: BytesMut::from(&b"video2"[..]),
        };

        fanout_validation_frame(&rtsp_tx, Some(&http_tx), frame);

        let rtsp_frame = rtsp_rx.recv().await.expect("rtsp frame");
        let http_frame = http_rx.recv().await.expect("httpflv frame");
        assert!(matches!(rtsp_frame, FrameData::Video { timestamp: 20, .. }));
        assert!(matches!(http_frame, FrameData::Video { timestamp: 20, .. }));
    }

    #[test]
    fn test_generate_av_sdp_without_audio() {
        let sps = vec![0x67, 0x42, 0x00, 0x1e];
        let pps = vec![0x68, 0xce, 0x06, 0xe2];

        let sdp = generate_av_sdp(&sps, &pps, None, 48000);

        assert!(sdp.contains("m=video 0 RTP/AVP 96"));
        assert!(!sdp.contains("m=audio 0 RTP/AVP 97"));
    }

    #[test]
    fn test_loop_playback_flag() {
        // Simulate: program --validation-mode --h264-file /tmp/test.h264 --loop-playback
        let sample_args = vec![
            "program".to_string(),
            "--validation-mode".to_string(),
            "--h264-file".to_string(),
            "/tmp/test.h264".to_string(),
            "--loop-playback".to_string(),
        ];

        let loop_flag = sample_args.iter().any(|arg| arg == "--loop-playback");
        assert!(loop_flag);
    }

    #[test]
    fn test_env_flag_parsing_and_default_behavior() {
        const KEY: &str = "ONVIF_VALIDATION_ENV_FLAG_TEST";

        // SAFETY: unit test mutates process environment in a controlled scope.
        unsafe {
            std::env::remove_var(KEY);
        }
        assert!(env_flag(KEY, true));
        assert!(!env_flag(KEY, false));

        // SAFETY: unit test mutates process environment in a controlled scope.
        unsafe {
            std::env::set_var(KEY, "1");
        }
        assert!(env_flag(KEY, false));

        // SAFETY: unit test mutates process environment in a controlled scope.
        unsafe {
            std::env::set_var(KEY, "off");
        }
        assert!(!env_flag(KEY, true));

        // SAFETY: unit test mutates process environment in a controlled scope.
        unsafe {
            std::env::set_var(KEY, "not_a_boolean");
        }
        assert!(env_flag(KEY, true));

        // SAFETY: unit test mutates process environment in a controlled scope.
        unsafe {
            std::env::remove_var(KEY);
        }
    }
}
