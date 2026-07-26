//! Video encoder implementation for Anyka platform.
//!
//! Manages dual video encoders (main + sub) with zero-copy frame delivery,
//! panic-isolated callbacks, and dynamic bitrate reconfiguration.

use std::collections::HashMap;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use async_trait::async_trait;
#[cfg(test)]
use bytes::BytesMut;
use parking_lot::RwLock;
use portable_atomic::AtomicU64;

use crate::hal::anyka::ipc::AnykaIpc;
use crate::hal::common::VideoFrameType;
use crate::hal::common::video::{
    VideoEncoderHandle, VideoStreamHandle, video_encoder_open, video_encoder_request_idr,
    video_encoder_set_rc,
};
use crate::hal::common::{
    bitrate_ctrl_mode, encode_group_type, encode_output_type, encode_param, encode_use_chn,
    profile_mode,
};
use crate::streaming::bridge::BytesMutPool;

use crate::platform::common::{
    CallbackId, FrameType, OwnedFrame, OwnedFrameCallback, PlatformError, PlatformResult,
    Resolution, StreamId, VideoEncoder, VideoEncoderConfig, VideoEncoderOptions, VideoEncoding,
};

use super::context::stream_stabilization_ms;

#[cfg(test)]
use super::context::env_var_u64;
#[cfg(test)]
use super::imaging::{
    LAST_IMAGING_UPDATE_SEQ, LAST_IMAGING_UPDATE_UNIX_MS,
    current_unix_ms as imaging_current_unix_ms,
};
#[cfg(test)]
use crate::hal::common::video_stream;

/// Encoder lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum EncoderState {
    /// Encoder created but not yet initialized.
    Uninitialized,
    /// Encoder initialized and ready to produce frames.
    Initialized,
}

/// Convert an SDK `VideoFrameType` to our `FrameType`.
pub(super) fn sdk_frame_type_to_frame_type(ft: VideoFrameType) -> FrameType {
    match ft {
        VideoFrameType::FrameTypeI => FrameType::VideoIFrame,
        VideoFrameType::FrameTypePi => FrameType::VideoPiFrame,
        VideoFrameType::FrameTypeP => FrameType::VideoPFrame,
        VideoFrameType::FrameTypeB => FrameType::VideoBFrame,
    }
}

pub(super) const SDK_ERROR_NO_DATA: i32 = 23;
#[cfg(test)]
pub(super) const NO_DATA_IDR_RECOVERY_EVERY_ERRORS: u32 = 3;
#[cfg(not(test))]
pub(super) const NO_DATA_IDR_RECOVERY_EVERY_ERRORS: u32 = 100;
const PIPELINE_READINESS_POLL_MS: u64 = 25;
const CALLBACK_HISTOGRAM_LOG_INTERVAL: u64 = 1000;
const CALLBACK_BUCKET_LIMITS_US: [u64; 6] = [250, 500, 1000, 2000, 5000, u64::MAX];
const CALLBACK_SLOW_WARN_THRESHOLD_US: u64 = 5000;
const CALLBACK_SLOW_LOG_INTERVAL: u64 = 50;

static CALLBACK_DURATION_TOTAL: AtomicU64 = AtomicU64::new(0);
static CALLBACK_DURATION_MAX_US: AtomicU64 = AtomicU64::new(0);
static CALLBACK_DURATION_BUCKET_0: AtomicU64 = AtomicU64::new(0);
static CALLBACK_DURATION_BUCKET_1: AtomicU64 = AtomicU64::new(0);
static CALLBACK_DURATION_BUCKET_2: AtomicU64 = AtomicU64::new(0);
static CALLBACK_DURATION_BUCKET_3: AtomicU64 = AtomicU64::new(0);
static CALLBACK_DURATION_BUCKET_4: AtomicU64 = AtomicU64::new(0);
static CALLBACK_DURATION_BUCKET_5: AtomicU64 = AtomicU64::new(0);
static CALLBACK_SLOW_TOTAL: AtomicU64 = AtomicU64::new(0);

fn callback_bucket_counter(index: usize) -> &'static AtomicU64 {
    match index {
        0 => &CALLBACK_DURATION_BUCKET_0,
        1 => &CALLBACK_DURATION_BUCKET_1,
        2 => &CALLBACK_DURATION_BUCKET_2,
        3 => &CALLBACK_DURATION_BUCKET_3,
        4 => &CALLBACK_DURATION_BUCKET_4,
        _ => &CALLBACK_DURATION_BUCKET_5,
    }
}

fn record_callback_duration(elapsed_us: u64) {
    CALLBACK_DURATION_TOTAL.fetch_add(1, Ordering::Relaxed);
    CALLBACK_DURATION_MAX_US.fetch_max(elapsed_us, Ordering::Relaxed);

    for (index, limit) in CALLBACK_BUCKET_LIMITS_US.iter().enumerate() {
        if elapsed_us <= *limit {
            callback_bucket_counter(index).fetch_add(1, Ordering::Relaxed);
            break;
        }
    }
}

fn histogram_percentile_bucket_us(percentile: f64) -> u64 {
    let total = CALLBACK_DURATION_TOTAL.load(Ordering::Relaxed);
    if total == 0 {
        return 0;
    }

    let threshold = (total as f64 * percentile).ceil() as u64;
    let mut cumulative = 0u64;
    for (index, limit) in CALLBACK_BUCKET_LIMITS_US.iter().enumerate() {
        cumulative += callback_bucket_counter(index).load(Ordering::Relaxed);
        if cumulative >= threshold {
            return *limit;
        }
    }

    CALLBACK_BUCKET_LIMITS_US[CALLBACK_BUCKET_LIMITS_US.len() - 1]
}

fn maybe_log_callback_histogram() {
    let total = CALLBACK_DURATION_TOTAL.load(Ordering::Relaxed);
    if total == 0 || !total.is_multiple_of(CALLBACK_HISTOGRAM_LOG_INTERVAL) {
        return;
    }

    tracing::debug!(
        callback_samples = total,
        callback_slow_over_5ms = CALLBACK_SLOW_TOTAL.load(Ordering::Relaxed),
        callback_p50_us = histogram_percentile_bucket_us(0.50),
        callback_p95_us = histogram_percentile_bucket_us(0.95),
        callback_p99_us = histogram_percentile_bucket_us(0.99),
        callback_max_us = CALLBACK_DURATION_MAX_US.load(Ordering::Relaxed),
        callback_bucket_le_250us = CALLBACK_DURATION_BUCKET_0.load(Ordering::Relaxed),
        callback_bucket_le_500us = CALLBACK_DURATION_BUCKET_1.load(Ordering::Relaxed),
        callback_bucket_le_1ms = CALLBACK_DURATION_BUCKET_2.load(Ordering::Relaxed),
        callback_bucket_le_2ms = CALLBACK_DURATION_BUCKET_3.load(Ordering::Relaxed),
        callback_bucket_le_5ms = CALLBACK_DURATION_BUCKET_4.load(Ordering::Relaxed),
        callback_bucket_gt_5ms = CALLBACK_DURATION_BUCKET_5.load(Ordering::Relaxed),
        "Frame callback duration histogram"
    );
}

fn maybe_log_slow_callback(callback_id: u64, elapsed_us: u64, callback_kind: &'static str) {
    let slow_total = CALLBACK_SLOW_TOTAL.fetch_add(1, Ordering::Relaxed) + 1;
    if slow_total == 1 || slow_total.is_multiple_of(CALLBACK_SLOW_LOG_INTERVAL) {
        tracing::warn!(
            callback_id,
            elapsed_us,
            slow_count = slow_total,
            threshold_us = CALLBACK_SLOW_WARN_THRESHOLD_US,
            callback_kind,
            "Frame callback exceeded latency threshold"
        );
    }
}

