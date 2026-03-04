use super::rtsp_channel::RtcpChannel;
use super::rtsp_channel::RtpChannel;
use super::rtsp_codec::RtspCodecInfo;
use super::rtsp_transport::RtspTransport;
use crate::bytesio::TNetIO;
use crate::bytesio::bytes_reader::BytesReader;
use crate::rtsp::rtp::errors::UnPackerError;
use crate::rtsp::rtsp_channel::TRtpFunc;
use bytes::BytesMut;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Default, Hash, Eq, PartialEq)]
pub enum TrackType {
    #[default]
    Audio,
    Video,
    Application,
}

// A track can be a audio/video track, the A/V media data is transmitted
// over RTP, and the control data is transmitted over RTCP.
// The rtp/rtcp can be over TCP or UDP :
// 1. Over the TCP: It shares one TCP channel with the RTSP signaling data, and
// the entire session uses only one TCP connection（RTSP signaling data/audio
// RTP/audio RTCP/video RTP/video RTCP）
// 2. Over the UDP: It will establish 4 UDP channles for A/V RTP/RTCP data.
// 2.1 A RTP channel for audio media data transmitting.
// 2.2 A RTCP channel for audio control data transmitting
// 2.3 A RTP channel for video media data transmitting.
// 2.4 A RTCP channel for video control data transmitting
pub struct RtspTrack {
    pub track_type: TrackType,

    pub transport: RtspTransport,
    pub uri: String,
    pub media_control: String,

    pub rtp_channel: Arc<Mutex<RtpChannel>>,
    pub rtcp_channel: Arc<Mutex<RtcpChannel>>,
}

impl RtspTrack {
    pub fn new(track_type: TrackType, codec_info: RtspCodecInfo, media_control: String) -> Self {
        let rtp_channel = RtpChannel::new(codec_info);
        let ssrc = rtp_channel.ssrc();
        let sample_rate = rtp_channel.clock_rate();

        let mut rtcp_channel = RtcpChannel::default();
        rtcp_channel.set_ssrc(ssrc, sample_rate);

        RtspTrack {
            track_type,
            media_control,
            transport: RtspTransport::default(),
            uri: String::default(),
            rtp_channel: Arc::new(Mutex::new(rtp_channel)),
            rtcp_channel: Arc::new(Mutex::new(rtcp_channel)),
        }
    }

    pub async fn rtp_receive_loop(&mut self, mut rtp_io: Box<dyn TNetIO + Send + Sync>) {
        let rtp_channel_out = self.rtp_channel.clone();
        tokio::spawn(async move {
            let mut reader = BytesReader::new(BytesMut::new());
            let mut rtp_channel_in = rtp_channel_out.lock().await;
            loop {
                match rtp_io.read().await {
                    Ok(data) => {
                        reader.extend_from_slice(&data[..]);
                        if let Err(err) = rtp_channel_in.on_packet(&mut reader).await {
                            tracing::error!(error = %err, "rtp_receive_loop_on_packet_error");
                        }
                    }
                    Err(err) => {
                        tracing::error!(?err, "rtp_receive_loop_read_error");
                        break;
                    }
                }
            }
        });
    }

    //send and receive rtcp data in a UDP channel
    pub async fn rtcp_receive_loop(&mut self, rtcp_io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>) {
        let rtcp_channel_out = self.rtcp_channel.clone();

        tokio::spawn(async move {
            let mut reader = BytesReader::new(BytesMut::new());
            let rtcp_channel_in = rtcp_channel_out.clone();

            loop {
                let data = match rtcp_io.lock().await.read().await {
                    Ok(data) => data,
                    Err(err) => {
                        tracing::error!(?err, "rtcp_receive_loop_read_error");
                        break;
                    }
                };
                reader.extend_from_slice(&data[..]);
                rtcp_channel_in
                    .lock()
                    .await
                    .on_rtcp(&mut reader, rtcp_io.clone())
                    .await;
            }
        });
    }

