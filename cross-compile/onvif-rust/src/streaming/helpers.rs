//! Shared streaming helper functions used by both normal and validation modes.
//!
//! These functions were extracted from `main.rs` to avoid duplication between
//! the validation-mode streaming pipeline and the normal-mode live streaming.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use bytes::BytesMut;
use portable_atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use streaming_lib::config::StreamingConfig;
use streaming_lib::streamhub::errors::StreamHubError;
use streaming_lib::streamhub::statistics::StatisticsStream;
use streaming_lib::{DefaultHttpFlvServer, DefaultRtspServer, FrameData, StreamsHub};

use crate::validation::httpflv_remux::ValidationHttpFlvRemuxer;

/// Fan out a frame to RTSP and optionally HTTP-FLV channels.
///
/// For RTSP, the frame is sent as-is (Annex-B).
/// For HTTP-FLV, the frame is remuxed to FLV tag format.
///
/// Uses bounded channels: awaits when full so backpressure reaches the
/// drop-oldest bridge queue instead of growing RSS unboundedly.
pub async fn fanout_frame(
    frame_tx_rtsp: &tokio::sync::mpsc::Sender<FrameData>,
    frame_tx_httpflv: Option<&tokio::sync::mpsc::Sender<FrameData>>,
    httpflv_remuxer: Option<&mut ValidationHttpFlvRemuxer>,
    frame: FrameData,
) {
    if frame_tx_httpflv.is_none() {
        let _ = frame_tx_rtsp.send(frame).await;
        return;
    }

    let _ = frame_tx_rtsp.send(frame.clone()).await;
    if let (Some(tx_httpflv), Some(remuxer)) = (frame_tx_httpflv, httpflv_remuxer) {
        match remuxer.remux_frame(frame) {
            Ok(Some(remuxed_frame)) => {
                let _ = tx_httpflv.send(remuxed_frame).await;
            }
            Ok(None) => {}
            Err(err) => {
                tracing::warn!(error = %err, "Failed to remux frame for HTTP-FLV");
            }
        }
    }
}

/// Read the current subscriber count from a `StatisticsStream`, falling back
/// to a cached atomic value if the lock is contended.
pub fn read_subscriber_count(
    stats_handle: &Arc<tokio::sync::Mutex<StatisticsStream>>,
    cached_count: &Arc<AtomicUsize>,
) -> usize {
    if let Ok(stats) = stats_handle.try_lock() {
        let count = stats.subscriber_count;
        cached_count.store(count, Ordering::Relaxed);
        count
    } else {
        cached_count.load(Ordering::Relaxed)
    }
}

/// Sum the subscriber counts for RTSP and HTTP-FLV streams.
pub fn combined_subscriber_count(
    rtsp_handle: &Arc<tokio::sync::Mutex<StatisticsStream>>,
    httpflv_handle: &Arc<tokio::sync::Mutex<StatisticsStream>>,
    rtsp_cached_count: &Arc<AtomicUsize>,
    httpflv_cached_count: &Arc<AtomicUsize>,
) -> usize {
    let rtsp_count = read_subscriber_count(rtsp_handle, rtsp_cached_count);
    let httpflv_count = read_subscriber_count(httpflv_handle, httpflv_cached_count);
    rtsp_count.saturating_add(httpflv_count)
}

/// Create a `StreamHubError` from a string message.
pub fn stream_hub_error(message: impl Into<String>) -> StreamHubError {
    StreamHubError::from(message.into())
}

/// Send a single `FrameData` to a subscriber's frame channel.
///
/// Uses `try_send` so a slow subscriber drops frames instead of blocking the
/// caller or growing an unbounded buffer.
pub fn send_frame(
    frame_sender: &tokio::sync::mpsc::Sender<FrameData>,
    frame: FrameData,
) -> Result<(), StreamHubError> {
    match frame_sender.try_send(frame) {
        Ok(()) => Ok(()),
        Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
            tracing::debug!("frame channel full; dropping frame for slow subscriber");
            Ok(())
        }
        Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => Err(stream_hub_error(
            "failed to send frame to subscriber: channel closed",
        )),
    }
}