fn maybe_log_slow_owned_callback(callback_id: u64, elapsed_us: u64) {
    maybe_log_slow_callback(callback_id, elapsed_us, "owned");
}

#[cfg(test)]
fn compute_no_data_recovery_interval_errors(trigger_ms: u64, cycle_sleep_ms: u64) -> u32 {
    let trigger_ms = trigger_ms.max(1);
    let cycle_sleep_ms = cycle_sleep_ms.max(1);
    let errors = trigger_ms.div_ceil(cycle_sleep_ms);
    errors.max(1).min(u64::from(u32::MAX)) as u32
}

#[inline]
pub(super) fn is_push_mode_transient_error(error: &PlatformError) -> bool {
    matches!(
        error,
        PlatformError::Timeout | PlatformError::ResourceBusy(_)
    )
}

#[derive(Default)]
pub(super) struct StreamHealthCounters {
    pub(super) main_frames: AtomicU64,
    pub(super) sub_frames: AtomicU64,
    pub(super) main_no_data_errors: AtomicU64,
    pub(super) sub_no_data_errors: AtomicU64,
    /// Monotonic millis of the most recent frame (0 = never).
    pub(super) last_frame_ms: AtomicU64,
}

#[derive(Debug, Clone, Copy)]
struct StreamHealthSnapshot {
    main_frames: u64,
    sub_frames: u64,
    main_no_data_errors: u64,
    sub_no_data_errors: u64,
}

impl StreamHealthCounters {
    fn now_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }

    fn reset(&self) {
        self.main_frames.store(0, Ordering::SeqCst);
        self.sub_frames.store(0, Ordering::SeqCst);
        self.main_no_data_errors.store(0, Ordering::SeqCst);
        self.sub_no_data_errors.store(0, Ordering::SeqCst);
        self.last_frame_ms.store(0, Ordering::SeqCst);
    }

    fn record_frame(&self, stream_id: StreamId) {
        match stream_id {
            StreamId::VideoMain => {
                self.main_frames.fetch_add(1, Ordering::SeqCst);
                self.last_frame_ms.store(Self::now_ms(), Ordering::SeqCst);
            }
            StreamId::VideoSub => {
                self.sub_frames.fetch_add(1, Ordering::SeqCst);
                self.last_frame_ms.store(Self::now_ms(), Ordering::SeqCst);
            }
            StreamId::Audio => {}
        }
    }

    /// Age of the most recent frame in milliseconds, if any frame was ever seen.
    pub(super) fn last_frame_age_ms(&self) -> Option<u64> {
        let last = self.last_frame_ms.load(Ordering::SeqCst);
        if last == 0 {
            return None;
        }
        Some(Self::now_ms().saturating_sub(last))
    }

    fn record_no_data_error(&self, stream_id: StreamId) {
        match stream_id {
            StreamId::VideoMain => {
                self.main_no_data_errors.fetch_add(1, Ordering::SeqCst);
            }
            StreamId::VideoSub => {
                self.sub_no_data_errors.fetch_add(1, Ordering::SeqCst);
            }
            StreamId::Audio => {}
        }
    }

    fn snapshot(&self) -> StreamHealthSnapshot {
        StreamHealthSnapshot {
            main_frames: self.main_frames.load(Ordering::SeqCst),
            sub_frames: self.sub_frames.load(Ordering::SeqCst),
            main_no_data_errors: self.main_no_data_errors.load(Ordering::SeqCst),
            sub_no_data_errors: self.sub_no_data_errors.load(Ordering::SeqCst),
        }
    }
}

