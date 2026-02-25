//! Frame delivery infrastructure for video and audio streams.
//!
//! This module provides zero-copy frame delivery from the Anyka SDK to multiple
//! consumers (RTSP, HTTP-FLV, recording) with reference-counted lifetime management.
//!
//! # Architecture
//!
//! ```text
//! SDK callback → Frame (zero-copy ptr) → FrameCallback consumers
//!                                          ├── RTSP packetizer
//!                                          └── HTTP-FLV muxer
//!
//! FrameHandle (ref-counted) ensures SDK buffer stays valid until
//! all async consumers finish processing.
//! ```
//!
//! # Safety
//!
//! - `Frame` holds a read-only pointer into SDK-owned memory
//! - `FrameHandle` provides Arc-based reference counting for extended lifetimes
//! - `ActiveFrames` tracks all outstanding references for proper cleanup

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use parking_lot::Mutex;

use bytes::BytesMut;

/// Unique identifier for a registered callback.
pub type CallbackId = u64;

/// Type of encoded frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameType {
    /// H.264/H.265 IDR (Instantaneous Decoder Refresh) frame.
    VideoIFrame,
    /// H.264/H.265 predicted frame.
    VideoPFrame,
    /// H.264/H.265 bi-directional predicted frame.
    VideoBFrame,
    /// Predicted Intra frame (partial refresh). Treated as P-frame for streaming
    /// purposes but carries intra-coded macroblocks for gradual decoder refresh.
    VideoPiFrame,
    /// Audio packet (G.711, AAC, etc.).
    AudioPacket,
}

/// Identifies which stream produced the frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StreamId {
    /// Main video stream (1920x1080).
    VideoMain,
    /// Sub video stream (1280x720).
    VideoSub,
    /// Audio stream.
    Audio,
}

/// Metadata parsed from the 28-byte IPC frame header.
///
/// This is extracted during `recv_frame_response` and used to populate
/// `OwnedFrame` fields and manage the remote token for frame release.
#[derive(Debug, Clone)]
pub struct FrameMetadata {
    /// Timestamp in milliseconds (from daemon).
    pub timestamp_ms: u64,
    /// Sequence number.
    pub seq_no: u32,
    /// Frame type (I-frame, P-frame, etc.).
    pub frame_type: FrameType,
    /// Opaque token the daemon uses to identify this frame on release.
    pub remote_token: u64,
}

/// Frame with owned data buffer — no copy needed downstream.
///
/// Unlike [`Frame`] which holds a borrowed raw pointer into SDK memory,
/// `OwnedFrame` owns its data via `BytesMut`. This enables zero-extra-copy
/// delivery through the streaming pipeline: the frame data is read from the
/// IPC socket directly into `BytesMut`, which is then moved (not copied)
/// through the callback chain to the streaming bridge.
///
/// # Usage
///
/// ```text
/// vendor_ipc::fetch_frame_owned()
///   → OwnedFrame { data: BytesMut, ... }
///     → OwnedFrameCallback::on_owned_frame()
///       → StreamingBridge::route_owned_frame()  // moves data, no copy
/// ```
pub struct OwnedFrame {
    /// Owned encoded frame data (already in BytesMut).
    pub data: BytesMut,
    /// Timestamp in microseconds since epoch.
    pub timestamp: u64,
    /// Type of frame (I-frame, P-frame, etc.).
    pub frame_type: FrameType,
    /// Which stream this frame belongs to.
    pub stream_id: StreamId,
}

/// Extended callback trait that receives owned frames.
///
/// This is the zero-copy counterpart to [`FrameCallback`]. Implementations
/// receive an `OwnedFrame` with `BytesMut` data that can be moved directly
/// into the streaming pipeline without additional copies.
///
/// Callbacks must be `Send + Sync` and should complete quickly (target < 2ms).
pub trait OwnedFrameCallback: Send + Sync {
    /// Called when a new owned frame is available.
    ///
    /// The `frame` is moved to the callback, transferring ownership of the
    /// `BytesMut` buffer. The callback may consume, store, or forward the
    /// frame data without any copy.
    fn on_owned_frame(&self, frame: OwnedFrame);
}

