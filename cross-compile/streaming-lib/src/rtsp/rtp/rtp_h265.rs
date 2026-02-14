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

pub struct RtpH265Packer {
    header: RtpHeader,
    mtu: usize,
    io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>,
    on_packet_handler: Option<OnRtpPacketFn>,
    on_packet_for_rtcp_handler: Option<OnRtpPacketFn2>,
}

const MAX_FU_BUFFER_SIZE: usize = 1024 * 1024;

impl RtpH265Packer {
    pub fn new(
        payload_type: u8,
        ssrc: u32,
        init_seq: u16,
        mtu: usize,
        io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>,
    ) -> Self {
        RtpH265Packer {
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

    pub async fn pack_fu(&mut self, nalu: BytesMut) -> Result<(), PackerError> {
        let mut nalu_reader = BytesReader::new(nalu);
        /* NALU header
        0               1
        0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5
        +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
        |F|    Type   |  LayerId  | TID |
        +-------------+-----------------+

        Forbidden zero(F) : 1 bit
        NAL unit type(Type) : 6 bits
        NUH layer ID(LayerId) : 6 bits
        NUH temporal ID plus 1 (TID) : 3 bits
        */
        let nalu_header_1st_byte = nalu_reader.read_u8()?;
        let nalu_header_2nd_byte = nalu_reader.read_u8()?;

        /* The PayloadHdr needs replace Type with the FU type value(49) */
        let payload_hdr: u16 = ((nalu_header_1st_byte as u16 & 0x81) | ((define::FU as u16) << 1))
            << 8
            | nalu_header_2nd_byte as u16;
        /* FU header
        +---------------+
        |0|1|2|3|4|5|6|7|
        +-+-+-+-+-+-+-+-+
        |S|E|   FuType  |
        +---------------+
        */
        /*set FuType from NALU header's Type */
        let mut fu_header = (nalu_header_1st_byte >> 1) & 0x3F | define::FU_START;

        let mut left_nalu_bytes: usize = nalu_reader.len();
        let mut fu_payload_len: usize;

        while left_nalu_bytes > 0 {
            /* 3 = PayloadHdr(2 bytes) + FU header(1 byte) */
            if left_nalu_bytes + define::RTP_FIXED_HEADER_LEN <= self.mtu - 3 {
                fu_header = ((nalu_header_1st_byte >> 1) & 0x3F) | define::FU_END;
                fu_payload_len = left_nalu_bytes;
            } else {
                fu_payload_len = self.mtu - define::RTP_FIXED_HEADER_LEN - 3;
            }

            let fu_payload = nalu_reader.read_bytes(fu_payload_len)?;

            let mut packet = RtpPacket::new(self.header.clone());
            packet.payload.put_u16(payload_hdr);
            packet.payload.put_u8(fu_header);
            packet.payload.put(fu_payload);
            packet.header.marker = if fu_header & define::FU_END > 0 { 1 } else { 0 };

            if fu_header & define::FU_START > 0 {
                fu_header &= 0x7F
            }

            if let Some(f) = &self.on_packet_for_rtcp_handler {
                f(packet.clone()).await;
            }

            if let Some(f) = &self.on_packet_handler {
                f(self.io.clone(), packet).await?;
            }
            left_nalu_bytes = nalu_reader.len();
            self.header.seq_number += 1;
        }

        Ok(())
    }
    pub async fn pack_single(&mut self, nalu: BytesMut) -> Result<(), PackerError> {
        let mut packet = RtpPacket::new(self.header.clone());
        packet.header.marker = 1;
        packet.payload.put(nalu);

        self.header.seq_number += 1;

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
impl TPacker for RtpH265Packer {
    async fn pack(&mut self, nalus: &mut BytesMut, timestamp: u32) -> Result<(), PackerError> {
        self.header.timestamp = timestamp;
        utils::split_annexb_and_process(nalus, self).await?;
        Ok(())
    }
    fn on_packet_handler(&mut self, f: OnRtpPacketFn) {
        self.on_packet_handler = Some(f);
    }
}

impl TRtpReceiverForRtcp for RtpH265Packer {
    fn on_packet_for_rtcp_handler(&mut self, f: OnRtpPacketFn2) {
        self.on_packet_for_rtcp_handler = Some(f);
    }
}

#[async_trait]
impl TVideoPacker for RtpH265Packer {
    async fn pack_nalu(&mut self, nalu: BytesMut) -> Result<(), PackerError> {
        if nalu.len() + define::RTP_FIXED_HEADER_LEN <= self.mtu {
            self.pack_single(nalu).await?;
        } else {
            self.pack_fu(nalu).await?;
        }
        Ok(())
    }
}

#[derive(Default)]
pub struct RtpH265UnPacker {
    sequence_number: u16,
    expected_seq: Option<u16>,
    timestamp: u32,
    fu_buffer: BytesMut,
    using_donl_field: bool,
    on_frame_handler: Option<OnFrameFn>,
    on_packet_for_rtcp_handler: Option<OnRtpPacketFn2>,
}

#[async_trait]
impl TUnPacker for RtpH265UnPacker {
    async fn unpack(&mut self, reader: &mut BytesReader) -> Result<(), UnPackerError> {
        let rtp_packet = RtpPacket::unmarshal(reader)?;

        if let Some(f) = &self.on_packet_for_rtcp_handler {
            f(rtp_packet.clone()).await;
        }

        self.timestamp = rtp_packet.header.timestamp;
        self.sequence_number = rtp_packet.header.seq_number;

        if let Some(packet_type) = rtp_packet.payload.first() {
            match *packet_type >> 1 & 0x3F {
                define::FU => {
                    return self.unpack_fu(rtp_packet.payload.clone());
                }
                define::AP => {
                    return self.unpack_ap(rtp_packet.payload);
                }
                define::PACI => return Ok(()),

                _ => {
                    return self.unpack_single(rtp_packet.payload.clone());
                }
            }
        }

        Ok(())
    }

    fn on_frame_handler(&mut self, f: OnFrameFn) {
        self.on_frame_handler = Some(f);
    }
}

impl RtpH265UnPacker {
    pub fn new() -> Self {
        RtpH265UnPacker::default()
    }

    fn unpack_single(&mut self, payload: BytesMut) -> Result<(), UnPackerError> {
        let mut annexb_payload = BytesMut::new();
        annexb_payload.extend_from_slice(&define::ANNEXB_NALU_START_CODE);
        annexb_payload.put(payload);

        if let Some(f) = &self.on_frame_handler {
            f(FrameData::Video {
                timestamp: self.timestamp,
                data: annexb_payload,
            })?;
        }
        Ok(())
    }

    /*
     0               1               2               3
     0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    |                          RTP Header                           |
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    |      PayloadHdr (Type=48)     |           NALU 1 DONL         |
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    |           NALU 1 Size         |            NALU 1 HDR         |
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    |                                                               |
    |                         NALU 1 Data . . .                     |
    |                                                               |
    +     . . .     +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    |               |  NALU 2 DOND  |            NALU 2 Size        |
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    |          NALU 2 HDR           |                               |
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+            NALU 2 Data        |
    |                                                               |
    |         . . .                 +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    |                               :    ...OPTIONAL RTP padding    |
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    */

    fn unpack_ap(&mut self, rtp_payload: BytesMut) -> Result<(), UnPackerError> {
        let mut payload_reader = BytesReader::new(rtp_payload);
        /*read PayloadHdr*/
        payload_reader.read_bytes(2)?;

        while !payload_reader.is_empty() {
            if self.using_donl_field {
                /*read DONL*/
                payload_reader.read_bytes(2)?;
            }
            /*read NALU Size*/
            let nalu_len = payload_reader.read_u16::<BigEndian>()? as usize;
            /*read NALU HDR + Data */
            let nalu = payload_reader.read_bytes(nalu_len)?;

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

    /*
    0               1
    0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    |F|    Type   |  LayerId  | TID |
    +-------------+-----------------+

    Forbidden zero(F) : 1 bit
    NAL unit type(Type) : 6 bits
    NUH layer ID(LayerId) : 6 bits
    NUH temporal ID plus 1 (TID) : 3 bits
    */

    /*
     0               1               2               3
     0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    |     PayloadHdr (Type=49)      |    FU header  |  DONL (cond)  |
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-|
    |  DONL (cond)  |                                               |
    |-+-+-+-+-+-+-+-+                                               |
    |                           FU payload                          |
    |                                                               |
    |                               +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    |                               :    ...OPTIONAL RTP padding    |
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    /* FU header */
    +---------------+
    |0|1|2|3|4|5|6|7|
    +-+-+-+-+-+-+-+-+
    |S|E|   FuType  |
    +---------------+
    */
    fn unpack_fu(&mut self, rtp_payload: BytesMut) -> Result<(), UnPackerError> {
        let mut payload_reader = BytesReader::new(rtp_payload);
        let payload_header_1st_byte = payload_reader.read_u8()?;
        let payload_header_2nd_byte = payload_reader.read_u8()?;
        let fu_header = payload_reader.read_u8()?;
        if self.using_donl_field {
            payload_reader.read_bytes(2)?;
        }

        if utils::is_fu_start(fu_header) {
            /*set NAL UNIT type 2 bytes */
            //replace Type of PayloadHdr with the FuType of FU header
            let nal_1st_byte = (payload_header_1st_byte & 0x81) | ((fu_header & 0x3F) << 1);
            self.fu_buffer.put_u8(nal_1st_byte);
            self.fu_buffer.put_u8(payload_header_2nd_byte);
            self.expected_seq = Some(self.sequence_number.wrapping_add(1));
        } else if let Some(expected) = self.expected_seq
            && self.sequence_number != expected
        {
            log::warn!(
                "rtp h265 fu sequence discontinuity: expected {}, got {}",
                expected,
                self.sequence_number
            );
            self.fu_buffer.clear();
            self.expected_seq = None;
            return Ok(());
        }

        if self.fu_buffer.len() + payload_reader.len() > MAX_FU_BUFFER_SIZE {
            log::warn!("rtp h265 fu buffer overflow; dropping fragment");
            self.fu_buffer.clear();
            self.expected_seq = None;
            return Ok(());
        }

        self.fu_buffer.put(payload_reader.extract_remaining_bytes());

        if utils::is_fu_end(fu_header) {
            let mut payload = BytesMut::new();
            payload.extend_from_slice(&define::ANNEXB_NALU_START_CODE);
            let fu_payload = std::mem::take(&mut self.fu_buffer);
            payload.put(fu_payload);
            self.expected_seq = None;

            if let Some(f) = &self.on_frame_handler {
                f(FrameData::Video {
                    timestamp: self.timestamp,
                    data: payload,
                })?;
            }
        } else {
            self.expected_seq = Some(self.sequence_number.wrapping_add(1));
        }

        Ok(())
    }
}

impl TRtpReceiverForRtcp for RtpH265UnPacker {
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

    fn create_test_h265_nalu() -> BytesMut {
        // Create a simple H.265 NAL unit (VPS type 32)
        let mut nalu = BytesMut::new();
        nalu.put_u8(0x40); // First byte: F=0, Type=32 (VPS), LayerId=0
        nalu.put_u8(0x01); // Second byte: TID=1
        nalu.extend_from_slice(&[0x00, 0x00, 0x00, 0x01, 0x40, 0x01, 0x0c, 0x01]);
        nalu
    }

    fn create_large_h265_nalu(size: usize) -> BytesMut {
        let mut nalu = BytesMut::new();
        nalu.put_u8(0x26); // First byte: F=0, Type=19 (IDR), LayerId=0
        nalu.put_u8(0x01); // Second byte: TID=1
        nalu.extend_from_slice(&vec![0x00; size]);
        nalu
    }

    #[tokio::test]
    async fn test_rtp_h265_packer_new() {
        let mock_io = Arc::new(Mutex::new(
            Box::new(MockNetIO::new()) as Box<dyn TNetIO + Send + Sync>
        ));
        let packer = RtpH265Packer::new(96, 12345, 0, 1500, mock_io);
        assert_eq!(packer.header.payload_type, 96);
        assert_eq!(packer.header.ssrc, 12345);
        assert_eq!(packer.header.seq_number, 0);
        assert_eq!(packer.mtu, 1500);
    }

    #[tokio::test]
    async fn test_rtp_h265_packer_pack_single_small_nalu() {
        let mock_io = Arc::new(Mutex::new(
            Box::new(MockNetIO::new()) as Box<dyn TNetIO + Send + Sync>
        ));
        let mut packer = RtpH265Packer::new(96, 12345, 0, 1500, mock_io);

        let packet_count = std::sync::Arc::new(std::sync::Mutex::new(0));
        let packet_count_clone = packet_count.clone();
        packer.on_packet_handler(Box::new(move |_io, packet| {
            *packet_count_clone.lock().unwrap() += 1;
            assert_eq!(packet.header.marker, 1);
            assert_eq!(packet.header.payload_type, 96);
            assert!(!packet.payload.is_empty());
            Box::pin(async move { Ok(()) })
        }));

        let nalu = create_test_h265_nalu();
        let result = packer.pack_single(nalu).await;
        assert!(result.is_ok());
        assert_eq!(*packet_count.lock().unwrap(), 1);
    }

    #[tokio::test]
    async fn test_rtp_h265_packer_pack_fu_large_nalu() {
        let mock_io = Arc::new(Mutex::new(
            Box::new(MockNetIO::new()) as Box<dyn TNetIO + Send + Sync>
        ));
        let mut packer = RtpH265Packer::new(96, 12345, 0, 1500, mock_io);

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
            // Verify FU structure (PayloadHdr + FU header)
            if packet.payload.len() >= 3 {
                let payload_hdr_1st = packet.payload[0];
                let _payload_hdr_2nd = packet.payload[1];
                let fu_header = packet.payload[2];
                // Check that PayloadHdr has FU type (49)
                assert_eq!((payload_hdr_1st >> 1) & 0x3F, define::FU);
                // Check FU header has S or E bit set
                assert!(
                    fu_header & define::FU_START > 0
                        || fu_header & define::FU_END > 0
                        || *count > 1
                );
            }
            Box::pin(async move { Ok(()) })
        }));

        // Create a NAL unit larger than MTU
        let nalu = create_large_h265_nalu(2000);
        let result = packer.pack_fu(nalu).await;
        assert!(result.is_ok());
        assert!(*packet_count.lock().unwrap() > 1); // Should be fragmented
        assert_eq!(*first_packet_marker.lock().unwrap(), 0); // First packet should not have marker
        assert_eq!(*last_packet_marker.lock().unwrap(), 1); // Last packet should have marker
    }

    #[tokio::test]
    async fn test_rtp_h265_packer_pack_nalu_small() {
        let mock_io = Arc::new(Mutex::new(
            Box::new(MockNetIO::new()) as Box<dyn TNetIO + Send + Sync>
        ));
        let mut packer = RtpH265Packer::new(96, 12345, 0, 1500, mock_io);

        let packet_count = std::sync::Arc::new(std::sync::Mutex::new(0));
        let packet_count_clone = packet_count.clone();
        packer.on_packet_handler(Box::new(move |_io, packet| {
            *packet_count_clone.lock().unwrap() += 1;
            assert_eq!(packet.header.marker, 1);
            Box::pin(async move { Ok(()) })
        }));

        let nalu = create_test_h265_nalu();
        let result = packer.pack_nalu(nalu).await;
        assert!(result.is_ok());
        assert_eq!(*packet_count.lock().unwrap(), 1);
    }

    #[tokio::test]
    async fn test_rtp_h265_packer_pack_nalu_large() {
        let mock_io = Arc::new(Mutex::new(
            Box::new(MockNetIO::new()) as Box<dyn TNetIO + Send + Sync>
        ));
        let mut packer = RtpH265Packer::new(96, 12345, 0, 1500, mock_io);

        let packet_count = std::sync::Arc::new(std::sync::Mutex::new(0));
        let packet_count_clone = packet_count.clone();
        packer.on_packet_handler(Box::new(move |_io, _packet| {
            *packet_count_clone.lock().unwrap() += 1;
            Box::pin(async move { Ok(()) })
        }));

        // Create a NAL unit larger than MTU
        let nalu = create_large_h265_nalu(2000);
        let result = packer.pack_nalu(nalu).await;
        assert!(result.is_ok());
        assert!(*packet_count.lock().unwrap() > 1); // Should be fragmented
    }

    #[tokio::test]
    async fn test_rtp_h265_packer_sequence_number_increment() {
        let mock_io = Arc::new(Mutex::new(
            Box::new(MockNetIO::new()) as Box<dyn TNetIO + Send + Sync>
        ));
        let mut packer = RtpH265Packer::new(96, 12345, 100, 1500, mock_io);

        let seq_numbers = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let seq_numbers_clone = seq_numbers.clone();
        packer.on_packet_handler(Box::new(move |_io, packet| {
            seq_numbers_clone
                .lock()
                .unwrap()
                .push(packet.header.seq_number);
            Box::pin(async move { Ok(()) })
        }));

        let nalu = create_large_h265_nalu(2000);
        let _ = packer.pack_fu(nalu).await;

        // Verify sequence numbers increment
        let seqs = seq_numbers.lock().unwrap();
        assert!(seqs.len() > 1);
        for i in 1..seqs.len() {
            assert_eq!(seqs[i], seqs[i - 1] + 1);
        }
    }

    #[tokio::test]
    async fn test_rtp_h265_unpacker_new() {
        let unpacker = RtpH265UnPacker::new();
        assert_eq!(unpacker.sequence_number, 0);
        assert_eq!(unpacker.timestamp, 0);
        assert!(unpacker.fu_buffer.is_empty());
        assert!(!unpacker.using_donl_field);
    }

    #[tokio::test]
    async fn test_rtp_h265_unpacker_unpack_single() {
        let mut unpacker = RtpH265UnPacker::new();
        let frame_data = std::sync::Arc::new(std::sync::Mutex::new(None::<FrameData>));
        let frame_data_clone = frame_data.clone();

        unpacker.on_frame_handler(Box::new(move |frame| {
            *frame_data_clone.lock().unwrap() = Some(frame);
            Ok(())
        }));

        // Create a single NAL unit RTP packet
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
        let nalu = create_test_h265_nalu();
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
    async fn test_rtp_h265_unpacker_unpack_fu() {
        let mut unpacker = RtpH265UnPacker::new();
        let frame_count = std::sync::Arc::new(std::sync::Mutex::new(0));
        let frame_count_clone = frame_count.clone();

        unpacker.on_frame_handler(Box::new(move |_frame| {
            *frame_count_clone.lock().unwrap() += 1;
            Ok(())
        }));

        // Create FU start packet
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
        // PayloadHdr with FU type (49)
        start_packet.payload.put_u8(0x62); // F=0, Type=49 (FU), LayerId=0
        start_packet.payload.put_u8(0x01); // TID=1
        // FU header with S bit
        start_packet.payload.put_u8(0x83); // S=1, E=0, FuType=19 (IDR)
        start_packet.payload.extend_from_slice(&[0x01, 0x02, 0x03]);

        let start_bytes = start_packet.marshal().unwrap();
        let mut reader = BytesReader::new(start_bytes);
        let _ = unpacker.unpack(&mut reader).await;

        // Create FU end packet
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
        end_packet.payload.put_u8(0x62); // PayloadHdr
        end_packet.payload.put_u8(0x01); // TID
        end_packet.payload.put_u8(0x43); // E=1, FuType=19
        end_packet.payload.extend_from_slice(&[0x04, 0x05, 0x06]);

        let end_bytes = end_packet.marshal().unwrap();
        let mut reader = BytesReader::new(end_bytes);
        let result = unpacker.unpack(&mut reader).await;

        assert!(result.is_ok());
        assert_eq!(*frame_count.lock().unwrap(), 1); // Should receive complete frame
    }

    #[tokio::test]
    async fn test_rtp_h265_unpacker_unpack_ap() {
        let mut unpacker = RtpH265UnPacker::new();
        let frame_count = std::sync::Arc::new(std::sync::Mutex::new(0));
        let frame_count_clone = frame_count.clone();

        unpacker.on_frame_handler(Box::new(move |_frame| {
            *frame_count_clone.lock().unwrap() += 1;
            Ok(())
        }));

        // Create AP (Aggregation Packet) with two NAL units
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

        // PayloadHdr with AP type (48)
        packet.payload.put_u8(0x60); // F=0, Type=48 (AP), LayerId=0
        packet.payload.put_u8(0x01); // TID=1

        // First NAL unit (without DONL)
        packet.payload.put_u16(4); // Size
        packet.payload.extend_from_slice(&[0x40, 0x01, 0x00, 0x00]);

        // Second NAL unit
        packet.payload.put_u16(2); // Size
        packet.payload.extend_from_slice(&[0x42, 0x01]);

        let packet_bytes = packet.marshal().unwrap();
        let mut reader = BytesReader::new(packet_bytes);

        let result = unpacker.unpack(&mut reader).await;
        assert!(result.is_ok());
        assert_eq!(*frame_count.lock().unwrap(), 2); // Should receive two frames
    }

    #[tokio::test]
    async fn test_rtp_h265_unpacker_timestamp_preservation() {
        let mut unpacker = RtpH265UnPacker::new();
        let received_timestamp = std::sync::Arc::new(std::sync::Mutex::new(0u32));
        let received_timestamp_clone = received_timestamp.clone();

        unpacker.on_frame_handler(Box::new(move |frame| {
            if let FrameData::Video { timestamp, .. } = frame {
                *received_timestamp_clone.lock().unwrap() = timestamp;
            }
            Ok(())
        }));

        let mut packet = RtpPacket {
            header: RtpHeader {
                payload_type: 96,
                seq_number: 1,
                timestamp: 5000,
                ssrc: 12345,
                version: 2,
                marker: 1,
                ..Default::default()
            },
            ..Default::default()
        };
        let nalu = create_test_h265_nalu();
        packet.payload.put(nalu);

        let packet_bytes = packet.marshal().unwrap();
        let mut reader = BytesReader::new(packet_bytes);

        let _ = unpacker.unpack(&mut reader).await;
        assert_eq!(*received_timestamp.lock().unwrap(), 5000);
    }

    #[tokio::test]
    async fn test_rtp_h265_unpacker_paci_type_ignored() {
        let mut unpacker = RtpH265UnPacker::new();
        let frame_count = std::sync::Arc::new(std::sync::Mutex::new(0));
        let frame_count_clone = frame_count.clone();

        unpacker.on_frame_handler(Box::new(move |_frame| {
            *frame_count_clone.lock().unwrap() += 1;
            Ok(())
        }));

        // Create a PACI packet (type 50) — should return Ok(()) without emitting a frame
        let mut packet = RtpPacket {
            header: RtpHeader {
                payload_type: 96,
                seq_number: 1,
                timestamp: 1000,
                ssrc: 12345,
                version: 2,
                ..Default::default()
            },
            ..Default::default()
        };
        // PayloadHdr with PACI type (50): first byte = (50 << 1) = 0x64
        packet.payload.put_u8(0x64); // F=0, Type=50 (PACI)
        packet.payload.put_u8(0x01); // TID=1
        packet.payload.extend_from_slice(&[0x01, 0x02, 0x03]);

        let packet_bytes = packet.marshal().unwrap();
        let mut reader = BytesReader::new(packet_bytes);

        let result = unpacker.unpack(&mut reader).await;
        assert!(result.is_ok());
        assert_eq!(*frame_count.lock().unwrap(), 0); // PACI emits no frames
    }

    #[tokio::test]
    async fn test_rtp_h265_unpacker_empty_payload() {
        let mut unpacker = RtpH265UnPacker::new();
        let frame_count = std::sync::Arc::new(std::sync::Mutex::new(0));
        let frame_count_clone = frame_count.clone();

        unpacker.on_frame_handler(Box::new(move |_frame| {
            *frame_count_clone.lock().unwrap() += 1;
            Ok(())
        }));

        // Create RTP packet with empty payload
        let packet = RtpPacket {
            header: RtpHeader {
                payload_type: 96,
                seq_number: 1,
                timestamp: 1000,
                ssrc: 12345,
                version: 2,
                ..Default::default()
            },
            ..Default::default()
        };

        let packet_bytes = packet.marshal().unwrap();
        let mut reader = BytesReader::new(packet_bytes);

        let result = unpacker.unpack(&mut reader).await;
        assert!(result.is_ok());
        assert_eq!(*frame_count.lock().unwrap(), 0); // No frames from empty payload
    }

    #[tokio::test]
    async fn test_rtp_h265_unpacker_fu_sequence_discontinuity() {
        let mut unpacker = RtpH265UnPacker::new();
        let frame_count = std::sync::Arc::new(std::sync::Mutex::new(0));
        let frame_count_clone = frame_count.clone();

        unpacker.on_frame_handler(Box::new(move |_frame| {
            *frame_count_clone.lock().unwrap() += 1;
            Ok(())
        }));

        // Send FU start packet with seq=1
        let mut start_packet = RtpPacket {
            header: RtpHeader {
                payload_type: 96,
                seq_number: 1,
                timestamp: 1000,
                ssrc: 12345,
                version: 2,
                ..Default::default()
            },
            ..Default::default()
        };
        start_packet.payload.put_u8(0x62); // PayloadHdr: Type=49 (FU)
        start_packet.payload.put_u8(0x01); // TID=1
        start_packet.payload.put_u8(0x83); // FU header: S=1, FuType=19
        start_packet.payload.extend_from_slice(&[0x01, 0x02, 0x03]);

        let start_bytes = start_packet.marshal().unwrap();
        let mut reader = BytesReader::new(start_bytes);
        let _ = unpacker.unpack(&mut reader).await;

        // Send FU middle packet with seq=5 (discontinuity: expected 2, got 5)
        let mut mid_packet = RtpPacket {
            header: RtpHeader {
                payload_type: 96,
                seq_number: 5, // Discontinuity!
                timestamp: 1000,
                ssrc: 12345,
                version: 2,
                ..Default::default()
            },
            ..Default::default()
        };
        mid_packet.payload.put_u8(0x62); // PayloadHdr
        mid_packet.payload.put_u8(0x01);
        mid_packet.payload.put_u8(0x13); // FU header: S=0, E=0, FuType=19 (middle)
        mid_packet.payload.extend_from_slice(&[0x04, 0x05, 0x06]);

        let mid_bytes = mid_packet.marshal().unwrap();
        let mut reader = BytesReader::new(mid_bytes);
        let result = unpacker.unpack(&mut reader).await;
        assert!(result.is_ok());

        // No frame should be emitted due to sequence discontinuity
        assert_eq!(*frame_count.lock().unwrap(), 0);
    }

    #[tokio::test]
    async fn test_rtp_h265_unpacker_fu_buffer_overflow_protection() {
        let mut unpacker = RtpH265UnPacker::new();
        let frame_count = std::sync::Arc::new(std::sync::Mutex::new(0));
        let frame_count_clone = frame_count.clone();

        unpacker.on_frame_handler(Box::new(move |_frame| {
            *frame_count_clone.lock().unwrap() += 1;
            Ok(())
        }));

        // Send FU start packet to initialize the fu_buffer
        let mut start_packet = RtpPacket {
            header: RtpHeader {
                payload_type: 96,
                seq_number: 1,
                timestamp: 1000,
                ssrc: 12345,
                version: 2,
                ..Default::default()
            },
            ..Default::default()
        };
        start_packet.payload.put_u8(0x62); // PayloadHdr: Type=49 (FU)
        start_packet.payload.put_u8(0x01);
        start_packet.payload.put_u8(0x93); // FU header: S=1, FuType=19
        // Fill with near-maximum data to trigger overflow on next packet
        let large_data = vec![0xAA; 1024 * 1024 - 10];
        start_packet.payload.extend_from_slice(&large_data);

        let start_bytes = start_packet.marshal().unwrap();
        let mut reader = BytesReader::new(start_bytes);
        let _ = unpacker.unpack(&mut reader).await;

        // Send FU middle packet that would exceed MAX_FU_BUFFER_SIZE
        let mut mid_packet = RtpPacket {
            header: RtpHeader {
                payload_type: 96,
                seq_number: 2,
                timestamp: 1000,
                ssrc: 12345,
                version: 2,
                ..Default::default()
            },
            ..Default::default()
        };
        mid_packet.payload.put_u8(0x62); // PayloadHdr
        mid_packet.payload.put_u8(0x01);
        mid_packet.payload.put_u8(0x13); // FU header: S=0, E=0, FuType=19
        mid_packet.payload.extend_from_slice(&vec![0xBB; 100]);

        let mid_bytes = mid_packet.marshal().unwrap();
        let mut reader = BytesReader::new(mid_bytes);
        let result = unpacker.unpack(&mut reader).await;
        assert!(result.is_ok());

        // Buffer should have been cleared — no frame emitted
        assert_eq!(*frame_count.lock().unwrap(), 0);
    }

    #[tokio::test]
    async fn test_rtp_h265_unpacker_rtcp_handler() {
        let mut unpacker = RtpH265UnPacker::new();

        let rtcp_count = std::sync::Arc::new(std::sync::Mutex::new(0));
        let rtcp_count_clone = rtcp_count.clone();

        unpacker.on_packet_for_rtcp_handler(Box::new(move |_packet| {
            *rtcp_count_clone.lock().unwrap() += 1;
            Box::pin(async {})
        }));

        unpacker.on_frame_handler(Box::new(move |_frame| Ok(())));

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
        let nalu = create_test_h265_nalu();
        packet.payload.put(nalu);

        let packet_bytes = packet.marshal().unwrap();
        let mut reader = BytesReader::new(packet_bytes);

        let _ = unpacker.unpack(&mut reader).await;
        assert_eq!(*rtcp_count.lock().unwrap(), 1);
    }

    #[tokio::test]
    async fn test_rtp_h265_packer_rtcp_handler() {
        let mock_io = Arc::new(Mutex::new(
            Box::new(MockNetIO::new()) as Box<dyn TNetIO + Send + Sync>
        ));
        let mut packer = RtpH265Packer::new(96, 12345, 0, 1500, mock_io);

        let rtcp_count = std::sync::Arc::new(std::sync::Mutex::new(0));
        let rtcp_count_clone = rtcp_count.clone();

        packer.on_packet_for_rtcp_handler(Box::new(move |_packet| {
            *rtcp_count_clone.lock().unwrap() += 1;
            Box::pin(async {})
        }));

        packer.on_packet_handler(Box::new(move |_io, _packet| {
            Box::pin(async move { Ok(()) })
        }));

        let nalu = create_test_h265_nalu();
        let _ = packer.pack_single(nalu).await;

        assert_eq!(*rtcp_count.lock().unwrap(), 1);
    }

    #[test]
    fn test_rtp_h265_unpacker_default() {
        let unpacker = RtpH265UnPacker::default();
        assert_eq!(unpacker.sequence_number, 0);
        assert_eq!(unpacker.timestamp, 0);
        assert!(unpacker.fu_buffer.is_empty());
        assert!(!unpacker.using_donl_field);
        assert!(unpacker.on_frame_handler.is_none());
        assert!(unpacker.on_packet_for_rtcp_handler.is_none());
    }
}
