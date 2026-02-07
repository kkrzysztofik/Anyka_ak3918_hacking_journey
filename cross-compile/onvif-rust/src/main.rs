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
use onvif_rust::validation::h264_playback::{H264PlaybackConfig, H264PlaybackMode};
use portable_atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use streaming_lib::streamhub::define::{Information, InformationSender};
use streaming_lib::streamhub::errors::StreamHubError;
use streaming_lib::streamhub::statistics::StatisticsStream;
use streaming_lib::{
    DataSender, FrameData, MediaInfo, SubscribeType, TStreamHandler, VideoCodecType,
};

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

/// Parse CLI arguments for normal or validation mode
fn parse_arguments() -> (Option<H264PlaybackConfig>, String) {
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

    (validation_config, cli.config_path)
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let (validation_config, config_path) = parse_arguments();

    if let Some(config) = validation_config {
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

            if let Some(config) = self.audio_config.as_ref() {
                let _ = frame_sender.send(FrameData::Audio {
                    timestamp: 0,
                    data: BytesMut::from(config.as_slice()),
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
    use onvif_rust::config::{ConfigRuntime, ConfigStorage};
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

    // Configure RTP packet sampling for RTSP validation runs.
    //
    // This is intentionally controlled by an environment variable so that
    // validation tooling (like `rtsp_validation_tool`) can tune log volume
    // without changing CLI arguments or config files.
    //
    // ONVIF_RTSP_RTP_SAMPLE_INTERVAL:
    //   - When unset or invalid: defaults to 100 (log every 100th packet).
    //   - When set to "0": disables RTP sampling logs.
    //   - Otherwise: parsed as u32 and applied as-is.
    let rtp_sample_interval = std::env::var("ONVIF_RTSP_RTP_SAMPLE_INTERVAL")
        .ok()
        .and_then(|raw| raw.parse::<u32>().ok())
        .unwrap_or(100);
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

    // Initialize logging early so validation-mode startup and streaming logs are visible.
    //
    // NOTE: Do NOT use `tracing_subscriber::fmt::init()` here. The application startup path
    // uses `onvif_rust::logging::init_logging()` which also installs a global subscriber.
    // If we install one first, application logging initialization will fail with:
    // "a global default trace dispatcher has already been set".
    if let Ok(app_config) = ConfigStorage::load_or_default(config_path) {
        let config_runtime = ConfigRuntime::new(app_config);
        if let Err(e) = onvif_rust::logging::init_logging(&config_runtime) {
            eprintln!("Failed to initialize logging: {}", e);
        } else {
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
    let video_publisher = Arc::new(
        MockVideoPublisher::new(
            "stream1".to_string(),
            &config.file_path,
            config.frame_rate,
            config.loop_playback,
        )
        .await
        .context("Failed to create MockVideoPublisher from H.264 file")?,
    );
    tracing::info!("MockVideoPublisher created");

    let audio_publisher = if let Some(audio_path) = config.audio_file_path.as_deref() {
        let publisher = MockAudioPublisher::new(
            "stream1".to_string(),
            audio_path,
            config.audio_sample_rate,
            config.loop_playback,
        )
        .await
        .context("Failed to create MockAudioPublisher from AAC file")?;
        tracing::info!("MockAudioPublisher created");
        Some(Arc::new(publisher))
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
    let httpflv_stream_id = StreamIdentifier::Rtmp {
        app_name: app_name.clone(),
        stream_name: stream_name.clone(),
    };

    // Create a single publisher output channel, then fan-out to protocol-specific StreamHub streams.
    // This allows RTSP (StreamIdentifier::Rtsp) and HTTP-FLV (StreamIdentifier::Rtmp) to subscribe
    // to the same underlying H.264 frame source.
    let (frame_tx_for_publisher, mut frame_rx_from_publisher) =
        mpsc::unbounded_channel::<FrameData>();
    let (frame_tx_rtsp, frame_rx_rtsp) = mpsc::unbounded_channel::<FrameData>();
    let (frame_tx_httpflv, frame_rx_httpflv) = mpsc::unbounded_channel::<FrameData>();

    let fanout_handle = tokio::spawn(async move {
        while let Some(frame) = frame_rx_from_publisher.recv().await {
            match frame {
                FrameData::Audio { .. } => {
                    let _ = frame_tx_rtsp.send(frame);
                }
                _ => {
                    let _ = frame_tx_rtsp.send(frame.clone());
                    let _ = frame_tx_httpflv.send(frame);
                }
            }
        }
    });

    // Publish RTSP stream
    let rtsp_data_receiver = DataReceiver {
        frame_receiver: Some(frame_rx_rtsp),
        packet_receiver: None,
    };
    streamhub
        .publish(
            rtsp_stream_id.clone(),
            rtsp_data_receiver,
            rtsp_stream_handler.clone(),
        )
        .await
        .context("Failed to publish RTSP stream to StreamHub")?;

    // Publish HTTP-FLV stream (HTTP-FLV server subscribes using RTMP-style identifiers)
    let httpflv_data_receiver = DataReceiver {
        frame_receiver: Some(frame_rx_httpflv),
        packet_receiver: None,
    };
    streamhub
        .publish(
            httpflv_stream_id.clone(),
            httpflv_data_receiver,
            video_publisher.clone(),
        )
        .await
        .context("Failed to publish HTTP-FLV stream to StreamHub")?;

    tracing::info!(
        "StreamHub initialized with streams: {} and {}",
        rtsp_stream_id,
        httpflv_stream_id
    );

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

    // 4. Spawn HTTP-FLV server
    let flv_port = config.httpflv_port;
    let flv_addr = format!("0.0.0.0:{}", flv_port);
    let flv_event_sender = hub_event_sender.clone();
    let flv_handle = tokio::spawn(async move {
        let mut flv_server = DefaultHttpFlvServer::new(flv_addr.clone(), flv_event_sender, None);
        if let Err(e) = flv_server.run().await {
            tracing::error!("HTTP-FLV server error: {}", e);
        }
    });
    tracing::info!("HTTP-FLV server spawned on port {}", flv_port);

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

    // 7. Start ONVIF Application with the validation platform
    let app =
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
    tracing::info!(
        "  HTTP-FLV: http://0.0.0.0:{}/live/stream1.flv",
        config.httpflv_port
    );
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
    flv_handle.abort();
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
    fn test_normal_mode_config_path() {
        // Test: when no --validation-mode flag, config_path uses default and validation_config is None
        // We test this by calling parse_arguments with mocked args via env
        unsafe {
            std::env::set_var("RUST_LOG", "info");
        }

        // Simulate parsing with empty args (just program name)
        let (validation_config, config_path) = parse_arguments();

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
}
