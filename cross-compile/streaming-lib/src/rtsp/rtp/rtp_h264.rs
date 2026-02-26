use super::RtpHeader;
use super::RtpPacket;
use super::define;
use super::errors::PackerError;
use super::errors::UnPackerError;
use super::utils;
use super::utils::OnFrameFn;
use super::utils::OnRtpPacketFn;
use super::utils::OnRtpPacketFn2;
use super::utils::TPacker;
use super::utils::TRtpReceiverForRtcp;
use super::utils::TUnPacker;
use super::utils::TVideoPacker;
use super::utils::Unmarshal;
use crate::bytesio::TNetIO;
use crate::bytesio::bytes_errors::{BytesReadError, BytesReadErrorValue};
use crate::bytesio::bytes_reader::BytesReader;
use crate::streamhub::define::FrameData;
use async_trait::async_trait;
use byteorder::BigEndian;
use bytes::{BufMut, BytesMut};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone, Copy)]
enum NalType {
    Single,
    Stap,
    Mtap,
    Fu,
    Unknown,
}

impl NalType {
    fn from_header(nal_header: u8) -> Self {
        match nal_header & 0x1F {
            1..=23 => NalType::Single,
            define::STAP_A | define::STAP_B => NalType::Stap,
            define::MTAP_16 | define::MTAP_24 => NalType::Mtap,
            define::FU_A | define::FU_B => NalType::Fu,
            _ => NalType::Unknown,
        }
    }
}

pub struct RtpH264Packer {
    header: RtpHeader,
    mtu: usize,
    on_packet_handler: Option<OnRtpPacketFn>,
    on_packet_for_rtcp_handler: Option<OnRtpPacketFn2>,
    io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>,
}

impl RtpH264Packer {
    #[inline]
    fn is_vcl_nalu_header(nalu_header: u8) -> bool {
        matches!(nalu_header & 0x1F, 1..=5)
    }

    pub fn new(
        payload_type: u8,
        ssrc: u32,
        init_seq: u16,
        mtu: usize,
        io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>,
    ) -> Self {
        RtpH264Packer {
            header: RtpHeader {
                payload_type,
                seq_number: init_seq,
                ssrc,
                version: 2,
                ..Default::default()
            },
            mtu,
            io,
            on_packet_handler: None,
            on_packet_for_rtcp_handler: None,
        }
    }

    async fn pack_fu_a_with_marker(
        &mut self,
        nalu: BytesMut,
        mark_end_of_access_unit: bool,
    ) -> Result<(), PackerError> {
        let mut nalu_reader = BytesReader::new(nalu);
        let byte_1st = nalu_reader.read_u8()?;

        let fu_indicator: u8 = (byte_1st & 0xE0) | define::FU_A;
        let mut fu_header: u8 = (byte_1st & 0x1F) | define::FU_START;

        let mut left_nalu_bytes: usize = nalu_reader.len();
        let mut fu_payload_len: usize;

        while left_nalu_bytes > 0 {
            if left_nalu_bytes + define::RTP_FIXED_HEADER_LEN <= self.mtu - 2 {
                fu_header = (byte_1st & 0x1F) | define::FU_END;
                fu_payload_len = left_nalu_bytes;
            } else {
                fu_payload_len = self.mtu - define::RTP_FIXED_HEADER_LEN - 2;
            }

            let fu_payload = nalu_reader.read_bytes(fu_payload_len)?;

            let mut packet = RtpPacket::new(self.header.clone());
            packet.payload.put_u8(fu_indicator);
            packet.payload.put_u8(fu_header);

            if fu_header & define::FU_START > 0 {
                fu_header &= 0x7F
            }

            packet.payload.put(fu_payload);
            packet.header.marker = if mark_end_of_access_unit && fu_header & define::FU_END > 0 {
                1
            } else {
                0
            };

            if let Some(f) = &self.on_packet_for_rtcp_handler {
                f(packet.clone()).await;
            }

            if let Some(f) = &self.on_packet_handler {
                // log::info!("seq number: {}", packet.header.seq_number);
                f(self.io.clone(), packet).await?;
            }

            left_nalu_bytes = nalu_reader.len();
            // rtp seq-numer should be adding wrapping
            self.header.seq_number = self.header.seq_number.wrapping_add(1);
        }

        Ok(())
    }

    async fn pack_single_with_marker(
        &mut self,
        nalu: BytesMut,
        mark_end_of_access_unit: bool,
    ) -> Result<(), PackerError> {
        let mut packet = RtpPacket::new(self.header.clone());
        packet.header.marker = u8::from(mark_end_of_access_unit);
        packet.payload.put(nalu);

        // let packet_bytesmut = packet.marshal()?;
        self.header.seq_number = self.header.seq_number.wrapping_add(1);

        if let Some(f) = &self.on_packet_for_rtcp_handler {
            f(packet.clone()).await;
        }

        if let Some(f) = &self.on_packet_handler {
            return f(self.io.clone(), packet).await;
        }

        Ok(())
    }

    pub async fn pack_fu_a(&mut self, nalu: BytesMut) -> Result<(), PackerError> {
        let mark_end_of_access_unit = nalu
            .first()
            .is_some_and(|header| Self::is_vcl_nalu_header(*header));
        self.pack_fu_a_with_marker(nalu, mark_end_of_access_unit)
            .await
    }

    pub async fn pack_single(&mut self, nalu: BytesMut) -> Result<(), PackerError> {
        let mark_end_of_access_unit = nalu
            .first()
            .is_some_and(|header| Self::is_vcl_nalu_header(*header));
        self.pack_single_with_marker(nalu, mark_end_of_access_unit)
            .await
    }

    fn extract_nalus_from_frame(frame: &mut BytesMut) -> Vec<BytesMut> {
        let mut nal_units = Vec::new();

        while !frame.is_empty() {
            let Some(first_pos) = utils::find_start_code(&frame[..]) else {
                let raw_nalu = frame.split_to(frame.len());
                if !raw_nalu.is_empty() {
                    nal_units.push(raw_nalu);
                }
                break;
            };

            let Some(distance) = utils::find_start_code(&frame[first_pos + 3..]) else {
                let mut nalu_with_start_code = frame.split_to(frame.len());
                let nalu = nalu_with_start_code.split_off(first_pos + 3);
                if !nalu.is_empty() {
                    nal_units.push(nalu);
                }
                break;
            };

            let mut end_pos = first_pos + 3 + distance;
            while end_pos > 0 && frame[end_pos - 1] == 0 {
                end_pos -= 1;
            }
            let mut nalu_with_start_code = frame.split_to(end_pos);
            let nalu = nalu_with_start_code.split_off(first_pos + 3);
            if !nalu.is_empty() {
                nal_units.push(nalu);
            }
        }

        nal_units
    }

    async fn pack_nalu_with_marker(
        &mut self,
        nalu: BytesMut,
        mark_end_of_access_unit: bool,
    ) -> Result<(), PackerError> {
        if nalu.len() + define::RTP_FIXED_HEADER_LEN <= self.mtu {
            self.pack_single_with_marker(nalu, mark_end_of_access_unit)
                .await
        } else {
            self.pack_fu_a_with_marker(nalu, mark_end_of_access_unit)
                .await
        }
    }
}

#[async_trait]
impl TPacker for RtpH264Packer {
    //pack annexb h264 data
    async fn pack(&mut self, nalus: &mut BytesMut, timestamp: u32) -> Result<(), PackerError> {
        self.header.timestamp = timestamp; // ((timestamp as u64 * self.clock_rate as u64) / 1000) as u32;
        let extracted_nalus = Self::extract_nalus_from_frame(nalus);
        let last_index = extracted_nalus.len().saturating_sub(1);

        for (index, nalu) in extracted_nalus.into_iter().enumerate() {
            let mark_end_of_access_unit = index == last_index;
            self.pack_nalu_with_marker(nalu, mark_end_of_access_unit)
                .await?;
        }
        Ok(())
    }

