//! Bridge between platform frame callbacks and streaming-lib channels.
//!
//! `StreamingBridge` implements [`FrameCallback`] to receive raw encoded frames
//! from the Anyka SDK and converts them into [`FrameData`] for the streaming
//! pipeline. It routes frames by [`StreamId`]:
//!
//! - `VideoMain` → main stream channel
//! - `VideoSub` → sub stream channel
//! - `Audio` → both main and sub stream channels

use bytes::BytesMut;
use parking_lot::{Mutex, RwLock};
use portable_atomic::{AtomicU32, AtomicU64, AtomicUsize};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Instant;
use streaming_lib::FrameData;
use tokio::sync::Notify;

use crate::platform::common::{Frame, FrameCallback, FrameType, OwnedFrame, StreamId};

const DEFAULT_MAIN_QUEUE_CAPACITY: usize = 4;
const DEFAULT_SUB_QUEUE_CAPACITY: usize = 6;
const QUEUE_TELEMETRY_LOG_INTERVAL: u64 = 500;

#[derive(Clone)]
pub struct QueuedFrame {
    pub frame: FrameData,
    pub enqueue_instant: Instant,
    pub capture_timestamp_ms: u32,
    pub is_video_idr: bool,
}

#[derive(Default)]
struct QueueTelemetry {
    enqueued: AtomicU64,
    dequeued: AtomicU64,
    dropped_on_overflow: AtomicU64,
    dropped_on_flush: AtomicU64,
    max_depth: AtomicUsize,
}

impl QueueTelemetry {
    /// Record a successful push: increment enqueued, update max depth, and
    /// accumulate any drop counters from eviction that occurred before the push.
    fn record_push(&self, depth: usize, dropped_on_overflow: u64, dropped_on_flush: u64) {
        self.enqueued
            .fetch_add(1, portable_atomic::Ordering::Relaxed);
        if dropped_on_overflow > 0 {
            self.dropped_on_overflow
                .fetch_add(dropped_on_overflow, portable_atomic::Ordering::Relaxed);
        }
        if dropped_on_flush > 0 {
            self.dropped_on_flush
                .fetch_add(dropped_on_flush, portable_atomic::Ordering::Relaxed);
        }
        self.max_depth
            .fetch_max(depth, portable_atomic::Ordering::Relaxed);
    }

    /// Record a frame drop without enqueue (all IDR slots occupied).
    fn record_drop(&self, count: u64) {
        self.dropped_on_overflow
            .fetch_add(count, portable_atomic::Ordering::Relaxed);
    }
}

pub struct LowLatencyFrameQueue {
    label: &'static str,
    capacity: usize,
    queue: Mutex<VecDeque<QueuedFrame>>,
    notify: Notify,
    telemetry: QueueTelemetry,
}

impl LowLatencyFrameQueue {
    pub fn new(label: &'static str, capacity: usize) -> Arc<Self> {
        let capacity = capacity.max(1);
        Arc::new(Self {
            label,
            capacity,
            queue: Mutex::new(VecDeque::with_capacity(capacity)),
            notify: Notify::new(),
            telemetry: QueueTelemetry::default(),
        })
    }

    pub fn default_main() -> Arc<Self> {
        Self::new("main", DEFAULT_MAIN_QUEUE_CAPACITY)
    }

