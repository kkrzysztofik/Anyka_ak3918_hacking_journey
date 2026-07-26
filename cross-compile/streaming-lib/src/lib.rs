//! streaming-lib: Streaming library for RTSP and HTTP-FLV protocols
//!
//! This library is forked from xiu (<https://github.com/harlanc/xiu>) and provides
//! minimal components needed for RTSP and HTTP-FLV streaming on Anyka AK3918 hardware.
//!
//! # Components
//!
//! - **protocol/rtsp**: RTSP server implementation
//! - **protocol/httpflv**: HTTP-FLV server implementation
//! - **codec/h264**: H.264 codec handling
//! - **container/flv**: FLV container format support
//! - **hub**: Stream management and routing
//! - **io**: Binary I/O utilities
//! - **common**: Common utilities and helpers

// Module declarations
pub mod codec;
pub mod common;
pub mod config;
pub mod container;
pub mod hub;
pub mod io;
mod logging_flags;
pub mod protocol;
pub mod service;
pub mod validation;

// Re-export Bytes for use in Frame
pub use bytes::Bytes;

// Re-export key types from RTSP
pub use logging_flags::{set_stream_frame_debug_logging, stream_frame_debug_logging_enabled};
pub use protocol::rtsp::DefaultRtspServer;
pub use protocol::rtsp::session::server_session::RtspServerSession;

/// Stream session type alias for ticket-specified API surface
/// Represents either an RTSP client or server session
pub type StreamSession = RtspServerSession;

// Re-export key types from HTTP-FLV
pub use protocol::httpflv::server::DefaultHttpFlvServer;

// Re-export key types from streamhub (hub)
pub use hub::StreamsHub;
pub use hub::define::{
    DataReceiver, DataSender, FRAME_DATA_CHANNEL_CAPACITY, FrameData, MediaInfo, PacketData,
    PublishType, PublisherInfo, StreamHubEvent, StreamHubEventSender, SubscribeType,
    SubscriberInfo, TStreamHandler, VideoCodecType, frame_data_channel,
};
pub use hub::stream::StreamIdentifier;

// Re-export key types from codec
pub use codec::h264::sps::Sps;

// Re-export key types from container
pub use container::demuxer::FlvDemuxer;
pub use container::muxer::FlvMuxer;

// ============================================================
// Backward Compatibility Aliases (DEPRECATED)
// ============================================================
// These aliases provide backward compatibility for code that
// imports from the old `streaming_lib::streamhub` path.
// All code should migrate to use `streaming_lib::hub` instead.
// ============================================================

#[allow(clippy::mixed_attributes_style)]
#[deprecated(since = "0.2.0", note = "Use hub module instead")]
/// Deprecated: Use `hub` module instead
pub mod streamhub {
    //! Deprecated: Re-exports from hub module for backward compatibility.
    //! Use `streaming_lib::hub` instead.

    pub use crate::hub::define::*;
    pub use crate::hub::errors::*;
    pub use crate::hub::mock_audio_publisher::*;
    pub use crate::hub::mock_publisher::*;
    pub use crate::hub::statistics::*;
    pub use crate::hub::stream::*;
    pub use crate::hub::*;
}

/// A video or audio frame from the encoder
///
/// Uses `bytes::Bytes` for safe, reference-counted data that can be
/// safely shared across threads without lifetime issues.
pub struct Frame {
    /// Reference-counted frame data (zero-copy, safe to share)
    pub data: Bytes,
    /// Timestamp in milliseconds (SDK source)
    pub timestamp: u32,
    /// Type of frame
    pub frame_type: FrameType,
}

/// Frame type for video and audio
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameType {
    /// H.264 I-frame (keyframe)
    VideoIFrame,
    /// H.264 P-frame (predicted)
    VideoPFrame,
    /// H.264 B-frame (bidirectional)
    VideoBFrame,
    /// Audio packet (AAC)
    AudioPacket,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========== FrameType Tests ==========

    #[test]
    fn test_frame_type_debug() {
        assert_eq!(format!("{:?}", FrameType::VideoIFrame), "VideoIFrame");
        assert_eq!(format!("{:?}", FrameType::VideoPFrame), "VideoPFrame");
        assert_eq!(format!("{:?}", FrameType::VideoBFrame), "VideoBFrame");
        assert_eq!(format!("{:?}", FrameType::AudioPacket), "AudioPacket");
    }

    #[test]
    fn test_frame_type_clone() {
        let ft = FrameType::VideoIFrame;
        let ft2 = ft;
        assert_eq!(ft, ft2);
    }

    #[test]
    fn test_frame_type_equality() {
        assert_eq!(FrameType::VideoIFrame, FrameType::VideoIFrame);
        assert_ne!(FrameType::VideoIFrame, FrameType::VideoPFrame);
        assert_ne!(FrameType::AudioPacket, FrameType::VideoBFrame);
    }

    // ========== Frame Tests ==========

    #[test]
    fn test_frame_construction() {
        let data = Bytes::from_static(b"test frame data");
        let frame = Frame {
            data: data.clone(),
            timestamp: 12345,
            frame_type: FrameType::VideoIFrame,
        };
        assert_eq!(frame.data.len(), 15);
        assert_eq!(frame.timestamp, 12345);
        assert_eq!(frame.frame_type, FrameType::VideoIFrame);
    }

    #[test]
    fn test_frame_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        assert_send::<Frame>();
        assert_sync::<Frame>();
    }

    #[test]
    fn test_frame_bytes_safe_to_share() {
        // Verify Bytes provides safe sharing
        let data = Bytes::from_static(b"shared data");

        // Frame can be safely cloned because Bytes uses reference counting
        let frame1 = Frame {
            data: data.clone(),
            timestamp: 1000,
            frame_type: FrameType::VideoIFrame,
        };

        let frame2 = Frame {
            data,
            timestamp: 2000,
            frame_type: FrameType::VideoPFrame,
        };

        // Both frames can access their data independently
        assert_eq!(frame1.data.len(), 11);
        assert_eq!(frame2.data.len(), 11);
    }

    // ========== Re-export Smoke Tests ==========

    #[test]
    fn test_logging_flags_reexport() {
        // Just verify the re-exports compile and are accessible
        set_stream_frame_debug_logging(false);
        let _ = stream_frame_debug_logging_enabled();
    }

    #[test]
    fn test_video_codec_type_reexport() {
        // Verify VideoCodecType re-export is accessible
        let _ = VideoCodecType::H264;
    }
}
