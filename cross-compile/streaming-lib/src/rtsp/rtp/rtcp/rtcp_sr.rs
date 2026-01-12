use super::errors::RtcpError;
use super::rtcp_header::RtcpHeader;
use super::rtcp_rr::ReportBlock;
use crate::bytesio::bytes_reader::BytesReader;
use crate::bytesio::bytes_writer::BytesWriter;
use crate::rtsp::rtp::utils::Marshal;
use crate::rtsp::rtp::utils::Unmarshal;
use byteorder::BigEndian;
use bytes::BytesMut;

// 0                   1                   2                   3
// 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
// 			+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// header 	|V=2|P|    RC   |   PT=SR=200   |             length            |
// 			+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// 			|                         SSRC of sender                        |
// 			+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+
// sender 	|              NTP timestamp, most significant word             |
// info   	+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// 			|             NTP timestamp, least significant word             |
// 			+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// 			|                         RTP timestamp                         |
// 			+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// 			|                     sender's packet count                     |
// 			+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// 			|                      sender's octet count                     |
// 			+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+
// report 	|                 SSRC_1 (SSRC of first source)                 |
// block  	+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// 1    	| fraction lost |       cumulative number of packets lost       |
// 			+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// 			|           extended highest sequence number received           |
// 			+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// 			|                      interarrival jitter                      |
// 			+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// 			|                         last SR (LSR)                         |
// 			+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// 			|                   delay since last SR (DLSR)                  |
// 			+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+
// report 	|                 SSRC_2 (SSRC of second source)                |
// block  	+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// 2    	:                               ...                             :
// 			+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+=+
// 			|                  profile-specific extensions                  |
// 			+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+

#[derive(Debug, Clone, Default)]
pub struct RtcpSenderReport {
    pub header: RtcpHeader,
    pub ssrc: u32,
    pub ntp: u64,
    rtp_timestamp: u32,
    sender_packet_count: u32,
    sender_octet_count: u32,
    pub report_blocks: Vec<ReportBlock>,
}

impl Unmarshal<&mut BytesReader, Result<Self, RtcpError>> for RtcpSenderReport {
    fn unmarshal(reader: &mut BytesReader) -> Result<Self, RtcpError>
    where
        Self: Sized,
    {
        let mut sender_report = RtcpSenderReport {
            header: RtcpHeader::unmarshal(reader)?,
            ssrc: reader.read_u32::<BigEndian>()?,
            ntp: reader.read_u64::<BigEndian>()?,
            rtp_timestamp: reader.read_u32::<BigEndian>()?,
            sender_packet_count: reader.read_u32::<BigEndian>()?,
            sender_octet_count: reader.read_u32::<BigEndian>()?,
            ..Default::default()
        };

        for _ in 0..sender_report.header.report_count {
            let report_block = ReportBlock::unmarshal(reader)?;
            sender_report.report_blocks.push(report_block);
        }

        Ok(sender_report)
    }
}