    pub fn default_sub() -> Arc<Self> {
        Self::new("sub", DEFAULT_SUB_QUEUE_CAPACITY)
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn push(&self, frame: FrameData, capture_timestamp_ms: u32, is_video_idr: bool) {
        let mut queue = self.queue.lock();
        let mut dropped_on_overflow = 0u64;
        let mut dropped_on_flush = 0u64;

        if queue.len() >= self.capacity {
            if is_video_idr {
                dropped_on_flush = queue.len() as u64;
                queue.clear();
            } else if let Some(index) = queue.iter().position(|candidate| !candidate.is_video_idr) {
                queue.remove(index);
                dropped_on_overflow = 1;
            } else {
                drop(queue);
                self.telemetry.record_drop(1);
                self.maybe_log_telemetry();
                return;
            }
        }

        queue.push_back(QueuedFrame {
            frame,
            enqueue_instant: Instant::now(),
            capture_timestamp_ms,
            is_video_idr,
        });
        let depth = queue.len();
        drop(queue);

        self.telemetry
            .record_push(depth, dropped_on_overflow, dropped_on_flush);
        self.notify.notify_one();
        self.maybe_log_telemetry();
    }

    pub async fn recv(&self) -> QueuedFrame {
        loop {
            if let Some(frame) = self.try_recv() {
                return frame;
            }
            self.notify.notified().await;
        }
    }

    pub fn try_recv(&self) -> Option<QueuedFrame> {
        let mut queue = self.queue.lock();
        let frame = queue.pop_front()?;
        drop(queue);

        self.telemetry
            .dequeued
            .fetch_add(1, portable_atomic::Ordering::Relaxed);
        Some(frame)
    }

    fn maybe_log_telemetry(&self) {
        let enqueued = self
            .telemetry
            .enqueued
            .load(portable_atomic::Ordering::Relaxed);
        if enqueued == 0 || !enqueued.is_multiple_of(QUEUE_TELEMETRY_LOG_INTERVAL) {
            return;
        }

        tracing::debug!(
            queue = self.label,
            capacity = self.capacity,
            enqueued,
            dequeued = self
                .telemetry
                .dequeued
                .load(portable_atomic::Ordering::Relaxed),
            dropped_on_overflow = self
                .telemetry
                .dropped_on_overflow
                .load(portable_atomic::Ordering::Relaxed),
            dropped_on_flush = self
                .telemetry
                .dropped_on_flush
                .load(portable_atomic::Ordering::Relaxed),
            max_depth = self
                .telemetry
                .max_depth
                .load(portable_atomic::Ordering::Relaxed),
            "Low-latency queue telemetry"
        );
    }

    /// Snapshot current queue statistics (non-atomic read of atomic counters).
    pub fn snapshot(&self) -> QueueSnapshot {
        QueueSnapshot {
            enqueued: self
                .telemetry
                .enqueued
                .load(portable_atomic::Ordering::Relaxed),
            dropped_on_overflow: self
                .telemetry
                .dropped_on_overflow
                .load(portable_atomic::Ordering::Relaxed),
            dropped_on_flush: self
                .telemetry
                .dropped_on_flush
                .load(portable_atomic::Ordering::Relaxed),
            max_depth: self
                .telemetry
                .max_depth
                .load(portable_atomic::Ordering::Relaxed),
        }
    }
}

/// Point-in-time snapshot of queue telemetry counters.
pub struct QueueSnapshot {
    pub enqueued: u64,
    pub dropped_on_overflow: u64,
    pub dropped_on_flush: u64,
    pub max_depth: usize,
}

/// Cached SPS/PPS for a single stream, used to carry parameter sets across
/// bridge re-creation (e.g. when queue capacities change in `StreamingService::start()`).
#[derive(Default, Clone)]
pub struct CachedParameterSets {
    /// Cached Sequence Parameter Set.
    pub sps: Option<Vec<u8>>,
    /// Cached Picture Parameter Set.
    pub pps: Option<Vec<u8>>,
}

/// Default buffer capacity in bytes (64 KB — covers most P-frames).
const DEFAULT_POOL_BUFFER_CAPACITY: usize = 64 * 1024;
/// Maximum number of buffers kept in the pool.
const DEFAULT_POOL_MAX_SIZE: usize = 8;

/// Fixed-capacity pool of reusable `BytesMut` buffers.
///
/// Designed for the frame receive path where buffers are allocated frequently
/// with similar sizes. Reduces malloc pressure on uClibc (target device).
///
/// # Pool Sizing (24MB budget)
///
/// - Default buffer capacity: 64 KB (covers most P-frames)
/// - Max pool size: 8 buffers (512 KB total)
/// - I-frames > 64KB get fresh allocations (infrequent, ~1/sec per stream)
///
/// # Thread Safety
///
/// Uses `parking_lot::Mutex` for the internal buffer store. The lock is held
/// only briefly during get/put operations (no allocations under lock).
pub struct BytesMutPool {
    pool: Mutex<Vec<BytesMut>>,
    default_capacity: usize,
    max_pool_size: usize,
}

impl BytesMutPool {
    /// Create a new buffer pool.
    ///
    /// # Arguments
    ///
    /// * `default_capacity` — Default capacity for newly allocated buffers.
    /// * `max_pool_size` — Maximum number of buffers to keep in the pool.
    pub fn new(default_capacity: usize, max_pool_size: usize) -> Self {
        Self {
            pool: Mutex::new(Vec::with_capacity(max_pool_size)),
            default_capacity,
            max_pool_size,
        }
    }

    /// Create a pool with default sizing for frame receive.
    pub fn default_frame_pool() -> Self {
        Self::new(DEFAULT_POOL_BUFFER_CAPACITY, DEFAULT_POOL_MAX_SIZE)
    }

    /// Get a buffer from the pool, or allocate a new one.
    ///
    /// If a pooled buffer with sufficient capacity is available, it is returned
    /// (cleared but not reallocated). Otherwise, a fresh buffer is allocated
    /// with `max(min_capacity, default_capacity)`.
    pub fn get(&self, min_capacity: usize) -> BytesMut {
        if let Some(mut buf) = self.pool.lock().pop() {
            buf.clear();
            if buf.capacity() >= min_capacity {
                return buf;
            }
            // Buffer too small — drop it, allocate fresh
        }
        BytesMut::with_capacity(min_capacity.max(self.default_capacity))
    }

