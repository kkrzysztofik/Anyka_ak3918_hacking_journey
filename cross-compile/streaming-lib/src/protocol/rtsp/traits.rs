//! RTSP protocol traits for testability
//!
//! These traits provide abstractions for:
//! - RtpSender: RTP packet transmission
//! - FrameRouter: Frame routing to subscribers
//! - StreamRegistry: Publisher/subscriber registration tracking
//!
//! Each trait uses `#[automock]` to generate mock implementations
//! for unit tests.

use async_trait::async_trait;
use mockall::automock;

use crate::rtsp::rtp::RtpPacket;
use crate::rtsp::rtp::rtcp::rtcp_sr::RtcpSenderReport;
use crate::streamhub::define::{PublisherInfo, SubscriberInfo};
use crate::streamhub::errors::StreamHubError;
use crate::streamhub::stream::StreamIdentifier;
use crate::streamhub::utils::Uuid;

/// Errors that can occur during RTP operations
#[derive(Debug, thiserror::Error)]
pub enum RtpError {
    #[error("send failed: {0}")]
    SendFailed(String),

    #[error("connection closed")]
    ConnectionClosed,

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Trait for RTP packet transmission
///
/// This trait abstracts the RTP sending logic, allowing for
/// different implementations (UDP, TCP, etc.) and easy mocking
/// in tests.
#[automock]
#[async_trait]
pub trait RtpSender: Send + Sync {
    /// Send an RTP packet to the client
    ///
    /// # Arguments
    ///
    /// * `packet` - The RTP packet to send
    ///
    /// # Errors
    ///
    /// Returns `RtpError` if the send operation fails
    async fn send_packet(&self, packet: &RtpPacket) -> Result<(), RtpError>;

    /// Send an RTCP Sender Report to the client
    ///
    /// # Arguments
    ///
    /// * `report` - The RTCP SR to send
    ///
    /// # Errors
    ///
    /// Returns `RtpError` if the send operation fails
    async fn send_rtcp_sr(&self, report: &RtcpSenderReport) -> Result<(), RtpError>;
}

/// Trait for frame routing to subscribers
///
/// This trait abstracts the logic for routing video/audio frames
/// to subscribed clients.
#[automock]
pub trait FrameRouter: Send + Sync {
    /// Route a frame to all subscribed clients
    ///
    /// # Arguments
    ///
    /// * `stream_id` - The stream identifier
    /// * `frame` - The frame data to route
    fn route_frame(&self, stream_id: StreamIdentifier, frame: FrameData);
}

/// Frame data for routing
#[derive(Debug, Clone)]
pub enum FrameData {
    /// Video frame data
    Video {
        data: bytes::Bytes,
        timestamp: u32,
        is_keyframe: bool,
    },
    /// Audio frame data
    Audio { data: bytes::Bytes, timestamp: u32 },
}

/// Trait for stream registry operations
///
/// This trait tracks publishers and subscribers in the streaming system,
/// providing a clean interface for registration and unregistration.
#[automock]
pub trait StreamRegistry: Send + Sync {
    /// Register a publisher in the system
    ///
    /// # Arguments
    ///
    /// * `info` - Publisher information
    ///
    /// # Errors
    ///
    /// Returns the publisher UUID on success, or `StreamHubError` on failure
    fn register_publisher(&self, info: PublisherInfo) -> Result<Uuid, StreamHubError>;

    /// Register a subscriber in the system
    ///
    /// # Arguments
    ///
    /// * `info` - Subscriber information
    ///
    /// # Errors
    ///
    /// Returns the subscriber UUID on success, or `StreamHubError` on failure
    fn register_subscriber(&self, info: SubscriberInfo) -> Result<Uuid, StreamHubError>;

    /// Unregister a publisher or subscriber
    ///
    /// # Arguments
    ///
    /// * `id` - The UUID of the entity to unregister
    fn unregister(&self, id: Uuid);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rtsp::rtp::RtpPacket;
    use crate::rtsp::rtp::rtcp::rtcp_sr::RtcpSenderReport;
    use crate::rtsp::rtp::rtp_header::RtpHeader;
    use crate::streamhub::define::{PubDataType, PublishType, SubDataType, SubscribeType};
    use crate::streamhub::utils::RandomDigitCount;
    use bytes::BytesMut;

    // ========== RtpSender Tests ==========

