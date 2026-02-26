//! streaming-lib: Streaming library for RTSP and HTTP-FLV protocols
//!
//! This library is forked from xiu (<https://github.com/harlanc/xiu>) and provides
//! minimal components needed for RTSP and HTTP-FLV streaming on Anyka AK3918 hardware.
//!
//! # Components
//!
//! - **rtsp**: RTSP server implementation
//! - **httpflv**: HTTP-FLV server implementation
//! - **codec**: H.264 codec handling
//! - **container**: FLV container format support
//! - **streamhub**: Stream management and routing
//! - **bytesio**: Binary I/O utilities
//! - **common**: Common utilities and helpers

// Module declarations
pub mod bytesio;
pub mod codec;
pub mod common;
pub mod container;
pub mod httpflv;
mod logging_flags;
pub mod rtsp;
pub mod streamhub;

// Re-export key types from RTSP
pub use logging_flags::{set_stream_frame_debug_logging, stream_frame_debug_logging_enabled};
pub use rtsp::session::client_session::RtspClientSession;
pub use rtsp::session::server_session::RtspServerSession;
pub use rtsp::{DefaultRtspServer, RtspServer};

/// Stream session type alias for ticket-specified API surface
/// Represents either an RTSP client or server session
pub type StreamSession = RtspServerSession;

// Re-export key types from HTTP-FLV
pub use httpflv::server::{DefaultHttpFlvServer, HttpFlvServer};

// Re-export key types from streamhub
pub use streamhub::StreamsHub;
pub use streamhub::define::{
    DataReceiver, DataSender, FrameData, MediaInfo, PacketData, PublishType, PublisherInfo,
    StreamHubEvent, StreamHubEventSender, SubscribeType, SubscriberInfo, TStreamHandler,
    VideoCodecType,
};
pub use streamhub::stream::StreamIdentifier;

// Re-export key types from codec
pub use codec::sps::Sps;

// Re-export key types from container
pub use container::demuxer::FlvDemuxer;
pub use container::muxer::FlvMuxer;

// Frame delivery interface (for future integration with platform layer)
/// Trait for frame sources that can register callbacks
pub trait FrameSource: Send + Sync {
    /// Register a frame callback
    fn register_callback(&self, callback: Box<dyn FrameCallback>);
}

/// Trait for receiving frame callbacks
pub trait FrameCallback: Send + Sync {
    /// Called when a new frame is available
    ///
    /// # Safety
    ///
    /// The frame data pointer is valid only during this call.
    /// Do not store the pointer. Use pre-allocated buffers for packetization.
    fn on_frame(&self, frame: &Frame);
}

/// A video or audio frame from the encoder
pub struct Frame {
    /// Read-only pointer to frame data (zero-copy)
    pub data: *const u8,
    /// Size of frame data in bytes
    pub size: usize,
    /// Timestamp in milliseconds (SDK source)
    pub timestamp: u32,
    /// Type of frame
    pub frame_type: FrameType,
}

// SAFETY: `Frame` holds a read-only pointer to immutable frame data that remains valid
// for `size` bytes during its use across threads. The data is not mutated or freed
// while shared, so it is safe to mark `Frame` as Send + Sync.
unsafe impl Send for Frame {}
unsafe impl Sync for Frame {}

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
        let data = [0u8; 10];
        let frame = Frame {
            data: data.as_ptr(),
            size: data.len(),
            timestamp: 12345,
            frame_type: FrameType::VideoIFrame,
        };
        assert_eq!(frame.size, 10);
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