/// A single encoded frame delivered from the SDK.
///
/// Contains a zero-copy read-only pointer into SDK-managed memory.
/// The pointer is only valid for the duration of the callback invocation
/// unless the consumer acquires a [`FrameHandle`] for extended lifetime.
///
/// # Safety
///
/// - `data` points into SDK-owned buffer memory
/// - Consumers must NOT write to `data`
/// - Consumers must NOT use `data` after the callback returns unless
///   they hold a `FrameHandle`
pub struct Frame {
    /// Read-only pointer to encoded frame data.
    pub data: *const u8,
    /// Size of the frame data in bytes.
    pub size: usize,
    /// Timestamp in microseconds since epoch.
    pub timestamp: u64,
    /// Type of frame (I-frame, P-frame, etc.).
    pub frame_type: FrameType,
    /// Which stream this frame belongs to.
    pub stream_id: StreamId,
}

// SAFETY: Frame contains a raw pointer but is only used as a read-only
// reference during callback invocation. The pointer is never written to
// and lifetime is managed by ActiveFrames.
unsafe impl Send for Frame {}
unsafe impl Sync for Frame {}

/// Trait for receiving frame callbacks.
///
/// Implementations must be `Send + Sync` as callbacks may be invoked from
/// any thread. Callbacks should complete quickly (target < 2ms) to avoid
/// blocking the encoder pipeline. Long-running operations should be
/// dispatched to an async task after acquiring a [`FrameHandle`].
pub trait FrameCallback: Send + Sync {
    /// Called when a new frame is available.
    ///
    /// The `frame` reference is only valid for the duration of this call.
    /// To retain the frame data, acquire a `FrameHandle` from the
    /// `ActiveFrames` manager.
    fn on_frame(&self, frame: &Frame);
}

/// Reference-counted handle extending an SDK frame's lifetime.
///
/// When all `FrameHandle` instances for a given buffer are dropped,
/// the SDK buffer is eligible for release. This enables zero-copy
/// async processing where multiple consumers can independently
/// finish at different times.
pub struct FrameHandle {
    /// Pointer to the SDK buffer (for identification).
    buffer_key: usize,
    /// Shared reference count for this buffer.
    ref_count: Arc<AtomicUsize>,
    /// Back-reference to the ActiveFrames manager for cleanup.
    active_frames: Arc<ActiveFrames>,
}

impl Clone for FrameHandle {
    fn clone(&self) -> Self {
        self.ref_count.fetch_add(1, Ordering::Acquire);
        Self {
            buffer_key: self.buffer_key,
            ref_count: Arc::clone(&self.ref_count),
            active_frames: Arc::clone(&self.active_frames),
        }
    }
}

impl Drop for FrameHandle {
    fn drop(&mut self) {
        if self.ref_count.fetch_sub(1, Ordering::Release) == 1 {
            // Last reference — remove from active set.
            // The actual SDK buffer release would be called here once
            // the ak_venc_release_frame() FFI binding is available.
            std::sync::atomic::fence(Ordering::Acquire);
            self.active_frames.remove_frame(self.buffer_key);
        }
    }
}

/// Tracks all outstanding frame references for proper SDK buffer lifecycle.
///
/// When the SDK delivers a frame, it is registered here. Consumers that need
/// the frame beyond the callback scope acquire a `FrameHandle`. When all
/// handles are dropped, the frame is removed from tracking and the SDK
/// buffer can be released.
pub struct ActiveFrames {
    frames: Mutex<HashMap<usize, Arc<AtomicUsize>>>,
}

impl ActiveFrames {
    /// Create a new `ActiveFrames` tracker.
    pub fn new() -> Self {
        Self {
            frames: Mutex::new(HashMap::new()),
        }
    }