    /// Return a buffer to the pool for reuse.
    ///
    /// If the pool is full, the buffer is dropped instead of stored.
    pub fn put(&self, buf: BytesMut) {
        let mut pool = self.pool.lock();
        if pool.len() < self.max_pool_size {
            pool.push(buf);
        }
        // else: drop buf (pool full)
    }

    /// Get the number of buffers currently in the pool.
    #[cfg(test)]
    pub fn available(&self) -> usize {
        self.pool.lock().len()
    }
}

/// Per-stream state maintained by the bridge.
pub struct StreamState {
    /// Bounded queue feeding the fanout task.
    pub frame_queue: Arc<LowLatencyFrameQueue>,
    /// Cached SPS NAL unit (extracted from IDR frames).
    pub sps: RwLock<Option<Vec<u8>>>,
    /// Cached PPS NAL unit (extracted from IDR frames).
    pub pps: RwLock<Option<Vec<u8>>>,
    /// Last video timestamp in milliseconds (for late-joining subscribers).
    pub last_timestamp_ms: Arc<AtomicU32>,
    /// Cached latest IDR frame for late-joining subscribers (updated on every I-frame).
    pub bootstrap_idr: RwLock<Option<Vec<u8>>>,
}

/// Bridge that receives raw SDK frames and routes them to streaming channels.
///
/// Implements [`FrameCallback`] for integration with the platform abstraction.
/// The `on_frame` method must complete within 2ms — it performs only a memcpy
/// into `BytesMut` and a bounded queue push.
pub struct StreamingBridge {
    /// State for the main video stream.
    pub main_stream: StreamState,
    /// State for the sub video stream.
    pub sub_stream: StreamState,
    /// Cached audio config bytes (AAC AudioSpecificConfig).
    pub audio_config: RwLock<Option<Vec<u8>>>,
    /// Audio sample rate in Hz.
    pub audio_sample_rate: u32,
}

impl StreamingBridge {
    /// Create a new bridge with the given frame channels.
    pub fn new(
        main_queue: Arc<LowLatencyFrameQueue>,
        sub_queue: Arc<LowLatencyFrameQueue>,
        audio_sample_rate: u32,
    ) -> Self {
        Self::new_with_cached_params(
            main_queue,
            sub_queue,
            audio_sample_rate,
            CachedParameterSets::default(),
            CachedParameterSets::default(),
        )
    }

    /// Create a new bridge with cached SPS/PPS from a previous bridge instance.
    ///
    /// This is used during bridge re-creation (e.g. when `StreamingService::start()`
    /// rebuilds the bridge with new queue capacities) to carry over parameter sets
    /// so late-joining subscribers can still receive SPS/PPS before the next IDR frame.
    pub fn new_with_cached_params(
        main_queue: Arc<LowLatencyFrameQueue>,
        sub_queue: Arc<LowLatencyFrameQueue>,
        audio_sample_rate: u32,
        main_cached: CachedParameterSets,
        sub_cached: CachedParameterSets,
    ) -> Self {
        Self {
            main_stream: StreamState {
                frame_queue: main_queue,
                sps: RwLock::new(main_cached.sps),
                pps: RwLock::new(main_cached.pps),
                last_timestamp_ms: Arc::new(AtomicU32::new(0)),
                bootstrap_idr: RwLock::new(None),
            },
            sub_stream: StreamState {
                frame_queue: sub_queue,
                sps: RwLock::new(sub_cached.sps),
                pps: RwLock::new(sub_cached.pps),
                last_timestamp_ms: Arc::new(AtomicU32::new(0)),
                bootstrap_idr: RwLock::new(None),
            },
            audio_config: RwLock::new(None),
            audio_sample_rate,
        }
    }

    /// Read the current SPS/PPS for a stream as a `CachedParameterSets`.
    pub fn cached_params(&self, stream_id: StreamId) -> CachedParameterSets {
        let stream = match stream_id {
            StreamId::VideoMain => &self.main_stream,
            StreamId::VideoSub => &self.sub_stream,
            StreamId::Audio => return CachedParameterSets::default(),
        };
        CachedParameterSets {
            sps: stream.sps.read().clone(),
            pps: stream.pps.read().clone(),
        }
    }

