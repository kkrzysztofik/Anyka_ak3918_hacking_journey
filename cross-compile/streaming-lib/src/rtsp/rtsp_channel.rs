use crate::rtsp::rtp::RtpPacket;
use crate::rtsp::rtp::errors::PackerError;
use crate::rtsp::rtp::errors::UnPackerError;
use crate::rtsp::rtp::rtcp::RTCP_RR;
use crate::rtsp::rtp::rtcp::RTCP_SR;
use crate::rtsp::rtp::rtcp::rtcp_header::RtcpHeader;
use crate::rtsp::rtp::utils::OnFrameFn;
use crate::rtsp::rtp::utils::OnRtpPacketFn;
use crate::rtsp::rtp::utils::OnRtpPacketFn2;

use rand::RngExt;

use super::rtp::rtp_aac::RtpAacPacker;
use super::rtp::rtp_h264::RtpH264Packer;
use super::rtp::rtp_h265::RtpH265Packer;

use super::rtp::rtp_aac::RtpAacUnPacker;
use super::rtp::rtp_h264::RtpH264UnPacker;
use super::rtp::rtp_h265::RtpH265UnPacker;

use super::rtp::rtcp::rtcp_context::RtcpContext;
use super::rtp::rtcp::rtcp_sr::RtcpSenderReport;
use super::rtp::utils::TPacker;
use super::rtp::utils::TUnPacker;
use super::rtsp_codec::RtspCodecId;
use super::rtsp_codec::RtspCodecInfo;
use crate::bytesio::TNetIO;
use crate::bytesio::bytes_errors::BytesWriteError;
use crate::bytesio::bytes_reader::BytesReader;
use crate::bytesio::bytes_writer::AsyncBytesWriter;
use crate::rtsp::rtp::utils::Marshal;
use crate::rtsp::rtp::utils::Unmarshal;
use byteorder::BigEndian;
use bytes::BytesMut;
use std::sync::Arc;
use tokio::sync::Mutex;

const DEFAULT_MAX_RTP_PAYLOAD_SIZE: usize = 1400;

pub trait TRtpFunc {
    fn create_packer(&mut self, writer: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>);
    fn create_unpacker(&mut self);
}

pub struct RtpChannel {
    codec_info: RtspCodecInfo,
    pub rtp_packer: Option<Box<dyn TPacker>>,
    pub rtp_unpacker: Option<Box<dyn TUnPacker>>,
    ssrc: u32,
    init_sequence: u16,
    init_timestamp: u32,
}

#[derive(Default)]
pub struct RtcpChannel {
    recv_ctx: RtcpContext,
    pub send_ctx: RtcpContext,
    channel_identifier: u8,
}

impl RtpChannel {
    pub fn new(codec_info: RtspCodecInfo) -> Self {
        let ssrc: u32 = rand::rng().random();
        // Use a non-zero random initial RTP sequence number to avoid ambiguous
        // PLAY/RTP-Info startup behavior in strict clients.
        let init_sequence: u16 = loop {
            let sequence: u16 = rand::rng().random();
            if sequence != 0 {
                break sequence;
            }
        };
        // Random initial RTP timestamp per RFC 3550 §5.1
        let init_timestamp: u32 = rand::rng().random();
        let mut rtp_channel = RtpChannel {
            codec_info,
            ssrc,
            rtp_packer: None,
            rtp_unpacker: None,
            init_sequence,
            init_timestamp,
        };
        rtp_channel.create_unpacker();
        rtp_channel
    }

    pub fn clock_rate(&self) -> u32 {
        self.codec_info.sample_rate
    }

    /// Initial RTP sequence number for this channel (used in RTP-Info header on PLAY).
    pub fn initial_sequence(&self) -> u16 {
        self.init_sequence
    }

    /// Initial RTP timestamp for this channel (used in RTP-Info header on PLAY).
    /// Per RFC 3550 §5.1, this SHOULD be random to avoid known-plaintext attacks.
    /// Returns the SSRC assigned to this RTP channel.
    pub fn ssrc(&self) -> u32 {
        self.ssrc
    }

