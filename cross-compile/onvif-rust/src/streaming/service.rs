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

use super::bridge::{LowLatencyFrameQueue, StreamState};
use super::telemetry::StreamTelemetry;
use streaming_lib::config::StreamingConfig as LibStreamingConfig;
use streaming_lib::streamhub::define::{
    DataReceiver, DataSender, Information, InformationSender, MediaInfo, SubscribeType,
    VideoCodecType,
};
use streaming_lib::streamhub::errors::StreamHubError;
use streaming_lib::streamhub::statistics::StatisticsStream;
use streaming_lib::streamhub::stream::StreamIdentifier;
use streaming_lib::{FrameData, StreamsHub, TStreamHandler};
#[cfg(test)]
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
    /// Video frame rate for SDP `a=framerate` attribute.
    video_framerate: u32,
}

impl LiveStreamHandler {
    /// Create a handler for a specific stream.
    pub fn new(is_main: bool, bridge: Arc<StreamingBridge>, video_framerate: u32) -> Self {
        Self {
            is_main,
            bridge,
            video_framerate,
        }
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
        let stream_name = if self.is_main { "main" } else { "sub" };
        tracing::debug!(
            stream = stream_name,
            sub_type = ?sub_type,
            "Subscriber requesting prior data"
        );

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

            let has_sps = sps.is_some();
            let has_pps = pps.is_some();
            let has_idr = bootstrap_idr.is_some();

            if !has_sps || !has_pps {
                tracing::warn!(
                    stream = stream_name,
                    has_sps,
                    has_pps,
                    "SPS/PPS missing when subscriber connects (client will see black screen)"
                );
            }

            if let (Some(sps), Some(pps)) = (sps, pps) {
                tracing::debug!(
                    stream = stream_name,
                    sps_size = sps.len(),
                    pps_size = pps.len(),
                    has_bootstrap_idr = has_idr,
                    idr_size = bootstrap_idr.as_ref().map(|i| i.len()).unwrap_or(0),
                    timestamp,
                    "Sending prior data to subscriber"
                );

                if matches!(sub_type, SubscribeType::HttpFlvPull) {
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
                    // Combine SPS+PPS(+IDR) into a single Annex-B access unit.
                    // Sending them as separate FrameData::Video messages with
                    // the same timestamp causes DTS collisions at the RTP layer
                    // (the assembler/normalizer interaction produces duplicate
                    // timestamps that ffmpeg reports as "non monotonically
                    // increasing dts").
                    let mut au = BytesMut::new();
                    au.extend_from_slice(&[0, 0, 0, 1]);
                    au.extend_from_slice(&sps);
                    au.extend_from_slice(&[0, 0, 0, 1]);
                    au.extend_from_slice(&pps);
                    if let Some(idr) = bootstrap_idr.as_ref() {
                        au.extend_from_slice(&[0, 0, 0, 1]);
                        au.extend_from_slice(idr);
                    }
                    send_frame(
                        &frame_sender,
                        FrameData::Video {
                            timestamp,
                            data: au,
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
        let stream_name = if self.is_main { "main" } else { "sub" };
        tracing::debug!(stream = stream_name, "Generating SDP information");

        let stream = self.stream();
        let sps = stream.sps.read().deref().clone();
        let pps = stream.pps.read().deref().clone();
        let has_sps = sps.is_some();
        let has_pps = pps.is_some();
        let audio_config = self.bridge.audio_config.read().deref().clone();

        if let (Some(sps), Some(pps)) = (sps, pps) {
            let sdp = generate_av_sdp(
                &sps,
                &pps,
                audio_config.as_deref(),
                self.bridge.audio_sample_rate,
                Some(self.video_framerate as f64),
            );
            tracing::debug!(
                stream = stream_name,
                sdp_length = sdp.len(),
                has_audio = audio_config.is_some(),
                "SDP generated"
            );
            let _ = sender.send(Information::Sdp { data: sdp });
        } else {
            tracing::warn!(
                stream = stream_name,
                has_sps,
                has_pps,
                "Cannot generate SDP: SPS/PPS missing"
            );
        }
    }
}

/// Async task that routes frames from a bridge queue to RTSP and HTTP-FLV channels.
///
/// Extracted from an inline closure to enable independent testing of each concern:
/// queue consumption, lag tracking, SPS/PPS change detection, and frame dispatch.
struct FanoutTask {
    bridge: Arc<StreamingBridge>,
    bridge_queue: Arc<LowLatencyFrameQueue>,
    is_main: bool,
    stream_name: String,
    rtsp_tx: tokio::sync::mpsc::Sender<FrameData>,
    httpflv_tx: tokio::sync::mpsc::Sender<FrameData>,

    // Mutable state
    httpflv_remuxer: Option<ValidationHttpFlvRemuxer>,
    cached_sps: Option<Vec<u8>>,
    cached_pps: Option<Vec<u8>>,
    telemetry: StreamTelemetry,
}

impl FanoutTask {
    fn new(
        bridge: Arc<StreamingBridge>,
        bridge_queue: Arc<LowLatencyFrameQueue>,
        is_main: bool,
        stream_name: String,
        rtsp_tx: tokio::sync::mpsc::Sender<FrameData>,
        httpflv_tx: tokio::sync::mpsc::Sender<FrameData>,
    ) -> Self {
        let telemetry = StreamTelemetry::new(stream_name.clone(), Arc::clone(&bridge_queue));
        Self {
            bridge,
            bridge_queue,
            is_main,
            stream_name,
            rtsp_tx,
            httpflv_tx,
            httpflv_remuxer: None,
            cached_sps: None,
            cached_pps: None,
            telemetry,
        }
    }

    /// Main loop — consumes frames from bridge queue and fans out to RTSP + HTTP-FLV.
    async fn run(&mut self) {
        loop {
            let queued = self.bridge_queue.recv().await;
            let frame = queued.frame;
            let queue_delay_ms = queued.enqueue_instant.elapsed().as_millis() as u64;

            self.telemetry.record_lag(queue_delay_ms);

            if !self.telemetry.first_frame_logged {
                tracing::debug!(
                    stream = %self.stream_name,
                    "First frame received in fanout task (pipeline is flowing)"
                );
                self.telemetry.first_frame_logged = true;
            }

            if let FrameData::Video { timestamp, data } = &frame {
                let stream_id = if self.is_main {
                    crate::platform::frame::StreamId::VideoMain
                } else {
                    crate::platform::frame::StreamId::VideoSub
                };
                self.bridge.update_video_metadata(
                    stream_id,
                    *timestamp,
                    data,
                    StreamingService::is_h264_idr(&frame),
                );
            }

            // Track frame statistics
            let frame_size = match &frame {
                FrameData::Video { data, .. } | FrameData::Audio { data, .. } => data.len() as u64,
                _ => 0,
            };
            self.telemetry.record_frame(frame_size);

            self.update_remuxer();

            tracing::trace!(
                stream = %self.stream_name,
                frame_count = self.telemetry.frame_count,
                "Fanout dispatching frame"
            );

            fanout_frame(
                &self.rtsp_tx,
                Some(&self.httpflv_tx),
                self.httpflv_remuxer.as_mut(),
                frame,
            )
            .await;

            self.telemetry.maybe_emit();
        }
    }

    /// Check if SPS/PPS changed and refresh the HTTP-FLV remuxer.
    fn update_remuxer(&mut self) {
        let stream = if self.is_main {
            &self.bridge.main_stream
        } else {
            &self.bridge.sub_stream
        };

        let current_sps = stream.sps.read().deref().clone();
        let current_pps = stream.pps.read().deref().clone();

        let needs_refresh = self.httpflv_remuxer.is_none()
            || current_sps != self.cached_sps
            || current_pps != self.cached_pps;

        if needs_refresh && let (Some(sps), Some(pps)) = (current_sps.clone(), current_pps.clone())
        {
            if self.httpflv_remuxer.is_none() {
                tracing::debug!(
                    stream = %self.stream_name,
                    "HTTP-FLV remuxer initialized"
                );
            } else {
                tracing::debug!(
                    stream = %self.stream_name,
                    "SPS/PPS changed, refreshing HTTP-FLV remuxer"
                );
            }
            let audio_config = self.bridge.audio_config.read().deref().clone();
            self.httpflv_remuxer = Some(ValidationHttpFlvRemuxer::new(
                sps.clone(),
                pps.clone(),
                audio_config,
                self.bridge.audio_sample_rate,
            ));
            self.cached_sps = Some(sps);
            self.cached_pps = Some(pps);
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
    fn parse_queue_capacity(env_key: &str, default: usize) -> usize {
        std::env::var(env_key)
            .ok()
            .and_then(|value| value.trim().parse::<usize>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(default)
    }

    fn is_h264_idr(frame: &FrameData) -> bool {
        let FrameData::Video { data, .. } = frame else {
            return false;
        };

        let bytes = data.as_ref();
        let mut index = 0usize;
        while index + 4 < bytes.len() {
            let (start_code_len, nal_start) =
                if bytes[index..].starts_with(&[0x00, 0x00, 0x00, 0x01]) {
                    (4usize, index + 4)
                } else if bytes[index..].starts_with(&[0x00, 0x00, 0x01]) {
                    (3usize, index + 3)
                } else {
                    index += 1;
                    continue;
                };

            if nal_start < bytes.len() && (bytes[nal_start] & 0x1F) == 5 {
                return true;
            }

            index += start_code_len;
        }

        false
    }

    /// Create a new streaming service with the given configuration.
    ///
    /// Does not start any servers. Call [`start`](Self::start) to begin.
    pub fn new(config: StreamingConfig) -> Self {
        // These queues are replaced in start(). We create the bridge here so
        // that `bridge()` is available before start() for wiring.
        let bridge = Arc::new(StreamingBridge::new(
            LowLatencyFrameQueue::default_main(),
            LowLatencyFrameQueue::default_sub(),
            0,
        ));

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

        let main_queue_capacity = Self::parse_queue_capacity("ONVIF_QUEUE_MAIN", 4);
        let sub_queue_capacity = Self::parse_queue_capacity("ONVIF_QUEUE_SUB", 6);
        let main_bridge_queue = LowLatencyFrameQueue::new("main", main_queue_capacity);
        let sub_bridge_queue = LowLatencyFrameQueue::new("sub", sub_queue_capacity);

        // Carry over cached SPS/PPS from the old bridge so late-joining
        // subscribers still receive parameter sets before the next IDR frame.
        let main_cached = self
            .bridge
            .cached_params(crate::platform::frame::StreamId::VideoMain);
        let sub_cached = self
            .bridge
            .cached_params(crate::platform::frame::StreamId::VideoSub);

        // Re-create the bridge with low-latency queues and cached params.
        self.bridge = Arc::new(StreamingBridge::new_with_cached_params(
            Arc::clone(&main_bridge_queue),
            Arc::clone(&sub_bridge_queue),
            self.config.audio_sample_rate,
            main_cached,
            sub_cached,
        ));

        // Publish main stream (RTSP + HTTP-FLV).
        let main_fanout = self
            .publish_stream(
                &mut streamhub,
                &self.config.main_stream_name.clone(),
                main_bridge_queue,
            )
            .await?;

        // Publish sub stream (RTSP + HTTP-FLV).
        let sub_fanout = self
            .publish_stream(
                &mut streamhub,
                &self.config.sub_stream_name.clone(),
                sub_bridge_queue,
            )
            .await?;

        // Spawn servers.
        let hub_event_sender = streamhub.get_hub_event_sender();
        let lib_config = LibStreamingConfig::new()
            .with_rtsp_listen_addr(format!("0.0.0.0:{}", self.config.rtsp_port));
        self.rtsp_task = Some(spawn_rtsp_server(
            hub_event_sender.clone(),
            self.config.auth.clone(),
            self.config.rtsp_port,
            lib_config,
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
            main_queue_capacity,
            sub_queue_capacity,
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
    ///
    /// Aborts fanout tasks first (closing the bridge channel receivers), then
    /// server tasks. The bridge senders (`frame_tx`) will return `Err` on
    /// subsequent sends, which is handled gracefully by `let _ =` in
    /// `route_frame`. This ordering ensures no new frames are forwarded to
    /// servers that are being shut down.
    pub async fn shutdown(&mut self) {
        tracing::info!("Shutting down streaming service...");

        // Abort fanout tasks first — this drops the bridge channel receivers,
        // causing any in-flight bridge sends to fail gracefully.
        tracing::info!("streaming shutdown: aborting fanout tasks...");
        if let Some(task) = self.main_fanout_task.take() {
            task.abort();
        }
        if let Some(task) = self.sub_fanout_task.take() {
            task.abort();
        }

        // Then abort server tasks.
        tracing::info!("streaming shutdown: aborting server tasks...");
        for task in [
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
        bridge_queue: Arc<LowLatencyFrameQueue>,
    ) -> Result<JoinHandle<()>, anyhow::Error> {
        let app_name = self.config.app_name.clone();

        // Bounded publisher→hub channels: when hub stalls, fanout awaits and the
        // LowLatencyFrameQueue bridge applies drop-oldest (keeps RSS flat).
        let (rtsp_tx, rtsp_rx) = streaming_lib::frame_data_channel();
        let rtsp_id = StreamIdentifier::Rtsp {
            stream_path: stream_name.to_string(),
        };

        let (httpflv_tx, httpflv_rx) = streaming_lib::frame_data_channel();
        // HTTP-FLV uses app_name/stream_name format to differentiate from RTSP
        let httpflv_id = StreamIdentifier::Rtsp {
            stream_path: format!("{}/{}", app_name, stream_name),
        };

        // The handler references the bridge's actual stream state so it reads
        // the latest SPS/PPS/IDR that the bridge caches from live frames.
        let is_main = stream_name == self.config.main_stream_name;
        let handler = Arc::new(LiveStreamHandler::new(
            is_main,
            Arc::clone(&self.bridge),
            self.config.video_framerate,
        ));

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

        // Spawn fanout task: bridge_queue → rtsp_tx + httpflv_tx.
        let mut task = FanoutTask::new(
            Arc::clone(&self.bridge),
            bridge_queue,
            is_main,
            stream_name.to_string(),
            rtsp_tx,
            httpflv_tx,
        );
        let fanout_handle = tokio::spawn(async move { task.run().await });

        Ok(fanout_handle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create a bridge and handler targeting the main stream.
    fn make_main_handler() -> (Arc<StreamingBridge>, LiveStreamHandler) {
        let bridge = Arc::new(StreamingBridge::new(
            LowLatencyFrameQueue::new("test-main", 8),
            LowLatencyFrameQueue::new("test-sub", 8),
            48_000,
        ));
        // Tests must not inherit process-wide SPS/PPS cache from other cases.
        *bridge.main_stream.sps.write() = None;
        *bridge.main_stream.pps.write() = None;
        *bridge.main_stream.bootstrap_idr.write() = None;
        *bridge.sub_stream.sps.write() = None;
        *bridge.sub_stream.pps.write() = None;
        *bridge.sub_stream.bootstrap_idr.write() = None;
        let handler = LiveStreamHandler::new(true, Arc::clone(&bridge), 15);
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

    #[tokio::test]
    async fn test_live_stream_handler_send_prior_data_rtsp_no_sps() {
        let (_bridge, handler) = make_main_handler();
        let (frame_tx, mut frame_rx) = streaming_lib::frame_data_channel();
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

        let (frame_tx, mut frame_rx) = streaming_lib::frame_data_channel();
        let sender = DataSender::Frame { sender: frame_tx };

        let result = handler
            .send_prior_data(sender, SubscribeType::RtspPull)
            .await;
        assert!(result.is_ok());

        // MediaInfo + combined SPS+PPS+IDR access unit = 2 frames.
        let media_info = frame_rx.try_recv().unwrap();
        assert!(matches!(media_info, FrameData::MediaInfo { .. }));

        let au_frame = frame_rx.try_recv().unwrap();
        match au_frame {
            FrameData::Video { timestamp, data } => {
                assert_eq!(timestamp, 2000);
                // Combined AU: start_code + SPS + start_code + PPS + start_code + IDR
                // = 4 + 4 + 4 + 4 + 4 + 4 = 24 bytes
                assert_eq!(data.len(), 24);
                // Verify Annex-B start codes and NALUs
                assert_eq!(&data[0..4], &[0, 0, 0, 1]); // SPS start code
                assert_eq!(&data[4..8], &[0x67, 0x42, 0x00, 0x1e]); // SPS
                assert_eq!(&data[8..12], &[0, 0, 0, 1]); // PPS start code
                assert_eq!(&data[12..16], &[0x68, 0xce, 0x06, 0xe2]); // PPS
                assert_eq!(&data[16..20], &[0, 0, 0, 1]); // IDR start code
                assert_eq!(&data[20..24], &[0x65, 0x88, 0x84, 0x21]); // IDR
            }
            _ => panic!("expected combined Video access unit"),
        }

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

        let (frame_tx, mut frame_rx) = streaming_lib::frame_data_channel();
        let sender = DataSender::Frame { sender: frame_tx };

        let result = handler
            .send_prior_data(sender, SubscribeType::HttpFlvPull)
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