    #[tokio::test]
    async fn test_mock_rtp_sender_send_packet() {
        let mut mock = MockRtpSender::new();

        let header = RtpHeader::default();
        let packet = RtpPacket::new(header);

        mock.expect_send_packet().times(1).returning(|_| Ok(()));

        let result = mock.send_packet(&packet).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_rtp_sender_send_packet_error() {
        let mut mock = MockRtpSender::new();

        let header = RtpHeader::default();
        let packet = RtpPacket::new(header);

        mock.expect_send_packet()
            .times(1)
            .returning(|_| Err(RtpError::ConnectionClosed));

        let result = mock.send_packet(&packet).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mock_rtp_sender_send_rtcp_sr() {
        let mut mock = MockRtpSender::new();

        let report = RtcpSenderReport::default();

        mock.expect_send_rtcp_sr().times(1).returning(|_| Ok(()));

        let result = mock.send_rtcp_sr(&report).await;
        assert!(result.is_ok());
    }

    // ========== FrameRouter Tests ==========

    #[test]
    fn test_mock_frame_router_route_video() {
        let mut mock = MockFrameRouter::new();

        let stream_id = StreamIdentifier::Rtsp {
            stream_path: "test_stream".to_string(),
        };
        let frame = FrameData::Video {
            data: BytesMut::from(&b"test video data"[..]).freeze(),
            timestamp: 1000,
            is_keyframe: true,
        };

        mock.expect_route_frame().times(1).returning(|_, _| {});

        mock.route_frame(stream_id, frame);
    }

    #[test]
    fn test_mock_frame_router_route_audio() {
        let mut mock = MockFrameRouter::new();

        let stream_id = StreamIdentifier::Rtsp {
            stream_path: "test_stream".to_string(),
        };
        let frame = FrameData::Audio {
            data: BytesMut::from(&b"test audio data"[..]).freeze(),
            timestamp: 1000,
        };

        mock.expect_route_frame().times(1).returning(|_, _| {});

        mock.route_frame(stream_id, frame);
    }

    // ========== StreamRegistry Tests ==========

    #[test]
    fn test_mock_stream_registry_register_publisher() {
        let mut mock = MockStreamRegistry::new();

        mock.expect_register_publisher()
            .times(1)
            .returning(|_| Ok(Uuid::new(RandomDigitCount::Zero)));

        let info = PublisherInfo {
            id: Uuid::new(RandomDigitCount::Zero),
            pub_type: PublishType::RtspPush,
            pub_data_type: PubDataType::Frame,
            notify_info: crate::streamhub::define::NotifyInfo {
                request_url: "/test".to_string(),
                remote_addr: "127.0.0.1:8080".to_string(),
            },
        };
        let result = mock.register_publisher(info);
        assert!(result.is_ok());
    }

    #[test]
    fn test_mock_stream_registry_register_subscriber() {
        let mut mock = MockStreamRegistry::new();

        mock.expect_register_subscriber()
            .times(1)
            .returning(|_| Ok(Uuid::new(RandomDigitCount::Zero)));

        let info = SubscriberInfo {
            id: Uuid::new(RandomDigitCount::Zero),
            sub_type: SubscribeType::RtspPull,
            notify_info: crate::streamhub::define::NotifyInfo {
                request_url: "/test".to_string(),
                remote_addr: "127.0.0.1:8080".to_string(),
            },
            sub_data_type: SubDataType::Frame,
        };
        let result = mock.register_subscriber(info);
        assert!(result.is_ok());
    }

    #[test]
    fn test_mock_stream_registry_unregister() {
        let mut mock = MockStreamRegistry::new();

        mock.expect_unregister().times(1).returning(|_| {});

        mock.unregister(Uuid::new(RandomDigitCount::Zero));
    }

    // ========== FrameData Tests ==========

    #[test]
    fn test_frame_data_video_debug() {
        let frame = FrameData::Video {
            data: BytesMut::from(&b"test"[..]).freeze(),
            timestamp: 1000,
            is_keyframe: true,
        };
        let debug = format!("{:?}", frame);
        assert!(debug.contains("Video"));
    }

    #[test]
    fn test_frame_data_audio_debug() {
        let frame = FrameData::Audio {
            data: BytesMut::from(&b"test"[..]).freeze(),
            timestamp: 1000,
        };
        let debug = format!("{:?}", frame);
        assert!(debug.contains("Audio"));
    }

    // ========== RtpError Tests ==========

    #[test]
    fn test_rtp_error_display() {
        let err = RtpError::SendFailed("test error".to_string());
        assert_eq!(format!("{}", err), "send failed: test error");

        let err = RtpError::ConnectionClosed;
        assert_eq!(format!("{}", err), "connection closed");
    }
}
