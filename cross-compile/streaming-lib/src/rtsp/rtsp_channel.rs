use crate::rtsp::rtp::RtpPacket;
use crate::rtsp::rtp::errors::PackerError;
use crate::rtsp::rtp::errors::UnPackerError;
use crate::rtsp::rtp::rtcp::RTCP_RR;
use crate::rtsp::rtp::rtcp::RTCP_SR;
use crate::rtsp::rtp::rtcp::rtcp_header::RtcpHeader;
use crate::rtsp::rtp::utils::OnFrameFn;
use crate::rtsp::rtp::utils::OnRtpPacketFn;
use crate::rtsp::rtp::utils::OnRtpPacketFn2;

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
use rand::Rng;
use std::sync::Arc;
use tokio::sync::Mutex;

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
        let mut rtp_channel = RtpChannel {
            codec_info,
            ssrc,
            rtp_packer: None,
            rtp_unpacker: None,
            init_sequence: 0,
        };
        rtp_channel.create_unpacker();
        rtp_channel
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
            RtspCodecId::G711A => {}
        }
    }
    fn create_packer(&mut self, io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>) {
        match self.codec_info.codec_id {
            RtspCodecId::H264 => {
                self.rtp_packer = Some(Box::new(RtpH264Packer::new(
                    self.codec_info.payload_type,
                    self.ssrc,
                    self.init_sequence,
                    1400,
                    io,
                )));
            }
            RtspCodecId::H265 => {
                self.rtp_packer = Some(Box::new(RtpH265Packer::new(
                    self.codec_info.payload_type,
                    self.ssrc,
                    self.init_sequence,
                    1400,
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
            RtspCodecId::G711A => {}
        }
    }
}

impl RtcpChannel {
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
        self.recv_ctx.received_rtp(packet);
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
        };
        channel.create_unpacker();
        assert!(channel.rtp_unpacker.is_some());
    }
}