impl Marshal<Result<BytesMut, RtcpError>> for RtcpSenderReport {
    fn marshal(&self) -> Result<BytesMut, RtcpError> {
        let mut writer = BytesWriter::default();

        let header_bytesmut = self.header.marshal()?;
        writer.write(&header_bytesmut[..])?;

        writer.write_u32::<BigEndian>(self.ssrc)?;
        writer.write_u64::<BigEndian>(self.ntp)?;
        writer.write_u32::<BigEndian>(self.rtp_timestamp)?;
        writer.write_u32::<BigEndian>(self.sender_packet_count)?;
        writer.write_u32::<BigEndian>(self.sender_octet_count)?;

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
    // Construction and Default Tests
    // ============================================

    #[test]
    fn test_default_rtcp_sender_report() {
        let sr = RtcpSenderReport::default();
        assert_eq!(sr.ssrc, 0);
        assert_eq!(sr.ntp, 0);
        assert!(sr.report_blocks.is_empty());
    }

    #[test]
    fn test_clone_rtcp_sender_report() {
        let mut sr = RtcpSenderReport::default();
        sr.ssrc = 0x12345678;
        sr.ntp = 0xABCDEF0123456789;
        sr.header.report_count = 2;

        let cloned = sr.clone();
        assert_eq!(sr.ssrc, cloned.ssrc);
        assert_eq!(sr.ntp, cloned.ntp);
        assert_eq!(sr.report_blocks.len(), cloned.report_blocks.len());
    }

    // ============================================
    // Marshal Tests
    // ============================================

    #[test]
    fn test_marshal_minimal_sender_report() {
        let sr = RtcpSenderReport {
            header: RtcpHeader {
                version: 2,
                payload_type: 200, // SR
                report_count: 0,
                length: 6, // (28 bytes - 4) / 4 = 6
                ..Default::default()
            },
            ssrc: 0x12345678,
            ntp: 0,
            rtp_timestamp: 0,
            sender_packet_count: 0,
            sender_octet_count: 0,
            report_blocks: vec![],
        };

        let result = sr.marshal().unwrap();
        // Header (4) + SSRC (4) + NTP (8) + RTP timestamp (4) + packet count (4) + octet count (4) = 28 bytes
        assert_eq!(result.len(), 28);
    }

    #[test]
    fn test_marshal_sender_report_with_report_blocks() {
        let mut sr = RtcpSenderReport {
            header: RtcpHeader {
                version: 2,
                payload_type: 200,
                report_count: 1,
                length: 8, // (28 + 24 - 4) / 4 = 12
                ..Default::default()
            },
            ssrc: 0x12345678,
            ntp: 0xABCDEF0123456789,
            rtp_timestamp: 0x11111111,
            sender_packet_count: 0x22222222,
            sender_octet_count: 0x33333333,
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

        let result = sr.marshal().unwrap();
        // Header (4) + SSRC (4) + NTP (8) + RTP timestamp (4) + packet count (4) + octet count (4) + ReportBlock (24) = 52 bytes
        assert_eq!(result.len(), 52);
    }

    // ============================================
    // Unmarshal Tests
    // ============================================

    #[test]
    fn test_unmarshal_minimal_sender_report() {
        let mut writer = BytesWriter::default();
        // Header
        writer.write_u8(0x80).unwrap(); // version(2) | padding(0) | report_count(0)
        writer.write_u8(200).unwrap(); // payload_type
        writer.write_u16::<BigEndian>(6).unwrap(); // length
        // SSRC
        writer.write_u32::<BigEndian>(0x12345678).unwrap();
        // NTP
        writer.write_u64::<BigEndian>(0xABCDEF0123456789).unwrap();
        // RTP timestamp
        writer.write_u32::<BigEndian>(0x11111111).unwrap();
        // Packet count
        writer.write_u32::<BigEndian>(0x22222222).unwrap();
        // Octet count
        writer.write_u32::<BigEndian>(0x33333333).unwrap();

        let mut reader = BytesReader::new(writer.extract_current_bytes());
        let sr = RtcpSenderReport::unmarshal(&mut reader).unwrap();

        assert_eq!(sr.header.version, 2);
        assert_eq!(sr.header.payload_type, 200);
        assert_eq!(sr.header.report_count, 0);
        assert_eq!(sr.ssrc, 0x12345678);
        assert_eq!(sr.ntp, 0xABCDEF0123456789);
        assert_eq!(sr.rtp_timestamp, 0x11111111);
        assert_eq!(sr.sender_packet_count, 0x22222222);
        assert_eq!(sr.sender_octet_count, 0x33333333);
        assert!(sr.report_blocks.is_empty());
    }

    #[test]
    fn test_unmarshal_sender_report_with_report_blocks() {
        let mut writer = BytesWriter::default();
        // Header
        writer.write_u8(0x81).unwrap(); // version(2) | padding(0) | report_count(1)
        writer.write_u8(200).unwrap(); // payload_type
        writer.write_u16::<BigEndian>(8).unwrap(); // length
        // SSRC
        writer.write_u32::<BigEndian>(0x12345678).unwrap();
        // NTP
        writer.write_u64::<BigEndian>(0xABCDEF0123456789).unwrap();
        // RTP timestamp
        writer.write_u32::<BigEndian>(0x11111111).unwrap();
        // Packet count
        writer.write_u32::<BigEndian>(0x22222222).unwrap();
        // Octet count
        writer.write_u32::<BigEndian>(0x33333333).unwrap();
        // ReportBlock
        writer.write_u32::<BigEndian>(0xAAAAAAAA).unwrap(); // SSRC
        writer.write_u8(0xBB).unwrap(); // fraction_lost
        writer.write_u24::<BigEndian>(0xCCDDEE).unwrap(); // cumulative_num_of_packets_lost
        writer.write_u32::<BigEndian>(0xEEEEEEEE).unwrap(); // extended_highest_seq_number
        writer.write_u32::<BigEndian>(0xFFFFFFFF).unwrap(); // jitter
        writer.write_u32::<BigEndian>(0x11111111).unwrap(); // lsr
        writer.write_u32::<BigEndian>(0x22222222).unwrap(); // dlsr

        let mut reader = BytesReader::new(writer.extract_current_bytes());
        let sr = RtcpSenderReport::unmarshal(&mut reader).unwrap();

        assert_eq!(sr.header.report_count, 1);
        assert_eq!(sr.report_blocks.len(), 1);
        assert_eq!(sr.report_blocks[0].ssrc, 0xAAAAAAAA);
        assert_eq!(sr.report_blocks[0].fraction_lost, 0xBB);
        assert_eq!(sr.report_blocks[0].cumulative_num_of_packets_lost, 0xCCDDEE);
    }

    #[test]
    fn test_unmarshal_sender_report_multiple_report_blocks() {
        let mut writer = BytesWriter::default();
        // Header with report_count = 2
        writer.write_u8(0x82).unwrap(); // version(2) | padding(0) | report_count(2)
        writer.write_u8(200).unwrap();
        writer.write_u16::<BigEndian>(14).unwrap(); // length for 2 report blocks
        // SSRC
        writer.write_u32::<BigEndian>(0x12345678).unwrap();
        // NTP
        writer.write_u64::<BigEndian>(0).unwrap();
        // RTP timestamp
        writer.write_u32::<BigEndian>(0).unwrap();
        // Packet count
        writer.write_u32::<BigEndian>(0).unwrap();
        // Octet count
        writer.write_u32::<BigEndian>(0).unwrap();
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

        let mut reader = BytesReader::new(writer.extract_current_bytes());
        let sr = RtcpSenderReport::unmarshal(&mut reader).unwrap();

        assert_eq!(sr.header.report_count, 2);
        assert_eq!(sr.report_blocks.len(), 2);
        assert_eq!(sr.report_blocks[0].ssrc, 0x11111111);
        assert_eq!(sr.report_blocks[1].ssrc, 0x22222222);
    }

    #[test]
    fn test_unmarshal_not_enough_bytes() {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[0x80, 200, 0x00, 0x06]); // Only header
        let mut reader = BytesReader::new(buf);
        let result = RtcpSenderReport::unmarshal(&mut reader);
        assert!(result.is_err());
    }

    // ============================================
    // Round-trip Tests (Marshal -> Unmarshal)
    // ============================================

    #[test]
    fn test_rtcp_sender_report_marshal_unmarshal_roundtrip() {
        let original = RtcpSenderReport {
            header: RtcpHeader {
                version: 2,
                payload_type: 200,
                report_count: 1,
                length: 8,
                ..Default::default()
            },
            ssrc: 0x12345678,
            ntp: 0xABCDEF0123456789,
            rtp_timestamp: 0x11111111,
            sender_packet_count: 0x22222222,
            sender_octet_count: 0x33333333,
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
        let mut reader = BytesReader::new(marshaled);
        let unmarshaled = RtcpSenderReport::unmarshal(&mut reader).unwrap();

        assert_eq!(original.ssrc, unmarshaled.ssrc);
        assert_eq!(original.ntp, unmarshaled.ntp);
        assert_eq!(original.rtp_timestamp, unmarshaled.rtp_timestamp);
        assert_eq!(original.sender_packet_count, unmarshaled.sender_packet_count);
        assert_eq!(original.sender_octet_count, unmarshaled.sender_octet_count);
        assert_eq!(original.report_blocks.len(), unmarshaled.report_blocks.len());
        if !original.report_blocks.is_empty() {
            assert_eq!(original.report_blocks[0].ssrc, unmarshaled.report_blocks[0].ssrc);
        }
    }

    #[test]
    fn test_rtcp_sender_report_roundtrip_no_report_blocks() {
        let original = RtcpSenderReport {
            header: RtcpHeader {
                version: 2,
                payload_type: 200,
                report_count: 0,
                length: 6,
                ..Default::default()
            },
            ssrc: 0x12345678,
            ntp: 0xABCDEF0123456789,
            rtp_timestamp: 0x11111111,
            sender_packet_count: 0x22222222,
            sender_octet_count: 0x33333333,
            report_blocks: vec![],
        };

        let marshaled = original.marshal().unwrap();
        let mut reader = BytesReader::new(marshaled);
        let unmarshaled = RtcpSenderReport::unmarshal(&mut reader).unwrap();

        assert_eq!(original.ssrc, unmarshaled.ssrc);
        assert_eq!(original.ntp, unmarshaled.ntp);
        assert!(unmarshaled.report_blocks.is_empty());
    }

    // ============================================
    // Property-based Tests (proptest)
    // ============================================

    use proptest::prelude::*;
    proptest! {
        #[test]
        fn test_rtcp_sender_report_property_roundtrip(ssrc in 0u32..=u32::MAX, ntp in 0u64..=u64::MAX, rtp_ts in 0u32..=u32::MAX, pkt_count in 0u32..=u32::MAX, octet_count in 0u32..=u32::MAX, report_count in 0u8..=15u8) {
            let mut report_blocks = Vec::new();
            for _ in 0..report_count {
                report_blocks.push(ReportBlock {
                    ssrc: 0xAAAAAAAA,
                    fraction_lost: 0,
                    cumulative_num_of_packets_lost: 0,
                    extended_highest_seq_number: 0,
                    jitter: 0,
                    lsr: 0,
                    dlsr: 0,
                });
            }

            let original = RtcpSenderReport {
                header: RtcpHeader {
                    version: 2,
                    payload_type: 200,
                    report_count,
                    length: 6 + (report_count as u16 * 6), // Base length + report blocks
                    ..Default::default()
                },
                ssrc,
                ntp,
                rtp_timestamp: rtp_ts,
                sender_packet_count: pkt_count,
                sender_octet_count: octet_count,
                report_blocks,
            };

            let marshaled = original.marshal().unwrap();
            let mut reader = BytesReader::new(marshaled);
            let unmarshaled = RtcpSenderReport::unmarshal(&mut reader).unwrap();

            assert_eq!(original.ssrc, unmarshaled.ssrc);
            assert_eq!(original.ntp, unmarshaled.ntp);
            assert_eq!(original.rtp_timestamp, unmarshaled.rtp_timestamp);
            assert_eq!(original.sender_packet_count, unmarshaled.sender_packet_count);
            assert_eq!(original.sender_octet_count, unmarshaled.sender_octet_count);
        }
    }
}