    /// Regenerate the SSRC with a new random value.
    /// Used to resolve SSRC collisions within a session (RFC 3550 §8.2).
    pub fn regenerate_ssrc(&mut self) {
        self.ssrc = rand::rng().random();
    }

    pub fn initial_timestamp(&self) -> u32 {
        self.init_timestamp
    }

    //Receive av frame from network -> pack AV frame to RTP packet -> send to stream hub
    pub async fn on_packet(&mut self, reader: &mut BytesReader) -> Result<(), UnPackerError> {
        if let Some(unpacker) = &mut self.rtp_unpacker {
            unpacker.unpack(reader).await?;
        }
        Ok(())
    }

    //Receive av frame from stream hub -> pack -> send out
    pub async fn on_frame(
        &mut self,
        nalus: &mut BytesMut,
        timestamp: u32,
    ) -> Result<(), PackerError> {
        if let Some(packer) = &mut self.rtp_packer {
            return packer.pack(nalus, timestamp).await;
        }
        Ok(())
    }

    //Set handler for processing AV frame when unpack a whole AV frame
    //from rtp packets received from network.
    pub fn on_frame_handler(&mut self, f: OnFrameFn) {
        if let Some(unpacker) = &mut self.rtp_unpacker {
            unpacker.on_frame_handler(f);
        }
    }

    //Set handler for processing rtp packet when packed a rtp packet
    pub fn on_packet_handler(&mut self, f: OnRtpPacketFn) {
        if let Some(packer) = &mut self.rtp_packer {
            packer.on_packet_handler(f);
        }
    }

    //Set handler for processing received AV rtp packet from network
    pub fn on_packet_for_rtcp_handler(&mut self, f: OnRtpPacketFn2) {
        if let Some(packer) = &mut self.rtp_packer {
            packer.on_packet_for_rtcp_handler(f);
        }
    }
}

impl TRtpFunc for RtpChannel {
    fn create_unpacker(&mut self) {
        match self.codec_info.codec_id {
            RtspCodecId::H264 => {
                self.rtp_unpacker = Some(Box::new(RtpH264UnPacker::new()));
            }
            RtspCodecId::H265 => {
                self.rtp_unpacker = Some(Box::new(RtpH265UnPacker::new()));
            }
            RtspCodecId::AAC => {
                self.rtp_unpacker = Some(Box::new(RtpAacUnPacker::new()));
            }
            RtspCodecId::G711A => {
                // TODO: implement G711A decoding/encoding and add tests
            }
        }
    }
    fn create_packer(&mut self, io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>) {
        match self.codec_info.codec_id {
            RtspCodecId::H264 => {
                self.rtp_packer = Some(Box::new(RtpH264Packer::new(
                    self.codec_info.payload_type,
                    self.ssrc,
                    self.init_sequence,
                    DEFAULT_MAX_RTP_PAYLOAD_SIZE,
                    io,
                )));
            }
            RtspCodecId::H265 => {
                self.rtp_packer = Some(Box::new(RtpH265Packer::new(
                    self.codec_info.payload_type,
                    self.ssrc,
                    self.init_sequence,
                    DEFAULT_MAX_RTP_PAYLOAD_SIZE,
                    io,
                )));
            }
            RtspCodecId::AAC => {
                self.rtp_packer = Some(Box::new(RtpAacPacker::new(
                    self.codec_info.payload_type,
                    self.ssrc,
                    self.init_sequence,
                    io,
                )));
            }
            RtspCodecId::G711A => {
                // TODO: implement G711A decoding/encoding and add tests
            }
        }
    }
}

impl RtcpChannel {
    /// Set the SSRC and sample rate for the send context so RTCP SR packets
    /// carry the same SSRC as the corresponding RTP stream (RFC 3550 §6.4.1)
    /// and can correctly extrapolate the RTP timestamp for NTP/RTP correlation.
    pub fn set_ssrc(&mut self, ssrc: u32, sample_rate: u32) {
        self.send_ctx = RtcpContext::new(ssrc, 0, sample_rate);
    }

    pub fn set_channel_identifier(&mut self, channel_identifier: u8) {
        self.channel_identifier = channel_identifier;
    }

