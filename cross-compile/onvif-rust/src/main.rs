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
#[cfg(not(use_stubs))]
use onvif_rust::platform::AnykaPlatform;
use onvif_rust::platform::Platform;
#[cfg(use_stubs)]
use onvif_rust::platform::ValidationPlatform;
use onvif_rust::streaming::helpers::{
    combined_subscriber_count, fanout_frame, generate_av_sdp, send_frame,
    send_httpflv_prior_frames, spawn_httpflv_server, spawn_rtsp_server, spawn_streamhub_event_loop,
};
use onvif_rust::validation::h264_playback::{H264PlaybackConfig, H264PlaybackMode};
use onvif_rust::validation::httpflv_remux::ValidationHttpFlvRemuxer;
use onvif_rust::validation::stream_auth::build_stream_auth_for_validation_mode;
use portable_atomic::{AtomicU32, AtomicUsize, Ordering};
use std::backtrace::Backtrace;
use std::panic::PanicHookInfo;
use std::sync::{Arc, Once};
use streaming_lib::streamhub::define::{Information, InformationSender};
use streaming_lib::streamhub::errors::StreamHubError;
use streaming_lib::streamhub::statistics::StatisticsStream;
use streaming_lib::{
    DataSender, FrameData, MediaInfo, StreamIdentifier, StreamsHub, SubscribeType, TStreamHandler,
    VideoCodecType,
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

    match startup_args_from_cli(cli) {
        Ok(startup_args) => startup_args,
        Err(message) => {
            eprintln!("error: {}", message);
            std::process::exit(1);
        }
    }
}