    /// Convert a raw SDK frame to `FrameData` and route to the appropriate channel(s).
    pub fn route_frame(&self, frame: &Frame) {
        // SAFETY: frame.data is valid for frame.size bytes during this callback.
        let data = BytesMut::from(unsafe { std::slice::from_raw_parts(frame.data, frame.size) });
        let timestamp_ms = frame.timestamp;

        tracing::trace!(
            stream = ?frame.stream_id,
            size = frame.size,
            timestamp_ms,
            frame_type = ?frame.frame_type,
            "Bridge routing frame"
        );

        match frame.stream_id {
            StreamId::VideoMain => {
                self.enqueue_video(&self.main_stream, data, frame.frame_type, timestamp_ms);
            }
            StreamId::VideoSub => {
                self.enqueue_video(&self.sub_stream, data, frame.frame_type, timestamp_ms);
            }
            StreamId::Audio => {
                let frame_data = FrameData::Audio {
                    timestamp: timestamp_ms,
                    data: data.clone(),
                };
                // Audio goes to both streams.
                self.main_stream
                    .frame_queue
                    .push(frame_data.clone(), timestamp_ms, false);
                self.sub_stream
                    .frame_queue
                    .push(frame_data, timestamp_ms, false);
            }
        }
    }

    /// Route an owned frame through the streaming pipeline — zero-copy path.
    ///
    /// Unlike [`Self::route_frame`] which copies frame data into a new `BytesMut`,
    /// this method moves the existing `BytesMut` from the `OwnedFrame` directly
    /// into the streaming queue. This eliminates Copy 2 from the frame path.
    pub fn route_owned_frame(&self, frame: OwnedFrame) {
        let timestamp_ms = frame.timestamp;

        tracing::trace!(
            stream = ?frame.stream_id,
            size = frame.data.len(),
            timestamp_ms,
            frame_type = ?frame.frame_type,
            "Bridge routing owned frame (zero-copy)"
        );

        match frame.stream_id {
            StreamId::VideoMain => {
                self.enqueue_video(
                    &self.main_stream,
                    frame.data,
                    frame.frame_type,
                    timestamp_ms,
                );
            }
            StreamId::VideoSub => {
                self.enqueue_video(&self.sub_stream, frame.data, frame.frame_type, timestamp_ms);
            }
            StreamId::Audio => {
                let frame_data = FrameData::Audio {
                    timestamp: timestamp_ms,
                    data: frame.data.clone(),
                };
                // Audio goes to both streams.
                self.main_stream
                    .frame_queue
                    .push(frame_data.clone(), timestamp_ms, false);
                self.sub_stream
                    .frame_queue
                    .push(frame_data, timestamp_ms, false);
            }
        }
    }

    /// Enqueue a video frame into a stream's low-latency queue.
    fn enqueue_video(
        &self,
        stream: &StreamState,
        data: BytesMut,
        frame_type: FrameType,
        timestamp_ms: u32,
    ) {
        stream
            .last_timestamp_ms
            .store(timestamp_ms, portable_atomic::Ordering::Relaxed);

        tracing::trace!(
            size = data.len(),
            timestamp_ms,
            frame_type = ?frame_type,
            "Video frame enqueued"
        );

        let frame_data = FrameData::Video {
            timestamp: timestamp_ms,
            data,
        };
        stream.frame_queue.push(
            frame_data,
            timestamp_ms,
            frame_type == FrameType::VideoIFrame,
        );
    }

    /// Update SPS/PPS and bootstrap IDR in fanout context.
    ///
    /// This intentionally runs outside the frame callback thread to keep callback
    /// execution near-constant time.
    pub fn update_video_metadata(
        &self,
        stream_id: StreamId,
        timestamp_ms: u32,
        data: &BytesMut,
        is_video_idr: bool,
    ) {
        let stream = match stream_id {
            StreamId::VideoMain => &self.main_stream,
            StreamId::VideoSub => &self.sub_stream,
            StreamId::Audio => return,
        };

        stream
            .last_timestamp_ms
            .store(timestamp_ms, portable_atomic::Ordering::Relaxed);

        if is_video_idr {
            *stream.bootstrap_idr.write() = Some(data.to_vec());
        }

        let needs_parameter_sets = stream.sps.read().is_none() || stream.pps.read().is_none();
        if is_video_idr || needs_parameter_sets {
            self.extract_parameter_sets(stream_id, stream, data, is_video_idr);
        }
    }

