use bytes::BytesMut;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{
    Arc, OnceLock,
    atomic::{AtomicBool, Ordering},
};
use std::time::Instant;
use tokio::sync::Mutex;
use tokio::sync::mpsc;
use tokio::time::Duration;
use tracing::{debug, info, warn};

use crate::config::StreamingConfig;
use crate::hub::define::FrameData;
use crate::protocol::rtsp::rtp::define::ANNEXB_NALU_START_CODE;
use crate::protocol::rtsp::rtsp_channel::RtpChannel;
use crate::protocol::rtsp::rtsp_track::TrackType;
use super::rtp_counters::RTP_TIMESTAMP_WRAP_THRESHOLD;

const DEFAULT_MAX_FRAME_AGE_MS: u32 = 1000;
const LAG_RECOVERY_THRESHOLD_MS: u32 = 500;
const LAG_RECOVERY_SUSTAINED_FRAMES: u32 = 4;
const SOURCE_TIMESTAMP_RESET_THRESHOLD_MS: u32 = 10_000;
const RTP_SEND_SLOW_WARN_MS: u128 = 25;
const PACER_SLEEP_DIAGNOSTIC_MIN_MS: u64 = 20;

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

pub(super) fn monotonic_millis() -> u64 {
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_millis() as u64
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
pub(super) fn scale_rtp_timestamp(timestamp_ms: u32, clock_rate: u32) -> u32 {
    if clock_rate == 0 {
        return timestamp_ms;
    }
    ((timestamp_ms as u64).saturating_mul(clock_rate as u64) / 1000) as u32
}

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
pub(super) struct PlaybackLatencyPolicy {
    pub(super) max_frame_age_ms: u32,
    pub(super) lag_recovery_mode: LagRecoveryMode,
    pub(super) lag_recovery_threshold_ms: u32,
    pub(super) sustained_lag_frames: u32,
}

impl PlaybackLatencyPolicy {
    /// Create a PlaybackLatencyPolicy from a StreamingConfig.
    ///
    /// Uses `config.max_frame_age_ms` with the same `> 0` guard,
    /// falling back to `DEFAULT_MAX_FRAME_AGE_MS` if the value is 0.
    /// Uses `config.lag_recovery_mode` directly.
    /// Keeps `lag_recovery_threshold_ms` and `sustained_lag_frames` as constants.
    pub(super) fn from_config(config: &StreamingConfig) -> Self {
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

    /// Peek at current lag without consuming a frame timestamp.
    ///
    /// Returns the same value that `lag_ms()` would report for a frame
    /// carrying `last_source_ts`, based on how much wall-clock time has
    /// elapsed since the anchor was set.
    fn current_lag_ms(&self) -> u32 {
        if !self.initialized {
            return 0;
        }
        let elapsed_ms = self.anchor_local.elapsed().as_millis() as u32;
        let expected = self.anchor_source_ts.wrapping_add(elapsed_ms);
        expected.saturating_sub(self.last_source_ts)
    }
}

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
    /// `current_lag_ms` is the current lag reported by the LagTracker.
    /// When the system is already behind real-time (`current_lag_ms > 0`),
    /// no sleep is introduced — the natural RTP send time provides
    /// sufficient pacing without explicit delay.
    /// On the first frame, no delay is introduced.
    async fn pace(&mut self, timestamp_ms: u32, current_lag_ms: u32) -> u64 {
        // If already behind real-time, don't add more delay.
        if current_lag_ms > 0 {
            self.last_send = Some(Instant::now());
            self.last_timestamp_ms = Some(timestamp_ms);
            return 0;
        }

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

    fn reset(&mut self) {
        self.last_send = None;
        self.last_timestamp_ms = None;
    }
}

#[cfg(test)]
#[inline]
pub(super) fn pacing_timestamp_ms(frame_data: &FrameData) -> Option<u32> {
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
            TrackType::Video => scale_rtp_timestamp(source_timestamp, clock_rate),
            TrackType::Application => scale_rtp_timestamp(source_timestamp, clock_rate),
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
    frame_pacer: &mut FramePacer,
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
    let was_waiting_for_idr_recovery = *waiting_for_idr_recovery;
    if *waiting_for_idr_recovery {
        if !contains_idr {
            *dropped_for_recovery += 1;
            return;
        }

        *waiting_for_idr_recovery = false;
        // Reset pacing baseline when resyncing on IDR so stale timeline
        // state does not induce additional sleep on the recovered stream.
        frame_pacer.reset();
        video_lag_tracker.anchor_local = Instant::now();
        video_lag_tracker.anchor_source_ts = flush_ts;
        video_lag_tracker.last_source_ts = flush_ts;
        video_lag_tracker.initialized = true;
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

    // Pace only when stream is healthy. Under lag/backpressure, extra sleep
    // worsens delay accumulation and slows recovery.
    if !was_waiting_for_idr_recovery && lag_ms <= latency_policy.lag_recovery_threshold_ms {
        let current_lag = video_lag_tracker.current_lag_ms();
        let paced_sleep_ms = frame_pacer.pace(flush_ts, current_lag).await;
        if paced_sleep_ms >= PACER_SLEEP_DIAGNOSTIC_MIN_MS {
            debug!(
                session_id = %session_id,
                remote_addr = %remote_addr,
                request_path = %request_path,
                source_ts = flush_ts,
                sleep_ms = paced_sleep_ms,
                lag_ms = lag_ms,
                diag_monotonic_ms = monotonic_millis(),
                "playback_pacer_sleep"
            );
        }
    } else if crate::stream_frame_debug_logging_enabled() {
        let skip_reason = if was_waiting_for_idr_recovery {
            "recovery"
        } else {
            "lagging"
        };
        debug!(
            session_id = %session_id,
            remote_addr = %remote_addr,
            request_path = %request_path,
            source_ts = flush_ts,
            lag_ms = lag_ms,
            threshold_ms = latency_policy.lag_recovery_threshold_ms,
            reason = skip_reason,
            diag_monotonic_ms = monotonic_millis(),
            "playback_pacer_skip"
        );
    }

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

    // Give headroom for I-frame send cost on slow ARM hardware.
    // Without headroom, lag from the next I-frame send (~50ms) plus pacing
    // immediately exceeds the threshold again, creating a recovery loop.
    // Half of max_frame_age (500ms at default 1000ms) absorbs the send cost
    // while still maintaining pressure to catch up.
    let headroom_ms = max_frame_age_ms / 2;
    video_lag_tracker.anchor_local =
        Instant::now() - Duration::from_millis(headroom_ms as u64);
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
    frame_pacer: &mut FramePacer,
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
                    frame_pacer,
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
pub(super) async fn run_playback_loop(
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
                    &mut frame_pacer,
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

#[cfg(test)]
#[path = "playback_tests.rs"]
mod tests;