fn startup_args_from_cli(cli: CliArgs) -> std::result::Result<StartupArgs, String> {
    let CliArgs {
        validation_mode,
        h264_file,
        aac_file,
        audio_sample_rate,
        rtsp_port,
        httpflv_port,
        loop_playback,
        config_path,
    } = cli;

    let validation_config = if validation_mode {
        let file_path =
            h264_file.ok_or_else(|| "--validation-mode requires --h264-file <path>".to_string())?;
        Some(H264PlaybackConfig {
            file_path,
            frame_rate: 25, // Default to 25 fps
            loop_playback,
            rtsp_port,
            httpflv_port,
            audio_file_path: aac_file,
            audio_sample_rate,
        })
    } else {
        None
    };

    Ok(StartupArgs {
        validation_config,
        config_path,
    })
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
        sub_type: SubscribeType,
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
            send_frame(&frame_sender, FrameData::MediaInfo { media_info })?;

            if matches!(sub_type, SubscribeType::RtmpRemux2HttpFlv) {
                let mut remuxer = ValidationHttpFlvRemuxer::new(
                    self.sps.clone(),
                    self.pps.clone(),
                    self.audio_config.clone(),
                    self.audio_sample_rate,
                );
                send_httpflv_prior_frames(
                    &frame_sender,
                    &mut remuxer,
                    timestamp,
                    self.bootstrap_idr.as_deref(),
                )?;
            } else {
                send_frame(
                    &frame_sender,
                    FrameData::Video {
                        timestamp,
                        data: BytesMut::from(self.sps.as_slice()),
                    },
                )?;
                send_frame(
                    &frame_sender,
                    FrameData::Video {
                        timestamp,
                        data: BytesMut::from(self.pps.as_slice()),
                    },
                )?;
                if let Some(idr) = self.bootstrap_idr.as_ref() {
                    send_frame(
                        &frame_sender,
                        FrameData::Video {
                            timestamp,
                            data: BytesMut::from(idr.as_slice()),
                        },
                    )?;
                }
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

/// Publisher initialization result containing video and optional audio publishers (unwrapped)
struct ValidationPublishers {
    video: streaming_lib::streamhub::mock_publisher::MockVideoPublisher,
    audio: Option<streaming_lib::streamhub::mock_audio_publisher::MockAudioPublisher>,
}

/// Initialize video publisher from H.264 file with timeout
async fn init_video_publisher(
    file_path: &str,
    frame_rate: u32,
    loop_playback: bool,
    timeout_sec: u64,
) -> Result<streaming_lib::streamhub::mock_publisher::MockVideoPublisher> {
    use streaming_lib::streamhub::mock_publisher::MockVideoPublisher;

    tracing::info!(
        file = %file_path,
        frame_rate,
        loop_playback,
        "Creating MockVideoPublisher"
    );

    match timeout(
        Duration::from_secs(timeout_sec),
        MockVideoPublisher::new("stream1".to_string(), file_path, frame_rate, loop_playback),
    )
    .await
    {
        Err(_elapsed) => {
            tracing::error!(file = %file_path, timeout_sec, "Timed out creating MockVideoPublisher");
            anyhow::bail!(
                "Timed out creating MockVideoPublisher from {} after {}s",
                file_path,
                timeout_sec
            )
        }
        Ok(result) => result.map_err(|e| {
            tracing::error!(error = %e, file = %file_path, "Failed to create MockVideoPublisher");
            anyhow::Error::new(e).context("Failed to create MockVideoPublisher from H.264 file")
        }),
    }
}

/// Initialize audio publisher from AAC file with timeout
async fn init_audio_publisher(
    file_path: &str,
    sample_rate: u32,
    loop_playback: bool,
    timeout_sec: u64,
) -> Result<streaming_lib::streamhub::mock_audio_publisher::MockAudioPublisher> {
    use streaming_lib::streamhub::mock_audio_publisher::MockAudioPublisher;

    tracing::info!(file = %file_path, sample_rate_hz = sample_rate, loop_playback, "Creating MockAudioPublisher");

    timeout(
        Duration::from_secs(timeout_sec),
        MockAudioPublisher::new("stream1".to_string(), file_path, sample_rate, loop_playback),
    )
    .await
    .map_err(|_elapsed| {
        tracing::error!(file = %file_path, timeout_sec, "Timed out creating MockAudioPublisher");
        anyhow::anyhow!(
            "Timed out creating MockAudioPublisher from {} after {}s",
            file_path,
            timeout_sec
        )
    })?
    .map_err(|e| {
        tracing::error!(error = %e, file = %file_path, "Failed to create MockAudioPublisher");
        anyhow::Error::new(e).context("Failed to create MockAudioPublisher from AAC file")
    })
}

/// Initialize both video and audio publishers for validation mode
async fn init_validation_publishers(
    config: &H264PlaybackConfig,
    timeout_sec: u64,
) -> Result<ValidationPublishers> {
    let video_publisher = init_video_publisher(
        &config.file_path,
        config.frame_rate,
        config.loop_playback,
        timeout_sec,
    )
    .await?;
    tracing::info!("MockVideoPublisher created");

    let audio_publisher = if let Some(audio_path) = config.audio_file_path.as_deref() {
        let publisher = init_audio_publisher(
            audio_path,
            config.audio_sample_rate,
            config.loop_playback,
            timeout_sec,
        )
        .await?;
        tracing::info!("MockAudioPublisher created");
        Some(publisher)
    } else {
        None
    };

    Ok(ValidationPublishers {
        video: video_publisher,
        audio: audio_publisher,
    })
}

/// Run the daemon in H.264 playback validation mode
async fn run_validation_mode(config: H264PlaybackConfig, config_path: &str) -> Result<()> {
    use tokio::sync::mpsc;

    let publisher_init_timeout_sec =
        parse_env_timeout("ONVIF_VALIDATION_PUBLISHER_INIT_TIMEOUT_SEC", 30);
    configure_rtp_sampling();
    tracing::info!("Validation mode: HTTP-FLV and ONVIF application enabled");
    init_validation_logging(config_path);
    validate_source_files(&config)?;

    let stream_auth = build_stream_auth(config_path)?;
    log_validation_startup(&config);

    let publishers = init_validation_publishers(&config, publisher_init_timeout_sec).await?;
    let (video_publisher, audio_publisher) = extract_publishers(publishers);
    let audio_sample_rate = audio_publisher.as_ref().map_or(0, |p| p.sample_rate());
    let video_sps = video_publisher.sps().to_vec();
    let video_pps = video_publisher.pps().to_vec();
    let video_bootstrap_idr = video_publisher.bootstrap_idr();
    let video_last_timestamp_handle = video_publisher.last_timestamp_handle();
    let audio_config = audio_publisher.as_ref().map(|p| p.audio_config().to_vec());

    let rtsp_stream_handler = Arc::new(ValidationAvStreamHandler::new(
        video_sps.clone(),
        video_pps.clone(),
        video_bootstrap_idr,
        video_last_timestamp_handle,
        audio_config.clone(),
        audio_sample_rate,
    ));

    let mut streamhub = StreamsHub::new(None);
    let (rtsp_stream_id, httpflv_stream_id) = create_stream_identifiers();
    let (frame_tx_for_publisher, frame_rx_from_publisher) = mpsc::unbounded_channel::<FrameData>();
    let (frame_tx_rtsp, frame_rx_rtsp) = mpsc::unbounded_channel::<FrameData>();
    let (frame_tx_httpflv, frame_rx_httpflv) = create_httpflv_channel();

    let fanout_handle = spawn_fanout_task(
        frame_rx_from_publisher,
        frame_tx_rtsp.clone(),
        frame_tx_httpflv,
        video_sps.clone(),
        video_pps.clone(),
        audio_config.clone(),
        audio_sample_rate,
    );

    let (rtsp_subscriber_handle, httpflv_subscriber_handle) = publish_to_streamhub(
        &mut streamhub,
        rtsp_stream_id.clone(),
        httpflv_stream_id.clone(),
        frame_rx_rtsp,
        frame_rx_httpflv,
        rtsp_stream_handler.clone(),
    )
    .await?;

    tracing::info!(
        "StreamHub initialized with streams: {} and {}",
        rtsp_stream_id,
        httpflv_stream_id
    );

    let (video_publisher, audio_publisher) = setup_subscriber_callbacks(
        video_publisher,
        audio_publisher,
        rtsp_subscriber_handle,
        httpflv_subscriber_handle,
    );
    tracing::info!(
        "On-demand publishing configured: publishers will pause when RTSP+HTTP-FLV subscriber count is 0"
    );

    let video_publisher = Arc::new(video_publisher);
    let audio_publisher = audio_publisher.map(Arc::new);
    let platform = create_and_init_platform().await?;

    let pub_handle = video_publisher.start_publishing(frame_tx_for_publisher.clone());
    tracing::info!("MockVideoPublisher started emitting frames");

    let audio_pub_handle = start_audio_publisher(&audio_publisher, frame_tx_for_publisher);

    let hub_event_sender = streamhub.get_hub_event_sender();
    let rtsp_handle = spawn_rtsp_server(
        hub_event_sender.clone(),
        stream_auth.clone(),
        config.rtsp_port,
    );
    let flv_handle = spawn_httpflv_server(hub_event_sender, stream_auth, config.httpflv_port);
    let streamhub_handle = spawn_streamhub_event_loop(streamhub);

    let app = start_onvif_application(config_path, platform.clone()).await;
    let playback_mode = init_playback_mode(config.clone(), platform.clone()).await?;

    log_streaming_uris(&config, app.is_some());

    tokio::signal::ctrl_c()
        .await
        .unwrap_or_else(|e| tracing::warn!("Failed to setup Ctrl+C handler: {}", e));
    tracing::info!("Shutdown signal (Ctrl+C) received");
    tracing::info!("Initiating graceful shutdown...");

    if let Some(application) = app {
        let report = application.shutdown().await;
        tracing::info!("ONVIF Application shutdown completed: {:?}", report.status);
    }

    playback_mode.stop().await?;
    tracing::info!("H264 playback stopped");

    video_publisher.stop_publishing().await;
    tracing::info!("MockVideoPublisher stopped");

    if let Some(audio_publisher) = audio_publisher.as_ref() {
        audio_publisher.stop_publishing().await;
        tracing::info!("MockAudioPublisher stopped");
    }

    abort_all_tasks(
        rtsp_handle,
        flv_handle,
        streamhub_handle,
        pub_handle,
        audio_pub_handle,
        fanout_handle,
    );

    platform.shutdown().await?;
    tracing::info!("Platform shutdown complete");

    tracing::info!("All servers and tasks shutdown");
    tracing::info!("H.264 Validation mode stopped");
    Ok(())
}

fn parse_env_timeout(var: &str, default: u64) -> u64 {
    std::env::var(var)
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(default)
}

fn configure_rtp_sampling() {
    let interval = std::env::var("ONVIF_RTSP_RTP_SAMPLE_INTERVAL")
        .ok()
        .and_then(|raw| raw.parse::<u32>().ok())
        .unwrap_or(0);
    streaming_lib::rtsp::session::server_session::set_rtp_sample_interval(interval);
    if interval > 0 {
        tracing::info!(
            "RTSP RTP sampling enabled for validation mode: interval={} packets",
            interval
        );
    } else {
        tracing::info!("RTSP RTP sampling disabled for validation mode");
    }
}

fn init_validation_logging(config_path: &str) {
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
    }
}

fn validate_source_files(config: &H264PlaybackConfig) -> Result<()> {
    use std::path::Path;
    if !Path::new(&config.file_path).exists() {
        anyhow::bail!("H.264 file not found: {}", config.file_path);
    }
    if let Some(audio_path) = &config.audio_file_path
        && !Path::new(audio_path).exists()
    {
        anyhow::bail!("AAC file not found: {}", audio_path);
    }
    Ok(())
}

fn build_stream_auth(config_path: &str) -> Result<Option<streaming_lib::common::auth::Auth>> {
    let stream_auth_config = ConfigRuntime::new(
        ConfigStorage::load_or_default(config_path)
            .context("Failed to load configuration for stream authentication")?,
    );
    build_stream_auth_for_validation_mode(&stream_auth_config, config_path)
        .context("Failed to initialize validation mode stream authentication")
}

fn log_validation_startup(config: &H264PlaybackConfig) {
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
}

fn extract_publishers(
    publishers: ValidationPublishers,
) -> (
    streaming_lib::streamhub::mock_publisher::MockVideoPublisher,
    Option<streaming_lib::streamhub::mock_audio_publisher::MockAudioPublisher>,
) {
    (publishers.video, publishers.audio)
}

fn create_stream_identifiers() -> (StreamIdentifier, StreamIdentifier) {
    let stream_name = "stream1".to_string();
    let app_name = "live".to_string();
    (
        StreamIdentifier::Rtsp {
            stream_path: stream_name.clone(),
        },
        StreamIdentifier::Rtmp {
            app_name,
            stream_name,
        },
    )
}

fn create_httpflv_channel() -> (
    Option<tokio::sync::mpsc::UnboundedSender<FrameData>>,
    Option<tokio::sync::mpsc::UnboundedReceiver<FrameData>>,
) {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<FrameData>();
    (Some(tx), Some(rx))
}

#[allow(clippy::too_many_arguments)]
fn spawn_fanout_task(
    mut frame_rx: tokio::sync::mpsc::UnboundedReceiver<FrameData>,
    frame_tx_rtsp: tokio::sync::mpsc::UnboundedSender<FrameData>,
    frame_tx_httpflv: Option<tokio::sync::mpsc::UnboundedSender<FrameData>>,
    video_sps: Vec<u8>,
    video_pps: Vec<u8>,
    audio_config: Option<Vec<u8>>,
    audio_sample_rate: u32,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut httpflv_remuxer = frame_tx_httpflv.as_ref().map(|_| {
            ValidationHttpFlvRemuxer::new(video_sps, video_pps, audio_config, audio_sample_rate)
        });
        while let Some(frame) = frame_rx.recv().await {
            fanout_frame(
                &frame_tx_rtsp,
                frame_tx_httpflv.as_ref(),
                httpflv_remuxer.as_mut(),
                frame,
            );
        }
    })
}