/// Unified frame reader loop for video callbacks.
///
/// Production mode is push-only: the loop blocks on `AnykaIpc::recv_pushed_frame()`,
/// routes frames by stream id, and invokes callbacks.
///
/// In unit tests (when `AnykaIpc` is unavailable), a test-only fallback polls
/// `venc_get_stream()` to preserve existing mock-based coverage.
///
/// # Unified reader thread
///
/// This function drains frames from **both** main and sub streams in a single
/// thread, alternating between them each cycle.  This eliminates the IPC mutex
/// contention that occurred when two independent threads (`venc-main-read` and
/// `venc-sub-read`) competed for the same `Mutex<UnixStream>` to the vendor
/// daemon.  The vendor daemon is single-threaded, so serialising requests from
/// one thread matches its dispatch model perfectly.
///
/// Each stream has independent per-stream counters (frame count, no-data
/// streak, adaptive sleep state) so IDR recovery and health tracking remain
/// per-channel.
#[allow(clippy::too_many_arguments)]
pub(super) fn unified_frame_read_loop(
    main_stream_handle: Arc<VideoStreamHandle>,
    sub_stream_handle: Option<Arc<VideoStreamHandle>>,
    _ffi: Arc<dyn crate::hal::common::video::VideoHalTrait>,
    owned_callbacks: Arc<RwLock<HashMap<CallbackId, Arc<dyn OwnedFrameCallback>>>>,
    stop_signal: Arc<AtomicBool>,
    stream_health: Arc<StreamHealthCounters>,
    main_enc_addr: Option<usize>,
    sub_enc_addr: Option<usize>,
    anyka_ipc: Option<Arc<AnykaIpc>>,
    frame_pool: Option<Arc<BytesMutPool>>,
) {
    #[cfg(test)]
    use crate::hal::common::AK_SUCCESS_I32;

    /// Per-stream state for the unified reader loop.
    struct StreamState {
        stream_id: StreamId,
        consecutive_no_data: u32,
        frame_count: u64,
        total_bytes: u64,
        iframe_count: u64,
        error_count: u64,
        last_error_was_no_data: bool,
        recovery_encoder_handle_addr: Option<usize>,
        last_imaging_seq_frame_logged: u64,
        last_imaging_seq_iframe_logged: u64,
    }

    impl StreamState {
        fn new(stream_id: StreamId, recovery_encoder_handle_addr: Option<usize>) -> Self {
            Self {
                stream_id,
                consecutive_no_data: 0,
                frame_count: 0,
                total_bytes: 0,
                iframe_count: 0,
                error_count: 0,
                last_error_was_no_data: true,
                recovery_encoder_handle_addr,
                last_imaging_seq_frame_logged: 0,
                last_imaging_seq_iframe_logged: 0,
            }
        }
    }

    /// Drain all available frames from a single stream handle.
    ///
    /// Returns the number of frames drained this cycle.
    #[cfg(test)]
    #[allow(clippy::too_many_arguments)]
    fn drain_stream(
        handle: &VideoStreamHandle,
        ffi: &dyn crate::hal::common::video::VideoHalTrait,
        state: &mut StreamState,
        callbacks: &RwLock<HashMap<CallbackId, Arc<dyn OwnedFrameCallback>>>,
        stop_signal: &AtomicBool,
        stream_health: &StreamHealthCounters,
        no_data_recovery_interval_errors: u32,
    ) -> u32 {
        let mut frames_this_cycle: u32 = 0;

        loop {
            if stop_signal.load(Ordering::SeqCst) {
                break;
            }

            // ── Legacy path (no AnykaIpc available, e.g. in tests) ──
            let mut stream = std::mem::MaybeUninit::<video_stream>::uninit();
            let stream_ptr = stream.as_mut_ptr();
            let ret = ffi.venc_get_stream(handle.as_ptr(), stream_ptr);

            if ret != AK_SUCCESS_I32 {
                // Optimistic fast-path: skip IPC get_error_no() call when we
                // expect no-data (the common case). Probe on first call to
                // establish baseline, then periodically to detect changes.
                let probe_interval = 50u32;
                let should_probe = state.consecutive_no_data == 0
                    || !state.last_error_was_no_data
                    || state
                        .consecutive_no_data
                        .wrapping_add(1)
                        .is_multiple_of(probe_interval);

                let is_no_data = if should_probe {
                    let sdk_errno = ffi.get_error_no();
                    sdk_errno == SDK_ERROR_NO_DATA
                } else {
                    true // Assume no-data (optimistic)
                };

                if is_no_data {
                    state.consecutive_no_data += 1;
                    state.last_error_was_no_data = true;
                    stream_health.record_no_data_error(state.stream_id);

                    if state
                        .consecutive_no_data
                        .is_multiple_of(no_data_recovery_interval_errors)
                    {
                        if state.frame_count > 0 {
                            if let Some(handle_addr) = state.recovery_encoder_handle_addr {
                                let idr_ret = ffi.venc_set_iframe_by_addr(handle_addr);
                                if idr_ret == AK_SUCCESS_I32 {
                                    tracing::warn!(
                                        stream = ?state.stream_id,
                                        consecutive_no_data = state.consecutive_no_data,
                                        recovery_interval = no_data_recovery_interval_errors,
                                        "Sustained no-data detected; requested IDR recovery frame"
                                    );
                                } else {
                                    tracing::warn!(
                                        stream = ?state.stream_id,
                                        consecutive_no_data = state.consecutive_no_data,
                                        recovery_interval = no_data_recovery_interval_errors,
                                        idr_error_code = idr_ret,
                                        "Sustained no-data detected; IDR recovery request failed"
                                    );
                                }
                            }
                        } else if state.recovery_encoder_handle_addr.is_some() {
                            tracing::debug!(
                                stream = ?state.stream_id,
                                consecutive_no_data = state.consecutive_no_data,
                                recovery_interval = no_data_recovery_interval_errors,
                                "Skipping no-data IDR recovery before first frame"
                            );
                        }
                    }
                } else {
                    state.last_error_was_no_data = false;
                    state.error_count += 1;
                    // Log non-no-data errors on first occurrence and every 50th
                    if state.error_count == 1 || state.error_count.is_multiple_of(50) {
                        let sdk_errstr = ffi.get_error_str();
                        tracing::warn!(
                            stream = ?state.stream_id,
                            error_code = ret,
                            "venc_get_stream failed (non-no-data error): {}",
                            sdk_errstr
                        );
                    }
                }
                break; // Exit inner drain loop
            }

            // Reset no-data counter on successful frame retrieval
            state.consecutive_no_data = 0;
            state.last_error_was_no_data = true;

            // SAFETY: venc_get_stream succeeded, so `stream` is fully initialized.
            let stream_data = unsafe { stream.assume_init_mut() };

            if !stream_data.data.is_null() && stream_data.len > 0 {
                let frame_type = sdk_frame_type_to_frame_type(stream_data.frame_type);
                let frame_size = stream_data.len as usize;

                tracing::trace!(
                    stream = ?state.stream_id,
                    size = frame_size,
                    timestamp_ms = stream_data.ts,
                    frame_type = ?frame_type,
                    "Frame retrieved from SDK"
                );

                state.frame_count += 1;
                frames_this_cycle += 1;
                stream_health.record_frame(state.stream_id);
                state.total_bytes += frame_size as u64;
                if matches!(frame_type, FrameType::VideoIFrame) {
                    state.iframe_count += 1;
                }

                let latest_imaging_seq = LAST_IMAGING_UPDATE_SEQ.load(Ordering::Relaxed);
                if latest_imaging_seq > state.last_imaging_seq_frame_logged {
                    let applied_ms = LAST_IMAGING_UPDATE_UNIX_MS.load(Ordering::Relaxed);
                    let latency_ms = imaging_current_unix_ms().saturating_sub(applied_ms);
                    tracing::info!(
                        stream = ?state.stream_id,
                        imaging_seq = latest_imaging_seq,
                        latency_ms,
                        frame_type = ?frame_type,
                        "First encoded frame observed after imaging update"
                    );
                    state.last_imaging_seq_frame_logged = latest_imaging_seq;
                }

                if matches!(frame_type, FrameType::VideoIFrame)
                    && latest_imaging_seq > state.last_imaging_seq_iframe_logged
                {
                    let applied_ms = LAST_IMAGING_UPDATE_UNIX_MS.load(Ordering::Relaxed);
                    let latency_ms = imaging_current_unix_ms().saturating_sub(applied_ms);
                    tracing::info!(
                        stream = ?state.stream_id,
                        imaging_seq = latest_imaging_seq,
                        latency_ms,
                        "First IDR observed after imaging update"
                    );
                    state.last_imaging_seq_iframe_logged = latest_imaging_seq;
                }

                // Periodic summary every 300 frames (~10s at 30fps)
                if state.frame_count.is_multiple_of(300) {
                    tracing::debug!(
                        stream = ?state.stream_id,
                        frames = state.frame_count,
                        total_bytes = state.total_bytes,
                        iframes = state.iframe_count,
                        errors = state.error_count,
                        "Frame read loop progress"
                    );
                }

                // SAFETY: the SDK guarantees `data` points to `frame_size` readable
                // bytes until `venc_release_stream` is called below.
                let payload = unsafe {
                    std::slice::from_raw_parts(stream_data.data as *const u8, frame_size)
                };
                let frame = OwnedFrame {
                    data: BytesMut::from(payload),
                    // SDK timestamps are in milliseconds
                    timestamp: stream_data.ts as u32,
                    frame_type,
                    stream_id: state.stream_id,
                };

                // Invoke all callbacks (panic-isolated)
                invoke_owned_callbacks_from_map(callbacks, frame);
            } else {
                tracing::trace!(
                    stream = ?state.stream_id,
                    data_null = stream_data.data.is_null(),
                    len = stream_data.len,
                    "Frame skipped: null data or zero length"
                );
            }

            // Release the SDK buffer back to the encoder.
            // SAFETY: We pass back the same stream struct that get_stream populated.
            // The data pointer is owned by the SDK and must be returned.
            // This MUST happen even during shutdown to avoid leaking SDK buffers.
            let _ = ffi.venc_release_stream(handle.as_ptr(), stream_data);
            tracing::trace!(stream = ?state.stream_id, "SDK buffer released");
        }

        frames_this_cycle
    }

    let has_sub = sub_stream_handle.is_some();

    tracing::info!(
        has_sub_stream = has_sub,
        "Unified frame read loop started (push-only mode)"
    );

    let mut main_state = StreamState::new(StreamId::VideoMain, main_enc_addr);
    let mut sub_state = StreamState::new(StreamId::VideoSub, sub_enc_addr);

    if anyka_ipc.is_none() {
        #[cfg(test)]
        {
            let idle_poll_sleep_ms = env_var_u64("ANYKA_FRAME_POLL_SLEEP_MS")
                .unwrap_or(50)
                .max(1);
            let active_poll_sleep_ms = env_var_u64("ONVIF_ACTIVE_POLL_SLEEP_MS")
                .unwrap_or(8)
                .max(1);
            let default_no_data_idr_trigger_ms =
                u64::from(NO_DATA_IDR_RECOVERY_EVERY_ERRORS) * idle_poll_sleep_ms;
            let no_data_idr_trigger_ms = env_var_u64("ONVIF_NO_DATA_IDR_TRIGGER_MS")
                .unwrap_or(default_no_data_idr_trigger_ms);
            let mut current_sleep_ms: u64 = idle_poll_sleep_ms;

            while !stop_signal.load(Ordering::SeqCst) {
                let has_active_callbacks = !owned_callbacks.read().is_empty();
                let cycle_sleep_ms = if has_active_callbacks {
                    active_poll_sleep_ms
                } else {
                    idle_poll_sleep_ms
                };
                let no_data_recovery_interval_errors = compute_no_data_recovery_interval_errors(
                    no_data_idr_trigger_ms,
                    cycle_sleep_ms,
                );

                let main_frames = drain_stream(
                    &main_stream_handle,
                    _ffi.as_ref(),
                    &mut main_state,
                    &owned_callbacks,
                    &stop_signal,
                    &stream_health,
                    no_data_recovery_interval_errors,
                );

                let sub_frames = if let Some(ref sub_sh) = sub_stream_handle {
                    drain_stream(
                        sub_sh,
                        _ffi.as_ref(),
                        &mut sub_state,
                        &owned_callbacks,
                        &stop_signal,
                        &stream_health,
                        no_data_recovery_interval_errors,
                    )
                } else {
                    0
                };

                let total_frames_this_cycle = main_frames + sub_frames;
                if has_active_callbacks || total_frames_this_cycle > 0 {
                    current_sleep_ms = cycle_sleep_ms;
                } else {
                    current_sleep_ms = (current_sleep_ms * 2).min(cycle_sleep_ms * 4);
                }
                std::thread::sleep(Duration::from_millis(current_sleep_ms));
            }

            tracing::info!("Unified frame read loop exited (test fallback mode)");
            return;
        }

        #[cfg(not(test))]
        {
            tracing::error!("Push-only mode requires AnykaIpc; unified reader exiting");
            return;
        }
    }

    let ipc = if let Some(ipc) = anyka_ipc.as_ref() {
        ipc
    } else {
        tracing::error!("Push-only mode requires AnykaIpc; unified reader exiting");
        return;
    };

    if let Err(e) = ipc.start_push(main_stream_handle.as_ptr(), StreamId::VideoMain) {
        tracing::error!("Failed to start push mode for main stream: {}", e);
        return;
    }
    let mut sub_push_started = false;
    if let Some(ref sub_handle) = sub_stream_handle {
        if let Err(e) = ipc.start_push(sub_handle.as_ptr(), StreamId::VideoSub) {
            tracing::error!("Failed to start push mode for sub stream: {}", e);
            let _ = ipc.stop_push(Some(StreamId::VideoMain));
            return;
        }
        sub_push_started = true;
    }

    tracing::info!(
        has_sub_stream = sub_push_started,
        "Push-based frame delivery active"
    );

    while !stop_signal.load(Ordering::SeqCst) {
        match ipc.recv_pushed_frame(frame_pool.as_deref()) {
            Ok(owned_frame) => {
                let state = match owned_frame.stream_id {
                    StreamId::VideoMain => &mut main_state,
                    StreamId::VideoSub => &mut sub_state,
                    StreamId::Audio => {
                        tracing::trace!("Ignoring unexpected audio frame in video loop");
                        continue;
                    }
                };

                state.consecutive_no_data = 0;
                state.last_error_was_no_data = true;
                let frame_type = owned_frame.frame_type;
                let frame_size = owned_frame.data.len();

                state.frame_count += 1;
                stream_health.record_frame(owned_frame.stream_id);
                state.total_bytes += frame_size as u64;
                if matches!(frame_type, FrameType::VideoIFrame) {
                    state.iframe_count += 1;
                }

                if state.frame_count.is_multiple_of(300) {
                    let (overflow, eviction, fallback, dropped) = ipc.shm_diagnostic_counters();
                    tracing::debug!(
                        stream = ?owned_frame.stream_id,
                        frames = state.frame_count,
                        total_bytes = state.total_bytes,
                        iframes = state.iframe_count,
                        shm_overflow = overflow,
                        shm_eviction = eviction,
                        shm_fallback = fallback,
                        shm_dropped = dropped,
                        "Push-mode frame delivery progress"
                    );
                }

                invoke_owned_callbacks_from_map(&owned_callbacks, owned_frame);
            }
            Err(PlatformError::Shutdown(reason)) => {
                // Orderly producer shutdown — not a failure, so exit quietly.
                tracing::info!(%reason, "Push mode ending: vendor-daemon shut down");
                break;
            }
            Err(e) => {
                tracing::debug!("Push recv error: {}", e);
                if is_push_mode_transient_error(&e) {
                    continue;
                }
                tracing::error!("Push mode interrupted by non-transient error: {}", e);
                break;
            }
        }
    }

    if sub_push_started {
        let _ = ipc.stop_push(Some(StreamId::VideoSub));
    }
    let _ = ipc.stop_push(Some(StreamId::VideoMain));
    tracing::info!("Push mode ended");

    tracing::info!(
        main_frames = main_state.frame_count,
        main_bytes = main_state.total_bytes,
        main_iframes = main_state.iframe_count,
        main_errors = main_state.error_count,
        sub_frames = sub_state.frame_count,
        sub_bytes = sub_state.total_bytes,
        sub_iframes = sub_state.iframe_count,
        sub_errors = sub_state.error_count,
        "Unified frame read loop exited"
    );
}

