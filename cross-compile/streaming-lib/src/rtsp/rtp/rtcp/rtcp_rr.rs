use super::errors::RtcpError;
use super::rtcp_header::RtcpHeader;
use crate::bytesio::bytes_reader::BytesReader;
use crate::bytesio::bytes_writer::BytesWriter;
use crate::rtsp::rtp::utils::Marshal;
use crate::rtsp::rtp::utils::Unmarshal;
use byteorder::BigEndian;
use bytes::BytesMut;

#[derive(Debug, Clone, Default)]
pub struct ReportBlock {
    pub ssrc: u32,
    pub fraction_lost: u8,
    pub cumulative_num_of_packets_lost: u32,
    pub extended_highest_seq_number: u32,
    pub jitter: u32,
    pub lsr: u32,
    pub dlsr: u32,
}

impl Unmarshal<&mut BytesReader, Result<Self, RtcpError>> for ReportBlock {
    fn unmarshal(reader: &mut BytesReader) -> Result<Self, RtcpError>
    where
        Self: Sized,
    {
        Ok(ReportBlock {
            ssrc: reader.read_u32::<BigEndian>()?,
            fraction_lost: reader.read_u8()?,
            cumulative_num_of_packets_lost: reader.read_u24::<BigEndian>()?,
            extended_highest_seq_number: reader.read_u32::<BigEndian>()?,
            jitter: reader.read_u32::<BigEndian>()?,
            lsr: reader.read_u32::<BigEndian>()?,
            dlsr: reader.read_u32::<BigEndian>()?,
        })
    }
}

impl Marshal<Result<BytesMut, RtcpError>> for ReportBlock {
    fn marshal(&self) -> Result<BytesMut, RtcpError> {
        let mut writer = BytesWriter::default();

        writer.write_u32::<BigEndian>(self.ssrc)?;
        writer.write_u8(self.fraction_lost)?;
        writer.write_u24::<BigEndian>(self.cumulative_num_of_packets_lost)?;
        writer.write_u32::<BigEndian>(self.extended_highest_seq_number)?;
        writer.write_u32::<BigEndian>(self.jitter)?;
        writer.write_u32::<BigEndian>(self.lsr)?;
        writer.write_u32::<BigEndian>(self.dlsr)?;

        Ok(writer.extract_current_bytes())
    }
}

#[derive(Debug, Clone, Default)]
pub struct RtcpReceiverReport {
    pub header: RtcpHeader,
    pub ssrc: u32,
    pub report_blocks: Vec<ReportBlock>,
}

impl Unmarshal<BytesMut, Result<Self, RtcpError>> for RtcpReceiverReport {
    fn unmarshal(data: BytesMut) -> Result<Self, RtcpError>
    where
        Self: Sized,
    {
        let mut reader = BytesReader::new(data);

        let mut receiver_report = RtcpReceiverReport {
            header: RtcpHeader::unmarshal(&mut reader)?,
            ssrc: reader.read_u32::<BigEndian>()?,
            ..Default::default()
        };

        for _ in 0..receiver_report.header.report_count {
            let report_block = ReportBlock::unmarshal(&mut reader)?;
            receiver_report.report_blocks.push(report_block);
        }

        Ok(receiver_report)
    }
}