    /// Register a new SDK frame buffer for tracking.
    ///
    /// Called when the SDK delivers a frame. Returns the initial ref count Arc
    /// which starts at 1 (the delivery context holds the first reference).
    pub fn register_frame(&self, buffer: *const u8) -> Arc<AtomicUsize> {
        let key = buffer as usize;
        let mut frames = self.frames.lock();
        if let Some(existing) = frames.get(&key) {
            return Arc::clone(existing);
        }

        let ref_count = Arc::new(AtomicUsize::new(1));
        frames.insert(key, Arc::clone(&ref_count));
        ref_count
    }

    /// Acquire a reference to an active frame buffer.
    ///
    /// Returns a `FrameHandle` that increments the reference count.
    /// Returns `None` if the buffer is not currently tracked (already released).
    pub fn acquire_ref(self: &Arc<Self>, buffer: *const u8) -> Option<FrameHandle> {
        let key = buffer as usize;
        let guard = self.frames.lock();
        guard.get(&key).map(|ref_count| {
            ref_count.fetch_add(1, Ordering::Acquire);
            FrameHandle {
                buffer_key: key,
                ref_count: Arc::clone(ref_count),
                active_frames: Arc::clone(self),
            }
        })
    }

    /// Release the registration-time reference for a frame buffer.
    ///
    /// The delivery path calls `register_frame()` for each incoming SDK buffer.
    /// Once callback fanout is complete, this function should be called to drop
    /// that registration-time reference. If no consumer handles are alive, the
    /// frame entry is removed immediately.
    pub fn release_frame(&self, buffer: *const u8) {
        let key = buffer as usize;

        let should_remove = {
            let guard = self.frames.lock();
            if let Some(ref_count) = guard.get(&key) {
                ref_count.fetch_sub(1, Ordering::Release) == 1
            } else {
                false
            }
        };

        if should_remove {
            std::sync::atomic::fence(Ordering::Acquire);
            self.remove_frame(key);
        }
    }

    /// Remove a frame from tracking (called when ref count reaches zero).
    fn remove_frame(&self, key: usize) {
        self.frames.lock().remove(&key);
    }

    /// Get the number of currently tracked frames (for diagnostics).
    #[cfg(test)]
    pub fn active_count(&self) -> usize {
        self.frames.lock().len()
    }
}

impl Default for ActiveFrames {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_type_equality() {
        assert_eq!(FrameType::VideoIFrame, FrameType::VideoIFrame);
        assert_ne!(FrameType::VideoIFrame, FrameType::VideoPFrame);
    }

    #[test]
    fn test_stream_id_equality() {
        assert_eq!(StreamId::VideoMain, StreamId::VideoMain);
        assert_ne!(StreamId::VideoMain, StreamId::VideoSub);
    }

    #[test]
    fn test_active_frames_register_and_count() {
        let af = ActiveFrames::new();
        let buf1 = 0x1000 as *const u8;
        let buf2 = 0x2000 as *const u8;

        let _rc1 = af.register_frame(buf1);
        assert_eq!(af.active_count(), 1);

        let _rc2 = af.register_frame(buf2);
        assert_eq!(af.active_count(), 2);
    }