    pub async fn on_rtcp(
        &mut self,
        reader: &mut BytesReader,
        rtcp_io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>,
    ) {
        let mut reader_clone = BytesReader::new(reader.get_remaining_bytes());
        if let Ok(rtcp_header) = RtcpHeader::unmarshal(&mut reader_clone) {
            match rtcp_header.payload_type {
                RTCP_SR => {
                    if let Ok(sr) = RtcpSenderReport::unmarshal(reader) {
                        self.recv_ctx.received_sr(&sr);
                        if let Err(err) = self.send_rr(rtcp_io).await {
                            log::error!("send rr error: {}", err);
                        }
                    }
                }
                RTCP_RR => {}
                _ => {}
            }
        }
    }

    pub fn on_packet(&mut self, packet: RtpPacket) {
        self.recv_ctx.received_rtp(packet.clone());
        self.send_ctx.send_rtp(packet);
    }

    pub async fn send_sr(
        &mut self,
        rtcp_io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>,
    ) -> Result<bool, BytesWriteError> {
        let Some(sr) = self.send_ctx.generate_sr() else {
            return Ok(false);
        };

        let net_type = rtcp_io.lock().await.get_net_type();
        if let Ok(msg) = sr.marshal() {
            let mut bytes_writer = AsyncBytesWriter::new(rtcp_io);
            match net_type {
                crate::bytesio::NetType::TCP => {
                    bytes_writer.write_u8(0x24)?;
                    bytes_writer.write_u8(self.channel_identifier)?;
                    bytes_writer.write_u16::<BigEndian>(msg.len() as u16)?;
                }
                crate::bytesio::NetType::UDP => {}
            }
            bytes_writer.write(&msg)?;
            bytes_writer.flush().await?;
        }
        Ok(true)
    }