/// Invoke all registered owned-frame callbacks, transferring ownership.
///
/// If there is exactly one callback (common case — just `StreamingBridge`),
/// the `OwnedFrame` is moved directly — true zero-copy.
///
/// If there are multiple callbacks, each except the last receives a clone.
/// With no callbacks registered the frame is simply dropped.
pub(super) fn invoke_owned_callbacks_from_map(
    owned_callbacks: &RwLock<HashMap<CallbackId, Arc<dyn OwnedFrameCallback>>>,
    owned_frame: OwnedFrame,
) {
    let cbs = owned_callbacks.read();
    let cb_count = cbs.len();

    if cb_count == 0 {
        return;
    }

    tracing::trace!(
        callback_count = cb_count,
        stream = ?owned_frame.stream_id,
        "Invoking owned frame callbacks (zero-copy)"
    );

    // Collect Arc refs so we can drop the read lock before invoking
    let callbacks: Vec<(CallbackId, Arc<dyn OwnedFrameCallback>)> =
        cbs.iter().map(|(id, cb)| (*id, Arc::clone(cb))).collect();
    drop(cbs);

    let mut failed = Vec::new();
    let last_idx = callbacks.len() - 1;

    for (i, (id, cb)) in callbacks.iter().enumerate() {
        let start = std::time::Instant::now();

        let frame_to_send = if i < last_idx {
            // Not the last callback — clone the data
            OwnedFrame {
                data: owned_frame.data.clone(),
                timestamp: owned_frame.timestamp,
                frame_type: owned_frame.frame_type,
                stream_id: owned_frame.stream_id,
            }
        } else {
            // Last callback — will get the moved frame below
            break;
        };

        let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
            cb.on_owned_frame(frame_to_send);
        }));

        let elapsed = start.elapsed();
        let elapsed_us = elapsed.as_micros() as u64;
        record_callback_duration(elapsed_us);

        if elapsed_us > CALLBACK_SLOW_WARN_THRESHOLD_US {
            maybe_log_slow_owned_callback(*id, elapsed_us);
        }

        if result.is_err() {
            tracing::error!("Owned frame callback {} panicked, marking for removal", id);
            failed.push(*id);
        }
    }

    // Invoke the last callback with the original owned_frame (moved)
    let (last_id, last_cb) = &callbacks[last_idx];
    let start = std::time::Instant::now();
    let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
        last_cb.on_owned_frame(owned_frame);
    }));
    let elapsed = start.elapsed();
    let elapsed_us = elapsed.as_micros() as u64;
    record_callback_duration(elapsed_us);
    if elapsed_us > CALLBACK_SLOW_WARN_THRESHOLD_US {
        maybe_log_slow_owned_callback(*last_id, elapsed_us);
    }
    if result.is_err() {
        tracing::error!(
            "Owned frame callback {} panicked, marking for removal",
            last_id
        );
        failed.push(*last_id);
    }

    // Remove failed callbacks
    if !failed.is_empty() {
        let mut cbs_write = owned_callbacks.write();
        for id in failed {
            cbs_write.remove(&id);
        }
    }

    maybe_log_callback_histogram();
}