/// Send HTTP-FLV prior data (sequence headers + optional bootstrap IDR) to a
/// late-joining subscriber.
pub fn send_httpflv_prior_frames(
    frame_sender: &tokio::sync::mpsc::Sender<FrameData>,
    remuxer: &mut ValidationHttpFlvRemuxer,
    timestamp: u32,
    bootstrap_idr: Option<&[u8]>,
) -> Result<(), StreamHubError> {
    tracing::debug!(
        timestamp,
        has_bootstrap_idr = bootstrap_idr.is_some(),
        "Sending HTTP-FLV prior frames to late-joining subscriber"
    );

    let video_sequence_header = remuxer.video_sequence_header(timestamp).map_err(|err| {
        stream_hub_error(format!(
            "failed to build HTTP-FLV video sequence header: {}",
            err
        ))
    })?;
    if let FrameData::Video { data, .. } = &video_sequence_header {
        tracing::debug!(size = data.len(), "Sending video sequence header");
    }
    send_frame(frame_sender, video_sequence_header)?;

    if let Some(audio_sequence_header) = remuxer.audio_sequence_header(timestamp) {
        if let FrameData::Audio { data, .. } = &audio_sequence_header {
            tracing::debug!(size = data.len(), "Sending audio sequence header");
        }
        send_frame(frame_sender, audio_sequence_header)?;
    }

    if let Some(idr) = bootstrap_idr {
        tracing::debug!(size = idr.len(), "Sending bootstrap IDR to subscriber");
        match remuxer.remux_video_frame(timestamp, BytesMut::from(idr)) {
            Ok(Some(frame)) => send_frame(frame_sender, frame)?,
            Ok(None) => {}
            Err(err) => {
                tracing::warn!(
                    error = %err,
                    "Skipping malformed bootstrap IDR for HTTP-FLV prior data"
                );
            }
        }
    }

    Ok(())
}

/// Generate an SDP description for an audio/video stream.
///
/// When `video_framerate` is provided, an `a=framerate` attribute is added
/// to the video media section per RFC 4566 §6, helping clients like VLC
/// pre-configure their jitter buffer for the expected frame rate.
pub fn generate_av_sdp(
    sps: &[u8],
    pps: &[u8],
    audio_config: Option<&[u8]>,
    audio_sample_rate: u32,
    video_framerate: Option<f64>,
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
        BASE64.encode(sps),
        BASE64.encode(pps),
        profile_level_id
    ));
    sdp.push_str("a=control:trackID=0\r\n");
    if let Some(fps) = video_framerate {
        sdp.push_str(&format!("a=framerate:{:.1}\r\n", fps));
    }
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

/// Extract the channel count from an AAC audio config (AudioSpecificConfig).
pub fn audio_channels_from_config(audio_config: &[u8]) -> u32 {
    if audio_config.len() >= 2 {
        ((audio_config[1] >> 3) & 0x0F) as u32
    } else {
        2
    }
}

/// Format audio config bytes as uppercase hex.
pub fn audio_config_hex(audio_config: &[u8]) -> String {
    audio_config
        .iter()
        .map(|b| format!("{:02X}", b))
        .collect::<String>()
}

/// Spawn an RTSP server on the given port.
pub fn spawn_rtsp_server(
    event_sender: streaming_lib::streamhub::define::StreamHubEventSender,
    stream_auth: Option<streaming_lib::common::auth::Auth>,
    port: u16,
    streaming_config: StreamingConfig,
) -> tokio::task::JoinHandle<()> {
    let addr = format!("0.0.0.0:{}", port);
    tokio::spawn(async move {
        let mut server = DefaultRtspServer::new(addr, event_sender, stream_auth, streaming_config);
        if let Err(e) = server.run(None).await {
            tracing::error!("RTSP server error: {}", e);
        }
    })
}

/// Spawn an HTTP-FLV server on the given port.
pub fn spawn_httpflv_server(
    event_sender: streaming_lib::streamhub::define::StreamHubEventSender,
    stream_auth: Option<streaming_lib::common::auth::Auth>,
    port: u16,
) -> tokio::task::JoinHandle<()> {
    let addr = format!("0.0.0.0:{}", port);
    tokio::spawn(async move {
        let mut server = DefaultHttpFlvServer::new(addr, event_sender, stream_auth);
        if let Err(e) = server.run().await {
            tracing::error!("HTTP-FLV server error: {}", e);
        }
    })
}