    fn on_packet_handler(&mut self, f: OnRtpPacketFn) {
        self.on_packet_handler = Some(f);
    }
}

impl TRtpReceiverForRtcp for RtpH264Packer {
    fn on_packet_for_rtcp_handler(&mut self, f: OnRtpPacketFn2) {
        self.on_packet_for_rtcp_handler = Some(f);
    }
}

#[async_trait]
impl TVideoPacker for RtpH264Packer {
    async fn pack_nalu(&mut self, nalu: BytesMut) -> Result<(), PackerError> {
        let mark_end_of_access_unit = nalu
            .first()
            .is_some_and(|header| Self::is_vcl_nalu_header(*header));
        self.pack_nalu_with_marker(nalu, mark_end_of_access_unit)
            .await
    }
}

#[derive(Default)]
pub struct RtpH264UnPacker {
    sequence_number: u16,
    timestamp: u32,
    fu_buffer: BytesMut,
    on_frame_handler: Option<OnFrameFn>,
    on_packet_for_rtcp_handler: Option<OnRtpPacketFn2>,
}

#[async_trait]
impl TUnPacker for RtpH264UnPacker {
    async fn unpack(&mut self, reader: &mut BytesReader) -> Result<(), UnPackerError> {
        let rtp_packet = RtpPacket::unmarshal(reader)?;

        if let Some(f) = &self.on_packet_for_rtcp_handler {
            f(rtp_packet.clone()).await;
        }

        self.timestamp = rtp_packet.header.timestamp;
        self.sequence_number = rtp_packet.header.seq_number;

        let Some(nal_header) = rtp_packet.payload.first() else {
            return Ok(());
        };

        let nal_type = NalType::from_header(*nal_header);
        match nal_type {
            NalType::Single => self.unpack_single(rtp_packet.payload.clone(), *nal_header),
            NalType::Stap => self.unpack_stap(rtp_packet.payload.clone(), *nal_header),
            NalType::Mtap => self.unpack_mtap(rtp_packet.payload.clone(), *nal_header),
            NalType::Fu => self.unpack_fu(rtp_packet.payload.clone(), *nal_header),
            NalType::Unknown => Ok(()),
        }
    }

    fn on_frame_handler(&mut self, f: OnFrameFn) {
        self.on_frame_handler = Some(f);
    }
}

impl RtpH264UnPacker {
    pub fn new() -> Self {
        RtpH264UnPacker {
            ..Default::default()
        }
    }

    fn unpack_single(
        &mut self,
        payload: BytesMut,
        _t: define::RtpNalType,
    ) -> Result<(), UnPackerError> {
        if let Some(f) = &self.on_frame_handler {
            let mut annexb_payload = BytesMut::new();
            annexb_payload.extend_from_slice(&define::ANNEXB_NALU_START_CODE);
            annexb_payload.put(payload);

            f(FrameData::Video {
                timestamp: self.timestamp,
                data: annexb_payload,
            })?;
        }
        Ok(())
    }

    //  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // | FU indicator  |   FU header   |                               |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+                               |
    // |                                                               |
    // |                         FU payload                            |
    // |                                                               |
    // |                               +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |                               :...OPTIONAL RTP padding        |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+

    //   RTP payload format for FU-A

    //  0                   1                   2                   3
    //  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // | FU indicator  |   FU header   |               DON             |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-|
    // |                                                               |
    // |                         FU payload                            |
    // |                                                               |
    // |                               +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |                               :...OPTIONAL RTP padding        |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+

    //   RTP payload format for FU-B

    // FU indicator
    // +---------------+
    // |0|1|2|3|4|5|6|7|
    // +-+-+-+-+-+-+-+-+
    // |F|NRI|  Type   |
    // +---------------+

    // FU header
    // +---------------+
    // |0|1|2|3|4|5|6|7|
    // +-+-+-+-+-+-+-+-+
    // |S|E|R|  Type   |
    // +---------------+
    fn unpack_fu(
        &mut self,
        rtp_payload: BytesMut,
        t: define::RtpNalType,
    ) -> Result<(), UnPackerError> {
        let mut payload_reader = BytesReader::new(rtp_payload);
        let fu_indicator = payload_reader.read_u8()?;
        let fu_header = payload_reader.read_u8()?;

        if t == define::FU_B {
            //read DON
            payload_reader.read_u16::<BigEndian>()?;
        }

        if utils::is_fu_start(fu_header) {
            self.fu_buffer
                .put_u8((fu_indicator & 0xE0) | (fu_header & 0x1F))
        }

        self.fu_buffer.put(payload_reader.extract_remaining_bytes());

        if utils::is_fu_end(fu_header) {
            let mut payload = BytesMut::new();
            payload.extend_from_slice(&define::ANNEXB_NALU_START_CODE);
            let fu_payload = std::mem::take(&mut self.fu_buffer);
            payload.put(fu_payload);
            if let Some(f) = &self.on_frame_handler {
                f(FrameData::Video {
                    timestamp: self.timestamp,
                    data: payload,
                })?;
            }
        }

        Ok(())
    }

    //  0                   1                   2                   3
    //  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |                          RTP Header                           |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |STAP-A NAL HDR |         NALU 1 Size           | NALU 1 HDR    |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |                         NALU 1 Data                           |
    // :                                                               :
    // +               +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |               | NALU 2 Size                   | NALU 2 HDR    |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |                         NALU 2 Data                           |
    // :                                                               :
    // |                               +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |                               :...OPTIONAL RTP padding        |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+

    //   An example of an RTP packet including an STAP-A
    //   containing two single-time aggregation units

    //  0                   1                   2                   3
    //  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |                          RTP Header                           |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |STAP-B NAL HDR | DON                           | NALU 1 Size   |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // | NALU 1 Size   | NALU 1 HDR    | NALU 1 Data                   |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+                               +
    // :                                                               :
    // +               +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |               | NALU 2 Size                   | NALU 2 HDR    |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |                       NALU 2 Data                             |
    // :                                                               :
    // |                               +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |                               :...OPTIONAL RTP padding        |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+

    //   An example of an RTP packet including an STAP-B
    //   containing two single-time aggregation units

    fn unpack_stap(
        &mut self,
        rtp_payload: BytesMut,
        t: define::RtpNalType,
    ) -> Result<(), UnPackerError> {
        let mut payload_reader = BytesReader::new(rtp_payload);
        //STAP-A / STAP-B HDR
        payload_reader.read_u8()?;

        if t == define::STAP_B {
            //read DON
            payload_reader.read_u16::<BigEndian>()?;
        }

        while !payload_reader.is_empty() {
            let length = payload_reader.read_u16::<BigEndian>()? as usize;
            let nalu = payload_reader.read_bytes(length)?;

            let mut payload = BytesMut::new();
            payload.extend_from_slice(&define::ANNEXB_NALU_START_CODE);
            payload.put(nalu);
            if let Some(f) = &self.on_frame_handler {
                f(FrameData::Video {
                    timestamp: self.timestamp,
                    data: payload,
                })?;
            }
        }
        Ok(())
    }

    //  0                   1                   2                   3
    //  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |                          RTP Header                           |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |MTAP16 NAL HDR |  decoding order number base   | NALU 1 Size   |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |  NALU 1 Size  |  NALU 1 DOND  |       NALU 1 TS offset        |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |  NALU 1 HDR   |  NALU 1 DATA                                  |
    // +-+-+-+-+-+-+-+-+                                               +
    // :                                                               :
    // +               +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |               | NALU 2 SIZE                   |  NALU 2 DOND  |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |       NALU 2 TS offset        |  NALU 2 HDR   |  NALU 2 DATA  |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+               |
    // :                                                               :
    // |                               +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |                               :...OPTIONAL RTP padding        |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+