/// Anyka video encoder implementation with FFI integration and callback support.
///
/// Manages dual video encoders (main 720p + sub 360p) with:
/// - RAII-based FFI handles via `VideoEncoderHandle`
/// - Zero-copy frame delivery to multiple subscribers
/// - Panic-isolated callback invocation
/// - Dynamic bitrate reconfiguration
///
/// # Architecture
///
/// ```text
/// AnykaVideoEncoder
///   ├── ffi: Arc<dyn VideoHalTrait>       (injected, mockable)
///   ├── main_handle: RwLock<Option<Arc<VideoEncoderHandle>>>
///   ├── sub_handle:  RwLock<Option<Arc<VideoEncoderHandle>>>
///   └── owned_callbacks: RwLock<HashMap<CallbackId, Arc<dyn OwnedFrameCallback>>>
/// ```
pub(super) struct AnykaVideoEncoder {
    pub(super) ffi: Arc<dyn crate::hal::common::video::VideoHalTrait>,
    /// Optional AnykaIpc reference for the zero-copy owned frame path.
    /// This is the same object as `ffi` (when using IPC mode), stored
    /// separately because we can't downcast `dyn VideoHalTrait` to `AnykaIpc`.
    pub(super) anyka_ipc: Option<Arc<AnykaIpc>>,
    pub(super) configurations: RwLock<Vec<VideoEncoderConfig>>,
    pub(super) main_handle: RwLock<Option<Arc<VideoEncoderHandle>>>,
    pub(super) sub_handle: RwLock<Option<Arc<VideoEncoderHandle>>>,
    pub(super) main_state: RwLock<EncoderState>,
    pub(super) sub_state: RwLock<EncoderState>,
    pub(super) owned_callbacks: Arc<RwLock<HashMap<CallbackId, Arc<dyn OwnedFrameCallback>>>>,
    pub(super) next_callback_id: AtomicU64,
    pub(super) main_stream_handle: RwLock<Option<Arc<VideoStreamHandle>>>,
    pub(super) sub_stream_handle: RwLock<Option<Arc<VideoStreamHandle>>>,
    pub(super) read_thread: RwLock<Option<std::thread::JoinHandle<()>>>,
    pub(super) stop_signal: Arc<AtomicBool>,
    pub(super) stream_health: Arc<StreamHealthCounters>,
    pub(super) unsafe_shutdown_required: AtomicBool,
}

#[cfg(test)]
const STREAM_THREAD_JOIN_TIMEOUT: Duration = Duration::from_millis(50);
#[cfg(not(test))]
const STREAM_THREAD_JOIN_TIMEOUT: Duration = Duration::from_secs(3);
#[cfg(test)]
const STREAM_CANCEL_TIMEOUT: Duration = Duration::from_millis(20);
#[cfg(not(test))]
const STREAM_CANCEL_TIMEOUT: Duration = Duration::from_secs(2);
/// Grace period after setting `stop_signal` before cancelling streams.
/// Gives non-stuck reader threads time to check `stop_signal` and exit the
/// drain loop naturally. Dana vendor uses 20ms; we use 50ms for ~1.5 poll
/// cycles at the default 30ms sleep interval.
#[cfg(test)]
const CANCEL_GRACE_PERIOD: Duration = Duration::from_millis(5);
#[cfg(not(test))]
const CANCEL_GRACE_PERIOD: Duration = Duration::from_millis(50);

/// Join a thread with a timeout. Returns `true` if the thread completed (success or
/// panic), otherwise returns the original join handle so caller can retry after an
/// emergency unblock action.
fn join_thread_with_timeout(
    thread: std::thread::JoinHandle<()>,
    name: &str,
    timeout: Duration,
) -> Result<(), std::thread::JoinHandle<()>> {
    let start = std::time::Instant::now();
    let thread = thread;
    while !thread.is_finished() {
        if start.elapsed() >= timeout {
            tracing::error!(
                "Thread '{}' join timed out after {:?} — thread may be stuck in kernel I/O",
                name,
                timeout
            );
            return Err(thread);
        }
        std::thread::sleep(Duration::from_millis(10));
    }

    match thread.join() {
        Ok(()) => {
            tracing::info!("Thread '{}' joined successfully", name);
        }
        Err(e) => {
            tracing::warn!("Thread '{}' panicked: {:?}", name, e);
        }
    }

    Ok(())
}

impl AnykaVideoEncoder {
    /// Create a new `AnykaVideoEncoder` with the default (real) FFI backend.
    ///
    /// Uses `AnykaIpc` to connect to the vendor daemon for vendor library access.
    pub(super) fn new() -> PlatformResult<Self> {
        let ipc = crate::hal::anyka::ipc::AnykaIpc::new().map_err(|e| {
            PlatformError::InitializationFailed(format!(
                "AnykaVideoEncoder: AnykaIpc connection failed: {}",
                e
            ))
        })?;
        tracing::info!("AnykaVideoEncoder: using AnykaIpc for vendor library access");
        Ok(Self::with_ffi(Arc::new(ipc)))
    }

    /// Create a new `AnykaVideoEncoder` with a custom FFI backend.
    ///
    /// Used by tests with `MockVideoHalTrait` for hardware-free testing.
    pub(super) fn with_ffi(ffi: Arc<dyn crate::hal::common::video::VideoHalTrait>) -> Self {
        Self {
            ffi,
            anyka_ipc: None, // No AnykaIpc available when using custom FFI
            configurations: RwLock::new(vec![
                VideoEncoderConfig {
                    token: "VideoEncoder_1".to_string(),
                    name: "Main Stream".to_string(),
                    resolution: Resolution::new(1280, 720),
                    framerate: 15,
                    bitrate: 2000,
                    encoding: VideoEncoding::H264,
                    gop_length: 30,
                    quality: 80,
                    ..Default::default()
                },
                VideoEncoderConfig {
                    token: "VideoEncoder_2".to_string(),
                    name: "Sub Stream".to_string(),
                    resolution: Resolution::new(640, 360),
                    framerate: 15,
                    bitrate: 300,
                    encoding: VideoEncoding::H264,
                    gop_length: 30,
                    quality: 70,
                    ..Default::default()
                },
            ]),
            main_handle: RwLock::new(None),
            sub_handle: RwLock::new(None),
            main_state: RwLock::new(EncoderState::Uninitialized),
            sub_state: RwLock::new(EncoderState::Uninitialized),
            owned_callbacks: Arc::new(RwLock::new(HashMap::new())),
            next_callback_id: AtomicU64::new(1),
            main_stream_handle: RwLock::new(None),
            sub_stream_handle: RwLock::new(None),
            read_thread: RwLock::new(None),
            stop_signal: Arc::new(AtomicBool::new(false)),
            stream_health: Arc::new(StreamHealthCounters::default()),
            unsafe_shutdown_required: AtomicBool::new(false),
        }
    }

    /// Create a new `AnykaVideoEncoder` with a shared AnykaIpc instance.
    ///
    /// The AnykaIpc is stored both as the `dyn VideoHalTrait` backend and as a
    /// concrete reference for the zero-copy frame fetch path.
    pub(super) fn with_ipc(ipc: Arc<AnykaIpc>) -> Self {
        let mut encoder =
            Self::with_ffi(ipc.clone() as Arc<dyn crate::hal::common::video::VideoHalTrait>);
        encoder.anyka_ipc = Some(ipc);
        encoder
    }