    pub async fn send_rr(
        &mut self,
        rtcp_io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>,
    ) -> Result<(), BytesWriteError> {
        let rr = self.recv_ctx.generate_rr();

        let net_type = rtcp_io.lock().await.get_net_type();
        if let Ok(msg) = rr.marshal() {
            let mut bytes_writer = AsyncBytesWriter::new(rtcp_io);
            match net_type {
                crate::bytesio::NetType::TCP => {
                    bytes_writer.write_u8(0x24)?;
                    bytes_writer.write_u8(self.channel_identifier)?;
                    bytes_writer.write_u16::<BigEndian>(msg.len() as u16)?;
                }
                crate::bytesio::NetType::UDP => {}
            }
            bytes_writer.write(&msg)?;
            bytes_writer.flush().await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rtsp::rtsp_codec::RtspCodecId;

    // ========================================================================
    // RtpChannel Tests
    // ========================================================================

    #[test]
    fn test_rtp_channel_new_h264() {
        let codec_info = RtspCodecInfo {
            codec_id: RtspCodecId::H264,
            payload_type: 96,
            sample_rate: 90000,
            ..Default::default()
        };
        let channel = RtpChannel::new(codec_info);
        assert!(channel.rtp_unpacker.is_some());
        assert!(channel.rtp_packer.is_none());
    }

    #[test]
    fn test_rtp_channel_new_h265() {
        let codec_info = RtspCodecInfo {
            codec_id: RtspCodecId::H265,
            payload_type: 98,
            sample_rate: 90000,
            ..Default::default()
        };
        let channel = RtpChannel::new(codec_info);
        assert!(channel.rtp_unpacker.is_some());
        assert!(channel.rtp_packer.is_none());
    }

    #[test]
    fn test_rtp_channel_new_aac() {
        let codec_info = RtspCodecInfo {
            codec_id: RtspCodecId::AAC,
            payload_type: 97,
            sample_rate: 44100,
            channel_count: 2,
        };
        let channel = RtpChannel::new(codec_info);
        assert!(channel.rtp_unpacker.is_some());
        assert!(channel.rtp_packer.is_none());
    }

    #[test]
    fn test_rtp_channel_new_g711a() {
        let codec_info = RtspCodecInfo {
            codec_id: RtspCodecId::G711A,
            payload_type: 8,
            sample_rate: 8000,
            ..Default::default()
        };
        let channel = RtpChannel::new(codec_info);
        // G711A doesn't have an unpacker implemented
        assert!(channel.rtp_unpacker.is_none());
        assert!(channel.rtp_packer.is_none());
    }

    #[test]
    fn test_rtp_channel_ssrc_is_random() {
        let codec_info = RtspCodecInfo::default();
        let channel1 = RtpChannel::new(codec_info.clone());
        let channel2 = RtpChannel::new(codec_info);
        // SSRCs should be different (random)
        assert_ne!(channel1.ssrc, channel2.ssrc);
    }

    #[test]
    fn test_rtp_channel_new_initial_sequence_non_zero() {
        let codec_info = RtspCodecInfo::default();
        let channel = RtpChannel::new(codec_info);
        assert_ne!(channel.initial_sequence(), 0);
    }

    // ========================================================================
    // RtcpChannel Tests
    // ========================================================================

    #[test]
    fn test_rtcp_channel_default() {
        let channel = RtcpChannel::default();
        assert_eq!(channel.channel_identifier, 0);
    }

    #[test]
    fn test_rtcp_channel_set_channel_identifier() {
        let mut channel = RtcpChannel::default();
        channel.set_channel_identifier(3);
        assert_eq!(channel.channel_identifier, 3);
    }

    #[test]
    fn test_rtcp_channel_set_channel_identifier_max() {
        let mut channel = RtcpChannel::default();
        channel.set_channel_identifier(255);
        assert_eq!(channel.channel_identifier, 255);
    }

    #[test]
    fn test_rtcp_channel_set_ssrc() {
        let mut channel = RtcpChannel::default();
        channel.set_ssrc(0x12345678, 90000);
        let sr = channel.send_ctx.generate_sr().expect("SR should be generated");
        assert_eq!(sr.ssrc, 0x12345678);
    }

    #[test]
    fn test_rtcp_channel_set_ssrc_default_is_zero() {
        let mut channel = RtcpChannel::default();
        let sr = channel.send_ctx.generate_sr().expect("SR should be generated");
        assert_eq!(sr.ssrc, 0);
    }

    #[test]
    fn test_rtcp_channel_on_packet() {
        let mut channel = RtcpChannel::default();
        let packet = RtpPacket::default();
        // Should not panic
        channel.on_packet(packet);
    }

    // ========================================================================
    // TRtpFunc Trait Tests
    // ========================================================================

    #[test]
    fn test_rtp_channel_initial_sequence() {
        let codec_info = RtspCodecInfo {
            codec_id: RtspCodecId::H264,
            payload_type: 96,
            sample_rate: 90000,
            ..Default::default()
        };
        let channel = RtpChannel {
            codec_info,
            rtp_packer: None,
            rtp_unpacker: None,
            ssrc: 0,
            init_sequence: 42,
            init_timestamp: 1000,
        };
        assert_eq!(channel.initial_sequence(), 42);
        assert_eq!(channel.initial_timestamp(), 1000);
    }

    #[test]
    fn test_create_unpacker_h264() {
        let codec_info = RtspCodecInfo {
            codec_id: RtspCodecId::H264,
            payload_type: 96,
            sample_rate: 90000,
            ..Default::default()
        };
        let mut channel = RtpChannel {
            codec_info,
            rtp_packer: None,
            rtp_unpacker: None,
            ssrc: 0,
            init_sequence: 0,
            init_timestamp: 0,
        };
        channel.create_unpacker();
        assert!(channel.rtp_unpacker.is_some());
    }

    #[test]
    fn test_create_unpacker_h265() {
        let codec_info = RtspCodecInfo {
            codec_id: RtspCodecId::H265,
            payload_type: 98,
            sample_rate: 90000,
            ..Default::default()
        };
        let mut channel = RtpChannel {
            codec_info,
            rtp_packer: None,
            rtp_unpacker: None,
            ssrc: 0,
            init_sequence: 0,
            init_timestamp: 0,
        };
        channel.create_unpacker();
        assert!(channel.rtp_unpacker.is_some());
    }

    #[test]
    fn test_create_unpacker_aac() {
        let codec_info = RtspCodecInfo {
            codec_id: RtspCodecId::AAC,
            payload_type: 97,
            sample_rate: 44100,
            channel_count: 2,
        };
        let mut channel = RtpChannel {
            codec_info,
            rtp_packer: None,
            rtp_unpacker: None,
            ssrc: 0,
            init_sequence: 0,
            init_timestamp: 0,
        };
        channel.create_unpacker();
        assert!(channel.rtp_unpacker.is_some());
    }

    // ========================================================================
    // Additional Coverage Tests
    // ========================================================================

    #[test]
    fn test_clock_rate_h264_returns_sample_rate() {
        let codec_info = RtspCodecInfo {
            codec_id: RtspCodecId::H264,
            payload_type: 96,
            sample_rate: 90000,
            ..Default::default()
        };
        let channel = RtpChannel::new(codec_info);
        assert_eq!(channel.clock_rate(), 90000);
    }

    #[test]
    fn test_clock_rate_aac_returns_sample_rate() {
        let codec_info = RtspCodecInfo {
            codec_id: RtspCodecId::AAC,
            payload_type: 97,
            sample_rate: 44100,
            channel_count: 2,
        };
        let channel = RtpChannel::new(codec_info);
        assert_eq!(channel.clock_rate(), 44100);
    }

    #[test]
    fn test_clock_rate_g711a_returns_sample_rate() {
        let codec_info = RtspCodecInfo {
            codec_id: RtspCodecId::G711A,
            payload_type: 8,
            sample_rate: 8000,
            ..Default::default()
        };
        let channel = RtpChannel::new(codec_info);
        assert_eq!(channel.clock_rate(), 8000);
    }

    #[test]
    fn test_regenerate_ssrc_changes_value() {
        let codec_info = RtspCodecInfo::default();
        let mut channel = RtpChannel::new(codec_info);
        let original_ssrc = channel.ssrc();
        channel.regenerate_ssrc();
        let new_ssrc = channel.ssrc();
        // With very high probability, the new SSRC should be different
        // (probability of collision is 1 in 2^32)
        assert_ne!(original_ssrc, new_ssrc);
    }

    #[test]
    fn test_initial_timestamp_returns_init_value() {
        let codec_info = RtspCodecInfo {
            codec_id: RtspCodecId::H264,
            payload_type: 96,
            sample_rate: 90000,
            ..Default::default()
        };
        let channel = RtpChannel {
            codec_info,
            rtp_packer: None,
            rtp_unpacker: None,
            ssrc: 0,
            init_sequence: 42,
            init_timestamp: 12345,
        };
        assert_eq!(channel.initial_timestamp(), 12345);
    }

    #[test]
    fn test_on_frame_handler_when_unpacker_is_none_noop() {
        let codec_info = RtspCodecInfo {
            codec_id: RtspCodecId::G711A,
            payload_type: 8,
            sample_rate: 8000,
            ..Default::default()
        };
        let mut channel = RtpChannel {
            codec_info,
            rtp_packer: None,
            rtp_unpacker: None,
            ssrc: 0,
            init_sequence: 0,
            init_timestamp: 0,
        };
        // Should not panic when unpacker is None
        channel.on_frame_handler(Box::new(|_| Ok(())));
    }

    #[test]
    fn test_on_packet_handler_when_packer_is_none_noop() {
        use std::future::Future;
        use std::pin::Pin;
        let codec_info = RtspCodecInfo {
            codec_id: RtspCodecId::H264,
            payload_type: 96,
            sample_rate: 90000,
            ..Default::default()
        };
        let mut channel = RtpChannel {
            codec_info,
            rtp_packer: None,
            rtp_unpacker: None,
            ssrc: 0,
            init_sequence: 0,
            init_timestamp: 0,
        };
        // Should not panic when packer is None
        channel.on_packet_handler(Box::new(|_, _| {
            Box::pin(async { Ok(()) })
                as Pin<Box<dyn Future<Output = Result<(), PackerError>> + Send>>
        }));
    }

    #[test]
    fn test_on_packet_for_rtcp_handler_when_packer_is_none_noop() {
        use std::future::Future;
        use std::pin::Pin;
        let codec_info = RtspCodecInfo {
            codec_id: RtspCodecId::H264,
            payload_type: 96,
            sample_rate: 90000,
            ..Default::default()
        };
        let mut channel = RtpChannel {
            codec_info,
            rtp_packer: None,
            rtp_unpacker: None,
            ssrc: 0,
            init_sequence: 0,
            init_timestamp: 0,
        };
        // Should not panic when packer is None
        channel.on_packet_for_rtcp_handler(Box::new(|_| {
            Box::pin(async {}) as Pin<Box<dyn Future<Output = ()> + Send>>
        }));
    }

    #[test]
    fn test_rtcp_channel_on_packet_with_non_default_packet() {
        use crate::rtsp::rtp::rtp_header::RtpHeader;
        use bytes::BytesMut;

        let mut channel = RtcpChannel::default();
        let packet = RtpPacket {
            header: RtpHeader {
                version: 2,
                padding_flag: 0,
                extension_flag: 0,
                cc: 0,
                marker: 1,
                payload_type: 96,
                seq_number: 1234,
                timestamp: 90000,
                ssrc: 0x12345678,
                csrcs: Vec::new(),
            },
            header_extension_profile: 0,
            header_extension_length: 0,
            header_extension_payload: BytesMut::new(),
            payload: BytesMut::from(&[1, 2, 3, 4][..]),
            padding: BytesMut::new(),
        };
        // Should not panic
        channel.on_packet(packet);
    }

    #[test]
    fn test_rtp_channel_ssrc_getter_returns_value() {
        let codec_info = RtspCodecInfo {
            codec_id: RtspCodecId::H264,
            payload_type: 96,
            sample_rate: 90000,
            ..Default::default()
        };
        let channel = RtpChannel {
            codec_info,
            rtp_packer: None,
            rtp_unpacker: None,
            ssrc: 0xABCDEF00,
            init_sequence: 0,
            init_timestamp: 0,
        };
        assert_eq!(channel.ssrc(), 0xABCDEF00);
    }

    // ========================================================================
    // TRtpFunc::create_packer Tests with Mock TNetIO
    // ========================================================================

    use crate::bytesio::bytesio_errors::BytesIOError;
    use crate::bytesio::{NetType, TNetIO};
    use async_trait::async_trait;
    use bytes::{Bytes, BytesMut};
    use mockall::mock;
    use std::time::Duration;

    mock! {
        NetIO {}
        #[async_trait]
        impl TNetIO for NetIO {
            async fn write(&mut self, bytes: Bytes) -> Result<(), BytesIOError>;
            async fn read(&mut self) -> Result<BytesMut, BytesIOError>;
            async fn read_timeout(&mut self, duration: Duration) -> Result<BytesMut, BytesIOError>;
            fn get_net_type(&self) -> NetType;
        }
    }

    #[test]
    fn test_create_packer_h264_creates_packer() {
        let codec_info = RtspCodecInfo {
            codec_id: RtspCodecId::H264,
            payload_type: 96,
            sample_rate: 90000,
            ..Default::default()
        };
        let mut channel = RtpChannel {
            codec_info,
            rtp_packer: None,
            rtp_unpacker: None,
            ssrc: 0x12345678,
            init_sequence: 100,
            init_timestamp: 0,
        };
        let mock_io = Arc::new(Mutex::new(
            Box::new(MockNetIO::new()) as Box<dyn TNetIO + Send + Sync>
        ));
        channel.create_packer(mock_io);
        assert!(channel.rtp_packer.is_some());
    }

    #[test]
    fn test_create_packer_h265_creates_packer() {
        let codec_info = RtspCodecInfo {
            codec_id: RtspCodecId::H265,
            payload_type: 98,
            sample_rate: 90000,
            ..Default::default()
        };
        let mut channel = RtpChannel {
            codec_info,
            rtp_packer: None,
            rtp_unpacker: None,
            ssrc: 0x12345678,
            init_sequence: 100,
            init_timestamp: 0,
        };
        let mock_io = Arc::new(Mutex::new(
            Box::new(MockNetIO::new()) as Box<dyn TNetIO + Send + Sync>
        ));
        channel.create_packer(mock_io);
        assert!(channel.rtp_packer.is_some());
    }

    #[test]
    fn test_create_packer_aac_creates_packer() {
        let codec_info = RtspCodecInfo {
            codec_id: RtspCodecId::AAC,
            payload_type: 97,
            sample_rate: 44100,
            channel_count: 2,
        };
        let mut channel = RtpChannel {
            codec_info,
            rtp_packer: None,
            rtp_unpacker: None,
            ssrc: 0x12345678,
            init_sequence: 100,
            init_timestamp: 0,
        };
        let mock_io = Arc::new(Mutex::new(
            Box::new(MockNetIO::new()) as Box<dyn TNetIO + Send + Sync>
        ));
        channel.create_packer(mock_io);
        assert!(channel.rtp_packer.is_some());
    }

    #[test]
    fn test_create_packer_g711a_no_packer_created() {
        let codec_info = RtspCodecInfo {
            codec_id: RtspCodecId::G711A,
            payload_type: 8,
            sample_rate: 8000,
            ..Default::default()
        };
        let mut channel = RtpChannel {
            codec_info,
            rtp_packer: None,
            rtp_unpacker: None,
            ssrc: 0x12345678,
            init_sequence: 100,
            init_timestamp: 0,
        };
        let mock_io = Arc::new(Mutex::new(
            Box::new(MockNetIO::new()) as Box<dyn TNetIO + Send + Sync>
        ));
        channel.create_packer(mock_io);
        // G711A packer not implemented yet
        assert!(channel.rtp_packer.is_none());
    }

    #[test]
    fn test_on_frame_handler_when_unpacker_is_some_accepts_handler() {
        let codec_info = RtspCodecInfo {
            codec_id: RtspCodecId::H264,
            payload_type: 96,
            sample_rate: 90000,
            ..Default::default()
        };
        let mut channel = RtpChannel::new(codec_info);
        // Unpacker is created in RtpChannel::new for H264
        assert!(channel.rtp_unpacker.is_some());
        // Should accept handler without panic
        channel.on_frame_handler(Box::new(|_| Ok(())));
    }

    #[test]
    fn test_on_packet_handler_when_packer_is_some_accepts_handler() {
        use std::future::Future;
        use std::pin::Pin;
        let codec_info = RtspCodecInfo {
            codec_id: RtspCodecId::H264,
            payload_type: 96,
            sample_rate: 90000,
            ..Default::default()
        };
        let mut channel = RtpChannel::new(codec_info);
        let mock_io = Arc::new(Mutex::new(
            Box::new(MockNetIO::new()) as Box<dyn TNetIO + Send + Sync>
        ));
        channel.create_packer(mock_io);
        assert!(channel.rtp_packer.is_some());
        // Should accept handler without panic
        channel.on_packet_handler(Box::new(|_, _| {
            Box::pin(async { Ok(()) })
                as Pin<Box<dyn Future<Output = Result<(), PackerError>> + Send>>
        }));
    }

    #[test]
    fn test_on_packet_for_rtcp_handler_when_packer_is_some_accepts_handler() {
        use std::future::Future;
        use std::pin::Pin;
        let codec_info = RtspCodecInfo {
            codec_id: RtspCodecId::H264,
            payload_type: 96,
            sample_rate: 90000,
            ..Default::default()
        };
        let mut channel = RtpChannel::new(codec_info);
        let mock_io = Arc::new(Mutex::new(
            Box::new(MockNetIO::new()) as Box<dyn TNetIO + Send + Sync>
        ));
        channel.create_packer(mock_io);
        assert!(channel.rtp_packer.is_some());
        // Should accept handler without panic
        channel.on_packet_for_rtcp_handler(Box::new(|_| {
            Box::pin(async {}) as Pin<Box<dyn Future<Output = ()> + Send>>
        }));
    }
}