#[allow(clippy::too_many_arguments)]
async fn publish_to_streamhub(
    streamhub: &mut StreamsHub,
    rtsp_stream_id: StreamIdentifier,
    httpflv_stream_id: StreamIdentifier,
    frame_rx_rtsp: tokio::sync::mpsc::UnboundedReceiver<FrameData>,
    frame_rx_httpflv: Option<tokio::sync::mpsc::UnboundedReceiver<FrameData>>,
    stream_handler: Arc<ValidationAvStreamHandler>,
) -> Result<(
    Arc<tokio::sync::Mutex<StatisticsStream>>,
    Arc<tokio::sync::Mutex<StatisticsStream>>,
)> {
    use streaming_lib::streamhub::define::DataReceiver;

    let rtsp_data_receiver = DataReceiver {
        frame_receiver: Some(frame_rx_rtsp),
        packet_receiver: None,
    };
    let (_, rtsp_subscriber_handle) = streamhub
        .publish(rtsp_stream_id, rtsp_data_receiver, stream_handler.clone())
        .await
        .context("Failed to publish RTSP stream to StreamHub")?;

    let httpflv_data_receiver = DataReceiver {
        frame_receiver: frame_rx_httpflv,
        packet_receiver: None,
    };
    let (_, httpflv_subscriber_handle) = streamhub
        .publish(httpflv_stream_id, httpflv_data_receiver, stream_handler)
        .await
        .context("Failed to publish HTTP-FLV stream to StreamHub")?;

    Ok((rtsp_subscriber_handle, httpflv_subscriber_handle))
}