impl Marshal<Result<BytesMut, RtcpError>> for RtcpReceiverReport {
    fn marshal(&self) -> Result<BytesMut, RtcpError> {
        let mut writer = BytesWriter::default();

        let header_bytesmut = self.header.marshal()?;
        writer.write(&header_bytesmut[..])?;

        writer.write_u32::<BigEndian>(self.ssrc)?;
        for report_block in &self.report_blocks {
            let data = report_block.marshal()?;
            writer.write(&data[..])?;
        }

        Ok(writer.extract_current_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    // ============================================
    // ReportBlock Tests
    // ============================================

    #[test]
    fn test_report_block_default() {
        let rb = ReportBlock::default();
        assert_eq!(rb.ssrc, 0);
        assert_eq!(rb.fraction_lost, 0);
        assert_eq!(rb.cumulative_num_of_packets_lost, 0);
        assert_eq!(rb.extended_highest_seq_number, 0);
        assert_eq!(rb.jitter, 0);
        assert_eq!(rb.lsr, 0);
        assert_eq!(rb.dlsr, 0);
    }

    #[test]
    fn test_report_block_marshal_unmarshal_roundtrip() {
        let original = ReportBlock {
            ssrc: 0x12345678,
            fraction_lost: 0xAB,
            cumulative_num_of_packets_lost: 0xCCDDEE,
            extended_highest_seq_number: 0xEEEEEEEE,
            jitter: 0xFFFFFFFF,
            lsr: 0x11111111,
            dlsr: 0x22222222,
        };

        let marshaled = original.marshal().unwrap();
        let mut reader = BytesReader::new(marshaled);
        let unmarshaled = ReportBlock::unmarshal(&mut reader).unwrap();

        assert_eq!(original.ssrc, unmarshaled.ssrc);
        assert_eq!(original.fraction_lost, unmarshaled.fraction_lost);
        assert_eq!(original.cumulative_num_of_packets_lost, unmarshaled.cumulative_num_of_packets_lost);
        assert_eq!(original.extended_highest_seq_number, unmarshaled.extended_highest_seq_number);
        assert_eq!(original.jitter, unmarshaled.jitter);
        assert_eq!(original.lsr, unmarshaled.lsr);
        assert_eq!(original.dlsr, unmarshaled.dlsr);
    }

    #[test]
    fn test_report_block_marshal_size() {
        let rb = ReportBlock::default();
        let marshaled = rb.marshal().unwrap();
        // SSRC (4) + fraction_lost (1) + cumulative_num_of_packets_lost (3) + extended_highest_seq_number (4) + jitter (4) + lsr (4) + dlsr (4) = 24 bytes
        assert_eq!(marshaled.len(), 24);
    }

    // ============================================
    // RtcpReceiverReport Tests
    // ============================================

    #[test]
    fn test_default_rtcp_receiver_report() {
        let rr = RtcpReceiverReport::default();
        assert_eq!(rr.ssrc, 0);
        assert!(rr.report_blocks.is_empty());
    }

    #[test]
    fn test_clone_rtcp_receiver_report() {
        let mut rr = RtcpReceiverReport::default();
        rr.ssrc = 0x12345678;
        rr.header.report_count = 1;
        rr.report_blocks.push(ReportBlock {
            ssrc: 0xAAAAAAAA,
            ..Default::default()
        });

        let cloned = rr.clone();
        assert_eq!(rr.ssrc, cloned.ssrc);
        assert_eq!(rr.report_blocks.len(), cloned.report_blocks.len());
    }

    #[test]
    fn test_marshal_minimal_receiver_report() {
        let rr = RtcpReceiverReport {
            header: RtcpHeader {
                version: 2,
                payload_type: 201, // RR
                report_count: 0,
                length: 1, // (8 bytes - 4) / 4 = 1
                ..Default::default()
            },
            ssrc: 0x12345678,
            report_blocks: vec![],
        };

        let result = rr.marshal().unwrap();
        // Header (4) + SSRC (4) = 8 bytes
        assert_eq!(result.len(), 8);
    }

    #[test]
    fn test_marshal_receiver_report_with_report_blocks() {
        let rr = RtcpReceiverReport {
            header: RtcpHeader {
                version: 2,
                payload_type: 201,
                report_count: 2,
                length: 7, // (8 + 24*2 - 4) / 4 = 13
                ..Default::default()
            },
            ssrc: 0x12345678,
            report_blocks: vec![
                ReportBlock {
                    ssrc: 0x11111111,
                    ..Default::default()
                },
                ReportBlock {
                    ssrc: 0x22222222,
                    ..Default::default()
                },
            ],
        };

        let result = rr.marshal().unwrap();
        // Header (4) + SSRC (4) + ReportBlock1 (24) + ReportBlock2 (24) = 56 bytes
        assert_eq!(result.len(), 56);
    }

    #[test]
    fn test_unmarshal_minimal_receiver_report() {
        let mut writer = BytesWriter::default();
        // Header
        writer.write_u8(0x80).unwrap(); // version(2) | padding(0) | report_count(0)
        writer.write_u8(201).unwrap(); // payload_type
        writer.write_u16::<BigEndian>(1).unwrap(); // length
        // SSRC
        writer.write_u32::<BigEndian>(0x12345678).unwrap();

        let data = writer.extract_current_bytes();
        let rr = RtcpReceiverReport::unmarshal(data).unwrap();

        assert_eq!(rr.header.version, 2);
        assert_eq!(rr.header.payload_type, 201);
        assert_eq!(rr.header.report_count, 0);
        assert_eq!(rr.ssrc, 0x12345678);
        assert!(rr.report_blocks.is_empty());
    }

    #[test]
    fn test_unmarshal_receiver_report_with_report_blocks() {
        let mut writer = BytesWriter::default();
        // Header
        writer.write_u8(0x81).unwrap(); // version(2) | padding(0) | report_count(1)
        writer.write_u8(201).unwrap(); // payload_type
        writer.write_u16::<BigEndian>(7).unwrap(); // length
        // SSRC
        writer.write_u32::<BigEndian>(0x12345678).unwrap();
        // ReportBlock
        writer.write_u32::<BigEndian>(0xAAAAAAAA).unwrap(); // SSRC
        writer.write_u8(0xBB).unwrap(); // fraction_lost
        writer.write_u24::<BigEndian>(0xCCDDEE).unwrap(); // cumulative_num_of_packets_lost
        writer.write_u32::<BigEndian>(0xEEEEEEEE).unwrap(); // extended_highest_seq_number
        writer.write_u32::<BigEndian>(0xFFFFFFFF).unwrap(); // jitter
        writer.write_u32::<BigEndian>(0x11111111).unwrap(); // lsr
        writer.write_u32::<BigEndian>(0x22222222).unwrap(); // dlsr

        let data = writer.extract_current_bytes();
        let rr = RtcpReceiverReport::unmarshal(data).unwrap();

        assert_eq!(rr.header.report_count, 1);
        assert_eq!(rr.report_blocks.len(), 1);
        assert_eq!(rr.report_blocks[0].ssrc, 0xAAAAAAAA);
        assert_eq!(rr.report_blocks[0].fraction_lost, 0xBB);
    }

    #[test]
    fn test_unmarshal_receiver_report_multiple_report_blocks() {
        let mut writer = BytesWriter::default();
        // Header with report_count = 2
        writer.write_u8(0x82).unwrap(); // version(2) | padding(0) | report_count(2)
        writer.write_u8(201).unwrap();
        writer.write_u16::<BigEndian>(13).unwrap(); // length for 2 report blocks
        // SSRC
        writer.write_u32::<BigEndian>(0x12345678).unwrap();
        // First ReportBlock
        writer.write_u32::<BigEndian>(0x11111111).unwrap();
        writer.write_u8(0).unwrap();
        writer.write_u24::<BigEndian>(0).unwrap();
        writer.write_u32::<BigEndian>(0).unwrap();
        writer.write_u32::<BigEndian>(0).unwrap();
        writer.write_u32::<BigEndian>(0).unwrap();
        writer.write_u32::<BigEndian>(0).unwrap();
        // Second ReportBlock
        writer.write_u32::<BigEndian>(0x22222222).unwrap();
        writer.write_u8(0).unwrap();
        writer.write_u24::<BigEndian>(0).unwrap();
        writer.write_u32::<BigEndian>(0).unwrap();
        writer.write_u32::<BigEndian>(0).unwrap();
        writer.write_u32::<BigEndian>(0).unwrap();
        writer.write_u32::<BigEndian>(0).unwrap();

        let data = writer.extract_current_bytes();
        let rr = RtcpReceiverReport::unmarshal(data).unwrap();

        assert_eq!(rr.header.report_count, 2);
        assert_eq!(rr.report_blocks.len(), 2);
        assert_eq!(rr.report_blocks[0].ssrc, 0x11111111);
        assert_eq!(rr.report_blocks[1].ssrc, 0x22222222);
    }

    #[test]
    fn test_unmarshal_not_enough_bytes() {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[0x80, 201, 0x00, 0x01]); // Only header
        let result = RtcpReceiverReport::unmarshal(buf);
        assert!(result.is_err());
    }

    // ============================================
    // Round-trip Tests (Marshal -> Unmarshal)
    // ============================================

    #[test]
    fn test_rtcp_receiver_report_marshal_unmarshal_roundtrip() {
        let original = RtcpReceiverReport {
            header: RtcpHeader {
                version: 2,
                payload_type: 201,
                report_count: 1,
                length: 7,
                ..Default::default()
            },
            ssrc: 0x12345678,
            report_blocks: vec![ReportBlock {
                ssrc: 0xAAAAAAAA,
                fraction_lost: 0xBB,
                cumulative_num_of_packets_lost: 0xCCDDEE,
                extended_highest_seq_number: 0xEEEEEEEE,
                jitter: 0xFFFFFFFF,
                lsr: 0x11111111,
                dlsr: 0x22222222,
            }],
        };

        let marshaled = original.marshal().unwrap();
        let unmarshaled = RtcpReceiverReport::unmarshal(marshaled).unwrap();

        assert_eq!(original.ssrc, unmarshaled.ssrc);
        assert_eq!(original.report_blocks.len(), unmarshaled.report_blocks.len());
        if !original.report_blocks.is_empty() {
            assert_eq!(original.report_blocks[0].ssrc, unmarshaled.report_blocks[0].ssrc);
            assert_eq!(original.report_blocks[0].fraction_lost, unmarshaled.report_blocks[0].fraction_lost);
        }
    }

    #[test]
    fn test_rtcp_receiver_report_roundtrip_no_report_blocks() {
        let original = RtcpReceiverReport {
            header: RtcpHeader {
                version: 2,
                payload_type: 201,
                report_count: 0,
                length: 1,
                ..Default::default()
            },
            ssrc: 0x12345678,
            report_blocks: vec![],
        };

        let marshaled = original.marshal().unwrap();
        let unmarshaled = RtcpReceiverReport::unmarshal(marshaled).unwrap();

        assert_eq!(original.ssrc, unmarshaled.ssrc);
        assert!(unmarshaled.report_blocks.is_empty());
    }

    // ============================================
    // Property-based Tests (proptest)
    // ============================================

    use proptest::prelude::*;
    proptest! {
        #[test]
        fn test_report_block_property_roundtrip(ssrc in 0u32..=u32::MAX, fraction_lost in 0u8..=255u8, cumulative_lost in 0u32..=0xFFFFFFu32, ext_seq in 0u32..=u32::MAX, jitter in 0u32..=u32::MAX, lsr in 0u32..=u32::MAX, dlsr in 0u32..=u32::MAX) {
            let original = ReportBlock {
                ssrc,
                fraction_lost,
                cumulative_num_of_packets_lost: cumulative_lost,
                extended_highest_seq_number: ext_seq,
                jitter,
                lsr,
                dlsr,
            };

            let marshaled = original.marshal().unwrap();
            let mut reader = BytesReader::new(marshaled);
            let unmarshaled = ReportBlock::unmarshal(&mut reader).unwrap();

            assert_eq!(original.ssrc, unmarshaled.ssrc);
            assert_eq!(original.fraction_lost, unmarshaled.fraction_lost);
            assert_eq!(original.cumulative_num_of_packets_lost, unmarshaled.cumulative_num_of_packets_lost);
            assert_eq!(original.extended_highest_seq_number, unmarshaled.extended_highest_seq_number);
            assert_eq!(original.jitter, unmarshaled.jitter);
            assert_eq!(original.lsr, unmarshaled.lsr);
            assert_eq!(original.dlsr, unmarshaled.dlsr);
        }
    }

    proptest! {
        #[test]
        fn test_rtcp_receiver_report_property_roundtrip(ssrc in 0u32..=u32::MAX, report_count in 0u8..=15u8) {
            let mut report_blocks = Vec::new();
            for i in 0..report_count {
                report_blocks.push(ReportBlock {
                    ssrc: 0x10000000 + (i as u32),
                    ..Default::default()
                });
            }

            let original = RtcpReceiverReport {
                header: RtcpHeader {
                    version: 2,
                    payload_type: 201,
                    report_count,
                    length: 1 + (report_count as u16 * 6), // Base length + report blocks
                    ..Default::default()
                },
                ssrc,
                report_blocks,
            };

            let marshaled = original.marshal().unwrap();
            let unmarshaled = RtcpReceiverReport::unmarshal(marshaled).unwrap();

            assert_eq!(original.ssrc, unmarshaled.ssrc);
            assert_eq!(original.report_blocks.len(), unmarshaled.report_blocks.len());
        }
    }
}
