use crate::config::StreamingConfig;
use crate::protocol::rtsp::global_trait::Marshal;
use crate::protocol::rtsp::global_trait::Unmarshal;
use crate::protocol::rtsp::rtsp_codec;
use byteorder::BigEndian;
use bytes::{BufMut, BytesMut};
use chrono::Utc;

use crate::protocol::rtsp::rtp::define::ANNEXB_NALU_START_CODE;
use crate::protocol::rtsp::rtp::utils::Marshal as RtpMarshal;

use crate::common::auth::SecretCarrier;
use crate::common::http::HttpRequest as RtspRequest;
use crate::common::http::HttpResponse as RtspResponse;
use crate::common::http::Marshal as RtspMarshal;
use crate::common::http::Unmarshal as RtspUnmarshal;
use crate::common::http::try_get_complete_message_len;

use crate::protocol::rtsp::rtp::RtpPacket;
use crate::protocol::rtsp::rtsp_range::RtspRange;

use crate::protocol::rtsp::sdp::fmtp::Fmtp;

use crate::protocol::rtsp::rtsp_channel::RtpChannel;
use crate::protocol::rtsp::rtsp_codec::RtspCodecInfo;
use crate::protocol::rtsp::rtsp_track::RtspTrack;
use crate::protocol::rtsp::rtsp_track::TrackType;
use crate::protocol::rtsp::rtsp_transport::ProtocolType;
use crate::protocol::rtsp::rtsp_transport::RtspTransport;

use crate::io::bytes_reader::BytesReader;
use crate::io::bytes_writer::AsyncBytesWriter;

use super::errors::SessionError;
use super::errors::SessionErrorValue;
use crate::hub::define::DataSender;
use crate::hub::define::MediaInfo;
use crate::hub::define::VideoCodecType;
use crate::io::UdpIO;
use crate::io::bytes_writer::BytesWriter;
use http::StatusCode;
use tokio::sync::oneshot;

use crate::protocol::rtsp::rtp::errors::UnPackerError;
use crate::protocol::rtsp::rtp::utils::OnRtpPacketFn;
use crate::protocol::rtsp::sdp::Sdp;

use super::define;
use super::define::rtsp_method_name;
use crate::io::TNetIO;
use crate::io::{TcpReadIO, TcpWriteIO};
use async_trait::async_trait;
use portable_atomic::AtomicU64;
use tokio::time::{Duration, timeout};

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{
    Arc, OnceLock,
    atomic::{AtomicBool, AtomicU32, Ordering},
};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::common::auth::Auth;
use crate::hub::{
    define::{
        FrameData, FrameDataSender, Information, InformationSender, NotifyInfo, PublishType,
        PublisherInfo, StreamHubEvent, StreamHubEventSender, SubscribeType, SubscriberInfo,
        TStreamHandler,
    },
    errors::{StreamHubError, StreamHubErrorValue},
    statistics::StatisticsStream,
    stream::StreamIdentifier,
    utils::{RandomDigitCount, Uuid},
};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn monotonic_millis() -> u64 {
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_millis() as u64
}

/// Lightweight per-track RTP counters for logging.
///
/// These use atomics so they can be safely updated from async contexts
/// without introducing additional locking on the hot path.
struct RtpTrackCounters {
    packet_count: AtomicU64,
    byte_count: AtomicU64,
    first_send_ms: AtomicU64,
    last_send_ms: AtomicU64,
    last_seq: AtomicU32,
    last_timestamp: AtomicU32,
}

const RTP_TIMESTAMP_WRAP_THRESHOLD: u32 = 0x8000_0000;
const SESSION_ID_RANDOM_DIGITS: RandomDigitCount = RandomDigitCount::Four;
const DEFAULT_MAX_FRAME_AGE_MS: u32 = 1500;
const LAG_RECOVERY_THRESHOLD_MS: u32 = 1000;
const LAG_RECOVERY_SUSTAINED_FRAMES: u32 = 8;
const SOURCE_TIMESTAMP_RESET_THRESHOLD_MS: u32 = 10_000;
const DEFAULT_PLAY_READY_TIMEOUT_MS: u64 = 1500;
const RTP_SEND_SLOW_WARN_MS: u128 = 25;
const PACER_SLEEP_DIAGNOSTIC_MIN_MS: u64 = 20;

/// RFC 2326 §12.36 — Server header value.
const SERVER_HEADER: &str = "streaming-lib/0.1";

/// Lag recovery mode for handling playback delays
///
/// This enum controls how the server handles situations where the client
/// falls behind in playback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LagRecoveryMode {
    /// Disabled - no recovery, just deliver frames as they arrive
    Disabled,
    /// Latest IDR - skip to latest keyframe when lag is detected
    LatestIdr,
}