    //   An RTP packet including a multi-time aggregation
    //   packet of type MTAP16 containing two multi-time
    //   aggregation units

    //  0                   1                   2                   3
    //  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |                          RTP Header                           |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |MTAP24 NAL HDR |  decoding order number base   | NALU 1 Size   |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |  NALU 1 Size  |  NALU 1 DOND  |       NALU 1 TS offs          |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |NALU 1 TS offs |  NALU 1 HDR   |  NALU 1 DATA                  |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+                               +
    // :                                                               :
    // +               +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |               | NALU 2 SIZE                   |  NALU 2 DOND  |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |       NALU 2 TS offset                        |  NALU 2 HDR   |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |  NALU 2 DATA                                                  |
    // :                                                               :
    // |                               +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |                               :...OPTIONAL RTP padding        |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+

    //   An RTP packet including a multi-time aggregation
    //   packet of type MTAP24 containing two multi-time
    //   aggregation units

    fn unpack_mtap(
        &mut self,
        rtp_payload: BytesMut,
        t: define::RtpNalType,
    ) -> Result<(), UnPackerError> {
        let mut payload_reader = BytesReader::new(rtp_payload);
        payload_reader.read_u8()?;
        payload_reader.read_u16::<BigEndian>()?;

        let ts_bytes = Self::get_mtap_ts_size(t);

        while !payload_reader.is_empty() {
            let nalu_size = payload_reader.read_u16::<BigEndian>()? as usize;
            payload_reader.read_u8()?;
            Self::skip_mtap_ts_offset(&mut payload_reader, t)?;

            if nalu_size < ts_bytes + 1 {
                return Err(BytesReadError {
                    value: BytesReadErrorValue::NotEnoughBytes,
                }
                .into());
            }
            let nalu = payload_reader.read_bytes(nalu_size - ts_bytes - 1)?;

            if let Some(f) = &self.on_frame_handler {
                let mut payload = BytesMut::new();
                payload.extend_from_slice(&define::ANNEXB_NALU_START_CODE);
                payload.put(nalu);
                f(FrameData::Video {
                    timestamp: self.timestamp,
                    data: payload,
                })?;
            }
        }

        Ok(())
    }

    fn get_mtap_ts_size(t: define::RtpNalType) -> usize {
        match t {
            define::MTAP_16 => 2,
            define::MTAP_24 => 3,
            _ => 0,
        }
    }

    fn skip_mtap_ts_offset(
        reader: &mut BytesReader,
        t: define::RtpNalType,
    ) -> Result<(), UnPackerError> {
        match t {
            define::MTAP_16 => {
                reader.read_u16::<BigEndian>()?;
            }
            define::MTAP_24 => {
                reader.read_u24::<BigEndian>()?;
            }
            _ => {
                log::warn!("Invalid MTAP type");
            }
        }
        Ok(())
    }
}

