//! Frame delivery infrastructure for video and audio streams.
//!
//! This module provides zero-copy frame delivery from the Anyka SDK to multiple
//! consumers (RTSP, HTTP-FLV, recording).
//!
//! # Architecture
//!
//! ```text
//! SDK callback → OwnedFrame (moved BytesMut) → OwnedFrameCallback consumers
//!                                                ├── RTSP packetizer
//!                                                └── HTTP-FLV muxer
//! ```

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

/// Metadata associated with an encoded frame delivered through IPC.
///
/// In push mode this metadata comes from the shared-memory slot header.
#[derive(Debug, Clone)]
pub struct FrameMetadata {
    /// Timestamp in milliseconds (SDK source, u32 wraps at ~49.7 days).
    pub timestamp_ms: u32,
    /// Sequence number.
    pub seq_no: u32,
    /// Frame type (I-frame, P-frame, etc.).
    pub frame_type: FrameType,
    /// Opaque token the daemon uses to identify this frame on release.
    pub remote_token: u64,
    /// Which stream this frame belongs to.
    pub stream_id: StreamId,
}

/// Frame with owned data buffer — no copy needed downstream.
///
/// `OwnedFrame` owns its data via `BytesMut`. This enables zero-extra-copy
/// delivery through the streaming pipeline: the frame data is read from the
/// IPC socket directly into `BytesMut`, which is then moved (not copied)
/// through the callback chain to the streaming bridge.
///
/// # Usage
///
/// ```text
/// ipc::recv_pushed_frame()
///   → OwnedFrame { data: BytesMut, ... }
///     → OwnedFrameCallback::on_owned_frame()
///       → StreamingBridge::route_owned_frame()  // moves data, no copy
/// ```
pub struct OwnedFrame {
    /// Owned encoded frame data (already in BytesMut).
    pub data: BytesMut,
    /// Timestamp in milliseconds (SDK source).
    pub timestamp: u32,
    /// Type of frame (I-frame, P-frame, etc.).
    pub frame_type: FrameType,
    /// Which stream this frame belongs to.
    pub stream_id: StreamId,
}

/// Callback trait for receiving encoded frames from the platform.
///
/// Implementations receive an `OwnedFrame` with `BytesMut` data that can be
/// moved directly into the streaming pipeline without additional copies.
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
    fn test_owned_frame_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<OwnedFrame>();
    }

    #[test]
    fn test_owned_frame_construction() {
        let data = BytesMut::from(&[0x00, 0x00, 0x00, 0x01, 0x65, 0x88][..]);
        let frame = OwnedFrame {
            data,
            timestamp: 2_000,
            frame_type: FrameType::VideoIFrame,
            stream_id: StreamId::VideoMain,
        };
        assert_eq!(frame.data.len(), 6);
        assert_eq!(frame.timestamp, 2_000);
        assert_eq!(frame.frame_type, FrameType::VideoIFrame);
        assert_eq!(frame.stream_id, StreamId::VideoMain);
    }

    #[test]
    fn test_owned_frame_data_ownership_transfer() {
        let original_data = vec![0xAA, 0xBB, 0xCC, 0xDD];
        let data = BytesMut::from(original_data.as_slice());
        let frame = OwnedFrame {
            data,
            timestamp: 1_000,
            frame_type: FrameType::VideoPFrame,
            stream_id: StreamId::VideoSub,
        };
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
            stream_id: StreamId::VideoMain,
        };
        assert_eq!(meta.timestamp_ms, 12345);
        assert_eq!(meta.seq_no, 42);
        assert_eq!(meta.frame_type, FrameType::VideoIFrame);
        assert_eq!(meta.remote_token, 0xDEAD_BEEF);
        assert_eq!(meta.stream_id, StreamId::VideoMain);
    }
}