    /// Scan Annex-B NAL units in an IDR frame to extract and cache SPS/PPS.
    fn extract_parameter_sets(
        &self,
        stream_id: StreamId,
        stream: &StreamState,
        data: &[u8],
        idr_hint: bool,
    ) {
        let mut found_sps = false;
        let mut found_pps = false;
        let mut found_idr_nal = false;

        for nal in AnnexBIterator::new(data) {
            if nal.is_empty() {
                continue;
            }
            let nal_type = nal[0] & 0x1F;
            tracing::trace!(nal_type, size = nal.len(), "Parsed NAL unit");
            match nal_type {
                7 => {
                    // SPS
                    tracing::debug!(size = nal.len(), "SPS extracted/updated");
                    *stream.sps.write() = Some(nal.to_vec());
                    found_sps = true;
                }
                8 => {
                    // PPS
                    tracing::debug!(size = nal.len(), "PPS extracted/updated");
                    *stream.pps.write() = Some(nal.to_vec());
                    found_pps = true;
                }
                5 => {
                    found_idr_nal = true;
                }
                _ => {}
            }
        }

        if !found_sps || !found_pps {
            let has_cached_sps = stream.sps.read().is_some();
            let has_cached_pps = stream.pps.read().is_some();

            if !has_cached_sps || !has_cached_pps {
                tracing::warn!(
                    stream = ?stream_id,
                    idr_hint,
                    found_idr_nal,
                    found_sps,
                    found_pps,
                    has_cached_sps,
                    has_cached_pps,
                    idr_size = data.len(),
                    "IDR frame missing SPS/PPS and cache incomplete"
                );
            } else {
                tracing::trace!(
                    stream = ?stream_id,
                    idr_hint,
                    found_idr_nal,
                    found_sps,
                    found_pps,
                    idr_size = data.len(),
                    "IDR frame missing inline SPS/PPS, using cached parameter sets"
                );
            }
        }
    }
}

impl FrameCallback for StreamingBridge {
    fn on_frame(&self, frame: &Frame) {
        self.route_frame(frame);
    }
}

impl crate::platform::frame::OwnedFrameCallback for StreamingBridge {
    fn on_owned_frame(&self, frame: OwnedFrame) {
        self.route_owned_frame(frame);
    }
}

/// Iterator over Annex-B delimited NAL units in a byte slice.
///
/// Finds `00 00 01` or `00 00 00 01` start codes and yields the data between them.
struct AnnexBIterator<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> AnnexBIterator<'a> {
    fn new(data: &'a [u8]) -> Self {
        let mut iter = Self { data, pos: 0 };
        // Skip to the first start code.
        iter.skip_to_first_start_code();
        iter
    }

    fn skip_to_first_start_code(&mut self) {
        while self.pos < self.data.len() {
            if let Some(sc_len) = self.start_code_at(self.pos) {
                self.pos += sc_len;
                return;
            }
            self.pos += 1;
        }
    }

    fn start_code_at(&self, pos: usize) -> Option<usize> {
        let remaining = &self.data[pos..];
        if remaining.len() >= 4
            && remaining[0] == 0
            && remaining[1] == 0
            && remaining[2] == 0
            && remaining[3] == 1
        {
            Some(4)
        } else if remaining.len() >= 3
            && remaining[0] == 0
            && remaining[1] == 0
            && remaining[2] == 1
        {
            Some(3)
        } else {
            None
        }
    }
}