    pub(super) fn mark_unsafe_shutdown(&self, reason: &str) {
        let first = !self.unsafe_shutdown_required.swap(true, Ordering::SeqCst);
        if first {
            tracing::error!(
                reason = reason,
                "Unsafe video teardown detected; hard process termination required"
            );
        } else {
            tracing::error!(
                reason = reason,
                "Unsafe video teardown already active; preserving hard-exit requirement"
            );
        }
    }

    pub(super) fn leak_stream_handles_for_hard_shutdown(&self) {
        if let Some(handle) = self.main_stream_handle.write().take() {
            std::mem::forget(handle);
        }
        if let Some(handle) = self.sub_stream_handle.write().take() {
            std::mem::forget(handle);
        }
    }

    pub(super) fn leak_encoder_handles_for_hard_shutdown(&self) {
        if let Some(handle) = self.main_handle.write().take() {
            *self.main_state.write() = EncoderState::Uninitialized;
            std::mem::forget(handle);
        }
        if let Some(handle) = self.sub_handle.write().take() {
            *self.sub_state.write() = EncoderState::Uninitialized;
            std::mem::forget(handle);
        }
    }

    pub(super) fn fail_fast_to_hard_shutdown(&self, reason: impl Into<String>) -> PlatformError {
        let reason = reason.into();
        self.mark_unsafe_shutdown(&reason);
        self.leak_stream_handles_for_hard_shutdown();
        self.leak_encoder_handles_for_hard_shutdown();
        PlatformError::HardwareFailure(format!("unsafe teardown required: {}", reason))
    }

    pub(super) fn requires_hard_shutdown(&self) -> bool {
        self.unsafe_shutdown_required.load(Ordering::SeqCst)
    }

    pub(super) fn sync_configurations_to_channel_layout(&self, main: Resolution, sub: Resolution) {
        let mut configs = self.configurations.write();
        for cfg in configs.iter_mut() {
            match cfg.token.as_str() {
                "VideoEncoder_1" => cfg.resolution = main,
                "VideoEncoder_2" => cfg.resolution = sub,
                _ => {}
            }
        }
        tracing::info!(
            "Aligned encoder configurations to VI layout: main={}x{}, sub={}x{}",
            main.width,
            main.height,
            sub.width,
            sub.height
        );
    }

    /// Age of the newest venc-read frame, for runtime health monitoring.
    pub(super) fn stream_frame_age_ms(&self) -> Option<u64> {
        self.stream_health.last_frame_age_ms()
    }

    pub(super) fn wait_for_stream_readiness(
        &self,
        timeout: Duration,
        require_sub: bool,
    ) -> PlatformResult<()> {
        let started = Instant::now();
        loop {
            let health = self.stream_health.snapshot();
            if health.main_frames > 0 && (!require_sub || health.sub_frames > 0) {
                if health.sub_frames == 0 {
                    tracing::warn!(
                        "VI/VENC readiness check passed on main stream only (sub stream has no frames)"
                    );
                } else {
                    tracing::info!(
                        "VI/VENC readiness check passed: main_frames={}, sub_frames={}",
                        health.main_frames,
                        health.sub_frames
                    );
                }
                return Ok(());
            }
            if started.elapsed() >= timeout {
                return Err(PlatformError::InitializationFailed(format!(
                    "VI/VENC pipeline readiness timeout after {:?}: main_frames={}, sub_frames={}, main_no_data_errors={}, sub_no_data_errors={}",
                    timeout,
                    health.main_frames,
                    health.sub_frames,
                    health.main_no_data_errors,
                    health.sub_no_data_errors
                )));
            }
            std::thread::sleep(Duration::from_millis(PIPELINE_READINESS_POLL_MS));
        }
    }

    /// Map a `VideoEncoderConfig` to FFI `encode_param`.
    pub(super) fn config_to_encode_param(
        config: &VideoEncoderConfig,
        channel: encode_use_chn,
    ) -> encode_param {
        let enc_out_type = match config.encoding {
            VideoEncoding::H264 => encode_output_type::H264_ENC_TYPE,
            VideoEncoding::H265 => encode_output_type::HEVC_ENC_TYPE,
            VideoEncoding::Mjpeg => encode_output_type::MJPEG_ENC_TYPE,
        };
        let br_mode = match config.bitrate_mode {
            crate::platform::BitrateMode::Cbr => bitrate_ctrl_mode::BR_MODE_CBR,
            crate::platform::BitrateMode::Vbr => bitrate_ctrl_mode::BR_MODE_VBR,
        };
        encode_param {
            width: config.resolution.width,
            height: config.resolution.height,
            minqp: 20,
            maxqp: 51,
            fps: config.framerate as i32,
            goplen: config.gop_length as i32,
            bps: config.bitrate as i32, // kbps (vendor SDK expects kbps despite field name)
            profile: profile_mode::PROFILE_MAIN,
            use_chn: channel,
            enc_grp: match channel {
                encode_use_chn::ENCODE_MAIN_CHN => encode_group_type::ENCODE_MAINCHN_NET,
                encode_use_chn::ENCODE_SUB_CHN => encode_group_type::ENCODE_SUBCHN_NET,
            },
            br_mode,
            enc_out_type,
        }
    }

    /// Register an owned frame callback (zero-copy path).
    ///
    /// Returns a `CallbackId` that can be used to unregister the callback.
    pub fn register_owned_frame_callback(
        &self,
        callback: Arc<dyn OwnedFrameCallback>,
    ) -> CallbackId {
        let id = self.next_callback_id.fetch_add(1, Ordering::SeqCst);
        self.owned_callbacks.write().insert(id, callback);
        id
    }

    /// Unregister a previously registered owned frame callback.
    pub fn unregister_owned_frame_callback(&self, id: CallbackId) -> bool {
        self.owned_callbacks.write().remove(&id).is_some()
    }