#[allow(clippy::type_complexity)]
fn setup_subscriber_callbacks(
    mut video_publisher: streaming_lib::streamhub::mock_publisher::MockVideoPublisher,
    mut audio_publisher: Option<streaming_lib::streamhub::mock_audio_publisher::MockAudioPublisher>,
    rtsp_handle: Arc<tokio::sync::Mutex<StatisticsStream>>,
    httpflv_handle: Arc<tokio::sync::Mutex<StatisticsStream>>,
) -> (
    streaming_lib::streamhub::mock_publisher::MockVideoPublisher,
    Option<streaming_lib::streamhub::mock_audio_publisher::MockAudioPublisher>,
) {
    let video_rtsp = Arc::clone(&rtsp_handle);
    let video_httpflv = Arc::clone(&httpflv_handle);
    let video_cached_rtsp = Arc::new(AtomicUsize::new(0));
    let video_cached_httpflv = Arc::new(AtomicUsize::new(0));
    let video_cached_rtsp_cb = Arc::clone(&video_cached_rtsp);
    let video_cached_httpflv_cb = Arc::clone(&video_cached_httpflv);
    video_publisher.set_subscriber_count_callback(Arc::new(move || {
        combined_subscriber_count(
            &video_rtsp,
            &video_httpflv,
            &video_cached_rtsp_cb,
            &video_cached_httpflv_cb,
        )
    }));

    if let Some(ref mut audio_pub) = audio_publisher {
        let audio_rtsp = Arc::clone(&rtsp_handle);
        let audio_httpflv = Arc::clone(&httpflv_handle);
        let audio_cached_rtsp = Arc::new(AtomicUsize::new(0));
        let audio_cached_httpflv = Arc::new(AtomicUsize::new(0));
        let audio_cached_rtsp_cb = Arc::clone(&audio_cached_rtsp);
        let audio_cached_httpflv_cb = Arc::clone(&audio_cached_httpflv);
        audio_pub.set_subscriber_count_callback(Arc::new(move || {
            combined_subscriber_count(
                &audio_rtsp,
                &audio_httpflv,
                &audio_cached_rtsp_cb,
                &audio_cached_httpflv_cb,
            )
        }));
    }

    (video_publisher, audio_publisher)
}

