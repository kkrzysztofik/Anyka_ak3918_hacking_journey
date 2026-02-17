//! Streaming service orchestration and live stream handler.
//!
//! [`StreamingService`] manages the full lifecycle of RTSP and HTTP-FLV
//! streaming: creating the `StreamsHub`, publishing streams, spawning servers,
//! and running fanout tasks that route frames from the bridge to subscribers.
//!
//! [`LiveStreamHandler`] implements [`TStreamHandler`] to serve prior data
//! (SPS/PPS, FLV sequence headers) and SDP information to late-joining
//! subscribers using dynamically-cached state from the [`StreamingBridge`].

use std::ops::Deref;
use std::sync::Arc;

use async_trait::async_trait;
use bytes::BytesMut;
use portable_atomic::Ordering;

use super::bridge::StreamState;
use streaming_lib::streamhub::define::{
    DataReceiver, DataSender, Information, InformationSender, MediaInfo, SubscribeType,
    VideoCodecType,
};
use streaming_lib::streamhub::errors::StreamHubError;
use streaming_lib::streamhub::statistics::StatisticsStream;
use streaming_lib::streamhub::stream::StreamIdentifier;
use streaming_lib::{FrameData, StreamsHub, TStreamHandler};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use super::bridge::StreamingBridge;
use super::config::StreamingConfig;
use super::helpers::{
    fanout_frame, generate_av_sdp, send_frame, send_httpflv_prior_frames, spawn_httpflv_server,
    spawn_rtsp_server, spawn_streamhub_event_loop,
};
use crate::validation::httpflv_remux::ValidationHttpFlvRemuxer;

/// Per-stream handler that reads cached SPS/PPS/IDR from the bridge.
///
/// Unlike `ValidationAvStreamHandler` which holds static copies of SPS/PPS,
/// this handler reads the latest values from the bridge's `StreamState`, so
/// it can serve up-to-date codec parameters even if the encoder changes
/// resolution or other settings mid-stream.
pub struct LiveStreamHandler {
    /// Whether this handler serves the main stream (vs sub).
    is_main: bool,
    /// Reference to the bridge for reading per-stream state.
    bridge: Arc<StreamingBridge>,
}

impl LiveStreamHandler {
    /// Create a handler for a specific stream.
    pub fn new(is_main: bool, bridge: Arc<StreamingBridge>) -> Self {
        Self { is_main, bridge }
    }

    /// Get the stream state this handler reads from.
    fn stream(&self) -> &StreamState {
        if self.is_main {
            &self.bridge.main_stream
        } else {
            &self.bridge.sub_stream
        }
    }
}