/// Spawn the StreamsHub event loop as a background task.
pub fn spawn_streamhub_event_loop(mut streamhub: StreamsHub) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        streamhub.run().await;
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_av_sdp_includes_audio() {
        let sps = vec![0x67, 0x42, 0x00, 0x1e];
        let pps = vec![0x68, 0xce, 0x06, 0xe2];
        let audio_config = vec![0x12, 0x10];

        let sdp = generate_av_sdp(&sps, &pps, Some(&audio_config), 48000, None);

        assert!(sdp.contains("m=video 0 RTP/AVP 96"));
        assert!(sdp.contains("a=control:trackID=0"));
        assert!(sdp.contains("m=audio 0 RTP/AVP 97"));
        assert!(sdp.contains("a=control:trackID=1"));
    }

    #[test]
    fn test_generate_av_sdp_without_audio() {
        let sps = vec![0x67, 0x42, 0x00, 0x1e];
        let pps = vec![0x68, 0xce, 0x06, 0xe2];

        let sdp = generate_av_sdp(&sps, &pps, None, 48000, None);

        assert!(sdp.contains("m=video 0 RTP/AVP 96"));
        assert!(!sdp.contains("m=audio 0 RTP/AVP 97"));
    }

    #[test]
    fn test_generate_av_sdp_short_sps_uses_fallback_profile_level_id() {
        let sdp = generate_av_sdp(
            &[0x67, 0x42, 0x00],
            &[0x68, 0xce, 0x06, 0xe2],
            None,
            48_000,
            None,
        );
        assert!(sdp.contains("profile-level-id=42e01e"));
    }

    #[test]
    fn test_generate_av_sdp_includes_framerate() {
        let sps = vec![0x67, 0x42, 0x00, 0x1e];
        let pps = vec![0x68, 0xce, 0x06, 0xe2];

        let sdp = generate_av_sdp(&sps, &pps, None, 48000, Some(15.0));

        assert!(sdp.contains("a=framerate:15.0\r\n"));
    }

    #[test]
    fn test_generate_av_sdp_no_framerate_when_none() {
        let sps = vec![0x67, 0x42, 0x00, 0x1e];
        let pps = vec![0x68, 0xce, 0x06, 0xe2];

        let sdp = generate_av_sdp(&sps, &pps, None, 48000, None);

        assert!(!sdp.contains("a=framerate"));
    }

    #[test]
    fn test_audio_channels_from_config_two_bytes_extracts_channel_count() {
        assert_eq!(audio_channels_from_config(&[0x12, 0x10]), 2);
    }

    #[test]
    fn test_audio_channels_from_config_short_input_returns_stereo_default() {
        assert_eq!(audio_channels_from_config(&[0x12]), 2);
    }

    #[test]
    fn test_audio_config_hex_formats_uppercase_hex() {
        assert_eq!(audio_config_hex(&[0x12, 0xab, 0x00]), "12AB00");
    }

    #[test]
    fn test_send_frame_closed_channel_returns_error() {
        let (tx, rx) = streaming_lib::frame_data_channel();
        drop(rx);

        let result = send_frame(
            &tx,
            FrameData::Video {
                timestamp: 99,
                data: BytesMut::from(&b"frame"[..]),
            },
        );

        assert!(result.is_err());
        let err = result.expect_err("send should fail when receiver is dropped");
        assert!(
            err.to_string()
                .contains("failed to send frame to subscriber"),
            "unexpected error: {err}"
        );
    }

    #[tokio::test]
    async fn test_combined_subscriber_count_sums_rtsp_and_httpflv() {
        let rtsp_stats = Arc::new(tokio::sync::Mutex::new(StatisticsStream::default()));
        let httpflv_stats = Arc::new(tokio::sync::Mutex::new(StatisticsStream::default()));
        let rtsp_cached = Arc::new(AtomicUsize::new(0));
        let httpflv_cached = Arc::new(AtomicUsize::new(0));

        rtsp_stats.lock().await.subscriber_count = 2;
        httpflv_stats.lock().await.subscriber_count = 3;

        let count =
            combined_subscriber_count(&rtsp_stats, &httpflv_stats, &rtsp_cached, &httpflv_cached);
        assert_eq!(count, 5);
    }

    #[tokio::test]
    async fn test_combined_subscriber_count_uses_cache_when_locks_contended() {
        let rtsp_stats = Arc::new(tokio::sync::Mutex::new(StatisticsStream::default()));
        let httpflv_stats = Arc::new(tokio::sync::Mutex::new(StatisticsStream::default()));
        let rtsp_cached = Arc::new(AtomicUsize::new(4));
        let httpflv_cached = Arc::new(AtomicUsize::new(5));

        let _rtsp_guard = rtsp_stats.lock().await;
        let _httpflv_guard = httpflv_stats.lock().await;
        let count =
            combined_subscriber_count(&rtsp_stats, &httpflv_stats, &rtsp_cached, &httpflv_cached);
        assert_eq!(count, 9);
    }

    #[tokio::test]
    async fn test_read_subscriber_count_lock_success_updates_cache() {
        let stats = Arc::new(tokio::sync::Mutex::new(StatisticsStream::default()));
        let cached = Arc::new(AtomicUsize::new(0));

        stats.lock().await.subscriber_count = 7;

        let count = read_subscriber_count(&stats, &cached);
        assert_eq!(count, 7);
        assert_eq!(cached.load(Ordering::Relaxed), 7);
    }

    #[tokio::test]
    async fn test_read_subscriber_count_lock_contended_returns_cached_value() {
        let stats = Arc::new(tokio::sync::Mutex::new(StatisticsStream::default()));
        let cached = Arc::new(AtomicUsize::new(11));

        let _guard = stats.lock().await;
        let count = read_subscriber_count(&stats, &cached);

        assert_eq!(count, 11);
    }

    #[tokio::test]
    async fn test_fanout_frame_rtsp_only_routes_without_httpflv() {
        let (rtsp_tx, mut rtsp_rx) = streaming_lib::frame_data_channel();
        let frame = FrameData::Video {
            timestamp: 10,
            data: BytesMut::from(&b"video"[..]),
        };

        fanout_frame(&rtsp_tx, None, None, frame).await;
        let received = rtsp_rx.recv().await.expect("rtsp frame");
        assert!(matches!(received, FrameData::Video { timestamp: 10, .. }));
    }

    #[tokio::test]
    async fn test_fanout_frame_with_httpflv_routes_to_both() {
        let (rtsp_tx, mut rtsp_rx) = streaming_lib::frame_data_channel();
        let (http_tx, mut http_rx) = streaming_lib::frame_data_channel();
        let mut remuxer = ValidationHttpFlvRemuxer::new(
            vec![0x67, 0x42, 0x00, 0x1e],
            vec![0x68, 0xce, 0x06, 0xe2],
            None,
            48_000,
        );
        let frame = FrameData::Video {
            timestamp: 20,
            data: BytesMut::from(
                &[
                    0x00, 0x00, 0x00, 0x01, 0x65, 0x88, 0x84, 0x21, 0xA0, 0x00, 0x00, 0x00, 0x01,
                    0x06, 0xE5,
                ][..],
            ),
        };

        fanout_frame(&rtsp_tx, Some(&http_tx), Some(&mut remuxer), frame).await;

        let rtsp_frame = rtsp_rx.recv().await.expect("rtsp frame");
        let http_frame = http_rx.recv().await.expect("httpflv frame");
        assert!(matches!(rtsp_frame, FrameData::Video { timestamp: 20, .. }));
        match http_frame {
            FrameData::Video { timestamp, data } => {
                assert_eq!(timestamp, 20);
                assert_eq!(data[0], 0x17);
                assert_eq!(data[1], 0x01);
            }
            _ => panic!("expected remuxed HTTP-FLV video frame"),
        }
    }

    #[tokio::test]
    async fn test_send_httpflv_prior_frames_no_audio_no_bootstrap_emits_video_header_only() {
        let (tx, mut rx) = streaming_lib::frame_data_channel();
        let mut remuxer = ValidationHttpFlvRemuxer::new(
            vec![0x67, 0x42, 0x00, 0x1e],
            vec![0x68, 0xce, 0x06, 0xe2],
            None,
            48_000,
        );

        send_httpflv_prior_frames(&tx, &mut remuxer, 4321, None)
            .expect("prior frame bootstrap should succeed");

        let first = rx.recv().await.expect("first frame should be present");
        match first {
            FrameData::Video { timestamp, data } => {
                assert_eq!(timestamp, 4321);
                assert!(data.len() >= 2);
                assert_eq!(data[1], 0);
            }
            _ => panic!("expected video sequence header"),
        }
        assert!(rx.try_recv().is_err(), "no additional frames expected");
    }

    #[tokio::test]
    async fn test_send_httpflv_prior_frames_malformed_bootstrap_skips_idr_frame() {
        let (tx, mut rx) = streaming_lib::frame_data_channel();
        let mut remuxer = ValidationHttpFlvRemuxer::new(
            vec![0x67, 0x42, 0x00, 0x1e],
            vec![0x68, 0xce, 0x06, 0xe2],
            None,
            48_000,
        );

        send_httpflv_prior_frames(&tx, &mut remuxer, 4322, Some(&[0x00, 0x00, 0x00]))
            .expect("malformed bootstrap should be ignored");

        let first = rx.recv().await.expect("first frame should be present");
        match first {
            FrameData::Video { data, .. } => {
                assert!(data.len() >= 2);
                assert_eq!(data[1], 0);
            }
            _ => panic!("expected video sequence header"),
        }
        assert!(
            rx.try_recv().is_err(),
            "malformed bootstrap should not emit IDR"
        );
    }
}
