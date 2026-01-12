use super::errors::RtcpError;
use crate::bytesio::bytes_reader::BytesReader;
use crate::bytesio::bytes_writer::BytesWriter;
use crate::rtsp::rtp::utils::Marshal;
use crate::rtsp::rtp::utils::Unmarshal;
use byteorder::BigEndian;
use bytes::BytesMut;

//  0                   1                   2                   3
//  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |V=2|P|    RC   |   PT          |             length            |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
#[derive(Debug, Clone, Default)]
pub struct RtcpHeader {
    pub version: u8,      // 2 bits
    pub padding_flag: u8, // 1 bit
    pub report_count: u8, // 5 bit
    pub payload_type: u8, // 8 bit
    pub length: u16,      // 16 bits
}

impl Unmarshal<&mut BytesReader, Result<Self, RtcpError>> for RtcpHeader {
    fn unmarshal(reader: &mut BytesReader) -> Result<Self, RtcpError>
    where
        Self: Sized,
    {
        let mut rtcp_header = RtcpHeader::default();

        let byte_1st: u8 = reader.read_u8()?;
        rtcp_header.version = byte_1st >> 6;
        rtcp_header.padding_flag = (byte_1st >> 5) & 0x01;
        rtcp_header.report_count = byte_1st & 0x1F;
        rtcp_header.payload_type = reader.read_u8()?;
        rtcp_header.length = reader.read_u16::<BigEndian>()?;

        Ok(rtcp_header)
    }
}

impl Marshal<Result<BytesMut, RtcpError>> for RtcpHeader {
    fn marshal(&self) -> Result<BytesMut, RtcpError> {
        let mut writer = BytesWriter::default();

        let byte_1st: u8 = (self.version << 6) | (self.padding_flag << 5) | self.report_count;

        writer.write_u8(byte_1st)?;
        writer.write_u8(self.payload_type)?;
        writer.write_u16::<BigEndian>(self.length)?;

        Ok(writer.extract_current_bytes())
    }
}