impl<'a> Iterator for AnnexBIterator<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.data.len() {
            return None;
        }

        let start = self.pos;

        // Scan for the next start code.
        while self.pos < self.data.len() {
            if let Some(sc_len) = self.start_code_at(self.pos) {
                let nal = &self.data[start..self.pos];
                self.pos += sc_len;
                return Some(nal);
            }
            self.pos += 1;
        }

        // Last NAL unit (no trailing start code).
        let nal = &self.data[start..self.data.len()];
        Some(nal)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    fn make_frame(data: &[u8], frame_type: FrameType, stream_id: StreamId) -> Frame {
        Frame {
            data: data.as_ptr(),
            size: data.len(),
            timestamp: 2_000, // 2 seconds in milliseconds
            frame_type,
            stream_id,
        }
    }

    fn make_bridge() -> StreamingBridge {
        let bridge = StreamingBridge::new(
            LowLatencyFrameQueue::new("test-main", 8),
            LowLatencyFrameQueue::new("test-sub", 8),
            48_000,
        );
        // Tests must not inherit process-wide SPS/PPS cache from other cases.
        *bridge.main_stream.sps.write() = None;
        *bridge.main_stream.pps.write() = None;
        *bridge.main_stream.bootstrap_idr.write() = None;
        *bridge.sub_stream.sps.write() = None;
        *bridge.sub_stream.pps.write() = None;
        *bridge.sub_stream.bootstrap_idr.write() = None;
        bridge
    }

    #[test]
    fn test_bridge_routes_video_main_to_main_channel() {
        let bridge = make_bridge();

        let payload = vec![0x00, 0x00, 0x00, 0x01, 0x41, 0x9a, 0x24];
        let frame = make_frame(&payload, FrameType::VideoPFrame, StreamId::VideoMain);
        bridge.on_frame(&frame);

        assert!(bridge.main_stream.frame_queue.try_recv().is_some());
        assert!(bridge.sub_stream.frame_queue.try_recv().is_none());
    }

    #[test]
    fn test_bridge_routes_video_sub_to_sub_channel() {
        let bridge = make_bridge();

        let payload = vec![0x00, 0x00, 0x00, 0x01, 0x41, 0x9a, 0x24];
        let frame = make_frame(&payload, FrameType::VideoPFrame, StreamId::VideoSub);
        bridge.on_frame(&frame);

        assert!(bridge.main_stream.frame_queue.try_recv().is_none());
        assert!(bridge.sub_stream.frame_queue.try_recv().is_some());
    }

    #[test]
    fn test_bridge_routes_audio_to_both_channels() {
        let bridge = make_bridge();

        let payload = vec![0xFF, 0xF1, 0x50, 0x80]; // Fake AAC frame
        let frame = make_frame(&payload, FrameType::AudioPacket, StreamId::Audio);
        bridge.on_frame(&frame);

        assert!(bridge.main_stream.frame_queue.try_recv().is_some());
        assert!(bridge.sub_stream.frame_queue.try_recv().is_some());
    }

    #[test]
    fn test_bridge_extracts_sps_from_annex_b_idr() {
        let bridge = make_bridge();

        // SPS NAL (type 7) + PPS NAL (type 8) + IDR (type 5)
        let idr_frame = vec![
            0x00, 0x00, 0x00, 0x01, 0x67, 0x42, 0x00, 0x1e, // SPS
            0x00, 0x00, 0x00, 0x01, 0x68, 0xce, 0x06, 0xe2, // PPS
            0x00, 0x00, 0x00, 0x01, 0x65, 0x88, 0x84, // IDR slice
        ];
        bridge.update_video_metadata(
            StreamId::VideoMain,
            2000,
            &BytesMut::from(idr_frame.as_slice()),
            true,
        );

        let sps = bridge.main_stream.sps.read();
        assert!(sps.is_some());
        let sps = sps.as_ref().unwrap();
        assert_eq!(sps[0] & 0x1F, 7); // NAL type SPS
        assert_eq!(sps, &[0x67, 0x42, 0x00, 0x1e]);
    }

    #[test]
    fn test_bridge_extracts_pps_from_annex_b_idr() {
        let bridge = make_bridge();

        let idr_frame = vec![
            0x00, 0x00, 0x00, 0x01, 0x67, 0x42, 0x00, 0x1e, // SPS
            0x00, 0x00, 0x00, 0x01, 0x68, 0xce, 0x06, 0xe2, // PPS
            0x00, 0x00, 0x00, 0x01, 0x65, 0x88, 0x84, // IDR slice
        ];
        bridge.update_video_metadata(
            StreamId::VideoMain,
            2000,
            &BytesMut::from(idr_frame.as_slice()),
            true,
        );

        let pps = bridge.main_stream.pps.read();
        assert!(pps.is_some());
        assert_eq!(pps.as_ref().unwrap(), &[0x68, 0xce, 0x06, 0xe2]);
    }

    #[test]
    fn test_bridge_caches_bootstrap_idr() {
        let bridge = make_bridge();

        let idr_frame = vec![0x00, 0x00, 0x00, 0x01, 0x65, 0x88, 0x84, 0x21, 0xA0];
        bridge.update_video_metadata(
            StreamId::VideoMain,
            2000,
            &BytesMut::from(idr_frame.as_slice()),
            true,
        );

        let cached = bridge.main_stream.bootstrap_idr.read();
        assert!(cached.is_some());
        assert_eq!(cached.as_ref().unwrap(), &idr_frame);
    }

    #[test]
    fn test_bridge_timestamp_passthrough_ms() {
        let bridge = make_bridge();

        let payload = vec![0x00, 0x00, 0x00, 0x01, 0x41, 0x9a];
        let frame = Frame {
            data: payload.as_ptr(),
            size: payload.len(),
            timestamp: 3500, // 3500ms directly
            frame_type: FrameType::VideoPFrame,
            stream_id: StreamId::VideoMain,
        };
        bridge.on_frame(&frame);

        let received = bridge.main_stream.frame_queue.try_recv().unwrap().frame;
        match received {
            FrameData::Video { timestamp, .. } => {
                assert_eq!(timestamp, 3500); // ms passthrough, no conversion
            }
            _ => panic!("expected video frame"),
        }
    }

    #[test]
    fn test_bridge_iframe_produces_correct_frame_data() {
        let bridge = make_bridge();

        let idr_frame = vec![0x00, 0x00, 0x00, 0x01, 0x65, 0x88, 0x84];
        let frame = make_frame(&idr_frame, FrameType::VideoIFrame, StreamId::VideoMain);
        bridge.on_frame(&frame);

        let received = bridge.main_stream.frame_queue.try_recv().unwrap().frame;
        assert!(matches!(received, FrameData::Video { .. }));
    }

    #[test]
    fn test_bridge_pframe_produces_correct_frame_data() {
        let bridge = make_bridge();

        let p_frame = vec![0x00, 0x00, 0x00, 0x01, 0x41, 0x9a, 0x24];
        let frame = make_frame(&p_frame, FrameType::VideoPFrame, StreamId::VideoMain);
        bridge.on_frame(&frame);

        let received = bridge.main_stream.frame_queue.try_recv().unwrap().frame;
        assert!(matches!(received, FrameData::Video { .. }));
    }

    #[test]
    fn test_low_latency_queue_overflow_drops_oldest_non_idr() {
        let queue = LowLatencyFrameQueue::new("overflow", 2);
        queue.push(
            FrameData::Video {
                timestamp: 1,
                data: BytesMut::from(&[0x00, 0x00, 0x00, 0x01, 0x41][..]),
            },
            1,
            false,
        );
        queue.push(
            FrameData::Video {
                timestamp: 2,
                data: BytesMut::from(&[0x00, 0x00, 0x00, 0x01, 0x41][..]),
            },
            2,
            false,
        );
        queue.push(
            FrameData::Video {
                timestamp: 3,
                data: BytesMut::from(&[0x00, 0x00, 0x00, 0x01, 0x41][..]),
            },
            3,
            false,
        );

        let first = queue.try_recv().expect("expected first frame");
        let second = queue.try_recv().expect("expected second frame");
        assert_eq!(first.capture_timestamp_ms, 2);
        assert_eq!(second.capture_timestamp_ms, 3);
        assert!(queue.try_recv().is_none());
    }

    #[test]
    fn test_low_latency_queue_idr_flushes_backlog() {
        let queue = LowLatencyFrameQueue::new("idr-flush", 3);
        queue.push(
            FrameData::Video {
                timestamp: 1,
                data: BytesMut::from(&[0x41, 0x00][..]),
            },
            1,
            false,
        );
        queue.push(
            FrameData::Video {
                timestamp: 2,
                data: BytesMut::from(&[0x41, 0x01][..]),
            },
            2,
            false,
        );
        queue.push(
            FrameData::Video {
                timestamp: 3,
                data: BytesMut::from(&[0x41, 0x02][..]),
            },
            3,
            false,
        );
        queue.push(
            FrameData::Video {
                timestamp: 4,
                data: BytesMut::from(&[0x65, 0x88][..]),
            },
            4,
            true,
        );

        let first = queue.try_recv().expect("expected IDR frame");
        assert_eq!(first.capture_timestamp_ms, 4);
        assert!(first.is_video_idr);
        assert!(queue.try_recv().is_none());
    }

    #[test]
    fn test_annex_b_iterator_parses_three_code_start_codes() {
        let data = vec![
            0x00, 0x00, 0x01, 0xAA, 0xBB, // NAL 1
            0x00, 0x00, 0x01, 0xCC, // NAL 2
        ];
        let nals: Vec<&[u8]> = AnnexBIterator::new(&data).collect();
        assert_eq!(nals.len(), 2);
        assert_eq!(nals[0], &[0xAA, 0xBB]);
        assert_eq!(nals[1], &[0xCC]);
    }

    #[test]
    fn test_annex_b_iterator_parses_four_byte_start_codes() {
        let data = vec![
            0x00, 0x00, 0x00, 0x01, 0x67, 0x42, // SPS
            0x00, 0x00, 0x00, 0x01, 0x68, 0xCE, // PPS
        ];
        let nals: Vec<&[u8]> = AnnexBIterator::new(&data).collect();
        assert_eq!(nals.len(), 2);
        assert_eq!(nals[0], &[0x67, 0x42]);
        assert_eq!(nals[1], &[0x68, 0xCE]);
    }

    #[test]
    fn test_annex_b_iterator_empty_data() {
        let data: Vec<u8> = vec![];
        let nals: Vec<&[u8]> = AnnexBIterator::new(&data).collect();
        assert!(nals.is_empty());
    }

    // === BytesMutPool tests ===

    #[test]
    fn test_bytes_mut_pool_get_returns_buffer_with_capacity() {
        let pool = BytesMutPool::new(1024, 4);
        let buf = pool.get(512);
        assert!(buf.capacity() >= 512);
        assert!(buf.is_empty());
    }

    #[test]
    fn test_bytes_mut_pool_put_and_reuse() {
        let pool = BytesMutPool::new(1024, 4);

        // Get a buffer, write to it, return it
        let mut buf = pool.get(1024);
        buf.extend_from_slice(&[0xAA; 100]);
        pool.put(buf);

        assert_eq!(pool.available(), 1);

        // Get it back — should be cleared but reuse the allocation
        let reused = pool.get(512);
        assert!(reused.is_empty());
        assert!(reused.capacity() >= 1024);

        assert_eq!(pool.available(), 0);
    }

    #[test]
    fn test_bytes_mut_pool_drops_when_full() {
        let pool = BytesMutPool::new(64, 2);

        pool.put(BytesMut::with_capacity(64));
        pool.put(BytesMut::with_capacity(64));
        assert_eq!(pool.available(), 2);

        // Third put should be dropped (pool full)
        pool.put(BytesMut::with_capacity(64));
        assert_eq!(pool.available(), 2);
    }

    #[test]
    fn test_bytes_mut_pool_get_allocates_when_too_small() {
        let pool = BytesMutPool::new(64, 4);

        // Put a small buffer
        pool.put(BytesMut::with_capacity(32));
        assert_eq!(pool.available(), 1);

        // Request more than available — should allocate fresh
        let buf = pool.get(128);
        assert!(buf.capacity() >= 128);
        // Pool buffer was too small and was dropped
        assert_eq!(pool.available(), 0);
    }

    #[test]
    fn test_bytes_mut_pool_default_frame_pool() {
        let pool = BytesMutPool::default_frame_pool();
        let buf = pool.get(1024);
        // Default capacity is 64KB
        assert!(buf.capacity() >= 64 * 1024);
    }

    // === route_owned_frame tests ===

    #[test]
    fn test_bridge_route_owned_frame_main_stream() {
        let bridge = make_bridge();

        let data = BytesMut::from(&[0x00, 0x00, 0x00, 0x01, 0x41, 0x9a, 0x24][..]);
        let frame = OwnedFrame {
            data,
            timestamp: 2_000,
            frame_type: FrameType::VideoPFrame,
            stream_id: StreamId::VideoMain,
        };
        bridge.route_owned_frame(frame);

        assert!(bridge.main_stream.frame_queue.try_recv().is_some());
        assert!(bridge.sub_stream.frame_queue.try_recv().is_none());
    }

    #[test]
    fn test_bridge_route_owned_frame_sub_stream() {
        let bridge = make_bridge();

        let data = BytesMut::from(&[0x00, 0x00, 0x00, 0x01, 0x41, 0x9a, 0x24][..]);
        let frame = OwnedFrame {
            data,
            timestamp: 2_000,
            frame_type: FrameType::VideoPFrame,
            stream_id: StreamId::VideoSub,
        };
        bridge.route_owned_frame(frame);

        assert!(bridge.main_stream.frame_queue.try_recv().is_none());
        assert!(bridge.sub_stream.frame_queue.try_recv().is_some());
    }

    #[test]
    fn test_bridge_route_owned_frame_audio_to_both() {
        let bridge = make_bridge();

        let data = BytesMut::from(&[0xFF, 0xF1, 0x50, 0x80][..]);
        let frame = OwnedFrame {
            data,
            timestamp: 2_000,
            frame_type: FrameType::AudioPacket,
            stream_id: StreamId::Audio,
        };
        bridge.route_owned_frame(frame);

        assert!(bridge.main_stream.frame_queue.try_recv().is_some());
        assert!(bridge.sub_stream.frame_queue.try_recv().is_some());
    }

    #[test]
    fn test_bridge_route_owned_frame_timestamp_passthrough() {
        let bridge = make_bridge();

        let data = BytesMut::from(&[0x00, 0x00, 0x00, 0x01, 0x41, 0x9a][..]);
        let frame = OwnedFrame {
            data,
            timestamp: 3500, // 3500ms directly
            frame_type: FrameType::VideoPFrame,
            stream_id: StreamId::VideoMain,
        };
        bridge.route_owned_frame(frame);

        let received = bridge.main_stream.frame_queue.try_recv().unwrap().frame;
        match received {
            FrameData::Video { timestamp, .. } => {
                assert_eq!(timestamp, 3500); // ms passthrough, no conversion
            }
            _ => panic!("expected video frame"),
        }
    }

    #[test]
    fn test_bridge_owned_frame_callback_trait_impl() {
        use crate::platform::common::OwnedFrameCallback;

        let bridge = make_bridge();
        let data = BytesMut::from(&[0x00, 0x00, 0x00, 0x01, 0x41, 0x9a][..]);
        let frame = OwnedFrame {
            data,
            timestamp: 1_000,
            frame_type: FrameType::VideoPFrame,
            stream_id: StreamId::VideoMain,
        };

        bridge.on_owned_frame(frame);
        assert!(bridge.main_stream.frame_queue.try_recv().is_some());
    }
}