fn start_audio_publisher(
    audio_publisher: &Option<
        Arc<streaming_lib::streamhub::mock_audio_publisher::MockAudioPublisher>,
    >,
    frame_tx: tokio::sync::mpsc::UnboundedSender<FrameData>,
) -> Option<tokio::task::JoinHandle<()>> {
    audio_publisher.as_ref().map(|pub_| {
        let handle = pub_.start_publishing(frame_tx);
        tracing::info!("MockAudioPublisher started emitting frames");
        handle
    })
}

async fn create_and_init_platform() -> Result<Arc<dyn Platform>> {
    #[cfg(not(use_stubs))]
    {
        // On real hardware (ARM cross-compile), use AnykaPlatform which
        // provides actual PTZ, imaging, and network control via FFI.
        let platform = AnykaPlatform::new().context("Failed to create AnykaPlatform")?;
        platform.initialize().await?;
        tracing::info!("AnykaPlatform initialized (real hardware)");
        Ok(Arc::new(platform) as Arc<dyn Platform>)
    }
    #[cfg(use_stubs)]
    {
        // On development builds (x86_64), use ValidationPlatform for testing.
        let platform = ValidationPlatform::new();
        platform.initialize().await?;
        tracing::info!("ValidationPlatform initialized (stub mode)");
        Ok(Arc::new(platform) as Arc<dyn Platform>)
    }
}

