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
use crate::bytesio::bytes_reader::BytesReader;
use crate::streamhub::define::FrameData;
use async_trait::async_trait;
use byteorder::BigEndian;
use bytes::{BufMut, BytesMut};
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct RtpH264Packer {
    header: RtpHeader,
    mtu: usize,
    on_packet_handler: Option<OnRtpPacketFn>,
    on_packet_for_rtcp_handler: Option<OnRtpPacketFn2>,
    io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>,
}

impl RtpH264Packer {
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

    pub async fn pack_fu_a(&mut self, nalu: BytesMut) -> Result<(), PackerError> {
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
            packet.header.marker = if fu_header & define::FU_END > 0 { 1 } else { 0 };

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
    pub async fn pack_single(&mut self, nalu: BytesMut) -> Result<(), PackerError> {
        let mut packet = RtpPacket::new(self.header.clone());
        packet.header.marker = 1;
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
}

#[async_trait]
impl TPacker for RtpH264Packer {
    //pack annexb h264 data
    async fn pack(&mut self, nalus: &mut BytesMut, timestamp: u32) -> Result<(), PackerError> {
        self.header.timestamp = timestamp; // ((timestamp as u64 * self.clock_rate as u64) / 1000) as u32;
        utils::split_annexb_and_process(nalus, self).await?;
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
        if nalu.len() + define::RTP_FIXED_HEADER_LEN <= self.mtu {
            self.pack_single(nalu).await?;
        } else {
            self.pack_fu_a(nalu).await?;
        }
        Ok(())
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

        if let Some(packet_type) = rtp_packet.payload.first() {
            match *packet_type & 0x1F {
                1..=23 => {
                    return self.unpack_single(rtp_packet.payload.clone(), *packet_type);
                }
                define::STAP_A | define::STAP_B => {
                    return self.unpack_stap(rtp_packet.payload.clone(), *packet_type);
                }
                define::MTAP_16 | define::MTAP_24 => {
                    return self.unpack_mtap(rtp_packet.payload.clone(), *packet_type);
                }
                define::FU_A | define::FU_B => {
                    return self.unpack_fu(rtp_packet.payload.clone(), *packet_type);
                }
                _ => {}
            }
        }

        Ok(())
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
        //read NAL HDR
        payload_reader.read_u8()?;
        //read decoding_order_number_base
        payload_reader.read_u16::<BigEndian>()?;

        while !payload_reader.is_empty() {
            //read nalu size
            let nalu_size = payload_reader.read_u16::<BigEndian>()? as usize;
            // read dond
            payload_reader.read_u8()?;
            // read TS offs - can be 0 (same timestamp as base) or any positive value
            let (ts, ts_bytes) = if t == define::MTAP_16 {
                (payload_reader.read_u16::<BigEndian>()? as u32, 2_usize)
            } else if t == define::MTAP_24 {
                (payload_reader.read_u24::<BigEndian>()?, 3_usize)
            } else {
                log::warn!("should not be here!");
                (0, 0)
            };
            // Note: ts can be 0 (indicates same timestamp as base), so no validation needed
            let nalu = payload_reader.read_bytes(nalu_size - ts_bytes - 1)?;

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
    use crate::bytesio::bytes_writer::BytesWriter;
    use crate::rtsp::rtp::utils::Marshal;
    use crate::streamhub::define::FrameData;
    use async_trait::async_trait;
    use byteorder::BigEndian;
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
        // Create a simple H.264 NAL unit (SPS type 7)
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

        let mut packet_count = 0;
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
        packer.on_packet_handler(Box::new(move |_io, packet| {
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

        // NALU 1
        packet.payload.put_u16(2); // Size
        packet.payload.put_u8(0); // DOND
        packet.payload.put_u16(100); // TS offset (Must be non-zero for test assertion)
        packet.payload.extend_from_slice(&[0x01, 0x01]);

        // NALU 2
        packet.payload.put_u16(2); // Size
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

        // NALU 1
        packet.payload.put_u16(2); // Size
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
}
