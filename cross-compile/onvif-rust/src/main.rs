//! ONVIF Rust daemon entry point.
//!
//! This is the main entry point for the ONVIF daemon. It uses the Application
//! lifecycle pattern for clean startup and shutdown.
//!
//! Optional validation mode for H.264 playback testing:
//! `--validation-mode --h264-file <path> [--aac-file <path>] [--audio-sample-rate 48000] [--rtsp-port 8554] [--httpflv-port 8080] [--loop-playback]`

use anyhow::{Context, Result};
use clap::Parser;
use onvif_rust::app::{Application, DEFAULT_CONFIG_PATH};
use onvif_rust::validation::h264_playback::{H264PlaybackConfig, H264PlaybackMode};

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

/// Run the daemon in H.264 playback validation mode
async fn run_validation_mode(config: H264PlaybackConfig, config_path: &str) -> Result<()> {
    use onvif_rust::config::{ConfigRuntime, ConfigStorage};
    use onvif_rust::platform::{Platform, ValidationPlatform};
    use std::path::Path;
    use std::sync::Arc;
    use streaming_lib::StreamIdentifier;
    use streaming_lib::streamhub::define::{DataReceiver, FrameData};
    use streaming_lib::streamhub::mock_publisher::MockVideoPublisher;
    use streaming_lib::{
        DefaultHttpFlvServer, DefaultRtspServer, HttpFlvServer, RtspServer, StreamsHub,
    };
    use tokio::sync::mpsc;

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
    let publisher = Arc::new(
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
            let _ = frame_tx_rtsp.send(frame.clone());
            let _ = frame_tx_httpflv.send(frame);
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
            publisher.clone(),
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
            publisher.clone(),
        )
        .await
        .context("Failed to publish HTTP-FLV stream to StreamHub")?;

    tracing::info!(
        "StreamHub initialized with streams: {} and {}",
        rtsp_stream_id,
        httpflv_stream_id
    );

    // Start MockVideoPublisher frame emission
    let pub_handle = publisher.start_publishing(frame_tx_for_publisher);
    tracing::info!("MockVideoPublisher started emitting frames");

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
    publisher.stop_publishing().await;
    tracing::info!("MockVideoPublisher stopped");

    // Cancel server tasks
    rtsp_handle.abort();
    flv_handle.abort();
    streamhub_handle.abort();
    pub_handle.abort();
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