// 88
// 10 0 01000
// 81
// 10 0 00001

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    // ============================================
    // Construction and Default Tests
    // ============================================

    #[test]
    fn test_default_rtcp_header() {
        let header = RtcpHeader::default();
        assert_eq!(header.version, 0);
        assert_eq!(header.padding_flag, 0);
        assert_eq!(header.report_count, 0);
        assert_eq!(header.payload_type, 0);
        assert_eq!(header.length, 0);
    }

    #[test]
    fn test_clone_rtcp_header() {
        let header = RtcpHeader {
            version: 2,
            padding_flag: 1,
            report_count: 5,
            payload_type: 200,
            length: 1000,
        };
        let cloned = header.clone();
        assert_eq!(header.version, cloned.version);
        assert_eq!(header.padding_flag, cloned.padding_flag);
        assert_eq!(header.report_count, cloned.report_count);
        assert_eq!(header.payload_type, cloned.payload_type);
        assert_eq!(header.length, cloned.length);
    }

    // ============================================
    // Marshal Tests - Bit Field Packing
    // ============================================

    #[test]
    fn test_marshal_minimal_header() {
        let header = RtcpHeader {
            version: 2,
            padding_flag: 0,
            report_count: 0,
            payload_type: 200,
            length: 0,
        };
        let result = header.marshal().unwrap();
        assert_eq!(result.len(), 4); // RTCP header is 4 bytes
        assert_eq!(result[0], 0x80); // Version 2 (10) in first 2 bits
        assert_eq!(result[1], 200); // Payload type
        assert_eq!(result[2], 0x00); // Length high byte
        assert_eq!(result[3], 0x00); // Length low byte
    }

    #[test]
    fn test_marshal_version_field() {
        // Version 2 (standard RTCP)
        let header = RtcpHeader {
            version: 2,
            ..Default::default()
        };
        let result = header.marshal().unwrap();
        assert_eq!(result[0] >> 6, 2);

        // Version 0
        let header = RtcpHeader {
            version: 0,
            ..Default::default()
        };
        let result = header.marshal().unwrap();
        assert_eq!(result[0] >> 6, 0);

        // Version 3 (max value for 2 bits)
        let header = RtcpHeader {
            version: 3,
            ..Default::default()
        };
        let result = header.marshal().unwrap();
        assert_eq!(result[0] >> 6, 3);
    }

    #[test]
    fn test_marshal_padding_flag() {
        // Padding flag set
        let header = RtcpHeader {
            version: 2,
            padding_flag: 1,
            ..Default::default()
        };
        let result = header.marshal().unwrap();
        assert_eq!((result[0] >> 5) & 0x01, 1);

        // Padding flag clear
        let header = RtcpHeader {
            version: 2,
            padding_flag: 0,
            ..Default::default()
        };
        let result = header.marshal().unwrap();
        assert_eq!((result[0] >> 5) & 0x01, 0);
    }

    #[test]
    fn test_marshal_report_count() {
        // Report count 0
        let header = RtcpHeader {
            version: 2,
            report_count: 0,
            ..Default::default()
        };
        let result = header.marshal().unwrap();
        assert_eq!(result[0] & 0x1F, 0);

        // Report count 5
        let header = RtcpHeader {
            version: 2,
            report_count: 5,
            ..Default::default()
        };
        let result = header.marshal().unwrap();
        assert_eq!(result[0] & 0x1F, 5);

        // Report count 31 (max value for 5 bits)
        let header = RtcpHeader {
            version: 2,
            report_count: 31,
            ..Default::default()
        };
        let result = header.marshal().unwrap();
        assert_eq!(result[0] & 0x1F, 31);
    }

    #[test]
    fn test_marshal_payload_type() {
        let header = RtcpHeader {
            version: 2,
            payload_type: 200, // SR
            ..Default::default()
        };
        let result = header.marshal().unwrap();
        assert_eq!(result[1], 200);

        let header = RtcpHeader {
            version: 2,
            payload_type: 201, // RR
            ..Default::default()
        };
        let result = header.marshal().unwrap();
        assert_eq!(result[1], 201);
    }

    #[test]
    fn test_marshal_length() {
        let header = RtcpHeader {
            version: 2,
            length: 0x1234,
            ..Default::default()
        };
        let result = header.marshal().unwrap();
        assert_eq!(result[2], 0x12);
        assert_eq!(result[3], 0x34);

        // Max length
        let header = RtcpHeader {
            version: 2,
            length: 0xFFFF,
            ..Default::default()
        };
        let result = header.marshal().unwrap();
        assert_eq!(result[2], 0xFF);
        assert_eq!(result[3], 0xFF);
    }

    #[test]
    fn test_marshal_all_fields_combined() {
        let header = RtcpHeader {
            version: 2,
            padding_flag: 1,
            report_count: 15,
            payload_type: 200,
            length: 0xABCD,
        };
        let result = header.marshal().unwrap();
        assert_eq!(result.len(), 4);
        // First byte: version(2) | padding(1) | report_count(15)
        // 10 | 1 | 01111 = 10101111 = 0xAF
        assert_eq!(result[0], 0xAF);
        assert_eq!(result[1], 200);
        assert_eq!(result[2], 0xAB);
        assert_eq!(result[3], 0xCD);
    }

    // ============================================
    // Unmarshal Tests
    // ============================================

    #[test]
    fn test_unmarshal_minimal_header() {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[0x80, 200, 0x00, 0x00]);
        let mut reader = BytesReader::new(buf);
        let header = RtcpHeader::unmarshal(&mut reader).unwrap();

        assert_eq!(header.version, 2);
        assert_eq!(header.padding_flag, 0);
        assert_eq!(header.report_count, 0);
        assert_eq!(header.payload_type, 200);
        assert_eq!(header.length, 0);
    }

    #[test]
    fn test_unmarshal_all_fields() {
        let mut buf = BytesMut::new();
        // version(2) | padding(1) | report_count(15) = 10101111 = 0xAF
        buf.extend_from_slice(&[0xAF, 200, 0x12, 0x34]);
        let mut reader = BytesReader::new(buf);
        let header = RtcpHeader::unmarshal(&mut reader).unwrap();

        assert_eq!(header.version, 2);
        assert_eq!(header.padding_flag, 1);
        assert_eq!(header.report_count, 15);
        assert_eq!(header.payload_type, 200);
        assert_eq!(header.length, 0x1234);
    }

    #[test]
    fn test_unmarshal_version_variations() {
        // Version 0
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[0x00, 200, 0x00, 0x00]);
        let mut reader = BytesReader::new(buf);
        let header = RtcpHeader::unmarshal(&mut reader).unwrap();
        assert_eq!(header.version, 0);

        // Version 2
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[0x80, 200, 0x00, 0x00]);
        let mut reader = BytesReader::new(buf);
        let header = RtcpHeader::unmarshal(&mut reader).unwrap();
        assert_eq!(header.version, 2);

        // Version 3
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[0xC0, 200, 0x00, 0x00]);
        let mut reader = BytesReader::new(buf);
        let header = RtcpHeader::unmarshal(&mut reader).unwrap();
        assert_eq!(header.version, 3);
    }

    #[test]
    fn test_unmarshal_report_count_variations() {
        for count in 0..=31u8 {
            let mut buf = BytesMut::new();
            // version(2) | padding(0) | report_count(count)
            let first_byte = (2 << 6) | count;
            buf.extend_from_slice(&[first_byte, 200, 0x00, 0x00]);
            let mut reader = BytesReader::new(buf);
            let header = RtcpHeader::unmarshal(&mut reader).unwrap();
            assert_eq!(header.report_count, count);
        }
    }

    #[test]
    fn test_unmarshal_not_enough_bytes() {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(&[0x80, 200]); // Only 2 bytes
        let mut reader = BytesReader::new(buf);
        let result = RtcpHeader::unmarshal(&mut reader);
        assert!(result.is_err());
    }

    // ============================================
    // Round-trip Tests (Marshal -> Unmarshal)
    // ============================================

    #[test]
    fn test_rtcp_header_marshal_unmarshal_roundtrip() {
        let original = RtcpHeader {
            version: 2,
            padding_flag: 1,
            report_count: 5,
            payload_type: 200,
            length: 0x1234,
        };

        let marshaled = original.marshal().unwrap();
        let mut reader = BytesReader::new(marshaled);
        let unmarshaled = RtcpHeader::unmarshal(&mut reader).unwrap();

        assert_eq!(original.version, unmarshaled.version);
        assert_eq!(original.padding_flag, unmarshaled.padding_flag);
        assert_eq!(original.report_count, unmarshaled.report_count);
        assert_eq!(original.payload_type, unmarshaled.payload_type);
        assert_eq!(original.length, unmarshaled.length);
    }

    #[test]
    fn test_rtcp_header_roundtrip_all_versions() {
        for version in 0..=3u8 {
            let original = RtcpHeader {
                version,
                ..Default::default()
            };
            let marshaled = original.marshal().unwrap();
            let mut reader = BytesReader::new(marshaled);
            let unmarshaled = RtcpHeader::unmarshal(&mut reader).unwrap();
            assert_eq!(original.version, unmarshaled.version);
        }
    }

    #[test]
    fn test_rtcp_header_roundtrip_all_report_counts() {
        for count in 0..=31u8 {
            let original = RtcpHeader {
                version: 2,
                report_count: count,
                ..Default::default()
            };
            let marshaled = original.marshal().unwrap();
            let mut reader = BytesReader::new(marshaled);
            let unmarshaled = RtcpHeader::unmarshal(&mut reader).unwrap();
            assert_eq!(original.report_count, unmarshaled.report_count);
        }
    }

    // ============================================
    // Property-based Tests (proptest)
    // ============================================

    use proptest::prelude::*;
    proptest! {
        #[test]
        fn test_rtcp_header_property_roundtrip(version in 0u8..=3u8, padding_flag in 0u8..=1u8, report_count in 0u8..=31u8, payload_type in 0u8..=255u8, length in 0u16..=65535u16) {
            let original = RtcpHeader {
                version,
                padding_flag,
                report_count,
                payload_type,
                length,
            };

            let marshaled = original.marshal().unwrap();
            let mut reader = BytesReader::new(marshaled);
            let unmarshaled = RtcpHeader::unmarshal(&mut reader).unwrap();

            assert_eq!(original.version, unmarshaled.version);
            assert_eq!(original.padding_flag, unmarshaled.padding_flag);
            assert_eq!(original.report_count, unmarshaled.report_count);
            assert_eq!(original.payload_type, unmarshaled.payload_type);
            assert_eq!(original.length, unmarshaled.length);
        }
    }
}