async fn start_onvif_application(
    config_path: &str,
    platform: Arc<dyn Platform>,
) -> Option<Application> {
    match Application::start_with_platform(config_path, platform).await {
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
}

async fn init_playback_mode(
    config: H264PlaybackConfig,
    platform: Arc<dyn Platform>,
) -> Result<H264PlaybackMode> {
    let playback_mode = H264PlaybackMode::new(config.clone());
    playback_mode.initialize(platform).await?;
    tracing::info!("H264PlaybackMode initialized with platform");
    playback_mode.start().await?;
    tracing::info!("H264 playback started");
    Ok(playback_mode)
}

fn log_streaming_uris(config: &H264PlaybackConfig, has_onvif: bool) {
    tracing::info!("Streaming URIs:");
    tracing::info!("  RTSP:     rtsp://0.0.0.0:{}/stream1", config.rtsp_port);
    tracing::info!(
        "  HTTP-FLV: http://0.0.0.0:{}/live/stream1.flv",
        config.httpflv_port
    );
    if has_onvif {
        tracing::info!("  ONVIF Device Service available");
    }
    tracing::info!("Press Ctrl+C to shutdown");
}

fn abort_all_tasks(
    rtsp_handle: tokio::task::JoinHandle<()>,
    flv_handle: tokio::task::JoinHandle<()>,
    streamhub_handle: tokio::task::JoinHandle<()>,
    pub_handle: tokio::task::JoinHandle<()>,
    audio_pub_handle: Option<tokio::task::JoinHandle<()>>,
    fanout_handle: tokio::task::JoinHandle<()>,
) {
    rtsp_handle.abort();
    flv_handle.abort();
    streamhub_handle.abort();
    pub_handle.abort();
    if let Some(h) = audio_pub_handle {
        h.abort();
    }
    fanout_handle.abort();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic;
    use std::sync::Mutex;

    static PANIC_HOOK_TEST_GUARD: Mutex<()> = Mutex::new(());

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
    fn test_startup_args_from_cli_normal_mode_returns_default_config_path() {
        let cli = CliArgs::try_parse_from(["onvifd"]).expect("cli parse should succeed");

        let startup_args = startup_args_from_cli(cli).expect("startup args should be valid");

        assert!(startup_args.validation_config.is_none());
        assert_eq!(startup_args.config_path, DEFAULT_CONFIG_PATH);
    }

    #[test]
    fn test_startup_args_from_cli_validation_mode_returns_validation_config() {
        let cli = CliArgs::try_parse_from([
            "onvifd",
            "--validation-mode",
            "--h264-file",
            "/tmp/test.h264",
            "--aac-file",
            "/tmp/test.aac",
            "--audio-sample-rate",
            "44100",
            "--rtsp-port",
            "9000",
            "--httpflv-port",
            "9001",
            "--loop-playback",
            "/tmp/config.toml",
        ])
        .expect("cli parse should succeed");

        let startup_args = startup_args_from_cli(cli).expect("startup args should be valid");
        let validation_config = startup_args
            .validation_config
            .expect("validation config should exist");

        assert_eq!(validation_config.file_path, "/tmp/test.h264");
        assert_eq!(
            validation_config.audio_file_path.as_deref(),
            Some("/tmp/test.aac")
        );
        assert_eq!(validation_config.audio_sample_rate, 44_100);
        assert_eq!(validation_config.rtsp_port, 9000);
        assert_eq!(validation_config.httpflv_port, 9001);
        assert!(validation_config.loop_playback);
        assert_eq!(startup_args.config_path, "/tmp/config.toml");
    }

    #[test]
    fn test_startup_args_from_cli_validation_mode_missing_h264_file_returns_error() {
        let cli = CliArgs::try_parse_from(["onvifd", "--validation-mode"])
            .expect("cli parse should succeed");

        let err = startup_args_from_cli(cli).expect_err("h264 file should be required");
        assert_eq!(err, "--validation-mode requires --h264-file <path>");
    }

    #[test]
    fn test_startup_args_from_cli_validation_mode_minimal_uses_default_config_path() {
        let cli = CliArgs::try_parse_from([
            "onvifd",
            "--validation-mode",
            "--h264-file",
            "/tmp/minimal.h264",
        ])
        .expect("cli parse should succeed");

        let startup_args = startup_args_from_cli(cli).expect("startup args should be valid");
        let validation_config = startup_args
            .validation_config
            .expect("validation config should exist");

        assert_eq!(validation_config.file_path, "/tmp/minimal.h264");
        assert!(validation_config.audio_file_path.is_none());
        assert_eq!(validation_config.audio_sample_rate, 48000);
        assert_eq!(validation_config.rtsp_port, 8554);
        assert_eq!(validation_config.httpflv_port, 8080);
        assert!(!validation_config.loop_playback);
        assert_eq!(startup_args.config_path, DEFAULT_CONFIG_PATH);
    }

    #[test]
    fn test_panic_message_str_payload_returns_payload() {
        let _guard = PANIC_HOOK_TEST_GUARD.lock().expect("panic hook guard");
        let captured = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
        let captured_for_hook = std::sync::Arc::clone(&captured);
        let previous_hook = panic::take_hook();

        panic::set_hook(Box::new(move |panic_info| {
            let mut message = captured_for_hook.lock().expect("captured lock");
            *message = panic_message(panic_info);
        }));

        let result = panic::catch_unwind(|| panic!("string slice panic"));
        panic::set_hook(previous_hook);

        assert!(result.is_err());
        let message = captured.lock().expect("captured lock").clone();
        assert_eq!(message, "string slice panic");
    }

    #[test]
    fn test_panic_message_string_payload_returns_payload() {
        let _guard = PANIC_HOOK_TEST_GUARD.lock().expect("panic hook guard");
        let captured = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
        let captured_for_hook = std::sync::Arc::clone(&captured);
        let previous_hook = panic::take_hook();

        panic::set_hook(Box::new(move |panic_info| {
            let mut message = captured_for_hook.lock().expect("captured lock");
            *message = panic_message(panic_info);
        }));

        let result = panic::catch_unwind(|| panic::panic_any(String::from("owned panic")));
        panic::set_hook(previous_hook);

        assert!(result.is_err());
        let message = captured.lock().expect("captured lock").clone();
        assert_eq!(message, "owned panic");
    }

    #[test]
    fn test_panic_message_non_string_payload_returns_fallback_message() {
        let _guard = PANIC_HOOK_TEST_GUARD.lock().expect("panic hook guard");
        let captured = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
        let captured_for_hook = std::sync::Arc::clone(&captured);
        let previous_hook = panic::take_hook();

        panic::set_hook(Box::new(move |panic_info| {
            let mut message = captured_for_hook.lock().expect("captured lock");
            *message = panic_message(panic_info);
        }));

        let result = panic::catch_unwind(|| panic::panic_any(7_u8));
        panic::set_hook(previous_hook);

        assert!(result.is_err());
        let message = captured.lock().expect("captured lock").clone();
        assert_eq!(message, "non-string panic payload");
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
    async fn test_validation_av_stream_handler_send_prior_data_httpflv_emits_sequence_headers() {
        let handler = ValidationAvStreamHandler::new(
            vec![0x67, 0x42, 0x00, 0x1e],
            vec![0x68, 0xce, 0x06, 0xe2],
            Some(vec![0x65, 0x88, 0x84, 0x21, 0xA0]),
            Arc::new(AtomicU32::new(777)),
            Some(vec![0x12, 0x10]),
            48_000,
        );

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        handler
            .send_prior_data(
                DataSender::Frame { sender: tx },
                SubscribeType::RtmpRemux2HttpFlv,
            )
            .await
            .expect("send_prior_data");

        let mut saw_video_sequence_header = false;
        let mut saw_audio_sequence_header = false;
        while let Ok(frame) = rx.try_recv() {
            match frame {
                FrameData::Video { data, .. } if data.len() >= 2 => {
                    if data[1] == 0 {
                        saw_video_sequence_header = true;
                    }
                }
                FrameData::Audio { data, .. } if data.len() >= 2 => {
                    if data[1] == 0 {
                        saw_audio_sequence_header = true;
                    }
                }
                _ => {}
            }
        }

        assert!(saw_video_sequence_header, "expected AVC sequence header");
        assert!(saw_audio_sequence_header, "expected AAC sequence header");
    }

    #[tokio::test]
    async fn test_validation_av_stream_handler_send_prior_data_httpflv_ignores_malformed_bootstrap()
    {
        let handler = ValidationAvStreamHandler::new(
            vec![0x67, 0x42, 0x00, 0x1e],
            vec![0x68, 0xce, 0x06, 0xe2],
            Some(vec![0x00, 0x00, 0x00]),
            Arc::new(AtomicU32::new(778)),
            Some(vec![0x12, 0x10]),
            48_000,
        );

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        handler
            .send_prior_data(
                DataSender::Frame { sender: tx },
                SubscribeType::RtmpRemux2HttpFlv,
            )
            .await
            .expect("send_prior_data");

        let mut saw_video_sequence_header = false;
        while let Ok(frame) = rx.try_recv() {
            if let FrameData::Video { data, .. } = frame
                && data.len() >= 2
                && data[1] == 0
            {
                saw_video_sequence_header = true;
            }
        }

        assert!(saw_video_sequence_header, "expected AVC sequence header");
    }

    #[test]
    fn test_parse_env_timeout_returns_default_when_unset() {
        let value = parse_env_timeout("ONVIF_TEST_NONEXISTENT_TIMEOUT_VAR_12345", 99);
        assert_eq!(value, 99);
    }

    #[test]
    fn test_parse_env_timeout_parses_valid_value() {
        let var = "ONVIF_TEST_PARSE_ENV_TIMEOUT_VALUE";
        unsafe { std::env::set_var(var, "42") };
        let value = parse_env_timeout(var, 10);
        unsafe { std::env::remove_var(var) };
        assert_eq!(value, 42);
    }

    #[test]
    fn test_parse_env_timeout_zero_uses_default() {
        let var = "ONVIF_TEST_PARSE_ENV_TIMEOUT_ZERO";
        unsafe { std::env::set_var(var, "0") };
        let value = parse_env_timeout(var, 30);
        unsafe { std::env::remove_var(var) };
        assert_eq!(value, 30);
    }

    #[test]
    fn test_validate_source_files_missing_h264_returns_err() {
        let config = H264PlaybackConfig {
            file_path: "/nonexistent/path/to/file.h264".to_string(),
            frame_rate: 25,
            loop_playback: false,
            rtsp_port: 8554,
            httpflv_port: 8080,
            audio_file_path: None,
            audio_sample_rate: 48000,
        };
        let result = validate_source_files(&config);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("H.264 file not found"));
        assert!(err_msg.contains("file.h264"));
    }

    #[test]
    fn test_validate_source_files_missing_aac_returns_err() {
        let config = H264PlaybackConfig {
            file_path: ".".to_string(),
            frame_rate: 25,
            loop_playback: false,
            rtsp_port: 8554,
            httpflv_port: 8080,
            audio_file_path: Some("/nonexistent/audio.aac".to_string()),
            audio_sample_rate: 48000,
        };
        let result = validate_source_files(&config);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("AAC file not found"));
        assert!(err_msg.contains("audio.aac"));
    }

    #[test]
    fn test_startup_args_from_cli_normal_mode_custom_config_path() {
        let cli = CliArgs::try_parse_from(["onvifd", "/custom/config.toml"])
            .expect("cli parse should succeed");

        let startup_args = startup_args_from_cli(cli).expect("startup args should be valid");

        assert!(startup_args.validation_config.is_none());
        assert_eq!(startup_args.config_path, "/custom/config.toml");
    }

    // -------------------------------------------------------------------------
    // get_runtime_config env fallback (Phase 3 coverage)
    // -------------------------------------------------------------------------

    #[test]
    fn test_get_runtime_config_arm_env_invalid_workers_uses_default() {
        const VAR: &str = "ONVIF_TOKIO_WORKER_THREADS";
        unsafe { std::env::set_var(VAR, "x") };
        let (workers, blocking) = get_runtime_config(false);
        unsafe { std::env::remove_var(VAR) };

        if cfg!(target_arch = "arm") {
            assert_eq!(
                workers, 2,
                "invalid env should fall back to default workers"
            );
            assert_eq!(blocking, 16);
        } else {
            assert!(workers >= 1);
            assert_eq!(blocking, 512);
        }
    }

    #[test]
    fn test_get_runtime_config_arm_env_zero_workers_uses_default() {
        const VAR: &str = "ONVIF_TOKIO_WORKER_THREADS";
        unsafe { std::env::set_var(VAR, "0") };
        let (workers, _blocking) = get_runtime_config(false);
        unsafe { std::env::remove_var(VAR) };

        if cfg!(target_arch = "arm") {
            assert_eq!(workers, 2, "zero is filtered out, should use default");
        } else {
            assert!(workers >= 1);
        }
    }

    #[test]
    fn test_get_runtime_config_arm_env_valid_workers_uses_env() {
        const VAR: &str = "ONVIF_TOKIO_WORKER_THREADS";
        unsafe { std::env::set_var(VAR, "3") };
        let (workers, blocking) = get_runtime_config(false);
        unsafe { std::env::remove_var(VAR) };

        if cfg!(target_arch = "arm") {
            assert_eq!(workers, 3);
            assert_eq!(blocking, 16);
        } else {
            assert!(workers >= 1);
            assert_eq!(blocking, 512);
        }
    }

    #[test]
    fn test_get_runtime_config_arm_env_blocking_uses_env() {
        const VAR: &str = "ONVIF_TOKIO_MAX_BLOCKING_THREADS";
        unsafe { std::env::set_var(VAR, "8") };
        let (workers, blocking) = get_runtime_config(false);
        unsafe { std::env::remove_var(VAR) };

        if cfg!(target_arch = "arm") {
            assert_eq!(workers, 2);
            assert_eq!(blocking, 8);
        } else {
            assert!(workers >= 1);
            assert_eq!(blocking, 512);
        }
    }

    #[test]
    fn test_get_runtime_config_validation_mode_arm_defaults() {
        let (workers, blocking) = get_runtime_config(true);

        if cfg!(target_arch = "arm") {
            assert_eq!(workers, 1);
            assert_eq!(blocking, 2);
        } else {
            assert!(workers >= 1);
            assert_eq!(blocking, 512);
        }
    }

    #[test]
    fn test_get_runtime_config_arm_env_blocking_invalid_uses_default() {
        const VAR: &str = "ONVIF_TOKIO_MAX_BLOCKING_THREADS";
        unsafe { std::env::set_var(VAR, "invalid") };
        let (workers, blocking) = get_runtime_config(false);
        unsafe { std::env::remove_var(VAR) };

        if cfg!(target_arch = "arm") {
            assert_eq!(workers, 2);
            assert_eq!(
                blocking, 16,
                "invalid blocking env should fall back to default"
            );
        } else {
            assert!(workers >= 1);
            assert_eq!(blocking, 512);
        }
    }

    #[test]
    fn test_get_runtime_config_arm_env_blocking_zero_uses_default() {
        const VAR: &str = "ONVIF_TOKIO_MAX_BLOCKING_THREADS";
        unsafe { std::env::set_var(VAR, "0") };
        let (workers, blocking) = get_runtime_config(false);
        unsafe { std::env::remove_var(VAR) };

        if cfg!(target_arch = "arm") {
            assert_eq!(workers, 2);
            assert_eq!(blocking, 16, "zero blocking should fall back to default");
        } else {
            assert!(workers >= 1);
            assert_eq!(blocking, 512);
        }
    }
}