#[async_trait]
impl TStreamHandler for LiveStreamHandler {
    async fn send_prior_data(
        &self,
        sender: DataSender,
        sub_type: SubscribeType,
    ) -> Result<(), StreamHubError> {
        if let DataSender::Frame {
            sender: frame_sender,
        } = sender
        {
            let stream = self.stream();
            let timestamp = stream.last_timestamp_ms.load(Ordering::Relaxed);
            let audio_config = self.bridge.audio_config.read().deref().clone();
            let audio_clock_rate = if audio_config.is_some() {
                self.bridge.audio_sample_rate
            } else {
                0
            };

            let media_info = MediaInfo {
                audio_clock_rate,
                video_clock_rate: 90000,
                vcodec: VideoCodecType::H264,
            };
            send_frame(&frame_sender, FrameData::MediaInfo { media_info })?;

            let sps = stream.sps.read().deref().clone();
            let pps = stream.pps.read().deref().clone();
            let bootstrap_idr = stream.bootstrap_idr.read().deref().clone();

            if let (Some(sps), Some(pps)) = (sps, pps) {
                if matches!(sub_type, SubscribeType::RtmpRemux2HttpFlv) {
                    let mut remuxer = ValidationHttpFlvRemuxer::new(
                        sps,
                        pps,
                        audio_config,
                        self.bridge.audio_sample_rate,
                    );
                    send_httpflv_prior_frames(
                        &frame_sender,
                        &mut remuxer,
                        timestamp,
                        bootstrap_idr.as_deref(),
                    )?;
                } else {
                    send_frame(
                        &frame_sender,
                        FrameData::Video {
                            timestamp,
                            data: BytesMut::from(sps.as_slice()),
                        },
                    )?;
                    send_frame(
                        &frame_sender,
                        FrameData::Video {
                            timestamp,
                            data: BytesMut::from(pps.as_slice()),
                        },
                    )?;
                    if let Some(idr) = bootstrap_idr.as_ref() {
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
        }

        Ok(())
    }

    async fn get_statistic_data(&self) -> Option<StatisticsStream> {
        None
    }

    async fn send_information(&self, sender: InformationSender) {
        let stream = self.stream();
        let sps = stream.sps.read().deref().clone();
        let pps = stream.pps.read().deref().clone();
        let audio_config = self.bridge.audio_config.read().deref().clone();

        if let (Some(sps), Some(pps)) = (sps, pps) {
            let sdp = generate_av_sdp(
                &sps,
                &pps,
                audio_config.as_deref(),
                self.bridge.audio_sample_rate,
            );
            let _ = sender.send(Information::Sdp { data: sdp });
        }
    }
}

/// Orchestrates the full streaming pipeline for normal mode.
///
/// Creates the `StreamsHub`, publishes main and sub streams (both RTSP and
/// HTTP-FLV), spawns RTSP and HTTP-FLV servers, and runs fanout tasks that
/// route frames from the bridge channels to the server channels.
pub struct StreamingService {
    /// The frame bridge shared with the platform callback.
    bridge: Arc<StreamingBridge>,
    /// Configuration for ports, stream names, etc.
    config: StreamingConfig,
    /// Spawned task handles for graceful shutdown.
    rtsp_task: Option<JoinHandle<()>>,
    httpflv_task: Option<JoinHandle<()>>,
    streamhub_task: Option<JoinHandle<()>>,
    main_fanout_task: Option<JoinHandle<()>>,
    sub_fanout_task: Option<JoinHandle<()>>,
}

impl StreamingService {
    /// Create a new streaming service with the given configuration.
    ///
    /// Does not start any servers. Call [`start`](Self::start) to begin.
    pub fn new(config: StreamingConfig) -> Self {
        let (main_tx, _main_rx) = mpsc::unbounded_channel();
        let (sub_tx, _sub_rx) = mpsc::unbounded_channel();

        // These dummy channels are replaced in start(). We create the bridge
        // here so that `bridge()` is available even before start(), but the
        // channels won't be used until start() wires them up properly.
        let bridge = Arc::new(StreamingBridge::new(main_tx, sub_tx, 0));

        Self {
            bridge,
            config,
            rtsp_task: None,
            httpflv_task: None,
            streamhub_task: None,
            main_fanout_task: None,
            sub_fanout_task: None,
        }
    }

    /// Start the streaming infrastructure.
    ///
    /// Creates the StreamsHub, publishes both streams, spawns servers, and
    /// starts fanout tasks. Returns the bridge that should be registered with
    /// the platform as a `FrameCallback`.
    pub async fn start(&mut self) -> Result<Arc<StreamingBridge>, anyhow::Error> {
        Self::verify_port_available(self.config.rtsp_port, "RTSP")?;
        Self::verify_port_available(self.config.httpflv_port, "HTTP-FLV")?;

        let mut streamhub = StreamsHub::new(None);

        // Create per-stream frame channels (bridge → fanout).
        let (main_bridge_tx, main_bridge_rx) = mpsc::unbounded_channel::<FrameData>();
        let (sub_bridge_tx, sub_bridge_rx) = mpsc::unbounded_channel::<FrameData>();

        // Re-create the bridge with real channels.
        self.bridge = Arc::new(StreamingBridge::new(
            main_bridge_tx,
            sub_bridge_tx,
            self.config.audio_sample_rate,
        ));

        // Publish main stream (RTSP + HTTP-FLV).
        let main_fanout = self
            .publish_stream(
                &mut streamhub,
                &self.config.main_stream_name.clone(),
                main_bridge_rx,
            )
            .await?;

        // Publish sub stream (RTSP + HTTP-FLV).
        let sub_fanout = self
            .publish_stream(
                &mut streamhub,
                &self.config.sub_stream_name.clone(),
                sub_bridge_rx,
            )
            .await?;

        // Spawn servers.
        let hub_event_sender = streamhub.get_hub_event_sender();
        self.rtsp_task = Some(spawn_rtsp_server(
            hub_event_sender.clone(),
            self.config.auth.clone(),
            self.config.rtsp_port,
        ));
        self.httpflv_task = Some(spawn_httpflv_server(
            hub_event_sender,
            self.config.auth.clone(),
            self.config.httpflv_port,
        ));
        self.streamhub_task = Some(spawn_streamhub_event_loop(streamhub));

        self.main_fanout_task = Some(main_fanout);
        self.sub_fanout_task = Some(sub_fanout);

        tracing::info!(
            rtsp_port = self.config.rtsp_port,
            httpflv_port = self.config.httpflv_port,
            main_stream = %self.config.main_stream_name,
            sub_stream = %self.config.sub_stream_name,
            "Streaming service started"
        );

        Ok(Arc::clone(&self.bridge))
    }

    fn verify_port_available(port: u16, service: &'static str) -> Result<(), anyhow::Error> {
        std::net::TcpListener::bind(("0.0.0.0", port))
            .map(drop)
            .map_err(|error| {
                anyhow::anyhow!(
                    "{} port {} is unavailable for startup: {}",
                    service,
                    port,
                    error
                )
            })
    }

    /// Get a reference to the streaming bridge.
    pub fn bridge(&self) -> &Arc<StreamingBridge> {
        &self.bridge
    }

    /// Gracefully shut down all streaming tasks.
    pub async fn shutdown(&mut self) {
        tracing::info!("Shutting down streaming service...");

        // Abort all tasks.
        for task in [
            self.main_fanout_task.take(),
            self.sub_fanout_task.take(),
            self.rtsp_task.take(),
            self.httpflv_task.take(),
            self.streamhub_task.take(),
        ]
        .into_iter()
        .flatten()
        {
            task.abort();
        }

        tracing::info!("Streaming service shutdown complete");
    }

    /// Publish a single stream (both RTSP and HTTP-FLV) to the StreamsHub
    /// and spawn a fanout task that routes frames from the bridge channel
    /// to the two server channels.
    async fn publish_stream(
        &self,
        streamhub: &mut StreamsHub,
        stream_name: &str,
        mut bridge_rx: mpsc::UnboundedReceiver<FrameData>,
    ) -> Result<JoinHandle<()>, anyhow::Error> {
        let app_name = self.config.app_name.clone();

        // Create RTSP channel.
        let (rtsp_tx, rtsp_rx) = mpsc::unbounded_channel::<FrameData>();
        let rtsp_id = StreamIdentifier::Rtsp {
            stream_path: stream_name.to_string(),
        };

        // Create HTTP-FLV channel.
        let (httpflv_tx, httpflv_rx) = mpsc::unbounded_channel::<FrameData>();
        let httpflv_id = StreamIdentifier::Rtmp {
            app_name: app_name.clone(),
            stream_name: stream_name.to_string(),
        };

        // The handler references the bridge's actual stream state so it reads
        // the latest SPS/PPS/IDR that the bridge caches from live frames.
        let is_main = stream_name == self.config.main_stream_name;
        let handler = Arc::new(LiveStreamHandler::new(is_main, Arc::clone(&self.bridge)));

        // Publish RTSP stream.
        let rtsp_receiver = DataReceiver {
            frame_receiver: Some(rtsp_rx),
            packet_receiver: None,
        };
        streamhub
            .publish(rtsp_id.clone(), rtsp_receiver, handler.clone())
            .await
            .map_err(|e| {
                anyhow::anyhow!("Failed to publish RTSP stream '{}': {}", stream_name, e)
            })?;

        // Publish HTTP-FLV stream.
        let httpflv_receiver = DataReceiver {
            frame_receiver: Some(httpflv_rx),
            packet_receiver: None,
        };
        streamhub
            .publish(httpflv_id.clone(), httpflv_receiver, handler)
            .await
            .map_err(|e| {
                anyhow::anyhow!("Failed to publish HTTP-FLV stream '{}': {}", stream_name, e)
            })?;

        tracing::info!(
            stream = %stream_name,
            rtsp = %rtsp_id,
            httpflv = %httpflv_id,
            "Published stream to StreamsHub"
        );

        // Clone bridge ref for the fanout task to read SPS/PPS for remuxer init.
        let bridge_ref = Arc::clone(&self.bridge);

        // Spawn fanout task: bridge_rx → rtsp_tx + httpflv_tx.
        let fanout_handle = tokio::spawn(async move {
            let mut httpflv_remuxer: Option<ValidationHttpFlvRemuxer> = None;
            let mut cached_sps: Option<Vec<u8>> = None;
            let mut cached_pps: Option<Vec<u8>> = None;

            while let Some(frame) = bridge_rx.recv().await {
                let stream = if is_main {
                    &bridge_ref.main_stream
                } else {
                    &bridge_ref.sub_stream
                };

                let current_sps = stream.sps.read().deref().clone();
                let current_pps = stream.pps.read().deref().clone();

                // Check if SPS/PPS have changed or remuxer needs initialization
                let needs_refresh = httpflv_remuxer.is_none()
                    || current_sps != cached_sps
                    || current_pps != cached_pps;

                if needs_refresh
                    && let (Some(sps), Some(pps)) = (current_sps.clone(), current_pps.clone())
                {
                    let audio_config = bridge_ref.audio_config.read().deref().clone();
                    httpflv_remuxer = Some(ValidationHttpFlvRemuxer::new(
                        sps.clone(),
                        pps.clone(),
                        audio_config,
                        bridge_ref.audio_sample_rate,
                    ));
                    cached_sps = Some(sps);
                    cached_pps = Some(pps);
                }

                // Always fan out to RTSP. HTTP-FLV gets the remuxed version.
                fanout_frame(&rtsp_tx, Some(&httpflv_tx), httpflv_remuxer.as_mut(), frame);
            }
        });

        Ok(fanout_handle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create a bridge and handler targeting the main stream.
    fn make_main_handler() -> (Arc<StreamingBridge>, LiveStreamHandler) {
        let (main_tx, _) = mpsc::unbounded_channel();
        let (sub_tx, _) = mpsc::unbounded_channel();
        let bridge = Arc::new(StreamingBridge::new(main_tx, sub_tx, 48000));
        let handler = LiveStreamHandler::new(true, Arc::clone(&bridge));
        (bridge, handler)
    }

    #[test]
    fn test_streaming_service_creates_with_config() {
        let config = StreamingConfig::default();
        let service = StreamingService::new(config);
        assert!(service.rtsp_task.is_none());
        assert!(service.httpflv_task.is_none());
        assert!(service.streamhub_task.is_none());
    }

    #[test]
    fn test_streaming_service_bridge_accessible_before_start() {
        let config = StreamingConfig::default();
        let service = StreamingService::new(config);
        let _bridge = service.bridge();
    }

    #[tokio::test]
    async fn test_live_stream_handler_send_prior_data_rtsp_no_sps() {
        let (_bridge, handler) = make_main_handler();
        let (frame_tx, mut frame_rx) = mpsc::unbounded_channel::<FrameData>();
        let sender = DataSender::Frame { sender: frame_tx };

        let result = handler
            .send_prior_data(sender, SubscribeType::RtspPull)
            .await;
        assert!(result.is_ok());

        // Should receive MediaInfo only (no SPS/PPS cached).
        let first = frame_rx.try_recv().unwrap();
        assert!(matches!(first, FrameData::MediaInfo { .. }));
        assert!(frame_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn test_live_stream_handler_send_prior_data_rtsp_with_sps_pps() {
        let (bridge, handler) = make_main_handler();

        // Populate the bridge's main stream state.
        *bridge.main_stream.sps.write() = Some(vec![0x67, 0x42, 0x00, 0x1e]);
        *bridge.main_stream.pps.write() = Some(vec![0x68, 0xce, 0x06, 0xe2]);
        *bridge.main_stream.bootstrap_idr.write() = Some(vec![0x65, 0x88, 0x84, 0x21]);
        bridge
            .main_stream
            .last_timestamp_ms
            .store(2000, Ordering::Relaxed);

        let (frame_tx, mut frame_rx) = mpsc::unbounded_channel::<FrameData>();
        let sender = DataSender::Frame { sender: frame_tx };

        let result = handler
            .send_prior_data(sender, SubscribeType::RtspPull)
            .await;
        assert!(result.is_ok());

        // MediaInfo + SPS + PPS + IDR = 4 frames.
        let media_info = frame_rx.try_recv().unwrap();
        assert!(matches!(media_info, FrameData::MediaInfo { .. }));

        let sps_frame = frame_rx.try_recv().unwrap();
        assert!(matches!(
            sps_frame,
            FrameData::Video {
                timestamp: 2000,
                ..
            }
        ));

        let pps_frame = frame_rx.try_recv().unwrap();
        assert!(matches!(
            pps_frame,
            FrameData::Video {
                timestamp: 2000,
                ..
            }
        ));

        let idr_frame = frame_rx.try_recv().unwrap();
        assert!(matches!(
            idr_frame,
            FrameData::Video {
                timestamp: 2000,
                ..
            }
        ));

        assert!(frame_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn test_live_stream_handler_send_prior_data_httpflv() {
        let (bridge, handler) = make_main_handler();

        *bridge.main_stream.sps.write() = Some(vec![0x67, 0x42, 0x00, 0x1e]);
        *bridge.main_stream.pps.write() = Some(vec![0x68, 0xce, 0x06, 0xe2]);
        bridge
            .main_stream
            .last_timestamp_ms
            .store(3000, Ordering::Relaxed);

        let (frame_tx, mut frame_rx) = mpsc::unbounded_channel::<FrameData>();
        let sender = DataSender::Frame { sender: frame_tx };

        let result = handler
            .send_prior_data(sender, SubscribeType::RtmpRemux2HttpFlv)
            .await;
        assert!(result.is_ok());

        // MediaInfo + FLV video sequence header = 2 frames.
        let media_info = frame_rx.try_recv().unwrap();
        assert!(matches!(media_info, FrameData::MediaInfo { .. }));

        let sequence_header = frame_rx.try_recv().unwrap();
        assert!(matches!(
            sequence_header,
            FrameData::Video {
                timestamp: 3000,
                ..
            }
        ));
    }

    #[tokio::test]
    async fn test_live_stream_handler_send_information_sdp() {
        let (bridge, handler) = make_main_handler();

        *bridge.main_stream.sps.write() = Some(vec![0x67, 0x42, 0x00, 0x1e]);
        *bridge.main_stream.pps.write() = Some(vec![0x68, 0xce, 0x06, 0xe2]);

        let (info_tx, mut info_rx) = mpsc::unbounded_channel::<Information>();

        handler.send_information(info_tx).await;

        let info = info_rx.try_recv().unwrap();
        match info {
            Information::Sdp { data } => {
                assert!(data.contains("m=video 0 RTP/AVP 96"));
                assert!(data.contains("H264/90000"));
                assert!(data.contains("profile-level-id=42001e"));
            }
        }
    }

    #[tokio::test]
    async fn test_live_stream_handler_send_information_no_sps_sends_nothing() {
        let (_bridge, handler) = make_main_handler();

        let (info_tx, mut info_rx) = mpsc::unbounded_channel::<Information>();

        handler.send_information(info_tx).await;

        // No SDP should be sent when SPS/PPS are not yet available.
        assert!(info_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn test_streaming_service_start_fails_when_rtsp_port_is_in_use() {
        let rtsp_listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let rtsp_port = rtsp_listener.local_addr().unwrap().port();

        let httpflv_listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let httpflv_port = httpflv_listener.local_addr().unwrap().port();
        drop(httpflv_listener);

        let config = StreamingConfig {
            rtsp_port,
            httpflv_port,
            ..StreamingConfig::default()
        };

        let mut service = StreamingService::new(config);
        let start_result = service.start().await;

        if start_result.is_ok() {
            service.shutdown().await;
        }

        assert!(
            start_result.is_err(),
            "expected startup to fail when RTSP port is already bound"
        );
    }

    #[tokio::test]
    async fn test_streaming_service_start_uses_configured_audio_sample_rate() {
        let rtsp_listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let rtsp_port = rtsp_listener.local_addr().unwrap().port();
        drop(rtsp_listener);

        let httpflv_listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let httpflv_port = httpflv_listener.local_addr().unwrap().port();
        drop(httpflv_listener);

        let config = StreamingConfig {
            rtsp_port,
            httpflv_port,
            audio_sample_rate: 44_100,
            ..StreamingConfig::default()
        };

        let mut service = StreamingService::new(config);
        let start_result = service.start().await;
        assert!(start_result.is_ok(), "streaming startup should succeed");

        let bridge = start_result.unwrap();
        assert_eq!(bridge.audio_sample_rate, 44_100);

        service.shutdown().await;
    }
}