impl LagRecoveryMode {
    /// Create a LagRecoveryMode from a string value.
    ///
    /// - "off", "none", "disabled" → Disabled
    /// - Anything else → LatestIdr
    pub fn from_str_value(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "off" | "none" | "disabled" => Self::Disabled,
            _ => Self::LatestIdr,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct PlaybackLatencyPolicy {
    max_frame_age_ms: u32,
    lag_recovery_mode: LagRecoveryMode,
    lag_recovery_threshold_ms: u32,
    sustained_lag_frames: u32,
}

impl PlaybackLatencyPolicy {
    /// Create a PlaybackLatencyPolicy from a StreamingConfig.
    ///
    /// Uses `config.max_frame_age_ms` with the same `> 0` guard,
    /// falling back to `DEFAULT_MAX_FRAME_AGE_MS` if the value is 0.
    /// Uses `config.lag_recovery_mode` directly.
    /// Keeps `lag_recovery_threshold_ms` and `sustained_lag_frames` as constants.
    fn from_config(config: &StreamingConfig) -> Self {
        let max_frame_age_ms = if config.max_frame_age_ms > 0 {
            config.max_frame_age_ms
        } else {
            DEFAULT_MAX_FRAME_AGE_MS
        };

        Self {
            max_frame_age_ms,
            lag_recovery_mode: config.lag_recovery_mode,
            lag_recovery_threshold_ms: LAG_RECOVERY_THRESHOLD_MS,
            sustained_lag_frames: LAG_RECOVERY_SUSTAINED_FRAMES,
        }
    }
}

#[derive(Debug)]
struct LagTracker {
    anchor_local: Instant,
    anchor_source_ts: u32,
    last_source_ts: u32,
    initialized: bool,
}

impl Default for LagTracker {
    fn default() -> Self {
        Self {
            anchor_local: Instant::now(),
            anchor_source_ts: 0,
            last_source_ts: 0,
            initialized: false,
        }
    }
}

impl LagTracker {
    fn lag_ms(&mut self, source_timestamp_ms: u32) -> u32 {
        if !self.initialized {
            self.initialized = true;
            self.anchor_local = Instant::now();
            self.anchor_source_ts = source_timestamp_ms;
            self.last_source_ts = source_timestamp_ms;
            return 0;
        }

        if source_timestamp_ms < self.last_source_ts
            && self.last_source_ts.wrapping_sub(source_timestamp_ms)
                > SOURCE_TIMESTAMP_RESET_THRESHOLD_MS
        {
            self.anchor_local = Instant::now();
            self.anchor_source_ts = source_timestamp_ms;
            self.last_source_ts = source_timestamp_ms;
            return 0;
        }

        self.last_source_ts = source_timestamp_ms;
        let elapsed_ms = self.anchor_local.elapsed().as_millis() as u32;
        let expected_source_ts = self.anchor_source_ts.wrapping_add(elapsed_ms);
        expected_source_ts.saturating_sub(source_timestamp_ms)
    }
}

/// Minimum sleep threshold in milliseconds.
///
/// Sleeps shorter than ~2ms are unreliable on Linux due to timer resolution
/// and context-switch overhead. On the Anyka ARM SoC this is especially true
/// at the default HZ=100 tick rate.
const PACE_MIN_SLEEP_MS: u64 = 2;

/// Maximum inter-frame sleep cap in milliseconds.
///
/// After a gap in the source stream (e.g. encoder restart, I/O stall) the
/// timestamp delta can be very large. Sleeping for the full delta would stall
/// playback. 200ms is long enough to absorb normal jitter but short enough
/// to avoid visible freezes.
const PACE_MAX_DELTA_MS: u64 = 200;

/// Paces RTP frame delivery to approximate real-time timing.
///
/// Without pacing, the playback loop dequeues and sends frames as fast as
/// the network allows, causing bursts that overwhelm VLC's jitter buffer.
/// `FramePacer` compares the wall-clock elapsed time against the frame
/// timestamp delta and sleeps when the sender is running ahead of real-time.
///
/// Each RTSP session gets its own `FramePacer` instance so clients joining
/// at different times are paced independently.
struct FramePacer {
    /// Wall-clock instant when the last frame was sent.
    last_send: Option<Instant>,
    /// Source timestamp (in milliseconds) of the last sent frame.
    last_timestamp_ms: Option<u32>,
}

impl FramePacer {
    fn new() -> Self {
        Self {
            last_send: None,
            last_timestamp_ms: None,
        }
    }

    /// Pace a frame by sleeping if the sender is ahead of real-time.
    ///
    /// `timestamp_ms` is the source frame timestamp in milliseconds.
    /// On the first frame, no delay is introduced.
    async fn pace(&mut self, timestamp_ms: u32) -> u64 {
        let mut slept_ms = 0;
        if let (Some(last_send), Some(last_ts)) = (self.last_send, self.last_timestamp_ms) {
            let ts_delta_ms = timestamp_ms.wrapping_sub(last_ts) as u64;
            let wall_delta = last_send.elapsed();
            let wall_delta_ms = wall_delta.as_millis() as u64;

            if ts_delta_ms > wall_delta_ms {
                let sleep_ms = std::cmp::min(ts_delta_ms - wall_delta_ms, PACE_MAX_DELTA_MS);
                if sleep_ms >= PACE_MIN_SLEEP_MS {
                    tokio::time::sleep(Duration::from_millis(sleep_ms)).await;
                    slept_ms = sleep_ms;
                }
            }
        }

        self.last_send = Some(Instant::now());
        self.last_timestamp_ms = Some(timestamp_ms);
        slept_ms
    }
}

#[inline]
fn pacing_timestamp_ms(frame_data: &FrameData) -> Option<u32> {
    match frame_data {
        FrameData::Video { timestamp, .. } => Some(*timestamp),
        FrameData::Audio { .. } | FrameData::MetaData { .. } | FrameData::MediaInfo { .. } => None,
    }
}

#[derive(Debug, Clone, Copy)]
struct RtpTimestampSample {
    output_timestamp: u32,
    scaled_timestamp: u32,
    previous_scaled_timestamp: Option<u32>,
    non_wrap_regressed: bool,
    non_wrap_regression_count: u64,
}

/// Keeps RTP timestamps monotonic for source-side resets while preserving true RTP wrap.
///
/// This normalizer addresses two issues:
/// 1. High initial timestamps from SDK (e.g., 42,543,800 ms → 3.8B RTP units)
///    cause VLC to miscalculate playback position as "late".
/// 2. Source-side timestamp resets need monotonic output without false wrap detection.
///
/// Solution: Capture the first scaled timestamp as an offset, then subtract it
/// from all subsequent timestamps. Output starts near 0 and stays monotonic.
#[derive(Debug, Default)]
struct RtpTimestampNormalizer {
    /// First scaled timestamp (captured on first frame).
    /// Used to normalize timestamps to start near 0.
    initial_offset: Option<u32>,
    /// Tracks the last output timestamp for regression detection.
    previous_output_timestamp: Option<u32>,
    /// Count of non-wrap regressions detected (for diagnostics).
    non_wrap_regression_count: u64,
}

impl RtpTimestampNormalizer {
    /// Normalize RTP timestamps for audio or video.
    ///
    /// # Arguments
    ///
    /// * `source_timestamp` - For audio: timestamp in sample units (per RFC 3640).
    ///   For video: timestamp in milliseconds.
    /// * `clock_rate` - RTP clock rate (e.g., 48000 for AAC, 90000 for H264)
    /// * `track_type` - Audio or Video track type
    ///
    /// # Timestamp Units
    ///
    /// - **Audio (AAC)**: Timestamps are already in sample units (1024 samples/frame)
    ///   and match the clock_rate. No scaling is performed.
    /// - **Video (H264/H265)**: Timestamps are in milliseconds and must be scaled
    ///   to 90kHz RTP clock rate.
    fn normalize(
        &mut self,
        source_timestamp: u32,
        clock_rate: u32,
        track_type: TrackType,
    ) -> RtpTimestampSample {
        // Audio timestamps are already in sample units (RFC 3640), don't scale.
        // Video timestamps are in milliseconds, scale to 90kHz RTP clock.
        let scaled_timestamp = match track_type {
            TrackType::Audio => source_timestamp,
            TrackType::Video => {
                RtspServerSession::scale_rtp_timestamp(source_timestamp, clock_rate)
            }
            TrackType::Application => {
                RtspServerSession::scale_rtp_timestamp(source_timestamp, clock_rate)
            }
        };
        let offset = *self.initial_offset.get_or_insert(scaled_timestamp);
        let normalized_timestamp = scaled_timestamp.wrapping_sub(offset);
        let previous_scaled_timestamp = self.previous_output_timestamp;

        let mut output_timestamp = normalized_timestamp;
        let mut non_wrap_regressed = false;
        if let Some(previous) = previous_scaled_timestamp
            && output_timestamp <= previous
            && previous.wrapping_sub(output_timestamp) <= RTP_TIMESTAMP_WRAP_THRESHOLD
        {
            output_timestamp = previous.wrapping_add(1);
            self.non_wrap_regression_count += 1;
            non_wrap_regressed = true;
        }

        self.previous_output_timestamp = Some(output_timestamp);
        RtpTimestampSample {
            output_timestamp,
            scaled_timestamp,
            previous_scaled_timestamp,
            non_wrap_regressed,
            non_wrap_regression_count: self.non_wrap_regression_count,
        }
    }
}

/// Coalesce H.264 access-units that arrive as multiple same-timestamp chunks.
///
/// Some publishers emit one `FrameData::Video` per NAL unit while keeping the
/// same timestamp for all NALs belonging to an access unit. RTP packetizers
/// expect a full access unit per `on_frame` call; sending NALs individually
/// causes multiple marker-terminated access units at the same RTP timestamp,
/// which strict demuxers flag as "same timestamp as previous access unit".
#[derive(Debug, Default)]
struct VideoAccessUnitAssembler {
    pending_timestamp: Option<u32>,
    pending_bytes: BytesMut,
}

impl VideoAccessUnitAssembler {
    fn push(&mut self, timestamp: u32, chunk: BytesMut) -> Option<(u32, BytesMut)> {
        if chunk.is_empty() {
            return None;
        }

        match self.pending_timestamp {
            None => {
                self.pending_timestamp = Some(timestamp);
                self.append_as_annexb(&chunk[..]);
                None
            }
            Some(pending_ts) if pending_ts == timestamp => {
                self.append_as_annexb(&chunk[..]);
                None
            }
            Some(_pending_ts) => {
                let flushed = self.flush();
                self.pending_timestamp = Some(timestamp);
                self.append_as_annexb(&chunk[..]);
                flushed
            }
        }
    }

    fn flush(&mut self) -> Option<(u32, BytesMut)> {
        let timestamp = self.pending_timestamp.take()?;
        if self.pending_bytes.is_empty() {
            return None;
        }
        let bytes = std::mem::take(&mut self.pending_bytes);
        Some((timestamp, bytes))
    }

    fn append_as_annexb(&mut self, chunk: &[u8]) {
        if chunk.is_empty() {
            return;
        }
        if has_annexb_start_code(chunk) {
            self.pending_bytes.extend_from_slice(chunk);
            return;
        }
        self.pending_bytes
            .extend_from_slice(&ANNEXB_NALU_START_CODE[..]);
        self.pending_bytes.extend_from_slice(chunk);
    }
}

fn has_annexb_start_code(data: &[u8]) -> bool {
    data.starts_with(&[0x00, 0x00, 0x01]) || data.starts_with(&ANNEXB_NALU_START_CODE[..])
}

fn contains_h264_idr(data: &[u8]) -> bool {
    let mut i = 0usize;
    while i + 4 < data.len() {
        let nal_start = if data[i..].starts_with(&ANNEXB_NALU_START_CODE[..]) {
            i + 4
        } else if data[i..].starts_with(&[0x00, 0x00, 0x01]) {
            i + 3
        } else {
            i += 1;
            continue;
        };

        if nal_start < data.len() && (data[nal_start] & 0x1F) == 5 {
            return true;
        }

        i = nal_start;
    }
    false
}

struct PlaybackVideoSendContext<'a> {
    session_id: &'a str,
    remote_addr: std::net::SocketAddr,
    request_path: &'a str,
    shutdown: &'a Arc<AtomicBool>,
}

async fn send_video_access_unit(
    video_channel: &Arc<Mutex<RtpChannel>>,
    timestamp_normalizers: &mut HashMap<TrackType, RtpTimestampNormalizer>,
    ctx: &PlaybackVideoSendContext<'_>,
    timestamp: u32,
    data: &mut BytesMut,
) {
    let mut channel = video_channel.lock().await;
    let payload_len = data.len();
    let normalized = timestamp_normalizers
        .entry(TrackType::Video)
        .or_default()
        .normalize(timestamp, channel.clock_rate(), TrackType::Video);
    if normalized.non_wrap_regressed {
        warn!(
            track = ?TrackType::Video,
            session_id = %ctx.session_id,
            remote_addr = %ctx.remote_addr,
            request_path = %ctx.request_path,
            prev_scaled = normalized.previous_scaled_timestamp.unwrap_or_default(),
            current_scaled = normalized.scaled_timestamp,
            corrected_timestamp = normalized.output_timestamp,
            regression_count = normalized.non_wrap_regression_count,
            "rtp_timestamp_non_wrap_regression"
        );
    }
    let send_started = Instant::now();
    match channel.on_frame(data, normalized.output_timestamp).await {
        Ok(()) => {
            let send_ms = send_started.elapsed().as_millis();
            if send_ms >= RTP_SEND_SLOW_WARN_MS {
                warn!(
                    track = ?TrackType::Video,
                    session_id = %ctx.session_id,
                    remote_addr = %ctx.remote_addr,
                    request_path = %ctx.request_path,
                    source_ts = timestamp,
                    rtp_ts = normalized.output_timestamp,
                    payload_len = payload_len,
                    send_ms = send_ms,
                    diag_monotonic_ms = monotonic_millis(),
                    "rtp_send_slow"
                );
            }
        }
        Err(err) => {
            info!(error = %err, "handle_play_error");
            ctx.shutdown.store(true, Ordering::Release);
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn process_audio_frame(
    audio_channel: &Arc<Mutex<RtpChannel>>,
    timestamp_normalizers: &mut HashMap<TrackType, RtpTimestampNormalizer>,
    latency_policy: PlaybackLatencyPolicy,
    audio_lag_tracker: &mut LagTracker,
    session_id: &str,
    remote_addr: SocketAddr,
    request_path: &str,
    shutdown: &Arc<AtomicBool>,
    timestamp: u32,
    data: &mut BytesMut,
    dropped_stale_frames: &mut u64,
    waiting_for_idr_recovery: bool,
    dropped_for_recovery: &mut u64,
) {
    if waiting_for_idr_recovery {
        *dropped_for_recovery += 1;
        return;
    }

    let previous_source_ts = audio_lag_tracker.last_source_ts;
    let lag_ms = audio_lag_tracker.lag_ms(timestamp);
    if crate::stream_frame_debug_logging_enabled()
        && lag_ms >= latency_policy.max_frame_age_ms.saturating_div(2)
    {
        debug!(
            track = ?TrackType::Audio,
            session_id = %session_id,
            remote_addr = %remote_addr,
            request_path = %request_path,
            lag_ms = lag_ms,
            threshold_ms = latency_policy.max_frame_age_ms,
            source_ts = timestamp,
            prev_source_ts = previous_source_ts,
            waiting_for_idr_recovery = waiting_for_idr_recovery,
            diag_monotonic_ms = monotonic_millis(),
            "lag_probe"
        );
    }
    if lag_ms > latency_policy.max_frame_age_ms {
        *dropped_stale_frames += 1;
        let elapsed_ms = audio_lag_tracker.anchor_local.elapsed().as_millis() as u32;
        let expected_source_ts = audio_lag_tracker.anchor_source_ts.wrapping_add(elapsed_ms);
        warn!(
            track = ?TrackType::Audio,
            session_id = %session_id,
            remote_addr = %remote_addr,
            request_path = %request_path,
            lag_ms = lag_ms,
            threshold_ms = latency_policy.max_frame_age_ms,
            source_ts = timestamp,
            prev_source_ts = previous_source_ts,
            expected_source_ts = expected_source_ts,
            anchor_source_ts = audio_lag_tracker.anchor_source_ts,
            anchor_elapsed_ms = elapsed_ms,
            "stale_frame_drop"
        );
        return;
    }

    let mut channel = audio_channel.lock().await;
    let normalized = timestamp_normalizers
        .entry(TrackType::Audio)
        .or_default()
        .normalize(timestamp, channel.clock_rate(), TrackType::Audio);

    if crate::stream_frame_debug_logging_enabled() {
        debug!(
            timestamp_in = timestamp,
            clock_rate = channel.clock_rate(),
            scaled = normalized.scaled_timestamp,
            output = normalized.output_timestamp,
            "audio_timestamp_info"
        );
    }

    if normalized.non_wrap_regressed {
        warn!(
            track = ?TrackType::Audio,
            session_id = %session_id,
            remote_addr = %remote_addr,
            request_path = %request_path,
            prev_scaled = normalized.previous_scaled_timestamp.unwrap_or_default(),
            current_scaled = normalized.scaled_timestamp,
            corrected_timestamp = normalized.output_timestamp,
            regression_count = normalized.non_wrap_regression_count,
            "rtp_timestamp_non_wrap_regression"
        );
    }

    let payload_len = data.len();
    let send_started = Instant::now();
    match channel.on_frame(data, normalized.output_timestamp).await {
        Ok(()) => {
            let send_ms = send_started.elapsed().as_millis();
            if send_ms >= RTP_SEND_SLOW_WARN_MS {
                warn!(
                    track = ?TrackType::Audio,
                    session_id = %session_id,
                    remote_addr = %remote_addr,
                    request_path = %request_path,
                    source_ts = timestamp,
                    rtp_ts = normalized.output_timestamp,
                    payload_len = payload_len,
                    send_ms = send_ms,
                    diag_monotonic_ms = monotonic_millis(),
                    "rtp_send_slow"
                );
            }
        }
        Err(err) => {
            info!(error = %err, "handle_play_error");
            shutdown.store(true, Ordering::Release);
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn process_video_frame(
    video_channel: &Arc<Mutex<RtpChannel>>,
    video_assembler: &mut VideoAccessUnitAssembler,
    timestamp_normalizers: &mut HashMap<TrackType, RtpTimestampNormalizer>,
    latency_policy: PlaybackLatencyPolicy,
    video_lag_tracker: &mut LagTracker,
    sustained_video_lag_frames: &mut u32,
    waiting_for_idr_recovery: &mut bool,
    dropped_stale_frames: &mut u64,
    dropped_for_recovery: &mut u64,
    idr_recovery_count: &mut u64,
    session_id: &str,
    remote_addr: SocketAddr,
    request_path: &str,
    shutdown: &Arc<AtomicBool>,
    timestamp: u32,
    data: BytesMut,
) {
    let Some((flush_ts, mut flush_data)) = video_assembler.push(timestamp, data) else {
        return;
    };

    let contains_idr = contains_h264_idr(flush_data.as_ref());
    if *waiting_for_idr_recovery {
        if !contains_idr {
            *dropped_for_recovery += 1;
            return;
        }

        *waiting_for_idr_recovery = false;
        info!(
            track = ?TrackType::Video,
            session_id = %session_id,
            remote_addr = %remote_addr,
            request_path = %request_path,
            "lag_recovery_resynced"
        );
    }

    let previous_source_ts = video_lag_tracker.last_source_ts;
    let lag_ms = video_lag_tracker.lag_ms(flush_ts);
    if crate::stream_frame_debug_logging_enabled()
        && lag_ms >= latency_policy.max_frame_age_ms.saturating_div(2)
    {
        debug!(
            track = ?TrackType::Video,
            session_id = %session_id,
            remote_addr = %remote_addr,
            request_path = %request_path,
            lag_ms = lag_ms,
            threshold_ms = latency_policy.max_frame_age_ms,
            source_ts = flush_ts,
            prev_source_ts = previous_source_ts,
            contains_idr = contains_idr,
            sustained_lag_frames = *sustained_video_lag_frames,
            waiting_for_idr_recovery = *waiting_for_idr_recovery,
            diag_monotonic_ms = monotonic_millis(),
            "lag_probe"
        );
    }
    if lag_ms > latency_policy.max_frame_age_ms {
        if maybe_reanchor_video_lag_tracker_on_stale_idr(
            video_lag_tracker,
            flush_ts,
            lag_ms,
            latency_policy.max_frame_age_ms,
            contains_idr,
        ) {
            warn!(
                track = ?TrackType::Video,
                session_id = %session_id,
                remote_addr = %remote_addr,
                request_path = %request_path,
                lag_ms = lag_ms,
                threshold_ms = latency_policy.max_frame_age_ms,
                source_ts = flush_ts,
                prev_source_ts = previous_source_ts,
                contains_idr = contains_idr,
                "stale_frame_reanchor"
            );
        } else {
            *dropped_stale_frames += 1;
            let elapsed_ms = video_lag_tracker.anchor_local.elapsed().as_millis() as u32;
            let expected_source_ts = video_lag_tracker.anchor_source_ts.wrapping_add(elapsed_ms);
            warn!(
                track = ?TrackType::Video,
                session_id = %session_id,
                remote_addr = %remote_addr,
                request_path = %request_path,
                lag_ms = lag_ms,
                threshold_ms = latency_policy.max_frame_age_ms,
                source_ts = flush_ts,
                prev_source_ts = previous_source_ts,
                expected_source_ts = expected_source_ts,
                anchor_source_ts = video_lag_tracker.anchor_source_ts,
                anchor_elapsed_ms = elapsed_ms,
                contains_idr = contains_idr,
                "stale_frame_drop"
            );
            return;
        }
    }

    if lag_ms > latency_policy.lag_recovery_threshold_ms {
        *sustained_video_lag_frames = sustained_video_lag_frames.saturating_add(1);
    } else {
        *sustained_video_lag_frames = 0;
    }

    if latency_policy.lag_recovery_mode == LagRecoveryMode::LatestIdr
        && *sustained_video_lag_frames >= latency_policy.sustained_lag_frames
        && !contains_idr
    {
        *waiting_for_idr_recovery = true;
        *sustained_video_lag_frames = 0;
        *dropped_for_recovery += 1;
        *idr_recovery_count += 1;
        warn!(
            track = ?TrackType::Video,
            session_id = %session_id,
            remote_addr = %remote_addr,
            request_path = %request_path,
            lag_ms = lag_ms,
            threshold_ms = latency_policy.lag_recovery_threshold_ms,
            sustained_frames = latency_policy.sustained_lag_frames,
            "lag_recovery_trigger"
        );
        return;
    }

    let ctx = PlaybackVideoSendContext {
        session_id,
        remote_addr,
        request_path,
        shutdown,
    };

    send_video_access_unit(
        video_channel,
        timestamp_normalizers,
        &ctx,
        flush_ts,
        &mut flush_data,
    )
    .await;
}

fn maybe_reanchor_video_lag_tracker_on_stale_idr(
    video_lag_tracker: &mut LagTracker,
    source_timestamp_ms: u32,
    lag_ms: u32,
    max_frame_age_ms: u32,
    contains_idr: bool,
) -> bool {
    if !contains_idr || lag_ms <= max_frame_age_ms {
        return false;
    }

    video_lag_tracker.anchor_local = Instant::now();
    video_lag_tracker.anchor_source_ts = source_timestamp_ms;
    video_lag_tracker.last_source_ts = source_timestamp_ms;
    true
}

async fn flush_pending_video(
    video_channel: &Option<Arc<Mutex<RtpChannel>>>,
    video_assembler: &mut VideoAccessUnitAssembler,
    timestamp_normalizers: &mut HashMap<TrackType, RtpTimestampNormalizer>,
    session_id: &str,
    remote_addr: SocketAddr,
    request_path: &str,
    shutdown: &Arc<AtomicBool>,
) {
    if let (Some(video_channel), Some((timestamp, mut data))) =
        (video_channel, video_assembler.flush())
    {
        let ctx = PlaybackVideoSendContext {
            session_id,
            remote_addr,
            request_path,
            shutdown,
        };

        send_video_access_unit(
            video_channel,
            timestamp_normalizers,
            &ctx,
            timestamp,
            &mut data,
        )
        .await;
    }
}

#[allow(clippy::too_many_arguments)]
async fn handle_playback_frame(
    frame_data: FrameData,
    audio_rtp_channel: &Option<Arc<Mutex<RtpChannel>>>,
    video_rtp_channel: &Option<Arc<Mutex<RtpChannel>>>,
    video_assembler: &mut VideoAccessUnitAssembler,
    timestamp_normalizers: &mut HashMap<TrackType, RtpTimestampNormalizer>,
    latency_policy: PlaybackLatencyPolicy,
    audio_lag_tracker: &mut LagTracker,
    video_lag_tracker: &mut LagTracker,
    sustained_video_lag_frames: &mut u32,
    waiting_for_idr_recovery: &mut bool,
    dropped_stale_frames: &mut u64,
    dropped_for_recovery: &mut u64,
    idr_recovery_count: &mut u64,
    session_id: &str,
    remote_addr: SocketAddr,
    request_path: &str,
    shutdown: &Arc<AtomicBool>,
) -> bool {
    match frame_data {
        FrameData::Audio {
            timestamp,
            mut data,
        } => {
            if let Some(audio_channel) = audio_rtp_channel {
                process_audio_frame(
                    audio_channel,
                    timestamp_normalizers,
                    latency_policy,
                    audio_lag_tracker,
                    session_id,
                    remote_addr,
                    request_path,
                    shutdown,
                    timestamp,
                    &mut data,
                    dropped_stale_frames,
                    *waiting_for_idr_recovery,
                    dropped_for_recovery,
                )
                .await;

                return shutdown.load(Ordering::Acquire);
            }
        }
        FrameData::Video { timestamp, data } => {
            if let Some(video_channel) = video_rtp_channel {
                process_video_frame(
                    video_channel,
                    video_assembler,
                    timestamp_normalizers,
                    latency_policy,
                    video_lag_tracker,
                    sustained_video_lag_frames,
                    waiting_for_idr_recovery,
                    dropped_stale_frames,
                    dropped_for_recovery,
                    idr_recovery_count,
                    session_id,
                    remote_addr,
                    request_path,
                    shutdown,
                    timestamp,
                    data,
                )
                .await;
            }
        }
        _ => {}
    }

    false
}

#[allow(clippy::too_many_arguments)]
async fn handle_no_frame_data(
    video_rtp_channel: &Option<Arc<Mutex<RtpChannel>>>,
    video_assembler: &mut VideoAccessUnitAssembler,
    timestamp_normalizers: &mut HashMap<TrackType, RtpTimestampNormalizer>,
    session_id: &str,
    remote_addr: SocketAddr,
    request_path: &str,
    shutdown: &Arc<AtomicBool>,
    retry_times: &mut usize,
) -> bool {
    flush_pending_video(
        video_rtp_channel,
        video_assembler,
        timestamp_normalizers,
        session_id,
        remote_addr,
        request_path,
        shutdown,
    )
    .await;

    *retry_times += 1;
    info!(retry_times = *retry_times, "no_frame_data_retry");

    if *retry_times > 10 {
        shutdown.store(true, Ordering::Release);
        return true;
    }

    false
}

#[allow(clippy::too_many_arguments)]
async fn run_playback_loop(
    mut receiver: mpsc::UnboundedReceiver<FrameData>,
    audio_rtp_channel: Option<Arc<Mutex<RtpChannel>>>,
    video_rtp_channel: Option<Arc<Mutex<RtpChannel>>>,
    playback_cancel: Arc<AtomicBool>,
    shutdown: Arc<AtomicBool>,
    session_id: String,
    remote_addr: SocketAddr,
    request_path: String,
    latency_policy: PlaybackLatencyPolicy,
) {
    let mut timestamp_normalizers: HashMap<TrackType, RtpTimestampNormalizer> = HashMap::new();
    let mut retry_times: usize = 0;
    let mut video_assembler = VideoAccessUnitAssembler::default();
    let mut audio_lag_tracker = LagTracker::default();
    let mut video_lag_tracker = LagTracker::default();
    let mut sustained_video_lag_frames = 0u32;
    let mut waiting_for_idr_recovery = false;
    let mut dropped_stale_frames = 0u64;
    let mut dropped_for_recovery = 0u64;
    let mut idr_recovery_count = 0u64;
    let mut frame_pacer = FramePacer::new();

    loop {
        if playback_cancel.load(Ordering::Acquire) {
            flush_pending_video(
                &video_rtp_channel,
                &mut video_assembler,
                &mut timestamp_normalizers,
                &session_id,
                remote_addr,
                &request_path,
                &shutdown,
            )
            .await;
            break;
        }

        match receiver.recv().await {
            Some(frame_data) => {
                retry_times = 0;

                // Pace frame delivery to approximate real-time timing.
                // Without this, frames dequeued from the ring buffer are
                // sent in bursts, overwhelming VLC's jitter buffer.
                let timestamp_ms = pacing_timestamp_ms(&frame_data);
                if let Some(ts) = timestamp_ms {
                    let paced_sleep_ms = frame_pacer.pace(ts).await;
                    if paced_sleep_ms >= PACER_SLEEP_DIAGNOSTIC_MIN_MS {
                        debug!(
                            session_id = %session_id,
                            remote_addr = %remote_addr,
                            request_path = %request_path,
                            source_ts = ts,
                            sleep_ms = paced_sleep_ms,
                            diag_monotonic_ms = monotonic_millis(),
                            "playback_pacer_sleep"
                        );
                    }
                }

                if handle_playback_frame(
                    frame_data,
                    &audio_rtp_channel,
                    &video_rtp_channel,
                    &mut video_assembler,
                    &mut timestamp_normalizers,
                    latency_policy,
                    &mut audio_lag_tracker,
                    &mut video_lag_tracker,
                    &mut sustained_video_lag_frames,
                    &mut waiting_for_idr_recovery,
                    &mut dropped_stale_frames,
                    &mut dropped_for_recovery,
                    &mut idr_recovery_count,
                    &session_id,
                    remote_addr,
                    &request_path,
                    &shutdown,
                )
                .await
                {
                    break;
                }
            }
            None => {
                if handle_no_frame_data(
                    &video_rtp_channel,
                    &mut video_assembler,
                    &mut timestamp_normalizers,
                    &session_id,
                    remote_addr,
                    &request_path,
                    &shutdown,
                    &mut retry_times,
                )
                .await
                {
                    break;
                }
            }
        }
    }

    info!(
        session_id = %session_id,
        remote_addr = %remote_addr,
        request_path = %request_path,
        dropped_stale_frames = dropped_stale_frames,
        dropped_for_recovery = dropped_for_recovery,
        idr_recovery_count = idr_recovery_count,
        "playback_loop_exit"
    );
}

#[derive(Debug, Clone, Copy)]
struct RtpPacketObservation {
    packets_sent: u64,
    bytes_sent: u64,
    prev_seq: Option<u16>,
    seq_delta: Option<u16>,
    prev_timestamp: Option<u32>,
    timestamp_delta: Option<u32>,
    seq_gap: bool,
    seq_regressed: bool,
    timestamp_regressed: bool,
}

impl RtpTrackCounters {
    fn new() -> Self {
        Self {
            packet_count: AtomicU64::new(0),
            byte_count: AtomicU64::new(0),
            first_send_ms: AtomicU64::new(0),
            last_send_ms: AtomicU64::new(0),
            last_seq: AtomicU32::new(u32::MAX),
            last_timestamp: AtomicU32::new(u32::MAX),
        }
    }

    /// Record a sent RTP packet and return counters plus monotonicity checks.
    fn on_packet_sent(&self, payload_len: usize, seq: u16, timestamp: u32) -> RtpPacketObservation {
        let now = now_millis();

        // First-send timestamp (best-effort, race-safe).
        let _ = self
            .first_send_ms
            .compare_exchange(0, now, Ordering::Relaxed, Ordering::Relaxed);
        self.last_send_ms.store(now, Ordering::Relaxed);

        let packets = self.packet_count.fetch_add(1, Ordering::Relaxed) + 1;
        let bytes = self
            .byte_count
            .fetch_add(payload_len as u64, Ordering::Relaxed)
            + payload_len as u64;

        let prev_seq_raw = self.last_seq.swap(seq as u32, Ordering::Relaxed);
        let prev_timestamp_raw = self.last_timestamp.swap(timestamp, Ordering::Relaxed);

        let prev_seq = if prev_seq_raw == u32::MAX {
            None
        } else {
            Some(prev_seq_raw as u16)
        };
        let prev_timestamp = if prev_timestamp_raw == u32::MAX {
            None
        } else {
            Some(prev_timestamp_raw)
        };

        let seq_delta = prev_seq.map(|prev| seq.wrapping_sub(prev));
        let seq_gap = matches!(seq_delta, Some(delta) if delta > 1 && delta < 0x8000);
        let seq_regressed = matches!(seq_delta, Some(delta) if delta >= 0x8000);

        let timestamp_delta = prev_timestamp.map(|prev| timestamp.wrapping_sub(prev));
        let timestamp_regressed =
            matches!(timestamp_delta, Some(delta) if delta > RTP_TIMESTAMP_WRAP_THRESHOLD);

        RtpPacketObservation {
            packets_sent: packets,
            bytes_sent: bytes,
            prev_seq,
            seq_delta,
            prev_timestamp,
            timestamp_delta,
            seq_gap,
            seq_regressed,
            timestamp_regressed,
        }
    }

    fn snapshot(&self) -> (u64, u64, Option<u64>) {
        let packets = self.packet_count.load(Ordering::Relaxed);
        let bytes = self.byte_count.load(Ordering::Relaxed);
        let first = self.first_send_ms.load(Ordering::Relaxed);
        let last = self.last_send_ms.load(Ordering::Relaxed);
        let duration_ms = if first > 0 && last >= first {
            Some(last - first)
        } else {
            None
        };
        (packets, bytes, duration_ms)
    }
}

type RtpCountersHandle = Arc<RtpTrackCounters>;

pub struct RtspServerSession {
    io_reader: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>,
    io_writer: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>,
    reader: BytesReader,
    writer: AsyncBytesWriter,

    tracks: HashMap<TrackType, RtspTrack>,
    sdp: Sdp,
    pub session_id: Option<Uuid>,
    pub session_type: define::ServerSessionType,
    has_published: bool,
    has_subscribed: bool,

    stream_handler: Arc<RtspStreamHandler>,
    event_producer: StreamHubEventSender,

    auth: Option<Auth>,

    pub stream_identifier: Option<StreamIdentifier>,
    pub is_normal_exit: bool,
    remote_addr: SocketAddr,
    shutdown: Arc<AtomicBool>,

    /// Optional shared shutdown flag for graceful shutdown from server
    shutdown_flag: Option<Arc<AtomicBool>>,

    /// Packet-level RTP logging configuration and counters.
    ///
    /// `rtp_sample_interval` stores the packet sampling interval configured
    /// at session creation time (every N packets to log, 0 = disabled).
    rtp_sample_interval: u32,

    /// Streaming configuration for session setup and playback.
    #[allow(dead_code)]
    config: StreamingConfig,

    rtp_counters: HashMap<TrackType, RtpCountersHandle>,
    playback_cancel: Option<Arc<AtomicBool>>,
    playback_task: Option<JoinHandle<()>>,
}

pub struct InterleavedBinaryData {
    pub channel_identifier: u8,
    pub length: u16,
}

impl InterleavedBinaryData {
    // 10.12 Embedded (Interleaved) Binary Data
    // Stream data such as RTP packets is encapsulated by an ASCII dollar
    // sign (24 hexadecimal), followed by a one-byte channel identifier,
    // followed by the length of the encapsulated binary data as a binary,
    // two-byte integer in network byte order
    pub fn new(reader: &mut BytesReader) -> Result<Option<Self>, SessionError> {
        let is_dollar_sign = reader.advance_u8()? == 0x24;
        if crate::stream_frame_debug_logging_enabled() {
            debug!(is_dollar_sign, "interleaved_parse");
        }
        if is_dollar_sign {
            reader.read_u8()?;
            let channel_identifier = reader.read_u8()?;
            if crate::stream_frame_debug_logging_enabled() {
                debug!(channel_identifier = channel_identifier, "channel_id_parse");
            }
            let length = reader.read_u16::<BigEndian>()?;
            if crate::stream_frame_debug_logging_enabled() {
                debug!(length = length, "interleaved_length");
            }
            // RFC 2326 §10.12: validate interleaved payload length
            if length == 0 {
                warn!(
                    channel = channel_identifier,
                    "zero_length_interleaved_payload"
                );
            }
            return Ok(Some(InterleavedBinaryData {
                channel_identifier,
                length,
            }));
        }
        Ok(None)
    }
}

impl RtspServerSession {
    pub fn new(
        stream: TcpStream,
        event_producer: StreamHubEventSender,
        auth: Option<Auth>,
        config: StreamingConfig,
    ) -> Self {
        let remote_addr = stream
            .peer_addr()
            .or_else(|_| stream.local_addr())
            .unwrap_or_else(|_| SocketAddr::from(([0, 0, 0, 0], 0)));

        // Enable TCP_NODELAY to reduce latency on small RTP packets
        if let Err(err) = stream.set_nodelay(true) {
            tracing::warn!(error = %err, remote_addr = %remote_addr, "failed_to_set_tcp_nodelay");
        }

        let (read_half, write_half) = stream.into_split();
        let read_io: Box<dyn TNetIO + Send + Sync> = Box::new(TcpReadIO::new(read_half));
        let write_io: Box<dyn TNetIO + Send + Sync> = Box::new(TcpWriteIO::new(write_half));

        Self::new_with_io_pair(read_io, write_io, event_producer, auth, remote_addr, config)
    }

    pub fn new_with_io(
        io: Box<dyn TNetIO + Send + Sync>,
        event_producer: StreamHubEventSender,
        auth: Option<Auth>,
        remote_addr: SocketAddr,
        config: StreamingConfig,
    ) -> Self {
        // Tests and in-memory sessions can share one IO object for both read and write.
        let io = Arc::new(Mutex::new(io));
        Self::new_with_shared_io(io, event_producer, auth, remote_addr, config)
    }

    pub fn new_with_io_pair(
        read_io: Box<dyn TNetIO + Send + Sync>,
        write_io: Box<dyn TNetIO + Send + Sync>,
        event_producer: StreamHubEventSender,
        auth: Option<Auth>,
        remote_addr: SocketAddr,
        config: StreamingConfig,
    ) -> Self {
        let read_io = Arc::new(Mutex::new(read_io));
        let write_io = Arc::new(Mutex::new(write_io));
        Self::new_with_reader_writer_io(
            read_io,
            write_io,
            event_producer,
            auth,
            remote_addr,
            config,
        )
    }

    fn new_with_shared_io(
        io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>,
        event_producer: StreamHubEventSender,
        auth: Option<Auth>,
        remote_addr: SocketAddr,
        config: StreamingConfig,
    ) -> Self {
        Self::new_with_reader_writer_io(io.clone(), io, event_producer, auth, remote_addr, config)
    }

    fn new_with_reader_writer_io(
        io_reader: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>,
        io_writer: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>,
        event_producer: StreamHubEventSender,
        auth: Option<Auth>,
        remote_addr: SocketAddr,
        config: StreamingConfig,
    ) -> Self {
        let sample_interval = config.rtp_sample_interval;
        Self {
            io_reader,
            io_writer: io_writer.clone(),
            reader: BytesReader::new(BytesMut::default()),
            writer: AsyncBytesWriter::new(io_writer),
            tracks: HashMap::new(),
            sdp: Sdp::default(),
            session_id: None,
            session_type: define::ServerSessionType::Push,
            has_published: false,
            has_subscribed: false,
            event_producer,
            stream_handler: Arc::new(RtspStreamHandler::new()),
            auth,
            stream_identifier: None,
            is_normal_exit: false,
            remote_addr,
            shutdown: Arc::new(AtomicBool::new(false)),
            shutdown_flag: None,
            rtp_sample_interval: sample_interval,
            config,
            rtp_counters: HashMap::new(),
            playback_cancel: None,
            playback_task: None,
        }
    }

    /// Set the shutdown flag for graceful shutdown from server
    pub fn set_shutdown_flag(&mut self, flag: Arc<AtomicBool>) {
        self.shutdown_flag = Some(flag);
    }

    /// RFC 2326 §12.37: default session timeout in seconds.
    const SESSION_TIMEOUT_SECS: u64 = 60;

    /// Read from the IO reader with session timeout. Returns error on timeout.
    async fn read_with_session_timeout(
        io_reader: &Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>,
        timeout_secs: u64,
        remote_addr: &SocketAddr,
    ) -> Result<BytesMut, SessionError> {
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(timeout_secs),
            io_reader.lock().await.read(),
        )
        .await;
        match result {
            Ok(Ok(data)) => Ok(data),
            Ok(Err(e)) => Err(e.into()),
            Err(_) => {
                info!(
                    timeout_secs = timeout_secs,
                    remote_addr = %remote_addr,
                    "rtsp_session_timeout"
                );
                Err(SessionError {
                    value: SessionErrorValue::SessionTimeout(timeout_secs),
                })
            }
        }
    }

    async fn read_rtsp_data(&mut self) -> Result<(), SessionError> {
        while self.reader.len() < 4 {
            let data = Self::read_with_session_timeout(
                &self.io_reader,
                Self::SESSION_TIMEOUT_SECS,
                &self.remote_addr,
            )
            .await?;
            self.reader.extend_from_slice(&data[..]);
        }
        Ok(())
    }

    async fn handle_interleaved_data(&mut self) -> Result<(), SessionError> {
        if let Ok(data) = InterleavedBinaryData::new(&mut self.reader) {
            match data {
                Some(a) => {
                    const INTERLEAVED_READ_TIMEOUT_SECS: u64 = 30;

                    while self.reader.len() < a.length as usize {
                        let read_result = timeout(
                            Duration::from_secs(INTERLEAVED_READ_TIMEOUT_SECS),
                            self.io_reader.lock().await.read(),
                        )
                        .await;

                        match read_result {
                            Ok(Ok(data)) => {
                                self.reader.extend_from_slice(&data[..]);
                            }
                            Ok(Err(e)) => {
                                return Err(SessionError::from(e));
                            }
                            Err(_) => {
                                // Timeout - close connection
                                return Err(SessionError::from(std::io::Error::new(
                                    std::io::ErrorKind::TimedOut,
                                    format!(
                                        "Interleaved read timeout after {}s",
                                        INTERLEAVED_READ_TIMEOUT_SECS
                                    ),
                                )));
                            }
                        }
                    }
                    self.on_rtp_over_rtsp_message(a.channel_identifier, a.length as usize)
                        .await?;
                }
                None => {
                    self.on_rtsp_message().await?;
                }
            }
        }
        Ok(())
    }

    pub async fn run(&mut self) -> Result<(), SessionError> {
        // Clone shutdown flag for the run loop
        let shutdown_flag = self.shutdown_flag.clone();

        let run_result: Result<(), SessionError> = async {
            loop {
                if self.shutdown.load(Ordering::Acquire) {
                    break;
                }

                // Check shared shutdown flag if provided
                if let Some(ref flag) = shutdown_flag
                    && flag.load(Ordering::Acquire)
                {
                    break;
                }

                self.read_rtsp_data().await?;

                self.handle_interleaved_data().await?;
            }
            Ok(())
        }
        .await;

        self.stop_playback_task().await;

        // Ensure StreamHub receives unsubscribe/unpublish even when the run loop exits
        // without an explicit TEARDOWN/PAUSE (e.g. remote disconnect or internal shutdown).
        // This prevents stale subscriber counts from keeping on-demand publishers active.
        if !self.is_normal_exit
            && let Some(identifier) = self.stream_identifier.clone()
        {
            match self.exit(identifier) {
                Ok(()) => {}
                Err(cleanup_err) => {
                    // Preserve the original run error when present, but surface cleanup
                    // failures when run itself was otherwise successful.
                    if run_result.is_ok() {
                        return Err(cleanup_err);
                    }
                }
            }
        }

        run_result
    }

    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::Release);
    }

    fn abort_playback_task(&mut self) {
        if let Some(cancel) = self.playback_cancel.take() {
            cancel.store(true, Ordering::Release);
        }
        if let Some(handle) = self.playback_task.take() {
            handle.abort();
        }
    }

    async fn stop_playback_task(&mut self) {
        if let Some(cancel) = self.playback_cancel.take() {
            cancel.store(true, Ordering::Release);
        }
        if let Some(handle) = self.playback_task.take() {
            handle.abort();
            match handle.await {
                Ok(()) => {}
                Err(err) if err.is_cancelled() => {}
                Err(err) => {
                    warn!(error = %err, "playback_task_join_error");
                }
            }
        }
    }

    async fn on_rtp_over_rtsp_message(
        &mut self,
        channel_identifier: u8,
        length: usize,
    ) -> Result<(), SessionError> {
        let mut cur_reader = BytesReader::new(self.reader.read_bytes(length)?);

        for track in self.tracks.values_mut() {
            if let Some(interleaveds) = track.transport.interleaved {
                let rtp_identifier = interleaveds[0];
                let rtcp_identifier = interleaveds[1];

                if channel_identifier == rtp_identifier {
                    track.on_rtp(&mut cur_reader).await?;
                } else if channel_identifier == rtcp_identifier {
                    track.on_rtcp(&mut cur_reader, self.io_writer.clone()).await;
                }
            }
        }
        Ok(())
    }

    async fn parse_rtsp_request(
        &mut self,
        message_len: usize,
    ) -> Result<RtspRequest, SessionError> {
        let message_bytes = self.reader.read_bytes(message_len)?;
        let message_str = std::str::from_utf8(&message_bytes)?;

        let Some(rtsp_request) = RtspRequest::unmarshal(message_str) else {
            return Err(SessionError {
                value: SessionErrorValue::RtspMessageCorrupted("request parse failed".to_string()),
            });
        };
        Ok(rtsp_request)
    }

    /// Read until we have a complete RTSP message; returns the message length.
    async fn read_until_complete_message_len(&mut self) -> Result<usize, SessionError> {
        const MAX_RETRIES: u32 = 16;
        let mut retry_count = 0;
        loop {
            let data = self.reader.get_remaining_bytes();
            match try_get_complete_message_len(&data) {
                Ok(Some(len)) => return Ok(len),
                Ok(None) => {
                    if retry_count >= MAX_RETRIES {
                        return Err(SessionError {
                            value: SessionErrorValue::RtspMessageCorrupted(
                                "max read retries exceeded".to_string(),
                            ),
                        });
                    }
                    retry_count += 1;
                    let data_recv = self.io_reader.lock().await.read().await?;
                    self.reader.extend_from_slice(&data_recv[..]);
                }
                Err(err) => {
                    return Err(SessionError {
                        value: SessionErrorValue::RtspMessageCorrupted(err),
                    });
                }
            }
        }
    }

    /// If the request fails version or Content-Length validation, returns the error response to send.
    fn validate_rtsp_request_headers(
        rtsp_request: &RtspRequest,
        remote_addr: &SocketAddr,
    ) -> Option<RtspResponse> {
        if rtsp_request.version != "RTSP/1.0" {
            warn!(
                version = %rtsp_request.version,
                remote_addr = %remote_addr,
                "rtsp_unsupported_version"
            );
            return Some(Self::gen_response(
                http::StatusCode::HTTP_VERSION_NOT_SUPPORTED,
                rtsp_request,
            ));
        }
        if let Some(cl_str) = rtsp_request.get_header("Content-Length") {
            let claimed: usize = cl_str.trim().parse().unwrap_or(0);
            let actual = rtsp_request.body.as_ref().map_or(0, |b| b.len());
            if claimed != actual {
                warn!(
                    claimed = claimed,
                    actual = actual,
                    remote_addr = %remote_addr,
                    "rtsp_content_length_mismatch"
                );
                return Some(Self::gen_response(
                    http::StatusCode::BAD_REQUEST,
                    rtsp_request,
                ));
            }
        }
        None
    }

    //publish stream: OPTIONS->ANNOUNCE->SETUP->RECORD->TEARDOWN
    //subscribe stream: OPTIONS->DESCRIBE->SETUP->PLAY->TEARDOWN
    async fn on_rtsp_message(&mut self) -> Result<(), SessionError> {
        let message_len = self.read_until_complete_message_len().await?;
        let rtsp_request = self.parse_rtsp_request(message_len).await?;

        if let Some(response) =
            Self::validate_rtsp_request_headers(&rtsp_request, &self.remote_addr)
        {
            self.send_response(&response).await?;
            return Ok(());
        }

        match rtsp_request.method.as_str() {
            rtsp_method_name::OPTIONS => {
                self.handle_options(&rtsp_request).await?;
            }
            rtsp_method_name::DESCRIBE => {
                self.handle_describe(&rtsp_request).await?;
            }
            rtsp_method_name::ANNOUNCE => {
                self.handle_announce(&rtsp_request).await?;
            }
            rtsp_method_name::SETUP => {
                self.handle_setup(&rtsp_request).await?;
            }
            rtsp_method_name::PLAY => {
                if let Err(err) = self.handle_play(&rtsp_request).await {
                    info!(error = %err, "handle_play_error");
                }
            }
            rtsp_method_name::RECORD => {
                self.handle_record(&rtsp_request).await?;
            }
            rtsp_method_name::TEARDOWN => {
                self.stop_playback_task().await;
                if let Some(response) = self.validate_session_id(&rtsp_request) {
                    self.send_response(&response).await?;
                } else {
                    self.handle_teardown(&rtsp_request)?;
                    let mut response = Self::gen_response(http::StatusCode::OK, &rtsp_request);
                    if let Some(session_id) = self.session_id {
                        response
                            .headers
                            .insert("Session".to_string(), session_id.to_string());
                    }
                    self.send_response(&response).await?;
                }
            }
            rtsp_method_name::PAUSE => {
                self.handle_pause(&rtsp_request).await?;
            }
            rtsp_method_name::GET_PARAMETER => {
                self.handle_get_parameter(&rtsp_request).await?;
            }
            rtsp_method_name::SET_PARAMETER => {
                self.handle_set_parameter(&rtsp_request).await?;
            }
            rtsp_method_name::REDIRECT => {
                self.handle_redirect(&rtsp_request).await?;
            }

            _ => {
                warn!(
                    method = %rtsp_request.method,
                    remote_addr = %self.remote_addr,
                    "rtsp_unknown_method"
                );
                let response = Self::gen_response(http::StatusCode::NOT_IMPLEMENTED, &rtsp_request);
                self.send_response(&response).await?;
            }
        }
        Ok(())
    }

    async fn handle_options(&mut self, rtsp_request: &RtspRequest) -> Result<(), SessionError> {
        let status_code = http::StatusCode::OK;
        let mut response = Self::gen_response(status_code, rtsp_request);
        let public_str = rtsp_method_name::PUBLIC_METHODS.join(", ");
        response.headers.insert("Public".to_string(), public_str);
        self.send_response(&response).await?;

        let cseq = rtsp_request.get_header("CSeq").cloned().unwrap_or_default();
        let session_id = self
            .session_id
            .map(|id| id.to_string())
            .unwrap_or_else(|| "none".to_string());
        info!(
            method = "OPTIONS",
            cseq = %cseq,
            session_id = %session_id,
            remote_addr = %self.remote_addr,
            stream_path = %rtsp_request.uri.path,
            session_type = ?self.session_type,
            "rtsp_request"
        );

        Ok(())
    }

    async fn handle_describe(&mut self, rtsp_request: &RtspRequest) -> Result<(), SessionError> {
        if self.auth.is_none() {
            let has_authorization_header = rtsp_request.get_header("Authorization").is_some();
            let has_userinfo_in_uri = rtsp_request.uri.host.contains('@');
            if has_authorization_header || has_userinfo_in_uri {
                self.send_unauthorized_response(rtsp_request).await?;
                return Ok(());
            }
        }

        if let Some(auth) = &self.auth {
            let stream_name = rtsp_request.uri.path.clone();
            let auth_result = auth.authenticate_request(
                &stream_name,
                &rtsp_request.uri.query,
                rtsp_request
                    .get_header("Authorization")
                    .map(std::string::String::as_str),
                true,
            );
            if auth_result.is_err() {
                self.send_unauthorized_response(rtsp_request).await?;
                return Ok(());
            }
        }

        // The sender is used for sending sdp information from the server session to client session
        // receiver is used to receive the sdp information
        let (sender, mut receiver) = mpsc::unbounded_channel();

        let stream_path = self.normalize_rtsp_stream_path(&rtsp_request.uri.path);
        let identifier = StreamIdentifier::Rtsp { stream_path };
        self.stream_identifier = Some(identifier.clone());

        let request_event = StreamHubEvent::Request { identifier, sender };

        if self.event_producer.send(request_event).is_err() {
            return Err(SessionError {
                value: SessionErrorValue::StreamHubEventSendErr,
            });
        }

        if let Some(Information::Sdp { data }) = receiver.recv().await
            && let Ok(sdp) = Sdp::unmarshal(&data)
        {
            self.sdp = sdp;
            //it can new tracks when get the sdp information;
            self.new_tracks()?;
        }

        // M-02: RFC 2326 §12.1: honour Accept header in DESCRIBE
        if let Some(accept) = rtsp_request.get_header("Accept") {
            let dominated = accept.contains("application/sdp") || accept.contains("*/*");
            if !dominated {
                warn!(
                    accept = %accept,
                    remote_addr = %self.remote_addr,
                    "rtsp_not_acceptable"
                );
                let response = Self::gen_response(http::StatusCode::NOT_ACCEPTABLE, rtsp_request);
                self.send_response(&response).await?;
                return Ok(());
            }
        }

        if self.sdp.medias.is_empty() {
            let response = Self::gen_response(http::StatusCode::NOT_FOUND, rtsp_request);
            self.send_response(&response).await?;
            return Ok(());
        }

        let mut response = Self::gen_response(http::StatusCode::OK, rtsp_request);
        let sdp = self.sdp.marshal();
        response.body = Some(sdp);
        response
            .headers
            .insert("Content-Type".to_string(), "application/sdp".to_string());
        if let Some(content_base) = self.build_content_base(rtsp_request) {
            response
                .headers
                .insert("Content-Base".to_string(), content_base);
        }
        self.send_response(&response).await?;

        let cseq = rtsp_request.get_header("CSeq").cloned().unwrap_or_default();
        let session_id = self
            .session_id
            .map(|id| id.to_string())
            .unwrap_or_else(|| "none".to_string());
        let media_count = self.sdp.medias.len();
        info!(
            method = "DESCRIBE",
            cseq = %cseq,
            session_id = %session_id,
            remote_addr = %self.remote_addr,
            stream_path = %rtsp_request.uri.path,
            session_type = ?self.session_type,
            sdp_media_count = media_count,
            "rtsp_request"
        );

        Ok(())
    }

    async fn handle_announce(&mut self, rtsp_request: &RtspRequest) -> Result<(), SessionError> {
        if let Some(auth) = &self.auth {
            let stream_name = rtsp_request.uri.path.clone();
            auth.authenticate(
                &stream_name,
                &rtsp_request
                    .uri
                    .query
                    .as_ref()
                    .map(|q| SecretCarrier::Query(q.to_string())),
                false,
            )?;
        }

        if let Some(request_body) = &rtsp_request.body
            && let Ok(sdp) = Sdp::unmarshal(request_body)
        {
            self.sdp = sdp.clone();
            self.stream_handler.set_sdp(sdp).await;
        }

        //new tracks for publish session
        self.new_tracks()?;

        let (event_result_sender, event_result_receiver) = oneshot::channel();

        let identifier = StreamIdentifier::Rtsp {
            stream_path: rtsp_request.uri.path.clone(),
        };
        self.stream_identifier = Some(identifier.clone());

        let publish_event = StreamHubEvent::Publish {
            identifier,
            result_sender: event_result_sender,
            info: self.get_publisher_info(),
            stream_handler: self.stream_handler.clone(),
        };

        if self.event_producer.send(publish_event).is_err() {
            return Err(SessionError {
                value: SessionErrorValue::StreamHubEventSendErr,
            });
        }

        let sender = event_result_receiver.await??.0.ok_or(SessionError {
            value: SessionErrorValue::MissingFrameSender,
        })?;

        for track in self.tracks.values_mut() {
            let sender_out = sender.clone();
            let mut rtp_channel_guard = track.rtp_channel.lock().await;

            rtp_channel_guard.on_frame_handler(Box::new(
                move |msg: FrameData| -> Result<(), UnPackerError> {
                    if let Err(err) = sender_out.send(msg) {
                        error!(error = %err, "send_frame_error");
                    }
                    Ok(())
                },
            ));

            let rtcp_channel = Arc::clone(&track.rtcp_channel);
            rtp_channel_guard.on_packet_for_rtcp_handler(Box::new(move |packet: RtpPacket| {
                let rtcp_channel_in = Arc::clone(&rtcp_channel);
                Box::pin(async move {
                    rtcp_channel_in.lock().await.on_packet(packet);
                })
            }));
        }

        self.has_published = true;
        self.session_type = define::ServerSessionType::Push;

        let status_code = http::StatusCode::OK;
        let mut response = Self::gen_response(status_code, rtsp_request);
        if let Some(session_id) = self.session_id {
            response
                .headers
                .insert("Session".to_string(), session_id.to_string());
        }
        self.send_response(&response).await?;

        Ok(())
    }

    fn ensure_stream_identifier(&mut self, request_path: &str) {
        if self.stream_identifier.is_none() {
            let normalized_path = self.normalize_rtsp_stream_path(request_path);
            if !normalized_path.is_empty() {
                self.stream_identifier = Some(StreamIdentifier::Rtsp {
                    stream_path: normalized_path,
                });
            }
        }
    }

    fn find_track_for_uri(&self, uri: &crate::common::http::Uri) -> Option<TrackType> {
        let request_uri = uri.marshal();
        for (track_type, track) in &self.tracks {
            if !track.media_control.is_empty() && request_uri.contains(&track.media_control) {
                return Some(track_type.clone());
            }
        }
        if self.tracks.contains_key(&TrackType::Video) {
            return Some(TrackType::Video);
        }
        self.tracks
            .iter()
            .next()
            .map(|(track_type, _)| track_type.clone())
    }

    async fn setup_tcp_transport(
        io_writer: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>,
        track: &mut RtspTrack,
        transport: &RtspTransport,
    ) {
        // set_transport MUST happen before rtcp_send_loop: the loop's first
        // RTCP SR fires immediately (tokio::time::interval first-tick semantics)
        // and needs the correct interleaved channel_identifier already assigned.
        track.set_transport(transport.clone()).await;
        track.create_packer(io_writer.clone()).await;
        track.rtcp_send_loop(io_writer).await;
    }

    async fn setup_udp_transport(
        remote_addr: SocketAddr,
        track: &mut RtspTrack,
        trans: &RtspTransport,
    ) -> Result<(Option<u16>, Option<u16>), SessionError> {
        let (rtp_port, rtcp_port) = trans
            .client_port
            .ok_or(SessionError {
                value: SessionErrorValue::MissingClientPort,
            })?
            .into();

        let address = remote_addr.ip().to_string();
        let mut rtp_server_port: Option<u16> = None;
        let mut rtcp_server_port: Option<u16> = None;

        if let Some(rtp_io) = UdpIO::new(address.clone(), rtp_port, 0).await {
            rtp_server_port = rtp_io.get_local_port();

            let box_udp_io: Box<dyn TNetIO + Send + Sync> = Box::new(rtp_io);
            let is_record = matches!(trans.transport_mod.as_deref(), Some("record"));
            if !is_record {
                track.create_packer(Arc::new(Mutex::new(box_udp_io))).await;
            } else {
                track.rtp_receive_loop(box_udp_io).await;
            }
        }

        let rtp_port = rtp_server_port.ok_or(SessionError {
            value: SessionErrorValue::MissingClientPort,
        })?;

        if let Some(rtcp_io) = UdpIO::new(address.clone(), rtcp_port, rtp_port + 1).await {
            rtcp_server_port = rtcp_io.get_local_port();
            let box_rtcp_io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>> =
                Arc::new(Mutex::new(Box::new(rtcp_io)));
            track.rtcp_receive_loop(box_rtcp_io.clone()).await;
            track.rtcp_send_loop(box_rtcp_io).await;
        }

        Ok((rtp_server_port, rtcp_server_port))
    }

    fn log_setup_request(&self, rtsp_request: &RtspRequest, track_type: &TrackType) {
        let cseq = rtsp_request.get_header("CSeq").cloned().unwrap_or_default();
        let session_id = self
            .session_id
            .map(|id| id.to_string())
            .unwrap_or_else(|| "none".to_string());
        let transport_hdr = rtsp_request
            .get_header("Transport")
            .cloned()
            .unwrap_or_default();
        info!(
            method = "SETUP",
            cseq = %cseq,
            session_id = %session_id,
            remote_addr = %self.remote_addr,
            stream_path = %rtsp_request.uri.path,
            session_type = ?self.session_type,
            track_type = ?track_type,
            transport_req = %transport_hdr,
            "rtsp_request"
        );
    }

    async fn handle_setup(&mut self, rtsp_request: &RtspRequest) -> Result<(), SessionError> {
        self.ensure_stream_identifier(&rtsp_request.uri.path);
        self.ensure_tracks_from_streamhub(rtsp_request).await?;

        let Some(track_type) = self.find_track_for_uri(&rtsp_request.uri) else {
            let response = Self::gen_response(http::StatusCode::NOT_FOUND, rtsp_request);
            self.send_response(&response).await?;
            return Ok(());
        };

        let Some(transport_data) = rtsp_request.get_header("Transport") else {
            self.send_response(&Self::gen_response(http::StatusCode::OK, rtsp_request))
                .await?;
            return Ok(());
        };

        if self.session_id.is_none() {
            self.session_id = Some(Uuid::new(SESSION_ID_RANDOM_DIGITS));
        }

        let transport = RtspTransport::unmarshal(transport_data);
        if let Err(ref err) = transport {
            warn!(
                error = %err,
                remote_addr = %self.remote_addr,
                "rtsp_unsupported_transport"
            );
            let response = Self::gen_response(
                http::StatusCode::from_u16(461).unwrap_or(http::StatusCode::BAD_REQUEST),
                rtsp_request,
            );
            self.send_response(&response).await?;
            return Ok(());
        }

        let Ok(mut trans) = transport else {
            unreachable!("transport error already handled");
        };

        let io_writer = self.io_writer.clone();
        let remote_addr = self.remote_addr;
        let (rtp_server_port, rtcp_server_port) = {
            let track = self.tracks.get_mut(&track_type).ok_or(SessionError {
                value: SessionErrorValue::RtspMessageCorrupted("track missing".to_string()),
            })?;

            match trans.protocol_type {
                ProtocolType::TCP => {
                    Self::setup_tcp_transport(io_writer, track, &trans).await;
                    (None, None)
                }
                ProtocolType::UDP => Self::setup_udp_transport(remote_addr, track, &trans).await?,
            }
        };

        let mut server_ports: [u16; 2] = [0, 0];
        if let Some(rtp_port) = rtp_server_port {
            server_ports[0] = rtp_port;
        }
        if let Some(rtcp_port) = rtcp_server_port {
            server_ports[1] = rtcp_port;
            trans.server_port = Some(server_ports);
        }

        let mut response = Self::gen_response(http::StatusCode::OK, rtsp_request);
        response
            .headers
            .insert("Transport".to_string(), trans.marshal());
        response.headers.insert(
            "Session".to_string(),
            self.session_id
                .ok_or(SessionError {
                    value: SessionErrorValue::MissingSessionId,
                })?
                .to_string(),
        );

        // For UDP, set_transport after server_port is assigned.
        // TCP already calls set_transport inside setup_tcp_transport (before rtcp_send_loop).
        if trans.protocol_type == ProtocolType::UDP {
            let track = self.tracks.get_mut(&track_type).ok_or(SessionError {
                value: SessionErrorValue::RtspMessageCorrupted("track missing".to_string()),
            })?;
            track.set_transport(trans).await;
        }

        self.send_response(&response).await?;
        self.log_setup_request(rtsp_request, &track_type);

        Ok(())
    }

    async fn ensure_tracks_from_streamhub(
        &mut self,
        rtsp_request: &RtspRequest,
    ) -> Result<(), SessionError> {
        if !self.tracks.is_empty() {
            return Ok(());
        }

        let stream_path = self.normalize_rtsp_stream_path(&rtsp_request.uri.path);
        if stream_path.is_empty() {
            return Ok(());
        }

        let (sender, mut receiver) = mpsc::unbounded_channel();
        let identifier = StreamIdentifier::Rtsp { stream_path };
        self.stream_identifier = Some(identifier.clone());

        let request_event = StreamHubEvent::Request { identifier, sender };

        if self.event_producer.send(request_event).is_err() {
            return Err(SessionError {
                value: SessionErrorValue::StreamHubEventSendErr,
            });
        }

        if let Some(Information::Sdp { data }) = receiver.recv().await
            && let Ok(sdp) = Sdp::unmarshal(&data)
        {
            self.sdp = sdp;
            self.new_tracks()?;
        }

        Ok(())
    }

    async fn wait_for_tracks(
        &mut self,
        rtsp_request: &RtspRequest,
        timeout: Duration,
    ) -> Result<bool, SessionError> {
        if !self.tracks.is_empty() {
            return Ok(true);
        }

        let deadline = Instant::now() + timeout;
        while Instant::now() <= deadline {
            self.ensure_tracks_from_streamhub(rtsp_request).await?;
            if !self.tracks.is_empty() {
                return Ok(true);
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }

        Ok(false)
    }

    fn resolve_stream_identifier(&mut self, request_path: &str) -> StreamIdentifier {
        if let Some(identifier) = &self.stream_identifier {
            return identifier.clone();
        }

        let normalized_path = self.normalize_rtsp_stream_path(request_path);
        let identifier = StreamIdentifier::Rtsp {
            stream_path: normalized_path,
        };
        self.stream_identifier = Some(identifier.clone());
        identifier
    }

    fn build_content_base(&self, request: &RtspRequest) -> Option<String> {
        let uri = &request.uri;
        let mut host = uri.host.clone();
        let mut port = uri.port;

        if host.is_empty()
            && let Some(host_header) = request.get_header("Host")
        {
            if let Some((host_val, port_val)) = host_header.split_once(':') {
                host = host_val.to_string();
                if let Ok(parsed_port) = port_val.parse::<u16>() {
                    port = Some(parsed_port);
                }
            } else {
                host = host_header.to_string();
            }
        }

        let normalized_path = self.normalize_rtsp_stream_path(&uri.path);
        let path = normalized_path.trim_matches('/');
        let mut base = if !host.is_empty() {
            let port_str = port.map(|val| format!(":{val}")).unwrap_or_default();
            if path.is_empty() {
                format!("rtsp://{host}{port_str}")
            } else {
                format!("rtsp://{host}{port_str}/{path}")
            }
        } else if !uri.path.is_empty() {
            uri.path.clone()
        } else {
            return None;
        };

        if !base.ends_with('/') {
            base.push('/');
        }

        Some(base)
    }

    async fn build_rtp_info_header(&self, rtsp_request: &RtspRequest) -> Option<String> {
        let content_base = self.build_content_base(rtsp_request)?;
        if self.tracks.is_empty() {
            return None;
        }

        let mut rtp_info_parts = Vec::new();
        for track_type in [TrackType::Video, TrackType::Audio, TrackType::Application] {
            if let Some(track) = self.tracks.get(&track_type) {
                let rtp_channel = track.rtp_channel.lock().await;
                let seq = rtp_channel.initial_sequence();
                // RFC 3550 §5.1 — use random initial timestamp, not hardcoded 0
                let rtptime = rtp_channel.initial_timestamp();
                let track_url = format!("{}{}", content_base, track.media_control);
                rtp_info_parts.push(format!("url={};seq={};rtptime={}", track_url, seq, rtptime));
            }
        }

        if rtp_info_parts.is_empty() {
            None
        } else {
            Some(rtp_info_parts.join(", "))
        }
    }

    async fn apply_range_header(
        &mut self,
        rtsp_request: &RtspRequest,
        response: &mut RtspResponse,
    ) -> Result<bool, SessionError> {
        if let Some(range_str) = rtsp_request.get_header("Range") {
            match RtspRange::unmarshal(range_str) {
                Ok(range) => {
                    response
                        .headers
                        .insert(String::from("Range"), range.marshal());
                    Ok(true)
                }
                Err(err) => {
                    // RFC 2326 §11.3.7 — invalid Range returns 457
                    warn!(error = %err, "invalid_range_header");
                    let err_response = Self::gen_rtsp_response(457, "Invalid Range", rtsp_request);
                    self.send_response(&err_response).await?;
                    Ok(false)
                }
            }
        } else {
            Ok(true)
        }
    }

    fn normalize_rtsp_stream_path(&self, request_path: &str) -> String {
        let trimmed = request_path.trim_matches('/');
        if let Some(last_slash) = trimmed.rfind('/') {
            let (base, last) = trimmed.split_at(last_slash);
            let last = &last[1..];
            let lower = last.to_ascii_lowercase();
            let is_track_segment = lower.starts_with("track")
                || lower.starts_with("streamid")
                || lower.contains("trackid");
            if is_track_segment && !base.is_empty() {
                return base.to_string();
            }
        }

        trimmed.to_string()
    }

    /// Scale timestamp from milliseconds to RTP clock rate units.
    ///
    /// Used for video timestamps only. Audio timestamps are already in sample units.
    ///
    /// # Arguments
    ///
    /// * `timestamp_ms` - Timestamp in milliseconds
    /// * `clock_rate` - Target RTP clock rate (typically 90000 for video)
    ///
    /// # Returns
    ///
    /// Timestamp scaled to clock_rate units
    fn scale_rtp_timestamp(timestamp_ms: u32, clock_rate: u32) -> u32 {
        if clock_rate == 0 {
            return timestamp_ms;
        }
        ((timestamp_ms as u64).saturating_mul(clock_rate as u64) / 1000) as u32
    }

    fn setup_tcp_play_packet_handler(
        channel_identifier: u8,
        counters: Arc<RtpTrackCounters>,
        stream_identifier: Option<StreamIdentifier>,
        track_label: String,
        session_id: String,
        remote_for_rtp: SocketAddr,
        sample_interval: u32,
    ) -> OnRtpPacketFn {
        // Frame buffer: accumulate interleaved RTP packets (200KB for large I-frames)
        let frame_buffer: Arc<Mutex<BytesMut>> =
            Arc::new(Mutex::new(BytesMut::with_capacity(200 * 1024)));

        Box::new(
            move |io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>, packet: RtpPacket| {
                let counters = counters.clone();
                let stream_identifier = stream_identifier.clone();
                let track_label = track_label.clone();
                let session_id = session_id.clone();
                let frame_buffer = frame_buffer.clone();
                Box::pin(async move {
                    let msg = packet.marshal()?;
                    let payload_len = msg.len();
                    let stream_path = stream_identifier
                        .as_ref()
                        .map(|id| id.to_string())
                        .unwrap_or_else(|| "unknown".to_string());
                    let stats = counters.on_packet_sent(
                        payload_len,
                        packet.header.seq_number,
                        packet.header.timestamp,
                    );

                    if stats.seq_gap || stats.seq_regressed || stats.timestamp_regressed {
                        warn!(
                            protocol = "TCP",
                            track = %track_label,
                            session_id = %session_id,
                            remote_addr = %remote_for_rtp,
                            stream_path = %stream_path,
                            prev_seq = ?stats.prev_seq,
                            seq = packet.header.seq_number,
                            seq_delta = ?stats.seq_delta,
                            prev_timestamp = ?stats.prev_timestamp,
                            timestamp = packet.header.timestamp,
                            timestamp_delta = ?stats.timestamp_delta,
                            seq_gap = stats.seq_gap,
                            seq_regressed = stats.seq_regressed,
                            timestamp_regressed = stats.timestamp_regressed,
                            "rtp_packet_anomaly"
                        );
                    }

                    if sample_interval > 0
                        && stats.packets_sent.is_multiple_of(sample_interval as u64)
                    {
                        debug!(
                            protocol = "TCP",
                            track = %track_label,
                            session_id = %session_id,
                            remote_addr = %remote_for_rtp,
                            stream_path = %stream_path,
                            seq = packet.header.seq_number,
                            timestamp = packet.header.timestamp,
                            marker = packet.header.marker,
                            size_bytes = payload_len,
                            packets_sent = stats.packets_sent,
                            bytes_sent = stats.bytes_sent,
                            "rtp_packet_sample"
                        );
                    }

                    // Build interleaved RTP packet: 0x24 + channel + length + payload
                    let mut buffer = frame_buffer.lock().await;
                    buffer.reserve(4 + msg.len());
                    buffer.put_u8(0x24);
                    buffer.put_u8(channel_identifier);
                    let len_bytes = (msg.len() as u16).to_be_bytes();
                    buffer.extend_from_slice(&len_bytes);
                    buffer.extend_from_slice(&msg);

                    // Flush only when marker bit is set (end of frame)
                    if packet.header.marker == 1 {
                        let start = std::time::Instant::now();
                        let data = buffer.split().freeze();
                        drop(buffer);
                        io.lock().await.write(data).await?;
                        let elapsed = start.elapsed();

                        // Log slow writes (>10ms threshold)
                        if elapsed.as_millis() >= 10 {
                            tracing::warn!(
                                protocol = "TCP",
                                track = %track_label,
                                session_id = %session_id,
                                remote_addr = %remote_for_rtp,
                                elapsed_ms = elapsed.as_millis(),
                                "slow_tcp_write"
                            );
                        }
                    }

                    Ok(())
                })
            },
        )
    }

    fn setup_udp_play_packet_handler(
        counters: Arc<RtpTrackCounters>,
        stream_identifier: Option<StreamIdentifier>,
        track_label: String,
        session_id: String,
        remote_for_rtp: SocketAddr,
        sample_interval: u32,
    ) -> OnRtpPacketFn {
        // Packet buffer: accumulate marshalled packets (150 packets for large I-frames)
        let packet_buffer: Arc<Mutex<Vec<BytesMut>>> =
            Arc::new(Mutex::new(Vec::with_capacity(150)));

        Box::new(
            move |io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>, packet: RtpPacket| {
                let counters = counters.clone();
                let stream_identifier = stream_identifier.clone();
                let track_label = track_label.clone();
                let session_id = session_id.clone();
                let packet_buffer = packet_buffer.clone();
                Box::pin(async move {
                    let msg = packet.marshal()?;
                    let payload_len = msg.len();
                    let stream_path = stream_identifier
                        .as_ref()
                        .map(|id| id.to_string())
                        .unwrap_or_else(|| "unknown".to_string());
                    let stats = counters.on_packet_sent(
                        payload_len,
                        packet.header.seq_number,
                        packet.header.timestamp,
                    );

                    if stats.seq_gap || stats.seq_regressed || stats.timestamp_regressed {
                        warn!(
                            protocol = "UDP",
                            track = %track_label,
                            session_id = %session_id,
                            remote_addr = %remote_for_rtp,
                            stream_path = %stream_path,
                            prev_seq = ?stats.prev_seq,
                            seq = packet.header.seq_number,
                            seq_delta = ?stats.seq_delta,
                            prev_timestamp = ?stats.prev_timestamp,
                            timestamp = packet.header.timestamp,
                            timestamp_delta = ?stats.timestamp_delta,
                            seq_gap = stats.seq_gap,
                            seq_regressed = stats.seq_regressed,
                            timestamp_regressed = stats.timestamp_regressed,
                            "rtp_packet_anomaly"
                        );
                    }

                    if sample_interval > 0
                        && stats.packets_sent.is_multiple_of(sample_interval as u64)
                    {
                        debug!(
                            protocol = "UDP",
                            track = %track_label,
                            session_id = %session_id,
                            remote_addr = %remote_for_rtp,
                            stream_path = %stream_path,
                            seq = packet.header.seq_number,
                            timestamp = packet.header.timestamp,
                            marker = packet.header.marker,
                            size_bytes = payload_len,
                            packets_sent = stats.packets_sent,
                            bytes_sent = stats.bytes_sent,
                            "rtp_packet_sample"
                        );
                    }

                    // Accumulate packet into buffer
                    let mut buffer = packet_buffer.lock().await;
                    buffer.push(msg);

                    // Write all accumulated packets when marker bit is set (end of frame)
                    if packet.header.marker == 1 {
                        let start = std::time::Instant::now();
                        let packets: Vec<BytesMut> = buffer.drain(..).collect();
                        drop(buffer);
                        let mut io = io.lock().await;
                        for pkt in packets {
                            io.write(pkt.into()).await?;
                        }
                        let elapsed = start.elapsed();

                        // Log slow writes (>10ms threshold)
                        if elapsed.as_millis() >= 10 {
                            tracing::warn!(
                                protocol = "UDP",
                                track = %track_label,
                                session_id = %session_id,
                                remote_addr = %remote_for_rtp,
                                elapsed_ms = elapsed.as_millis(),
                                "slow_udp_write"
                            );
                        }
                    }

                    Ok(())
                })
            },
        )
    }

    async fn handle_play(&mut self, rtsp_request: &RtspRequest) -> Result<(), SessionError> {
        if let Some(auth) = &self.auth {
            let stream_name = rtsp_request.uri.path.clone();
            let auth_result = auth.authenticate_request(
                &stream_name,
                &rtsp_request.uri.query,
                rtsp_request
                    .get_header("Authorization")
                    .map(std::string::String::as_str),
                true,
            );
            if auth_result.is_err() {
                self.send_unauthorized_response(rtsp_request).await?;
                return Ok(());
            }
        }

        let cseq = rtsp_request.get_header("CSeq").cloned().unwrap_or_default();
        let session_id = self
            .session_id
            .map(|id| id.to_string())
            .unwrap_or_else(|| "none".to_string());
        info!(
            method = "PLAY",
            cseq = %cseq,
            session_id = %session_id,
            remote_addr = %self.remote_addr,
            stream_path = %rtsp_request.uri.path,
            session_type = ?self.session_type,
            "rtsp_request"
        );

        if let Some(range_str) = rtsp_request.get_header("Range")
            && RtspRange::unmarshal(range_str).is_err()
        {
            warn!(
                session_id = %session_id,
                remote_addr = %self.remote_addr,
                stream_path = %rtsp_request.uri.path,
                range = %range_str,
                "play_rejected_invalid_range"
            );
            let response = Self::gen_rtsp_response(457, "Invalid Range", rtsp_request);
            self.send_response(&response).await?;
            return Ok(());
        }

        let play_gate_timeout_ms = if self.config.play_ready_timeout_ms > 0 {
            self.config.play_ready_timeout_ms
        } else {
            DEFAULT_PLAY_READY_TIMEOUT_MS
        };
        debug!(
            session_id = %session_id,
            remote_addr = %self.remote_addr,
            stream_path = %rtsp_request.uri.path,
            timeout_ms = play_gate_timeout_ms,
            "play_waiting_for_tracks"
        );
        if !self
            .wait_for_tracks(
                rtsp_request,
                Duration::from_millis(play_gate_timeout_ms.max(50)),
            )
            .await?
        {
            warn!(
                session_id = %session_id,
                remote_addr = %self.remote_addr,
                stream_path = %rtsp_request.uri.path,
                timeout_ms = play_gate_timeout_ms,
                "play_rejected_waiting_for_sps_pps"
            );
            let response = Self::gen_response(http::StatusCode::SERVICE_UNAVAILABLE, rtsp_request);
            self.send_response(&response).await?;
            return Ok(());
        }

        let sample_interval = self.rtp_sample_interval;
        let session_id_for_rtp = session_id.clone();
        let remote_for_rtp = self.remote_addr;
        let stream_identifier = self.stream_identifier.clone();

        for (track_type, track) in self.tracks.iter_mut() {
            let protocol_type = track.transport.protocol_type.clone();
            let counters = self
                .rtp_counters
                .entry(track_type.clone())
                .or_insert_with(|| Arc::new(RtpTrackCounters::new()))
                .clone();

            let track_label = format!("{:?}", track_type);
            let session_id_clone = session_id_for_rtp.clone();
            let stream_id_clone = stream_identifier.clone();

            match protocol_type {
                ProtocolType::TCP => {
                    let channel_identifier = track
                        .transport
                        .interleaved
                        .map(|i| i[0])
                        .unwrap_or_else(|| {
                            error!("unexpected_state");
                            0
                        });
                    let handler = Self::setup_tcp_play_packet_handler(
                        channel_identifier,
                        counters,
                        stream_id_clone,
                        track_label,
                        session_id_clone,
                        remote_for_rtp,
                        sample_interval,
                    );
                    track.rtp_channel.lock().await.on_packet_handler(handler);
                }
                ProtocolType::UDP => {
                    let handler = Self::setup_udp_play_packet_handler(
                        counters,
                        stream_id_clone,
                        track_label,
                        session_id_clone,
                        remote_for_rtp,
                        sample_interval,
                    );
                    track.rtp_channel.lock().await.on_packet_handler(handler);
                }
            }
        }

        // RFC 2326 §10.5 — validate session ID if provided
        if let Some(response) = self.validate_session_id(rtsp_request) {
            self.send_response(&response).await?;
            return Ok(());
        }

        let status_code = http::StatusCode::OK;
        let mut response = Self::gen_response(status_code, rtsp_request);

        // RFC 2326 §12.37 — Session header MUST be in PLAY response
        if let Some(session_id) = self.session_id {
            response
                .headers
                .insert("Session".to_string(), session_id.to_string());
        }

        if let Some(rtp_info) = self.build_rtp_info_header(rtsp_request).await {
            response.headers.insert("RTP-Info".to_string(), rtp_info);
        }

        if !self.apply_range_header(rtsp_request, &mut response).await? {
            return Ok(());
        }

        self.send_response(&response).await?;

        let (event_result_sender, event_result_receiver) = oneshot::channel();

        let identifier = self.resolve_stream_identifier(&rtsp_request.uri.path);
        let subscribe_event = StreamHubEvent::Subscribe {
            identifier,
            info: self.get_subscriber_info(),
            result_sender: event_result_sender,
        };

        if self.event_producer.send(subscribe_event).is_err() {
            return Err(SessionError {
                value: SessionErrorValue::StreamHubEventSendErr,
            });
        }

        let receiver = event_result_receiver
            .await??
            .0
            .frame_receiver
            .ok_or(SessionError {
                value: SessionErrorValue::MissingFrameReceiver,
            })?;

        self.has_subscribed = true;
        self.session_type = define::ServerSessionType::Pull;
        let audio_rtp_channel = self
            .tracks
            .get(&TrackType::Audio)
            .map(|track| track.rtp_channel.clone());
        let video_rtp_channel = self
            .tracks
            .get(&TrackType::Video)
            .map(|track| track.rtp_channel.clone());
        let remote_addr = self.remote_addr;
        let request_path = rtsp_request.uri.path.clone();
        let session_id_for_task = session_id.clone();
        let shutdown = self.shutdown.clone();
        let latency_policy = PlaybackLatencyPolicy::from_config(&self.config);

        info!(
            session_id = %session_id,
            remote_addr = %self.remote_addr,
            stream_path = %rtsp_request.uri.path,
            max_frame_age_ms = latency_policy.max_frame_age_ms,
            lag_recovery_mode = ?latency_policy.lag_recovery_mode,
            lag_recovery_threshold_ms = latency_policy.lag_recovery_threshold_ms,
            sustained_lag_frames = latency_policy.sustained_lag_frames,
            "playback_latency_policy"
        );

        self.stop_playback_task().await;
        let playback_cancel = Arc::new(AtomicBool::new(false));
        let playback_cancel_for_task = playback_cancel.clone();
        self.playback_cancel = Some(playback_cancel);

        self.playback_task = Some(tokio::spawn(run_playback_loop(
            receiver,
            audio_rtp_channel,
            video_rtp_channel,
            playback_cancel_for_task,
            shutdown,
            session_id_for_task,
            remote_addr,
            request_path,
            latency_policy,
        )));

        Ok(())
    }

    async fn handle_record(&mut self, rtsp_request: &RtspRequest) -> Result<(), SessionError> {
        let status_code = http::StatusCode::OK;
        let mut response = Self::gen_response(status_code, rtsp_request);

        //A stream published by gstreamer does not support the Range header
        //https://github.com/harlanc/xiu/issues/135
        if let Some(range_str) = rtsp_request.get_header("Range") {
            match RtspRange::unmarshal(range_str) {
                Ok(range) => {
                    response
                        .headers
                        .insert(String::from("Range"), range.marshal());
                }
                Err(err) => {
                    warn!(error = %err, "invalid_range_header_ignored");
                }
            }
        }

        response.headers.insert(
            "Session".to_string(),
            self.session_id
                .ok_or(SessionError {
                    value: SessionErrorValue::MissingSessionId,
                })?
                .to_string(),
        );

        self.send_response(&response).await?;

        Ok(())
    }

    async fn handle_get_parameter(
        &mut self,
        rtsp_request: &RtspRequest,
    ) -> Result<(), SessionError> {
        self.handle_keep_alive(rtsp_request, rtsp_method_name::GET_PARAMETER)
            .await
    }

    async fn handle_set_parameter(
        &mut self,
        rtsp_request: &RtspRequest,
    ) -> Result<(), SessionError> {
        self.handle_keep_alive(rtsp_request, rtsp_method_name::SET_PARAMETER)
            .await
    }

    async fn handle_pause(&mut self, rtsp_request: &RtspRequest) -> Result<(), SessionError> {
        self.stop_playback_task().await;

        if self.has_subscribed {
            let identifier = self.resolve_stream_identifier(&rtsp_request.uri.path);
            let unsubscribe = StreamHubEvent::UnSubscribe {
                identifier,
                info: self.get_subscriber_info(),
            };

            if self.event_producer.send(unsubscribe).is_err() {
                return Err(SessionError {
                    value: SessionErrorValue::StreamHubEventSendErr,
                });
            }

            self.has_subscribed = false;
        }

        let mut response = Self::gen_response(http::StatusCode::OK, rtsp_request);
        if let Some(session_id) = self.session_id {
            response
                .headers
                .insert("Session".to_string(), session_id.to_string());
        }
        self.send_response(&response).await?;

        Ok(())
    }

    async fn handle_redirect(&mut self, rtsp_request: &RtspRequest) -> Result<(), SessionError> {
        let mut response = Self::gen_rtsp_response(405, "Method Not Allowed", rtsp_request);
        response.headers.insert(
            "Allow".to_string(),
            rtsp_method_name::PUBLIC_METHODS.join(", "),
        );
        self.send_response(&response).await?;
        Ok(())
    }

    async fn handle_keep_alive(
        &mut self,
        rtsp_request: &RtspRequest,
        method: &str,
    ) -> Result<(), SessionError> {
        if let Some(session_hdr) = rtsp_request.get_header("Session")
            && let Some(current) = self.session_id
        {
            let requested = Self::parse_session_header(session_hdr);
            if !requested.is_empty() && requested != current.to_string() {
                let response = Self::gen_rtsp_response(454, "Session Not Found", rtsp_request);
                self.send_response(&response).await?;
                return Ok(());
            }
        }

        let mut response = Self::gen_response(http::StatusCode::OK, rtsp_request);
        if let Some(session_id) = self.session_id {
            response
                .headers
                .insert("Session".to_string(), session_id.to_string());
        }
        self.send_response(&response).await?;

        let cseq = rtsp_request.get_header("CSeq").cloned().unwrap_or_default();
        let session_id = self
            .session_id
            .map(|id| id.to_string())
            .unwrap_or_else(|| "none".to_string());
        info!(
            method = %method,
            cseq = %cseq,
            session_id = %session_id,
            remote_addr = %self.remote_addr,
            stream_path = %rtsp_request.uri.path,
            session_type = ?self.session_type,
            "rtsp_request"
        );

        Ok(())
    }

    fn handle_teardown(&mut self, rtsp_request: &RtspRequest) -> Result<(), SessionError> {
        let identifier = self.resolve_stream_identifier(&rtsp_request.uri.path);
        info!(
            session_id = %self.session_id
                .map(|id| id.to_string())
                .unwrap_or_else(|| "none".to_string()),
            remote_addr = %self.remote_addr,
            stream_path = %rtsp_request.uri.path,
            session_type = ?self.session_type,
            "rtsp_teardown"
        );

        // Best-effort per-track RTP summary at session teardown.
        for (track_type, counters) in &self.rtp_counters {
            let (packets, bytes, duration_ms) = counters.snapshot();
            if packets == 0 {
                continue;
            }
            let bitrate_kbps = duration_ms
                .filter(|d| *d > 0)
                .map(|d| ((bytes.saturating_mul(8) as u128 * 1000) / d as u128 / 1000) as u64)
                .unwrap_or(0);
            let stream_path = self
                .stream_identifier
                .as_ref()
                .map(|id| id.to_string())
                .unwrap_or_else(|| "unknown".to_string());
            info!(
                track = ?track_type,
                session_id = %self.session_id
                    .map(|id| id.to_string())
                    .unwrap_or_else(|| "none".to_string()),
                remote_addr = %self.remote_addr,
                stream_path = %stream_path,
                packets = packets,
                bytes = bytes,
                duration_ms = duration_ms.unwrap_or(0),
                bitrate_kbps = bitrate_kbps,
                "rtp_session_summary"
            );
        }
        self.exit(identifier)
    }

    pub fn exit(&mut self, identifier: StreamIdentifier) -> Result<(), SessionError> {
        let event = if self.has_published {
            Some(StreamHubEvent::UnPublish {
                identifier,
                info: self.get_publisher_info(),
            })
        } else if self.has_subscribed {
            Some(StreamHubEvent::UnSubscribe {
                identifier,
                info: self.get_subscriber_info(),
            })
        } else {
            None
        };

        let event = match event {
            Some(event) => event,
            None => {
                self.is_normal_exit = true;
                info!("session_exit_no_pubsub");
                return Ok(());
            }
        };
        let event_json_str =
            serde_json::to_string(&event).unwrap_or_else(|_| "<serialize failed>".to_string());

        let rv = self.event_producer.send(event);
        match rv {
            Err(err) => {
                error!(error = %err, event_json = %event_json_str, "session_exit_send_error");
                Err(SessionError {
                    value: SessionErrorValue::StreamHubEventSendErr,
                })
            }
            Ok(()) => {
                self.is_normal_exit = true;
                info!(event_json = %event_json_str, "session_exit_success");
                Ok(())
            }
        }
    }

    fn new_tracks(&mut self) -> Result<(), SessionError> {
        for media in &self.sdp.medias {
            let media_control = if let Some(media_control_val) = media.attributes.get("control") {
                media_control_val.clone()
            } else {
                String::from("")
            };

            let media_name = &media.media_type;
            info!(media_name = %media_name, "media_track_info");
            match media_name.as_str() {
                "audio" => {
                    let codec_name = media.rtpmap.encoding_name.to_lowercase();
                    let codec_id = rtsp_codec::RTSP_CODEC_NAME_2_ID
                        .get(codec_name.as_str())
                        .cloned()
                        .ok_or(SessionError {
                            value: SessionErrorValue::RtspMessageCorrupted(format!(
                                "unsupported audio codec: {}",
                                codec_name
                            )),
                        })?;
                    let channel_count = if media.rtpmap.encoding_param.is_empty() {
                        1
                    } else {
                        media
                            .rtpmap
                            .encoding_param
                            .parse()
                            .map_err(|_| SessionError {
                                value: SessionErrorValue::RtspMessageCorrupted(
                                    "invalid audio channel count".to_string(),
                                ),
                            })?
                    };
                    let codec_info = RtspCodecInfo {
                        codec_id,
                        payload_type: media.rtpmap.payload_type as u8,
                        sample_rate: media.rtpmap.clock_rate,
                        channel_count,
                    };

                    info!(codec_info = ?codec_info, "audio_codec_info");

                    let track = RtspTrack::new(TrackType::Audio, codec_info, media_control);
                    self.tracks.insert(TrackType::Audio, track);
                    self.rtp_counters
                        .entry(TrackType::Audio)
                        .or_insert_with(|| Arc::new(RtpTrackCounters::new()));
                }
                "video" => {
                    let codec_name = media.rtpmap.encoding_name.to_lowercase();
                    let codec_id = rtsp_codec::RTSP_CODEC_NAME_2_ID
                        .get(codec_name.as_str())
                        .cloned()
                        .ok_or(SessionError {
                            value: SessionErrorValue::RtspMessageCorrupted(format!(
                                "unsupported video codec: {}",
                                codec_name
                            )),
                        })?;
                    let codec_info = RtspCodecInfo {
                        codec_id,
                        payload_type: media.rtpmap.payload_type as u8,
                        sample_rate: media.rtpmap.clock_rate,
                        ..Default::default()
                    };
                    let track = RtspTrack::new(TrackType::Video, codec_info, media_control);
                    self.tracks.insert(TrackType::Video, track);
                    self.rtp_counters
                        .entry(TrackType::Video)
                        .or_insert_with(|| Arc::new(RtpTrackCounters::new()));
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// RFC 2326 §12.18 — Date header in HTTP-date format (RFC 7231 §7.1.1.1).
    fn http_date_now() -> String {
        Utc::now().format("%a, %d %b %Y %H:%M:%S GMT").to_string()
    }

    fn gen_response(status_code: StatusCode, rtsp_request: &RtspRequest) -> RtspResponse {
        let reason_phrase = if let Some(reason) = status_code.canonical_reason() {
            reason.to_string()
        } else {
            String::from("")
        };

        let mut response = RtspResponse {
            version: "RTSP/1.0".to_string(),
            status_code: status_code.as_u16(),
            reason_phrase,
            ..Default::default()
        };

        if let Some(cseq) = rtsp_request.get_header("CSeq") {
            response
                .headers
                .insert("CSeq".to_string(), cseq.to_string());
        }

        // RFC 2326 §12.18 — Date header SHOULD be included
        response
            .headers
            .insert("Date".to_string(), Self::http_date_now());
        // RFC 2326 §12.36 — Server header identifies the implementation
        response
            .headers
            .insert("Server".to_string(), SERVER_HEADER.to_string());

        response
    }

    async fn send_unauthorized_response(
        &mut self,
        rtsp_request: &RtspRequest,
    ) -> Result<(), SessionError> {
        let mut response = Self::gen_response(http::StatusCode::UNAUTHORIZED, rtsp_request);
        if let Some(auth) = &self.auth {
            response
                .headers
                .insert("WWW-Authenticate".to_string(), auth.basic_challenge());
        }
        self.send_response(&response).await
    }

    fn gen_rtsp_response(
        code: u16,
        reason_phrase: &str,
        rtsp_request: &RtspRequest,
    ) -> RtspResponse {
        let mut response = RtspResponse {
            version: "RTSP/1.0".to_string(),
            status_code: code,
            reason_phrase: reason_phrase.to_string(),
            ..Default::default()
        };

        if let Some(cseq) = rtsp_request.get_header("CSeq") {
            response
                .headers
                .insert("CSeq".to_string(), cseq.to_string());
        }

        response
    }

    fn parse_session_header(session_hdr: &str) -> String {
        session_hdr
            .split(';')
            .next()
            .unwrap_or_default()
            .trim()
            .to_string()
    }

    /// Validate that the Session header in the request (if any) matches the current session ID.
    /// Returns `Some(response)` with a 454 "Session Not Found" if there is a mismatch,
    /// or `None` if the session ID is valid or no Session header is present.
    /// Per RFC 2326 §12.37.
    fn validate_session_id(&self, rtsp_request: &RtspRequest) -> Option<RtspResponse> {
        if let Some(session_hdr) = rtsp_request.get_header("Session")
            && let Some(current) = self.session_id
        {
            let requested = Self::parse_session_header(session_hdr);
            if !requested.is_empty() && requested != current.to_string() {
                return Some(Self::gen_rtsp_response(
                    454,
                    "Session Not Found",
                    rtsp_request,
                ));
            }
        }
        None
    }

    fn get_subscriber_info(&mut self) -> SubscriberInfo {
        let id = if let Some(session_id) = &self.session_id {
            *session_id
        } else {
            Uuid::new(SESSION_ID_RANDOM_DIGITS)
        };

        SubscriberInfo {
            id,
            sub_type: SubscribeType::RtspPull,
            sub_data_type: crate::hub::define::SubDataType::Frame,
            notify_info: NotifyInfo {
                request_url: String::from(""),
                remote_addr: String::from(""),
            },
        }
    }

    fn get_publisher_info(&mut self) -> PublisherInfo {
        let id = if let Some(session_id) = &self.session_id {
            *session_id
        } else {
            Uuid::new(SESSION_ID_RANDOM_DIGITS)
        };

        PublisherInfo {
            id,
            pub_type: PublishType::RtspPush,
            pub_data_type: crate::hub::define::PubDataType::Frame,
            notify_info: NotifyInfo {
                request_url: String::from(""),
                remote_addr: String::from(""),
            },
        }
    }

    async fn send_response(&mut self, response: &RtspResponse) -> Result<(), SessionError> {
        self.writer.write(response.marshal().as_bytes())?;
        self.writer.flush().await?;

        Ok(())
    }
}

impl Drop for RtspServerSession {
    fn drop(&mut self) {
        self.abort_playback_task();
    }
}

#[derive(Default)]
pub struct RtspStreamHandler {
    sdp: Mutex<Sdp>,
}

impl RtspStreamHandler {
    pub fn new() -> Self {
        Self {
            sdp: Mutex::new(Sdp::default()),
        }
    }
    pub async fn set_sdp(&self, sdp: Sdp) {
        *self.sdp.lock().await = sdp;
    }

    #[allow(dead_code)]
    async fn send_prior_data_rtsp_remux_rtmp(
        &self,
        sender: &FrameDataSender,
    ) -> Result<(), StreamHubError> {
        let sdp_info = self.sdp.lock().await;
        let mut video_clock_rate: u32 = 0;
        let mut audio_clock_rate: u32 = 0;
        let mut vcodec: VideoCodecType = VideoCodecType::H264;

        for media in &sdp_info.medias {
            Self::send_prior_fmtp_frames(
                sender,
                media,
                &mut video_clock_rate,
                &mut audio_clock_rate,
                &mut vcodec,
            )?;
        }

        if let Err(err) = sender.send(FrameData::MediaInfo {
            media_info: MediaInfo {
                audio_clock_rate,
                video_clock_rate,
                vcodec,
            },
        }) {
            error!(error = %err, "send_media_info_error");
        }
        Ok(())
    }

    #[allow(dead_code)]
    fn send_prior_fmtp_frames(
        sender: &FrameDataSender,
        media: &crate::protocol::rtsp::sdp::SdpMediaInfo,
        video_clock_rate: &mut u32,
        audio_clock_rate: &mut u32,
        vcodec: &mut VideoCodecType,
    ) -> Result<(), StreamHubError> {
        let Some(fmtp) = &media.fmtp else {
            return Ok(());
        };
        match fmtp {
            Fmtp::H264(data) => {
                let mut bytes_writer = BytesWriter::new();
                bytes_writer.write(&ANNEXB_NALU_START_CODE)?;
                bytes_writer.write(&data.sps)?;
                bytes_writer.write(&ANNEXB_NALU_START_CODE)?;
                bytes_writer.write(&data.pps)?;
                let frame_data = FrameData::Video {
                    timestamp: 0,
                    data: bytes_writer.extract_current_bytes(),
                };
                if let Err(err) = sender.send(frame_data) {
                    error!(error = %err, "send_sps_pps_error");
                }
                *video_clock_rate = media.rtpmap.clock_rate;
            }
            Fmtp::H265(data) => {
                let mut bytes_writer = BytesWriter::new();
                bytes_writer.write(&ANNEXB_NALU_START_CODE)?;
                bytes_writer.write(&data.sps)?;
                bytes_writer.write(&ANNEXB_NALU_START_CODE)?;
                bytes_writer.write(&data.pps)?;
                bytes_writer.write(&ANNEXB_NALU_START_CODE)?;
                bytes_writer.write(&data.vps)?;
                let frame_data = FrameData::Video {
                    timestamp: 0,
                    data: bytes_writer.extract_current_bytes(),
                };
                if let Err(err) = sender.send(frame_data) {
                    error!(error = %err, "send_sps_pps_vps_error");
                }
                *vcodec = VideoCodecType::H265;
                *video_clock_rate = media.rtpmap.clock_rate;
            }
            Fmtp::Mpeg4(data) => {
                let frame_data = FrameData::Audio {
                    timestamp: 0,
                    data: data.asc.clone(),
                };
                if let Err(err) = sender.send(frame_data) {
                    error!(error = %err, "send_asc_error");
                }
                *audio_clock_rate = media.rtpmap.clock_rate;
            }
        }
        Ok(())
    }
}

#[async_trait]
impl TStreamHandler for RtspStreamHandler {
    async fn send_prior_data(
        &self,
        data_sender: DataSender,
        sub_type: SubscribeType,
    ) -> Result<(), StreamHubError> {
        let _sender = match data_sender {
            DataSender::Frame { sender } => sender,
            DataSender::Packet { sender: _ } => {
                return Err(StreamHubError {
                    value: StreamHubErrorValue::NotCorrectDataSenderType,
                });
            }
        };
        // No specific handling needed for remaining SubscribeType variants
        let _ = sub_type;
        Ok(())
    }
    async fn get_statistic_data(&self) -> Option<StatisticsStream> {
        None
    }

    async fn send_information(&self, sender: InformationSender) {
        if let Err(err) = sender.send(Information::Sdp {
            data: self.sdp.lock().await.marshal(),
        }) {
            error!(error = %err, "send_information_error");
        }
    }
}

#[cfg(test)]
#[path = "server_session_tests.rs"]
mod tests;