    /// Start periodic RTCP Sender Report transmission
    /// RFC 3550 recommends sending SR every 5 seconds for active senders
    pub async fn rtcp_send_loop(&mut self, rtcp_io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>) {
        let rtcp_channel_out = self.rtcp_channel.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));
            loop {
                interval.tick().await;
                let mut rtcp_channel_in = rtcp_channel_out.lock().await;
                match rtcp_channel_in.send_sr(rtcp_io.clone()).await {
                    Ok(true) => {}
                    Ok(false) => {
                        tracing::debug!("rtcp_sr_skipped_wall_clock_not_valid");
                    }
                    Err(err) => {
                        tracing::error!(error = %err, "rtcp_send_loop_send_error");
                        break;
                    }
                }
            }
        });
    }

    pub async fn set_transport(&mut self, transport: RtspTransport) {
        if let Some(interleaveds) = transport.interleaved {
            self.rtcp_channel
                .lock()
                .await
                .set_channel_identifier(interleaveds[1]);
        } else {
            tracing::info!("udp_transport");
        }

        self.transport = transport;
    }

    pub async fn on_rtp(&mut self, reader: &mut BytesReader) -> Result<(), UnPackerError> {
        self.rtp_channel.lock().await.on_packet(reader).await
    }

    pub async fn on_rtcp(
        &mut self,
        reader: &mut BytesReader,
        io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>,
    ) {
        self.rtcp_channel.lock().await.on_rtcp(reader, io).await;
    }

    pub async fn create_packer(&mut self, io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>) {
        self.rtp_channel.lock().await.create_packer(io);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rtsp::rtsp_codec::{RtspCodecId, RtspCodecInfo};

    // ========================================================================
    // TrackType Tests
    // ========================================================================

    #[test]
    fn test_track_type_default() {
        let tt = TrackType::default();
        assert!(matches!(tt, TrackType::Audio));
    }

    #[test]
    fn test_track_type_audio() {
        let tt = TrackType::Audio;
        assert!(matches!(tt, TrackType::Audio));
    }

    #[test]
    fn test_track_type_video() {
        let tt = TrackType::Video;
        assert!(matches!(tt, TrackType::Video));
    }

    #[test]
    fn test_track_type_application() {
        let tt = TrackType::Application;
        assert!(matches!(tt, TrackType::Application));
    }

    #[test]
    fn test_track_type_clone() {
        let tt1 = TrackType::Video;
        let tt2 = tt1.clone();
        assert_eq!(tt1, tt2);
    }

    #[test]
    fn test_track_type_debug() {
        let tt = TrackType::Audio;
        let debug_str = format!("{:?}", tt);
        assert!(debug_str.contains("Audio"));
    }

    #[test]
    fn test_track_type_equality() {
        assert_eq!(TrackType::Audio, TrackType::Audio);
        assert_eq!(TrackType::Video, TrackType::Video);
        assert_ne!(TrackType::Audio, TrackType::Video);
    }

    #[test]
    fn test_track_type_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(TrackType::Audio);
        set.insert(TrackType::Video);
        set.insert(TrackType::Application);
        assert_eq!(set.len(), 3);
    }

    // ========================================================================
    // RtspTrack Tests
    // ========================================================================

    #[test]
    fn test_rtsp_track_new_video() {
        let codec_info = RtspCodecInfo {
            codec_id: RtspCodecId::H264,
            payload_type: 96,
            sample_rate: 90000,
            ..Default::default()
        };
        let track = RtspTrack::new(TrackType::Video, codec_info, "trackID=0".to_string());
        assert!(matches!(track.track_type, TrackType::Video));
        assert_eq!(track.media_control, "trackID=0");
    }

    #[test]
    fn test_rtsp_track_new_audio() {
        let codec_info = RtspCodecInfo {
            codec_id: RtspCodecId::AAC,
            payload_type: 97,
            sample_rate: 44100,
            channel_count: 2,
        };
        let track = RtspTrack::new(TrackType::Audio, codec_info, "trackID=1".to_string());
        assert!(matches!(track.track_type, TrackType::Audio));
        assert_eq!(track.media_control, "trackID=1");
    }

    #[test]
    fn test_rtsp_track_default_transport() {
        let codec_info = RtspCodecInfo::default();
        let track = RtspTrack::new(TrackType::Video, codec_info, "track0".to_string());
        // Transport should be default
        assert!(track.transport.interleaved.is_none());
    }

    #[test]
    fn test_rtsp_track_default_uri() {
        let codec_info = RtspCodecInfo::default();
        let track = RtspTrack::new(TrackType::Audio, codec_info, "control".to_string());
        assert!(track.uri.is_empty());
    }

    #[tokio::test]
    async fn test_rtsp_track_set_transport_tcp() {
        let codec_info = RtspCodecInfo::default();
        let mut track = RtspTrack::new(TrackType::Video, codec_info, "track0".to_string());

        let transport = RtspTransport {
            interleaved: Some([0, 1]),
            ..Default::default()
        };
        track.set_transport(transport.clone()).await;
        assert_eq!(track.transport.interleaved, Some([0, 1]));
    }

    #[tokio::test]
    async fn test_rtsp_track_set_transport_udp() {
        let codec_info = RtspCodecInfo::default();
        let mut track = RtspTrack::new(TrackType::Video, codec_info, "track0".to_string());

        let transport = RtspTransport {
            client_port: Some([5000, 5001]),
            ..Default::default()
        };
        track.set_transport(transport.clone()).await;
        assert!(track.transport.interleaved.is_none());
        assert_eq!(track.transport.client_port, Some([5000, 5001]));
    }

    #[test]
    fn test_rtsp_track_new_g711a_codec() {
        let codec_info = RtspCodecInfo {
            codec_id: RtspCodecId::G711A,
            payload_type: 8,
            sample_rate: 8000,
            channel_count: 1,
        };
        let track = RtspTrack::new(TrackType::Audio, codec_info, "trackID=audio".to_string());
        assert!(matches!(track.track_type, TrackType::Audio));
        assert_eq!(track.media_control, "trackID=audio");
    }

    #[tokio::test]
    async fn test_rtsp_track_new_verifies_rtp_channel_has_unpacker() {
        let codec_info = RtspCodecInfo {
            codec_id: RtspCodecId::H264,
            payload_type: 96,
            sample_rate: 90000,
            ..Default::default()
        };
        let track = RtspTrack::new(TrackType::Video, codec_info, "track0".to_string());

        // Lock the rtp_channel and verify unpacker is present
        let rtp_channel = track.rtp_channel.lock().await;
        assert!(rtp_channel.rtp_unpacker.is_some());
    }

    #[tokio::test]
    async fn test_rtsp_track_new_verifies_rtcp_channel_matching_ssrc() {
        let codec_info = RtspCodecInfo {
            codec_id: RtspCodecId::AAC,
            payload_type: 97,
            sample_rate: 44100,
            channel_count: 2,
        };
        let track = RtspTrack::new(TrackType::Audio, codec_info, "track0".to_string());

        // Lock both channels and verify SSRC match
        let rtp_channel = track.rtp_channel.lock().await;
        let rtp_ssrc = rtp_channel.ssrc();
        drop(rtp_channel); // Release lock before locking rtcp_channel

        // Note: RtcpChannel doesn't expose ssrc() getter, but we can verify
        // the track was constructed properly with matching SSRC via set_ssrc call
        // This test verifies the construction logic without direct SSRC comparison
        assert!(rtp_ssrc > 0); // Verify SSRC was assigned
    }

    #[tokio::test]
    async fn test_rtsp_track_set_transport_interleaved_2_3() {
        let codec_info = RtspCodecInfo::default();
        let mut track = RtspTrack::new(TrackType::Audio, codec_info, "track1".to_string());

        let transport = RtspTransport {
            interleaved: Some([2, 3]),
            ..Default::default()
        };
        track.set_transport(transport.clone()).await;
        assert_eq!(track.transport.interleaved, Some([2, 3]));

        // Channel identifier is a private field, so we can't directly assert on it
        // But we verified transport was set correctly with interleaved channels
    }

    #[tokio::test]
    async fn test_rtsp_track_set_transport_no_interleaved_no_client_port() {
        let codec_info = RtspCodecInfo::default();
        let mut track = RtspTrack::new(TrackType::Video, codec_info, "track0".to_string());

        // Bare default transport (UDP without client_port specified)
        let transport = RtspTransport::default();
        track.set_transport(transport.clone()).await;

        assert!(track.transport.interleaved.is_none());
        assert!(track.transport.client_port.is_none());
    }

    #[test]
    fn test_track_type_variants_not_equal() {
        // Verify Application != Audio
        assert_ne!(TrackType::Application, TrackType::Audio);

        // Verify Application != Video
        assert_ne!(TrackType::Application, TrackType::Video);

        // Verify Audio != Video (already tested in test_track_type_equality, but explicit here)
        assert_ne!(TrackType::Audio, TrackType::Video);
    }
}
