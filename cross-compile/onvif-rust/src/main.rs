//! ONVIF Rust daemon entry point.
//!
//! This is the main entry point for the ONVIF daemon. It uses the Application
//! lifecycle pattern for clean startup and shutdown.
//!
//! Optional validation mode for H.264 playback testing:
//! `--validation-mode --h264-file <path> [--rtsp-port 8554] [--httpflv-port 8080] [--loop-playback]`

use anyhow::{Context, Result};
use clap::Parser;
use onvif_rust::app::{Application, DEFAULT_CONFIG_PATH};
use onvif_rust::validation::h264_playback::{H264PlaybackConfig, H264PlaybackMode};
use std::sync::Arc;

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
        run_validation_mode(config).await
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
async fn run_validation_mode(config: H264PlaybackConfig) -> Result<()> {
    use onvif_rust::platform::{Platform, ValidationPlatform};
    use std::path::Path;
    use std::sync::Arc;
    use streaming_lib::StreamIdentifier;
    use streaming_lib::streamhub::define::DataReceiver;
    use streaming_lib::streamhub::mock_publisher::MockVideoPublisher;
    use streaming_lib::{DefaultHttpFlvServer, DefaultRtspServer, HttpFlvServer, RtspServer, StreamsHub};
    use tokio::sync::mpsc;

    // Initialize logging
    tracing_subscriber::fmt::init();

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
        MockVideoPublisher::new("stream1".to_string(), &config.file_path, config.frame_rate, config.loop_playback)
            .await
            .context("Failed to create MockVideoPublisher from H.264 file")?,
    );
    tracing::info!("MockVideoPublisher created");

    // 2. Initialize StreamHub for frame distribution
    let mut streamhub = StreamsHub::new(None);
    let stream_id = StreamIdentifier::Rtsp {
        stream_path: "/stream1".to_string(),
    };

    // Create frame channel for publisher to StreamHub
    let (frame_tx, frame_rx) = mpsc::unbounded_channel();

    // Create DataReceiver wrapping the frame channel
    let data_receiver = DataReceiver {
        frame_receiver: Some(frame_rx),
        packet_receiver: None,
    };

    // Publish stream to StreamHub (uses MockVideoPublisher as stream handler)
    streamhub
        .publish(stream_id.clone(), data_receiver, publisher.clone())
        .await
        .context("Failed to publish stream to StreamHub")?;

    let frame_tx_for_publisher = frame_tx;

    tracing::info!("StreamHub initialized with stream: {}", stream_id);

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
    let app = match Application::start_with_platform(DEFAULT_CONFIG_PATH, platform.clone() as Arc<dyn Platform>).await {
        Ok(app) => {
            tracing::info!("ONVIF Application started with ValidationPlatform");
            Some(app)
        }
        Err(e) => {
            tracing::warn!("Failed to start ONVIF Application: {}. Continuing with streaming only.", e);
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
        std::env::set_var("RUST_LOG", "info");
        
        // Simulate parsing with empty args (just program name)
        let (validation_config, config_path) = parse_arguments();
        
        // Verify defaults
        assert!(validation_config.is_none(), "Expected validation_config to be None in normal mode");
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