impl TRtpReceiverForRtcp for RtpH264UnPacker {
    fn on_packet_for_rtcp_handler(&mut self, f: OnRtpPacketFn2) {
        self.on_packet_for_rtcp_handler = Some(f);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytesio::bytes_reader::BytesReader;

    use crate::rtsp::rtp::utils::Marshal;
    use crate::streamhub::define::FrameData;
    use async_trait::async_trait;

    use bytes::BytesMut;
    use mockall::mock;
    use tokio::sync::Mutex;

    use crate::bytesio::bytesio_errors::BytesIOError;
    use crate::bytesio::{NetType, TNetIO};
    use bytes::Bytes;
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

    fn create_test_nalu() -> BytesMut {
        // Create a simple H.264 VCL NAL unit (IDR type 5)
        let mut nalu = BytesMut::new();
        nalu.put_u8(0x65); // NAL header: type 5 (IDR), NRI 3
        nalu.extend_from_slice(&[0x88, 0x84, 0x21, 0xa0, 0x10, 0x04, 0x48, 0x00]);
        nalu
    }

    fn create_test_sps_nalu() -> BytesMut {
        // Create a simple H.264 non-VCL NAL unit (SPS type 7)
        let mut nalu = BytesMut::new();
        nalu.put_u8(0x67); // NAL header: type 7 (SPS), NRI 3
        nalu.extend_from_slice(&[0x42, 0x00, 0x1e, 0x9a, 0x74, 0x90, 0x24, 0x00]);
        nalu
    }

    fn create_large_nalu(size: usize) -> BytesMut {
        let mut nalu = BytesMut::new();
        nalu.put_u8(0x65); // NAL header: type 5 (IDR), NRI 3
        nalu.extend_from_slice(&vec![0x00; size]);
        nalu
    }

    fn create_large_sps_nalu(size: usize) -> BytesMut {
        let mut nalu = BytesMut::new();
        nalu.put_u8(0x67); // NAL header: type 7 (SPS), NRI 3
        nalu.extend_from_slice(&vec![0x00; size]);
        nalu
    }

    #[tokio::test]
    async fn test_rtp_h264_packer_new() {
        let mock_io = Arc::new(Mutex::new(
            Box::new(MockNetIO::new()) as Box<dyn TNetIO + Send + Sync>
        ));
        let packer = RtpH264Packer::new(96, 12345, 0, 1500, mock_io);
        assert_eq!(packer.header.payload_type, 96);
        assert_eq!(packer.header.ssrc, 12345);
        assert_eq!(packer.header.seq_number, 0);
        assert_eq!(packer.mtu, 1500);
    }

    #[tokio::test]
    async fn test_rtp_h264_packer_pack_single_small_nalu() {
        let mock_io = Arc::new(Mutex::new(
            Box::new(MockNetIO::new()) as Box<dyn TNetIO + Send + Sync>
        ));
        let mut packer = RtpH264Packer::new(96, 12345, 0, 1500, mock_io);

        let _packet_count = 0;
        let packet_count_clone = std::sync::Arc::new(std::sync::Mutex::new(0));
        let packet_count_clone2 = packet_count_clone.clone();
        packer.on_packet_handler(Box::new(move |_io, packet| {
            let mut count = packet_count_clone2.lock().unwrap();
            *count += 1;
            assert_eq!(packet.header.marker, 1);
            assert_eq!(packet.header.payload_type, 96);
            assert!(!packet.payload.is_empty());
            Box::pin(async move { Ok(()) })
        }));

        let nalu = create_test_nalu();
        let result = packer.pack_single(nalu).await;
        assert!(result.is_ok());
        assert_eq!(*packet_count_clone.lock().unwrap(), 1);
    }

    #[tokio::test]
    async fn test_rtp_h264_packer_pack_single_non_vcl_marker_cleared() {
        let mock_io = Arc::new(Mutex::new(
            Box::new(MockNetIO::new()) as Box<dyn TNetIO + Send + Sync>
        ));
        let mut packer = RtpH264Packer::new(96, 12345, 0, 1500, mock_io);

        let packet_count = std::sync::Arc::new(std::sync::Mutex::new(0));
        let packet_count_clone = packet_count.clone();
        packer.on_packet_handler(Box::new(move |_io, packet| {
            let mut count = packet_count_clone.lock().unwrap();
            *count += 1;
            assert_eq!(packet.header.marker, 0);
            Box::pin(async move { Ok(()) })
        }));

        let sps = create_test_sps_nalu();
        let result = packer.pack_single(sps).await;
        assert!(result.is_ok());
        assert_eq!(*packet_count.lock().unwrap(), 1);
    }

    #[tokio::test]
    async fn test_rtp_h264_packer_pack_fu_a_large_nalu() {
        let mock_io = Arc::new(Mutex::new(
            Box::new(MockNetIO::new()) as Box<dyn TNetIO + Send + Sync>
        ));
        let mut packer = RtpH264Packer::new(96, 12345, 0, 1500, mock_io);

        let packet_count = std::sync::Arc::new(std::sync::Mutex::new(0));
        let first_packet_marker = std::sync::Arc::new(std::sync::Mutex::new(0));
        let last_packet_marker = std::sync::Arc::new(std::sync::Mutex::new(0));
        let packet_count_clone = packet_count.clone();
        let first_marker_clone = first_packet_marker.clone();
        let last_marker_clone = last_packet_marker.clone();

        packer.on_packet_handler(Box::new(move |_io, packet| {
            let mut count = packet_count_clone.lock().unwrap();
            *count += 1;
            if *count == 1 {
                *first_marker_clone.lock().unwrap() = packet.header.marker;
            }
            *last_marker_clone.lock().unwrap() = packet.header.marker;
            assert_eq!(packet.header.payload_type, 96);
            // Verify FU-A structure
            if packet.payload.len() > 0 {
                let fu_indicator = packet.payload[0];
                assert_eq!(fu_indicator & 0x1F, define::FU_A);
            }
            Box::pin(async move { Ok(()) })
        }));

        // Create a NAL unit larger than MTU
        let nalu = create_large_nalu(2000);
        let result = packer.pack_fu_a(nalu).await;
        assert!(result.is_ok());
        assert!(*packet_count.lock().unwrap() > 1); // Should be fragmented
        assert_eq!(*first_packet_marker.lock().unwrap(), 0); // First packet should not have marker
        assert_eq!(*last_packet_marker.lock().unwrap(), 1); // Last packet should have marker
    }

    #[tokio::test]
    async fn test_rtp_h264_packer_pack_fu_a_non_vcl_marker_cleared() {
        let mock_io = Arc::new(Mutex::new(
            Box::new(MockNetIO::new()) as Box<dyn TNetIO + Send + Sync>
        ));
        let mut packer = RtpH264Packer::new(96, 12345, 0, 1500, mock_io);

        let packet_count = std::sync::Arc::new(std::sync::Mutex::new(0));
        let last_packet_marker = std::sync::Arc::new(std::sync::Mutex::new(0));
        let packet_count_clone = packet_count.clone();
        let last_marker_clone = last_packet_marker.clone();

        packer.on_packet_handler(Box::new(move |_io, packet| {
            let mut count = packet_count_clone.lock().unwrap();
            *count += 1;
            *last_marker_clone.lock().unwrap() = packet.header.marker;
            Box::pin(async move { Ok(()) })
        }));

        let sps = create_large_sps_nalu(2000);
        let result = packer.pack_fu_a(sps).await;
        assert!(result.is_ok());
        assert!(*packet_count.lock().unwrap() > 1); // Should be fragmented
        assert_eq!(*last_packet_marker.lock().unwrap(), 0);
    }

    #[tokio::test]
    async fn test_rtp_h264_packer_pack_nalu_small() {
        let mock_io = Arc::new(Mutex::new(
            Box::new(MockNetIO::new()) as Box<dyn TNetIO + Send + Sync>
        ));
        let mut packer = RtpH264Packer::new(96, 12345, 0, 1500, mock_io);

        let packet_count = std::sync::Arc::new(std::sync::Mutex::new(0));
        let packet_count_clone = packet_count.clone();
        packer.on_packet_handler(Box::new(move |_io, packet| {
            *packet_count_clone.lock().unwrap() += 1;
            assert_eq!(packet.header.marker, 1);
            Box::pin(async move { Ok(()) })
        }));

        let nalu = create_test_nalu();
        let result = packer.pack_nalu(nalu).await;
        assert!(result.is_ok());
        assert_eq!(*packet_count.lock().unwrap(), 1);
    }

    #[tokio::test]
    async fn test_rtp_h264_packer_pack_nalu_large() {
        let mock_io = Arc::new(Mutex::new(
            Box::new(MockNetIO::new()) as Box<dyn TNetIO + Send + Sync>
        ));
        let mut packer = RtpH264Packer::new(96, 12345, 0, 1500, mock_io);

        let packet_count = std::sync::Arc::new(std::sync::Mutex::new(0));
        let packet_count_clone = packet_count.clone();
        packer.on_packet_handler(Box::new(move |_io, _packet| {
            *packet_count_clone.lock().unwrap() += 1;
            Box::pin(async move { Ok(()) })
        }));

        // Create a NAL unit larger than MTU
        let nalu = create_large_nalu(2000);
        let result = packer.pack_nalu(nalu).await;
        assert!(result.is_ok());
        assert!(*packet_count.lock().unwrap() > 1); // Should be fragmented
    }

    #[tokio::test]
    async fn test_rtp_h264_packer_pack_marks_only_last_vcl_nalu() {
        let mock_io = Arc::new(Mutex::new(
            Box::new(MockNetIO::new()) as Box<dyn TNetIO + Send + Sync>
        ));
        let mut packer = RtpH264Packer::new(96, 12345, 0, 1500, mock_io);

        let markers = std::sync::Arc::new(std::sync::Mutex::new(Vec::<u8>::new()));
        let markers_clone = markers.clone();
        packer.on_packet_handler(Box::new(move |_io, packet| {
            markers_clone.lock().unwrap().push(packet.header.marker);
            Box::pin(async move { Ok(()) })
        }));

        let mut frame = BytesMut::new();
        frame.extend_from_slice(&[0x00, 0x00, 0x00, 0x01, 0x65, 0x88, 0x84, 0x21]); // IDR
        frame.extend_from_slice(&[0x00, 0x00, 0x00, 0x01, 0x41, 0x9a, 0x22, 0x11]); // non-IDR

        let result = packer.pack(&mut frame, 1000).await;
        assert!(result.is_ok());

        let recorded_markers = markers.lock().unwrap();
        assert_eq!(recorded_markers.len(), 2);
        assert_eq!(*recorded_markers, vec![0, 1]);
    }

    #[tokio::test]
    async fn test_rtp_h264_packer_sequence_number_increment() {
        let mock_io = Arc::new(Mutex::new(
            Box::new(MockNetIO::new()) as Box<dyn TNetIO + Send + Sync>
        ));
        let mut packer = RtpH264Packer::new(96, 12345, 100, 1500, mock_io);

        let seq_numbers = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let seq_numbers_clone = seq_numbers.clone();
        packer.on_packet_handler(Box::new(move |_io, packet| {
            seq_numbers_clone
                .lock()
                .unwrap()
                .push(packet.header.seq_number);
            Box::pin(async move { Ok(()) })
        }));

        let nalu = create_large_nalu(2000);
        let _ = packer.pack_fu_a(nalu).await;

        // Verify sequence numbers increment
        let seqs = seq_numbers.lock().unwrap();
        assert!(seqs.len() > 1);
        for i in 1..seqs.len() {
            assert_eq!(seqs[i], seqs[i - 1].wrapping_add(1));
        }
    }

    #[tokio::test]
    async fn test_rtp_h264_unpacker_new() {
        let unpacker = RtpH264UnPacker::new();
        assert_eq!(unpacker.sequence_number, 0);
        assert_eq!(unpacker.timestamp, 0);
        assert!(unpacker.fu_buffer.is_empty());
    }

    #[tokio::test]
    async fn test_rtp_h264_unpacker_unpack_single() {
        let mut unpacker = RtpH264UnPacker::new();
        let frame_data = std::sync::Arc::new(std::sync::Mutex::new(None::<FrameData>));
        let frame_data_clone = frame_data.clone();

        unpacker.on_frame_handler(Box::new(move |frame| {
            *frame_data_clone.lock().unwrap() = Some(frame);
            Ok(())
        }));

        // Create a single NAL unit RTP packet
        let mut packet = RtpPacket::new(RtpHeader {
            payload_type: 96,
            seq_number: 1,
            timestamp: 1000,
            ssrc: 12345,
            version: 2,
            ..Default::default()
        });
        let nalu = create_test_nalu();
        packet.payload.put(nalu);

        let packet_bytes = packet.marshal().unwrap();
        let mut reader = BytesReader::new(packet_bytes);

        let result = unpacker.unpack(&mut reader).await;
        assert!(result.is_ok());
        let frame_opt = frame_data.lock().unwrap();
        assert!(frame_opt.is_some());
        if let Some(FrameData::Video { timestamp, data }) = frame_opt.as_ref() {
            assert_eq!(*timestamp, 1000);
            // Verify Annex-B start code
            assert_eq!(data[0..4], define::ANNEXB_NALU_START_CODE);
        }
    }

    #[tokio::test]
    async fn test_rtp_h264_unpacker_unpack_fu_a() {
        let mut unpacker = RtpH264UnPacker::new();
        let frame_count = std::sync::Arc::new(std::sync::Mutex::new(0));
        let frame_count_clone = frame_count.clone();

        unpacker.on_frame_handler(Box::new(move |_frame| {
            *frame_count_clone.lock().unwrap() += 1;
            Ok(())
        }));

        // Create FU-A start packet
        let mut start_packet = RtpPacket {
            header: RtpHeader {
                payload_type: 96,
                seq_number: 1,
                timestamp: 1000,
                ssrc: 12345,
                version: 2,
                marker: 0,
                ..Default::default()
            },
            ..Default::default()
        };
        let nalu_header = 0x65; // IDR frame
        start_packet
            .payload
            .put_u8((nalu_header & 0xE0) | define::FU_A);
        start_packet
            .payload
            .put_u8((nalu_header & 0x1F) | define::FU_START);
        start_packet.payload.extend_from_slice(&[0x01, 0x02, 0x03]);

        let start_bytes = start_packet.marshal().unwrap();
        let mut reader = BytesReader::new(start_bytes);
        let _ = unpacker.unpack(&mut reader).await;

        // Create FU-A end packet
        let mut end_packet = RtpPacket {
            header: RtpHeader {
                payload_type: 96,
                seq_number: 2,
                timestamp: 1000,
                ssrc: 12345,
                version: 2,
                marker: 1,
                ..Default::default()
            },
            ..Default::default()
        };
        end_packet
            .payload
            .put_u8((nalu_header & 0xE0) | define::FU_A);
        end_packet
            .payload
            .put_u8((nalu_header & 0x1F) | define::FU_END);
        end_packet.payload.extend_from_slice(&[0x04, 0x05, 0x06]);

        let end_bytes = end_packet.marshal().unwrap();
        let mut reader = BytesReader::new(end_bytes);
        let result = unpacker.unpack(&mut reader).await;

        assert!(result.is_ok());
        assert_eq!(*frame_count.lock().unwrap(), 1); // Should receive complete frame
    }

    #[tokio::test]
    async fn test_rtp_h264_unpacker_unpack_stap_a() {
        let mut unpacker = RtpH264UnPacker::new();
        let frame_count = std::sync::Arc::new(std::sync::Mutex::new(0));
        let frame_count_clone = frame_count.clone();

        unpacker.on_frame_handler(Box::new(move |_frame| {
            *frame_count_clone.lock().unwrap() += 1;
            Ok(())
        }));

        // Create STAP-A packet with two NAL units
        let mut packet = RtpPacket {
            header: RtpHeader {
                payload_type: 96,
                seq_number: 1,
                timestamp: 1000,
                ssrc: 12345,
                version: 2,
                marker: 1,
                ..Default::default()
            },
            ..Default::default()
        };

        // STAP-A header
        packet.payload.put_u8((0x67 & 0xE0) | define::STAP_A);

        // First NAL unit
        packet.payload.put_u16(4); // Size
        packet.payload.extend_from_slice(&[0x67, 0x42, 0x00, 0x1e]);

        // Second NAL unit
        packet.payload.put_u16(2); // Size
        packet.payload.extend_from_slice(&[0x68, 0xce]);

        let packet_bytes = packet.marshal().unwrap();
        let mut reader = BytesReader::new(packet_bytes);

        let result = unpacker.unpack(&mut reader).await;
        assert!(result.is_ok());
        assert_eq!(*frame_count.lock().unwrap(), 2); // Should receive two frames
    }

    #[tokio::test]
    async fn test_rtp_h264_unpacker_timestamp_preservation() {
        let mut unpacker = RtpH264UnPacker::new();
        let received_timestamp = std::sync::Arc::new(std::sync::Mutex::new(0u32));
        let received_timestamp_clone = received_timestamp.clone();

        unpacker.on_frame_handler(Box::new(move |frame| {
            if let FrameData::Video { timestamp, .. } = frame {
                *received_timestamp_clone.lock().unwrap() = timestamp;
            }
            Ok(())
        }));

        let mut packet = RtpPacket::new(RtpHeader {
            payload_type: 96,
            seq_number: 1,
            timestamp: 5000,
            ssrc: 12345,
            version: 2,
            marker: 1,
            ..Default::default()
        });
        let nalu = create_test_nalu();
        packet.payload.put(nalu);

        let packet_bytes = packet.marshal().unwrap();
        let mut reader = BytesReader::new(packet_bytes);

        let _ = unpacker.unpack(&mut reader).await;
        assert_eq!(*received_timestamp.lock().unwrap(), 5000);
    }

    // ========== Additional Tests for Coverage ==========

    #[tokio::test]
    async fn test_rtp_h264_packer_rtcp_handler() {
        let mock_io = Arc::new(Mutex::new(
            Box::new(MockNetIO::new()) as Box<dyn TNetIO + Send + Sync>
        ));
        let mut packer = RtpH264Packer::new(96, 12345, 0, 1500, mock_io);

        let rtcp_packet_count = std::sync::Arc::new(std::sync::Mutex::new(0));
        let rtcp_count_clone = rtcp_packet_count.clone();

        packer.on_packet_for_rtcp_handler(Box::new(move |_packet| {
            *rtcp_count_clone.lock().unwrap() += 1;
            Box::pin(async {})
        }));

        packer.on_packet_handler(Box::new(move |_io, _packet| {
            Box::pin(async move { Ok(()) })
        }));

        let nalu = create_test_nalu();
        let _ = packer.pack_single(nalu).await;

        assert_eq!(*rtcp_packet_count.lock().unwrap(), 1);
    }

    #[tokio::test]
    async fn test_rtp_h264_unpacker_rtcp_handler() {
        let mut unpacker = RtpH264UnPacker::new();

        let rtcp_packet_count = std::sync::Arc::new(std::sync::Mutex::new(0));
        let rtcp_count_clone = rtcp_packet_count.clone();

        unpacker.on_packet_for_rtcp_handler(Box::new(move |_packet| {
            *rtcp_count_clone.lock().unwrap() += 1;
            Box::pin(async {})
        }));

        unpacker.on_frame_handler(Box::new(move |_frame| Ok(())));

        let mut packet = RtpPacket::new(RtpHeader {
            payload_type: 96,
            seq_number: 1,
            timestamp: 1000,
            ssrc: 12345,
            version: 2,
            ..Default::default()
        });
        let nalu = create_test_nalu();
        packet.payload.put(nalu);

        let packet_bytes = packet.marshal().unwrap();
        let mut reader = BytesReader::new(packet_bytes);

        let _ = unpacker.unpack(&mut reader).await;
        assert_eq!(*rtcp_packet_count.lock().unwrap(), 1);
    }

    #[tokio::test]
    async fn test_rtp_h264_unpacker_unknown_nalu_type() {
        let mut unpacker = RtpH264UnPacker::new();

        let mut packet = RtpPacket::new(RtpHeader {
            payload_type: 96,
            seq_number: 1,
            timestamp: 1000,
            ssrc: 12345,
            version: 2,
            ..Default::default()
        });
        // Unknown NAL type (0x1E = 30)
        packet.payload.put_u8(0x1E);
        packet.payload.extend_from_slice(&[0x01, 0x02]);

        let packet_bytes = packet.marshal().unwrap();
        let mut reader = BytesReader::new(packet_bytes);

        let result = unpacker.unpack(&mut reader).await;
        assert!(result.is_ok()); // Unknown types are silently ignored
    }

    #[tokio::test]
    async fn test_rtp_h264_packer_mtu_boundary() {
        let mock_io = Arc::new(Mutex::new(
            Box::new(MockNetIO::new()) as Box<dyn TNetIO + Send + Sync>
        ));
        // MTU exactly matching RTP fixed header + NAL
        let mut packer = RtpH264Packer::new(96, 12345, 0, 100, mock_io);

        let packet_count = std::sync::Arc::new(std::sync::Mutex::new(0));
        let packet_count_clone = packet_count.clone();
        packer.on_packet_handler(Box::new(move |_io, _packet| {
            *packet_count_clone.lock().unwrap() += 1;
            Box::pin(async move { Ok(()) })
        }));

        // Create NAL that's exactly at MTU boundary
        let nalu = create_large_nalu(100 - define::RTP_FIXED_HEADER_LEN - 1);
        let _ = packer.pack_nalu(nalu).await;

        // Should fit in single packet
        assert_eq!(*packet_count.lock().unwrap(), 1);
    }
    #[tokio::test]
    async fn test_rtp_h264_unpacker_unpack_stap_b() {
        let mut unpacker = RtpH264UnPacker::new();
        let frame_count = std::sync::Arc::new(std::sync::Mutex::new(0));
        let frame_count_clone = frame_count.clone();

        unpacker.on_frame_handler(Box::new(move |_frame| {
            *frame_count_clone.lock().unwrap() += 1;
            Ok(())
        }));

        let mut packet = RtpPacket::new(RtpHeader {
            payload_type: 96,
            seq_number: 1,
            timestamp: 1000,
            ssrc: 12345,
            version: 2,
            ..Default::default()
        });

        // STAP-B Header (Type 26)
        packet.payload.put_u8(define::STAP_B);
        // DON (16-bit)
        packet.payload.put_u16(0x1234);

        // NALU 1
        packet.payload.put_u16(2); // Size
        packet.payload.extend_from_slice(&[0x01, 0x01]);

        // NALU 2
        packet.payload.put_u16(2); // Size
        packet.payload.extend_from_slice(&[0x02, 0x02]);

        let packet_bytes = packet.marshal().unwrap();
        let mut reader = BytesReader::new(packet_bytes);

        let result = unpacker.unpack(&mut reader).await;
        assert!(result.is_ok());
        assert_eq!(*frame_count.lock().unwrap(), 2);
    }

    #[tokio::test]
    async fn test_rtp_h264_unpacker_unpack_fu_b() {
        let mut unpacker = RtpH264UnPacker::new();
        let frame_count = std::sync::Arc::new(std::sync::Mutex::new(0));
        let frame_count_clone = frame_count.clone();

        unpacker.on_frame_handler(Box::new(move |_frame| {
            *frame_count_clone.lock().unwrap() += 1;
            Ok(())
        }));

        // Start Packet (FU-B)
        let mut start_packet = RtpPacket::new(RtpHeader {
            payload_type: 96,
            seq_number: 1,
            timestamp: 1000,
            ssrc: 12345,
            version: 2,
            marker: 0,
            ..Default::default()
        });

        // Indicator: FU-B (Type 29)
        start_packet.payload.put_u8(define::FU_B);
        // Header: Start bit set, Type 5 (IDR)
        start_packet.payload.put_u8(define::FU_START | 5);
        // DON (16-bit) - Only for FU-B
        start_packet.payload.put_u16(0x1234);
        // Data
        start_packet.payload.extend_from_slice(&[0x01, 0x02]);

        let start_bytes = start_packet.marshal().unwrap();
        let mut reader = BytesReader::new(start_bytes);
        let result = unpacker.unpack(&mut reader).await;
        assert!(result.is_ok());

        // End Packet (FU-B)
        let mut end_packet = RtpPacket::new(RtpHeader {
            payload_type: 96,
            seq_number: 2,
            timestamp: 1000,
            ssrc: 12345,
            version: 2,
            marker: 1,
            ..Default::default()
        });

        end_packet.payload.put_u8(define::FU_B);
        // Header: End bit set, Type 5 (IDR)
        end_packet.payload.put_u8(define::FU_END | 5);
        // DON (16-bit) - Only for FU-B
        end_packet.payload.put_u16(0x1234);
        // Data
        end_packet.payload.extend_from_slice(&[0x03, 0x04]);

        let end_bytes = end_packet.marshal().unwrap();
        let mut reader = BytesReader::new(end_bytes);
        let result = unpacker.unpack(&mut reader).await;
        assert!(result.is_ok());

        assert_eq!(*frame_count.lock().unwrap(), 1);
    }

    #[tokio::test]
    async fn test_rtp_h264_unpacker_unpack_mtap16() {
        let mut unpacker = RtpH264UnPacker::new();
        let frame_count = std::sync::Arc::new(std::sync::Mutex::new(0));
        let frame_count_clone = frame_count.clone();

        unpacker.on_frame_handler(Box::new(move |_frame| {
            *frame_count_clone.lock().unwrap() += 1;
            Ok(())
        }));

        let mut packet = RtpPacket::new(RtpHeader {
            payload_type: 96,
            seq_number: 1,
            timestamp: 1000,
            ssrc: 12345,
            version: 2,
            ..Default::default()
        });

        // MTAP-16 Header (Type 26)
        packet.payload.put_u8(define::MTAP_16);
        // DON Base (16-bit)
        packet.payload.put_u16(0x1000);

        // NALU 1 (size includes DOND(1) + TS offset(2) + NALU data(2) = 5)
        packet.payload.put_u16(5); // Size
        packet.payload.put_u8(0); // DOND
        packet.payload.put_u16(100); // TS offset (Must be non-zero for test assertion)
        packet.payload.extend_from_slice(&[0x01, 0x01]);

        // NALU 2 (size includes DOND(1) + TS offset(2) + NALU data(2) = 5)
        packet.payload.put_u16(5); // Size
        packet.payload.put_u8(1); // DOND
        packet.payload.put_u16(200); // TS offset
        packet.payload.extend_from_slice(&[0x02, 0x02]);

        let packet_bytes = packet.marshal().unwrap();
        let mut reader = BytesReader::new(packet_bytes);

        let result = unpacker.unpack(&mut reader).await;
        assert!(result.is_ok());
        assert_eq!(*frame_count.lock().unwrap(), 2);
    }

    #[tokio::test]
    async fn test_rtp_h264_unpacker_unpack_mtap24() {
        let mut unpacker = RtpH264UnPacker::new();
        let frame_count = std::sync::Arc::new(std::sync::Mutex::new(0));
        let frame_count_clone = frame_count.clone();

        unpacker.on_frame_handler(Box::new(move |_frame| {
            *frame_count_clone.lock().unwrap() += 1;
            Ok(())
        }));

        let mut packet = RtpPacket::new(RtpHeader {
            payload_type: 96,
            seq_number: 1,
            timestamp: 1000,
            ssrc: 12345,
            version: 2,
            ..Default::default()
        });

        // MTAP-24 Header (Type 27)
        packet.payload.put_u8(define::MTAP_24);
        // DON Base (16-bit)
        packet.payload.put_u16(0x1000);

        // NALU 1 (size includes DOND(1) + TS offset(3) + NALU data(2) = 6)
        packet.payload.put_u16(6); // Size
        packet.payload.put_u8(0); // DOND
        // TS Offset (24-bit)
        packet.payload.put_u8(0);
        packet.payload.put_u16(100);
        packet.payload.extend_from_slice(&[0x01, 0x01]);

        let packet_bytes = packet.marshal().unwrap();
        let mut reader = BytesReader::new(packet_bytes);

        let result = unpacker.unpack(&mut reader).await;
        assert!(result.is_ok());
        assert_eq!(*frame_count.lock().unwrap(), 1);
    }

    // ========== NalType::from_header Tests ==========

    #[test]
    fn test_nal_type_from_header_single_type_1() {
        assert!(matches!(NalType::from_header(0x61), NalType::Single)); // type 1 (non-IDR)
    }

    #[test]
    fn test_nal_type_from_header_single_type_5() {
        assert!(matches!(NalType::from_header(0x65), NalType::Single)); // type 5 (IDR)
    }

    #[test]
    fn test_nal_type_from_header_single_type_7() {
        assert!(matches!(NalType::from_header(0x67), NalType::Single)); // type 7 (SPS)
    }

    #[test]
    fn test_nal_type_from_header_single_type_8() {
        assert!(matches!(NalType::from_header(0x68), NalType::Single)); // type 8 (PPS)
    }

    #[test]
    fn test_nal_type_from_header_single_type_23() {
        assert!(matches!(NalType::from_header(23), NalType::Single)); // type 23
    }

    #[test]
    fn test_nal_type_from_header_stap_a() {
        assert!(matches!(
            NalType::from_header(define::STAP_A),
            NalType::Stap
        ));
    }

    #[test]
    fn test_nal_type_from_header_stap_b() {
        assert!(matches!(
            NalType::from_header(define::STAP_B),
            NalType::Stap
        ));
    }

    #[test]
    fn test_nal_type_from_header_mtap_16() {
        assert!(matches!(
            NalType::from_header(define::MTAP_16),
            NalType::Mtap
        ));
    }

    #[test]
    fn test_nal_type_from_header_mtap_24() {
        assert!(matches!(
            NalType::from_header(define::MTAP_24),
            NalType::Mtap
        ));
    }

    #[test]
    fn test_nal_type_from_header_fu_a() {
        assert!(matches!(NalType::from_header(define::FU_A), NalType::Fu));
    }

    #[test]
    fn test_nal_type_from_header_fu_b() {
        assert!(matches!(NalType::from_header(define::FU_B), NalType::Fu));
    }

    #[test]
    fn test_nal_type_from_header_unknown_type_0() {
        assert!(matches!(NalType::from_header(0x00), NalType::Unknown));
    }

    #[test]
    fn test_nal_type_from_header_unknown_type_30() {
        assert!(matches!(NalType::from_header(30), NalType::Unknown));
    }

    #[test]
    fn test_nal_type_from_header_unknown_type_31() {
        assert!(matches!(NalType::from_header(31), NalType::Unknown));
    }

    // ========== is_vcl_nalu_header Tests ==========

    #[test]
    fn test_is_vcl_nalu_header_type_1() {
        assert!(RtpH264Packer::is_vcl_nalu_header(0x61)); // type 1
    }

    #[test]
    fn test_is_vcl_nalu_header_type_5() {
        assert!(RtpH264Packer::is_vcl_nalu_header(0x65)); // type 5 (IDR)
    }

    #[test]
    fn test_is_vcl_nalu_header_type_3() {
        assert!(RtpH264Packer::is_vcl_nalu_header(0x03)); // type 3
    }

    #[test]
    fn test_is_vcl_nalu_header_sps_not_vcl() {
        assert!(!RtpH264Packer::is_vcl_nalu_header(0x67)); // type 7 (SPS)
    }

    #[test]
    fn test_is_vcl_nalu_header_pps_not_vcl() {
        assert!(!RtpH264Packer::is_vcl_nalu_header(0x68)); // type 8 (PPS)
    }

    #[test]
    fn test_is_vcl_nalu_header_type_0_not_vcl() {
        assert!(!RtpH264Packer::is_vcl_nalu_header(0x00)); // type 0
    }

    #[test]
    fn test_is_vcl_nalu_header_type_6_not_vcl() {
        assert!(!RtpH264Packer::is_vcl_nalu_header(0x06)); // type 6 (SEI)
    }

    // ========== extract_nalus_from_frame Tests ==========

    #[test]
    fn test_extract_nalus_from_frame_empty() {
        let mut frame = BytesMut::new();
        let nalus = RtpH264Packer::extract_nalus_from_frame(&mut frame);
        assert!(nalus.is_empty());
    }

    #[test]
    fn test_extract_nalus_from_frame_single_nalu_with_start_code() {
        let mut frame = BytesMut::new();
        frame.extend_from_slice(&[0x00, 0x00, 0x01]); // 3-byte start code
        frame.extend_from_slice(&[0x65, 0x88, 0x84]); // IDR NALU
        let nalus = RtpH264Packer::extract_nalus_from_frame(&mut frame);
        assert_eq!(nalus.len(), 1);
        assert_eq!(nalus[0][0], 0x65);
    }

    #[test]
    fn test_extract_nalus_from_frame_two_nalus() {
        let mut frame = BytesMut::new();
        frame.extend_from_slice(&[0x00, 0x00, 0x01]); // start code
        frame.extend_from_slice(&[0x67, 0x42, 0x00]); // SPS
        frame.extend_from_slice(&[0x00, 0x00, 0x01]); // start code
        frame.extend_from_slice(&[0x68, 0xCE, 0x38]); // PPS
        let nalus = RtpH264Packer::extract_nalus_from_frame(&mut frame);
        assert_eq!(nalus.len(), 2);
        assert_eq!(nalus[0][0] & 0x1F, 7); // SPS
        assert_eq!(nalus[1][0] & 0x1F, 8); // PPS
    }

    #[test]
    fn test_extract_nalus_from_frame_four_byte_start_code() {
        let mut frame = BytesMut::new();
        frame.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]); // 4-byte start code
        frame.extend_from_slice(&[0x65, 0x01, 0x02]); // IDR
        let nalus = RtpH264Packer::extract_nalus_from_frame(&mut frame);
        assert_eq!(nalus.len(), 1);
    }

    #[test]
    fn test_extract_nalus_from_frame_raw_nalu_no_start_code() {
        let mut frame = BytesMut::new();
        frame.extend_from_slice(&[0x65, 0x01, 0x02, 0x03]); // Raw NALU without start code
        let nalus = RtpH264Packer::extract_nalus_from_frame(&mut frame);
        assert_eq!(nalus.len(), 1);
        assert_eq!(nalus[0].len(), 4);
    }

    // ========== get_mtap_ts_size Tests ==========

    #[test]
    fn test_get_mtap_ts_size_mtap16() {
        assert_eq!(RtpH264UnPacker::get_mtap_ts_size(define::MTAP_16), 2);
    }

    #[test]
    fn test_get_mtap_ts_size_mtap24() {
        assert_eq!(RtpH264UnPacker::get_mtap_ts_size(define::MTAP_24), 3);
    }

    #[test]
    fn test_get_mtap_ts_size_unknown_type() {
        assert_eq!(RtpH264UnPacker::get_mtap_ts_size(0), 0);
    }

    #[test]
    fn test_get_mtap_ts_size_stap_a() {
        assert_eq!(RtpH264UnPacker::get_mtap_ts_size(define::STAP_A), 0);
    }

    // ========== skip_mtap_ts_offset Tests ==========

    #[test]
    fn test_skip_mtap_ts_offset_mtap16() {
        let data = BytesMut::from(&[0x00, 0x64][..]); // 2-byte TS offset
        let mut reader = BytesReader::new(data);
        let result = RtpH264UnPacker::skip_mtap_ts_offset(&mut reader, define::MTAP_16);
        assert!(result.is_ok());
    }

    #[test]
    fn test_skip_mtap_ts_offset_mtap24() {
        let data = BytesMut::from(&[0x00, 0x00, 0x64][..]); // 3-byte TS offset
        let mut reader = BytesReader::new(data);
        let result = RtpH264UnPacker::skip_mtap_ts_offset(&mut reader, define::MTAP_24);
        assert!(result.is_ok());
    }

    #[test]
    fn test_skip_mtap_ts_offset_unknown_type_does_nothing() {
        let data = BytesMut::from(&[0x00][..]);
        let mut reader = BytesReader::new(data);
        let result = RtpH264UnPacker::skip_mtap_ts_offset(&mut reader, 0);
        assert!(result.is_ok());
    }

    #[test]
    fn test_skip_mtap_ts_offset_mtap16_insufficient_data() {
        let data = BytesMut::from(&[0x00][..]); // Only 1 byte, need 2
        let mut reader = BytesReader::new(data);
        let result = RtpH264UnPacker::skip_mtap_ts_offset(&mut reader, define::MTAP_16);
        assert!(result.is_err());
    }

    #[test]
    fn test_skip_mtap_ts_offset_mtap24_insufficient_data() {
        let data = BytesMut::from(&[0x00, 0x01][..]); // Only 2 bytes, need 3
        let mut reader = BytesReader::new(data);
        let result = RtpH264UnPacker::skip_mtap_ts_offset(&mut reader, define::MTAP_24);
        assert!(result.is_err());
    }

    // ========== End-of-Access-Unit Marker Tests (RFC 6184) ==========
    //
    // RFC 6184 Section 5.1: "The RTP marker bit is set to 1 to indicate the
    // last RTP packet of an access unit."

    #[tokio::test]
    async fn test_marker_on_last_nal_even_with_trailing_non_vcl() {
        let mock_io = Arc::new(Mutex::new(
            Box::new(MockNetIO::new()) as Box<dyn TNetIO + Send + Sync>
        ));
        let mut packer = RtpH264Packer::new(96, 12345, 0, 1500, mock_io);

        let markers = Arc::new(std::sync::Mutex::new(Vec::<u8>::new()));
        let markers_clone = markers.clone();
        packer.on_packet_handler(Box::new(move |_io, packet| {
            markers_clone.lock().unwrap().push(packet.header.marker);
            Box::pin(async move { Ok(()) })
        }));

        let mut frame = BytesMut::new();
        frame.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
        frame.extend_from_slice(&[0x65, 0x88, 0x84, 0x21]);
        frame.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
        frame.extend_from_slice(&[0x06, 0x05, 0xff, 0xff]);

        let result = packer.pack(&mut frame, 1000).await;
        assert!(result.is_ok());

        let recorded_markers = markers.lock().unwrap();
        assert_eq!(
            recorded_markers.len(),
            2,
            "Should have 2 packets (IDR + SEI)"
        );
        assert_eq!(
            recorded_markers[0], 0,
            "First packet (IDR VCL) should NOT have marker"
        );
        assert_eq!(
            recorded_markers[1], 1,
            "Last packet (SEI non-VCL) SHOULD have marker - end of access unit"
        );
    }

    #[tokio::test]
    async fn test_marker_exactly_one_per_multi_nal_access_unit() {
        let mock_io = Arc::new(Mutex::new(
            Box::new(MockNetIO::new()) as Box<dyn TNetIO + Send + Sync>
        ));
        let mut packer = RtpH264Packer::new(96, 12345, 0, 1500, mock_io);

        let markers = Arc::new(std::sync::Mutex::new(Vec::<u8>::new()));
        let markers_clone = markers.clone();
        packer.on_packet_handler(Box::new(move |_io, packet| {
            markers_clone.lock().unwrap().push(packet.header.marker);
            Box::pin(async move { Ok(()) })
        }));

        let mut frame = BytesMut::new();
        frame.extend_from_slice(&[0x00, 0x00, 0x00, 0x01, 0x67, 0x42, 0x00]);
        frame.extend_from_slice(&[0x00, 0x00, 0x00, 0x01, 0x68, 0xce, 0x38]);
        frame.extend_from_slice(&[0x00, 0x00, 0x00, 0x01, 0x65, 0x88, 0x84]);
        frame.extend_from_slice(&[0x00, 0x00, 0x00, 0x01, 0x41, 0x9a, 0x22]);
        frame.extend_from_slice(&[0x00, 0x00, 0x00, 0x01, 0x06, 0x05, 0xff]);

        let result = packer.pack(&mut frame, 1000).await;
        assert!(result.is_ok());

        let recorded_markers = markers.lock().unwrap();
        assert_eq!(recorded_markers.len(), 5, "Should have 5 NAL packets");
        let marker_count = recorded_markers.iter().filter(|&&m| m == 1).count();
        assert_eq!(
            marker_count, 1,
            "Exactly one marker=1 expected at end of access unit, got markers: {:?}",
            *recorded_markers
        );
        assert_eq!(
            *recorded_markers.last().unwrap(),
            1,
            "Final packet must have marker=1"
        );
    }

    #[tokio::test]
    async fn test_marker_on_fu_a_single_large_vcl_at_access_unit_end() {
        let mock_io = Arc::new(Mutex::new(
            Box::new(MockNetIO::new()) as Box<dyn TNetIO + Send + Sync>
        ));
        let mut packer = RtpH264Packer::new(96, 12345, 0, 1500, mock_io);

        let markers = Arc::new(std::sync::Mutex::new(Vec::<u8>::new()));
        let fu_end_marker = Arc::new(std::sync::Mutex::new(None::<u8>));
        let markers_clone = markers.clone();
        let fu_end_marker_clone = fu_end_marker.clone();
        packer.on_packet_handler(Box::new(move |_io, packet| {
            markers_clone.lock().unwrap().push(packet.header.marker);
            if packet.payload.len() >= 2 {
                let fu_indicator = packet.payload[0];
                let fu_header = packet.payload[1];
                if (fu_indicator & 0x1F) == define::FU_A && (fu_header & define::FU_END) > 0 {
                    *fu_end_marker_clone.lock().unwrap() = Some(packet.header.marker);
                }
            }
            Box::pin(async move { Ok(()) })
        }));

        let nalu = create_large_nalu(2000);
        let result = packer.pack_fu_a(nalu).await;
        assert!(result.is_ok());

        let recorded_markers = markers.lock().unwrap();
        assert!(
            recorded_markers.len() >= 2,
            "Should have multiple FU-A fragments, got {} packets",
            recorded_markers.len()
        );
        let fu_end_marker_val = fu_end_marker.lock().unwrap();
        assert_eq!(
            *fu_end_marker_val,
            Some(1),
            "FU-A last fragment MUST have marker=1 when at access unit end (single VCL)"
        );
    }
}