    #[test]
    fn test_register_frame_existing_buffer_returns_existing_ref_count() {
        let af = ActiveFrames::new();
        let buf = 0x1000 as *const u8;

        let rc1 = af.register_frame(buf);
        let rc2 = af.register_frame(buf);

        assert!(Arc::ptr_eq(&rc1, &rc2));
        assert_eq!(af.active_count(), 1);
        assert_eq!(rc1.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_active_frames_acquire_ref_success() {
        let af = Arc::new(ActiveFrames::new());
        let buf = 0x1000 as *const u8;

        let _rc = af.register_frame(buf);
        let handle = af.acquire_ref(buf);
        assert!(handle.is_some());
    }

    #[test]
    fn test_active_frames_acquire_ref_unknown_buffer() {
        let af = Arc::new(ActiveFrames::new());
        let buf = 0x1000 as *const u8;

        let handle = af.acquire_ref(buf);
        assert!(handle.is_none());
    }

    #[test]
    fn test_frame_handle_clone_increments_ref_count() {
        let af = Arc::new(ActiveFrames::new());
        let buf = 0x1000 as *const u8;
        let rc = af.register_frame(buf);

        // rc starts at 1 from register_frame
        let handle = af.acquire_ref(buf).unwrap();
        // acquire_ref adds 1, so rc = 2
        assert_eq!(rc.load(Ordering::SeqCst), 2);

        let _handle2 = handle.clone();
        // clone adds 1, so rc = 3
        assert_eq!(rc.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn test_frame_handle_drop_decrements_ref_count() {
        let af = Arc::new(ActiveFrames::new());
        let buf = 0x1000 as *const u8;
        let rc = af.register_frame(buf);

        {
            let handle = af.acquire_ref(buf).unwrap();
            assert_eq!(rc.load(Ordering::SeqCst), 2);

            let _handle2 = handle.clone();
            assert_eq!(rc.load(Ordering::SeqCst), 3);
            // _handle2 dropped here → rc = 2
        }
        // handle dropped here → rc = 1 (register_frame reference still held)
        assert_eq!(rc.load(Ordering::SeqCst), 1);
        assert_eq!(af.active_count(), 1);
    }

    #[test]
    fn test_full_lifecycle_frame_removed() {
        let af = Arc::new(ActiveFrames::new());
        let buf = 0x1000 as *const u8;
        let _rc = af.register_frame(buf);
        assert_eq!(af.active_count(), 1);

        {
            let handle = af.acquire_ref(buf).unwrap();
            drop(handle);
        }

        // Registration reference is still held by ActiveFrames map.
        assert_eq!(af.active_count(), 1);

        // Explicitly release registration-time reference.
        af.release_frame(buf);
        assert_eq!(af.active_count(), 0);
    }

    #[test]
    fn test_frame_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Frame>();
    }

    #[test]
    fn test_frame_construction() {
        let data = [0u8; 64];
        let frame = Frame {
            data: data.as_ptr(),
            size: 64,
            timestamp: 1_000_000,
            frame_type: FrameType::VideoIFrame,
            stream_id: StreamId::VideoMain,
        };
        assert_eq!(frame.size, 64);
        assert_eq!(frame.timestamp, 1_000_000);
        assert_eq!(frame.frame_type, FrameType::VideoIFrame);
        assert_eq!(frame.stream_id, StreamId::VideoMain);
    }

    #[test]
    fn test_active_frames_default() {
        let af = ActiveFrames::default();
        assert_eq!(af.active_count(), 0);
    }

    #[test]
    fn test_owned_frame_construction() {
        let data = BytesMut::from(&[0x00, 0x00, 0x00, 0x01, 0x65, 0x88][..]);
        let frame = OwnedFrame {
            data,
            timestamp: 2_000_000,
            frame_type: FrameType::VideoIFrame,
            stream_id: StreamId::VideoMain,
        };
        assert_eq!(frame.data.len(), 6);
        assert_eq!(frame.timestamp, 2_000_000);
        assert_eq!(frame.frame_type, FrameType::VideoIFrame);
        assert_eq!(frame.stream_id, StreamId::VideoMain);
    }

    #[test]
    fn test_owned_frame_data_ownership_transfer() {
        let original_data = vec![0xAA, 0xBB, 0xCC, 0xDD];
        let data = BytesMut::from(original_data.as_slice());
        let frame = OwnedFrame {
            data,
            timestamp: 1_000_000,
            frame_type: FrameType::VideoPFrame,
            stream_id: StreamId::VideoSub,
        };
        // Data is owned by frame — can be moved without copy
        let moved_data = frame.data;
        assert_eq!(&moved_data[..], &[0xAA, 0xBB, 0xCC, 0xDD]);
    }

    #[test]
    fn test_frame_metadata_construction() {
        let meta = FrameMetadata {
            timestamp_ms: 12345,
            seq_no: 42,
            frame_type: FrameType::VideoIFrame,
            remote_token: 0xDEAD_BEEF,
        };
        assert_eq!(meta.timestamp_ms, 12345);
        assert_eq!(meta.seq_no, 42);
        assert_eq!(meta.frame_type, FrameType::VideoIFrame);
        assert_eq!(meta.remote_token, 0xDEAD_BEEF);
    }
}