    /// Start streaming from the encoder by requesting stream handles and spawning
    /// dedicated reader threads that poll the SDK for encoded frames.
    ///
    /// # Arguments
    ///
    /// * `vi_handle` - Video input handle (provides raw sensor data)
    /// * `main_enc` - Main encoder handle (720p)
    /// * `sub_enc` - Optional sub encoder handle (360p)
    pub fn start_streaming(
        &self,
        vi_handle: &Arc<crate::hal::common::video::VideoInputHandle>,
        main_enc: &Arc<VideoEncoderHandle>,
        sub_enc: Option<&Arc<VideoEncoderHandle>>,
    ) -> PlatformResult<()> {
        self.stop_signal.store(false, Ordering::SeqCst);
        self.stream_health.reset();

        // ── Phase 1: Request all streams (no reader threads yet) ─────────
        //
        // The vendor reference (ak_onvif_demo.c:498-541) requests ALL streams
        // before reading from any of them. This avoids a race condition where
        // venc_get_stream() (called by a reader thread) iterates the SDK's
        // internal venc_list under `cancel_mutex` while venc_request_stream()
        // modifies that same list under a different lock (`cancel_lock`).

        // 1a. Request main stream
        let main_sh = Arc::new(VideoStreamHandle::new(
            vi_handle.as_ptr(),
            main_enc.as_ptr(),
            Arc::clone(&self.ffi),
        )?);
        *self.main_stream_handle.write() = Some(Arc::clone(&main_sh));

        // 1b. Request sub stream (if encoder exists)
        let sub_sh = if let Some(sub) = sub_enc {
            let sh = Arc::new(VideoStreamHandle::new(
                vi_handle.as_ptr(),
                sub.as_ptr(),
                Arc::clone(&self.ffi),
            )?);
            *self.sub_stream_handle.write() = Some(Arc::clone(&sh));
            Some(sh)
        } else {
            None
        };

        // 1c. Kick initial IDR on requested streams after both requests are complete.
        // This keeps request ordering deterministic while still forcing fast decoder sync.
        if let Err(e) = video_encoder_request_idr(main_enc, self.ffi.as_ref()) {
            tracing::warn!("Failed to set initial I-frame for main stream: {}", e);
        }
        if let Some(sub) = sub_enc
            && let Err(e) = video_encoder_request_idr(sub, self.ffi.as_ref())
        {
            tracing::warn!("Failed to set initial I-frame for sub stream: {}", e);
        }
        tracing::debug!("Video streams requested and IDR kicks issued");

        // ── Phase 2: Single stabilization delay ─────────────────────────
        //
        // Both encoders are now requested and IDR-kicked. Give the ISP/encoder
        // pipeline time to produce the first frames. Configurable via env var
        // for on-device tuning; default 300ms exceeds the vendor's recommended
        // PLATFORM_DELAY_MS_RETRY (200ms) stabilization window.
        let stabilization_ms = stream_stabilization_ms();
        std::thread::sleep(Duration::from_millis(stabilization_ms));
        tracing::debug!(stabilization_ms, "Stream stabilization delay complete");

        // ── Phase 3: Spawn unified reader thread ─────────────────────────
        //
        // A single thread drains both main and sub streams in alternating
        // fashion.  This eliminates IPC mutex contention that occurred when
        // two independent threads competed for the shared UnixStream to the
        // vendor daemon.

        let reader_thread = {
            let ffi = Arc::clone(&self.ffi);
            let stop = Arc::clone(&self.stop_signal);
            let stream_health = Arc::clone(&self.stream_health);
            let main_sh_clone = Arc::clone(&main_sh);
            let sub_sh_clone = sub_sh.as_ref().map(Arc::clone);
            let main_enc_addr = main_enc.as_ptr() as usize;
            let sub_enc_addr = sub_enc.map(|h| h.as_ptr() as usize);
            let anyka_ipc = self.anyka_ipc.clone();
            let owned_callbacks = Arc::clone(&self.owned_callbacks_arc());
            let frame_pool = anyka_ipc
                .as_ref()
                .map(|_| Arc::new(BytesMutPool::default_frame_pool()));
            std::thread::Builder::new()
                .name("venc-read".to_string())
                .spawn(move || {
                    unified_frame_read_loop(
                        main_sh_clone,
                        sub_sh_clone,
                        ffi,
                        owned_callbacks,
                        stop,
                        stream_health,
                        Some(main_enc_addr),
                        sub_enc_addr,
                        anyka_ipc,
                        frame_pool,
                    );
                })
                .map_err(|e| {
                    PlatformError::InitializationFailed(format!(
                        "Failed to spawn reader thread: {}",
                        e
                    ))
                })?
        };
        *self.read_thread.write() = Some(reader_thread);
        tracing::info!(
            has_sub_stream = sub_sh.is_some(),
            "Unified stream reader thread started"
        );

        Ok(())
    }

    /// Stop all frame-read threads and cancel the active video streams.
    ///
    /// **Cancel-first ordering** (matching vendor SDK pattern):
    ///   1. Set `stop_signal` — cooperative exit for non-stuck threads
    ///   2. Sleep `CANCEL_GRACE_PERIOD` — non-stuck threads exit naturally
    ///   3. `cancel_stream` — stops SDK internal threads, unblocks `get_stream`
    ///   4. Join reader threads — should complete quickly after cancel
    ///   5. Drop stream handles
    ///
    /// If cancel fails or join times out after cancel, we fail fast into
    /// unsafe teardown mode requiring process termination.
    pub fn stop_streaming(&self) -> PlatformResult<()> {
        if self.requires_hard_shutdown() {
            return Err(PlatformError::HardwareFailure(
                "unsafe teardown required: previous shutdown failure".to_string(),
            ));
        }

        // Phase 1: Signal stop and give non-stuck threads a grace period to exit.
        tracing::info!("stop_streaming: signalling stop...");
        self.stop_signal.store(true, Ordering::SeqCst);
        std::thread::sleep(CANCEL_GRACE_PERIOD);

        let mut main_stream_handle = self.main_stream_handle.read().clone();
        let mut sub_stream_handle = self.sub_stream_handle.read().clone();
        let mut failures: Vec<String> = Vec::new();
        let mut cancel_failed = false;

        // Phase 2: Cancel streams — stops SDK internal threads and unblocks
        // any reader stuck in ak_venc_get_stream().
        if let Some(ref handle) = main_stream_handle {
            tracing::info!("stop_streaming: cancelling main stream...");
            if let Err(e) = handle.cancel_checked_with_timeout(STREAM_CANCEL_TIMEOUT) {
                tracing::error!("stop_streaming: main stream cancel failed: {}", e);
                failures.push(format!("main stream cancel failed: {}", e));
                cancel_failed = true;
            } else {
                tracing::info!("stop_streaming: main stream cancelled");
            }
        }

        if cancel_failed {
            if sub_stream_handle.is_some() {
                tracing::warn!(
                    "stop_streaming: skipping sub stream cancel after main cancel failure \
                     to avoid lock contention with an in-flight vendor cancel call"
                );
            }
        } else if let Some(ref handle) = sub_stream_handle {
            tracing::info!("stop_streaming: cancelling sub stream...");
            if let Err(e) = handle.cancel_checked_with_timeout(STREAM_CANCEL_TIMEOUT) {
                tracing::error!("stop_streaming: sub stream cancel failed: {}", e);
                failures.push(format!("sub stream cancel failed: {}", e));
                cancel_failed = true;
            } else {
                tracing::info!("stop_streaming: sub stream cancelled");
            }
        }

        if cancel_failed {
            if let Some(handle) = main_stream_handle.take() {
                std::mem::forget(handle);
            }
            if let Some(handle) = sub_stream_handle.take() {
                std::mem::forget(handle);
            }
            return Err(self.fail_fast_to_hard_shutdown(failures.join("; ")));
        }

        // Phase 3: Join the unified reader thread — should complete quickly
        // now that cancel has unblocked any stuck SDK calls.
        if let Some(thread) = self.read_thread.write().take() {
            tracing::info!("stop_streaming: joining reader thread...");
            if let Err(_thread) =
                join_thread_with_timeout(thread, "venc-read", STREAM_THREAD_JOIN_TIMEOUT)
            {
                failures.push(
                    "reader thread join timeout after cancel (possible blocked kernel I/O)"
                        .to_string(),
                );
            }
        }

        if !failures.is_empty() {
            if let Some(handle) = main_stream_handle.take() {
                std::mem::forget(handle);
            }
            if let Some(handle) = sub_stream_handle.take() {
                std::mem::forget(handle);
            }
            return Err(self.fail_fast_to_hard_shutdown(failures.join("; ")));
        }

        // Phase 4: Drop stream handles (cancel already completed).
        tracing::info!("stop_streaming: dropping stream handles...");
        let _ = self.main_stream_handle.write().take();
        let _ = self.sub_stream_handle.write().take();

        tracing::info!("Streaming stopped");
        Ok(())
    }

    /// Get a cloned `Arc` reference to the owned callbacks map for thread sharing.
    fn owned_callbacks_arc(&self) -> Arc<RwLock<HashMap<CallbackId, Arc<dyn OwnedFrameCallback>>>> {
        Arc::clone(&self.owned_callbacks)
    }

    /// Request an IDR (I-frame) from the specified encoder channel.
    ///
    /// # Arguments
    ///
    /// * `main` - If true, request IDR from main encoder; otherwise from sub encoder.
    pub fn request_idr_frame(&self, main: bool) -> PlatformResult<()> {
        let handle_guard = if main {
            self.main_handle.read()
        } else {
            self.sub_handle.read()
        };

        let handle = handle_guard.as_ref().ok_or_else(|| {
            let channel = if main { "main" } else { "sub" };
            PlatformError::HardwareUnavailable(format!("{} encoder not initialized", channel))
        })?;

        video_encoder_request_idr(handle, self.ffi.as_ref())
    }

    /// Close a single encoder by token.
    ///
    /// This is used for initialization rollback when one encoder fails after
    /// previous encoders have been successfully opened.
    pub(super) fn close_encoder(&self, token: &str) -> PlatformResult<()> {
        if self.requires_hard_shutdown() {
            return Err(PlatformError::HardwareFailure(
                "unsafe teardown required: skipping encoder close".to_string(),
            ));
        }

        match token {
            "VideoEncoder_1" => {
                let old_handle = self.main_handle.write().take();
                if let Some(handle) = old_handle {
                    if let Err(e) = handle.close_blocking_with_ffi(self.ffi.as_ref()) {
                        return Err(self.fail_fast_to_hard_shutdown(format!(
                            "main encoder close failed: {}",
                            e
                        )));
                    }
                    *self.main_state.write() = EncoderState::Uninitialized;
                    tracing::info!("Closed video encoder token={}", token);
                }
                Ok(())
            }
            "VideoEncoder_2" => {
                let old_handle = self.sub_handle.write().take();
                if let Some(handle) = old_handle {
                    if let Err(e) = handle.close_blocking_with_ffi(self.ffi.as_ref()) {
                        return Err(self.fail_fast_to_hard_shutdown(format!(
                            "sub encoder close failed: {}",
                            e
                        )));
                    }
                    *self.sub_state.write() = EncoderState::Uninitialized;
                    tracing::info!("Closed video encoder token={}", token);
                }
                Ok(())
            }
            _ => Err(PlatformError::InvalidParameter(format!(
                "Unknown encoder token: {}",
                token
            ))),
        }
    }

    pub(super) fn close_all_encoders(&self) -> PlatformResult<()> {
        if self.requires_hard_shutdown() {
            return Err(PlatformError::HardwareFailure(
                "unsafe teardown required: skipping encoder close".to_string(),
            ));
        }
        self.close_encoder("VideoEncoder_2")?;
        self.close_encoder("VideoEncoder_1")?;
        Ok(())
    }
}

#[async_trait]
impl VideoEncoder for AnykaVideoEncoder {
    async fn init(&self, config: &VideoEncoderConfig) -> PlatformResult<()> {
        let (channel, handle_lock, state_lock) = match config.token.as_str() {
            "VideoEncoder_1" => (
                encode_use_chn::ENCODE_MAIN_CHN,
                &self.main_handle,
                &self.main_state,
            ),
            "VideoEncoder_2" => (
                encode_use_chn::ENCODE_SUB_CHN,
                &self.sub_handle,
                &self.sub_state,
            ),
            _ => {
                return Err(PlatformError::InvalidParameter(format!(
                    "Unknown encoder token: {}. Expected VideoEncoder_1 or VideoEncoder_2",
                    config.token
                )));
            }
        };

        // Validate encoder resolution before opening
        if config.resolution.width == 0 || config.resolution.height == 0 {
            return Err(PlatformError::InvalidParameter(
                "Encoder resolution must be non-zero".to_string(),
            ));
        }
        if !config.resolution.width.is_multiple_of(4) || !config.resolution.height.is_multiple_of(4)
        {
            return Err(PlatformError::InvalidParameter(format!(
                "Encoder resolution {}x{} must be divisible by 4",
                config.resolution.width, config.resolution.height
            )));
        }

        let param = Self::config_to_encode_param(config, channel);

        tracing::debug!(
            "Opening encoder {}: {}x{} @ {}fps, {}kbps, goplen={}, enc_grp={:?}, use_chn={:?}, param_size={}",
            config.token,
            param.width,
            param.height,
            param.fps,
            param.bps,
            param.goplen,
            param.enc_grp,
            param.use_chn,
            std::mem::size_of::<encode_param>(),
        );

        // Point the vendor library at our SD card venc.cfg before opening.
        // The V2 encoder doesn't read this file but ak_venc_open requires it to exist.
        let cfg_path = c"/mnt/anyka_hack/onvif/venc.cfg";
        self.ffi.venc_set_cfg_path(cfg_path.as_ptr());

        let enc_handle = video_encoder_open(&param, self.ffi.as_ref())?;

        *handle_lock.write() = Some(Arc::new(enc_handle));
        *state_lock.write() = EncoderState::Initialized;

        // Update stored configuration
        let mut configs = self.configurations.write();
        if let Some(cfg) = configs.iter_mut().find(|c| c.token == config.token) {
            *cfg = config.clone();
        } else {
            configs.push(config.clone());
        }

        tracing::info!(
            "Video encoder {} initialized: {}x{} @ {}fps, {}kbps",
            config.token,
            config.resolution.width,
            config.resolution.height,
            config.framerate,
            config.bitrate
        );

        Ok(())
    }

    async fn get_configuration(&self) -> PlatformResult<VideoEncoderConfig> {
        let configs = self.configurations.read();
        configs
            .first()
            .cloned()
            .ok_or_else(|| PlatformError::HardwareUnavailable("No encoder configured".to_string()))
    }

    async fn set_configuration(&self, config: &VideoEncoderConfig) -> PlatformResult<()> {
        let handle_guard = match config.token.as_str() {
            "VideoEncoder_1" => self.main_handle.read(),
            "VideoEncoder_2" => self.sub_handle.read(),
            _ => {
                return Err(PlatformError::InvalidParameter(format!(
                    "Unknown encoder token: {}",
                    config.token
                )));
            }
        };

        // If the encoder handle exists, apply bitrate change via FFI
        if let Some(handle) = handle_guard.as_ref() {
            let current_config = {
                let configs = self.configurations.read();
                configs.iter().find(|c| c.token == config.token).cloned()
            };

            if let Some(ref current) = current_config {
                if current.bitrate != config.bitrate {
                    let bps = config.bitrate as i32; // kbps (vendor SDK expects kbps)
                    video_encoder_set_rc(handle, bps, self.ffi.as_ref())?;
                    tracing::info!(
                        "Encoder {} bitrate changed: {}kbps → {}kbps",
                        config.token,
                        current.bitrate,
                        config.bitrate
                    );
                }

                // Warn about changes that require encoder restart
                if current.resolution != config.resolution
                    || current.framerate != config.framerate
                    || current.gop_length != config.gop_length
                    || current.encoding != config.encoding
                {
                    tracing::warn!(
                        "Encoder {} configuration change requires restart for: resolution/fps/gop/encoding",
                        config.token
                    );
                }
            }
        }
        drop(handle_guard);

        // Update stored configuration
        let mut configs = self.configurations.write();
        if let Some(cfg) = configs.iter_mut().find(|c| c.token == config.token) {
            *cfg = config.clone();
            Ok(())
        } else {
            Err(PlatformError::InvalidParameter(format!(
                "Unknown encoder token: {}",
                config.token
            )))
        }
    }

    async fn get_configurations(&self) -> PlatformResult<Vec<VideoEncoderConfig>> {
        Ok(self.configurations.read().clone())
    }

    async fn get_options(&self) -> PlatformResult<VideoEncoderOptions> {
        Ok(VideoEncoderOptions {
            resolutions: vec![
                Resolution::new(1920, 1080),
                Resolution::new(1280, 720),
                Resolution::new(640, 360),
            ],
            encodings: vec![VideoEncoding::H264],
            framerate_range: (1, 30),
            bitrate_range: (128, 8000),
            gop_range: (1, 300),
            quality_range: (0, 100),
        })
    }
}
